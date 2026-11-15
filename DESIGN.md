# Design

kite-mcp lets a language model run trade policy scenarios in KITE and return KITE's own numbers. This document records the decisions behind it and what the tests show about each. All timings are from one machine (WSL 2, Ubuntu 22.04, R 4.6.2, reference BLAS) and are given for scale only.

## A Rust server and a long-lived R worker

KITE is an R package. Something has to run R, and something has to speak the Model Context Protocol, check inputs, keep runs and watch over the solver. kite-mcp splits these between two processes.

The server is written in Rust. It holds the MCP protocol (the official `rmcp` SDK), the tool schemas, which are generated from the same Rust types that parse the input, the validation, the run store, the cache, timeouts and the restart of a failed worker.

The worker is one R script, `worker/kite_worker.R`. It loads KITE once and then answers requests, one JSON line in and one JSON line out. It builds KITE's inputs from typed data, calls `update_equilibrium()` and `process_results()`, and returns the tables, the `info` block and every warning with its class. It does nothing else.

Keeping R alive matters because starting it costs more than solving the example. A bare `Rscript` start takes 0.12 s and loading KITE and data.table 0.09 s; one solve of the example takes 0.06 s for each of the three models. The worker answers its first request about 0.3 s after it is started (`kite-mcp doctor` reports the figure), and every later request pays only for the solve.

Two processes also isolate failures. If a solve hangs, the server stops the worker when the timeout passes (120 s by default), reports a timeout to the client and starts a new worker for the next request. The tests in `tests/worker_restart.rs` cover a worker that hangs, one that exits, one that writes a malformed reply and one that answers the wrong request. The worker keeps no state between requests, so a restart loses nothing.

An R web framework such as plumber would have put the HTTP layer, the validation and the process in R itself. A hang inside a solve would then stop the server, and the MCP side would still need writing. The REST interface of kite-mcp (`kite-mcp serve --http`) runs the same Rust handlers as the MCP tools, and its OpenAPI document is generated from the same Rust types as the tool schemas.

## Typed input, no evaluation

Every input is typed JSON. Rust parses it into fixed types that reject unknown fields, checks it against the model and the baseline, and builds the tables KITE receives. The worker turns those tables into R data with fixed code. Nothing that came from a request is passed to `eval()`, `parse()` or `source()`. The worker sources only its own two files.

Validation errors name the field, the value and the allowed set, for example `scenario.tariff_new[0].origin` is `c9`, allowed `[c1, c2, c3]`. A scenario variable that the chosen model does not read is refused by default, with the names of the models that do read it. KITE itself warns in that case (`kite_unknown_scenario_variable`, `R/update_equilibrium.R:246-285`) and runs anyway. Setting `allow_unread_variables` passes the variable to KITE so that its warning comes back; the conformance tests use this.

Scenario tables are given as cells: `{"origin": "c1", "destination": "c2", "value": 1.2}`. An omitted dimension means every code, and cells whose origin equals their destination are never changed, so "a tariff on all imports into c1" leaves c1's purchases from itself alone. Cells not listed keep their baseline value. This convention is kite-mcp's own. The reproduce script writes it out as the data.table code it amounts to.

## Numbers that cross the boundary unchanged

Between the server and the worker a double travels as the 16 hexadecimal digits of its bit pattern (`docs/worker-protocol.md`). Towards clients it is written as the shortest decimal that parses back to the same double. A unit test checks this for 10,000 random bit patterns. NaN, R's missing value and infinities, which JSON cannot hold as numbers, are written as the strings `NaN`, `NA`, `Inf` and `-Inf`.

The reproduce scripts needed one more step. They are R source, and R's parser does not round every decimal correctly: of 20,000 shortest decimals generated in `tests/conformance.rs`, R read 4 to a neighbouring double, among them `0.00648016`. Before writing a script, the server therefore asks the worker how R reads each number it will write. Where R reads the decimal correctly, the script shows the decimal; otherwise it shows R's hexadecimal literal, which R reads exactly, with the decimal in a comment at the top. The conformance tests include a carbon tax of `0.00648016` and check that its script reproduces the run.

## Why exact equality is the test

