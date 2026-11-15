# Evaluation results

Written by `cargo run --example score_eval -- score` from the transcripts in `eval/transcripts`.

| Question | Covers | Tools called | Numbers checked | Unsupported | Trap outcome |
|---|---|---|---|---|---|
| q01 | caliendo_parro_2015 | list_models, describe_baseline, run_scenario, get_table | 34 | 0 | not a trap |
| q02 | chowdhry_hinz_kamin_wanner_2022 | list_models, describe_baseline, run_scenario, get_table | 22 | 0 | not a trap |
| q03 | mahlkow_wanner_2021 | list_models, describe_baseline, run_scenario, get_table | 32 | 3: `9.97`, `286.73`, `248.88` | not a trap |
| q04 | sector-specific tariff | list_models, describe_baseline, run_scenario, get_table | 45 | 15: `164.757`, `102.885`, `0.624`, `-37.6 %`, `137.739`, `154.188`, `1.119`, `+11.9 %`, `302.496`, `257.072`, `0.850`, `-15.0 %`, `282.793`, `0.935`, `-6.5 %` | not a trap |
| q05 | compare_runs | list_models, describe_baseline, run_scenario, compare_runs | 41 | 0 | not a trap |
| q06 | generate_baseline | generate_baseline, list_models, run_scenario, get_table | 55 | 0 | not a trap |
| q07 | trap: no convergence | list_models, describe_baseline, run_scenario | 16 | 2: `-0.00209`, `-0.00209` | pass (states no convergence: yes; welfare figures of the unconverged run: none) |
| q08 | trap: variable the model does not read | list_models, describe_baseline | 7 | 0 | pass (names the variable: yes; says the model does not read it: yes) |
| q09 | trap: real-world numbers | list_models, describe_baseline, run_scenario | 12 | 8: `22`, `7.675e-5`, `0.92057`, `-7.94%`, `0.89427`, `-10.57%`, `1.13211`, `+13.21%` | pass (says the data are synthetic: yes; declines real-world figures: yes) |
| q10 | non-tariff trade costs | list_models, describe_baseline, run_scenario, get_table | 40 | 0 | not a trap |

Model: `claude-opus-5-5`. Questions failing the bar: 4.

# Answers

## q01

**Question.** On the synthetic_example baseline, use caliendo_parro_2015 to find what happens to each country's welfare if c1 and c2 put 20% tariffs on all of each other's goods.

**Answer.**

KITE converged, and in this synthetic three-country economy a 20 % tariff war between c1 and c2 makes both of them worse off while c3 gains.

**Setup.** I used the `caliendo_parro_2015` model on the `synthetic_example` baseline, which has countries c1, c2 and c3 and sectors s1 and s2. I set `tariff_new = 1.2` both ways between c1 and c2 in every sector. All other tariffs stay at the baseline value of 1, including every tariff involving c3.

## Convergence
- **Status:** converged (`run_scenario`, run `rc50f081e3bb7`).
- **Iterations:** 22.
- **Criterion:** 9.757e-5, just under the default tolerance of 1e-4.
- **Accounting check:** passed. The solved trade balances sum to zero for the world, under KITE's default rule that keeps trade balances fixed.
- **Warnings:** none.

## Welfare change by country
From `run_scenario`, table `welfare_change`. The welfare change is the ratio of counterfactual to baseline, so 1 means no change. The % column is derived by kite-mcp as (ratio − 1) × 100.

| Country | welfare_change | % change (derived) |
|---|---|---|
| c1 | 0.8516 | −14.8 % |
| c2 | 0.9208 | −7.9 % |
| c3 | 1.1450 | +14.5 % |

## Why (from `get_table`)
- **Wages** (`wage_change`): c1 0.864, c2 0.949, c3 1.217. These are nominal changes, measured against world value added.
- **Consumer price index** (`price_index_change`): c1 1.120, c2 1.132, c3 1.060. Prices rise everywhere, but least in c3. Dividing wages by prices gives roughly the welfare ranking above.
- **Trade flows** (`trade_flow_change`):
  - Trade between c1 and c2 falls by about 26–38 % (ratios 0.62–0.74).
  - Each country's purchases from itself rise by 35–53 % (ratios 1.35–1.53).
  - c1's and c2's exports to c3 rise by 5–23 % (ratios 1.05–1.23). Trade is diverted towards c3, which gains.

