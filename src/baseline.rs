//! Baselines: KITE's initial conditions as typed tables.
//!
//! Three sources: the synthetic economy of KITE's example, synthetic
//! economies of other sizes built by the same code, and directories of CSV
//! files. All input formats are read in this module; a reader for KITE's
//! planned `.ice` files (KITE issue #19, format not yet published) would be
//! added here.

use crate::protocol::{ColumnData, Named, Table, Value};
use schemars::JsonSchema;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Need {
    Required,
    Optional,
    /// Read by mahlkow_wanner_2021 only. kite-mcp passes these tables to that
    /// model alone, as KITE's example does (inst/examples/example.R:127-130).
    CarbonOnly,
}

pub struct TableSpec {
    pub name: &'static str,
    pub dims: &'static [&'static str],
    pub need: Need,
    pub integer: bool,
    pub description: &'static str,
}

const D3: &[&str] = &["origin", "destination", "sector"];

/// The initial-condition tables, in the order KITE's example builds them.
pub const TABLES: &[TableSpec] = &[
    TableSpec {
        name: "trade_share",
        dims: D3,
        need: Need::Required,
        integer: false,
        description: "Share of the destination's expenditure on the sector bought from the origin; sums to 1 over origin for each destination and sector.",
    },
    TableSpec {
        name: "intermediate_share",
        dims: &["country", "input", "output"],
        need: Need::Required,
        integer: false,
        description: "Share of the input sector in the intermediate inputs of the output sector; sums to 1 over input for each country and output sector.",
    },
    TableSpec {
        name: "factor_share",
        dims: &["country", "sector"],
        need: Need::Required,
        integer: false,
        description: "Share of value added (labour) in gross output, between 0 and 1.",
    },
    TableSpec {
        name: "consumption_share",
        dims: &["country", "sector"],
        need: Need::Required,
        integer: false,
        description: "Share of the sector in final consumption; sums to 1 over sector for each country.",
    },
    TableSpec {
        name: "expenditure",
        dims: &["destination", "sector"],
        need: Need::Required,
        integer: false,
        description: "Expenditure on the sector's goods in the destination.",
    },
    TableSpec {
        name: "value_added",
        dims: &["country"],
        need: Need::Required,
        integer: false,
        description: "Value added (labour income), greater than 0.",
    },
    TableSpec {
        name: "trade_balance",
        dims: &["country"],
        need: Need::Required,
        integer: false,
        description: "Trade balance; the world total should be 0.",
    },
    TableSpec {
        name: "trade_elasticity",
        dims: &["sector"],
        need: Need::Required,
        integer: false,
        description: "Trade elasticity theta, the positive Frechet parameter (README.md, Quick start).",
    },
    TableSpec {
        name: "tariff",
        dims: D3,
        need: Need::Optional,
        integer: false,
        description: "Baseline gross tariff factor (1 = none). KITE assumes 1 when absent.",
    },
    TableSpec {
        name: "ntb",
        dims: D3,
        need: Need::Optional,
        integer: false,
        description: "Baseline gross non-tariff trade cost factor. KITE assumes 1 when absent.",
    },
    TableSpec {
        name: "export_subsidy",
        dims: D3,
        need: Need::Optional,
        integer: false,
        description: "Baseline gross export subsidy factor. KITE assumes 1 when absent.",
    },
    TableSpec {
        name: "coalition_member",
        dims: &["country"],
        need: Need::Optional,
        integer: true,
        description: "Coalition membership for chowdhry_hinz_kamin_wanner_2022, 1 or 0 by country.",
    },
    TableSpec {
        name: "price",
        dims: &["country", "sector"],
        need: Need::CarbonOnly,
        integer: false,
        description: "Initial price levels, needed for a carbon tax in mahlkow_wanner_2021.",
    },
    TableSpec {
        name: "carbon_intensity",
        dims: &["sector"],
        need: Need::CarbonOnly,
        integer: false,
        description: "Carbon released by burning one unit of the sector's goods, for mahlkow_wanner_2021.",
    },
    TableSpec {
        name: "tax",
        dims: &["country", "sector"],
        need: Need::CarbonOnly,
        integer: false,
        description: "Baseline gross tax wedge for mahlkow_wanner_2021 (1 = none).",
    },
];

