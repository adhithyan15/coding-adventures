// =========================================================================
// decoder.go — ARM1 Instruction Decoder
// =========================================================================
//
// The ARM1's instruction decoder takes a 32-bit instruction word and
// extracts the fields that control execution: which operation, which
// registers, what shift, what offset, etc.
//
// # Instruction Classes
//
// ARMv1 instructions are classified by bits 27:25:
//
//   Bits 27:26  Bit 25  Class
//   ──────────  ──────  ─────
//   00          —       Data Processing / PSR Transfer
//   01          —       Single Data Transfer (LDR/STR)
//   10          0       Block Data Transfer (LDM/STM)
//   10          1       Branch (B/BL)
//   11          —       Coprocessor / SWI
//
// # Encoding Format (Data Processing)
//
//   31  28 27 26 25 24  21 20 19  16 15  12 11           0
//   ┌─────┬─────┬──┬──────┬──┬──────┬──────┬─────────────┐
//   │Cond │ 00  │I │Opcode│S │  Rn  │  Rd  │  Operand2   │
//   └─────┴─────┴──┴──────┴──┴──────┴──────┴─────────────┘
//
// The decoder on the real ARM1 was a PLA (Programmable Logic Array) with
// just 42 rows of 36-bit microinstructions — 1,512 bits total. This is
// dramatically less than the 68000's 34,000+ bits of microcode.

package arm1simulator

import "fmt"

// =========================================================================
// Instruction types
// =========================================================================

const (
	InstDataProcessing = iota
	InstLoadStore
	InstBlockTransfer
	InstBranch
	InstSWI
	InstCoprocessor
	InstUndefined
)

// =========================================================================
// Decoded instruction
// =========================================================================

// DecodedInstruction holds all fields extracted from a 32-bit instruction.
type DecodedInstruction struct {
	Raw  uint32 // Original 32-bit instruction word
	Type int    // InstDataProcessing, InstLoadStore, etc.

	// Condition (bits 31:28) — present on ALL instructions
	Cond int

	// ── Data Processing fields ─────────────────────────────────────────
	Opcode    int  // ALU operation (bits 24:21)
	S         bool // Set flags (bit 20)
	Rn        int  // First operand register (bits 19:16)
	Rd        int  // Destination register (bits 15:12)
	Immediate bool // I bit (bit 25): true = rotated immediate, false = shifted register

	// Operand2 — immediate form
	Imm8   uint32 // 8-bit immediate (bits 7:0)
	Rotate uint32 // Rotation amount (bits 11:8)

	// Operand2 — register form
	Rm            int  // Second operand register (bits 3:0)
	ShiftType     int  // 0=LSL, 1=LSR, 2=ASR, 3=ROR (bits 6:5)
	ShiftByReg    bool // Shift amount from register? (bit 4)
	ShiftImm      int  // Immediate shift amount (bits 11:7)
	Rs            int  // Shift amount register (bits 11:8)

	// ── Load/Store fields ──────────────────────────────────────────────
	Load       bool // L bit: true=LDR, false=STR
	Byte       bool // B bit: true=byte transfer
	PreIndex   bool // P bit: true=pre-indexed, false=post-indexed
	Up         bool // U bit: true=add offset, false=subtract
	WriteBack  bool // W bit: true=write back address to Rn
	Offset12   uint32 // 12-bit immediate offset (bits 11:0)

	// ── Block Transfer fields ──────────────────────────────────────────
	RegisterList uint16 // 16-bit register bitmap (bits 15:0)
	ForceUser    bool   // S bit for block transfer

	// ── Branch fields ──────────────────────────────────────────────────
	Link         bool   // L bit: true=BL (Branch with Link)
	BranchOffset int32  // Sign-extended 24-bit offset × 4

	// ── SWI fields ─────────────────────────────────────────────────────
	SWIComment uint32 // 24-bit comment field (bits 23:0)
}

