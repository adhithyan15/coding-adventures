/**
 * # Network Stack — From Ethernet Frames to HTTP Requests
 *
 * This module implements a complete networking stack, covering every layer of
 * the TCP/IP model:
 *
 * ```
 *   Layer 7: HTTP         — "What are we saying?"
 *   Layer 7: DNS          — "What is the IP for this hostname?"
 *   Layer 4: TCP          — "How do we ensure reliable delivery?"
 *   Layer 4: UDP          — "How do we send fast, unreliable datagrams?"
 *   Layer 3: IP           — "How do we route across networks?"
 *   Layer 2: Ethernet     — "How do we talk to the next hop?"
 *   Layer 1: NetworkWire  — "How do we transmit bits?" (simulated)
 * ```
 *
 * ## The Postal Analogy
 *
 * Sending data over a network is like sending a letter:
 * - **Ethernet** is the local mail carrier — delivers between houses on the same street.
 * - **IP** is the postal routing system — figures out which city and post office.
 * - **TCP** is registered mail with tracking — guarantees delivery and order.
 * - **UDP** is a postcard — fast, no guarantee it arrives.
 * - **HTTP** is the letter itself — "Dear Server, please send me the homepage."
 *
 * ## Packet Encapsulation
 *
 * As data moves down the stack, each layer wraps the data from the layer above
 * in its own header. On the receiving side, each layer strips its header and
 * passes the payload up. This is the fundamental principle of layered networking.
 *
 * ```
 *   Application:  "Hello, World!"
 *   HTTP layer:   GET / HTTP/1.1\r\nHost: example.com\r\n\r\nHello, World!
 *   TCP layer:    [TCP Header] + HTTP data
 *   IP layer:     [IP Header] + TCP segment
 *   Ethernet:     [Eth Header] + IP packet
 *   Wire:         raw bytes
 * ```
 */

// =============================================================================
// LAYER 2: ETHERNET — Local Delivery
// =============================================================================

/**
 * ## Ethernet Frames — The Envelope for Local Delivery
 *
 * Every network interface card (NIC) has a unique 48-bit MAC address burned
 * in at the factory. Ethernet frames carry data between devices on the same
 * local network segment.
 *
 * A MAC address is 6 bytes, like: [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF].
 * The broadcast address [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] reaches every
 * device on the local network.
 *
 * The `ether_type` field tells the receiver how to interpret the payload:
 *   - 0x0800 = IPv4 (an IP packet is inside)
 *   - 0x0806 = ARP  (an address resolution request/reply is inside)
 */
export class EthernetFrame {
  dest_mac: number[];
  src_mac: number[];
  ether_type: number;
  payload: number[];

  constructor(
    dest_mac: number[],
    src_mac: number[],
    ether_type: number,
    payload: number[],
  ) {
    this.dest_mac = dest_mac;
    this.src_mac = src_mac;
    this.ether_type = ether_type;
    this.payload = payload;
  }

  /**
   * Serialize the frame to bytes for transmission over the wire.
   *
   * Wire format:
   * ```
   *   [6 bytes dest_mac][6 bytes src_mac][2 bytes ether_type][N bytes payload]
   * ```
   *
   * The ether_type is stored in big-endian (network byte order), which is the
   * standard for all network protocols. Big-endian means the most significant
   * byte comes first — like how we write numbers (1234 = 1*1000 + 2*100 + ...).
   */
  serialize(): number[] {
    const bytes: number[] = [];
    bytes.push(...this.dest_mac);
    bytes.push(...this.src_mac);
    // Ether type in big-endian (network byte order)
    bytes.push((this.ether_type >> 8) & 0xff);
    bytes.push(this.ether_type & 0xff);
    bytes.push(...this.payload);
    return bytes;
  }

  /**
   * Deserialize bytes received from the wire back into a structured frame.
   *
   * This is the reverse of serialize(). We read the fixed-size header fields
   * first (6 + 6 + 2 = 14 bytes), then everything remaining is the payload.
   */
  static deserialize(bytes: number[]): EthernetFrame {
    const dest_mac = bytes.slice(0, 6);
    const src_mac = bytes.slice(6, 12);
    const ether_type = (bytes[12] << 8) | bytes[13];
    const payload = bytes.slice(14);
    return new EthernetFrame(dest_mac, src_mac, ether_type, payload);
  }
}

/**
 * ## ARP Table — Bridging IP Addresses and MAC Addresses
 *
 * When a computer wants to send an IP packet to 192.168.1.5, it needs the MAC
 * address of that device's network card. ARP (Address Resolution Protocol)
 * solves this by maintaining a table of IP-to-MAC mappings.
 *
 * The ARP process:
 * 1. Host A wants to send to Host B (10.0.0.2) but doesn't know B's MAC.
 * 2. A broadcasts: "Who has 10.0.0.2? Tell 10.0.0.1"
 * 3. B responds: "10.0.0.2 is at BB:BB:BB:BB:BB:BB"
 * 4. A stores the mapping and can now address frames to B.
 *
 * We represent IP addresses as strings ("10.0.0.1") and MAC addresses as
 * arrays of 6 bytes for simplicity in this educational implementation.
 */
export class ARPTable {
  private entries: Map<string, number[]> = new Map();

  /**
   * Look up the MAC address for a given IP. Returns undefined if unknown.
   * In a real network stack, a miss here would trigger an ARP broadcast.
   */
  lookup(ip: string): number[] | undefined {
    return this.entries.get(ip);
  }

  /** Learn a new IP-to-MAC mapping (or update an existing one). */
  insert(ip: string, mac: number[]): void {
    this.entries.set(ip, mac);
  }

  /** Return the number of entries in the ARP table. */
  size(): number {
    return this.entries.size;
  }
}

// =============================================================================
// LAYER 3: IP — Routing Across Networks
// =============================================================================

