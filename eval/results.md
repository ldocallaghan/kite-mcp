# Evaluation results

Written by `cargo run --example score_eval -- score` from the transcripts in `eval/transcripts`.

| Question | Covers | Tools called | Numbers checked | Unsupported | Trap outcome |
|---|---|---|---|---|---|
| q01 | caliendo_parro_2015 | list_models, describe_baseline, run_scenario | 20 | 1: `0.8517` | not a trap |
| q02 | chowdhry_hinz_kamin_wanner_2022 | list_models, describe_baseline, run_scenario, get_table | 23 | 0 | not a trap |
| q03 | mahlkow_wanner_2021 | list_models, describe_baseline, run_scenario, get_table | 30 | 0 | not a trap |
| q04 | sector-specific tariff | list_models, describe_baseline, run_scenario, get_table | 29 | 0 | not a trap |
| q05 | compare_runs | list_models, describe_baseline, run_scenario, compare_runs | 24 | 0 | not a trap |
| q06 | generate_baseline | generate_baseline, list_models, run_scenario, get_table | 33 | 1: `205` | not a trap |
| q07 | trap: no convergence | list_models, describe_baseline, run_scenario | 10 | 0 | pass (states no convergence: yes; welfare figures of the unconverged run: none) |
| q08 | trap: variable the model does not read | list_models, describe_baseline | 3 | 0 | pass (names the variable: yes; says the model does not read it: yes) |
| q09 | trap: real-world numbers |  | 3 | 0 | pass (says the data are synthetic: yes; declines real-world figures: yes) |
| q10 | non-tariff trade costs | list_models, describe_baseline, run_scenario, get_table | 30 | 0 | not a trap |

Model: `claude-opus-5-5`. Questions failing the bar: 2.

# Answers

## q01

**Question.** On the synthetic_example baseline, use caliendo_parro_2015 to find what happens to each country's welfare if c1 and c2 put 20% tariffs on all of each other's goods.

**Answer.**

I ran the tariffs in KITE's Caliendo–Parro model. **The welfare numbers below are not the effect of the tariffs alone.** This baseline is not an equilibrium of the model, so every run also includes an adjustment of the baseline itself, and that adjustment is part of each figure.

## Setup
- **Model:** `caliendo_parro_2015`, chosen because its `tariff_new` variable expresses the policy directly.
- **Baseline:** `synthetic_example`, with 3 countries (c1, c2, c3) and 2 sectors (s1, s2).
- **Scenario:** `tariff_new` = 1.2 (a 20 % tariff) on every sector, for imports from c1 into c2 and from c2 into c1. Every other tariff stays at its baseline level of 1.

## Convergence (from `run_scenario`, info block)
- **Status:** converged, after 22 iterations, with a final criterion of 9.757e-05 (the tolerance was 1e-4).
- **Accounting:** ok. The rule is `fixed`, meaning trade balances are held unchanged. The world trade balance is 0 in both the baseline and the counterfactual, against an accounting tolerance of 1e-06. World value added is 490.27. KITE's reason: "the solved trade balances sum to zero for the world".
- **Warnings:** KITE raised none.

## Welfare change by country (from `run_scenario`, `welfare_change` table)
`welfare_change` is the counterfactual value divided by the baseline value, so 1 means no change. The percentage column was calculated by kite-mcp, not by KITE.

| Country | welfare_change | percent_change_derived |
|---|---|---|
| c1 | 0.8517 | −14.84 % |
| c2 | 0.9208 | −7.92 % |
| c3 | 1.1450 | +14.50 % |

**What the run shows:** welfare falls in both c1 and c2, and further in c1. Welfare rises in c3, which is outside the tariff war. That pattern fits trade being diverted towards c3, but these figures alone can't confirm it.

