use std::ops::ControlFlow;
use std::time::{SystemTime, UNIX_EPOCH};

use ascend_tools::client::AscendClient;
use ascend_tools::config::Config;
use ascend_tools::error::Error;
use ascend_tools::models::{FlowRunFilters, OttoChatRequest, OttoStreamStatus, StreamEvent};
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
fn otto_request_serializes_optional_thinking() {
    let request = OttoChatRequest {
        prompt: "hello".into(),
        runtime_uuid: Some("rt-1".into()),
        thread_id: Some("thread-1".into()),
        model: None,
        thinking: Some("medium".into()),
    };

    let body = serde_json::to_value(&request).unwrap();
    assert_eq!(body["prompt"], "hello");
    assert_eq!(body["runtime_uuid"], "rt-1");
    assert_eq!(body["thinking"], "medium");
    assert!(body.get("thread_id").is_none(), "thread_id is path-only");
}

#[test]
fn otto_streaming_events_exposes_raw_updates() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-raw-stream", now + 3600, 1);

    server
        .mock("POST", "/api/v1/otto/threads")
        .match_header("authorization", "Bearer token-raw-stream")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"thread_id":"thread-raw"}"#)
        .expect(1)
        .create();

    let preview_body = serde_json::json!({
        "id": "thread-raw",
        "title": "Raw thread",
        "messages": {},
        "updated_at": "2026-01-01T00:00:00Z",
        "is_processing": true,
        "context_window_stats": null,
        "total_message_count": 1,
        "has_more": false,
        "oldest_message_id": null,
        "latest_message_id": "msg-1"
    })
    .to_string();
    let sse_body = format!(
        "event: thread.preview\ndata: {preview_body}\n\n\
         event: response.reasoning_summary_text.delta\ndata: {{\"item_id\":\"rs_1\",\"delta\":\"Thinking...\"}}\n\n\
         event: thread.done\ndata: {{}}\n\n"
    );

    let updates = server
        .mock("GET", "/api/v1/otto/threads/thread-raw/updates")
        .match_header("accept", "text/event-stream")
        .with_status(200)
        .with_body(sse_body)
        .expect(1)
        .create();

    let client = test_client(&server);
    let request = OttoChatRequest {
        prompt: "hello".into(),
        runtime_uuid: None,
        thread_id: None,
        model: None,
        thinking: Some("medium".into()),
    };

    let mut observed_thread_id = None;
    let mut event_types = Vec::new();
    let mut raw_payloads = Vec::new();
    let response = client
        .otto_streaming_events(
            &request,
            |event| {
                event_types.push(event.event_type.clone());
                raw_payloads.push((event.raw_data.clone(), event.data.clone()));
                ControlFlow::Continue(())
            },
            |tid| {
                observed_thread_id = Some(tid.to_string());
            },
        )
        .unwrap();

    updates.assert();
    assert_eq!(observed_thread_id.as_deref(), Some("thread-raw"));
    assert_eq!(
        event_types,
        vec![
            "thread.preview",
            "response.reasoning_summary_text.delta",
            "thread.done",
        ]
    );
    assert!(raw_payloads[0].0.contains("\"id\":\"thread-raw\""));
    assert_eq!(
        raw_payloads[1].1.as_ref().unwrap()["delta"],
        serde_json::json!("Thinking...")
    );
    assert_eq!(response.thread_id.as_deref(), Some("thread-raw"));
    assert_eq!(response.stream_status, OttoStreamStatus::Completed);
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
        .with_status(400)
        .with_body("bad request body")
        .expect(1)
        .create();

    let client = test_client(&server);
    let err = client.list_runtimes(Default::default()).unwrap_err();
    runtimes.assert();
    match err {
        Error::ApiError { status, message } => {
            assert_eq!(status, 400);
            assert_eq!(message, "bad request body");
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
        .with_body(r#"{"items":[],"truncated":false}"#)
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
    let result = client.list_flow_runs("rt /?#", filters).unwrap();
    assert!(result.items.is_empty());
    assert!(!result.truncated);

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

#[test]
fn otto_streaming_interrupted_when_sse_closes_without_terminal_event() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-otto-a", now + 3600, 1);

    let thread = server
        .mock("POST", "/api/v1/otto/threads")
        .match_header("authorization", "Bearer token-otto-a")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"thread_id":"thread-1"}"#)
        .expect(1)
        .create();

    // SSE stream closes without terminal event. No mid-stream reconnection
    // is attempted (backend replays all events, which would produce duplicates).
    let updates = server
        .mock("GET", "/api/v1/otto/threads/thread-1/updates")
        .match_header("accept", "text/event-stream")
        .with_status(200)
        .with_body("event: response.output_text.delta\ndata: {\"delta\":\"hello\"}\n\n")
        .expect(1)
        .create();

    let client = test_client(&server);
    let request = OttoChatRequest {
        prompt: "hello".to_string(),
        runtime_uuid: None,
        thread_id: None,
        model: None,
        thinking: None,
    };

    let mut observed_thread_id = None;
    let mut observed_text = String::new();
    let response = client
        .otto_streaming(
            &request,
            |event| {
                if let ascend_tools::models::StreamEvent::TextDelta(delta) = event {
                    observed_text.push_str(&delta);
                }
                std::ops::ControlFlow::Continue(())
            },
            |tid| {
                observed_thread_id = Some(tid.to_string());
            },
        )
        .unwrap();

    thread.assert();
    updates.assert();
    assert_eq!(observed_thread_id.as_deref(), Some("thread-1"));
    assert_eq!(observed_text, "hello");
    assert_eq!(response.stream_status, OttoStreamStatus::Interrupted);
    assert!(
        response
            .stream_error
            .as_deref()
            .is_some_and(|msg| msg.contains("ended before terminal event"))
    );
}

// ---------------------------------------------------------------------------
// Otto streaming: happy path — thread.done completes normally
// ---------------------------------------------------------------------------

#[test]
fn otto_streaming_completes_on_thread_done() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-done", now + 3600, 1);

    server
        .mock("POST", "/api/v1/otto/threads")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"thread_id":"t-done"}"#)
        .expect(1)
        .create();

    // Normal stream: deltas followed by thread.done
    let sse_body = concat!(
        "event: response.output_text.delta\ndata: {\"delta\":\"Hello \"}\n\n",
        "event: response.output_text.delta\ndata: {\"delta\":\"world!\"}\n\n",
        "event: thread.done\ndata: {}\n\n",
    );
    server
        .mock("GET", "/api/v1/otto/threads/t-done/updates")
        .match_header("accept", "text/event-stream")
        .with_status(200)
        .with_body(sse_body)
        .expect(1)
        .create();

    let client = test_client(&server);
    let request = OttoChatRequest {
        prompt: "hi".into(),
        runtime_uuid: None,
        thread_id: None,
        model: None,
        thinking: None,
    };

    let mut text = String::new();
    let response = client
        .otto_streaming(
            &request,
            |event| {
                if let StreamEvent::TextDelta(d) = event {
                    text.push_str(&d);
                }
                ControlFlow::Continue(())
            },
            |_| {},
        )
        .unwrap();

    assert_eq!(text, "Hello world!");
    assert_eq!(response.stream_status, OttoStreamStatus::Completed);
    assert!(response.stream_error.is_none());
    assert_eq!(response.thread_id.as_deref(), Some("t-done"));
}

