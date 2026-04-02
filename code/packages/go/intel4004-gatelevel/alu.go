package intel4004gatelevel

// 4-bit ALU — the arithmetic heart of the Intel 4004.
//
// # How the real 4004's ALU worked
//
// The Intel 4004 had a 4-bit ALU that could add, subtract, and perform
// logical operations on 4-bit values. It used a ripple-carry adder built
// from full adders, which were themselves built from AND, OR, and XOR gates.
//
// This struct wraps the arithmetic package's ALU(bitWidth=4) to provide
// the exact operations the 4004 needs. Every addition and subtraction
// physically routes through the gate chain:
//
//	XOR -> AND -> OR -> full_adder -> ripple_carry_adder -> ALU
//
// That's real hardware simulation — not behavioral shortcuts.
//
// # Subtraction via complement-add
//
// The 4004 doesn't have a dedicated subtractor. Instead, it uses the
// ones' complement method:
//
//	A - B = A + NOT(B) + borrow_in
//
// where borrow_in = 0 if carry_flag else 1 (inverted carry semantics).
// The ALU's NOT operation flips all bits using NOT gates, then feeding
// through the same adder.

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/arithmetic"
)

// GateALU is a 4-bit ALU for the Intel 4004 gate-level simulator.
//
// All operations route through real logic gates via the arithmetic
// package's ALU struct. No behavioral shortcuts.
//
// The ALU provides:
//   - Add(a, b, carryIn) -> (result, carryOut)
//   - Subtract(a, b, borrowIn) -> (result, carryOut)
//   - Complement(a) -> result (4-bit NOT)
//   - Increment(a) -> (result, carryOut)
//   - Decrement(a) -> (result, borrowOut)
type GateALU struct {
	alu *arithmetic.ALU
}

// NewGateALU creates a 4-bit ALU using real logic gates.
func NewGateALU() *GateALU {
	result, _ := StartNew[*GateALU]("intel4004-gatelevel.NewGateALU", nil,
		func(op *Operation[*GateALU], rf *ResultFactory[*GateALU]) *OperationResult[*GateALU] {
			return rf.Generate(true, false, &GateALU{alu: arithmetic.NewALU(4)})
		}).GetResult()
	return result
}

// Add adds two 4-bit values with carry.
//
// Routes through: XOR -> AND -> OR -> full_adder x 4 -> ripple_carry
//
// Parameters:
//   - a: First operand (0-15).
//   - b: Second operand (0-15).
//   - carryIn: Carry from previous operation (0 or 1).
//
// Returns (result, carryOut) where result is 4-bit (0-15).
func (g *GateALU) Add(a, b, carryIn int) (int, bool) {
	type addResult struct {
		val   int
		carry bool
	}
	r, _ := StartNew[addResult]("intel4004-gatelevel.GateALU.Add", addResult{},
		func(op *Operation[addResult], rf *ResultFactory[addResult]) *OperationResult[addResult] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			op.AddProperty("carryIn", carryIn)
			aBits := IntToBits(a, 4)
			bBits := IntToBits(b, 4)

			if carryIn != 0 {
				// Add carry_in by first adding a+b, then adding 1
				// This simulates the carry input to the LSB full adder
				result1 := g.alu.Execute(arithmetic.ADD, aBits, bBits)
				oneBits := IntToBits(1, 4)
				result2 := g.alu.Execute(arithmetic.ADD, result1.Value, oneBits)
				// Carry is set if either addition overflowed
				carry := result1.Carry || result2.Carry
				return rf.Generate(true, false, addResult{val: BitsToInt(result2.Value), carry: carry})
			}

			res := g.alu.Execute(arithmetic.ADD, aBits, bBits)
			return rf.Generate(true, false, addResult{val: BitsToInt(res.Value), carry: res.Carry})
		}).GetResult()
	return r.val, r.carry
}

