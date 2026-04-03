use bytes::Bytes;
use hickory_proto::{
    op::{Message, MessageType, ResponseCode},
    serialize::binary::{BinDecodable, BinEncodable},
};
use tracing::{debug, warn};

use crate::{error::Result, resolver::Resolver};

/// Processes a raw DNS query, delegates to the resolver, and returns a raw DNS response.
/// On resolver error a SERVFAIL response is constructed instead of propagating.
pub struct Proxy {
    resolver: Resolver,
}

impl Proxy {
    pub fn new(resolver: Resolver) -> Self {
        Self { resolver }
    }

    pub async fn handle(&self, raw_query: &[u8]) -> Bytes {
        match self.resolve(raw_query).await {
            Ok(response) => response,
            Err(e) => {
                warn!(error = %e, "resolver error, returning SERVFAIL");
                servfail(raw_query)
            }
        }
    }

    async fn resolve(&self, raw_query: &[u8]) -> Result<Bytes> {
        debug!("resolving query ({} bytes)", raw_query.len());
        self.resolver.resolve(raw_query).await
    }
}

/// Build a minimal SERVFAIL response that echoes back the query ID.
fn servfail(raw_query: &[u8]) -> Bytes {
    if let Ok(query) = Message::from_bytes(raw_query) {
        let mut response = Message::new();
        response.set_id(query.id());
        response.set_message_type(MessageType::Response);
        response.set_response_code(ResponseCode::ServFail);
        if let Ok(encoded) = response.to_bytes() {
            return Bytes::from(encoded);
        }
    }
    Bytes::new()
}
