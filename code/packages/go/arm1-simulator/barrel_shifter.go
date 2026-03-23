// =========================================================================
// barrel_shifter.go — ARM1 Barrel Shifter (Operand2 Processing)
// =========================================================================
//
// The barrel shifter is the ARM1's most distinctive hardware feature. On the
// real chip, it was implemented as a 32×32 crossbar network of pass
// transistors — each of the 32 output bits could be connected to any of the
// 32 input bits. This allowed shifting and rotating a value by any amount
// in a single clock cycle, at zero additional cost.
//
// Every data processing instruction has a "second operand" (Operand2) that
// passes through the barrel shifter before reaching the ALU. This means
// instructions like:
//
//   ADD R0, R1, R2, LSL #3    → R0 = R1 + (R2 << 3)
//
// execute in a single cycle — the shift is free.
//
// # Operand2 Encoding
//
// The 12-bit Operand2 field (bits 11:0) has two forms:
//
// Form 1 — Immediate (I bit = 1):
//   ┌────────┬──────────┐
//   │ Rotate │  Imm8    │   value = Imm8 ROR (2 × Rotate)
//   │ bits   │ bits 7:0 │
//   │ 11:8   │          │
//   └────────┴──────────┘
//
// Form 2 — Register (I bit = 0):
//   ┌─────────┬───┬──────┬───┬──────┐
//   │Shift Amt│ 0 │ Type │ 0 │  Rm  │   Shift by immediate amount
//   │ bits    │   │ 6:5  │   │ 3:0  │
//   │ 11:7    │   │      │   │      │
//   └─────────┴───┴──────┴───┴──────┘
//
//   ┌──────┬───┬───┬──────┬───┬──────┐
//   │  Rs  │ 0 │ 0 │ Type │ 1 │  Rm  │   Shift by register value
//   │ 11:8 │ 7 │   │ 6:5  │ 4 │ 3:0  │
//   └──────┴───┴───┴──────┴───┴──────┘
//
// # Shift Types
//
//   00 = LSL (Logical Shift Left):     Fills vacated bits with 0
//   01 = LSR (Logical Shift Right):    Fills vacated bits with 0
//   10 = ASR (Arithmetic Shift Right): Fills vacated bits with sign bit
//   11 = ROR (Rotate Right):           Bits shifted out re-enter at top
//
// # Special Case: RRX (Rotate Right Extended)
//
// When the shift type is ROR and the immediate amount is 0, this encodes
// a 33-bit rotation through the carry flag:
//
//   [C|bit31|bit30|...|bit1|bit0] → rotate right by 1 →
//   [bit0|C|bit31|...|bit2|bit1]
//
// The old carry becomes bit 31, and the old bit 0 becomes the new carry.

package arm1simulator

// BarrelShift applies a shift operation to a 32-bit value.
//
// Parameters:
//   - value:       the 32-bit input (from register Rm)
//   - shiftType:   0=LSL, 1=LSR, 2=ASR, 3=ROR
//   - amount:      number of positions to shift (0–31 for immediate encoding)
//   - carryIn:     current carry flag (used for RRX and amount=0 cases)
//   - byRegister:  true if the shift amount comes from a register (affects
//                  special-case handling for amount=0)
//
// Returns:
//   - result:   the shifted 32-bit value
//   - carryOut: the carry output from the shifter (used to update C flag
//               when S bit is set for logical operations)
//
// # The carry output rules:
//
// For LSL:  carry = last bit shifted out (bit [32-amount]), or carryIn if amount=0
// For LSR:  carry = last bit shifted out (bit [amount-1]),  or carryIn if amount=0
// For ASR:  carry = last bit shifted out (bit [amount-1]),  or carryIn if amount=0
// For ROR:  carry = last bit shifted out (bit [amount-1]),  or old bit 0 for RRX
func BarrelShift(value uint32, shiftType int, amount int, carryIn bool, byRegister bool) (result uint32, carryOut bool) {
	// When shifting by a register value, the bottom 8 bits are used as the
	// shift amount. If the amount is 0, the value passes through unchanged
	// and the carry flag is unaffected.
	if byRegister && amount == 0 {
		return value, carryIn
	}

	switch shiftType {
	case ShiftLSL:
		return shiftLSL(value, amount, carryIn, byRegister)
	case ShiftLSR:
		return shiftLSR(value, amount, carryIn, byRegister)
	case ShiftASR:
		return shiftASR(value, amount, carryIn, byRegister)
	case ShiftROR:
		return shiftROR(value, amount, carryIn, byRegister)
	default:
		return value, carryIn
	}
}

// shiftLSL — Logical Shift Left
//
//   Before (LSL #3):  [b31 b30 ... b3 b2 b1 b0]
//   After:            [b28 b27 ... b0  0  0  0 ]
//   Carry out:        b29 (the last bit shifted out)
//
// Special case: LSL #0 means "no shift" — value unchanged, carry unchanged.
func shiftLSL(value uint32, amount int, carryIn bool, byRegister bool) (uint32, bool) {
	if amount == 0 {
		// LSL #0: no shift, carry unchanged
		return value, carryIn
	}
	if amount >= 32 {
		if amount == 32 {
			// Carry = bit 0 of original value
			return 0, (value & 1) != 0
		}
		// Amount > 32: result is 0, carry is 0
		return 0, false
	}
	carry := (value >> (32 - amount)) & 1
	return value << amount, carry != 0
}

