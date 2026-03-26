/**
 * # Network Stack Tests
 *
 * These tests verify every layer of the network stack, from Ethernet frames
 * at the bottom to HTTP at the top. Each test group corresponds to one layer
 * of the TCP/IP model.
 */

import { describe, it, expect } from "vitest";
import {
  EthernetFrame,
  ARPTable,
  IPv4Header,
  RoutingTable,
  IPLayer,
  TCPState,
  TCPHeader,
  TCPConnection,
  TCP_SYN,
  TCP_ACK,
  TCP_FIN,
  TCP_PSH,
  TCP_RST,
  UDPHeader,
  UDPSocket,
  SocketType,
  Socket,
  SocketManager,
  DNSResolver,
  HTTPRequest,
  HTTPResponse,
  HTTPClient,
  NetworkWire,
} from "../src/index.js";

// =============================================================================
// Layer 2: Ethernet Tests
// =============================================================================

describe("EthernetFrame", () => {
  it("should serialize and deserialize a frame (round-trip)", () => {
    /**
     * The most fundamental test: create a frame, serialize it to bytes,
     * deserialize it back, and verify every field matches. This ensures
     * our wire format is self-consistent.
     */
    const dest = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];
    const src = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
    const payload = [0x01, 0x02, 0x03, 0x04];
    const frame = new EthernetFrame(dest, src, 0x0800, payload);

    const bytes = frame.serialize();
    const restored = EthernetFrame.deserialize(bytes);

    expect(restored.dest_mac).toEqual(dest);
    expect(restored.src_mac).toEqual(src);
    expect(restored.ether_type).toBe(0x0800);
    expect(restored.payload).toEqual(payload);
  });

  it("should handle ARP ether_type (0x0806)", () => {
    const frame = new EthernetFrame(
      [0xff, 0xff, 0xff, 0xff, 0xff, 0xff], // broadcast
      [0x11, 0x22, 0x33, 0x44, 0x55, 0x66],
      0x0806, // ARP
      [0x00, 0x01],
    );
    const bytes = frame.serialize();
    const restored = EthernetFrame.deserialize(bytes);
    expect(restored.ether_type).toBe(0x0806);
  });

  it("should serialize correct byte count", () => {
    const frame = new EthernetFrame(
      [0, 0, 0, 0, 0, 0],
      [0, 0, 0, 0, 0, 0],
      0x0800,
      [1, 2, 3],
    );
    // 6 (dest) + 6 (src) + 2 (type) + 3 (payload) = 17 bytes
    expect(frame.serialize().length).toBe(17);
  });

  it("should handle empty payload", () => {
    const frame = new EthernetFrame(
      [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff],
      [0x11, 0x22, 0x33, 0x44, 0x55, 0x66],
      0x0800,
      [],
    );
    const bytes = frame.serialize();
    expect(bytes.length).toBe(14); // header only
    const restored = EthernetFrame.deserialize(bytes);
    expect(restored.payload).toEqual([]);
  });
});

describe("ARPTable", () => {
  it("should insert and look up entries", () => {
    const arp = new ARPTable();
    const mac = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];
    arp.insert("10.0.0.1", mac);

    expect(arp.lookup("10.0.0.1")).toEqual(mac);
    expect(arp.size()).toBe(1);
  });

  it("should return undefined for unknown IPs", () => {
    const arp = new ARPTable();
    expect(arp.lookup("10.0.0.99")).toBeUndefined();
  });

  it("should update existing entries", () => {
    const arp = new ARPTable();
    arp.insert("10.0.0.1", [0x11, 0x11, 0x11, 0x11, 0x11, 0x11]);
    arp.insert("10.0.0.1", [0x22, 0x22, 0x22, 0x22, 0x22, 0x22]);
    expect(arp.lookup("10.0.0.1")).toEqual([0x22, 0x22, 0x22, 0x22, 0x22, 0x22]);
    expect(arp.size()).toBe(1);
  });

  it("should store multiple entries", () => {
    const arp = new ARPTable();
    arp.insert("10.0.0.1", [0x11, 0x11, 0x11, 0x11, 0x11, 0x11]);
    arp.insert("10.0.0.2", [0x22, 0x22, 0x22, 0x22, 0x22, 0x22]);
    expect(arp.size()).toBe(2);
    expect(arp.lookup("10.0.0.1")).toEqual([0x11, 0x11, 0x11, 0x11, 0x11, 0x11]);
    expect(arp.lookup("10.0.0.2")).toEqual([0x22, 0x22, 0x22, 0x22, 0x22, 0x22]);
  });
});