/**
 * ## IPv4 Header — The Routing Label
 *
 * IP (Internet Protocol) is the backbone of the Internet. Every device on the
 * Internet has an IP address, and IP headers tell routers where to send each
 * packet.
 *
 * The IPv4 header is exactly 20 bytes (when IHL=5, meaning no options):
 *
 * ```
 *   Byte 0:  version (4 bits) + IHL (4 bits)
 *   Byte 1:  Type of Service (we ignore this)
 *   Byte 2-3: Total Length (header + payload)
 *   Byte 4-5: Identification (we set to 0)
 *   Byte 6-7: Flags + Fragment Offset (we set to 0)
 *   Byte 8:  TTL (Time To Live)
 *   Byte 9:  Protocol (6=TCP, 17=UDP)
 *   Byte 10-11: Header Checksum
 *   Byte 12-15: Source IP Address
 *   Byte 16-19: Destination IP Address
 * ```
 *
 * The TTL field prevents packets from looping forever. Each router decrements
 * it by 1; when it hits 0, the packet is discarded. This is why traceroute
 * works — it sends packets with increasing TTL values and sees which router
 * sends back the "TTL exceeded" error.
 */
export class IPv4Header {
  version: number;
  ihl: number;
  total_length: number;
  ttl: number;
  protocol: number;
  header_checksum: number;
  src_ip: number[];
  dst_ip: number[];

  constructor(
    src_ip: number[],
    dst_ip: number[],
    protocol: number,
    total_length: number,
    ttl: number = 64,
  ) {
    this.version = 4;
    this.ihl = 5;
    this.total_length = total_length;
    this.ttl = ttl;
    this.protocol = protocol;
    this.header_checksum = 0;
    this.src_ip = src_ip;
    this.dst_ip = dst_ip;
  }

  /**
   * Serialize the IPv4 header to 20 bytes.
   *
   * The first byte packs both version and IHL into a single byte using bit
   * shifting: (version << 4) | ihl. Since version=4 and ihl=5, this gives
   * us 0x45 — the most common first byte in any IP packet on the Internet.
   */
  serialize(): number[] {
    const bytes: number[] = new Array(20).fill(0);
    bytes[0] = (this.version << 4) | this.ihl;
    bytes[1] = 0; // Type of Service
    bytes[2] = (this.total_length >> 8) & 0xff;
    bytes[3] = this.total_length & 0xff;
    // bytes[4-7]: identification, flags, fragment offset — all zero
    bytes[8] = this.ttl;
    bytes[9] = this.protocol;
    bytes[10] = (this.header_checksum >> 8) & 0xff;
    bytes[11] = this.header_checksum & 0xff;
    bytes[12] = this.src_ip[0];
    bytes[13] = this.src_ip[1];
    bytes[14] = this.src_ip[2];
    bytes[15] = this.src_ip[3];
    bytes[16] = this.dst_ip[0];
    bytes[17] = this.dst_ip[1];
    bytes[18] = this.dst_ip[2];
    bytes[19] = this.dst_ip[3];
    return bytes;
  }

  /**
   * Deserialize 20 bytes back into an IPv4 header.
   */
  static deserialize(bytes: number[]): IPv4Header {
    const version = (bytes[0] >> 4) & 0x0f;
    const ihl = bytes[0] & 0x0f;
    const total_length = (bytes[2] << 8) | bytes[3];
    const ttl = bytes[8];
    const protocol = bytes[9];
    const header_checksum = (bytes[10] << 8) | bytes[11];
    const src_ip = [bytes[12], bytes[13], bytes[14], bytes[15]];
    const dst_ip = [bytes[16], bytes[17], bytes[18], bytes[19]];

    const header = new IPv4Header(src_ip, dst_ip, protocol, total_length, ttl);
    header.version = version;
    header.ihl = ihl;
    header.header_checksum = header_checksum;
    return header;
  }

  /**
   * ## IP Checksum — Ones' Complement Sum
   *
   * The IP checksum catches accidental bit flips during transmission. It is
   * NOT a security hash — just a simple error-detection mechanism.
   *
   * Algorithm:
   * 1. Treat the header as a sequence of 16-bit words.
   * 2. Set the checksum field to 0 before computing.
   * 3. Sum all words. If the sum overflows 16 bits, add the carry back.
   * 4. Take the bitwise NOT (ones' complement) of the result.
   *
   * The beauty of ones' complement arithmetic: if you include the checksum
   * in the computation, a valid header always sums to 0xFFFF.
   */
  compute_checksum(): number {
    const saved = this.header_checksum;
    this.header_checksum = 0;
    const bytes = this.serialize();
    this.header_checksum = saved;

    let sum = 0;
    for (let i = 0; i < bytes.length; i += 2) {
      sum += (bytes[i] << 8) | bytes[i + 1];
    }
    // Fold carry bits back into the 16-bit sum
    while (sum > 0xffff) {
      sum = (sum & 0xffff) + (sum >> 16);
    }
    return (~sum & 0xffff);
  }

  /**
   * Verify the checksum of a received header. A valid header produces
   * a checksum of 0 when computed over all bytes including the checksum field.
   */
  verify_checksum(): boolean {
    const bytes = this.serialize();
    let sum = 0;
    for (let i = 0; i < bytes.length; i += 2) {
      sum += (bytes[i] << 8) | bytes[i + 1];
    }
    while (sum > 0xffff) {
      sum = (sum & 0xffff) + (sum >> 16);
    }
    return (sum & 0xffff) === 0xffff;
  }
}

/**
 * ## Routing Table — Finding the Path
 *
 * A routing table tells the IP layer where to send packets. Each entry says:
 * "If the destination matches this network/mask, send it to this gateway
 * via this interface."
 *
 * The key algorithm is **longest prefix match**: when multiple routes match,
 * pick the most specific one (the one with the longest subnet mask).
 *
 * Example:
 * ```
 *   Route 1: 10.0.0.0/8   → gateway 10.0.0.1   (matches any 10.x.x.x)
 *   Route 2: 10.0.1.0/24  → gateway 10.0.1.1   (matches 10.0.1.x only)
 *
 *   Destination 10.0.1.5 matches BOTH routes.
 *   Longest prefix match picks Route 2 (mask 255.255.255.0 is more specific
 *   than 255.0.0.0).
 * ```
 */
