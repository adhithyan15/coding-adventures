// ============================================================================
// NetworkWire — Simulated Physical Network Medium
// ============================================================================
//
// In a real network, an Ethernet cable carries electrical signals between
// two devices. We simulate this with an in-memory bidirectional channel.
//
// The wire connects two endpoints (A and B). When A sends a frame, it
// appears in B's receive queue, and vice versa.
//
//   +---------+                              +---------+
//   | Host A  | -------- NetworkWire ------> | Host B  |
//   |         | <------- NetworkWire ------- |         |
//   +---------+                              +---------+
//
// Two FIFO queues:
//   a_to_b: frames sent by A, waiting for B
//   b_to_a: frames sent by B, waiting for A
//
// No latency, no packet loss, no bandwidth limits — just a clean pipe
// for understanding how protocol layers work.
//
// ============================================================================

use std::collections::VecDeque;

pub struct NetworkWire {
    a_to_b: VecDeque<Vec<u8>>,
    b_to_a: VecDeque<Vec<u8>>,
}

impl NetworkWire {
    pub fn new() -> Self {
        Self {
            a_to_b: VecDeque::new(),
            b_to_a: VecDeque::new(),
        }
    }

    /// Host A sends a frame (received by Host B).
    pub fn send_a(&mut self, frame: Vec<u8>) {
        self.a_to_b.push_back(frame);
    }

    /// Host B sends a frame (received by Host A).
    pub fn send_b(&mut self, frame: Vec<u8>) {
        self.b_to_a.push_back(frame);
    }

    /// Host A receives a frame sent by Host B.
    pub fn receive_a(&mut self) -> Option<Vec<u8>> {
        self.b_to_a.pop_front()
    }

    /// Host B receives a frame sent by Host A.
    pub fn receive_b(&mut self) -> Option<Vec<u8>> {
        self.a_to_b.pop_front()
    }

    /// Is there data waiting for Host A?
    pub fn has_data_for_a(&self) -> bool {
        !self.b_to_a.is_empty()
    }

    /// Is there data waiting for Host B?
    pub fn has_data_for_b(&self) -> bool {
        !self.a_to_b.is_empty()
    }

    /// Number of frames queued A->B.
    pub fn pending_a_to_b(&self) -> usize {
        self.a_to_b.len()
    }

    /// Number of frames queued B->A.
    pub fn pending_b_to_a(&self) -> usize {
        self.b_to_a.len()
    }
}

impl Default for NetworkWire {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ethernet::{EthernetFrame, ETHER_TYPE_IPV4};

    #[test]
    fn test_send_a_receive_b() {
        let mut wire = NetworkWire::new();
        wire.send_a(vec![1, 2, 3]);
        assert_eq!(wire.receive_b(), Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_send_b_receive_a() {
        let mut wire = NetworkWire::new();
        wire.send_b(vec![4, 5, 6]);
        assert_eq!(wire.receive_a(), Some(vec![4, 5, 6]));
    }

    #[test]
    fn test_bidirectional() {
        let mut wire = NetworkWire::new();
        wire.send_a(vec![1, 2]);
        wire.send_b(vec![3, 4]);

        assert_eq!(wire.receive_a(), Some(vec![3, 4]));
        assert_eq!(wire.receive_b(), Some(vec![1, 2]));
    }

    #[test]
    fn test_fifo_ordering() {
        let mut wire = NetworkWire::new();
        wire.send_a(vec![1]);
        wire.send_a(vec![2]);
        wire.send_a(vec![3]);

        assert_eq!(wire.receive_b(), Some(vec![1]));
        assert_eq!(wire.receive_b(), Some(vec![2]));
        assert_eq!(wire.receive_b(), Some(vec![3]));
    }

    #[test]
    fn test_receive_empty() {
        let mut wire = NetworkWire::new();
        assert_eq!(wire.receive_a(), None);
        assert_eq!(wire.receive_b(), None);
    }

    #[test]
    fn test_has_data_for_a() {
        let mut wire = NetworkWire::new();
        assert!(!wire.has_data_for_a());

        wire.send_b(vec![1]);
        assert!(wire.has_data_for_a());

        wire.receive_a();
        assert!(!wire.has_data_for_a());
    }

    #[test]
    fn test_has_data_for_b() {
        let mut wire = NetworkWire::new();
        assert!(!wire.has_data_for_b());

        wire.send_a(vec![1]);
        assert!(wire.has_data_for_b());

        wire.receive_b();
        assert!(!wire.has_data_for_b());
    }

    #[test]
    fn test_pending_counts() {
        let mut wire = NetworkWire::new();
        assert_eq!(wire.pending_a_to_b(), 0);
        assert_eq!(wire.pending_b_to_a(), 0);

        wire.send_a(vec![1]);
        wire.send_a(vec![2]);
        assert_eq!(wire.pending_a_to_b(), 2);

        wire.send_b(vec![3]);
        assert_eq!(wire.pending_b_to_a(), 1);
    }

    #[test]
    fn test_no_cross_talk() {
        let mut wire = NetworkWire::new();
        wire.send_a(vec![42]);
        assert_eq!(wire.receive_a(), None); // A should not see its own data
        assert_eq!(wire.receive_b(), Some(vec![42]));
    }

    #[test]
    fn test_ethernet_frame_over_wire() {
        let mut wire = NetworkWire::new();

        let frame = EthernetFrame::new(
            [0xBB; 6],
            [0xAA; 6],
            ETHER_TYPE_IPV4,
            vec![0xDE, 0xAD],
        );

        wire.send_a(frame.serialize());
        let received = wire.receive_b().unwrap();
        let restored = EthernetFrame::deserialize(&received).unwrap();

        assert_eq!(restored.dest_mac, [0xBB; 6]);
        assert_eq!(restored.src_mac, [0xAA; 6]);
        assert_eq!(restored.payload, vec![0xDE, 0xAD]);
    }

    #[test]
    fn test_default() {
        let wire = NetworkWire::default();
        assert!(!wire.has_data_for_a());
        assert!(!wire.has_data_for_b());
    }
}
