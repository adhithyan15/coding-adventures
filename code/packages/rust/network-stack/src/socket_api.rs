// ============================================================================
// Socket API — The Berkeley Sockets Interface
// ============================================================================
//
// The Berkeley Sockets API (invented at UC Berkeley in 1983) is the standard
// network programming interface used by every operating system. Network
// connections are treated like files — you open, read/write, and close them.
//
// Client workflow:
//   fd = socket(STREAM)
//   connect(fd, server_ip, 80)
//   send(fd, "GET / HTTP/1.1")
//   data = recv(fd)
//   close(fd)
//
// Server workflow:
//   fd = socket(STREAM)
//   bind(fd, ip, 80)
//   listen(fd, 5)
//   client_fd = accept(fd)
//   data = recv(client_fd)
//   send(client_fd, response)
//   close(client_fd)
//
// ============================================================================

use std::collections::HashMap;
use crate::tcp::{TcpConnection, TcpHeader};
use crate::udp::UdpSocket;
use crate::ethernet::Ipv4Address;

/// Socket type: STREAM (TCP) or DGRAM (UDP).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    Stream = 1,
    Dgram = 2,
}

/// Information about a queued TCP connection waiting to be accepted.
pub struct AcceptedConnection {
    pub remote_ip: Ipv4Address,
    pub remote_port: u16,
    pub connection: TcpConnection,
}

/// A single network endpoint — one end of a connection (or potential connection).
pub struct Socket {
    pub fd: i32,
    pub socket_type: SocketType,
    pub local_ip: Option<Ipv4Address>,
    pub local_port: Option<u16>,
    pub remote_ip: Option<Ipv4Address>,
    pub remote_port: Option<u16>,
    pub tcp_connection: Option<TcpConnection>,
    pub udp_socket: Option<UdpSocket>,
    pub listening: bool,
    pub accept_queue: Vec<AcceptedConnection>,
}

impl Socket {
    fn new(fd: i32, socket_type: SocketType) -> Self {
        Self {
            fd,
            socket_type,
            local_ip: None,
            local_port: None,
            remote_ip: None,
            remote_port: None,
            tcp_connection: None,
            udp_socket: None,
            listening: false,
            accept_queue: Vec::new(),
        }
    }
}

// ============================================================================
// SocketManager — Kernel-Side Socket Management
// ============================================================================
//
// Allocates file descriptors, tracks all open sockets, and dispatches
// operations to the appropriate protocol handler (TCP or UDP).
//
// ============================================================================
pub struct SocketManager {
    sockets: HashMap<i32, Socket>,
    next_fd: i32,
    used_ports: HashMap<u16, i32>,
}

impl SocketManager {
    pub fn new() -> Self {
        Self {
            sockets: HashMap::new(),
            next_fd: 3, // 0=stdin, 1=stdout, 2=stderr
            used_ports: HashMap::new(),
        }
    }

    /// Create a new socket. Returns a file descriptor.
    pub fn create_socket(&mut self, socket_type: SocketType) -> i32 {
        let fd = self.next_fd;
        self.next_fd += 1;
        self.sockets.insert(fd, Socket::new(fd, socket_type));
        fd
    }

    /// Bind a socket to a local IP and port. Returns true on success.
    pub fn bind(&mut self, fd: i32, ip: Ipv4Address, port: u16) -> bool {
        if self.used_ports.contains_key(&port) {
            return false;
        }

        if let Some(sock) = self.sockets.get_mut(&fd) {
            sock.local_ip = Some(ip);
            sock.local_port = Some(port);
            self.used_ports.insert(port, fd);

            if sock.socket_type == SocketType::Dgram {
                sock.udp_socket = Some(UdpSocket::new(port));
            }
            true
        } else {
            false
        }
    }

    /// Mark a TCP socket as listening.
    pub fn listen(&mut self, fd: i32) -> bool {
        if let Some(sock) = self.sockets.get_mut(&fd) {
            if sock.socket_type != SocketType::Stream {
                return false;
            }
            sock.listening = true;
            let port = sock.local_port.unwrap_or(0);
            let mut conn = TcpConnection::new(port);
            conn.initiate_listen();
            sock.tcp_connection = Some(conn);
            true
        } else {
            false
        }
    }

    /// Accept an incoming connection. Returns (new_fd, remote_ip, remote_port).
    pub fn accept(&mut self, fd: i32) -> Option<(i32, Ipv4Address, u16)> {
        let (local_ip, local_port, conn_info) = {
            let sock = self.sockets.get_mut(&fd)?;
            if !sock.listening || sock.accept_queue.is_empty() {
                return None;
            }
            let info = sock.accept_queue.remove(0);
            (sock.local_ip, sock.local_port, info)
        };

        let new_fd = self.create_socket(SocketType::Stream);
        let new_sock = self.sockets.get_mut(&new_fd)?;
        new_sock.local_ip = local_ip;
        new_sock.local_port = local_port;
        new_sock.remote_ip = Some(conn_info.remote_ip);
        new_sock.remote_port = Some(conn_info.remote_port);
        new_sock.tcp_connection = Some(conn_info.connection);

        Some((new_fd, conn_info.remote_ip, conn_info.remote_port))
    }

