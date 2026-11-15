//! REST interface: the handlers of src/service.rs over HTTP, with an
//! OpenAPI 3.1 document generated from the same Rust types as the MCP tool
//! schemas.

use crate::service::{self, App, ToolError};
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::sync::Arc;

fn status(e: &ToolError) -> StatusCode {
    match e {
        ToolError::ValidationFailed { .. } => StatusCode::UNPROCESSABLE_ENTITY,
        ToolError::NotFound { .. } => StatusCode::NOT_FOUND,
        ToolError::NotConverged { .. } => StatusCode::CONFLICT,
        ToolError::KiteError { .. } => StatusCode::UNPROCESSABLE_ENTITY,
        ToolError::WorkerFailed { .. } => StatusCode::SERVICE_UNAVAILABLE,
    }
}

fn reply<T: Serialize>(r: Result<T, ToolError>) -> Response {
    match r {
        Ok(v) => Json(v).into_response(),
        Err(e) => (status(&e), Json(e)).into_response(),
    }
}

/// Parse the body as `T`; a body that does not match is a 422 naming the problem.
fn body<T: serde::de::DeserializeOwned>(v: Value) -> Result<T, Box<Response>> {
    serde_json::from_value(v).map_err(|e| {
        let err = ToolError::ValidationFailed {
            errors: vec![crate::scenario::FieldError {
                field: "body".into(),
                value: "see message".into(),
                allowed: "the request schema in /openapi.json".into(),
                message: e.to_string(),
            }],
        };
        Box::new((StatusCode::UNPROCESSABLE_ENTITY, Json(err)).into_response())
    })
}

type St = State<Arc<App>>;

async fn models() -> Response {
    Json(service::list_models()).into_response()
}

async fn baselines(State(app): St) -> Response {
    reply(service::describe_baseline(&app, service::DescribeBaselineInput { baseline: None }).await)
}

async fn baseline(State(app): St, Path(id): Path<String>) -> Response {
    reply(
        service::describe_baseline(&app, service::DescribeBaselineInput { baseline: Some(id) })
            .await,
    )
}

async fn generate(State(app): St, Json(v): Json<Value>) -> Response {
    match body(v) {
        Ok(i) => reply(service::generate_baseline(&app, i).await),
        Err(r) => *r,
    }
}

async fn run(State(app): St, Json(v): Json<Value>) -> Response {
    match body(v) {
        Ok(i) => reply(service::run_scenario(&app, i).await),
        Err(r) => *r,
    }
}

async fn table(State(app): St, Json(v): Json<Value>) -> Response {
    match body(v) {
        Ok(i) => reply(service::get_table(&app, i).await),
        Err(r) => *r,
    }
}

async fn compare(State(app): St, Json(v): Json<Value>) -> Response {
    match body(v) {
        Ok(i) => reply(service::compare_runs(&app, i).await),
        Err(r) => *r,
    }
}

async fn reproduce(State(app): St, Path(id): Path<String>) -> Response {
    match app.run(&id).await {
        Ok(r) => (
            [(header::CONTENT_TYPE, "text/x-r; charset=utf-8")],
            r.reproduce_script.clone(),
        )
            .into_response(),
        Err(e) => (status(&e), Json(e)).into_response(),
    }
}

async fn provenance(State(app): St, Path(id): Path<String>) -> Response {
    match app.run(&id).await {
        Ok(r) => Json(r.provenance.clone()).into_response(),
        Err(e) => (status(&e), Json(e)).into_response(),
    }
}

async fn openapi_doc() -> Response {
    Json(openapi()).into_response()
}

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/openapi.json", get(openapi_doc))
        .route("/v1/models", get(models))
        .route("/v1/baselines", get(baselines))
        .route("/v1/baselines/{id}", get(baseline))
        .route("/v1/baselines/generate", post(generate))
        .route("/v1/runs", post(run))
        .route("/v1/runs/table", post(table))
        .route("/v1/runs/compare", post(compare))
        .route("/v1/runs/{id}/reproduce.R", get(reproduce))
        .route("/v1/runs/{id}/provenance", get(provenance))
        .with_state(app)
}

// ---- OpenAPI -------------------------------------------------------------------

/// Schema of `T`, with its `$defs` moved into `defs` and references
/// rewritten to point at `#/components/schemas`.
fn component<T: JsonSchema>(defs: &mut Map<String, Value>) -> Value {
    let mut v = serde_json::to_value(schemars::schema_for!(T)).expect("schema");
    if let Some(Value::Object(d)) = v.as_object_mut().and_then(|o| o.remove("$defs")) {
        for (k, s) in d {
            defs.insert(k, s);
        }
    }
    if let Some(o) = v.as_object_mut() {
        o.remove("$schema");
    }
    v
}

fn rewrite_refs(v: &mut Value) {
    match v {
        Value::Object(m) => {
            if let Some(Value::String(r)) = m.get_mut("$ref")
                && let Some(name) = r.strip_prefix("#/$defs/")
            {
                *r = format!("#/components/schemas/{name}");
            }
            for (_, x) in m.iter_mut() {
                rewrite_refs(x);
            }
        }
        Value::Array(a) => a.iter_mut().for_each(rewrite_refs),
        _ => {}
    }
}

fn op(summary: &str, request: Option<&str>, response: &str, errors: &[(&str, &str)]) -> Value {
    let mut responses = Map::new();
    responses.insert(
        "200".into(),
        json!({"description": "success", "content": {"application/json": {"schema": {"$ref": format!("#/components/schemas/{response}")}}}}),
    );
    for (code, why) in errors {
        responses.insert(
            code.to_string(),
            json!({"description": why, "content": {"application/json": {"schema": {"$ref": "#/components/schemas/ToolError"}}}}),
        );
    }
    let mut o = json!({"summary": summary, "responses": responses});
    if let Some(req) = request {
        o["requestBody"] = json!({"required": true, "content": {"application/json": {"schema": {"$ref": format!("#/components/schemas/{req}")}}}});
    }
    o
}