pub fn spec(name: &str) -> Option<&'static TableSpec> {
    TABLES.iter().find(|t| t.name == name)
}

#[derive(Clone, Debug, Serialize, JsonSchema, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Source {
    /// The economy of KITE's inst/examples/example.R, seed 1
    SyntheticExample,
    /// Built by generate_baseline with the example's construction
    Generated {
        countries: usize,
        sectors: usize,
        seed: i64,
    },
    /// Read from a directory of CSV files given to the server at start-up
    Csv { directory: String },
}

#[derive(Clone, Debug)]
pub struct Baseline {
    pub id: String,
    pub source: Source,
    pub synthetic: bool,
    pub countries: Vec<String>,
    pub sectors: Vec<String>,
    /// Tables in KITE's order
    pub tables: Vec<Named>,
    pub hash: String,
    /// Findings of validation that do not stop a run
    pub notes: Vec<String>,
}

#[derive(Debug, Error, PartialEq)]
pub enum BaselineError {
    #[error("baseline table `{table}` is missing; required tables: {required}")]
    Missing { table: String, required: String },
    #[error("baseline table `{0}` is not a table kite-mcp knows; known tables: {1}")]
    Unknown(String, String),
    #[error("baseline table `{table}`: {message}")]
    Invalid { table: String, message: String },
    #[error("cannot read `{path}`: {message}")]
    Io { path: String, message: String },
}

fn invalid(table: &str, message: impl Into<String>) -> BaselineError {
    BaselineError::Invalid {
        table: table.to_string(),
        message: message.into(),
    }
}

fn hash_tables(tables: &[Named]) -> String {
    let bytes = serde_json::to_vec(tables).expect("tables serialise");
    let digest = Sha256::digest(&bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn table_of<'a>(tables: &'a [Named], name: &str) -> Option<&'a Table> {
    tables.iter().find_map(|n| match (&n.name, &n.value) {
        (k, Value::Table(t)) if k == name => Some(t),
        _ => None,
    })
}

/// Distinct values of a string column, in order of first appearance.
fn codes(t: &Table, column: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    if let Some(v) = t.strings(column) {
        for s in v.iter().flatten() {
            if seen.insert(s.clone()) {
                out.push(s.clone());
            }
        }
    }
    out
}

impl Baseline {
    /// Check `tables` against the table specifications and build a baseline.
    /// Tables are put in KITE's order.
    pub fn new(id: String, source: Source, tables: Vec<Named>) -> Result<Baseline, BaselineError> {
        let known: Vec<&str> = TABLES.iter().map(|t| t.name).collect();
        for n in &tables {
            if spec(&n.name).is_none() {
                return Err(BaselineError::Unknown(n.name.clone(), known.join(", ")));
            }
            if !matches!(n.value, Value::Table(_)) {
                return Err(invalid(&n.name, "must be a table"));
            }
        }
        let mut ordered = Vec::new();
        for s in TABLES {
            match tables.iter().find(|n| n.name == s.name) {
                Some(n) => ordered.push(n.clone()),
                None if s.need == Need::Required => {
                    let required: Vec<&str> = TABLES
                        .iter()
                        .filter(|t| t.need == Need::Required)
                        .map(|t| t.name)
                        .collect();
                    return Err(BaselineError::Missing {
                        table: s.name.to_string(),
                        required: required.join(", "),
                    });
                }
                None => {}
            }
        }

        for n in &ordered {
            let Value::Table(t) = &n.value else {
                unreachable!()
            };
            check_columns(spec(&n.name).unwrap(), t)?;
        }
        let countries = codes(table_of(&ordered, "value_added").unwrap(), "country");
        let sectors = codes(table_of(&ordered, "trade_elasticity").unwrap(), "sector");
        if countries.len() < 2 {
            return Err(invalid("value_added", "needs at least 2 countries"));
        }
        if sectors.is_empty() {
            return Err(invalid("trade_elasticity", "needs at least 1 sector"));
        }
        let mut notes = Vec::new();
        for n in &ordered {
            let Value::Table(t) = &n.value else {
                unreachable!()
            };
            check_table(spec(&n.name).unwrap(), t, &countries, &sectors, &mut notes)?;
        }
        let synthetic = !matches!(source, Source::Csv { .. });
        let hash = hash_tables(&ordered);
        Ok(Baseline {
            id,
            source,
            synthetic,
            countries,
            sectors,
            tables: ordered,
            hash,
            notes,
        })
    }

