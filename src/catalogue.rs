//! What kite-mcp knows about KITE's models, their scenario variables and
//! their output tables. Every statement cites the KITE source it comes from
//! (KITE 26.10, commit 5995dd1); paths are relative to the KITE repository.

use schemars::JsonSchema;
use serde::Serialize;

pub const KITE_REPOSITORY: &str = "https://github.com/julianhinz/KITE";
pub const KITE_COMMIT: &str = "5995dd1e67803d0a499b2badbd8c3d859e08fdc5";
pub const KITE_VERSION: &str = "26.10";

pub const MODEL_IDS: [&str; 3] = [
    "caliendo_parro_2015",
    "chowdhry_hinz_kamin_wanner_2022",
    "mahlkow_wanner_2021",
];

pub const KITE_CITATION: &str = "Hinz, J., Mahlkow, H. and Wanner, J. (2025). The KITE Model Suite: A Quantitative Framework for International Trade Analysis. KITE White Paper, Kiel Institute for the World Economy. https://trade.kielinstitut.de/KTTM/KITE_whitepaper.pdf";

pub const KITE_BIBTEX: &str = "@techreport{HinzMahlkowWanner2025,\n  author      = {Hinz, Julian and Mahlkow, Hendrik and Wanner, Joschka},\n  title       = {The {KITE} Model Suite: A Quantitative Framework for International Trade Analysis},\n  institution = {Kiel Institute for the World Economy},\n  type        = {KITE White Paper},\n  year        = {2025},\n  url         = {https://trade.kielinstitut.de/KTTM/KITE_whitepaper.pdf}\n}";

pub const KITE_PACKAGE_CITATION: &str = "Hinz, J., Mahlkow, H. and Wanner, J. (2026). KITE Model Suite. R package version 26.10 (Brisk Fehmarn). https://github.com/julianhinz/KITE";

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct VariableInfo {
    /// KITE's own name for the variable
    pub name: &'static str,
    /// Dimensions of the table KITE receives, empty for a single value
    pub dimensions: Vec<&'static str>,
    /// How the value is given in kite-mcp's run_scenario input
    pub input_form: &'static str,
    pub meaning: &'static str,
    /// Value when the scenario does not set the variable
    pub default: &'static str,
    /// Values kite-mcp accepts
    pub allowed: &'static str,
    /// KITE source lines this description rests on
    pub source: &'static str,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ModelInfo {
    pub id: &'static str,
    pub name: &'static str,
    /// Reference as KITE's README gives it
    pub reference: &'static str,
    pub citation: &'static str,
    pub adds: &'static str,
    /// Scenario variables this model reads, as kite-mcp accepts them
    pub scenario_variables: Vec<VariableInfo>,
    pub source: &'static str,
}

