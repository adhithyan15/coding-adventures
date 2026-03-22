// ============================================================================
// Layer 4: TCP — Transmission Control Protocol
// ============================================================================
//
// TCP provides a **reliable, ordered, byte-stream** service on top of IP's
// unreliable packet delivery. It is the workhorse of the Internet — HTTP,
// email, file transfers, and SSH all run on TCP.
//
// TCP achieves reliability through:
//   1. Sequence numbers — every byte is numbered for ordering
//   2. Acknowledgments — receiver confirms what it has received
//   3. Retransmission — sender resends unacknowledged data
//   4. Flow control — receiver advertises available buffer space
//
// The TCP state machine has 11 states. The most important transitions:
//
//   Three-way handshake:
//     Client: SYN          -> SYN_SENT
//     Server: SYN+ACK      -> SYN_RECEIVED
//     Client: ACK           -> ESTABLISHED (both sides)
//
//   Four-way teardown:
//     Initiator: FIN -> FIN_WAIT_1 -> FIN_WAIT_2 -> TIME_WAIT -> CLOSED
//     Responder: ACK -> CLOSE_WAIT -> LAST_ACK -> CLOSED
//
// ============================================================================

use crate::ethernet::Ipv4Address;

/// TCP flag bit masks — can be combined with bitwise OR.
pub const TCP_FIN: u8 = 0x01;  // Finish — sender is done
pub const TCP_SYN: u8 = 0x02;  // Synchronize — initiate connection
pub const TCP_RST: u8 = 0x04;  // Reset — abort connection
pub const TCP_PSH: u8 = 0x08;  // Push — deliver immediately
pub const TCP_ACK: u8 = 0x10;  // Acknowledge — ack field is valid

/// The 11 TCP states from RFC 793.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    LastAck,
    TimeWait,
    Closing,
}

// ============================================================================
// TcpHeader
// ============================================================================
//
// The 20-byte TCP header (no options). Contains source/destination ports,
// sequence and acknowledgment numbers, flags, and window size.
//
// Wire format:
//
//    0                   1                   2                   3
//   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//   |          Source Port          |       Destination Port        |
//   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//   |                        Sequence Number                       |
//   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//   |                    Acknowledgment Number                     |
//   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//   |  Data |           |   Flags   |          Window Size         |
//   | Offset| Reserved  |           |                              |
//   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//
// ============================================================================
#[derive(Debug, Clone, PartialEq)]
pub struct TcpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq_num: u32,
    pub ack_num: u32,
    pub data_offset: u8,
    pub flags: u8,
    pub window_size: u16,
}

impl TcpHeader {
    pub fn new(src_port: u16, dst_port: u16, seq_num: u32, ack_num: u32, flags: u8) -> Self {
        Self {
            src_port,
            dst_port,
            seq_num,
            ack_num,
            data_offset: 5,
            flags,
            window_size: 65535,
        }
    }

    /// Serialize into a 20-byte vector.
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; 20];

        bytes[0] = (self.src_port >> 8) as u8;
        bytes[1] = (self.src_port & 0xFF) as u8;
        bytes[2] = (self.dst_port >> 8) as u8;
        bytes[3] = (self.dst_port & 0xFF) as u8;

        bytes[4] = (self.seq_num >> 24) as u8;
        bytes[5] = (self.seq_num >> 16) as u8;
        bytes[6] = (self.seq_num >> 8) as u8;
        bytes[7] = (self.seq_num & 0xFF) as u8;

        bytes[8] = (self.ack_num >> 24) as u8;
        bytes[9] = (self.ack_num >> 16) as u8;
        bytes[10] = (self.ack_num >> 8) as u8;
        bytes[11] = (self.ack_num & 0xFF) as u8;

        bytes[12] = (self.data_offset & 0x0F) << 4;
        bytes[13] = self.flags & 0x3F;

        bytes[14] = (self.window_size >> 8) as u8;
        bytes[15] = (self.window_size & 0xFF) as u8;

        bytes
    }

    /// Deserialize from a byte slice.
    pub fn deserialize(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 20 {
            return None;
        }

        Some(Self {
            src_port: ((bytes[0] as u16) << 8) | (bytes[1] as u16),
            dst_port: ((bytes[2] as u16) << 8) | (bytes[3] as u16),
            seq_num: ((bytes[4] as u32) << 24) | ((bytes[5] as u32) << 16)
                   | ((bytes[6] as u32) << 8) | (bytes[7] as u32),
            ack_num: ((bytes[8] as u32) << 24) | ((bytes[9] as u32) << 16)
                   | ((bytes[10] as u32) << 8) | (bytes[11] as u32),
            data_offset: (bytes[12] >> 4) & 0x0F,
            flags: bytes[13] & 0x3F,
            window_size: ((bytes[14] as u16) << 8) | (bytes[15] as u16),
        })
    }

    pub fn is_syn(&self) -> bool { self.flags & TCP_SYN != 0 }
    pub fn is_ack(&self) -> bool { self.flags & TCP_ACK != 0 }
    pub fn is_fin(&self) -> bool { self.flags & TCP_FIN != 0 }
    pub fn is_rst(&self) -> bool { self.flags & TCP_RST != 0 }
    pub fn is_psh(&self) -> bool { self.flags & TCP_PSH != 0 }
}

