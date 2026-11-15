//! The standalone R script that reproduces a run without kite-mcp.

use crate::baseline::{Baseline, Source};
use crate::num::shortest_decimal;
use crate::protocol::{ColumnData, Named, Table, Value};
use crate::scenario::{BuiltVariable, Recipe};
use std::collections::HashSet;
use std::fmt::Write;

/// The synthetic baseline construction, shared with the worker.
pub const SYNTHETIC_R: &str = include_str!("../worker/synthetic.R");

/// `x` as an R hexadecimal floating-point literal, which R reads exactly.
pub fn r_hex_literal(x: f64) -> String {
    if x.is_nan() {
        return if crate::num::is_r_na(x) {
            "NA_real_".into()
        } else {
            "NaN".into()
        };
    }
    if x.is_infinite() {
        return if x > 0.0 { "Inf".into() } else { "-Inf".into() };
    }
    let bits = x.to_bits();
    let sign = if bits >> 63 == 1 { "-" } else { "" };
    let exp = ((bits >> 52) & 0x7ff) as i64;
    let mant = bits & 0x000f_ffff_ffff_ffff;
    if exp == 0 && mant == 0 {
        return format!("{sign}0x0p+0");
    }
    let (lead, e) = if exp == 0 {
        (0, -1022)
    } else {
        (1, exp - 1023)
    };
    format!("{sign}0x{lead}.{mant:013x}p{e:+}")
}

fn r_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn r_strings(v: &[String]) -> String {
    format!(
        "c({})",
        v.iter().map(|s| r_string(s)).collect::<Vec<_>>().join(", ")
    )
}

/// A double as an R numeric literal: the shortest decimal when R reads it to
/// the same bits (`r_exact` holds those bit patterns, as the worker reported
/// them), otherwise an exact hexadecimal literal.
fn r_num(x: f64, r_exact: &HashSet<u64>) -> String {
    if r_exact.contains(&x.to_bits()) {
        shortest_decimal(x)
    } else {
        r_hex_literal(x)
    }
}

/// Every double written as a literal into the script for this scenario and
/// these settings, to be checked against R's reading of its decimal form.
pub fn literal_values(scenario: &[BuiltVariable], settings: &[Named]) -> Vec<f64> {
    let mut out = Vec::new();
    for v in scenario {
        match &v.recipe {
            Recipe::Table { overrides, .. } => out.extend(overrides.iter().map(|o| o.1)),
            Recipe::Double(x) => out.push(*x),
            _ => {}
        }
    }
    for n in settings {
        if let Value::Double(x) = &n.value {
            out.push(x.0);
        }
    }
    out.retain(|x| x.is_finite());
    out
}

