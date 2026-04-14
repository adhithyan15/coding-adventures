package intel8008gatelevel

// 8-bit ALU — the arithmetic heart of the Intel 8008.
//
// # How the real 8008's ALU worked
//
// The Intel 8008 had an 8-bit ALU — double the width of the 4004's 4-bit ALU.
// Those extra bits require double the full-adder count: 8 full adders in the
// ripple-carry chain instead of 4, plus 8 NOT gates for the subtraction path.
//
// Every addition routes through:
//
//	XOR → AND → OR → full_adder → ripple_carry_adder → ALU
//
// That's real hardware simulation, not behavioral arithmetic.
//
// # Gate count vs the 4004
//
//	4004 ALU: 4 full adders × 5 gates = 20 gates
//	8008 ALU: 8 full adders × 5 gates = 40 gates (for the adder alone)
//
// The 8008 also adds:
//   - 8 NOT gates for subtraction (vs 4 in the 4004)
//   - 8 AND gates for bitwise AND
//   - 8 OR gates for bitwise OR
//   - 8 XOR gates for bitwise XOR
//   - 7-gate XOR tree + NOT for parity computation
//
// # Subtraction via two's complement
//
// The 8008 computes A - B by two's complement:
//
//	A - B = A + NOT(B) + 1
//
// The NOT gates invert all 8 bits of B. The +1 is provided as carry_in=1.
// This matches how real subtractors are built from adder hardware.
//
// # 8008 carry vs borrow convention
//
// After SUB: CY=1 means a borrow occurred (result underflowed).
// This is the OPPOSITE of the 4004's convention where CY=1 means no borrow.
// We implement this by inverting the adder's carry output:
//
//	borrow_occurred = NOT(adder_carry_out)
//
// So if A < B, the adder overflows → adder_carry=true → borrow=NOT(true)=false.
// Wait — let me restate: the 8008 sets CY=1 when a borrow DID occur, meaning
// the result wrapped negative. The two's-complement adder produces carry_out=1
// when NO borrow occurs (the result is ≥ 0). So:
//
//	8008 CY = NOT(adder_carry_out after two's complement)

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/arithmetic"
	logicgates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// GateALU is an 8-bit ALU for the Intel 8008 gate-level simulator.
//
// All operations route through real logic gates via the arithmetic
// package's ALU struct. No behavioral shortcuts.
type GateALU struct {
	alu *arithmetic.ALU
}

// NewGateALU creates an 8-bit ALU using real logic gates.
//
// The arithmetic.ALU(8) internally chains 8 full adders (each built from
// XOR, AND, OR gates), producing a 40-gate ripple-carry adder.
func NewGateALU() *GateALU {
	result, _ := StartNew[*GateALU]("intel8008-gatelevel.NewGateALU", nil,
		func(op *Operation[*GateALU], rf *ResultFactory[*GateALU]) *OperationResult[*GateALU] {
			return rf.Generate(true, false, &GateALU{alu: arithmetic.NewALU(8)})
		}).GetResult()
	return result
}

// Add adds two 8-bit values with carry.
//
// Routes through: XOR → AND → OR → full_adder × 8 → ripple_carry
//
// Parameters:
//   - a: First operand (0-255).
//   - b: Second operand (0-255).
//   - carryIn: 0 for ADD, carry flag value for ADC.
//
// Returns (result, carryOut) where result is 8-bit (0-255).
func (g *GateALU) Add(a, b, carryIn int) (int, bool) {
	type addResult struct {
		val   int
		carry bool
	}
	r, _ := StartNew[addResult]("intel8008-gatelevel.GateALU.Add", addResult{},
		func(op *Operation[addResult], rf *ResultFactory[addResult]) *OperationResult[addResult] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			op.AddProperty("carryIn", carryIn)
			aBits := IntToBits(a, 8)
			bBits := IntToBits(b, 8)

			if carryIn != 0 {
				// Simulate carry_in by adding a+b, then adding 1.
				// This models how the LSB full adder receives carry_in.
				result1 := g.alu.Execute(arithmetic.ADD, aBits, bBits)
				oneBits := IntToBits(1, 8)
				result2 := g.alu.Execute(arithmetic.ADD, result1.Value, oneBits)
				// Carry out if either addition overflowed
				carry := result1.Carry || result2.Carry
				return rf.Generate(true, false, addResult{val: BitsToInt(result2.Value), carry: carry})
			}

			res := g.alu.Execute(arithmetic.ADD, aBits, bBits)
			return rf.Generate(true, false, addResult{val: BitsToInt(res.Value), carry: res.Carry})
		}).GetResult()
	return r.val, r.carry
}