// ============================================================================
// TcpConnection — The TCP State Machine
// ============================================================================
//
// Each TCP connection tracks:
//   - Current state (one of 11 states)
//   - Local and remote port/IP
//   - Sequence numbers for send and receive
//   - Send and receive buffers
//
// The state machine is driven by:
//   - initiate_connect() — client opens connection
//   - initiate_listen() — server starts listening
//   - handle_segment() — process an incoming segment
//   - send_data() — queue data for sending
//   - receive() — read received data
//   - initiate_close() — start connection teardown
//
// ============================================================================
pub struct TcpConnection {
    pub state: TcpState,
    pub local_port: u16,
    pub remote_port: u16,
    pub remote_ip: Ipv4Address,
    pub seq_num: u32,
    pub ack_num: u32,
    pub send_buffer: Vec<u8>,
    pub recv_buffer: Vec<u8>,
}

impl TcpConnection {
    pub fn new(local_port: u16) -> Self {
        Self {
            state: TcpState::Closed,
            local_port,
            remote_port: 0,
            remote_ip: [0; 4],
            seq_num: rand_seq(),
            ack_num: 0,
            send_buffer: Vec::new(),
            recv_buffer: Vec::new(),
        }
    }

    /// Initiate active open (client side). CLOSED -> SYN_SENT.
    pub fn initiate_connect(&mut self, remote_ip: Ipv4Address, remote_port: u16) -> TcpHeader {
        self.remote_ip = remote_ip;
        self.remote_port = remote_port;
        self.state = TcpState::SynSent;

        TcpHeader::new(self.local_port, self.remote_port, self.seq_num, 0, TCP_SYN)
    }

    /// Start listening. CLOSED -> LISTEN.
    pub fn initiate_listen(&mut self) {
        self.state = TcpState::Listen;
    }

    /// Handle an incoming TCP segment. Returns a response header or None.
    pub fn handle_segment(&mut self, header: &TcpHeader, payload: &[u8]) -> Option<TcpHeader> {
        match self.state {
            TcpState::Listen => self.handle_listen(header),
            TcpState::SynSent => self.handle_syn_sent(header),
            TcpState::SynReceived => self.handle_syn_received(header),
            TcpState::Established => self.handle_established(header, payload),
            TcpState::FinWait1 => self.handle_fin_wait_1(header),
            TcpState::FinWait2 => self.handle_fin_wait_2(header),
            TcpState::LastAck => self.handle_last_ack(header),
            TcpState::Closing => self.handle_closing(header),
            _ => None,
        }
    }

    /// Queue data for sending. Returns a segment header.
    pub fn send_data(&mut self, data: &[u8]) -> Option<TcpHeader> {
        if self.state != TcpState::Established {
            return None;
        }

        self.send_buffer.extend_from_slice(data);
        let header = TcpHeader::new(
            self.local_port, self.remote_port,
            self.seq_num, self.ack_num,
            TCP_ACK | TCP_PSH,
        );
        self.seq_num = self.seq_num.wrapping_add(data.len() as u32);
        Some(header)
    }

    /// Read up to `count` bytes from the receive buffer.
    pub fn receive(&mut self, count: usize) -> Vec<u8> {
        let n = count.min(self.recv_buffer.len());
        self.recv_buffer.drain(..n).collect()
    }

    /// Initiate connection close. ESTABLISHED -> FIN_WAIT_1 or CLOSE_WAIT -> LAST_ACK.
    pub fn initiate_close(&mut self) -> Option<TcpHeader> {
        match self.state {
            TcpState::Established => {
                self.state = TcpState::FinWait1;
            }
            TcpState::CloseWait => {
                self.state = TcpState::LastAck;
            }
            _ => return None,
        }

        let header = TcpHeader::new(
            self.local_port, self.remote_port,
            self.seq_num, self.ack_num,
            TCP_FIN | TCP_ACK,
        );
        self.seq_num = self.seq_num.wrapping_add(1);
        Some(header)
    }

    // --- Private state handlers ---

    fn handle_listen(&mut self, header: &TcpHeader) -> Option<TcpHeader> {
        if !header.is_syn() {
            return None;
        }

        self.remote_port = header.src_port;
        self.ack_num = header.seq_num.wrapping_add(1);
        self.state = TcpState::SynReceived;

        Some(TcpHeader::new(
            self.local_port, self.remote_port,
            self.seq_num, self.ack_num,
            TCP_SYN | TCP_ACK,
        ))
    }

