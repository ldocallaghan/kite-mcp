# How R reads numeric literals, for the reproduce scripts.
#
# Input: CSV with columns decimal (Rust's shortest decimal), hex (the bits
# Rust parsed it to) and hexlit (the value as an R hexadecimal literal).
# Checks, and exits with status 1 if either fails:
#   1. every hexadecimal literal, read as R source, gives exactly the bits;
#   2. for every decimal, R's parser and as.numeric() give the same bits
#      (the worker decides with as.numeric(); scripts are read by the parser).
# Reports, without failing, how many decimals R reads to other bits than
# the correctly rounded value: the reason for check 1.
# This reads test-generated literals only.
# Usage: Rscript --vanilla tests/conformance/parse_check.R <pairs.csv>
args <- commandArgs(trailingOnly = TRUE)
here <- dirname(normalizePath(sub("^--file=", "", grep("^--file=", commandArgs(FALSE), value = TRUE))))
source(file.path(here, "encode.R"))
pairs <- utils::read.csv(args[[1]], colClasses = "character")
from_source <- function(s) vapply(s, function(x) eval(parse(text = x, keep.source = FALSE)[[1]]), numeric(1))
dec_parsed <- conf_hex(from_source(pairs$decimal))
dec_numeric <- conf_hex(as.numeric(pairs$decimal))
hex_parsed <- conf_hex(from_source(pairs$hexlit))
hex_bad <- which(hex_parsed != pairs$hex)
disagree <- which(dec_parsed != dec_numeric)
misread <- which(dec_parsed != pairs$hex)
cat("checked", nrow(pairs), "literals\n")
cat("hexadecimal literals read to other bits:", length(hex_bad), "\n")
cat("decimals where the parser and as.numeric() disagree:", length(disagree), "\n")
cat("decimals R reads to other bits than the correctly rounded value:", length(misread), "\n")
if (length(misread) > 0) {
  print(utils::head(data.frame(decimal = pairs$decimal[misread], correctly_rounded = pairs$hex[misread],
                               r_reads = dec_parsed[misread]), 10))
}
if (length(hex_bad) > 0 || length(disagree) > 0) quit(status = 1)
