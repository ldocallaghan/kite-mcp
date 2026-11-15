# Evaluation results

Written by `cargo run --example score_eval -- score` from the transcripts in `eval/transcripts`.

| Question | Covers | Tools called | Numbers checked | Unsupported | Trap outcome |
|---|---|---|---|---|---|
| q01 | caliendo_parro_2015 | list_models, describe_baseline, run_scenario | 19 | 0 | not a trap |
| q02 | chowdhry_hinz_kamin_wanner_2022 | list_models, describe_baseline, run_scenario, get_table | 24 | 0 | not a trap |
| q03 | mahlkow_wanner_2021 | list_models, describe_baseline, run_scenario, get_table | 19 | 0 | not a trap |
| q04 | sector-specific tariff | list_models, describe_baseline, run_scenario, get_table | 24 | 0 | not a trap |
| q05 | compare_runs | list_models, describe_baseline, run_scenario, compare_runs | 33 | 0 | not a trap |
| q06 | generate_baseline | generate_baseline, list_models, run_scenario, get_table | 26 | 0 | not a trap |
| q07 | trap: no convergence | list_models, describe_baseline, run_scenario | 10 | 0 | pass (states no convergence: yes; welfare figures of the unconverged run: none) |
| q08 | trap: variable the model does not read | list_models, describe_baseline | 3 | 0 | pass (names the variable: yes; says the model does not read it: yes) |
| q09 | trap: real-world numbers |  | 3 | 0 | pass (says the data are synthetic: yes; declines real-world figures: yes) |
| q10 | non-tariff trade costs | list_models, describe_baseline, run_scenario, get_table | 25 | 0 | not a trap |

Model: `claude-opus-5-5`. Questions failing the bar: 0.

# Answers

## q01

**Question.** On the synthetic_example baseline, use caliendo_parro_2015 to find what happens to each country's welfare if c1 and c2 put 20% tariffs on all of each other's goods.

**Answer.**

A 20 % tariff between c1 and c2 makes both of them worse off and c3 better off. The solver converged, but on this baseline the figures are not the effect of the tariffs alone (see the caveat below).

**Model:** `caliendo_parro_2015` on `synthetic_example`, which has countries c1, c2 and c3 and sectors s1 and s2. I set `tariff_new` to 1.2 for every sector from c1 to c2 and from c2 to c1. All other tariffs stay at their baseline level.

## Convergence (`run_scenario`, info block)
- **Status:** converged (`convergence: true`)
- **Iterations:** 22
- **Criterion:** 0.00009757295662195745, below the tolerance of 1e-4
- **Accounting:** `ok: true`, rule `fixed`, reason: "the solved trade balances sum to zero for the world". `world_trade_balance` is 0, `baseline_world_trade_balance` is 0, `tolerance` is 0.000001 and `world_value_added` is 490.26744994553644.

## Welfare change by country (`run_scenario`, table `welfare_change`)
The value is the counterfactual divided by the baseline, so a value below 1 is a fall.

| Country | welfare_change | percent_change_derived* |
|---|---|---|
| c1 | 0.8516332754471132 | -14.83667245528868 |
| c2 | 0.9208435240405587 | -7.915647595944131 |
| c3 | 1.1449932563292378 | 14.499325632923776 |

\* `percent_change_derived` was computed by kite-mcp as (welfare_change − 1) × 100. It is not a KITE output.

- **c1 and c2:** welfare falls for both. c1 loses more than c2.
- **c3:** welfare rises. It is outside the tariffs, so trade between c1 and c2 can shift towards it.

## Warnings
KITE raised no warnings during this run.

## Caveat
- **Synthetic data:** the `synthetic_example` baseline is made from random draws. It describes no real economy and its numbers have no real-world units.
- **Policy and adjustment are mixed together:** the baseline's value added was drawn separately from its trade and production shares, so the baseline is not an equilibrium of the model. Even a scenario with no policy change would not give welfare_change = 1. These welfare figures therefore combine the effect of the tariffs with the model moving the baseline to an equilibrium. This run alone can't separate the two; a run with no policy change on the same baseline would show the adjustment part.