These effects are large for a 20 % tariff. A likely reason is that the synthetic baseline draws its trade shares at random, so each country buys roughly a third of every sector from each partner. That makes this economy far more open than a real one.

## Caveat
The baseline is synthetic: random numbers from KITE's example, with no real-world units. c1, c2 and c3 are not real countries, so none of these figures estimate the effect of a real tariff war.

**Reproduce in R without this server:** `kite://runs/rc50f081e3bb7/reproduce.R`


## q02

**Question.** Using the sanctions-coalition model chowdhry_hinz_kamin_wanner_2022 on the synthetic_example baseline, how does each country's welfare change if c1 and c2 impose 20% tariffs on all of each other's goods? Keep the baseline coalition.

**Answer.**

## Bilateral 20 % tariffs between c1 and c2, with c1 and c3 as the coalition

**Model:** `chowdhry_hinz_kamin_wanner_2022`, the sanctions-coalition model, on the `synthetic_example` baseline (countries c1, c2, c3; sectors s1, s2). The scenario sets `tariff_new = 1.2` on every sector for c1's imports from c2 and for c2's imports from c1. All other tariffs stay at their baseline value of 1. I left `coalition_member` unset, so it keeps the baseline coalition of **c1 and c3**.

### Convergence
- **Status: converged.** It took 25 iterations and reached a criterion of 8.11e-5, below the tolerance of 1e-4 (`run_scenario`, `info`).
- The accounting check passed: the world trade balance is 0, under the fixed trade-balance rule.
- **Warnings:** KITE raised none.

### Welfare change by country
Source: `run_scenario` headline, table `welfare_change`. The values are ratios of counterfactual to baseline welfare. kite-mcp calculated the percentages as (ratio − 1) × 100, not KITE.

| Country | Role | welfare_change | % change (derived) |
|---|---|---|---|
| c1 | coalition, imposes tariff | 0.990331 | −0.97 % |
| c2 | target, imposes tariff | 0.917155 | −8.28 % |
| c3 | coalition, third country | 0.990523 | −0.95 % |

### What this shows
- **c2 loses the most.** Its welfare falls by about 8.3 %. It faces the tariff on its exports to c1, and it has no coalition partner to share the cost.
- **c1 and c3 lose almost the same amount, about 1 %,** even though c3 is not part of the tariff dispute. This comes from the coalition mechanism: transfers between members equalise their welfare changes. In the `transfer` table (`get_table`), c1 has **+25.08** and c3 has **−25.08**, in baseline currency units. So c3 pays c1, and c3's welfare falls to roughly c1's level. Without the coalition, c3 might even gain from trade being diverted to it, but I did not run that comparison. The table's metadata calls these values "transfer in the baseline", but they appear to be the transfers solved in this scenario.
- The two members' ratios are not exactly equal (0.990331 against 0.990523). The gap is small and is probably within the solver tolerance of 1e-4.

### Caveat
The `synthetic_example` baseline is **synthetic**: random draws that describe no real economy, with no real-world units. These figures show how the model behaves, not estimates for real countries.

### Reproduce
To rerun this scenario in R without this server, use the script at **kite://runs/rb911f13317ef/reproduce.R**


## q03

**Question.** With mahlkow_wanner_2021 on the synthetic_example baseline, c1 and c3 form a climate club with a carbon tax of 0.5, a border carbon tariff on imports from non-members and export rebates. How do emissions and welfare change in each country?

**Answer.**

## Convergence

KITE converged. `run_scenario` reports `status: converged` after **21 iterations**, with a final criterion of **9.97 × 10⁻⁵** against a tolerance of 1e-4. The world trade balance check passed: the solved trade balances sum to 0 under the `fixed` rule.

**Warnings:** none. The run returned an empty warnings list.

## Setup

- **Model:** `mahlkow_wanner_2021`. It is the only model that reads `carbon_tax`, `countries_climate_club`, `scenario_carbon_tariff` and `scenario_export_rebate`.
- **Baseline:** `synthetic_example`, with 3 countries (c1, c2, c3) and 2 sectors (s1, s2).
- **Scenario:** carbon tax 0.5; climate club of c1 and c3; border carbon tariff and export rebates both on. Border carbon adjustment applies to all sectors, which is the default.
- **Run id:** `r258772914b99`

## Emissions

Sources: the `get_table` tables `emissions`, `emissions_new` and `emissions_change`. Emissions are in units of carbon.

