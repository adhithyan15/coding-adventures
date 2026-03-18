package arithmetic

import (
	lg "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

type ALUOp string

const (
	ADD ALUOp = "add"
	SUB ALUOp = "sub"
	AND ALUOp = "and"
	OR  ALUOp = "or"
	XOR ALUOp = "xor"
	NOT ALUOp = "not"
)

type ALUResult struct {
	Value    []int
	Zero     bool
	Carry    bool
	Negative bool
	Overflow bool
}

type ALU struct {
	BitWidth int
}

// NewALU constructs a new ALU.
func NewALU(bitWidth int) *ALU {
	if bitWidth < 1 {
		panic("bitWidth must be at least 1")
	}
	return &ALU{BitWidth: bitWidth}
}

// bitwiseOp applies a 2-input gate bitwise across two bit lists.
func bitwiseOp(a, b []int, op func(int, int) int) []int {
	result := make([]int, len(a))
	for i := 0; i < len(a); i++ {
		result[i] = op(a[i], b[i])
	}
	return result
}

// twosComplementNegate negates a number using two's complement.
func twosComplementNegate(bits []int) ([]int, int) {
	inverted := make([]int, len(bits))
	for i, b := range bits {
		inverted[i] = lg.NOT(b)
	}
	one := make([]int, len(bits))
	one[0] = 1 // LSB first
	return RippleCarryAdder(inverted, one, 0)
}

// Execute performs the requested operation.
func (alu *ALU) Execute(op ALUOp, a, b []int) ALUResult {
	if len(a) != alu.BitWidth {
		panic("a length must match bitWidth")
	}
	if op != NOT && len(b) != alu.BitWidth {
		panic("b length must match bitWidth")
	}

	var value []int
	carryBit := 0

	switch op {
	case ADD:
		value, carryBit = RippleCarryAdder(a, b, 0)
	case SUB:
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

	zero := true
	for _, bit := range value {
		if bit != 0 {
			zero = false
			break
		}
	}

	negative := len(value) > 0 && value[len(value)-1] == 1
	carry := carryBit == 1

	overflow := false
	if op == ADD || op == SUB {
		aSign := a[len(a)-1]
		var bSign int
		if op == ADD {
			bSign = b[len(b)-1]
		} else {
			bSign = lg.NOT(b[len(b)-1])
		}
		resultSign := value[len(value)-1]
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