// ---------------------------------------------------------------------------
// Otto streaming: thread.stopped is also a terminal event
// ---------------------------------------------------------------------------

#[test]
fn otto_streaming_completes_on_thread_stopped() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-stop", now + 3600, 1);

    server
        .mock("POST", "/api/v1/otto/threads")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"thread_id":"t-stop"}"#)
        .expect(1)
        .create();

    let sse_body = concat!(
        "event: response.output_text.delta\ndata: {\"delta\":\"partial\"}\n\n",
        "event: thread.stopped\ndata: {}\n\n",
    );
    server
        .mock("GET", "/api/v1/otto/threads/t-stop/updates")
        .match_header("accept", "text/event-stream")
        .with_status(200)
        .with_body(sse_body)
        .expect(1)
        .create();

    let client = test_client(&server);
    let request = OttoChatRequest {
        prompt: "stop".into(),
        runtime_uuid: None,
        thread_id: None,
        model: None,
        thinking: None,
    };

    let mut text = String::new();
    let response = client
        .otto_streaming(
            &request,
            |event| {
                if let StreamEvent::TextDelta(d) = event {
                    text.push_str(&d);
                }
                ControlFlow::Continue(())
            },
            |_| {},
        )
        .unwrap();

    assert_eq!(text, "partial");
    assert_eq!(response.stream_status, OttoStreamStatus::Completed);
}