// Decode extracts all fields from a 32-bit ARM instruction.
//
// This is the behavioral equivalent of the ARM1's PLA decoder. The real
// hardware uses combinational gate trees to extract these fields in
// parallel. We do the same thing with bit masking and shifting.
func Decode(instruction uint32) DecodedInstruction {
	d := DecodedInstruction{
		Raw:  instruction,
		Cond: int((instruction >> 28) & 0xF),
	}

	// Classify by bits 27:25
	bits2726 := (instruction >> 26) & 0x3
	bit25 := (instruction >> 25) & 0x1

	switch {
	case bits2726 == 0:
		// Data Processing
		d.Type = InstDataProcessing
		d.decodeDataProcessing(instruction)

	case bits2726 == 1:
		// Single Data Transfer (LDR/STR)
		d.Type = InstLoadStore
		d.decodeLoadStore(instruction)

	case bits2726 == 2 && bit25 == 0:
		// Block Data Transfer (LDM/STM)
		d.Type = InstBlockTransfer
		d.decodeBlockTransfer(instruction)

	case bits2726 == 2 && bit25 == 1:
		// Branch (B/BL)
		d.Type = InstBranch
		d.decodeBranch(instruction)

	case bits2726 == 3:
		// Check if SWI (bits 27:24 = 1111)
		if (instruction>>24)&0xF == 0xF {
			d.Type = InstSWI
			d.SWIComment = instruction & 0x00FFFFFF
		} else {
			// Coprocessor — ARM1 has no coprocessor, so this traps
			d.Type = InstCoprocessor
		}

	default:
		d.Type = InstUndefined
	}

	return d
}

func (d *DecodedInstruction) decodeDataProcessing(inst uint32) {
	d.Immediate = ((inst >> 25) & 1) == 1
	d.Opcode = int((inst >> 21) & 0xF)
	d.S = ((inst >> 20) & 1) == 1
	d.Rn = int((inst >> 16) & 0xF)
	d.Rd = int((inst >> 12) & 0xF)

	if d.Immediate {
		d.Imm8 = inst & 0xFF
		d.Rotate = (inst >> 8) & 0xF
	} else {
		d.Rm = int(inst & 0xF)
		d.ShiftType = int((inst >> 5) & 0x3)
		d.ShiftByReg = ((inst >> 4) & 1) == 1
		if d.ShiftByReg {
			d.Rs = int((inst >> 8) & 0xF)
		} else {
			d.ShiftImm = int((inst >> 7) & 0x1F)
		}
	}
}

func (d *DecodedInstruction) decodeLoadStore(inst uint32) {
	d.Immediate = ((inst >> 25) & 1) == 1 // Note: for LDR/STR, I=1 means REGISTER offset
	d.PreIndex = ((inst >> 24) & 1) == 1
	d.Up = ((inst >> 23) & 1) == 1
	d.Byte = ((inst >> 22) & 1) == 1
	d.WriteBack = ((inst >> 21) & 1) == 1
	d.Load = ((inst >> 20) & 1) == 1
	d.Rn = int((inst >> 16) & 0xF)
	d.Rd = int((inst >> 12) & 0xF)

	if d.Immediate {
		// Register offset (I=1 for load/store means register, opposite of data processing!)
		d.Rm = int(inst & 0xF)
		d.ShiftType = int((inst >> 5) & 0x3)
		d.ShiftImm = int((inst >> 7) & 0x1F)
	} else {
		// Immediate offset
		d.Offset12 = inst & 0xFFF
	}
}

func (d *DecodedInstruction) decodeBlockTransfer(inst uint32) {
	d.PreIndex = ((inst >> 24) & 1) == 1
	d.Up = ((inst >> 23) & 1) == 1
	d.ForceUser = ((inst >> 22) & 1) == 1
	d.WriteBack = ((inst >> 21) & 1) == 1
	d.Load = ((inst >> 20) & 1) == 1
	d.Rn = int((inst >> 16) & 0xF)
	d.RegisterList = uint16(inst & 0xFFFF)
}

func (d *DecodedInstruction) decodeBranch(inst uint32) {
	d.Link = ((inst >> 24) & 1) == 1

	// The 24-bit offset is sign-extended to 32 bits, then shifted left by 2
	// (since instructions are word-aligned). This gives a range of ±32 MiB.
	offset := inst & 0x00FFFFFF
	// Sign-extend from 24 bits to 32 bits
	if (offset >> 23) != 0 {
		offset |= 0xFF000000
	}
	d.BranchOffset = int32(offset) << 2
}

// =========================================================================
// Disassembly
// =========================================================================

// Disassemble returns a human-readable assembly string for the instruction.
func (d *DecodedInstruction) Disassemble() string {
	cond := CondString(d.Cond)

	switch d.Type {
	case InstDataProcessing:
		return d.disasmDataProcessing(cond)
	case InstLoadStore:
		return d.disasmLoadStore(cond)
	case InstBlockTransfer:
		return d.disasmBlockTransfer(cond)
	case InstBranch:
		return d.disasmBranch(cond)
	case InstSWI:
		if d.SWIComment == HaltSWI {
			return fmt.Sprintf("HLT%s", cond)
		}
		return fmt.Sprintf("SWI%s #0x%X", cond, d.SWIComment)
	case InstCoprocessor:
		return fmt.Sprintf("CDP%s (undefined)", cond)
	default:
		return fmt.Sprintf("UND%s #0x%08X", cond, d.Raw)
	}
}

