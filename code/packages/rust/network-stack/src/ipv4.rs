// ============================================================================
// Layer 3: IPv4 — Internet Protocol Version 4
// ============================================================================
//
// IP is the routing layer. While Ethernet delivers on a single local network
// (one street), IP delivers across the entire Internet (between cities).
//
// Every device has an IP address (e.g., 10.0.0.1). The IP layer adds a
// header with source and destination addresses, computes a checksum for
// error detection, then hands the packet to Ethernet for local delivery.
//
// IPv4 Header (20 bytes, no options):
//
//    0                   1                   2                   3
//    0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//   |Version|  IHL  |    TOS        |          Total Length         |
//   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//   |         Identification        |Flags|      Fragment Offset   |
//   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//   |  Time to Live |    Protocol   |         Header Checksum      |
//   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//   |                       Source Address                         |
//   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//   |                    Destination Address                       |
//   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//
// ============================================================================

use crate::ethernet::Ipv4Address;

/// Protocol number for TCP (Layer 4).
pub const PROTOCOL_TCP: u8 = 6;

/// Protocol number for UDP (Layer 4).
pub const PROTOCOL_UDP: u8 = 17;

// ============================================================================
// IPv4Header
// ============================================================================
//
// The 20-byte header that prefixes every IP packet. Key fields:
//   - version (always 4)
//   - ihl (always 5 = 20 bytes, no options)
//   - total_length (header + payload)
//   - ttl (time to live — prevents infinite routing loops)
//   - protocol (6=TCP, 17=UDP)
//   - header_checksum (ones' complement error detection)
//   - src_ip, dst_ip (4 bytes each)
//
// ============================================================================
#[derive(Debug, Clone, PartialEq)]
pub struct Ipv4Header {
    pub version: u8,
    pub ihl: u8,
    pub total_length: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub header_checksum: u16,
    pub src_ip: Ipv4Address,
    pub dst_ip: Ipv4Address,
}

impl Ipv4Header {
    /// Create a new IPv4 header with sensible defaults.
    pub fn new(src_ip: Ipv4Address, dst_ip: Ipv4Address, protocol: u8, total_length: u16) -> Self {
        Self {
            version: 4,
            ihl: 5,
            total_length,
            ttl: 64,
            protocol,
            header_checksum: 0,
            src_ip,
            dst_ip,
        }
    }