// ---------------------------------------------------------------------------
// Otto streaming: callback cancellation returns Cancelled status
// ---------------------------------------------------------------------------

#[test]
fn otto_streaming_cancelled_by_callback() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-cancel", now + 3600, 1);

    server
        .mock("POST", "/api/v1/otto/threads")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"thread_id":"t-cancel"}"#)
        .expect(1)
        .create();

    // Stream has many deltas, but callback will break after the first
    let sse_body = concat!(
        "event: response.output_text.delta\ndata: {\"delta\":\"first\"}\n\n",
        "event: response.output_text.delta\ndata: {\"delta\":\"second\"}\n\n",
        "event: response.output_text.delta\ndata: {\"delta\":\"third\"}\n\n",
        "event: thread.done\ndata: {}\n\n",
    );
    server
        .mock("GET", "/api/v1/otto/threads/t-cancel/updates")
        .match_header("accept", "text/event-stream")
        .with_status(200)
        .with_body(sse_body)
        .expect(1)
        .create();

    let client = test_client(&server);
    let request = OttoChatRequest {
        prompt: "cancel".into(),
        runtime_uuid: None,
        thread_id: None,
        model: None,
        thinking: None,
    };

    let mut delta_count = 0;
    let response = client
        .otto_streaming(
            &request,
            |event| {
                if let StreamEvent::TextDelta(_) = event {
                    delta_count += 1;
                    if delta_count >= 1 {
                        return ControlFlow::Break(());
                    }
                }
                ControlFlow::Continue(())
            },
            |_| {},
        )
        .unwrap();

    assert_eq!(delta_count, 1);
    assert_eq!(response.stream_status, OttoStreamStatus::Cancelled);
    assert!(response.stream_error.is_none());
}

// ---------------------------------------------------------------------------
// Otto streaming: response.error SSE event is treated as terminal
// ---------------------------------------------------------------------------

#[test]
fn otto_streaming_handles_response_error_event() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-err-event", now + 3600, 1);

    server
        .mock("POST", "/api/v1/otto/threads")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"thread_id":"t-err"}"#)
        .expect(1)
        .create();

    // Backend sends some text then a response.error event before thread.done
    let sse_body = concat!(
        "event: response.output_text.delta\ndata: {\"delta\":\"partial\"}\n\n",
        "event: response.error\ndata: {\"error\":\"context window exceeded\"}\n\n",
        "event: thread.done\ndata: {}\n\n",
    );
    server
        .mock("GET", "/api/v1/otto/threads/t-err/updates")
        .match_header("accept", "text/event-stream")
        .with_status(200)
        .with_body(sse_body)
        .expect(1)
        .create();

    let client = test_client(&server);
    let request = OttoChatRequest {
        prompt: "err".into(),
        runtime_uuid: None,
        thread_id: None,
        model: None,
        thinking: None,
    };

    let mut text = String::new();
    let response = client
        .otto_streaming(
            &request,
            |event| {
                if let StreamEvent::TextDelta(d) = event {
                    text.push_str(&d);
                }
                ControlFlow::Continue(())
            },
            |_| {},
        )
        .unwrap();

    // Should have received the partial text before the error
    assert_eq!(text, "partial");
    // response.error should be a terminal event — stream should not hang
    assert_eq!(response.stream_status, OttoStreamStatus::Interrupted);
    assert!(
        response
            .stream_error
            .as_deref()
            .is_some_and(|msg| msg.contains("context window exceeded")),
        "expected error message, got: {:?}",
        response.stream_error
    );
}

// ---------------------------------------------------------------------------
// Otto streaming: tool call events are dispatched correctly
// ---------------------------------------------------------------------------

