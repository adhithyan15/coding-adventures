package networkstack

// IPv4 — Layer 3 (Network)
//
// IP (Internet Protocol) is the routing layer. While Ethernet delivers frames
// between devices on the same local network, IP delivers packets across
// networks — from your laptop to a server on the other side of the world.
//
// # How IP Routing Works
//
// Imagine sending a letter from New York to Tokyo. It goes through a chain of
// post offices: local -> regional -> airport -> Tokyo airport -> local office ->
// recipient. At each hop, the postal worker looks at the destination address and
// decides which direction to forward. That's routing.
//
// # IPv4 Header Format (20 bytes minimum)
//
//	Byte 0:    version(4) | ihl(4)
//	Byte 1:    type of service (unused)
//	Bytes 2-3: total_length
//	Bytes 4-5: identification
//	Bytes 6-7: flags/fragment offset (unused)
//	Byte 8:    ttl (time to live)
//	Byte 9:    protocol (6=TCP, 17=UDP)
//	Bytes 10-11: header checksum
//	Bytes 12-15: source IP
//	Bytes 16-19: destination IP
//
// # The Checksum Algorithm (RFC 1071)
//
// 1. Treat the header as 16-bit words
// 2. Sum them all (with checksum field = 0)
// 3. Fold carries: add high 16 bits to low 16 bits
// 4. One's complement (flip all bits)

import (
	"encoding/binary"
	"errors"
	"math/bits"
)

// Protocol numbers for the IPv4 header's protocol field.
const (
	ProtocolTCP uint8 = 6  // Payload is a TCP segment
	ProtocolUDP uint8 = 17 // Payload is a UDP datagram
)

// IPv4Header represents a 20-byte IPv4 packet header containing routing and
// control information.
type IPv4Header struct {
	Version        uint8  // IP version, always 4
	IHL            uint8  // Internet Header Length in 32-bit words (min 5)
	TotalLength    uint16 // Total packet size (header + payload) in bytes
	Identification uint16 // Used for fragment reassembly
	TTL            uint8  // Time To Live — decremented at each router
	Protocol       uint8  // Layer 4 protocol: 6=TCP, 17=UDP
	Checksum       uint16 // Header checksum for error detection
	SrcIP          uint32 // Source IP as 32-bit int (e.g., 10.0.0.1 = 0x0A000001)
	DstIP          uint32 // Destination IP as 32-bit int
}

// NewIPv4Header creates a header with sensible defaults.
func NewIPv4Header() *IPv4Header {
	result, _ := StartNew[*IPv4Header]("network-stack.NewIPv4Header", nil,
		func(op *Operation[*IPv4Header], rf *ResultFactory[*IPv4Header]) *OperationResult[*IPv4Header] {
			return rf.Generate(true, false, &IPv4Header{
				Version:     4,
				IHL:         5,
				TotalLength: 20,
				TTL:         64,
				Protocol:    ProtocolTCP,
			})
		}).GetResult()
	return result
}

// Serialize converts the header to 20 raw bytes in network byte order.
// It computes and embeds the checksum automatically.
func (h *IPv4Header) Serialize() []byte {
	result, _ := StartNew[[]byte]("network-stack.IPv4Header.Serialize", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			buf := make([]byte, 20)

			// Pack version (upper 4 bits) and IHL (lower 4 bits) into byte 0
			buf[0] = (h.Version << 4) | h.IHL
			buf[1] = 0 // Type of service (unused)
			binary.BigEndian.PutUint16(buf[2:4], h.TotalLength)
			binary.BigEndian.PutUint16(buf[4:6], h.Identification)
			binary.BigEndian.PutUint16(buf[6:8], 0) // Flags + fragment offset
			buf[8] = h.TTL
			buf[9] = h.Protocol
			binary.BigEndian.PutUint16(buf[10:12], 0) // Checksum placeholder
			binary.BigEndian.PutUint32(buf[12:16], h.SrcIP)
			binary.BigEndian.PutUint32(buf[16:20], h.DstIP)

			// Compute checksum over the header with checksum field = 0
			h.Checksum = computeChecksum(buf)
			binary.BigEndian.PutUint16(buf[10:12], h.Checksum)

			return rf.Generate(true, false, buf)
		}).GetResult()
	return result
}