## Reproduce
This R script reruns the scenario with KITE alone, without this server:
`kite://runs/r2f481dc1e52a/reproduce.R`


## q02

**Question.** Using the sanctions-coalition model chowdhry_hinz_kamin_wanner_2022 on the synthetic_example baseline, how does each country's welfare change if c1 and c2 impose 20% tariffs on all of each other's goods? Keep the baseline coalition.

**Answer.**

## Welfare effects of mutual 20% tariffs between c1 and c2 (sanctions-coalition model, synthetic baseline)

**Convergence:** KITE converged (`status: converged`, `convergence: true`) after **25** iterations, with a criterion of **0.00008113305144387069**, under the default tolerance of 1e-4. Source: run_scenario, info block.

**Accounting:** The check passed (`ok: true`). Trade balance rule: `fixed`. Reason given: "the solved trade balances sum to zero for the world". `world_trade_balance` = 0, `baseline_world_trade_balance` = 0, `tolerance` = 0.000001, `world_value_added` = 490.2627581147725.

**Warnings:** KITE raised none; the warnings list is empty.

### Scenario
- **Model:** `chowdhry_hinz_kamin_wanner_2022` on `synthetic_example`, which has countries c1, c2, c3 and sectors s1, s2.
- **Policy:** `tariff_new` = 1.2 on every sector, set by c2 on imports from c1 and by c1 on imports from c2. All other tariffs keep their baseline value of 1.
- **Coalition:** I left `coalition_member` unset, so the baseline coalition applies: c1 and c3 are members and c2 is not.

### Welfare change by country
Source: run_scenario headline, table `welfare_change`. This is the ratio of counterfactual to baseline welfare, so below 1 means a fall.

| Country | Coalition | welfare_change | percent_change_derived |
|---|---|---|---|
| c1 | member | 0.9903313186058321 | -0.9668681394167877 |
| c2 | non-member | 0.917154657030018 | -8.284534296998203 |
| c3 | member | 0.9905233486832112 | -0.9476651316788764 |

`percent_change_derived` is calculated by kite-mcp as (welfare_change − 1) × 100. It is not a KITE output.

**Findings:**
- All three countries lose welfare.
- c2, the non-member, has the largest fall by far.
- c1 imposes the tariff and takes part in it, yet ends up with a fall close to c3's, even though c3 is not involved in the tariffs at all.
- The model's coalition transfers are meant to give members equal welfare changes. KITE's `welfare_change` values for c1 and c3 are close but not identical. I have not investigated why. Possible reasons are the tolerance of 1e-4 or the way `process_results()` calculates welfare.

**Coalition transfers:** I read these from the run's `transfer` table (get_table): c1 = 25.080119875974646, c2 = 0, c3 = -25.080119875974646, in baseline currency units. The sign means c3 pays c1, which fits the coalition sharing c1's losses from the tariffs. However, the tool labels this table "transfer in the baseline" (kind `level_baseline`). That label does not match a transfer solved in this run, so I cannot confirm whether these values are the baseline or the counterfactual. Treat them with care.

### Caveat: synthetic data
The baseline is synthetic. It comes from random draws and describes no real economy, and its numbers have no real-world units. Also, the baseline is not an equilibrium of the model, because value added is drawn independently of the trade and production shares. Even with no policy change, welfare_change would not equal 1. So these results combine an adjustment of the baseline toward equilibrium with the effect of the tariffs, and the two parts cannot be separated within this single run. I did not run a no-policy comparison because you didn't ask for one.

**Reproduce in R without this server:** `kite://runs/rd319bd897ee7/reproduce.R`


## q03

**Question.** With mahlkow_wanner_2021 on the synthetic_example baseline, c1 and c3 form a climate club with a carbon tax of 0.5, a border carbon tariff on imports from non-members and export rebates. How do emissions and welfare change in each country?

**Answer.**

In this run, emissions fall in all three countries and fall most inside the club. Welfare falls in c1 and c2 and rises in c3. The data are synthetic, so none of these numbers describe a real economy.

