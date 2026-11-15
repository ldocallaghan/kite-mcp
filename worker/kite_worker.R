# kite-mcp R worker. Part of kite-mcp, GPL-3.
#
# Reads one JSON request per line on stdin and writes one JSON reply per line
# on stdout. The protocol is described in docs/worker-protocol.md.
#
# Nothing from a request is evaluated. Requests carry typed data only; this
# file turns them into R values with fixed code and calls KITE's exported
# functions. There is no call to eval(), parse() or source() on request data.

kite_mcp_worker_dir <- local({
  override <- Sys.getenv("KITE_MCP_WORKER_DIR")
  if (nzchar(override)) return(override)
  args <- commandArgs(trailingOnly = FALSE)
  file_arg <- sub("^--file=", "", args[grepl("^--file=", args)])
  if (length(file_arg) == 1L) dirname(normalizePath(file_arg)) else getwd()
})

# the worker's own files, located next to this script
source(file.path(kite_mcp_worker_dir, "codec.R"), local = FALSE)
source(file.path(kite_mcp_worker_dir, "synthetic.R"), local = FALSE)

suppressPackageStartupMessages({
  library(data.table)
  library(KITE)
})

KITE_MCP_PROTOCOL_VERSION <- 1L

kite_mcp_models <- list(
  caliendo_parro_2015 = KITE::caliendo_parro_2015,
  chowdhry_hinz_kamin_wanner_2022 = KITE::chowdhry_hinz_kamin_wanner_2022,
  mahlkow_wanner_2021 = KITE::mahlkow_wanner_2021
)

kite_mcp_pkg_version <- function(p) {
  tryCatch(as.character(utils::packageVersion(p)), error = function(e) NULL)
}

# ---- methods -----------------------------------------------------------------

kite_mcp_hello <- function(params) {
  desc <- utils::packageDescription("KITE")
  list(
    protocol = KITE_MCP_PROTOCOL_VERSION,
    language = "R",
    runtime_version = R.version.string,
    platform = R.version$platform,
    model_package = list(name = "KITE",
                         version = desc$Version,
                         release_name = if (is.null(desc$ReleaseName)) NULL else desc$ReleaseName,
                         commit = if (is.null(desc$RemoteSha)) NULL else desc$RemoteSha),
    packages = list(data.table = kite_mcp_pkg_version("data.table"),
                    cli = kite_mcp_pkg_version("cli"),
                    jsonlite = kite_mcp_pkg_version("jsonlite")),
    rng_kind = as.list(RNGkind()),
    models = as.list(names(kite_mcp_models))
  )
}

kite_mcp_int_param <- function(params, name, lo, hi) {
  x <- params[[name]]
  if (is.null(x) || !is.numeric(x) || length(x) != 1L || x != round(x) || x < lo || x > hi) {
    kite_mcp_protocol_error(sprintf("`%s` must be an integer from %d to %d", name, lo, hi))
  }
  as.integer(x)
}

kite_mcp_synthetic <- function(params) {
  n_c <- kite_mcp_int_param(params, "countries", 2L, 200L)
  n_s <- kite_mcp_int_param(params, "sectors", 1L, 200L)
  seed <- kite_mcp_int_param(params, "seed", 0L, .Machine$integer.max)
  b <- kite_mcp_synthetic_baseline(n_c, n_s, seed)
  ic <- b$initial_conditions
  tables <- list()
  for (n in names(ic)) {
    if (n == "elasticities") {
      tables[[length(tables) + 1L]] <- list(name = "trade_elasticity",
                                            value = list(table = kite_mcp_encode_table(ic$elasticities$trade_elasticity)))
    } else {
      tables[[length(tables) + 1L]] <- list(name = n, value = list(table = kite_mcp_encode_table(ic[[n]])))
    }
  }
  tables[[length(tables) + 1L]] <- list(name = "price", value = list(table = kite_mcp_encode_table(b$price)))
  tables[[length(tables) + 1L]] <- list(name = "carbon_intensity", value = list(table = kite_mcp_encode_table(b$carbon_intensity)))
  list(initial_conditions = tables)
}

# Build an R list from ordered [{name, value}] pairs. `trade_elasticity`
# goes under `elasticities` at its position, as KITE's example does.
kite_mcp_named_list <- function(pairs, what) {
  out <- list()
  for (p in pairs) {
    name <- p$name
    if (!is.character(name) || length(name) != 1L || !grepl("^[a-z][a-z0-9_]*$", name)) {
      kite_mcp_protocol_error(sprintf("invalid %s name", what))
    }
    value <- kite_mcp_decode_value(p$value)
    if (name == "trade_elasticity" && what == "initial condition") {
      out[["elasticities"]] <- list(trade_elasticity = value)
    } else {
      out[[name]] <- value
    }
  }
  out
}