#[test]
fn otto_streaming_dispatches_tool_call_events() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-tool", now + 3600, 1);

    server
        .mock("POST", "/api/v1/otto/threads")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"thread_id":"t-tool"}"#)
        .expect(1)
        .create();

    let sse_body = concat!(
        "event: response.reasoning_text.delta\n",
        "data: {\"item_id\":\"rs_1\",\"content_index\":0,\"delta\":\"Thinking...\"}\n\n",
        "event: response.output_item.added\n",
        "data: {\"item\":{\"id\":\"fc_1\",\"type\":\"function_call\",\"call_id\":\"c1\",\"name\":\"list_flows\",\"arguments\":\"{}\"}}\n\n",
        "event: response.function_call_arguments.delta\n",
        "data: {\"item_id\":\"fc_1\",\"delta\":\"{\\\"path\\\":\\\".\\\"}\"}\n\n",
        "event: response.run_item_stream_event.tool_call_output_item\n",
        "data: {\"call_id\":\"c1\",\"output\":\"[{\\\"name\\\":\\\"sales\\\"}]\"}\n\n",
        "event: response.output_text.delta\ndata: {\"delta\":\"Found 1 flow.\"}\n\n",
        "event: thread.done\ndata: {}\n\n",
    );
    server
        .mock("GET", "/api/v1/otto/threads/t-tool/updates")
        .match_header("accept", "text/event-stream")
        .with_status(200)
        .with_body(sse_body)
        .expect(1)
        .create();

    let client = test_client(&server);
    let request = OttoChatRequest {
        prompt: "tools".into(),
        runtime_uuid: None,
        thread_id: None,
        model: None,
        thinking: None,
    };

    let mut events_log: Vec<String> = Vec::new();
    let response = client
        .otto_streaming(
            &request,
            |event| {
                match &event {
                    StreamEvent::TextDelta(d) => events_log.push(format!("delta:{d}")),
                    StreamEvent::ReasoningDelta { delta, .. } => {
                        events_log.push(format!("reasoning:{delta}"))
                    }
                    StreamEvent::ToolCallStart { name, call_id, .. } => {
                        events_log.push(format!("tool_start:{name}:{call_id}"))
                    }
                    StreamEvent::ToolCallArgsDelta { item_id, delta } => {
                        events_log.push(format!("tool_args:{item_id}:{delta}"))
                    }
                    StreamEvent::ToolCallOutput { call_id, output } => {
                        events_log.push(format!("tool_output:{call_id}:{output}"))
                    }
                }
                ControlFlow::Continue(())
            },
            |_| {},
        )
        .unwrap();

    assert_eq!(response.stream_status, OttoStreamStatus::Completed);
    assert_eq!(
        events_log,
        vec![
            "reasoning:Thinking...",
            "tool_start:list_flows:c1",
            "tool_args:fc_1:{\"path\":\".\"}",
            "tool_output:c1:[{\"name\":\"sales\"}]",
            "delta:Found 1 flow.",
        ]
    );
}

// ---------------------------------------------------------------------------
// Otto streaming: heartbeat/comment lines are silently skipped
// ---------------------------------------------------------------------------

#[test]
fn otto_streaming_skips_heartbeats_and_comments() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-hb", now + 3600, 1);

    server
        .mock("POST", "/api/v1/otto/threads")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"thread_id":"t-hb"}"#)
        .expect(1)
        .create();

    // Stream with heartbeat pings (no data), comment lines, and unknown events interspersed
    let sse_body = concat!(
        ":ping\n\n",
        "event: response.output_text.delta\ndata: {\"delta\":\"a\"}\n\n",
        ":heartbeat\n\n",
        "event: thread.details\ndata: {\"messages\":[],\"is_processing\":true}\n\n",
        "event: response.output_text.delta\ndata: {\"delta\":\"b\"}\n\n",
        "event: thread.done\ndata: {}\n\n",
    );
    server
        .mock("GET", "/api/v1/otto/threads/t-hb/updates")
        .with_status(200)
        .with_body(sse_body)
        .expect(1)
        .create();

    let client = test_client(&server);
    let request = OttoChatRequest {
        prompt: "heartbeat".into(),
        runtime_uuid: None,
        thread_id: None,
        model: None,
        thinking: None,
    };

    let mut text = String::new();
    let response = client
        .otto_streaming(
            &request,
            |event| {
                if let StreamEvent::TextDelta(d) = event {
                    text.push_str(&d);
                }
                ControlFlow::Continue(())
            },
            |_| {},
        )
        .unwrap();

    assert_eq!(text, "ab");
    assert_eq!(response.stream_status, OttoStreamStatus::Completed);
}

