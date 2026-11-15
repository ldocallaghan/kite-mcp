//! The tools, independent of transport. MCP (src/mcp.rs) and HTTP
//! (src/http.rs) both call these functions.

use crate::baseline::{self, Baseline, Source};
use crate::catalogue::{self, ModelInfo, TableInfo};
use crate::num::Num;
use crate::protocol::{
    ColumnData, Condition, Hello, InitialConditions, NamedTable, SolveParams, SolveResult, Table,
    Value, value_as_bool, value_as_f64, value_as_i64, value_as_string,
};
use crate::reproduce;
use crate::scenario::{self, FieldError, ScenarioInput, SettingsInput};
use crate::worker::{Worker, WorkerFailure};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::sync::RwLock;

pub const KITE_MCP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SYNTHETIC_EXAMPLE: &str = "synthetic_example";
pub const DEFAULT_ROW_LIMIT: usize = 100;
pub const MAX_ROW_LIMIT: usize = 10_000;

// ---- errors ------------------------------------------------------------------

/// A tool failure, shown to the client as the tool's error result.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum ToolError {
    /// The input did not pass validation; nothing was run
    ValidationFailed { errors: Vec<FieldError> },
    /// A run, baseline or table was not found
    NotFound {
        field: String,
        value: String,
        allowed: Vec<String>,
        message: String,
    },
    /// The result is not available as a result because KITE did not converge
    NotConverged { run_id: String, message: String },
    /// KITE stopped with an error; nothing was computed
    KiteError { class: Vec<String>, message: String },
    /// The worker that runs KITE failed
    WorkerFailed { message: String, hint: String },
}

impl ToolError {
    fn not_found(field: &str, value: &str, allowed: Vec<String>, message: String) -> Self {
        ToolError::NotFound {
            field: field.into(),
            value: value.into(),
            allowed,
            message,
        }
    }
}

impl From<WorkerFailure> for ToolError {
    fn from(f: WorkerFailure) -> Self {
        match f {
            WorkerFailure::Model(e) => ToolError::KiteError {
                class: e.class,
                message: e.message,
            },
            other => ToolError::WorkerFailed {
                message: other.to_string(),
                hint: "run `kite-mcp doctor` to check R, KITE and the worker".into(),
            },
        }
    }
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap_or_default())
    }
}

// ---- state -------------------------------------------------------------------

pub struct Run {
    pub id: String,
    pub model: String,
    pub baseline: Arc<Baseline>,
    pub converged: bool,
    pub result: SolveResult,
    pub provenance: Provenance,
    pub reproduce_script: String,
}

pub struct App {
    pub worker: Worker,
    baselines: RwLock<BTreeMap<String, Arc<Baseline>>>,
    runs: RwLock<HashMap<String, Arc<Run>>>,
    run_lock: tokio::sync::Mutex<()>,
}

impl App {
    pub fn new(worker: Worker) -> Self {
        App {
            worker,
            baselines: RwLock::new(BTreeMap::new()),
            runs: RwLock::new(HashMap::new()),
            run_lock: tokio::sync::Mutex::new(()),
        }
    }

    pub async fn add_baseline(&self, b: Baseline) -> Arc<Baseline> {
        let b = Arc::new(b);
        self.baselines.write().await.insert(b.id.clone(), b.clone());
        b
    }

    async fn synthetic(
        &self,
        id: &str,
        source: Source,
        n_c: usize,
        n_s: usize,
        seed: i64,
    ) -> Result<Arc<Baseline>, ToolError> {
        if let Some(b) = self.baselines.read().await.get(id) {
            return Ok(b.clone());
        }
        let ic: InitialConditions = self
            .worker
            .call_typed(
                "synthetic_baseline",
                &serde_json::json!({"countries": n_c, "sectors": n_s, "seed": seed}),
            )
            .await?;
        let b = Baseline::new(id.to_string(), source, ic.initial_conditions).map_err(|e| {
            ToolError::WorkerFailed {
                message: format!("the worker built a baseline that fails validation: {e}"),
                hint: "this is a defect in kite-mcp".into(),
            }
        })?;
        Ok(self.add_baseline(b).await)
    }

