mod common;

use common::{call_tool, initialize, spawn_daemon, write_roles};
use serde_json::json;

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
