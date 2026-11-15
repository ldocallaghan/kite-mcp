# Answers to the evaluation questions (eval/questions.json), computed by
# calling KITE directly. The scorer accepts a number in a model's answer only
# if it matches a value here, within the rounding the answer shows.
#
# Usage: Rscript --vanilla eval/answers.R eval/answers.json

args <- commandArgs(trailingOnly = TRUE)
here <- dirname(normalizePath(sub("^--file=", "", grep("^--file=", commandArgs(FALSE), value = TRUE))))
source(file.path(here, "..", "tests", "conformance", "encode.R"))
source(file.path(here, "..", "worker", "synthetic.R"))
suppressPackageStartupMessages({ library(KITE); library(data.table) })

settings <- list(verbose = 0L, tolerance = 1e-4, vfactor = 0.1, max_iterations = 200)

run <- function(model, ic, scenario, s = settings) {
  warnings <- list()
  withCallingHandlers({
    result <- update_equilibrium(model = model, initial_conditions = ic, model_scenario = scenario, settings = s)
    processed <- process_results(result)
  }, warning = function(w) {
    warnings[[length(warnings) + 1L]] <<- list(class = as.list(class(w)), message = conditionMessage(w))
    invokeRestart("muffleWarning")
  })
  conf_case(result$info, processed$output, warnings)
}

ex <- kite_mcp_synthetic_baseline(3L, 2L, 1L)
ic <- ex$initial_conditions
ic_carbon <- ic
ic_carbon$price <- ex$price
ic_carbon$carbon_intensity <- ex$carbon_intensity

war <- function(ic, rate = 1.2) {
  t <- data.table::copy(ic$tariff)
  t[origin == "c1" & destination == "c2", value := rate]
  t[origin == "c2" & destination == "c1", value := rate]
  t
}
club <- list(carbon_tax = 0.5, countries_climate_club = c("c1", "c3"),
             scenario_carbon_tariff = TRUE, scenario_export_rebate = TRUE)

cases <- list()
cases$q01_cp_war <- run(caliendo_parro_2015, ic, list(tariff_new = war(ic)))
cases$q02_chkw_war <- run(chowdhry_hinz_kamin_wanner_2022, ic, list(tariff_new = war(ic)))
cases$q03_mw_club <- run(mahlkow_wanner_2021, ic_carbon, club)

s1 <- data.table::copy(ic$tariff)
s1[destination == "c1" & sector == "s1" & origin != destination, value := 1.25]
cases$q04_cp_s1_tariff <- run(caliendo_parro_2015, ic, list(tariff_new = s1))

cases$q05_mw_war <- run(mahlkow_wanner_2021, ic_carbon, list(tariff_new = war(ic_carbon)))
cases$q05_mw_war_club <- run(mahlkow_wanner_2021, ic_carbon, c(list(tariff_new = war(ic_carbon)), club))

g <- kite_mcp_synthetic_baseline(5L, 3L, 7L)$initial_conditions
cases$q06_gen_cp_war <- run(caliendo_parro_2015, g, list(tariff_new = war(g)))

cases$q07_cp_war_fcs <- run(caliendo_parro_2015, ic, list(tariff_new = war(ic)),
                            c(settings, list(trade_balance_rule = "fixed_country_share")))

# caliendo_parro_2015 reads productivity_change; an answer may offer this run
prod <- data.table::CJ(country = c("c1", "c2", "c3"), sector = c("s1", "s2"), sorted = FALSE)[, value := 1]
prod[country == "c3", value := 1.1]
cases$q08_cp_productivity <- run(caliendo_parro_2015, ic, list(productivity_change = prod))

ntb <- data.table::copy(ic$ntb)
ntb[destination == "c3" & origin != destination, value := 1.1]
cases$q10_cp_ntb <- run(caliendo_parro_2015, ic, list(ntb_new = ntb))

stopifnot(isTRUE(cases$q01_cp_war$info$convergence$logical))
stopifnot(identical(cases$q07_cp_war_fcs$info$convergence$logical, FALSE))
conf_write(cases, args[[1]])
cat("wrote", length(cases), "cases\n")