// =============================================================================
// Layer 3: IP Tests
// =============================================================================

describe("IPv4Header", () => {
  it("should serialize and deserialize (round-trip)", () => {
    const header = new IPv4Header([10, 0, 0, 1], [10, 0, 0, 2], 6, 40, 64);
    const bytes = header.serialize();
    expect(bytes.length).toBe(20);

    const restored = IPv4Header.deserialize(bytes);
    expect(restored.version).toBe(4);
    expect(restored.ihl).toBe(5);
    expect(restored.total_length).toBe(40);
    expect(restored.ttl).toBe(64);
    expect(restored.protocol).toBe(6);
    expect(restored.src_ip).toEqual([10, 0, 0, 1]);
    expect(restored.dst_ip).toEqual([10, 0, 0, 2]);
  });

  it("should compute and verify checksum", () => {
    /**
     * The checksum should be non-zero after computation, and
     * verify_checksum() should return true when the checksum is valid.
     */
    const header = new IPv4Header([10, 0, 0, 1], [10, 0, 0, 2], 6, 40);
    header.header_checksum = header.compute_checksum();
    expect(header.header_checksum).not.toBe(0);
    expect(header.verify_checksum()).toBe(true);
  });

  it("should detect corrupted headers via checksum", () => {
    /**
     * If we compute the checksum, then modify a field, the checksum
     * should no longer verify. This is the whole point of checksums —
     * detecting bit flips during transmission.
     */
    const header = new IPv4Header([10, 0, 0, 1], [10, 0, 0, 2], 6, 40);
    header.header_checksum = header.compute_checksum();
    header.ttl = 32; // Corrupt a field
    expect(header.verify_checksum()).toBe(false);
  });

  it("should handle UDP protocol number", () => {
    const header = new IPv4Header([192, 168, 1, 1], [192, 168, 1, 2], 17, 28);
    const bytes = header.serialize();
    const restored = IPv4Header.deserialize(bytes);
    expect(restored.protocol).toBe(17);
  });

  it("should handle different TTL values", () => {
    const header = new IPv4Header([10, 0, 0, 1], [10, 0, 0, 2], 6, 40, 128);
    expect(header.ttl).toBe(128);
    const bytes = header.serialize();
    const restored = IPv4Header.deserialize(bytes);
    expect(restored.ttl).toBe(128);
  });
});

describe("RoutingTable", () => {
  it("should match a route", () => {
    const rt = new RoutingTable();
    rt.add_route([10, 0, 0, 0], [255, 255, 255, 0], [0, 0, 0, 0], "eth0");

    const route = rt.lookup([10, 0, 0, 5]);
    expect(route).toBeDefined();
    expect(route!.iface).toBe("eth0");
  });

  it("should return undefined for no match", () => {
    const rt = new RoutingTable();
    rt.add_route([10, 0, 0, 0], [255, 255, 255, 0], [0, 0, 0, 0], "eth0");
    expect(rt.lookup([192, 168, 1, 1])).toBeUndefined();
  });

  it("should select the longest prefix match", () => {
    /**
     * This is the core routing algorithm. When multiple routes match,
     * the most specific one wins. A /24 mask (255.255.255.0) is more
     * specific than a /8 mask (255.0.0.0).
     */
    const rt = new RoutingTable();
    rt.add_route([10, 0, 0, 0], [255, 0, 0, 0], [10, 0, 0, 1], "eth0");
    rt.add_route([10, 0, 1, 0], [255, 255, 255, 0], [10, 0, 1, 1], "eth1");

    const route = rt.lookup([10, 0, 1, 5]);
    expect(route).toBeDefined();
    expect(route!.iface).toBe("eth1");
    expect(route!.gateway).toEqual([10, 0, 1, 1]);
  });

  it("should match default route (0.0.0.0/0)", () => {
    const rt = new RoutingTable();
    rt.add_route([0, 0, 0, 0], [0, 0, 0, 0], [10, 0, 0, 1], "eth0");
    const route = rt.lookup([8, 8, 8, 8]);
    expect(route).toBeDefined();
    expect(route!.gateway).toEqual([10, 0, 0, 1]);
  });
});

