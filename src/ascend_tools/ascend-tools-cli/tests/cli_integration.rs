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
fn runtime_list_text_output_regression() {
    let mut server = Server::new();
    mock_auth(&mut server);

    let runtimes = server
        .mock("GET", "/api/v1/runtimes")
        .match_header("authorization", "Bearer cli-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!([{
                "uuid": "rt-1",
                "id": "runtime-1",
                "title": "Runtime One",
                "kind": "deployment",
                "project_uuid": "p-1",
                "environment_uuid": "e-1",
                "build_uuid": null,
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z",
                "health": "running",
                "paused": false
            }])
            .to_string(),
        )
        .expect(1)
        .create();

    let mut cmd = command_with_auth(&server);
    cmd.args(["runtime", "list"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("UUID  ID"))
        .stdout(predicate::str::contains("rt-1"))
        .stdout(predicate::str::contains("running"));

    runtimes.assert();
}

#[test]
fn runtime_list_json_output_regression() {
    let mut server = Server::new();
    mock_auth(&mut server);

    let runtimes = server
        .mock("GET", "/api/v1/runtimes")
        .match_header("authorization", "Bearer cli-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!([{
                "uuid": "rt-2",
                "id": "runtime-2",
                "title": "Runtime Two",
                "kind": "deployment",
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
    cmd.args(["-o", "json", "runtime", "list"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"uuid\": \"rt-2\""))
        .stdout(predicate::str::contains("\"title\": \"Runtime Two\""));

    runtimes.assert();
}

#[test]
fn runtime_list_empty_results_go_to_stderr() {
    let mut server = Server::new();
    mock_auth(&mut server);

    let runtimes = server
        .mock("GET", "/api/v1/runtimes")
        .match_header("authorization", "Bearer cli-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .expect(1)
        .create();

    let mut cmd = command_with_auth(&server);
    cmd.args(["runtime", "list"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("No results."));

    runtimes.assert();
}

#[test]
fn runtime_list_surfaces_api_errors() {
    let mut server = Server::new();
    mock_auth(&mut server);

    let runtimes = server
        .mock("GET", "/api/v1/runtimes")
        .match_header("authorization", "Bearer cli-token")
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(r#"{"detail":"bad filter"}"#)
        .expect(1)
        .create();

    let mut cmd = command_with_auth(&server);
    cmd.args(["runtime", "list"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("API error (HTTP 400): bad filter"));

    runtimes.assert();
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
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Installed ascend-tools skill to"));

    let skill_path = target.join("ascend-tools").join("SKILL.md");
    let content = fs::read_to_string(skill_path).unwrap();
    assert!(content.contains("Ascend"));
}
