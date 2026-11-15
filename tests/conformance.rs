//! Conformance: every table and the info block returned through the MCP
//! server equal a direct call to KITE, bit for bit; every reproduce script
//! reproduces them in a fresh R session.
//!
//! Needs R with KITE at the pinned commit. Runs when KITE_MCP_CONFORMANCE=1;
//! otherwise each test returns at once.

use kite_mcp::num::{f64_to_hex, hex_to_f64, is_r_na, shortest_decimal};
use kite_mcp::reproduce::r_hex_literal;
use rmcp::model::{CallToolRequestParams, ReadResourceRequestParams, ResourceContents};
use rmcp::service::RunningService;
use rmcp::transport::TokioChildProcess;
use rmcp::{RoleClient, ServiceExt};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Command;

fn enabled() -> bool {
    std::env::var("KITE_MCP_CONFORMANCE").as_deref() == Ok("1")
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rscript(args: &[&str], env: &[(&str, &str)]) -> std::process::Output {
    let program = std::env::var("KITE_MCP_RSCRIPT").unwrap_or_else(|_| "Rscript".into());
    let mut c = Command::new(program);
    c.arg("--vanilla").args(args);
    for (k, v) in env {
        c.env(k, v);
    }
    let out = c.output().expect("run Rscript");
    if !out.status.success() {
        panic!(
            "Rscript {:?} failed:\n{}\n{}",
            args,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    out
}

type Client = RunningService<RoleClient, ()>;

async fn start(args: &[String]) -> Client {
    let mut cmd = tokio::process::Command::new(env!("CARGO_BIN_EXE_kite-mcp"));
    cmd.arg("serve").args(args);
    ().serve(TokioChildProcess::new(cmd).expect("spawn kite-mcp"))
        .await
        .expect("MCP handshake")
}

/// (is_error, structured content)
async fn call(c: &Client, tool: &str, args: Value) -> (bool, Value) {
    let Value::Object(map) = args else { panic!() };
    let r = c
        .call_tool(CallToolRequestParams::new(tool.to_string()).with_arguments(map))
        .await
        .expect("tools/call");
    (
        r.is_error == Some(true),
        r.structured_content.expect("structured content"),
    )
}

async fn read(c: &Client, uri: &str) -> String {
    let r = c
        .read_resource(ReadResourceRequestParams::new(uri))
        .await
        .expect("resources/read");
    match &r.contents[0] {
        ResourceContents::TextResourceContents { text, .. } => text.clone(),
        other => panic!("not text: {other:?}"),
    }
}

/// Bits of a number as the server writes it to clients.
fn client_bits(v: &Value) -> String {
    match v {
        Value::Number(n) => f64_to_hex(n.as_f64().unwrap()),
        Value::String(s) if s == "NA" => "NA".into(),
        Value::String(s) if s == "NaN" => "NaN".into(),
        Value::String(s) if s == "Inf" => f64_to_hex(f64::INFINITY),
        Value::String(s) if s == "-Inf" => f64_to_hex(f64::NEG_INFINITY),
        other => panic!("not a number: {other}"),
    }
}

/// The same label for a reference bit pattern.
fn ref_bits(hex: &str) -> String {
    let x = hex_to_f64(hex).unwrap();
    if is_r_na(x) {
        "NA".into()
    } else if x.is_nan() {
        "NaN".into()
    } else {
        hex.into()
    }
}

fn ref_scalar_bits(v: &Value) -> Option<String> {
    v.get("double").map(|h| ref_bits(h.as_str().unwrap()))
}

fn compare_info(case: &str, server: &Value, reference: &Value) {
    assert_eq!(
        server["convergence"],
        reference["convergence"]
            .get("logical")
            .cloned()
            .unwrap_or(Value::Null),
        "{case}: convergence"
    );
    assert_eq!(
        server["iterations"],
        reference["iterations"]
            .get("integer")
            .cloned()
            .unwrap_or(Value::Null),
        "{case}: iterations"
    );
    assert_eq!(
        Some(client_bits(&server["criterion"])),
        ref_scalar_bits(&reference["criterion"]),
        "{case}: criterion"
    );
    let (sa, ra) = (&server["accounting"], &reference["accounting"]);
    assert_eq!(
        sa["ok"],
        ra["ok"].get("logical").cloned().unwrap_or(Value::Null),
        "{case}: accounting.ok"
    );
    for f in ["rule", "reason"] {
        assert_eq!(
            sa[f],
            ra[f].get("string").cloned().unwrap_or(Value::Null),
            "{case}: accounting.{f}"
        );
    }
    for f in [
        "world_trade_balance",
        "baseline_world_trade_balance",
        "world_value_added",
        "tolerance",
    ] {
        let s = if sa[f].is_null() {
            None
        } else {
            Some(client_bits(&sa[f]))
        };
        assert_eq!(s, ref_scalar_bits(&ra[f]), "{case}: accounting.{f}");
    }
}

/// Compare one table returned by get_table with the reference, cell by cell.
fn compare_table(case: &str, name: &str, got: &Value, reference: &Value) -> usize {
    let rows = got["rows"].as_array().unwrap();
    let cols = reference["columns"].as_array().unwrap();
    let n = cols[0]["values"].as_array().unwrap().len();
    assert_eq!(
        got["total_rows"].as_u64().unwrap() as usize,
        n,
        "{case}/{name}: row count"
    );
    assert_eq!(rows.len(), n, "{case}/{name}: rows returned");
    let mut cells = 0;
    for col in cols {
        let cname = col["name"].as_str().unwrap();
        let ty = col["type"].as_str().unwrap();
        for (i, rv) in col["values"].as_array().unwrap().iter().enumerate() {
            let gv = &rows[i][cname];
            match ty {
                "double" => assert_eq!(
                    client_bits(gv),
                    ref_bits(rv.as_str().unwrap()),
                    "{case}/{name} row {i} column {cname}"
                ),
                _ => assert_eq!(gv, rv, "{case}/{name} row {i} column {cname}"),
            }
            cells += 1;
        }
    }
    cells
}

struct Case {
    id: &'static str,
    reference: &'static str,
    input: Value,
    converged: bool,
}

fn tariff_war() -> Value {
    json!([
        {"origin": "c1", "destination": "c2", "value": 1.2},
        {"origin": "c2", "destination": "c1", "value": 1.2}
    ])
}

fn carbon_club() -> Value {
    json!({"carbon_tax": 0.5, "countries_climate_club": ["c1", "c3"],
           "scenario_carbon_tariff": true, "scenario_export_rebate": true})
}

fn cases() -> Vec<Case> {
    let gen_id = "synthetic_c5_s3_seed7";
    vec![
        Case {
            id: "example_cp",
            reference: "example_cp",
            converged: true,
            input: json!({"model": "caliendo_parro_2015", "scenario": {"tariff_new": tariff_war()}}),
        },
        Case {
            id: "example_chkw",
            reference: "example_chkw",
            converged: true,
            input: json!({"model": "chowdhry_hinz_kamin_wanner_2022", "scenario": {"tariff_new": tariff_war()}}),
        },
        Case {
            id: "example_mw",
            reference: "example_mw",
            converged: true,
            input: json!({"model": "mahlkow_wanner_2021", "scenario": carbon_club()}),
        },
        Case {
            id: "gen5x3_cp",
            reference: "gen5x3_cp",
            converged: true,
            input: json!({"model": "caliendo_parro_2015", "baseline": gen_id, "scenario": {"tariff_new": tariff_war()}}),
        },
        Case {
            id: "gen5x3_chkw",
            reference: "gen5x3_chkw",
            converged: true,
            input: json!({"model": "chowdhry_hinz_kamin_wanner_2022", "baseline": gen_id, "scenario": {"tariff_new": tariff_war()}}),
        },
        Case {
            id: "gen5x3_mw",
            reference: "gen5x3_mw",
            converged: true,
            input: json!({"model": "mahlkow_wanner_2021", "baseline": gen_id, "scenario": carbon_club()}),
        },
        Case {
            id: "sector_tariff_cp",
            reference: "sector_tariff_cp",
            converged: true,
            input: json!({"model": "caliendo_parro_2015", "scenario": {"tariff_new": [{"destination": "c1", "sector": "s1", "value": 1.25}]}}),
        },
        Case {
            id: "nonconv_cp",
            reference: "nonconv_cp",
            converged: false,
            input: json!({"model": "caliendo_parro_2015", "scenario": {"tariff_new": tariff_war()},
                          "settings": {"trade_balance_rule": "fixed_country_share"}}),
        },
        Case {
            id: "unread_chkw",
            reference: "unread_chkw",
            converged: true,
            input: json!({"model": "chowdhry_hinz_kamin_wanner_2022", "allow_unread_variables": true,
                          "scenario": {"tariff_new": tariff_war(), "carbon_tax": 0.5}}),
        },
        Case {
            id: "awkward_literal_mw",
            reference: "awkward_literal_mw",
            converged: true,
            input: json!({"model": "mahlkow_wanner_2021", "scenario": {"carbon_tax": 0.00648016,
                "countries_climate_club": ["c1", "c3"], "scenario_carbon_tariff": true, "scenario_export_rebate": true}}),
        },
        Case {
            id: "csv_cp",
            reference: "example_cp",
            converged: true,
            input: json!({"model": "caliendo_parro_2015", "baseline": "csv_example", "scenario": {"tariff_new": tariff_war()}}),
        },
    ]
}

fn condition_classes(v: &Value) -> Vec<Vec<String>> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|w| {
            w["class"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c.as_str().unwrap().to_string())
                .collect()
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn every_table_equals_a_direct_kite_call_and_every_script_reproduces_it() {
    if !enabled() {
        eprintln!("skipped: set KITE_MCP_CONFORMANCE=1 to run (needs R and KITE)");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let ref_path = tmp.path().join("reference.json");
    let csv_dir = tmp.path().join("csv_example");
    let script = repo().join("tests/conformance/reference.R");
    rscript(
        &[
            script.to_str().unwrap(),
            ref_path.to_str().unwrap(),
            csv_dir.to_str().unwrap(),
        ],
        &[],
    );
    let reference: Value =
        serde_json::from_str(&std::fs::read_to_string(&ref_path).unwrap()).unwrap();

    let client = start(&[format!("--baseline-dir=csv_example={}", csv_dir.display())]).await;

    let (err, generated) = call(
        &client,
        "generate_baseline",
        json!({"countries": 5, "sectors": 3, "seed": 7}),
    )
    .await;
    assert!(!err, "{generated}");
    assert_eq!(generated["countries"].as_array().unwrap().len(), 5);
    assert_eq!(generated["synthetic"], true);
    let (_, csv) = call(
        &client,
        "describe_baseline",
        json!({"baseline": "csv_example"}),
    )
    .await;
    assert_eq!(csv["synthetic"], false);

    let mut total_cells = 0;
    let mut total_tables = 0;
    for case in cases() {
        let r = &reference[case.reference];
        let (err, out) = call(&client, "run_scenario", case.input.clone()).await;
        assert!(!err, "{}: {out}", case.id);
        assert_eq!(
            out["status"],
            if case.converged {
                "converged"
            } else {
                "not_converged"
            },
            "{}",
            case.id
        );
        compare_info(case.id, &out["info"], &r["info"]);

        // the result contract on unconverged runs: no headline, a reason
        if case.converged {
            assert!(out.get("headline").is_some(), "{}: headline", case.id);
            assert!(out.get("not_converged").is_none());
        } else {
            assert!(
                out.get("headline").is_none(),
                "{}: an unconverged run has no headline",
                case.id
            );
            assert!(
                out["not_converged"]
                    .as_str()
                    .unwrap()
                    .contains("No result is reported")
            );
        }

        // synthetic baselines carry the note; the CSV baseline does not
        assert_eq!(
            out.get("baseline_note").is_some(),
            case.id != "csv_cp",
            "{}: baseline note",
            case.id
        );

        // warnings: same classes and messages as the direct call, none dropped
        let ref_w = r.get("warnings").cloned().unwrap_or(json!([]));
        assert_eq!(
            condition_classes(&out["warnings"]),
            condition_classes(&ref_w),
            "{}: warning classes",
            case.id
        );
        for (a, b) in out["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .zip(ref_w.as_array().unwrap())
        {
            assert_eq!(a["message"], b["message"], "{}: warning message", case.id);
        }

        // every table, every row
        let ref_tables = r["tables"].as_object().unwrap();
        let mut names: Vec<&str> = out["tables"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap())
            .collect();
        names.sort();
        let mut ref_names: Vec<&str> = ref_tables.keys().map(|k| k.as_str()).collect();
        ref_names.sort();
        assert_eq!(names, ref_names, "{}: table names", case.id);
        let run_id = out["run_id"].as_str().unwrap().to_string();
        for (name, rt) in ref_tables {
            let (err, got) = call(
                &client,
                "get_table",
                json!({"run_id": run_id, "table": name, "limit": 10000}),
            )
            .await;
            assert!(!err, "{}/{name}: {got}", case.id);
            if !case.converged {
                assert_eq!(got["status"], "not_converged");
                assert!(
                    got["rows"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .all(|row| row["unconverged"] == true),
                    "{}/{name}: every row flagged",
                    case.id
                );
            }
            total_cells += compare_table(case.id, name, &got, rt);
            total_tables += 1;
        }

        // the headline is welfare_change exactly, with a derived percentage
        if case.converged {
            let wc = &ref_tables["welfare_change"]["columns"];
            let values = wc
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["name"] == "value")
                .unwrap()["values"]
                .as_array()
                .unwrap();
            for (row, v) in out["headline"]["by_country"]
                .as_array()
                .unwrap()
                .iter()
                .zip(values)
            {
                assert_eq!(client_bits(&row["welfare_change"]), v.as_str().unwrap());
                let x = hex_to_f64(v.as_str().unwrap()).unwrap();
                assert_eq!(
                    client_bits(&row["percent_change_derived"]),
                    f64_to_hex((x - 1.0) * 100.0)
                );
            }
        }

        // the reproduce script, run in a fresh R session, gives the same bits
        let script = read(&client, &format!("kite://runs/{run_id}/reproduce.R")).await;
        assert_eq!(script, out["reproduce"]["script"].as_str().unwrap());
        if case.id == "csv_cp" {
            assert!(
                script.contains("as.numeric(c(\"0x"),
                "the CSV script writes its tables as hexadecimal literals"
            );
            assert!(script.contains("data from CSV files"));
        }
        if case.id == "awkward_literal_mw" {
            assert!(
                script.contains("carbon_tax <- 0x1.a8af0b45d9531p-8"),
                "{script}"
            );
        } else {
            assert!(
                !script.contains("hexadecimal literals"),
                "{}: unexpected hex literal",
                case.id
            );
        }
        let sp = tmp.path().join(format!("{}.R", case.id));
        std::fs::write(&sp, &script).unwrap();
        let rds = tmp.path().join(format!("{}.rds", case.id));
        rscript(
            &[sp.to_str().unwrap()],
            &[("KITE_MCP_REPRODUCE_OUT", rds.to_str().unwrap())],
        );
        let rj = tmp.path().join(format!("{}.json", case.id));
        let conv = repo().join("tests/conformance/rds_to_json.R");
        rscript(
            &[
                conv.to_str().unwrap(),
                rds.to_str().unwrap(),
                rj.to_str().unwrap(),
            ],
            &[],
        );
        let reproduced: Value =
            serde_json::from_str(&std::fs::read_to_string(&rj).unwrap()).unwrap();
        assert_eq!(
            reproduced["info"]["convergence"], r["info"]["convergence"],
            "{}: reproduced convergence",
            case.id
        );
        assert_eq!(
            reproduced["info"]["criterion"], r["info"]["criterion"],
            "{}: reproduced criterion",
            case.id
        );
        assert_eq!(
            reproduced["info"]["iterations"], r["info"]["iterations"],
            "{}: reproduced iterations",
            case.id
        );
        assert_eq!(
            &reproduced["tables"], &r["tables"],
            "{}: reproduced tables",
            case.id
        );
        eprintln!(
            "{}: {} tables identical through MCP and through the reproduce script",
            case.id,
            ref_tables.len()
        );
    }
    eprintln!("conformance: {total_tables} tables, {total_cells} cells compared, all identical");
    client.cancel().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_variable_the_model_does_not_read_is_refused_by_default() {
    if !enabled() {
        return;
    }
    let client = start(&[]).await;
    let (err, out) = call(
        &client,
        "run_scenario",
        json!({"model": "chowdhry_hinz_kamin_wanner_2022", "scenario": {"tariff_new": tariff_war(), "carbon_tax": 0.5}}),
    )
    .await;
    assert!(err);
    assert_eq!(out["error"], "validation_failed");
    assert_eq!(out["errors"][0]["field"], "scenario.carbon_tax");
    assert!(
        out["errors"][0]["message"]
            .as_str()
            .unwrap()
            .contains("mahlkow_wanner_2021")
    );
    client.cancel().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn compare_runs_refuses_an_unconverged_run_and_differences_converged_ones() {
    if !enabled() {
        return;
    }
    let client = start(&[]).await;
    let (_, a) = call(
        &client,
        "run_scenario",
        json!({"model": "caliendo_parro_2015", "scenario": {"tariff_new": tariff_war()}}),
    )
    .await;
    let (_, b) = call(
        &client,
        "run_scenario",
        json!({"model": "caliendo_parro_2015", "scenario": {"tariff_new": [{"destination": "c1", "sector": "s1", "value": 1.25}]}}),
    )
    .await;
    let (_, n) = call(
        &client,
        "run_scenario",
        json!({"model": "caliendo_parro_2015", "scenario": {"tariff_new": tariff_war()}, "settings": {"trade_balance_rule": "fixed_country_share"}}),
    )
    .await;
    let (err, cmp) = call(
        &client,
        "compare_runs",
        json!({"run_a": a["run_id"], "run_b": b["run_id"], "table": "welfare_change"}),
    )
    .await;
    assert!(!err, "{cmp}");
    let (_, ta) = call(
        &client,
        "get_table",
        json!({"run_id": a["run_id"], "table": "welfare_change"}),
    )
    .await;
    let (_, tb) = call(
        &client,
        "get_table",
        json!({"run_id": b["run_id"], "table": "welfare_change"}),
    )
    .await;
    for (i, row) in cmp["rows"].as_array().unwrap().iter().enumerate() {
        assert_eq!(row["value_a"], ta["rows"][i]["value"]);
        assert_eq!(row["value_b"], tb["rows"][i]["value"]);
        let d = tb["rows"][i]["value"].as_f64().unwrap() - ta["rows"][i]["value"].as_f64().unwrap();
        assert_eq!(client_bits(&row["difference_derived"]), f64_to_hex(d));
    }
    let (err, refused) = call(
        &client,
        "compare_runs",
        json!({"run_a": a["run_id"], "run_b": n["run_id"], "table": "welfare_change"}),
    )
    .await;
    assert!(err);
    assert_eq!(refused["error"], "not_converged");
    assert!(refused.get("rows").is_none());
    client.cancel().await.unwrap();
}

#[test]
fn r_reads_rust_shortest_decimals_and_hex_literals_to_the_same_bits() {
    if !enabled() {
        return;
    }
    // literals of the kind a scenario holds (short decimals across exponents),
    // and arbitrary finite doubles
    let mut lines = vec!["decimal,hex,hexlit".to_string()];
    let mut state: u64 = 0x1234_5678_9abc_def1;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for i in 0..20_000 {
        let x = if i % 2 == 0 {
            let mantissa = (next() % 999_999 + 1) as f64;
            let exp = (next() % 19) as i32 - 12;
            format!("{mantissa}e{exp}").parse::<f64>().unwrap()
        } else {
            loop {
                let y = f64::from_bits(next());
                if y.is_finite() {
                    break y;
                }
            }
        };
        lines.push(format!(
            "{},{},{}",
            shortest_decimal(x),
            f64_to_hex(x),
            r_hex_literal(x)
        ));
    }
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("pairs.csv");
    std::fs::write(&p, lines.join("\n")).unwrap();
    let check = repo().join("tests/conformance/parse_check.R");
    let out = rscript(&[check.to_str().unwrap(), p.to_str().unwrap()], &[]);
    eprint!("{}", String::from_utf8_lossy(&out.stdout));
}
