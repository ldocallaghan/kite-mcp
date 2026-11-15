# kite-mcp

kite-mcp is an independent, unofficial server that lets a language model run trade policy scenarios in [KITE](https://github.com/julianhinz/KITE), the open-source trade model suite of the Kiel Institute for the World Economy, through the Model Context Protocol (MCP). It has no affiliation to or endorsement from the Kiel Institute. Every number it returns is the number KITE computes for the same inputs, bit for bit, and every run comes with an R script that reproduces it with KITE alone.

<!-- DEMO -->

## Install

kite-mcp needs R (tested with 4.6.2; KITE itself needs 3.5.0 or later), Rust (1.88 or later) and git.

```bash
git clone https://github.com/ldocallaghan/kite-mcp && cd kite-mcp
```

```bash
Rscript scripts/install-r-deps.R
```

```bash
cargo install --path . && kite-mcp doctor
```

The second command installs data.table, cli, jsonlite and testthat, and KITE at commit `5995dd1`. `kite-mcp doctor` checks R, the packages, KITE's version and commit, starts the worker and solves KITE's example, and says how to fix anything missing. The installed binary finds the R worker in the cloned repository; set `KITE_MCP_WORKER` to `worker/kite_worker.R` if you move it, and `KITE_MCP_RSCRIPT` if `Rscript` is not on the path.

With Docker the image holds R, KITE at the pinned commit and the server:

```bash
docker build -t kite-mcp . && docker run --rm kite-mcp doctor
```

### Claude Code

```bash
claude mcp add kite -- kite-mcp serve
```

With Docker: `claude mcp add kite -- docker run -i --rm kite-mcp`.

### Claude Desktop

In `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "kite": { "command": "kite-mcp", "args": ["serve"] }
  }
}
```

With Docker, use `"command": "docker", "args": ["run", "-i", "--rm", "kite-mcp"]`.

### Any MCP client

kite-mcp serves MCP over stdio. Start `kite-mcp serve` (or `docker run -i --rm kite-mcp`) as the server process. Options:

- `--baseline-dir ID=DIRECTORY` loads a baseline from CSV files (repeatable, see below);
- `--timeout-seconds N` sets the longest a solve may take before the worker is restarted (default 120);
- `--http` serves the REST interface on `--addr` (default `127.0.0.1:8080`) in place of MCP.

## Tools

| Tool | Does |
|---|---|
| `list_models` | KITE's three models: reference and citation, what each adds, and the scenario variables each reads, with meaning, units, defaults and the KITE source lines behind them |
| `describe_baseline` | Countries, sectors, tables and hash of a baseline, or of every baseline |
| `generate_baseline` | Builds a synthetic economy of 2 to 30 countries and 1 to 10 sectors from a seed, with the construction of KITE's example |
| `run_scenario` | Runs one scenario: status, KITE's `info` block, every warning with its class, the welfare change by country, provenance and the reproduce script |
| `get_table` | One `process_results()` table of a run, filtered by country, sector, origin or destination, with its unit |
| `compare_runs` | One table of two converged runs side by side, with their difference |

Scenario variables use KITE's own names. A bilateral variable is a list of cells; an omitted dimension means every code, and a country's trade with itself is never changed:

```json
{"model": "caliendo_parro_2015",
 "scenario": {"tariff_new": [{"origin": "c1", "destination": "c2", "value": 1.2},
                             {"origin": "c2", "destination": "c1", "value": 1.2}]}}
```

Tariffs are gross factors, so 1.2 is a 20 % tariff. Settings default to those of KITE's example (tolerance 1e-4, `vfactor` 0.1, 200 iterations) and are bounded; `trade_balance_rule` takes KITE's four rules.

Resources: each model's KITE help page (`kite://models/{id}/documentation`) and citation (`kite://models/{id}/citation`), KITE's citation (`kite://citation`), and for each run its reproduce script (`kite://runs/{id}/reproduce.R`) and provenance (`kite://runs/{id}/provenance.json`). The prompt `analyse_trade_policy_scenario` asks the model to report convergence, quote numbers only from tool results and give the reproduce script.

## The result contract

1. **Exactly KITE's numbers.** Every number equals what a direct call to KITE returns for the same inputs, bit for bit. The only numbers kite-mcp computes are named `*_derived`: the welfare percentage `(welfare_change - 1) * 100` and the differences of `compare_runs`.
2. **No unconverged result is presented as a result.** If KITE's `info$convergence` is not TRUE, the status is `not_converged` and the response gives the criterion, the iterations and KITE's accounting reason, and no headline. `get_table` still returns the tables with every row flagged `unconverged`; `compare_runs` refuses the run.
3. **Warnings are never dropped.** Every R warning comes back with its class and message.
4. **Provenance on every run:** KITE version and commit, the R version and packages, model, baseline id and hash, scenario, resolved settings, convergence, the kite-mcp version and the time.
5. **A reproduce script with every run.** A standalone R script that installs KITE at the recorded commit, rebuilds the baseline, runs the scenario and prints every table. Run in a fresh R session, it gives the same bits.
6. **Errors that say what to fix.** A validation error names the field, the value and the allowed set; a variable the model does not read is refused with the names of the models that read it.

The conformance tests (`tests/conformance.rs`, run in CI) hold the server to points 1 to 5: eleven scenarios across the three models, compared table by table with direct calls to KITE (501 tables, 17,394 values, all identical), and every reproduce script run in a fresh R session with the same result. `DESIGN.md` explains why the test is exact equality.

## Baselines

- `synthetic_example`: the economy of KITE's `inst/examples/example.R`, built by the same code with the same seed. A test checks that it is identical to `run_kite_example()`.
- `generate_baseline`: synthetic economies of other sizes by the same construction.
- CSV directories given with `--baseline-dir`: one file per table, named after the table (`trade_share.csv`, `value_added.csv` and so on), with one column per dimension and then `value`. `describe_baseline` lists the tables and their dimensions; `src/baseline.rs` has the full specification. Loading checks the columns, that every code is known, that each table covers its full grid once, that shares sum to 1 within 1e-9, and that factors are positive.

The synthetic baselines are random draws and describe no real economy. They are also not equilibria of the models: their value added is drawn independently of the shares, so a run with an empty scenario does not return a welfare change of 1, and every result on them includes that adjustment. `DESIGN.md` gives the figures.

## REST interface

`kite-mcp serve --http` serves the same handlers over HTTP: `GET /v1/models`, `GET /v1/baselines`, `GET /v1/baselines/{id}`, `POST /v1/baselines/generate`, `POST /v1/runs`, `POST /v1/runs/table`, `POST /v1/runs/compare`, `GET /v1/runs/{id}/reproduce.R` and `GET /v1/runs/{id}/provenance`. The OpenAPI 3.1 document at `/openapi.json` is generated from the same Rust types as the MCP tool schemas.

## Tests

```bash
cargo test
```

runs everything that needs no R. With R and KITE installed:

```bash
KITE_MCP_CONFORMANCE=1 cargo test --test conformance --test http -- --test-threads=1
```

runs the conformance, reproduce-script and REST tests, and `Rscript -e 'testthat::test_dir("worker/tests/testthat")'` the worker's own tests. `eval/` holds ten questions put to a language model through this server, the transcripts, and a scorer that checks every number in each answer against direct KITE results (`eval/results.md`).

## Out of scope

- Synthetic data only. kite-mcp ships no calibrated baselines; KITE's README says these are available from the Kiel Institute on request.
- The three models KITE publishes: `caliendo_parro_2015`, `chowdhry_hinz_kamin_wanner_2022` and `mahlkow_wanner_2021`.
- No `.ice` files yet. KITE plans to read them from version 26.11 (issue #19) and has not published the format.

## Cite KITE

Cite KITE through its white paper, as KITE's README asks:

> Hinz, J., Mahlkow, H. and Wanner, J. (2025). The KITE Model Suite: A Quantitative Framework for International Trade Analysis. KITE White Paper, Kiel Institute for the World Economy. https://trade.kielinstitut.de/KTTM/KITE_whitepaper.pdf

`kite://citation` gives the BibTeX entry, and `kite://models/{id}/citation` each model's own reference.

## Licence

GPL-3, the licence of KITE. `worker/synthetic.R` adapts the construction of KITE's `inst/examples/example.R`, copyright the KITE authors.
