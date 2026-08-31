// Copyright (c) 2026 term4u contributors. Licensed under the MIT license.

use std::net::{IpAddr, SocketAddr};

#[derive(Debug, thiserror::Error)]
#[error("offline build refused an outbound connection to {0}")]
pub struct OutboundRefused(pub String);

pub fn check_peer(addr: &SocketAddr) -> Result<(), OutboundRefused> {
    if addr.ip().is_loopback() {
        Ok(())
    } else {
        Err(OutboundRefused(addr.to_string()))
    }
}

pub fn loopback_addrs(host: &str, port: u16) -> Result<Vec<SocketAddr>, OutboundRefused> {
    let host = host.trim_start_matches('[').trim_end_matches(']');
    if host.eq_ignore_ascii_case("localhost") {
        return Ok(vec![
            SocketAddr::new(IpAddr::V6(std::net::Ipv6Addr::LOCALHOST), port),
            SocketAddr::new(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port),
        ]);
    }

    let ip = host
        .parse::<IpAddr>()
        .map_err(|_| OutboundRefused(host.to_string()))?;
    let addr = SocketAddr::new(ip, port);
    check_peer(&addr)?;
    Ok(vec![addr])
}

#[cfg(all(test, unix))]
#[path = "lib_tests.rs"]
mod tests;
