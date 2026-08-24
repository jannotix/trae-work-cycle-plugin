use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

pub const BINARY: &str = env!("CARGO_BIN_EXE_trae-cycle");
pub const STEP_TIMEOUT: Duration = Duration::from_secs(30);

pub struct McpClient {
    child: Child,
    stdin: Option<ChildStdin>,
    receiver: mpsc::Receiver<Value>,
    stderr_path: std::path::PathBuf,
    next_id: u64,
}

impl Drop for McpClient {
    fn drop(&mut self) {
        self.stdin.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct DaemonGuard(Child);

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl McpClient {
    pub fn spawn(data_dir: &std::path::Path) -> Self {
        let stderr_log =
            std::fs::File::create(data_dir.join("mcp-stderr.log")).expect("stderr log file");
        let mut child = Command::new(BINARY)
            .arg("mcp")
            .arg("--data-dir")
            .arg(data_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(stderr_log))
            .spawn()
            .expect("mcp frontend spawns");
        let stdin = child.stdin.take().expect("stdin pipe");
        let stdout = child.stdout.take().expect("stdout pipe");
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(message) = serde_json::from_str::<Value>(&line)
                    && sender.send(message).is_err()
                {
                    break;
                }
            }
        });
        Self {
            child,
            stdin: Some(stdin),
            receiver,
            stderr_path: data_dir.join("mcp-stderr.log"),
            next_id: 1,
        }
    }

    pub fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let message = json!({"id": id, "jsonrpc": "2.0", "method": method, "params": params});
        self.write(message);
        loop {
            let response = self
                .receiver
                .recv_timeout(STEP_TIMEOUT)
                .unwrap_or_else(|error| {
                    let stderr = std::fs::read_to_string(&self.stderr_path).unwrap_or_default();
                    panic!("mcp response within timeout: {error}; frontend stderr: {stderr}");
                });
            if response.get("id").and_then(Value::as_u64) == Some(id) {
                return response;
            }
        }
    }

    pub fn notify(&mut self, method: &str) {
        self.write(json!({"jsonrpc": "2.0", "method": method}));
    }

    fn write(&mut self, message: Value) {
        let mut line = message.to_string();
        line.push('\n');
        let stdin = self.stdin.as_mut().expect("stdin pipe");
        stdin.write_all(line.as_bytes()).expect("stdin write");
        stdin.flush().expect("stdin flush");
    }
}

pub fn initialize(client: &mut McpClient) {
    let initialized = client.request(
        "initialize",
        json!({"capabilities": {}, "protocolVersion": "2025-06-18"}),
    );
    assert_eq!(initialized["result"]["serverInfo"]["name"], "trae-cycle");
    client.notify("notifications/initialized");
}

pub fn call_tool(client: &mut McpClient, name: &str, arguments: Value) -> Value {
    let response = client.request("tools/call", json!({"arguments": arguments, "name": name}));
    let result = response["result"].clone();
    assert!(
        result.get("isError").is_some(),
        "tools/call must return an MCP result envelope"
    );
    let text = result["content"][0]["text"].as_str().expect("text content");
    serde_json::from_str(text).unwrap_or_else(|_| json!({"raw": text}))
}

pub fn spawn_daemon(data_dir: &std::path::Path) -> DaemonGuard {
    let guard = DaemonGuard(
        Command::new(BINARY)
            .arg("serve")
            .arg("--data-dir")
            .arg(data_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("daemon spawns"),
    );
    wait_for_daemon(data_dir);
    guard
}

pub fn wait_for_daemon(data_dir: &std::path::Path) {
    let secret = data_dir.join("runtime").join("ipc.secret");
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if secret.is_file() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("daemon did not create its IPC credential in time");
}

pub fn write_roles(data_dir: &std::path::Path, base_url: &str) {
    let config = data_dir.join("config");
    std::fs::create_dir_all(&config).unwrap();
    let roles = [
        "architect",
        "functional_reviewer",
        "security_reviewer",
        "arbiter",
    ]
    .iter()
    .map(|role| {
        (
            (*role).to_owned(),
            json!({
                "api_key_env": "PATH",
                "base_url": base_url,
                "model_id": format!("provider/{role}"),
            }),
        )
    })
    .collect::<serde_json::Map<_, _>>();
    std::fs::write(
        config.join("roles.json"),
        serde_json::to_string_pretty(&json!({"roles": roles, "version": 1})).unwrap(),
    )
    .unwrap();
}