export interface RouteEntry {
  network: number[];
  mask: number[];
  gateway: number[];
  iface: string;
}

export class RoutingTable {
  entries: RouteEntry[] = [];

  /** Add a route to the table. */
  add_route(
    network: number[],
    mask: number[],
    gateway: number[],
    iface: string,
  ): void {
    this.entries.push({ network, mask, gateway, iface });
  }

  /**
   * Find the best route for a destination IP using longest prefix match.
   *
   * For each entry, we AND the destination IP with the entry's mask and
   * check if the result equals the entry's network address. Among all
   * matching entries, we pick the one with the most 1-bits in its mask
   * (the most specific route).
   */
  lookup(dst_ip: number[]): RouteEntry | undefined {
    let best: RouteEntry | undefined = undefined;
    let best_mask_bits = -1;

    for (const entry of this.entries) {
      // Check: (dst_ip & mask) === network?
      let matches = true;
      for (let i = 0; i < 4; i++) {
        if ((dst_ip[i] & entry.mask[i]) !== entry.network[i]) {
          matches = false;
          break;
        }
      }

      if (matches) {
        // Count mask bits for longest-prefix comparison
        const mask_bits = this.count_mask_bits(entry.mask);
        if (mask_bits > best_mask_bits) {
          best = entry;
          best_mask_bits = mask_bits;
        }
      }
    }

    return best;
  }

  /** Count the number of 1-bits in a subnet mask. */
  private count_mask_bits(mask: number[]): number {
    let count = 0;
    for (const byte of mask) {
      let b = byte;
      while (b > 0) {
        count += b & 1;
        b >>= 1;
      }
    }
    return count;
  }
}

/**
 * ## IP Layer — The Routing Engine
 *
 * The IP layer sits between Ethernet (below) and TCP/UDP (above). It handles:
 * - Building IP packets with correct headers and checksums
 * - Looking up routes to determine where to send packets
 * - Parsing incoming IP packets and passing payloads up to the transport layer
 */
export class IPLayer {
  local_ip: number[];
  routing_table: RoutingTable;
  arp_table: ARPTable;

  constructor(local_ip: number[]) {
    this.local_ip = local_ip;
    this.routing_table = new RoutingTable();
    this.arp_table = new ARPTable();
  }

  /**
   * Create an IP packet ready for transmission.
   * Returns the serialized IPv4 header + payload bytes.
   */
  create_packet(
    dst_ip: number[],
    protocol: number,
    payload: number[],
  ): number[] {
    const total_length = 20 + payload.length;
    const header = new IPv4Header(
      this.local_ip,
      dst_ip,
      protocol,
      total_length,
    );
    header.header_checksum = header.compute_checksum();
    const header_bytes = header.serialize();
    return [...header_bytes, ...payload];
  }

  /**
   * Parse a received IP packet. Returns the source IP, protocol, and payload.
   * Returns undefined if the checksum is invalid.
   */
  parse_packet(
    bytes: number[],
  ): { src_ip: number[]; protocol: number; payload: number[] } | undefined {
    const header = IPv4Header.deserialize(bytes.slice(0, 20));
    if (!header.verify_checksum()) {
      return undefined;
    }
    const payload = bytes.slice(20);
    return {
      src_ip: header.src_ip,
      protocol: header.protocol,
      payload,
    };
  }
}

// =============================================================================
// LAYER 4: TCP — Reliable, Ordered Delivery
// =============================================================================

/**
 * ## TCP States — The Connection Lifecycle
 *
 * A TCP connection is a state machine with 11 states. This is one of the most
 * important concepts in networking:
 *
 * ```
 *   CLOSED → SYN_SENT → ESTABLISHED → FIN_WAIT_1 → FIN_WAIT_2 → TIME_WAIT → CLOSED
 *   CLOSED → LISTEN → SYN_RECEIVED → ESTABLISHED → CLOSE_WAIT → LAST_ACK → CLOSED
 * ```
 *
 * The client path (top) initiates connections with SYN.
 * The server path (bottom) waits for connections with LISTEN.
 * Both paths meet at ESTABLISHED for data transfer, then diverge again
 * for connection teardown.
 */
export enum TCPState {
  CLOSED = 0,
  LISTEN = 1,
  SYN_SENT = 2,
  SYN_RECEIVED = 3,
  ESTABLISHED = 4,
  FIN_WAIT_1 = 5,
  FIN_WAIT_2 = 6,
  CLOSE_WAIT = 7,
  LAST_ACK = 8,
  TIME_WAIT = 9,
  CLOSING = 10,
}

/**
 * ## TCP Header — 20 Bytes of Connection Management
 *
 * The TCP header carries everything needed for reliable delivery:
 * - Port numbers identify which application on each end
 * - Sequence numbers order the bytes
 * - Acknowledgment numbers confirm receipt
 * - Flags control the connection state machine
 * - Window size provides flow control
 *
 * ```
 *   TCP Flags (each is a single bit):
 *     FIN (0x01) — "I am done sending"
 *     SYN (0x02) — "Let's synchronize sequence numbers"
 *     RST (0x04) — "Abort this connection"
 *     PSH (0x08) — "Push this data to the application immediately"
 *     ACK (0x10) — "The ack_num field is valid"
 * ```
 */
export const TCP_FIN = 0x01;
export const TCP_SYN = 0x02;
export const TCP_RST = 0x04;
export const TCP_PSH = 0x08;
export const TCP_ACK = 0x10;

export class TCPHeader {
  src_port: number;
  dst_port: number;
  seq_num: number;
  ack_num: number;
  data_offset: number;
  flags: number;
  window_size: number;

  constructor(
    src_port: number,
    dst_port: number,
    seq_num: number = 0,
    ack_num: number = 0,
    flags: number = 0,
    window_size: number = 65535,
  ) {
    this.src_port = src_port;
    this.dst_port = dst_port;
    this.seq_num = seq_num;
    this.ack_num = ack_num;
    this.data_offset = 5; // 5 * 4 = 20 bytes (no options)
    this.flags = flags;
    this.window_size = window_size;
  }

