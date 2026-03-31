-- ============================================================================
-- network_stack — Layered Network Protocol Stack
-- ============================================================================
--
-- The network stack implements the TCP/IP model: a hierarchy of layers where
-- each layer wraps data from the layer above in its own header, and strips
-- headers from data arriving from below.
--
-- ## The Postal Analogy
--
--   Ethernet  — local mail carrier: delivers between houses on the same street
--   IP        — postal routing system: figures out which city and post office
--   TCP       — registered mail with tracking: guarantees delivery and order
--   UDP       — a postcard: fast, no guarantee it arrives
--
-- ## Packet Encapsulation (downward)
--
--   Application:   "Hello, World!"
--   TCP segment:   [TCP Header | "Hello, World!"]
--   IP packet:     [IP Header  | TCP segment    ]
--   Ethernet frame:[Eth Header | IP packet      ]
--   Wire:          raw bytes →→→→→→→→→→→→→→→→→→→
--
-- On the receiving side, each layer strips its header and passes the payload up.
--
-- ## Layer Stack
--
--   +---------------------+
--   | Layer 4: TCP / UDP  |  — Transport: reliable / unreliable byte streams
--   +---------------------+
--   | Layer 3: IP         |  — Network: routing, addresses, TTL
--   +---------------------+
--   | Layer 2: Ethernet   |  — Link: MAC addresses, local delivery
--   +---------------------+
--   | Layer 1: Wire       |  — Physical: raw bits (simulated as byte list)
--   +---------------------+
--
-- ## Module Structure
--
--   network_stack
--   ├── EthernetFrame   — Layer 2: MAC header + payload
--   ├── ARPTable        — IP → MAC address resolution cache
--   ├── IPv4Header      — Layer 3: IP header with checksum
--   ├── RoutingTable    — Longest-prefix-match routing
--   ├── IPLayer         — IP packet construction and parsing
--   ├── TCPSegment      — Layer 4: TCP header with checksum
--   ├── UDPDatagram     — Layer 4: UDP header
--   └── NetworkStack    — Full stack: send/receive end-to-end
--
-- ============================================================================

local M = {}

-- IP protocol numbers (IANA assigned)
M.PROTO_TCP = 6    -- Transmission Control Protocol
M.PROTO_UDP = 17   -- User Datagram Protocol

-- Ethernet type codes
M.ETHERTYPE_IPV4 = 0x0800  -- IPv4 packet
M.ETHERTYPE_ARP  = 0x0806  -- ARP request/reply

-- Broadcast MAC address
M.MAC_BROADCAST = { 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF }

-- ============================================================================
-- EthernetFrame — Layer 2: Local Delivery
-- ============================================================================
--
-- Every network interface card (NIC) has a unique 48-bit MAC address burned
-- in at the factory.  Ethernet frames carry data between devices on the same
-- local network segment.
--
-- ### Wire Format
--
--   ┌──────────┬──────────┬───────────┬───────────────────┐
--   │ dest_mac │  src_mac │ ether_type │      payload      │
--   │  6 bytes │  6 bytes │   2 bytes  │      N bytes      │
--   └──────────┴──────────┴───────────┴───────────────────┘
--
-- The ether_type field identifies what is inside the payload:
--   0x0800 = IPv4    0x0806 = ARP
--
-- MAC addresses are written as: AA:BB:CC:DD:EE:FF (6 hex bytes)
-- The broadcast address FF:FF:FF:FF:FF:FF reaches every device.

M.EthernetFrame = {}
M.EthernetFrame.__index = M.EthernetFrame

--- Create a new Ethernet frame.
-- @param dest_mac    List of 6 bytes
-- @param src_mac     List of 6 bytes
-- @param ether_type  16-bit integer (0x0800=IPv4, 0x0806=ARP)
-- @param payload     List of bytes
function M.EthernetFrame.new(dest_mac, src_mac, ether_type, payload)
  return setmetatable({
    dest_mac   = dest_mac,
    src_mac    = src_mac,
    ether_type = ether_type,
    payload    = payload,
  }, M.EthernetFrame)