fn var(name: &str) -> VariableInfo {
    match name {
        "tariff_new" => VariableInfo {
            name: "tariff_new",
            dimensions: vec!["origin", "destination", "sector"],
            input_form: "list of cells {origin?, destination?, sector?, value}; an omitted dimension means every code; cells with origin equal to destination are never changed",
            meaning: "Counterfactual gross ad-valorem tariff factor that the destination applies to imports from the origin. A level, not a change: 1.2 is a 20 % tariff, 1 is none. KITE forms tariff_change = tariff_new / tariff. Tariff revenue accrues to the destination.",
            default: "the baseline tariff (identity, 1, in the synthetic baselines)",
            allowed: "finite and greater than 0",
            source: "R/model_caliendo_parro_2015.R:29-31; R/process_results.R:239-245; inst/examples/example.R:96-98",
        },
        "ntb_new" => VariableInfo {
            name: "ntb_new",
            dimensions: vec!["origin", "destination", "sector"],
            input_form: "list of cells, as for tariff_new",
            meaning: "Counterfactual gross factor of non-tariff trade costs. A level: 1.1 raises trade costs by 10 % with no revenue. Trade costs are tariff * ntb * export_subsidy.",
            default: "the baseline ntb (1 in the synthetic baselines)",
            allowed: "finite and greater than 0; not together with ntb_change",
            source: "R/model_caliendo_parro_2015.R:34-36,44-46",
        },
        "ntb_change" => VariableInfo {
            name: "ntb_change",
            dimensions: vec!["origin", "destination", "sector"],
            input_form: "list of cells, as for tariff_new; unset cells are 1",
            meaning: "Ratio of counterfactual to baseline non-tariff trade costs. KITE uses it directly when it is set, in place of ntb_new / ntb.",
            default: "ntb_new / ntb",
            allowed: "finite and greater than 0; not together with ntb_new",
            source: "R/model_caliendo_parro_2015.R:36",
        },
        "export_subsidy_new" => VariableInfo {
            name: "export_subsidy_new",
            dimensions: vec!["origin", "destination", "sector"],
            input_form: "list of cells, as for tariff_new",
            meaning: "Counterfactual gross factor on the trade costs of the origin's exports. A level: a value below 1 is a subsidy (the export rebates of mahlkow_wanner_2021 lower it). The origin bears the cost, reported as export_subsidy_costs.",
            default: "the baseline export_subsidy (1 in the synthetic baselines)",
            allowed: "finite and greater than 0",
            source: "R/model_caliendo_parro_2015.R:39-46; R/process_results.R:250-257; R/model_mahlkow_wanner_2021.R:54-56",
        },
        "productivity_change" => VariableInfo {
            name: "productivity_change",
            dimensions: vec!["country", "sector"],
            input_form: "list of cells {country?, sector?, value}; unset cells are 1",
            meaning: "Ratio of counterfactual to baseline productivity. The input cost change is divided by it.",
            default: "1",
            allowed: "finite and greater than 0",
            source: "R/model_caliendo_parro_2015.R:56,246",
        },
        "population_change" => VariableInfo {
            name: "population_change",
            dimensions: vec!["country"],
            input_form: "list of cells {country?, value}; unset cells are 1",
            meaning: "Ratio of counterfactual to baseline population. Wage changes and welfare_change are per head.",
            default: "1",
            allowed: "finite and greater than 0",
            source: "R/model_caliendo_parro_2015.R:57,168; R/process_results.R:71-78,345",
        },
        "global_value_added_change" => VariableInfo {
            name: "global_value_added_change",
            dimensions: vec![],
            input_form: "a single number",
            meaning: "Factor by which every country's baseline value added is multiplied before solving.",
            default: "unset (value added unchanged)",
            allowed: "finite and greater than 0",
            source: "R/model_caliendo_parro_2015.R:58",
        },
        "coalition_member" => VariableInfo {
            name: "coalition_member",
            dimensions: vec!["country"],
            input_form: "list of member country codes; kite-mcp sends KITE a 0/1 table by country",
            meaning: "Members of the coalition. Transfers between members give every member the same welfare change (Chowdhry, Hinz, Kamin and Wanner, equation 13).",
            default: "the baseline coalition_member (c1 and c3 in synthetic_example)",
            allowed: "country codes of the baseline; an empty list means no coalition",
            source: "R/model_chowdhry_hinz_kamin_wanner_2022.R:170,407-500",
        },
        "tax_new" => VariableInfo {
            name: "tax_new",
            dimensions: vec!["country", "sector"],
            input_form: "list of cells {country?, sector?, value}",
            meaning: "Exogenous counterfactual gross tax wedge on the price of sector goods in the country. The carbon tax adds to it.",
            default: "the baseline tax (1 when the baseline has none)",
            allowed: "finite and greater than 0",
            source: "R/model_mahlkow_wanner_2021.R:28-33,69,136-138",
        },
        "carbon_tax" => VariableInfo {
            name: "carbon_tax",
            dimensions: vec![],
            input_form: "a single number",
            meaning: "Carbon tax per unit of carbon, levied by climate club members on all domestic use of sector goods. A specific tax in units of baseline currency (world value added is the numeraire); its ad-valorem equivalent depends on the baseline price levels.",
            default: "0 (no carbon policy)",
            allowed: "finite and at least 0; above 0 needs the baseline tables price and carbon_intensity",
            source: "R/model_mahlkow_wanner_2021.R:10-26,70",
        },
        "countries_climate_club" => VariableInfo {
            name: "countries_climate_club",
            dimensions: vec!["country"],
            input_form: "list of country codes",
            meaning: "Members of the climate club, which levy the carbon tax.",
            default: "all countries",
            allowed: "country codes of the baseline",
            source: "R/model_mahlkow_wanner_2021.R:71-74",
        },
        "cbam_sector" => VariableInfo {
            name: "cbam_sector",
            dimensions: vec!["sector"],
            input_form: "list of sector codes",
            meaning: "Sectors with border carbon adjustment.",
            default: "all sectors",
            allowed: "sector codes of the baseline",
            source: "R/model_mahlkow_wanner_2021.R:75-76",
        },
        "scenario_carbon_tariff" => VariableInfo {
            name: "scenario_carbon_tariff",
            dimensions: vec![],
            input_form: "true or false",
            meaning: "Whether club members levy the carbon tax on the carbon embodied in imports from non-members (border carbon tariff).",
            default: "false",
            allowed: "true or false",
            source: "R/model_mahlkow_wanner_2021.R:43-53,77",
        },
        "scenario_export_rebate" => VariableInfo {
            name: "scenario_export_rebate",
            dimensions: vec![],
            input_form: "true or false",
            meaning: "Whether club members refund the carbon tax on embodied carbon on their exports to non-members.",
            default: "false",
            allowed: "true or false",
            source: "R/model_mahlkow_wanner_2021.R:54-56,78",
        },
        other => panic!("no catalogue entry for {other}"),
    }
}

