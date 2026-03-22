package fpga

// =========================================================================
// FPGA Fabric — The Top-Level FPGA Model
// =========================================================================
//
// An FPGA (Field-Programmable Gate Array) is a chip containing:
//   - A grid of CLBs (Configurable Logic Blocks) for computation
//   - A routing fabric (switch matrices) for interconnection
//   - I/O blocks at the perimeter for external connections
//   - Block RAM tiles for on-chip memory
//
// The key property: all of this is programmable. By loading a bitstream
// (configuration data), the same physical chip can become any digital
// circuit — a CPU, a signal processor, a network switch, or anything
// else that fits within its resources.
//
// Our FPGA Model:
//
//	┌────────────────────────────────────────────────────┐
//	│                    FPGA Fabric                      │
//	│                                                     │
//	│  [IO] [IO] [IO] [IO] [IO] [IO] [IO] [IO]          │
//	│                                                     │
//	│  [IO] [CLB]──[SW]──[CLB]──[SW]──[CLB] [IO]        │
//	│         │            │            │                  │
//	│        [SW]         [SW]         [SW]               │
//	│         │            │            │                  │
//	│  [IO] [CLB]──[SW]──[CLB]──[SW]──[CLB] [IO]        │
//	│                                                     │
//	│  [IO] [IO] [IO] [IO] [IO] [IO] [IO] [IO]          │
//	│                                                     │
//	│            [BRAM]        [BRAM]                     │
//	└────────────────────────────────────────────────────┘
//
// The FPGA type:
//  1. Creates CLBs, switch matrices, and I/O blocks from a bitstream
//  2. Configures each element according to the bitstream
//  3. Provides methods for evaluating CLBs, routing signals, and
//     reading/writing I/O pins

import "fmt"

// FPGA is the top-level FPGA fabric model.
//
// Creates and configures CLBs, switch matrices, and I/O blocks
// from a Bitstream, then provides methods to evaluate the configured
// circuit.
type FPGA struct {
	bitstream *Bitstream
	clbs      map[string]*CLB
	switches  map[string]*SwitchMatrix
	ios       map[string]*IOBlock
}

// NewFPGA creates and configures an FPGA from a bitstream.
func NewFPGA(bs *Bitstream) *FPGA {
	f := &FPGA{
		bitstream: bs,
		clbs:      make(map[string]*CLB),
		switches:  make(map[string]*SwitchMatrix),
		ios:       make(map[string]*IOBlock),
	}
	f.configure(bs)
	return f
}

// configure applies the bitstream configuration to create and program
// all elements.
func (f *FPGA) configure(bs *Bitstream) {
	// Create and configure CLBs
	for name, clbCfg := range bs.CLBs {
		clb := NewCLB(bs.LutK)

		clb.Slice0().Configure(
			clbCfg.Slice0.LutA,
			clbCfg.Slice0.LutB,
			clbCfg.Slice0.FFAEnabled,
			clbCfg.Slice0.FFBEnabled,
			clbCfg.Slice0.CarryEnabled,
		)
		clb.Slice1().Configure(
			clbCfg.Slice1.LutA,
			clbCfg.Slice1.LutB,
			clbCfg.Slice1.FFAEnabled,
			clbCfg.Slice1.FFBEnabled,
			clbCfg.Slice1.CarryEnabled,
		)

		f.clbs[name] = clb
	}

	// Create and configure switch matrices
	for swName, routes := range bs.Routing {
		// Collect all port names referenced in routes
		portSet := make(map[string]bool)
		for _, route := range routes {
			portSet[route.Source] = true
			portSet[route.Destination] = true
		}

		if len(portSet) > 0 {
			ports := make([]string, 0, len(portSet))
			for p := range portSet {
				ports = append(ports, p)
			}

			sm := NewSwitchMatrix(ports)
			for _, route := range routes {
				sm.Connect(route.Source, route.Destination)
			}
			f.switches[swName] = sm
		}
	}

	// Create I/O blocks
	for pinName, ioCfg := range bs.IO {
		mode := IOInput // default
		switch ioCfg.Mode {
		case "output":
			mode = IOOutput
		case "tristate":
			mode = IOTristate
		}
		f.ios[pinName] = NewIOBlock(pinName, mode)
	}
}

// EvaluateCLB evaluates a specific CLB.
//
// Panics if clbName is not found.
func (f *FPGA) EvaluateCLB(
	clbName string,
	slice0InputsA, slice0InputsB []int,
	slice1InputsA, slice1InputsB []int,
	clock, carryIn int,
) CLBOutput {
	clb, ok := f.clbs[clbName]
	if !ok {
		panic(fmt.Sprintf("fpga: CLB %q not found", clbName))
	}
	return clb.Evaluate(slice0InputsA, slice0InputsB, slice1InputsA, slice1InputsB, clock, carryIn)
}

// Route routes signals through a switch matrix.
//
// Panics if switchName is not found.
func (f *FPGA) Route(switchName string, signals map[string]int) map[string]int {
	sm, ok := f.switches[switchName]
	if !ok {
		panic(fmt.Sprintf("fpga: Switch matrix %q not found", switchName))
	}
	return sm.Route(signals)
}

// SetInput drives an input pin.
//
// Panics if pinName is not found.
func (f *FPGA) SetInput(pinName string, value int) {
	io, ok := f.ios[pinName]
	if !ok {
		panic(fmt.Sprintf("fpga: I/O pin %q not found", pinName))
	}
	io.DrivePad(value)
}

// ReadOutput reads an output pin.
//
// Panics if pinName is not found.
func (f *FPGA) ReadOutput(pinName string) *int {
	io, ok := f.ios[pinName]
	if !ok {
		panic(fmt.Sprintf("fpga: I/O pin %q not found", pinName))
	}
	return io.ReadPad()
}

// DriveOutput drives the internal side of an output pin (fabric → external).
//
// Panics if pinName is not found.
func (f *FPGA) DriveOutput(pinName string, value int) {
	io, ok := f.ios[pinName]
	if !ok {
		panic(fmt.Sprintf("fpga: I/O pin %q not found", pinName))
	}
	io.DriveInternal(value)
}

// CLBs returns all CLBs in the fabric.
func (f *FPGA) CLBs() map[string]*CLB {
	result := make(map[string]*CLB, len(f.clbs))
	for k, v := range f.clbs {
		result[k] = v
	}
	return result
}

// Switches returns all switch matrices in the fabric.
func (f *FPGA) Switches() map[string]*SwitchMatrix {
	result := make(map[string]*SwitchMatrix, len(f.switches))
	for k, v := range f.switches {
		result[k] = v
	}
	return result
}

// IOs returns all I/O blocks.
func (f *FPGA) IOs() map[string]*IOBlock {
	result := make(map[string]*IOBlock, len(f.ios))
	for k, v := range f.ios {
		result[k] = v
	}
	return result
}

// GetBitstream returns the loaded bitstream configuration.
func (f *FPGA) GetBitstream() *Bitstream { return f.bitstream }