  /**
   * Serialize the TCP header to 20 bytes.
   *
   * ```
   *   Bytes 0-1:  Source port
   *   Bytes 2-3:  Destination port
   *   Bytes 4-7:  Sequence number (32-bit)
   *   Bytes 8-11: Acknowledgment number (32-bit)
   *   Byte 12:    Data offset (upper 4 bits) + reserved (lower 4 bits)
   *   Byte 13:    Flags
   *   Bytes 14-15: Window size
   *   Bytes 16-17: Checksum (we set to 0 — simplified)
   *   Bytes 18-19: Urgent pointer (0)
   * ```
   */
  serialize(): number[] {
    const bytes: number[] = new Array(20).fill(0);
    bytes[0] = (this.src_port >> 8) & 0xff;
    bytes[1] = this.src_port & 0xff;
    bytes[2] = (this.dst_port >> 8) & 0xff;
    bytes[3] = this.dst_port & 0xff;
    // Sequence number — 4 bytes big-endian
    bytes[4] = (this.seq_num >> 24) & 0xff;
    bytes[5] = (this.seq_num >> 16) & 0xff;
    bytes[6] = (this.seq_num >> 8) & 0xff;
    bytes[7] = this.seq_num & 0xff;
    // Acknowledgment number — 4 bytes big-endian
    bytes[8] = (this.ack_num >> 24) & 0xff;
    bytes[9] = (this.ack_num >> 16) & 0xff;
    bytes[10] = (this.ack_num >> 8) & 0xff;
    bytes[11] = this.ack_num & 0xff;
    // Data offset (upper 4 bits of byte 12)
    bytes[12] = (this.data_offset << 4) & 0xf0;
    bytes[13] = this.flags & 0xff;
    bytes[14] = (this.window_size >> 8) & 0xff;
    bytes[15] = this.window_size & 0xff;
    return bytes;
  }

  /** Deserialize 20 bytes into a TCPHeader. */
  static deserialize(bytes: number[]): TCPHeader {
    const src_port = (bytes[0] << 8) | bytes[1];
    const dst_port = (bytes[2] << 8) | bytes[3];
    const seq_num =
      ((bytes[4] << 24) | (bytes[5] << 16) | (bytes[6] << 8) | bytes[7]) >>>
      0;
    const ack_num =
      ((bytes[8] << 24) | (bytes[9] << 16) | (bytes[10] << 8) | bytes[11]) >>>
      0;
    const data_offset = (bytes[12] >> 4) & 0x0f;
    const flags = bytes[13];
    const window_size = (bytes[14] << 8) | bytes[15];

    const header = new TCPHeader(
      src_port,
      dst_port,
      seq_num,
      ack_num,
      flags,
      window_size,
    );
    header.data_offset = data_offset;
    return header;
  }
}

/**
 * ## TCP Connection — The State Machine
 *
 * A TCPConnection manages the full lifecycle of a single TCP connection:
 * - Three-way handshake (SYN, SYN+ACK, ACK) to establish
 * - Data transfer with sequence numbers and acknowledgments
 * - Four-way teardown (FIN, ACK, FIN, ACK) to close
 *
 * The send and receive buffers hold data in transit:
 * - send_buffer: data the application wants to send, not yet acknowledged
 * - recv_buffer: data received from the network, not yet read by the app
 */
export class TCPConnection {
  state: TCPState = TCPState.CLOSED;
  local_port: number;
  remote_port: number;
  local_ip: string;
  remote_ip: string;
  send_seq: number = 0;
  recv_next: number = 0;
  send_buffer: number[] = [];
  recv_buffer: number[] = [];

  constructor(
    local_port: number = 0,
    remote_port: number = 0,
    local_ip: string = "0.0.0.0",
    remote_ip: string = "0.0.0.0",
  ) {
    this.local_port = local_port;
    this.remote_port = remote_port;
    this.local_ip = local_ip;
    this.remote_ip = remote_ip;
  }

  /**
   * ## Initiating a Connection — The Client Side
   *
   * The client calls connect(), which:
   * 1. Generates an initial sequence number (ISN)
   * 2. Sends a SYN segment to the server
   * 3. Transitions to SYN_SENT state
   *
   * The ISN should ideally be random (to prevent spoofing attacks), but for
   * our educational implementation we use a simple counter starting at 1000.
   */
  initiate_connect(): TCPHeader | undefined {
    if (this.state !== TCPState.CLOSED) return undefined;

    this.send_seq = 1000; // Initial Sequence Number
    this.state = TCPState.SYN_SENT;

    return new TCPHeader(
      this.local_port,
      this.remote_port,
      this.send_seq,
      0,
      TCP_SYN,
    );
  }

  /**
   * Start listening for incoming connections (server side).
   */
  listen(): void {
    this.state = TCPState.LISTEN;
  }