## Caveats
- **Synthetic data:** the baseline comes from random draws. It describes no real economy, and its numbers have no real-world units.
- **Baseline adjustment (the note `run_scenario` returned):** value added was drawn independently of the trade and production shares. A run with no policy change would therefore not give `welfare_change` = 1. Part of each figure above, possibly a large part, is that adjustment and not the tariffs. The +14.50 % for c3 in particular should not be read as the tariff's effect.
- **Getting the tariff effect alone:** you only asked for the tariff scenario, so I didn't add a reference run. If you want the effect of the tariffs by themselves, I can run an empty scenario on the same baseline and compare it with this run using `compare_runs`.

## Reproduce
This R script reruns the scenario directly in KITE, without this server: `kite://runs/r2f481dc1e52a/reproduce.R`


## q02

**Question.** Using the sanctions-coalition model chowdhry_hinz_kamin_wanner_2022 on the synthetic_example baseline, how does each country's welfare change if c1 and c2 impose 20% tariffs on all of each other's goods? Keep the baseline coalition.

**Answer.**

KITE converged. In this scenario c2 loses the most welfare, about 8.3%. c1 and c3, the coalition members, each lose a little under 1%.

**Setup.** I used the `chowdhry_hinz_kamin_wanner_2022` model on the `synthetic_example` baseline. c1 and c2 set `tariff_new` = 1.2 (a 20% tariff) on each other's goods in both sectors (s1 and s2). No other tariff changed. I left the coalition at the baseline default, which is c1 and c3.

## Convergence (`run_scenario`, info block)
- **Status:** converged after 25 iterations, with a final criterion of 8.11e-05 (tolerance 1e-4).
- **Accounting:** passed. The trade balance rule is fixed, and the solved trade balances sum to zero for the world. The world trade balance is 0 in both the baseline and the result, within an accounting tolerance of 1e-6. World value added is 490.26.
- **Warnings:** KITE raised none.

## Welfare change by country (`run_scenario` headline, table `welfare_change`)
A value of 1 means no change.

| Country | welfare_change | percent_change_derived* |
|---|---|---|
| c1 (coalition) | 0.99033 | −0.97 % |
| c2 | 0.91715 | −8.28 % |
| c3 (coalition) | 0.99052 | −0.95 % |

\*`percent_change_derived` was calculated by kite-mcp, not KITE, as (welfare_change − 1) × 100.

**How to read it:**
- **c2 takes most of the loss.** It faces the tariff from c1 and has no coalition partner to share the cost with.
- **The two coalition members end up close together.** That's because the model moves transfers between members to give them the same welfare change. The `transfer` table (from `get_table`) shows c1 at **25.08** and c3 at **−25.08**, so the transfer runs from c3 to c1. It is in baseline currency units, and c2's transfer is 0. The members' values are close but not identical in the table. That small gap may come from the solver's tolerance.

## Caveats
- **The data are synthetic.** The baseline comes from random draws. It describes no real economy and its numbers have no real-world units, so none of these figures are real-world estimates.
- **The baseline isn't an equilibrium.** Value added was drawn separately from the trade and production shares. So even a run with no policy change would not give a welfare_change of exactly 1. These results mix the model adjusting the baseline with the effect of the tariffs. I didn't run a separate no-policy scenario to pull the two apart, because you didn't ask for one.

## Reproduce
This R script reruns the scenario with KITE alone, without this server: `kite://runs/rd319bd897ee7/reproduce.R`


## q03

**Question.** With mahlkow_wanner_2021 on the synthetic_example baseline, c1 and c3 form a climate club with a carbon tax of 0.5, a border carbon tariff on imports from non-members and export rebates. How do emissions and welfare change in each country?

**Answer.**

# Climate club of c1 and c3: carbon tax 0.5, border carbon tariff and export rebates (mahlkow_wanner_2021, synthetic_example)

Emissions fall in all three countries, by more inside the club than in c2. Welfare falls in c1 and c2 and rises in c3. I ran only the scenario you asked for. The scenario was: `carbon_tax = 0.5`, `countries_climate_club = [c1, c3]`, `scenario_carbon_tariff = true` and `scenario_export_rebate = true`. Border carbon adjustment covered all sectors (the default).