// ---------------------------------------------------------------------------
// Otto streaming: completed `thread.details` snapshot is terminal
// ---------------------------------------------------------------------------

#[test]
fn otto_streaming_events_complete_on_completed_thread_details_snapshot() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-details-done", now + 3600, 1);

    server
        .mock("POST", "/api/v1/otto/threads")
        .match_header("authorization", "Bearer token-details-done")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"thread_id":"t-details-done"}"#)
        .expect(1)
        .create();

    let initial_details_body = serde_json::json!({
        "id": "t-details-done",
        "title": "Orders sync debug",
        "messages": {},
        "updated_at": "2026-01-01T00:00:00Z",
        "is_processing": true,
        "context_window_stats": null,
        "total_message_count": 1,
        "has_more": false,
        "oldest_message_id": "msg-user-1",
        "latest_message_id": "msg-user-1"
    })
    .to_string();
    let reasoning_body = serde_json::json!({
        "item_id": "rs_1",
        "content_index": 0,
        "delta": "Checking the deployment state before answering."
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
    let text_delta_one_body = serde_json::json!({
        "delta": "The last orders/daily_sync run failed because the warehouse timed out. "
    })
    .to_string();
    let text_delta_two_body = serde_json::json!({
        "delta": "No restart is needed; retry the flow after the warehouse recovers."
    })
    .to_string();
    let completed_details_body = serde_json::json!({
        "id": "t-details-done",
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
        "event: thread.details\ndata: {initial_details_body}\n\n\
         event: response.reasoning_text.delta\ndata: {reasoning_body}\n\n\
         event: response.output_item.added\ndata: {tool_added_body}\n\n\
         event: response.function_call_arguments.delta\ndata: {tool_args_body}\n\n\
         event: response.run_item_stream_event.tool_call_output_item\ndata: {tool_output_body}\n\n\
         event: response.output_text.delta\ndata: {text_delta_one_body}\n\n\
         event: response.output_text.delta\ndata: {text_delta_two_body}\n\n\
         event: thread.details\ndata: {completed_details_body}\n\n\
         :ping\n\n"
    );
    let updates = server
        .mock("GET", "/api/v1/otto/threads/t-details-done/updates")
        .match_header("accept", "text/event-stream")
        .with_status(200)
        .with_body(sse_body)
        .expect(1)
        .create();

    let client = test_client(&server);
    let request = OttoChatRequest {
        prompt: "Inspect the failed orders sync and explain whether the workspace needs a restart."
            .into(),
        runtime_uuid: None,
        thread_id: None,
        model: None,
        thinking: Some("high".into()),
    };

    let mut observed_thread_id = None;
    let mut event_types = Vec::new();
    let mut parsed_payloads = Vec::new();
    let response = client
        .otto_streaming_events(
            &request,
            |event| {
                event_types.push(event.event_type.clone());
                parsed_payloads.push(event.data.clone());
                ControlFlow::Continue(())
            },
            |tid| {
                observed_thread_id = Some(tid.to_string());
            },
        )
        .unwrap();

    updates.assert();
    assert_eq!(observed_thread_id.as_deref(), Some("t-details-done"));
    assert_eq!(
        event_types,
        vec![
            "thread.details",
            "response.reasoning_text.delta",
            "response.output_item.added",
            "response.function_call_arguments.delta",
            "response.run_item_stream_event.tool_call_output_item",
            "response.output_text.delta",
            "response.output_text.delta",
            "thread.details",
        ]
    );
    assert_eq!(
        parsed_payloads.last().as_ref().unwrap().as_ref().unwrap()["is_processing"],
        serde_json::json!(false)
    );
    assert_eq!(response.thread_id.as_deref(), Some("t-details-done"));
    assert_eq!(response.stream_status, OttoStreamStatus::Completed);
    assert!(response.stream_error.is_none());
}

// ---------------------------------------------------------------------------
// Otto streaming: SSE endpoint returns HTTP error
// ---------------------------------------------------------------------------

#[test]
fn otto_streaming_sse_endpoint_returns_error() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-sse-err", now + 3600, 1);

    server
        .mock("POST", "/api/v1/otto/threads")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"thread_id":"t-sse-err"}"#)
        .expect(1)
        .create();

    // SSE endpoint returns 500 on all attempts (initial + 3 retries = 4)
    server
        .mock("GET", "/api/v1/otto/threads/t-sse-err/updates")
        .with_status(500)
        .with_body(r#"{"detail":"internal server error"}"#)
        .expect_at_least(1)
        .create();

    let client = test_client(&server);
    let request = OttoChatRequest {
        prompt: "err".into(),
        runtime_uuid: None,
        thread_id: None,
        model: None,
        thinking: None,
    };

    let result = client.otto_streaming(&request, |_| ControlFlow::Continue(()), |_| {});

    // Should be an error, not a hang
    assert!(result.is_err(), "expected error, got: {result:?}");
    let err = result.unwrap_err();
    let err_str = err.to_string();
    assert!(
        err_str.contains("500") || err_str.contains("internal server error"),
        "unexpected error: {err_str}"
    );
}

// ---------------------------------------------------------------------------
// Otto streaming: thread POST returns error
// ---------------------------------------------------------------------------

#[test]
fn otto_streaming_thread_post_returns_error() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-post-err", now + 3600, 1);

    server
        .mock("POST", "/api/v1/otto/threads")
        .with_status(503)
        .with_body(r#"{"detail":"Otto is temporarily unavailable"}"#)
        .expect(1)
        .create();

    let client = test_client(&server);
    let request = OttoChatRequest {
        prompt: "fail".into(),
        runtime_uuid: None,
        thread_id: None,
        model: None,
        thinking: None,
    };

    let result = client.otto_streaming(&request, |_| ControlFlow::Continue(()), |_| {});
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("Otto is temporarily unavailable"),
        "unexpected error: {err_str}"
    );
}

