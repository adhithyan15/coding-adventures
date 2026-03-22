// ============================================================================
// DNS — Domain Name System (Simplified Static Resolver)
// ============================================================================
//
// DNS is the Internet's phone book. It translates human-readable hostnames
// ("example.com") into IP addresses (93.184.216.34).
//
// Real DNS resolution involves querying root servers, TLD servers, and
// authoritative servers. We simplify this to a static lookup table —
// a hash map from hostname to IP address.
//
// Default entries:
//   "localhost" -> [127, 0, 0, 1]
//
// ============================================================================

use std::collections::HashMap;
use crate::ethernet::Ipv4Address;

pub struct DnsResolver {
    static_table: HashMap<String, Ipv4Address>,
}

impl DnsResolver {
    pub fn new() -> Self {
        let mut table = HashMap::new();
        table.insert("localhost".to_string(), [127, 0, 0, 1]);
        Self { static_table: table }
    }

    /// Resolve a hostname to an IP address. Returns None if unknown.
    pub fn resolve(&self, hostname: &str) -> Option<Ipv4Address> {
        self.static_table.get(hostname).copied()
    }

    /// Add a static hostname-to-IP mapping (like /etc/hosts).
    pub fn add_static(&mut self, hostname: &str, ip: Ipv4Address) {
        self.static_table.insert(hostname.to_string(), ip);
    }

    /// Number of entries in the static table.
    pub fn len(&self) -> usize {
        self.static_table.len()
    }

    pub fn is_empty(&self) -> bool {
        self.static_table.is_empty()
    }
}

impl Default for DnsResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_localhost() {
        let resolver = DnsResolver::new();
        assert_eq!(resolver.resolve("localhost"), Some([127, 0, 0, 1]));
    }

    #[test]
    fn test_resolve_unknown() {
        let resolver = DnsResolver::new();
        assert_eq!(resolver.resolve("unknown.example.com"), None);
    }

    #[test]
    fn test_add_static_and_resolve() {
        let mut resolver = DnsResolver::new();
        resolver.add_static("example.com", [93, 184, 216, 34]);
        assert_eq!(resolver.resolve("example.com"), Some([93, 184, 216, 34]));
    }

    #[test]
    fn test_multiple_entries() {
        let mut resolver = DnsResolver::new();
        resolver.add_static("a.com", [1, 2, 3, 4]);
        resolver.add_static("b.com", [5, 6, 7, 8]);
        assert_eq!(resolver.resolve("a.com"), Some([1, 2, 3, 4]));
        assert_eq!(resolver.resolve("b.com"), Some([5, 6, 7, 8]));
    }

    #[test]
    fn test_overwrite_entry() {
        let mut resolver = DnsResolver::new();
        resolver.add_static("example.com", [1, 1, 1, 1]);
        resolver.add_static("example.com", [8, 8, 8, 8]);
        assert_eq!(resolver.resolve("example.com"), Some([8, 8, 8, 8]));
    }

    #[test]
    fn test_len() {
        let resolver = DnsResolver::new();
        assert_eq!(resolver.len(), 1); // localhost
        assert!(!resolver.is_empty());
    }

    #[test]
    fn test_default() {
        let resolver = DnsResolver::default();
        assert_eq!(resolver.resolve("localhost"), Some([127, 0, 0, 1]));
    }
}