end

--- Serialize frame to a flat list of bytes for wire transmission.
function M.EthernetFrame:serialize()
  local bytes = {}
  for _, b in ipairs(self.dest_mac)  do table.insert(bytes, b) end
  for _, b in ipairs(self.src_mac)   do table.insert(bytes, b) end
  table.insert(bytes, (self.ether_type >> 8) & 0xFF)  -- high byte
  table.insert(bytes, self.ether_type & 0xFF)          -- low byte
  for _, b in ipairs(self.payload)   do table.insert(bytes, b) end
  return bytes
end

--- Deserialize a flat list of bytes into an EthernetFrame.
function M.EthernetFrame.deserialize(bytes)
  local dest_mac = {}
  for i = 1, 6 do dest_mac[i] = bytes[i] end
  local src_mac = {}
  for i = 1, 6 do src_mac[i] = bytes[6 + i] end
  local ether_type = (bytes[13] << 8) | bytes[14]
  local payload = {}
  for i = 15, #bytes do table.insert(payload, bytes[i]) end
  return M.EthernetFrame.new(dest_mac, src_mac, ether_type, payload)
end

-- ============================================================================
-- ARPTable — IP-to-MAC Address Resolution Cache
-- ============================================================================
--
-- When a computer wants to send an IP packet to 192.168.1.5, it needs the MAC
-- address of that device's NIC.  ARP (Address Resolution Protocol) discovers
-- and caches these mappings.
--
-- Format: IP address string → MAC address (list of 6 bytes)
--
-- In a real network, an ARP broadcast goes out asking "who has IP X?".
-- The owner replies "I have X, my MAC is Y".  We simulate this by letting
-- callers insert entries directly.

M.ARPTable = {}
M.ARPTable.__index = M.ARPTable

function M.ARPTable.new()
  return setmetatable({ entries = {} }, M.ARPTable)
end

local function copy_arp(t)
  local e = {}
  for k, v in pairs(t.entries) do e[k] = v end
  return setmetatable({ entries = e }, M.ARPTable)
end

--- Look up MAC for an IP string.  Returns list of 6 bytes or nil.
function M.ARPTable:lookup(ip)
  return self.entries[ip]
end

--- Insert or update an IP-to-MAC mapping.
function M.ARPTable:insert(ip, mac)
  local t = copy_arp(self)
  t.entries[ip] = mac
  return t
end

--- Number of entries.
function M.ARPTable:size()
  local n = 0
  for _ in pairs(self.entries) do n = n + 1 end
  return n
end

