# Reference results for the conformance tests: each case run by calling KITE
# directly, with no kite-mcp code in the path except the synthetic baseline
# construction of worker/synthetic.R (whose equality with KITE's example is
# tested in worker/tests). The Rust test tests/conformance.rs sends the same
# cases through the MCP server and requires identical bits.
#
# Usage: Rscript --vanilla tests/conformance/reference.R <out.json>

args <- commandArgs(trailingOnly = TRUE)
out <- args[[1]]
here <- dirname(normalizePath(sub("^--file=", "", grep("^--file=", commandArgs(FALSE), value = TRUE))))
source(file.path(here, "encode.R"))
source(file.path(here, "..", "..", "worker", "synthetic.R"))
suppressPackageStartupMessages({ library(KITE); library(data.table) })

settings <- list(verbose = 0L, tolerance = 1e-4, vfactor = 0.1, max_iterations = 200)

run_case <- function(model, ic, scenario, settings) {
  warnings <- list()
  withCallingHandlers({
    result <- update_equilibrium(model = model, initial_conditions = ic,
                                 model_scenario = scenario, settings = settings)
    processed <- process_results(result)
  }, warning = function(w) {
    warnings[[length(warnings) + 1L]] <<- list(class = as.list(class(w)), message = conditionMessage(w))
    invokeRestart("muffleWarning")
  })
  conf_case(result$info, processed$output, warnings)
}

# `rate`, not `value`: inside data.table, `value` on the right of := names the column
tariff_war <- function(ic, rate = 1.2) {
  t <- data.table::copy(ic$tariff)
  t[origin == "c1" & destination == "c2", value := rate]
  t[origin == "c2" & destination == "c1", value := rate]
  stopifnot(sum(t$value == rate) == 2L * length(unique(t$sector)))
  t
}

cases <- list()

# ---- KITE's own example, through run_kite_example() ----------------------------
source(system.file("examples", "example.R", package = "KITE"))
ex <- run_kite_example()
cases$example_cp <- conf_case(ex$cp$info, ex$cp_summary$output)
cases$example_chkw <- conf_case(ex$chkw$info, ex$chkw_summary$output)
cases$example_mw <- conf_case(ex$mw$info, ex$mw_summary$output)
ic <- ex$cp$initial_conditions
ic_carbon <- ex$mw$initial_conditions

# ---- a generated economy: 5 countries, 3 sectors, seed 7 -------------------------
g <- kite_mcp_synthetic_baseline(5L, 3L, 7L)
gic <- g$initial_conditions
gic_carbon <- gic
gic_carbon$price <- g$price
gic_carbon$carbon_intensity <- g$carbon_intensity
cases$gen5x3_cp <- run_case(caliendo_parro_2015, gic, list(tariff_new = tariff_war(gic)), settings)
cases$gen5x3_chkw <- run_case(chowdhry_hinz_kamin_wanner_2022, gic, list(tariff_new = tariff_war(gic)), settings)
cases$gen5x3_mw <- run_case(mahlkow_wanner_2021, gic_carbon,
                            list(carbon_tax = 0.5, countries_climate_club = c("c1", "c3"),
                                 scenario_carbon_tariff = TRUE, scenario_export_rebate = TRUE),
                            settings)

# ---- a carbon tax R's parser reads 1 bit away from its correctly rounded value ---
# The server reads the JSON number 0.00648016 with correct rounding; so does
# this reference, through the exact hexadecimal literal.
cases$awkward_literal_mw <- run_case(mahlkow_wanner_2021, ic_carbon,
                                     list(carbon_tax = 0x1.a8af0b45d9531p-8, countries_climate_club = c("c1", "c3"),
                                          scenario_carbon_tariff = TRUE, scenario_export_rebate = TRUE),
                                     settings)
stopifnot(!identical(0x1.a8af0b45d9531p-8, 0.00648016))

# ---- a tariff on one sector only: c1 levies 25 % on s1 imports from everyone -----
st <- data.table::copy(ic$tariff)
st[destination == "c1" & sector == "s1" & origin != destination, value := 1.25]
cases$sector_tariff_cp <- run_case(caliendo_parro_2015, ic, list(tariff_new = st), settings)

# ---- does not converge: fixed_country_share after the tariff war ----------------
cases$nonconv_cp <- run_case(caliendo_parro_2015, ic, list(tariff_new = tariff_war(ic)),
                             c(settings, list(trade_balance_rule = "fixed_country_share")))

# ---- a scenario variable the model does not read --------------------------------
cases$unread_chkw <- run_case(chowdhry_hinz_kamin_wanner_2022, ic,
                              list(tariff_new = tariff_war(ic), carbon_tax = 0.5), settings)

# ---- the example's baseline as CSV files, values with 17 significant digits ------
csv_dir <- if (length(args) >= 2) args[[2]] else NULL
if (!is.null(csv_dir)) {
  dir.create(csv_dir, showWarnings = FALSE, recursive = TRUE)
  tabs <- ic_carbon
  tabs$trade_elasticity <- tabs$elasticities$trade_elasticity
  tabs$elasticities <- NULL
  for (n in names(tabs)) {
    dt <- data.table::copy(tabs[[n]])
    if (is.double(dt$value)) dt[, value := sprintf("%.17g", value)]
    data.table::fwrite(dt, file.path(csv_dir, paste0(n, ".csv")))
  }
}

conf_write(cases, out)
cat("wrote", length(cases), "cases to", out, "\n")
