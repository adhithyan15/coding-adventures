package intel4004gatelevel

// Instruction decoder — combinational logic that maps opcodes to control signals.
//
// # How instruction decoding works in hardware
//
// The decoder takes an 8-bit instruction byte and produces control signals
// that tell the rest of the CPU what to do. In the real 4004, this was a
// combinational logic network — a forest of AND, OR, and NOT gates that
// pattern-match the opcode bits.
//
// For example, to detect LDM (0xD_):
//
//	is_ldm = AND(bit7, bit6, NOT(bit5), bit4)  -> bits 7654 = 1101
//
// The decoder doesn't use sequential logic — it's purely combinational.
// Given the same input bits, it always produces the same output signals.
//
// # Control signals
//
// The decoder outputs tell the control unit what to do:
//   - IsLDM, IsLD, etc.: instruction family detection
//   - IsTwoByte: instruction is 2 bytes
//   - RegIndex: which register (lower nibble)
//   - PairIndex: which register pair
//   - Immediate: immediate value from instruction
//   - Addr12: 12-bit address (JUN/JMS)
//   - Addr8: 8-bit address/data (JCN/ISZ/FIM)

import (
	logicgates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// DecodedInstruction holds the control signals produced by the instruction decoder.
//
// Every field represents a wire carrying a 0 or 1 signal, or a
// multi-bit value extracted from the instruction.
type DecodedInstruction struct {
	// Original instruction bytes
	Raw  int
	Raw2 int  // -1 if no second byte
	HasRaw2 bool

	// Upper and lower nibbles
	Upper int // bits [7:4]
	Lower int // bits [3:0]

	// Instruction family detection (from gate logic)
	IsNOP   int
	IsHLT   int
	IsLDM   int
	IsLD    int
	IsXCH   int
	IsINC   int
	IsADD   int
	IsSUB   int
	IsJUN   int
	IsJCN   int
	IsISZ   int
	IsJMS   int
	IsBBL   int
	IsFIM   int
	IsSRC   int
	IsFIN   int
	IsJIN   int
	IsIO    int // 0xE_ range
	IsAccum int // 0xF_ range

	// Two-byte flag
	IsTwoByte int

	// Operand extraction
	RegIndex  int // lower nibble (register index)
	PairIndex int // lower nibble >> 1 (pair index)
	Immediate int // lower nibble (immediate value)
	Condition int // lower nibble (JCN condition code)

	// For 2-byte instructions
	Addr12 int // 12-bit address (JUN/JMS)
	Addr8  int // 8-bit address/data (JCN/ISZ/FIM)
}

// Decode decodes an instruction byte into control signals using gates.
//
// In real hardware, this is a combinational circuit — no clock needed.
// The input bits propagate through AND/OR/NOT gate trees to produce
// the output control signals.
//
// Parameters:
//   - raw: First instruction byte (0x00-0xFF).
//   - raw2: Second byte for 2-byte instructions, or -1 if none.
//
// Returns DecodedInstruction with all control signals set.
func Decode(raw int, raw2 int) DecodedInstruction {
	// Extract individual bits using AND gates (masking)
	b7 := (raw >> 7) & 1
	b6 := (raw >> 6) & 1
	b5 := (raw >> 5) & 1
	b4 := (raw >> 4) & 1
	b3 := (raw >> 3) & 1
	b2 := (raw >> 2) & 1
	b1 := (raw >> 1) & 1
	b0 := raw & 1

	upper := (raw >> 4) & 0xF
	lower := raw & 0xF

	// --- Instruction family detection ---
	// Each family is detected by AND-ing the upper nibble bits.
	// Using NOT for inverted bits.

	// NOP = 0x00: all bits zero
	isNOP := logicgates.AND(
		logicgates.AND(logicgates.NOT(b7), logicgates.NOT(b6)),
		logicgates.AND(logicgates.AND(logicgates.NOT(b5), logicgates.NOT(b4)),
			logicgates.AND(logicgates.NOT(b3), logicgates.NOT(b2))),
	)
	isNOP = logicgates.AND(isNOP, logicgates.AND(logicgates.NOT(b1), logicgates.NOT(b0)))

	// HLT = 0x01: only b0 is 1
	isHLT := logicgates.AND(
		logicgates.AND(logicgates.NOT(b7), logicgates.NOT(b6)),
		logicgates.AND(logicgates.AND(logicgates.NOT(b5), logicgates.NOT(b4)),
			logicgates.AND(logicgates.NOT(b3), logicgates.NOT(b2))),
	)
	isHLT = logicgates.AND(isHLT, logicgates.AND(logicgates.NOT(b1), b0))

	// Upper nibble patterns (using gate logic):
	// 0x1_ = 0001 : JCN
	isJCNFamily := logicgates.AND(logicgates.AND(logicgates.NOT(b7), logicgates.NOT(b6)),
		logicgates.AND(logicgates.NOT(b5), b4))

	// 0x2_ = 0010 : FIM (even b0) or SRC (odd b0)
	is2x := logicgates.AND(logicgates.AND(logicgates.NOT(b7), logicgates.NOT(b6)),
		logicgates.AND(b5, logicgates.NOT(b4)))
	isFIM := logicgates.AND(is2x, logicgates.NOT(b0))
	isSRC := logicgates.AND(is2x, b0)

	// 0x3_ = 0011 : FIN (even b0) or JIN (odd b0)
	is3x := logicgates.AND(logicgates.AND(logicgates.NOT(b7), logicgates.NOT(b6)),
		logicgates.AND(b5, b4))
	isFIN := logicgates.AND(is3x, logicgates.NOT(b0))
	isJIN := logicgates.AND(is3x, b0)

	// 0x4_ = 0100 : JUN
	isJUNFamily := logicgates.AND(logicgates.AND(logicgates.NOT(b7), b6),
		logicgates.AND(logicgates.NOT(b5), logicgates.NOT(b4)))

	// 0x5_ = 0101 : JMS
	isJMSFamily := logicgates.AND(logicgates.AND(logicgates.NOT(b7), b6),
		logicgates.AND(logicgates.NOT(b5), b4))

	// 0x6_ = 0110 : INC
	isINCFamily := logicgates.AND(logicgates.AND(logicgates.NOT(b7), b6),
		logicgates.AND(b5, logicgates.NOT(b4)))

	// 0x7_ = 0111 : ISZ
	isISZFamily := logicgates.AND(logicgates.AND(logicgates.NOT(b7), b6),
		logicgates.AND(b5, b4))

	// 0x8_ = 1000 : ADD
	isADDFamily := logicgates.AND(logicgates.AND(b7, logicgates.NOT(b6)),
		logicgates.AND(logicgates.NOT(b5), logicgates.NOT(b4)))

	// 0x9_ = 1001 : SUB
	isSUBFamily := logicgates.AND(logicgates.AND(b7, logicgates.NOT(b6)),
		logicgates.AND(logicgates.NOT(b5), b4))

	// 0xA_ = 1010 : LD
	isLDFamily := logicgates.AND(logicgates.AND(b7, logicgates.NOT(b6)),
		logicgates.AND(b5, logicgates.NOT(b4)))

	// 0xB_ = 1011 : XCH
	isXCHFamily := logicgates.AND(logicgates.AND(b7, logicgates.NOT(b6)),
		logicgates.AND(b5, b4))

	// 0xC_ = 1100 : BBL
	isBBLFamily := logicgates.AND(logicgates.AND(b7, b6),
		logicgates.AND(logicgates.NOT(b5), logicgates.NOT(b4)))

	// 0xD_ = 1101 : LDM
	isLDMFamily := logicgates.AND(logicgates.AND(b7, b6),
		logicgates.AND(logicgates.NOT(b5), b4))

	// 0xE_ = 1110 : I/O operations
	isIOFamily := logicgates.AND(logicgates.AND(b7, b6),
		logicgates.AND(b5, logicgates.NOT(b4)))

	// 0xF_ = 1111 : accumulator operations
	isAccumFamily := logicgates.AND(logicgates.AND(b7, b6),
		logicgates.AND(b5, b4))

	// Two-byte detection
	isTwoByte := logicgates.OR(
		logicgates.OR(isJCNFamily, isJUNFamily),
		logicgates.OR(logicgates.OR(isJMSFamily, isISZFamily), isFIM),
	)

	// Operand extraction
	regIndex := lower
	pairIndex := lower >> 1
	immediate := lower
	condition := lower

	// 12-bit address for JUN/JMS
	second := 0
	hasRaw2 := false
	if raw2 >= 0 {
		second = raw2
		hasRaw2 = true
	}
	addr12 := (lower << 8) | second
	addr8 := second

	return DecodedInstruction{
		Raw:     raw,
		Raw2:    raw2,
		HasRaw2: hasRaw2,
		Upper:   upper,
		Lower:   lower,

		IsNOP:   isNOP,
		IsHLT:   isHLT,
		IsLDM:   isLDMFamily,
		IsLD:    isLDFamily,
		IsXCH:   isXCHFamily,
		IsINC:   isINCFamily,
		IsADD:   isADDFamily,
		IsSUB:   isSUBFamily,
		IsJUN:   isJUNFamily,
		IsJCN:   isJCNFamily,
		IsISZ:   isISZFamily,
		IsJMS:   isJMSFamily,
		IsBBL:   isBBLFamily,
		IsFIM:   isFIM,
		IsSRC:   isSRC,
		IsFIN:   isFIN,
		IsJIN:   isJIN,
		IsIO:    isIOFamily,
		IsAccum: isAccumFamily,

		IsTwoByte: isTwoByte,

		RegIndex:  regIndex,
		PairIndex: pairIndex,
		Immediate: immediate,
		Condition: condition,

		Addr12: addr12,
		Addr8:  addr8,
	}
}
