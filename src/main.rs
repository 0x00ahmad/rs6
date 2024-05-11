use std::{
    net::{Ipv6Addr, SocketAddr},
    str::FromStr,
};

use cidr::IpCidr;

pub mod error;
mod proxy;

type Result<T, E = error::Error> = std::result::Result<T, E>;

fn main() {
    let bind = SocketAddr::from(([0, 0, 0, 0], 8100));
    let concurrent = 1024;

    println!("Starting proxy server on {}", bind);

    let base_address = Ipv6Addr::from_str("2a12:6b80::");
    match base_address {
        Ok(addr) => {
            let subnet_mask = 29;
            let cidrr = IpCidr::new(std::net::IpAddr::V6(addr), subnet_mask).ok();
            let _ = proxy::run(bind, concurrent, cidrr);
        }
        Err(e) => {
            eprintln!("Invalid base address: {}", e);
            return;
        }
    }
}
