// Package riscvsimulator — instruction decoder for all RV32I formats.
//
// === How decoding works ===
//
// The decoder's job is to take a raw 32-bit instruction and break it apart
// into meaningful fields: which registers are involved, what immediate value
// is encoded, and what specific operation to perform.
//
// The first step is always the same: read bits [6:0] to get the opcode.
// The opcode tells us which *format* the instruction uses, and the format
// tells us where each field lives within the 32 bits.
//
// === Sign extension ===
//
// Immediate values in RISC-V are always sign-extended. This means if the
// most significant bit of the immediate is 1, the value is negative, and
// we fill the upper bits with 1s to preserve the two's complement meaning.
//
// For example, a 12-bit immediate of 0xFFF represents -1, not 4095.
// After sign extension to 32 bits, it becomes 0xFFFFFFFF = -1.
package riscvsimulator

import (
	"fmt"

	cpu "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-simulator"
)

// Decode determines the instruction format by examining the opcode bits,
// then delegates to the appropriate format-specific decoder.
//
// This is the main entry point — the CPU calls this once per instruction
// in the decode stage of the pipeline.
func (d *RiscVDecoder) Decode(raw uint32, pc int) cpu.DecodeResult {
	opcode := raw & 0x7F

	switch opcode {
	case OpcodeOpImm:
		return d.decodeOpImm(raw)
	case OpcodeOp:
		return d.decodeRType(raw)
	case OpcodeLoad:
		return d.decodeLoad(raw)
	case OpcodeStore:
		return d.decodeSType(raw)
	case OpcodeBranch:
		return d.decodeBType(raw)
	case OpcodeJAL:
		return d.decodeJType(raw, pc)
	case OpcodeJALR:
		return d.decodeJALR(raw)
	case OpcodeLUI:
		return d.decodeUType(raw, "lui")
	case OpcodeAUIPC:
		return d.decodeUType(raw, "auipc")
	case OpcodeSystem:
		return d.decodeSystem(raw)
	default:
		return cpu.DecodeResult{
			Mnemonic:       fmt.Sprintf("UNKNOWN(0x%02x)", opcode),
			Fields:         map[string]int{"opcode": int(opcode)},
			RawInstruction: raw,
		}
	}
}

// === I-type arithmetic (OpcodeOpImm) decoder ===
//
// Format: [imm[11:0] | rs1 | funct3 | rd | opcode]
//
// The funct3 field selects the specific operation. For shift instructions,
// the immediate encodes the shift amount in the lower 5 bits, and funct7
// (embedded in the upper bits of the immediate) distinguishes logical
// vs arithmetic shifts.
func (d *RiscVDecoder) decodeOpImm(raw uint32) cpu.DecodeResult {
	rd := int((raw >> 7) & 0x1F)
	funct3 := int((raw >> 12) & 0x7)
	rs1 := int((raw >> 15) & 0x1F)
	imm := int(raw >> 20) // raw 12-bit immediate (not yet sign-extended)

	// Sign-extend the 12-bit immediate to 32 bits.
	// Bit 11 is the sign bit of a 12-bit value.
	if imm&0x800 != 0 {
		imm -= 0x1000
	}

	var mnemonic string
	switch funct3 {
	case Funct3ADDI:
		mnemonic = "addi"
	case Funct3SLTI:
		mnemonic = "slti"
	case Funct3SLTIU:
		mnemonic = "sltiu"
	case Funct3XORI:
		mnemonic = "xori"
	case Funct3ORI:
		mnemonic = "ori"
	case Funct3ANDI:
		mnemonic = "andi"
	case Funct3SLLI:
		// For shift instructions, the shift amount is in imm[4:0],
		// and funct7 is in imm[11:5]. We extract funct7 here.
		mnemonic = "slli"
		imm = imm & 0x1F // shift amount only
	case Funct3SRLI:
		// funct7 in bits [31:25] distinguishes srli (0x00) from srai (0x20)
		funct7 := int((raw >> 25) & 0x7F)
		if funct7 == Funct7Alt {
			mnemonic = "srai"
		} else {
			mnemonic = "srli"
		}
		imm = imm & 0x1F // shift amount only
	default:
		mnemonic = fmt.Sprintf("opimm(f3=%d)", funct3)
	}

	return cpu.DecodeResult{
		Mnemonic: mnemonic,
		Fields: map[string]int{
			"rd":     rd,
			"rs1":    rs1,
			"imm":    imm,
			"funct3": funct3,
		},
		RawInstruction: raw,
	}
}

