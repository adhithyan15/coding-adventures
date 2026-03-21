// Package riscvsimulator — opcode constants for the RV32I base integer ISA.
//
// === How RISC-V encodes instructions ===
//
// Every RISC-V instruction is exactly 32 bits wide. The lowest 7 bits ([6:0])
// always contain the "opcode" — a number that tells the CPU what general
// category of work this instruction does. Think of it as a filing system:
//
//     bits [6:0]  =  opcode  =  "which drawer to open"
//
// Within each drawer, additional fields (funct3, funct7) narrow down the
// exact operation. For example, opcode 0b0110011 means "R-type arithmetic,"
// and then funct3=0 + funct7=0 means "add," while funct3=0 + funct7=0x20
// means "sub."
//
// === The six instruction formats ===
//
// RISC-V uses only six encoding formats. Each format rearranges the 32 bits
// differently, but the opcode is always in the same place:
//
//     R-type:  [funct7 | rs2 | rs1 | funct3 | rd | opcode]  — register-to-register
//     I-type:  [imm[11:0]   | rs1 | funct3 | rd | opcode]  — immediate operand
//     S-type:  [imm[11:5] | rs2 | rs1 | funct3 | imm[4:0] | opcode]  — stores
//     B-type:  [imm[12|10:5] | rs2 | rs1 | funct3 | imm[4:1|11] | opcode]  — branches
//     U-type:  [imm[31:12]                       | rd | opcode]  — upper immediate
//     J-type:  [imm[20|10:1|11|19:12]            | rd | opcode]  — jumps
//
// This file defines the opcode constants — the "drawer labels."
package riscvsimulator

// === RV32I Base Integer Opcodes ===
//
// These are all the opcodes needed for the full RV32I base instruction set,
// plus the SYSTEM opcode for privileged operations (ecall, CSR access, mret).
const (
	// OpcodeLoad handles all memory load instructions (lb, lh, lw, lbu, lhu).
	// These are I-type: the 12-bit immediate is added to rs1 to form the
	// memory address, and the result is placed in rd.
	OpcodeLoad = 0b0000011

	// OpcodeStore handles all memory store instructions (sb, sh, sw).
	// These are S-type: the immediate is split across two bit fields,
	// and the value in rs2 is written to memory at address rs1 + imm.
	OpcodeStore = 0b0100011

	// OpcodeBranch handles all conditional branch instructions
	// (beq, bne, blt, bge, bltu, bgeu).
	// These are B-type: the immediate encodes a signed offset (in multiples
	// of 2 bytes) that is added to the PC if the branch condition is true.
	OpcodeBranch = 0b1100011

	// OpcodeJAL is the Jump And Link instruction (J-type).
	// It stores PC+4 in rd, then jumps to PC + sign-extended 20-bit offset.
	// This is the primary mechanism for function calls.
	OpcodeJAL = 0b1101111

	// OpcodeJALR is the Jump And Link Register instruction (I-type).
	// It stores PC+4 in rd, then jumps to (rs1 + imm) with the lowest
	// bit cleared. Used for returning from functions and indirect jumps.
	OpcodeJALR = 0b1100111

	// OpcodeLUI loads a 20-bit immediate into the upper 20 bits of rd,
	// zeroing the lower 12 bits. Together with addi, this lets you
	// construct any 32-bit constant in two instructions.
	OpcodeLUI = 0b0110111

	// OpcodeAUIPC adds a 20-bit immediate (shifted left by 12) to the
	// current PC and stores the result in rd. This enables PC-relative
	// addressing for data that is far away in memory.
	OpcodeAUIPC = 0b0010111

	// OpcodeOpImm handles I-type arithmetic with an immediate value.
	// The funct3 field selects the operation:
	//   0=addi, 2=slti, 3=sltiu, 4=xori, 6=ori, 7=andi
	//   1=slli, 5=srli/srai (distinguished by funct7 bit)
	OpcodeOpImm = 0b0010011

	// OpcodeOp handles R-type register-to-register arithmetic.
	// The funct3 + funct7 fields together select the operation:
	//   f3=0,f7=0x00: add    f3=0,f7=0x20: sub
	//   f3=1: sll     f3=2: slt     f3=3: sltu
	//   f3=4: xor     f3=5,f7=0x00: srl   f3=5,f7=0x20: sra
	//   f3=6: or      f3=7: and
	OpcodeOp = 0b0110011

	// OpcodeSystem handles system-level instructions:
	//   funct3=0: ecall/ebreak/mret (distinguished by funct7)
	//   funct3=1: csrrw (CSR read-write)
	//   funct3=2: csrrs (CSR read-set)
	//   funct3=3: csrrc (CSR read-clear)
	OpcodeSystem = 0b1110011
)

// === Funct3 constants for I-type immediate arithmetic (OpcodeOpImm) ===
const (
	Funct3ADDI  = 0
	Funct3SLTI  = 2
	Funct3SLTIU = 3
	Funct3XORI  = 4
	Funct3ORI   = 6
	Funct3ANDI  = 7
	Funct3SLLI  = 1
	Funct3SRLI  = 5 // also SRAI — distinguished by funct7
)

// === Funct3 constants for R-type arithmetic (OpcodeOp) ===
const (
	Funct3ADD  = 0 // also SUB — distinguished by funct7
	Funct3SLL  = 1
	Funct3SLT  = 2
	Funct3SLTU = 3
	Funct3XOR  = 4
	Funct3SRL  = 5 // also SRA — distinguished by funct7
	Funct3OR   = 6
	Funct3AND  = 7
)

// === Funct7 constants for distinguishing add/sub and srl/sra ===
const (
	Funct7Normal = 0x00 // add, srl, slli, srli
	Funct7Alt    = 0x20 // sub, sra, srai
)

// === Funct3 constants for load instructions (OpcodeLoad) ===
const (
	Funct3LB  = 0 // load byte, sign-extend to 32 bits
	Funct3LH  = 1 // load halfword (2 bytes), sign-extend
	Funct3LW  = 2 // load word (4 bytes)
	Funct3LBU = 4 // load byte, zero-extend
	Funct3LHU = 5 // load halfword, zero-extend
)

// === Funct3 constants for store instructions (OpcodeStore) ===
const (
	Funct3SB = 0 // store byte (lowest 8 bits of rs2)
	Funct3SH = 1 // store halfword (lowest 16 bits of rs2)
	Funct3SW = 2 // store word (all 32 bits of rs2)
)

// === Funct3 constants for branch instructions (OpcodeBranch) ===
const (
	Funct3BEQ  = 0 // branch if equal
	Funct3BNE  = 1 // branch if not equal
	Funct3BLT  = 4 // branch if less than (signed)
	Funct3BGE  = 5 // branch if greater or equal (signed)
	Funct3BLTU = 6 // branch if less than (unsigned)
	Funct3BGEU = 7 // branch if greater or equal (unsigned)
)

// === Funct3 constants for system instructions (OpcodeSystem) ===
const (
	Funct3PRIV  = 0 // ecall, ebreak, mret — funct7 distinguishes
	Funct3CSRRW = 1 // CSR read-write
	Funct3CSRRS = 2 // CSR read-set (bitwise OR)
	Funct3CSRRC = 3 // CSR read-clear (bitwise AND NOT)
)

// === Funct7 for privileged instructions (funct3=0) ===
const (
	Funct7ECALL = 0x00 // environment call — triggers trap
	Funct7MRET  = 0x18 // return from machine-mode trap handler
)
