# Evaluation results

Written by `cargo run --example score_eval -- score` from the transcripts in `eval/transcripts`.

| Question | Covers | Tools called | Numbers checked | Unsupported | Trap outcome |
|---|---|---|---|---|---|
| h01 | generate_baseline, unilateral tariff | generate_baseline, list_models, run_scenario | 20 | 0 | not a trap |
| h02 | mahlkow_wanner_2021, carbon tax without border measures | list_models, describe_baseline, run_scenario, get_table | 26 | 0 | not a trap |
| h03 | chowdhry_hinz_kamin_wanner_2022, changed coalition | list_models, describe_baseline, run_scenario | 19 | 0 | not a trap |
| h04 | caliendo_parro_2015, productivity shock | list_models, describe_baseline, run_scenario, get_table | 29 | 0 | not a trap |
| h05 | trap: variable the model does not read | list_models, describe_baseline | 1 | 0 | pass (names the variable: yes; says the model does not read it: yes) |

Model: `claude-opus-5-5`. Questions failing the bar: 0.

# Answers

## h01

**Question.** Generate a synthetic baseline with 4 countries and 2 sectors using seed 11. Under caliendo_parro_2015, c3 imposes a 15% tariff on all imports from c1. How does each country's welfare change?

**Answer.**

The run converged, but these welfare changes are not the effect of the tariff alone. This synthetic baseline is not an equilibrium of the model, so the figures also include the model moving to its own equilibrium, and I can't separate the two parts from this one run.

**Setup:** I generated the baseline `synthetic_c4_s2_seed11` (countries c1–c4, sectors s1–s2). I used the `caliendo_parro_2015` model with `tariff_new` = 1.15 on every sector for goods going from c1 into c3. That is a 15 % tariff; all other tariffs stay at their baseline level of 1.

## Convergence
- **Status:** converged after **19** iterations, with a convergence criterion of **8.223e-5** (tolerance 1e-4).
- **Trade balance check:** passed. The solved trade balances add up to zero for the world (rule: fixed balances; world trade balance **0**, tolerance **1.000e-6**). World value added is **610.5**.
- **Warnings:** KITE raised none.

## Welfare change by country
Source: `run_scenario`, table `welfare_change`. The value is counterfactual welfare divided by baseline welfare, so below 1 is a fall and 1 is no change. kite-mcp calculated the percentage changes, not KITE.

| Country | welfare_change | % change (derived) |
|---|---|---|
| c1 | 0.9749 | -2.505 % |
| c2 | 1.133 | 13.33 % |
| c3 | 0.8923 | -10.77 % |
| c4 | 0.9844 | -1.558 % |

Taken at face value:
- **c2** gains.
- **c3**, which sets the tariff, has the largest fall.
- **c1**, the country being taxed, also falls, and so does **c4**.

## Caveats
- **Synthetic data:** the baseline comes from random draws. It describes no real economy, and its numbers have no real-world units. None of these figures is a real-world estimate.
- **What the figures include:** the baseline's value added is drawn separately from its trade and production shares, so the baseline doesn't balance on its own terms. Running an empty scenario would not give welfare_change = 1. Some of the gains and losses above, possibly large ones like c2's, may come from this adjustment rather than from the tariff. To isolate the tariff's effect, I can run a scenario with no policy change on the same baseline and compare the two with `compare_runs`.

**To reproduce:** `kite://runs/r10706f3c3eca/reproduce.R` reruns the scenario in R with KITE alone, without this server.


## h02

**Question.** On the synthetic_example baseline with mahlkow_wanner_2021, every country levies a carbon tax of 0.3, with no border carbon tariff and no export rebates. How do each country's emissions and welfare change?

**Answer.**

A uniform carbon tax of 0.3 in all three countries cuts emissions in every country. Welfare falls in c1 and c2 and rises in c3. Part of each welfare figure comes from the baseline adjusting to equilibrium, not from the tax (see the caveats below).

## Convergence
- **Status:** converged, after **21** iterations, with criterion **8.856e-5** (the tolerance was 1e-4).
- **Accounting check:** passed. KITE reports that the solved trade balances sum to zero for the world. The trade-balance rule was `fixed` and the accounting tolerance was **1.000e-6**. The world trade balance was **0** in the baseline and **0** in the counterfactual, and world value added was **490.3**.
- **Warnings:** KITE raised none, at any stage.

