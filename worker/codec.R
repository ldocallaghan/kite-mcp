# Encoding and decoding of values for the kite-mcp worker protocol.
# See docs/worker-protocol.md. Part of kite-mcp, GPL-3.
#
# Doubles cross the protocol as 16 hexadecimal digits: the big-endian bit
# pattern of the IEEE 754 binary64 value. This keeps every bit, including the
# sign of zero and the payload of R's NA_real_.

kite_mcp_f64_to_hex <- function(x) {
  x <- as.double(x)
  if (length(x) == 0L) return(character())
  bytes <- as.character(writeBin(x, raw(), size = 8L, endian = "big"))
  m <- matrix(bytes, nrow = 8L)
  apply(m, 2L, paste, collapse = "")
}

kite_mcp_hex_to_f64 <- function(h) {
  h <- as.character(h)
  if (length(h) == 0L) return(double())
  if (!all(grepl("^[0-9a-f]{16}$", h))) {
    stop(structure(class = c("kite_mcp_protocol_error", "error", "condition"),
                   list(message = "a double must be 16 lower-case hexadecimal digits", call = NULL)))
  }
  all_hex <- paste(h, collapse = "")
  starts <- seq.int(1L, nchar(all_hex), by = 2L)
  bytes <- as.raw(strtoi(substring(all_hex, starts, starts + 1L), 16L))
  readBin(bytes, "double", n = length(h), size = 8L, endian = "big")
}

kite_mcp_protocol_error <- function(message) {
  stop(structure(class = c("kite_mcp_protocol_error", "error", "condition"),
                 list(message = message, call = NULL)))
}

# A table is {"columns": [{"name", "type", "values"}, ...]}.
# Column types: "string", "double", "integer", "logical".
# Missing values: null in string, integer and logical columns; doubles carry
# their own NA bit pattern.

kite_mcp_encode_column <- function(name, x) {
  if (is.factor(x)) x <- as.character(x)
  if (is.character(x)) {
    list(name = name, type = "string", values = as.list(x))
  } else if (is.double(x)) {
    list(name = name, type = "double", values = as.list(kite_mcp_f64_to_hex(x)))
  } else if (is.integer(x)) {
    list(name = name, type = "integer", values = lapply(x, function(v) if (is.na(v)) NULL else v))
  } else if (is.logical(x)) {
    list(name = name, type = "logical", values = lapply(x, function(v) if (is.na(v)) NULL else v))
  } else {
    kite_mcp_protocol_error(sprintf("column `%s` has unsupported type %s", name, class(x)[1]))
  }
}

kite_mcp_encode_table <- function(dt) {
  if (!is.data.frame(dt)) kite_mcp_protocol_error("only data frames can be encoded as tables")
  cols <- lapply(names(dt), function(n) kite_mcp_encode_column(n, dt[[n]]))
  list(columns = cols)
}

kite_mcp_decode_column <- function(col) {
  values <- col$values
  n <- length(values)
  null_to_na <- function(v, na) if (is.null(v)) na else v
  switch(col$type,
         string = vapply(values, function(v) null_to_na(v, NA_character_), character(1), USE.NAMES = FALSE),
         double = kite_mcp_hex_to_f64(vapply(values, as.character, character(1), USE.NAMES = FALSE)),
         integer = vapply(values, function(v) as.integer(null_to_na(v, NA_integer_)), integer(1), USE.NAMES = FALSE),
         logical = vapply(values, function(v) as.logical(null_to_na(v, NA)), logical(1), USE.NAMES = FALSE),
         kite_mcp_protocol_error(sprintf("column `%s` has unknown type `%s`", col$name, col$type)))
}

kite_mcp_decode_table <- function(tab) {
  cols <- tab$columns
  if (is.null(cols) || length(cols) == 0L) kite_mcp_protocol_error("a table needs at least one column")
  lens <- vapply(cols, function(c) length(c$values), integer(1))
  if (length(unique(lens)) != 1L) kite_mcp_protocol_error("table columns differ in length")
  out <- lapply(cols, kite_mcp_decode_column)
  names(out) <- vapply(cols, function(c) as.character(c$name), character(1))
  data.table::as.data.table(out)
}

# A scenario or setting value is one of
#   {"table": <table>}            -> data.table
#   {"double": "<hex>"}           -> numeric(1)
#   {"integer": <int>}            -> integer(1)
#   {"logical": true|false}       -> logical(1)
#   {"string": "<s>"}             -> character(1)
#   {"strings": ["a", "b"]}       -> character vector
kite_mcp_decode_value <- function(v) {
  if (!is.list(v) || length(v) != 1L || is.null(names(v))) {
    kite_mcp_protocol_error("a value must be an object with exactly one type key")
  }
  key <- names(v)[1]
  x <- v[[1]]
  switch(key,
         table = kite_mcp_decode_table(x),
         double = kite_mcp_hex_to_f64(x),
         integer = as.integer(x),
         logical = { if (!is.logical(x) || length(x) != 1L) kite_mcp_protocol_error("`logical` must be true or false"); x },
         string = as.character(x),
         strings = as.character(unlist(x)),
         kite_mcp_protocol_error(sprintf("unknown value type `%s`", key)))
}

# Scalars in the info block
kite_mcp_scalar <- function(x) {
  if (is.null(x) || length(x) == 0L) return(NULL)
  x <- x[[1]]
  if (is.double(x)) return(list(double = kite_mcp_f64_to_hex(x)))
  if (is.integer(x)) return(if (is.na(x)) NULL else list(integer = x))
  if (is.logical(x)) return(if (is.na(x)) NULL else list(logical = x))
  if (is.character(x)) return(if (is.na(x)) NULL else list(string = x))
  NULL
}