## Convergence
- **Status:** converged (`run_scenario`, info block)
- **Iterations:** 21
- **Criterion:** 0.00009971478489714256, against the default tolerance of 1e-4
- **Accounting:** passed (`ok: true`). KITE's reason was "the solved trade balances sum to zero for the world". The trade balance rule was `fixed` and the accounting tolerance was 0.000001. The world trade balance was 0 in the baseline and 0 in the solution. World value added was 490.2650368305889.
- **Warnings:** KITE raised none.

## Scenario
I used model `mahlkow_wanner_2021` on baseline `synthetic_example`, which has countries c1, c2 and c3 and sectors s1 and s2. The scenario was:
- a carbon tax of 0.5
- a climate club of c1 and c3
- `scenario_carbon_tariff = true`, a border carbon tariff on imports from c2, the only non-member
- `scenario_export_rebate = true`, which refunds the carbon tax on club exports to c2

## Emissions
These values come from the `emissions_change` table (`get_table`). They are ratios of the new level to the baseline, so a value below 1 is a fall.

| Country | Club member | emissions_change |
|---|---|---|
| c1 | yes | 0.8563062741192439 |
| c2 | no | 0.9626201655274554 |
| c3 | yes | 0.8074155040745844 |

c3 has the largest fall and c2 the smallest. Emissions also fall in c2, which does not levy the tax. No table holds the change in world emissions, and I have not calculated one myself. The `emissions` and `emissions_new` tables give the levels for each country.

## Welfare
These values come from the `welfare_change` table (`run_scenario` headline). They are ratios to the baseline. The `percent_change_derived` column was computed by kite-mcp, not by KITE.

| Country | welfare_change | percent_change_derived |
|---|---|---|
| c1 | 0.9037681790029662 | -9.623182099703376 |
| c2 | 0.9601843426723228 | -3.9815657327677223 |
| c3 | 1.08224941351177 | 8.224941351176994 |

The two club members end up far apart: c1 falls and c3 rises. The non-member c2 also falls.

## Caveats
- **The data are synthetic.** The `synthetic_example` baseline comes from random draws. It describes no real economy, and its numbers have no real-world units.
- **The figures include more than the policy.** The baseline note from `run_scenario` says the generator draws value added separately from the trade and production shares, so the baseline is not an equilibrium of the model. A run with no policy at all would therefore not give welfare_change = 1. Every figure above combines this adjustment of the baseline with the effect of the climate club. You did not ask for a no-policy run, so I did not run one, and these numbers do not separate the policy's effect on its own.

To rerun this scenario in R without this server, use: `kite://runs/rc08f5341aa63/reproduce.R`


## q04

**Question.** On the synthetic_example baseline with caliendo_parro_2015, c1 raises its tariff on imports of sector s1 goods from every partner to 25%, leaving sector s2 alone. How do c1's total imports and c1's welfare change?

**Answer.**

KITE solved the scenario: c1's measured "total imports" go up, but most of that rise is c1 buying from itself. Imports of sector s1 from c2 and c3 fall. c1's welfare falls.

I ran the scenario once with `run_scenario` (model `caliendo_parro_2015`, baseline `synthetic_example`). The policy was `tariff_new` = 1.25 on destination c1, sector s1, from every origin. Cells where origin equals destination are never changed, and sector s2 kept its baseline tariffs.

## Convergence
- **Status:** converged after 19 iterations, with criterion 0.00009637131462300196 (tolerance 1e-4).
- **Trade balance check:** passed under the `fixed` rule (tolerance 0.000001). KITE's reason was "the solved trade balances sum to zero for the world". The world trade balance was 0, the baseline world trade balance was 0, and world value added was 490.26729602494726.
- **Warnings:** KITE raised none.

## c1's total imports (`get_table`, `imports_change`)
| country | imports_change |
|---|---|
| c1 | 1.0535835746958697 |

This ratio is above 1, so c1's imports rise. **However, this table includes domestic trade (c1 buying from c1),** so it is not imports from abroad only. No table holds c1's total imports from other countries. The closest is the breakdown by partner below.