  /**
   * ## Handling Incoming Segments — The Heart of TCP
   *
   * This method implements the TCP state machine. Depending on the current
   * state and the flags in the incoming segment, we transition to a new state
   * and optionally generate a response segment.
   *
   * The three-way handshake:
   * ```
   *   Client: SYN         →  Server: SYN_RECEIVED, responds SYN+ACK
   *   Client: SYN+ACK     →  Client: ESTABLISHED, responds ACK
   *   Server: ACK         →  Server: ESTABLISHED
   * ```
   *
   * Connection teardown:
   * ```
   *   Active:  FIN        →  Passive: CLOSE_WAIT, responds ACK
   *   Passive: FIN        →  Active:  TIME_WAIT, responds ACK
   * ```
   */
  handle_segment(
    header: TCPHeader,
    data: number[] = [],
  ): TCPHeader | undefined {
    switch (this.state) {
      case TCPState.LISTEN: {
        // Server receives SYN → send SYN+ACK, move to SYN_RECEIVED
        if (header.flags & TCP_SYN) {
          this.remote_port = header.src_port;
          this.recv_next = header.seq_num + 1;
          this.send_seq = 3000; // Server ISN
          this.state = TCPState.SYN_RECEIVED;
          return new TCPHeader(
            this.local_port,
            this.remote_port,
            this.send_seq,
            this.recv_next,
            TCP_SYN | TCP_ACK,
          );
        }
        break;
      }

      case TCPState.SYN_SENT: {
        // Client receives SYN+ACK → send ACK, move to ESTABLISHED
        if ((header.flags & TCP_SYN) && (header.flags & TCP_ACK)) {
          this.recv_next = header.seq_num + 1;
          this.send_seq += 1; // SYN consumed one sequence number
          this.state = TCPState.ESTABLISHED;
          return new TCPHeader(
            this.local_port,
            this.remote_port,
            this.send_seq,
            this.recv_next,
            TCP_ACK,
          );
        }
        break;
      }

      case TCPState.SYN_RECEIVED: {
        // Server receives ACK → move to ESTABLISHED
        if (header.flags & TCP_ACK) {
          this.send_seq += 1; // SYN consumed one sequence number
          this.state = TCPState.ESTABLISHED;
          return undefined; // No response needed
        }
        break;
      }

      case TCPState.ESTABLISHED: {
        // Receive data with PSH+ACK or just ACK
        if (header.flags & TCP_FIN) {
          // Passive close: peer is done sending
          this.recv_next = header.seq_num + 1;
          this.state = TCPState.CLOSE_WAIT;
          return new TCPHeader(
            this.local_port,
            this.remote_port,
            this.send_seq,
            this.recv_next,
            TCP_ACK,
          );
        }

        if (data.length > 0) {
          // Data received — buffer it and send ACK
          this.recv_buffer.push(...data);
          this.recv_next = header.seq_num + data.length;
          return new TCPHeader(
            this.local_port,
            this.remote_port,
            this.send_seq,
            this.recv_next,
            TCP_ACK,
          );
        }

        // Plain ACK (acknowledging our data)
        if (header.flags & TCP_ACK) {
          return undefined;
        }
        break;
      }

      case TCPState.FIN_WAIT_1: {
        // We sent FIN, waiting for ACK
        if (header.flags & TCP_ACK) {
          this.state = TCPState.FIN_WAIT_2;
          return undefined;
        }
        break;
      }

      case TCPState.FIN_WAIT_2: {
        // Waiting for peer's FIN
        if (header.flags & TCP_FIN) {
          this.recv_next = header.seq_num + 1;
          this.state = TCPState.TIME_WAIT;
          return new TCPHeader(
            this.local_port,
            this.remote_port,
            this.send_seq,
            this.recv_next,
            TCP_ACK,
          );
        }
        break;
      }

      case TCPState.CLOSE_WAIT: {
        // We ACKed peer's FIN. Application calls close → send our FIN.
        break;
      }

      case TCPState.LAST_ACK: {
        // We sent our FIN after CLOSE_WAIT, waiting for ACK
        if (header.flags & TCP_ACK) {
          this.state = TCPState.CLOSED;
          return undefined;
        }
        break;
      }

      case TCPState.TIME_WAIT: {
        // Waiting 2*MSL before going to CLOSED (simplified: go immediately)
        this.state = TCPState.CLOSED;
        break;
      }
    }

    return undefined;
  }

  /**
   * Send data over an established connection. Adds data to the send buffer
   * and returns a TCP segment ready for transmission.
   */
  send(data: number[]): { header: TCPHeader; data: number[] } | undefined {
    if (this.state !== TCPState.ESTABLISHED) return undefined;

    this.send_buffer.push(...data);
    const header = new TCPHeader(
      this.local_port,
      this.remote_port,
      this.send_seq,
      this.recv_next,
      TCP_PSH | TCP_ACK,
    );
    this.send_seq += data.length;
    return { header, data };
  }

  /**
   * Read data from the receive buffer. Returns and clears the buffer.
   */
  recv(): number[] {
    const data = [...this.recv_buffer];
    this.recv_buffer = [];
    return data;
  }

  /**
   * Initiate connection close by sending FIN.
   */
  initiate_close(): TCPHeader | undefined {
    if (this.state === TCPState.ESTABLISHED) {
      this.state = TCPState.FIN_WAIT_1;
      return new TCPHeader(
        this.local_port,
        this.remote_port,
        this.send_seq,
        this.recv_next,
        TCP_FIN | TCP_ACK,
      );
    }
    if (this.state === TCPState.CLOSE_WAIT) {
      this.state = TCPState.LAST_ACK;
      return new TCPHeader(
        this.local_port,
        this.remote_port,
        this.send_seq,
        this.recv_next,
        TCP_FIN | TCP_ACK,
      );
    }
    return undefined;
  }
}

// =============================================================================
// LAYER 4: UDP — Fast, Unreliable Datagrams
// =============================================================================

/**
 * ## UDP Header — Just 8 Bytes
 *
 * UDP is the "anti-TCP." No handshake, no acknowledgments, no ordering,
 * no flow control. You fire and forget. The header reflects this simplicity:
 * just source port, destination port, length, and an optional checksum.
 *
 * UDP is ideal when speed matters more than reliability:
 * - DNS lookups (one question, one answer — just resend if lost)
 * - Video streaming (a dropped frame is better than waiting)
 * - Games (latest position update matters, old ones don't)
 */
export class UDPHeader {
  src_port: number;
  dst_port: number;
  length: number;
  checksum: number;

  constructor(
    src_port: number,
    dst_port: number,
    length: number,
    checksum: number = 0,
  ) {
    this.src_port = src_port;
    this.dst_port = dst_port;
    this.length = length;
    this.checksum = checksum;
  }

