# Answers to the held-out questions (eval/heldout/questions.json), computed by
# calling KITE directly. Written and committed before any model saw the
# questions; scored by the same scorer as eval/.
#
# Usage: Rscript --vanilla eval/heldout/answers.R eval/heldout/answers.json

args <- commandArgs(trailingOnly = TRUE)
here <- dirname(normalizePath(sub("^--file=", "", grep("^--file=", commandArgs(FALSE), value = TRUE))))
source(file.path(here, "..", "..", "tests", "conformance", "encode.R"))
source(file.path(here, "..", "..", "worker", "synthetic.R"))
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

cases <- list()

# h01: generated 4 x 2, seed 11; c3 levies 15 % on all imports from c1
g <- kite_mcp_synthetic_baseline(4L, 2L, 11L)$initial_conditions
t1 <- data.table::copy(g$tariff)
t1[origin == "c1" & destination == "c3", value := 1.15]
cases$h01_gen_cp_tariff <- run(caliendo_parro_2015, g, list(tariff_new = t1))

# h02: carbon tax 0.3 in every country (the club defaults to all), no border measures
cases$h02_mw_tax_all <- run(mahlkow_wanner_2021, ic_carbon, list(carbon_tax = 0.3))

# h03: coalition c1 and c2; c3 levies 20 % on all imports from c1 and c2
coalition <- data.table::data.table(country = c("c1", "c2", "c3"), value = c(1L, 1L, 0L))
t3 <- data.table::copy(ic$tariff)
t3[destination == "c3" & origin %in% c("c1", "c2"), value := 1.2]
cases$h03_chkw_coalition <- run(chowdhry_hinz_kamin_wanner_2022, ic,
                                list(tariff_new = t3, coalition_member = coalition))

# h04: productivity of c2 in s2 rises 5 %
p4 <- data.table::CJ(country = c("c1", "c2", "c3"), sector = c("s1", "s2"), sorted = FALSE)[, value := 1]
p4[country == "c2" & sector == "s2", value := 1.05]
cases$h04_cp_productivity <- run(caliendo_parro_2015, ic, list(productivity_change = p4))

# h05 (trap): caliendo_parro_2015 does not read carbon_tax. An answer may
# offer the model that does; this is that run.
cases$h05_mw_tax_alternative <- run(mahlkow_wanner_2021, ic_carbon, list(carbon_tax = 0.5))

for (n in names(cases)) stopifnot(isTRUE(cases[[n]]$info$convergence$logical))
conf_write(cases, args[[1]])
cat("wrote", length(cases), "cases\n")
