package networkstack

import (
	"bytes"
	"testing"
)

func TestUDPHeaderRoundtrip(t *testing.T) {
	h := &UDPHeader{SrcPort: 12345, DstPort: 53, Length: 42}
	raw := h.Serialize()
	r, err := DeserializeUDPHeader(raw)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if r.SrcPort != 12345 || r.DstPort != 53 || r.Length != 42 {
		t.Errorf("field mismatch")
	}
}

func TestUDPHeaderLength(t *testing.T) {
	h := NewUDPHeader()
	if len(h.Serialize()) != 8 {
		t.Errorf("expected 8 bytes")
	}
}

func TestUDPDeserializeTooShort(t *testing.T) {
	_, err := DeserializeUDPHeader(make([]byte, 7))
	if err == nil {
		t.Error("expected error")
	}
}

func TestUDPDefaults(t *testing.T) {
	h := NewUDPHeader()
	if h.SrcPort != 0 || h.DstPort != 0 || h.Length != 8 || h.Checksum != 0 {
		t.Errorf("unexpected defaults")
	}
}

func TestUDPSocketSendTo(t *testing.T) {
	sock := NewUDPSocket(12345)
	header, payload := sock.SendTo([]byte("hello"), 0x08080808, 53)
	if header.SrcPort != 12345 || header.DstPort != 53 {
		t.Errorf("port mismatch")
	}
	if header.Length != 13 { // 8 + 5
		t.Errorf("length: got %d, want 13", header.Length)
	}
	if !bytes.Equal(payload, []byte("hello")) {
		t.Errorf("payload mismatch")
	}
}

func TestUDPSocketDeliverAndReceive(t *testing.T) {
	sock := NewUDPSocket(53)
	sock.Deliver([]byte("response"), 0x08080808, 12345)
	d := sock.ReceiveFrom()
	if d == nil {
		t.Fatal("expected datagram")
	}
	if !bytes.Equal(d.Data, []byte("response")) {
		t.Errorf("data mismatch")
	}
	if d.SrcIP != 0x08080808 || d.SrcPort != 12345 {
		t.Errorf("address mismatch")
	}
}

func TestUDPSocketReceiveEmpty(t *testing.T) {
	sock := NewUDPSocket(53)
	if sock.ReceiveFrom() != nil {
		t.Error("expected nil")
	}
}

func TestUDPSocketHasData(t *testing.T) {
	sock := NewUDPSocket(53)
	if sock.HasData() {
		t.Error("should be empty initially")
	}
	sock.Deliver([]byte("data"), 0, 0)
	if !sock.HasData() {
		t.Error("should have data after deliver")
	}
	sock.ReceiveFrom()
	if sock.HasData() {
		t.Error("should be empty after receive")
	}
}

func TestUDPSocketFIFO(t *testing.T) {
	sock := NewUDPSocket(53)
	sock.Deliver([]byte("first"), 1, 100)
	sock.Deliver([]byte("second"), 2, 200)

	d1 := sock.ReceiveFrom()
	if string(d1.Data) != "first" {
		t.Errorf("expected first")
	}
	d2 := sock.ReceiveFrom()
	if string(d2.Data) != "second" {
		t.Errorf("expected second")
	}
	if sock.ReceiveFrom() != nil {
		t.Error("expected nil after drain")
	}
}
