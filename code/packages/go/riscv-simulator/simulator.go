// Package riscvsimulator implements a clean, modern instruction set.
//
// === What is RISC-V? ===
//
// RISC-V (pronounced "risk-five") is an open-source instruction set architecture (ISA). 
// Unlike the highly complex x86 architecture (CISC), RISC-V is built on the philosophy
// of a Reduced Instruction Set Computer — the idea that a CPU should have a small number
// of simple instructions rather than many complex ones.
//
// === Register conventions ===
//
// RISC-V has 32 registers, each 32 bits wide. The most important quirk is:
//     x0  = always 0 (hardwired — writes are ignored, reads always return 0)
//
// Because x0 is always 0, it enables clever optimizations without dedicated instructions:
//     addi x1, x0, 42    →    x1 = 0 + 42 = 42 (effectively a "load 42 into x1" operation)
//
// This simulator bridges the gap between binary encoded bits and the generic
// fetch-decode-execute cycle of our CPU base.
package riscvsimulator

import (
	"fmt"

	cpu "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-simulator"
)

// Constants for RISC-V RV32I opcodes (always found in the lower 7 bits: [6:0]).
const (
	OpcodeOpImm  = 0b0010011 // I-type arithmetic with immediate (e.g. addi)
	OpcodeOp     = 0b0110011 // R-type arithmetic (e.g. add, sub)
	OpcodeSystem = 0b1110011 // System instructions (e.g. ecall)
)

// RiscVDecoder translates binary bits into human readable instruction fields.
type RiscVDecoder struct{}

// Decode determines the instruction type by looking at the opcode bits.
func (d *RiscVDecoder) Decode(raw uint32, pc int) cpu.DecodeResult {
	opcode := raw & 0x7F
	if opcode == OpcodeOpImm {
		return d.decodeIType(raw, "addi")
	} else if opcode == OpcodeOp {
		return d.decodeRType(raw)
	} else if opcode == OpcodeSystem {
		return cpu.DecodeResult{
			Mnemonic:       "ecall",
			Fields:         map[string]int{"opcode": int(opcode)},
			RawInstruction: raw,
		}
	}
	return cpu.DecodeResult{
		Mnemonic:       fmt.Sprintf("UNKNOWN(0x%02x)", opcode),
		Fields:         map[string]int{"opcode": int(opcode)},
		RawInstruction: raw,
	}
}

// decodeRType extracts fields for Register-to-Register operations.
// R-type format: [funct7 | rs2 | rs1 | funct3 | rd | opcode]
func (d *RiscVDecoder) decodeRType(raw uint32) cpu.DecodeResult {
	rd := int((raw >> 7) & 0x1F)
	funct3 := int((raw >> 12) & 0x7)
	rs1 := int((raw >> 15) & 0x1F)
	rs2 := int((raw >> 20) & 0x1F)
	funct7 := int((raw >> 25) & 0x7F)

	var mnemonic string
	if funct3 == 0 && funct7 == 0 {
		mnemonic = "add"
	} else if funct3 == 0 && funct7 == 0x20 {
		mnemonic = "sub"
	} else {
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

// decodeIType extracts fields for operations executing with an Immediate value.
// I-type format: [imm[11:0] | rs1 | funct3 | rd | opcode]
func (d *RiscVDecoder) decodeIType(raw uint32, defaultMnemonic string) cpu.DecodeResult {
	rd := int((raw >> 7) & 0x1F)
	funct3 := int((raw >> 12) & 0x7)
	rs1 := int((raw >> 15) & 0x1F)
	imm := int((raw >> 20) & 0xFFF)

	// Since immediate values can be negative, we must "sign-extend" the 12-bit
	// value to represent its full 32-bit counterpart. 
	// The MSB of a 12-bit string is bit 11 (0x800).
	if imm&0x800 != 0 {
		imm -= 0x1000
	}

	return cpu.DecodeResult{
		Mnemonic: defaultMnemonic,
		Fields: map[string]int{
			"rd":     rd,
			"rs1":    rs1,
			"imm":    imm,
			"funct3": funct3,
		},
		RawInstruction: raw,
	}
}

// RiscVExecutor applies the decoded operations onto the CPU memory and registers.
type RiscVExecutor struct{}

// Execute accepts the instruction struct and performs math or memory writes.
func (e *RiscVExecutor) Execute(decoded cpu.DecodeResult, registers *cpu.RegisterFile, memory *cpu.Memory, pc int) cpu.ExecuteResult {
	mnemonic := decoded.Mnemonic
	switch mnemonic {
	case "addi":
		return e.execAddi(decoded, registers, pc)
	case "add":
		return e.execAdd(decoded, registers, pc)
	case "sub":
		return e.execSub(decoded, registers, pc)
	case "ecall":
		// 'ecall' halts our simple CPU
		return cpu.ExecuteResult{
			Description:      "System call (halt)",
			RegistersChanged: map[string]uint32{},
			MemoryChanged:    map[int]byte{},
			NextPC:           pc,
			Halted:           true,
		}
	default:
		return cpu.ExecuteResult{
			Description:      fmt.Sprintf("Unknown instruction: %s", mnemonic),
			RegistersChanged: map[string]uint32{},
			MemoryChanged:    map[int]byte{},
			NextPC:           pc + 4,
		}
	}
}

func (e *RiscVExecutor) execAddi(decoded cpu.DecodeResult, registers *cpu.RegisterFile, pc int) cpu.ExecuteResult {
	rd := decoded.Fields["rd"]
	rs1 := decoded.Fields["rs1"]
	imm := decoded.Fields["imm"]

	rs1Val := int32(registers.Read(rs1))
	result := uint32(rs1Val + int32(imm))

	changes := map[string]uint32{}
	if rd != 0 {
		registers.Write(rd, result)
		changes[fmt.Sprintf("x%d", rd)] = result
	}

	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("x%d = x%d(%d) + %d = %d", rd, rs1, rs1Val, imm, int32(result)),
		RegistersChanged: changes,
		MemoryChanged:    map[int]byte{},
		NextPC:           pc + 4,
	}
}

func (e *RiscVExecutor) execAdd(decoded cpu.DecodeResult, registers *cpu.RegisterFile, pc int) cpu.ExecuteResult {
	rd := decoded.Fields["rd"]
	rs1 := decoded.Fields["rs1"]
	rs2 := decoded.Fields["rs2"]

	rs1Val := int32(registers.Read(rs1))
	rs2Val := int32(registers.Read(rs2))
	result := uint32(rs1Val + rs2Val)

	changes := map[string]uint32{}
	// x0 is strictly hardwired to zero, intercept edits here
	if rd != 0 {
		registers.Write(rd, result)
		changes[fmt.Sprintf("x%d", rd)] = result
	}

	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("x%d = x%d(%d) + x%d(%d) = %d", rd, rs1, rs1Val, rs2, rs2Val, int32(result)),
		RegistersChanged: changes,
		MemoryChanged:    map[int]byte{},
		NextPC:           pc + 4,
	}
}

