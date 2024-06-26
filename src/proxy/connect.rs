use cidr::IpCidr;
use hyper_util::client::legacy::connect::HttpConnector;
use rand::Rng;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use tokio::net::{TcpSocket, TcpStream};

/// `Connector` struct is used to create HTTP connectors, optionally configured
/// with an IPv6 CIDR and a fallback IP address.
#[derive(Clone)]
pub struct Connector {
    /// Optional IPv6 CIDR (Classless Inter-Domain Routing), used to optionally
    /// configure an IPv6 address.
    cidr: Option<IpCidr>,
    /// Optional IP address as a fallback option in case of connection failure.
    fallback: Option<IpAddr>,
}

impl Connector {
    /// Constructs a new `Connector` instance, accepting optional IPv6 CIDR and
    /// fallback IP address as parameters.
    pub(super) fn new(cidr: Option<IpCidr>, fallback: Option<IpAddr>) -> Self {
        Connector { cidr, fallback }
    }

    /// Generates a new `HttpConnector` based on the configuration. This method
    /// configures the connector considering the IPv6 CIDR and fallback IP
    /// address.
    pub fn new_http_connector(&self) -> HttpConnector {
        let mut connector = HttpConnector::new();

        match (self.cidr, self.fallback) {
            (Some(IpCidr::V6(cidr)), Some(IpAddr::V4(v4))) => {
                let v6 = get_rand_ipv6(cidr.first_address().into(), cidr.network_length());
                connector.set_local_addresses(v4, v6);
            }
            (Some(IpCidr::V4(cidr)), Some(IpAddr::V6(v6))) => {
                let v4 = get_rand_ipv4(cidr.first_address().into(), cidr.network_length());
                connector.set_local_addresses(v4, v6);
            }
            (Some(IpCidr::V4(cidr)), None) => {
                let v6 = get_rand_ipv4(cidr.first_address().into(), cidr.network_length());
                connector.set_local_address(Some(v6.into()));
            }
            (Some(IpCidr::V6(v6)), None) => {
                let v6 = get_rand_ipv6(v6.first_address().into(), v6.network_length());
                connector.set_local_address(Some(v6.into()));
            }
            // ipv4 or ipv6
            (None, Some(ip)) => connector.set_local_address(Some(ip)),
            _ => {}
        }

        connector
    }

    /// Attempts to establish a connection to a given SocketAddr.
    /// If an IPv6 subnet and a fallback IP are provided, it will attempt to
    /// connect using them. If no IPv6 subnet is provided but a fallback IP
    /// is, it will attempt to connect using the fallback IP. If neither are
    /// provided, it will attempt to connect directly to the given SocketAddr.
    pub async fn try_connect(&self, addr: SocketAddr) -> std::io::Result<TcpStream> {
        match (self.cidr, self.fallback) {
            (Some(ipv6_cidr), ip_addr) => {
                try_connect_with_ipv6_and_fallback(addr, ipv6_cidr, ip_addr).await
            }
            (None, Some(ip)) => try_connect_with_fallback(addr, ip).await,
            _ => TcpStream::connect(addr).await,
        }
        .and_then(|stream| {
            tracing::info!("{} via {}", addr, stream.local_addr()?);
            Ok(stream)
        })
    }
}

/// Try to connect with ipv6 and fallback to ipv4/ipv6
async fn try_connect_with_ipv6_and_fallback(
    target_addr: SocketAddr,
    cidr: IpCidr,
    fallback: Option<IpAddr>,
) -> std::io::Result<TcpStream> {
    let (bind, socket) = match cidr {
        IpCidr::V4(cidr) => {
            let socket = TcpSocket::new_v4()?;
            (
                IpAddr::V4(get_rand_ipv4(
                    cidr.first_address().into(),
                    cidr.network_length(),
                )),
                socket,
            )
        }
        IpCidr::V6(cidr) => {
            let socket = TcpSocket::new_v6()?;
            (
                IpAddr::V6(get_rand_ipv6(
                    cidr.first_address().into(),
                    cidr.network_length(),
                )),
                socket,
            )
        }
    };
    socket.bind(SocketAddr::new(bind, 0))?;

    // Try to connect with ipv6
    match socket.connect(target_addr).await {
        Ok(first) => Ok(first),
        Err(err) => {
            tracing::debug!("try connect with ipv6 failed: {}", err);
            if let Some(ip) = fallback {
                // Try to connect with fallback ip (ipv4 or ipv6)
                let socket = create_socket_for_ip(ip)?;
                let bind_addr = SocketAddr::new(ip, 0);
                socket.bind(bind_addr)?;
                socket.connect(target_addr).await
            } else {
                // Try to connect with system default ip
                TcpStream::connect(target_addr).await
            }
        }
    }
}

/// Try to connect with fallback to ipv4/ipv6
async fn try_connect_with_fallback(
    target_addr: SocketAddr,
    ip: IpAddr,
) -> std::io::Result<TcpStream> {
    let socket = create_socket_for_ip(ip)?;
    let bind_addr = SocketAddr::new(ip, 0);
    socket.bind(bind_addr)?;
    socket.connect(target_addr).await
}

/// Create a socket for ip
fn create_socket_for_ip(ip: IpAddr) -> std::io::Result<TcpSocket> {
    match ip {
        IpAddr::V4(_) => TcpSocket::new_v4(),
        IpAddr::V6(_) => TcpSocket::new_v6(),
    }
}

/// Get a random ipv4 address
fn get_rand_ipv4(mut ipv4: u32, prefix_len: u8) -> Ipv4Addr {
    let rand: u32 = rand::thread_rng().gen();
    let net_part = (ipv4 >> (32 - prefix_len)) << (32 - prefix_len);
    let host_part = (rand << prefix_len) >> prefix_len;
    ipv4 = net_part | host_part;
    ipv4.into()
}

/// Get a random ipv6 address
fn get_rand_ipv6(mut ipv6: u128, prefix_len: u8) -> Ipv6Addr {
    let rand: u128 = rand::thread_rng().gen();
    let net_part = (ipv6 >> (128 - prefix_len)) << (128 - prefix_len);
    let host_part = (rand << prefix_len) >> prefix_len;
    ipv6 = net_part | host_part;
    ipv6.into()
}
