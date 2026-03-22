package logicgates

// =========================================================================
// Combinational Circuits — Building Blocks Between Gates and Arithmetic
// =========================================================================
//
// # What are combinational circuits?
//
// Combinational circuits produce outputs that depend ONLY on the current
// inputs — no memory, no state, no clock. They are built entirely from
// the primitive gates defined in gates.go (AND, OR, NOT, XOR, etc.).
//
// These circuits fill the gap between individual gates and the ALU:
//
//	Primitive gates (gates.go)
//	    │
//	Combinational circuits (THIS FILE)
//	    │  MUX, DEMUX, decoder, encoder, tri-state buffer
//	    │
//	Arithmetic circuits (arithmetic package)
//	    │  half adder, full adder, ALU
//	    │
//	CPU, FPGA, memory controllers
//	    │  everything above uses these building blocks
//
// # Why these circuits matter
//
//   - MUX (Multiplexer): The selector switch of digital logic. A K-input
//     LUT in an FPGA is literally a 2^K-to-1 MUX with SRAM storing the
//     truth table. CPUs use MUXes to select between register outputs, ALU
//     inputs, and forwarded values.
//
//   - DEMUX (Demultiplexer): Routes one signal to one of many destinations.
//     Used in memory write addressing and bus arbitration.
//
//   - Decoder: Converts binary addresses into one-hot select lines. Every
//     memory chip has a row decoder that activates exactly one word line
//     based on the address.
//
//   - Encoder / Priority Encoder: The inverse of a decoder. Priority
//     encoders are the heart of interrupt controllers — when multiple
//     interrupts fire simultaneously, the priority encoder picks the most
//     important one.
//
//   - Tri-state buffer: Enables shared buses by letting devices "disconnect"
//     from the wire when they are not talking. Without tri-state buffers,
//     you would need separate wires for every device pair.

import "fmt"

// =========================================================================
// MULTIPLEXER (MUX) — The Selector Switch
// =========================================================================
//
// A multiplexer takes N data inputs and a set of select lines, and routes
// exactly one input to the output. Think of it as a railroad switch that
// directs one of several trains onto a single track.
//
// The number of select lines determines how many inputs can be selected:
//
//	1 select line  → 2 inputs  (2:1 MUX)
//	2 select lines → 4 inputs  (4:1 MUX)
//	3 select lines → 8 inputs  (8:1 MUX)
//	N select lines → 2^N inputs (2^N:1 MUX)
//
// Every larger MUX can be built recursively from 2:1 MUXes:
//
//	4:1  = two 2:1 MUXes feeding a third 2:1 MUX
//	8:1  = two 4:1 MUXes feeding a 2:1 MUX
//	16:1 = two 8:1 MUXes feeding a 2:1 MUX
//
// This recursive structure is exactly how FPGA look-up tables (LUTs) work:
// a 4-input LUT is a 16:1 MUX tree with the truth table stored in SRAM.

// Mux2 is a 2-to-1 Multiplexer — the simplest selector circuit.
//
// Routes one of two data inputs to the output based on a select signal.
//
// Built from gates:
//
//	output = OR(AND(d0, NOT(sel)), AND(d1, sel))
//
// When sel=0, the NOT(sel)=1 enables d0 through the top AND gate.
// When sel=1, sel itself enables d1 through the bottom AND gate.
//
// Truth table:
//
//	sel | output
//	----|-------
//	 0  |  d0
//	 1  |  d1
func Mux2(d0, d1, sel int) int {
	validateBit(d0, "d0")
	validateBit(d1, "d1")
	validateBit(sel, "sel")

	return OR(AND(d0, NOT(sel)), AND(d1, sel))
}

// Mux4 is a 4-to-1 Multiplexer — selects one of four inputs using 2
// select lines.
//
// Built from three 2:1 MUXes arranged in a tree:
//
//	d0 ──┐                     sel[0] controls first level
//	     MUX ── r0 ──┐
//	d1 ──┘             │        sel[1] controls second level
//	                    MUX ── output
//	d2 ──┐             │
//	     MUX ── r1 ──┘
//	d3 ──┘
//
// Truth table:
//
//	sel[1] sel[0] | output
//	--------------+-------
//	  0      0    |  d0
//	  0      1    |  d1
//	  1      0    |  d2
//	  1      1    |  d3
//
// The sel parameter is a slice of 2 bits [s0, s1] (LSB first).
func Mux4(d0, d1, d2, d3 int, sel []int) int {
	validateBit(d0, "d0")
	validateBit(d1, "d1")
	validateBit(d2, "d2")
	validateBit(d3, "d3")

	if len(sel) != 2 {
		panic(fmt.Sprintf("logicgates: Mux4 sel must have exactly 2 bits, got %d", len(sel)))
	}
	validateBits(sel, "sel")

	// First level: sel[0] selects within each pair
	r0 := Mux2(d0, d1, sel[0])
	r1 := Mux2(d2, d3, sel[0])

	// Second level: sel[1] selects between the two pairs
	return Mux2(r0, r1, sel[1])
}