## Scenario
- **Model:** `mahlkow_wanner_2021`
- **Baseline:** `synthetic_example`, with 3 countries (c1, c2, c3) and 2 sectors (s1, s2)
- **Policy:** `carbon_tax = 0.3`. Every country is in the climate club, which is the default. `scenario_carbon_tariff = false` and `scenario_export_rebate = false`.

## Results by country

| Country | Emissions change (ratio) | Emissions % (derived) | Welfare change (ratio) | Welfare % (derived) |
|---|---|---|---|---|
| c1 | 0.9070 | -9.303 % | 0.9005 | -9.952 % |
| c2 | 0.9004 | -9.963 % | 0.9826 | -1.739 % |
| c3 | 0.8471 | -15.29 % | 1.080 | 7.987 % |

Sources:
- **Emissions:** `get_table`, table `emissions_change`. The ratio is counterfactual emissions over baseline emissions.
- **Welfare:** `run_scenario`, headline table `welfare_change`.
- The percentage columns were computed by kite-mcp, not by KITE, so they are derived.
- For both measures a ratio below 1 is a fall and 1 means no change.

**What the results show:**
- **Emissions** fall in all three countries. c3 has the largest fall and c1 the smallest.
- **Welfare** falls in c1 and c2, with c1 hit much harder, and rises in c3.
- Because every country is in the club and there is no border adjustment, the differences between countries come from the synthetic baseline's price levels, input–output structure and trade patterns. They are not caused by any difference in policy between countries.

## Caveats
- **The data are synthetic.** The baseline comes from random draws. It describes no real economy, and its numbers have no real-world units, so none of these figures is a real-world estimate.
- **The welfare figures include more than the tax.** The synthetic baseline draws value added separately from the trade and production shares, so the baseline is not an equilibrium of the model. Even an empty scenario would not give a welfare change of 1. Each figure above therefore mixes the effect of the carbon tax with the model adjusting the baseline to equilibrium. You asked for no comparison, so I did not run a no-policy reference to separate the two.

## Reproduce
This R script reruns the scenario with KITE alone, without this server: `kite://runs/rcd67b4df6b86/reproduce.R`


## h03

**Question.** With chowdhry_hinz_kamin_wanner_2022 on the synthetic_example baseline, c1 and c2 form the coalition and c3 is outside it. c3 imposes a 20% tariff on all imports from c1 and from c2. How does welfare change in each country?

**Answer.**

KITE converged on this scenario: the coalition members c1 and c2 both lose welfare, and c3 gains.

**Setup:** I used the `chowdhry_hinz_kamin_wanner_2022` model on the `synthetic_example` baseline, which has countries c1, c2 and c3 and sectors s1 and s2. The coalition is c1 and c2. In the baseline it is c1 and c3, so I overrode it. c3 sets a gross tariff of 1.2 (a 20 % tariff) on imports from c1 and from c2 in both sectors. No other cells change.

## Convergence
- **Status:** converged, after 22 iterations, with a criterion of 8.375e-5 against a tolerance of 1e-4.
- **Accounting check:** passed under the `fixed` trade-balance rule (trade balances stay unchanged). The solved trade balances add up to zero for the world. The world trade balance is 0 in the baseline and 0 after the policy, the tolerance is 1.000e-6, and world value added is 490.3.
- **Warnings:** KITE raised none.

## Welfare change by country
These come from the `welfare_change` table returned by `run_scenario`. The values are ratios of counterfactual to baseline, so below 1 is a fall. kite-mcp computed the percentages from KITE's ratios.

| Country | Role | welfare_change | % change (derived) |
|---|---|---|---|
| c1 | coalition | 0.8976 | -10.24 % |
| c2 | coalition | 0.8978 | -10.22 % |
| c3 | outside, imposes the tariff | 1.166 | 16.64 % |

**What the results show:**
- **c3 gains** from tariffing both of its trading partners.
- **c1 and c2 lose by almost the same amount.** The model adds transfers between coalition members that are meant to give each member the same welfare change. Their results are close but not identical: 0.8976 and 0.8978. They are probably not exactly equal because the solver stops at a tolerance, but I haven't checked that. The `transfer` table from this run shows the transfers themselves if you want them.

## Caveat about the synthetic baseline
The baseline is made from random numbers. It describes no real economy, and its figures have no real-world units. Value added was drawn separately from the trade and production shares, so the baseline is not an equilibrium of the model. Even with no policy change, KITE would show welfare moving away from 1. The figures above therefore mix the effect of the tariff with that adjustment of the baseline. I didn't run a no-policy comparison because you didn't ask for one, so the two parts can't be separated here.