kite-mcp does no arithmetic on KITE's results. It only moves them: from R to JSON, across a pipe, into Rust and out to the client. A result that differs from a direct call to KITE in any bit therefore has a cause in kite-mcp, such as a cell set in the wrong place, a table in the wrong order or a value read wrongly, and is not numerical noise. A tolerance would hide exactly these faults.

The tests found two of them while being written. A data.table expression in the test's own reference script set a tariff to its existing value, and the run through the server differed in its iteration count (16 against 15). And the decimal problem above showed up as a 1-bit difference. Both would pass a comparison within 1e-6.

The conformance test (`tests/conformance.rs`) runs eleven scenarios through a real kite-mcp process over MCP and compares every table of `process_results()` and the `info` block with a direct call to KITE, bit for bit: 501 tables and 17,394 values. It then runs each run's reproduce script in a fresh R session and requires the same bits again. The only numbers kite-mcp computes are named `*_derived` or `*_display`: the welfare percentage `(welfare_change - 1) * 100`, the differences of `compare_runs`, and display values. The tests check them against the same formula applied to KITE's values.

## Display values

Exact values are long: `0.8516332754471132`. In the evaluation's last rounds the prompt asked the model to copy them in full, which kept every number checkable and made the answers hard to read. Each exact number now comes with a display value: `welfare_change_display` is `0.8516`, and for a ratio `percent_change_display` is `-14.84 %`. A display value is the exact value correctly rounded to 4 significant figures, in decimals from 0.001 up to 10,000 and in e-notation outside that range (`9.757e-5`). A unit test checks the rounding on 10,000 values, and the conformance test checks that every `*_display` field matches its exact value. The prompt asks the model to quote display values and to give exact values when the user asks for them. Rounding stays with the server: in round 3 of the evaluation the model rounded 0.8516333 to 0.8517 by itself.

## Runs that do not converge

KITE reports `info$convergence` as TRUE, FALSE or NA (`R/update_equilibrium.R:163-195`). It is FALSE when the criterion stays above the tolerance, when an inner solve fails, when an output is not finite, or when the solved trade balances do not sum to zero for the world. The last case comes with a warning of class `kite_world_trade_balance`.

kite-mcp treats anything other than TRUE as `not_converged`. `run_scenario` then returns the status, the criterion, the iterations and KITE's accounting reason, and no headline figures. `get_table` still returns the tables, because they help to diagnose the failure, but flags every row as unconverged and says the rows are not results. `compare_runs` refuses a run that did not converge. The test case is the one KITE's documentation names: `trade_balance_rule = "fixed_country_share"` after the example tariff war (`R/model_commons.R:10-16`).

## Synthetic baselines are not equilibria

The baselines kite-mcp can build use the construction of KITE's example, which draws value added independently of the trade and production shares. Such a baseline does not satisfy the models' equilibrium conditions. In KITE's example baseline, factor share times output is 0.79, 0.98 and 1.27 times value added for c1, c2 and c3. A run with no policy change at all therefore moves the economy: under `caliendo_parro_2015` the empty scenario gives `welfare_change` of 0.902, 0.996 and 1.092. The example's tariff war gives 0.852, 0.921 and 1.145, so most of c3's reported gain is this adjustment.

kite-mcp returns KITE's numbers as they are. It says this in `describe_baseline` and next to every run on a synthetic baseline. A policy's own effect on a synthetic baseline is its difference from a run with an empty scenario, which `compare_runs` gives.

## A worker in another language

The protocol in `docs/worker-protocol.md` does not depend on R. A Julia worker would read JSON lines, build its model's inputs from the typed tables, solve, and write back the tables, the convergence block and its warnings. The server starts whatever program `WorkerConfig` names. The parts that are specific to KITE are the model catalogue and the scenario variables in `src/catalogue.rs` and `src/scenario.rs`; a second model suite would add its own entries there. No Julia worker is built.

## Left out

- `.ice` initial-condition files. KITE plans to read them from version 26.11 (issue #19), and no format is published. The CSV loader is in one module, `src/baseline.rs`, where an `.ice` reader would go.
- Calibrated baselines. kite-mcp uses KITE's synthetic example and data generated by the same construction, and CSV files a user supplies.
- KITE's other models. The package ships three; the README says the full suite of 13 is available on request.
- Runs persist for the life of the server process only.
- One worker serves all requests, so solves run one at a time.