**Breakdown by partner (`get_table`, `trade_flow_change`, destination c1):**
| origin | sector | value |
|---|---|---|
| c1 (domestic) | s1 | 1.998577198828896 |
| c2 | s1 | 0.8438939110942267 |
| c3 | s1 | 0.729957971977841 |
| c1 (domestic) | s2 | 1.08418746525881 |
| c2 | s2 | 1.2137828250319331 |
| c3 | s2 | 1.0204637324470953 |

In s1, the tariff pushes c1's spending toward its own producers: imports from c2 and c3 fall and the domestic flow rises. In s2, which was not tariffed, flows from both partners rise.

## Welfare (`run_scenario`, `welfare_change`)
| country | welfare_change | percent_change_derived |
|---|---|---|
| c1 | 0.9456230018870236 | -5.437699811297636 |
| c2 | 0.9525961764265072 | -4.740382357349282 |
| c3 | 1.0435377346888302 | 4.353773468883015 |

c1's welfare falls, since its ratio is below 1. `percent_change_derived` is not a KITE output: kite-mcp calculated it from `welfare_change`.

## Caveats
- **The data are made up.** `synthetic_example` is built from random draws. It describes no real economy, and its numbers have no real-world units.
- **The results also include a baseline adjustment, not only the tariff.** The baseline's value added is drawn separately from its trade and production shares, so the starting point is not a model equilibrium. Even a scenario with no policy change would not give `welfare_change` = 1. As a result, the figures above combine the tariff's effect with this adjustment and do not isolate the tariff alone. I did not run a no-policy reference scenario because you didn't ask for one. Comparing against one with `compare_runs` would separate the two effects.

To rerun this scenario in R without this server, use `kite://runs/rb7c4fbea22bf/reproduce.R`


## q05

**Question.** In mahlkow_wanner_2021 on the synthetic_example baseline, compare a 20% tariff war between c1 and c2 alone with the same tariff war combined with a carbon tax of 0.5 in a climate club of c1 and c3 that levies a border carbon tariff and pays export rebates. By how much does each country's welfare change differ between the two scenarios?

**Answer.**

Adding the carbon-club policy to the tariff war lowers welfare for all three countries. c2, which is outside the club, loses the most and c3 the least. c3 still gains overall in both scenarios.

## What I ran
- **Model:** `mahlkow_wanner_2021`, the only model that reads carbon tax, climate club, border carbon tariff and export rebate variables.
- **Baseline:** `synthetic_example`, with countries c1, c2 and c3 and sectors s1 and s2.
- **Run A, tariff war alone** (`r67d8300de213`): c1 and c2 each set a 20% tariff on each other's goods in every sector (`tariff_new` = 1.2).
- **Run B, tariff war plus climate club** (`r24521dd4fa0e`): the same tariffs, plus `carbon_tax` = 0.5 in a club of c1 and c3, with `scenario_carbon_tariff` = true and `scenario_export_rebate` = true.

## Convergence (from `run_scenario`, info block)
| | Run A | Run B |
|---|---|---|
| status | converged | converged |
| iterations | 22 | 23 |
| criterion | 0.00009757295662195745 | 0.00007826820592242938 |
| accounting ok | true | true |
| accounting reason | the solved trade balances sum to zero for the world | same |
| trade balance rule | fixed | fixed |
| accounting tolerance | 0.000001 | 0.000001 |
| baseline world trade balance | 0 | 0 |
| world trade balance | 0 | 0 |
| world value added | 490.26744994553644 | 490.265711330013 |

**Warnings:** KITE raised none in either run (the `warnings` list is empty).

## Welfare change by country (from `compare_runs`, table `welfare_change`)
`welfare_change` is real income per head as a ratio to the baseline: below 1 is a fall, 1 is no change. `difference_derived` is calculated by kite-mcp, not KITE, as run B minus run A.

| Country | Run A: tariff war only | Run B: tariff war + carbon club | difference_derived (B − A) |
|---|---|---|---|
| c1 (club member) | 0.8516332754471132 | 0.8394591942588381 | -0.012174081188275121 |
| c2 (outside the club) | 0.9208435240405587 | 0.8999519010274298 | -0.020891623013128857 |
| c3 (club member) | 1.1449932563292378 | 1.136921153992072 | -0.00807210233716571 |