    fn handle_syn_sent(&mut self, header: &TcpHeader) -> Option<TcpHeader> {
        if !header.is_syn() || !header.is_ack() {
            return None;
        }

        self.ack_num = header.seq_num.wrapping_add(1);
        self.seq_num = self.seq_num.wrapping_add(1); // SYN consumes 1 seq
        self.state = TcpState::Established;

        Some(TcpHeader::new(
            self.local_port, self.remote_port,
            self.seq_num, self.ack_num,
            TCP_ACK,
        ))
    }

    fn handle_syn_received(&mut self, header: &TcpHeader) -> Option<TcpHeader> {
        if !header.is_ack() {
            return None;
        }

        self.seq_num = self.seq_num.wrapping_add(1); // SYN consumes 1 seq
        self.state = TcpState::Established;
        None
    }

    fn handle_established(&mut self, header: &TcpHeader, payload: &[u8]) -> Option<TcpHeader> {
        if header.is_fin() {
            self.ack_num = header.seq_num.wrapping_add(1);
            self.state = TcpState::CloseWait;
            return Some(TcpHeader::new(
                self.local_port, self.remote_port,
                self.seq_num, self.ack_num,
                TCP_ACK,
            ));
        }

        if !payload.is_empty() {
            self.recv_buffer.extend_from_slice(payload);
            self.ack_num = header.seq_num.wrapping_add(payload.len() as u32);
            return Some(TcpHeader::new(
                self.local_port, self.remote_port,
                self.seq_num, self.ack_num,
                TCP_ACK,
            ));
        }

        None
    }

    fn handle_fin_wait_1(&mut self, header: &TcpHeader) -> Option<TcpHeader> {
        if header.is_fin() && header.is_ack() {
            self.ack_num = header.seq_num.wrapping_add(1);
            self.state = TcpState::TimeWait;
            return Some(TcpHeader::new(
                self.local_port, self.remote_port,
                self.seq_num, self.ack_num,
                TCP_ACK,
            ));
        } else if header.is_ack() {
            self.state = TcpState::FinWait2;
            return None;
        } else if header.is_fin() {
            self.ack_num = header.seq_num.wrapping_add(1);
            self.state = TcpState::Closing;
            return Some(TcpHeader::new(
                self.local_port, self.remote_port,
                self.seq_num, self.ack_num,
                TCP_ACK,
            ));
        }
        None
    }

    fn handle_fin_wait_2(&mut self, header: &TcpHeader) -> Option<TcpHeader> {
        if !header.is_fin() {
            return None;
        }

        self.ack_num = header.seq_num.wrapping_add(1);
        self.state = TcpState::TimeWait;

        Some(TcpHeader::new(
            self.local_port, self.remote_port,
            self.seq_num, self.ack_num,
            TCP_ACK,
        ))
    }

    fn handle_last_ack(&mut self, header: &TcpHeader) -> Option<TcpHeader> {
        if header.is_ack() {
            self.state = TcpState::Closed;
        }
        None
    }

    fn handle_closing(&mut self, header: &TcpHeader) -> Option<TcpHeader> {
        if header.is_ack() {
            self.state = TcpState::TimeWait;
        }
        None
    }
}

