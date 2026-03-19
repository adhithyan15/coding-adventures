// Package armsimulator encapsulates the historical and pervasive ARM Architecture.
//
// === What is ARM? ===
//
// ARM (originally Acorn RISC Machine) powers your phone, your tablet, and probably
// your laptop. It was designed in 1985 with a focus on power efficiency.
//
// Unlike RISC-V's strict, zero-magic layout, ARM features several unique design quirks.
// Most notably: conditionally executed instructions. Every instruction possesses a
// 4-bit cond-code. Thus you can execute an ADD exclusively if the prior CMP equaled Zero.
//
// === Register conventions ===
//
// ARM has 16 registers, wide 32-bits each:
//     R15 = PC (Program Counter is fully visible to assembly writers!)
//     R14 = LR (Link Register - Return address)
//     R13 = SP (Stack Pointer)
package armsimulator

import (
	"fmt"

	cpu "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-simulator"
)

const (
	CondAL = 0b1110 // Condition code for Execute ALWAYS

	OpcodeMov = 0b1101
	OpcodeAdd = 0b0100
	OpcodeSub = 0b0010

	HltInstruction = 0xFFFFFFFF
)

// ARMDecoder parses 32-bit ARM data processing instructions.
type ARMDecoder struct{}

// Decode determines instruction mappings based on condition fields and opcode splits.
func (d *ARMDecoder) Decode(raw uint32, pc int) cpu.DecodeResult {
	if raw == HltInstruction {
		return cpu.DecodeResult{
			Mnemonic:       "hlt",
			Fields:         map[string]int{},
			RawInstruction: raw,
		}
	}
	return d.decodeDataProcessing(raw)
}

func (d *ARMDecoder) decodeDataProcessing(raw uint32) cpu.DecodeResult {
	cond := int((raw >> 28) & 0xF)
	iBit := int((raw >> 25) & 0x1)
	opcode := int((raw >> 21) & 0xF)
	sBit := int((raw >> 20) & 0x1)
	rn := int((raw >> 16) & 0xF)
	rd := int((raw >> 12) & 0xF)
	operand2 := int(raw & 0xFFF)

	var mnemonic string
	if opcode == OpcodeMov {
		mnemonic = "mov"
	} else if opcode == OpcodeAdd {
		mnemonic = "add"
	} else if opcode == OpcodeSub {
		mnemonic = "sub"
	} else {
		mnemonic = fmt.Sprintf("dp_op(%04b)", opcode)
	}

	fields := map[string]int{
		"cond":   cond,
		"i_bit":  iBit,
		"opcode": opcode,
		"s_bit":  sBit,
		"rn":     rn,
		"rd":     rd,
	}

	if iBit == 1 {
		// ARM incorporates a fascinating bit-saving trick.
		// Since we only have 12 bits for Immediate value in the instruction layout,
		// it splits this into an 8-bit Value (imm8) and a 4-bit Rotational multiplier.
		rotate := (operand2 >> 8) & 0xF
		imm8 := operand2 & 0xFF
		shift := rotate * 2
		
		var immValue int
		if shift > 0 {
			// Circular right shift (ROR)
			immValue = int((uint32(imm8) >> shift) | (uint32(imm8) << (32 - shift)))
		} else {
			immValue = imm8
		}
		fields["imm"] = immValue
	} else {
		rm := operand2 & 0xF
		fields["rm"] = rm
	}

	return cpu.DecodeResult{
		Mnemonic:       mnemonic,
		Fields:         fields,
		RawInstruction: raw,
	}
}

// ARMExecutor modifies the CPU state referencing ARM decoding.
type ARMExecutor struct{}

// Execute implements processor mutations aligned with the ARM execution standard.
func (e *ARMExecutor) Execute(decoded cpu.DecodeResult, registers *cpu.RegisterFile, memory *cpu.Memory, pc int) cpu.ExecuteResult {
	mnemonic := decoded.Mnemonic
	if mnemonic == "mov" {
		return e.execMov(decoded, registers, pc)
	} else if mnemonic == "add" {
		return e.execAdd(decoded, registers, pc)
	} else if mnemonic == "sub" {
		return e.execSub(decoded, registers, pc)
	} else if mnemonic == "hlt" {
		return cpu.ExecuteResult{
			Description:      "Halt",
			RegistersChanged: map[string]uint32{},
			MemoryChanged:    map[int]byte{},
			NextPC:           pc,
			Halted:           true,
		}
	}
	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("Unknown instruction: %s", mnemonic),
		RegistersChanged: map[string]uint32{},
		MemoryChanged:    map[int]byte{},
		NextPC:           pc + 4,
	}
}

