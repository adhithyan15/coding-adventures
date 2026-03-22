package networkstack

import (
	"testing"
)

func TestTCPHeaderRoundtrip(t *testing.T) {
	h := &TCPHeader{
		SrcPort: 49152, DstPort: 80, SeqNum: 1000, AckNum: 2000,
		DataOffset: 5, Flags: TCPSyn | TCPAck, WindowSize: 32768,
	}
	raw := h.Serialize()
	r, err := DeserializeTCPHeader(raw)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if r.SrcPort != 49152 || r.DstPort != 80 {
		t.Errorf("port mismatch")
	}
	if r.SeqNum != 1000 || r.AckNum != 2000 {
		t.Errorf("seq/ack mismatch")
	}
	if r.Flags != TCPSyn|TCPAck {
		t.Errorf("flags mismatch: got %d", r.Flags)
	}
	if r.WindowSize != 32768 {
		t.Errorf("window mismatch")
	}
}

func TestTCPHeaderLength(t *testing.T) {
	h := NewTCPHeader()
	if len(h.Serialize()) != 20 {
		t.Errorf("expected 20 bytes")
	}
}

func TestTCPDeserializeTooShort(t *testing.T) {
	_, err := DeserializeTCPHeader(make([]byte, 19))
	if err == nil {
		t.Error("expected error")
	}
}

func TestTCPFlagCombinations(t *testing.T) {
	for _, flags := range []uint8{TCPSyn, TCPAck, TCPFin, TCPPsh | TCPAck, TCPSyn | TCPAck, TCPFin | TCPAck} {
		h := &TCPHeader{Flags: flags, DataOffset: 5, WindowSize: 65535}
		raw := h.Serialize()
		r, _ := DeserializeTCPHeader(raw)
		if r.Flags != flags {
			t.Errorf("flags mismatch: want %d, got %d", flags, r.Flags)
		}
	}
}

func TestTCPLargeSeqNum(t *testing.T) {
	h := &TCPHeader{SeqNum: 0xFFFFFFFF, DataOffset: 5, WindowSize: 65535}
	raw := h.Serialize()
	r, _ := DeserializeTCPHeader(raw)
	if r.SeqNum != 0xFFFFFFFF {
		t.Errorf("large seq mismatch")
	}
}

func establishConnection(t *testing.T) (*TCPConnection, *TCPConnection) {
	t.Helper()
	client := NewTCPConnection(49152, 0x0A000002, 80)
	server := NewTCPConnection(80, 0, 0)
	server.InitiateListen()

	syn := client.InitiateConnect()
	synack := server.HandleSegment(syn, nil)
	if synack == nil {
		t.Fatal("expected SYN+ACK")
	}
	ack := client.HandleSegment(synack, nil)
	if ack == nil {
		t.Fatal("expected ACK")
	}
	server.HandleSegment(ack, nil)

	if client.State != StateEstablished {
		t.Fatalf("client not established")
	}
	if server.State != StateEstablished {
		t.Fatalf("server not established")
	}
	return client, server
}

func TestTCPClientSendsSYN(t *testing.T) {
	c := NewTCPConnection(49152, 0x0A000002, 80)
	syn := c.InitiateConnect()
	if c.State != StateSynSent {
		t.Errorf("expected SYN_SENT")
	}
	if syn.Flags != TCPSyn {
		t.Errorf("expected SYN flag")
	}
}

func TestTCPServerRespondsSYNACK(t *testing.T) {
	s := NewTCPConnection(80, 0, 0)
	s.InitiateListen()

	syn := &TCPHeader{SrcPort: 49152, DstPort: 80, SeqNum: 1000, Flags: TCPSyn, DataOffset: 5, WindowSize: 65535}
	synack := s.HandleSegment(syn, nil)
	if s.State != StateSynReceived {
		t.Errorf("expected SYN_RECEIVED")
	}
	if synack == nil {
		t.Fatal("expected SYN+ACK")
	}
	if synack.Flags != TCPSyn|TCPAck {
		t.Errorf("expected SYN+ACK flags")
	}
	if synack.AckNum != 1001 {
		t.Errorf("ack should be 1001")
	}
}

func TestTCPFullHandshake(t *testing.T) {
	client, server := establishConnection(t)
	_ = client
	_ = server
}

func TestTCPSendData(t *testing.T) {
	client, _ := establishConnection(t)
	seg := client.Send([]byte("Hello"))
	if seg == nil {
		t.Fatal("expected segment")
	}
	if seg.Flags != TCPPsh|TCPAck {
		t.Errorf("expected PSH+ACK")
	}
}

