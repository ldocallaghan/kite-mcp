//! MCP server: tools, resources and the prompt, over the handlers in
//! src/service.rs.

use crate::catalogue;
use crate::service::{self, App, ToolError};
use rmcp::model::*;
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};
use schemars::JsonSchema;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;

pub const INSTRUCTIONS: &str = "kite-mcp runs trade policy scenarios in KITE, the Kiel Institute's open-source trade model suite, through an independent and unofficial interface. Every number in a tool result is exactly what KITE computes, except fields whose names end in _derived. Quote numbers only as they appear in tool results and say which tool and table each comes from; do not compute sums, shares or ratios yourself. If run_scenario returns status not_converged there is no result: say so and give no figures. Report every warning. The baselines are synthetic: they describe no real economy, so do not present any figure as a real-world estimate. Each run comes with an R script that reproduces it without this server; give the user its resource URI.";

#[derive(Clone)]
pub struct KiteMcp {
    pub app: Arc<App>,
}

struct ToolDef {
    name: &'static str,
    title: &'static str,
    description: &'static str,
    schema: fn() -> serde_json::Value,
}

fn schema<T: JsonSchema>() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(T)).expect("schema")
}

const TOOLS: &[ToolDef] = &[
    ToolDef {
        name: "list_models",
        title: "List KITE models",
        description: "List KITE's three models: reference and citation, what each adds, and the scenario variables each reads, with meaning, units, defaults and the KITE source lines they come from. Call this before run_scenario to choose variables.",
        schema: schema::<service::ListModelsInput>,
    },
    ToolDef {
        name: "describe_baseline",
        title: "Describe a baseline",
        description: "Countries, sectors, tables and hash of a baseline. Omit `baseline` to list every baseline. All baselines built by this server are synthetic: random draws that describe no real country or economy, with no real-world units.",
        schema: schema::<service::DescribeBaselineInput>,
    },
    ToolDef {
        name: "generate_baseline",
        title: "Generate a synthetic baseline",
        description: "Build a synthetic economy of a number of countries (2 to 30) and sectors (1 to 10) from a seed, with the construction of KITE's example (inst/examples/example.R). Returns the baseline id to pass to run_scenario. The data are random and synthetic.",
        schema: schema::<service::GenerateBaselineInput>,
    },
    ToolDef {
        name: "run_scenario",
        title: "Run a KITE scenario",
        description: "Run one scenario in KITE: update_equilibrium() then process_results(). Returns status (converged or not_converged), KITE's info block, every warning with its R class, the welfare change by country (only when converged), provenance and an R script that reproduces the run without this server. Numbers are exactly KITE's; fields ending in _derived are computed by kite-mcp. welfare_change is a ratio of counterfactual to baseline: below 1 is a fall, 1 means no change. If status is not_converged there is no result: report that, quote the reason, and give no welfare or other figures. Scenario variables use KITE's names; tariff_new is a gross factor (1.2 = 20 % tariff) set by the destination on imports from the origin, and cells not listed keep their baseline value. A variable the chosen model does not read is refused with a message saying which models read it. Quote only numbers that tool results contain; do not compute totals, shares or ratios yourself.",
        schema: schema::<service::RunScenarioInput>,
    },
    ToolDef {
        name: "get_table",
        title: "Get a result table",
        description: "One process_results() table of a run, filtered by country, sector, origin or destination, with its unit and meaning. Tables named *_change are ratios of counterfactual to baseline (1 = no change). If the run did not converge, every row is flagged unconverged and is not a result. Check truncated and raise limit if needed. Quote values as returned; do not sum or divide rows yourself.",
        schema: schema::<service::GetTableInput>,
    },
    ToolDef {
        name: "compare_runs",
        title: "Compare two runs",
        description: "Compare one table between two converged runs on matching rows: value_a and value_b exactly as KITE computed them, and difference_derived = value_b - value_a computed by kite-mcp. Use it, for example, for a tariff war with and without a carbon tax. Refuses runs that did not converge.",
        schema: schema::<service::CompareRunsInput>,
    },
];