    pub fn table(&self, name: &str) -> Option<&Table> {
        table_of(&self.tables, name)
    }

    pub fn has(&self, name: &str) -> bool {
        self.table(name).is_some()
    }

    /// Initial conditions for one model, in KITE's order.
    pub fn tables_for_model(&self, model: &str) -> Vec<Named> {
        self.tables
            .iter()
            .filter(|n| {
                model == "mahlkow_wanner_2021"
                    || spec(&n.name).map(|s| s.need) != Some(Need::CarbonOnly)
            })
            .cloned()
            .collect()
    }

    pub fn table_names(&self) -> Vec<String> {
        self.tables.iter().map(|n| n.name.clone()).collect()
    }
}

fn check_columns(spec: &TableSpec, t: &Table) -> Result<(), BaselineError> {
    let name = spec.name;
    let expected: Vec<String> = spec
        .dims
        .iter()
        .map(|d| d.to_string())
        .chain(std::iter::once("value".to_string()))
        .collect();
    let actual: Vec<String> = t.columns.iter().map(|c| c.name.clone()).collect();
    if actual != expected {
        return Err(invalid(
            name,
            format!(
                "columns are [{}]; expected [{}] in that order",
                actual.join(", "),
                expected.join(", ")
            ),
        ));
    }
    for d in spec.dims {
        if t.strings(d).is_none() {
            return Err(invalid(
                name,
                format!("column `{d}` must hold codes (text)"),
            ));
        }
    }
    Ok(())
}