  /** Serialize to 8 bytes (the entire UDP header). */
  serialize(): number[] {
    return [
      (this.src_port >> 8) & 0xff,
      this.src_port & 0xff,
      (this.dst_port >> 8) & 0xff,
      this.dst_port & 0xff,
      (this.length >> 8) & 0xff,
      this.length & 0xff,
      (this.checksum >> 8) & 0xff,
      this.checksum & 0xff,
    ];
  }

  /** Deserialize 8 bytes into a UDPHeader. */
  static deserialize(bytes: number[]): UDPHeader {
    return new UDPHeader(
      (bytes[0] << 8) | bytes[1],
      (bytes[2] << 8) | bytes[3],
      (bytes[4] << 8) | bytes[5],
      (bytes[6] << 8) | bytes[7],
    );
  }
}

/**
 * ## UDP Socket — Connectionless Communication
 *
 * A UDP socket simply has a local port and a queue of received datagrams.
 * There is no connection state — each datagram is independent.
 */
export class UDPSocket {
  local_port: number;
  recv_queue: Array<{ data: number[]; src_ip: string; src_port: number }> = [];

  constructor(local_port: number) {
    this.local_port = local_port;
  }

  /**
   * Send a datagram to a specific destination. Returns the serialized
   * UDP header + data, ready to be wrapped in an IP packet.
   */
  send_to(
    data: number[],
    dst_port: number,
  ): { header: UDPHeader; data: number[] } {
    const length = 8 + data.length; // 8-byte header + payload
    const header = new UDPHeader(this.local_port, dst_port, length);
    return { header, data };
  }

  /**
   * Deliver a received datagram into this socket's receive queue.
   * Called by the IP layer when a UDP packet arrives for this port.
   */
  deliver(data: number[], src_ip: string, src_port: number): void {
    this.recv_queue.push({ data, src_ip, src_port });
  }

  /**
   * Receive the next datagram from the queue.
   * Returns undefined if no datagrams are available.
   */
  receive_from():
    | { data: number[]; src_ip: string; src_port: number }
    | undefined {
    return this.recv_queue.shift();
  }
}

// =============================================================================
// SOCKET API — The Application Interface
// =============================================================================

/**
 * ## Socket Types
 *
 * The Berkeley Sockets API (invented in the 1980s and still used today)
 * defines two primary socket types:
 *
 * - STREAM (TCP): reliable, ordered byte stream. Think of it as a phone call —
 *   you dial, talk, and hang up.
 * - DGRAM (UDP): unreliable, unordered datagrams. Think of it as sending
 *   postcards — each one is independent, no guarantee of delivery.
 */
export enum SocketType {
  STREAM = 1,
  DGRAM = 2,
}

/**
 * ## Socket — The File Descriptor Abstraction
 *
 * In Unix, "everything is a file." Sockets are file descriptors, just like
 * files and pipes. This means read(), write(), and close() work on sockets.
 *
 * A socket can be in one of several roles:
 * - Unbound: just created, not yet associated with an address
 * - Bound: associated with a local IP and port (via bind())
 * - Listening: waiting for incoming connections (server, TCP only)
 * - Connected: actively exchanging data (client or accepted connection)
 */
export class Socket {
  fd: number;
  socket_type: SocketType;
  local_ip: string = "0.0.0.0";
  local_port: number = 0;
  remote_ip: string = "0.0.0.0";
  remote_port: number = 0;
  tcp_connection: TCPConnection | undefined;
  udp_socket: UDPSocket | undefined;
  is_listening: boolean = false;
  accept_queue: TCPConnection[] = [];

  constructor(fd: number, socket_type: SocketType) {
    this.fd = fd;
    this.socket_type = socket_type;
    if (socket_type === SocketType.STREAM) {
      this.tcp_connection = new TCPConnection();
    } else {
      this.udp_socket = new UDPSocket(0);
    }
  }
}

/**
 * ## Socket Manager — The Kernel's Network Interface
 *
 * The SocketManager is what the OS kernel uses to manage all sockets. It
 * provides the familiar socket API: socket(), bind(), listen(), accept(),
 * connect(), send(), recv(), sendto(), recvfrom(), close().
 *
 * This is the layer that user programs interact with through system calls.
 */
export class SocketManager {
  private sockets: Map<number, Socket> = new Map();
  private next_fd: number = 3; // 0=stdin, 1=stdout, 2=stderr
  private used_ports: Set<number> = new Set();

  /** Create a new socket and return its file descriptor. */
  socket(socket_type: SocketType): number {
    const fd = this.next_fd++;
    const sock = new Socket(fd, socket_type);
    this.sockets.set(fd, sock);
    return fd;
  }

  /**
   * Bind a socket to a local IP address and port.
   * Fails if the port is already in use — this prevents two servers from
   * fighting over the same port.
   */
  bind(fd: number, ip: string, port: number): boolean {
    const sock = this.sockets.get(fd);
    if (!sock) return false;
    if (this.used_ports.has(port)) return false;

    sock.local_ip = ip;
    sock.local_port = port;
    this.used_ports.add(port);

    if (sock.tcp_connection) {
      sock.tcp_connection.local_port = port;
      sock.tcp_connection.local_ip = ip;
    }
    if (sock.udp_socket) {
      sock.udp_socket.local_port = port;
    }
    return true;
  }

  /**
   * Mark a TCP socket as listening for incoming connections.
   * Only makes sense for STREAM (TCP) sockets.
   */
  listen(fd: number): boolean {
    const sock = this.sockets.get(fd);
    if (!sock || sock.socket_type !== SocketType.STREAM) return false;
    sock.is_listening = true;
    sock.tcp_connection?.listen();
    return true;
  }

  /**
   * Accept an incoming connection from the accept queue.
   * Returns a new socket fd for the accepted connection.
   */
  accept(fd: number): number | undefined {
    const sock = this.sockets.get(fd);
    if (!sock || !sock.is_listening) return undefined;

    const conn = sock.accept_queue.shift();
    if (!conn) return undefined;

    const new_fd = this.next_fd++;
    const new_sock = new Socket(new_fd, SocketType.STREAM);
    new_sock.tcp_connection = conn;
    new_sock.local_ip = sock.local_ip;
    new_sock.local_port = sock.local_port;
    new_sock.remote_ip = conn.remote_ip;
    new_sock.remote_port = conn.remote_port;
    this.sockets.set(new_fd, new_sock);
    return new_fd;
  }

