-- Tests for coding_adventures.network_stack
-- Coverage target: 95%+

local NS = require("coding_adventures.network_stack")

-- Helper: string to byte list
local function str_to_bytes(s)
  local b = {}
  for i = 1, #s do b[i] = string.byte(s, i) end
  return b
end

-- Helper: byte list to string
local function bytes_to_str(b)
  local t = {}
  for i, v in ipairs(b) do t[i] = string.char(v) end
  return table.concat(t)
end

local MAC1 = { 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01 }
local MAC2 = { 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x02 }
local IP1  = { 192, 168, 1, 10 }
local IP2  = { 192, 168, 1, 20 }

-- ============================================================================
-- Constants
-- ============================================================================

describe("constants", function()
  it("protocol numbers", function()
    assert.are.equal(NS.PROTO_TCP, 6)
    assert.are.equal(NS.PROTO_UDP, 17)
  end)

  it("ether types", function()
    assert.are.equal(NS.ETHERTYPE_IPV4, 0x0800)
    assert.are.equal(NS.ETHERTYPE_ARP,  0x0806)
  end)

  it("TCP flags", function()
    assert.are.equal(NS.TCP_FLAG_FIN, 0x01)
    assert.are.equal(NS.TCP_FLAG_SYN, 0x02)
    assert.are.equal(NS.TCP_FLAG_RST, 0x04)
    assert.are.equal(NS.TCP_FLAG_PSH, 0x08)
    assert.are.equal(NS.TCP_FLAG_ACK, 0x10)
    assert.are.equal(NS.TCP_FLAG_URG, 0x20)
  end)

  it("broadcast MAC is all FF", function()
    assert.are.same(NS.MAC_BROADCAST, { 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF })
  end)
end)

-- ============================================================================
-- EthernetFrame tests
-- ============================================================================