func (e *RiscVExecutor) execSub(decoded cpu.DecodeResult, registers *cpu.RegisterFile, pc int) cpu.ExecuteResult {
	rd := decoded.Fields["rd"]
	rs1 := decoded.Fields["rs1"]
	rs2 := decoded.Fields["rs2"]

	rs1Val := int32(registers.Read(rs1))
	rs2Val := int32(registers.Read(rs2))
	result := uint32(rs1Val - rs2Val)

	changes := map[string]uint32{}
	if rd != 0 {
		registers.Write(rd, result)
		changes[fmt.Sprintf("x%d", rd)] = result
	}

	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("x%d = x%d(%d) - x%d(%d) = %d", rd, rs1, rs1Val, rs2, rs2Val, int32(result)),
		RegistersChanged: changes,
		MemoryChanged:    map[int]byte{},
		NextPC:           pc + 4,
	}
}

// RiscVSimulator encompasses the full RISC-V environment loop setup.
type RiscVSimulator struct {
	Decoder  *RiscVDecoder
	Executor *RiscVExecutor
	CPU      *cpu.CPU
}

// NewRiscVSimulator sets up the instruction parsers and limits to 32 bits and 32 registers.
func NewRiscVSimulator(memorySize int) *RiscVSimulator {
	decoder := &RiscVDecoder{}
	executor := &RiscVExecutor{}
	return &RiscVSimulator{
		Decoder:  decoder,
		Executor: executor,
		CPU:      cpu.NewCPU(decoder, executor, 32, 32, memorySize),
	}
}

// Run executes the program bytes to completion.
func (s *RiscVSimulator) Run(program []byte) []cpu.PipelineTrace {
	s.CPU.LoadProgram(program, 0)
	return s.CPU.Run(10000)
}

// Step advances the pipeline by a single transaction.
func (s *RiscVSimulator) Step() cpu.PipelineTrace {
	return s.CPU.Step()
}

// Encoding Helpers for creating machine-code natively for testing

func EncodeAddi(rd, rs1, imm int) uint32 {
	immBits := imm & 0xFFF
	return uint32((immBits << 20) | (rs1 << 15) | (0 << 12) | (rd << 7) | OpcodeOpImm)
}

func EncodeAdd(rd, rs1, rs2 int) uint32 {
	return uint32((0 << 25) | (rs2 << 20) | (rs1 << 15) | (0 << 12) | (rd << 7) | OpcodeOp)
}

func EncodeSub(rd, rs1, rs2 int) uint32 {
	return uint32((0x20 << 25) | (rs2 << 20) | (rs1 << 15) | (0 << 12) | (rd << 7) | OpcodeOp)
}

func EncodeEcall() uint32 {
	return uint32(OpcodeSystem)
}

// Assemble reduces a list of abstract 32-bit instructions to physical Memory-ready Little-Endian structures.
func Assemble(instructions []uint32) []byte {
	result := make([]byte, 0, len(instructions)*4)
	for _, inst := range instructions {
		result = append(result, byte(inst&0xFF), byte((inst>>8)&0xFF), byte((inst>>16)&0xFF), byte((inst>>24)&0xFF))
	}
	return result
}