  /**
   * Initiate a TCP connection to a remote host.
   * Returns the SYN segment to send.
   */
  connect(
    fd: number,
    remote_ip: string,
    remote_port: number,
  ): TCPHeader | undefined {
    const sock = this.sockets.get(fd);
    if (!sock) return undefined;

    sock.remote_ip = remote_ip;
    sock.remote_port = remote_port;

    if (sock.tcp_connection) {
      sock.tcp_connection.remote_ip = remote_ip;
      sock.tcp_connection.remote_port = remote_port;
      return sock.tcp_connection.initiate_connect();
    }
    return undefined;
  }

  /** Send data on a connected TCP socket. */
  send(
    fd: number,
    data: number[],
  ): { header: TCPHeader; data: number[] } | undefined {
    const sock = this.sockets.get(fd);
    if (!sock || !sock.tcp_connection) return undefined;
    return sock.tcp_connection.send(data);
  }

  /** Receive data from a connected TCP socket. */
  recv(fd: number): number[] | undefined {
    const sock = this.sockets.get(fd);
    if (!sock || !sock.tcp_connection) return undefined;
    return sock.tcp_connection.recv();
  }

  /** Send a UDP datagram to a specific destination. */
  sendto(
    fd: number,
    data: number[],
    dst_port: number,
  ): { header: UDPHeader; data: number[] } | undefined {
    const sock = this.sockets.get(fd);
    if (!sock || !sock.udp_socket) return undefined;
    return sock.udp_socket.send_to(data, dst_port);
  }

  /** Receive a UDP datagram. */
  recvfrom(
    fd: number,
  ): { data: number[]; src_ip: string; src_port: number } | undefined {
    const sock = this.sockets.get(fd);
    if (!sock || !sock.udp_socket) return undefined;
    return sock.udp_socket.receive_from();
  }

  /** Close a socket and free its resources. */
  close(fd: number): TCPHeader | undefined {
    const sock = this.sockets.get(fd);
    if (!sock) return undefined;

    let fin_header: TCPHeader | undefined;

    if (sock.tcp_connection) {
      fin_header = sock.tcp_connection.initiate_close();
    }

    if (sock.local_port > 0) {
      this.used_ports.delete(sock.local_port);
    }
    this.sockets.delete(fd);
    return fin_header;
  }

  /** Get a socket by its file descriptor (for internal use). */
  get_socket(fd: number): Socket | undefined {
    return this.sockets.get(fd);
  }
}

// =============================================================================
// LAYER 7: DNS — Domain Name Resolution
// =============================================================================

/**
 * ## DNS Resolver — Turning Names into Numbers
 *
 * Humans remember names (example.com); computers use numbers (93.184.216.34).
 * DNS (Domain Name System) bridges this gap.
 *
 * Our simplified DNS resolver uses a static table — a hardcoded mapping of
 * hostnames to IP addresses. In a real DNS resolver, you would send a UDP
 * query to port 53 of a DNS server and parse the response.
 *
 * Default entries:
 * - "localhost" → [127, 0, 0, 1]  (the loopback address — "this computer")
 */
export class DNSResolver {
  private static_table: Map<string, number[]> = new Map();

  constructor() {
    // Every DNS resolver knows that localhost is 127.0.0.1.
    // This is the "loopback" address — packets sent here never leave
    // the machine. It's how a computer talks to itself.
    this.static_table.set("localhost", [127, 0, 0, 1]);
  }

  /** Resolve a hostname to an IP address. Returns undefined if not found. */
  resolve(hostname: string): number[] | undefined {
    return this.static_table.get(hostname);
  }

  /** Add a static DNS entry. */
  add_static(hostname: string, ip: number[]): void {
    this.static_table.set(hostname, ip);
  }
}

// =============================================================================
// LAYER 7: HTTP — The Language of the Web
// =============================================================================

/**
 * ## HTTP Request — Asking for a Resource
 *
 * HTTP (Hypertext Transfer Protocol) is the text-based protocol that powers
 * the World Wide Web. Every time your browser loads a page, it sends an HTTP
 * request and receives an HTTP response.
 *
 * An HTTP request looks like this on the wire:
 * ```
 *   GET /index.html HTTP/1.1\r\n
 *   Host: example.com\r\n
 *   Content-Length: 0\r\n
 *   \r\n
 * ```
 *
 * Key parts:
 * - Request line: METHOD PATH HTTP/VERSION
 * - Headers: key-value pairs, one per line
 * - Empty line (\r\n\r\n): separates headers from body
 * - Body: optional payload (used with POST/PUT)
 */
export class HTTPRequest {
  method: string;
  path: string;
  headers: Map<string, string>;
  body: string;

  constructor(
    method: string,
    path: string,
    headers?: Map<string, string>,
    body: string = "",
  ) {
    this.method = method;
    this.path = path;
    this.headers = headers || new Map();
    this.body = body;
  }

  /** Serialize the request to its wire format. */
  serialize(): string {
    let result = `${this.method} ${this.path} HTTP/1.1\r\n`;
    for (const [key, value] of this.headers) {
      result += `${key}: ${value}\r\n`;
    }
    result += "\r\n";
    if (this.body) {
      result += this.body;
    }
    return result;
  }

  /**
   * Deserialize raw text into an HTTPRequest.
   *
   * We split on \r\n to get lines. The first line is the request line
   * (METHOD PATH VERSION). Subsequent lines until the empty line are
   * headers. Everything after the empty line is the body.
   */
  static deserialize(text: string): HTTPRequest {
    const parts = text.split("\r\n");
    const request_line = parts[0].split(" ");
    const method = request_line[0];
    const path = request_line[1];

    const headers = new Map<string, string>();
    let i = 1;
    while (i < parts.length && parts[i] !== "") {
      const colon = parts[i].indexOf(": ");
      if (colon !== -1) {
        headers.set(parts[i].substring(0, colon), parts[i].substring(colon + 2));
      }
      i++;
    }
    // Body is everything after the empty line
    const body = parts.slice(i + 1).join("\r\n");

    return new HTTPRequest(method, path, headers, body);
  }
}

