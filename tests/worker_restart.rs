//! The worker is restarted after a timeout or a crash. These tests use small
//! shell scripts in place of R, so they need no R installation.

use kite_mcp::worker::{Worker, WorkerConfig, WorkerFailure};
use std::time::Duration;

fn shell(script: &str, timeout_ms: u64) -> Worker {
    Worker::new(WorkerConfig {
        program: "sh".into(),
        args: vec!["-c".into(), script.into()],
        request_timeout: Duration::from_millis(timeout_ms),
        startup_timeout: Duration::from_millis(timeout_ms),
    })
}

/// Answers each request with ok:true and the request id, read with sed.
const ECHO: &str = r#"while IFS= read -r line; do id=$(printf '%s' "$line" | sed 's/.*"id":"\([^"]*\)".*/\1/'); printf '{"id":"%s","ok":true,"result":{"n":1}}\n' "$id"; done"#;

#[tokio::test]
async fn a_well_behaved_worker_answers() {
    let w = shell(ECHO, 5000);
    let r = w.call("ping", serde_json::Value::Null).await.unwrap();
    assert_eq!(r["n"], 1);
    let r = w.call("ping", serde_json::Value::Null).await.unwrap();
    assert_eq!(r["n"], 1);
    assert_eq!(w.restarts(), 0);
}

#[tokio::test]
async fn a_worker_that_exits_is_reported_and_restarted() {
    let w = shell("read line; exit 3", 5000);
    let e = w.call("ping", serde_json::Value::Null).await.unwrap_err();
    assert!(matches!(e, WorkerFailure::Crashed(_)), "{e:?}");
    let e = w.call("ping", serde_json::Value::Null).await.unwrap_err();
    assert!(matches!(e, WorkerFailure::Crashed(_)), "{e:?}");
    assert_eq!(w.restarts(), 1);
}

#[tokio::test]
async fn a_worker_that_hangs_times_out_and_is_restarted() {
    let w = shell("read line; sleep 30", 300);
    let started = std::time::Instant::now();
    let e = w.call("solve", serde_json::Value::Null).await.unwrap_err();
    assert!(matches!(e, WorkerFailure::Timeout(_)), "{e:?}");
    assert!(started.elapsed() < Duration::from_secs(5));
    let _ = w.call("solve", serde_json::Value::Null).await.unwrap_err();
    assert_eq!(w.restarts(), 1);
}

#[tokio::test]
async fn a_malformed_reply_is_a_protocol_failure() {
    let w = shell("read line; echo 'not json'; sleep 5", 5000);
    let e = w.call("ping", serde_json::Value::Null).await.unwrap_err();
    assert!(matches!(e, WorkerFailure::Protocol(_)), "{e:?}");
}

#[tokio::test]
async fn a_reply_to_another_request_is_refused() {
    let w = shell(
        r#"read line; echo '{"id":"other","ok":true,"result":{}}'; sleep 5"#,
        5000,
    );
    let e = w.call("ping", serde_json::Value::Null).await.unwrap_err();
    assert!(matches!(e, WorkerFailure::Protocol(_)), "{e:?}");
}

#[tokio::test]
async fn a_missing_program_is_a_spawn_failure() {
    let w = Worker::new(WorkerConfig {
        program: "/nonexistent/Rscript".into(),
        args: vec![],
        request_timeout: Duration::from_secs(1),
        startup_timeout: Duration::from_secs(1),
    });
    let e = w.call("hello", serde_json::Value::Null).await.unwrap_err();
    assert!(matches!(e, WorkerFailure::Spawn(_)), "{e:?}");
}