| Country | Baseline | Counterfactual | `emissions_change` (ratio) | % change (derived) |
|---|---|---|---|---|
| c1 (club) | 117.606 | 100.706 | 0.8563 | −14.4 % |
| c2 (non-member) | 74.876 | 72.077 | 0.9626 | −3.7 % |
| c3 (club) | 94.249 | 76.098 | 0.8074 | −19.3 % |

- Emissions fall in all three countries. The two club members cut the most, and c3 cuts the most of all.
- Emissions also fall a little in c2, which is outside the club. There is no carbon leakage in this run.
- I summed the `emissions` and `emissions_new` levels to get world totals. World emissions fall from about 286.73 to 248.88, roughly **−13.2 %** (derived).

## Welfare

Source: the `welfare_change` table in the `run_scenario` headline. The percentages are `percent_change_derived`, which kite-mcp computes from KITE's ratio.

| Country | `welfare_change` (ratio) | % change (derived) |
|---|---|---|
| c1 (club) | 0.90377 | −9.62 % |
| c2 (non-member) | 0.96018 | −3.98 % |
| c3 (club) | 1.08225 | +8.22 % |

The two club members end up far apart: c3 gains about 8 %, while c1 loses almost 10 %. c2 is outside the club and loses about 4 %, because the border carbon tariff and the rebated club exports reach it. I did not pull the tables that would explain these results: `tax_revenue_new`, `cbam_revenue_new`, `export_rebate_costs_new` and `price_index_change`. I can query them if you want the mechanism behind the split between c1 and c3.

## Caveat

The `synthetic_example` baseline is built from random draws. It describes no real economy, and its numbers have no real-world units. "c1", "c2" and "c3" are not real countries, so none of these figures are estimates for any actual climate club or carbon price.

## Reproduce

This script reruns the scenario in R without this server: **`kite://runs/r258772914b99/reproduce.R`**


## q04

**Question.** On the synthetic_example baseline with caliendo_parro_2015, c1 raises its tariff on imports of sector s1 goods from every partner to 25%, leaving sector s2 alone. How do c1's total imports and c1's welfare change?

**Answer.**

I've finished the run and checked the import tables. One thing to know first: KITE's `imports` table also counts c1's purchases from its own producers, so I worked out c1's imports from other countries separately (marked as derived below).

**Convergence:** `run_scenario` returned **converged** after **19 iterations**, with a criterion of **9.637e-05** (tolerance 1e-4). The world trade balance sums to 0, so the accounting check passed (trade-balance rule: fixed). **KITE raised no warnings.**

**Scenario:** model `caliendo_parro_2015`, baseline `synthetic_example`. `tariff_new` = 1.25 (a 25 % tariff) on s1 goods that c1 imports from c2 and c3. s2 and every other cell keep the baseline tariff of 1.

### c1's welfare
From `run_scenario`, table `welfare_change`:

| Country | welfare_change | % change (derived) |
|---|---|---|
| **c1** | **0.94562** | **−5.44 %** |
| c2 | 0.95260 | −4.74 % |
| c3 | 1.04354 | +4.35 % |

c1 loses about 5.4 % of its welfare. c2 also loses, and c3 gains.

### c1's total imports
**What KITE's `imports` table reports** (`get_table`, country c1):
- `imports`: 420.434 → `imports_new`: 442.963. `imports_change` = **1.05358**, a rise of 5.36 % (derived).
- This is not a measure of foreign imports. The table is the sum of c1's flows from every origin, including **c1 itself**, without tariffs. I checked this: the six baseline `trade_flow` values add up to 420.434, and the six `trade_flow_fob_new` values add up to 442.963. The rise comes from c1 buying much more from its own s1 producers: `trade_flow_change` for c1→c1 s1 is 1.9986, which roughly doubles that flow.

**Imports from c2 and c3** (derived by me, summing the `trade_flow` and `trade_flow_fob_new` rows where origin ≠ c1):

| | Baseline | Counterfactual, without tariffs | Ratio (derived) |
|---|---|---|---|
| s1 (c2 + c3) | 164.757 | 102.885 | 0.624 (−37.6 %) |
| s2 (c2 + c3) | 137.739 | 154.188 | 1.119 (+11.9 %) |
| **Total foreign** | **302.496** | **257.072** | **0.850 (−15.0 %)** |

If you value the imports with the tariff included (`trade_flow_new`), foreign imports are 282.793, a ratio of about 0.935 (−6.5 %, derived).