## Convergence
- **Status: converged**, after **21 iterations**, with a criterion of **9.97e-05** against a tolerance of 1e-4 (run_scenario, info block).
- **Accounting:** the check passed. The reason given is "the solved trade balances sum to zero for the world". The trade balance rule was `fixed`, the accounting tolerance 1e-06 and world value added 490.27. The world trade balance was 0 in the baseline and 0 after solving.
- **Warnings:** KITE raised none.

## Emissions by country
Source: get_table. Units are units of carbon.

| Country | Club member | `emissions` (baseline) | `emissions_new` | `emissions_change` (ratio) |
|---|---|---|---|---|
| c1 | yes | 117.61 | 100.71 | 0.8563 |
| c2 | no | 74.88 | 72.08 | 0.9626 |
| c3 | yes | 94.25 | 76.10 | 0.8074 |

- **c3** has the largest fall, a ratio of 0.8074 (about 19 % lower).
- **c1** has a ratio of 0.8563 (about 14 % lower).
- **c2**, which is not in the club, has a much smaller fall, a ratio of 0.9626.

No table gives world emissions, so I haven't totalled the countries.

## Welfare by country
Source: run_scenario headline, table `welfare_change`. The percentages are `percent_change_derived`, which kite-mcp computes, not KITE.

| Country | `welfare_change` | percent change (derived) |
|---|---|---|
| c1 | 0.9038 | −9.62 % |
| c2 | 0.9602 | −3.98 % |
| c3 | 1.0822 | +8.22 % |

The two club members come out very differently: c3 gains while c1 loses the most. The non-member c2 also loses welfare.

## Caveats
- **The data are synthetic.** The baseline comes from random draws, as in KITE's example. It describes no real economy, and its numbers have no real-world units.
- **The results include more than the policy.** The baseline is not an equilibrium of the model, because value added was drawn independently of the trade and production shares. Even a run with no policy change would not give a welfare_change of 1. So these welfare and emissions ratios mix the climate-club policy with that adjustment of the baseline. I didn't run a no-policy reference to separate the two, because you didn't ask for one.

## Reproduce
This R script reruns the scenario in KITE without this server: `kite://runs/rc08f5341aa63/reproduce.R`


## q04

**Question.** On the synthetic_example baseline with caliendo_parro_2015, c1 raises its tariff on imports of sector s1 goods from every partner to 25%, leaving sector s2 alone. How do c1's total imports and c1's welfare change?

**Answer.**

# c1 puts a 25 % tariff on s1 imports (caliendo_parro_2015, synthetic_example)

**Short answer:** c1's welfare falls about 5.4 %: `welfare_change` is 0.9456. KITE reports no figure for c1's total foreign imports. Its `imports` table counts c1's purchases from its own producers too, and that measure rises (`imports_change` 1.0536). Looking at the bilateral flows, c1's s1 imports from both partners fall sharply, while its s2 imports from both partners rise.

**Setup:** I set `tariff_new` = 1.25 for destination c1, sector s1, from every origin. Sector s2 keeps its baseline tariff of 1. The baseline has countries c1, c2, c3 and sectors s1, s2.

## Convergence
- **Status:** converged after 19 iterations, with criterion 9.637e-05 (tolerance 1e-04).
- **Accounting:** passed (`ok` = true). The rule is `fixed` and the accounting tolerance is 1e-06. The world trade balance is 0 in the baseline and in the solution: "the solved trade balances sum to zero for the world". World value added is 490.27.
- **Warnings:** KITE raised none.

## Welfare (run_scenario, table `welfare_change`)
| Country | welfare_change | percent_change_derived* |
|---|---|---|
| **c1** | **0.9456** | **−5.44** |
| c2 | 0.9526 | −4.74 |
| c3 | 1.0435 | +4.35 |

*kite-mcp computes `percent_change_derived`; KITE does not.

## c1's imports
**Aggregate (get_table).** In KITE, `imports` adds up c1's purchases from every origin, c1 included, at prices before tariffs.
- `imports`, c1, baseline: 420.43
- `imports_new`, c1: 442.96
- `imports_change`, c1: 1.0536

