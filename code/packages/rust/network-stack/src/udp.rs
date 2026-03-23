// ============================================================================
// Layer 4: UDP — User Datagram Protocol
// ============================================================================
//
// UDP is the "anti-TCP" — no reliability, no ordering, no flow control,
// no connection state. You send a datagram; it either arrives or it doesn't.
//
// Why use something so unreliable? Because simplicity and speed matter:
//   - DNS lookups: one question, one answer
//   - Video streaming: a dropped frame is better than pausing
//   - Online games: latest position matters, old ones don't
//
// The entire UDP header is only 8 bytes (vs TCP's 20). Less overhead per
// packet means higher throughput for high-frequency small messages.
//
// UDP Header:
//   +-----------+-----------+--------+----------+
//   | src_port  | dst_port  | length | checksum |
//   | (2 bytes) | (2 bytes) | (2 B)  | (2 B)    |
//   +-----------+-----------+--------+----------+
//
// ============================================================================

use crate::ethernet::Ipv4Address;

// ============================================================================
// UdpHeader — 8-byte datagram header
// ============================================================================
#[derive(Debug, Clone, PartialEq)]
pub struct UdpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub length: u16,
    pub checksum: u16,
}

impl UdpHeader {
    pub fn new(src_port: u16, dst_port: u16, length: u16) -> Self {
        Self { src_port, dst_port, length, checksum: 0 }
    }

    /// Serialize into an 8-byte vector.
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; 8];
        bytes[0] = (self.src_port >> 8) as u8;
        bytes[1] = (self.src_port & 0xFF) as u8;
        bytes[2] = (self.dst_port >> 8) as u8;
        bytes[3] = (self.dst_port & 0xFF) as u8;
        bytes[4] = (self.length >> 8) as u8;
        bytes[5] = (self.length & 0xFF) as u8;
        bytes[6] = (self.checksum >> 8) as u8;
        bytes[7] = (self.checksum & 0xFF) as u8;
        bytes
    }

    /// Deserialize from a byte slice.
    pub fn deserialize(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 8 {
            return None;
        }

        Some(Self {
            src_port: ((bytes[0] as u16) << 8) | (bytes[1] as u16),
            dst_port: ((bytes[2] as u16) << 8) | (bytes[3] as u16),
            length: ((bytes[4] as u16) << 8) | (bytes[5] as u16),
            checksum: ((bytes[6] as u16) << 8) | (bytes[7] as u16),
        })
    }
}

// ============================================================================
// UdpSocket — Connectionless Datagram Socket
// ============================================================================
//
// Unlike TCP, a UDP socket has no connection state. Each send_to() specifies
// the destination independently. Incoming datagrams arrive in a queue with
// sender information attached (so the application knows who sent what).
//
// ============================================================================

/// A received UDP datagram with sender information.
#[derive(Debug, Clone)]
pub struct UdpDatagram {
    pub data: Vec<u8>,
    pub src_ip: Ipv4Address,
    pub src_port: u16,
}

pub struct UdpSocket {
    pub local_port: u16,
    recv_queue: Vec<UdpDatagram>,
}

impl UdpSocket {
    pub fn new(local_port: u16) -> Self {
        Self { local_port, recv_queue: Vec::new() }
    }

    /// Prepare a datagram for sending. Returns (header, data).
    pub fn send_to(&self, data: &[u8], dst_port: u16) -> (UdpHeader, Vec<u8>) {
        let header = UdpHeader::new(self.local_port, dst_port, 8 + data.len() as u16);
        (header, data.to_vec())
    }

    /// Deliver an incoming datagram to this socket's receive queue.
    pub fn deliver(&mut self, data: Vec<u8>, src_ip: Ipv4Address, src_port: u16) {
        self.recv_queue.push(UdpDatagram { data, src_ip, src_port });
    }

    /// Retrieve the next datagram from the queue, or None if empty.
    pub fn receive_from(&mut self) -> Option<UdpDatagram> {
        if self.recv_queue.is_empty() {
            None
        } else {
            Some(self.recv_queue.remove(0))
        }
    }

    /// Number of datagrams waiting in the queue.
    pub fn pending(&self) -> usize {
        self.recv_queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_serialize_deserialize() {
        let header = UdpHeader { src_port: 12345, dst_port: 53, length: 20, checksum: 0xABCD };
        let bytes = header.serialize();
        let restored = UdpHeader::deserialize(&bytes).unwrap();

        assert_eq!(restored.src_port, 12345);
        assert_eq!(restored.dst_port, 53);
        assert_eq!(restored.length, 20);
        assert_eq!(restored.checksum, 0xABCD);
    }

    #[test]
    fn test_header_8_bytes() {
        let header = UdpHeader::new(1000, 2000, 8);
        assert_eq!(header.serialize().len(), 8);
    }

    #[test]
    fn test_deserialize_too_short() {
        assert!(UdpHeader::deserialize(&[0u8; 7]).is_none());
        assert!(UdpHeader::deserialize(&[]).is_none());
    }

    #[test]
    fn test_default_checksum() {
        let header = UdpHeader::new(100, 200, 8);
        assert_eq!(header.checksum, 0);
    }

    #[test]
    fn test_send_to() {
        let sock = UdpSocket::new(5000);
        let (header, data) = sock.send_to(&[1, 2, 3, 4, 5], 53);

        assert_eq!(header.src_port, 5000);
        assert_eq!(header.dst_port, 53);
        assert_eq!(header.length, 13); // 8 + 5
        assert_eq!(data, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_deliver_and_receive() {
        let mut sock = UdpSocket::new(5000);
        sock.deliver(vec![10, 20, 30], [192, 168, 1, 1], 12345);

        let dgram = sock.receive_from().unwrap();
        assert_eq!(dgram.data, vec![10, 20, 30]);
        assert_eq!(dgram.src_ip, [192, 168, 1, 1]);
        assert_eq!(dgram.src_port, 12345);
    }

    #[test]
    fn test_receive_empty() {
        let mut sock = UdpSocket::new(5000);
        assert!(sock.receive_from().is_none());
    }

    #[test]
    fn test_fifo_ordering() {
        let mut sock = UdpSocket::new(5000);
        sock.deliver(vec![1], [10, 0, 0, 1], 1000);
        sock.deliver(vec![2], [10, 0, 0, 2], 2000);
        sock.deliver(vec![3], [10, 0, 0, 3], 3000);

        assert_eq!(sock.receive_from().unwrap().data, vec![1]);
        assert_eq!(sock.receive_from().unwrap().data, vec![2]);
        assert_eq!(sock.receive_from().unwrap().data, vec![3]);
        assert!(sock.receive_from().is_none());
    }

    #[test]
    fn test_pending() {
        let mut sock = UdpSocket::new(5000);
        assert_eq!(sock.pending(), 0);
        sock.deliver(vec![1], [0; 4], 100);
        sock.deliver(vec![2], [0; 4], 200);
        assert_eq!(sock.pending(), 2);
    }
}
