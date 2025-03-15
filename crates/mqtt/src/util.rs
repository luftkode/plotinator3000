use std::{
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};

pub fn validate_broker(host: &str, port: &str) -> Result<(), String> {
    // Validate port first
    let port: u16 = port.parse().map_err(|e| format!("Invalid port: {e}"))?;

    // Format host properly for IPv6 if needed
    let formatted_host = if let Ok(ipv6) = host.parse::<std::net::Ipv6Addr>() {
        format!("[{ipv6}]")
    } else {
        host.to_owned()
    };

    // Create proper address string
    let addr_str = format!("{formatted_host}:{port}");

    // Resolve hostname using DNS (including mDNS if supported by system)
    let addrs = addr_str
        .to_socket_addrs()
        .map_err(|e| format!("DNS resolution failed: {e}"))?;

    // Try all resolved addresses with timeout
    let mut last_error = None;
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, Duration::from_secs(2)) {
            Ok(_) => return Ok(()),
            Err(e) => last_error = Some(e),
        }
    }

    Err(last_error.map_or_else(
        || "No addresses found".to_owned(),
        |e| format!("Connection failed: {e}"),
    ))
}
