# syntax=docker/dockerfile:1
# kite-mcp: an unofficial MCP server for KITE. `docker run -i` serves MCP on stdio.
#
#   docker build -t kite-mcp .
#   docker run -i --rm kite-mcp            # serve
#   docker run --rm kite-mcp doctor        # check the installation

FROM rust:1.97-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY worker ./worker
RUN cargo build --release --locked --bin kite-mcp

FROM rocker/r-ver:4.6.2
COPY scripts/install-r-deps.R /tmp/install-r-deps.R
# a GitHub token, when given, avoids rate limits on installing KITE from GitHub
RUN --mount=type=secret,id=GITHUB_PAT,env=GITHUB_PAT,required=false \
    Rscript /tmp/install-r-deps.R /usr/local/lib/R/site-library && rm /tmp/install-r-deps.R
COPY --from=build /src/target/release/kite-mcp /usr/local/bin/kite-mcp
COPY worker/kite_worker.R worker/codec.R worker/synthetic.R /usr/local/share/kite-mcp/worker/
RUN useradd --create-home kite
USER kite
ENTRYPOINT ["kite-mcp"]
CMD ["serve"]
