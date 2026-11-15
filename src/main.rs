use rmcp::{ServerHandler, ServiceExt, model::*, transport::stdio};

#[derive(Clone)]
struct Probe;

impl ServerHandler for Probe {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::default()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = Probe.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