describe("IPLayer", () => {
  it("should create and parse a packet", () => {
    const ip = new IPLayer([10, 0, 0, 1]);
    const payload = [0x01, 0x02, 0x03];
    const packet = ip.create_packet([10, 0, 0, 2], 6, payload);

    // 20-byte header + 3-byte payload
    expect(packet.length).toBe(23);

    const parsed = ip.parse_packet(packet);
    expect(parsed).toBeDefined();
    expect(parsed!.src_ip).toEqual([10, 0, 0, 1]);
    expect(parsed!.protocol).toBe(6);
    expect(parsed!.payload).toEqual(payload);
  });

  it("should reject packets with invalid checksum", () => {
    const ip = new IPLayer([10, 0, 0, 1]);
    const packet = ip.create_packet([10, 0, 0, 2], 6, [0x01]);
    // Corrupt a byte in the header
    packet[8] = packet[8] ^ 0xff;
    const parsed = ip.parse_packet(packet);
    expect(parsed).toBeUndefined();
  });

  it("should set up routing and ARP tables", () => {
    const ip = new IPLayer([10, 0, 0, 1]);
    ip.routing_table.add_route(
      [10, 0, 0, 0], [255, 255, 255, 0], [0, 0, 0, 0], "eth0",
    );
    ip.arp_table.insert("10.0.0.2", [0xbb, 0xbb, 0xbb, 0xbb, 0xbb, 0xbb]);

    const route = ip.routing_table.lookup([10, 0, 0, 2]);
    expect(route).toBeDefined();
    expect(ip.arp_table.lookup("10.0.0.2")).toBeDefined();
  });
});

// =============================================================================
// Layer 4: TCP Tests
// =============================================================================

describe("TCPHeader", () => {
  it("should serialize and deserialize (round-trip)", () => {
    const header = new TCPHeader(49152, 80, 1000, 0, TCP_SYN);
    const bytes = header.serialize();
    expect(bytes.length).toBe(20);

    const restored = TCPHeader.deserialize(bytes);
    expect(restored.src_port).toBe(49152);
    expect(restored.dst_port).toBe(80);
    expect(restored.seq_num).toBe(1000);
    expect(restored.ack_num).toBe(0);
    expect(restored.flags).toBe(TCP_SYN);
    expect(restored.window_size).toBe(65535);
    expect(restored.data_offset).toBe(5);
  });

  it("should handle SYN+ACK flags", () => {
    const header = new TCPHeader(80, 49152, 3000, 1001, TCP_SYN | TCP_ACK);
    const bytes = header.serialize();
    const restored = TCPHeader.deserialize(bytes);
    expect(restored.flags).toBe(TCP_SYN | TCP_ACK);
    expect(restored.seq_num).toBe(3000);
    expect(restored.ack_num).toBe(1001);
  });

  it("should handle large sequence numbers", () => {
    const header = new TCPHeader(80, 80, 0xfffffffe, 0xffffffff, TCP_ACK);
    const bytes = header.serialize();
    const restored = TCPHeader.deserialize(bytes);
    expect(restored.seq_num).toBe(0xfffffffe);
    expect(restored.ack_num).toBe(0xffffffff);
  });

  it("should handle all flag combinations", () => {
    const all_flags = TCP_FIN | TCP_SYN | TCP_RST | TCP_PSH | TCP_ACK;
    const header = new TCPHeader(1000, 2000, 0, 0, all_flags);
    const bytes = header.serialize();
    const restored = TCPHeader.deserialize(bytes);
    expect(restored.flags).toBe(all_flags);
  });
});

describe("TCPConnection — Three-Way Handshake", () => {
  it("should complete a three-way handshake", () => {
    /**
     * The three-way handshake is how TCP connections are established:
     *   Client → SYN → Server
     *   Server → SYN+ACK → Client
     *   Client → ACK → Server
     *
     * After this exchange, both sides are in ESTABLISHED state.
     */
    const client = new TCPConnection(49152, 80, "10.0.0.1", "10.0.0.2");
    const server = new TCPConnection(80, 0, "10.0.0.2", "10.0.0.1");

    // Server starts listening
    server.listen();
    expect(server.state).toBe(TCPState.LISTEN);

    // Client sends SYN
    const syn = client.initiate_connect();
    expect(syn).toBeDefined();
    expect(syn!.flags & TCP_SYN).toBeTruthy();
    expect(client.state).toBe(TCPState.SYN_SENT);

    // Server receives SYN, sends SYN+ACK
    const syn_ack = server.handle_segment(syn!);
    expect(syn_ack).toBeDefined();
    expect(syn_ack!.flags & TCP_SYN).toBeTruthy();
    expect(syn_ack!.flags & TCP_ACK).toBeTruthy();
    expect(server.state).toBe(TCPState.SYN_RECEIVED);

    // Client receives SYN+ACK, sends ACK
    const ack = client.handle_segment(syn_ack!);
    expect(ack).toBeDefined();
    expect(ack!.flags & TCP_ACK).toBeTruthy();
    expect(client.state).toBe(TCPState.ESTABLISHED);

    // Server receives ACK
    server.handle_segment(ack!);
    expect(server.state).toBe(TCPState.ESTABLISHED);
  });

  it("should not connect from non-CLOSED state", () => {
    const conn = new TCPConnection(49152, 80);
    conn.state = TCPState.ESTABLISHED;
    expect(conn.initiate_connect()).toBeUndefined();
  });
});

