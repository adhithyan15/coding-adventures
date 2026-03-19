package core

import (
	cpupipeline "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-pipeline"
)

// =========================================================================
// ISADecoder -- the interface between the Core and any instruction set
// =========================================================================

// ISADecoder is the protocol that any instruction set architecture (ISA) must
// implement to plug into a Core.
//
// # Why an Interface?
//
// The Core knows how to move instructions through a pipeline, predict
// branches, detect hazards, and access caches. But it does NOT know what
// any instruction means. That is the ISA decoder's job.
//
// This separation mirrors real CPU design:
//   - ARM defines the decoder semantics (what ADD, LDR, BEQ mean)
//   - Apple/Qualcomm build the pipeline and caches
//   - The decoder plugs into the pipeline via a well-defined interface
//
// Our ISADecoder interface is that well-defined interface. Any ISA
// (ARM, RISC-V, x86, or a custom teaching ISA) can implement it and
// immediately run on any Core configuration.
//
// # The Two Methods
//
// The decoder has exactly two responsibilities:
//
//  1. Decode: turn raw instruction bits into a structured PipelineToken
//     (fill in opcode, registers, control signals, immediate value)
//
//  2. Execute: perform the actual computation (ALU operation, branch
//     resolution, effective address calculation)
//
// These map directly to the ID and EX stages of the pipeline:
//
//	IF stage:  fetch raw bits from memory
//	ID stage:  decoder.Decode(raw, token) -> fills in decoded fields
//	EX stage:  decoder.Execute(token, regFile) -> computes ALU result
//	MEM stage: core handles cache access
//	WB stage:  core handles register writeback
//
// # Example: ARM vs RISC-V
//
//	ARM decoder:
//	  Decode(0xE0821003, token) -> opcode="ADD", Rd=1, Rs1=2, Rs2=3
//	  Execute(token, regs) -> ALUResult = regs[2] + regs[3]
//
//	RISC-V decoder:
//	  Decode(0x003100B3, token) -> opcode="ADD", Rd=1, Rs1=2, Rs2=3
//	  Execute(token, regs) -> ALUResult = regs[2] + regs[3]
//
// Same semantics (ADD R1, R2, R3) but different binary encodings.
// The Core does not care -- it just calls Decode and Execute.
type ISADecoder interface {
	// Decode turns raw instruction bits into a structured PipelineToken.
	//
	// The decoder fills in:
	//   - Opcode (string name like "ADD", "LDR", "BEQ")
	//   - Rs1, Rs2 (source register numbers, -1 if unused)
	//   - Rd (destination register number, -1 if unused)
	//   - Immediate (sign-extended immediate value)
	//   - Control signals: RegWrite, MemRead, MemWrite, IsBranch, IsHalt
	//
	// The raw instruction is the 32-bit (or 16-bit) value fetched from memory.
	// The token is pre-allocated by the pipeline; the decoder fills in fields.
	Decode(rawInstruction int, token *cpupipeline.PipelineToken) *cpupipeline.PipelineToken

	// Execute performs the ALU operation for a decoded instruction.
	//
	// The executor fills in:
	//   - ALUResult (computed value, or effective address for loads/stores)
	//   - BranchTaken (was the branch actually taken?)
	//   - BranchTarget (where does the branch go?)
	//   - WriteData (final value to write to Rd, if RegWrite is true)
	//
	// The RegisterFile is passed so the executor can read source register
	// values. The executor does NOT write registers -- that is the WB stage's
	// job, handled by the Core.
	Execute(token *cpupipeline.PipelineToken, regFile *RegisterFile) *cpupipeline.PipelineToken

	// InstructionSize returns the size of one instruction in bytes.
	//
	// This determines how much the PC advances after each fetch:
	//   - ARM (A64): 4 bytes (fixed-width 32-bit instructions)
	//   - RISC-V:    4 bytes (base ISA) or 2 bytes (compressed)
	//   - x86:       variable (1-15 bytes)
	//   - Thumb:     2 bytes
	//
	// For our simple mock decoder, this is always 4.
	InstructionSize() int
}

// =========================================================================
// MockDecoder -- a simple decoder for testing the Core
// =========================================================================

