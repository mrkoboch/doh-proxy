use clap::Parser;
use doh_proxy::{config::Config, server::Server};
use tracing::info;

#[derive(Parser)]
#[command(name = "doh-proxy", about = "A DNS-over-HTTPS proxy server")]
struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "config/default.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let config = Config::from_file(&cli.config)?;

    info!(listen = %config.listen_addr, "starting DoH proxy");

    let server = Server::new(config).await?;
    server.run().await
}