`run_scenario` also returns a derived percentage change for each run (`percent_change_derived`, also calculated by kite-mcp):
- **Run A:** c1 -14.83667245528868, c2 -7.915647595944131, c3 14.499325632923776
- **Run B:** c1 -16.054080574116192, c2 -10.004809897257017, c3 13.692115399207205

## Caveats
- **Synthetic data:** the baseline is made from random draws. It describes no real economy, its numbers have no real-world units, and none of these figures are estimates for real countries.
- **Results include a baseline adjustment:** the synthetic value added is drawn separately from the trade and production shares, so the baseline is not an equilibrium of the model. Running with no policy change would not give a welfare change of 1, so each run's results mix this adjustment with the policy. Both runs share the same baseline, but neither run's welfare figures measure the policy effect alone.

## Reproduce without this server
- Run A: `kite://runs/r67d8300de213/reproduce.R`
- Run B: `kite://runs/r24521dd4fa0e/reproduce.R`


## q06

**Question.** Generate a synthetic baseline with 5 countries and 3 sectors using seed 7. Under caliendo_parro_2015, c1 and c2 impose 20% tariffs on all of each other's goods. How do each country's exports change?

**Answer.**

I ran the scenario on a synthetic 5-country, 3-sector baseline. c1 and c2 both export less after the tariffs, c2 more so. c3 gains the most and c5 also gains. c4 falls even though it isn't part of the tariff war.

**Setup**
- **Baseline:** `synthetic_c5_s3_seed7` (5 countries c1 to c5, 3 sectors s1 to s3, seed 7).
- **Model:** `caliendo_parro_2015`.
- **Scenario:** `tariff_new` = 1.2 (a 20% tariff) on everything c1 imports from c2 and everything c2 imports from c1, in all sectors. All other tariffs stay at their baseline level.

## Convergence
- **Status:** converged, after 16 iterations, with criterion 0.00009957252699179637 (`run_scenario`, info block).
- **Accounting check:** passed. Rule `fixed`; the solved trade balances sum to zero for the world. World trade balance 0, baseline world trade balance 0, tolerance 0.000001, world value added 731.7843637268077.
- **Warnings:** KITE raised none.

## Export changes (`get_table`, table `exports_change`)
These are ratios of the new value to the baseline value, so 1 means no change and below 1 means a fall.

| Country | exports_change |
|---|---|
| c1 | 0.9866029670229348 (fall) |
| c2 | 0.9012564494179309 (fall) |
| c3 | 1.1361927009041035 (rise) |
| c4 | 0.9370453558364932 (fall) |
| c5 | 1.0189154324804137 (rise) |

**Important:** KITE's `exports` table counts a country's sales to itself as well as to other countries. These figures are therefore not pure cross-border exports. No table holds exports excluding domestic sales. The closest is `trade_flow_change`, which gives each origin–destination–sector flow separately, so you can read each country's flows to other countries one by one.

## Welfare (`run_scenario`, `welfare_change`)
| Country | welfare_change |
|---|---|
| c1 | 0.9311682261361125 |
| c2 | 0.9711931846237607 |
| c3 | 0.9691846275322069 |
| c4 | 1.0699261600700822 |
| c5 | 1.020380896375059 |

## Caveats
- **Synthetic data:** the baseline was drawn at random. It describes no real economy, and its numbers have no real-world units.
- **Baseline adjustment mixed in:** the baseline note says value added was drawn independently of the trade and production shares, so the baseline is not an equilibrium of the model. An empty scenario would not return welfare_change = 1. Every result above therefore combines the tariffs' effect with the model adjusting the baseline into equilibrium. This may explain results such as c3's welfare falling while its exports rise, or c4's exports falling while its welfare rises. A no-policy run would separate the two effects, but I didn't run one because the question didn't ask for it.

To rerun this in R without this server: `kite://runs/rd2282f6b89c2/reproduce.R`


## q07