// === R-type (OpcodeOp) decoder ===
//
// Format: [funct7 | rs2 | rs1 | funct3 | rd | opcode]
//
// Both funct3 and funct7 are needed to identify the exact operation.
// For example, add and sub share funct3=0 but differ in funct7.
func (d *RiscVDecoder) decodeRType(raw uint32) cpu.DecodeResult {
	rd := int((raw >> 7) & 0x1F)
	funct3 := int((raw >> 12) & 0x7)
	rs1 := int((raw >> 15) & 0x1F)
	rs2 := int((raw >> 20) & 0x1F)
	funct7 := int((raw >> 25) & 0x7F)

	var mnemonic string
	switch {
	case funct3 == Funct3ADD && funct7 == Funct7Normal:
		mnemonic = "add"
	case funct3 == Funct3ADD && funct7 == Funct7Alt:
		mnemonic = "sub"
	case funct3 == Funct3SLL:
		mnemonic = "sll"
	case funct3 == Funct3SLT:
		mnemonic = "slt"
	case funct3 == Funct3SLTU:
		mnemonic = "sltu"
	case funct3 == Funct3XOR:
		mnemonic = "xor"
	case funct3 == Funct3SRL && funct7 == Funct7Normal:
		mnemonic = "srl"
	case funct3 == Funct3SRL && funct7 == Funct7Alt:
		mnemonic = "sra"
	case funct3 == Funct3OR:
		mnemonic = "or"
	case funct3 == Funct3AND:
		mnemonic = "and"
	default:
		mnemonic = fmt.Sprintf("r_op(f3=%d,f7=%d)", funct3, funct7)
	}

	return cpu.DecodeResult{
		Mnemonic: mnemonic,
		Fields: map[string]int{
			"rd":     rd,
			"rs1":    rs1,
			"rs2":    rs2,
			"funct3": funct3,
			"funct7": funct7,
		},
		RawInstruction: raw,
	}
}

// === Load instruction decoder (I-type format) ===
//
// Format: [imm[11:0] | rs1 | funct3 | rd | opcode]
//
// The address is computed as rs1 + sign_extend(imm).
// funct3 selects the width and sign-extension behavior:
//   lb (byte, sign), lh (half, sign), lw (word), lbu (byte, zero), lhu (half, zero)
func (d *RiscVDecoder) decodeLoad(raw uint32) cpu.DecodeResult {
	rd := int((raw >> 7) & 0x1F)
	funct3 := int((raw >> 12) & 0x7)
	rs1 := int((raw >> 15) & 0x1F)
	imm := int(raw >> 20)

	if imm&0x800 != 0 {
		imm -= 0x1000
	}

	var mnemonic string
	switch funct3 {
	case Funct3LB:
		mnemonic = "lb"
	case Funct3LH:
		mnemonic = "lh"
	case Funct3LW:
		mnemonic = "lw"
	case Funct3LBU:
		mnemonic = "lbu"
	case Funct3LHU:
		mnemonic = "lhu"
	default:
		mnemonic = fmt.Sprintf("load(f3=%d)", funct3)
	}

	return cpu.DecodeResult{
		Mnemonic: mnemonic,
		Fields: map[string]int{
			"rd":     rd,
			"rs1":    rs1,
			"imm":    imm,
			"funct3": funct3,
		},
		RawInstruction: raw,
	}
}

// === S-type (store) decoder ===
//
// Format: [imm[11:5] | rs2 | rs1 | funct3 | imm[4:0] | opcode]
//
// The immediate is split into two non-contiguous fields — this is a key
// difference from I-type. The CPU hardware reconstructs the full immediate
// by concatenating these pieces.
//
// Why split the immediate? So that rs1, rs2, and rd fields stay in the
// same bit positions across all formats, simplifying the hardware.
func (d *RiscVDecoder) decodeSType(raw uint32) cpu.DecodeResult {
	funct3 := int((raw >> 12) & 0x7)
	rs1 := int((raw >> 15) & 0x1F)
	rs2 := int((raw >> 20) & 0x1F)

	// Reconstruct the 12-bit immediate from two pieces:
	//   imm[4:0]  = bits [11:7]  of the instruction
	//   imm[11:5] = bits [31:25] of the instruction
	immLow := int((raw >> 7) & 0x1F)
	immHigh := int((raw >> 25) & 0x7F)
	imm := (immHigh << 5) | immLow

	// Sign-extend from 12 bits
	if imm&0x800 != 0 {
		imm -= 0x1000
	}

	var mnemonic string
	switch funct3 {
	case Funct3SB:
		mnemonic = "sb"
	case Funct3SH:
		mnemonic = "sh"
	case Funct3SW:
		mnemonic = "sw"
	default:
		mnemonic = fmt.Sprintf("store(f3=%d)", funct3)
	}

	return cpu.DecodeResult{
		Mnemonic: mnemonic,
		Fields: map[string]int{
			"rs1":    rs1,
			"rs2":    rs2,
			"imm":    imm,
			"funct3": funct3,
		},
		RawInstruction: raw,
	}
}