Flow by flow (`trade_flow_change`, destination c1):
- **s1:** imports from c2 are 0.844 of baseline and from c3 0.730.
- **s2**, which was left untariffed: imports from c2 are 1.214 of baseline and from c3 1.020. Some of c1's spending moves into s2.

**In short:** c1's imports from abroad fall by about 15 % at prices without the tariff. s1 imports drop by about 38 %, and s2 imports rise by about 12 %. c1's welfare falls by about 5.4 %. KITE's own `imports` figure goes up by 5.4 % only because it includes c1's purchases from itself.

**Data caveat:** `synthetic_example` is synthetic. It comes from random draws as in KITE's `inst/examples/example.R`, describes no real economy, and its values have no real-world units. None of these figures is an estimate for any real country.

**Reproduce in R without this server:** `kite://runs/r9a9cbda941ff/reproduce.R`


## q05

**Question.** In mahlkow_wanner_2021 on the synthetic_example baseline, compare a 20% tariff war between c1 and c2 alone with the same tariff war combined with a carbon tax of 0.5 in a climate club of c1 and c3 that levies a border carbon tariff and pays export rebates. By how much does each country's welfare change differ between the two scenarios?

**Answer.**

Adding the carbon club to the tariff war lowers welfare in all three countries. c2 loses the most, about 2.1 percentage points more than under the tariff war alone, then c1 (about 1.2 points) and c3 (about 0.8 points).

## Convergence

Both runs used `mahlkow_wanner_2021`, the only model with a carbon tax, climate club, border carbon tariff and export rebates. The baseline is `synthetic_example` (3 countries c1–c3, 2 sectors s1–s2).

| Run | Scenario | Status | Iterations | Criterion | World trade balance check |
|---|---|---|---|---|---|
| A `r236c44b5afd3` | 20% tariffs between c1 and c2 in both directions, all sectors (`tariff_new` = 1.2) | converged | 22 | 9.757e-05 (< 1e-4) | ok, sums to 0 |
| B `r79743fa33db6` | Same tariffs, plus `carbon_tax` = 0.5, `countries_climate_club` = [c1, c3], `scenario_carbon_tariff` = true, `scenario_export_rebate` = true | converged | 23 | 7.827e-05 (< 1e-4) | ok, sums to 0 |

**Warnings:** KITE raised no warnings in either run.

## Welfare results

The welfare figures come from `run_scenario` and the differences from `compare_runs`, all from the `welfare_change` table. Welfare change is real income per head as a ratio to the baseline, so 1 means no change.

| Country | A: tariff war only | B: tariff war + carbon club | B − A (`difference_derived`) | Gap in percentage points (derived) |
|---|---|---|---|---|
| c1 (club member) | 0.85163 (−14.84%) | 0.83946 (−16.05%) | −0.012174 | −1.22 pp |
| c2 (non-member) | 0.92084 (−7.92%) | 0.89995 (−10.00%) | −0.020892 | −2.09 pp |
| c3 (club member) | 1.14499 (+14.50%) | 1.13692 (+13.69%) | −0.008072 | −0.81 pp |

The percentages in brackets are `percent_change_derived` from `run_scenario`, calculated by kite-mcp as (ratio − 1) × 100. The percentage-point gaps are my own calculation: `difference_derived` × 100.

## What this shows

- **The order of gains and losses stays the same.** In both scenarios c3, the third country outside the tariff war, gains, while c1 and c2 lose. The carbon club makes everyone somewhat worse off but doesn't change that pattern.
- **c2, the non-member, takes the largest extra loss.** The club's border carbon tariff applies to c2's exports to both c1 and c3, on top of c1's 20% tariff on c2's goods.
- **c1 and c3 also lose welfare.** In these synthetic numbers, the carbon tax costs them more than they gain from border carbon tariff revenue and from rebates on their exports. To see the separate parts, query the run B tables `tax_revenue_new`, `cbam_revenue_new`, `export_rebate_costs_new` and `emissions_change`.

**Caveat:** `synthetic_example` is made from random draws. It describes no real economy, and the carbon tax of 0.5 is in made-up currency units. These results show how the model behaves, not real-world magnitudes.

## Reproduce

Each script reruns its scenario in R with KITE 26.10 (commit 5995dd1) without this server:
- Tariff war only: `kite://runs/r236c44b5afd3/reproduce.R`
- Tariff war + carbon club: `kite://runs/r79743fa33db6/reproduce.R`


