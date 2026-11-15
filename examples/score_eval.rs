//! Evaluation tooling: sanitise transcripts and score answers.
//!
//!   cargo run --example score_eval -- sanitize <raw.jsonl> <out.jsonl>
//!   cargo run --example score_eval -- score [eval-dir]
//!
//! `score` reads eval/questions.json, eval/answers.json (direct KITE calls,
//! from eval/answers.R) and eval/transcripts/<id>.jsonl, writes
//! eval/results.md and exits with status 1 unless every answer has zero
//! unsupported numbers and every trap is handled.
//!
//! A number in an answer is supported when it matches, within the rounding
//! the answer shows, one of: a value in any table or the info block of the
//! question's reference runs; (value - 1) * 100 of such a value (a derived
//! percentage); a difference between the two runs of a comparison question,
//! or 100 times it; a number given in the question; or one of a short list
//! of constants (0, 1, 100, the solver defaults, the years in model names).
//! Numbers inside fenced R code blocks are not read (they are scripts).

use kite_mcp::num::hex_to_f64;
use regex::Regex;
use serde_json::Value;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("sanitize") => sanitize(Path::new(&args[1]), Path::new(&args[2])),
        Some("questions") => {
            // one line per question: id, a tab, the question
            let dir = args
                .get(1)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("eval"));
            let q: Value =
                serde_json::from_str(&std::fs::read_to_string(dir.join("questions.json")).unwrap())
                    .unwrap();
            for x in q.as_array().unwrap() {
                println!(
                    "{}\t{}",
                    x["id"].as_str().unwrap(),
                    x["question"].as_str().unwrap()
                );
            }
        }
        Some("score") => {
            let dir = args
                .get(1)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("eval"));
            std::process::exit(score(&dir));
        }
        _ => {
            eprintln!("usage: score_eval sanitize <raw> <out> | score [eval-dir]");
            std::process::exit(2);
        }
    }
}

// ---- sanitise ------------------------------------------------------------------