    pub async fn baseline(&self, id: &str) -> Result<Arc<Baseline>, ToolError> {
        if id == SYNTHETIC_EXAMPLE {
            return self
                .synthetic(SYNTHETIC_EXAMPLE, Source::SyntheticExample, 3, 2, 1)
                .await;
        }
        if let Some(b) = self.baselines.read().await.get(id) {
            return Ok(b.clone());
        }
        Err(ToolError::not_found(
            "baseline",
            id,
            self.baseline_ids().await,
            format!(
                "no baseline `{id}`; generate one with generate_baseline or use one of the listed ids"
            ),
        ))
    }

    async fn baseline_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.baselines.read().await.keys().cloned().collect();
        if !ids.iter().any(|i| i == SYNTHETIC_EXAMPLE) {
            ids.insert(0, SYNTHETIC_EXAMPLE.into());
        }
        ids
    }

    pub async fn run(&self, id: &str) -> Result<Arc<Run>, ToolError> {
        if let Some(r) = self.runs.read().await.get(id) {
            return Ok(r.clone());
        }
        let mut ids: Vec<String> = self.runs.read().await.keys().cloned().collect();
        ids.sort();
        Err(ToolError::not_found(
            "run_id",
            id,
            ids,
            format!("no run `{id}` in this server session"),
        ))
    }

    pub async fn run_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.runs.read().await.keys().cloned().collect();
        ids.sort();
        ids
    }
}

// ---- list_models -------------------------------------------------------------

#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListModelsInput {}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ListModelsOutput {
    pub kite_version: &'static str,
    pub kite_commit: &'static str,
    pub kite_citation: &'static str,
    pub models: Vec<ModelInfo>,
    pub conventions: Vec<&'static str>,
}

pub fn list_models() -> ListModelsOutput {
    ListModelsOutput {
        kite_version: catalogue::KITE_VERSION,
        kite_commit: catalogue::KITE_COMMIT,
        kite_citation: catalogue::KITE_CITATION,
        models: catalogue::MODEL_IDS
            .iter()
            .filter_map(|m| catalogue::model(m))
            .collect(),
        conventions: vec![
            "Variables named *_new are counterfactual levels; tariffs are gross factors, so 1.2 is a 20 % tariff (R/model_caliendo_parro_2015.R:29-31).",
            "Output tables named *_change are ratios of counterfactual to baseline: 1 means no change (R/process_results.R).",
            "World value added is the numeraire (R/model_mahlkow_wanner_2021.R:25,88).",
            "All baselines this server holds are synthetic unless describe_baseline says otherwise; they describe no real economy.",
        ],
    }
}

// ---- describe_baseline / generate_baseline -----------------------------------