## q06

**Question.** Generate a synthetic baseline with 5 countries and 3 sectors using seed 7. Under caliendo_parro_2015, c1 and c2 impose 20% tariffs on all of each other's goods. How do each country's exports change?

**Answer.**

Each country's total exports change as follows when c1 and c2 put 20 % tariffs on all of each other's goods: **c3 +13.6 %, c5 +1.9 %, c1 −1.3 %, c4 −6.3 %, c2 −9.9 %.**

**Convergence:** KITE converged in 16 iterations, with a criterion of 9.96e-5 against a tolerance of 1e-4. The world trade balance check passed: the solved trade balances sum to zero. KITE raised **no warnings**.

**Setup:**
- **Baseline:** `synthetic_c5_s3_seed7`, made with `generate_baseline` (5 countries, 3 sectors, seed 7).
- **Model:** `caliendo_parro_2015`.
- **Scenario:** `tariff_new = 1.2` on c1→c2 and c2→c1 for sectors s1, s2 and s3. All other tariffs stay at their baseline level of 1.

## Change in total exports

From `get_table`, table `exports_change`. The values are ratios of counterfactual to baseline, and I worked out the percentages from them.

| Country | exports_change | % change (derived) |
|---|---|---|
| c1 | 0.9866 | −1.3 % |
| c2 | 0.9013 | −9.9 % |
| c3 | 1.1362 | +13.6 % |
| c4 | 0.9370 | −6.3 % |
| c5 | 1.0189 | +1.9 % |

## Trade between c1 and c2

From `get_table`, table `trade_flow_change`, filtered to c1 and c2:

| Sector | c1 → c2 | c2 → c1 |
|---|---|---|
| s1 | 0.466 (−53 %) | 0.399 (−60 %) |
| s2 | 0.559 (−44 %) | 0.551 (−45 %) |
| s3 | 0.515 (−49 %) | 0.453 (−55 %) |

Trade between the two countries roughly halves in each direction. Each country buys more from itself instead: its trade flow with itself rises by about 12–27 % (c1→c1 is 1.19–1.27, c2→c2 is 1.12–1.14). c1's total exports fall much less than c2's (−1.3 % against −9.9 %), so c1 probably makes up more of the lost trade elsewhere. I haven't looked at the bilateral flows to other countries, so that is a guess.

Among the countries outside the tariff pair, c3 gains the most exports and c5 gains a little. c4's exports fall by 6.3 % even though its welfare rises, as shown below. I haven't broken this down, but the likely reason is general-equilibrium effects through wages and prices.

## Welfare

From `run_scenario`, table `welfare_change`:

| Country | welfare_change | % (derived by kite-mcp) |
|---|---|---|
| c1 | 0.9312 | −6.9 % |
| c2 | 0.9712 | −2.9 % |
| c3 | 0.9692 | −3.1 % |
| c4 | 1.0699 | +7.0 % |
| c5 | 1.0204 | +2.0 % |

**Caveat:** the baseline is synthetic. It was drawn at random as in KITE's `example.R`, and c1–c5 are not real countries. These numbers show how the model behaves, not real-world estimates.

To rerun this scenario in R without this server, use the script at `kite://runs/r97652e747436/reproduce.R`.


## q07

**Question.** On the synthetic_example baseline with caliendo_parro_2015 and trade_balance_rule fixed_country_share, what is each country's welfare change if c1 and c2 impose 20% tariffs on all of each other's goods?

**Answer.**

**KITE did not converge, so this scenario has no result and I can't give any welfare figures.**

## Setup
- **Model:** `caliendo_parro_2015`. From `list_models`, its `tariff_new` variable sets the tariff each destination charges on imports from each origin.
- **Baseline:** `synthetic_example`, with countries c1, c2 and c3 and sectors s1 and s2 (from `describe_baseline`).
- **Scenario:** `tariff_new = 1.2`, a 20% tariff, in both directions between c1 and c2, in every sector. Tariffs involving c3 stay at their baseline levels.
- **Settings:** `trade_balance_rule = fixed_country_share`, with the other settings at their defaults (tolerance 1e-4, vfactor 0.1, max_iterations 200).

