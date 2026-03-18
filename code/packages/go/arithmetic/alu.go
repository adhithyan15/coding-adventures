package arithmetic

import (
	lg "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// ALUOp represents the instruction code telling the ALU what to do.
type ALUOp string

const (
	ADD ALUOp = "add"
	SUB ALUOp = "sub"
	AND ALUOp = "and"
	OR  ALUOp = "or"
	XOR ALUOp = "xor"
	NOT ALUOp = "not"
)

// ALUResult captures the mathematical result and the status flags describing it.
type ALUResult struct {
	Value    []int // The calculated result (LSB at index 0).
	Zero     bool  // Is the result entirely zeros? (Useful for branching: "if x == 0")
	Carry    bool  // Did the unsigned addition overflow out of the top bit?
	Negative bool  // Is the highest bit 1? (In two's complement, 1=negative, 0=positive).
	Overflow bool  // Did signed arithmetic wrap around incorrectly?
}

// ALU represents an Arithmetic Logic Unit.
//
// # The CPU's Calculator
//
// An ALU is the part of a CPU that actually executes commands. You give it
// two numbers (A and B) and a control signal (the operation). It routes
// those numbers through various circuits (like our RippleCarryAdder) and
// outputs the result alongside helpful "flags" that let the CPU make decisions
// based on the result (like "Jump if Zero").
type ALU struct {
	BitWidth int // How wide are the data busses? (e.g., 8-bit, 16-bit, 32-bit CPU)
}

// NewALU initializes an ALU with a fixed bus width.
func NewALU(bitWidth int) *ALU {
	if bitWidth < 1 {
		panic("bitWidth must be at least 1")
	}
	return &ALU{BitWidth: bitWidth}
}

// bitwiseOp runs a single-bit logic gate parallel across an entire array of bits.
func bitwiseOp(a, b []int, op func(int, int) int) []int {
	result := make([]int, len(a))
	for i := 0; i < len(a); i++ {
		result[i] = op(a[i], b[i])
	}
	return result
}

// twosComplementNegate converts a positive binary number to negative.
//
// # Two's Complement Magic
//
// How do computing systems represent negative numbers? They use a trick called
// Two's Complement. To turn `x` into `-x`:
//  1. Flip every bit (NOT operation).
//  2. Add 1.
//
// Why this works: A number `x` plus its bitwise inverse `NOT(x)` is always
// a number with all 1s (e.g., 1111). If you add 1 to `1111`, it rolls over
// to `0000` (disregarding the carry out). So:
//   x + NOT(x) = 1111
//   x + NOT(x) + 1 = 0000
// Therefore:
//   NOT(x) + 1 = -x
//
// The beauty of this is that the ALU can use the EXACT same adder circuit for
// both positive and negative math. No special subtraction hardware is needed!
func twosComplementNegate(bits []int) ([]int, int) {
	inverted := make([]int, len(bits))
	for i, b := range bits {
		inverted[i] = lg.NOT(b)
	}
	one := make([]int, len(bits))
	one[0] = 1 // Add 1 (LSB is at index 0)
	return RippleCarryAdder(inverted, one, 0)
}

// Execute performs an arithmetic or logical operation.
//
// It routes the A and B buses into the appropriate circuit based on the
// op code, and then computes the condition flags corresponding to the output.
func (alu *ALU) Execute(op ALUOp, a, b []int) ALUResult {
	if len(a) != alu.BitWidth {
		panic("a length must match bitWidth")
	}
	// The NOT instruction only uses the A bus, so B can be empty.
	if op != NOT && len(b) != alu.BitWidth {
		panic("b length must match bitWidth")
	}

	var value []int
	carryBit := 0

	// 1. Calculate the result based on the requested operation.
	switch op {
	case ADD:
		value, carryBit = RippleCarryAdder(a, b, 0)
	case SUB:
		// A - B is mathematically equivalent to A + (-B).
		// We use Two's Complement to negate B, and add them!
		negB, _ := twosComplementNegate(b)
		value, carryBit = RippleCarryAdder(a, negB, 0)
	case AND:
		value = bitwiseOp(a, b, lg.AND)
	case OR:
		value = bitwiseOp(a, b, lg.OR)
	case XOR:
		value = bitwiseOp(a, b, lg.XOR)
	case NOT:
		value = make([]int, len(a))
		for i, bit := range a {
			value[i] = lg.NOT(bit)
		}
	default:
		panic("unknown operation")
	}

	// 2. Calculate the condition flags.
	
	// Zero flag is true if every single bit is 0.
	zero := true
	for _, bit := range value {
		if bit != 0 {
			zero = false
			break
		}
	}

	// Negative flag simply checks the Most Significant Bit (MSB).
	// In two's complement, an MSB of 1 signifies a negative number.
	negative := len(value) > 0 && value[len(value)-1] == 1
	carry := carryBit == 1

	// Overflow flag indicates when the sign of the result is mathematically
	// impossible, implying we "ran out of bits" to represent the magnitude.
	// E.g., Adding two large positive numbers shouldn't give a negative sum.
	overflow := false
	if op == ADD || op == SUB {
		aSign := a[len(a)-1]
		var bSign int
		if op == ADD {
			bSign = b[len(b)-1]
		} else {
			// For subtraction, we are adding NOT(B) + 1, so the effective
			// sign of the second operand in the inner addition is inverted.
			bSign = lg.NOT(b[len(b)-1])
		}
		resultSign := value[len(value)-1]
		
		// If both operands had the same sign, but the result has a different sign,
		// an overflow corruption occurred.
		if aSign == bSign && resultSign != aSign {
			overflow = true
		}
	}

	return ALUResult{
		Value:    value,
		Zero:     zero,
		Carry:    carry,
		Negative: negative,
		Overflow: overflow,
	}
}
