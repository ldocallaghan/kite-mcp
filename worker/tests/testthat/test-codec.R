test_that("doubles keep every bit through hex encoding and JSON", {
  set.seed(42)
  x <- c(0, -0, 1, -1, 1.2, 0.1, 1 / 3, pi, 1e-300, 4.9e-324, .Machine$double.xmax,
         .Machine$double.eps, Inf, -Inf, NaN, NA_real_, runif(500, -1e6, 1e6),
         exp(rnorm(500, 0, 50)))
  h <- kite_mcp_f64_to_hex(x)
  expect_true(all(grepl("^[0-9a-f]{16}$", h)))
  json <- jsonlite::toJSON(as.list(h), auto_unbox = TRUE)
  back <- kite_mcp_hex_to_f64(unlist(jsonlite::fromJSON(json, simplifyVector = FALSE)))
  expect_identical(writeBin(back, raw()), writeBin(x, raw()))
  expect_identical(kite_mcp_f64_to_hex(1), "3ff0000000000000")
  expect_identical(kite_mcp_f64_to_hex(-0), "8000000000000000")
})

test_that("malformed doubles are rejected", {
  expect_error(kite_mcp_hex_to_f64("3FF0000000000000"), class = "kite_mcp_protocol_error")
  expect_error(kite_mcp_hex_to_f64("1.2"), class = "kite_mcp_protocol_error")
})

test_that("tables of every column type round-trip through JSON", {
  dt <- data.table::data.table(country = c("c1", NA, "c3"),
                               value = c(1.2, NA, -0),
                               n = c(1L, NA, 3L),
                               flag = c(TRUE, NA, FALSE))
  json <- jsonlite::toJSON(kite_mcp_encode_table(dt), auto_unbox = TRUE, null = "null")
  back <- kite_mcp_decode_table(jsonlite::fromJSON(json, simplifyVector = FALSE))
  expect_identical(back, dt)
})

test_that("values decode to the documented R types", {
  expect_identical(kite_mcp_decode_value(list(double = "3ff3333333333333")), 1.2)
  expect_identical(kite_mcp_decode_value(list(logical = TRUE)), TRUE)
  expect_identical(kite_mcp_decode_value(list(strings = list("c1", "c3"))), c("c1", "c3"))
  expect_identical(kite_mcp_decode_value(list(string = "zero")), "zero")
  expect_identical(kite_mcp_decode_value(list(integer = 3)), 3L)
  expect_error(kite_mcp_decode_value(list(expression = "1 + 1")), class = "kite_mcp_protocol_error")
})