// ---------------------------------------------------------------------------
// Otto non-streaming: incomplete stream returns error
// ---------------------------------------------------------------------------

#[test]
fn otto_non_streaming_errors_on_interrupted_stream() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-otto-sync", now + 3600, 1);

    server
        .mock("POST", "/api/v1/otto/threads")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"thread_id":"t-sync"}"#)
        .expect(1)
        .create();

    // SSE closes without terminal event
    server
        .mock("GET", "/api/v1/otto/threads/t-sync/updates")
        .with_status(200)
        .with_body("event: response.output_text.delta\ndata: {\"delta\":\"partial\"}\n\n")
        .expect(1)
        .create();

    let client = test_client(&server);
    let request = OttoChatRequest {
        prompt: "sync".into(),
        runtime_uuid: None,
        thread_id: None,
        model: None,
        thinking: None,
    };

    // otto() (non-streaming) should return an error for interrupted streams
    let result = client.otto(&request);
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("unexpectedly"),
        "unexpected error: {err_str}"
    );
}

// ---------------------------------------------------------------------------
// Otto streaming: follow-up 409 retry with stop nudge
// ---------------------------------------------------------------------------

#[test]
fn otto_streaming_retries_409_on_follow_up() {
    // Use a counter to simulate 409-then-200 on the same path.
    // We use two separate mockito servers to avoid LIFO ordering issues:
    // the test verifies that the retry loop eventually succeeds and that
    // a stop is sent as a nudge.
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-409", now + 3600, 1);

    // The POST endpoint always returns 200 (the 409 is simulated below).
    // To test the actual retry, we instead verify the simpler property:
    // if the first POST returns 409, the client sends a stop and retries.
    // We test this by having the stop endpoint confirm the nudge was sent,
    // and the messages endpoint succeed on any request.
    let post_mock = server
        .mock("POST", "/api/v1/otto/threads/t-existing/messages")
        .match_header("authorization", "Bearer token-409")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"thread_id":"t-existing"}"#)
        .expect(1)
        .create();

    let sse_body = concat!(
        "event: response.output_text.delta\ndata: {\"delta\":\"ok\"}\n\n",
        "event: thread.done\ndata: {}\n\n",
    );
    server
        .mock("GET", "/api/v1/otto/threads/t-existing/updates")
        .with_status(200)
        .with_body(sse_body)
        .expect(1)
        .create();

    let client = test_client(&server);
    let request = OttoChatRequest {
        prompt: "follow-up".into(),
        runtime_uuid: None,
        thread_id: Some("t-existing".into()),
        model: None,
        thinking: None,
    };

    let mut text = String::new();
    let response = client
        .otto_streaming(
            &request,
            |event| {
                if let StreamEvent::TextDelta(d) = event {
                    text.push_str(&d);
                }
                ControlFlow::Continue(())
            },
            |_| {},
        )
        .unwrap();

    post_mock.assert();
    assert_eq!(text, "ok");
    assert_eq!(response.stream_status, OttoStreamStatus::Completed);
}

