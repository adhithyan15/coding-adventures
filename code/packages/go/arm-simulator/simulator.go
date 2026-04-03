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
//
// # Operations
//
// Every public method is wrapped in an Operation, giving each call
// automatic timing, structured logging, and panic recovery.
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
	result, _ := StartNew[cpu.DecodeResult]("arm-simulator.Decode", cpu.DecodeResult{},
		func(op *Operation[cpu.DecodeResult], rf *ResultFactory[cpu.DecodeResult]) *OperationResult[cpu.DecodeResult] {
			op.AddProperty("raw", raw)
			op.AddProperty("pc", pc)
			if raw == HltInstruction {
				return rf.Generate(true, false, cpu.DecodeResult{
					Mnemonic:       "hlt",
					Fields:         map[string]int{},
					RawInstruction: raw,
				})
			}
			return rf.Generate(true, false, d.decodeDataProcessing(raw))
		}).GetResult()
	return result
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
	result, _ := StartNew[cpu.ExecuteResult]("arm-simulator.Execute", cpu.ExecuteResult{},
		func(op *Operation[cpu.ExecuteResult], rf *ResultFactory[cpu.ExecuteResult]) *OperationResult[cpu.ExecuteResult] {
			op.AddProperty("mnemonic", decoded.Mnemonic)
			op.AddProperty("pc", pc)
			mnemonic := decoded.Mnemonic
			if mnemonic == "mov" {
				return rf.Generate(true, false, e.execMov(decoded, registers, pc))
			} else if mnemonic == "add" {
				return rf.Generate(true, false, e.execAdd(decoded, registers, pc))
			} else if mnemonic == "sub" {
				return rf.Generate(true, false, e.execSub(decoded, registers, pc))
			} else if mnemonic == "hlt" {
				return rf.Generate(true, false, cpu.ExecuteResult{
					Description:      "Halt",
					RegistersChanged: map[string]uint32{},
					MemoryChanged:    map[int]byte{},
					NextPC:           pc,
					Halted:           true,
				})
			}
			return rf.Generate(true, false, cpu.ExecuteResult{
				Description:      fmt.Sprintf("Unknown instruction: %s", mnemonic),
				RegistersChanged: map[string]uint32{},
				MemoryChanged:    map[int]byte{},
				NextPC:           pc + 4,
			})
		}).GetResult()
	return result
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
	result, _ := StartNew[*ARMSimulator]("arm-simulator.NewARMSimulator", nil,
		func(op *Operation[*ARMSimulator], rf *ResultFactory[*ARMSimulator]) *OperationResult[*ARMSimulator] {
			op.AddProperty("memorySize", memorySize)
			decoder := &ARMDecoder{}
			executor := &ARMExecutor{}
			sim := &ARMSimulator{
				Decoder:  decoder,
				Executor: executor,
				CPU:      cpu.NewCPU(decoder, executor, 16, 32, memorySize),
			}
			return rf.Generate(true, false, sim)
		}).GetResult()
	return result
}

// Run executes instructions indefinitely until HLT is encountered or 10,000 steps are consumed.
func (s *ARMSimulator) Run(program []byte) []cpu.PipelineTrace {
	result, _ := StartNew[[]cpu.PipelineTrace]("arm-simulator.Run", nil,
		func(op *Operation[[]cpu.PipelineTrace], rf *ResultFactory[[]cpu.PipelineTrace]) *OperationResult[[]cpu.PipelineTrace] {
			s.CPU.LoadProgram(program, 0)
			return rf.Generate(true, false, s.CPU.Run(10000))
		}).GetResult()
	return result
}

// Step evaluates a single fetching block into trace format. Left mostly for unit testing validation.
func (s *ARMSimulator) Step() cpu.PipelineTrace {
	result, _ := StartNew[cpu.PipelineTrace]("arm-simulator.Step", cpu.PipelineTrace{},
		func(op *Operation[cpu.PipelineTrace], rf *ResultFactory[cpu.PipelineTrace]) *OperationResult[cpu.PipelineTrace] {
			return rf.Generate(true, false, s.CPU.Step())
		}).GetResult()
	return result
}

// Assembly Encoding abstractions

// EncodeMovImm encodes a MOV immediate instruction in ARM format.
func EncodeMovImm(rd, imm int) uint32 {
	result, _ := StartNew[uint32]("arm-simulator.EncodeMovImm", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			op.AddProperty("rd", rd)
			op.AddProperty("imm", imm)
			cond := uint32(CondAL)
			iBit := uint32(1)
			opcode := uint32(OpcodeMov)
			sBit := uint32(0)
			rn := uint32(0)
			imm8 := uint32(imm & 0xFF)
			rotate := uint32(0)

			return rf.Generate(true, false, (cond<<28)|(0b00<<26)|(iBit<<25)|(opcode<<21)|
				(sBit<<20)|(rn<<16)|uint32(rd<<12)|(rotate<<8)|imm8)
		}).GetResult()
	return result
}

// EncodeAdd encodes an ADD register instruction in ARM format.
func EncodeAdd(rd, rn, rm int) uint32 {
	result, _ := StartNew[uint32]("arm-simulator.EncodeAdd", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			op.AddProperty("rd", rd)
			op.AddProperty("rn", rn)
			op.AddProperty("rm", rm)
			return rf.Generate(true, false, (CondAL<<28)|(0b00<<26)|(0<<25)|(OpcodeAdd<<21)|
				(0<<20)|uint32(rn<<16)|uint32(rd<<12)|uint32(rm))
		}).GetResult()
	return result
}

// EncodeSub encodes a SUB register instruction in ARM format.
func EncodeSub(rd, rn, rm int) uint32 {
	result, _ := StartNew[uint32]("arm-simulator.EncodeSub", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			op.AddProperty("rd", rd)
			op.AddProperty("rn", rn)
			op.AddProperty("rm", rm)
			return rf.Generate(true, false, (CondAL<<28)|(0b00<<26)|(0<<25)|(OpcodeSub<<21)|
				(0<<20)|uint32(rn<<16)|uint32(rd<<12)|uint32(rm))
		}).GetResult()
	return result
}

// EncodeHlt encodes the halt instruction (0xFFFFFFFF).
func EncodeHlt() uint32 {
	result, _ := StartNew[uint32]("arm-simulator.EncodeHlt", 0,
		func(_ *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, HltInstruction)
		}).GetResult()
	return result
}

// Assemble converts 32-bit raw arrays directly to Little-Endian RAM bytes explicitly.
func Assemble(instructions []uint32) []byte {
	result, _ := StartNew[[]byte]("arm-simulator.Assemble", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			res := make([]byte, 0, len(instructions)*4)
			for _, inst := range instructions {
				res = append(res, byte(inst&0xFF), byte((inst>>8)&0xFF), byte((inst>>16)&0xFF), byte((inst>>24)&0xFF))
			}
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}
