package networkstack

// TCP — Layer 4 (Transport), Reliable Stream
//
// TCP provides a reliable, ordered, byte-stream connection between two
// endpoints. It is the workhorse of the Internet — HTTP, SSH, email, and
// most application protocols run over TCP.
//
// # Why TCP Exists
//
// IP packets can arrive out of order, get duplicated, or be lost. TCP
// solves all three: reliability (every byte is ack'd), ordering (sequence
// numbers), and flow control (window size).
//
// # The Three-Way Handshake
//
//	Client                    Server
//	SYN (seq=1000)    ---->
//	                  <----   SYN+ACK (seq=3000, ack=1001)
//	ACK (ack=3001)    ---->
//	                          ESTABLISHED!
//
// # TCP Header (20 bytes minimum)
//
//	Bytes 0-1:   src_port
//	Bytes 2-3:   dst_port
//	Bytes 4-7:   sequence number
//	Bytes 8-11:  acknowledgment number
//	Byte 12:     data_offset (upper 4 bits)
//	Byte 13:     flags (FIN, SYN, RST, PSH, ACK)
//	Bytes 14-15: window size
//	Bytes 16-17: checksum
//	Bytes 18-19: urgent pointer

import (
	"encoding/binary"
	"errors"
)

// TCP flag constants. Multiple flags can be set simultaneously by OR-ing them
// together (e.g., SYN|ACK for the second step of the handshake).
const (
	TCPFin uint8 = 0x01 // Finish: sender has no more data
	TCPSyn uint8 = 0x02 // Synchronize: initiates connection
	TCPRst uint8 = 0x04 // Reset: abruptly terminates
	TCPPsh uint8 = 0x08 // Push: deliver data immediately
	TCPAck uint8 = 0x10 // Acknowledge: ack_num field is valid
)

// TCPState represents the states of a TCP connection's state machine.
//
// Normal client path:
//
//	CLOSED -> SYN_SENT -> ESTABLISHED -> FIN_WAIT_1 -> FIN_WAIT_2 ->
//	TIME_WAIT -> CLOSED
//
// Normal server path:
//
//	CLOSED -> LISTEN -> SYN_RECEIVED -> ESTABLISHED -> CLOSE_WAIT ->
//	LAST_ACK -> CLOSED
type TCPState int

const (
	StateClosed      TCPState = 0
	StateListen      TCPState = 1
	StateSynSent     TCPState = 2
	StateSynReceived TCPState = 3
	StateEstablished TCPState = 4
	StateFinWait1    TCPState = 5
	StateFinWait2    TCPState = 6
	StateCloseWait   TCPState = 7
	StateClosing     TCPState = 8
	StateLastAck     TCPState = 9
	StateTimeWait    TCPState = 10
)

// TCPHeader represents a 20-byte TCP segment header.
type TCPHeader struct {
	SrcPort    uint16 // Source port (0-65535)
	DstPort    uint16 // Destination port (0-65535)
	SeqNum     uint32 // Sequence number of first byte in payload
	AckNum     uint32 // Next byte expected from remote (if ACK set)
	DataOffset uint8  // Header length in 32-bit words (min 5)
	Flags      uint8  // Combination of TCP flag constants
	WindowSize uint16 // Flow control: bytes sender will accept
	Checksum   uint16 // Error detection (simplified here)
}

// NewTCPHeader creates a header with sensible defaults.
func NewTCPHeader() *TCPHeader {
	result, _ := StartNew[*TCPHeader]("network-stack.NewTCPHeader", nil,
		func(op *Operation[*TCPHeader], rf *ResultFactory[*TCPHeader]) *OperationResult[*TCPHeader] {
			return rf.Generate(true, false, &TCPHeader{
				DataOffset: 5,
				WindowSize: 65535,
			})
		}).GetResult()
	return result
}

// Serialize converts the TCP header to 20 raw bytes.
func (h *TCPHeader) Serialize() []byte {
	result, _ := StartNew[[]byte]("network-stack.TCPHeader.Serialize", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			buf := make([]byte, 20)
			binary.BigEndian.PutUint16(buf[0:2], h.SrcPort)
			binary.BigEndian.PutUint16(buf[2:4], h.DstPort)
			binary.BigEndian.PutUint32(buf[4:8], h.SeqNum)
			binary.BigEndian.PutUint32(buf[8:12], h.AckNum)
			buf[12] = (h.DataOffset << 4) & 0xF0
			buf[13] = h.Flags
			binary.BigEndian.PutUint16(buf[14:16], h.WindowSize)
			binary.BigEndian.PutUint16(buf[16:18], h.Checksum)
			binary.BigEndian.PutUint16(buf[18:20], 0) // urgent pointer
			return rf.Generate(true, false, buf)
		}).GetResult()
	return result
}

