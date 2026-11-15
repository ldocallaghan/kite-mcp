# Worker protocol, version 1

The kite-mcp server does not run models itself. It starts one long-lived worker process, writes requests to the worker's standard input and reads replies from its standard output. The worker in this repository is `worker/kite_worker.R`, which runs KITE in R. The protocol does not depend on R: any program that follows this document can serve models to the server.

## Transport

Each request is one line of UTF-8 JSON on the worker's standard input, ended by a newline. Each reply is one line of JSON on the worker's standard output. The worker answers requests one at a time and in order. Nothing else may be written to standard output; logs and console output go to standard error, which the server passes through.

The server waits a fixed time for each reply (120 seconds by default, `--timeout-seconds`). If the time runs out, the worker exits or a reply cannot be parsed, the server stops the process and starts a new one for the next request. The worker keeps no state between requests, so a restart loses nothing.

## Messages

A request:

```json
{"id": "r7", "method": "solve", "params": {}}
```

A reply that succeeded:

```json
{"id": "r7", "ok": true, "result": {}}
```

A reply that failed:

```json
{"id": "r7", "ok": false, "error": {"class": ["kite_error", "error", "condition"], "message": "..."}}
```

`id` in the reply must equal `id` in the request. `error.class` lists the classes of the error condition, most specific first; a worker in a language without condition classes sends the name of the error type. An error from the model itself (for example KITE rejecting an unknown country code) is a failed reply. The server shows its class and message to the client unchanged.

## Doubles

A double is written as a string of 16 lower-case hexadecimal digits: the bit pattern of the IEEE 754 binary64 value, most significant byte first. `1.0` is `"3ff0000000000000"` and `-0.0` is `"8000000000000000"`. This keeps every bit, including the sign of zero, infinities and the payload of NaN values (R marks a missing value with the NaN whose low word is 1954). A decimal text form cannot do this in general: `tests/conformance` shows R's own parser reading 4 of 20,000 shortest decimals to a neighbouring double.

## Values

A value is a JSON object with exactly one key, which gives its type:

| Form | Meaning |
|---|---|
| `{"double": "3ff3333333333333"}` | one double |
| `{"integer": 3}` | one integer |
| `{"logical": true}` | one boolean |
| `{"string": "zero"}` | one string |
| `{"strings": ["c1", "c3"]}` | a vector of strings |
| `{"table": {...}}` | a table |

A table is a list of columns of equal length:

```json
{"columns": [
  {"name": "country", "type": "string", "values": ["c1", "c2"]},
  {"name": "value", "type": "double", "values": ["3ff0000000000000", "3ff3333333333333"]}
]}
```

Column types are `string`, `double`, `integer` and `logical`. In `string`, `integer` and `logical` columns a missing value is `null`. Initial conditions and scenario tables have their dimension columns first, as strings, and a last column named `value`.

A named value is `{"name": "tariff_new", "value": {...}}`. Lists of named values keep their order.

## Methods

### hello

No parameters. Returns what the worker is and runs:

```json
{"protocol": 1, "language": "R", "runtime_version": "R version 4.6.2 (2026-10-29)",
 "platform": "x86_64-pc-linux-gnu",
 "model_package": {"name": "KITE", "version": "26.10", "release_name": "Brisk Fehmarn",
                   "commit": "5995dd1e67803d0a499b2badbd8c3d859e08fdc5"},
 "packages": {"data.table": "1.18.6.1", "cli": "3.6.6", "jsonlite": "2.0.0"},
 "rng_kind": ["Mersenne-Twister", "Inversion", "Rejection"],
 "models": ["caliendo_parro_2015", "chowdhry_hinz_kamin_wanner_2022", "mahlkow_wanner_2021"]}
```

The server refuses a worker whose `protocol` differs from its own. `commit` goes into the provenance of every run.

### synthetic_baseline

Parameters `countries`, `sectors` and `seed`, all integers. Returns `{"initial_conditions": [named values]}`: the tables of a synthetic economy. The R worker builds them with `worker/synthetic.R`, the construction of KITE's example. A worker for another model suite may build its own, or answer with an error.

### solve

Parameters:

| Field | Content |
|---|---|
| `model` | a model id from `hello` |
| `initial_conditions` | named values: the baseline tables the model receives |
| `scenario` | named values: the counterfactual variables |
| `settings` | named values: solver settings |

The worker builds the model's inputs from these values with fixed code and runs the model. In the R worker that is `update_equilibrium()` followed by `process_results()`. The worker must not evaluate any request content as code.

The result:

| Field | Content |
|---|---|
| `info` | `convergence` (logical, or `null` when it cannot be established), `criterion`, `iterations`, `elapsed_seconds`, and `accounting` with `ok`, `rule`, `world_trade_balance`, `baseline_world_trade_balance`, `world_value_added`, `tolerance` and `reason`. Each field is a value or `null`. |
| `warnings` | every warning raised, each `{"stage", "class", "message"}`, in order |
| `messages` | every informational message, in the same form |
| `tables` | every output table, each `{"name", "table"}` |
| `values` | output entries that are not tables, each `{"name", "value"}` |

The R worker reports the stage as `update_equilibrium` or `process_results`. Warnings must not be dropped: the server passes every one to the client.

### documentation

Parameter `topic`, one of a fixed list. Returns `{"topic", "text"}`, the plain-text help page. The R worker reads KITE's installed help pages.

### read_decimals

Parameter `decimals`, a list of decimal numerals such as `"1.2"`. Returns `{"doubles": [...]}`, the double the worker's language reads each numeral as. The server calls this before it writes a reproduce script. A literal that reads back to a different double than the one the server used is written in hexadecimal form.

## Adding a worker

A worker for models in another language implements `hello`, `solve` and, if it can, `documentation`. The server's tool schemas, validation and model catalogue (`src/catalogue.rs`) describe KITE's three models. Serving other models means adding their entries there. The transport, the run store, the cache and the result contract stay as they are.
