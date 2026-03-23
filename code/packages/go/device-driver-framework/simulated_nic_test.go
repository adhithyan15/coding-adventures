package devicedriverframework

import (
	"bytes"
	"testing"
)

// =========================================================================
// SharedWire Tests
// =========================================================================

func TestSharedWireConnect(t *testing.T) {
	wire := NewSharedWire()
	nic := NewSimulatedNIC("nic0", 0, []byte{0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01}, wire)
	wire.Connect(nic)
	if wire.NICCount() != 1 {
		t.Errorf("NICCount = %d, want 1", wire.NICCount())
	}
}

func TestSharedWireConnectIdempotent(t *testing.T) {
	wire := NewSharedWire()
	nic := NewSimulatedNIC("nic0", 0, []byte{0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01}, wire)
	wire.Connect(nic)
	wire.Connect(nic) // duplicate
	if wire.NICCount() != 1 {
		t.Errorf("NICCount = %d, want 1 (should be idempotent)", wire.NICCount())
	}
}

func TestSharedWireDisconnect(t *testing.T) {
	wire := NewSharedWire()
	nic := NewSimulatedNIC("nic0", 0, []byte{0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01}, wire)
	wire.Connect(nic)
	wire.Disconnect(nic)
	if wire.NICCount() != 0 {
		t.Errorf("NICCount = %d, want 0 after disconnect", wire.NICCount())
	}
}

func TestSharedWireDisconnectNonexistent(t *testing.T) {
	wire := NewSharedWire()
	nic := NewSimulatedNIC("nic0", 0, []byte{0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01}, wire)
	wire.Disconnect(nic) // should not panic
	if wire.NICCount() != 0 {
		t.Errorf("NICCount = %d, want 0", wire.NICCount())
	}
}

func TestSharedWireBroadcast(t *testing.T) {
	wire := NewSharedWire()
	nicA := NewSimulatedNIC("nic0", 0, []byte{0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01}, wire)
	nicB := NewSimulatedNIC("nic1", 1, []byte{0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x02}, wire)
	wire.Connect(nicA)
	wire.Connect(nicB)

	wire.Broadcast([]byte("test packet"), nicA)
	if len(nicB.RxQueue) != 1 {
		t.Fatalf("NIC B should have 1 packet, got %d", len(nicB.RxQueue))
	}
	if !bytes.Equal(nicB.RxQueue[0], []byte("test packet")) {
		t.Errorf("Packet = %q, want test packet", nicB.RxQueue[0])
	}
	if len(nicA.RxQueue) != 0 {
		t.Error("Sender should not receive its own packet")
	}
}

func TestSharedWireBroadcastToMultiple(t *testing.T) {
	wire := NewSharedWire()
	nicA := NewSimulatedNIC("nic0", 0, []byte{0x01}, wire)
	nicB := NewSimulatedNIC("nic1", 1, []byte{0x02}, wire)
	nicC := NewSimulatedNIC("nic2", 2, []byte{0x03}, wire)
	wire.Connect(nicA)
	wire.Connect(nicB)
	wire.Connect(nicC)

	wire.Broadcast([]byte("hello"), nicA)
	if len(nicB.RxQueue) != 1 {
		t.Errorf("NIC B: %d packets, want 1", len(nicB.RxQueue))
	}
	if len(nicC.RxQueue) != 1 {
		t.Errorf("NIC C: %d packets, want 1", len(nicC.RxQueue))
	}
	if len(nicA.RxQueue) != 0 {
		t.Errorf("NIC A: %d packets, want 0", len(nicA.RxQueue))
	}
}

// =========================================================================
// SimulatedNIC Tests
// =========================================================================

func TestSimulatedNICDefaults(t *testing.T) {
	mac := []byte{0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01}
	nic := NewSimulatedNIC("nic0", 0, mac, nil)
	if nic.Name != "nic0" {
		t.Errorf("Name = %q, want nic0", nic.Name)
	}
	if nic.Type != DeviceNetwork {
		t.Errorf("Type = %v, want NETWORK", nic.Type)
	}
	if nic.Major != MajorNIC {
		t.Errorf("Major = %d, want %d", nic.Major, MajorNIC)
	}
	if nic.InterruptNumber != IntNIC {
		t.Errorf("InterruptNumber = %d, want %d", nic.InterruptNumber, IntNIC)
	}
	if !bytes.Equal(nic.MACAddress(), mac) {
		t.Errorf("MACAddress mismatch")
	}
}

func TestSimulatedNICReceiveEmpty(t *testing.T) {
	nic := NewSimulatedNIC("nic0", 0, []byte{0x01, 0x02, 0x03, 0x04, 0x05, 0x06}, nil)
	if pkt := nic.ReceivePacket(); pkt != nil {
		t.Errorf("ReceivePacket on empty should be nil, got %v", pkt)
	}
}