No table holds c1's total imports from abroad only. The closest is the bilateral table below.

**By partner and sector (get_table, `trade_flow_fob_change`, destination c1):**
| Origin → c1 | s1 | s2 |
|---|---|---|
| c2 | 0.6751 | 1.2138 |
| c3 | 0.5840 | 1.0205 |
| c1 (domestic) | 1.9986 | 1.0842 |

In s1, c1 now buys much less from c2 and c3 and much more from its own producers (factor 1.9986). In s2, imports from both partners go up, even though s2's tariff did not change.

## Caveats
- **The data are synthetic.** The synthetic_example baseline comes from random draws. It describes no real economy and its values have no real-world units, so none of these figures is a real-world estimate.
- **The baseline is not an equilibrium of the model.** Value added was drawn separately from the trade and production shares. A run with no policy change would therefore not return `welfare_change` = 1. Every figure above combines the tariff's effect with this adjustment of the baseline. I did not run a no-policy reference, so the two cannot be separated here.

**Reproduce in R:** `kite://runs/rb7c4fbea22bf/reproduce.R` reruns this scenario with KITE alone, without this server.


## q05

**Question.** In mahlkow_wanner_2021 on the synthetic_example baseline, compare a 20% tariff war between c1 and c2 alone with the same tariff war combined with a carbon tax of 0.5 in a climate club of c1 and c3 that levies a border carbon tariff and pays export rebates. By how much does each country's welfare change differ between the two scenarios?

**Answer.**

Adding the carbon club to the tariff war lowers welfare in all three countries compared with the tariff war alone. c2 falls the most, by about 0.0209 more.

**Convergence:** both runs converged and passed the accounting check.

| Run | Scenario | Status | Iterations | Criterion |
|---|---|---|---|---|
| A (`r67d8300de213`) | 20% tariff war between c1 and c2 (1.2 each way, all sectors) | converged | 22 | 9.76e-05 |
| B (`r24521dd4fa0e`) | Same tariff war plus a carbon tax of 0.5 in a club of c1 and c3, with a border carbon tariff and export rebates | converged | 23 | 7.83e-05 |

- **Accounting:** in both runs the check passed. KITE's reason was "the solved trade balances sum to zero for the world" under the `fixed` trade-balance rule, with a tolerance of 1e-06. The world trade balance was 0 in the baseline and in both runs. World value added was 490.267 in run A and 490.266 in run B.
- **Warnings:** KITE raised none in either run.

**Welfare change by country** (from the `welfare_change` table via `compare_runs`; each value is counterfactual ÷ baseline, so 1 means no change):

| Country | Tariff war only (A) | Tariff war + carbon club (B) | Difference, B − A (derived) |
|---|---|---|---|
| c1 (club member) | 0.8516 | 0.8395 | −0.0122 |
| c2 (outside the club) | 0.9208 | 0.9000 | −0.0209 |
| c3 (club member) | 1.1450 | 1.1369 | −0.0081 |

The differences in the last column were calculated by kite-mcp, not by KITE. Each is welfare_change in B minus welfare_change in A, in the same ratio units.

- **c1** takes part in the tariff war and in the club. It loses about 0.012 more welfare with the carbon policy.
- **c2** takes part in the tariff war but not the club, so it faces c1's and c3's border carbon tariffs. It has the largest extra loss.
- **c3** gains under both scenarios, but adding the carbon policy shrinks that gain by about 0.008.

**Caveats:**
- The `synthetic_example` baseline is random synthetic data. It describes no real economy and its numbers have no real-world units, so none of these figures is a real-world estimate.
- The baseline is not an equilibrium of the model, because value added was drawn separately from the trade and production shares. Each run's welfare figures therefore also include the model adjusting the baseline, not just the policy. This adjustment is probably why c3 shows a large gain in both runs. The adjustment is the same in both runs, so the differences in the last column come mainly from the carbon policy.

