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
    cmd.env("ASCEND_SERVICE_ACCOUNT_ID", "asc-sa-test");
    cmd.env("ASCEND_SERVICE_ACCOUNT_KEY", key);
    cmd.env("ASCEND_INSTANCE_API_URL", server.url());
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
fn otto_run_jsonl_emits_raw_thread_updates() {
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

    let preview_body = serde_json::json!({
        "id": "thread-jsonl",
        "title": "JSONL thread",
        "messages": {
            "msg-1": {
                "id": "msg-1",
                "role": "user",
                "content": "hello",
                "created_at": "2026-01-01T00:00:00Z"
            }
        },
        "updated_at": "2026-01-01T00:00:00Z",
        "is_processing": true,
        "context_window_stats": null,
        "total_message_count": 1,
        "has_more": false,
        "oldest_message_id": "msg-1",
        "latest_message_id": "msg-1"
    })
    .to_string();
    let sse_body = format!(
        "event: thread.preview\ndata: {preview_body}\n\n\
         event: response.output_text.delta\ndata: {{\"delta\":\"hi\",\"item_id\":\"item-1\",\"content_index\":0,\"output_index\":0,\"type\":\"response.output_text.delta\"}}\n\n\
         event: thread.done\ndata: {{\"latest_message_id\":\"msg-2\"}}\n\n"
    );

    server
        .mock("GET", "/api/v1/otto/threads/thread-jsonl/updates")
        .match_header("accept", "text/event-stream")
        .with_status(200)
        .with_body(sse_body)
        .expect(1)
        .create();

    let mut cmd = command_with_auth(&server);
    cmd.args(["otto", "run", "hello", "--jsonl"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"thread_id\":\"thread-jsonl\""))
        .stdout(predicate::str::contains(
            "\"event_type\":\"thread.preview\"",
        ))
        .stdout(predicate::str::contains(
            "\"event_type\":\"response.output_text.delta\"",
        ))
        .stdout(predicate::str::contains("\"event_type\":\"thread.done\""));
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