/// A table written out in full; doubles as hexadecimal literals.
fn r_table(t: &Table) -> String {
    let mut cols = Vec::new();
    for c in &t.columns {
        let body = match &c.data {
            ColumnData::String(v) => r_strings(
                &v.iter()
                    .map(|s| s.clone().unwrap_or_default())
                    .collect::<Vec<_>>(),
            ),
            ColumnData::Double(v) => format!(
                "as.numeric(c({}))",
                v.iter()
                    .map(|x| r_string(&r_hex_literal(x.0)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            ColumnData::Integer(v) => format!(
                "c({})",
                v.iter()
                    .map(|x| x.map(|i| format!("{i}L")).unwrap_or("NA_integer_".into()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            ColumnData::Logical(v) => format!(
                "c({})",
                v.iter()
                    .map(|x| match x {
                        Some(true) => "TRUE",
                        Some(false) => "FALSE",
                        None => "NA",
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
        cols.push(format!("    {} = {}", c.name, body));
    }
    format!("data.table::data.table(\n{}\n  )", cols.join(",\n"))
}

pub struct ScriptInput<'a> {
    pub run_id: &'a str,
    pub kite_mcp_version: &'a str,
    pub created_at: &'a str,
    pub kite_version: &'a str,
    pub kite_commit: &'a str,
    pub model: &'a str,
    pub baseline: &'a Baseline,
    pub scenario: &'a [BuiltVariable],
    pub settings: &'a [Named],
    /// Bit patterns whose shortest decimal R reads exactly
    pub r_exact: &'a HashSet<u64>,
}

pub fn script(i: &ScriptInput) -> String {
    let b = i.baseline;
    let mut s = String::new();
    let _ = writeln!(s, "# Reproduce kite-mcp run {} with KITE alone.", i.run_id);
    let _ = writeln!(s, "#");
    let _ = writeln!(
        s,
        "# Written by kite-mcp {} on {}. kite-mcp is an independent, unofficial",
        i.kite_mcp_version, i.created_at
    );
    let _ = writeln!(
        s,
        "# interface to KITE with no affiliation to or endorsement from the Kiel"
    );
    let _ = writeln!(
        s,
        "# Institute for the World Economy. This script does not use kite-mcp: it"
    );
    let _ = writeln!(
        s,
        "# installs KITE at the recorded commit, rebuilds the baseline, runs the"
    );
    let _ = writeln!(s, "# scenario and prints every table of process_results().");
    let _ = writeln!(s, "#");
    let _ = writeln!(s, "# Model:    {}", i.model);
    let _ = writeln!(
        s,
        "# Baseline: {} ({}), hash {}",
        b.id,
        if b.synthetic {
            "synthetic data"
        } else {
            "data from CSV files"
        },
        b.hash
    );
    let _ = writeln!(
        s,
        "# KITE:     {} at commit {}",
        i.kite_version, i.kite_commit
    );
    let _ = writeln!(s, "#");
    let mut hex_values: Vec<f64> = literal_values(i.scenario, i.settings)
        .into_iter()
        .filter(|x| !i.r_exact.contains(&x.to_bits()))
        .collect();
    hex_values.dedup_by_key(|x| x.to_bits());
    if !hex_values.is_empty() {
        let _ = writeln!(
            s,
            "# Some numbers below are hexadecimal literals: R's parser reads their"
        );
        let _ = writeln!(
            s,
            "# shortest decimal form to a neighbouring double, so the exact value is"
        );
        let _ = writeln!(s, "# written instead:");
        for x in &hex_values {
            let _ = writeln!(s, "#   {} is {}", r_hex_literal(*x), shortest_decimal(*x));
        }
        let _ = writeln!(s, "#");
    }
    let _ = writeln!(s, "# Run with: Rscript --vanilla reproduce.R");
    let _ = writeln!(
        s,
        "# Set KITE_MCP_REPRODUCE_OUT to a file path to also save the results with saveRDS()."
    );
    let _ = writeln!(s);
    let _ = writeln!(s, "kite_commit <- {}", r_string(i.kite_commit));
    let _ = writeln!(
        s,
        "installed <- if (requireNamespace(\"KITE\", quietly = TRUE)) utils::packageDescription(\"KITE\")$RemoteSha else NULL"
    );
    let _ = writeln!(s, "if (!identical(installed, kite_commit)) {{");
    let _ = writeln!(
        s,
        "  if (!requireNamespace(\"remotes\", quietly = TRUE)) install.packages(\"remotes\", repos = \"https://cloud.r-project.org\")"
    );
    let _ = writeln!(
        s,
        "  remotes::install_github(paste0(\"julianhinz/KITE@\", kite_commit), upgrade = \"never\")"
    );
    let _ = writeln!(s, "}}");
    let _ = writeln!(s, "suppressPackageStartupMessages({{");
    let _ = writeln!(s, "  library(KITE)");
    let _ = writeln!(s, "  library(data.table)");
    let _ = writeln!(s, "}})");
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "# ---- baseline ----------------------------------------------------------------"
    );
    let _ = writeln!(s);
    let with_carbon = i.model == "mahlkow_wanner_2021";
    match &b.source {
        Source::SyntheticExample | Source::Generated { .. } => {
            let (nc, ns, seed) = match &b.source {
                Source::Generated {
                    countries,
                    sectors,
                    seed,
                } => (*countries, *sectors, *seed),
                _ => (3, 2, 1),
            };
            s.push_str(SYNTHETIC_R);
            let _ = writeln!(s);
            let _ = writeln!(
                s,
                "baseline <- kite_mcp_synthetic_baseline(n_countries = {nc}L, n_sectors = {ns}L, seed = {seed}L)"
            );
            let _ = writeln!(s, "ic <- baseline$initial_conditions");
            if with_carbon {
                let _ = writeln!(s, "ic$price <- baseline$price");
                let _ = writeln!(s, "ic$carbon_intensity <- baseline$carbon_intensity");
            }
        }
        Source::Csv { directory } => {
            let _ = writeln!(
                s,
                "# Read from CSV files in {directory}. Doubles are written as hexadecimal"
            );
            let _ = writeln!(
                s,
                "# literals so that R reads exactly the values kite-mcp passed to KITE."
            );
            let _ = writeln!(s, "ic <- list(");
            let tables = b.tables_for_model(i.model);
            let parts: Vec<String> = tables
                .iter()
                .map(|n| {
                    let Value::Table(t) = &n.value else {
                        unreachable!()
                    };
                    if n.name == "trade_elasticity" {
                        format!("  elasticities = list(trade_elasticity = {})", r_table(t))
                    } else {
                        format!("  {} = {}", n.name, r_table(t))
                    }
                })
                .collect();
            let _ = writeln!(s, "{}", parts.join(",\n"));
            let _ = writeln!(s, ")");
        }
    }
    let _ = writeln!(s, "countries <- {}", r_strings(&b.countries));
    let _ = writeln!(s, "sectors <- {}", r_strings(&b.sectors));
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "# ---- scenario ----------------------------------------------------------------"
    );
    let _ = writeln!(s);
    for v in i.scenario {
        match &v.recipe {
            Recipe::Table {
                dims,
                base,
                overrides,
            } => {
                match base {
                    Some(t) => {
                        let _ = writeln!(s, "{} <- data.table::copy(ic${t})", v.name);
                    }
                    None => {
                        let grid: Vec<String> = dims
                            .iter()
                            .map(|d| {
                                format!(
                                    "{d} = {}",
                                    if *d == "sector" {
                                        "sectors"
                                    } else {
                                        "countries"
                                    }
                                )
                            })
                            .collect();
                        let _ = writeln!(
                            s,
                            "{} <- data.table::CJ({}, sorted = FALSE)[, value := 1][]",
                            v.name,
                            grid.join(", ")
                        );
                    }
                }
                for (filters, value, exclude_diagonal) in overrides {
                    let mut conds: Vec<String> = filters
                        .iter()
                        .map(|(d, c)| format!("{d} == {}", r_string(c)))
                        .collect();
                    if *exclude_diagonal {
                        conds.push("origin != destination".into());
                    }
                    let _ = writeln!(
                        s,
                        "{}[{}, value := {}]",
                        v.name,
                        conds.join(" & "),
                        r_num(*value, i.r_exact)
                    );
                }
            }
            Recipe::Double(x) => {
                let _ = writeln!(s, "{} <- {}", v.name, r_num(*x, i.r_exact));
            }
            Recipe::Strings(list) => {
                let _ = writeln!(s, "{} <- {}", v.name, r_strings(list));
            }
            Recipe::Logical(x) => {
                let _ = writeln!(s, "{} <- {}", v.name, if *x { "TRUE" } else { "FALSE" });
            }
            Recipe::Members(list) => {
                let _ = writeln!(
                    s,
                    "{} <- data.table::data.table(country = countries, value = as.integer(countries %in% {}))",
                    v.name,
                    r_strings(list)
                );
            }
        }
    }
    let names: Vec<String> = i
        .scenario
        .iter()
        .map(|v| format!("{} = {}", v.name, v.name))
        .collect();
    let _ = writeln!(s, "model_scenario <- list({})", names.join(", "));
    let settings: Vec<String> = i
        .settings
        .iter()
        .map(|n| {
            let v = match &n.value {
                Value::Integer(x) => format!("{x}L"),
                Value::Double(x) => r_num(x.0, i.r_exact),
                Value::String(x) => r_string(x),
                Value::Logical(x) => {
                    if *x {
                        "TRUE".into()
                    } else {
                        "FALSE".into()
                    }
                }
                other => format!("{other:?}"),
            };
            format!("{} = {}", n.name, v)
        })
        .collect();
    let _ = writeln!(s, "settings <- list({})", settings.join(", "));
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "# ---- run -----------------------------------------------------------------------"
    );
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "result <- update_equilibrium(model = {}, initial_conditions = ic,",
        i.model
    );
    let _ = writeln!(
        s,
        "                             model_scenario = model_scenario, settings = settings)"
    );
    let _ = writeln!(s, "processed <- process_results(result)");
    let _ = writeln!(s);
    let _ = writeln!(s, "info <- result$info");
    let _ = writeln!(
        s,
        "cat(\"convergence:\", format(info$convergence), \"| iterations:\", info$iterations,"
    );
    let _ = writeln!(
        s,
        "    \"| criterion:\", format(info$criterion, digits = 17), \"\\n\")"
    );
    let _ = writeln!(
        s,
        "cat(\"accounting:\", format(info$accounting$ok), \"-\", info$accounting$reason, \"\\n\")"
    );
    let _ = writeln!(s, "for (name in names(processed$output)) {{");
    let _ = writeln!(s, "  cat(\"\\n==\", name, \"==\\n\")");
    let _ = writeln!(
        s,
        "  print(processed$output[[name]], digits = 17, nrows = Inf)"
    );
    let _ = writeln!(s, "}}");
    let _ = writeln!(s);
    let _ = writeln!(s, "out <- Sys.getenv(\"KITE_MCP_REPRODUCE_OUT\")");
    let _ = writeln!(
        s,
        "if (nzchar(out)) saveRDS(list(info = info, output = processed$output), out)"
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_literals_follow_r_syntax() {
        assert_eq!(r_hex_literal(1.0), "0x1.0000000000000p+0");
        assert_eq!(r_hex_literal(1.2), "0x1.3333333333333p+0");
        assert_eq!(r_hex_literal(-0.5), "-0x1.0000000000000p-1");
        assert_eq!(r_hex_literal(0.0), "0x0p+0");
        assert_eq!(r_hex_literal(-0.0), "-0x0p+0");
        assert_eq!(r_hex_literal(5e-324), "0x0.0000000000001p-1022");
        assert_eq!(
            r_hex_literal(f64::from_bits(0x7ff0_0000_0000_07a2)),
            "NA_real_"
        );
    }

    #[test]
    fn strings_are_escaped() {
        assert_eq!(r_string("a\"b\\c"), "\"a\\\"b\\\\c\"");
    }
}
