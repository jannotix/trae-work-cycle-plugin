use std::{ffi::OsString, path::PathBuf};

#[derive(Debug)]
enum Command {
    Backup {
        data_directory: PathBuf,
        destination: PathBuf,
    },
    Mcp {
        data_directory: PathBuf,
    },
    Serve {
        data_directory: PathBuf,
    },
}

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        // Tool dispatch chains are deep; debug builds need stack headroom.
        .thread_stack_size(8 * 1024 * 1024)
        .build()
        .expect("tokio runtime starts");
    let exit_code = match parse_command(std::env::args_os().skip(1).collect()) {
        Ok(Command::Mcp { data_directory }) => {
            match runtime.block_on(trae_cycle::mcp::serve_stdio(data_directory)) {
                Ok(()) => 0,
                Err(error) => {
                    eprintln!("trae-cycle mcp failed: {error}");
                    1
                }
            }
        }
        Ok(Command::Serve { data_directory }) => {
            match runtime.block_on(workflowd::lifecycle::run(&data_directory)) {
                Ok(()) => 0,
                Err(error) => {
                    eprintln!("trae-cycle serve failed: {error}");
                    1
                }
            }
        }
        Ok(Command::Backup {
            data_directory,
            destination,
        }) => match workflow_store::backup_existing_database(
            data_directory.join("control-plane.db"),
            destination,
        ) {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("trae-cycle backup failed: {error}");
                1
            }
        },
        Err(error) => {
            eprintln!("trae-cycle failed: {error}");
            2
        }
    };
    drop(runtime);
    std::process::exit(exit_code);
}

fn parse_command(arguments: Vec<OsString>) -> Result<Command, &'static str> {
    match arguments.as_slice() {
        [subcommand, flag, path] if subcommand == "mcp" && flag == "--data-dir" => {
            Ok(Command::Mcp {
                data_directory: absolute(path)?,
            })
        }
        [subcommand, flag, path] if subcommand == "serve" && flag == "--data-dir" => {
            Ok(Command::Serve {
                data_directory: absolute(path)?,
            })
        }
        [
            subcommand,
            data_flag,
            data_directory,
            backup_flag,
            destination,
        ] if subcommand == "backup" && data_flag == "--data-dir" && backup_flag == "--to" => {
            Ok(Command::Backup {
                data_directory: absolute(data_directory)?,
                destination: absolute(destination)?,
            })
        }
        _ => Err(
            "expected mcp --data-dir <path> | serve --data-dir <path> | backup --data-dir <path> --to <path>",
        ),
    }
}

fn absolute(path: &OsString) -> Result<PathBuf, &'static str> {
    let path = PathBuf::from(path);
    path.is_absolute()
        .then_some(path)
        .ok_or("path must be absolute")
}

#[cfg(test)]
mod tests {
    use super::{Command, parse_command};
    use std::ffi::OsString;

    fn data_directory() -> OsString {
        let path = std::env::temp_dir().join("trae-cycle-cli");
        std::fs::create_dir_all(&path).unwrap();
        path.into_os_string()
    }

    #[test]
    fn subcommands_require_explicit_absolute_paths() {
        let data = data_directory();
        assert!(matches!(
            parse_command(vec![
                OsString::from("serve"),
                OsString::from("--data-dir"),
                data.clone()
            ]),
            Ok(Command::Serve { .. })
        ));
        assert!(matches!(
            parse_command(vec![
                OsString::from("mcp"),
                OsString::from("--data-dir"),
                data.clone()
            ]),
            Ok(Command::Mcp { .. })
        ));
        assert!(parse_command(vec![OsString::from("serve")]).is_err());
        assert!(
            parse_command(vec![
                OsString::from("mcp"),
                OsString::from("--data-dir"),
                OsString::from("relative")
            ])
            .is_err()
        );
    }
}
