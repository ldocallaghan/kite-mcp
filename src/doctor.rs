//! `kite-mcp doctor`: check R, KITE and the worker, and say how to fix what
//! is missing.

use crate::catalogue::{KITE_COMMIT, KITE_VERSION};
use crate::protocol::Value;
use crate::service::{self, App, RunScenarioInput};
use crate::worker::{Worker, WorkerConfig};
use std::process::Command;
use std::time::{Duration, Instant};

struct Report {
    failures: usize,
}

impl Report {
    fn ok(&self, what: &str, detail: &str) {
        println!("ok    {what}: {detail}");
    }
    fn fail(&mut self, what: &str, detail: &str, fix: &str) {
        self.failures += 1;
        println!("FAIL  {what}: {detail}");
        println!("      fix: {fix}");
    }
}

fn rscript(config: &WorkerConfig, expr: &str) -> Result<String, String> {
    let out = Command::new(&config.program)
        .args(["--vanilla", "-e", expr])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub async fn run(timeout: Duration) -> i32 {
    let config = WorkerConfig::r_from_env(timeout);
    let mut r = Report { failures: 0 };
    println!("kite-mcp {} doctor\n", service::KITE_MCP_VERSION);

    match rscript(&config, "cat(R.version.string)") {
        Ok(v) => r.ok("R", &format!("{v} ({})", config.program.display())),
        Err(e) => {
            r.fail(
                "R",
                &format!("cannot run {}: {e}", config.program.display()),
                "install R from https://cloud.r-project.org, or set KITE_MCP_RSCRIPT to the path of Rscript",
            );
            println!("\n{} problem(s) found.", r.failures);
            return 1;
        }
    }

    for pkg in ["jsonlite", "data.table", "cli"] {
        match rscript(&config, &format!("cat(as.character(packageVersion('{pkg}')))")) {
            Ok(v) => r.ok(&format!("R package {pkg}"), &v),
            Err(_) => r.fail(
                &format!("R package {pkg}"),
                "not installed",
                &format!("Rscript -e \"install.packages('{pkg}', repos = 'https://cloud.r-project.org')\""),
            ),
        }
    }

    let install = format!(
        "Rscript -e \"remotes::install_github('julianhinz/KITE@{KITE_COMMIT}')\" (or Rscript scripts/install-r-deps.R)"
    );
    match rscript(
        &config,
        "d <- utils::packageDescription('KITE'); cat(d$Version, if (is.null(d$RemoteSha)) 'none' else d$RemoteSha)",
    ) {
        Ok(v) => {
            let mut parts = v.split_whitespace();
            let version = parts.next().unwrap_or("");
            let sha = parts.next().unwrap_or("none");
            if version == KITE_VERSION && sha == KITE_COMMIT {
                r.ok("KITE", &format!("{version} at commit {sha}"));
            } else {
                r.fail(
                    "KITE",
                    &format!("version {version}, commit {sha}; kite-mcp is pinned to {KITE_VERSION} at {KITE_COMMIT}"),
                    &install,
                );
            }
        }
        Err(_) => r.fail("KITE", "not installed", &install),
    }

    let script = config.args.last().cloned().unwrap_or_default();
    if std::path::Path::new(&script).exists() {
        r.ok("worker script", &script);
    } else {
        r.fail(
            "worker script",
            &format!("{script} does not exist"),
            "set KITE_MCP_WORKER to the path of worker/kite_worker.R",
        );
    }
    if r.failures > 0 {
        println!("\n{} problem(s) found.", r.failures);
        return 1;
    }

    let app = App::new(Worker::new(config));
    let started = Instant::now();
    match app.worker.hello().await {
        Ok(h) => r.ok(
            "worker start-up",
            &format!(
                "{} ms, protocol {}, {} {}",
                started.elapsed().as_millis(),
                h.protocol,
                h.model_package.name,
                h.model_package.version
            ),
        ),
        Err(e) => {
            r.fail(
                "worker start-up",
                &e.to_string(),
                "run the worker by hand to see R's error: Rscript worker/kite_worker.R < /dev/null",
            );
            println!("\n{} problem(s) found.", r.failures);
            return 1;
        }
    }

    // KITE's README: the example tariff war converges in 22 iterations under caliendo_parro_2015
    let input: RunScenarioInput = serde_json::from_value(serde_json::json!({
        "model": "caliendo_parro_2015",
        "scenario": {"tariff_new": [
            {"origin": "c1", "destination": "c2", "value": 1.2},
            {"origin": "c2", "destination": "c1", "value": 1.2}
        ]}
    }))
    .expect("doctor scenario");
    let started = Instant::now();
    match service::run_scenario(&app, input).await {
        Ok(out) if out.status == "converged" && out.info.iterations == Some(22) => r.ok(
            "example solve",
            &format!(
                "caliendo_parro_2015 tariff war converged in 22 iterations, as KITE's README states ({} ms)",
                started.elapsed().as_millis()
            ),
        ),
        Ok(out) => r.fail(
            "example solve",
            &format!("status {}, iterations {:?}; KITE's README gives 22", out.status, out.info.iterations),
            "check that KITE is at the pinned commit",
        ),
        Err(e) => r.fail("example solve", &e.to_string(), "see the error above"),
    }
    let _ = Value::Logical(true);

    if r.failures == 0 {
        println!("\nAll checks passed.");
        0
    } else {
        println!("\n{} problem(s) found.", r.failures);
        1
    }
}
