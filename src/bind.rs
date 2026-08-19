// Parses the `--bind` flag: a comma-separated list of listen addresses.
// Each entry is either a bare IPv4 literal (optionally suffixed `:port`) or
// a bracketed IPv6 literal (`[addr]`, optionally suffixed `:port`). Brackets
// are reserved for IPv6 and are never valid around an IPv4 literal — this
// also disambiguates the optional `:port` suffix from IPv6's own colons.
// An entry without a `:port` suffix uses the caller-supplied default port
// (the `--port` flag).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

pub fn parse_bind_spec(spec: &str, default_port: u16) -> Result<Vec<SocketAddr>, String> {
    let mut addrs = Vec::new();
    for entry in spec.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            return Err(format!("empty entry in --bind spec {:?}", spec));
        }
        addrs.push(parse_entry(entry, default_port)?);
    }
    if addrs.is_empty() {
        return Err("--bind must specify at least one address".to_string());
    }
    Ok(addrs)
}

fn parse_entry(entry: &str, default_port: u16) -> Result<SocketAddr, String> {
    if let Some(rest) = entry.strip_prefix('[') {
        let (addr_str, after) = rest
            .split_once(']')
            .ok_or_else(|| format!("--bind entry {:?}: missing closing ']'", entry))?;
        let ip: Ipv6Addr = addr_str
            .parse()
            .map_err(|e| format!("--bind entry {:?}: invalid IPv6 address: {}", entry, e))?;
        let port = if after.is_empty() {
            default_port
        } else {
            let port_str = after
                .strip_prefix(':')
                .ok_or_else(|| format!("--bind entry {:?}: expected ':port' after ']'", entry))?;
            parse_port(entry, port_str)?
        };
        Ok(SocketAddr::new(IpAddr::V6(ip), port))
    } else if entry.contains('[') || entry.contains(']') {
        Err(format!(
            "--bind entry {:?}: brackets are only valid around an IPv6 address",
            entry
        ))
    } else {
        let (addr_str, port) = match entry.rsplit_once(':') {
            Some((addr_str, port_str)) => (addr_str, parse_port(entry, port_str)?),
            None => (entry, default_port),
        };
        let ip: Ipv4Addr = addr_str
            .parse()
            .map_err(|e| format!("--bind entry {:?}: invalid IPv4 address: {}", entry, e))?;
        Ok(SocketAddr::new(IpAddr::V4(ip), port))
    }
}

fn parse_port(entry: &str, port_str: &str) -> Result<u16, String> {
    port_str
        .parse()
        .map_err(|e| format!("--bind entry {:?}: invalid port: {}", entry, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_spec() {
        let addrs = parse_bind_spec("0.0.0.0,[::]", 1111).unwrap();
        assert_eq!(
            addrs,
            vec![
                SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 1111),
                SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 1111),
            ]
        );
    }

    #[test]
    fn ipv4_bare_uses_default_port() {
        let addrs = parse_bind_spec("127.0.0.1", 9000).unwrap();
        assert_eq!(addrs, vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9000)]);
    }

    #[test]
    fn ipv4_with_port() {
        let addrs = parse_bind_spec("127.0.0.1:9000", 1111).unwrap();
        assert_eq!(addrs, vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9000)]);
    }

    #[test]
    fn ipv6_bracketed_no_port_uses_default() {
        let addrs = parse_bind_spec("[::1]", 9000).unwrap();
        assert_eq!(addrs, vec![SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 9000)]);
    }

    #[test]
    fn ipv6_bracketed_with_port() {
        let addrs = parse_bind_spec("[::1]:9000", 1111).unwrap();
        assert_eq!(addrs, vec![SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 9000)]);
    }

    #[test]
    fn multiple_entries_mixed() {
        let addrs = parse_bind_spec("0.0.0.0:8080,[::1]:9090,10.0.0.1", 1111).unwrap();
        assert_eq!(
            addrs,
            vec![
                SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080),
                SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 9090),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 1111),
            ]
        );
    }

    #[test]
    fn ipv6_without_brackets_is_rejected() {
        // Ambiguous with the addr:port syntax, and disallowed per spec.
        assert!(parse_bind_spec("::1", 1111).is_err());
    }

    #[test]
    fn brackets_around_ipv4_are_rejected() {
        assert!(parse_bind_spec("[127.0.0.1]", 1111).is_err());
    }

    #[test]
    fn missing_closing_bracket_is_rejected() {
        assert!(parse_bind_spec("[::1", 1111).is_err());
    }

    #[test]
    fn empty_entry_is_rejected() {
        assert!(parse_bind_spec("0.0.0.0,,[::]", 1111).is_err());
    }

    #[test]
    fn empty_spec_is_rejected() {
        assert!(parse_bind_spec("", 1111).is_err());
    }

    #[test]
    fn invalid_port_is_rejected() {
        assert!(parse_bind_spec("127.0.0.1:notaport", 1111).is_err());
    }

    #[test]
    fn garbage_ipv4_is_rejected() {
        assert!(parse_bind_spec("not.an.ip.addr", 1111).is_err());
    }
}
