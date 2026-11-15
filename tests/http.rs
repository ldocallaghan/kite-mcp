//! One test per REST endpoint. Tests marked `needs R` run when
//! KITE_MCP_CONFORMANCE=1, as the conformance tests do.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use kite_mcp::http::router;
use kite_mcp::service::App;
use kite_mcp::worker::{Worker, WorkerConfig};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;

fn with_r() -> bool {
    std::env::var("KITE_MCP_CONFORMANCE").as_deref() == Ok("1")
}

fn app() -> Arc<App> {
    Arc::new(App::new(Worker::new(WorkerConfig::r_from_env(
        Duration::from_secs(120),
    ))))
}

async fn send(
    app: &Arc<App>,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, String) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    let req = match body {
        Some(b) => req.body(Body::from(b.to_string())).unwrap(),
        None => req.body(Body::empty()).unwrap(),
    };
    let res = router(app.clone()).oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

fn json_of(s: &str) -> Value {
    serde_json::from_str(s).unwrap()
}

fn tariff_war() -> Value {
    json!({"model": "caliendo_parro_2015", "scenario": {"tariff_new": [
        {"origin": "c1", "destination": "c2", "value": 1.2},
        {"origin": "c2", "destination": "c1", "value": 1.2}]}})
}

#[tokio::test]
async fn get_openapi_json() {
    let (s, b) = send(&app(), "GET", "/openapi.json", None).await;
    assert_eq!(s, StatusCode::OK);
    let doc = json_of(&b);
    assert_eq!(doc["openapi"], "3.1.0");
    for p in [
        "/v1/models",
        "/v1/baselines",
        "/v1/baselines/{id}",
        "/v1/baselines/generate",
        "/v1/runs",
        "/v1/runs/table",
        "/v1/runs/compare",
        "/v1/runs/{id}/reproduce.R",
        "/v1/runs/{id}/provenance",
    ] {
        assert!(doc["paths"].get(p).is_some(), "{p}");
    }
}

#[tokio::test]
async fn get_models() {
    let (s, b) = send(&app(), "GET", "/v1/models", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(json_of(&b)["models"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn get_baselines() {
    if !with_r() {
        return;
    }
    let (s, b) = send(&app(), "GET", "/v1/baselines", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(json_of(&b)["baselines"][0]["id"], "synthetic_example");
}

#[tokio::test]
async fn get_baseline_by_id() {
    let a = app();
    let (s, b) = send(&a, "GET", "/v1/baselines/nonesuch", None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(json_of(&b)["error"], "not_found");
    if !with_r() {
        return;
    }
    let (s, b) = send(&a, "GET", "/v1/baselines/synthetic_example", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(json_of(&b)["countries"], json!(["c1", "c2", "c3"]));
}

#[tokio::test]
async fn post_baselines_generate() {
    let a = app();
    let (s, b) = send(
        &a,
        "POST",
        "/v1/baselines/generate",
        Some(json!({"countries": 99, "sectors": 2, "seed": 1})),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json_of(&b)["errors"][0]["field"], "countries");
    if !with_r() {
        return;
    }
    let (s, b) = send(
        &a,
        "POST",
        "/v1/baselines/generate",
        Some(json!({"countries": 4, "sectors": 2, "seed": 3})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(json_of(&b)["id"], "synthetic_c4_s2_seed3");
}

#[tokio::test]
async fn post_runs() {
    let a = app();
    let (s, b) = send(
        &a,
        "POST",
        "/v1/runs",
        Some(json!({"model": "caliendo_parro_2015", "scenario": {"tarif_new": []}})),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{b}");
    if !with_r() {
        return;
    }
    let (s, b) = send(&a, "POST", "/v1/runs", Some(tariff_war())).await;
    assert_eq!(s, StatusCode::OK);
    let out = json_of(&b);
    assert_eq!(out["status"], "converged");
    assert_eq!(out["info"]["iterations"], 22);
}

#[tokio::test]
async fn post_runs_table() {
    if !with_r() {
        return;
    }
    let a = app();
    let (_, b) = send(&a, "POST", "/v1/runs", Some(tariff_war())).await;
    let id = json_of(&b)["run_id"].clone();
    let (s, b) = send(
        &a,
        "POST",
        "/v1/runs/table",
        Some(json!({"run_id": id, "table": "welfare_change", "filters": {"country": ["c3"]}})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let t = json_of(&b);
    assert_eq!(t["returned_rows"], 1);
    assert_eq!(t["rows"][0]["country"], "c3");
    let (s, _) = send(
        &a,
        "POST",
        "/v1/runs/table",
        Some(json!({"run_id": id, "table": "nonesuch"})),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn post_runs_compare() {
    if !with_r() {
        return;
    }
    let a = app();
    let (_, war) = send(&a, "POST", "/v1/runs", Some(tariff_war())).await;
    let (_, empty) = send(
        &a,
        "POST",
        "/v1/runs",
        Some(json!({"model": "caliendo_parro_2015"})),
    )
    .await;
    let (s, b) = send(&a, "POST", "/v1/runs/compare", Some(json!({
        "run_a": json_of(&empty)["run_id"], "run_b": json_of(&war)["run_id"], "table": "welfare_change"}))).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(json_of(&b)["matching_rows"], 3);
    let mut nc = tariff_war();
    nc["settings"] = json!({"trade_balance_rule": "fixed_country_share"});
    let (_, n) = send(&a, "POST", "/v1/runs", Some(nc)).await;
    let (s, b) = send(&a, "POST", "/v1/runs/compare", Some(json!({
        "run_a": json_of(&war)["run_id"], "run_b": json_of(&n)["run_id"], "table": "welfare_change"}))).await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(json_of(&b)["error"], "not_converged");
}

#[tokio::test]
async fn get_run_reproduce_script() {
    let a = app();
    let (s, _) = send(&a, "GET", "/v1/runs/rnonesuch/reproduce.R", None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    if !with_r() {
        return;
    }
    let (_, b) = send(&a, "POST", "/v1/runs", Some(tariff_war())).await;
    let id = json_of(&b)["run_id"].as_str().unwrap().to_string();
    let (s, script) = send(&a, "GET", &format!("/v1/runs/{id}/reproduce.R"), None).await;
    assert_eq!(s, StatusCode::OK);
    assert!(script.contains("update_equilibrium(model = caliendo_parro_2015"));
}

#[tokio::test]
async fn get_run_provenance() {
    let a = app();
    let (s, _) = send(&a, "GET", "/v1/runs/rnonesuch/provenance", None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    if !with_r() {
        return;
    }
    let (_, b) = send(&a, "POST", "/v1/runs", Some(tariff_war())).await;
    let id = json_of(&b)["run_id"].as_str().unwrap().to_string();
    let (s, p) = send(&a, "GET", &format!("/v1/runs/{id}/provenance"), None).await;
    assert_eq!(s, StatusCode::OK);
    let p = json_of(&p);
    assert_eq!(
        p["kite"]["commit"],
        "5995dd1e67803d0a499b2badbd8c3d859e08fdc5"
    );
    assert_eq!(p["baseline"]["id"], "synthetic_example");
}
