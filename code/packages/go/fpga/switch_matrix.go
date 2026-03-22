package fpga

// =========================================================================
// Switch Matrix — Programmable Routing Crossbar for the FPGA Fabric
// =========================================================================
//
// The routing fabric is what makes an FPGA truly programmable. LUTs and
// CLBs compute boolean functions, but the switch matrix determines how
// those functions connect to each other.
//
// A switch matrix sits at each intersection of the routing grid. It is a
// crossbar that can connect any of its input wires to any of its output
// wires, based on configuration bits stored in SRAM.
//
// Grid Layout:
//
//	┌─────┐     ┌─────┐     ┌─────┐
//	│ CLB │──SW──│ CLB │──SW──│ CLB │
//	└──┬──┘     └──┬──┘     └──┬──┘
//	   │SW          │SW          │SW
//	┌──┴──┐     ┌──┴──┐     ┌──┴──┐
//	│ CLB │──SW──│ CLB │──SW──│ CLB │
//	└─────┘     └─────┘     └─────┘
//
//	SW = Switch Matrix
//
// Connection Model:
//
// We model the switch matrix as a set of named ports and a configurable
// connection map. Each connection maps a source port to a destination port.
// When a signal arrives at a source port, the switch matrix routes it to
// the connected destination port.

import "fmt"

// SwitchMatrix is a programmable routing crossbar.
//
// Connects named signal ports via configurable routes. Each route maps
// a source port to a destination port. Multiple routes can share the same
// source (fan-out) but each destination can only have one source (no bus
// contention).
type SwitchMatrix struct {
	ports       map[string]bool
	connections map[string]string // destination → source
}

// NewSwitchMatrix creates a switch matrix with the given port names.
//
// Panics if ports is empty or contains empty strings.
func NewSwitchMatrix(ports []string) *SwitchMatrix {
	if len(ports) == 0 {
		panic("fpga: SwitchMatrix ports must be non-empty")
	}

	portSet := make(map[string]bool, len(ports))
	for _, p := range ports {
		if p == "" {
			panic("fpga: SwitchMatrix port names must be non-empty strings")
		}
		portSet[p] = true
	}

	return &SwitchMatrix{
		ports:       portSet,
		connections: make(map[string]string),
	}
}

// Connect creates a route from source to destination.
//
// Panics if:
//   - source or destination is unknown
//   - source equals destination
//   - destination is already connected
func (sm *SwitchMatrix) Connect(source, destination string) {
	if !sm.ports[source] {
		panic(fmt.Sprintf("fpga: SwitchMatrix unknown source port: %q", source))
	}
	if !sm.ports[destination] {
		panic(fmt.Sprintf("fpga: SwitchMatrix unknown destination port: %q", destination))
	}
	if source == destination {
		panic(fmt.Sprintf("fpga: SwitchMatrix cannot connect port %q to itself", source))
	}
	if existing, ok := sm.connections[destination]; ok {
		panic(fmt.Sprintf("fpga: SwitchMatrix destination %q already connected to %q", destination, existing))
	}

	sm.connections[destination] = source
}

// Disconnect removes the route to a destination port.
//
// Panics if the port is unknown or not connected.
func (sm *SwitchMatrix) Disconnect(destination string) {
	if !sm.ports[destination] {
		panic(fmt.Sprintf("fpga: SwitchMatrix unknown port: %q", destination))
	}
	if _, ok := sm.connections[destination]; !ok {
		panic(fmt.Sprintf("fpga: SwitchMatrix port %q is not connected", destination))
	}

	delete(sm.connections, destination)
}

// Clear removes all connections (resets the switch matrix).
func (sm *SwitchMatrix) Clear() {
	sm.connections = make(map[string]string)
}

// Route propagates signals through the switch matrix.
//
// Parameters:
//   - inputs: map of port name → signal value (0 or 1) for ports that
//     have external signals driving them.
//
// Returns a map of destination port → routed signal value for all
// connected destinations whose source appears in inputs.
func (sm *SwitchMatrix) Route(inputs map[string]int) map[string]int {
	outputs := make(map[string]int)
	for dest, src := range sm.connections {
		if val, ok := inputs[src]; ok {
			outputs[dest] = val
		}
	}
	return outputs
}

// Ports returns the set of all port names.
func (sm *SwitchMatrix) Ports() map[string]bool {
	result := make(map[string]bool, len(sm.ports))
	for k, v := range sm.ports {
		result[k] = v
	}
	return result
}

// Connections returns the current connection map (destination → source).
func (sm *SwitchMatrix) Connections() map[string]string {
	result := make(map[string]string, len(sm.connections))
	for k, v := range sm.connections {
		result[k] = v
	}
	return result
}

// ConnectionCount returns the number of active connections.
func (sm *SwitchMatrix) ConnectionCount() int {
	return len(sm.connections)
}