// Subtract subtracts b from a using two's complement: A + NOT(B) + 1.
//
// The 8008 carry convention for subtraction:
//
//	CY=1 means a borrow occurred (A < B, result underflowed)
//	CY=0 means no borrow (A >= B, result is non-negative)
//
// Internally: adder_carry = NOT(borrow), so we invert the adder output.
//
// Parameters:
//   - a: Minuend (0-255).
//   - b: Subtrahend (0-255).
//   - borrowIn: 0 for SUB, carry flag value for SBB (1 = extra borrow).
//
// Returns (result, borrowOut) where borrowOut=true means borrow occurred.
func (g *GateALU) Subtract(a, b, borrowIn int) (int, bool) {
	type subResult struct {
		val   int
		borrow bool
	}
	r, _ := StartNew[subResult]("intel8008-gatelevel.GateALU.Subtract", subResult{},
		func(op *Operation[subResult], rf *ResultFactory[subResult]) *OperationResult[subResult] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			op.AddProperty("borrowIn", borrowIn)

			// NOT each bit of b using NOT gates (8 NOT gates)
			bBits := IntToBits(b, 8)
			bComp := g.alu.Execute(arithmetic.NOT, bBits, bBits)
			bCompVal := BitsToInt(bComp.Value)

			// A + NOT(B) + 1 (the "+1" implements two's complement)
			// The carry_in for the adder is 1 always (for two's complement).
			// For SBB: also subtract the existing borrow by reducing carry_in.
			// If borrowIn=0 (no extra borrow): carry_in = 1 (normal two's complement)
			// If borrowIn=1 (subtract borrow too): carry_in = 0 (borrow steals the +1)
			carryIn := 1 - borrowIn // borrowIn=0 → carryIn=1, borrowIn=1 → carryIn=0
			val, adderCarry := g.Add(a, bCompVal, carryIn)

			// 8008 convention: CY=1 means borrow occurred = NOT(adderCarry)
			borrow := logicgates.NOT(func() int {
				if adderCarry {
					return 1
				}
				return 0
			}()) == 1

			return rf.Generate(true, false, subResult{val: val, borrow: borrow})
		}).GetResult()
	return r.val, r.borrow
}

// BitwiseAnd performs 8-bit AND using 8 AND gates.
//
// Used by ANA instruction. Carry is cleared (not set) by AND.
//
// Parameters:
//   - a: First operand (0-255).
//   - b: Second operand (0-255).
func (g *GateALU) BitwiseAnd(a, b int) int {
	result, _ := StartNew[int]("intel8008-gatelevel.GateALU.BitwiseAnd", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			aBits := IntToBits(a, 8)
			bBits := IntToBits(b, 8)
			res := g.alu.Execute(arithmetic.AND, aBits, bBits)
			return rf.Generate(true, false, BitsToInt(res.Value))
		}).GetResult()
	return result
}

// BitwiseOr performs 8-bit OR using 8 OR gates.
//
// Used by ORA instruction. Carry is cleared by OR.
func (g *GateALU) BitwiseOr(a, b int) int {
	result, _ := StartNew[int]("intel8008-gatelevel.GateALU.BitwiseOr", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			aBits := IntToBits(a, 8)
			bBits := IntToBits(b, 8)
			res := g.alu.Execute(arithmetic.OR, aBits, bBits)
			return rf.Generate(true, false, BitsToInt(res.Value))
		}).GetResult()
	return result
}

// BitwiseXor performs 8-bit XOR using 8 XOR gates.
//
// Used by XRA instruction. Carry is cleared by XOR.
func (g *GateALU) BitwiseXor(a, b int) int {
	result, _ := StartNew[int]("intel8008-gatelevel.GateALU.BitwiseXor", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			aBits := IntToBits(a, 8)
			bBits := IntToBits(b, 8)
			res := g.alu.Execute(arithmetic.XOR, aBits, bBits)
			return rf.Generate(true, false, BitsToInt(res.Value))
		}).GetResult()
	return result
}

// Increment adds 1 to an 8-bit value using the ripple-carry adder.
//
// Used by INR instruction. Carry and borrow flags are NOT affected by INR
// (the 8008 manual specifies this). We still compute carry_out for internal use.
func (g *GateALU) Increment(a int) (int, bool) {
	type incResult struct {
		val   int
		carry bool
	}
	r, _ := StartNew[incResult]("intel8008-gatelevel.GateALU.Increment", incResult{},
		func(op *Operation[incResult], rf *ResultFactory[incResult]) *OperationResult[incResult] {
			op.AddProperty("a", a)
			val, carry := g.Add(a, 1, 0)
			return rf.Generate(true, false, incResult{val: val, carry: carry})
		}).GetResult()
	return r.val, r.carry
}