    /// Initiate a TCP connection. Returns the SYN header.
    pub fn connect(&mut self, fd: i32, remote_ip: Ipv4Address, remote_port: u16) -> Option<TcpHeader> {
        // Check socket type first
        {
            let sock = self.sockets.get(&fd)?;
            if sock.socket_type != SocketType::Stream {
                return None;
            }
        }

        // Auto-assign ephemeral port if not bound
        let needs_port = self.sockets.get(&fd)?.local_port.is_none();
        if needs_port {
            let port = self.allocate_ephemeral_port();
            self.used_ports.insert(port, fd);
            self.sockets.get_mut(&fd)?.local_port = Some(port);
        }

        let sock = self.sockets.get_mut(&fd)?;
        sock.remote_ip = Some(remote_ip);
        sock.remote_port = Some(remote_port);

        let local_port = sock.local_port.unwrap();
        let mut conn = TcpConnection::new(local_port);
        let syn = conn.initiate_connect(remote_ip, remote_port);
        sock.tcp_connection = Some(conn);
        Some(syn)
    }

    /// Send data on a connected socket.
    pub fn send_data(&mut self, fd: i32, data: &[u8]) -> Option<TcpHeader> {
        let sock = self.sockets.get_mut(&fd)?;
        if sock.socket_type == SocketType::Stream {
            sock.tcp_connection.as_mut()?.send_data(data)
        } else {
            None
        }
    }

    /// Receive data from a TCP socket.
    pub fn recv(&mut self, fd: i32, count: usize) -> Option<Vec<u8>> {
        let sock = self.sockets.get_mut(&fd)?;
        if sock.socket_type == SocketType::Stream {
            Some(sock.tcp_connection.as_mut()?.receive(count))
        } else {
            None
        }
    }

    /// Close a socket. Returns true on success.
    pub fn close(&mut self, fd: i32) -> bool {
        if let Some(sock) = self.sockets.remove(&fd) {
            if let Some(port) = sock.local_port {
                self.used_ports.remove(&port);
            }
            true
        } else {
            false
        }
    }

    /// Get a reference to a socket by fd.
    pub fn get_socket(&self, fd: i32) -> Option<&Socket> {
        self.sockets.get(&fd)
    }

    /// Get a mutable reference to a socket by fd.
    pub fn get_socket_mut(&mut self, fd: i32) -> Option<&mut Socket> {
        self.sockets.get_mut(&fd)
    }

    fn allocate_ephemeral_port(&self) -> u16 {
        let mut port = 49152u16;
        while self.used_ports.contains_key(&port) {
            port += 1;
        }
        port
    }
}