// MuxN is an N-to-1 Multiplexer — selects one of N inputs using log2(N)
// select lines.
//
// N must be a power of 2 (2, 4, 8, 16, 32, 64, ...).
//
// Built recursively: split inputs in half, recurse on each half with
// sel[:-1], then use a 2:1 MUX with sel[last] to pick between the two
// halves.
//
// This recursive construction is exactly how FPGA look-up tables work:
// a K-input LUT is a 2^K-to-1 MUX tree.
//
// Parameters:
//   - inputs: slice of N data inputs (N must be power of 2, N >= 2)
//   - sel: slice of log2(N) select bits (LSB first)
//
// Returns the selected data input value (0 or 1).
func MuxN(inputs, sel []int) int {
	n := len(inputs)

	if n < 2 {
		panic("logicgates: MuxN inputs must have at least 2 elements")
	}

	// Check power of 2: a number is a power of 2 if it has exactly one bit set
	if n&(n-1) != 0 {
		panic(fmt.Sprintf("logicgates: MuxN inputs length must be a power of 2, got %d", n))
	}

	expectedSelBits := 0
	for tmp := n; tmp > 1; tmp >>= 1 {
		expectedSelBits++
	}

	if len(sel) != expectedSelBits {
		panic(fmt.Sprintf("logicgates: MuxN sel must have %d bits for %d inputs, got %d", expectedSelBits, n, len(sel)))
	}

	validateBits(inputs, "inputs")
	validateBits(sel, "sel")

	return muxNInner(inputs, sel)
}

// muxNInner is the inner recursive helper for MuxN — skips validation
// (already done by the public function).
func muxNInner(inputs, sel []int) int {
	n := len(inputs)
	if n == 2 {
		return Mux2(inputs[0], inputs[1], sel[0])
	}

	half := n / 2
	lower := muxNInner(inputs[:half], sel[:len(sel)-1])
	upper := muxNInner(inputs[half:], sel[:len(sel)-1])
	return Mux2(lower, upper, sel[len(sel)-1])
}

// =========================================================================
// DEMULTIPLEXER (DEMUX) — The Inverse of MUX
// =========================================================================
//
// A demultiplexer takes one data input and routes it to one of N outputs.
// The select lines determine which output receives the data; all other
// outputs are 0.
//
// Think of it as an address decoder that also carries data: the decoder
// picks which output line is active, and the data signal determines
// whether that line is 0 or 1.

// Demux is a 1-to-N Demultiplexer — routes one input to one of N outputs.
//
// The selected output receives the data value; all other outputs are 0.
//
// Built from a decoder + AND gates:
//  1. Decoder converts sel bits into one-hot (exactly one output = 1)
//  2. AND each decoder output with the data input
//
// 1-to-4 DEMUX truth table:
//
//	sel[1] sel[0]  data | y0  y1  y2  y3
//	---------------------+-----------------
//	  0      0      0   |  0   0   0   0
//	  0      0      1   |  1   0   0   0
//	  0      1      0   |  0   0   0   0
//	  0      1      1   |  0   1   0   0
//	  1      0      0   |  0   0   0   0
//	  1      0      1   |  0   0   1   0
//	  1      1      0   |  0   0   0   0
//	  1      1      1   |  0   0   0   1
//
// Parameters:
//   - data: the data bit to route (0 or 1)
//   - sel: slice of select bits (LSB first), length = log2(nOutputs)
//   - nOutputs: number of outputs (must be power of 2, >= 2)
//
// Returns a slice of nOutputs bits. Exactly one equals data, rest are 0.
func Demux(data int, sel []int, nOutputs int) []int {
	validateBit(data, "data")

	if nOutputs < 2 || nOutputs&(nOutputs-1) != 0 {
		panic(fmt.Sprintf("logicgates: Demux nOutputs must be a power of 2 >= 2, got %d", nOutputs))
	}

	expectedSelBits := 0
	for tmp := nOutputs; tmp > 1; tmp >>= 1 {
		expectedSelBits++
	}

	if len(sel) != expectedSelBits {
		panic(fmt.Sprintf("logicgates: Demux sel must have %d bits for %d outputs, got %d", expectedSelBits, nOutputs, len(sel)))
	}

	validateBits(sel, "sel")

	// Use decoder to get one-hot output, then AND each with data
	decoded := Decoder(sel)
	outputs := make([]int, nOutputs)
	for i := 0; i < nOutputs; i++ {
		outputs[i] = AND(decoded[i], data)
	}
	return outputs
}