/// Policy variables each model reads. The full list of variables KITE
/// accepts per model is `kite_scenario_variables()` in
/// R/update_equilibrium.R:224-244; kite-mcp leaves out the initial
/// conditions there (trade_share, value_added and so on), which a scenario
/// could replace but which belong to the baseline.
pub fn model_variables(model: &str) -> &'static [&'static str] {
    const COMMON: [&str; 4] = ["tariff_new", "ntb_new", "ntb_change", "export_subsidy_new"];
    match model {
        "caliendo_parro_2015" => &[
            COMMON[0],
            COMMON[1],
            COMMON[2],
            COMMON[3],
            "productivity_change",
            "population_change",
            "global_value_added_change",
        ],
        "chowdhry_hinz_kamin_wanner_2022" => &[
            COMMON[0],
            COMMON[1],
            COMMON[2],
            COMMON[3],
            "coalition_member",
        ],
        "mahlkow_wanner_2021" => &[
            COMMON[0],
            COMMON[1],
            COMMON[2],
            COMMON[3],
            "productivity_change",
            "population_change",
            "global_value_added_change",
            "tax_new",
            "carbon_tax",
            "countries_climate_club",
            "cbam_sector",
            "scenario_carbon_tariff",
            "scenario_export_rebate",
        ],
        _ => &[],
    }
}

pub fn all_variables() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for m in MODEL_IDS {
        for v in model_variables(m) {
            if !out.contains(v) {
                out.push(v);
            }
        }
    }
    out
}

pub fn models_reading(variable: &str) -> Vec<&'static str> {
    MODEL_IDS
        .iter()
        .copied()
        .filter(|m| model_variables(m).contains(&variable))
        .collect()
}