#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DescribeBaselineInput {
    /// Baseline id; omit to list every baseline
    #[serde(default)]
    pub baseline: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TableSummary {
    pub name: String,
    pub dimensions: Vec<String>,
    pub rows: usize,
    pub description: &'static str,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct BaselineDescription {
    pub id: String,
    pub source: Source,
    /// True for every baseline built by kite-mcp: invented data, not a real economy
    pub synthetic: bool,
    pub countries: Vec<String>,
    pub sectors: Vec<String>,
    pub tables: Vec<TableSummary>,
    /// SHA-256 of the baseline's tables as passed to KITE
    pub hash: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum DescribeBaselineOutput {
    One(BaselineDescription),
    All { baselines: Vec<BaselineDescription> },
}

fn describe(b: &Baseline) -> BaselineDescription {
    let mut notes = b.notes.clone();
    if b.synthetic {
        notes.insert(0, "Synthetic data generated from random draws, as in KITE's inst/examples/example.R. It describes no real economy, and its numbers have no real-world units.".into());
    }
    BaselineDescription {
        id: b.id.clone(),
        source: b.source.clone(),
        synthetic: b.synthetic,
        countries: b.countries.clone(),
        sectors: b.sectors.clone(),
        tables: b
            .tables
            .iter()
            .map(|n| {
                let Value::Table(t) = &n.value else {
                    unreachable!()
                };
                TableSummary {
                    name: n.name.clone(),
                    dimensions: t.dimension_names(),
                    rows: t.n_rows(),
                    description: baseline::spec(&n.name).map(|s| s.description).unwrap_or(""),
                }
            })
            .collect(),
        hash: b.hash.clone(),
        notes,
    }
}

pub async fn describe_baseline(
    app: &App,
    input: DescribeBaselineInput,
) -> Result<DescribeBaselineOutput, ToolError> {
    match input.baseline {
        Some(id) => {
            let b: Arc<Baseline> = app.baseline(&id).await?;
            Ok(DescribeBaselineOutput::One(describe(&b)))
        }
        None => {
            app.baseline(SYNTHETIC_EXAMPLE).await?;
            let all = app
                .baselines
                .read()
                .await
                .values()
                .map(|b| describe(b))
                .collect();
            Ok(DescribeBaselineOutput::All { baselines: all })
        }
    }
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerateBaselineInput {
    /// Number of countries, from 2 to 30
    pub countries: usize,
    /// Number of sectors, from 1 to 10
    pub sectors: usize,
    /// Random seed, from 0 to 2147483647
    pub seed: i64,
}

pub async fn generate_baseline(
    app: &App,
    input: GenerateBaselineInput,
) -> Result<BaselineDescription, ToolError> {
    let mut errors = Vec::new();
    let mut check = |field: &str, v: i64, lo: i64, hi: i64| {
        if v < lo || v > hi {
            errors.push(FieldError {
                field: field.into(),
                value: v.to_string(),
                allowed: format!("[{lo}, {hi}]"),
                message: format!("`{field}` is {v}; it must be from {lo} to {hi}"),
            });
        }
    };
    check("countries", input.countries as i64, 2, 30);
    check("sectors", input.sectors as i64, 1, 10);
    check("seed", input.seed, 0, i32::MAX as i64);
    if !errors.is_empty() {
        return Err(ToolError::ValidationFailed { errors });
    }
    let id = format!(
        "synthetic_c{}_s{}_seed{}",
        input.countries, input.sectors, input.seed
    );
    let b = app
        .synthetic(
            &id,
            Source::Generated {
                countries: input.countries,
                sectors: input.sectors,
                seed: input.seed,
            },
            input.countries,
            input.sectors,
            input.seed,
        )
        .await?;
    Ok(describe(&b))
}

// ---- run_scenario ------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunScenarioInput {
    /// One of the model ids from list_models
    pub model: String,
    /// Baseline id; default synthetic_example
    #[serde(default = "default_baseline")]
    pub baseline: String,
    /// Scenario variables under KITE's names
    #[serde(default)]
    pub scenario: ScenarioInput,
    /// Solver settings; omitted fields take the defaults of KITE's example
    #[serde(default)]
    pub settings: SettingsInput,
    /// Pass variables the model does not read on to KITE, which warns that they have no effect. Default false: such variables are refused.
    #[serde(default)]
    pub allow_unread_variables: bool,
}

fn default_baseline() -> String {
    SYNTHETIC_EXAMPLE.into()
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct AccountingOut {
    pub ok: Option<bool>,
    pub rule: Option<String>,
    pub world_trade_balance: Option<Num>,
    pub baseline_world_trade_balance: Option<Num>,
    pub world_value_added: Option<Num>,
    pub tolerance: Option<Num>,
    pub reason: Option<String>,
}

/// KITE's `info` block, as returned by update_equilibrium().
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct InfoOut {
    /// TRUE, FALSE or NA (null) as KITE reports it
    pub convergence: Option<bool>,
    pub criterion: Option<Num>,
    pub iterations: Option<i64>,
    pub elapsed_seconds: Option<Num>,
    pub accounting: AccountingOut,
}

fn info_out(r: &SolveResult) -> InfoOut {
    let i = &r.info;
    let a = &i.accounting;
    InfoOut {
        convergence: value_as_bool(&i.convergence),
        criterion: value_as_f64(&i.criterion).map(Num),
        iterations: value_as_i64(&i.iterations),
        elapsed_seconds: value_as_f64(&i.elapsed_seconds).map(Num),
        accounting: AccountingOut {
            ok: value_as_bool(&a.ok),
            rule: value_as_string(&a.rule),
            world_trade_balance: value_as_f64(&a.world_trade_balance).map(Num),
            baseline_world_trade_balance: value_as_f64(&a.baseline_world_trade_balance).map(Num),
            world_value_added: value_as_f64(&a.world_value_added).map(Num),
            tolerance: value_as_f64(&a.tolerance).map(Num),
            reason: value_as_string(&a.reason),
        },
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct KiteInfo {
    pub version: String,
    pub release_name: Option<String>,
    pub commit: Option<String>,
    pub repository: &'static str,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct WorkerInfo {
    pub language: String,
    pub runtime_version: String,
    pub platform: String,
    pub packages: serde_json::Map<String, serde_json::Value>,
    pub rng_kind: Vec<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct BaselineRef {
    pub id: String,
    pub source: Source,
    pub synthetic: bool,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ResolvedSettings {
    pub tolerance: Num,
    pub vfactor: Num,
    pub max_iterations: u32,
    /// The rule passed to KITE; null means KITE's default, fixed
    pub trade_balance_rule: Option<String>,
    /// Null means KITE's default, 1e-6
    pub tolerance_accounting: Option<Num>,
}

/// Everything needed to know where a number came from.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Provenance {
    pub kite: KiteInfo,
    pub worker: WorkerInfo,
    pub kite_mcp_version: &'static str,
    pub model: String,
    pub baseline: BaselineRef,
    /// The scenario as given to run_scenario
    pub scenario: ScenarioInput,
    pub allow_unread_variables: bool,
    pub settings: ResolvedSettings,
    pub convergence: InfoOut,
    /// When the run was computed, UTC
    pub computed_at: String,
    /// SHA-256 of (KITE commit, model, baseline hash, scenario, settings)
    pub cache_key: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct WelfareRow {
    pub country: String,
    /// welfare_change as KITE reports it: ratio of counterfactual to baseline real income per head
    pub welfare_change: Num,
    /// Derived by kite-mcp: (welfare_change - 1) * 100
    pub percent_change_derived: Num,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Headline {
    pub table: &'static str,
    pub by_country: Vec<WelfareRow>,
    pub note: &'static str,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Reproduce {
    pub resource_uri: String,
    pub script: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RunScenarioOutput {
    pub run_id: String,
    /// converged or not_converged
    pub status: &'static str,
    /// True when this exact run was computed before in this session
    pub cached: bool,
    pub info: InfoOut,
    /// Every warning KITE raised, with its R class
    pub warnings: Vec<Condition>,
    /// Present only when KITE converged
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headline: Option<Headline>,
    /// Present only when KITE did not converge
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_converged: Option<String>,
    pub tables: Vec<String>,
    pub provenance: Provenance,
    pub reproduce: Reproduce,
}

fn cache_key(commit: &str, model: &str, b: &Baseline, built: &scenario::Built) -> String {
    let v = serde_json::json!({
        "kite_commit": commit,
        "model": model,
        "baseline_hash": b.hash,
        "scenario": built.scenario.iter().map(|v| (v.name, &v.value)).collect::<Vec<_>>(),
        "settings": built.settings,
    });
    let digest = Sha256::digest(serde_json::to_vec(&v).unwrap());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn not_converged_message(info: &InfoOut) -> String {
    let conv = match info.convergence {
        Some(false) => "FALSE",
        None => "NA (could not be established)",
        Some(true) => "TRUE",
    };
    let mut s = format!(
        "KITE reports convergence = {conv} after {} iterations (criterion {}).",
        info.iterations
            .map(|i| i.to_string())
            .unwrap_or("NA".into()),
        info.criterion
            .map(|c| serde_json::to_string(&c).unwrap())
            .unwrap_or("NA".into())
    );
    if info.accounting.ok == Some(false) {
        s.push_str(&format!(
            " World trade balance check failed: {}.",
            info.accounting.reason.clone().unwrap_or_default()
        ));
    }
    s.push_str(" No result is reported. The tables remain available through get_table, every row flagged unconverged.");
    s
}

fn welfare_headline(r: &SolveResult) -> Option<Headline> {
    let t = r.tables.iter().find(|t| t.name == "welfare_change")?;
    let countries = t.table.strings("country")?;
    let values = t.table.doubles("value")?;
    Some(Headline {
        table: "welfare_change",
        by_country: countries
            .iter()
            .zip(values)
            .map(|(c, v)| WelfareRow {
                country: c.clone().unwrap_or_default(),
                welfare_change: Num(v),
                percent_change_derived: Num((v - 1.0) * 100.0),
            })
            .collect(),
        note: "welfare_change is exactly the value KITE's process_results() returns (R/process_results.R:345). percent_change_derived is computed by kite-mcp as (welfare_change - 1) * 100.",
    })
}

pub async fn run_scenario(
    app: &App,
    input: RunScenarioInput,
) -> Result<RunScenarioOutput, ToolError> {
    let b = app.baseline(&input.baseline).await?;
    let built = scenario::build(
        &input.model,
        &b,
        &input.scenario,
        &input.settings,
        input.allow_unread_variables,
    )
    .map_err(|errors| ToolError::ValidationFailed { errors })?;
    let hello: Hello = app.worker.hello().await?;
    let commit = hello.model_package.commit.clone().unwrap_or_default();
    let key = cache_key(&commit, &input.model, &b, &built);
    let run_id = format!("r{}", &key[..12]);

    // one solve at a time; a second identical request waits and hits the cache
    let _guard = app.run_lock.lock().await;
    if let Some(r) = app.runs.read().await.get(&run_id).cloned() {
        return Ok(run_output(&r, true));
    }

    let ic = b.tables_for_model(&input.model);
    let scenario_named: Vec<crate::protocol::Named> = built
        .scenario
        .iter()
        .map(|v| crate::protocol::Named::new(v.name, v.value.clone()))
        .collect();
    let params = SolveParams {
        model: &input.model,
        initial_conditions: &ic,
        scenario: &scenario_named,
        settings: &built.settings,
    };
    let result: SolveResult = app.worker.call_typed("solve", &params).await?;
    let info = info_out(&result);
    let converged = info.convergence == Some(true);
    let computed_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let settings = ResolvedSettings {
        tolerance: Num(input
            .settings
            .tolerance
            .unwrap_or(scenario::DEFAULT_TOLERANCE)),
        vfactor: Num(input.settings.vfactor.unwrap_or(scenario::DEFAULT_VFACTOR)),
        max_iterations: input
            .settings
            .max_iterations
            .unwrap_or(scenario::DEFAULT_MAX_ITERATIONS),
        trade_balance_rule: input
            .settings
            .trade_balance_rule
            .map(|r| r.as_str().to_string()),
        tolerance_accounting: input.settings.tolerance_accounting.map(Num),
    };
    let provenance = Provenance {
        kite: KiteInfo {
            version: hello.model_package.version.clone(),
            release_name: hello.model_package.release_name.clone(),
            commit: hello.model_package.commit.clone(),
            repository: catalogue::KITE_REPOSITORY,
        },
        worker: WorkerInfo {
            language: hello.language.clone(),
            runtime_version: hello.runtime_version.clone(),
            platform: hello.platform.clone(),
            packages: hello.packages.clone(),
            rng_kind: hello.rng_kind.clone(),
        },
        kite_mcp_version: KITE_MCP_VERSION,
        model: input.model.clone(),
        baseline: BaselineRef {
            id: b.id.clone(),
            source: b.source.clone(),
            synthetic: b.synthetic,
            hash: b.hash.clone(),
        },
        scenario: input.scenario.clone(),
        allow_unread_variables: input.allow_unread_variables,
        settings,
        convergence: info,
        computed_at: computed_at.clone(),
        cache_key: key,
    };
    // which literals R's parser reads to the same bits as their shortest decimal
    let literals = reproduce::literal_values(&built.scenario, &built.settings);
    let decimals: Vec<String> = literals
        .iter()
        .map(|x| crate::num::shortest_decimal(*x))
        .collect();
    let read: ReadDecimals = app
        .worker
        .call_typed(
            "read_decimals",
            &serde_json::json!({ "decimals": decimals }),
        )
        .await?;
    let r_exact: std::collections::HashSet<u64> = literals
        .iter()
        .zip(&read.doubles)
        .filter(|(x, r)| x.to_bits() == r.0.to_bits())
        .map(|(x, _)| x.to_bits())
        .collect();
    let script = reproduce::script(&reproduce::ScriptInput {
        run_id: &run_id,
        kite_mcp_version: KITE_MCP_VERSION,
        created_at: &computed_at,
        kite_version: &hello.model_package.version,
        kite_commit: &commit,
        model: &input.model,
        baseline: &b,
        scenario: &built.scenario,
        settings: &built.settings,
        r_exact: &r_exact,
    });
    let run = Arc::new(Run {
        id: run_id.clone(),
        model: input.model.clone(),
        baseline: b.clone(),
        converged,
        result,
        provenance,
        reproduce_script: script,
    });
    app.runs.write().await.insert(run_id, run.clone());
    Ok(run_output(&run, false))
}

#[derive(Deserialize)]
struct ReadDecimals {
    doubles: Vec<crate::num::HexF64>,
}

pub fn reproduce_uri(id: &str) -> String {
    format!("kite://runs/{id}/reproduce.R")
}

fn run_output(r: &Run, cached: bool) -> RunScenarioOutput {
    let info = info_out(&r.result);
    RunScenarioOutput {
        run_id: r.id.clone(),
        status: if r.converged {
            "converged"
        } else {
            "not_converged"
        },
        cached,
        headline: if r.converged {
            welfare_headline(&r.result)
        } else {
            None
        },
        not_converged: if r.converged {
            None
        } else {
            Some(not_converged_message(&info))
        },
        info,
        warnings: r.result.warnings.clone(),
        tables: r.result.tables.iter().map(|t| t.name.clone()).collect(),
        provenance: r.provenance.clone(),
        reproduce: Reproduce {
            resource_uri: reproduce_uri(&r.id),
            script: r.reproduce_script.clone(),
        },
    }
}

// ---- get_table / compare_runs ------------------------------------------------

/// Keep rows whose dimension is one of the listed codes. Omitted means all.
#[derive(Debug, Default, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Filters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sector: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination: Option<Vec<String>>,
}

impl Filters {
    fn pairs(&self) -> Vec<(&'static str, &Vec<String>)> {
        let mut v = Vec::new();
        if let Some(x) = &self.country {
            v.push(("country", x))
        }
        if let Some(x) = &self.sector {
            v.push(("sector", x))
        }
        if let Some(x) = &self.origin {
            v.push(("origin", x))
        }
        if let Some(x) = &self.destination {
            v.push(("destination", x))
        }
        v
    }
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetTableInput {
    pub run_id: String,
    /// A table name from run_scenario's `tables`, for example welfare_change or trade_flow_change
    pub table: String,
    #[serde(default)]
    pub filters: Filters,
    /// Maximum rows returned, from 1 to 10000. Default 100.
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetTableOutput {
    pub run_id: String,
    pub table: String,
    pub status: &'static str,
    /// Present when the run did not converge: these rows are not results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    pub info: TableInfo,
    pub columns: Vec<String>,
    pub total_rows: usize,
    pub matching_rows: usize,
    pub returned_rows: usize,
    pub truncated: bool,
    pub rows: Vec<serde_json::Map<String, serde_json::Value>>,
    pub baseline_synthetic: bool,
}

fn find_table<'a>(r: &'a Run, name: &str) -> Result<&'a Table, ToolError> {
    r.result
        .tables
        .iter()
        .find(|t| t.name == name)
        .map(|t: &NamedTable| &t.table)
        .ok_or_else(|| {
            ToolError::not_found(
                "table",
                name,
                r.result.tables.iter().map(|t| t.name.clone()).collect(),
                format!("run `{}` has no table `{name}`", r.id),
            )
        })
}

fn cell_json(c: &ColumnData, i: usize) -> serde_json::Value {
    match c {
        ColumnData::String(v) => v[i]
            .clone()
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
        ColumnData::Double(v) => serde_json::to_value(Num(v[i].0)).unwrap(),
        ColumnData::Integer(v) => v[i]
            .map(serde_json::Value::from)
            .unwrap_or(serde_json::Value::Null),
        ColumnData::Logical(v) => v[i]
            .map(serde_json::Value::from)
            .unwrap_or(serde_json::Value::Null),
    }
}

/// Row indices that pass the filters; errors name unknown dimensions and codes.
fn filter_rows(t: &Table, table: &str, f: &Filters) -> Result<Vec<usize>, ToolError> {
    let dims = t.dimension_names();
    let mut errors = Vec::new();
    for (d, codes) in f.pairs() {
        match t.strings(d) {
            None => errors.push(FieldError {
                field: format!("filters.{d}"),
                value: codes.join(", "),
                allowed: format!("filters on [{}]", dims.join(", ")),
                message: format!("table `{table}` has no dimension `{d}`"),
            }),
            Some(col) => {
                let mut present: Vec<String> = col.iter().flatten().cloned().collect();
                present.sort();
                present.dedup();
                for c in codes {
                    if !present.contains(c) {
                        errors.push(FieldError {
                            field: format!("filters.{d}"),
                            value: c.clone(),
                            allowed: format!("[{}]", present.join(", ")),
                            message: format!("`{c}` does not occur in `{d}` of table `{table}`"),
                        });
                    }
                }
            }
        }
    }
    if !errors.is_empty() {
        return Err(ToolError::ValidationFailed { errors });
    }
    Ok((0..t.n_rows())
        .filter(|&i| {
            f.pairs().iter().all(|(d, codes)| {
                let v = t.strings(d).unwrap()[i].clone().unwrap_or_default();
                codes.contains(&v)
            })
        })
        .collect())
}

fn check_limit(limit: Option<usize>) -> Result<usize, ToolError> {
    let l = limit.unwrap_or(DEFAULT_ROW_LIMIT);
    if l == 0 || l > MAX_ROW_LIMIT {
        return Err(ToolError::ValidationFailed {
            errors: vec![FieldError {
                field: "limit".into(),
                value: l.to_string(),
                allowed: format!("[1, {MAX_ROW_LIMIT}]"),
                message: format!("`limit` is {l}; it must be from 1 to {MAX_ROW_LIMIT}"),
            }],
        });
    }
    Ok(l)
}

pub async fn get_table(app: &App, input: GetTableInput) -> Result<GetTableOutput, ToolError> {
    let r = app.run(&input.run_id).await?;
    let limit = check_limit(input.limit)?;
    let t = find_table(&r, &input.table)?;
    let keep = filter_rows(t, &input.table, &input.filters)?;
    let mut rows = Vec::new();
    for &i in keep.iter().take(limit) {
        let mut row = serde_json::Map::new();
        for c in &t.columns {
            row.insert(c.name.clone(), cell_json(&c.data, i));
        }
        if !r.converged {
            row.insert("unconverged".into(), serde_json::Value::Bool(true));
        }
        rows.push(row);
    }
    let mut columns: Vec<String> = t.columns.iter().map(|c| c.name.clone()).collect();
    if !r.converged {
        columns.push("unconverged".into());
    }
    Ok(GetTableOutput {
        run_id: r.id.clone(),
        table: input.table.clone(),
        status: if r.converged {
            "converged"
        } else {
            "not_converged"
        },
        warning: if r.converged {
            None
        } else {
            Some("KITE did not converge for this run. These rows are the solver's last iterate, not an equilibrium, and must not be reported as results.".into())
        },
        info: catalogue::table_info(&input.table),
        columns,
        total_rows: t.n_rows(),
        matching_rows: keep.len(),
        returned_rows: rows.len(),
        truncated: keep.len() > rows.len(),
        rows,
        baseline_synthetic: r.baseline.synthetic,
    })
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompareRunsInput {
    /// The reference run
    pub run_a: String,
    /// The run compared with run_a
    pub run_b: String,
    pub table: String,
    #[serde(default)]
    pub filters: Filters,
    /// Maximum rows returned, from 1 to 10000. Default 100.
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CompareRunsOutput {
    pub run_a: String,
    pub run_b: String,
    pub table: String,
    pub info: TableInfo,
    pub dimensions: Vec<String>,
    pub matching_rows: usize,
    pub returned_rows: usize,
    pub truncated: bool,
    /// Rows with value_a and value_b exactly as KITE computed them, and difference_derived = value_b - value_a computed by kite-mcp
    pub rows: Vec<serde_json::Map<String, serde_json::Value>>,
    /// Dimension keys present in one run only
    pub unmatched: Vec<String>,
    pub baseline_synthetic: bool,
}

pub async fn compare_runs(
    app: &App,
    input: CompareRunsInput,
) -> Result<CompareRunsOutput, ToolError> {
    let a = app.run(&input.run_a).await?;
    let b = app.run(&input.run_b).await?;
    let limit = check_limit(input.limit)?;
    for r in [&a, &b] {
        if !r.converged {
            return Err(ToolError::NotConverged {
                run_id: r.id.clone(),
                message: format!(
                    "run `{}` did not converge, so it has no results to compare. {}",
                    r.id,
                    not_converged_message(&info_out(&r.result))
                ),
            });
        }
    }
    let ta = find_table(&a, &input.table)?;
    let tb = find_table(&b, &input.table)?;
    let dims = ta.dimension_names();
    if dims != tb.dimension_names() {
        return Err(ToolError::ValidationFailed {
            errors: vec![FieldError {
                field: "table".into(),
                value: input.table.clone(),
                allowed: "a table with the same dimensions in both runs".into(),
                message: "the two runs' tables have different dimensions".into(),
            }],
        });
    }
    let key = |t: &Table, i: usize| -> Vec<String> {
        dims.iter()
            .map(|d| t.strings(d).unwrap()[i].clone().unwrap_or_default())
            .collect()
    };
    let va = ta.doubles("value").unwrap_or_default();
    let vb = tb.doubles("value").unwrap_or_default();
    let b_index: HashMap<Vec<String>, usize> = (0..tb.n_rows()).map(|i| (key(tb, i), i)).collect();
    let keep = filter_rows(ta, &input.table, &input.filters)?;
    let mut rows = Vec::new();
    let mut matched = 0;
    let mut unmatched = Vec::new();
    for &i in &keep {
        let k = key(ta, i);
        match b_index.get(&k) {
            Some(&j) => {
                matched += 1;
                if rows.len() < limit {
                    let mut row = serde_json::Map::new();
                    for (d, c) in dims.iter().zip(&k) {
                        row.insert(d.clone(), serde_json::Value::String(c.clone()));
                    }
                    row.insert("value_a".into(), serde_json::to_value(Num(va[i])).unwrap());
                    row.insert("value_b".into(), serde_json::to_value(Num(vb[j])).unwrap());
                    row.insert(
                        "difference_derived".into(),
                        serde_json::to_value(Num(vb[j] - va[i])).unwrap(),
                    );
                    rows.push(row);
                }
            }
            None => unmatched.push(format!("({}) in run_a only", k.join(", "))),
        }
    }
    let a_keys: std::collections::HashSet<Vec<String>> =
        (0..ta.n_rows()).map(|i| key(ta, i)).collect();
    for j in 0..tb.n_rows() {
        let k = key(tb, j);
        if !a_keys.contains(&k) {
            unmatched.push(format!("({}) in run_b only", k.join(", ")));
        }
    }
    Ok(CompareRunsOutput {
        run_a: a.id.clone(),
        run_b: b.id.clone(),
        table: input.table.clone(),
        info: catalogue::table_info(&input.table),
        dimensions: dims,
        matching_rows: matched,
        returned_rows: rows.len(),
        truncated: matched > rows.len(),
        rows,
        unmatched,
        baseline_synthetic: a.baseline.synthetic && b.baseline.synthetic,
    })
}

// ---- documentation -----------------------------------------------------------

#[derive(Deserialize)]
struct DocText {
    text: String,
}

pub async fn documentation(app: &App, topic: &str) -> Result<String, ToolError> {
    let d: DocText = app
        .worker
        .call_typed("documentation", &serde_json::json!({"topic": topic}))
        .await?;
    Ok(d.text)
}

pub fn citation_text(model: Option<&str>) -> String {
    let mut s = String::new();
    s.push_str("Cite KITE through its white paper, as KITE's README asks:\n\n");
    s.push_str(catalogue::KITE_CITATION);
    s.push_str("\n\n");
    s.push_str(catalogue::KITE_BIBTEX);
    s.push_str("\n\nSoftware entry:\n\n");
    s.push_str(catalogue::KITE_PACKAGE_CITATION);
    s.push('\n');
    if let Some(m) = model.and_then(catalogue::model) {
        s.push_str(&format!("\nModel {}: {}\n", m.id, m.citation));
    }
    s.push_str("\nkite-mcp is an independent, unofficial interface to KITE with no affiliation to or endorsement from the Kiel Institute for the World Economy.\n");
    s
}