describe("TCPConnection — Data Transfer", () => {
  /**
   * Helper to set up two connected TCP endpoints.
   */
  function setup_connected(): { client: TCPConnection; server: TCPConnection } {
    const client = new TCPConnection(49152, 80, "10.0.0.1", "10.0.0.2");
    const server = new TCPConnection(80, 0, "10.0.0.2", "10.0.0.1");
    server.listen();
    const syn = client.initiate_connect()!;
    const syn_ack = server.handle_segment(syn)!;
    const ack = client.handle_segment(syn_ack)!;
    server.handle_segment(ack);
    return { client, server };
  }

  it("should send and receive data", () => {
    const { client, server } = setup_connected();

    // Client sends data
    const data = [72, 101, 108, 108, 111]; // "Hello"
    const segment = client.send(data);
    expect(segment).toBeDefined();
    expect(segment!.data).toEqual(data);

    // Server receives the data segment
    const ack = server.handle_segment(segment!.header, segment!.data);
    expect(ack).toBeDefined();

    // Server reads the data from its buffer
    const received = server.recv();
    expect(received).toEqual(data);
  });

  it("should advance sequence numbers", () => {
    const { client } = setup_connected();

    const before_seq = client.send_seq;
    client.send([1, 2, 3, 4, 5]);
    expect(client.send_seq).toBe(before_seq + 5);
  });

  it("should not send in non-ESTABLISHED state", () => {
    const conn = new TCPConnection(49152, 80);
    expect(conn.send([1, 2, 3])).toBeUndefined();
  });

  it("should return empty array when recv buffer is empty", () => {
    const { server } = setup_connected();
    expect(server.recv()).toEqual([]);
  });
});

describe("TCPConnection — Connection Close", () => {
  function setup_connected(): { client: TCPConnection; server: TCPConnection } {
    const client = new TCPConnection(49152, 80, "10.0.0.1", "10.0.0.2");
    const server = new TCPConnection(80, 0, "10.0.0.2", "10.0.0.1");
    server.listen();
    const syn = client.initiate_connect()!;
    const syn_ack = server.handle_segment(syn)!;
    const ack = client.handle_segment(syn_ack)!;
    server.handle_segment(ack);
    return { client, server };
  }

  it("should perform four-way close", () => {
    /**
     * Connection teardown:
     *   Client → FIN → Server  (client done sending)
     *   Server → ACK → Client  (server acknowledges)
     *   Server → FIN → Client  (server done sending)
     *   Client → ACK → Server  (client acknowledges)
     */
    const { client, server } = setup_connected();

    // Client initiates close
    const fin = client.initiate_close();
    expect(fin).toBeDefined();
    expect(fin!.flags & TCP_FIN).toBeTruthy();
    expect(client.state).toBe(TCPState.FIN_WAIT_1);

    // Server receives FIN, sends ACK
    const fin_ack = server.handle_segment(fin!);
    expect(fin_ack).toBeDefined();
    expect(server.state).toBe(TCPState.CLOSE_WAIT);

    // Client receives ACK
    client.handle_segment(fin_ack!);
    expect(client.state).toBe(TCPState.FIN_WAIT_2);

    // Server sends its FIN
    const server_fin = server.initiate_close();
    expect(server_fin).toBeDefined();
    expect(server.state).toBe(TCPState.LAST_ACK);

    // Client receives server's FIN, sends ACK
    const final_ack = client.handle_segment(server_fin!);
    expect(final_ack).toBeDefined();
    expect(client.state).toBe(TCPState.TIME_WAIT);

    // Server receives final ACK
    server.handle_segment(final_ack!);
    expect(server.state).toBe(TCPState.CLOSED);
  });

  it("should not close from CLOSED state", () => {
    const conn = new TCPConnection();
    expect(conn.initiate_close()).toBeUndefined();
  });
});