// === B-type (branch) decoder ===
//
// Format: [imm[12|10:5] | rs2 | rs1 | funct3 | imm[4:1|11] | opcode]
//
// Branches encode a signed offset in multiples of 2 bytes (since all
// RISC-V instructions are at least 2 bytes aligned). The immediate
// bits are scattered across the instruction for hardware efficiency:
//
//   bit 31     -> imm[12]   (sign bit)
//   bits 30:25 -> imm[10:5]
//   bits 11:8  -> imm[4:1]
//   bit 7      -> imm[11]
//
// Note: imm[0] is always 0 (2-byte alignment), so it's not stored.
func (d *RiscVDecoder) decodeBType(raw uint32) cpu.DecodeResult {
	funct3 := int((raw >> 12) & 0x7)
	rs1 := int((raw >> 15) & 0x1F)
	rs2 := int((raw >> 20) & 0x1F)

	// Reconstruct the 13-bit immediate (bit 0 is implicitly 0):
	imm12 := int((raw >> 31) & 0x1)   // bit 12 (sign)
	imm11 := int((raw >> 7) & 0x1)    // bit 11
	imm10_5 := int((raw >> 25) & 0x3F) // bits 10:5
	imm4_1 := int((raw >> 8) & 0xF)   // bits 4:1

	imm := (imm12 << 12) | (imm11 << 11) | (imm10_5 << 5) | (imm4_1 << 1)

	// Sign-extend from 13 bits (bit 12 is the sign bit)
	if imm&0x1000 != 0 {
		imm -= 0x2000
	}

	var mnemonic string
	switch funct3 {
	case Funct3BEQ:
		mnemonic = "beq"
	case Funct3BNE:
		mnemonic = "bne"
	case Funct3BLT:
		mnemonic = "blt"
	case Funct3BGE:
		mnemonic = "bge"
	case Funct3BLTU:
		mnemonic = "bltu"
	case Funct3BGEU:
		mnemonic = "bgeu"
	default:
		mnemonic = fmt.Sprintf("branch(f3=%d)", funct3)
	}

	return cpu.DecodeResult{
		Mnemonic: mnemonic,
		Fields: map[string]int{
			"rs1":    rs1,
			"rs2":    rs2,
			"imm":    imm,
			"funct3": funct3,
		},
		RawInstruction: raw,
	}
}

// === J-type (JAL) decoder ===
//
// Format: [imm[20|10:1|11|19:12] | rd | opcode]
//
// JAL encodes a 21-bit signed offset (bit 0 implicit 0) for jumping
// up to +/- 1 MiB from the current PC. The bits are scrambled for
// hardware efficiency:
//
//   bit 31     -> imm[20]    (sign bit)
//   bits 30:21 -> imm[10:1]
//   bit 20     -> imm[11]
//   bits 19:12 -> imm[19:12]
func (d *RiscVDecoder) decodeJType(raw uint32, pc int) cpu.DecodeResult {
	rd := int((raw >> 7) & 0x1F)

	// Reconstruct the 21-bit immediate (bit 0 is implicitly 0):
	imm20 := int((raw >> 31) & 0x1)      // bit 20 (sign)
	imm10_1 := int((raw >> 21) & 0x3FF)   // bits 10:1
	imm11 := int((raw >> 20) & 0x1)       // bit 11
	imm19_12 := int((raw >> 12) & 0xFF)   // bits 19:12

	imm := (imm20 << 20) | (imm19_12 << 12) | (imm11 << 11) | (imm10_1 << 1)

	// Sign-extend from 21 bits
	if imm&0x100000 != 0 {
		imm -= 0x200000
	}

	return cpu.DecodeResult{
		Mnemonic: "jal",
		Fields: map[string]int{
			"rd":  rd,
			"imm": imm,
		},
		RawInstruction: raw,
	}
}

