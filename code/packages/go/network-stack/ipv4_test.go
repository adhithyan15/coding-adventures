package networkstack

import (
	"bytes"
	"testing"
)

func TestIPv4HeaderRoundtrip(t *testing.T) {
	h := &IPv4Header{
		Version:     4,
		IHL:         5,
		TotalLength: 60,
		TTL:         64,
		Protocol:    ProtocolTCP,
		SrcIP:       0x0A000001,
		DstIP:       0x0A000002,
	}
	raw := h.Serialize()
	recovered, err := DeserializeIPv4Header(raw)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if recovered.Version != 4 {
		t.Errorf("version: got %d, want 4", recovered.Version)
	}
	if recovered.IHL != 5 {
		t.Errorf("IHL: got %d, want 5", recovered.IHL)
	}
	if recovered.SrcIP != 0x0A000001 {
		t.Errorf("SrcIP mismatch")
	}
	if recovered.DstIP != 0x0A000002 {
		t.Errorf("DstIP mismatch")
	}
	if recovered.Protocol != ProtocolTCP {
		t.Errorf("Protocol mismatch")
	}
	if recovered.TotalLength != 60 {
		t.Errorf("TotalLength: got %d, want 60", recovered.TotalLength)
	}
}

func TestIPv4HeaderLength(t *testing.T) {
	h := NewIPv4Header()
	if len(h.Serialize()) != 20 {
		t.Errorf("expected 20 bytes")
	}
}

func TestIPv4VersionIHLPacking(t *testing.T) {
	h := NewIPv4Header()
	raw := h.Serialize()
	if raw[0] != 0x45 {
		t.Errorf("first byte should be 0x45, got 0x%02x", raw[0])
	}
}

func TestIPv4ChecksumNonzero(t *testing.T) {
	h := &IPv4Header{
		Version: 4, IHL: 5, TotalLength: 40, TTL: 64,
		Protocol: ProtocolTCP, SrcIP: 0x0A000001, DstIP: 0x0A000002,
	}
	h.Serialize()
	if h.Checksum == 0 {
		t.Error("checksum should be nonzero")
	}
}

func TestIPv4ComputeChecksum(t *testing.T) {
	h := &IPv4Header{
		Version: 4, IHL: 5, TotalLength: 28, TTL: 128,
		Protocol: ProtocolUDP, SrcIP: 0xC0A80001, DstIP: 0xC0A80002,
	}
	h.Serialize()
	saved := h.Checksum
	computed := h.ComputeChecksum()
	if computed != saved {
		t.Errorf("checksum mismatch: computed=%d, saved=%d", computed, saved)
	}
}

func TestIPv4DeserializeTooShort(t *testing.T) {
	_, err := DeserializeIPv4Header(make([]byte, 19))
	if err == nil {
		t.Error("expected error for short header")
	}
}

func TestIPv4Identification(t *testing.T) {
	h := &IPv4Header{
		Version: 4, IHL: 5, TotalLength: 20,
		Identification: 0x1234,
	}
	raw := h.Serialize()
	recovered, _ := DeserializeIPv4Header(raw)
	if recovered.Identification != 0x1234 {
		t.Errorf("identification mismatch")
	}
}

func TestRoutingTableEmpty(t *testing.T) {
	rt := NewRoutingTable()
	_, _, found := rt.Lookup(0x0A000001)
	if found {
		t.Error("expected no match on empty table")
	}
}

func TestRoutingTableDefaultRoute(t *testing.T) {
	rt := NewRoutingTable()
	rt.AddRoute(0, 0, 0x0A000001, "eth0")
	hop, iface, found := rt.Lookup(0x08080808)
	if !found {
		t.Fatal("expected match")
	}
	if hop != 0x0A000001 {
		t.Errorf("next hop: got 0x%08x, want 0x0A000001", hop)
	}
	if iface != "eth0" {
		t.Errorf("interface: got %s, want eth0", iface)
	}
}

func TestRoutingTableLongestPrefix(t *testing.T) {
	rt := NewRoutingTable()
	rt.AddRoute(0, 0, 0x0A000001, "eth0")              // default
	rt.AddRoute(0x0A000000, 0xFFFFFF00, 0, "eth1")      // /24
	rt.AddRoute(0x0A000000, 0xFFFFFFF0, 0x0A000002, "eth2") // /28

	// 10.0.0.5 matches /28
	hop, iface, _ := rt.Lookup(0x0A000005)
	if iface != "eth2" {
		t.Errorf("expected eth2, got %s", iface)
	}
	if hop != 0x0A000002 {
		t.Errorf("expected gateway 0x0A000002, got 0x%08x", hop)
	}

	// 10.0.0.20 matches /24 but not /28
	_, iface, _ = rt.Lookup(0x0A000014)
	if iface != "eth1" {
		t.Errorf("expected eth1, got %s", iface)
	}

	// 8.8.8.8 matches default only
	_, iface, _ = rt.Lookup(0x08080808)
	if iface != "eth0" {
		t.Errorf("expected eth0, got %s", iface)
	}
}

func TestIPLayerCreateAndParse(t *testing.T) {
	rt := NewRoutingTable()
	ip := NewIPLayer(0x0A000001, rt)

	payload := []byte("TCP segment here")
	packet := ip.CreatePacket(0x0A000002, ProtocolTCP, payload)

	h, p := ip.ParsePacket(packet)
	if h == nil {
		t.Fatal("expected valid header")
	}
	if h.SrcIP != 0x0A000001 {
		t.Errorf("SrcIP mismatch")
	}
	if h.DstIP != 0x0A000002 {
		t.Errorf("DstIP mismatch")
	}
	if !bytes.Equal(p, payload) {
		t.Errorf("payload mismatch")
	}
}

func TestIPLayerParseTooShort(t *testing.T) {
	rt := NewRoutingTable()
	ip := NewIPLayer(0x0A000001, rt)
	h, _ := ip.ParsePacket(make([]byte, 19))
	if h != nil {
		t.Error("expected nil for short packet")
	}
}
