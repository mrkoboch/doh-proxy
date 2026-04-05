mod cli;

use clap::{Parser, Subcommand};

/// A DNS-over-HTTPS proxy with a polished terminal interface.
#[derive(Parser)]
#[command(
    name = "doh-proxy",
    version,
    about,
    long_about = "doh-proxy forwards DNS queries to upstream DoH resolvers.\n\
                  Config is stored at ~/.config/doh-proxy/config.toml.\n\
                  Run `doh-proxy config` to set up interactively."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the DoH proxy (runs in the foreground; Ctrl+C or `stop` to quit)
    Start {
        /// Override the listen address (e.g. 0.0.0.0:53)
        #[arg(short, long, value_name = "ADDR")]
        listen: Option<String>,

        /// Override upstream DoH resolvers (can be repeated)
        #[arg(short, long = "upstream", value_name = "URL")]
        upstreams: Vec<String>,
    },

    /// Interactive configuration setup
    Config,

    /// Show proxy status and statistics
    Status,

    /// Tail live proxy logs (Ctrl+C to quit)
    Logs {
        /// Number of historical lines to show on start
        #[arg(short = 'n', long, default_value = "20")]
        lines: usize,

        /// Stop after showing history (do not follow new lines)
        #[arg(long = "no-follow")]
        no_follow: bool,
    },

    /// Gracefully stop a running proxy
    Stop,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Start { listen, upstreams } => cli::start::run(listen, upstreams).await,
        Commands::Config => cli::config_cmd::run(),
        Commands::Status => cli::status::run(),
        Commands::Logs { lines, no_follow } => cli::logs::run(!no_follow, lines),
        Commands::Stop => cli::stop::run(),
    }
}
