use std::time::{SystemTime, UNIX_EPOCH};

use ascend_tools::client::AscendClient;
use ascend_tools::config::Config;
use ascend_tools::error::Error;
use ascend_tools::models::FlowRunFilters;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use mockito::{Matcher, Server};

fn test_client(server: &Server) -> AscendClient {
    let key = URL_SAFE_NO_PAD.encode([42u8; 32]);
    let config =
        Config::with_overrides(Some("asc-sa-test"), Some(&key), Some(server.url().as_str()))
            .unwrap();
    AscendClient::new(config).unwrap()
}

fn mock_auth(server: &mut Server, token: &str, expiration: u64, token_expect: usize) {
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
        .with_body(
            serde_json::json!({
                "access_token": token,
                "expiration": expiration,
            })
            .to_string(),
        )
        .expect(token_expect)
        .create();
}

#[test]
fn api_error_prefers_detail_field_when_present() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-a", now + 3600, 1);

    let runtimes = server
        .mock("GET", "/api/v1/runtimes")
        .match_header("authorization", "Bearer token-a")
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(r#"{"detail":"bad runtime filter"}"#)
        .expect(1)
        .create();

    let client = test_client(&server);
    let err = client.list_runtimes(Default::default()).unwrap_err();
    runtimes.assert();
    match err {
        Error::ApiError { status, message } => {
            assert_eq!(status, 400);
            assert_eq!(message, "bad runtime filter");
        }
        _ => panic!("unexpected error variant: {err:?}"),
    }
}

#[test]
fn api_error_uses_raw_body_for_non_json_errors() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-b", now + 3600, 1);

    let runtimes = server
        .mock("GET", "/api/v1/runtimes")
        .match_header("authorization", "Bearer token-b")
        .with_status(502)
        .with_body("upstream failure")
        .expect(1)
        .create();

    let client = test_client(&server);
    let err = client.list_runtimes(Default::default()).unwrap_err();
    runtimes.assert();
    match err {
        Error::ApiError { status, message } => {
            assert_eq!(status, 502);
            assert_eq!(message, "upstream failure");
        }
        _ => panic!("unexpected error variant: {err:?}"),
    }
}

#[test]
fn encodes_query_values_and_path_segments() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-c", now + 3600, 1);

    let flow_runs = server
        .mock("GET", "/api/v1/flow-runs")
        .match_header("authorization", "Bearer token-c")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("runtime_uuid".into(), "rt /?#".into()),
            Matcher::UrlEncoded("status".into(), "running & done".into()),
            Matcher::UrlEncoded("flow".into(), "sales/etl".into()),
            Matcher::UrlEncoded("since".into(), "2026-01-01T00:00:00Z".into()),
            Matcher::UrlEncoded("until".into(), "2026-01-02T00:00:00Z".into()),
            Matcher::UrlEncoded("offset".into(), "10".into()),
            Matcher::UrlEncoded("limit".into(), "50".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .expect(1)
        .create();

    let flow_run = server
        .mock("GET", "/api/v1/flow-runs/fr%2Fwith%20space%23hash")
        .match_header("authorization", "Bearer token-c")
        .match_query(Matcher::UrlEncoded("runtime_uuid".into(), "rt /?#".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "name": "fr/with space#hash",
                "flow": "sales/etl",
                "build_uuid": "build-1",
                "runtime_uuid": "rt /?#",
                "status": "running",
                "created_at": "2026-01-01T00:00:00Z",
                "error": null,
            })
            .to_string(),
        )
        .expect(1)
        .create();

    let client = test_client(&server);
    let mut filters = FlowRunFilters::default();
    filters.status = Some("running & done".to_string());
    filters.flow = Some("sales/etl".to_string());
    filters.since = Some("2026-01-01T00:00:00Z".to_string());
    filters.until = Some("2026-01-02T00:00:00Z".to_string());
    filters.offset = Some(10);
    filters.limit = Some(50);
    let runs = client.list_flow_runs("rt /?#", filters).unwrap();
    assert!(runs.is_empty());

    let run = client.get_flow_run("rt /?#", "fr/with space#hash").unwrap();
    assert_eq!(run.name, "fr/with space#hash");
    flow_runs.assert();
    flow_run.assert();
}