func (e *ARMExecutor) execMov(decoded cpu.DecodeResult, registers *cpu.RegisterFile, pc int) cpu.ExecuteResult {
	rd := decoded.Fields["rd"]
	imm := decoded.Fields["imm"]
	
	result := uint32(imm)
	registers.Write(rd, result)
	
	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("R%d = %d", rd, result),
		RegistersChanged: map[string]uint32{fmt.Sprintf("R%d", rd): result},
		MemoryChanged:    map[int]byte{},
		NextPC:           pc + 4,
	}
}

func (e *ARMExecutor) execAdd(decoded cpu.DecodeResult, registers *cpu.RegisterFile, pc int) cpu.ExecuteResult {
	rd := decoded.Fields["rd"]
	rn := decoded.Fields["rn"]
	rm := decoded.Fields["rm"]

	rnVal := registers.Read(rn)
	rmVal := registers.Read(rm)
	result := rnVal + rmVal

	registers.Write(rd, result)

	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("R%d = R%d(%d) + R%d(%d) = %d", rd, rn, rnVal, rm, rmVal, result),
		RegistersChanged: map[string]uint32{fmt.Sprintf("R%d", rd): result},
		MemoryChanged:    map[int]byte{},
		NextPC:           pc + 4,
	}
}

func (e *ARMExecutor) execSub(decoded cpu.DecodeResult, registers *cpu.RegisterFile, pc int) cpu.ExecuteResult {
	rd := decoded.Fields["rd"]
	rn := decoded.Fields["rn"]
	rm := decoded.Fields["rm"]

	rnVal := registers.Read(rn)
	rmVal := registers.Read(rm)
	result := rnVal - rmVal

	registers.Write(rd, result)

	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("R%d = R%d(%d) - R%d(%d) = %d", rd, rn, rnVal, rm, rmVal, result),
		RegistersChanged: map[string]uint32{fmt.Sprintf("R%d", rd): result},
		MemoryChanged:    map[int]byte{},
		NextPC:           pc + 4,
	}
}

// ARMSimulator packages the overarching platform environment bounding the decoding and generic layers.
type ARMSimulator struct {
	Decoder  *ARMDecoder
	Executor *ARMExecutor
	CPU      *cpu.CPU
}

// NewARMSimulator restricts CPU allocation inherently to ARM's 16 general registers format.
func NewARMSimulator(memorySize int) *ARMSimulator {
	decoder := &ARMDecoder{}
	executor := &ARMExecutor{}
	return &ARMSimulator{
		Decoder:  decoder,
		Executor: executor,
		CPU:      cpu.NewCPU(decoder, executor, 16, 32, memorySize),
	}
}

// Run executes instructions indefinitely until HLT is encountered or 10,000 steps are consumed.
func (s *ARMSimulator) Run(program []byte) []cpu.PipelineTrace {
	s.CPU.LoadProgram(program, 0)
	return s.CPU.Run(10000)
}

// Step evaluates a single fetching block into trace format. Left mostly for unit testing validation.
func (s *ARMSimulator) Step() cpu.PipelineTrace {
	return s.CPU.Step()
}

// Assembly Encoding abstractions

func EncodeMovImm(rd, imm int) uint32 {
	cond := uint32(CondAL)
	iBit := uint32(1)
	opcode := uint32(OpcodeMov)
	sBit := uint32(0)
	rn := uint32(0)
	imm8 := uint32(imm & 0xFF)
	rotate := uint32(0)
	
	return (cond << 28) | (0b00 << 26) | (iBit << 25) | (opcode << 21) |
		(sBit << 20) | (rn << 16) | uint32(rd<<12) | (rotate << 8) | imm8
}

func EncodeAdd(rd, rn, rm int) uint32 {
	return (CondAL << 28) | (0b00 << 26) | (0 << 25) | (OpcodeAdd << 21) |
		(0 << 20) | uint32(rn<<16) | uint32(rd<<12) | uint32(rm)
}

func EncodeSub(rd, rn, rm int) uint32 {
	return (CondAL << 28) | (0b00 << 26) | (0 << 25) | (OpcodeSub << 21) |
		(0 << 20) | uint32(rn<<16) | uint32(rd<<12) | uint32(rm)
}

func EncodeHlt() uint32 {
	return HltInstruction
}

// Assemble converts 32-bit raw arrays directly to Little-Endian RAM bytes explicitly.
func Assemble(instructions []uint32) []byte {
	result := make([]byte, 0, len(instructions)*4)
	for _, inst := range instructions {
		result = append(result, byte(inst&0xFF), byte((inst>>8)&0xFF), byte((inst>>16)&0xFF), byte((inst>>24)&0xFF))
	}
	return result
}
