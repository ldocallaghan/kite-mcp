# Test-side encoding of KITE results for the conformance tests. Written
# separately from worker/codec.R so that a fault in the worker's encoding
# cannot hide itself. Doubles are the 16 hexadecimal digits of their
# big-endian IEEE 754 bit pattern.

conf_hex <- function(x) {
  if (length(x) == 0L) return(character())
  raw <- writeBin(as.double(x), raw(), size = 8L, endian = "big")
  hex <- sprintf("%02x", as.integer(raw))
  vapply(seq_along(x), function(i) paste(hex[(8L * i - 7L):(8L * i)], collapse = ""), character(1))
}

conf_column <- function(name, x) {
  if (is.factor(x)) x <- as.character(x)
  type <- if (is.character(x)) "string" else if (is.double(x)) "double" else if (is.integer(x)) "integer" else if (is.logical(x)) "logical" else stop("unsupported column type")
  values <- switch(type,
                   string = lapply(x, function(v) if (is.na(v)) NULL else v),
                   double = as.list(conf_hex(x)),
                   integer = lapply(x, function(v) if (is.na(v)) NULL else v),
                   logical = lapply(x, function(v) if (is.na(v)) NULL else v))
  list(name = name, type = type, values = values)
}

conf_table <- function(dt) list(columns = lapply(names(dt), function(n) conf_column(n, dt[[n]])))

conf_scalar <- function(x) {
  if (is.null(x) || length(x) == 0L) return(NULL)
  x <- x[[1]]
  if (is.double(x)) return(list(double = conf_hex(x)))
  if (is.na(x)) return(NULL)
  if (is.integer(x)) return(list(integer = x))
  if (is.logical(x)) return(list(logical = x))
  list(string = as.character(x))
}

conf_info <- function(info) {
  a <- info$accounting
  list(convergence = conf_scalar(info$convergence),
       criterion = conf_scalar(info$criterion),
       iterations = conf_scalar(info$iterations),
       accounting = list(ok = conf_scalar(a$ok), rule = conf_scalar(a$rule),
                         world_trade_balance = conf_scalar(a$world_trade_balance),
                         baseline_world_trade_balance = conf_scalar(a$baseline_world_trade_balance),
                         world_value_added = conf_scalar(a$world_value_added),
                         tolerance = conf_scalar(a$tolerance),
                         reason = conf_scalar(a$reason)))
}

conf_case <- function(info, output, warnings = list()) {
  tables <- list()
  for (n in names(output)) {
    if (is.data.frame(output[[n]])) tables[[n]] <- conf_table(output[[n]])
  }
  list(info = conf_info(info), tables = tables, warnings = warnings)
}

conf_write <- function(x, path) {
  writeLines(jsonlite::toJSON(x, auto_unbox = TRUE, null = "null", digits = NA), path)
}
