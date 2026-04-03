use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::net::UdpSocket;
use tracing::{error, info};

use crate::{
    cache::DnsCache,
    config::Config,
    error::Result,
    proxy::Proxy,
    resolver::Resolver,
    stats::Stats,
    upstream::UpstreamClient,
};

pub struct Server {
    proxy: Arc<Proxy>,
    config: Config,
}

impl Server {
    pub async fn new(config: Config, stats: Option<Arc<Stats>>) -> Result<Self> {
        let upstream = UpstreamClient::new(config.upstreams.clone())?;
        let cache = if config.cache.enabled {
            Some(DnsCache::new(config.cache.capacity))
        } else {
            None
        };
        let resolver = Resolver::new(upstream, cache, stats);
        let proxy = Arc::new(Proxy::new(resolver));
        Ok(Self { proxy, config })
    }

    /// Run until `stop` is set to true.
    pub async fn run_cancellable(self, stop: Arc<AtomicBool>) -> anyhow::Result<()> {
        let socket = Arc::new(UdpSocket::bind(self.config.listen_addr).await?);
        info!(addr = %self.config.listen_addr, "UDP listener ready");

        let mut buf = vec![0u8; 4096]; // reused across iterations; data is copied into query before each spawn
        let mut tasks = tokio::task::JoinSet::new();
        let mut consecutive_errors: u32 = 0;

        loop {
            if stop.load(Ordering::Relaxed) {
                // Relaxed: no other shared state is guarded by this flag
                info!("stop signal received, shutting down");
                break;
            }
            match tokio::time::timeout(
                Duration::from_millis(100),
                socket.recv_from(&mut buf),
            )
            .await
            {
                Ok(Ok((len, peer))) => {
                    consecutive_errors = 0;
                    let query = buf[..len].to_vec();
                    let proxy = Arc::clone(&self.proxy);
                    let socket = Arc::clone(&socket);
                    tasks.spawn(async move {
                        let response = proxy.handle(&query).await;
                        if let Err(e) = socket.send_to(&response, peer).await {
                            error!(peer = %peer, error = %e, "failed to send response");
                        }
                    });
                }
                Ok(Err(e)) => {
                    consecutive_errors += 1;
                    error!(error = %e, consecutive = consecutive_errors, "recv_from error");
                    if consecutive_errors >= 5 {
                        error!("too many consecutive recv_from errors, stopping server");
                        break;
                    }
                }
                Err(_) => {} // timeout — loop back and check stop flag
            }
            // Reap any completed tasks to avoid unbounded JoinSet growth
            while tasks.try_join_next().is_some() {}
        }

        // Drain in-flight tasks before returning
        while let Some(res) = tasks.join_next().await {
            if let Err(e) = res {
                error!(error = ?e, "task panicked during shutdown drain");
            }
        }
        Ok(())
    }

    /// Convenience: run forever (used by the CLI binary).
    pub async fn run(self) -> anyhow::Result<()> {
        let stop = Arc::new(AtomicBool::new(false));
        self.run_cancellable(stop).await
    }
}