pub fn model(id: &str) -> Option<ModelInfo> {
    let scenario_variables = model_variables(id).iter().map(|v| var(v)).collect();
    let m = match id {
        "caliendo_parro_2015" => ModelInfo {
            id: "caliendo_parro_2015",
            name: "Multi-sector Ricardian model with input-output linkages",
            reference: "Caliendo & Parro (2015), Review of Economic Studies",
            citation: "Caliendo, L. and Parro, F. (2015). Estimates of the Trade and Welfare Effects of NAFTA. Review of Economic Studies, 82(1), 1-44.",
            adds: "The base model: tariffs, non-tariff trade costs, export subsidies, productivity and population changes, solved in changes (exact hat algebra) with world value added as numeraire.",
            scenario_variables,
            source: "R/model_caliendo_parro_2015.R; README.md (Models)",
        },
        "chowdhry_hinz_kamin_wanner_2022" => ModelInfo {
            id: "chowdhry_hinz_kamin_wanner_2022",
            name: "Sanctions-coalition extension",
            reference: "Chowdhry, Hinz, Kamin & Wanner (2024), Economic Policy",
            citation: "Chowdhry, S., Hinz, J., Kamin, K. and Wanner, J. (2024). Brothers in arms: The value of coalitions in sanctions regimes. Economic Policy, 39(118), 471-512. doi:10.1093/epolic/eiae019",
            adds: "Transfers within a coalition that equalise the members' welfare change. It recomputes baseline expenditure from shares and does not read productivity_change, population_change or global_value_added_change.",
            scenario_variables,
            source: "R/model_chowdhry_hinz_kamin_wanner_2022.R:11-14,62-76,407-500; R/update_equilibrium.R:235",
        },
        "mahlkow_wanner_2021" => ModelInfo {
            id: "mahlkow_wanner_2021",
            name: "Carbon tax in a climate club with border carbon adjustment (CBAM) and export rebates",
            reference: "Mahlkow & Wanner (2023), CESifo Working Paper 10729",
            citation: "Mahlkow, H. and Wanner, J. (2023). The Carbon Footprint of Global Trade Imbalances. CESifo Working Paper 10729. https://www.cesifo.org/en/publications/2023/working-paper/carbon-footprint-global-trade-imbalances",
            adds: "A carbon tax in a climate club, border carbon tariffs on imports from non-members and export rebates, solved for together with prices. Without a carbon policy it solves the caliendo_parro_2015 equilibrium. Adds outputs for tax revenue, border carbon tariff revenue, export rebate costs, emissions and price levels.",
            scenario_variables,
            source: "R/model_mahlkow_wanner_2021.R:1-100",
        },
        _ => return None,
    };
    Some(m)
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct TableInfo {
    /// `ratio`, `share`, `level_baseline`, `level_counterfactual` or `solver`
    pub kind: &'static str,
    pub unit: &'static str,
    pub description: String,
    pub source: &'static str,
}

/// Units and meaning of a `process_results()` output table.
pub fn table_info(name: &str) -> TableInfo {
    let money = "baseline currency unit, the unit of value_added (synthetic baselines have no real-world unit)";
    let (kind, unit, description, source): (&str, &str, String, &str) = match name {
        "welfare_change" => (
            "ratio",
            "ratio of counterfactual to baseline, 1 = no change",
            "Real income per head: income_change / price_index_change / population_change. Also has a column welfare_change (a copy of value) and weight (the country's share of world baseline value added).".into(),
            "R/process_results.R:345,350-361",
        ),
        "wage_change" | "price_change" | "input_cost_change" => (
            "ratio",
            "ratio of counterfactual to baseline, 1 = no change",
            format!("{name} as solved by the model."),
            "R/model_caliendo_parro_2015.R:49-51",
        ),
        "wage_change_real" => (
            "ratio",
            "ratio of counterfactual to baseline, 1 = no change",
            "Wage change deflated by the consumer price index change.".into(),
            "R/process_results.R:340-342",
        ),
        "price_index_change" => (
            "ratio",
            "ratio of counterfactual to baseline, 1 = no change",
            "Consumer price index change; tax-inclusive in mahlkow_wanner_2021.".into(),
            "R/process_results.R:224-233",
        ),
        "trade_share_new" => (
            "share",
            "share of the destination's expenditure on the sector, between 0 and 1",
            "Counterfactual trade shares by origin, destination and sector.".into(),
            "R/process_results.R:236",
        ),
        "trade_flow" | "trade_flow_new" => (
            if name.ends_with("_new") { "level_counterfactual" } else { "level_baseline" },
            money,
            "Trade value including tariffs: trade_share * expenditure reaching suppliers.".into(),
            "R/process_results.R:189-196",
        ),
        "trade_flow_fob" | "trade_flow_fob_new" => (
            if name.ends_with("_new") { "level_counterfactual" } else { "level_baseline" },
            money,
            "Trade value excluding tariffs: trade_flow / tariff.".into(),
            "R/process_results.R:200-203",
        ),
        "emissions" | "emissions_new" => (
            if name.ends_with("_new") { "level_counterfactual" } else { "level_baseline" },
            "units of carbon, the unit of carbon_intensity",
            "Carbon emissions.".into(),
            "R/model_mahlkow_wanner_2021.R (carbon_policy_outputs_mw_2021)",
        ),
        "price_new" => (
            "level_counterfactual",
            "price level, the unit of the baseline table price",
            "Counterfactual price level of sector goods: price * price_change.".into(),
            "R/model_mahlkow_wanner_2021.R:19-22",
        ),
        "tax_new" | "tariff_new" | "export_subsidy_new" => (
            "level_counterfactual",
            "gross factor (1 = no wedge)",
            format!("Equilibrium {name} including the carbon policy, as solved."),
            "R/model_mahlkow_wanner_2021.R:58-60",
        ),
        "criterion" | "iterations" => (
            "solver",
            "solver diagnostic",
            format!("Solver {name} as returned in the results."),
            "R/update_equilibrium.R:146-150",
        ),
        n if n.ends_with("_change") => (
            "ratio",
            "ratio of counterfactual to baseline, 1 = no change",
            format!("{} counterfactual divided by baseline.", n.trim_end_matches("_change")),
            "R/process_results.R",
        ),
        n if n.ends_with("_new") => (
            "level_counterfactual",
            money,
            format!("{} in the counterfactual.", n.trim_end_matches("_new")),
            "R/process_results.R",
        ),
        n => ("level_baseline", money, format!("{n} in the baseline."), "R/process_results.R"),
    };
    TableInfo {
        kind,
        unit,
        description,
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_model_has_an_entry_and_every_variable_a_description() {
        for m in MODEL_IDS {
            let info = model(m).expect(m);
            assert!(!info.scenario_variables.is_empty());
        }
        for v in all_variables() {
            let _ = var(v);
            assert!(!models_reading(v).is_empty());
        }
    }

    #[test]
    fn variables_match_kite_scenario_variables() {
        // R/update_equilibrium.R:231-243, without the initial conditions
        assert!(
            !model_variables("chowdhry_hinz_kamin_wanner_2022").contains(&"productivity_change")
        );
        assert!(model_variables("mahlkow_wanner_2021").contains(&"carbon_tax"));
        assert!(!model_variables("caliendo_parro_2015").contains(&"carbon_tax"));
        assert_eq!(
            models_reading("coalition_member"),
            vec!["chowdhry_hinz_kamin_wanner_2022"]
        );
    }

    #[test]
    fn change_tables_are_ratios() {
        assert_eq!(table_info("exports_change").kind, "ratio");
        assert_eq!(table_info("income_new").kind, "level_counterfactual");
        assert_eq!(table_info("income").kind, "level_baseline");
    }
}