// MockDecoder is a minimal ISA decoder for testing purposes.
//
// It supports a handful of instructions encoded in a simple format:
//
//	Bits 31-24: opcode (0=NOP, 1=ADD, 2=LOAD, 3=STORE, 4=BRANCH, 5=HALT,
//	                     6=ADDI, 7=SUB)
//	Bits 23-20: Rd  (destination register)
//	Bits 19-16: Rs1 (first source register)
//	Bits 15-12: Rs2 (second source register)
//	Bits 11-0:  immediate (12-bit, sign-extended)
//
// This encoding does not match any real ISA. It exists solely to exercise
// the Core's pipeline, hazard detection, branch prediction, and caches.
//
// # Instruction Reference
//
//	NOP    (0x00): Do nothing. Occupies a pipeline slot but has no effect.
//	ADD    (0x01): Rd = Rs1 + Rs2
//	LOAD   (0x02): Rd = Memory[Rs1 + imm]  (word load)
//	STORE  (0x03): Memory[Rs1 + imm] = Rs2  (word store)
//	BRANCH (0x04): If Rs1 == Rs2, PC = PC + imm (conditional branch)
//	HALT   (0x05): Stop execution.
//	ADDI   (0x06): Rd = Rs1 + imm
//	SUB    (0x07): Rd = Rs1 - Rs2
type MockDecoder struct{}

// NewMockDecoder creates a new MockDecoder.
func NewMockDecoder() *MockDecoder {
	return &MockDecoder{}
}

// InstructionSize returns 4 (all mock instructions are 32 bits).
func (d *MockDecoder) InstructionSize() int {
	return 4
}

// Decode extracts fields from a raw 32-bit instruction and fills in the token.
//
// Encoding layout:
//
//	  31      24 23    20 19    16 15    12 11           0
//	+----------+--------+--------+--------+--------------+
//	|  opcode  |   Rd   |  Rs1   |  Rs2   |  immediate   |
//	+----------+--------+--------+--------+--------------+
//
// The immediate is sign-extended from 12 bits to a full int.
func (d *MockDecoder) Decode(raw int, token *cpupipeline.PipelineToken) *cpupipeline.PipelineToken {
	// Extract fields using bit masking and shifting.
	opcode := (raw >> 24) & 0xFF
	rd := (raw >> 20) & 0x0F
	rs1 := (raw >> 16) & 0x0F
	rs2 := (raw >> 12) & 0x0F
	imm := raw & 0xFFF

	// Sign-extend the 12-bit immediate to a full int.
	// If bit 11 is set, the value is negative.
	if imm&0x800 != 0 {
		imm |= ^0xFFF // sign-extend by filling upper bits with 1s
	}

	// Fill in decoded fields based on opcode.
	switch opcode {
	case 0x00: // NOP
		token.Opcode = "NOP"
		token.Rd = -1
		token.Rs1 = -1
		token.Rs2 = -1

	case 0x01: // ADD Rd, Rs1, Rs2
		token.Opcode = "ADD"
		token.Rd = rd
		token.Rs1 = rs1
		token.Rs2 = rs2
		token.RegWrite = true

	case 0x02: // LOAD Rd, [Rs1 + imm]
		token.Opcode = "LOAD"
		token.Rd = rd
		token.Rs1 = rs1
		token.Rs2 = -1
		token.Immediate = imm
		token.RegWrite = true
		token.MemRead = true

	case 0x03: // STORE [Rs1 + imm], Rs2
		token.Opcode = "STORE"
		token.Rd = -1
		token.Rs1 = rs1
		token.Rs2 = rs2
		token.Immediate = imm
		token.MemWrite = true

	case 0x04: // BRANCH Rs1, Rs2, imm (branch if Rs1 == Rs2)
		token.Opcode = "BRANCH"
		token.Rd = -1
		token.Rs1 = rs1
		token.Rs2 = rs2
		token.Immediate = imm
		token.IsBranch = true

	case 0x05: // HALT
		token.Opcode = "HALT"
		token.Rd = -1
		token.Rs1 = -1
		token.Rs2 = -1
		token.IsHalt = true

	case 0x06: // ADDI Rd, Rs1, imm
		token.Opcode = "ADDI"
		token.Rd = rd
		token.Rs1 = rs1
		token.Rs2 = -1
		token.Immediate = imm
		token.RegWrite = true

	case 0x07: // SUB Rd, Rs1, Rs2
		token.Opcode = "SUB"
		token.Rd = rd
		token.Rs1 = rs1
		token.Rs2 = rs2
		token.RegWrite = true

	default: // Unknown opcode -- treat as NOP
		token.Opcode = "NOP"
		token.Rd = -1
		token.Rs1 = -1
		token.Rs2 = -1
	}

	return token
}

