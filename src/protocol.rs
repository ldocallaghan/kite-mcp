//! Types of the worker protocol (docs/worker-protocol.md).

use crate::num::HexF64;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: i64 = 1;

/// A column of a table. Doubles travel as hexadecimal bit patterns.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "values", rename_all = "lowercase")]
pub enum ColumnData {
    String(Vec<Option<String>>),
    Double(Vec<HexF64>),
    Integer(Vec<Option<i64>>),
    Logical(Vec<Option<bool>>),
}

impl ColumnData {
    pub fn len(&self) -> usize {
        match self {
            ColumnData::String(v) => v.len(),
            ColumnData::Double(v) => v.len(),
            ColumnData::Integer(v) => v.len(),
            ColumnData::Logical(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    #[serde(flatten)]
    pub data: ColumnData,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Table {
    pub columns: Vec<Column>,
}

impl Table {
    pub fn n_rows(&self) -> usize {
        self.columns.first().map(|c| c.data.len()).unwrap_or(0)
    }

    pub fn column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|c| c.name == name)
    }

    pub fn strings(&self, name: &str) -> Option<&Vec<Option<String>>> {
        match &self.column(name)?.data {
            ColumnData::String(v) => Some(v),
            _ => None,
        }
    }

    pub fn doubles(&self, name: &str) -> Option<Vec<f64>> {
        match &self.column(name)?.data {
            ColumnData::Double(v) => Some(v.iter().map(|x| x.0).collect()),
            _ => None,
        }
    }

    /// Names of the string columns, which are the table's dimensions.
    pub fn dimension_names(&self) -> Vec<String> {
        self.columns
            .iter()
            .filter(|c| matches!(c.data, ColumnData::String(_)))
            .map(|c| c.name.clone())
            .collect()
    }

    /// A table of string dimension columns followed by one double `value` column.
    pub fn from_rows(dims: &[&str], rows: &[(Vec<String>, f64)]) -> Table {
        let mut columns: Vec<Column> = dims
            .iter()
            .enumerate()
            .map(|(i, d)| Column {
                name: d.to_string(),
                data: ColumnData::String(rows.iter().map(|r| Some(r.0[i].clone())).collect()),
            })
            .collect();
        columns.push(Column {
            name: "value".into(),
            data: ColumnData::Double(rows.iter().map(|r| HexF64(r.1)).collect()),
        });
        Table { columns }
    }

    /// Rows as (dimension values, value) for a table of string dimensions and
    /// a double `value` column.
    pub fn rows(&self) -> Option<Vec<(Vec<String>, f64)>> {
        let dims = self.dimension_names();
        let values = self.doubles("value")?;
        let cols: Vec<&Vec<Option<String>>> =
            dims.iter().map(|d| self.strings(d).unwrap()).collect();
        let mut out = Vec::with_capacity(values.len());
        for (i, v) in values.iter().enumerate() {
            let key = cols
                .iter()
                .map(|c| c[i].clone().unwrap_or_default())
                .collect();
            out.push((key, *v));
        }
        Some(out)
    }
}

/// A typed value of a scenario variable, setting or initial condition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Value {
    Table(Table),
    Double(HexF64),
    Integer(i64),
    Logical(bool),
    String(String),
    Strings(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Named {
    pub name: String,
    pub value: Value,
}

impl Named {
    pub fn new(name: &str, value: Value) -> Self {
        Named {
            name: name.to_string(),
            value,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Request<'a> {
    pub id: String,
    pub method: &'a str,
    pub params: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Reply {
    pub id: Option<String>,
    pub ok: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<WorkerError>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WorkerError {
    pub class: Vec<String>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelPackage {
    pub name: String,
    pub version: String,
    pub release_name: Option<String>,
    pub commit: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hello {
    pub protocol: i64,
    pub language: String,
    pub runtime_version: String,
    pub platform: String,
    pub model_package: ModelPackage,
    pub packages: serde_json::Map<String, serde_json::Value>,
    pub rng_kind: Vec<String>,
    pub models: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InitialConditions {
    pub initial_conditions: Vec<Named>,
}

/// A warning or message raised while KITE ran.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
pub struct Condition {
    /// `update_equilibrium` or `process_results`
    pub stage: String,
    /// The condition's R classes, most specific first
    pub class: Vec<String>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Accounting {
    pub ok: Option<Value>,
    pub rule: Option<Value>,
    pub world_trade_balance: Option<Value>,
    pub baseline_world_trade_balance: Option<Value>,
    pub world_value_added: Option<Value>,
    pub tolerance: Option<Value>,
    pub reason: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Info {
    pub convergence: Option<Value>,
    pub criterion: Option<Value>,
    pub iterations: Option<Value>,
    pub elapsed_seconds: Option<Value>,
    pub accounting: Accounting,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NamedTable {
    pub name: String,
    pub table: Table,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NamedValue {
    pub name: String,
    pub value: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SolveResult {
    pub info: Info,
    pub warnings: Vec<Condition>,
    pub messages: Vec<Condition>,
    pub tables: Vec<NamedTable>,
    pub values: Vec<NamedValue>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SolveParams<'a> {
    pub model: &'a str,
    pub initial_conditions: &'a [Named],
    pub scenario: &'a [Named],
    pub settings: &'a [Named],
}

pub fn value_as_bool(v: &Option<Value>) -> Option<bool> {
    match v {
        Some(Value::Logical(b)) => Some(*b),
        _ => None,
    }
}

pub fn value_as_f64(v: &Option<Value>) -> Option<f64> {
    match v {
        Some(Value::Double(x)) => Some(x.0),
        Some(Value::Integer(i)) => Some(*i as f64),
        _ => None,
    }
}

pub fn value_as_i64(v: &Option<Value>) -> Option<i64> {
    match v {
        Some(Value::Integer(i)) => Some(*i),
        _ => None,
    }
}

pub fn value_as_string(v: &Option<Value>) -> Option<String> {
    match v {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_use_the_documented_wire_shape() {
        let v = Value::Double(HexF64(1.2));
        assert_eq!(
            serde_json::to_string(&v).unwrap(),
            r#"{"double":"3ff3333333333333"}"#
        );
        let t = Table::from_rows(&["country"], &[(vec!["c1".into()], 1.0)]);
        let s = serde_json::to_string(&Value::Table(t.clone())).unwrap();
        assert_eq!(
            s,
            r#"{"table":{"columns":[{"name":"country","type":"string","values":["c1"]},{"name":"value","type":"double","values":["3ff0000000000000"]}]}}"#
        );
        let back: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(back, Value::Table(t));
    }

    #[test]
    fn worker_info_block_parses() {
        let json = r#"{"convergence":{"logical":true},"criterion":{"double":"3f1994ce4c9c5f5d"},
            "iterations":{"integer":22},"elapsed_seconds":{"double":"3fb27ba3e5da1a1c"},
            "accounting":{"ok":{"logical":true},"rule":{"string":"fixed"},"world_trade_balance":{"double":"0000000000000000"},
            "baseline_world_trade_balance":{"double":"0000000000000000"},"world_value_added":{"double":"4080000000000000"},
            "tolerance":{"double":"3eb0c6f7a0b5ed8d"},"reason":{"string":"ok"}}}"#;
        let info: Info = serde_json::from_str(json).unwrap();
        assert_eq!(value_as_bool(&info.convergence), Some(true));
        assert_eq!(value_as_i64(&info.iterations), Some(22));
        let na: Info = serde_json::from_str(
            r#"{"convergence":null,"criterion":null,"iterations":null,"elapsed_seconds":null,
            "accounting":{"ok":null,"rule":null,"world_trade_balance":null,"baseline_world_trade_balance":null,
            "world_value_added":null,"tolerance":null,"reason":null}}"#,
        )
        .unwrap();
        assert_eq!(value_as_bool(&na.convergence), None);
    }
}
