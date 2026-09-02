mod common;

use common::{call_tool, initialize, spawn_daemon, write_roles};
use serde_json::json;
use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};

struct SpawnedDaemonGuard(u32);

impl Drop for SpawnedDaemonGuard {
    fn drop(&mut self) {
        #[cfg(windows)]
        let _ = Command::new("taskkill")
            .args(["/PID", &self.0.to_string(), "/T", "/F"])
            .status();
        #[cfg(unix)]
        let _ = Command::new("kill")
            .args(["-TERM", &self.0.to_string()])
            .status();
    }
}

fn wait_for_spawned_daemon(data_dir: &std::path::Path) -> SpawnedDaemonGuard {
    let pid_file = data_dir.join("runtime").join("workflowd.pid");
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Ok(pid) = std::fs::read_to_string(&pid_file)
            && let Ok(pid) = pid.trim().parse::<u32>()
            && pid != 0
        {
            return SpawnedDaemonGuard(pid);
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("status did not start a daemon");
}

#[test]
fn status_starts_the_daemon_after_an_mcp_restart() {
    let data_dir = tempfile::tempdir().unwrap();
    let mut client = common::McpClient::spawn(data_dir.path());
    initialize(&mut client);

    // This simulates a restarted Trae Work MCP frontend before any other tool
    // has recreated the local daemon. The public status tool must self-heal.
    let status = call_tool(
        &mut client,
        "cycle_status",
        json!({"project_key": "restarted-mcp"}),
    );
    let _daemon = wait_for_spawned_daemon(data_dir.path());
    assert!(
        status["jobs"].is_array(),
        "status must return its normal envelope after daemon startup: {status}"
    );
    assert!(status["workflow"].is_null());
}

#[test]
fn mcp_frontend_drives_the_control_plane_end_to_end() {
    let data_dir = tempfile::tempdir().unwrap();
    let data_path = data_dir.path().to_path_buf();

    let _daemon = spawn_daemon(&data_path);
    let mut client = common::McpClient::spawn(&data_path);
    initialize(&mut client);

    let listed = client.request("tools/list", json!({}));
    let names = listed["result"]["tools"]
        .as_array()
        .expect("tool list")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name").to_owned())
        .collect::<Vec<_>>();
    assert_eq!(names.len(), 37);
    assert!(names.contains(&"cycle_status".to_owned()));
    assert!(names.contains(&"cycle_consent".to_owned()));

    let doctor = call_tool(&mut client, "cycle_doctor", json!({}));
    assert!(doctor.get("controlPlane").is_some());

    let limits = call_tool(&mut client, "cycle_limits", json!({}));
    assert!(limits["admission"]["maximumActive"].as_u64().is_some());

    let denied = client.request(
        "tools/call",
        json!({
            "arguments": {"mode": "quick", "original_request": "e2e request", "project_key": "e2e-project"},
            "name": "cycle_start",
        }),
    );
    assert_eq!(denied["result"]["isError"], true);

    write_roles(&data_path, "https://api.example.invalid/v1");

    let started = call_tool(
        &mut client,
        "cycle_start",
        json!({"mode": "quick", "original_request": "e2e request", "project_key": "e2e-project"}),
    );
    let workflow_id = started["workflowId"]
        .as_str()
        .expect("workflow id")
        .to_owned();
    assert_eq!(started["mode"], "quick");

    let status = call_tool(
        &mut client,
        "cycle_status",
        json!({"project_key": "e2e-project", "workflow_id": workflow_id}),
    );
    assert!(status["workflow"]["state"].is_string());

    let cancelled = call_tool(
        &mut client,
        "cycle_cancel",
        json!({"confirm": true, "project_key": "e2e-project"}),
    );
    assert_eq!(cancelled["state"], "cancelled");

    let verified = call_tool(
        &mut client,
        "cycle_history_verify",
        json!({"project_key": "e2e-project"}),
    );
    assert_eq!(verified["chain"]["status"], "valid");
}