fn path_param(name: &str) -> Value {
    json!([{"name": name, "in": "path", "required": true, "schema": {"type": "string"}}])
}

/// The OpenAPI 3.1 document for the REST interface.
pub fn openapi() -> Value {
    let mut defs = Map::new();
    let mut schemas = Map::new();
    schemas.insert(
        "ListModelsOutput".into(),
        component::<service::ListModelsOutput>(&mut defs),
    );
    schemas.insert(
        "DescribeBaselineOutput".into(),
        component::<service::DescribeBaselineOutput>(&mut defs),
    );
    schemas.insert(
        "BaselineDescription".into(),
        component::<service::BaselineDescription>(&mut defs),
    );
    schemas.insert(
        "GenerateBaselineInput".into(),
        component::<service::GenerateBaselineInput>(&mut defs),
    );
    schemas.insert(
        "RunScenarioInput".into(),
        component::<service::RunScenarioInput>(&mut defs),
    );
    schemas.insert(
        "RunScenarioOutput".into(),
        component::<service::RunScenarioOutput>(&mut defs),
    );
    schemas.insert(
        "GetTableInput".into(),
        component::<service::GetTableInput>(&mut defs),
    );
    schemas.insert(
        "GetTableOutput".into(),
        component::<service::GetTableOutput>(&mut defs),
    );
    schemas.insert(
        "CompareRunsInput".into(),
        component::<service::CompareRunsInput>(&mut defs),
    );
    schemas.insert(
        "CompareRunsOutput".into(),
        component::<service::CompareRunsOutput>(&mut defs),
    );
    schemas.insert(
        "Provenance".into(),
        component::<service::Provenance>(&mut defs),
    );
    schemas.insert("ToolError".into(), component::<ToolError>(&mut defs));
    for (k, v) in defs {
        schemas.entry(k).or_insert(v);
    }
    let not_found = ("404", "no such baseline or run");
    let invalid = ("422", "the input failed validation, or KITE rejected it");
    let worker = ("503", "the worker running KITE failed");
    let mut doc = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "kite-mcp",
            "version": service::KITE_MCP_VERSION,
            "description": "An independent, unofficial interface to KITE, the trade policy model suite of the Kiel Institute for the World Economy, with no affiliation to or endorsement from the Institute. The same handlers serve the MCP tools. Numbers are exactly what KITE computes, except fields whose names end in _derived.",
            "license": {"name": "GPL-3.0-only"}
        },
        "paths": {
            "/v1/models": {"get": op("The three KITE models and the scenario variables each reads", None, "ListModelsOutput", &[])},
            "/v1/baselines": {"get": op("Every baseline", None, "DescribeBaselineOutput", &[worker])},
            "/v1/baselines/{id}": {"get": op("One baseline", None, "BaselineDescription", &[not_found, worker]), "parameters": path_param("id")},
            "/v1/baselines/generate": {"post": op("Build a synthetic baseline", Some("GenerateBaselineInput"), "BaselineDescription", &[invalid, worker])},
            "/v1/runs": {"post": op("Run a scenario", Some("RunScenarioInput"), "RunScenarioOutput", &[not_found, invalid, worker])},
            "/v1/runs/table": {"post": op("One result table of a run", Some("GetTableInput"), "GetTableOutput", &[not_found, invalid])},
            "/v1/runs/compare": {"post": op("Compare one table between two converged runs", Some("CompareRunsInput"), "CompareRunsOutput", &[not_found, invalid, ("409", "a run did not converge")])},
            "/v1/runs/{id}/provenance": {"get": op("Provenance of a run", None, "Provenance", &[not_found]), "parameters": path_param("id")},
            "/v1/runs/{id}/reproduce.R": {
                "get": {"summary": "R script that reproduces a run with KITE alone",
                        "responses": {"200": {"description": "the script", "content": {"text/x-r": {"schema": {"type": "string"}}}},
                                      "404": {"description": "no such run", "content": {"application/json": {"schema": {"$ref": "#/components/schemas/ToolError"}}}}}},
                "parameters": path_param("id")
            }
        },
        "components": {"schemas": schemas}
    });
    rewrite_refs(&mut doc);
    doc
}

pub async fn serve(app: Arc<App>, addr: std::net::SocketAddr) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    eprintln!("kite-mcp REST interface on http://{addr} (OpenAPI document at /openapi.json)");
    axum::serve(listener, router(app)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refs(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::Object(m) => {
                if let Some(Value::String(r)) = m.get("$ref") {
                    out.push(r.clone());
                }
                m.values().for_each(|x| refs(x, out));
            }
            Value::Array(a) => a.iter().for_each(|x| refs(x, out)),
            _ => {}
        }
    }

    #[test]
    fn every_reference_in_the_openapi_document_resolves() {
        let doc = openapi();
        let mut all = Vec::new();
        refs(&doc, &mut all);
        assert!(!all.is_empty());
        for r in all {
            let name = r
                .strip_prefix("#/components/schemas/")
                .unwrap_or_else(|| panic!("{r}"));
            assert!(
                doc["components"]["schemas"].get(name).is_some(),
                "{r} does not resolve"
            );
        }
        assert_eq!(doc["openapi"], "3.1.0");
        assert!(
            doc["components"]["schemas"]["RunScenarioInput"]["properties"]["scenario"].is_object()
        );
    }
}