// =============================================================================
// Layer 4: UDP Tests
// =============================================================================

describe("UDPHeader", () => {
  it("should serialize and deserialize (round-trip)", () => {
    const header = new UDPHeader(12345, 53, 20, 0);
    const bytes = header.serialize();
    expect(bytes.length).toBe(8);

    const restored = UDPHeader.deserialize(bytes);
    expect(restored.src_port).toBe(12345);
    expect(restored.dst_port).toBe(53);
    expect(restored.length).toBe(20);
    expect(restored.checksum).toBe(0);
  });

  it("should handle non-zero checksum", () => {
    const header = new UDPHeader(1000, 2000, 16, 0xabcd);
    const bytes = header.serialize();
    const restored = UDPHeader.deserialize(bytes);
    expect(restored.checksum).toBe(0xabcd);
  });
});

describe("UDPSocket", () => {
  it("should send_to and produce correct header", () => {
    const sock = new UDPSocket(12345);
    const result = sock.send_to([1, 2, 3], 53);
    expect(result.header.src_port).toBe(12345);
    expect(result.header.dst_port).toBe(53);
    expect(result.header.length).toBe(11); // 8 header + 3 data
    expect(result.data).toEqual([1, 2, 3]);
  });

  it("should deliver and receive_from datagrams", () => {
    const sock = new UDPSocket(53);
    sock.deliver([10, 20, 30], "10.0.0.1", 12345);

    const result = sock.receive_from();
    expect(result).toBeDefined();
    expect(result!.data).toEqual([10, 20, 30]);
    expect(result!.src_ip).toBe("10.0.0.1");
    expect(result!.src_port).toBe(12345);
  });

  it("should return undefined when queue is empty", () => {
    const sock = new UDPSocket(53);
    expect(sock.receive_from()).toBeUndefined();
  });

  it("should handle multiple datagrams in order", () => {
    const sock = new UDPSocket(53);
    sock.deliver([1], "10.0.0.1", 1000);
    sock.deliver([2], "10.0.0.2", 2000);

    const first = sock.receive_from();
    expect(first!.data).toEqual([1]);
    expect(first!.src_port).toBe(1000);

    const second = sock.receive_from();
    expect(second!.data).toEqual([2]);
    expect(second!.src_port).toBe(2000);
  });
});

// =============================================================================
// Socket API Tests
// =============================================================================

describe("Socket", () => {
  it("should create TCP socket", () => {
    const sock = new Socket(3, SocketType.STREAM);
    expect(sock.fd).toBe(3);
    expect(sock.socket_type).toBe(SocketType.STREAM);
    expect(sock.tcp_connection).toBeDefined();
    expect(sock.udp_socket).toBeUndefined();
  });

  it("should create UDP socket", () => {
    const sock = new Socket(4, SocketType.DGRAM);
    expect(sock.fd).toBe(4);
    expect(sock.socket_type).toBe(SocketType.DGRAM);
    expect(sock.tcp_connection).toBeUndefined();
    expect(sock.udp_socket).toBeDefined();
  });

  it("should have default values", () => {
    const sock = new Socket(5, SocketType.STREAM);
    expect(sock.local_ip).toBe("0.0.0.0");
    expect(sock.local_port).toBe(0);
    expect(sock.is_listening).toBe(false);
    expect(sock.accept_queue).toEqual([]);
  });
});

