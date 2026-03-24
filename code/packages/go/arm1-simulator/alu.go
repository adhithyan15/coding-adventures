// =========================================================================
// alu.go — ARM1 32-bit ALU
// =========================================================================
//
// The ARM1's ALU performs 16 operations selected by a 4-bit opcode. It takes
// two 32-bit inputs (Rn and the barrel-shifted Operand2) and produces a
// 32-bit result plus four condition flags (N, Z, C, V).
//
// # Flag computation
//
// The N, Z, C, V flags are computed differently for logical vs arithmetic ops:
//
// Arithmetic (ADD, SUB, ADC, SBC, RSB, RSC, CMP, CMN):
//   N = result bit 31
//   Z = result == 0
//   C = carry out from the 32-bit adder
//   V = signed overflow (carry into bit 31 != carry out of bit 31)
//
// Logical (AND, EOR, TST, TEQ, ORR, MOV, BIC, MVN):
//   N = result bit 31
//   Z = result == 0
//   C = carry out from the barrel shifter (not from the ALU)
//   V = unchanged (the ALU does not modify V for logical ops)
//
// # Subtraction via addition
//
// The ARM1 implements subtraction using two's complement addition:
//   A - B = A + NOT(B) + 1
//
// This means SUB sets carry=1 when there is NO borrow (the opposite of
// what you might expect). Carry=0 means a borrow occurred.

package arm1simulator

// ALUResult holds the output of an ALU operation.
type ALUResult struct {
	Result       uint32 // The 32-bit result
	N            bool   // Negative flag (bit 31 of result)
	Z            bool   // Zero flag (result == 0)
	C            bool   // Carry flag
	V            bool   // Overflow flag
	WriteResult  bool   // Should the result be written to Rd?
}

// ALUExecute performs one of the 16 ALU operations.
//
// Parameters:
//   - opcode:       4-bit ALU operation (0x0=AND ... 0xF=MVN)
//   - a:            first operand (value of Rn)
//   - b:            second operand (barrel-shifted Operand2)
//   - carryIn:      current carry flag (used by ADC, SBC, RSC)
//   - shifterCarry: carry output from the barrel shifter (used for logical ops)
//   - oldV:         current overflow flag (preserved for logical ops)
//
// Returns an ALUResult with the computed result and flags.
func ALUExecute(opcode int, a, b uint32, carryIn bool, shifterCarry bool, oldV bool) ALUResult {
	var result uint32
	var carry, overflow bool
	writeResult := !IsTestOp(opcode)

	switch opcode {
	// ── Logical operations ─────────────────────────────────────────────
	// C flag comes from the barrel shifter, V flag is preserved.

	case OpAND, OpTST:
		result = a & b
		carry = shifterCarry
		overflow = oldV

	case OpEOR, OpTEQ:
		result = a ^ b
		carry = shifterCarry
		overflow = oldV

	case OpORR:
		result = a | b
		carry = shifterCarry
		overflow = oldV

	case OpMOV:
		result = b
		carry = shifterCarry
		overflow = oldV

	case OpBIC:
		result = a & ^b
		carry = shifterCarry
		overflow = oldV

	case OpMVN:
		result = ^b
		carry = shifterCarry
		overflow = oldV

	// ── Arithmetic operations ──────────────────────────────────────────
	// C flag comes from the adder carry-out, V flag detects signed overflow.

	case OpADD, OpCMN:
		// A + B
		result, carry, overflow = add32(a, b, false)

	case OpADC:
		// A + B + C
		result, carry, overflow = add32(a, b, carryIn)

	case OpSUB, OpCMP:
		// A - B = A + NOT(B) + 1
		result, carry, overflow = add32(a, ^b, true)

	case OpSBC:
		// A - B - NOT(C) = A + NOT(B) + C
		result, carry, overflow = add32(a, ^b, carryIn)

	case OpRSB:
		// B - A = B + NOT(A) + 1
		result, carry, overflow = add32(b, ^a, true)

	case OpRSC:
		// B - A - NOT(C) = B + NOT(A) + C
		result, carry, overflow = add32(b, ^a, carryIn)
	}

	return ALUResult{
		Result:      result,
		N:           (result >> 31) != 0,
		Z:           result == 0,
		C:           carry,
		V:           overflow,
		WriteResult: writeResult,
	}
}

// add32 performs a 32-bit addition with carry-in and computes carry-out and
// overflow flags.
//
// Carry-out: the unsigned overflow of A + B + carryIn (does the result
//            exceed 32 bits?)
//
// Overflow:  the signed overflow (the sign of the result is wrong given
//            the signs of the inputs). Detected by:
//            overflow = (A and B have same sign) AND (result has different sign)
//
// We compute this using 64-bit arithmetic for clarity. The real ARM1 uses
// a 32-stage ripple-carry adder.
func add32(a, b uint32, carryIn bool) (result uint32, carry bool, overflow bool) {
	var cin uint64
	if carryIn {
		cin = 1
	}
	sum := uint64(a) + uint64(b) + cin
	result = uint32(sum)
	carry = (sum >> 32) != 0

	// Overflow detection: both operands have the same sign, but the result
	// has a different sign. We check bit 31 of each.
	//
	//   overflow = ((a ^ result) & (b ^ result)) >> 31
	//
	// This formula works because:
	// - If a and b have different signs, addition cannot overflow
	// - If a and b have the same sign but result differs, overflow occurred
	overflow = (((a ^ result) & (b ^ result)) >> 31) != 0
	return
}
