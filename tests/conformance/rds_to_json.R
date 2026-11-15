# Encode the results a reproduce script saved (KITE_MCP_REPRODUCE_OUT) in the
# conformance test format.
# Usage: Rscript --vanilla tests/conformance/rds_to_json.R <in.rds> <out.json>
args <- commandArgs(trailingOnly = TRUE)
here <- dirname(normalizePath(sub("^--file=", "", grep("^--file=", commandArgs(FALSE), value = TRUE))))
source(file.path(here, "encode.R"))
x <- readRDS(args[[1]])
conf_write(conf_case(x$info, x$output), args[[2]])