// DeserializeIPv4Header parses 20 bytes into an IPv4Header.
func DeserializeIPv4Header(data []byte) (*IPv4Header, error) {
	return StartNew[*IPv4Header]("network-stack.DeserializeIPv4Header", nil,
		func(op *Operation[*IPv4Header], rf *ResultFactory[*IPv4Header]) *OperationResult[*IPv4Header] {
			if len(data) < 20 {
				return rf.Fail(nil, errors.New("IPv4 header too short: minimum 20 bytes"))
			}
			h := &IPv4Header{
				Version:        (data[0] >> 4) & 0x0F,
				IHL:            data[0] & 0x0F,
				TotalLength:    binary.BigEndian.Uint16(data[2:4]),
				Identification: binary.BigEndian.Uint16(data[4:6]),
				TTL:            data[8],
				Protocol:       data[9],
				Checksum:       binary.BigEndian.Uint16(data[10:12]),
				SrcIP:          binary.BigEndian.Uint32(data[12:16]),
				DstIP:          binary.BigEndian.Uint32(data[16:20]),
			}
			return rf.Generate(true, false, h)
		}).GetResult()
}

// ComputeChecksum calculates the Internet checksum for this header.
func (h *IPv4Header) ComputeChecksum() uint16 {
	result, _ := StartNew[uint16]("network-stack.IPv4Header.ComputeChecksum", 0,
		func(op *Operation[uint16], rf *ResultFactory[uint16]) *OperationResult[uint16] {
			buf := make([]byte, 20)
			buf[0] = (h.Version << 4) | h.IHL
			buf[1] = 0
			binary.BigEndian.PutUint16(buf[2:4], h.TotalLength)
			binary.BigEndian.PutUint16(buf[4:6], h.Identification)
			binary.BigEndian.PutUint16(buf[6:8], 0)
			buf[8] = h.TTL
			buf[9] = h.Protocol
			binary.BigEndian.PutUint16(buf[10:12], 0) // checksum = 0 for computation
			binary.BigEndian.PutUint32(buf[12:16], h.SrcIP)
			binary.BigEndian.PutUint32(buf[16:20], h.DstIP)
			return rf.Generate(true, false, computeChecksum(buf))
		}).GetResult()
	return result
}

// computeChecksum implements the Internet checksum algorithm (RFC 1071).
//
// The algorithm:
// 1. Sum all 16-bit words in the data
// 2. Fold carries: add high 16 bits to low 16 bits, repeat until no carry
// 3. Return the one's complement (bitwise NOT)
func computeChecksum(data []byte) uint16 {
	var total uint32

	for i := 0; i+1 < len(data); i += 2 {
		total += uint32(binary.BigEndian.Uint16(data[i : i+2]))
	}
	// Handle odd byte
	if len(data)%2 == 1 {
		total += uint32(data[len(data)-1]) << 8
	}

	// Fold carries
	for total > 0xFFFF {
		total = (total >> 16) + (total & 0xFFFF)
	}

	return uint16(^total & 0xFFFF)
}

// RoutingTable implements longest-prefix-match IP routing.
//
// Each route is a rule: "if (dest_ip & mask) == (network & mask), send it
// to gateway via interface." When multiple rules match, the one with the
// longest prefix (most 1-bits in the mask) wins.
//
// Example:
//
//	Network        Mask             Gateway      Interface
//	10.0.0.0       255.255.255.0    0.0.0.0      eth0    (direct)
//	0.0.0.0        0.0.0.0          10.0.0.1     eth0    (default route)
type RoutingTable struct {
	routes []route
}

type route struct {
	network   uint32
	mask      uint32
	gateway   uint32
	iface     string
}

// NewRoutingTable creates a new, empty routing table.
func NewRoutingTable() *RoutingTable {
	result, _ := StartNew[*RoutingTable]("network-stack.NewRoutingTable", nil,
		func(op *Operation[*RoutingTable], rf *ResultFactory[*RoutingTable]) *OperationResult[*RoutingTable] {
			return rf.Generate(true, false, &RoutingTable{})
		}).GetResult()
	return result
}