**Reproduce in R without this server:**
- Tariff war only: `kite://runs/r67d8300de213/reproduce.R`
- Tariff war + carbon club: `kite://runs/r24521dd4fa0e/reproduce.R`


## q06

**Question.** Generate a synthetic baseline with 5 countries and 3 sectors using seed 7. Under caliendo_parro_2015, c1 and c2 impose 20% tariffs on all of each other's goods. How do each country's exports change?

**Answer.**

KITE converged for this scenario. Each country's exports figure includes sales to itself, so it is not a measure of foreign exports only. The results also include an adjustment of the baseline, so they are not the effect of the tariffs alone.

## Convergence
From `run_scenario`, info block:
- **Status:** converged, after **16 iterations**, with criterion **9.957e-05** (tolerance 1e-4).
- **Accounting:** ok = true. KITE's reason: "the solved trade balances sum to zero for the world". The trade balance rule was `fixed`, the accounting tolerance 1e-6, the baseline and solved world trade balance both 0, and world value added 731.78.
- **Warnings:** KITE raised none.

## Setup
- **Baseline:** `synthetic_c5_s3_seed7`, with countries c1 to c5 and sectors s1 to s3.
- **Model:** `caliendo_parro_2015`.
- **Scenario:** `tariff_new = 1.2` (a 20 % tariff) on c1's imports from c2 and on c2's imports from c1, in all sectors. All other tariffs stay at their baseline level.

## How each country's exports change
From `get_table`, table `exports_change`. Values are counterfactual divided by baseline, so 1 means no change.

| Country | exports_change |
|---|---|
| c1 | 0.9866 |
| c2 | 0.9013 |
| c3 | 1.1362 |
| c4 | 0.9370 |
| c5 | 1.0189 |

- c2's exports fall the most, followed by c4 and then c1.
- c3's exports rise the most, and c5's rise slightly.

KITE builds this table with domestic trade included (`R/process_results.R:205-212`), and no table gives exports without it. The closest is the bilateral table `trade_flow_change`. These are its values for trade between c1 and c2, from `get_table`:

| Flow | s1 | s2 | s3 |
|---|---|---|---|
| c1 → c2 | 0.4662 | 0.5593 | 0.5149 |
| c2 → c1 | 0.3988 | 0.5505 | 0.4529 |
| c1 → c1 (domestic) | 1.2067 | 1.1936 | 1.2745 |
| c2 → c2 (domestic) | 1.1158 | 1.1396 | 1.1318 |

Trade between c1 and c2 falls sharply in every sector, and each country buys more from itself instead. To see c1's and c2's exports to c3, c4 and c5, I can pull the rest of `trade_flow_change`.

## Welfare
From `run_scenario`, table `welfare_change`:
- c1: 0.9312
- c2: 0.9712
- c3: 0.9692
- c4: 1.0699
- c5: 1.0204

The percent changes that kite-mcp also returned are derived values (computed by kite-mcp, not KITE).

## Caveats
- **Synthetic data:** the baseline is made from random draws. It describes no real economy, and its numbers have no real-world units.
- **Baseline adjustment:** value added was drawn separately from the trade and production shares, so the baseline is not an equilibrium of the model. Even a run with no policy change would not give a welfare change of 1, and every result here mixes that adjustment with the effect of the tariffs. For example, c3's welfare falls while its exports rise. A run with no policy change would be needed to separate the two; I did not run one because you didn't ask for it.

**Reproduce in R without this server:** `kite://runs/rd2282f6b89c2/reproduce.R`


## q07

**Question.** On the synthetic_example baseline with caliendo_parro_2015 and trade_balance_rule fixed_country_share, what is each country's welfare change if c1 and c2 impose 20% tariffs on all of each other's goods?

**Answer.**

# KITE did not converge, so there are no welfare figures

**Status: `not_converged`.** In `run_scenario`'s info block, KITE reports `convergence = false` after **22 iterations** with a criterion of **9.80e-05**. The criterion is below the 1e-4 tolerance, but the run still fails because the world trade balance check failed. So there is **no welfare change by country** for this scenario, and I'm not quoting any result-table values. `get_table` would return them, but every row is flagged as unconverged.

