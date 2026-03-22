package networkstack

import (
	"bytes"
	"testing"
)

func TestEthernetFrameRoundtrip(t *testing.T) {
	frame := &EthernetFrame{
		DestMAC:   [6]byte{0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff},
		SrcMAC:    [6]byte{0x11, 0x22, 0x33, 0x44, 0x55, 0x66},
		EtherType: EthertypeIPv4,
		Payload:   []byte("Hello, Ethernet!"),
	}
	raw := frame.Serialize()
	recovered, err := DeserializeEthernetFrame(raw)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if recovered.DestMAC != frame.DestMAC {
		t.Errorf("DestMAC mismatch")
	}
	if recovered.SrcMAC != frame.SrcMAC {
		t.Errorf("SrcMAC mismatch")
	}
	if recovered.EtherType != frame.EtherType {
		t.Errorf("EtherType mismatch")
	}
	if !bytes.Equal(recovered.Payload, frame.Payload) {
		t.Errorf("Payload mismatch")
	}
}

func TestEthernetFrameLength(t *testing.T) {
	frame := &EthernetFrame{Payload: []byte("test")}
	raw := frame.Serialize()
	if len(raw) != 14+4 {
		t.Errorf("expected length %d, got %d", 18, len(raw))
	}
}

func TestEthernetFrameFormat(t *testing.T) {
	frame := &EthernetFrame{
		DestMAC:   [6]byte{0x01, 0x02, 0x03, 0x04, 0x05, 0x06},
		SrcMAC:    [6]byte{0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f},
		EtherType: EthertypeARP,
		Payload:   []byte{0xde, 0xad},
	}
	raw := frame.Serialize()
	if raw[12] != 0x08 || raw[13] != 0x06 {
		t.Errorf("EtherType bytes wrong: %x %x", raw[12], raw[13])
	}
}

func TestEthernetDeserializeTooShort(t *testing.T) {
	_, err := DeserializeEthernetFrame(make([]byte, 13))
	if err == nil {
		t.Error("expected error for short frame")
	}
}

func TestEthernetEmptyPayload(t *testing.T) {
	frame := &EthernetFrame{
		DestMAC:   [6]byte{0xff, 0xff, 0xff, 0xff, 0xff, 0xff},
		EtherType: EthertypeARP,
	}
	raw := frame.Serialize()
	recovered, err := DeserializeEthernetFrame(raw)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(recovered.Payload) != 0 {
		t.Errorf("expected empty payload")
	}
}

func TestARPTableLookupMiss(t *testing.T) {
	table := NewARPTable()
	_, ok := table.Lookup(0x0A000001)
	if ok {
		t.Error("expected miss on empty table")
	}
}

func TestARPTableUpdateAndLookup(t *testing.T) {
	table := NewARPTable()
	mac := [6]byte{0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff}
	table.Update(0x0A000001, mac)
	got, ok := table.Lookup(0x0A000001)
	if !ok {
		t.Fatal("expected hit")
	}
	if got != mac {
		t.Errorf("MAC mismatch")
	}
}

func TestARPTableOverwrite(t *testing.T) {
	table := NewARPTable()
	mac1 := [6]byte{0x11, 0x22, 0x33, 0x44, 0x55, 0x66}
	mac2 := [6]byte{0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff}
	table.Update(0x0A000001, mac1)
	table.Update(0x0A000001, mac2)
	got, _ := table.Lookup(0x0A000001)
	if got != mac2 {
		t.Errorf("expected overwritten MAC")
	}
}

func TestARPTableEntries(t *testing.T) {
	table := NewARPTable()
	table.Update(0x0A000001, [6]byte{0x11, 0x11, 0x11, 0x11, 0x11, 0x11})
	entries := table.Entries()
	if len(entries) != 1 {
		t.Errorf("expected 1 entry, got %d", len(entries))
	}
}