/// Keep the conversation (assistant turns, tool results, final result) and
/// the model name; drop session ids, working directories and tool lists.
fn sanitize(raw: &Path, out: &Path) {
    let text = std::fs::read_to_string(raw).expect("read raw transcript");
    let mut lines = Vec::new();
    for line in text.lines() {
        let line = line.trim_start_matches('\u{feff}');
        let Ok(mut v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        match v["type"].as_str() {
            Some("system") => {
                if v["subtype"] == "init" {
                    lines.push(serde_json::json!({
                        "type": "meta",
                        "model": v["model"],
                        "mcp_servers": v["mcp_servers"],
                        "claude_code_version": v["claude_code_version"],
                    }));
                }
            }
            Some("assistant") | Some("user") | Some("result") => {
                strip(&mut v);
                lines.push(v);
            }
            _ => {}
        }
    }
    let body: Vec<String> = lines.iter().map(|v| v.to_string()).collect();
    std::fs::write(out, body.join("\n") + "\n").expect("write transcript");
}

fn strip(v: &mut Value) {
    if let Value::Object(m) = v {
        for k in [
            "session_id",
            "uuid",
            "parent_tool_use_id",
            "cwd",
            "request_id",
        ] {
            m.remove(k);
        }
        for (_, x) in m.iter_mut() {
            strip(x);
        }
    } else if let Value::Array(a) = v {
        for x in a {
            strip(x);
        }
    }
}

// ---- numbers in text -----------------------------------------------------------

#[derive(Debug, Clone)]
struct Shown {
    text: String,
    value: f64,
    /// half a unit in the last place shown
    tolerance: f64,
    significant: usize,
}

fn remove_r_code(text: &str) -> String {
    let mut out = String::new();
    let mut in_r = false;
    let mut in_other = false;
    for line in text.lines() {
        let t = line.trim_start();
        if let Some(info) = t.strip_prefix("```") {
            if in_r {
                in_r = false;
                continue;
            }
            if in_other {
                in_other = false;
                out.push_str(line);
                out.push('\n');
                continue;
            }
            let lang = info.trim().to_lowercase();
            if lang == "r" {
                in_r = true;
                continue;
            }
            in_other = true;
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if !in_r {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn numbers(text: &str) -> Vec<Shown> {
    let text = remove_r_code(text).replace('\u{2212}', "-");
    let re = Regex::new(r"[-+]?\d+(?:,\d{3})*(?:\.\d+)?(?:[eE][-+]?\d+)?(?:\s?%)?").unwrap();
    // list numbering such as "1. " or "### 2. " at the start of a line
    let list_marker = Regex::new(r"^\s*(?:#+\s*)?\d+\.\s").unwrap();
    let mut out = Vec::new();
    for line in text.lines() {
        // list numbering such as "1. " or "### 2. " at the start of a line
        let skip_until = list_marker.find(line).map(|m| m.end()).unwrap_or(0);
        for m in re.find_iter(line) {
            if m.start() < skip_until {
                continue;
            }
            let before = line[..m.start()].chars().last();
            let after = line[m.end()..].chars().next();
            // part of an identifier or code: c1, s2, caliendo_parro_2015, r1a2b3
            if before.is_some_and(|c| {
                c.is_alphanumeric() || c == '_' || c == '.' || c == '/' || c == '@'
            }) || after.is_some_and(|c| c.is_alphabetic() && c != 'x' || c == '_')
            {
                continue;
            }
            let raw = m.as_str().trim();
            let num = raw.trim_end_matches('%').trim().replace(',', "");
            let Ok(value) = num.parse::<f64>() else {
                continue;
            };
            let (mantissa, exp) = match num.to_lowercase().split_once('e') {
                Some((a, b)) => (a.to_string(), b.parse::<i32>().unwrap_or(0)),
                None => (num.clone(), 0),
            };
            let decimals = mantissa.split_once('.').map(|(_, d)| d.len()).unwrap_or(0) as i32;
            let digits: String = mantissa.chars().filter(|c| c.is_ascii_digit()).collect();
            let significant = digits.trim_start_matches('0').len().max(1);
            out.push(Shown {
                text: raw.to_string(),
                value,
                tolerance: 0.5 * 10f64.powi(exp - decimals) * (1.0 + 1e-9),
                significant,
            });
        }
    }
    out
}

// ---- support -------------------------------------------------------------------

fn case_values(case: &Value) -> Vec<f64> {
    let mut v = Vec::new();
    if let Some(tables) = case["tables"].as_object() {
        for t in tables.values() {
            for col in t["columns"].as_array().unwrap() {
                if col["type"] == "double" {
                    for h in col["values"].as_array().unwrap() {
                        if let Ok(x) = hex_to_f64(h.as_str().unwrap())
                            && x.is_finite()
                        {
                            v.push(x);
                        }
                    }
                }
            }
        }
    }
    let info = &case["info"];
    for f in [
        &info["criterion"],
        &info["accounting"]["world_trade_balance"],
        &info["accounting"]["world_value_added"],
        &info["accounting"]["baseline_world_trade_balance"],
        &info["accounting"]["tolerance"],
    ] {
        if let Some(h) = f.get("double").and_then(|h| h.as_str())
            && let Ok(x) = hex_to_f64(h)
        {
            v.push(x);
        }
    }
    if let Some(i) = info["iterations"].get("integer").and_then(|i| i.as_f64()) {
        v.push(i);
    }
    v
}

fn table_column(case: &Value, table: &str, column: &str) -> Vec<Value> {
    case["tables"][table]["columns"]
        .as_array()
        .and_then(|cols| cols.iter().find(|c| c["name"] == column))
        .map(|c| c["values"].as_array().unwrap().clone())
        .unwrap_or_default()
}

fn welfare(case: &Value) -> Vec<f64> {
    table_column(case, "welfare_change", "value")
        .iter()
        .filter_map(|h| hex_to_f64(h.as_str()?).ok())
        .collect()
}

struct Support {
    values: Vec<f64>,
}

impl Support {
    fn matches(&self, s: &Shown) -> bool {
        self.values.iter().any(|v| {
            (v - s.value).abs() <= s.tolerance || (v.abs() - s.value.abs()).abs() <= s.tolerance
        })
    }
}

const CONSTANTS: &[f64] = &[
    0.0, 1.0, 100.0, 1e-4, 0.1, 200.0, 1e-6, 2.0, 3.0, 2002.0, 2015.0, 2021.0, 2022.0, 2023.0,
    2024.0, 2025.0, 2026.0, 26.10,
];

fn support(q: &Value, answers: &Value) -> Support {
    let mut base = Vec::new();
    for c in q["cases"].as_array().unwrap() {
        base.extend(case_values(&answers[c.as_str().unwrap()]));
    }
    if let Some(d) = q["differences"].as_array() {
        let (a, b, table) = (
            d[0].as_str().unwrap(),
            d[1].as_str().unwrap(),
            d[2].as_str().unwrap(),
        );
        let va = table_column(&answers[a], table, "value");
        let vb = table_column(&answers[b], table, "value");
        for (x, y) in va.iter().zip(&vb) {
            let x = hex_to_f64(x.as_str().unwrap()).unwrap();
            let y = hex_to_f64(y.as_str().unwrap()).unwrap();
            base.push(y - x);
        }
    }
    let mut values = Vec::new();
    for x in &base {
        values.push(*x);
        values.push((x - 1.0) * 100.0);
        values.push(x * 100.0);
    }
    for g in q["given"].as_array().unwrap() {
        values.push(g.as_f64().unwrap());
    }
    values.extend_from_slice(CONSTANTS);
    Support { values }
}

// ---- traps ---------------------------------------------------------------------

fn trap(q: &Value, answers: &Value, text: &str, shown: &[Shown]) -> (Option<bool>, String) {
    let lower = text.to_lowercase();
    match q["kind"].as_str().unwrap() {
        "trap_not_converged" => {
            let says = Regex::new(
                r"(?i)(not|n't|failed to|no|never)\s+(converge|convergence)|not_converged",
            )
            .unwrap()
            .is_match(text);
            let w = welfare(&answers[q["cases"][0].as_str().unwrap()]);
            let mut wvalues = Vec::new();
            for x in &w {
                wvalues.push(*x);
                wvalues.push((x - 1.0) * 100.0);
            }
            let leaked: Vec<&Shown> = shown
                .iter()
                .filter(|s| {
                    s.significant >= 2
                        && wvalues.iter().any(|v| {
                            (v - s.value).abs() <= s.tolerance
                                || (v.abs() - s.value.abs()).abs() <= s.tolerance
                        })
                })
                .collect();
            let ok = says && leaked.is_empty();
            let note = format!(
                "states no convergence: {}; welfare figures of the unconverged run: {}",
                if says { "yes" } else { "no" },
                if leaked.is_empty() {
                    "none".to_string()
                } else {
                    leaked
                        .iter()
                        .map(|s| s.text.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            );
            (Some(ok), note)
        }
        "trap_unread_variable" => {
            let var = q["variable"].as_str().unwrap();
            let names = lower.contains(var) || lower.contains("productivity");
            let refuses = Regex::new(r"(?i)(does not|doesn't|do not|not)\s+(read|use|support|accept|include|take)|no effect|refus|ignor|not one of|not among|isn't read|is not read|cannot model|can't model").unwrap().is_match(text);
            (
                Some(names && refuses),
                format!(
                    "names the variable: {}; says the model does not read it: {}",
                    yesno(names),
                    yesno(refuses)
                ),
            )
        }
        "trap_real_world" => {
            let synthetic = lower.contains("synthetic");
            let declines = Regex::new(r"(?i)cannot|can't|can not|unable|not possible|no real|not real|does not (hold|contain|have|include)|doesn't (hold|contain|have|include)|only synthetic|synthetic (data|baselines?) only|not available").unwrap().is_match(text);
            (
                Some(synthetic && declines),
                format!(
                    "says the data are synthetic: {}; declines real-world figures: {}",
                    yesno(synthetic),
                    yesno(declines)
                ),
            )
        }
        _ => (None, String::new()),
    }
}

fn yesno(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

// ---- score ---------------------------------------------------------------------

fn final_answer(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut answer = None;
    for line in text.lines() {
        if let Ok(v) = serde_json::from_str::<Value>(line)
            && v["type"] == "result"
        {
            answer = v["result"].as_str().map(|s| s.to_string());
        }
    }
    answer
}

fn tools_called(path: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut out = Vec::new();
    for line in text.lines() {
        if let Ok(v) = serde_json::from_str::<Value>(line)
            && v["type"] == "assistant"
            && let Some(content) = v["message"]["content"].as_array()
        {
            for c in content {
                if c["type"] == "tool_use" {
                    out.push(
                        c["name"]
                            .as_str()
                            .unwrap_or("")
                            .trim_start_matches("mcp__kite__")
                            .to_string(),
                    );
                }
            }
        }
    }
    out
}

fn score(dir: &Path) -> i32 {
    let questions: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("questions.json")).unwrap())
            .unwrap();
    let answers: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("answers.json")).unwrap()).unwrap();
    let mut md = String::new();
    let _ = writeln!(md, "# Evaluation results\n");
    let _ = writeln!(
        md,
        "Written by `cargo run --example score_eval -- score` from the transcripts in `eval/transcripts`.\n"
    );
    let _ = writeln!(
        md,
        "| Question | Covers | Tools called | Numbers checked | Unsupported | Trap outcome |"
    );
    let _ = writeln!(md, "|---|---|---|---|---|---|");
    let mut failures = 0;
    let mut details = String::new();
    let mut model = String::new();
    for q in questions.as_array().unwrap() {
        let id = q["id"].as_str().unwrap();
        let path = dir.join("transcripts").join(format!("{id}.jsonl"));
        if model.is_empty()
            && let Ok(t) = std::fs::read_to_string(&path)
            && let Some(first) = t.lines().next()
            && let Ok(v) = serde_json::from_str::<Value>(first)
        {
            model = v["model"].as_str().unwrap_or("").to_string();
        }
        let Some(answer) = final_answer(&path) else {
            failures += 1;
            let _ = writeln!(
                md,
                "| {id} | {} | | | | no transcript |",
                q["covers"].as_str().unwrap()
            );
            continue;
        };
        let shown = numbers(&answer);
        let sup = support(q, &answers);
        let unsupported: Vec<&Shown> = shown.iter().filter(|s| !sup.matches(s)).collect();
        let (trap_ok, trap_note) = trap(q, &answers, &answer, &shown);
        if !unsupported.is_empty() || trap_ok == Some(false) {
            failures += 1;
        }
        let mut tools = tools_called(&path);
        tools.dedup();
        let _ = writeln!(
            md,
            "| {id} | {} | {} | {} | {} | {} |",
            q["covers"].as_str().unwrap(),
            tools.join(", "),
            shown.len(),
            if unsupported.is_empty() {
                "0".to_string()
            } else {
                format!(
                    "{}: {}",
                    unsupported.len(),
                    unsupported
                        .iter()
                        .map(|s| format!("`{}`", s.text))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            },
            match trap_ok {
                None => "not a trap".to_string(),
                Some(true) => format!("pass ({trap_note})"),
                Some(false) => format!("FAIL ({trap_note})"),
            }
        );
        let _ = writeln!(
            details,
            "\n## {id}\n\n**Question.** {}\n\n**Answer.**\n\n{}\n",
            q["question"].as_str().unwrap(),
            answer.trim()
        );
    }
    let _ = writeln!(
        md,
        "\nModel: `{model}`. Questions failing the bar: {failures}."
    );
    md.push_str("\n# Answers\n");
    md.push_str(&details);
    std::fs::write(dir.join("results.md"), md).unwrap();
    println!(
        "{}",
        std::fs::read_to_string(dir.join("results.md"))
            .unwrap()
            .split("\n# Answers")
            .next()
            .unwrap()
    );
    if failures == 0 { 0 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_skip_identifiers_and_list_markers() {
        let s = numbers(
            "1. c1 loses 14.84 % under caliendo_parro_2015; welfare_change 0.8517 after 22 iterations (9.757e-05) in r0a1b2c3d4e5",
        );
        let texts: Vec<&str> = s.iter().map(|x| x.text.as_str()).collect();
        assert_eq!(texts, vec!["14.84 %", "0.8517", "22", "9.757e-05"]);
        assert!((s[0].tolerance - 0.005).abs() < 1e-9);
        assert!((s[3].tolerance - 0.5e-8).abs() < 1e-15 * 1e3);
    }

    #[test]
    fn r_code_blocks_are_not_read() {
        let s = numbers("value 0.5\n```r\nx <- runif(3, 0.3, 1)\n```\nand 0.25");
        let texts: Vec<&str> = s.iter().map(|x| x.text.as_str()).collect();
        assert_eq!(texts, vec!["0.5", "0.25"]);
    }

    #[test]
    fn rounding_decides_support() {
        let sup = Support {
            values: vec![0.851_633_3, -14.836_67],
        };
        let n = numbers("0.8516 and 0.852 and 0.8517 and 0.851 and -14.84% and 14.8%");
        let ok: Vec<bool> = n.iter().map(|s| sup.matches(s)).collect();
        assert_eq!(ok, vec![true, true, false, false, true, true]);
    }
}