// Execute performs the ALU operation for a decoded instruction.
//
// This reads register values, computes the result, and fills in
// ALUResult, BranchTaken, BranchTarget, and WriteData.
func (d *MockDecoder) Execute(token *cpupipeline.PipelineToken, regFile *RegisterFile) *cpupipeline.PipelineToken {
	// Read source register values.
	var rs1Val, rs2Val int
	if token.Rs1 >= 0 {
		rs1Val = regFile.Read(token.Rs1)
	}
	if token.Rs2 >= 0 {
		rs2Val = regFile.Read(token.Rs2)
	}

	switch token.Opcode {
	case "ADD":
		// ALU result = Rs1 + Rs2
		token.ALUResult = rs1Val + rs2Val
		token.WriteData = token.ALUResult

	case "SUB":
		// ALU result = Rs1 - Rs2
		token.ALUResult = rs1Val - rs2Val
		token.WriteData = token.ALUResult

	case "ADDI":
		// ALU result = Rs1 + immediate
		token.ALUResult = rs1Val + token.Immediate
		token.WriteData = token.ALUResult

	case "LOAD":
		// Effective address = Rs1 + immediate
		// The actual memory read happens in the MEM stage (handled by Core).
		token.ALUResult = rs1Val + token.Immediate

	case "STORE":
		// Effective address = Rs1 + immediate
		// The data to store comes from Rs2.
		token.ALUResult = rs1Val + token.Immediate
		token.WriteData = rs2Val

	case "BRANCH":
		// Branch condition: Rs1 == Rs2
		// Branch target: PC + (immediate * instruction_size)
		taken := rs1Val == rs2Val
		token.BranchTaken = taken
		target := token.PC + (token.Immediate * 4)
		token.BranchTarget = target
		if taken {
			token.ALUResult = target
		} else {
			token.ALUResult = token.PC + 4
		}

	case "NOP", "HALT":
		// No computation needed.

	default:
		// Unknown opcode -- no computation.
	}

	return token
}

// =========================================================================
// MockInstruction -- helpers for encoding mock instructions
// =========================================================================

// EncodeNOP returns the raw encoding for a NOP instruction.
func EncodeNOP() int {
	return 0x00 << 24
}

// EncodeADD returns the raw encoding for ADD Rd, Rs1, Rs2.
func EncodeADD(rd, rs1, rs2 int) int {
	return (0x01 << 24) | (rd << 20) | (rs1 << 16) | (rs2 << 12)
}

// EncodeSUB returns the raw encoding for SUB Rd, Rs1, Rs2.
func EncodeSUB(rd, rs1, rs2 int) int {
	return (0x07 << 24) | (rd << 20) | (rs1 << 16) | (rs2 << 12)
}

// EncodeADDI returns the raw encoding for ADDI Rd, Rs1, imm.
func EncodeADDI(rd, rs1, imm int) int {
	return (0x06 << 24) | (rd << 20) | (rs1 << 16) | (imm & 0xFFF)
}

// EncodeLOAD returns the raw encoding for LOAD Rd, [Rs1 + imm].
func EncodeLOAD(rd, rs1, imm int) int {
	return (0x02 << 24) | (rd << 20) | (rs1 << 16) | (imm & 0xFFF)
}

// EncodeSTORE returns the raw encoding for STORE [Rs1 + imm], Rs2.
func EncodeSTORE(rs1, rs2, imm int) int {
	return (0x03 << 24) | (rs1 << 16) | (rs2 << 12) | (imm & 0xFFF)
}

// EncodeBRANCH returns the raw encoding for BRANCH Rs1, Rs2, imm.
// The branch is taken if Rs1 == Rs2, jumping to PC + imm*4.
func EncodeBRANCH(rs1, rs2, imm int) int {
	return (0x04 << 24) | (rs1 << 16) | (rs2 << 12) | (imm & 0xFFF)
}

// EncodeHALT returns the raw encoding for a HALT instruction.
func EncodeHALT() int {
	return 0x05 << 24
}
