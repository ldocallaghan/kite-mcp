test_that("hello reports the protocol, KITE version and commit", {
  r <- call_worker("hello")
  expect_true(r$ok)
  expect_identical(r$result$protocol, 1L)
  expect_identical(r$result$model_package$version, "26.10")
  expect_identical(r$result$model_package$commit, "5995dd1e67803d0a499b2badbd8c3d859e08fdc5")
  expect_setequal(unlist(r$result$models),
                  c("caliendo_parro_2015", "chowdhry_hinz_kamin_wanner_2022", "mahlkow_wanner_2021"))
})

test_that("the synthetic baseline for 3 countries, 2 sectors, seed 1 is the example's", {
  ex <- example_results()
  r <- call_worker("synthetic_baseline", list(countries = 3L, sectors = 2L, seed = 1L))
  expect_true(r$ok)
  pairs <- r$result$initial_conditions
  ic <- kite_mcp_named_list(Filter(function(p) !p$name %in% c("price", "carbon_intensity"), pairs),
                            "initial condition")
  # update_equilibrium returns the initial conditions it was given
  expect_identical(ic, ex$cp$initial_conditions)
  ic_mw <- kite_mcp_named_list(pairs, "initial condition")
  expect_identical(ic_mw, ex$mw$initial_conditions)
})

test_that("unknown methods and malformed lines give protocol errors", {
  r <- call_worker("eval", list(code = "1 + 1"))
  expect_false(r$ok)
  expect_identical(r$error$message, "unknown method")
  bad <- kite_mcp_handle_line("{not json")
  expect_false(bad$ok)
})

solve_example <- function(model, scenario) {
  call_worker("solve", list(model = model,
                            initial_conditions = synthetic_pairs(model),
                            scenario = scenario,
                            settings = example_settings))
}

expect_same_as_direct <- function(reply, direct) {
  expect_true(reply$ok)
  tables <- decode_tables(reply$result)
  direct_out <- direct$output
  table_names <- names(direct_out)[vapply(direct_out, is.data.frame, logical(1))]
  expect_setequal(names(tables), table_names)
  for (n in table_names) {
    expect_identical(tables[[n]], data.table::as.data.table(direct_out[[n]]), label = n)
  }
}

test_that("caliendo_parro_2015 through the worker equals the example, bit for bit", {
  ex <- example_results()
  r <- solve_example("caliendo_parro_2015",
                     list(list(name = "tariff_new", value = tab(tariff_war_table()))))
  expect_same_as_direct(r, ex$cp_summary)
  expect_identical(r$result$info$convergence$logical, TRUE)
  expect_identical(r$result$info$iterations$integer, 22L)
  expect_identical(kite_mcp_hex_to_f64(r$result$info$criterion$double), ex$cp$info$criterion)
  expect_length(r$result$warnings, 0)
})

test_that("chowdhry_hinz_kamin_wanner_2022 through the worker equals the example, bit for bit", {
  ex <- example_results()
  r <- solve_example("chowdhry_hinz_kamin_wanner_2022",
                     list(list(name = "tariff_new", value = tab(tariff_war_table()))))
  expect_same_as_direct(r, ex$chkw_summary)
  expect_identical(r$result$info$iterations$integer, 25L)
  expect_identical(kite_mcp_hex_to_f64(r$result$info$criterion$double), ex$chkw$info$criterion)
})

test_that("mahlkow_wanner_2021 through the worker equals the example, bit for bit", {
  ex <- example_results()
  r <- solve_example("mahlkow_wanner_2021", list(
    list(name = "carbon_tax", value = dbl(0.5)),
    list(name = "countries_climate_club", value = list(strings = list("c1", "c3"))),
    list(name = "scenario_carbon_tariff", value = list(logical = TRUE)),
    list(name = "scenario_export_rebate", value = list(logical = TRUE))))
  expect_same_as_direct(r, ex$mw_summary)
  expect_identical(r$result$info$iterations$integer, 21L)
  expect_identical(kite_mcp_hex_to_f64(r$result$info$criterion$double), ex$mw$info$criterion)
})

test_that("a scenario variable the model does not read returns KITE's warning with its class", {
  r <- solve_example("chowdhry_hinz_kamin_wanner_2022", list(
    list(name = "tariff_new", value = tab(tariff_war_table())),
    list(name = "carbon_tax", value = dbl(0.5))))
  expect_true(r$ok)
  classes <- lapply(r$result$warnings, function(w) unlist(w$class))
  hit <- vapply(classes, function(cl) "kite_unknown_scenario_variable" %in% cl, logical(1))
  expect_true(any(hit))
  w <- r$result$warnings[[which(hit)[1]]]
  expect_identical(w$stage, "update_equilibrium")
  expect_match(w$message, "carbon_tax")
})

test_that("fixed_country_share after the tariff war does not converge and says why", {
  settings <- c(example_settings,
                list(list(name = "trade_balance_rule", value = list(string = "fixed_country_share"))))
  r <- call_worker("solve", list(model = "caliendo_parro_2015",
                                 initial_conditions = synthetic_pairs("caliendo_parro_2015"),
                                 scenario = list(list(name = "tariff_new", value = tab(tariff_war_table()))),
                                 settings = settings))
  expect_true(r$ok)
  expect_identical(r$result$info$convergence$logical, FALSE)
  expect_identical(r$result$info$accounting$ok$logical, FALSE)
  classes <- unlist(lapply(r$result$warnings, function(w) unlist(w$class)))
  expect_true("kite_world_trade_balance" %in% classes)
  # the same inputs called directly give the same verdict
  ex <- example_results()
  direct <- suppressWarnings(update_equilibrium(caliendo_parro_2015, ex$cp$initial_conditions,
                                                ex$cp$model_scenario,
                                                list(verbose = 0L, tolerance = 1e-4, vfactor = 0.1,
                                                     max_iterations = 200,
                                                     trade_balance_rule = "fixed_country_share")))
  expect_false(direct$info$convergence)
  expect_identical(r$result$info$iterations$integer, direct$info$iterations)
})

test_that("a KITE error comes back as an error reply with its class", {
  r <- call_worker("solve", list(model = "mahlkow_wanner_2021",
                                 initial_conditions = synthetic_pairs("mahlkow_wanner_2021"),
                                 scenario = list(list(name = "carbon_tax", value = dbl(0.5)),
                                                 list(name = "countries_climate_club",
                                                      value = list(strings = list("c9")))),
                                 settings = example_settings))
  expect_false(r$ok)
  expect_true(length(r$error$class) >= 1)
  expect_true(nzchar(r$error$message))
})

test_that("the worker answers over real stdin and stdout", {
  rscript <- file.path(R.home("bin"), "Rscript")
  input <- tempfile(fileext = ".jsonl")
  writeLines(c('{"id":"a","method":"hello","params":null}',
               '{"id":"b","method":"nope"}'), input)
  out <- system2(rscript, c(shQuote(file.path(worker_dir, "kite_worker.R"))),
                 stdin = input, stdout = TRUE, stderr = FALSE,
                 env = c("KITE_MCP_WORKER_NO_MAIN=0", "KITE_MCP_WORKER_DIR="))
  expect_length(out, 2)
  a <- jsonlite::fromJSON(out[1], simplifyVector = FALSE)
  b <- jsonlite::fromJSON(out[2], simplifyVector = FALSE)
  expect_identical(a$id, "a"); expect_true(a$ok)
  expect_identical(b$id, "b"); expect_false(b$ok)
})