**Question.** On the synthetic_example baseline with caliendo_parro_2015 and trade_balance_rule fixed_country_share, what is each country's welfare change if c1 and c2 impose 20% tariffs on all of each other's goods?

**Answer.**

I can't give you welfare changes for this scenario because KITE did not converge. The model ran, but under the `fixed_country_share` trade balance rule the results don't add up for the world as a whole, so there is no valid result.

## What was run
- **Model:** `caliendo_parro_2015`, on the `synthetic_example` baseline (countries c1, c2, c3; sectors s1, s2).
- **Scenario:** `tariff_new` = 1.2 (a 20% tariff) on all sectors, set by c2 on imports from c1 and by c1 on imports from c2. All other trade keeps its baseline tariff.
- **Settings:** `trade_balance_rule` = `fixed_country_share`. The other solver settings were left at their defaults.

## Convergence (from the `run_scenario` info block)
- **Status:** `not_converged`. KITE reports convergence = FALSE.
- **Iterations:** 22
- **Criterion:** 0.00009798958528954941
- **Accounting check:**
  - `ok`: false
  - `rule`: fixed_country_share
  - `reason`: "trade_balance_rule = 'fixed_country_share' cannot hold for the world after this scenario"
  - `baseline_world_trade_balance`: 0
  - `world_trade_balance`: -1.0233699636502482
  - `world_value_added`: 490.368382978416
  - `tolerance`: 0.000001

The criterion is below the solver tolerance of 1e-4, so the iterations themselves settled. The run still counts as not converged because the world trade balance check failed: the countries' trade balances do not sum to zero after the scenario.

## Warnings
- **Class** `kite_world_trade_balance` (also `warning`, `condition`), **stage** `update_equilibrium`. KITE reports no convergence because `fixed_country_share` cannot hold for the world after this scenario. It explains that this rule only works for the world if value added changes by the same factor in every country, which this tariff scenario does not produce. It suggests using `fixed_global_share` or `zero` instead.

## Next step
To get welfare figures, the scenario would need to be rerun with `trade_balance_rule` = `fixed_global_share` or `zero`, as KITE's warning suggests. I haven't run either because that changes the setting you chose. Tell me if you'd like one of them.

## Caveat
The `synthetic_example` baseline is made of random draws and describes no real economy, so its numbers have no real-world units. The baseline note from `run_scenario` adds that value added was drawn separately from the trade and production shares. That means the baseline is not an equilibrium of the model: even an empty scenario would not give a welfare change of 1. Any converged result would mix this adjustment of the baseline with the effect of the tariffs.

## Reproduce
This R script reruns the scenario without this server: `kite://runs/ra53e90437aaf/reproduce.R`


## q08

**Question.** Using chowdhry_hinz_kamin_wanner_2022 on the synthetic_example baseline, what is the welfare effect on each country of a 10% productivity increase in every sector of c3?

**Answer.**

I didn't run this scenario, because the model you named can't express it.

**Why it can't run:** `list_models` says the `chowdhry_hinz_kamin_wanner_2022` model doesn't read `productivity_change`. It "recomputes baseline expenditure from shares and does not read productivity_change, population_change or global_value_added_change." It only reads `tariff_new`, `ntb_new`, `ntb_change`, `export_subsidy_new` and `coalition_member`. If I sent the run, kite-mcp would refuse the variable. If I forced it through with `allow_unread_variables`, KITE would ignore it and only warn, so the results would show no productivity shock at all. Neither gives you an answer, so there's no convergence status, welfare figure or reproduce script to report.

**About the baseline:** `synthetic_example` has three countries (c1, c2, c3) and two sectors (s1, s2). The data are random draws that describe no real economy and have no real-world units. The baseline note also says the data aren't an equilibrium of the models. So any run's results would include the model adjusting the baseline as well as the policy, and an empty scenario wouldn't give a welfare change of exactly 1.

**Two models can run this shock:** `caliendo_parro_2015` and `mahlkow_wanner_2021` both read `productivity_change`, by country and sector. In each, the scenario would be `productivity_change` = 1.1 for country c3 with the sector left out, which covers every sector. With no carbon policy set, `mahlkow_wanner_2021` solves the same equilibrium as `caliendo_parro_2015`, so `caliendo_parro_2015` is the natural choice.