func (d *DecodedInstruction) disasmDataProcessing(cond string) string {
	op := OpString(d.Opcode)
	suf := ""
	if d.S && !IsTestOp(d.Opcode) {
		suf = "S"
	}

	op2 := d.disasmOperand2()

	switch {
	case d.Opcode == OpMOV || d.Opcode == OpMVN:
		return fmt.Sprintf("%s%s%s R%d, %s", op, cond, suf, d.Rd, op2)
	case IsTestOp(d.Opcode):
		return fmt.Sprintf("%s%s R%d, %s", op, cond, d.Rn, op2)
	default:
		return fmt.Sprintf("%s%s%s R%d, R%d, %s", op, cond, suf, d.Rd, d.Rn, op2)
	}
}

func (d *DecodedInstruction) disasmOperand2() string {
	if d.Immediate {
		val, _ := DecodeImmediate(d.Imm8, d.Rotate)
		return fmt.Sprintf("#%d", val)
	}
	if !d.ShiftByReg && d.ShiftImm == 0 && d.ShiftType == ShiftLSL {
		return fmt.Sprintf("R%d", d.Rm)
	}
	if d.ShiftByReg {
		return fmt.Sprintf("R%d, %s R%d", d.Rm, ShiftString(d.ShiftType), d.Rs)
	}
	// Special case: LSR #0 encodes LSR #32, etc.
	amount := d.ShiftImm
	if amount == 0 {
		switch d.ShiftType {
		case ShiftLSR, ShiftASR:
			amount = 32
		case ShiftROR:
			return fmt.Sprintf("R%d, RRX", d.Rm)
		}
	}
	return fmt.Sprintf("R%d, %s #%d", d.Rm, ShiftString(d.ShiftType), amount)
}

func (d *DecodedInstruction) disasmLoadStore(cond string) string {
	op := "STR"
	if d.Load {
		op = "LDR"
	}
	bSuf := ""
	if d.Byte {
		bSuf = "B"
	}

	var offset string
	if d.Immediate {
		// Register offset
		offset = fmt.Sprintf("R%d", d.Rm)
		if d.ShiftImm != 0 {
			offset += fmt.Sprintf(", %s #%d", ShiftString(d.ShiftType), d.ShiftImm)
		}
	} else {
		offset = fmt.Sprintf("#%d", d.Offset12)
	}

	sign := ""
	if !d.Up {
		sign = "-"
	}

	if d.PreIndex {
		wb := ""
		if d.WriteBack {
			wb = "!"
		}
		return fmt.Sprintf("%s%s%s R%d, [R%d, %s%s]%s", op, cond, bSuf, d.Rd, d.Rn, sign, offset, wb)
	}
	return fmt.Sprintf("%s%s%s R%d, [R%d], %s%s", op, cond, bSuf, d.Rd, d.Rn, sign, offset)
}

func (d *DecodedInstruction) disasmBlockTransfer(cond string) string {
	op := "STM"
	if d.Load {
		op = "LDM"
	}

	// Determine addressing mode suffix
	var mode string
	switch {
	case !d.PreIndex && d.Up:
		mode = "IA"
	case d.PreIndex && d.Up:
		mode = "IB"
	case !d.PreIndex && !d.Up:
		mode = "DA"
	case d.PreIndex && !d.Up:
		mode = "DB"
	}

	wb := ""
	if d.WriteBack {
		wb = "!"
	}

	regs := disasmRegList(d.RegisterList)
	return fmt.Sprintf("%s%s%s R%d%s, {%s}", op, cond, mode, d.Rn, wb, regs)
}

func (d *DecodedInstruction) disasmBranch(cond string) string {
	op := "B"
	if d.Link {
		op = "BL"
	}
	// Show the offset as a relative value
	return fmt.Sprintf("%s%s #%d", op, cond, d.BranchOffset)
}

func disasmRegList(list uint16) string {
	result := ""
	for i := 0; i < 16; i++ {
		if (list>>i)&1 == 1 {
			if result != "" {
				result += ", "
			}
			if i == 15 {
				result += "PC"
			} else if i == 14 {
				result += "LR"
			} else if i == 13 {
				result += "SP"
			} else {
				result += fmt.Sprintf("R%d", i)
			}
		}
	}
	return result
}
