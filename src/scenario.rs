//! Scenario and settings input: typed JSON, validated against the model and
//! the baseline, then turned into the values KITE receives.

use crate::baseline::Baseline;
use crate::catalogue;
use crate::num::HexF64;
use crate::protocol::{Column, ColumnData, Named, Table, Value};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One cell or block of cells of a bilateral variable. An omitted dimension
/// means every code of that dimension. Cells whose origin equals their
/// destination are never changed.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BilateralCell {
    /// Exporting country code; omit for every origin
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    /// Importing country code; omit for every destination
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    /// Sector code; omit for every sector
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sector: Option<String>,
    /// Gross factor, for example 1.2 for a 20 % tariff
    pub value: f64,
}

/// One cell or block of cells of a country by sector variable.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CountrySectorCell {
    /// Country code; omit for every country
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// Sector code; omit for every sector
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sector: Option<String>,
    pub value: f64,
}

/// One country, or every country, of a by-country variable.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CountryCell {
    /// Country code; omit for every country
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    pub value: f64,
}

/// Scenario variables, under KITE's own names. Set only those the chosen
/// model reads (see list_models). Cells not mentioned keep their baseline
/// value.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ScenarioInput {
    /// Counterfactual gross tariff factor (a level: 1.2 = 20 % tariff) set by the destination on imports from the origin. All models.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tariff_new: Option<Vec<BilateralCell>>,
    /// Counterfactual gross non-tariff trade cost factor (a level). All models. Not with ntb_change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ntb_new: Option<Vec<BilateralCell>>,
    /// Ratio of new to baseline non-tariff trade costs. All models. Not with ntb_new.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ntb_change: Option<Vec<BilateralCell>>,
    /// Counterfactual gross export subsidy factor (a level; below 1 is a subsidy). All models.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export_subsidy_new: Option<Vec<BilateralCell>>,
    /// Ratio of new to baseline productivity. caliendo_parro_2015 and mahlkow_wanner_2021.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub productivity_change: Option<Vec<CountrySectorCell>>,
    /// Ratio of new to baseline population. caliendo_parro_2015 and mahlkow_wanner_2021.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub population_change: Option<Vec<CountryCell>>,
    /// Factor applied to every country's baseline value added. caliendo_parro_2015 and mahlkow_wanner_2021.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_value_added_change: Option<f64>,
    /// Coalition member country codes. chowdhry_hinz_kamin_wanner_2022 only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coalition_member: Option<Vec<String>>,
    /// Exogenous counterfactual gross tax wedge by country and sector. mahlkow_wanner_2021 only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_new: Option<Vec<CountrySectorCell>>,
    /// Carbon tax per unit of carbon, in baseline currency units. mahlkow_wanner_2021 only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carbon_tax: Option<f64>,
    /// Climate club member country codes (default: all countries). mahlkow_wanner_2021 only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub countries_climate_club: Option<Vec<String>>,
    /// Sector codes with border carbon adjustment (default: all sectors). mahlkow_wanner_2021 only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cbam_sector: Option<Vec<String>>,
    /// Border carbon tariff on imports from non-members. mahlkow_wanner_2021 only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario_carbon_tariff: Option<bool>,
    /// Export rebate of the carbon tax on exports to non-members. mahlkow_wanner_2021 only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario_export_rebate: Option<bool>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TradeBalanceRule {
    /// Trade balances unchanged (KITE's default)
    Fixed,
    /// Each country's trade balance stays a constant share of its own value added
    FixedCountryShare,
    /// Trade balances stay a constant share of world value added
    FixedGlobalShare,
    /// All trade balances set to zero
    Zero,
}

impl TradeBalanceRule {
    pub fn as_str(&self) -> &'static str {
        match self {
            TradeBalanceRule::Fixed => "fixed",
            TradeBalanceRule::FixedCountryShare => "fixed_country_share",
            TradeBalanceRule::FixedGlobalShare => "fixed_global_share",
            TradeBalanceRule::Zero => "zero",
        }
    }
}