func TestTCPReceiveData(t *testing.T) {
	client, server := establishConnection(t)
	seg := client.Send([]byte("Hello, server!"))
	ack := server.HandleSegment(seg, []byte("Hello, server!"))
	if ack == nil {
		t.Fatal("expected ACK")
	}
	data := server.Receive(100)
	if string(data) != "Hello, server!" {
		t.Errorf("got %q", string(data))
	}
}

func TestTCPReceiveEmpty(t *testing.T) {
	client, _ := establishConnection(t)
	data := client.Receive(100)
	if len(data) != 0 {
		t.Errorf("expected empty")
	}
}

func TestTCPSendNotEstablished(t *testing.T) {
	c := NewTCPConnection(49152, 0, 0)
	seg := c.Send([]byte("data"))
	if seg != nil {
		t.Error("expected nil for non-established")
	}
}

func TestTCPPartialReceive(t *testing.T) {
	client, server := establishConnection(t)
	seg := client.Send([]byte("Hello, World!"))
	server.HandleSegment(seg, []byte("Hello, World!"))

	part1 := server.Receive(5)
	if string(part1) != "Hello" {
		t.Errorf("part1: got %q", string(part1))
	}
	part2 := server.Receive(100)
	if string(part2) != ", World!" {
		t.Errorf("part2: got %q", string(part2))
	}
}

func TestTCPActiveClose(t *testing.T) {
	client, _ := establishConnection(t)
	fin := client.InitiateClose()
	if fin == nil {
		t.Fatal("expected FIN")
	}
	if client.State != StateFinWait1 {
		t.Errorf("expected FIN_WAIT_1")
	}
	if fin.Flags != TCPFin|TCPAck {
		t.Errorf("expected FIN+ACK")
	}
}

func TestTCPFullCloseSequence(t *testing.T) {
	client, server := establishConnection(t)

	fin1 := client.InitiateClose()
	ack1 := server.HandleSegment(fin1, nil)
	if server.State != StateCloseWait {
		t.Errorf("server should be CLOSE_WAIT")
	}

	client.HandleSegment(ack1, nil)
	if client.State != StateFinWait2 {
		t.Errorf("client should be FIN_WAIT_2")
	}

	fin2 := server.InitiateClose()
	if server.State != StateLastAck {
		t.Errorf("server should be LAST_ACK")
	}

	ack2 := client.HandleSegment(fin2, nil)
	if client.State != StateTimeWait {
		t.Errorf("client should be TIME_WAIT")
	}

	server.HandleSegment(ack2, nil)
	if server.State != StateClosed {
		t.Errorf("server should be CLOSED")
	}
}

func TestTCPCloseFromClosed(t *testing.T) {
	c := NewTCPConnection(49152, 0, 0)
	fin := c.InitiateClose()
	if fin != nil {
		t.Error("expected nil from CLOSED state")
	}
}

func TestTCPInitialState(t *testing.T) {
	c := NewTCPConnection(80, 0, 0)
	if c.State != StateClosed {
		t.Errorf("expected CLOSED")
	}
}

func TestTCPListenTransition(t *testing.T) {
	c := NewTCPConnection(80, 0, 0)
	c.InitiateListen()
	if c.State != StateListen {
		t.Errorf("expected LISTEN")
	}
}

func TestTCPClosingState(t *testing.T) {
	c := NewTCPConnection(80, 0, 0)
	c.State = StateClosing
	c.HandleSegment(&TCPHeader{Flags: TCPAck}, nil)
	if c.State != StateTimeWait {
		t.Errorf("expected TIME_WAIT")
	}
}

func TestTCPLastAckState(t *testing.T) {
	c := NewTCPConnection(80, 0, 0)
	c.State = StateLastAck
	c.HandleSegment(&TCPHeader{Flags: TCPAck}, nil)
	if c.State != StateClosed {
		t.Errorf("expected CLOSED")
	}
}

func TestTCPTimeWaitReturnsNil(t *testing.T) {
	c := NewTCPConnection(80, 0, 0)
	c.State = StateTimeWait
	result := c.HandleSegment(&TCPHeader{Flags: TCPAck}, nil)
	if result != nil {
		t.Error("expected nil")
	}
}

func TestTCPFinWait1ReceivesFINACK(t *testing.T) {
	client, server := establishConnection(t)
	client.InitiateClose()

	finack := &TCPHeader{
		SrcPort: 80, DstPort: 49152,
		SeqNum: server.SeqNum, AckNum: client.SeqNum,
		Flags: TCPFin | TCPAck, DataOffset: 5, WindowSize: 65535,
	}
	result := client.HandleSegment(finack, nil)
	if result == nil {
		t.Fatal("expected ACK response")
	}
	if client.State != StateTimeWait {
		t.Errorf("expected TIME_WAIT, got %d", client.State)
	}
}