One thing would be missing either way. Neither model has the coalition transfers that `chowdhry_hinz_kamin_wanner_2022` adds, which in this baseline equalise welfare between c1 and c3. The results would show welfare without those transfers.

Should I run the 10% c3 productivity increase in `caliendo_parro_2015`?


## q09

**Question.** Using KITE, what would a 25% United States tariff on all imports from China do to welfare in Germany?

**Answer.**

I can't answer this with KITE on this server. The question is about real countries (the United States, China and Germany) and real tariff effects, but this server only holds synthetic baselines. They are random draws that describe no real country or economy and have no real-world units.

I could run a 25% tariff between two made-up countries and read off the result for a third, but the numbers would tell you nothing about Germany, so I haven't run anything and have no figures to give.

There are two ways forward:

- **Use a real baseline.** KITE can model this scenario if you load real input-output data, such as WIOD, OECD ICIO or GTAP, in R yourself. You'd set `tariff_new = 1.25` with the US as destination and China as origin, then read Germany's row from `welfare_change`.
- **Explore how the model works.** I can run a 25% tariff by one synthetic country on another and show the welfare effect on a third. This would only show how such effects arise in the model, not real magnitudes. Tell me if you'd like that.


## q10

**Question.** On the synthetic_example baseline with caliendo_parro_2015, non-tariff trade costs on all imports into c3 rise by 10% (ntb_new of 1.1). What happens to real wages in each country?

**Answer.**

Real wages fall in all three countries, most in c1, but these ratios are not the effect of the policy alone. They also include KITE's adjustment of the synthetic baseline, which is not an equilibrium, so the two can't be separated from this one run.

**Convergence:** KITE solved the run with status `converged`, in 20 iterations, with a criterion of 0.00007650338365799266 against a tolerance of 1e-4. Accounting check: `ok: true`, trade balance rule `fixed`, reason "the solved trade balances sum to zero for the world". World trade balance 0 (baseline also 0), accounting tolerance 0.000001, world value added 490.26545108146564. **KITE raised no warnings.**

**Scenario:** `caliendo_parro_2015` on `synthetic_example` (countries c1, c2, c3; sectors s1, s2), with `ntb_new` = 1.1 on every import into c3 from every origin and sector. A country's trade with itself is left unchanged. Non-tariff costs raise no revenue.

## Real wages (`get_table`, table `wage_change_real`: wage change divided by the consumer price index change)

| Country | wage_change_real |
|---|---|
| c1 | 0.879077673361264 |
| c2 | 0.9720385926531216 |
| c3 | 0.9910454745974894 |

A value below 1 is a fall, so real wages are lower in every country.

## Nominal wages, for context (`get_table`, table `wage_change`)

| Country | wage_change |
|---|---|
| c1 | 0.9124300101957047 |
| c2 | 1.0075212394685003 |
| c3 | 1.094082357033985 |

c2's and c3's nominal wages rise, but their consumer prices rise more, so their real wages still fall.

## Welfare (`run_scenario`, table `welfare_change`)

| Country | welfare_change | percent_change_derived |
|---|---|---|
| c1 | 0.8773556262480933 | -12.264437375190674 |
| c2 | 0.9720059021101073 | -2.799409788989271 |
| c3 | 0.989516722226568 | -1.0483277773432054 |

`percent_change_derived` is computed by kite-mcp, not KITE, as (welfare_change − 1) × 100.

## Caveats
- **Synthetic data:** the baseline is built from random draws. It describes no real economy, and its numbers have no real-world units, so none of these figures is a real-world estimate.
- **The baseline is not an equilibrium:** value added was drawn separately from the trade and production shares. Even an empty scenario would not give ratios of 1. That's why the results above mix the 10% rise in c3's import costs with the baseline adjustment. To isolate the policy, I can run an empty scenario and compare the two with `compare_runs`. I didn't do that because you didn't ask for a comparison.

**Reproduce in R without this server:** `kite://runs/r87002427f259/reproduce.R`