// =========================================================================
// DECODER — Binary to One-Hot
// =========================================================================
//
// A decoder converts an N-bit binary input into a one-hot output:
// exactly one of 2^N output lines is 1, the rest are 0.
//
// It is essentially a DEMUX with data hardwired to 1.
//
// Decoders are fundamental to memory addressing: the row decoder in an
// SRAM chip takes the address bits and activates exactly one word line,
// enabling read/write access to that row of cells.
//
// Construction: each output Y_i is an AND of all N input bits (or their
// complements), corresponding to the binary representation of i.
//
// Example for 2-to-4:
//
//	Y0 = AND(NOT(A1), NOT(A0))  — active when input = 00
//	Y1 = AND(NOT(A1), A0)       — active when input = 01
//	Y2 = AND(A1, NOT(A0))       — active when input = 10
//	Y3 = AND(A1, A0)            — active when input = 11

// Decoder converts an N-bit binary input to a one-hot 2^N-bit output.
//
// For an N-bit input, produces 2^N outputs where exactly one is 1.
// The output at index i is 1 when the input represents the binary
// value i.
//
// 2-to-4 Decoder truth table:
//
//	A1  A0  | Y0  Y1  Y2  Y3
//	--------|------------------
//	 0   0  |  1   0   0   0
//	 0   1  |  0   1   0   0
//	 1   0  |  0   0   1   0
//	 1   1  |  0   0   0   1
//
// Parameters:
//   - inputs: slice of N input bits (LSB first). N >= 1.
//
// Returns a slice of 2^N bits, exactly one of which is 1 (one-hot encoding).
func Decoder(inputs []int) []int {
	if len(inputs) < 1 {
		panic("logicgates: Decoder inputs must have at least 1 element")
	}

	validateBits(inputs, "inputs")

	n := len(inputs)
	nOutputs := 1 << n // 2^n

	// Precompute complements once
	complements := make([]int, n)
	for i, b := range inputs {
		complements[i] = NOT(b)
	}

	outputs := make([]int, nOutputs)
	for i := 0; i < nOutputs; i++ {
		// Output i is the AND of all input bits where the bit corresponding
		// to the binary representation of i is taken directly, and the rest
		// are complemented.
		//
		// For i=5 (binary 101) with 3 inputs [A0, A1, A2]:
		//   Y5 = AND(A0, NOT(A1), A2)
		//   because 5 in binary is: bit0=1, bit1=0, bit2=1
		result := 1
		for bitPos := 0; bitPos < n; bitPos++ {
			if (i>>bitPos)&1 == 1 {
				result = AND(result, inputs[bitPos])
			} else {
				result = AND(result, complements[bitPos])
			}
		}
		outputs[i] = result
	}

	return outputs
}

// =========================================================================
// ENCODER — One-Hot to Binary
// =========================================================================
//
// The inverse of a decoder: takes a one-hot input (exactly one bit is 1)
// and produces the binary index of that bit.
//
// If input bit 5 is active (out of 8 inputs), the encoder outputs 101
// (the binary representation of 5).

// Encoder converts a one-hot 2^N-bit input to an N-bit binary output.
//
// Exactly one input bit must be 1. The output is the binary
// representation of the index of that active bit.
//
// 4-to-2 Encoder truth table:
//
//	I0  I1  I2  I3  | A1  A0
//	----------------|--------
//	 1   0   0   0  |  0   0
//	 0   1   0   0  |  0   1
//	 0   0   1   0  |  1   0
//	 0   0   0   1  |  1   1
//
// Parameters:
//   - inputs: slice of 2^N bits in one-hot encoding (exactly one must be 1).
//     Length must be a power of 2, >= 2.
//
// Returns a slice of N bits representing the binary index of the active
// input (LSB first).
//
// Panics if input is not valid one-hot (zero or multiple bits set).
func Encoder(inputs []int) []int {
	nInputs := len(inputs)

	if nInputs < 2 || nInputs&(nInputs-1) != 0 {
		panic(fmt.Sprintf("logicgates: Encoder inputs length must be a power of 2 >= 2, got %d", nInputs))
	}

	validateBits(inputs, "inputs")

	// Validate one-hot: exactly one bit must be 1
	activeCount := 0
	activeIndex := 0
	for i, v := range inputs {
		activeCount += v
		if v == 1 {
			activeIndex = i
		}
	}
	if activeCount != 1 {
		panic(fmt.Sprintf("logicgates: Encoder inputs must be one-hot (exactly one bit = 1), got %d active bits", activeCount))
	}

	// Compute number of output bits: log2(nInputs)
	nOutputBits := 0
	for tmp := nInputs; tmp > 1; tmp >>= 1 {
		nOutputBits++
	}

	// Convert to binary (LSB first)
	output := make([]int, nOutputBits)
	for bitPos := 0; bitPos < nOutputBits; bitPos++ {
		output[bitPos] = (activeIndex >> bitPos) & 1
	}

	return output
}

