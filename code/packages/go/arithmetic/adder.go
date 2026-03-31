// Package arithmetic builds number computation circuits from logic gates.
//
// # Moving from Logic to Math
//
// In the logic-gates package, we saw how transistors combine to form gates
// that perform basic Boolean operations (AND, OR, XOR). But how do we get a
// computer to do actual math?
//
// This package answers that question. By creatively wiring together those
// fundamental logic gates, we can build circuits that add, subtract, and
// manipulate binary numbers. From a simple "Half Adder" that adds two
// individual bits, we will build up to an entire "Arithmetic Logic Unit"
// (ALU) — the mathematical heart of every CPU.
//
// # Operations
//
// Every public function is wrapped in an Operation, giving each call
// automatic timing, structured logging, and panic recovery.
package arithmetic

import (
	lg "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// HalfAdder adds two single bits.
//
// # Why "Half"?
//
// Adding two binary bits is simple, but we have to account for carrying over
// to the next column — exactly like grade-school addition:
//
//	  1
//	+ 1
//	---
//	 10  (which is 2 in binary)
//
// In the 1s column, the sum is 0, and we "carry" a 1 to the next column.
// The Half Adder produces both these outputs: a Sum bit and a Carry bit.
// It is called a "Half" adder because, while it can generate a carry, it
// cannot ACCEPT a carry input from a previous column.
//
// Truth table:
//
//	A | B | Sum | Carry
//	--|---|-----|------
//	0 | 0 |  0  |   0
//	0 | 1 |  1  |   0
//	1 | 0 |  1  |   0
//	1 | 1 |  0  |   1
//
// If you look closely at the truth table:
// - Sum is exactly the XOR operation (1 only when inputs differ).
// - Carry is exactly the AND operation (1 only when both inputs are 1).
func HalfAdder(a, b int) (sum int, carry int) {
	type halfAdderResult struct {
		sum   int
		carry int
	}
	result, _ := StartNew[halfAdderResult]("arithmetic.HalfAdder", halfAdderResult{},
		func(op *Operation[halfAdderResult], rf *ResultFactory[halfAdderResult]) *OperationResult[halfAdderResult] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			s := lg.XOR(a, b)
			c := lg.AND(a, b)
			return rf.Generate(true, false, halfAdderResult{sum: s, carry: c})
		}).GetResult()
	return result.sum, result.carry
}

// FullAdder adds two bits plus a carry bit from a previous addition.
//
// # Handling the Ripple
//
// To add multi-bit numbers, every column beyond the first might receive a
// carry from the column to its right. A Full Adder takes three inputs (A, B,
// and CarryIn) and correctly produces a Sum and a CarryOut.
//
// How to build it? We can just chain two Half Adders!
//  1. Add A and B together. This gives a partial sum and a partial carry.
//  2. Add that partial sum to the CarryIn. This gives the final sum and a
//     second partial carry.
//  3. If EITHER step generated a carry, our final CarryOut is 1 (we use an OR gate for this).
//
// Truth table (excerpt):
//
//	A | B | Cin | Sum | Cout
//	--|---|-----|-----|-----
//	0 | 1 |  1  |  0  |  1   (1 + 1 = 10 -> Sum 0, Carry 1)
//	1 | 1 |  1  |  1  |  1   (1 + 1 + 1 = 11 -> Sum 1, Carry 1)
func FullAdder(a, b, carryIn int) (sum int, carryOut int) {
	type fullAdderResult struct {
		sum     int
		carryOut int
	}
	result, _ := StartNew[fullAdderResult]("arithmetic.FullAdder", fullAdderResult{},
		func(op *Operation[fullAdderResult], rf *ResultFactory[fullAdderResult]) *OperationResult[fullAdderResult] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			op.AddProperty("carryIn", carryIn)
			partialSum, partialCarry := HalfAdder(a, b)
			s, carry2 := HalfAdder(partialSum, carryIn)
			co := lg.OR(partialCarry, carry2)
			return rf.Generate(true, false, fullAdderResult{sum: s, carryOut: co})
		}).GetResult()
	return result.sum, result.carryOut
}

// RippleCarryAdder adds two N-bit numbers by chaining Full Adders.
//
// # The Ripple Effect
//
// Just like you add large numbers on paper starting from the rightmost digit
// and moving left, the Ripple Carry Adder lines up a series of Full Adders.
// The CarryOut of bit 0 is wired directly into the CarryIn of bit 1. The
// CarryOut of bit 1 goes into bit 2, and so on.
//
// The worst-case performance is when adding something like 1111 + 0001. The
// carry generated at the first bit must "ripple" all the way through every
// single adder before the final sum is ready. In physical hardware, this
// takes time, which is why modern CPUs use faster tricks like "Carry Lookahead".
//
// Inputs are slices of integers (0 or 1), structured Little-Endian (LSB is at index 0).
func RippleCarryAdder(a, b []int, carryIn int) (sumBits []int, carryOut int) {
	type rcaResult struct {
		sumBits  []int
		carryOut int
	}
	result, _ := StartNew[rcaResult]("arithmetic.RippleCarryAdder", rcaResult{},
		func(op *Operation[rcaResult], rf *ResultFactory[rcaResult]) *OperationResult[rcaResult] {
			op.AddProperty("carryIn", carryIn)
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

			return rf.Generate(true, false, rcaResult{sumBits: sumBits, carryOut: carry})
		}).PanicOnUnexpected().GetResult()
	return result.sumBits, result.carryOut
}
