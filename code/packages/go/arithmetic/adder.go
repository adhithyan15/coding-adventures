package arithmetic

import (
	lg "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// HalfAdder adds two single bits, returning (sum, carry)
func HalfAdder(a, b int) (int, int) {
	sum := lg.XOR(a, b)
	carry := lg.AND(a, b)
	return sum, carry
}

// FullAdder adds two bits plus a carry-in, returning (sum, carry)
func FullAdder(a, b, carryIn int) (int, int) {
	partialSum, partialCarry := HalfAdder(a, b)
	sum, carry2 := HalfAdder(partialSum, carryIn)
	carryOut := lg.OR(partialCarry, carry2)
	return sum, carryOut
}

// RippleCarryAdder adds two N-bit numbers using a chain of full adders.
func RippleCarryAdder(a, b []int, carryIn int) ([]int, int) {
	if len(a) != len(b) {
		panic("a and b must have the same length")
	}
	if len(a) == 0 {
		panic("bit lists must not be empty")
	}

	sumBits := make([]int, len(a))
	carry := carryIn

	for i := 0; i < len(a); i++ {
		var sumBit int
		sumBit, carry = FullAdder(a[i], b[i], carry)
		sumBits[i] = sumBit
	}

	return sumBits, carry
}