// DeserializeTCPHeader parses 20 bytes into a TCPHeader.
func DeserializeTCPHeader(data []byte) (*TCPHeader, error) {
	if len(data) < 20 {
		return nil, errors.New("TCP header too short: minimum 20 bytes")
	}

	return &TCPHeader{
		SrcPort:    binary.BigEndian.Uint16(data[0:2]),
		DstPort:    binary.BigEndian.Uint16(data[2:4]),
		SeqNum:     binary.BigEndian.Uint32(data[4:8]),
		AckNum:     binary.BigEndian.Uint32(data[8:12]),
		DataOffset: (data[12] >> 4) & 0x0F,
		Flags:      data[13],
		WindowSize: binary.BigEndian.Uint16(data[14:16]),
		Checksum:   binary.BigEndian.Uint16(data[16:18]),
	}, nil
}

// TCPConnection manages the full lifecycle of a single TCP connection —
// from handshake through data transfer to close.
type TCPConnection struct {
	State      TCPState
	LocalPort  uint16
	RemoteIP   uint32
	RemotePort uint16
	SeqNum     uint32 // Our next sequence number
	AckNum     uint32 // What we've ack'd from remote
	SendBuffer []byte
	RecvBuffer []byte
}

// NewTCPConnection creates a new connection in CLOSED state.
func NewTCPConnection(localPort uint16, remoteIP uint32, remotePort uint16) *TCPConnection {
	return &TCPConnection{
		State:      StateClosed,
		LocalPort:  localPort,
		RemoteIP:   remoteIP,
		RemotePort: remotePort,
	}
}

// InitiateConnect begins the three-way handshake (client side).
// Transitions: CLOSED -> SYN_SENT. Returns the SYN segment to send.
func (c *TCPConnection) InitiateConnect() *TCPHeader {
	c.SeqNum = 1000 // Initial sequence number
	c.State = StateSynSent

	syn := &TCPHeader{
		SrcPort:    c.LocalPort,
		DstPort:    c.RemotePort,
		SeqNum:     c.SeqNum,
		Flags:      TCPSyn,
		DataOffset: 5,
		WindowSize: 65535,
	}
	c.SeqNum++ // SYN consumes one sequence number
	return syn
}

// InitiateListen transitions to LISTEN state (server side).
func (c *TCPConnection) InitiateListen() {
	c.State = StateListen
}