/// Simple deterministic "random" sequence number for testing predictability.
fn rand_seq() -> u32 {
    1000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_serialize_deserialize() {
        let header = TcpHeader::new(49152, 80, 1000, 2000, TCP_SYN | TCP_ACK);
        let bytes = header.serialize();
        let restored = TcpHeader::deserialize(&bytes).unwrap();

        assert_eq!(restored.src_port, 49152);
        assert_eq!(restored.dst_port, 80);
        assert_eq!(restored.seq_num, 1000);
        assert_eq!(restored.ack_num, 2000);
        assert_eq!(restored.flags, TCP_SYN | TCP_ACK);
        assert_eq!(restored.window_size, 65535);
        assert_eq!(restored.data_offset, 5);
    }

    #[test]
    fn test_header_20_bytes() {
        let header = TcpHeader::new(80, 443, 0, 0, 0);
        assert_eq!(header.serialize().len(), 20);
    }

    #[test]
    fn test_deserialize_too_short() {
        assert!(TcpHeader::deserialize(&[0u8; 19]).is_none());
    }

    #[test]
    fn test_flag_helpers() {
        let header = TcpHeader::new(80, 443, 0, 0, TCP_SYN | TCP_ACK);
        assert!(header.is_syn());
        assert!(header.is_ack());
        assert!(!header.is_fin());
        assert!(!header.is_rst());
        assert!(!header.is_psh());
    }

    #[test]
    fn test_large_seq_num() {
        let header = TcpHeader::new(80, 443, 0xFFFFFFFF, 0, 0);
        let bytes = header.serialize();
        let restored = TcpHeader::deserialize(&bytes).unwrap();
        assert_eq!(restored.seq_num, 0xFFFFFFFF);
    }

    #[test]
    fn test_initial_state_closed() {
        let conn = TcpConnection::new(8080);
        assert_eq!(conn.state, TcpState::Closed);
    }

    #[test]
    fn test_initiate_connect() {
        let mut conn = TcpConnection::new(49152);
        let syn = conn.initiate_connect([10, 0, 0, 2], 80);

        assert_eq!(conn.state, TcpState::SynSent);
        assert!(syn.is_syn());
        assert!(!syn.is_ack());
        assert_eq!(syn.src_port, 49152);
        assert_eq!(syn.dst_port, 80);
    }

    #[test]
    fn test_initiate_listen() {
        let mut conn = TcpConnection::new(80);
        conn.initiate_listen();
        assert_eq!(conn.state, TcpState::Listen);
    }

    #[test]
    fn test_three_way_handshake() {
        let mut client = TcpConnection::new(49152);
        let mut server = TcpConnection::new(80);

        server.initiate_listen();
        assert_eq!(server.state, TcpState::Listen);

        let syn = client.initiate_connect([10, 0, 0, 2], 80);
        assert_eq!(client.state, TcpState::SynSent);

        let syn_ack = server.handle_segment(&syn, &[]).unwrap();
        assert_eq!(server.state, TcpState::SynReceived);
        assert!(syn_ack.is_syn());
        assert!(syn_ack.is_ack());

        let ack = client.handle_segment(&syn_ack, &[]).unwrap();
        assert_eq!(client.state, TcpState::Established);
        assert!(ack.is_ack());

        server.handle_segment(&ack, &[]);
        assert_eq!(server.state, TcpState::Established);
    }

    #[test]
    fn test_data_transfer() {
        let (mut client, mut server) = establish_connection();

        let data = vec![72, 101, 108, 108, 111]; // "Hello"
        let data_header = client.send_data(&data).unwrap();
        assert!(data_header.is_psh());

        let ack = server.handle_segment(&data_header, &data).unwrap();
        assert!(ack.is_ack());

        let received = server.receive(10);
        assert_eq!(received, data);
    }

    #[test]
    fn test_send_data_not_established() {
        let mut conn = TcpConnection::new(80);
        assert!(conn.send_data(&[1, 2, 3]).is_none());
    }

    #[test]
    fn test_receive_empty() {
        let (mut client, _) = establish_connection();
        assert!(client.receive(10).is_empty());
    }

    #[test]
    fn test_connection_teardown() {
        let (mut client, mut server) = establish_connection();

        // Client sends FIN
        let fin = client.initiate_close().unwrap();
        assert_eq!(client.state, TcpState::FinWait1);
        assert!(fin.is_fin());

        // Server ACKs
        let ack = server.handle_segment(&fin, &[]).unwrap();
        assert_eq!(server.state, TcpState::CloseWait);

        // Client moves to FIN_WAIT_2
        client.handle_segment(&ack, &[]);
        assert_eq!(client.state, TcpState::FinWait2);

        // Server sends FIN
        let server_fin = server.initiate_close().unwrap();
        assert_eq!(server.state, TcpState::LastAck);

        // Client ACKs, moves to TIME_WAIT
        let final_ack = client.handle_segment(&server_fin, &[]).unwrap();
        assert_eq!(client.state, TcpState::TimeWait);

        // Server receives final ACK, moves to CLOSED
        server.handle_segment(&final_ack, &[]);
        assert_eq!(server.state, TcpState::Closed);
    }

    #[test]
    fn test_initiate_close_when_closed() {
        let mut conn = TcpConnection::new(80);
        assert!(conn.initiate_close().is_none());
    }

    #[test]
    fn test_simultaneous_close() {
        let (mut client, server) = establish_connection();

        let _fin = client.initiate_close().unwrap();
        assert_eq!(client.state, TcpState::FinWait1);

        let combined = TcpHeader::new(80, 49152, server.seq_num, client.seq_num, TCP_FIN | TCP_ACK);
        let result = client.handle_segment(&combined, &[]).unwrap();
        assert_eq!(client.state, TcpState::TimeWait);
        assert!(result.is_ack());
    }

    fn establish_connection() -> (TcpConnection, TcpConnection) {
        let mut client = TcpConnection::new(49152);
        let mut server = TcpConnection::new(80);

        server.initiate_listen();
        let syn = client.initiate_connect([10, 0, 0, 2], 80);
        let syn_ack = server.handle_segment(&syn, &[]).unwrap();
        let ack = client.handle_segment(&syn_ack, &[]).unwrap();
        server.handle_segment(&ack, &[]);

        (client, server)
    }
}