    /// Serialize the header into a 20-byte array (network byte order).
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; 20];

        // Byte 0: version (4 bits) | IHL (4 bits)
        bytes[0] = ((self.version & 0x0F) << 4) | (self.ihl & 0x0F);
        // Byte 1: TOS (0)
        bytes[1] = 0;
        // Bytes 2-3: Total Length
        bytes[2] = (self.total_length >> 8) as u8;
        bytes[3] = (self.total_length & 0xFF) as u8;
        // Bytes 4-7: Identification, Flags, Fragment Offset (all 0)
        // Byte 8: TTL
        bytes[8] = self.ttl;
        // Byte 9: Protocol
        bytes[9] = self.protocol;
        // Bytes 10-11: Header Checksum
        bytes[10] = (self.header_checksum >> 8) as u8;
        bytes[11] = (self.header_checksum & 0xFF) as u8;
        // Bytes 12-15: Source IP
        bytes[12..16].copy_from_slice(&self.src_ip);
        // Bytes 16-19: Destination IP
        bytes[16..20].copy_from_slice(&self.dst_ip);

        bytes
    }

    /// Deserialize a 20-byte slice into an IPv4Header.
    pub fn deserialize(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 20 {
            return None;
        }

        Some(Self {
            version: (bytes[0] >> 4) & 0x0F,
            ihl: bytes[0] & 0x0F,
            total_length: ((bytes[2] as u16) << 8) | (bytes[3] as u16),
            ttl: bytes[8],
            protocol: bytes[9],
            header_checksum: ((bytes[10] as u16) << 8) | (bytes[11] as u16),
            src_ip: [bytes[12], bytes[13], bytes[14], bytes[15]],
            dst_ip: [bytes[16], bytes[17], bytes[18], bytes[19]],
        })
    }

    /// Compute the IP header checksum using the ones' complement algorithm.
    ///
    /// Algorithm (from RFC 791):
    ///   1. Set checksum field to 0.
    ///   2. Treat header as 16-bit words, sum them all.
    ///   3. Fold any carry bits back into the 16-bit sum.
    ///   4. Take the ones' complement (bitwise NOT).
    ///
    /// This catches single-bit errors and most multi-bit errors.
    pub fn compute_checksum(&self) -> u16 {
        let mut copy = self.clone();
        copy.header_checksum = 0;
        let bytes = copy.serialize();

        let mut sum: u32 = 0;
        for i in (0..bytes.len()).step_by(2) {
            let word = ((bytes[i] as u32) << 8) | (bytes.get(i + 1).copied().unwrap_or(0) as u32);
            sum += word;
        }

        // Fold carry
        while sum > 0xFFFF {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        (!sum as u16) & 0xFFFF
    }

    /// Verify the checksum of a received header.
    ///
    /// Sum all 16-bit words including the checksum field. A valid header
    /// produces 0xFFFF (all ones) because the checksum was chosen to make
    /// the total sum come out to all ones.
    pub fn verify_checksum(&self) -> bool {
        let bytes = self.serialize();
        let mut sum: u32 = 0;
        for i in (0..bytes.len()).step_by(2) {
            let word = ((bytes[i] as u32) << 8) | (bytes.get(i + 1).copied().unwrap_or(0) as u32);
            sum += word;
        }
        while sum > 0xFFFF {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        sum == 0xFFFF
    }
}

// ============================================================================
// RoutingTable — Longest-Prefix Match Router
// ============================================================================
//
// Each entry says: "If (destination_ip & mask) == network, send the packet
// to `gateway` via `interface`."
//
// When multiple entries match, we pick the one with the longest (most
// specific) mask. Example:
//   10.0.0.0/8  -> gateway A   (any 10.x.x.x)
//   10.0.1.0/24 -> gateway B   (only 10.0.1.x)
//   Destination 10.0.1.5 matches both, but /24 wins because it's more specific.
//
// ============================================================================
#[derive(Debug, Clone)]
pub struct RouteEntry {
    pub network: Ipv4Address,
    pub mask: Ipv4Address,
    pub gateway: Ipv4Address,
    pub interface_name: String,
}

#[derive(Debug, Clone)]
pub struct RoutingTable {
    routes: Vec<RouteEntry>,
}

impl RoutingTable {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Add a route to the table.
    pub fn add_route(&mut self, network: Ipv4Address, mask: Ipv4Address, gateway: Ipv4Address, interface_name: &str) {
        self.routes.push(RouteEntry {
            network,
            mask,
            gateway,
            interface_name: interface_name.to_string(),
        });
    }

    /// Look up the best route for a destination IP using longest-prefix match.
    pub fn lookup(&self, dst_ip: &Ipv4Address) -> Option<&RouteEntry> {
        let mut best: Option<&RouteEntry> = None;
        let mut best_mask_bits: i32 = -1;

        for route in &self.routes {
            // Check if (dst_ip & mask) == network
            let matches = (0..4).all(|i| (dst_ip[i] & route.mask[i]) == route.network[i]);

            if matches {
                // Count mask bits (more bits = more specific route)
                let mask_bits: i32 = route.mask.iter()
                    .map(|b| b.count_ones() as i32)
                    .sum();

                if mask_bits > best_mask_bits {
                    best_mask_bits = mask_bits;
                    best = Some(route);
                }
            }
        }

        best
    }

    pub fn len(&self) -> usize {
        self.routes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }
}

impl Default for RoutingTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// IpLayer — Send and Receive IP Packets
// ============================================================================
//
// Ties together IPv4Header, RoutingTable, and ArpTable. Creates outbound
// packets with proper checksums and parses inbound packets.
//
// ============================================================================
#[derive(Debug, Clone)]
pub struct IpLayer {
    pub local_ip: Ipv4Address,
    pub routing_table: RoutingTable,
}

impl IpLayer {
    pub fn new(local_ip: Ipv4Address) -> Self {
        Self {
            local_ip,
            routing_table: RoutingTable::new(),
        }
    }

    /// Create an IP packet (header + payload) ready for transmission.
    pub fn create_packet(&self, dst_ip: Ipv4Address, protocol: u8, payload: &[u8]) -> Vec<u8> {
        let total_length = 20 + payload.len() as u16;
        let mut header = Ipv4Header::new(self.local_ip, dst_ip, protocol, total_length);
        header.header_checksum = header.compute_checksum();

        let mut packet = header.serialize();
        packet.extend_from_slice(payload);
        packet
    }

    /// Parse a received IP packet. Returns (src_ip, protocol, payload) or None.
    pub fn parse_packet(&self, bytes: &[u8]) -> Option<(Ipv4Address, u8, Vec<u8>)> {
        if bytes.len() < 20 {
            return None;
        }

        let header = Ipv4Header::deserialize(bytes)?;
        if !header.verify_checksum() {
            return None;
        }

        let payload = bytes[20..].to_vec();
        Some((header.src_ip, header.protocol, payload))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_serialize_deserialize_round_trip() {
        let mut header = Ipv4Header::new([10, 0, 0, 1], [10, 0, 0, 2], PROTOCOL_TCP, 60);
        header.header_checksum = header.compute_checksum();

        let bytes = header.serialize();
        let restored = Ipv4Header::deserialize(&bytes).unwrap();

        assert_eq!(restored.version, 4);
        assert_eq!(restored.ihl, 5);
        assert_eq!(restored.total_length, 60);
        assert_eq!(restored.ttl, 64);
        assert_eq!(restored.protocol, PROTOCOL_TCP);
        assert_eq!(restored.src_ip, [10, 0, 0, 1]);
        assert_eq!(restored.dst_ip, [10, 0, 0, 2]);
    }

    #[test]
    fn test_header_is_20_bytes() {
        let header = Ipv4Header::new([0; 4], [0; 4], PROTOCOL_UDP, 20);
        assert_eq!(header.serialize().len(), 20);
    }

    #[test]
    fn test_deserialize_too_short() {
        assert!(Ipv4Header::deserialize(&[0u8; 19]).is_none());
        assert!(Ipv4Header::deserialize(&[]).is_none());
    }

    #[test]
    fn test_checksum_nonzero() {
        let header = Ipv4Header::new([10, 0, 0, 1], [10, 0, 0, 2], PROTOCOL_TCP, 40);
        assert_ne!(header.compute_checksum(), 0);
    }

    #[test]
    fn test_verify_checksum_succeeds() {
        let mut header = Ipv4Header::new([10, 0, 0, 1], [10, 0, 0, 2], PROTOCOL_TCP, 40);
        header.header_checksum = header.compute_checksum();
        assert!(header.verify_checksum());
    }

    #[test]
    fn test_verify_checksum_fails_on_corruption() {
        let mut header = Ipv4Header::new([10, 0, 0, 1], [10, 0, 0, 2], PROTOCOL_TCP, 40);
        header.header_checksum = header.compute_checksum();
        header.ttl = 32; // corrupt
        assert!(!header.verify_checksum());
    }

    #[test]
    fn test_routing_table_single_route() {
        let mut table = RoutingTable::new();
        table.add_route([10, 0, 0, 0], [255, 255, 255, 0], [0, 0, 0, 0], "eth0");

        let route = table.lookup(&[10, 0, 0, 5]).unwrap();
        assert_eq!(route.interface_name, "eth0");
    }

    #[test]
    fn test_routing_table_longest_prefix_match() {
        let mut table = RoutingTable::new();
        table.add_route([10, 0, 0, 0], [255, 0, 0, 0], [10, 0, 0, 1], "eth0");
        table.add_route([10, 0, 1, 0], [255, 255, 255, 0], [10, 0, 1, 1], "eth1");

        // 10.0.1.5 matches both, but /24 is more specific
        let route = table.lookup(&[10, 0, 1, 5]).unwrap();
        assert_eq!(route.interface_name, "eth1");
        assert_eq!(route.gateway, [10, 0, 1, 1]);

        // 10.0.2.5 only matches /8
        let route = table.lookup(&[10, 0, 2, 5]).unwrap();
        assert_eq!(route.interface_name, "eth0");
    }

    #[test]
    fn test_routing_table_no_match() {
        let mut table = RoutingTable::new();
        table.add_route([10, 0, 0, 0], [255, 255, 255, 0], [0; 4], "eth0");
        assert!(table.lookup(&[192, 168, 1, 1]).is_none());
    }

    #[test]
    fn test_routing_table_default_route() {
        let mut table = RoutingTable::new();
        table.add_route([0, 0, 0, 0], [0, 0, 0, 0], [10, 0, 0, 1], "eth0");
        let route = table.lookup(&[8, 8, 8, 8]).unwrap();
        assert_eq!(route.gateway, [10, 0, 0, 1]);
    }

    #[test]
    fn test_routing_table_len() {
        let mut table = RoutingTable::default();
        assert!(table.is_empty());
        table.add_route([10, 0, 0, 0], [255, 255, 255, 0], [0; 4], "eth0");
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_ip_layer_create_packet() {
        let layer = IpLayer::new([10, 0, 0, 1]);
        let payload = vec![0x01, 0x02, 0x03, 0x04];
        let packet = layer.create_packet([10, 0, 0, 2], PROTOCOL_TCP, &payload);
        assert_eq!(packet.len(), 24); // 20 header + 4 payload
    }

    #[test]
    fn test_ip_layer_parse_round_trip() {
        let layer = IpLayer::new([10, 0, 0, 1]);
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let packet = layer.create_packet([10, 0, 0, 2], PROTOCOL_UDP, &payload);

        let (src_ip, protocol, parsed_payload) = layer.parse_packet(&packet).unwrap();
        assert_eq!(src_ip, [10, 0, 0, 1]);
        assert_eq!(protocol, PROTOCOL_UDP);
        assert_eq!(parsed_payload, payload);
    }

    #[test]
    fn test_ip_layer_parse_too_short() {
        let layer = IpLayer::new([10, 0, 0, 1]);
        assert!(layer.parse_packet(&[0u8; 10]).is_none());
    }

    #[test]
    fn test_ip_layer_parse_bad_checksum() {
        let layer = IpLayer::new([10, 0, 0, 1]);
        let mut packet = layer.create_packet([10, 0, 0, 2], PROTOCOL_TCP, &[1, 2, 3]);
        packet[8] = packet[8].wrapping_add(1); // corrupt TTL
        assert!(layer.parse_packet(&packet).is_none());
    }
}