describe("SocketManager", () => {
  it("should create sockets with unique fds", () => {
    const mgr = new SocketManager();
    const fd1 = mgr.socket(SocketType.STREAM);
    const fd2 = mgr.socket(SocketType.DGRAM);
    expect(fd1).not.toBe(fd2);
    expect(fd1).toBeGreaterThanOrEqual(3);
  });

  it("should bind to a port", () => {
    const mgr = new SocketManager();
    const fd = mgr.socket(SocketType.STREAM);
    expect(mgr.bind(fd, "10.0.0.1", 80)).toBe(true);

    const sock = mgr.get_socket(fd);
    expect(sock!.local_ip).toBe("10.0.0.1");
    expect(sock!.local_port).toBe(80);
  });

  it("should reject binding to an already-used port", () => {
    const mgr = new SocketManager();
    const fd1 = mgr.socket(SocketType.STREAM);
    const fd2 = mgr.socket(SocketType.STREAM);
    mgr.bind(fd1, "10.0.0.1", 80);
    expect(mgr.bind(fd2, "10.0.0.2", 80)).toBe(false);
  });

  it("should listen on a TCP socket", () => {
    const mgr = new SocketManager();
    const fd = mgr.socket(SocketType.STREAM);
    mgr.bind(fd, "10.0.0.1", 80);
    expect(mgr.listen(fd)).toBe(true);

    const sock = mgr.get_socket(fd);
    expect(sock!.is_listening).toBe(true);
  });

  it("should not listen on a UDP socket", () => {
    const mgr = new SocketManager();
    const fd = mgr.socket(SocketType.DGRAM);
    expect(mgr.listen(fd)).toBe(false);
  });

  it("should connect and return SYN", () => {
    const mgr = new SocketManager();
    const fd = mgr.socket(SocketType.STREAM);
    mgr.bind(fd, "10.0.0.1", 49152);
    const syn = mgr.connect(fd, "10.0.0.2", 80);
    expect(syn).toBeDefined();
    expect(syn!.flags & TCP_SYN).toBeTruthy();
  });

  it("should send and recv TCP data", () => {
    const mgr = new SocketManager();
    const fd = mgr.socket(SocketType.STREAM);
    mgr.bind(fd, "10.0.0.1", 49152);
    mgr.connect(fd, "10.0.0.2", 80);

    // Manually establish the connection for testing
    const sock = mgr.get_socket(fd)!;
    sock.tcp_connection!.state = TCPState.ESTABLISHED;
    sock.tcp_connection!.send_seq = 1001;

    const segment = mgr.send(fd, [72, 101, 108, 108, 111]);
    expect(segment).toBeDefined();

    // Recv on empty buffer
    const data = mgr.recv(fd);
    expect(data).toEqual([]);
  });

  it("should sendto and recvfrom UDP", () => {
    const mgr = new SocketManager();
    const fd = mgr.socket(SocketType.DGRAM);
    mgr.bind(fd, "10.0.0.1", 12345);

    const result = mgr.sendto(fd, [1, 2, 3], 53);
    expect(result).toBeDefined();

    // Deliver a datagram manually
    const sock = mgr.get_socket(fd)!;
    sock.udp_socket!.deliver([4, 5, 6], "10.0.0.2", 53);

    const received = mgr.recvfrom(fd);
    expect(received).toBeDefined();
    expect(received!.data).toEqual([4, 5, 6]);
  });

  it("should close and free port", () => {
    const mgr = new SocketManager();
    const fd = mgr.socket(SocketType.STREAM);
    mgr.bind(fd, "10.0.0.1", 80);
    mgr.close(fd);

    // Port should be free again
    const fd2 = mgr.socket(SocketType.STREAM);
    expect(mgr.bind(fd2, "10.0.0.1", 80)).toBe(true);
  });

  it("should return undefined for invalid fd", () => {
    const mgr = new SocketManager();
    expect(mgr.bind(999, "10.0.0.1", 80)).toBe(false);
    expect(mgr.listen(999)).toBe(false);
    expect(mgr.connect(999, "10.0.0.2", 80)).toBeUndefined();
    expect(mgr.send(999, [1])).toBeUndefined();
    expect(mgr.recv(999)).toBeUndefined();
    expect(mgr.sendto(999, [1], 80)).toBeUndefined();
    expect(mgr.recvfrom(999)).toBeUndefined();
    expect(mgr.close(999)).toBeUndefined();
  });

  it("should accept incoming connections", () => {
    const mgr = new SocketManager();
    const server_fd = mgr.socket(SocketType.STREAM);
    mgr.bind(server_fd, "10.0.0.2", 80);
    mgr.listen(server_fd);

    // Simulate an incoming connection in the accept queue
    const sock = mgr.get_socket(server_fd)!;
    const conn = new TCPConnection(80, 49152, "10.0.0.2", "10.0.0.1");
    conn.state = TCPState.ESTABLISHED;
    sock.accept_queue.push(conn);

    const new_fd = mgr.accept(server_fd);
    expect(new_fd).toBeDefined();

    const new_sock = mgr.get_socket(new_fd!);
    expect(new_sock).toBeDefined();
    expect(new_sock!.remote_ip).toBe("10.0.0.1");
    expect(new_sock!.remote_port).toBe(49152);
  });

  it("should return undefined when accept queue is empty", () => {
    const mgr = new SocketManager();
    const fd = mgr.socket(SocketType.STREAM);
    mgr.bind(fd, "10.0.0.2", 80);
    mgr.listen(fd);
    expect(mgr.accept(fd)).toBeUndefined();
  });

  it("should not accept on non-listening socket", () => {
    const mgr = new SocketManager();
    const fd = mgr.socket(SocketType.STREAM);
    expect(mgr.accept(fd)).toBeUndefined();
  });

  it("should return FIN header when closing established TCP", () => {
    const mgr = new SocketManager();
    const fd = mgr.socket(SocketType.STREAM);
    mgr.bind(fd, "10.0.0.1", 49152);

    // Manually establish
    const sock = mgr.get_socket(fd)!;
    sock.tcp_connection!.state = TCPState.ESTABLISHED;
    sock.tcp_connection!.remote_port = 80;

    const fin = mgr.close(fd);
    expect(fin).toBeDefined();
    expect(fin!.flags & TCP_FIN).toBeTruthy();
  });
});