// Decrement subtracts 1 from an 8-bit value.
//
// A - 1 = A + NOT(1) + 1 = A + 0xFE + 1 via the full adder chain.
// Carry flag NOT affected by DCR (per 8008 manual).
func (g *GateALU) Decrement(a int) (int, bool) {
	type decResult struct {
		val    int
		borrow bool
	}
	r, _ := StartNew[decResult]("intel8008-gatelevel.GateALU.Decrement", decResult{},
		func(op *Operation[decResult], rf *ResultFactory[decResult]) *OperationResult[decResult] {
			op.AddProperty("a", a)
			val, borrow := g.Subtract(a, 1, 0)
			return rf.Generate(true, false, decResult{val: val, borrow: borrow})
		}).GetResult()
	return r.val, r.borrow
}

// RotateLeftCircular rotates A left circular: A7→CY, A0←A7.
//
// No adder gates needed — this is pure bit rewiring:
//
//	new_A = [old_A7, old_A0, old_A1, ..., old_A6]  (bits shifted left, A7 wraps to bit 0)
//	new_CY = old_A7
//
// "Circular" means the rotated-out bit goes to both CY and bit 0.
func (g *GateALU) RotateLeftCircular(a int) (int, bool) {
	type rotResult struct {
		val   int
		carry bool
	}
	r, _ := StartNew[rotResult]("intel8008-gatelevel.GateALU.RotateLeftCircular", rotResult{},
		func(op *Operation[rotResult], rf *ResultFactory[rotResult]) *OperationResult[rotResult] {
			op.AddProperty("a", a)
			bits := IntToBits(a, 8)
			// A7 goes to CY and to bit 0; bits 0-6 shift to bits 1-7
			newBits := make([]int, 8)
			newBits[0] = bits[7] // A7 → A0 (wraparound)
			for i := 1; i < 8; i++ {
				newBits[i] = bits[i-1] // shift left
			}
			return rf.Generate(true, false, rotResult{val: BitsToInt(newBits), carry: bits[7] == 1})
		}).GetResult()
	return r.val, r.carry
}

// RotateRightCircular rotates A right circular: A0→CY, A7←A0.
//
// new_A = [old_A1, old_A2, ..., old_A7, old_A0]  (bits shifted right, A0 wraps to bit 7)
// new_CY = old_A0
func (g *GateALU) RotateRightCircular(a int) (int, bool) {
	type rotResult struct {
		val   int
		carry bool
	}
	r, _ := StartNew[rotResult]("intel8008-gatelevel.GateALU.RotateRightCircular", rotResult{},
		func(op *Operation[rotResult], rf *ResultFactory[rotResult]) *OperationResult[rotResult] {
			op.AddProperty("a", a)
			bits := IntToBits(a, 8)
			// A0 goes to CY and to bit 7; bits 1-7 shift to bits 0-6
			newBits := make([]int, 8)
			newBits[7] = bits[0] // A0 → A7 (wraparound)
			for i := 0; i < 7; i++ {
				newBits[i] = bits[i+1] // shift right
			}
			return rf.Generate(true, false, rotResult{val: BitsToInt(newBits), carry: bits[0] == 1})
		}).GetResult()
	return r.val, r.carry
}

// RotateLeftThroughCarry is a 9-bit rotate left: [CY|A7..A0] → [A7..A0|CY] shifted left.
//
// new_CY = old_A7
// new_A0 = old_CY
// new_A[i+1] = old_A[i] for i in 0..6
//
// This is RAL (Rotate Accumulator Left) in 8008 terminology.
// Unlike RLC (circular), the carry flag acts as an extra bit in the rotation.
func (g *GateALU) RotateLeftThroughCarry(a int, carryIn bool) (int, bool) {
	type rotResult struct {
		val   int
		carry bool
	}
	r, _ := StartNew[rotResult]("intel8008-gatelevel.GateALU.RotateLeftThroughCarry", rotResult{},
		func(op *Operation[rotResult], rf *ResultFactory[rotResult]) *OperationResult[rotResult] {
			op.AddProperty("a", a)
			op.AddProperty("carryIn", carryIn)
			bits := IntToBits(a, 8)
			oldCarry := 0
			if carryIn {
				oldCarry = 1
			}
			// Shift left: bit[i] → bit[i+1]; bit 0 ← old carry; new carry ← bit 7
			newBits := make([]int, 8)
			newBits[0] = oldCarry
			for i := 1; i < 8; i++ {
				newBits[i] = bits[i-1]
			}
			return rf.Generate(true, false, rotResult{val: BitsToInt(newBits), carry: bits[7] == 1})
		}).GetResult()
	return r.val, r.carry
}