#[test]
fn reuses_cached_token_until_refresh_buffer() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "cached-token", now + 3600, 1);

    let runtimes = server
        .mock("GET", "/api/v1/runtimes")
        .match_header("authorization", "Bearer cached-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .expect(2)
        .create();

    let client = test_client(&server);
    let _ = client.list_runtimes(Default::default()).unwrap();
    let _ = client.list_runtimes(Default::default()).unwrap();
    runtimes.assert();
}

#[test]
fn refreshes_token_when_expiration_is_within_buffer() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "short-lived", now + 120, 2);

    let runtimes = server
        .mock("GET", "/api/v1/runtimes")
        .match_header("authorization", "Bearer short-lived")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .expect(2)
        .create();

    let client = test_client(&server);
    let _ = client.list_runtimes(Default::default()).unwrap();
    let _ = client.list_runtimes(Default::default()).unwrap();
    runtimes.assert();
}

#[test]
fn run_flow_returns_typed_error_when_runtime_is_paused_and_resume_is_false() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-flow-a", now + 3600, 1);

    let runtime = server
        .mock("GET", "/api/v1/runtimes/rt-1")
        .match_header("authorization", "Bearer token-flow-a")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "uuid": "rt-1",
                "id": "runtime-1",
                "title": "Runtime",
                "kind": "deployment",
                "project_uuid": "p-1",
                "environment_uuid": "e-1",
                "build_uuid": null,
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z",
                "health": "running",
                "paused": true
            })
            .to_string(),
        )
        .expect(1)
        .create();

    let resume = server
        .mock("POST", "/api/v1/runtimes/rt-1:resume")
        .expect(0)
        .create();
    let run = server
        .mock("POST", "/api/v1/runtimes/rt-1/flows/sales:run")
        .expect(0)
        .create();

    let client = test_client(&server);
    let err = client.run_flow("rt-1", "sales", None, false).unwrap_err();
    runtime.assert();
    resume.assert();
    run.assert();
    assert!(matches!(err, Error::RuntimePaused));
}

#[test]
fn run_flow_resumes_paused_runtime_when_requested() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-flow-b", now + 3600, 1);

    let runtime = server
        .mock("GET", "/api/v1/runtimes/rt-1")
        .match_header("authorization", "Bearer token-flow-b")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "uuid": "rt-1",
                "id": "runtime-1",
                "title": "Runtime",
                "kind": "deployment",
                "project_uuid": "p-1",
                "environment_uuid": "e-1",
                "build_uuid": null,
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z",
                "health": "running",
                "paused": true
            })
            .to_string(),
        )
        .expect(1)
        .create();

    let resume = server
        .mock("POST", "/api/v1/runtimes/rt-1:resume")
        .match_header("authorization", "Bearer token-flow-b")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "uuid": "rt-1",
                "id": "runtime-1",
                "title": "Runtime",
                "kind": "deployment",
                "project_uuid": "p-1",
                "environment_uuid": "e-1",
                "build_uuid": null,
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z",
                "health": "running",
                "paused": false
            })
            .to_string(),
        )
        .expect(1)
        .create();

    let run = server
        .mock("POST", "/api/v1/runtimes/rt-1/flows/sales:run")
        .match_header("authorization", "Bearer token-flow-b")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"event_uuid":"event-1","event_type":"flow_run_requested"}"#)
        .expect(1)
        .create();

    let client = test_client(&server);
    let trigger = client.run_flow("rt-1", "sales", None, true).unwrap();
    assert_eq!(trigger.event_uuid, "event-1");
    runtime.assert();
    resume.assert();
    run.assert();
}

#[test]
fn run_flow_returns_typed_error_for_starting_runtime() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-flow-c", now + 3600, 1);

    let runtime = server
        .mock("GET", "/api/v1/runtimes/rt-1")
        .match_header("authorization", "Bearer token-flow-c")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "uuid": "rt-1",
                "id": "runtime-1",
                "title": "Runtime",
                "kind": "deployment",
                "project_uuid": "p-1",
                "environment_uuid": "e-1",
                "build_uuid": null,
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z",
                "health": "starting",
                "paused": false
            })
            .to_string(),
        )
        .expect(1)
        .create();

    let run = server
        .mock("POST", "/api/v1/runtimes/rt-1/flows/sales:run")
        .expect(0)
        .create();

    let client = test_client(&server);
    let err = client.run_flow("rt-1", "sales", None, false).unwrap_err();
    runtime.assert();
    run.assert();
    assert!(matches!(err, Error::RuntimeStarting));
}