fn tool_list() -> Vec<Tool> {
    TOOLS
        .iter()
        .map(|t| {
            let schema = match (t.schema)() {
                serde_json::Value::Object(m) => m,
                _ => unreachable!(),
            };
            let mut tool = Tool::new(t.name, t.description, Arc::new(schema));
            tool.title = Some(t.title.to_string());
            tool
        })
        .collect()
}

fn ok_json<T: Serialize>(v: &T) -> CallToolResult {
    CallToolResult::structured(serde_json::to_value(v).expect("serialise"))
}

fn err_json(e: &ToolError) -> CallToolResult {
    CallToolResult::structured_error(serde_json::to_value(e).expect("serialise"))
}

fn parse<T: DeserializeOwned>(args: Option<JsonObject>) -> Result<T, CallToolResult> {
    let v = serde_json::Value::Object(args.unwrap_or_default());
    serde_json::from_value(v).map_err(|e| {
        err_json(&ToolError::ValidationFailed {
            errors: vec![crate::scenario::FieldError {
                field: "arguments".into(),
                value: "see message".into(),
                allowed: "the tool's input schema".into(),
                message: e.to_string(),
            }],
        })
    })
}

fn respond<T: Serialize>(r: Result<T, ToolError>) -> CallToolResult {
    match r {
        Ok(v) => ok_json(&v),
        Err(e) => err_json(&e),
    }
}

impl KiteMcp {
    pub async fn call(&self, name: &str, args: Option<JsonObject>) -> Option<CallToolResult> {
        let app = &self.app;
        macro_rules! run {
            ($t:ty, $f:expr) => {
                match parse::<$t>(args) {
                    Ok(input) => respond($f(input).await),
                    Err(e) => e,
                }
            };
        }
        Some(match name {
            "list_models" => match parse::<service::ListModelsInput>(args) {
                Ok(_) => ok_json(&service::list_models()),
                Err(e) => e,
            },
            "describe_baseline" => run!(service::DescribeBaselineInput, |i| {
                service::describe_baseline(app, i)
            }),
            "generate_baseline" => run!(service::GenerateBaselineInput, |i| {
                service::generate_baseline(app, i)
            }),
            "run_scenario" => run!(service::RunScenarioInput, |i| service::run_scenario(app, i)),
            "get_table" => run!(service::GetTableInput, |i| service::get_table(app, i)),
            "compare_runs" => run!(service::CompareRunsInput, |i| service::compare_runs(app, i)),
            _ => return None,
        })
    }

    async fn read(&self, uri: &str) -> Result<String, ErrorData> {
        let not_found = || ErrorData::resource_not_found(format!("no resource {uri}"), None);
        let tool_err = |e: ToolError| ErrorData::internal_error(e.to_string(), None);
        if uri == "kite://citation" {
            return Ok(service::citation_text(None));
        }
        if let Some(rest) = uri.strip_prefix("kite://models/") {
            let (model, what) = rest.split_once('/').ok_or_else(not_found)?;
            if !catalogue::MODEL_IDS.contains(&model) {
                return Err(not_found());
            }
            return match what {
                "documentation" => service::documentation(&self.app, model)
                    .await
                    .map_err(tool_err),
                "citation" => Ok(service::citation_text(Some(model))),
                _ => Err(not_found()),
            };
        }
        if let Some(topic) = uri.strip_prefix("kite://docs/") {
            if !DOC_TOPICS.contains(&topic) {
                return Err(not_found());
            }
            return service::documentation(&self.app, topic)
                .await
                .map_err(tool_err);
        }
        if let Some(rest) = uri.strip_prefix("kite://runs/") {
            let (id, what) = rest.split_once('/').ok_or_else(not_found)?;
            let run = self.app.run(id).await.map_err(|_| not_found())?;
            return match what {
                "reproduce.R" => Ok(run.reproduce_script.clone()),
                "provenance.json" => Ok(serde_json::to_string_pretty(&run.provenance).unwrap()),
                _ => Err(not_found()),
            };
        }
        Err(not_found())
    }
}