// =============================================================================
// Layer 7: DNS Tests
// =============================================================================

describe("DNSResolver", () => {
  it("should resolve localhost by default", () => {
    const dns = new DNSResolver();
    expect(dns.resolve("localhost")).toEqual([127, 0, 0, 1]);
  });

  it("should return undefined for unknown hostnames", () => {
    const dns = new DNSResolver();
    expect(dns.resolve("unknown.com")).toBeUndefined();
  });

  it("should add and resolve static entries", () => {
    const dns = new DNSResolver();
    dns.add_static("example.com", [93, 184, 216, 34]);
    expect(dns.resolve("example.com")).toEqual([93, 184, 216, 34]);
  });

  it("should overwrite existing entries", () => {
    const dns = new DNSResolver();
    dns.add_static("example.com", [1, 1, 1, 1]);
    dns.add_static("example.com", [2, 2, 2, 2]);
    expect(dns.resolve("example.com")).toEqual([2, 2, 2, 2]);
  });
});

// =============================================================================
// Layer 7: HTTP Tests
// =============================================================================

describe("HTTPRequest", () => {
  it("should serialize a GET request", () => {
    const headers = new Map<string, string>();
    headers.set("Host", "example.com");
    const req = new HTTPRequest("GET", "/index.html", headers);

    const text = req.serialize();
    expect(text).toContain("GET /index.html HTTP/1.1\r\n");
    expect(text).toContain("Host: example.com\r\n");
    expect(text).toContain("\r\n\r\n"); // empty line after headers
  });

  it("should serialize a POST request with body", () => {
    const headers = new Map<string, string>();
    headers.set("Content-Type", "text/plain");
    headers.set("Content-Length", "5");
    const req = new HTTPRequest("POST", "/api", headers, "hello");

    const text = req.serialize();
    expect(text).toContain("POST /api HTTP/1.1\r\n");
    expect(text).toContain("hello");
  });

  it("should deserialize a GET request", () => {
    const text = "GET /path HTTP/1.1\r\nHost: example.com\r\n\r\n";
    const req = HTTPRequest.deserialize(text);
    expect(req.method).toBe("GET");
    expect(req.path).toBe("/path");
    expect(req.headers.get("Host")).toBe("example.com");
  });

  it("should round-trip serialize/deserialize", () => {
    const headers = new Map<string, string>();
    headers.set("Host", "example.com");
    const original = new HTTPRequest("GET", "/", headers);

    const restored = HTTPRequest.deserialize(original.serialize());
    expect(restored.method).toBe("GET");
    expect(restored.path).toBe("/");
    expect(restored.headers.get("Host")).toBe("example.com");
  });

  it("should handle request with body", () => {
    const text =
      "POST /data HTTP/1.1\r\nContent-Length: 3\r\n\r\nabc";
    const req = HTTPRequest.deserialize(text);
    expect(req.method).toBe("POST");
    expect(req.body).toBe("abc");
  });
});

