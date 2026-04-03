// Package networkstack implements a complete layered networking stack — from raw
// Ethernet frames to HTTP requests — as a teaching tool.
//
// # The Layered Model
//
// Networking is organized in layers, each solving one problem:
//
//	Layer 7: Application (HTTP, DNS)    — "What are we saying?"
//	Layer 4: Transport (TCP, UDP)       — "How do we ensure delivery?"
//	Layer 3: Network (IP)               — "How do we route across networks?"
//	Layer 2: Data Link (Ethernet)       — "How do we talk to the next hop?"
//	Layer 1: Physical (NetworkWire)     — "How do we transmit bits?"
//
// # Ethernet (Layer 2)
//
// Ethernet is the local mail carrier — it delivers frames between devices on
// the same local network segment. Every frame has a destination MAC address
// (who should receive it), a source MAC address (who sent it), an EtherType
// (what protocol the payload contains), and the payload itself.
//
// A MAC address is a 6-byte hardware address burned into every network
// interface card at the factory. The special broadcast address
// FF:FF:FF:FF:FF:FF causes every device on the network to read the frame.
//
// # ARP (Address Resolution Protocol)
//
// ARP bridges IP addresses (Layer 3) and MAC addresses (Layer 2). When a
// host wants to send to IP 10.0.0.5 but doesn't know the MAC, it broadcasts
// "Who has 10.0.0.5?" and the owner replies with its MAC address.
package networkstack

import (
	"encoding/binary"
	"errors"
)

// EtherType constants identify the protocol carried in an Ethernet frame's
// payload. These are standardized by the IEEE.
const (
	EthertypeIPv4 uint16 = 0x0800 // Payload is an IPv4 packet
	EthertypeARP  uint16 = 0x0806 // Payload is an ARP message
)

// EthernetFrame represents a single Ethernet frame — the fundamental unit
// of data on a local network.
//
// Wire format (14-byte header + variable payload):
//
//	+-----------+-----------+------------+---------+
//	| Dest MAC  | Src MAC   | EtherType  | Payload |
//	| (6 bytes) | (6 bytes) | (2 bytes)  | (var)   |
//	+-----------+-----------+------------+---------+
type EthernetFrame struct {
	DestMAC   [6]byte // 6-byte destination hardware address
	SrcMAC    [6]byte // 6-byte source hardware address
	EtherType uint16  // Protocol identifier (0x0800=IPv4, 0x0806=ARP)
	Payload   []byte  // The data carried by this frame
}

// Serialize converts this frame to raw bytes for transmission on the wire.
//
// The format is simply the fields concatenated: dest_mac(6) + src_mac(6) +
// ether_type(2, big-endian) + payload(N).
func (f *EthernetFrame) Serialize() []byte {
	result, _ := StartNew[[]byte]("network-stack.EthernetFrame.Serialize", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			buf := make([]byte, 14+len(f.Payload))
			copy(buf[0:6], f.DestMAC[:])
			copy(buf[6:12], f.SrcMAC[:])
			binary.BigEndian.PutUint16(buf[12:14], f.EtherType)
			copy(buf[14:], f.Payload)
			return rf.Generate(true, false, buf)
		}).GetResult()
	return result
}

// DeserializeEthernetFrame parses raw bytes from the wire into an EthernetFrame.
//
// The minimum frame is 14 bytes (6+6+2, with empty payload). Everything after
// byte 14 is the payload.
func DeserializeEthernetFrame(data []byte) (*EthernetFrame, error) {
	return StartNew[*EthernetFrame]("network-stack.DeserializeEthernetFrame", nil,
		func(op *Operation[*EthernetFrame], rf *ResultFactory[*EthernetFrame]) *OperationResult[*EthernetFrame] {
			if len(data) < 14 {
				return rf.Fail(nil, errors.New("ethernet frame too short: minimum 14 bytes"))
			}
			frame := &EthernetFrame{
				EtherType: binary.BigEndian.Uint16(data[12:14]),
				Payload:   make([]byte, len(data)-14),
			}
			copy(frame.DestMAC[:], data[0:6])
			copy(frame.SrcMAC[:], data[6:12])
			copy(frame.Payload, data[14:])
			return rf.Generate(true, false, frame)
		}).GetResult()
}

// ARPTable maps IP addresses to MAC addresses. This is the ARP cache — it
// remembers which MAC address corresponds to which IP. In a real OS, entries
// expire after a timeout (typically 20 minutes). Our simulation keeps them
// forever.
//
// IP addresses are stored as 32-bit integers. For example, 10.0.0.1 is
// stored as 0x0A000001.
type ARPTable struct {
	entries map[uint32][6]byte
}

// NewARPTable creates a new, empty ARP table.
func NewARPTable() *ARPTable {
	result, _ := StartNew[*ARPTable]("network-stack.NewARPTable", nil,
		func(op *Operation[*ARPTable], rf *ResultFactory[*ARPTable]) *OperationResult[*ARPTable] {
			return rf.Generate(true, false, &ARPTable{entries: make(map[uint32][6]byte)})
		}).GetResult()
	return result
}

// Lookup returns the MAC address for the given IP, and a boolean indicating
// whether the entry exists. Returns false if the IP is not in the table,
// meaning an ARP request would be needed.
func (t *ARPTable) Lookup(ip uint32) ([6]byte, bool) {
	var found bool
	mac, _ := StartNew[[6]byte]("network-stack.ARPTable.Lookup", [6]byte{},
		func(op *Operation[[6]byte], rf *ResultFactory[[6]byte]) *OperationResult[[6]byte] {
			v, ok := t.entries[ip]
			found = ok
			return rf.Generate(true, false, v)
		}).GetResult()
	return mac, found
}

// Update adds or overwrites an IP-to-MAC mapping. Called when we receive an
// ARP reply or see a packet that reveals a (source IP, source MAC) pair.
func (t *ARPTable) Update(ip uint32, mac [6]byte) {
	_, _ = StartNew[struct{}]("network-stack.ARPTable.Update", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			t.entries[ip] = mac
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Entries returns a copy of all entries in the table.
func (t *ARPTable) Entries() map[uint32][6]byte {
	result, _ := StartNew[map[uint32][6]byte]("network-stack.ARPTable.Entries", nil,
		func(op *Operation[map[uint32][6]byte], rf *ResultFactory[map[uint32][6]byte]) *OperationResult[map[uint32][6]byte] {
			copy := make(map[uint32][6]byte, len(t.entries))
			for k, v := range t.entries {
				copy[k] = v
			}
			return rf.Generate(true, false, copy)
		}).GetResult()
	return result
}