func TestSimulatedNICHasPacketEmpty(t *testing.T) {
	nic := NewSimulatedNIC("nic0", 0, []byte{0x01, 0x02, 0x03, 0x04, 0x05, 0x06}, nil)
	if nic.HasPacket() {
		t.Error("HasPacket should be false on empty queue")
	}
}

func TestSimulatedNICSendAndReceive(t *testing.T) {
	wire := NewSharedWire()
	nicA := NewSimulatedNIC("nic0", 0, []byte{0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01}, wire)
	nicB := NewSimulatedNIC("nic1", 1, []byte{0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x02}, wire)
	nicA.Init()
	nicB.Init()

	sent := nicA.SendPacket([]byte("Hello from A!"))
	if sent != 13 {
		t.Errorf("SendPacket returned %d, want 13", sent)
	}
	if !nicB.HasPacket() {
		t.Error("NIC B should have a packet")
	}
	pkt := nicB.ReceivePacket()
	if !bytes.Equal(pkt, []byte("Hello from A!")) {
		t.Errorf("Received = %q, want Hello from A!", pkt)
	}
}

func TestSimulatedNICSenderDoesNotReceive(t *testing.T) {
	wire := NewSharedWire()
	nicA := NewSimulatedNIC("nic0", 0, []byte{0x01}, wire)
	nicB := NewSimulatedNIC("nic1", 1, []byte{0x02}, wire)
	nicA.Init()
	nicB.Init()

	nicA.SendPacket([]byte("echo?"))
	if nicA.HasPacket() {
		t.Error("Sender should not receive its own packet")
	}
}

func TestSimulatedNICSendWithoutWire(t *testing.T) {
	nic := NewSimulatedNIC("nic0", 0, []byte{0x01}, nil)
	if n := nic.SendPacket([]byte("data")); n != -1 {
		t.Errorf("SendPacket without wire returned %d, want -1", n)
	}
}

func TestSimulatedNICInit(t *testing.T) {
	wire := NewSharedWire()
	nic := NewSimulatedNIC("nic0", 0, []byte{0x01}, wire)
	nic.RxQueue = append(nic.RxQueue, []byte("stale"))
	nic.Init()
	if !nic.Initialized {
		t.Error("Should be initialized after Init()")
	}
	if len(nic.RxQueue) != 0 {
		t.Error("RxQueue should be empty after Init()")
	}
	if wire.NICCount() != 1 {
		t.Errorf("Wire should have 1 NIC, got %d", wire.NICCount())
	}
}

func TestSimulatedNICMultiplePacketsFIFO(t *testing.T) {
	wire := NewSharedWire()
	nicA := NewSimulatedNIC("nic0", 0, []byte{0x01}, wire)
	nicB := NewSimulatedNIC("nic1", 1, []byte{0x02}, wire)
	nicA.Init()
	nicB.Init()

	nicA.SendPacket([]byte("first"))
	nicA.SendPacket([]byte("second"))
	nicA.SendPacket([]byte("third"))

	pkt1 := nicB.ReceivePacket()
	pkt2 := nicB.ReceivePacket()
	pkt3 := nicB.ReceivePacket()
	pkt4 := nicB.ReceivePacket()

	if !bytes.Equal(pkt1, []byte("first")) {
		t.Errorf("Packet 1 = %q, want first", pkt1)
	}
	if !bytes.Equal(pkt2, []byte("second")) {
		t.Errorf("Packet 2 = %q, want second", pkt2)
	}
	if !bytes.Equal(pkt3, []byte("third")) {
		t.Errorf("Packet 3 = %q, want third", pkt3)
	}
	if pkt4 != nil {
		t.Errorf("Packet 4 should be nil, got %q", pkt4)
	}
}

func TestSimulatedNICBidirectional(t *testing.T) {
	wire := NewSharedWire()
	nicA := NewSimulatedNIC("nic0", 0, []byte{0x01}, wire)
	nicB := NewSimulatedNIC("nic1", 1, []byte{0x02}, wire)
	nicA.Init()
	nicB.Init()

	nicA.SendPacket([]byte("ping"))
	pkt := nicB.ReceivePacket()
	if !bytes.Equal(pkt, []byte("ping")) {
		t.Errorf("B received %q, want ping", pkt)
	}

	nicB.SendPacket([]byte("pong"))
	pkt = nicA.ReceivePacket()
	if !bytes.Equal(pkt, []byte("pong")) {
		t.Errorf("A received %q, want pong", pkt)
	}
}

func TestSimulatedNICWireProperty(t *testing.T) {
	wire := NewSharedWire()
	nic := NewSimulatedNIC("nic0", 0, []byte{0x01}, wire)
	if nic.Wire() != wire {
		t.Error("Wire() should return the SharedWire")
	}
}

func TestSimulatedNICInitWithoutWire(t *testing.T) {
	nic := NewSimulatedNIC("nic0", 0, []byte{0x01}, nil)
	nic.Init()
	if !nic.Initialized {
		t.Error("Init without wire should still set Initialized")
	}
}