-- ============================================================================
-- IPv4Header — Layer 3: Routing Label
-- ============================================================================
--
-- IP is the backbone of the Internet.  Every device has an IP address, and IP
-- headers tell routers where to send each packet.
--
-- ### IPv4 Header (20 bytes, no options)
--
--   Byte  0:    version(4 bits) + IHL(4 bits)  → 0x45 for standard IPv4
--   Byte  1:    Type of Service (ignored here)
--   Byte  2-3:  Total length (header + payload)
--   Byte  4-7:  Identification, Flags, Fragment offset (all 0)
--   Byte  8:    TTL (Time To Live) — decremented by each router; drop at 0
--   Byte  9:    Protocol (6=TCP, 17=UDP)
--   Byte  10-11: Header checksum (ones' complement)
--   Byte  12-15: Source IP address (4 bytes)
--   Byte  16-19: Destination IP address (4 bytes)
--
-- ### TTL — Time To Live
--
-- Each router that forwards the packet decrements TTL by 1.  When TTL hits 0,
-- the packet is discarded and an ICMP "Time Exceeded" message is sent back.
-- This prevents packets from looping forever in the event of a routing loop.
-- Default TTL is 64 (Linux default).

M.IPv4Header = {}
M.IPv4Header.__index = M.IPv4Header

--- Create a new IPv4 header.
-- @param src_ip       List of 4 bytes (source address)
-- @param dst_ip       List of 4 bytes (destination address)
-- @param protocol     8-bit protocol number (6=TCP, 17=UDP)
-- @param total_length Total length of IP packet (header + payload) in bytes
-- @param ttl          Time To Live (default 64)
function M.IPv4Header.new(src_ip, dst_ip, protocol, total_length, ttl)
  return setmetatable({
    version         = 4,
    ihl             = 5,       -- 5 × 4 = 20 bytes header, no options
    total_length    = total_length,
    ttl             = ttl or 64,
    protocol        = protocol,
    header_checksum = 0,
    src_ip          = src_ip,
    dst_ip          = dst_ip,
  }, M.IPv4Header)
end

--- Serialize to 20 bytes (list of integers).
function M.IPv4Header:serialize()
  local b = {}
  b[1]  = ((self.version << 4) | self.ihl) & 0xFF
  b[2]  = 0  -- TOS
  b[3]  = (self.total_length >> 8) & 0xFF
  b[4]  = self.total_length & 0xFF
  b[5]  = 0; b[6] = 0; b[7] = 0; b[8] = 0  -- ID, flags, fragment
  b[9]  = self.ttl & 0xFF
  b[10] = self.protocol & 0xFF
  b[11] = (self.header_checksum >> 8) & 0xFF
  b[12] = self.header_checksum & 0xFF
  b[13] = self.src_ip[1]; b[14] = self.src_ip[2]
  b[15] = self.src_ip[3]; b[16] = self.src_ip[4]
  b[17] = self.dst_ip[1]; b[18] = self.dst_ip[2]
  b[19] = self.dst_ip[3]; b[20] = self.dst_ip[4]
  return b
end

--- Deserialize 20 bytes into an IPv4Header.
function M.IPv4Header.deserialize(bytes)
  local h = setmetatable({}, M.IPv4Header)
  h.version         = (bytes[1] >> 4) & 0x0F
  h.ihl             = bytes[1] & 0x0F
  h.total_length    = (bytes[3] << 8) | bytes[4]
  h.ttl             = bytes[9]
  h.protocol        = bytes[10]
  h.header_checksum = (bytes[11] << 8) | bytes[12]
  h.src_ip = { bytes[13], bytes[14], bytes[15], bytes[16] }
  h.dst_ip = { bytes[17], bytes[18], bytes[19], bytes[20] }
  return h
end

--- Compute the IP header checksum (ones' complement sum of 16-bit words).
function M.IPv4Header:compute_checksum()
  local saved = self.header_checksum
  self.header_checksum = 0
  local b = self:serialize()
  self.header_checksum = saved

  -- Sum all 16-bit words
  local sum = 0
  for i = 1, #b, 2 do
    local word = (b[i] << 8) | (b[i + 1] or 0)
    sum = sum + word
  end
  -- Fold carry bits
  while sum > 0xFFFF do
    sum = (sum & 0xFFFF) + (sum >> 16)
  end
  -- Ones' complement
  return (~sum) & 0xFFFF
end

--- Verify the checksum of a received header (valid if sum+checksum = 0xFFFF).
function M.IPv4Header:verify_checksum()
  local b = self:serialize()
  local sum = 0
  for i = 1, #b, 2 do
    local word = (b[i] << 8) | (b[i + 1] or 0)
    sum = sum + word
  end
  while sum > 0xFFFF do
    sum = (sum & 0xFFFF) + (sum >> 16)
  end
  return (sum & 0xFFFF) == 0xFFFF
end

-- ============================================================================
-- RoutingTable — Longest-Prefix-Match Routing
-- ============================================================================
--
-- A routing table is a list of rules: "packets destined for network X with
-- mask Y should go to gateway G via interface I."
--
-- The key algorithm is **longest prefix match**: if two routes both match a
-- destination, use the more specific one (more 1-bits in the mask = more
-- specific = longer prefix).
--
-- Example routing table:
--
--   Network      | Mask          | Gateway      | Iface
--   0.0.0.0      | 0.0.0.0       | 192.168.1.1  | eth0  ← default route
--   192.168.1.0  | 255.255.255.0 | 0.0.0.0      | eth0  ← local subnet
--   10.0.0.0     | 255.0.0.0     | 192.168.1.254| eth0  ← more specific
--
-- Packet to 10.5.3.1: matches both default route and 10.0.0.0/8.
-- Longest prefix match picks 10.0.0.0/8 (8 bits > 0 bits).

M.RoutingTable = {}
M.RoutingTable.__index = M.RoutingTable

function M.RoutingTable.new()
  return setmetatable({ routes = {} }, M.RoutingTable)
end

local function copy_rt(t)
  local r = {}
  for _, v in ipairs(t.routes) do table.insert(r, v) end
  return setmetatable({ routes = r }, M.RoutingTable)
end

--- Add a route entry.
-- @param network  List of 4 bytes (network address)
-- @param mask     List of 4 bytes (subnet mask)
-- @param gateway  List of 4 bytes (next hop; 0.0.0.0 for directly connected)
-- @param iface    Interface name (string)
function M.RoutingTable:add_route(network, mask, gateway, iface)
  local t = copy_rt(self)
  table.insert(t.routes, { network = network, mask = mask, gateway = gateway, iface = iface })
  return t
end

-- Count the number of 1-bits in a mask (prefix length)
local function count_mask_bits(mask)
  local total = 0
  for _, byte in ipairs(mask) do
    local b = byte
    while b > 0 do
      total = total + (b & 1)
      b = b >> 1
    end
  end
  return total
end

-- Check if dst_ip matches network/mask
local function matches_route(dst_ip, network, mask)
  for i = 1, 4 do
    if (dst_ip[i] & mask[i]) ~= network[i] then return false end
  end
  return true
end

--- Find the best route for a destination IP (longest prefix match).
-- Returns route entry table or nil.
function M.RoutingTable:lookup(dst_ip)
  local best = nil
  local best_len = -1
  for _, route in ipairs(self.routes) do
    if matches_route(dst_ip, route.network, route.mask) then
      local len = count_mask_bits(route.mask)
      if len > best_len then
        best = route
        best_len = len
      end
    end
  end
  return best
end

-- ============================================================================
-- IPLayer — IP Packet Construction and Parsing
-- ============================================================================

M.IPLayer = {}
M.IPLayer.__index = M.IPLayer

function M.IPLayer.new(local_ip)
  return setmetatable({
    local_ip      = local_ip,
    routing_table = M.RoutingTable.new(),
    arp_table     = M.ARPTable.new(),
  }, M.IPLayer)
end

--- Create an IP packet (header + payload) as a list of bytes.
-- @param dst_ip    Destination IP (list of 4 bytes)
-- @param protocol  Protocol number
-- @param payload   List of bytes (TCP/UDP segment)
function M.IPLayer:create_packet(dst_ip, protocol, payload)
  local total_length = 20 + #payload
  local header = M.IPv4Header.new(self.local_ip, dst_ip, protocol, total_length)
  local checksum = header:compute_checksum()
  header.header_checksum = checksum
  local bytes = header:serialize()
  for _, b in ipairs(payload) do table.insert(bytes, b) end
  return bytes
end

--- Parse a received IP packet.
-- @return "ok", src_ip, protocol, payload  OR  "bad_checksum", nil...
function M.IPLayer.parse_packet(bytes)
  local header_bytes = {}
  for i = 1, 20 do header_bytes[i] = bytes[i] end
  local header = M.IPv4Header.deserialize(header_bytes)
  if not header:verify_checksum() then
    return "bad_checksum", nil, nil, nil
  end
  local payload = {}
  for i = 21, #bytes do table.insert(payload, bytes[i]) end
  return "ok", header.src_ip, header.protocol, payload
end

-- ============================================================================
-- TCPSegment — Layer 4: Reliable Transport
-- ============================================================================
--
-- TCP (Transmission Control Protocol) provides:
--   - Reliable delivery: lost packets are retransmitted
--   - Ordered delivery: bytes arrive in the order they were sent
--   - Flow control:     fast senders don't overwhelm slow receivers
--
-- ### TCP Header (20 bytes, no options)
--
--   Byte 0-1:   Source port
--   Byte 2-3:   Destination port
--   Byte 4-7:   Sequence number (position in the byte stream)
--   Byte 8-11:  Acknowledgment number (next expected byte from peer)
--   Byte 12:    Data offset (4 bits = header length in 32-bit words) + Reserved
--   Byte 13:    Control flags: FIN SYN RST PSH ACK URG (6 bits)
--   Byte 14-15: Window size (flow control: how many bytes peer can receive)
--   Byte 16-17: Checksum
--   Byte 18-19: Urgent pointer (used with URG flag)
--
-- ### Three-Way Handshake (SYN → SYN-ACK → ACK)
--
--   Client                     Server
--     │── SYN (seq=100) ──────────►│
--     │◄── SYN-ACK (seq=200,ack=101)│
--     │── ACK (ack=201) ───────────►│
--     │        [connection open]     │
--
-- ### TCP Flags
--
--   SYN (0x02) — Synchronize: initiates connection
--   ACK (0x10) — Acknowledgment: ack_num is valid
--   FIN (0x01) — Finish: no more data from sender
--   RST (0x04) — Reset: abort connection
--   PSH (0x08) — Push: deliver to application immediately

M.TCP_FLAG_FIN = 0x01
M.TCP_FLAG_SYN = 0x02
M.TCP_FLAG_RST = 0x04
M.TCP_FLAG_PSH = 0x08
M.TCP_FLAG_ACK = 0x10
M.TCP_FLAG_URG = 0x20

M.TCPSegment = {}
M.TCPSegment.__index = M.TCPSegment

--- Create a new TCP segment.
function M.TCPSegment.new(src_port, dst_port, seq_num, ack_num, flags, window_size, payload)
  return setmetatable({
    src_port    = src_port,
    dst_port    = dst_port,
    seq_num     = seq_num   or 0,
    ack_num     = ack_num   or 0,
    data_offset = 5,   -- 5 × 4 = 20 bytes (no options)
    flags       = flags or 0,
    window_size = window_size or 65535,
    checksum    = 0,
    urgent_ptr  = 0,
    payload     = payload or {},
  }, M.TCPSegment)
end

--- Serialize TCP segment to bytes.
function M.TCPSegment:serialize()
  local b = {}
  b[1]  = (self.src_port >> 8) & 0xFF
  b[2]  = self.src_port & 0xFF
  b[3]  = (self.dst_port >> 8) & 0xFF
  b[4]  = self.dst_port & 0xFF
  b[5]  = (self.seq_num >> 24) & 0xFF
  b[6]  = (self.seq_num >> 16) & 0xFF
  b[7]  = (self.seq_num >> 8) & 0xFF
  b[8]  = self.seq_num & 0xFF
  b[9]  = (self.ack_num >> 24) & 0xFF
  b[10] = (self.ack_num >> 16) & 0xFF
  b[11] = (self.ack_num >> 8) & 0xFF
  b[12] = self.ack_num & 0xFF
  b[13] = (self.data_offset << 4) & 0xFF
  b[14] = self.flags & 0xFF
  b[15] = (self.window_size >> 8) & 0xFF
  b[16] = self.window_size & 0xFF
  b[17] = (self.checksum >> 8) & 0xFF
  b[18] = self.checksum & 0xFF
  b[19] = (self.urgent_ptr >> 8) & 0xFF
  b[20] = self.urgent_ptr & 0xFF
  for _, by in ipairs(self.payload) do table.insert(b, by) end
  return b
end

--- Deserialize bytes into a TCP segment.
function M.TCPSegment.deserialize(bytes)
  local seg = setmetatable({}, M.TCPSegment)
  seg.src_port    = (bytes[1] << 8) | bytes[2]
  seg.dst_port    = (bytes[3] << 8) | bytes[4]
  seg.seq_num     = (bytes[5] << 24) | (bytes[6] << 16) | (bytes[7] << 8) | bytes[8]
  seg.ack_num     = (bytes[9] << 24) | (bytes[10] << 16) | (bytes[11] << 8) | bytes[12]
  seg.data_offset = (bytes[13] >> 4) & 0x0F
  seg.flags       = bytes[14]
  seg.window_size = (bytes[15] << 8) | bytes[16]
  seg.checksum    = (bytes[17] << 8) | bytes[18]
  seg.urgent_ptr  = (bytes[19] << 8) | bytes[20]
  seg.payload = {}
  local hdr_len = seg.data_offset * 4
  for i = hdr_len + 1, #bytes do table.insert(seg.payload, bytes[i]) end
  return seg
end

--- Check if a specific flag is set.
function M.TCPSegment:has_flag(flag)
  return (self.flags & flag) ~= 0
end

-- ============================================================================
-- UDPDatagram — Layer 4: Unreliable Transport
-- ============================================================================
--
-- UDP (User Datagram Protocol) provides:
--   - Fast delivery with no setup (no handshake)
--   - No guarantee of delivery or order
--   - Perfect for: DNS, streaming video, real-time games
--
-- ### UDP Header (8 bytes — much simpler than TCP)
--
--   Byte 0-1:  Source port
--   Byte 2-3:  Destination port
--   Byte 4-5:  Length (header + data)
--   Byte 6-7:  Checksum (optional in IPv4, required in IPv6)

M.UDPDatagram = {}
M.UDPDatagram.__index = M.UDPDatagram

function M.UDPDatagram.new(src_port, dst_port, payload)
  payload = payload or {}
  return setmetatable({
    src_port = src_port,
    dst_port = dst_port,
    length   = 8 + #payload,
    checksum = 0,
    payload  = payload,
  }, M.UDPDatagram)
end

--- Serialize UDP datagram to bytes.
function M.UDPDatagram:serialize()
  local b = {}
  b[1] = (self.src_port >> 8) & 0xFF
  b[2] = self.src_port & 0xFF
  b[3] = (self.dst_port >> 8) & 0xFF
  b[4] = self.dst_port & 0xFF
  b[5] = (self.length >> 8) & 0xFF
  b[6] = self.length & 0xFF
  b[7] = (self.checksum >> 8) & 0xFF
  b[8] = self.checksum & 0xFF
  for _, by in ipairs(self.payload) do table.insert(b, by) end
  return b
end

--- Deserialize bytes into a UDPDatagram.
function M.UDPDatagram.deserialize(bytes)
  local dg = setmetatable({}, M.UDPDatagram)
  dg.src_port = (bytes[1] << 8) | bytes[2]
  dg.dst_port = (bytes[3] << 8) | bytes[4]
  dg.length   = (bytes[5] << 8) | bytes[6]
  dg.checksum = (bytes[7] << 8) | bytes[8]
  dg.payload  = {}
  for i = 9, #bytes do table.insert(dg.payload, bytes[i]) end
  return dg
end

-- ============================================================================
-- NetworkStack — Full End-to-End Stack
-- ============================================================================
--
-- The NetworkStack combines all layers into a single cohesive object.
-- Sending a TCP/UDP payload goes through each layer in order (encapsulation).
-- Receiving a raw Ethernet frame goes through layers in reverse (decapsulation).

M.NetworkStack = {}
M.NetworkStack.__index = M.NetworkStack

--- Create a new NetworkStack.
-- @param local_ip   List of 4 bytes (our IP address)
-- @param local_mac  List of 6 bytes (our MAC address)
function M.NetworkStack.new(local_ip, local_mac)
  return setmetatable({
    local_ip      = local_ip,
    local_mac     = local_mac,
    ip_layer      = M.IPLayer.new(local_ip),
    routing_table = M.RoutingTable.new(),
    arp_table     = M.ARPTable.new(),
  }, M.NetworkStack)
end

local function copy_stack(s)
  return setmetatable({
    local_ip      = s.local_ip,
    local_mac     = s.local_mac,
    ip_layer      = s.ip_layer,
    routing_table = s.routing_table,
    arp_table     = s.arp_table,
  }, M.NetworkStack)
end

--- Add a route to the routing table.
function M.NetworkStack:add_route(network, mask, gateway, iface)
  local st = copy_stack(self)
  st.routing_table = st.routing_table:add_route(network, mask, gateway, iface)
  return st
end

--- Add an ARP entry.
function M.NetworkStack:add_arp(ip, mac)
  local st = copy_stack(self)
  st.arp_table = st.arp_table:insert(ip, mac)
  return st
end

--- Send a UDP payload: construct UDP → IP → Ethernet → return wire bytes.
-- @param dst_ip    List of 4 bytes
-- @param dst_mac   List of 6 bytes
-- @param src_port  UDP source port
-- @param dst_port  UDP destination port
-- @param data      List of bytes (application payload)
-- @return list of raw wire bytes
function M.NetworkStack:send_udp(dst_ip, dst_mac, src_port, dst_port, data)
  -- Layer 4: UDP
  local udp = M.UDPDatagram.new(src_port, dst_port, data)
  local udp_bytes = udp:serialize()

  -- Layer 3: IP
  local ip_bytes = self.ip_layer:create_packet(dst_ip, M.PROTO_UDP, udp_bytes)

  -- Layer 2: Ethernet
  local frame = M.EthernetFrame.new(dst_mac, self.local_mac, M.ETHERTYPE_IPV4, ip_bytes)
  return frame:serialize()
end

--- Send a TCP payload: construct TCP → IP → Ethernet → return wire bytes.
function M.NetworkStack:send_tcp(dst_ip, dst_mac, src_port, dst_port, seq, ack, flags, data)
  -- Layer 4: TCP
  local tcp = M.TCPSegment.new(src_port, dst_port, seq, ack, flags, 65535, data)
  local tcp_bytes = tcp:serialize()

  -- Layer 3: IP
  local ip_bytes = self.ip_layer:create_packet(dst_ip, M.PROTO_TCP, tcp_bytes)

  -- Layer 2: Ethernet
  local frame = M.EthernetFrame.new(dst_mac, self.local_mac, M.ETHERTYPE_IPV4, ip_bytes)
  return frame:serialize()
end

--- Receive and decapsulate wire bytes.
-- Strips Ethernet → IP → TCP/UDP headers.
-- Returns: "ok", protocol, src_ip, src_port, dst_port, payload
--       OR "error", reason, nil...
function M.NetworkStack:receive(wire_bytes)
  -- Layer 2: Ethernet
  if #wire_bytes < 14 then return "error", "too_short", nil, nil, nil, nil end
  local frame = M.EthernetFrame.deserialize(wire_bytes)

  if frame.ether_type ~= M.ETHERTYPE_IPV4 then
    return "error", "not_ipv4", nil, nil, nil, nil
  end

  -- Layer 3: IP
  if #frame.payload < 20 then return "error", "ip_too_short", nil, nil, nil, nil end
  local ip_status, src_ip, protocol, transport_bytes = M.IPLayer.parse_packet(frame.payload)
  if ip_status ~= "ok" then
    return "error", ip_status, nil, nil, nil, nil
  end

  -- Layer 4: TCP or UDP
  if protocol == M.PROTO_UDP then
    if #transport_bytes < 8 then return "error", "udp_too_short", nil, nil, nil, nil end
    local dg = M.UDPDatagram.deserialize(transport_bytes)
    return "ok", "udp", src_ip, dg.src_port, dg.dst_port, dg.payload
  elseif protocol == M.PROTO_TCP then
    if #transport_bytes < 20 then return "error", "tcp_too_short", nil, nil, nil, nil end
    local seg = M.TCPSegment.deserialize(transport_bytes)
    return "ok", "tcp", src_ip, seg.src_port, seg.dst_port, seg.payload
  else
    return "error", "unknown_protocol", nil, nil, nil, nil
  end
end

return M
