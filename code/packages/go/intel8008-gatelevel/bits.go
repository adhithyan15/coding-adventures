// Package intel8008gatelevel implements a gate-level Intel 8008 CPU simulator.
//
// Every arithmetic operation routes through real logic gates (AND, OR, XOR, NOT)
// chained into half-adders, full-adders, a ripple-carry adder, and then an 8-bit
// ALU from the arithmetic package. Registers are built from D flip-flops via the
// logic-gates package.
//
// This file provides bit conversion helpers that bridge the integer world
// (test programs, external API) and the gate-level world (slices of 0s and 1s).
//
// # Bit ordering: LSB first
//
// All bit slices use LSB-first ordering, matching the logic-gates and arithmetic
// packages. Index 0 is the least significant bit.
//
//	IntToBits(5, 8)  ->  [1, 0, 1, 0, 0, 0, 0, 0]
//	// bit0=1(x1) + bit2=1(x4) = 5
//
// This convention maps naturally to how adders chain: bit 0 feeds the first
// full adder, bit 1 feeds the second, and so on up to bit 7 for 8-bit values
// and bit 13 for 14-bit PC/stack values.
package intel8008gatelevel

import (
	logicgates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// IntToBits converts an integer to a slice of bits (LSB first).
//
// Parameters:
//   - value: Non-negative integer to convert.
//   - width: Number of bits in the output slice.
//
// Returns a slice of 0s and 1s, length = width, LSB at index 0.
//
// Examples:
//
//	IntToBits(5, 8)       -> [1, 0, 1, 0, 0, 0, 0, 0]
//	IntToBits(0, 8)       -> [0, 0, 0, 0, 0, 0, 0, 0]
//	IntToBits(255, 8)     -> [1, 1, 1, 1, 1, 1, 1, 1]
//	IntToBits(0x3FFF, 14) -> [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
func IntToBits(value, width int) []int {
	result, _ := StartNew[[]int]("intel8008-gatelevel.IntToBits", nil,
		func(op *Operation[[]int], rf *ResultFactory[[]int]) *OperationResult[[]int] {
			op.AddProperty("value", value)
			op.AddProperty("width", width)
			// Mask to width to handle negative or oversized values
			v := value & ((1 << width) - 1)
			bits := make([]int, width)
			for i := 0; i < width; i++ {
				bits[i] = (v >> i) & 1
			}
			return rf.Generate(true, false, bits)
		}).GetResult()
	return result
}

// BitsToInt converts a slice of bits (LSB first) to an integer.
//
// Parameters:
//   - bits: Slice of 0s and 1s, LSB at index 0.
//
// Returns a non-negative integer.
//
// Examples:
//
//	BitsToInt([1, 0, 1, 0, 0, 0, 0, 0])  -> 5
//	BitsToInt([0, 0, 0, 0, 0, 0, 0, 0])  -> 0
//	BitsToInt([1, 1, 1, 1, 1, 1, 1, 1])  -> 255
func BitsToInt(bits []int) int {
	result, _ := StartNew[int]("intel8008-gatelevel.BitsToInt", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			val := 0
			for i, bit := range bits {
				val |= bit << i
			}
			return rf.Generate(true, false, val)
		}).GetResult()
	return result
}

// ComputeParity computes even parity of an 8-bit value using XOR gates.
//
// The 8008 defines P=1 as even parity (an even number of 1 bits).
// This is implemented as a chain of XOR gates followed by NOT:
//
//	xor_chain = XOR(XOR(XOR(b0,b1), XOR(b2,b3)), XOR(XOR(b4,b5), XOR(b6,b7)))
//	parity    = NOT(xor_chain)
//
// Why NOT at the end? If the XOR chain is 0, all bits cancel out — that
// means an even number of 1s, so parity = NOT(0) = 1 (even parity = true).
// If the chain is 1, there are an odd number of 1s, parity = NOT(1) = 0.
//
// The 7-gate XOR reduction tree is more efficient than 7 sequential XORs
// because it has log2(8) = 3 gate delays instead of 7.
//
// Parameters:
//   - value: 8-bit integer (0-255).
//
// Returns 1 if even parity, 0 if odd parity.
func ComputeParity(value int) int {
	result, _ := StartNew[int]("intel8008-gatelevel.ComputeParity", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("value", value)
			bits := IntToBits(value, 8)

			// Level 1: 4 XOR gates, each combining 2 adjacent bits
			//   x01 = XOR(b0, b1)
			//   x23 = XOR(b2, b3)
			//   x45 = XOR(b4, b5)
			//   x67 = XOR(b6, b7)
			x01 := logicgates.XOR(bits[0], bits[1])
			x23 := logicgates.XOR(bits[2], bits[3])
			x45 := logicgates.XOR(bits[4], bits[5])
			x67 := logicgates.XOR(bits[6], bits[7])

			// Level 2: 2 XOR gates, combining pairs
			//   x0123 = XOR(x01, x23)
			//   x4567 = XOR(x45, x67)
			x0123 := logicgates.XOR(x01, x23)
			x4567 := logicgates.XOR(x45, x67)

			// Level 3: 1 XOR gate, combining halves
			//   xAll = XOR(x0123, x4567)
			xAll := logicgates.XOR(x0123, x4567)

			// NOT: even parity means xAll==0 → P=1; odd parity means xAll==1 → P=0
			parity := logicgates.NOT(xAll)
			return rf.Generate(true, false, parity)
		}).GetResult()
	return result
}