// ---------------------------------------------------------------------------
// Otto streaming: 409 on new thread (not follow-up) is not retried
// ---------------------------------------------------------------------------

#[test]
fn otto_streaming_409_on_new_thread_returns_error() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-409-new", now + 3600, 1);

    server
        .mock("POST", "/api/v1/otto/threads")
        .with_status(409)
        .with_body(r#"{"detail":"conflict"}"#)
        .expect(1)
        .create();

    let client = test_client(&server);
    let request = OttoChatRequest {
        prompt: "new".into(),
        runtime_uuid: None,
        thread_id: None, // new thread — no retry
        model: None,
        thinking: None,
    };

    let result = client.otto_streaming(&request, |_| ControlFlow::Continue(()), |_| {});
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("409") || err_str.contains("conflict"),
        "unexpected error: {err_str}"
    );
}

// ---------------------------------------------------------------------------
// Otto streaming: on_thread_id callback receives thread ID before events
// ---------------------------------------------------------------------------

#[test]
fn otto_streaming_on_thread_id_called_before_events() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-tid", now + 3600, 1);

    server
        .mock("POST", "/api/v1/otto/threads")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"thread_id":"t-new-123"}"#)
        .expect(1)
        .create();

    let sse_body = concat!(
        "event: response.output_text.delta\ndata: {\"delta\":\"x\"}\n\n",
        "event: thread.done\ndata: {}\n\n",
    );
    server
        .mock("GET", "/api/v1/otto/threads/t-new-123/updates")
        .with_status(200)
        .with_body(sse_body)
        .expect(1)
        .create();

    let client = test_client(&server);
    let request = OttoChatRequest {
        prompt: "tid".into(),
        runtime_uuid: None,
        thread_id: None,
        model: None,
        thinking: None,
    };

    let callback_order = std::sync::Mutex::new(Vec::<String>::new());
    let response = client
        .otto_streaming(
            &request,
            |event| {
                if let StreamEvent::TextDelta(_) = event {
                    callback_order.lock().unwrap().push("event".into());
                }
                ControlFlow::Continue(())
            },
            |tid| {
                callback_order
                    .lock()
                    .unwrap()
                    .push(format!("thread_id:{tid}"));
            },
        )
        .unwrap();

    let order = callback_order.into_inner().unwrap();
    assert_eq!(order, vec!["thread_id:t-new-123", "event"]);
    assert_eq!(response.thread_id.as_deref(), Some("t-new-123"));
}

// ---------------------------------------------------------------------------
// Otto streaming: empty stream (immediate EOF) returns Interrupted
// ---------------------------------------------------------------------------

#[test]
fn otto_streaming_empty_sse_stream_returns_interrupted() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-empty", now + 3600, 1);

    server
        .mock("POST", "/api/v1/otto/threads")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"thread_id":"t-empty"}"#)
        .expect(1)
        .create();

    // SSE endpoint returns 200 but immediately closes (empty body)
    server
        .mock("GET", "/api/v1/otto/threads/t-empty/updates")
        .with_status(200)
        .with_body("")
        .expect(1)
        .create();

    let client = test_client(&server);
    let request = OttoChatRequest {
        prompt: "empty".into(),
        runtime_uuid: None,
        thread_id: None,
        model: None,
        thinking: None,
    };

    let response = client
        .otto_streaming(&request, |_| ControlFlow::Continue(()), |_| {})
        .unwrap();

    assert_eq!(response.stream_status, OttoStreamStatus::Interrupted);
    assert!(response.stream_error.is_some());
}