/**
 * ## HTTP Response — The Server's Answer
 *
 * An HTTP response looks like:
 * ```
 *   HTTP/1.1 200 OK\r\n
 *   Content-Type: text/html\r\n
 *   Content-Length: 13\r\n
 *   \r\n
 *   Hello, World!
 * ```
 *
 * Common status codes:
 * - 200 OK: success
 * - 404 Not Found: the requested resource doesn't exist
 * - 500 Internal Server Error: something broke on the server
 */
export class HTTPResponse {
  status_code: number;
  status_text: string;
  headers: Map<string, string>;
  body: string;

  constructor(
    status_code: number,
    status_text: string,
    headers?: Map<string, string>,
    body: string = "",
  ) {
    this.status_code = status_code;
    this.status_text = status_text;
    this.headers = headers || new Map();
    this.body = body;
  }

  /** Serialize the response to its wire format. */
  serialize(): string {
    let result = `HTTP/1.1 ${this.status_code} ${this.status_text}\r\n`;
    for (const [key, value] of this.headers) {
      result += `${key}: ${value}\r\n`;
    }
    result += "\r\n";
    if (this.body) {
      result += this.body;
    }
    return result;
  }

  /**
   * Deserialize raw text into an HTTPResponse.
   *
   * Same structure as request parsing, but the first line is
   * VERSION STATUS_CODE STATUS_TEXT instead of METHOD PATH VERSION.
   */
  static deserialize(text: string): HTTPResponse {
    const parts = text.split("\r\n");
    const status_line = parts[0].split(" ");
    const status_code = parseInt(status_line[1], 10);
    const status_text = status_line.slice(2).join(" ");

    const headers = new Map<string, string>();
    let i = 1;
    while (i < parts.length && parts[i] !== "") {
      const colon = parts[i].indexOf(": ");
      if (colon !== -1) {
        headers.set(parts[i].substring(0, colon), parts[i].substring(colon + 2));
      }
      i++;
    }
    const body = parts.slice(i + 1).join("\r\n");

    return new HTTPResponse(status_code, status_text, headers, body);
  }
}

/**
 * ## HTTP Client — Building and Parsing HTTP Over TCP
 *
 * The HTTP client knows how to construct HTTP requests from URLs and parse
 * HTTP responses from raw text. In a full implementation, it would use
 * the socket manager to establish TCP connections; here we focus on the
 * HTTP-specific logic.
 */
export class HTTPClient {
  dns: DNSResolver;

  constructor(dns?: DNSResolver) {
    this.dns = dns || new DNSResolver();
  }

  /**
   * Build an HTTP GET request from a URL.
   *
   * URL format: http://hostname:port/path
   * If port is omitted, defaults to 80 (the standard HTTP port).
   * If path is omitted, defaults to "/".
   */
  build_request(url: string): { request: HTTPRequest; host: string; port: number } {
    // Strip "http://"
    let rest = url;
    if (rest.startsWith("http://")) {
      rest = rest.substring(7);
    }

    // Split host and path
    const slash_idx = rest.indexOf("/");
    let host_part: string;
    let path: string;
    if (slash_idx === -1) {
      host_part = rest;
      path = "/";
    } else {
      host_part = rest.substring(0, slash_idx);
      path = rest.substring(slash_idx);
    }

    // Split host and port
    const colon_idx = host_part.indexOf(":");
    let host: string;
    let port: number;
    if (colon_idx === -1) {
      host = host_part;
      port = 80;
    } else {
      host = host_part.substring(0, colon_idx);
      port = parseInt(host_part.substring(colon_idx + 1), 10);
    }

    const headers = new Map<string, string>();
    headers.set("Host", host);

    const request = new HTTPRequest("GET", path, headers);
    return { request, host, port };
  }

  /** Parse an HTTP response from raw text. */
  parse_response(text: string): HTTPResponse {
    return HTTPResponse.deserialize(text);
  }
}

// =============================================================================
// NETWORK WIRE — The Simulated Physical Medium
// =============================================================================

/**
 * ## Network Wire — A Virtual Ethernet Cable
 *
 * Since we don't have real network hardware, we simulate it with an in-memory
 * bidirectional channel. A NetworkWire connects two endpoints (think of them
 * as two network cards plugged into the same cable).
 *
 * The wire has two queues, one for each direction:
 * - queue_a_to_b: frames sent by A, to be received by B
 * - queue_b_to_a: frames sent by B, to be received by A
 *
 * ```
 *   ┌─────┐     ┌──────────────┐     ┌─────┐
 *   │  A  │────►│ queue_a_to_b │────►│  B  │
 *   │     │◄────│ queue_b_to_a │◄────│     │
 *   └─────┘     └──────────────┘     └─────┘
 * ```
 */
export class NetworkWire {
  private queue_a_to_b: number[][] = [];
  private queue_b_to_a: number[][] = [];

  /** Side A sends data (will be received by side B). */
  send_a(data: number[]): void {
    this.queue_a_to_b.push([...data]);
  }

  /** Side B sends data (will be received by side A). */
  send_b(data: number[]): void {
    this.queue_b_to_a.push([...data]);
  }

  /** Side A receives data (sent by side B). */
  receive_a(): number[] | undefined {
    return this.queue_b_to_a.shift();
  }

  /** Side B receives data (sent by side A). */
  receive_b(): number[] | undefined {
    return this.queue_a_to_b.shift();
  }

  /** Check if side A has data waiting. */
  has_data_for_a(): boolean {
    return this.queue_b_to_a.length > 0;
  }

  /** Check if side B has data waiting. */
  has_data_for_b(): boolean {
    return this.queue_a_to_b.length > 0;
  }
}