/// Solver settings. Omitted fields take the values of KITE's example
/// (tolerance 1e-4, vfactor 0.1, max_iterations 200) and KITE's own
/// defaults for the rest.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SettingsInput {
    /// Convergence tolerance, from 1e-10 to 1e-2. Default 1e-4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<f64>,
    /// Dampening factor of the wage update, above 0 and at most 1. Default 0.1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vfactor: Option<f64>,
    /// Iteration limit, from 1 to 5000. Default 200.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u32>,
    /// How counterfactual trade balances adjust. KITE's default is fixed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_balance_rule: Option<TradeBalanceRule>,
    /// Tolerance of the world trade balance check as a share of world value added, from 0 to 1e-2. KITE's default 1e-6.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance_accounting: Option<f64>,
}

pub const DEFAULT_TOLERANCE: f64 = 1e-4;
pub const DEFAULT_VFACTOR: f64 = 0.1;
pub const DEFAULT_MAX_ITERATIONS: u32 = 200;

/// A validation failure: which field, what it held, and what is allowed.
#[derive(Clone, Debug, Serialize, JsonSchema, PartialEq)]
pub struct FieldError {
    pub field: String,
    pub value: String,
    pub allowed: String,
    pub message: String,
}

fn ferr(
    field: impl Into<String>,
    value: impl ToString,
    allowed: impl Into<String>,
    message: impl Into<String>,
) -> FieldError {
    FieldError {
        field: field.into(),
        value: value.to_string(),
        allowed: allowed.into(),
        message: message.into(),
    }
}