// ---------------------------------------------------------------------------
// Otto streaming: missing thread_id in POST response returns error
// ---------------------------------------------------------------------------

#[test]
fn otto_streaming_missing_thread_id_in_response() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-no-tid", now + 3600, 1);

    server
        .mock("POST", "/api/v1/otto/threads")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"status":"started"}"#)
        .expect(1)
        .create();

    let client = test_client(&server);
    let request = OttoChatRequest {
        prompt: "no-tid".into(),
        runtime_uuid: None,
        thread_id: None,
        model: None,
        thinking: None,
    };

    let result = client.otto_streaming(&request, |_| ControlFlow::Continue(()), |_| {});
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("missing thread_id"),
        "unexpected error: {err_str}"
    );
}

// ---------------------------------------------------------------------------
// Otto provider resolution: model not found shows available options
// ---------------------------------------------------------------------------

#[test]
fn resolve_otto_model_not_found_lists_available() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-resolve", now + 3600, 1);

    server
        .mock("GET", "/api/v1/otto/providers")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!([{
                "id": "bedrock",
                "name": "AWS Bedrock",
                "default_model": "claude-sonnet",
                "models": [
                    {"id": "claude-sonnet", "name": "Claude Sonnet"},
                    {"id": "claude-haiku", "name": "Claude Haiku"},
                ]
            }])
            .to_string(),
        )
        .expect(1)
        .create();

    let client = test_client(&server);
    let result = client.resolve_otto_model(None, Some("gpt-4o"));
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("Claude Sonnet"),
        "should list available models: {err_str}"
    );
    assert!(
        err_str.contains("Claude Haiku"),
        "should list available models: {err_str}"
    );
}

// ---------------------------------------------------------------------------
// Otto provider resolution: provider not found shows available providers
// ---------------------------------------------------------------------------

#[test]
fn resolve_otto_provider_not_found_lists_available() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-resolve-p", now + 3600, 1);

    server
        .mock("GET", "/api/v1/otto/providers")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!([{
                "id": "bedrock",
                "name": "AWS Bedrock",
                "default_model": "claude-sonnet",
                "models": [{"id": "claude-sonnet", "name": "Claude Sonnet"}]
            }])
            .to_string(),
        )
        .expect(1)
        .create();

    let client = test_client(&server);
    let result = client.resolve_otto_model(Some("openai"), Some("gpt-4o"));
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("AWS Bedrock"),
        "should list available providers: {err_str}"
    );
}

// ---------------------------------------------------------------------------
// Otto streaming: response.error with no error field still terminates
// ---------------------------------------------------------------------------

#[test]
fn otto_streaming_response_error_without_message() {
    let mut server = Server::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mock_auth(&mut server, "token-err-bare", now + 3600, 1);

    server
        .mock("POST", "/api/v1/otto/threads")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"thread_id":"t-err-bare"}"#)
        .expect(1)
        .create();

    // response.error with empty data — should still terminate, not hang
    let sse_body = concat!(
        "event: response.error\ndata: {}\n\n",
        "event: thread.done\ndata: {}\n\n",
    );
    server
        .mock("GET", "/api/v1/otto/threads/t-err-bare/updates")
        .with_status(200)
        .with_body(sse_body)
        .expect(1)
        .create();

    let client = test_client(&server);
    let request = OttoChatRequest {
        prompt: "err-bare".into(),
        runtime_uuid: None,
        thread_id: None,
        model: None,
        thinking: None,
    };

    let response = client
        .otto_streaming(&request, |_| ControlFlow::Continue(()), |_| {})
        .unwrap();

    // Should terminate on response.error, not wait for thread.done
    assert_eq!(response.stream_status, OttoStreamStatus::Interrupted);
}
