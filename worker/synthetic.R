# Synthetic initial conditions for KITE.
#
# Adapted from `run_kite_example()` in KITE's inst/examples/example.R
# (KITE 26.10, commit 5995dd1, https://github.com/julianhinz/KITE),
# Copyright the KITE authors, GPL-3. This file is part of kite-mcp, also GPL-3.
#
# With n_countries = 3, n_sectors = 2 and seed = 1 the random draws, their
# order and the resulting tables are those of the example. For other sizes
# three choices are kite-mcp's own, since the example fixes them for 3
# countries and 2 sectors only:
#   * country codes are "c1", "c2", ... padded with zeros to equal width when
#     there are 10 or more countries ("c01" ... "c12");
#   * coalition_member is 1 for odd-numbered countries and 0 otherwise
#     (the example's c(1, 0, 1));
#   * carbon_intensity is 0.4 for the first sector, 0.1 for the second and 0
#     for any further sector (the example's c(0.4, 0.1)).
#
# The function returns the initial conditions in the order the example builds
# them, plus `price` and `carbon_intensity`, which the example adds only for
# mahlkow_wanner_2021.

kite_mcp_synthetic_baseline <- function(n_countries = 3L, n_sectors = 2L, seed = 1L) {
  set.seed(seed)
  width <- if (n_countries >= 10) nchar(as.character(n_countries)) else 1L
  countries <- sprintf(paste0("c%0", width, "d"), seq_len(n_countries))
  sectors <- paste0("s", seq_len(n_sectors))
  n_c <- length(countries); n_s <- length(sectors)

  dt_3d <- function(values, names_list) {
    grid <- data.table::CJ(d1 = names_list[[1]],
                           d2 = names_list[[2]],
                           d3 = names_list[[3]],
                           sorted = FALSE)
    data.table::setnames(grid, c("d1", "d2", "d3"), names(names_list))
    grid[, value := values][]
  }
  dt_2d <- function(values, names_list) {
    grid <- data.table::CJ(d1 = names_list[[1]], d2 = names_list[[2]], sorted = FALSE)
    data.table::setnames(grid, c("d1", "d2"), names(names_list))
    grid[, value := values][]
  }
  dt_1d <- function(values, name, levels) {
    dt <- data.table::data.table(d1 = levels, value = values)
    data.table::setnames(dt, "d1", name); dt[]
  }

  ts_arr <- array(runif(n_c * n_c * n_s, 0.3, 1), c(n_c, n_c, n_s),
                  dimnames = list(origin = countries,
                                  destination = countries,
                                  sector = sectors))
  ts_norm <- sweep(ts_arr, 2:3, apply(ts_arr, 2:3, sum), "/")
  trade_share <- dt_3d(as.numeric(aperm(ts_norm, c(3, 2, 1))),
                       list(origin = countries,
                            destination = countries,
                            sector = sectors))

  is_arr <- array(runif(n_c * n_s * n_s, 0.3, 1), c(n_c, n_s, n_s),
                  dimnames = list(country = countries,
                                  input = sectors, output = sectors))
  is_norm <- sweep(is_arr, c(1, 3), apply(is_arr, c(1, 3), sum), "/")
  intermediate_share <- dt_3d(as.numeric(aperm(is_norm, c(3, 2, 1))),
                              list(country = countries,
                                   input = sectors, output = sectors))

  factor_share <- dt_2d(
    as.numeric(t(matrix(runif(n_c * n_s, 0.25, 0.55), n_c, n_s))),
    list(country = countries, sector = sectors))

  cs_raw <- matrix(runif(n_c * n_s, 0.3, 1), n_c, n_s)
  consumption_share <- dt_2d(as.numeric(t(cs_raw / rowSums(cs_raw))),
                             list(country = countries, sector = sectors))

  expenditure <- dt_2d(
    as.numeric(t(matrix(runif(n_c * n_s, 100, 200), n_c, n_s))),
    list(destination = countries, sector = sectors))

  value_added <- dt_1d(runif(n_c, 100, 200), "country", countries)

  tb_raw <- runif(n_c, -5, 5)
  trade_balance <- dt_1d(tb_raw - mean(tb_raw), "country", countries)

  trade_elasticity <- dt_1d(runif(n_s, 4, 6), "sector", sectors)

  identity_tariff <- dt_3d(rep(1, n_c * n_c * n_s),
                           list(origin = countries,
                                destination = countries,
                                sector = sectors))

  coalition <- as.integer(seq_len(n_c) %% 2L == 1L)

  ic <- list(
    trade_share = trade_share,
    intermediate_share = intermediate_share,
    factor_share = factor_share,
    consumption_share = consumption_share,
    expenditure = expenditure,
    value_added = value_added,
    trade_balance = trade_balance,
    elasticities = list(trade_elasticity = trade_elasticity),
    tariff = identity_tariff,
    ntb = data.table::copy(identity_tariff),
    export_subsidy = data.table::copy(identity_tariff),
    coalition_member = dt_1d(coalition, "country", countries)
  )

  # drawn after the scenario-independent tables, as in the example
  price <- dt_2d(runif(n_c * n_s, 0.8, 1.2),
                 list(country = countries, sector = sectors))
  intensity <- c(0.4, 0.1, rep(0, max(0L, n_s - 2L)))[seq_len(n_s)]
  carbon_intensity <- dt_1d(intensity, "sector", sectors)

  list(initial_conditions = ic,
       price = price,
       carbon_intensity = carbon_intensity)
}