// =========================================================================
// PRIORITY ENCODER — Multiple Inputs, Highest Wins
// =========================================================================
//
// A regular encoder requires exactly one active input (one-hot). In real
// systems, multiple signals can be active simultaneously — for example,
// multiple interrupt lines firing at the same time.
//
// The priority encoder solves this: it outputs the binary index of the
// HIGHEST-PRIORITY active input. Priority is determined by index — the
// highest index has the highest priority.
//
// It also outputs a "valid" flag that indicates whether ANY input is active.
// This distinguishes "no input active" from "input 0 is active" (both would
// produce output 00 without the valid flag).

// PriorityEncoder encodes the highest-priority active input.
//
// When multiple inputs are active, the one with the highest index wins.
// A "valid" output indicates whether any input is active at all.
//
// 4-to-2 Priority Encoder truth table:
//
//	I0  I1  I2  I3  | A1  A0  Valid
//	----------------|---------------
//	 0   0   0   0  |  0   0    0     No input active
//	 1   0   0   0  |  0   0    1     I0 wins (only one)
//	 X   1   0   0  |  0   1    1     I1 wins over I0
//	 X   X   1   0  |  1   0    1     I2 wins over I0,I1
//	 X   X   X   1  |  1   1    1     I3 always wins
//
// Parameters:
//   - inputs: slice of 2^N input bits. Length must be a power of 2, >= 2.
//
// Returns (binaryOutput, valid) where:
//   - binaryOutput: slice of N bits (LSB first) — index of highest active input
//   - valid: 1 if any input is active, 0 if all inputs are 0
func PriorityEncoder(inputs []int) ([]int, int) {
	nInputs := len(inputs)

	if nInputs < 2 || nInputs&(nInputs-1) != 0 {
		panic(fmt.Sprintf("logicgates: PriorityEncoder inputs length must be a power of 2 >= 2, got %d", nInputs))
	}

	validateBits(inputs, "inputs")

	// Compute number of output bits: log2(nInputs)
	nOutputBits := 0
	for tmp := nInputs; tmp > 1; tmp >>= 1 {
		nOutputBits++
	}

	// Scan from highest index to lowest — first active input wins
	highestActive := -1
	for i := nInputs - 1; i >= 0; i-- {
		if inputs[i] == 1 {
			highestActive = i
			break
		}
	}

	// Valid flag: 1 if any input was active
	valid := 0
	if highestActive != -1 {
		valid = 1
	}

	// Convert active index to binary (LSB first)
	// If no input is active, output all zeros
	index := highestActive
	if index < 0 {
		index = 0
	}

	output := make([]int, nOutputBits)
	for bitPos := 0; bitPos < nOutputBits; bitPos++ {
		output[bitPos] = (index >> bitPos) & 1
	}

	return output, valid
}

// =========================================================================
// TRI-STATE BUFFER — Three Output States
// =========================================================================
//
// Normal gates have two possible outputs: 0 or 1. A tri-state buffer adds
// a third state: HIGH-IMPEDANCE (Z), which means the output is electrically
// disconnected — as if the wire were cut.
//
// This is essential for shared buses. In a computer, the data bus connects
// the CPU, memory, and I/O devices on the same wires. Only one device can
// drive the bus at a time. Tri-state buffers let each device disconnect
// when it is not its turn, preventing electrical conflicts.
//
// In FPGAs, tri-state buffers appear in I/O blocks where pins can be
// configured as inputs (high-Z) or outputs (driven).
//
// We represent high-impedance as nil (*int pointer) in Go:
//   - enable=1: output = &data (0 or 1)
//   - enable=0: output = nil (high-Z, disconnected)

// TriState is a tri-state buffer — output can be 0, 1, or high-impedance (nil).
//
// When enabled, the buffer passes the data input through to the output.
// When disabled, the output is high-impedance (nil) — electrically
// disconnected from the wire.
//
// Truth table:
//
//	data  enable | output
//	-------------|-------
//	  0      0   |  nil     (high-Z, disconnected)
//	  1      0   |  nil     (high-Z, disconnected)
//	  0      1   |  &0      (driving low)
//	  1      1   |  &1      (driving high)
//
// Parameters:
//   - data: the data bit to pass through (0 or 1)
//   - enable: when 1, buffer is active; when 0, output is high-Z (nil)
//
// Returns a pointer to the data value when enabled, nil when disabled.
func TriState(data, enable int) *int {
	validateBit(data, "data")
	validateBit(enable, "enable")

	if enable == 0 {
		return nil
	}

	result := data
	return &result
}