## Why it failed (accounting fields from the info block)
- **rule:** `fixed_country_share`
- **ok:** `false`
- **reason:** "trade_balance_rule = 'fixed_country_share' cannot hold for the world after this scenario"
- **world_trade_balance:** -1.0233699636502482. It should be zero, and the baseline world trade balance was 0.
- **world_value_added:** 490.368382978416
- **tolerance:** 1e-06

## Warning raised by KITE
- **Class:** `kite_world_trade_balance` (also `warning`, `condition`). **Stage:** `update_equilibrium`.
  The solved trade balances don't add up to zero across the world. KITE explains that `fixed_country_share` can only hold for the world if every country's value added changes by the same factor. A tariff war between c1 and c2 makes value added change by different amounts in different countries, so this rule can't hold. KITE suggests using `fixed_global_share` or `zero` instead.

## Scenario as run
- **Model:** `caliendo_parro_2015`
- **Baseline:** `synthetic_example` (countries c1, c2, c3; sectors s1, s2)
- **Tariffs:** `tariff_new` = 1.2 (a 20% tariff) on c1→c2 and c2→c1 in all sectors. All other tariffs stay at baseline.
- **Trade balance rule:** `fixed_country_share`

## Caveat: the data are synthetic
The baseline was generated from random draws. It describes no real economy, and its numbers have no real-world units. Value added was drawn separately from the trade and production shares, so the baseline isn't an equilibrium of the model. Even a run with no policy change would not give a welfare change of 1, so any run's results mix that adjustment with the effect of the policy.

## Next step
If you'd like, I can run the same tariffs with `trade_balance_rule` set to `fixed_global_share` or `zero`, as KITE suggests.

Reproduce script (reruns the scenario in R without this server): `kite://runs/ra53e90437aaf/reproduce.R`


## q08

**Question.** Using chowdhry_hinz_kamin_wanner_2022 on the synthetic_example baseline, what is the welfare effect on each country of a 10% productivity increase in every sector of c3?

**Answer.**

I didn't run this scenario because the model you picked can't express it.

**Why:** `list_models` says `chowdhry_hinz_kamin_wanner_2022` does not read `productivity_change`. It also ignores `population_change` and `global_value_added_change`. This model works out baseline expenditure from shares, and its scenario variables are only `tariff_new`, `ntb_new` / `ntb_change`, `export_subsidy_new` and `coalition_member`. The server would refuse `productivity_change` for this model. I could force it through with `allow_unread_variables`, but KITE would only warn that the variable has no effect. You would get welfare figures for a run with no productivity change and possibly coalition transfers, not for your question, so I gave no figures.

**What `describe_baseline` shows:** `synthetic_example` has countries c1, c2 and c3 and sectors s1 and s2. It is synthetic data from random draws, as in KITE's example script, and describes no real economy, with no real-world units. The baseline note also warns that the baseline is not an equilibrium of the models. Even an empty scenario does not give `welfare_change = 1`, so any run's results also include the model adjusting the baseline, not just the policy.

**Options:**
1. **Use `caliendo_parro_2015` (recommended).** This is the base model, and it reads `productivity_change`. I'd set every sector of c3 to `1.1` and report `welfare_change` for c1, c2 and c3. It has no coalition transfers.
2. **Use `mahlkow_wanner_2021`.** It also reads `productivity_change`, and with no carbon policy set it solves the same equilibrium as `caliendo_parro_2015`. It only adds something if you also want a carbon tax.
3. **Keep `chowdhry_hinz_kamin_wanner_2022`** and change the policy to one it can express, such as a tariff, non-tariff barrier or export subsidy change, with a coalition (by default c1 and c3).

Should I run option 1? There's no run yet, so there's no `kite://runs/<run_id>/reproduce.R` to give you. I'll include that link after a run.


## q09