describe("HTTPResponse", () => {
  it("should serialize a 200 OK response", () => {
    const headers = new Map<string, string>();
    headers.set("Content-Type", "text/html");
    const resp = new HTTPResponse(200, "OK", headers, "Hello, World!");

    const text = resp.serialize();
    expect(text).toContain("HTTP/1.1 200 OK\r\n");
    expect(text).toContain("Content-Type: text/html\r\n");
    expect(text).toContain("Hello, World!");
  });

  it("should deserialize a response", () => {
    const text =
      "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\n\r\nNot Found";
    const resp = HTTPResponse.deserialize(text);
    expect(resp.status_code).toBe(404);
    expect(resp.status_text).toBe("Not Found");
    expect(resp.body).toBe("Not Found");
    expect(resp.headers.get("Content-Type")).toBe("text/plain");
  });

  it("should round-trip serialize/deserialize", () => {
    const headers = new Map<string, string>();
    headers.set("Content-Length", "5");
    const original = new HTTPResponse(200, "OK", headers, "hello");

    const restored = HTTPResponse.deserialize(original.serialize());
    expect(restored.status_code).toBe(200);
    expect(restored.status_text).toBe("OK");
    expect(restored.body).toBe("hello");
  });

  it("should handle empty body", () => {
    const resp = new HTTPResponse(204, "No Content");
    const text = resp.serialize();
    const restored = HTTPResponse.deserialize(text);
    expect(restored.status_code).toBe(204);
    expect(restored.body).toBe("");
  });
});

describe("HTTPClient", () => {
  it("should build a request from a URL", () => {
    const client = new HTTPClient();
    const { request, host, port } = client.build_request(
      "http://example.com/path",
    );
    expect(request.method).toBe("GET");
    expect(request.path).toBe("/path");
    expect(request.headers.get("Host")).toBe("example.com");
    expect(host).toBe("example.com");
    expect(port).toBe(80);
  });

  it("should handle URL with port", () => {
    const client = new HTTPClient();
    const { host, port } = client.build_request("http://example.com:8080/api");
    expect(host).toBe("example.com");
    expect(port).toBe(8080);
  });

  it("should default to / path", () => {
    const client = new HTTPClient();
    const { request } = client.build_request("http://example.com");
    expect(request.path).toBe("/");
  });

  it("should parse an HTTP response", () => {
    const client = new HTTPClient();
    const text =
      "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html></html>";
    const resp = client.parse_response(text);
    expect(resp.status_code).toBe(200);
    expect(resp.body).toBe("<html></html>");
  });

  it("should use DNS resolver", () => {
    const dns = new DNSResolver();
    dns.add_static("example.com", [93, 184, 216, 34]);
    const client = new HTTPClient(dns);
    expect(client.dns.resolve("example.com")).toEqual([93, 184, 216, 34]);
  });
});

// =============================================================================
// NetworkWire Tests
// =============================================================================

describe("NetworkWire", () => {
  it("should deliver data from A to B", () => {
    const wire = new NetworkWire();
    wire.send_a([1, 2, 3]);

    expect(wire.has_data_for_b()).toBe(true);
    expect(wire.has_data_for_a()).toBe(false);

    const data = wire.receive_b();
    expect(data).toEqual([1, 2, 3]);
  });

  it("should deliver data from B to A", () => {
    const wire = new NetworkWire();
    wire.send_b([4, 5, 6]);

    expect(wire.has_data_for_a()).toBe(true);
    expect(wire.has_data_for_b()).toBe(false);

    const data = wire.receive_a();
    expect(data).toEqual([4, 5, 6]);
  });

  it("should handle bidirectional communication", () => {
    /**
     * Both sides can send and receive simultaneously. This simulates
     * full-duplex Ethernet, where data flows in both directions at once.
     */
    const wire = new NetworkWire();
    wire.send_a([1, 2, 3]);
    wire.send_b([4, 5, 6]);

    expect(wire.receive_b()).toEqual([1, 2, 3]);
    expect(wire.receive_a()).toEqual([4, 5, 6]);
  });

  it("should return undefined when no data available", () => {
    const wire = new NetworkWire();
    expect(wire.receive_a()).toBeUndefined();
    expect(wire.receive_b()).toBeUndefined();
  });

  it("should preserve order (FIFO)", () => {
    const wire = new NetworkWire();
    wire.send_a([1]);
    wire.send_a([2]);
    wire.send_a([3]);

    expect(wire.receive_b()).toEqual([1]);
    expect(wire.receive_b()).toEqual([2]);
    expect(wire.receive_b()).toEqual([3]);
  });

  it("should make defensive copies of data", () => {
    /**
     * The wire should copy data so that modifying the original array
     * after sending does not corrupt what the receiver gets.
     */
    const wire = new NetworkWire();
    const original = [1, 2, 3];
    wire.send_a(original);
    original[0] = 99; // Modify after sending
    expect(wire.receive_b()).toEqual([1, 2, 3]); // Should be original
  });
});