// === I-type JALR decoder ===
//
// Format: [imm[11:0] | rs1 | funct3 | rd | opcode]
//
// JALR computes the target address as (rs1 + imm) with bit 0 cleared.
// It stores PC+4 in rd before jumping. This is used for:
//   - Returning from functions (jalr x0, x1, 0  ≡  ret)
//   - Indirect jumps through function pointers
func (d *RiscVDecoder) decodeJALR(raw uint32) cpu.DecodeResult {
	rd := int((raw >> 7) & 0x1F)
	rs1 := int((raw >> 15) & 0x1F)
	imm := int(raw >> 20)

	if imm&0x800 != 0 {
		imm -= 0x1000
	}

	return cpu.DecodeResult{
		Mnemonic: "jalr",
		Fields: map[string]int{
			"rd":  rd,
			"rs1": rs1,
			"imm": imm,
		},
		RawInstruction: raw,
	}
}

// === U-type decoder (LUI / AUIPC) ===
//
// Format: [imm[31:12] | rd | opcode]
//
// U-type instructions carry a 20-bit immediate in the upper bits.
// The hardware shifts this left by 12 to produce the final value.
//
//   LUI:   rd = imm << 12
//   AUIPC: rd = PC + (imm << 12)
//
// Together with addi, LUI can construct any 32-bit constant:
//   lui  x1, 0x12345     // x1 = 0x12345000
//   addi x1, x1, 0x678   // x1 = 0x12345678
func (d *RiscVDecoder) decodeUType(raw uint32, mnemonic string) cpu.DecodeResult {
	rd := int((raw >> 7) & 0x1F)
	// The immediate is the upper 20 bits, already in position.
	// We store it as the raw 20-bit value; the executor shifts it.
	imm := int(raw >> 12)

	// Sign-extend from 20 bits to allow negative values
	if imm&0x80000 != 0 {
		imm -= 0x100000
	}

	return cpu.DecodeResult{
		Mnemonic: mnemonic,
		Fields: map[string]int{
			"rd":  rd,
			"imm": imm,
		},
		RawInstruction: raw,
	}
}

// === System instruction decoder ===
//
// Format depends on funct3:
//   funct3=0: PRIV instructions (ecall, ebreak, mret) — distinguished by funct7
//   funct3=1/2/3: CSR instructions — the immediate field holds the CSR address
//
// CSR (Control and Status Register) instructions allow reading and writing
// special registers that control CPU behavior: interrupt enables, trap
// handler addresses, exception causes, etc.
func (d *RiscVDecoder) decodeSystem(raw uint32) cpu.DecodeResult {
	funct3 := int((raw >> 12) & 0x7)

	if funct3 == Funct3PRIV {
		// For privileged instructions, funct7 (bits [31:25]) identifies the operation
		funct7 := int((raw >> 25) & 0x7F)
		switch funct7 {
		case Funct7MRET:
			return cpu.DecodeResult{
				Mnemonic:       "mret",
				Fields:         map[string]int{"funct7": funct7},
				RawInstruction: raw,
			}
		default:
			// ecall (funct7=0x00) and ebreak (funct7=0x01)
			return cpu.DecodeResult{
				Mnemonic:       "ecall",
				Fields:         map[string]int{"funct7": funct7},
				RawInstruction: raw,
			}
		}
	}

	// CSR instructions: I-type format where imm[11:0] is the CSR address
	rd := int((raw >> 7) & 0x1F)
	rs1 := int((raw >> 15) & 0x1F)
	csr := int((raw >> 20) & 0xFFF) // CSR address (12 bits, unsigned)

	var mnemonic string
	switch funct3 {
	case Funct3CSRRW:
		mnemonic = "csrrw"
	case Funct3CSRRS:
		mnemonic = "csrrs"
	case Funct3CSRRC:
		mnemonic = "csrrc"
	default:
		mnemonic = fmt.Sprintf("system(f3=%d)", funct3)
	}

	return cpu.DecodeResult{
		Mnemonic: mnemonic,
		Fields: map[string]int{
			"rd":     rd,
			"rs1":    rs1,
			"csr":    csr,
			"funct3": funct3,
		},
		RawInstruction: raw,
	}
}