describe("EthernetFrame", function()
  it("new creates frame with fields", function()
    local f = NS.EthernetFrame.new(MAC2, MAC1, NS.ETHERTYPE_IPV4, { 1, 2, 3 })
    assert.are.same(f.dest_mac, MAC2)
    assert.are.same(f.src_mac, MAC1)
    assert.are.equal(f.ether_type, NS.ETHERTYPE_IPV4)
    assert.are.same(f.payload, { 1, 2, 3 })
  end)

  it("serialize produces correct byte layout", function()
    local payload = { 0x08, 0x00 }
    local f = NS.EthernetFrame.new(MAC2, MAC1, 0x0800, payload)
    local b = f:serialize()
    -- first 6 bytes = dest_mac
    assert.are.equal(b[1], 0xAA)
    assert.are.equal(b[6], 0x02)
    -- next 6 = src_mac
    assert.are.equal(b[7], 0xAA)
    assert.are.equal(b[12], 0x01)
    -- ether_type (2 bytes, big-endian)
    assert.are.equal(b[13], 0x08)
    assert.are.equal(b[14], 0x00)
    -- payload
    assert.are.equal(b[15], 0x08)
    assert.are.equal(b[16], 0x00)
  end)

  it("deserialize round-trips serialize", function()
    local payload = { 10, 20, 30 }
    local f1 = NS.EthernetFrame.new(MAC2, MAC1, NS.ETHERTYPE_ARP, payload)
    local b  = f1:serialize()
    local f2 = NS.EthernetFrame.deserialize(b)
    assert.are.same(f2.dest_mac, MAC2)
    assert.are.same(f2.src_mac, MAC1)
    assert.are.equal(f2.ether_type, NS.ETHERTYPE_ARP)
    assert.are.same(f2.payload, payload)
  end)

  it("empty payload", function()
    local f = NS.EthernetFrame.new(MAC1, MAC2, 0x0800, {})
    local b = f:serialize()
    assert.are.equal(#b, 14)  -- 6+6+2 = 14 bytes header only
    local f2 = NS.EthernetFrame.deserialize(b)
    assert.are.same(f2.payload, {})
  end)
end)

-- ============================================================================
-- ARPTable tests
-- ============================================================================

describe("ARPTable", function()
  it("new creates empty table", function()
    local t = NS.ARPTable.new()
    assert.are.equal(t:size(), 0)
    assert.is_nil(t:lookup("192.168.1.1"))
  end)

  it("insert and lookup", function()
    local t = NS.ARPTable.new()
    local t2 = t:insert("192.168.1.5", MAC1)
    assert.are.same(t2:lookup("192.168.1.5"), MAC1)
    assert.are.equal(t2:size(), 1)
  end)

  it("insert is immutable", function()
    local t = NS.ARPTable.new()
    t:insert("10.0.0.1", MAC1)
    assert.are.equal(t:size(), 0)
  end)

  it("multiple entries", function()
    local t = NS.ARPTable.new()
    local t2 = t:insert("192.168.1.1", MAC1)
    local t3 = t2:insert("192.168.1.2", MAC2)
    assert.are.equal(t3:size(), 2)
    assert.are.same(t3:lookup("192.168.1.1"), MAC1)
    assert.are.same(t3:lookup("192.168.1.2"), MAC2)
  end)

  it("update existing entry", function()
    local t = NS.ARPTable.new()
    local t2 = t:insert("10.0.0.1", MAC1)
    local t3 = t2:insert("10.0.0.1", MAC2)
    assert.are.same(t3:lookup("10.0.0.1"), MAC2)
  end)
end)

-- ============================================================================
-- IPv4Header tests
-- ============================================================================

describe("IPv4Header", function()
  it("new creates header with defaults", function()
    local h = NS.IPv4Header.new(IP1, IP2, NS.PROTO_TCP, 60)
    assert.are.equal(h.version, 4)
    assert.are.equal(h.ihl, 5)
    assert.are.equal(h.ttl, 64)
    assert.are.equal(h.protocol, NS.PROTO_TCP)
    assert.are.equal(h.total_length, 60)
    assert.are.same(h.src_ip, IP1)
    assert.are.same(h.dst_ip, IP2)
  end)

  it("new with custom TTL", function()
    local h = NS.IPv4Header.new(IP1, IP2, NS.PROTO_UDP, 28, 128)
    assert.are.equal(h.ttl, 128)
  end)

  it("serialize produces 20 bytes", function()
    local h = NS.IPv4Header.new(IP1, IP2, NS.PROTO_TCP, 60)
    local b = h:serialize()
    assert.are.equal(#b, 20)
  end)

  it("serialize first byte is version+IHL = 0x45", function()
    local h = NS.IPv4Header.new(IP1, IP2, NS.PROTO_TCP, 60)
    local b = h:serialize()
    assert.are.equal(b[1], 0x45)
  end)

  it("deserialize round-trips serialize", function()
    local h1 = NS.IPv4Header.new(IP1, IP2, NS.PROTO_UDP, 28)
    h1.header_checksum = h1:compute_checksum()
    local b  = h1:serialize()
    local h2 = NS.IPv4Header.deserialize(b)
    assert.are.equal(h2.version, 4)
    assert.are.equal(h2.protocol, NS.PROTO_UDP)
    assert.are.equal(h2.total_length, 28)
    assert.are.equal(h2.ttl, 64)
    assert.are.same(h2.src_ip, IP1)
    assert.are.same(h2.dst_ip, IP2)
  end)

  it("compute_checksum is non-zero", function()
    local h = NS.IPv4Header.new(IP1, IP2, NS.PROTO_TCP, 60)
    local cs = h:compute_checksum()
    assert.is_true(cs ~= 0)
    assert.is_true(cs <= 0xFFFF)
  end)

  it("verify_checksum passes for correct checksum", function()
    local h = NS.IPv4Header.new(IP1, IP2, NS.PROTO_TCP, 60)
    h.header_checksum = h:compute_checksum()
    assert.is_true(h:verify_checksum())
  end)

  it("verify_checksum fails for corrupted checksum", function()
    local h = NS.IPv4Header.new(IP1, IP2, NS.PROTO_TCP, 60)
    h.header_checksum = 0xDEAD  -- wrong
    assert.is_false(h:verify_checksum())
  end)
end)

-- ============================================================================
-- RoutingTable tests
-- ============================================================================

describe("RoutingTable", function()
  local DEFAULT_NETWORK = { 0, 0, 0, 0 }
  local DEFAULT_MASK    = { 0, 0, 0, 0 }
  local SUBNET_NETWORK  = { 192, 168, 1, 0 }
  local SUBNET_MASK     = { 255, 255, 255, 0 }
  local GW              = { 192, 168, 1, 1 }
  local ZERO            = { 0, 0, 0, 0 }

  it("new creates empty table", function()
    local t = NS.RoutingTable.new()
    assert.is_nil(t:lookup(IP1))
  end)

  it("add_route and lookup match", function()
    local t = NS.RoutingTable.new()
    local t2 = t:add_route(SUBNET_NETWORK, SUBNET_MASK, ZERO, "eth0")
    local route = t2:lookup(IP1)  -- 192.168.1.10 matches 192.168.1.0/24
    assert.is_not_nil(route)
    assert.are.equal(route.iface, "eth0")
  end)

  it("lookup returns nil for no match", function()
    local t = NS.RoutingTable.new()
    local t2 = t:add_route(SUBNET_NETWORK, SUBNET_MASK, ZERO, "eth0")
    assert.is_nil(t2:lookup({ 10, 0, 0, 1 }))
  end)

  it("default route matches anything", function()
    local t = NS.RoutingTable.new()
    local t2 = t:add_route(DEFAULT_NETWORK, DEFAULT_MASK, GW, "eth0")
    local route = t2:lookup({ 8, 8, 8, 8 })
    assert.is_not_nil(route)
  end)

  it("longest prefix match selects more specific route", function()
    local t = NS.RoutingTable.new()
    local t2 = t:add_route(DEFAULT_NETWORK, DEFAULT_MASK, GW, "eth0")
    local t3 = t2:add_route(SUBNET_NETWORK, SUBNET_MASK, ZERO, "eth1")
    local route = t3:lookup(IP1)  -- matches both; /24 wins over /0
    assert.are.equal(route.iface, "eth1")
  end)

  it("add_route is immutable", function()
    local t = NS.RoutingTable.new()
    t:add_route(DEFAULT_NETWORK, DEFAULT_MASK, GW, "eth0")
    assert.is_nil(t:lookup(IP1))
  end)

  it("multiple routes all match correctly", function()
    local t = NS.RoutingTable.new()
    local t2 = t:add_route({ 10, 0, 0, 0 }, { 255, 0, 0, 0 }, GW, "eth0")
    local t3 = t2:add_route({ 10, 1, 0, 0 }, { 255, 255, 0, 0 }, GW, "eth1")
    -- 10.1.5.1 should match /16 (more specific) over /8
    local route = t3:lookup({ 10, 1, 5, 1 })
    assert.are.equal(route.iface, "eth1")
  end)
end)

-- ============================================================================
-- IPLayer tests
-- ============================================================================

describe("IPLayer", function()
  it("new creates layer", function()
    local layer = NS.IPLayer.new(IP1)
    assert.are.same(layer.local_ip, IP1)
  end)

  it("create_packet produces IP bytes with correct header", function()
    local layer = NS.IPLayer.new(IP1)
    local payload = { 1, 2, 3, 4 }
    local bytes = layer:create_packet(IP2, NS.PROTO_UDP, payload)
    assert.are.equal(#bytes, 24)  -- 20 header + 4 payload
    -- First byte should be 0x45
    assert.are.equal(bytes[1], 0x45)
  end)

  it("create_packet has valid checksum", function()
    local layer = NS.IPLayer.new(IP1)
    local bytes = layer:create_packet(IP2, NS.PROTO_TCP, { 10, 20 })
    local header = NS.IPv4Header.deserialize(bytes)
    assert.is_true(header:verify_checksum())
  end)

  it("parse_packet round-trips create_packet", function()
    local layer = NS.IPLayer.new(IP1)
    local payload = { 7, 8, 9 }
    local bytes = layer:create_packet(IP2, NS.PROTO_UDP, payload)
    local status, src_ip, protocol, got_payload = NS.IPLayer.parse_packet(bytes)
    assert.are.equal(status, "ok")
    assert.are.same(src_ip, IP1)
    assert.are.equal(protocol, NS.PROTO_UDP)
    assert.are.same(got_payload, payload)
  end)

  it("parse_packet returns bad_checksum for corrupted packet", function()
    local layer = NS.IPLayer.new(IP1)
    local bytes = layer:create_packet(IP2, NS.PROTO_UDP, { 1, 2, 3 })
    bytes[12] = bytes[12] ~ 0xFF  -- corrupt checksum byte
    local status, _, _, _ = NS.IPLayer.parse_packet(bytes)
    assert.are.equal(status, "bad_checksum")
  end)
end)

-- ============================================================================
-- TCPSegment tests
-- ============================================================================

describe("TCPSegment", function()
  it("new creates segment with defaults", function()
    local seg = NS.TCPSegment.new(12345, 80, 1000, 0, NS.TCP_FLAG_SYN, 65535, {})
    assert.are.equal(seg.src_port, 12345)
    assert.are.equal(seg.dst_port, 80)
    assert.are.equal(seg.seq_num, 1000)
    assert.are.equal(seg.flags, NS.TCP_FLAG_SYN)
  end)

  it("has_flag works", function()
    local seg = NS.TCPSegment.new(1, 2, 0, 0, NS.TCP_FLAG_SYN | NS.TCP_FLAG_ACK, 65535, {})
    assert.is_true(seg:has_flag(NS.TCP_FLAG_SYN))
    assert.is_true(seg:has_flag(NS.TCP_FLAG_ACK))
    assert.is_false(seg:has_flag(NS.TCP_FLAG_FIN))
  end)

  it("serialize produces at least 20 bytes", function()
    local seg = NS.TCPSegment.new(1024, 80, 0, 0, 0, 65535, {})
    local b = seg:serialize()
    assert.is_true(#b >= 20)
  end)

  it("deserialize round-trips serialize", function()
    local payload = { 72, 101, 108, 108, 111 }  -- "Hello"
    local seg1 = NS.TCPSegment.new(54321, 443, 999, 1000, NS.TCP_FLAG_ACK | NS.TCP_FLAG_PSH, 8192, payload)
    local b    = seg1:serialize()
    local seg2 = NS.TCPSegment.deserialize(b)
    assert.are.equal(seg2.src_port, 54321)
    assert.are.equal(seg2.dst_port, 443)
    assert.are.equal(seg2.seq_num, 999)
    assert.are.equal(seg2.ack_num, 1000)
    assert.is_true(seg2:has_flag(NS.TCP_FLAG_ACK))
    assert.is_true(seg2:has_flag(NS.TCP_FLAG_PSH))
    assert.are.same(seg2.payload, payload)
  end)

  it("SYN packet construction", function()
    local seg = NS.TCPSegment.new(60000, 80, 100, 0, NS.TCP_FLAG_SYN, 65535, {})
    local b = seg:serialize()
    local s2 = NS.TCPSegment.deserialize(b)
    assert.is_true(s2:has_flag(NS.TCP_FLAG_SYN))
    assert.is_false(s2:has_flag(NS.TCP_FLAG_ACK))
  end)

  it("FIN-ACK packet", function()
    local seg = NS.TCPSegment.new(1, 2, 0, 0, NS.TCP_FLAG_FIN | NS.TCP_FLAG_ACK, 65535, {})
    assert.is_true(seg:has_flag(NS.TCP_FLAG_FIN))
    assert.is_true(seg:has_flag(NS.TCP_FLAG_ACK))
  end)
end)

-- ============================================================================
-- UDPDatagram tests
-- ============================================================================

describe("UDPDatagram", function()
  it("new creates datagram", function()
    local dg = NS.UDPDatagram.new(12345, 53, { 1, 2, 3 })
    assert.are.equal(dg.src_port, 12345)
    assert.are.equal(dg.dst_port, 53)
    assert.are.equal(dg.length, 11)  -- 8 header + 3 payload
    assert.are.same(dg.payload, { 1, 2, 3 })
  end)

  it("serialize produces at least 8 bytes", function()
    local dg = NS.UDPDatagram.new(1000, 2000, {})
    local b = dg:serialize()
    assert.are.equal(#b, 8)
  end)

  it("deserialize round-trips serialize", function()
    local payload = { 100, 200 }
    local dg1 = NS.UDPDatagram.new(5000, 6000, payload)
    local b   = dg1:serialize()
    local dg2 = NS.UDPDatagram.deserialize(b)
    assert.are.equal(dg2.src_port, 5000)
    assert.are.equal(dg2.dst_port, 6000)
    assert.are.same(dg2.payload, payload)
  end)

  it("empty payload UDP datagram", function()
    local dg = NS.UDPDatagram.new(0, 0, {})
    local b  = dg:serialize()
    local d2 = NS.UDPDatagram.deserialize(b)
    assert.are.same(d2.payload, {})
  end)
end)

-- ============================================================================
-- NetworkStack integration tests
-- ============================================================================

describe("NetworkStack", function()
  it("new creates stack", function()
    local stack = NS.NetworkStack.new(IP1, MAC1)
    assert.are.same(stack.local_ip, IP1)
    assert.are.same(stack.local_mac, MAC1)
  end)

  it("send_udp and receive round-trip", function()
    local stack1 = NS.NetworkStack.new(IP1, MAC1)
    local stack2 = NS.NetworkStack.new(IP2, MAC2)

    -- Stack1 sends UDP to Stack2
    local payload = str_to_bytes("hello")
    local wire = stack1:send_udp(IP2, MAC2, 5000, 9000, payload)

    -- Wire bytes should be non-trivial
    assert.is_true(#wire > 14 + 20 + 8 + #payload)

    -- Stack2 receives
    local status, proto, src_ip, src_port, dst_port, got_payload = stack2:receive(wire)
    assert.are.equal(status, "ok")
    assert.are.equal(proto, "udp")
    assert.are.same(src_ip, IP1)
    assert.are.equal(src_port, 5000)
    assert.are.equal(dst_port, 9000)
    assert.are.same(got_payload, payload)
  end)

  it("send_tcp and receive round-trip", function()
    local stack1 = NS.NetworkStack.new(IP1, MAC1)
    local stack2 = NS.NetworkStack.new(IP2, MAC2)

    local payload = str_to_bytes("world")
    local wire = stack1:send_tcp(IP2, MAC2, 6000, 443, 1000, 2000,
                                 NS.TCP_FLAG_ACK | NS.TCP_FLAG_PSH, payload)

    local status, proto, src_ip, src_port, dst_port, got_payload = stack2:receive(wire)
    assert.are.equal(status, "ok")
    assert.are.equal(proto, "tcp")
    assert.are.same(src_ip, IP1)
    assert.are.equal(src_port, 6000)
    assert.are.equal(dst_port, 443)
    assert.are.same(got_payload, payload)
  end)

  it("receive returns error for too-short frame", function()
    local stack = NS.NetworkStack.new(IP1, MAC1)
    local status, reason, _, _, _, _ = stack:receive({ 1, 2, 3 })
    assert.are.equal(status, "error")
    assert.are.equal(reason, "too_short")
  end)

  it("receive returns error for non-IPv4 ether_type", function()
    local stack = NS.NetworkStack.new(IP1, MAC1)
    local frame = NS.EthernetFrame.new(MAC1, MAC2, NS.ETHERTYPE_ARP, { 1, 2, 3, 4 })
    local wire = frame:serialize()
    local status, reason, _, _, _, _ = stack:receive(wire)
    assert.are.equal(status, "error")
    assert.are.equal(reason, "not_ipv4")
  end)

  it("add_route and add_arp are immutable", function()
    local stack = NS.NetworkStack.new(IP1, MAC1)
    stack:add_route({ 0, 0, 0, 0 }, { 0, 0, 0, 0 }, IP2, "eth0")
    stack:add_arp("192.168.1.20", MAC2)
    -- Original is unchanged
    assert.is_nil(stack.routing_table:lookup(IP2))
    assert.is_nil(stack.arp_table:lookup("192.168.1.20"))
  end)

  it("stack with routing and ARP entries", function()
    local stack = NS.NetworkStack.new(IP1, MAC1)
    local s2 = stack:add_route({ 192, 168, 1, 0 }, { 255, 255, 255, 0 }, { 0, 0, 0, 0 }, "eth0")
    local s3 = s2:add_arp("192.168.1.20", MAC2)
    local route = s3.routing_table:lookup(IP2)
    assert.is_not_nil(route)
    local mac = s3.arp_table:lookup("192.168.1.20")
    assert.are.same(mac, MAC2)
  end)

  it("send and receive multiple packets in sequence", function()
    local sender   = NS.NetworkStack.new(IP1, MAC1)
    local receiver = NS.NetworkStack.new(IP2, MAC2)
    for i = 1, 5 do
      local wire = sender:send_udp(IP2, MAC2, 1000 + i, 2000, { i })
      local st, proto, _, sp, _, pl = receiver:receive(wire)
      assert.are.equal(st, "ok")
      assert.are.equal(proto, "udp")
      assert.are.equal(sp, 1000 + i)
      assert.are.same(pl, { i })
    end
  end)
end)
