// =========================================================================
// alu.go — 32-Bit Gate-Level ALU for the ARM1
// =========================================================================
//
// This ALU wraps the arithmetic package's ripple-carry adder and uses
// logic gate functions from the logic-gates package. Every ADD instruction
// traverses a chain of 32 full adders, each built from XOR, AND, and OR
// gates. Total: ~160 gate calls per addition.
//
// The ARM1's ALU supports 16 operations. For arithmetic ops (ADD, SUB, etc.),
// we use the ripple-carry adder. For logical ops (AND, EOR, ORR, etc.),
// we apply gate functions bit-by-bit across all 32 bits.

package arm1gatelevel

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/arithmetic"
	gates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// GateALUResult holds the output of a gate-level ALU operation.
type GateALUResult struct {
	Result []int // 32-bit result (LSB first)
	N      int   // Negative flag (bit 31 of result)
	Z      int   // Zero flag (1 if result is all zeros)
	C      int   // Carry flag
	V      int   // Overflow flag
}

// GateALUExecute performs one of the 16 ALU operations using gate-level logic.
//
// Every operation routes through actual gate function calls:
//   - Arithmetic: ripple_carry_adder (32 full adders → 160+ gate calls)
//   - Logical: AND/OR/XOR/NOT applied to each of 32 bits (32-64 gate calls)
//
// Parameters:
//   - opcode: 4-bit ALU operation (0=AND ... 15=MVN)
//   - a: first operand (Rn value), 32 bits LSB-first
//   - b: second operand (after barrel shifter), 32 bits LSB-first
//   - carryIn: current carry flag (0 or 1)
//   - shifterCarry: carry from barrel shifter (0 or 1)
//   - oldV: current overflow flag (0 or 1)
func GateALUExecute(opcode int, a, b []int, carryIn, shifterCarry, oldV int) GateALUResult {
	result, _ := StartNew[GateALUResult]("arm1-gatelevel.GateALUExecute", GateALUResult{},
		func(op *Operation[GateALUResult], rf *ResultFactory[GateALUResult]) *OperationResult[GateALUResult] {
			op.AddProperty("opcode", opcode)
			op.AddProperty("carryIn", carryIn)
			op.AddProperty("shifterCarry", shifterCarry)
			op.AddProperty("oldV", oldV)

			var res []int
			var carry, overflow int

			switch opcode {
			// ── Logical operations ─────────────────────────────────────────────
			// Each bit processed independently through gate functions.
			// C flag comes from barrel shifter, V flag preserved.

			case 0x0, 0x8: // AND, TST
				res = bitwiseGate(a, b, gates.AND)
				carry = shifterCarry
				overflow = oldV

			case 0x1, 0x9: // EOR, TEQ
				res = bitwiseGate(a, b, gates.XOR)
				carry = shifterCarry
				overflow = oldV

			case 0xC: // ORR
				res = bitwiseGate(a, b, gates.OR)
				carry = shifterCarry
				overflow = oldV

			case 0xD: // MOV
				res = make([]int, len(b))
				copy(res, b)
				carry = shifterCarry
				overflow = oldV

			case 0xE: // BIC = AND(a, NOT(b))
				notB := bitwiseNot(b)
				res = bitwiseGate(a, notB, gates.AND)
				carry = shifterCarry
				overflow = oldV

			case 0xF: // MVN = NOT(b)
				res = bitwiseNot(b)
				carry = shifterCarry
				overflow = oldV

			// ── Arithmetic operations ──────────────────────────────────────────
			// All route through the ripple-carry adder (32 full adders chained).

			case 0x4, 0xB: // ADD, CMN: A + B
				res, carry = arithmetic.RippleCarryAdder(a, b, 0)
				overflow = computeOverflow(a, b, res)

			case 0x5: // ADC: A + B + C
				res, carry = arithmetic.RippleCarryAdder(a, b, carryIn)
				overflow = computeOverflow(a, b, res)

			case 0x2, 0xA: // SUB, CMP: A - B = A + NOT(B) + 1
				notB := bitwiseNot(b)
				res, carry = arithmetic.RippleCarryAdder(a, notB, 1)
				overflow = computeOverflow(a, notB, res)

			case 0x6: // SBC: A - B - !C = A + NOT(B) + C
				notB := bitwiseNot(b)
				res, carry = arithmetic.RippleCarryAdder(a, notB, carryIn)
				overflow = computeOverflow(a, notB, res)

			case 0x3: // RSB: B - A = B + NOT(A) + 1
				notA := bitwiseNot(a)
				res, carry = arithmetic.RippleCarryAdder(b, notA, 1)
				overflow = computeOverflow(b, notA, res)

			case 0x7: // RSC: B - A - !C = B + NOT(A) + C
				notA := bitwiseNot(a)
				res, carry = arithmetic.RippleCarryAdder(b, notA, carryIn)
				overflow = computeOverflow(b, notA, res)

			default:
				res = make([]int, 32)
			}

			// Compute N and Z flags from result bits using gates
			n := res[31] // Negative = MSB

			// Zero flag: NOR of all 32 result bits
			// Z = 1 only when all bits are 0
			z := computeZero(res)

			return rf.Generate(true, false, GateALUResult{
				Result: res,
				N:      n,
				Z:      z,
				C:      carry,
				V:      overflow,
			})
		}).GetResult()
	return result
}

// bitwiseGate applies a 2-input gate function to each bit pair.
// This is how the real ARM1 does AND, OR, XOR — 32 gate instances in parallel.
func bitwiseGate(a, b []int, gate func(int, int) int) []int {
	result := make([]int, len(a))
	for i := range a {
		result[i] = gate(a[i], b[i])
	}
	return result
}

// bitwiseNot applies NOT to each bit.
func bitwiseNot(bits []int) []int {
	result := make([]int, len(bits))
	for i := range bits {
		result[i] = gates.NOT(bits[i])
	}
	return result
}

// computeZero checks if all 32 bits are zero using NOR gates.
// In hardware, this is a tree of NOR/OR gates reducing 32 bits to 1.
func computeZero(bits []int) int {
	// OR all bits together, then NOT the result
	// A tree reduction: OR pairs, then OR the results, etc.
	combined := bits[0]
	for i := 1; i < len(bits); i++ {
		combined = gates.OR(combined, bits[i])
	}
	return gates.NOT(combined)
}

// computeOverflow detects signed overflow using XOR gates.
// Overflow occurs when the carry into the MSB differs from the carry out.
// We detect this by checking if both operands have the same sign bit but
// the result has a different sign bit.
func computeOverflow(a, b, result []int) int {
	// V = (a[31] XOR result[31]) AND (b[31] XOR result[31])
	// This is 1 when both inputs have the same sign but result differs.
	xor1 := gates.XOR(a[31], result[31])
	xor2 := gates.XOR(b[31], result[31])
	return gates.AND(xor1, xor2)
}
