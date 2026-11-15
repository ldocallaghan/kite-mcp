use clap::{Parser, Subcommand};
use kite_mcp::baseline;
use kite_mcp::mcp::KiteMcp;
use kite_mcp::service::App;
use kite_mcp::worker::{Worker, WorkerConfig};
use rmcp::ServiceExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// kite-mcp: an independent, unofficial MCP server for KITE trade policy
/// scenarios. Not affiliated with or endorsed by the Kiel Institute for the
/// World Economy.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Serve MCP on stdin and stdout (the default)
    Serve(ServeArgs),
    /// Check R, KITE and the worker, and say how to fix what is missing
    Doctor {
        /// Longest time one solve may take, in seconds
        #[arg(long, default_value_t = 120)]
        timeout_seconds: u64,
    },
}

#[derive(clap::Args)]
struct ServeArgs {
    /// A baseline read from a directory of CSV files, as ID=DIRECTORY; repeatable
    #[arg(long = "baseline-dir", value_name = "ID=DIRECTORY")]
    baseline_dirs: Vec<String>,
    /// Longest time one solve may take, in seconds, before the worker is restarted
    #[arg(long, default_value_t = 120)]
    timeout_seconds: u64,
    /// Serve the REST interface over HTTP in place of MCP on stdio
    #[arg(long)]
    http: bool,
    /// Address for --http
    #[arg(long, default_value = "127.0.0.1:8080")]
    addr: std::net::SocketAddr,
}

async fn build_app(args: &ServeArgs) -> anyhow::Result<Arc<App>> {
    let worker = Worker::new(WorkerConfig::r_from_env(Duration::from_secs(
        args.timeout_seconds,
    )));
    let app = App::new(worker);
    for spec in &args.baseline_dirs {
        let (id, dir) = spec
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("--baseline-dir takes ID=DIRECTORY, got `{spec}`"))?;
        let b = baseline::read_csv_dir(id, &PathBuf::from(dir))
            .map_err(|e| anyhow::anyhow!("baseline `{id}`: {e}"))?;
        app.add_baseline(b).await;
    }
    Ok(Arc::new(app))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("KITE_MCP_LOG")
                .unwrap_or_else(|_| "warn".into()),
        )
        .init();
    let cli = Cli::parse();
    let command = cli
        .command
        .unwrap_or_else(|| Cli::parse_from(["kite-mcp", "serve"]).command.unwrap());
    match command {
        Cmd::Doctor { timeout_seconds } => {
            let code = kite_mcp::doctor::run(Duration::from_secs(timeout_seconds)).await;
            std::process::exit(code);
        }
        Cmd::Serve(args) => {
            let app = build_app(&args).await?;
            if args.http {
                kite_mcp::http::serve(app, args.addr).await?;
            } else {
                let service = KiteMcp { app }.serve(rmcp::transport::stdio()).await?;
                service.waiting().await?;
            }
        }
    }
    Ok(())
}