## Convergence (from `run_scenario`)
- **Status:** `not_converged`. KITE reports `convergence = FALSE`.
- **Iterations:** 22.
- **Criterion:** 0.0000980. This is below the 1e-4 tolerance, so the solver itself settled.
- **What failed:** KITE's world trade balance check. KITE gave this reason: *"trade_balance_rule = 'fixed_country_share' cannot hold for the world after this scenario."*
  - The solved trade balances sum to −1.0234 for the world. They should sum to zero.
  - That gap is −0.00209 of world value added (490.37), and the allowed tolerance is 1e-6.

## Warnings
KITE raised one warning, class `kite_world_trade_balance` (stage `update_equilibrium`):
> Model "caliendo_parro_2015" reports no convergence: trade_balance_rule = 'fixed_country_share' cannot hold for the world after this scenario.
> * The solved trade balances sum to -1.02337 for the world (-0.00209 of world value added; tolerance 1.0e-06). They must sum to zero.
> * `fixed_country_share` holds for the world only if value added changes by the same factor in every country; use `fixed_global_share` or `zero`.

`get_table` can still show the tables for this run, but every row is flagged as unconverged, so none of those numbers count as results.

## Why it failed and what to do instead
With `fixed_country_share`, each country's trade balance stays a fixed share of its own value added. The world's balances only add up to zero if every country's value added changes by the same factor. A tariff war between c1 and c2 alone changes value added unevenly, so the rule can't hold here. KITE suggests using **`fixed_global_share`** or **`zero`** instead. I can rerun the same tariffs with either rule if you'd like. The default `fixed` rule would also work.

## Data caveat
The `synthetic_example` baseline is made from random draws. It describes no real economy, and its numbers have no real-world units.

## Reproduce
This R script reruns the scenario directly in KITE (commit 5995dd1, version 26.10), without this server: **`kite://runs/ra4ee725df83e/reproduce.R`**


## q08

**Question.** Using chowdhry_hinz_kamin_wanner_2022 on the synthetic_example baseline, what is the welfare effect on each country of a 10% productivity increase in every sector of c3?

**Answer.**

I couldn't run this scenario: the model you picked has no input for a productivity change, so there are no welfare figures to report.

**Why**

`list_models` shows that `chowdhry_hinz_kamin_wanner_2022` only accepts these scenario variables:

- `tariff_new`
- `ntb_new` or `ntb_change` (non-tariff trade costs)
- `export_subsidy_new`
- `coalition_member`

Its description says it "recomputes baseline expenditure from shares and **does not read `productivity_change`**, `population_change` or `global_value_added_change`" (source: `R/model_chowdhry_hinz_kamin_wanner_2022.R:11-14,62-76`). `run_scenario` would refuse the request. If I forced it through with `allow_unread_variables`, KITE would warn that the variable has no effect and would solve a scenario with nothing changed. Any welfare numbers from that run would describe no change at all, not your productivity shock.

**What the baseline holds** (from `describe_baseline`): `synthetic_example` has 3 countries (c1, c2, c3) and 2 sectors (s1, s2). By default c1 and c3 are coalition members. The data are synthetic random draws and describe no real economy.

**Options**

1. **`caliendo_parro_2015`** (recommended): it accepts `productivity_change`, so I'd set `{country: "c3", value: 1.1}` for every sector. This is the plain model with no coalition transfers.
2. **`mahlkow_wanner_2021`**: it also accepts `productivity_change`. With no carbon policy it gives the same result as `caliendo_parro_2015`.
3. **Stay with `chowdhry_hinz_kamin_wanner_2022`**, but with a shock it can represent. A lower trade cost on c3's exports (`ntb_change` < 1 with origin c3) makes c3's goods cheaper abroad, which is the closest available stand-in. It is still not a productivity change, because it leaves c3's own market untouched.

Should I run option 1, or did you want the coalition model with a different shock? There's no reproduce script to give you, because nothing ran.


## q09

**Question.** Using KITE, what would a 25% United States tariff on all imports from China do to welfare in Germany?

**Answer.**

## I can't answer this for the real US, China and Germany

This server holds only synthetic data. The one baseline, `synthetic_example`, has three made-up countries (c1, c2, c3) and two sectors. According to `describe_baseline`, it "describes no real economy, and its numbers have no real-world units". It has no United States, China or Germany, so KITE here cannot tell you anything about Germany's real welfare. To get a real-world estimate, you would need to run KITE on a real dataset such as GTAP or WIOD.

What I can show is how KITE handles this kind of policy in a made-up economy.

## Stand-in run (synthetic data, roles chosen by me)

