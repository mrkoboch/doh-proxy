use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{error, info};

use crate::{
    cache::DnsCache,
    config::Config,
    error::Result,
    proxy::Proxy,
    resolver::Resolver,
    upstream::UpstreamClient,
};

pub struct Server {
    proxy: Arc<Proxy>,
    config: Config,
}

impl Server {
    pub async fn new(config: Config) -> Result<Self> {
        let upstream = UpstreamClient::new(config.upstreams.clone())?;
        let cache = if config.cache.enabled {
            Some(DnsCache::new(config.cache.capacity))
        } else {
            None
        };
        let resolver = Resolver::new(upstream, cache, None);
        let proxy = Arc::new(Proxy::new(resolver));
        Ok(Self { proxy, config })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let socket = Arc::new(UdpSocket::bind(self.config.listen_addr).await?);
        info!(addr = %self.config.listen_addr, "UDP listener ready");

        let mut buf = vec![0u8; 4096];
        loop {
            let (len, peer) = socket.recv_from(&mut buf).await?;
            let query = buf[..len].to_vec();
            let proxy = Arc::clone(&self.proxy);
            let socket = Arc::clone(&socket);

            tokio::spawn(async move {
                let response = proxy.handle(&query).await;
                if let Err(e) = socket.send_to(&response, peer).await {
                    error!(peer = %peer, error = %e, "failed to send response");
                }
            });
        }
    }
}