// HandleSegment processes an incoming TCP segment and returns a response
// (or nil if no response is needed). This is the heart of the TCP state
// machine.
func (c *TCPConnection) HandleSegment(header *TCPHeader, payload []byte) *TCPHeader {
	flags := header.Flags

	switch c.State {

	// LISTEN: waiting for incoming connections
	case StateListen:
		if flags&TCPSyn != 0 {
			c.RemotePort = header.SrcPort
			c.AckNum = header.SeqNum + 1
			c.SeqNum = 3000 // Server ISN
			c.State = StateSynReceived

			synack := &TCPHeader{
				SrcPort:    c.LocalPort,
				DstPort:    c.RemotePort,
				SeqNum:     c.SeqNum,
				AckNum:     c.AckNum,
				Flags:      TCPSyn | TCPAck,
				DataOffset: 5,
				WindowSize: 65535,
			}
			c.SeqNum++
			return synack
		}

	// SYN_SENT: client waiting for SYN+ACK
	case StateSynSent:
		if flags&TCPSyn != 0 && flags&TCPAck != 0 {
			c.AckNum = header.SeqNum + 1
			c.State = StateEstablished

			return &TCPHeader{
				SrcPort:    c.LocalPort,
				DstPort:    c.RemotePort,
				SeqNum:     c.SeqNum,
				AckNum:     c.AckNum,
				Flags:      TCPAck,
				DataOffset: 5,
				WindowSize: 65535,
			}
		}

	// SYN_RECEIVED: server waiting for final ACK
	case StateSynReceived:
		if flags&TCPAck != 0 {
			c.State = StateEstablished
			return nil
		}

	// ESTABLISHED: connection open, data can flow
	case StateEstablished:
		if flags&TCPFin != 0 {
			c.AckNum = header.SeqNum + 1
			c.State = StateCloseWait
			return &TCPHeader{
				SrcPort:    c.LocalPort,
				DstPort:    c.RemotePort,
				SeqNum:     c.SeqNum,
				AckNum:     c.AckNum,
				Flags:      TCPAck,
				DataOffset: 5,
				WindowSize: 65535,
			}
		}
		if len(payload) > 0 {
			c.RecvBuffer = append(c.RecvBuffer, payload...)
			c.AckNum = header.SeqNum + uint32(len(payload))
			return &TCPHeader{
				SrcPort:    c.LocalPort,
				DstPort:    c.RemotePort,
				SeqNum:     c.SeqNum,
				AckNum:     c.AckNum,
				Flags:      TCPAck,
				DataOffset: 5,
				WindowSize: 65535,
			}
		}
		return nil

	// FIN_WAIT_1: we sent FIN, waiting for ACK
	case StateFinWait1:
		if flags&TCPFin != 0 && flags&TCPAck != 0 {
			c.AckNum = header.SeqNum + 1
			c.State = StateTimeWait
			return &TCPHeader{
				SrcPort:    c.LocalPort,
				DstPort:    c.RemotePort,
				SeqNum:     c.SeqNum,
				AckNum:     c.AckNum,
				Flags:      TCPAck,
				DataOffset: 5,
				WindowSize: 65535,
			}
		}
		if flags&TCPAck != 0 {
			c.State = StateFinWait2
			return nil
		}
		if flags&TCPFin != 0 {
			c.AckNum = header.SeqNum + 1
			c.State = StateClosing
			return &TCPHeader{
				SrcPort:    c.LocalPort,
				DstPort:    c.RemotePort,
				SeqNum:     c.SeqNum,
				AckNum:     c.AckNum,
				Flags:      TCPAck,
				DataOffset: 5,
				WindowSize: 65535,
			}
		}

	// FIN_WAIT_2: our FIN was ACK'd, waiting for remote FIN
	case StateFinWait2:
		if flags&TCPFin != 0 {
			c.AckNum = header.SeqNum + 1
			c.State = StateTimeWait
			return &TCPHeader{
				SrcPort:    c.LocalPort,
				DstPort:    c.RemotePort,
				SeqNum:     c.SeqNum,
				AckNum:     c.AckNum,
				Flags:      TCPAck,
				DataOffset: 5,
				WindowSize: 65535,
			}
		}

	// CLOSING: both sides sent FIN
	case StateClosing:
		if flags&TCPAck != 0 {
			c.State = StateTimeWait
			return nil
		}

	// LAST_ACK: sent FIN from CLOSE_WAIT, waiting for ACK
	case StateLastAck:
		if flags&TCPAck != 0 {
			c.State = StateClosed
			return nil
		}

	// TIME_WAIT: waiting for stale segments to expire
	case StateTimeWait:
		return nil

	case StateCloseWait:
		return nil
	}

	return nil
}

// Send queues data for sending. Returns a segment header if the connection
// is ESTABLISHED, nil otherwise. The caller must transmit the returned
// header along with the data as payload.
func (c *TCPConnection) Send(data []byte) *TCPHeader {
	c.SendBuffer = append(c.SendBuffer, data...)

	if c.State != StateEstablished {
		return nil
	}

	seg := &TCPHeader{
		SrcPort:    c.LocalPort,
		DstPort:    c.RemotePort,
		SeqNum:     c.SeqNum,
		AckNum:     c.AckNum,
		Flags:      TCPPsh | TCPAck,
		DataOffset: 5,
		WindowSize: 65535,
	}
	c.SeqNum += uint32(len(data))
	return seg
}

// Receive reads up to count bytes from the receive buffer. The bytes are
// removed from the buffer (they can only be read once).
func (c *TCPConnection) Receive(count int) []byte {
	if count > len(c.RecvBuffer) {
		count = len(c.RecvBuffer)
	}
	result := make([]byte, count)
	copy(result, c.RecvBuffer[:count])
	c.RecvBuffer = c.RecvBuffer[count:]
	return result
}

// InitiateClose begins connection teardown by sending a FIN.
//
// Transitions:
//   - ESTABLISHED -> FIN_WAIT_1 (active close)
//   - CLOSE_WAIT -> LAST_ACK (passive close)
func (c *TCPConnection) InitiateClose() *TCPHeader {
	switch c.State {
	case StateEstablished:
		c.State = StateFinWait1
	case StateCloseWait:
		c.State = StateLastAck
	default:
		return nil
	}

	fin := &TCPHeader{
		SrcPort:    c.LocalPort,
		DstPort:    c.RemotePort,
		SeqNum:     c.SeqNum,
		AckNum:     c.AckNum,
		Flags:      TCPFin | TCPAck,
		DataOffset: 5,
		WindowSize: 65535,
	}
	c.SeqNum++ // FIN consumes one sequence number
	return fin
}
