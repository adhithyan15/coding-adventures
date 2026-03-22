// ============================================================================
// Layer 2: Ethernet — Local Network Delivery
// ============================================================================
//
// Ethernet is the foundation of local area networking. Every device on a
// local network has a unique 48-bit MAC (Media Access Control) address
// burned into its network interface card at the factory.
//
// When you send data on a local network, the Ethernet layer wraps it in
// a "frame" — a header (destination MAC, source MAC, type) plus the payload.
// The frame is transmitted as raw bytes on the wire.
//
// Analogy: Ethernet is like the local mail carrier — it delivers between
// neighboring houses on the same street. To reach another street (network),
// you need a router (that's IP's job at Layer 3).
//
// Frame structure:
//
//   +-----------+-----------+------------+---------+
//   | dest_mac  | src_mac   | ether_type | payload |
//   | (6 bytes) | (6 bytes) | (2 bytes)  | (var)   |
//   +-----------+-----------+------------+---------+
//
// Common ether_type values:
//   0x0800 = IPv4 — payload is an IP packet
//   0x0806 = ARP  — payload is an ARP message
//
// ============================================================================

use std::collections::HashMap;

/// EtherType for IPv4 packets.
pub const ETHER_TYPE_IPV4: u16 = 0x0800;

/// EtherType for ARP messages.
pub const ETHER_TYPE_ARP: u16 = 0x0806;

/// Broadcast MAC address — sent to all devices on the local network.
pub const BROADCAST_MAC: [u8; 6] = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];

/// A MAC address is 6 bytes (48 bits), uniquely identifying a network card.
pub type MacAddress = [u8; 6];

/// An IPv4 address is 4 bytes (32 bits), e.g. [10, 0, 0, 1] = "10.0.0.1".
pub type Ipv4Address = [u8; 4];

// ============================================================================
// EthernetFrame
// ============================================================================
//
// The fundamental unit of data on a local network. Each frame carries:
//   - dest_mac:   who should receive this frame (6 bytes)
//   - src_mac:    who sent this frame (6 bytes)
//   - ether_type: what the payload contains (2 bytes)
//   - payload:    the actual data (variable length)
//
// ============================================================================
#[derive(Debug, Clone, PartialEq)]
pub struct EthernetFrame {
    pub dest_mac: MacAddress,
    pub src_mac: MacAddress,
    pub ether_type: u16,
    pub payload: Vec<u8>,
}

impl EthernetFrame {
    /// Create a new Ethernet frame.
    pub fn new(dest_mac: MacAddress, src_mac: MacAddress, ether_type: u16, payload: Vec<u8>) -> Self {
        Self { dest_mac, src_mac, ether_type, payload }
    }

    /// Serialize the frame into bytes for transmission on the wire.
    ///
    /// Layout:
    ///   bytes[0..6]   = destination MAC
    ///   bytes[6..12]  = source MAC
    ///   bytes[12..14] = ether_type (big-endian)
    ///   bytes[14..]   = payload
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(14 + self.payload.len());
        bytes.extend_from_slice(&self.dest_mac);
        bytes.extend_from_slice(&self.src_mac);
        bytes.push((self.ether_type >> 8) as u8);
        bytes.push((self.ether_type & 0xFF) as u8);
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    /// Deserialize bytes from the wire back into an EthernetFrame.
    ///
    /// Returns None if the input is too short (minimum 14 bytes for the header).
    pub fn deserialize(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 14 {
            return None;
        }

        let mut dest_mac = [0u8; 6];
        let mut src_mac = [0u8; 6];
        dest_mac.copy_from_slice(&bytes[0..6]);
        src_mac.copy_from_slice(&bytes[6..12]);
        let ether_type = ((bytes[12] as u16) << 8) | (bytes[13] as u16);
        let payload = bytes[14..].to_vec();

        Some(Self { dest_mac, src_mac, ether_type, payload })
    }
}

