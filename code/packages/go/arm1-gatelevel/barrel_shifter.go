// =========================================================================
// barrel_shifter.go — Gate-Level Barrel Shifter for the ARM1
// =========================================================================
//
// On the real ARM1, the barrel shifter was implemented as a 32×32 crossbar
// network of pass transistors. We model it with a 5-level tree of Mux2
// gates from the logic-gates package.
//
// Each level handles one bit of the shift amount:
//   Level 0: shift by 0 or 1   (controlled by amount bit 0)
//   Level 1: shift by 0 or 2   (controlled by amount bit 1)
//   Level 2: shift by 0 or 4   (controlled by amount bit 2)
//   Level 3: shift by 0 or 8   (controlled by amount bit 3)
//   Level 4: shift by 0 or 16  (controlled by amount bit 4)
//
// Each level uses 32 Mux2 gates, for a total of 5 × 32 = 160 Mux2 gates.
// Each Mux2 uses ~4 primitive gates (2 AND, 1 OR, 1 NOT), giving
// ~640 gate calls per shift operation.

package arm1gatelevel

import (
	gates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// GateBarrelShift performs a shift operation on a 32-bit value using
// a tree of multiplexer gates.
//
// Returns the shifted value (32 bits) and carry-out (1 bit).
func GateBarrelShift(value []int, shiftType, amount int, carryIn int, byRegister bool) ([]int, int) {
	if byRegister && amount == 0 {
		result := make([]int, 32)
		copy(result, value)
		return result, carryIn
	}

	switch shiftType {
	case 0: // LSL
		return gateLSL(value, amount, carryIn, byRegister)
	case 1: // LSR
		return gateLSR(value, amount, carryIn, byRegister)
	case 2: // ASR
		return gateASR(value, amount, carryIn, byRegister)
	case 3: // ROR
		return gateROR(value, amount, carryIn, byRegister)
	default:
		result := make([]int, 32)
		copy(result, value)
		return result, carryIn
	}
}

// gateLSL — Logical Shift Left using a 5-level multiplexer tree.
//
// For LSL, each output bit i gets the input from bit (i - shiftAmount),
// or 0 if i < shiftAmount. We implement this as a cascade of conditional
// shifts, each controlled by one bit of the shift amount.
func gateLSL(value []int, amount int, carryIn int, byRegister bool) ([]int, int) {
	if amount == 0 {
		result := make([]int, 32)
		copy(result, value)
		return result, carryIn
	}
	if amount >= 32 {
		result := make([]int, 32)
		if amount == 32 {
			return result, value[0]
		}
		return result, 0
	}

	// Build through 5 levels of muxes
	current := make([]int, 32)
	copy(current, value)

	carry := carryIn
	for level := 0; level < 5; level++ {
		shift := 1 << level
		sel := (amount >> level) & 1
		next := make([]int, 32)
		for i := 0; i < 32; i++ {
			var shifted int
			if i >= shift {
				shifted = current[i-shift]
			} else {
				shifted = 0
			}
			// Mux2: sel=0 → keep current, sel=1 → take shifted
			next[i] = gates.Mux2(current[i], shifted, sel)
		}
		current = next
	}

	// Carry = last bit shifted out = bit (32 - amount) of original
	if amount > 0 && amount <= 32 {
		carry = value[32-amount]
	}
	return current, carry
}

// gateLSR — Logical Shift Right using mux tree.
func gateLSR(value []int, amount int, carryIn int, byRegister bool) ([]int, int) {
	if amount == 0 && !byRegister {
		// Immediate LSR #0 encodes LSR #32
		result := make([]int, 32)
		return result, value[31]
	}
	if amount == 0 {
		result := make([]int, 32)
		copy(result, value)
		return result, carryIn
	}
	if amount >= 32 {
		result := make([]int, 32)
		if amount == 32 {
			return result, value[31]
		}
		return result, 0
	}

	current := make([]int, 32)
	copy(current, value)

	for level := 0; level < 5; level++ {
		shift := 1 << level
		sel := (amount >> level) & 1
		next := make([]int, 32)
		for i := 0; i < 32; i++ {
			var shifted int
			if i+shift < 32 {
				shifted = current[i+shift]
			} else {
				shifted = 0 // Fill with 0
			}
			next[i] = gates.Mux2(current[i], shifted, sel)
		}
		current = next
	}

	carry := value[amount-1]
	return current, carry
}

// gateASR — Arithmetic Shift Right (sign-extending) using mux tree.
func gateASR(value []int, amount int, carryIn int, byRegister bool) ([]int, int) {
	signBit := value[31]

	if amount == 0 && !byRegister {
		// Immediate ASR #0 encodes ASR #32
		result := make([]int, 32)
		for i := range result {
			result[i] = signBit
		}
		return result, signBit
	}
	if amount == 0 {
		result := make([]int, 32)
		copy(result, value)
		return result, carryIn
	}
	if amount >= 32 {
		result := make([]int, 32)
		for i := range result {
			result[i] = signBit
		}
		return result, signBit
	}

	current := make([]int, 32)
	copy(current, value)

	for level := 0; level < 5; level++ {
		shift := 1 << level
		sel := (amount >> level) & 1
		next := make([]int, 32)
		for i := 0; i < 32; i++ {
			var shifted int
			if i+shift < 32 {
				shifted = current[i+shift]
			} else {
				shifted = signBit // Fill with sign bit
			}
			next[i] = gates.Mux2(current[i], shifted, sel)
		}
		current = next
	}

	carry := value[amount-1]
	return current, carry
}

// gateROR — Rotate Right using mux tree.
func gateROR(value []int, amount int, carryIn int, byRegister bool) ([]int, int) {
	if amount == 0 && !byRegister {
		// RRX: 33-bit rotate through carry
		result := make([]int, 32)
		for i := 1; i < 32; i++ {
			result[i-1] = value[i]
		}
		result[31] = carryIn // Old carry becomes MSB
		carry := value[0]    // Old LSB becomes new carry
		return result, carry
	}
	if amount == 0 {
		result := make([]int, 32)
		copy(result, value)
		return result, carryIn
	}

	// Normalize to 0-31
	amount = amount & 31
	if amount == 0 {
		result := make([]int, 32)
		copy(result, value)
		return result, value[31]
	}

	current := make([]int, 32)
	copy(current, value)

	for level := 0; level < 5; level++ {
		shift := 1 << level
		sel := (amount >> level) & 1
		next := make([]int, 32)
		for i := 0; i < 32; i++ {
			// Rotate: bits wrap around
			shifted := current[(i+shift)%32]
			next[i] = gates.Mux2(current[i], shifted, sel)
		}
		current = next
	}

	// Carry = MSB of result
	return current, current[31]
}

// GateDecodeImmediate decodes a rotated immediate using gate-level rotation.
func GateDecodeImmediate(imm8 uint32, rotate uint32) ([]int, int) {
	// Convert 8-bit immediate to 32-bit
	bits := IntToBits(imm8, 32)
	rotateAmount := int(rotate * 2)
	if rotateAmount == 0 {
		return bits, 0
	}
	result, carry := gateROR(bits, rotateAmount, 0, false)
	return result, carry
}
