package networkstack

// NetworkWire — Simulated Physical Medium
//
// In the real world, Ethernet frames travel as electrical signals over copper,
// light pulses over fiber, or radio waves over WiFi. Our simulation uses an
// in-memory wire connecting two endpoints.
//
// The wire is full-duplex: both sides can send simultaneously, just like a
// real Ethernet cable with separate transmit and receive wires.
//
//	┌──────────┐                        ┌──────────┐
//	│ Endpoint │ ── SendA() ────────>   │ Endpoint │
//	│    A     │ <── ReceiveA() ─────   │    B     │
//	│          │                        │          │
//	│          │ <── SendB() ────────   │          │
//	│          │ ── ReceiveB() ────>    │          │
//	└──────────┘                        └──────────┘

// NetworkWire simulates an Ethernet cable with two independent queues
// for full-duplex communication.
type NetworkWire struct {
	queueAtoB [][]byte // Frames sent by A, waiting for B
	queueBtoA [][]byte // Frames sent by B, waiting for A
}

// NewNetworkWire creates a new empty wire.
func NewNetworkWire() *NetworkWire {
	return &NetworkWire{}
}

// SendA queues a frame from endpoint A. It will be received by B.
func (w *NetworkWire) SendA(frame []byte) {
	w.queueAtoB = append(w.queueAtoB, frame)
}

// SendB queues a frame from endpoint B. It will be received by A.
func (w *NetworkWire) SendB(frame []byte) {
	w.queueBtoA = append(w.queueBtoA, frame)
}

// ReceiveA returns the next frame for endpoint A (sent by B), or nil.
func (w *NetworkWire) ReceiveA() []byte {
	if len(w.queueBtoA) == 0 {
		return nil
	}
	frame := w.queueBtoA[0]
	w.queueBtoA = w.queueBtoA[1:]
	return frame
}

// ReceiveB returns the next frame for endpoint B (sent by A), or nil.
func (w *NetworkWire) ReceiveB() []byte {
	if len(w.queueAtoB) == 0 {
		return nil
	}
	frame := w.queueAtoB[0]
	w.queueAtoB = w.queueAtoB[1:]
	return frame
}

// HasDataForA returns true if frames are waiting for endpoint A.
func (w *NetworkWire) HasDataForA() bool {
	return len(w.queueBtoA) > 0
}

// HasDataForB returns true if frames are waiting for endpoint B.
func (w *NetworkWire) HasDataForB() bool {
	return len(w.queueAtoB) > 0
}