// shiftLSR — Logical Shift Right
//
//   Before (LSR #3):  [b31 b30 ... b3 b2 b1 b0]
//   After:            [ 0   0   0  b31 b30 ... b3]
//   Carry out:        b2 (the last bit shifted out)
//
// Special case: immediate LSR #0 encodes LSR #32 (result = 0, carry = bit 31).
func shiftLSR(value uint32, amount int, carryIn bool, byRegister bool) (uint32, bool) {
	if amount == 0 && !byRegister {
		// Immediate LSR #0 encodes LSR #32
		return 0, (value >> 31) != 0
	}
	if amount == 0 {
		return value, carryIn
	}
	if amount >= 32 {
		if amount == 32 {
			return 0, (value >> 31) != 0
		}
		return 0, false
	}
	carry := (value >> (amount - 1)) & 1
	return value >> amount, carry != 0
}

// shiftASR — Arithmetic Shift Right (sign-extending)
//
//   Before (ASR #3):  [b31 b30 ... b3 b2 b1 b0]
//   After:            [b31 b31 b31 b31 b30 ... b3]
//   Carry out:        b2 (the last bit shifted out)
//
// The sign bit (bit 31) is replicated into the vacated positions.
// This preserves the sign of a two's complement number.
//
// Special case: immediate ASR #0 encodes ASR #32:
//   If bit 31 = 0: result = 0x00000000, carry = 0
//   If bit 31 = 1: result = 0xFFFFFFFF, carry = 1
func shiftASR(value uint32, amount int, carryIn bool, byRegister bool) (uint32, bool) {
	signBit := (value >> 31) != 0

	if amount == 0 && !byRegister {
		// Immediate ASR #0 encodes ASR #32
		if signBit {
			return 0xFFFFFFFF, true
		}
		return 0, false
	}
	if amount == 0 {
		return value, carryIn
	}
	if amount >= 32 {
		if signBit {
			return 0xFFFFFFFF, true
		}
		return 0, false
	}

	// Arithmetic right shift: cast to signed, shift, cast back
	signed := int32(value)
	result := uint32(signed >> amount)
	carry := (value >> (amount - 1)) & 1
	return result, carry != 0
}

// shiftROR — Rotate Right
//
//   Before (ROR #3):  [b31 b30 ... b3 b2 b1 b0]
//   After:            [b2  b1  b0  b31 b30 ... b3]
//   Carry out:        b2 (the last bit rotated, which is also the new bit 31)
//
// Special case: immediate ROR #0 encodes RRX (Rotate Right Extended):
//   33-bit rotation through carry flag. Old carry → bit 31, old bit 0 → new carry.
//
//   Before:  C=1, value = [b31 b30 ... b1 b0]
//   After:   C=b0, value = [1 b31 b30 ... b1]
func shiftROR(value uint32, amount int, carryIn bool, byRegister bool) (uint32, bool) {
	if amount == 0 && !byRegister {
		// RRX — Rotate Right Extended (33-bit rotation through carry)
		carry := (value & 1) != 0
		result := value >> 1
		if carryIn {
			result |= 0x80000000
		}
		return result, carry
	}
	if amount == 0 {
		return value, carryIn
	}

	// Normalize rotation amount to 0-31
	amount = amount & 31
	if amount == 0 {
		// ROR by 32 (or multiple of 32): value unchanged, carry = bit 31
		return value, (value>>31) != 0
	}

	result := (value >> amount) | (value << (32 - amount))
	carry := (result >> 31) & 1
	return result, carry != 0
}

// DecodeImmediate decodes a rotated immediate value from the Operand2 field
// when the I bit is set (bit 25 = 1).
//
// The encoding packs a wide range of constants into 12 bits:
//   - Bits 7:0:   8-bit immediate value
//   - Bits 11:8:  4-bit rotation amount (actual rotation = 2 × this value)
//
// The 8-bit value is rotated right by an even number of positions (0, 2, 4, ..., 30).
//
// Examples:
//   imm8=0xFF, rotate=0  → 0x000000FF (255)
//   imm8=0xFF, rotate=4  → 0xFF000000 (rotated right by 8)
//   imm8=0xFF, rotate=8  → 0x00FF0000 (rotated right by 16)
//   imm8=0x01, rotate=1  → 0x40000000 (1 rotated right by 2)
func DecodeImmediate(imm8 uint32, rotate uint32) (value uint32, carryOut bool) {
	rotateAmount := int(rotate * 2)
	if rotateAmount == 0 {
		return imm8, false // carry is unchanged (we return false as default)
	}
	value = (imm8 >> rotateAmount) | (imm8 << (32 - rotateAmount))
	carryOut = (value >> 31) != 0
	return value, carryOut
}