// ============================================================================
// ARPTable — IP-to-MAC Address Resolution Cache
// ============================================================================
//
// ARP (Address Resolution Protocol) bridges the gap between Layer 3 (IP)
// and Layer 2 (Ethernet). When you want to send an IP packet to 10.0.0.2,
// you need the MAC address of 10.0.0.2's NIC.
//
// The ARP table caches recently learned IP-to-MAC mappings. In a real
// system, entries expire after ~20 minutes.
//
// ============================================================================
#[derive(Debug, Clone)]
pub struct ArpTable {
    entries: HashMap<Ipv4Address, MacAddress>,
}

impl ArpTable {
    pub fn new() -> Self {
        Self { entries: HashMap::new() }
    }

    /// Look up the MAC address for a given IP. Returns None if unknown.
    pub fn lookup(&self, ip: &Ipv4Address) -> Option<MacAddress> {
        self.entries.get(ip).copied()
    }

    /// Insert or update an IP-to-MAC mapping.
    pub fn insert(&mut self, ip: Ipv4Address, mac: MacAddress) {
        self.entries.insert(ip, mac);
    }

    /// Number of entries in the table.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is the table empty?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for ArpTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_serialize_deserialize_round_trip() {
        let frame = EthernetFrame::new(
            [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
            [0x11, 0x22, 0x33, 0x44, 0x55, 0x66],
            ETHER_TYPE_IPV4,
            vec![0x01, 0x02, 0x03, 0x04],
        );

        let bytes = frame.serialize();
        let restored = EthernetFrame::deserialize(&bytes).unwrap();

        assert_eq!(restored.dest_mac, [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        assert_eq!(restored.src_mac, [0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
        assert_eq!(restored.ether_type, ETHER_TYPE_IPV4);
        assert_eq!(restored.payload, vec![0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_frame_serialize_correct_layout() {
        let frame = EthernetFrame::new(
            BROADCAST_MAC,
            [0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
            ETHER_TYPE_ARP,
            vec![0xDE, 0xAD],
        );

        let bytes = frame.serialize();
        assert_eq!(bytes.len(), 16); // 6 + 6 + 2 + 2
        assert_eq!(&bytes[0..6], &BROADCAST_MAC);
        assert_eq!(bytes[12], 0x08);
        assert_eq!(bytes[13], 0x06);
    }

    #[test]
    fn test_frame_deserialize_too_short() {
        assert!(EthernetFrame::deserialize(&[0u8; 13]).is_none());
        assert!(EthernetFrame::deserialize(&[]).is_none());
    }

    #[test]
    fn test_frame_empty_payload() {
        let frame = EthernetFrame::new(
            [1, 2, 3, 4, 5, 6],
            [7, 8, 9, 10, 11, 12],
            ETHER_TYPE_IPV4,
            vec![],
        );
        let bytes = frame.serialize();
        assert_eq!(bytes.len(), 14);
        let restored = EthernetFrame::deserialize(&bytes).unwrap();
        assert!(restored.payload.is_empty());
    }

    #[test]
    fn test_arp_table_insert_and_lookup() {
        let mut table = ArpTable::new();
        let ip = [10, 0, 0, 1];
        let mac = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];

        table.insert(ip, mac);
        assert_eq!(table.lookup(&ip), Some(mac));
    }

    #[test]
    fn test_arp_table_lookup_unknown() {
        let table = ArpTable::new();
        assert_eq!(table.lookup(&[192, 168, 1, 1]), None);
    }

    #[test]
    fn test_arp_table_update_existing() {
        let mut table = ArpTable::new();
        let ip = [10, 0, 0, 1];
        table.insert(ip, [0x11; 6]);
        table.insert(ip, [0xAA; 6]);

        assert_eq!(table.lookup(&ip), Some([0xAA; 6]));
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_arp_table_multiple_entries() {
        let mut table = ArpTable::new();
        table.insert([10, 0, 0, 1], [0x11; 6]);
        table.insert([10, 0, 0, 2], [0x22; 6]);
        table.insert([10, 0, 0, 3], [0x33; 6]);

        assert_eq!(table.len(), 3);
        assert!(!table.is_empty());
        assert_eq!(table.lookup(&[10, 0, 0, 2]), Some([0x22; 6]));
    }

    #[test]
    fn test_arp_table_default() {
        let table = ArpTable::default();
        assert!(table.is_empty());
    }
}