kite_mcp_solve <- function(params) {
  model_id <- params$model
  if (!is.character(model_id) || length(model_id) != 1L || is.null(kite_mcp_models[[model_id]])) {
    kite_mcp_protocol_error("unknown model")
  }
  model_fn <- kite_mcp_models[[model_id]]
  ic <- kite_mcp_named_list(params$initial_conditions, "initial condition")
  scenario <- kite_mcp_named_list(params$scenario, "scenario variable")
  settings <- kite_mcp_named_list(params$settings, "setting")
  settings$verbose <- 0L

  warnings <- list()
  messages <- list()
  stage <- "update_equilibrium"
  record <- function(type) function(c) {
    entry <- list(stage = stage, class = as.list(class(c)), message = conditionMessage(c))
    if (type == "warning") {
      warnings[[length(warnings) + 1L]] <<- entry
      invokeRestart("muffleWarning")
    } else {
      messages[[length(messages) + 1L]] <<- entry
      invokeRestart("muffleMessage")
    }
  }

  withCallingHandlers({
    result <- update_equilibrium(model = model_fn,
                                 initial_conditions = ic,
                                 model_scenario = scenario,
                                 settings = settings)
    stage <- "process_results"
    processed <- process_results(result)
  }, warning = record("warning"), message = record("message"))

  info <- result$info
  acc <- info$accounting
  tables <- list()
  values <- list()
  for (n in names(processed$output)) {
    x <- processed$output[[n]]
    if (is.data.frame(x)) {
      tables[[length(tables) + 1L]] <- list(name = n, table = kite_mcp_encode_table(x))
    } else {
      values[[length(values) + 1L]] <- list(name = n, value = kite_mcp_scalar(x))
    }
  }

  list(
    info = list(
      convergence = kite_mcp_scalar(info$convergence),
      criterion = kite_mcp_scalar(info$criterion),
      iterations = kite_mcp_scalar(info$iterations),
      elapsed_seconds = kite_mcp_scalar(info$elapsed_seconds),
      accounting = list(
        ok = kite_mcp_scalar(acc$ok),
        rule = kite_mcp_scalar(acc$rule),
        world_trade_balance = kite_mcp_scalar(acc$world_trade_balance),
        baseline_world_trade_balance = kite_mcp_scalar(acc$baseline_world_trade_balance),
        world_value_added = kite_mcp_scalar(acc$world_value_added),
        tolerance = kite_mcp_scalar(acc$tolerance),
        reason = kite_mcp_scalar(acc$reason)
      )
    ),
    warnings = warnings,
    messages = messages,
    tables = tables,
    values = values
  )
}

# Plain-text help page of one of a fixed set of KITE topics.
kite_mcp_doc_topics <- c("KITE-package", "caliendo_parro_2015", "chowdhry_hinz_kamin_wanner_2022",
                         "mahlkow_wanner_2021", "update_equilibrium", "process_results",
                         "update_trade_balance")

kite_mcp_documentation <- function(params) {
  topic <- params$topic
  if (!is.character(topic) || length(topic) != 1L || !topic %in% kite_mcp_doc_topics) {
    kite_mcp_protocol_error(sprintf("`topic` must be one of %s", paste(kite_mcp_doc_topics, collapse = ", ")))
  }
  db <- tools::Rd_db("KITE")
  rd <- db[[paste0(topic, ".Rd")]]
  if (is.null(rd)) kite_mcp_protocol_error(sprintf("KITE has no help page `%s`", topic))
  text <- utils::capture.output(tools::Rd2txt(rd, options = list(underline_titles = FALSE)))
  list(topic = topic, text = paste(text, collapse = "\n"))
}

kite_mcp_methods <- list(
  hello = kite_mcp_hello,
  synthetic_baseline = kite_mcp_synthetic,
  solve = kite_mcp_solve,
  documentation = kite_mcp_documentation
)

# ---- loop --------------------------------------------------------------------

kite_mcp_reply <- function(x) {
  json <- jsonlite::toJSON(x, auto_unbox = TRUE, null = "null", digits = NA)
  cat(json, "\n", sep = "", file = stdout())
  flush(stdout())
}

kite_mcp_handle_line <- function(line) {
  req <- tryCatch(jsonlite::fromJSON(line, simplifyVector = FALSE),
                  error = function(e) NULL)
  if (is.null(req) || !is.list(req)) {
    return(list(id = NULL, ok = FALSE,
                error = list(class = list("kite_mcp_protocol_error"), message = "request is not a JSON object")))
  }
  id <- req$id
  method <- req$method
  if (!is.character(method) || length(method) != 1L || is.null(kite_mcp_methods[[method]])) {
    return(list(id = id, ok = FALSE,
                error = list(class = list("kite_mcp_protocol_error"), message = "unknown method")))
  }
  result <- NULL
  err <- NULL
  console <- utils::capture.output({
    result <- tryCatch(kite_mcp_methods[[method]](req$params),
                       error = function(e) { err <<- e; NULL })
  })
  if (length(console) > 0L) cat(console, sep = "\n", file = stderr())
  if (!is.null(err)) {
    return(list(id = id, ok = FALSE,
                error = list(class = as.list(class(err)), message = conditionMessage(err))))
  }
  list(id = id, ok = TRUE, result = result)
}

kite_mcp_main <- function() {
  con <- file("stdin", open = "r")
  on.exit(close(con))
  repeat {
    line <- readLines(con, n = 1L, warn = FALSE)
    if (length(line) == 0L) break
    if (!nzchar(trimws(line))) next
    kite_mcp_reply(kite_mcp_handle_line(line))
  }
}

if (!identical(Sys.getenv("KITE_MCP_WORKER_NO_MAIN"), "1")) kite_mcp_main()
