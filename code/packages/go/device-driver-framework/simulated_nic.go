package devicedriverframework

// =========================================================================
// SharedWire -- a simulated network cable connecting multiple NICs
// =========================================================================
//
// In early Ethernet (10BASE2, 10BASE5), all computers were literally connected
// to the same coaxial cable. When one computer sent a packet, every other
// computer on the cable received it.
//
// Our SharedWire simulates this: when one NIC sends a packet, all other NICs
// on the same wire receive a copy. The sender does NOT receive its own packet.

// SharedWire is a simulated network cable connecting multiple NICs.
type SharedWire struct {
	nics []*SimulatedNIC
}

// NewSharedWire creates a new empty shared wire.
func NewSharedWire() *SharedWire {
	return &SharedWire{}
}

// Connect adds a NIC to this wire. After connecting, the NIC will receive
// packets broadcast by other NICs on the same wire.
func (w *SharedWire) Connect(nic *SimulatedNIC) {
	// Prevent duplicate connections
	for _, n := range w.nics {
		if n == nic {
			return
		}
	}
	w.nics = append(w.nics, nic)
}

// Disconnect removes a NIC from this wire.
func (w *SharedWire) Disconnect(nic *SimulatedNIC) {
	for i, n := range w.nics {
		if n == nic {
			w.nics = append(w.nics[:i], w.nics[i+1:]...)
			return
		}
	}
}

// Broadcast sends a packet to all connected NICs except the sender.
func (w *SharedWire) Broadcast(data []byte, sender *SimulatedNIC) {
	for _, nic := range w.nics {
		if nic != sender {
			// Make a copy so each NIC gets its own slice
			pkt := make([]byte, len(data))
			copy(pkt, data)
			nic.RxQueue = append(nic.RxQueue, pkt)
		}
	}
}

// NICCount returns the number of NICs connected to this wire.
func (w *SharedWire) NICCount() int {
	return len(w.nics)
}

// =========================================================================
// SimulatedNIC -- a network interface card backed by in-memory queues
// =========================================================================
//
// A NIC connects a computer to a network. Every NIC has a MAC address --
// a 6-byte unique identifier. When another computer sends a packet, it
// addresses it to the recipient's MAC address.
//
// Two SimulatedNICs connected to the same SharedWire can exchange packets.
// Packets are NOT echoed back to the sender.

// SimulatedNIC is a simulated network interface card.
type SimulatedNIC struct {
	DeviceBase
	macAddress []byte
	RxQueue    [][]byte // Packets waiting to be received (public for SharedWire)
	wire       *SharedWire
}

// NewSimulatedNIC creates a new simulated NIC.
//
// Parameters:
//   - name: device name (e.g., "nic0")
//   - minor: minor number for this NIC instance
//   - macAddress: 6-byte MAC address
//   - wire: the SharedWire connecting this NIC to others (can be nil)
func NewSimulatedNIC(name string, minor int, macAddress []byte, wire *SharedWire) *SimulatedNIC {
	return &SimulatedNIC{
		DeviceBase: DeviceBase{
			Name:            name,
			Type:            DeviceNetwork,
			Major:           MajorNIC,
			Minor:           minor,
			InterruptNumber: IntNIC,
		},
		macAddress: macAddress,
		wire:       wire,
	}
}

// Init initializes the NIC by clearing the receive queue and connecting
// to the shared wire.
func (n *SimulatedNIC) Init() {
	n.RxQueue = nil
	if n.wire != nil {
		n.wire.Connect(n)
	}
	n.Initialized = true
}

// SendPacket sends a packet over the network via the shared wire.
// Returns the number of bytes sent, or -1 if not connected.
func (n *SimulatedNIC) SendPacket(data []byte) int {
	if n.wire == nil {
		return -1
	}
	n.wire.Broadcast(data, n)
	return len(data)
}

// ReceivePacket receives the next packet from the network.
// Returns the packet data, or nil if no packets are waiting.
func (n *SimulatedNIC) ReceivePacket() []byte {
	if len(n.RxQueue) == 0 {
		return nil
	}
	pkt := n.RxQueue[0]
	n.RxQueue = n.RxQueue[1:]
	return pkt
}

// HasPacket returns true if there is a packet waiting to be received.
func (n *SimulatedNIC) HasPacket() bool {
	return len(n.RxQueue) > 0
}

// MACAddress returns the 6-byte MAC address of this NIC.
func (n *SimulatedNIC) MACAddress() []byte {
	return n.macAddress
}

// Wire returns the SharedWire this NIC is connected to.
func (n *SimulatedNIC) Wire() *SharedWire {
	return n.wire
}
