mod connect;
mod http;

use cidr::IpCidr;
use std::net::{IpAddr, SocketAddr};

struct ProxyContext {
    /// Bind address
    pub bind: SocketAddr,
    /// Number of concurrent connections
    pub concurrent: usize,
    /// Connector
    pub connector: connect::Connector,
}

///
/// Idk man, just read the kode
#[tokio::main(flavor = "multi_thread")]
pub async fn run(bind: SocketAddr, concurrent: usize, cidr: Option<IpCidr>) -> crate::Result<()> {
    std::env::set_var("RUST_LOG", "debug");

    let fallback = Option::<IpAddr>::None;

    http::proxy(ProxyContext {
        bind,
        concurrent,
        connector: connect::Connector::new(cidr, fallback),
    })
    .await
}