// AddRoute adds a routing rule.
//
// Parameters:
//   - network: Network address (e.g., 0x0A000000 for 10.0.0.0)
//   - mask: Subnet mask (e.g., 0xFFFFFF00 for /24)
//   - gateway: Next-hop IP (0 for directly connected networks)
//   - iface: Outgoing interface name
func (rt *RoutingTable) AddRoute(network, mask, gateway uint32, iface string) {
	_, _ = StartNew[struct{}]("network-stack.RoutingTable.AddRoute", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("iface", iface)
			rt.routes = append(rt.routes, route{network, mask, gateway, iface})
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Lookup finds the best route for a destination IP.
//
// Returns (nextHop, interface, found). If gateway is 0 (directly connected),
// nextHop is the destination IP itself. Returns found=false if no route matches.
func (rt *RoutingTable) Lookup(destIP uint32) (uint32, string, bool) {
	type lookupResult struct {
		hop   uint32
		iface string
		found bool
	}
	lr, _ := StartNew[lookupResult]("network-stack.RoutingTable.Lookup", lookupResult{},
		func(op *Operation[lookupResult], rf *ResultFactory[lookupResult]) *OperationResult[lookupResult] {
			bestMask := -1
			var bestHop uint32
			var bestIface string
			found := false

			for _, r := range rt.routes {
				if (destIP & r.mask) == (r.network & r.mask) {
					maskBits := bits.OnesCount32(r.mask)
					if maskBits > bestMask {
						bestMask = maskBits
						if r.gateway != 0 {
							bestHop = r.gateway
						} else {
							bestHop = destIP
						}
						bestIface = r.iface
						found = true
					}
				}
			}

			return rf.Generate(true, false, lookupResult{hop: bestHop, iface: bestIface, found: found})
		}).GetResult()
	return lr.hop, lr.iface, lr.found
}

// IPLayer creates outgoing IP packets and parses incoming ones.
// It is the glue between transport (TCP/UDP) and data link (Ethernet).
type IPLayer struct {
	LocalIP      uint32
	RoutingTable *RoutingTable
}

// NewIPLayer creates an IP layer with the given local address and routing table.
func NewIPLayer(localIP uint32, rt *RoutingTable) *IPLayer {
	result, _ := StartNew[*IPLayer]("network-stack.NewIPLayer", nil,
		func(op *Operation[*IPLayer], rf *ResultFactory[*IPLayer]) *OperationResult[*IPLayer] {
			return rf.Generate(true, false, &IPLayer{LocalIP: localIP, RoutingTable: rt})
		}).GetResult()
	return result
}

// CreatePacket builds an IP packet with the given destination, protocol, and payload.
func (l *IPLayer) CreatePacket(destIP uint32, protocol uint8, payload []byte) []byte {
	result, _ := StartNew[[]byte]("network-stack.IPLayer.CreatePacket", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			h := &IPv4Header{
				Version:     4,
				IHL:         5,
				TotalLength: uint16(20 + len(payload)),
				TTL:         64,
				Protocol:    protocol,
				SrcIP:       l.LocalIP,
				DstIP:       destIP,
			}
			header := h.Serialize()
			pkt := make([]byte, len(header)+len(payload))
			copy(pkt, header)
			copy(pkt[len(header):], payload)
			return rf.Generate(true, false, pkt)
		}).GetResult()
	return result
}

// ParsePacket splits a received IP packet into header and payload.
// Returns nil header if the data is too short.
func (l *IPLayer) ParsePacket(data []byte) (*IPv4Header, []byte) {
	type parseResult struct {
		header  *IPv4Header
		payload []byte
	}
	pr, _ := StartNew[parseResult]("network-stack.IPLayer.ParsePacket", parseResult{},
		func(op *Operation[parseResult], rf *ResultFactory[parseResult]) *OperationResult[parseResult] {
			if len(data) < 20 {
				return rf.Generate(true, false, parseResult{nil, nil})
			}
			h, err := DeserializeIPv4Header(data)
			if err != nil {
				return rf.Generate(true, false, parseResult{nil, nil})
			}
			offset := int(h.IHL) * 4
			if offset > len(data) {
				offset = len(data)
			}
			return rf.Generate(true, false, parseResult{h, data[offset:]})
		}).GetResult()
	return pr.header, pr.payload
}