## Reproduce
This R script reruns the scenario with KITE alone, without this server:
`kite://runs/rc02902e4c6ad/reproduce.R`


## h04

**Question.** On the synthetic_example baseline with caliendo_parro_2015, productivity in sector s2 of c2 rises by 5%. How do real wages and welfare change in each country?

**Answer.**

I ran the 5% productivity rise in sector s2 of c2 once with `caliendo_parro_2015`. In the results, c2 and c3 gain while c1 loses, but these numbers are not the effect of the productivity rise alone (see the caveat below).

## Convergence
- **Status:** converged after **20 iterations**, criterion **9.507e-5** (tolerance 1e-4).
- **Accounting check:** passed, rule `fixed` (trade balances unchanged). KITE's reason: "the solved trade balances sum to zero for the world". The world trade balance is **0** in the baseline and in the result, against an accounting tolerance of **1.000e-6**. World value added is **490.3**.
- **Warnings:** none.

## Results by country
All figures are ratios of counterfactual to baseline: 1 means no change and below 1 is a fall. The percentage changes are derived by kite-mcp.

| Country | Welfare (`welfare_change`) | Real wage (`wage_change_real`) | Nominal wage (`wage_change`) |
|---|---|---|---|
| c1 | 0.9093 (−9.073 %) | 0.9113 (−8.871 %) | 0.9020 (−9.803 %) |
| c2 | 1.043 (4.330 %) | 1.043 (4.343 %) | 1.028 (2.780 %) |
| c3 | 1.097 (9.655 %) | 1.098 (9.806 %) | 1.083 (8.313 %) |

Sources: welfare comes from `run_scenario` (table `welfare_change`). Real wages come from `get_table` (table `wage_change_real`), which is the wage change divided by the change in the consumer price index. Nominal wages come from `get_table` (table `wage_change`). World value added is the numeraire, so nominal wages are measured relative to it.

## How to read these numbers
1. **The baseline is synthetic.** It is made from random draws and describes no real economy, and its numbers have no real-world units. None of these figures is a real-world estimate.
2. **The results include an adjustment of the baseline as well as the shock.** The baseline note says value added is drawn separately from the trade and production shares. So the baseline is not an equilibrium of the model, and even a run with no policy change does not give `welfare_change` = 1. Every result here therefore mixes two things: the economy settling into equilibrium and the productivity rise itself. This probably explains c1's large fall and c3's large gain, which are much bigger than a 5% rise in one sector of c2 would usually cause. You asked for one scenario, so I did not run a second one. To see the effect of the productivity rise alone, I can run the same model with no policy change and compare the two runs with `compare_runs`.

**Reproduce script:** `kite://runs/radb128dae849/reproduce.R` reruns this scenario in R without this server.


## h05

**Question.** Using caliendo_parro_2015 on the synthetic_example baseline, what is the welfare effect on each country of a carbon tax of 0.5 levied by every country?

**Answer.**

I didn't run this scenario because the model you asked for can't express a carbon tax.

**Why not:** `list_models` shows that `caliendo_parro_2015` reads only tariffs, non-tariff trade costs, export subsidies, productivity changes, population changes and a global value-added factor. It has no `carbon_tax` variable. kite-mcp would refuse the variable for this model. If I forced it through with `allow_unread_variables`, KITE would only warn that it has no effect, so the "result" would not reflect a carbon tax. That's why there are no welfare figures, warnings or reproduce script here.

**What can express it:** `mahlkow_wanner_2021` reads `carbon_tax`. It's a specific tax per unit of carbon, levied by climate-club members on all domestic use of sector goods. The default club is every country, which matches "levied by every country." Without a carbon policy, this model gives the same equilibrium as `caliendo_parro_2015`. The `synthetic_example` baseline has the `price` and `carbon_intensity` tables that a carbon tax needs.

**The baseline:** `describe_baseline` lists three countries (c1, c2, c3) and two sectors (s1, s2). It is synthetic: random draws that describe no real economy, with no real-world units. Its own note gives a caveat. Value added is drawn separately from the trade and production shares, so the baseline isn't an equilibrium of the models. Any run's results therefore mix this adjustment of the baseline with the effect of the policy.

**Proposed run:** use `mahlkow_wanner_2021` on `synthetic_example` with `carbon_tax = 0.5`, all countries in the club, and no border carbon tariff or export rebate (their defaults). I'd then report convergence, the welfare change by country, every warning and the reproduce script's URI. Should I run it?