// RotateRightThroughCarry is a 9-bit rotate right: [A7..A0|CY] → [CY|A7..A0] shifted right.
//
// new_CY = old_A0
// new_A7 = old_CY
// new_A[i] = old_A[i+1] for i in 0..6
//
// This is RAR (Rotate Accumulator Right) in 8008 terminology.
func (g *GateALU) RotateRightThroughCarry(a int, carryIn bool) (int, bool) {
	type rotResult struct {
		val   int
		carry bool
	}
	r, _ := StartNew[rotResult]("intel8008-gatelevel.GateALU.RotateRightThroughCarry", rotResult{},
		func(op *Operation[rotResult], rf *ResultFactory[rotResult]) *OperationResult[rotResult] {
			op.AddProperty("a", a)
			op.AddProperty("carryIn", carryIn)
			bits := IntToBits(a, 8)
			oldCarry := 0
			if carryIn {
				oldCarry = 1
			}
			// Shift right: bit[i] → bit[i-1]; bit 7 ← old carry; new carry ← bit 0
			newBits := make([]int, 8)
			newBits[7] = oldCarry
			for i := 0; i < 7; i++ {
				newBits[i] = bits[i+1]
			}
			return rf.Generate(true, false, rotResult{val: BitsToInt(newBits), carry: bits[0] == 1})
		}).GetResult()
	return r.val, r.carry
}

// ComputeFlags computes all four 8008 flags from an 8-bit result.
//
// The 8008 has 4 flags:
//
//	Zero:   NOR of all 8 result bits — 1 if result == 0
//	Sign:   bit 7 of result — 1 if result is negative (MSB set)
//	Carry:  provided by the caller (from the adder or subtract operation)
//	Parity: even parity of result — 1 if even number of 1 bits
//
// Gate implementation:
//
//	zero   = NOT(OR(OR(OR(b0,b1),OR(b2,b3)),OR(OR(b4,b5),OR(b6,b7))))
//	sign   = b7  (direct wire — no gates needed)
//	parity = NOT(XOR(XOR(XOR(b0,b1),XOR(b2,b3)),XOR(XOR(b4,b5),XOR(b6,b7))))
//
// Returns (zero, sign, carry, parity) as bools.
func (g *GateALU) ComputeFlags(result int, carry bool) (zero, sign, newCarry, parity bool) {
	type flagResult struct {
		zero, sign, carry, parity bool
	}
	fr, _ := StartNew[flagResult]("intel8008-gatelevel.GateALU.ComputeFlags", flagResult{},
		func(op *Operation[flagResult], rf *ResultFactory[flagResult]) *OperationResult[flagResult] {
			op.AddProperty("result", result)
			op.AddProperty("carry", carry)
			bits := IntToBits(result, 8)

			// Zero: 8-input NOR — true only if all bits are 0
			// Built as: NOT( OR(OR(b0,b1,b2,b3), OR(b4,b5,b6,b7)) ) via cascaded ORs
			orLow := logicgates.OR(logicgates.OR(bits[0], bits[1]), logicgates.OR(bits[2], bits[3]))
			orHigh := logicgates.OR(logicgates.OR(bits[4], bits[5]), logicgates.OR(bits[6], bits[7]))
			z := logicgates.NOT(logicgates.OR(orLow, orHigh)) == 1

			// Sign: direct wire to bit 7 (MSB)
			s := bits[7] == 1

			// Parity: 7-gate XOR tree + NOT (see ComputeParity in bits.go)
			p := ComputeParity(result) == 1

			return rf.Generate(true, false, flagResult{zero: z, sign: s, carry: carry, parity: p})
		}).GetResult()
	return fr.zero, fr.sign, fr.carry, fr.parity
}

// GateCount returns the estimated gate count for the 8-bit ALU.
//
// Gate breakdown:
//
//	8 full adders × 5 gates each = 40 gates (ripple-carry adder)
//	8 NOT gates for subtraction complement
//	8 AND gates for BitwiseAnd
//	8 OR gates for BitwiseOr
//	8 XOR gates for BitwiseXor
//	7-gate XOR tree + 1 NOT for parity = 8 gates
//	Control muxing: ~16 gates
//	Total: ~96 gates
func (g *GateALU) GateCount() int {
	result, _ := StartNew[int]("intel8008-gatelevel.GateALU.GateCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, 96)
		}).GetResult()
	return result
}
