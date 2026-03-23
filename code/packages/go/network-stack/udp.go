package networkstack

// UDP — Layer 4 (Transport), Unreliable Datagrams
//
// UDP is the simpler sibling of TCP. Where TCP is registered mail with
// tracking, UDP is a postcard — fast, simple, and unreliable.
//
// No connection setup, no acknowledgments, no retransmissions. You write
// the message and drop it in the mailbox. It might get lost.
//
// Used for DNS (single question-answer), video streaming (a lost frame is
// better than a delayed frame), and online gaming (newest position update
// is all that matters).
//
// # UDP Header (8 bytes)
//
//	Bytes 0-1: source port
//	Bytes 2-3: destination port
//	Bytes 4-5: length (header + payload)
//	Bytes 6-7: checksum

import (
	"encoding/binary"
	"errors"
)

// UDPHeader represents an 8-byte UDP datagram header.
type UDPHeader struct {
	SrcPort  uint16 // Source port (can be 0 if no reply expected)
	DstPort  uint16 // Destination port
	Length   uint16 // Total datagram size (header + payload), minimum 8
	Checksum uint16 // Error detection (0 = "not computed" in IPv4)
}

// NewUDPHeader creates a header with default values (length=8, ports=0).
func NewUDPHeader() *UDPHeader {
	return &UDPHeader{Length: 8}
}

// Serialize converts the header to 8 raw bytes in network byte order.
func (h *UDPHeader) Serialize() []byte {
	buf := make([]byte, 8)
	binary.BigEndian.PutUint16(buf[0:2], h.SrcPort)
	binary.BigEndian.PutUint16(buf[2:4], h.DstPort)
	binary.BigEndian.PutUint16(buf[4:6], h.Length)
	binary.BigEndian.PutUint16(buf[6:8], h.Checksum)
	return buf
}

// DeserializeUDPHeader parses 8 bytes into a UDPHeader.
func DeserializeUDPHeader(data []byte) (*UDPHeader, error) {
	if len(data) < 8 {
		return nil, errors.New("UDP header too short: minimum 8 bytes")
	}
	return &UDPHeader{
		SrcPort:  binary.BigEndian.Uint16(data[0:2]),
		DstPort:  binary.BigEndian.Uint16(data[2:4]),
		Length:   binary.BigEndian.Uint16(data[4:6]),
		Checksum: binary.BigEndian.Uint16(data[6:8]),
	}, nil
}

// Datagram represents a received UDP datagram with its source address.
type Datagram struct {
	Data    []byte
	SrcIP   uint32
	SrcPort uint16
}

// UDPSocket sends and receives individual datagrams. Unlike TCP, there is
// no connection — each datagram is independent. Think of it like a mailbox:
// you can send to anyone and receive from anyone.
type UDPSocket struct {
	LocalPort uint16
	recvQueue []Datagram
}

// NewUDPSocket creates a UDP socket bound to the given port.
func NewUDPSocket(localPort uint16) *UDPSocket {
	return &UDPSocket{LocalPort: localPort}
}

// SendTo creates a UDP datagram for the given destination.
// Returns the header and payload. The caller (IP layer) handles transmission.
func (s *UDPSocket) SendTo(data []byte, destIP uint32, destPort uint16) (*UDPHeader, []byte) {
	header := &UDPHeader{
		SrcPort: s.LocalPort,
		DstPort: destPort,
		Length:  uint16(8 + len(data)),
	}
	return header, data
}

// ReceiveFrom returns the next datagram from the queue, or nil if empty.
// Unlike TCP's Receive, this returns the sender's address because UDP is
// connectionless.
func (s *UDPSocket) ReceiveFrom() *Datagram {
	if len(s.recvQueue) == 0 {
		return nil
	}
	d := s.recvQueue[0]
	s.recvQueue = s.recvQueue[1:]
	return &d
}

// Deliver places a received datagram into this socket's receive queue.
// This simulates the kernel's job of demultiplexing incoming datagrams.
func (s *UDPSocket) Deliver(data []byte, srcIP uint32, srcPort uint16) {
	s.recvQueue = append(s.recvQueue, Datagram{
		Data:    data,
		SrcIP:   srcIP,
		SrcPort: srcPort,
	})
}

// HasData returns true if there are datagrams waiting to be read.
func (s *UDPSocket) HasData() bool {
	return len(s.recvQueue) > 0
}
