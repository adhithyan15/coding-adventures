// Package intel4004gatelevel implements a gate-level Intel 4004 CPU simulator.
//
// Every computation routes through real logic gates (AND, OR, NOT, XOR) and
// flip-flops from the logic-gates and arithmetic packages. No behavioral
// shortcuts are used — all state is stored in D flip-flop registers, and all
// arithmetic flows through ripple-carry adders built from full adders.
//
// This file provides bit conversion helpers that bridge the integer world
// (test programs, external API) and the gate-level world (slices of 0s and 1s).
//
// # Bit ordering: LSB first
//
// All bit slices use LSB-first ordering, matching the logic-gates and arithmetic
// packages. Index 0 is the least significant bit.
//
//	IntToBits(5, 4)  ->  [1, 0, 1, 0]
//	// bit0=1(x1) + bit1=0(x2) + bit2=1(x4) + bit3=0(x8) = 5
//
// This convention maps naturally to how adders chain: bit 0 feeds the first
// full adder, bit 1 feeds the second, and so on.
package intel4004gatelevel

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
//	IntToBits(5, 4)       -> [1, 0, 1, 0]
//	IntToBits(0, 4)       -> [0, 0, 0, 0]
//	IntToBits(15, 4)      -> [1, 1, 1, 1]
//	IntToBits(0xABC, 12)  -> [0, 0, 1, 1, 1, 1, 0, 1, 0, 1, 0, 1]
func IntToBits(value, width int) []int {
	// Mask to width to handle negative or oversized values
	value = value & ((1 << width) - 1)
	bits := make([]int, width)
	for i := 0; i < width; i++ {
		bits[i] = (value >> i) & 1
	}
	return bits
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
//	BitsToInt([1, 0, 1, 0])  -> 5
//	BitsToInt([0, 0, 0, 0])  -> 0
//	BitsToInt([1, 1, 1, 1])  -> 15
func BitsToInt(bits []int) int {
	result := 0
	for i, bit := range bits {
		result |= bit << i
	}
	return result
}
