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
	result, _ := StartNew[*SharedWire]("device-driver-framework.NewSharedWire", nil,
		func(op *Operation[*SharedWire], rf *ResultFactory[*SharedWire]) *OperationResult[*SharedWire] {
			return rf.Generate(true, false, &SharedWire{})
		}).GetResult()
	return result
}

// Connect adds a NIC to this wire. After connecting, the NIC will receive
// packets broadcast by other NICs on the same wire.
func (w *SharedWire) Connect(nic *SimulatedNIC) {
	_, _ = StartNew[struct{}]("device-driver-framework.SharedWire.Connect", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			// Prevent duplicate connections
			for _, n := range w.nics {
				if n == nic {
					return rf.Generate(true, false, struct{}{})
				}
			}
			w.nics = append(w.nics, nic)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Disconnect removes a NIC from this wire.
func (w *SharedWire) Disconnect(nic *SimulatedNIC) {
	_, _ = StartNew[struct{}]("device-driver-framework.SharedWire.Disconnect", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			for i, n := range w.nics {
				if n == nic {
					w.nics = append(w.nics[:i], w.nics[i+1:]...)
					return rf.Generate(true, false, struct{}{})
				}
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Broadcast sends a packet to all connected NICs except the sender.
func (w *SharedWire) Broadcast(data []byte, sender *SimulatedNIC) {
	_, _ = StartNew[struct{}]("device-driver-framework.SharedWire.Broadcast", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			for _, nic := range w.nics {
				if nic != sender {
					// Make a copy so each NIC gets its own slice
					pkt := make([]byte, len(data))
					copy(pkt, data)
					nic.RxQueue = append(nic.RxQueue, pkt)
				}
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// NICCount returns the number of NICs connected to this wire.
func (w *SharedWire) NICCount() int {
	result, _ := StartNew[int]("device-driver-framework.SharedWire.NICCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(w.nics))
		}).GetResult()
	return result
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
	result, _ := StartNew[*SimulatedNIC]("device-driver-framework.NewSimulatedNIC", nil,
		func(op *Operation[*SimulatedNIC], rf *ResultFactory[*SimulatedNIC]) *OperationResult[*SimulatedNIC] {
			op.AddProperty("name", name)
			op.AddProperty("minor", minor)
			return rf.Generate(true, false, &SimulatedNIC{
				DeviceBase: DeviceBase{
					Name:            name,
					Type:            DeviceNetwork,
					Major:           MajorNIC,
					Minor:           minor,
					InterruptNumber: IntNIC,
				},
				macAddress: macAddress,
				wire:       wire,
			})
		}).GetResult()
	return result
}

// Init initializes the NIC by clearing the receive queue and connecting
// to the shared wire.
func (n *SimulatedNIC) Init() {
	_, _ = StartNew[struct{}]("device-driver-framework.SimulatedNIC.Init", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			n.RxQueue = nil
			if n.wire != nil {
				n.wire.Connect(n)
			}
			n.Initialized = true
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// SendPacket sends a packet over the network via the shared wire.
// Returns the number of bytes sent, or -1 if not connected.
func (n *SimulatedNIC) SendPacket(data []byte) int {
	result, _ := StartNew[int]("device-driver-framework.SimulatedNIC.SendPacket", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			if n.wire == nil {
				return rf.Generate(true, false, -1)
			}
			n.wire.Broadcast(data, n)
			return rf.Generate(true, false, len(data))
		}).GetResult()
	return result
}

// ReceivePacket receives the next packet from the network.
// Returns the packet data, or nil if no packets are waiting.
func (n *SimulatedNIC) ReceivePacket() []byte {
	result, _ := StartNew[[]byte]("device-driver-framework.SimulatedNIC.ReceivePacket", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			if len(n.RxQueue) == 0 {
				return rf.Generate(true, false, nil)
			}
			pkt := n.RxQueue[0]
			n.RxQueue = n.RxQueue[1:]
			return rf.Generate(true, false, pkt)
		}).GetResult()
	return result
}

// HasPacket returns true if there is a packet waiting to be received.
func (n *SimulatedNIC) HasPacket() bool {
	result, _ := StartNew[bool]("device-driver-framework.SimulatedNIC.HasPacket", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, len(n.RxQueue) > 0)
		}).GetResult()
	return result
}

// MACAddress returns the 6-byte MAC address of this NIC.
func (n *SimulatedNIC) MACAddress() []byte {
	result, _ := StartNew[[]byte]("device-driver-framework.SimulatedNIC.MACAddress", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			return rf.Generate(true, false, n.macAddress)
		}).GetResult()
	return result
}

// Wire returns the SharedWire this NIC is connected to.
func (n *SimulatedNIC) Wire() *SharedWire {
	result, _ := StartNew[*SharedWire]("device-driver-framework.SimulatedNIC.Wire", nil,
		func(op *Operation[*SharedWire], rf *ResultFactory[*SharedWire]) *OperationResult[*SharedWire] {
			return rf.Generate(true, false, n.wire)
		}).GetResult()
	return result
}