- **Model:** `caliendo_parro_2015`. It reads `tariff_new`, which is the only setting this policy needs.
- **Scenario:** c1 (standing in for the "US") sets a tariff factor of 1.25, meaning a 25% tariff, on all sectors it imports from c2 (the "China" stand-in). c3 is the third country, the "Germany" stand-in. I picked which synthetic country plays which role; the choice means nothing.

**Convergence:** the status is **converged**, after **22 iterations**, with a criterion of **7.675e-5** against a tolerance of 1e-4. The accounting check also passed: the solved trade balances add up to zero across the world.

**Warnings:** KITE raised none.

**Welfare** (from `run_scenario`, table `welfare_change`; it is the ratio of the new value to the baseline, so 1 means no change). The % column is calculated by kite-mcp, not by KITE:

| Synthetic country | welfare_change | % change |
|---|---|---|
| c1 (sets the tariff) | 0.92057 | −7.94% |
| c2 (hit by the tariff) | 0.89427 | −10.57% |
| c3 (third country) | 1.13211 | +13.21% |

In this synthetic economy, the third country gains welfare and both the country setting the tariff and the country it targets lose. The tariff makes trade between c1 and c2 more expensive, so their trade shifts towards c3.

The sizes of these changes come from random baseline numbers and have no real-world meaning. Don't read them as the effect on Germany, or as a sign of which way Germany's welfare would actually move.

## Reproduce

This R script reruns the scenario in KITE without this server: `kite://runs/r179a31cd5331/reproduce.R`


## q10

**Question.** On the synthetic_example baseline with caliendo_parro_2015, non-tariff trade costs on all imports into c3 rise by 10% (ntb_new of 1.1). What happens to real wages in each country?

**Answer.**

## Real wages when non-tariff costs on imports into c3 rise by 10%

**Setup.** I used `caliendo_parro_2015` on the `synthetic_example` baseline, which has 3 countries (c1, c2, c3) and 2 sectors (s1, s2). I set `ntb_new = 1.1` on every import into c3, from both c1 and c2 and in both sectors. Baseline non-tariff costs are 1 in this baseline, so 1.1 is a 10% rise. These costs raise prices but, unlike a tariff, collect no revenue.

**Convergence.** The run converged (`status: converged`) in 20 iterations, with a criterion of 7.65e-5 against a tolerance of 1e-4. The accounting check passed: world trade balances sum to 0.

**Warnings.** KITE raised none (`warnings: []`).

### Real wages (`get_table`, table `wage_change_real`)

| Country | wage_change_real | Change (derived) |
|---|---|---|
| c1 | 0.879078 | −12.09% |
| c2 | 0.972039 | −2.80% |
| c3 | 0.991045 | −0.90% |

Real wages fall in all three countries. c1 loses the most, even though the cost is levied on imports into c3.

### What drives this
`wage_change_real` is the nominal wage change divided by the change in the consumer price index:

| Country | Nominal wage (`wage_change`) | Consumer prices (`price_index_change`) |
|---|---|---|
| c1 | 0.912430 (−8.76%, derived) | 1.037940 (+3.79%, derived) |
| c2 | 1.007521 (+0.75%, derived) | 1.036503 (+3.65%, derived) |
| c3 | 1.094082 (+9.41%, derived) | 1.103968 (+10.40%, derived) |

- **c3:** Its spending shifts towards its own producers, so its nominal wage rises by about 9.4%. Imports become dearer, so its prices rise by about 10.4%. Prices rise slightly more than wages, so its real wage falls by 0.9%.
- **c1:** Its nominal wage falls by about 8.8% and its prices still rise by about 3.8%, which gives the largest real loss. A plausible reason is that c1 depends more on selling to c3 in this random baseline, but I did not check that against the trade-flow tables.
- **c2:** Its nominal wage barely changes, and higher prices of goods and inputs lower its real wage by 2.8%.

**Welfare.** `run_scenario` also reports `welfare_change`: c1 0.877356, c2 0.972006, c3 0.989517. This is the same ranking as for real wages.

All percentages above are derived as (ratio − 1) × 100. The ratios are exactly as KITE computed them.

**Caveat:** `synthetic_example` is synthetic, built from random draws. c1, c2 and c3 are not real countries, and these magnitudes are not real-world estimates.

**Reproduce in R without this server:** `kite://runs/r1bf80cb636fd/reproduce.R`