impl Default for SocketManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tcp::{TcpState, TCP_SYN, TCP_ACK};

    #[test]
    fn test_create_stream_socket() {
        let mut mgr = SocketManager::new();
        let fd = mgr.create_socket(SocketType::Stream);
        assert!(fd >= 3);
        assert_eq!(mgr.get_socket(fd).unwrap().socket_type, SocketType::Stream);
    }

    #[test]
    fn test_create_dgram_socket() {
        let mut mgr = SocketManager::new();
        let fd = mgr.create_socket(SocketType::Dgram);
        assert_eq!(mgr.get_socket(fd).unwrap().socket_type, SocketType::Dgram);
    }

    #[test]
    fn test_bind() {
        let mut mgr = SocketManager::new();
        let fd = mgr.create_socket(SocketType::Stream);
        assert!(mgr.bind(fd, [10, 0, 0, 1], 8080));

        let sock = mgr.get_socket(fd).unwrap();
        assert_eq!(sock.local_ip, Some([10, 0, 0, 1]));
        assert_eq!(sock.local_port, Some(8080));
    }

    #[test]
    fn test_bind_duplicate_port() {
        let mut mgr = SocketManager::new();
        let fd1 = mgr.create_socket(SocketType::Stream);
        let fd2 = mgr.create_socket(SocketType::Stream);
        assert!(mgr.bind(fd1, [10, 0, 0, 1], 80));
        assert!(!mgr.bind(fd2, [10, 0, 0, 1], 80));
    }

    #[test]
    fn test_bind_invalid_fd() {
        let mut mgr = SocketManager::new();
        assert!(!mgr.bind(999, [10, 0, 0, 1], 80));
    }

    #[test]
    fn test_listen() {
        let mut mgr = SocketManager::new();
        let fd = mgr.create_socket(SocketType::Stream);
        mgr.bind(fd, [10, 0, 0, 1], 80);
        assert!(mgr.listen(fd));
        assert!(mgr.get_socket(fd).unwrap().listening);
    }

    #[test]
    fn test_listen_dgram_fails() {
        let mut mgr = SocketManager::new();
        let fd = mgr.create_socket(SocketType::Dgram);
        mgr.bind(fd, [10, 0, 0, 1], 80);
        assert!(!mgr.listen(fd));
    }

    #[test]
    fn test_connect() {
        let mut mgr = SocketManager::new();
        let fd = mgr.create_socket(SocketType::Stream);
        let syn = mgr.connect(fd, [10, 0, 0, 2], 80).unwrap();

        assert!(syn.is_syn());
        let sock = mgr.get_socket(fd).unwrap();
        assert_eq!(sock.remote_ip, Some([10, 0, 0, 2]));
        assert!(sock.local_port.unwrap() >= 49152);
    }

    #[test]
    fn test_connect_dgram_fails() {
        let mut mgr = SocketManager::new();
        let fd = mgr.create_socket(SocketType::Dgram);
        assert!(mgr.connect(fd, [10, 0, 0, 1], 80).is_none());
    }

    #[test]
    fn test_close() {
        let mut mgr = SocketManager::new();
        let fd = mgr.create_socket(SocketType::Stream);
        mgr.bind(fd, [10, 0, 0, 1], 8080);
        assert!(mgr.close(fd));
        assert!(mgr.get_socket(fd).is_none());
    }

    #[test]
    fn test_close_invalid_fd() {
        let mut mgr = SocketManager::new();
        assert!(!mgr.close(999));
    }

    #[test]
    fn test_dgram_bind_creates_udp_socket() {
        let mut mgr = SocketManager::new();
        let fd = mgr.create_socket(SocketType::Dgram);
        mgr.bind(fd, [10, 0, 0, 1], 5000);
        assert!(mgr.get_socket(fd).unwrap().udp_socket.is_some());
    }

    #[test]
    fn test_accept_empty_queue() {
        let mut mgr = SocketManager::new();
        let fd = mgr.create_socket(SocketType::Stream);
        mgr.bind(fd, [10, 0, 0, 1], 80);
        mgr.listen(fd);
        assert!(mgr.accept(fd).is_none());
    }

    #[test]
    fn test_accept_with_queued_connection() {
        let mut mgr = SocketManager::new();
        let fd = mgr.create_socket(SocketType::Stream);
        mgr.bind(fd, [10, 0, 0, 1], 80);
        mgr.listen(fd);

        // Manually enqueue a connection
        let mut conn = TcpConnection::new(80);
        conn.state = TcpState::Established;
        mgr.get_socket_mut(fd).unwrap().accept_queue.push(AcceptedConnection {
            remote_ip: [10, 0, 0, 2],
            remote_port: 49152,
            connection: conn,
        });

        let (new_fd, remote_ip, remote_port) = mgr.accept(fd).unwrap();
        assert_eq!(remote_ip, [10, 0, 0, 2]);
        assert_eq!(remote_port, 49152);

        let new_sock = mgr.get_socket(new_fd).unwrap();
        assert_eq!(new_sock.tcp_connection.as_ref().unwrap().state, TcpState::Established);
    }

    #[test]
    fn test_listen_invalid_fd() {
        let mut mgr = SocketManager::new();
        assert!(!mgr.listen(999));
    }

    #[test]
    fn test_send_recv_stream() {
        let mut mgr = SocketManager::new();
        let fd = mgr.create_socket(SocketType::Stream);
        mgr.connect(fd, [10, 0, 0, 2], 80);

        // Complete handshake
        let sock = mgr.get_socket(fd).unwrap();
        let client_seq = sock.tcp_connection.as_ref().unwrap().seq_num;
        let local_port = sock.local_port.unwrap();

        let syn_ack = TcpHeader::new(80, local_port, 5000, client_seq + 1, TCP_SYN | TCP_ACK);
        mgr.get_socket_mut(fd).unwrap().tcp_connection.as_mut().unwrap()
            .handle_segment(&syn_ack, &[]);

        // Send data
        let header = mgr.send_data(fd, &[72, 101, 108]).unwrap();
        assert!(header.is_psh());
    }

    #[test]
    fn test_send_invalid_fd() {
        let mut mgr = SocketManager::new();
        assert!(mgr.send_data(999, &[1]).is_none());
    }

    #[test]
    fn test_recv_invalid_fd() {
        let mut mgr = SocketManager::new();
        assert!(mgr.recv(999, 10).is_none());
    }

    #[test]
    fn test_default() {
        let mgr = SocketManager::default();
        assert!(mgr.get_socket(3).is_none());
    }
}