/// A cell override: (dimension filters, value, whether domestic cells are excluded).
pub type Override = (Vec<(&'static str, String)>, f64, bool);

/// How a scenario table was built, for the reproduce script.
#[derive(Clone, Debug, PartialEq)]
pub enum Recipe {
    /// Copy of a baseline table, then cell overrides
    Table {
        dims: Vec<&'static str>,
        base: Option<&'static str>,
        overrides: Vec<Override>,
    },
    Double(f64),
    Strings(Vec<String>),
    Logical(bool),
    Members(Vec<String>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuiltVariable {
    pub name: &'static str,
    pub recipe: Recipe,
    pub value: Value,
}

#[derive(Debug)]
pub struct Built {
    pub scenario: Vec<BuiltVariable>,
    pub settings: Vec<Named>,
    /// Variables set that the model does not read (passed on when allowed)
    pub unread: Vec<&'static str>,
}

fn in_range(field: &str, x: f64, lo: f64, hi: f64, lo_open: bool, errors: &mut Vec<FieldError>) {
    let ok = x.is_finite() && (if lo_open { x > lo } else { x >= lo }) && x <= hi;
    if !ok {
        let allowed = format!("{}{lo}, {hi}]", if lo_open { "(" } else { "[" });
        errors.push(ferr(
            field,
            x,
            allowed.clone(),
            format!("`{field}` is {x}; it must be in {allowed}"),
        ));
    }
}

const BILATERAL: [&str; 3] = ["origin", "destination", "sector"];
const COUNTRY_SECTOR: [&str; 2] = ["country", "sector"];

/// Full grid over `dims` in data.table::CJ(sorted = FALSE) order.
fn grid(dims: &[&'static str], b: &Baseline) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = vec![vec![]];
    for d in dims {
        let codes = if *d == "sector" {
            &b.sectors
        } else {
            &b.countries
        };
        let mut next = Vec::with_capacity(rows.len() * codes.len());
        for r in &rows {
            for c in codes {
                let mut r2 = r.clone();
                r2.push(c.clone());
                next.push(r2);
            }
        }
        rows = next;
    }
    rows
}

struct CellSpec {
    filters: Vec<(&'static str, Option<String>)>,
    value: f64,
}

#[allow(clippy::too_many_arguments)]
fn build_table(
    name: &'static str,
    dims: &[&'static str],
    base: Option<&'static str>,
    cells: &[CellSpec],
    b: &Baseline,
    lo: f64,
    errors: &mut Vec<FieldError>,
) -> Option<BuiltVariable> {
    if cells.is_empty() {
        errors.push(ferr(
            format!("scenario.{name}"),
            "[]",
            "at least one cell",
            format!("`{name}` is an empty list; give at least one cell or leave it out"),
        ));
        return None;
    }
    let bilateral = dims.contains(&"origin");
    let n_before = errors.len();
    let mut overrides = Vec::new();
    for (i, c) in cells.iter().enumerate() {
        let field = format!("scenario.{name}[{i}]");
        in_range(&format!("{field}.value"), c.value, lo, 1e6, true, errors);
        let mut filters = Vec::new();
        for (d, code) in &c.filters {
            if let Some(code) = code {
                let codes = if *d == "sector" {
                    &b.sectors
                } else {
                    &b.countries
                };
                if !codes.contains(code) {
                    errors.push(ferr(
                        format!("{field}.{d}"),
                        code,
                        format!("[{}]", codes.join(", ")),
                        format!(
                            "`{field}.{d}` is `{code}`, which is not a {} of baseline `{}`",
                            if *d == "sector" { "sector" } else { "country" },
                            b.id
                        ),
                    ));
                }
                filters.push((*d, code.clone()));
            }
        }
        let origin = c
            .filters
            .iter()
            .find(|f| f.0 == "origin")
            .and_then(|f| f.1.clone());
        let destination = c
            .filters
            .iter()
            .find(|f| f.0 == "destination")
            .and_then(|f| f.1.clone());
        let mut exclude_diagonal = false;
        if bilateral {
            if origin.is_some() && origin == destination {
                errors.push(ferr(
                    format!("{field}.destination"),
                    destination.clone().unwrap(),
                    "a country other than the origin",
                    format!("`{field}` has origin and destination both `{}`; {name} applies to trade between two countries", origin.clone().unwrap()),
                ));
            }
            exclude_diagonal = !(origin.is_some() && destination.is_some());
        }
        overrides.push((filters, c.value, exclude_diagonal));
    }
    if errors.len() > n_before {
        return None;
    }

    // start from the baseline table, or 1 on the full grid
    let rows: Vec<(Vec<String>, f64)> = match base.and_then(|t| b.table(t)) {
        Some(t) => t.rows().expect("baseline table of codes and values"),
        None => grid(dims, b).into_iter().map(|k| (k, 1.0)).collect(),
    };
    let base_used = base.filter(|t| b.has(t));
    let mut rows = rows;
    for (filters, value, exclude_diagonal) in &overrides {
        for (key, v) in rows.iter_mut() {
            let matches = filters.iter().all(|(d, code)| {
                let j = dims.iter().position(|x| x == d).unwrap();
                &key[j] == code
            });
            let diagonal = bilateral && key[0] == key[1];
            if matches && !(*exclude_diagonal && diagonal) {
                *v = *value;
            }
        }
    }
    Some(BuiltVariable {
        name,
        recipe: Recipe::Table {
            dims: dims.to_vec(),
            base: base_used,
            overrides,
        },
        value: Value::Table(Table::from_rows(dims, &rows)),
    })
}

fn codes_list(
    field: &'static str,
    list: &[String],
    allowed: &[String],
    what: &str,
    b: &Baseline,
    errors: &mut Vec<FieldError>,
) -> bool {
    let mut ok = true;
    let mut seen = std::collections::HashSet::new();
    for (i, code) in list.iter().enumerate() {
        if !allowed.contains(code) {
            errors.push(ferr(
                format!("{field}[{i}]"),
                code,
                format!("[{}]", allowed.join(", ")),
                format!(
                    "`{field}[{i}]` is `{code}`, which is not a {what} of baseline `{}`",
                    b.id
                ),
            ));
            ok = false;
        }
        if !seen.insert(code) {
            errors.push(ferr(
                format!("{field}[{i}]"),
                code,
                "each code once",
                format!("`{code}` appears twice in `{field}`"),
            ));
            ok = false;
        }
    }
    ok
}

/// Validate a scenario and settings for one model and baseline, and build
/// the values KITE receives. All errors are returned together.
pub fn build(
    model: &str,
    b: &Baseline,
    s: &ScenarioInput,
    settings: &SettingsInput,
    allow_unread: bool,
) -> Result<Built, Vec<FieldError>> {
    let mut errors = Vec::new();
    if !catalogue::MODEL_IDS.contains(&model) {
        errors.push(ferr(
            "model",
            model,
            format!("[{}]", catalogue::MODEL_IDS.join(", ")),
            format!("`{model}` is not a KITE model kite-mcp serves"),
        ));
        return Err(errors);
    }

    // which variables are set, in KITE's order
    let set: Vec<&'static str> = [
        ("tariff_new", s.tariff_new.is_some()),
        ("ntb_new", s.ntb_new.is_some()),
        ("ntb_change", s.ntb_change.is_some()),
        ("export_subsidy_new", s.export_subsidy_new.is_some()),
        ("productivity_change", s.productivity_change.is_some()),
        ("population_change", s.population_change.is_some()),
        (
            "global_value_added_change",
            s.global_value_added_change.is_some(),
        ),
        ("coalition_member", s.coalition_member.is_some()),
        ("tax_new", s.tax_new.is_some()),
        ("carbon_tax", s.carbon_tax.is_some()),
        ("countries_climate_club", s.countries_climate_club.is_some()),
        ("cbam_sector", s.cbam_sector.is_some()),
        ("scenario_carbon_tariff", s.scenario_carbon_tariff.is_some()),
        ("scenario_export_rebate", s.scenario_export_rebate.is_some()),
    ]
    .into_iter()
    .filter(|(_, is_set)| *is_set)
    .map(|(n, _)| n)
    .collect();

    let reads = catalogue::model_variables(model);
    let unread: Vec<&'static str> = set.iter().copied().filter(|v| !reads.contains(v)).collect();
    if !allow_unread {
        for v in &unread {
            let readers = catalogue::models_reading(v);
            errors.push(ferr(
                format!("scenario.{v}"),
                "set",
                format!("variables {model} reads: [{}]", reads.join(", ")),
                format!(
                    "{model} does not read `{v}`, so it would have no effect; {} read it. Remove it, choose one of those models, or set allow_unread_variables to true to pass it to KITE, which then warns that it has no effect (R/update_equilibrium.R:246-285)",
                    readers.join(" and ")
                ),
            ));
        }
    }
    if s.ntb_new.is_some() && s.ntb_change.is_some() {
        errors.push(ferr(
            "scenario.ntb_change",
            "set together with ntb_new",
            "one of ntb_new, ntb_change",
            "set either ntb_new or ntb_change; KITE uses ntb_change and ignores ntb_new when both are set (R/model_caliendo_parro_2015.R:36)",
        ));
    }

    let mut scenario = Vec::new();
    let bilateral = |cells: &Vec<BilateralCell>| -> Vec<CellSpec> {
        cells
            .iter()
            .map(|c| CellSpec {
                filters: vec![
                    ("origin", c.origin.clone()),
                    ("destination", c.destination.clone()),
                    ("sector", c.sector.clone()),
                ],
                value: c.value,
            })
            .collect()
    };
    let country_sector = |cells: &Vec<CountrySectorCell>| -> Vec<CellSpec> {
        cells
            .iter()
            .map(|c| CellSpec {
                filters: vec![("country", c.country.clone()), ("sector", c.sector.clone())],
                value: c.value,
            })
            .collect()
    };
    let mut push = |v: Option<BuiltVariable>| {
        if let Some(v) = v {
            scenario.push(v)
        }
    };
    if let Some(c) = &s.tariff_new {
        push(build_table(
            "tariff_new",
            &BILATERAL,
            Some("tariff"),
            &bilateral(c),
            b,
            0.0,
            &mut errors,
        ));
    }
    if let Some(c) = &s.ntb_new {
        push(build_table(
            "ntb_new",
            &BILATERAL,
            Some("ntb"),
            &bilateral(c),
            b,
            0.0,
            &mut errors,
        ));
    }
    if let Some(c) = &s.ntb_change {
        push(build_table(
            "ntb_change",
            &BILATERAL,
            None,
            &bilateral(c),
            b,
            0.0,
            &mut errors,
        ));
    }
    if let Some(c) = &s.export_subsidy_new {
        push(build_table(
            "export_subsidy_new",
            &BILATERAL,
            Some("export_subsidy"),
            &bilateral(c),
            b,
            0.0,
            &mut errors,
        ));
    }
    if let Some(c) = &s.productivity_change {
        push(build_table(
            "productivity_change",
            &COUNTRY_SECTOR,
            None,
            &country_sector(c),
            b,
            0.0,
            &mut errors,
        ));
    }
    if let Some(c) = &s.population_change {
        let cells: Vec<CellSpec> = c
            .iter()
            .map(|c| CellSpec {
                filters: vec![("country", c.country.clone())],
                value: c.value,
            })
            .collect();
        push(build_table(
            "population_change",
            &["country"],
            None,
            &cells,
            b,
            0.0,
            &mut errors,
        ));
    }
    if let Some(x) = s.global_value_added_change {
        in_range(
            "scenario.global_value_added_change",
            x,
            0.0,
            1e6,
            true,
            &mut errors,
        );
        push(Some(BuiltVariable {
            name: "global_value_added_change",
            recipe: Recipe::Double(x),
            value: Value::Double(HexF64(x)),
        }));
    }
    if let Some(list) = &s.coalition_member
        && codes_list(
            "scenario.coalition_member",
            list,
            &b.countries,
            "country",
            b,
            &mut errors,
        )
    {
        let table = Table {
            columns: vec![
                Column {
                    name: "country".into(),
                    data: ColumnData::String(b.countries.iter().map(|c| Some(c.clone())).collect()),
                },
                Column {
                    name: "value".into(),
                    data: ColumnData::Integer(
                        b.countries
                            .iter()
                            .map(|c| Some(list.contains(c) as i64))
                            .collect(),
                    ),
                },
            ],
        };
        push(Some(BuiltVariable {
            name: "coalition_member",
            recipe: Recipe::Members(list.clone()),
            value: Value::Table(table),
        }));
    }
    if let Some(c) = &s.tax_new {
        push(build_table(
            "tax_new",
            &COUNTRY_SECTOR,
            Some("tax"),
            &country_sector(c),
            b,
            0.0,
            &mut errors,
        ));
    }
    if let Some(x) = s.carbon_tax {
        in_range("scenario.carbon_tax", x, 0.0, 1e6, false, &mut errors);
        if x > 0.0 && model == "mahlkow_wanner_2021" {
            for t in ["price", "carbon_intensity"] {
                if !b.has(t) {
                    errors.push(ferr(
                        "scenario.carbon_tax",
                        x,
                        format!("0 for baseline `{}`", b.id),
                        format!("a carbon tax above 0 needs the baseline table `{t}`, which baseline `{}` does not have (R/model_mahlkow_wanner_2021.R:67-68)", b.id),
                    ));
                }
            }
        }
        push(Some(BuiltVariable {
            name: "carbon_tax",
            recipe: Recipe::Double(x),
            value: Value::Double(HexF64(x)),
        }));
    }
    if let Some(list) = &s.countries_climate_club
        && codes_list(
            "scenario.countries_climate_club",
            list,
            &b.countries,
            "country",
            b,
            &mut errors,
        )
    {
        push(Some(BuiltVariable {
            name: "countries_climate_club",
            recipe: Recipe::Strings(list.clone()),
            value: Value::Strings(list.clone()),
        }));
    }
    if let Some(list) = &s.cbam_sector
        && codes_list(
            "scenario.cbam_sector",
            list,
            &b.sectors,
            "sector",
            b,
            &mut errors,
        )
    {
        push(Some(BuiltVariable {
            name: "cbam_sector",
            recipe: Recipe::Strings(list.clone()),
            value: Value::Strings(list.clone()),
        }));
    }
    if let Some(x) = s.scenario_carbon_tariff {
        push(Some(BuiltVariable {
            name: "scenario_carbon_tariff",
            recipe: Recipe::Logical(x),
            value: Value::Logical(x),
        }));
    }
    if let Some(x) = s.scenario_export_rebate {
        push(Some(BuiltVariable {
            name: "scenario_export_rebate",
            recipe: Recipe::Logical(x),
            value: Value::Logical(x),
        }));
    }

    // settings, in the order of KITE's example
    let tolerance = settings.tolerance.unwrap_or(DEFAULT_TOLERANCE);
    in_range(
        "settings.tolerance",
        tolerance,
        1e-10,
        1e-2,
        false,
        &mut errors,
    );
    let vfactor = settings.vfactor.unwrap_or(DEFAULT_VFACTOR);
    in_range("settings.vfactor", vfactor, 0.0, 1.0, true, &mut errors);
    let max_iterations = settings.max_iterations.unwrap_or(DEFAULT_MAX_ITERATIONS);
    if !(1..=5000).contains(&max_iterations) {
        errors.push(ferr(
            "settings.max_iterations",
            max_iterations,
            "[1, 5000]",
            format!("`settings.max_iterations` is {max_iterations}; it must be from 1 to 5000"),
        ));
    }
    let mut named_settings = vec![
        Named::new("verbose", Value::Integer(0)),
        Named::new("tolerance", Value::Double(HexF64(tolerance))),
        Named::new("vfactor", Value::Double(HexF64(vfactor))),
        Named::new(
            "max_iterations",
            Value::Double(HexF64(max_iterations as f64)),
        ),
    ];
    if let Some(r) = settings.trade_balance_rule {
        named_settings.push(Named::new(
            "trade_balance_rule",
            Value::String(r.as_str().into()),
        ));
    }
    if let Some(t) = settings.tolerance_accounting {
        in_range(
            "settings.tolerance_accounting",
            t,
            0.0,
            1e-2,
            false,
            &mut errors,
        );
        named_settings.push(Named::new("tolerance_accounting", Value::Double(HexF64(t))));
    }

    if errors.is_empty() {
        Ok(Built {
            scenario,
            settings: named_settings,
            unread,
        })
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::{Baseline, Source};

    fn three_by_two() -> Baseline {
        // a 3-country, 2-sector baseline with identity tariffs, built in Rust
        let countries = ["c1", "c2", "c3"];
        let sectors = ["s1", "s2"];
        let mut ts = Vec::new();
        let mut ones = Vec::new();
        for o in countries {
            for d in countries {
                for s in sectors {
                    ts.push((vec![o.into(), d.into(), s.into()], 1.0 / 3.0));
                    ones.push((vec![o.into(), d.into(), s.into()], 1.0));
                }
            }
        }
        // make trade shares sum to one exactly
        for r in ts.iter_mut() {
            if r.0[0] == "c3" {
                r.1 = 1.0 - 2.0 / 3.0;
            }
        }
        let cs = |v: f64| {
            let mut rows = Vec::new();
            for c in countries {
                for s in sectors {
                    rows.push((vec![c.to_string(), s.to_string()], v));
                }
            }
            rows
        };
        let mut is = Vec::new();
        for c in countries {
            for i in sectors {
                for o in sectors {
                    is.push((vec![c.to_string(), i.to_string(), o.to_string()], 0.5));
                }
            }
        }
        let by_country = |v: f64| {
            countries
                .iter()
                .map(|c| (vec![c.to_string()], v))
                .collect::<Vec<_>>()
        };
        let tables = vec![
            Named::new(
                "trade_share",
                Value::Table(Table::from_rows(&BILATERAL, &ts)),
            ),
            Named::new(
                "intermediate_share",
                Value::Table(Table::from_rows(&["country", "input", "output"], &is)),
            ),
            Named::new(
                "factor_share",
                Value::Table(Table::from_rows(&COUNTRY_SECTOR, &cs(0.4))),
            ),
            Named::new(
                "consumption_share",
                Value::Table(Table::from_rows(&COUNTRY_SECTOR, &cs(0.5))),
            ),
            Named::new(
                "expenditure",
                Value::Table(Table::from_rows(&["destination", "sector"], &cs(100.0))),
            ),
            Named::new(
                "value_added",
                Value::Table(Table::from_rows(&["country"], &by_country(100.0))),
            ),
            Named::new(
                "trade_balance",
                Value::Table(Table::from_rows(&["country"], &by_country(0.0))),
            ),
            Named::new(
                "trade_elasticity",
                Value::Table(Table::from_rows(
                    &["sector"],
                    &[(vec!["s1".into()], 4.0), (vec!["s2".into()], 5.0)],
                )),
            ),
            Named::new("tariff", Value::Table(Table::from_rows(&BILATERAL, &ones))),
        ];
        Baseline::new("t".into(), Source::SyntheticExample, tables).unwrap()
    }

    fn cell(o: Option<&str>, d: Option<&str>, s: Option<&str>, v: f64) -> BilateralCell {
        BilateralCell {
            origin: o.map(Into::into),
            destination: d.map(Into::into),
            sector: s.map(Into::into),
            value: v,
        }
    }

    #[test]
    fn the_example_tariff_war_sets_four_cells() {
        let b = three_by_two();
        let s = ScenarioInput {
            tariff_new: Some(vec![
                cell(Some("c1"), Some("c2"), None, 1.2),
                cell(Some("c2"), Some("c1"), None, 1.2),
            ]),
            ..Default::default()
        };
        let built = build(
            "caliendo_parro_2015",
            &b,
            &s,
            &SettingsInput::default(),
            false,
        )
        .unwrap();
        let Value::Table(t) = &built.scenario[0].value else {
            panic!()
        };
        let changed: Vec<_> = t
            .rows()
            .unwrap()
            .into_iter()
            .filter(|r| r.1 != 1.0)
            .collect();
        assert_eq!(changed.len(), 4);
        assert!(changed.iter().all(|r| r.1 == 1.2));
        assert_eq!(built.settings.len(), 4);
    }

    #[test]
    fn an_omitted_origin_leaves_domestic_cells_alone() {
        let b = three_by_two();
        let s = ScenarioInput {
            tariff_new: Some(vec![cell(None, Some("c1"), Some("s1"), 1.1)]),
            ..Default::default()
        };
        let built = build(
            "caliendo_parro_2015",
            &b,
            &s,
            &SettingsInput::default(),
            false,
        )
        .unwrap();
        let Value::Table(t) = &built.scenario[0].value else {
            panic!()
        };
        let changed: Vec<_> = t
            .rows()
            .unwrap()
            .into_iter()
            .filter(|r| r.1 != 1.0)
            .collect();
        assert_eq!(changed.len(), 2);
        assert!(
            changed
                .iter()
                .all(|r| r.0[0] != "c1" && r.0[1] == "c1" && r.0[2] == "s1")
        );
    }

    #[test]
    fn errors_name_the_field_the_value_and_the_allowed_set() {
        let b = three_by_two();
        let s = ScenarioInput {
            tariff_new: Some(vec![cell(Some("c9"), Some("c2"), None, -1.2)]),
            ..Default::default()
        };
        let errs = build(
            "caliendo_parro_2015",
            &b,
            &s,
            &SettingsInput::default(),
            false,
        )
        .unwrap_err();
        let origin = errs
            .iter()
            .find(|e| e.field == "scenario.tariff_new[0].origin")
            .unwrap();
        assert_eq!(origin.value, "c9");
        assert_eq!(origin.allowed, "[c1, c2, c3]");
        let value = errs
            .iter()
            .find(|e| e.field == "scenario.tariff_new[0].value")
            .unwrap();
        assert_eq!(value.value, "-1.2");
    }

    #[test]
    fn a_variable_the_model_does_not_read_is_refused_unless_allowed() {
        let b = three_by_two();
        let s = ScenarioInput {
            carbon_tax: Some(0.5),
            ..Default::default()
        };
        let errs = build(
            "caliendo_parro_2015",
            &b,
            &s,
            &SettingsInput::default(),
            false,
        )
        .unwrap_err();
        assert_eq!(errs[0].field, "scenario.carbon_tax");
        assert!(errs[0].message.contains("mahlkow_wanner_2021"));
        let built = build(
            "caliendo_parro_2015",
            &b,
            &s,
            &SettingsInput::default(),
            true,
        )
        .unwrap();
        assert_eq!(built.unread, vec!["carbon_tax"]);
    }

    #[test]
    fn same_origin_and_destination_is_refused() {
        let b = three_by_two();
        let s = ScenarioInput {
            tariff_new: Some(vec![cell(Some("c1"), Some("c1"), None, 1.2)]),
            ..Default::default()
        };
        assert!(
            build(
                "caliendo_parro_2015",
                &b,
                &s,
                &SettingsInput::default(),
                false
            )
            .is_err()
        );
    }

    #[test]
    fn settings_are_bounded() {
        let b = three_by_two();
        let st = SettingsInput {
            tolerance: Some(0.5),
            vfactor: Some(0.0),
            max_iterations: Some(0),
            ..Default::default()
        };
        let errs = build(
            "caliendo_parro_2015",
            &b,
            &ScenarioInput::default(),
            &st,
            false,
        )
        .unwrap_err();
        let fields: Vec<&str> = errs.iter().map(|e| e.field.as_str()).collect();
        assert_eq!(
            fields,
            vec![
                "settings.tolerance",
                "settings.vfactor",
                "settings.max_iterations"
            ]
        );
    }

    #[test]
    fn carbon_tax_needs_price_and_carbon_intensity() {
        let b = three_by_two();
        let s = ScenarioInput {
            carbon_tax: Some(0.5),
            ..Default::default()
        };
        let errs = build(
            "mahlkow_wanner_2021",
            &b,
            &s,
            &SettingsInput::default(),
            false,
        )
        .unwrap_err();
        assert!(errs.iter().any(|e| e.message.contains("`price`")));
    }

    #[test]
    fn unknown_fields_are_rejected_by_the_schema() {
        let e = serde_json::from_str::<ScenarioInput>(r#"{"tarif_new": []}"#).unwrap_err();
        assert!(e.to_string().contains("tarif_new"));
    }
}
