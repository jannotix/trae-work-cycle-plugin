use std::{io, path::PathBuf, sync::Arc};

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use workflow_roles::{CONSULT_TIMEOUT, RolesClient, UsageLedger};

use crate::tools::{self, ToolContext};

const SUPPORTED_PROTOCOL_VERSION: &str = "2025-06-18";

pub async fn serve_stdio(data_dir: PathBuf) -> io::Result<()> {
    let context = Arc::new(ToolContext {
        daemon: crate::daemon::Daemon::new(data_dir.clone()),
        jobs: Arc::new(crate::jobs::Jobs::new()),
        roles: RolesClient::new(CONSULT_TIMEOUT),
        usage: Arc::new(UsageLedger::new()),
        data_dir,
    });
    let stdin = BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();
    let mut lines = stdin.lines();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(error) => {
                write_response(
                    &mut stdout,
                    json!({
                        "error": {"code": -32700, "message": format!("parse error: {error}")},
                        "id": null,
                        "jsonrpc": "2.0",
                    }),
                )
                .await?;
                continue;
            }
        };
        let Some(method) = request
            .get("method")
            .and_then(Value::as_str)
            .map(str::to_owned)
        else {
            continue;
        };
        let id = request.get("id").cloned();
        let Some(id) = id else {
            continue;
        };
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
        // Each request runs as a heap task: debug dispatch chains stay off the
        // main thread stack.
        let response = match tokio::spawn(respond(Arc::clone(&context), method, params)).await {
            Ok(response) => response,
            Err(error) => Err((-32603, format!("request handler failed: {error}"))),
        };
        let message = match response {
            Ok(result) => json!({"id": id, "jsonrpc": "2.0", "result": result}),
            Err((code, message)) => json!({
                "error": {"code": code, "message": message},
                "id": id,
                "jsonrpc": "2.0",
            }),
        };
        write_response(&mut stdout, message).await?;
    }
    Ok(())
}

async fn respond(
    context: Arc<ToolContext>,
    method: String,
    params: Value,
) -> Result<Value, (i64, String)> {
    match method.as_str() {
        "initialize" => {
            let requested = params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or(SUPPORTED_PROTOCOL_VERSION);
            Ok(json!({
                "capabilities": {"tools": {"listChanged": false}},
                "protocolVersion": requested,
                "serverInfo": {
                    "name": "trae-cycle",
                    "version": env!("CARGO_PKG_VERSION"),
                },
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tools::descriptors()})),
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or((-32602, "tools/call requires a name".to_owned()))?;
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let name = name.to_owned();
            match tools::call(&context, &name, &arguments).await {
                Ok(result) => Ok(json!({
                    "content": [{"text": result.to_string(), "type": "text"}],
                    "isError": false,
                })),
                Err(message) => Ok(json!({
                    "content": [{"text": message, "type": "text"}],
                    "isError": true,
                })),
            }
        }
        other => Err((-32601, format!("unknown method {other}"))),
    }
}

async fn write_response<W>(writer: &mut W, message: Value) -> io::Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    let mut line = message.to_string();
    line.push('\n');
    writer.write_all(line.as_bytes()).await?;
    writer.flush().await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::respond;
    use crate::tools::ToolContext;
    use serde_json::json;
    use workflow_roles::{CONSULT_TIMEOUT, RolesClient, UsageLedger};

    fn context() -> Arc<ToolContext> {
        Arc::new(ToolContext {
            daemon: crate::daemon::Daemon::new(std::env::temp_dir().join("trae-cycle-test")),
            data_dir: std::env::temp_dir().join("trae-cycle-test"),
            jobs: Arc::new(crate::jobs::Jobs::new()),
            roles: RolesClient::new(CONSULT_TIMEOUT),
            usage: Arc::new(UsageLedger::new()),
        })
    }

    #[tokio::test]
    async fn initialize_reports_the_server_identity() {
        let result = respond(
            context(),
            "initialize".to_owned(),
            json!({"protocolVersion": "2025-06-18"}),
        )
        .await
        .unwrap();
        assert_eq!(result["serverInfo"]["name"], "trae-cycle");
        assert_eq!(result["protocolVersion"], "2025-06-18");
    }

    #[tokio::test]
    async fn unknown_methods_return_method_not_found() {
        let error = respond(context(), "resources/list".to_owned(), json!({}))
            .await
            .unwrap_err();
        assert_eq!(error.0, -32601);
    }

    #[tokio::test]
    async fn tools_call_wraps_tool_errors_as_mcp_results() {
        let result = respond(
            context(),
            "tools/call".to_owned(),
            json!({"arguments": {}, "name": "cycle_models"}),
        )
        .await
        .unwrap();
        assert_eq!(result["isError"], false);
    }
}