// Subtract subtracts using complement-add: A + NOT(B) + borrowIn.
//
// The 4004's carry flag semantics for subtraction:
//
//	carry=true  -> no borrow (result >= 0)
//	carry=false -> borrow occurred
//
// Parameters:
//   - a: Minuend (0-15).
//   - b: Subtrahend (0-15).
//   - borrowIn: 1 if no previous borrow, 0 if borrow.
//
// Returns (result, carryOut) where carryOut=true means no borrow.
func (g *GateALU) Subtract(a, b, borrowIn int) (int, bool) {
	type subResult struct {
		val   int
		carry bool
	}
	r, _ := StartNew[subResult]("intel4004-gatelevel.GateALU.Subtract", subResult{},
		func(op *Operation[subResult], rf *ResultFactory[subResult]) *OperationResult[subResult] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			op.AddProperty("borrowIn", borrowIn)
			// Complement b using NOT gates
			bBits := IntToBits(b, 4)
			bComp := g.alu.Execute(arithmetic.NOT, bBits, bBits)
			// A + NOT(B) + borrowIn
			val, carry := g.Add(a, BitsToInt(bComp.Value), borrowIn)
			return rf.Generate(true, false, subResult{val: val, carry: carry})
		}).GetResult()
	return r.val, r.carry
}

// Complement performs a 4-bit NOT: invert all bits using NOT gates.
//
// Parameters:
//   - a: Value to complement (0-15).
//
// Returns the complemented value (0-15).
func (g *GateALU) Complement(a int) int {
	result, _ := StartNew[int]("intel4004-gatelevel.GateALU.Complement", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("a", a)
			aBits := IntToBits(a, 4)
			res := g.alu.Execute(arithmetic.NOT, aBits, aBits)
			return rf.Generate(true, false, BitsToInt(res.Value))
		}).GetResult()
	return result
}

// Increment adds 1 using the adder. Returns (result, carry).
func (g *GateALU) Increment(a int) (int, bool) {
	type incResult struct {
		val   int
		carry bool
	}
	r, _ := StartNew[incResult]("intel4004-gatelevel.GateALU.Increment", incResult{},
		func(op *Operation[incResult], rf *ResultFactory[incResult]) *OperationResult[incResult] {
			op.AddProperty("a", a)
			val, carry := g.Add(a, 1, 0)
			return rf.Generate(true, false, incResult{val: val, carry: carry})
		}).GetResult()
	return r.val, r.carry
}

// Decrement subtracts 1 using complement-add.
//
// A - 1 = A + NOT(1) + 1 = A + 14 + 1 = A + 15.
// carry=true if A > 0 (no borrow), false if A == 0.
func (g *GateALU) Decrement(a int) (int, bool) {
	type decResult struct {
		val   int
		carry bool
	}
	r, _ := StartNew[decResult]("intel4004-gatelevel.GateALU.Decrement", decResult{},
		func(op *Operation[decResult], rf *ResultFactory[decResult]) *OperationResult[decResult] {
			op.AddProperty("a", a)
			val, carry := g.Subtract(a, 1, 1)
			return rf.Generate(true, false, decResult{val: val, carry: carry})
		}).GetResult()
	return r.val, r.carry
}

// BitwiseAnd performs a 4-bit AND using AND gates.
func (g *GateALU) BitwiseAnd(a, b int) int {
	result, _ := StartNew[int]("intel4004-gatelevel.GateALU.BitwiseAnd", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			aBits := IntToBits(a, 4)
			bBits := IntToBits(b, 4)
			res := g.alu.Execute(arithmetic.AND, aBits, bBits)
			return rf.Generate(true, false, BitsToInt(res.Value))
		}).GetResult()
	return result
}

// BitwiseOr performs a 4-bit OR using OR gates.
func (g *GateALU) BitwiseOr(a, b int) int {
	result, _ := StartNew[int]("intel4004-gatelevel.GateALU.BitwiseOr", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			aBits := IntToBits(a, 4)
			bBits := IntToBits(b, 4)
			res := g.alu.Execute(arithmetic.OR, aBits, bBits)
			return rf.Generate(true, false, BitsToInt(res.Value))
		}).GetResult()
	return result
}

// GateCount returns the estimated gate count for a 4-bit ALU.
//
// Each full adder: 5 gates (2 XOR + 2 AND + 1 OR).
// 4-bit ripple carry: 4 x 5 = 20 gates.
// SUB complement: 4 NOT gates.
// Control muxing: ~8 gates.
// Total: ~32 gates.
func (g *GateALU) GateCount() int {
	result, _ := StartNew[int]("intel4004-gatelevel.GateALU.GateCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, 32)
		}).GetResult()
	return result
}
