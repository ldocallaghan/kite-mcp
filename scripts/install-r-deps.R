# Install the R packages kite-mcp needs, and KITE at the pinned commit.
#
# Usage: Rscript scripts/install-r-deps.R [library]
# With no argument, packages go into the default library (R_LIBS_USER if set).

kite_commit <- "5995dd1e67803d0a499b2badbd8c3d859e08fdc5"

args <- commandArgs(trailingOnly = TRUE)
lib <- if (length(args) >= 1) args[[1]] else Sys.getenv("R_LIBS_USER", .libPaths()[1])
if (!nzchar(lib)) lib <- .libPaths()[1]
dir.create(lib, recursive = TRUE, showWarnings = FALSE)
.libPaths(c(lib, .libPaths()))

if (is.null(getOption("repos")) || identical(unname(getOption("repos")["CRAN"]), "@CRAN@")) {
  options(repos = c(CRAN = "https://cloud.r-project.org"))
}

needed <- c("data.table", "cli", "jsonlite", "testthat", "remotes")
missing <- needed[!vapply(needed, requireNamespace, logical(1), quietly = TRUE)]
if (length(missing) > 0) install.packages(missing, lib = lib)

installed_sha <- tryCatch(utils::packageDescription("KITE")$RemoteSha, error = function(e) NULL)
if (!identical(installed_sha, kite_commit)) {
  remotes::install_github(paste0("julianhinz/KITE@", kite_commit),
                          lib = lib, upgrade = "never", dependencies = TRUE)
}

desc <- utils::packageDescription("KITE")
cat("KITE", desc$Version, desc$RemoteSha, "\n")
stopifnot(identical(desc$RemoteSha, kite_commit))