const DOC_TOPICS: [&str; 3] = [
    "update_equilibrium",
    "process_results",
    "update_trade_balance",
];

pub const PROMPT_NAME: &str = "analyse_trade_policy_scenario";

pub fn prompt_text(question: &str) -> String {
    format!(
        "Analyse this trade policy question with the kite-mcp tools: {question}

         Work as follows.
         1. If the question asks about real countries or real-world magnitudes, say that this server holds synthetic data only and cannot answer it. Do not run a stand-in scenario with synthetic countries in place of real ones, and give no figures.
         2. Otherwise call list_models and choose the model whose scenario variables express the policy. Use describe_baseline to see the countries and sectors.
         3. Run only the scenarios the question asks for. If it compares policies, run each and use compare_runs for the differences. Do not add reference runs of your own, such as an empty scenario, when the question does not ask for a comparison.
         4. Report convergence first: the status, the iterations and the criterion. If the status is not_converged, say that KITE did not converge, quote the accounting reason, and give no welfare or other figures.
         5. The numbers in your answer are values from result tables and the info block, copied exactly as the tool returned them, with all their digits and without rounding, and with the tool and table each comes from. Do not repeat other numbers found in tool results: cite no file names or line numbers, and no numbers from citations. Do not compute numbers yourself: no sums, totals, shares, ratios or differences that a tool did not return. If the question needs a quantity that no table holds, say so and give the closest table instead. Fields ending in _derived are computed by kite-mcp; say they are derived.
         6. welfare_change and every *_change table are ratios of counterfactual to baseline, so a value below 1 is a fall and 1 means no change. Write numbers as decimals or in e-notation such as 9.97e-05.
         7. Report every warning KITE raised by its class and stage, and the accounting fields from the info block. Do not restate the numbers inside a warning message.
         8. State that the baseline is synthetic and describes no real economy, and pass on the baseline note that run_scenario returns as a caveat, in words.
         9. End with the reproduce script's resource URI (kite://runs/<run_id>/reproduce.R), which reruns the scenario in R without this server."
    )
}

