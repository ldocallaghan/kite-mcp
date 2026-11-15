# Load the worker without starting its stdin loop.
worker_dir <- normalizePath(file.path(testthat::test_path(), "..", ".."))
Sys.setenv(KITE_MCP_WORKER_NO_MAIN = "1", KITE_MCP_WORKER_DIR = worker_dir)
source(file.path(worker_dir, "kite_worker.R"), local = FALSE)

# Send one request through the same path as a line on stdin: R value ->
# JSON text -> handler -> JSON text -> R value.
call_worker <- function(method, params = NULL, id = "t1") {
  line <- jsonlite::toJSON(list(id = id, method = method, params = params),
                           auto_unbox = TRUE, null = "null", digits = NA)
  reply <- kite_mcp_handle_line(as.character(line))
  json <- jsonlite::toJSON(reply, auto_unbox = TRUE, null = "null", digits = NA)
  jsonlite::fromJSON(as.character(json), simplifyVector = FALSE)
}

example_results <- local({
  cache <- NULL
  function() {
    if (is.null(cache)) {
      source(system.file("examples", "example.R", package = "KITE"), local = TRUE)
      cache <<- run_kite_example()
    }
    cache
  }
})

tab <- function(dt) list(table = kite_mcp_encode_table(dt))
dbl <- function(x) list(double = kite_mcp_f64_to_hex(x))

# The initial conditions as the server sends them for one model: the example
# list in order, with `price` and `carbon_intensity` for mahlkow_wanner_2021.
synthetic_pairs <- function(model, n_c = 3L, n_s = 2L, seed = 1L) {
  r <- call_worker("synthetic_baseline", list(countries = n_c, sectors = n_s, seed = seed))
  pairs <- r$result$initial_conditions
  if (model != "mahlkow_wanner_2021") {
    pairs <- Filter(function(p) !p$name %in% c("price", "carbon_intensity"), pairs)
  }
  pairs
}

example_settings <- list(
  list(name = "verbose", value = list(integer = 0L)),
  list(name = "tolerance", value = dbl(1e-4)),
  list(name = "vfactor", value = dbl(0.1)),
  list(name = "max_iterations", value = dbl(200))
)

tariff_war_table <- function(countries = c("c1", "c2", "c3"), sectors = c("s1", "s2")) {
  grid <- data.table::CJ(origin = countries, destination = countries, sector = sectors, sorted = FALSE)
  grid[, value := 1][origin == "c1" & destination == "c2", value := 1.2][origin == "c2" & destination == "c1", value := 1.2][]
}

decode_tables <- function(result) {
  out <- lapply(result$tables, function(t) kite_mcp_decode_table(t$table))
  names(out) <- vapply(result$tables, function(t) t$name, character(1))
  out
}