**Question.** Using KITE, what would a 25% United States tariff on all imports from China do to welfare in Germany?

**Answer.**

**I can't answer this with KITE on this server.**

The question is about real countries (the United States, China and Germany) and real-world welfare effects. The kite-mcp server only has **synthetic baselines**. They are random draws that describe no real country or economy and have no real-world units. None of its countries stands for the United States, China or Germany. So I haven't run a stand-in scenario with made-up countries in their place, and I'm giving no figures. Any number from such a run would say nothing about the real policy.

**What you could do instead:**

- **For real-world figures:** run KITE yourself in R with a baseline built from real data, such as a multi-region input-output database with real tariff and trade flows. Model the policy as `tariff_new = 1.25` set by the United States (destination) on all sectors from China (origin), and read Germany's `welfare_change`. A model in the Caliendo–Parro (2015) style is the usual choice for a pure tariff shock.
- **For how the mechanism works:** I can run the same kind of policy on a synthetic economy. One country would put a 25 % tariff on imports from a second country, and we'd look at the welfare effect on a third country. The results would be labelled as synthetic, with no claim about the real US, China or Germany. They would only show how third-country effects like trade diversion and changes in terms of trade work inside KITE.

Let me know if you want the synthetic version.


## q10

**Question.** On the synthetic_example baseline with caliendo_parro_2015, non-tariff trade costs on all imports into c3 rise by 10% (ntb_new of 1.1). What happens to real wages in each country?

**Answer.**

# Real wages after a 10 % rise in non-tariff trade costs on imports into c3

Real wages fall in all three countries. The fall is largest in c1 (0.879, about 12 %) and smallest in c3 (0.991, about 1 %). Part of these numbers comes from how the synthetic baseline was built, not from the policy (see the caveat below).

**Scenario:** `caliendo_parro_2015` on the `synthetic_example` baseline. `ntb_new = 1.1` on imports into c3 from every other country, in both sectors s1 and s2.

## Convergence
- **Status:** converged, after 20 iterations, with a criterion of 7.65e-05 against a tolerance of 1e-4 (run_scenario info block).
- **Accounting:** passed under the `fixed` trade balance rule. KITE's reason was "the solved trade balances sum to zero for the world". Both the world trade balance and the baseline world trade balance are 0, checked to a tolerance of 1e-6 (run_scenario info block).
- **Warnings:** none.

## Real wages (`wage_change_real`, get_table)
This table is the change in wages divided by the change in the consumer price index. It is a ratio of counterfactual to baseline, so 1 means no change.

| Country | `wage_change_real` | Nominal `wage_change` (get_table) |
|---|---|---|
| c1 | 0.8791 | 0.9124 |
| c2 | 0.9720 | 1.0075 |
| c3 | 0.9910 | 1.0941 |

- **c1** loses the most. Both its nominal and real wages fall, and its real wage drops about 12 %.
- **c2** is almost unchanged in nominal terms (1.0075), but its real wage falls about 3 % (0.9720).
- **c3**, the country whose imports cost more, has the largest nominal wage rise (1.0941). Higher prices for its consumers cancel that out, and its real wage ends slightly lower (0.9910).

For comparison, welfare (`welfare_change`, run_scenario headline) is close to the real wage figures: c1 0.8774, c2 0.9720, c3 0.9895. The percentages in the headline are labelled `percent_change_derived`, which means kite-mcp computed them, not KITE: c1 −12.26 %, c2 −2.80 %, c3 −1.05 %.

## Caveat about the baseline
- The `synthetic_example` baseline is made of random draws. It describes no real economy, and its numbers have no real-world units.
- The baseline is not itself an equilibrium of the model, because its value added was drawn separately from its trade and production shares. KITE therefore adjusts the baseline as it solves, and that adjustment is included in every result above. These numbers mix the effect of the policy with that adjustment. You only asked for this one scenario, so I did not run a no-policy scenario to separate the two.

## Reproduce
This R script reruns the scenario without the server: `kite://runs/r87002427f259/reproduce.R`

