use std::fs;

use assert_cmd::Command;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use mockito::Server;
use predicates::prelude::*;
use tempfile::TempDir;

fn command_with_auth(server: &Server) -> Command {
    let mut cmd = Command::from_std(std::process::Command::new(assert_cmd::cargo::cargo_bin!(
        "ascend-tools"
    )));
    let key = URL_SAFE_NO_PAD.encode([11u8; 32]);
    // Pass auth as CLI flags (highest priority) so instance config files don't interfere.
    cmd.args([
        "--service-account-id",
        "asc-sa-test",
        "--service-account-key",
        &key,
        "--instance-api-url",
        &server.url(),
    ]);
    cmd
}

fn mock_auth(server: &mut Server) {
    server
        .mock("GET", "/api/v1/auth/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cloud_api_domain":"api.cloud.ascend.io"}"#)
        .expect(1)
        .create();

    server
        .mock("POST", "/api/v1/auth/token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"access_token":"cli-token","expiration":4102444800}"#)
        .expect(1)
        .create();
}

#[test]
fn workspace_list_text_output_regression() {
    let mut server = Server::new();
    mock_auth(&mut server);

    let runtimes = server
        .mock("GET", "/api/v1/runtimes")
        .match_header("authorization", "Bearer cli-token")
        .match_query(mockito::Matcher::UrlEncoded(
            "kind".into(),
            "workspace".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!([{
                "uuid": "rt-1",
                "id": "runtime-1",
                "title": "My Workspace",
                "kind": "workspace",
                "project_uuid": "p-1",
                "environment_uuid": "e-1",
                "build_uuid": null,
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z",
                "health": "running",
                "paused": false,
                "profile_name": "default"
            }])
            .to_string(),
        )
        .expect(1)
        .create();

    let mut cmd = command_with_auth(&server);
    cmd.args(["workspace", "list"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("TITLE"))
        .stdout(predicate::str::contains("My Workspace"))
        .stdout(predicate::str::contains("running"));

    runtimes.assert();
}

#[test]
fn workspace_list_json_output_regression() {
    let mut server = Server::new();
    mock_auth(&mut server);

    let runtimes = server
        .mock("GET", "/api/v1/runtimes")
        .match_header("authorization", "Bearer cli-token")
        .match_query(mockito::Matcher::UrlEncoded(
            "kind".into(),
            "workspace".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!([{
                "uuid": "rt-2",
                "id": "runtime-2",
                "title": "Dev Workspace",
                "kind": "workspace",
                "project_uuid": "p-2",
                "environment_uuid": "e-2",
                "build_uuid": null,
                "created_at": "2026-01-02T00:00:00Z",
                "updated_at": "2026-01-02T00:00:00Z",
                "health": "running",
                "paused": false
            }])
            .to_string(),
        )
        .expect(1)
        .create();

    let mut cmd = command_with_auth(&server);
    cmd.args(["-o", "json", "workspace", "list"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"uuid\": \"rt-2\""))
        .stdout(predicate::str::contains("\"title\": \"Dev Workspace\""));

    runtimes.assert();
}

#[test]
fn workspace_list_empty_results_go_to_stderr() {
    let mut server = Server::new();
    mock_auth(&mut server);

    let runtimes = server
        .mock("GET", "/api/v1/runtimes")
        .match_header("authorization", "Bearer cli-token")
        .match_query(mockito::Matcher::UrlEncoded(
            "kind".into(),
            "workspace".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .expect(1)
        .create();

    let mut cmd = command_with_auth(&server);
    cmd.args(["workspace", "list"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("No results."));

    runtimes.assert();
}

#[test]
fn workspace_list_surfaces_api_errors() {
    let mut server = Server::new();
    mock_auth(&mut server);

    let runtimes = server
        .mock("GET", "/api/v1/runtimes")
        .match_header("authorization", "Bearer cli-token")
        .match_query(mockito::Matcher::UrlEncoded(
            "kind".into(),
            "workspace".into(),
        ))
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(r#"{"detail":"bad filter"}"#)
        .expect(1)
        .create();

    let mut cmd = command_with_auth(&server);
    cmd.args(["workspace", "list"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("API error (HTTP 400): bad filter"));

    runtimes.assert();
}

#[test]
fn deployment_list_filters_by_kind() {
    let mut server = Server::new();
    mock_auth(&mut server);

    let runtimes = server
        .mock("GET", "/api/v1/runtimes")
        .match_header("authorization", "Bearer cli-token")
        .match_query(mockito::Matcher::UrlEncoded(
            "kind".into(),
            "deployment".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!([{
                "uuid": "rt-3",
                "id": "prod-deploy",
                "title": "Production",
                "kind": "deployment",
                "project_uuid": "p-1",
                "environment_uuid": "e-1",
                "build_uuid": null,
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z",
                "health": "running",
                "paused": false,
                "enable_automations": true
            }])
            .to_string(),
        )
        .expect(1)
        .create();

    let mut cmd = command_with_auth(&server);
    cmd.args(["deployment", "list"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Production"))
        .stdout(predicate::str::contains("on"));

    runtimes.assert();
}

#[test]
fn otto_run_jsonl_emits_request_event_and_terminal_records() {
    let mut server = Server::new();
    mock_auth(&mut server);

    server
        .mock("POST", "/api/v1/otto/threads")
        .match_header("authorization", "Bearer cli-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"thread_id":"thread-jsonl"}"#)
        .expect(1)
        .create();

    let reasoning_body = serde_json::json!({
        "item_id": "rs_1",
        "content_index": 0,
        "delta": "Checking whether the failed flow needs a workspace restart."
    })
    .to_string();
    let tool_added_body = serde_json::json!({
        "item": {
            "id": "fc_1",
            "type": "function_call",
            "call_id": "call_1",
            "name": "get_flow_run",
            "arguments": "{}"
        }
    })
    .to_string();
    let tool_args_body = serde_json::json!({
        "item_id": "fc_1",
        "delta": "{\"runtime\":\"workspace-prod\",\"flow\":\"orders/daily_sync\"}"
    })
    .to_string();
    let tool_output_body = serde_json::json!({
        "call_id": "call_1",
        "output": "{\"status\":\"failed\",\"error\":\"warehouse timeout\"}"
    })
    .to_string();
    let text_delta_body = serde_json::json!({
        "delta": "The workspace is healthy; retry the flow once the warehouse is reachable."
    })
    .to_string();
    let completed_details_body = serde_json::json!({
        "id": "thread-jsonl",
        "title": "Orders sync debug",
        "messages": {},
        "updated_at": "2026-01-01T00:00:05Z",
        "is_processing": false,
        "context_window_stats": null,
        "total_message_count": 2,
        "has_more": false,
        "oldest_message_id": "msg-user-1",
        "latest_message_id": "msg-assistant-1"
    })
    .to_string();
    let sse_body = format!(
        "event: response.reasoning_text.delta\ndata: {reasoning_body}\n\n\
         event: response.output_item.added\ndata: {tool_added_body}\n\n\
         event: response.function_call_arguments.delta\ndata: {tool_args_body}\n\n\
         event: response.run_item_stream_event.tool_call_output_item\ndata: {tool_output_body}\n\n\
         event: response.output_text.delta\ndata: {text_delta_body}\n\n\
         event: thread.details\ndata: {completed_details_body}\n\n\
         :ping\n\n"
    );

    server
        .mock("GET", "/api/v1/otto/threads/thread-jsonl/updates")
        .match_header("accept", "text/event-stream")
        .with_status(200)
        .with_body(sse_body)
        .expect(1)
        .create();

    let mut cmd = command_with_auth(&server);
    cmd.args([
        "otto",
        "run",
        "Inspect the failed flow and explain whether it needs a restart.",
        "--thinking",
        "medium",
        "--jsonl",
    ]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"record_type\":\"request\""))
        .stdout(predicate::str::contains("\"thinking\":\"medium\""))
        .stdout(predicate::str::contains("\"record_type\":\"event\""))
        .stdout(predicate::str::contains(
            "\"event_type\":\"response.reasoning_text.delta\"",
        ))
        .stdout(predicate::str::contains(
            "\"event_type\":\"response.output_item.added\"",
        ))
        .stdout(predicate::str::contains(
            "\"event_type\":\"response.function_call_arguments.delta\"",
        ))
        .stdout(predicate::str::contains(
            "\"event_type\":\"response.run_item_stream_event.tool_call_output_item\"",
        ))
        .stdout(predicate::str::contains(
            "\"event_type\":\"thread.details\"",
        ))
        .stdout(predicate::str::contains("\"is_processing\":false"))
        .stdout(predicate::str::contains("\"thread_id\":\"thread-jsonl\""))
        .stdout(predicate::str::contains("\"record_type\":\"terminal\""))
        .stdout(predicate::str::contains("\"stream_status\":\"completed\""));
}

#[test]
fn otto_run_jsonl_rejects_json_output_mode() {
    let server = Server::new();
    let mut cmd = command_with_auth(&server);
    cmd.args(["-o", "json", "otto", "run", "hello", "--jsonl"]);
    cmd.assert().failure().stderr(predicate::str::contains(
        "--jsonl cannot be combined with -o json",
    ));
}

#[test]
fn otto_run_jsonl_accepts_xhigh_thinking_level() {
    let mut server = Server::new();
    mock_auth(&mut server);

    server
        .mock("POST", "/api/v1/otto/threads")
        .match_header("authorization", "Bearer cli-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"thread_id":"thread-xhigh"}"#)
        .expect(1)
        .create();

    let completed_details_body = serde_json::json!({
        "id": "thread-xhigh",
        "title": "Xhigh thinking",
        "messages": {},
        "updated_at": "2026-01-01T00:00:05Z",
        "is_processing": false,
        "context_window_stats": null,
        "total_message_count": 2,
        "has_more": false,
        "oldest_message_id": "msg-user-1",
        "latest_message_id": "msg-assistant-1"
    })
    .to_string();
    let sse_body = format!("event: thread.details\ndata: {completed_details_body}\n\n");

    server
        .mock("GET", "/api/v1/otto/threads/thread-xhigh/updates")
        .match_header("accept", "text/event-stream")
        .with_status(200)
        .with_body(sse_body)
        .expect(1)
        .create();

    let mut cmd = command_with_auth(&server);
    cmd.args([
        "otto",
        "run",
        "Explain why xhigh matters.",
        "--thinking",
        "xhigh",
        "--jsonl",
    ]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"thinking\":\"xhigh\""))
        .stdout(predicate::str::contains("\"stream_status\":\"completed\""));
}

#[test]
fn skill_install_writes_skill_file_to_target() {
    let temp_dir = TempDir::new().unwrap();
    let target = temp_dir.path().join("skills");
    let target_str = target.to_string_lossy().to_string();

    let mut cmd = Command::from_std(std::process::Command::new(assert_cmd::cargo::cargo_bin!(
        "ascend-tools"
    )));
    cmd.args(["skill", "install", "--target", &target_str]);
    cmd.assert().success().stdout(predicate::str::contains(
        "Installed ascend-tools-cli skill to",
    ));

    let skill_path = target.join("ascend-tools-cli").join("SKILL.md");
    let content = fs::read_to_string(skill_path).unwrap();
    assert!(content.contains("Ascend"));
}

#[test]
fn skill_install_all_variants() {
    let temp_dir = TempDir::new().unwrap();
    let target = temp_dir.path().join("skills");
    let target_str = target.to_string_lossy().to_string();

    let mut cmd = Command::from_std(std::process::Command::new(assert_cmd::cargo::cargo_bin!(
        "ascend-tools"
    )));
    cmd.args([
        "skill",
        "install",
        "--target",
        &target_str,
        "--cli",
        "--python",
        "--mcp",
    ]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "Installed ascend-tools-cli skill to",
        ))
        .stdout(predicate::str::contains(
            "Installed ascend-tools-python skill to",
        ))
        .stdout(predicate::str::contains(
            "Installed ascend-tools-mcp skill to",
        ));

    for (dir, marker) in [
        ("ascend-tools-cli", "ascend-tools CLI"),
        ("ascend-tools-python", "Python SDK"),
        ("ascend-tools-mcp", "MCP server"),
    ] {
        let path = target.join(dir).join("SKILL.md");
        let content =
            fs::read_to_string(&path).unwrap_or_else(|_| panic!("{} should exist", path.display()));
        assert!(
            content.contains(marker),
            "{dir}/SKILL.md should contain \"{marker}\""
        );
    }
}