impl ServerHandler for KiteMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
        )
        .with_server_info(Implementation::new("kite-mcp", service::KITE_MCP_VERSION))
        .with_instructions(INSTRUCTIONS)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(tool_list()))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        tool_list().into_iter().find(|t| t.name == name)
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        match self.call(&request.name, request.arguments).await {
            Some(r) => Ok(r.into()),
            None => Err(ErrorData::invalid_params(
                format!("unknown tool `{}`", request.name),
                None,
            )),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let mut out = vec![
            Resource::new("kite://citation", "citation")
                .with_title("How to cite KITE")
                .with_mime_type("text/plain"),
        ];
        for m in catalogue::MODEL_IDS {
            out.push(
                Resource::new(
                    format!("kite://models/{m}/documentation"),
                    format!("{m} documentation"),
                )
                .with_description(format!(
                    "KITE's help page for {m}, from the installed package"
                ))
                .with_mime_type("text/plain"),
            );
            out.push(
                Resource::new(
                    format!("kite://models/{m}/citation"),
                    format!("{m} citation"),
                )
                .with_mime_type("text/plain"),
            );
        }
        for t in DOC_TOPICS {
            out.push(
                Resource::new(format!("kite://docs/{t}"), format!("{t} documentation"))
                    .with_description(format!("KITE's help page for {t}()"))
                    .with_mime_type("text/plain"),
            );
        }
        for id in self.app.run_ids().await {
            out.push(
                Resource::new(
                    service::reproduce_uri(&id),
                    format!("run {id} reproduce script"),
                )
                .with_description("Standalone R script that reproduces this run with KITE alone")
                .with_mime_type("text/x-r"),
            );
            out.push(
                Resource::new(
                    format!("kite://runs/{id}/provenance.json"),
                    format!("run {id} provenance"),
                )
                .with_mime_type("application/json"),
            );
        }
        Ok(ListResourcesResult::with_all_items(out))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        let mut reproduce =
            ResourceTemplate::new("kite://runs/{run_id}/reproduce.R", "reproduce script");
        reproduce.description =
            Some("Standalone R script that reproduces a run with KITE alone".into());
        reproduce.mime_type = Some("text/x-r".into());
        let mut provenance =
            ResourceTemplate::new("kite://runs/{run_id}/provenance.json", "run provenance");
        provenance.mime_type = Some("application/json".into());
        Ok(ListResourceTemplatesResult::with_all_items(vec![
            reproduce, provenance,
        ]))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        let text = self.read(&request.uri).await?;
        Ok(ReadResourceResult::new(vec![ResourceContents::text(text, request.uri)]).into())
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        let arg = PromptArgument::new("question")
            .with_description("The trade policy question to analyse")
            .with_required(true);
        let mut p = Prompt::new(
            PROMPT_NAME,
            Some(
                "Analyse a trade policy scenario with KITE: report convergence, quote numbers only from tool results, and give the reproduce script",
            ),
            Some(vec![arg]),
        );
        p.title = Some("Analyse a trade policy scenario".into());
        Ok(ListPromptsResult::with_all_items(vec![p]))
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResponse, ErrorData> {
        if request.name != PROMPT_NAME {
            return Err(ErrorData::invalid_params(
                format!("unknown prompt `{}`", request.name),
                None,
            ));
        }
        let question = request
            .arguments
            .as_ref()
            .and_then(|a| a.get("question"))
            .and_then(|q| q.as_str())
            .ok_or_else(|| ErrorData::invalid_params("argument `question` is required", None))?;
        Ok(GetPromptResult::new(vec![PromptMessage::new_text(
            Role::User,
            prompt_text(question),
        )])
        .with_description("Analyse a trade policy scenario")
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_has_an_object_schema() {
        let tools = tool_list();
        assert_eq!(tools.len(), 6);
        for t in &tools {
            assert_eq!(
                t.input_schema.get("type").and_then(|v| v.as_str()),
                Some("object"),
                "{}",
                t.name
            );
        }
    }

    #[test]
    fn run_scenario_schema_uses_kite_names_and_refuses_unknown_fields() {
        let s = serde_json::to_string(&schema::<service::RunScenarioInput>()).unwrap();
        for name in [
            "tariff_new",
            "carbon_tax",
            "countries_climate_club",
            "trade_balance_rule",
            "allow_unread_variables",
        ] {
            assert!(s.contains(name), "{name}");
        }
        assert!(s.contains("\"additionalProperties\":false"));
        let e = serde_json::from_value::<service::RunScenarioInput>(serde_json::json!({
            "model": "caliendo_parro_2015", "scenario": {"tarif_new": []}
        }));
        assert!(e.is_err());
    }

    #[test]
    fn defaults_fill_omitted_fields() {
        let i: service::RunScenarioInput =
            serde_json::from_value(serde_json::json!({"model": "caliendo_parro_2015"})).unwrap();
        assert_eq!(i.baseline, "synthetic_example");
        assert!(!i.allow_unread_variables);
    }

    #[test]
    fn the_prompt_asks_for_convergence_sources_and_the_script() {
        let p = prompt_text("What does a tariff war do?");
        assert!(p.contains("not_converged"));
        assert!(p.contains("values from result tables and the info block"));
        assert!(p.contains("Run only the scenarios the question asks for"));
        assert!(p.contains("Do not compute numbers yourself"));
        assert!(p.contains("Do not run a stand-in scenario"));
        assert!(p.contains("reproduce.R"));
        assert!(p.contains("synthetic data only"));
    }
}
