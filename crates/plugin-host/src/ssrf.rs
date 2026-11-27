// SPDX-License-Identifier: AGPL-3.0-or-later

//! SSRF protection: addresses the shared client never connects to
//! (docs/architecture/security.md#threats-and-mitigations).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

fn v4_is_public(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_private()
        // Link-local, including the cloud metadata address 169.254.169.254.
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_multicast()
        || ip.is_documentation()
        // 0.0.0.0/8, shared address space 100.64.0.0/10, IETF protocol assignments 192.0.0.0/24,
        // benchmarking 198.18.0.0/15, reserved 240.0.0.0/4.
        || a == 0
        || (a == 100 && (64..128).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 198 && (b == 18 || b == 19))
        || a >= 240)
}

fn v6_is_public(ip: Ipv6Addr) -> bool {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return v4_is_public(v4);
    }
    let first = ip.segments()[0];
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_multicast()
        // Unique local fc00::/7, link-local fe80::/10, documentation 2001:db8::/32,
        // NAT64 64:ff9b::/96 (maps to IPv4 that is checked separately when resolved).
        || (first & 0xfe00) == 0xfc00
        || (first & 0xffc0) == 0xfe80
        || (first == 0x2001 && ip.segments()[1] == 0x0db8)
        || first == 0x0064)
}

/// Whether the client may connect to `ip`.
pub(crate) fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4_is_public(v4),
        IpAddr::V6(v6) => v6_is_public(v6),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_loopback_link_local_and_metadata_addresses_are_refused() {
        // security.md: private, loopback, link-local, and metadata addresses.
        let refused = [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "0.0.0.0",
            "100.64.0.1",
            "224.0.0.1",
            "255.255.255.255",
            "192.0.2.1",
            "::1",
            "::",
            "fc00::1",
            "fd12:3456::1",
            "fe80::1",
            "::ffff:127.0.0.1",
            "::ffff:169.254.169.254",
            "2001:db8::1",
        ];
        for ip in refused {
            assert!(!is_public(ip.parse().unwrap()), "{ip} must be refused");
        }
        for ip in ["104.20.23.46", "8.8.8.8", "2606:4700::6810:172e"] {
            assert!(is_public(ip.parse().unwrap()), "{ip} must be allowed");
        }
    }
}