fn check_table(
    spec: &TableSpec,
    t: &Table,
    countries: &[String],
    sectors: &[String],
    notes: &mut Vec<String>,
) -> Result<(), BaselineError> {
    let name = spec.name;
    let values: Vec<f64> = match &t.column("value").unwrap().data {
        ColumnData::Double(v) => v.iter().map(|x| x.0).collect(),
        ColumnData::Integer(v) if spec.integer => v
            .iter()
            .map(|x| x.map(|i| i as f64).unwrap_or(f64::NAN))
            .collect(),
        _ => {
            return Err(invalid(
                name,
                if spec.integer {
                    "column `value` must hold integers 0 or 1"
                } else {
                    "column `value` must hold numbers"
                },
            ));
        }
    };
    if let Some(i) = values.iter().position(|v| !v.is_finite()) {
        return Err(invalid(
            name,
            format!("row {} has a value that is not a finite number", i + 1),
        ));
    }

    // every dimension holds known codes, and the table covers the full grid once
    let allowed = |d: &str| -> &[String] {
        match d {
            "country" | "origin" | "destination" => countries,
            _ => sectors,
        }
    };
    let mut seen = HashSet::new();
    for i in 0..t.n_rows() {
        let mut key = Vec::new();
        for d in spec.dims {
            let code = t.strings(d).unwrap()[i].clone().unwrap_or_default();
            if !allowed(d).contains(&code) {
                return Err(invalid(
                    name,
                    format!(
                        "row {}: `{d}` is `{code}`, which is not one of [{}]",
                        i + 1,
                        allowed(d).join(", ")
                    ),
                ));
            }
            key.push(code);
        }
        if !seen.insert(key.clone()) {
            return Err(invalid(
                name,
                format!("row {}: ({}) appears twice", i + 1, key.join(", ")),
            ));
        }
    }
    let full: usize = spec.dims.iter().map(|d| allowed(d).len()).product();
    if seen.len() != full {
        return Err(invalid(
            name,
            format!(
                "has {} rows; the full grid over [{}] has {full}",
                seen.len(),
                spec.dims.join(", ")
            ),
        ));
    }

    let rows: Vec<Vec<String>> = (0..t.n_rows())
        .map(|i| {
            spec.dims
                .iter()
                .map(|d| t.strings(d).unwrap()[i].clone().unwrap_or_default())
                .collect()
        })
        .collect();
    let sums_to_one = |group_dims: &[usize], label: &str| -> Result<(), BaselineError> {
        let mut sums: BTreeMap<Vec<String>, f64> = BTreeMap::new();
        for (r, v) in rows.iter().zip(&values) {
            let key: Vec<String> = group_dims.iter().map(|&j| r[j].clone()).collect();
            *sums.entry(key).or_insert(0.0) += v;
        }
        for (k, s) in sums {
            if (s - 1.0).abs() > 1e-9 {
                return Err(invalid(
                    name,
                    format!(
                        "values for ({}) sum to {s} over {label}; they must sum to 1 within 1e-9",
                        k.join(", ")
                    ),
                ));
            }
        }
        Ok(())
    };
    let positive = || -> Result<(), BaselineError> {
        if let Some(i) = values.iter().position(|v| *v <= 0.0) {
            return Err(invalid(
                name,
                format!("row {}: value {} must be greater than 0", i + 1, values[i]),
            ));
        }
        Ok(())
    };
    match name {
        "trade_share" => {
            if values.iter().any(|v| *v < 0.0) {
                return Err(invalid(name, "shares must not be negative"));
            }
            sums_to_one(&[1, 2], "origin")?;
        }
        "intermediate_share" => {
            if values.iter().any(|v| *v < 0.0) {
                return Err(invalid(name, "shares must not be negative"));
            }
            sums_to_one(&[0, 2], "input")?;
        }
        "consumption_share" => {
            if values.iter().any(|v| *v < 0.0) {
                return Err(invalid(name, "shares must not be negative"));
            }
            sums_to_one(&[0], "sector")?;
        }
        "factor_share" => {
            if let Some(i) = values.iter().position(|v| *v <= 0.0 || *v > 1.0) {
                return Err(invalid(
                    name,
                    format!("row {}: {} is outside (0, 1]", i + 1, values[i]),
                ));
            }
        }
        "value_added" | "expenditure" | "trade_elasticity" | "tariff" | "ntb"
        | "export_subsidy" | "price" | "tax" => positive()?,
        "carbon_intensity" => {
            if values.iter().any(|v| *v < 0.0) {
                return Err(invalid(name, "values must not be negative"));
            }
        }
        "coalition_member" => {
            if values.iter().any(|v| *v != 0.0 && *v != 1.0) {
                return Err(invalid(name, "values must be 0 or 1"));
            }
        }
        "trade_balance" => {
            let total: f64 = values.iter().sum();
            if total.abs() > 1e-9 {
                notes.push(format!(
                "trade_balance sums to {total} for the world; KITE reports a run as not converged when the solved balances do not sum to 0 within tolerance_accounting (default 1e-6) of world value added (R/update_equilibrium.R:18-28)"
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

// ---- CSV ---------------------------------------------------------------------

/// Read a baseline from a directory holding one `<table>.csv` per table.
/// Each file has one column per dimension, in the order of `TABLES`, then
/// `value`. Files for tables kite-mcp does not know are an error.
pub fn read_csv_dir(id: &str, dir: &Path) -> Result<Baseline, BaselineError> {
    let io = |p: &Path, e: &dyn std::fmt::Display| BaselineError::Io {
        path: p.display().to_string(),
        message: e.to_string(),
    };
    let entries = std::fs::read_dir(dir).map_err(|e| io(dir, &e))?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("csv"))
        .collect();
    files.sort();
    let mut tables = Vec::new();
    for path in files {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let Some(spec) = spec(&name) else {
            let known: Vec<&str> = TABLES.iter().map(|t| t.name).collect();
            return Err(BaselineError::Unknown(
                format!("{name}.csv"),
                known.join(", "),
            ));
        };
        let table = read_csv_table(spec, &path)?;
        tables.push(Named::new(spec.name, Value::Table(table)));
    }
    Baseline::new(
        id.to_string(),
        Source::Csv {
            directory: dir.display().to_string(),
        },
        tables,
    )
}

fn read_csv_table(spec: &TableSpec, path: &Path) -> Result<Table, BaselineError> {
    let file = std::fs::File::open(path).map_err(|e| BaselineError::Io {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(file);
    let header: Vec<String> = reader
        .headers()
        .map_err(|e| invalid(spec.name, e.to_string()))?
        .iter()
        .map(|s| s.to_string())
        .collect();
    let expected: Vec<String> = spec
        .dims
        .iter()
        .map(|d| d.to_string())
        .chain(std::iter::once("value".into()))
        .collect();
    if header != expected {
        return Err(invalid(
            spec.name,
            format!(
                "{}: header is [{}]; expected [{}]",
                path.display(),
                header.join(", "),
                expected.join(", ")
            ),
        ));
    }
    let mut dims: Vec<Vec<Option<String>>> = vec![Vec::new(); spec.dims.len()];
    let mut values: Vec<f64> = Vec::new();
    for (i, rec) in reader.records().enumerate() {
        let rec = rec.map_err(|e| invalid(spec.name, e.to_string()))?;
        for (j, d) in dims.iter_mut().enumerate() {
            d.push(Some(rec[j].to_string()));
        }
        let raw = &rec[spec.dims.len()];
        let v: f64 = raw.parse().map_err(|_| {
            invalid(
                spec.name,
                format!("{} row {}: `{raw}` is not a number", path.display(), i + 1),
            )
        })?;
        values.push(v);
    }
    let mut columns: Vec<crate::protocol::Column> = spec
        .dims
        .iter()
        .zip(dims)
        .map(|(d, v)| crate::protocol::Column {
            name: d.to_string(),
            data: ColumnData::String(v),
        })
        .collect();
    let data = if spec.integer {
        if values.iter().any(|v| v.fract() != 0.0) {
            return Err(invalid(spec.name, "values must be whole numbers 0 or 1"));
        }
        ColumnData::Integer(values.iter().map(|v| Some(*v as i64)).collect())
    } else {
        ColumnData::Double(values.into_iter().map(crate::num::HexF64).collect())
    };
    columns.push(crate::protocol::Column {
        name: "value".into(),
        data,
    });
    Ok(Table { columns })
}

/// Write a baseline as CSV files, with doubles as the shortest decimals that
/// parse back to the same value.
pub fn write_csv_dir(b: &Baseline, dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    for n in &b.tables {
        let Value::Table(t) = &n.value else { continue };
        let mut w = csv::Writer::from_path(dir.join(format!("{}.csv", n.name)))?;
        w.write_record(t.columns.iter().map(|c| c.name.as_str()))?;
        for i in 0..t.n_rows() {
            let rec: Vec<String> = t
                .columns
                .iter()
                .map(|c| match &c.data {
                    ColumnData::String(v) => v[i].clone().unwrap_or_default(),
                    ColumnData::Double(v) => crate::num::shortest_decimal(v[i].0),
                    ColumnData::Integer(v) => v[i].map(|x| x.to_string()).unwrap_or_default(),
                    ColumnData::Logical(v) => v[i].map(|x| x.to_string()).unwrap_or_default(),
                })
                .collect();
            w.write_record(rec)?;
        }
        w.flush()?;
    }
    Ok(())
}

/// Lookup from dimension codes to value, for building scenario tables.
pub fn value_map(t: &Table) -> HashMap<Vec<String>, f64> {
    t.rows().unwrap_or_default().into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(dims: &[&str], rows: &[(&[&str], f64)]) -> Value {
        let rows: Vec<(Vec<String>, f64)> = rows
            .iter()
            .map(|(k, v)| (k.iter().map(|s| s.to_string()).collect(), *v))
            .collect();
        Value::Table(Table::from_rows(dims, &rows))
    }

    /// A two-country, one-sector baseline.
    pub(crate) fn tiny() -> Vec<Named> {
        let c = ["c1", "c2"];
        let mut ts = Vec::new();
        for o in c {
            for d in c {
                ts.push((vec![o.to_string(), d.to_string(), "s1".to_string()], 0.5));
            }
        }
        vec![
            Named::new("trade_share", Value::Table(Table::from_rows(D3, &ts))),
            Named::new(
                "intermediate_share",
                t(
                    &["country", "input", "output"],
                    &[(&["c1", "s1", "s1"], 1.0), (&["c2", "s1", "s1"], 1.0)],
                ),
            ),
            Named::new(
                "factor_share",
                t(
                    &["country", "sector"],
                    &[(&["c1", "s1"], 0.4), (&["c2", "s1"], 0.5)],
                ),
            ),
            Named::new(
                "consumption_share",
                t(
                    &["country", "sector"],
                    &[(&["c1", "s1"], 1.0), (&["c2", "s1"], 1.0)],
                ),
            ),
            Named::new(
                "expenditure",
                t(
                    &["destination", "sector"],
                    &[(&["c1", "s1"], 100.0), (&["c2", "s1"], 120.0)],
                ),
            ),
            Named::new(
                "value_added",
                t(&["country"], &[(&["c1"], 100.0), (&["c2"], 120.0)]),
            ),
            Named::new(
                "trade_balance",
                t(&["country"], &[(&["c1"], 1.0), (&["c2"], -1.0)]),
            ),
            Named::new("trade_elasticity", t(&["sector"], &[(&["s1"], 5.0)])),
        ]
    }

    #[test]
    fn a_valid_baseline_is_accepted_and_hashed() {
        let b = Baseline::new("tiny".into(), Source::SyntheticExample, tiny()).unwrap();
        assert_eq!(b.countries, vec!["c1", "c2"]);
        assert_eq!(b.sectors, vec!["s1"]);
        assert_eq!(b.hash.len(), 64);
        let again = Baseline::new("tiny2".into(), Source::SyntheticExample, tiny()).unwrap();
        assert_eq!(b.hash, again.hash);
    }

    #[test]
    fn a_missing_table_is_named() {
        let tables: Vec<Named> = tiny()
            .into_iter()
            .filter(|n| n.name != "factor_share")
            .collect();
        let e = Baseline::new("x".into(), Source::SyntheticExample, tables).unwrap_err();
        assert!(
            matches!(e, BaselineError::Missing { ref table, .. } if table == "factor_share"),
            "{e}"
        );
    }

    #[test]
    fn shares_that_do_not_sum_to_one_are_rejected() {
        let mut tables = tiny();
        tables[3] = Named::new(
            "consumption_share",
            t(
                &["country", "sector"],
                &[(&["c1", "s1"], 0.9), (&["c2", "s1"], 1.0)],
            ),
        );
        let e = Baseline::new("x".into(), Source::SyntheticExample, tables).unwrap_err();
        assert!(e.to_string().contains("sum to 0.9"), "{e}");
    }

    #[test]
    fn unknown_codes_and_incomplete_grids_are_rejected() {
        let mut tables = tiny();
        tables[2] = Named::new(
            "factor_share",
            t(
                &["country", "sector"],
                &[(&["c1", "s1"], 0.4), (&["c9", "s1"], 0.5)],
            ),
        );
        let e = Baseline::new("x".into(), Source::SyntheticExample, tables).unwrap_err();
        assert!(e.to_string().contains("`c9`"), "{e}");
        let mut tables = tiny();
        tables[2] = Named::new(
            "factor_share",
            t(&["country", "sector"], &[(&["c1", "s1"], 0.4)]),
        );
        let e = Baseline::new("x".into(), Source::SyntheticExample, tables).unwrap_err();
        assert!(e.to_string().contains("full grid"), "{e}");
    }

    #[test]
    fn wrong_columns_are_rejected() {
        let mut tables = tiny();
        tables[5] = Named::new(
            "value_added",
            t(&["destination"], &[(&["c1"], 100.0), (&["c2"], 120.0)]),
        );
        let e = Baseline::new("x".into(), Source::SyntheticExample, tables).unwrap_err();
        assert!(e.to_string().contains("expected [country, value]"), "{e}");
    }

    #[test]
    fn csv_round_trip_keeps_values_exactly() {
        let b = Baseline::new("tiny".into(), Source::SyntheticExample, tiny()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        write_csv_dir(&b, dir.path()).unwrap();
        let back = read_csv_dir("tiny", dir.path()).unwrap();
        assert_eq!(back.tables, b.tables);
        assert_eq!(back.hash, b.hash);
        assert!(!back.synthetic);
    }

    #[test]
    fn carbon_tables_go_to_mahlkow_wanner_only() {
        let mut tables = tiny();
        tables.push(Named::new(
            "carbon_intensity",
            t(&["sector"], &[(&["s1"], 0.4)]),
        ));
        let b = Baseline::new("x".into(), Source::SyntheticExample, tables).unwrap();
        assert!(
            b.tables_for_model("caliendo_parro_2015")
                .iter()
                .all(|n| n.name != "carbon_intensity")
        );
        assert!(
            b.tables_for_model("mahlkow_wanner_2021")
                .iter()
                .any(|n| n.name == "carbon_intensity")
        );
    }
}
