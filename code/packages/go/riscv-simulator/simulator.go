// Package riscvsimulator implements the RISC-V RV32I base integer instruction set
// with M-mode privileged extensions for OS support.
//
// === What is RISC-V? ===
//
// RISC-V (pronounced "risk-five") is an open-source instruction set architecture (ISA).
// Unlike the highly complex x86 architecture (CISC), RISC-V is built on the philosophy
// of a Reduced Instruction Set Computer — the idea that a CPU should have a small number
// of simple instructions rather than many complex ones.
//
// === What this simulator supports ===
//
// The full RV32I base integer instruction set (37 instructions):
//   - Arithmetic: add, sub, addi, slt, sltu, slti, sltiu, and, or, xor, andi, ori, xori
//   - Shifts: sll, srl, sra, slli, srli, srai
//   - Loads: lb, lh, lw, lbu, lhu
//   - Stores: sb, sh, sw
//   - Branches: beq, bne, blt, bge, bltu, bgeu
//   - Jumps: jal, jalr
//   - Upper immediates: lui, auipc
//   - System: ecall
//
// Plus M-mode privileged extensions:
//   - CSR access: csrrw, csrrs, csrrc
//   - Trap return: mret
//   - CSR registers: mstatus, mtvec, mepc, mcause, mscratch
//
// === Register conventions ===
//
// RISC-V has 32 registers, each 32 bits wide. The most important quirk is:
//
//	x0  = always 0 (hardwired — writes are ignored, reads always return 0)
//
// Because x0 is always 0, it enables clever optimizations without dedicated instructions:
//
//	addi x1, x0, 42    →    x1 = 0 + 42 = 42 (effectively a "load 42 into x1" operation)
//
// === Architecture ===
//
// This simulator bridges the gap between binary encoded bits and the generic
// fetch-decode-execute cycle provided by the cpu-simulator package:
//
//	simulator.go  — top-level simulator struct and factory
//	opcodes.go    — opcode and funct3/funct7 constants
//	decode.go     — instruction decoder (binary → structured fields)
//	execute.go    — instruction executor (structured fields → state changes)
//	csr.go        — Control and Status Register file for M-mode
//	encoding.go   — helpers to construct machine code for testing
package riscvsimulator

import (
	cpu "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-simulator"
)

// RiscVDecoder translates binary bits into human-readable instruction fields.
// It implements the cpu.InstructionDecoder interface.
//
// The Decode method (defined in decode.go) examines the opcode bits and
// delegates to format-specific decoders for R-type, I-type, S-type,
// B-type, U-type, and J-type instructions.
type RiscVDecoder struct{}

// RiscVExecutor applies decoded operations onto the CPU's registers and memory.
// It implements the cpu.InstructionExecutor interface.
//
// The Execute method (defined in execute.go) dispatches to instruction-specific
// handlers. The CSR field provides access to M-mode Control and Status Registers
// for privileged operations (ecall trap handling, mret, CSR read/write).
type RiscVExecutor struct {
	CSR  *CSRFile
	Host *HostIO
}

// RiscVSimulator encompasses the full RISC-V environment: decoder, executor,
// CSR file, and the generic CPU pipeline from cpu-simulator.
type RiscVSimulator struct {
	Decoder  *RiscVDecoder
	Executor *RiscVExecutor
	CPU      *cpu.CPU
	CSR      *CSRFile
	Host     *HostIO
}

// NewRiscVSimulator creates a fully initialized RISC-V simulator with the
// specified memory size (in bytes). The simulator includes:
//   - 32 general-purpose registers (x0-x31), each 32 bits wide
//   - A CSR file for M-mode privileged registers
//   - The specified amount of byte-addressable memory
//
// Memory size should be large enough to hold both the program and any
// data it accesses. 65536 (64 KiB) is a good default for testing.
func NewRiscVSimulator(memorySize int) *RiscVSimulator {
	return NewRiscVSimulatorWithHost(memorySize, nil)
}

// NewRiscVSimulatorWithHost creates a simulator with optional host syscall I/O.
//
// When host is nil, ecall keeps the legacy no-trap-handler behavior and halts.
// When host is present and mtvec is unset, syscall numbers in x17 are handled
// directly for simple language runtime tests.
func NewRiscVSimulatorWithHost(memorySize int, host *HostIO) *RiscVSimulator {
	result, _ := StartNew[*RiscVSimulator]("riscv-simulator.NewRiscVSimulator", nil,
		func(op *Operation[*RiscVSimulator], rf *ResultFactory[*RiscVSimulator]) *OperationResult[*RiscVSimulator] {
			decoder := &RiscVDecoder{}
			csrFile := NewCSRFile()
			executor := &RiscVExecutor{CSR: csrFile, Host: host}
			return rf.Generate(true, false, &RiscVSimulator{
				Decoder:  decoder,
				Executor: executor,
				CPU:      cpu.NewCPU(decoder, executor, 32, 32, memorySize),
				CSR:      csrFile,
				Host:     host,
			})
		}).GetResult()
	return result
}

// Run loads a program (as raw bytes) into memory starting at address 0,
// then executes instructions until the CPU halts or the 10,000-step
// safety limit is reached (preventing infinite loops in tests).
func (s *RiscVSimulator) Run(program []byte) []cpu.PipelineTrace {
	result, _ := StartNew[[]cpu.PipelineTrace]("riscv-simulator.RiscVSimulator.Run", nil,
		func(op *Operation[[]cpu.PipelineTrace], rf *ResultFactory[[]cpu.PipelineTrace]) *OperationResult[[]cpu.PipelineTrace] {
			s.CPU.LoadProgram(program, 0)
			return rf.Generate(true, false, s.CPU.Run(10000))
		}).GetResult()
	return result
}

// Step advances the pipeline by a single instruction, returning a trace
// of what happened during fetch, decode, and execute.
func (s *RiscVSimulator) Step() cpu.PipelineTrace {
	result, _ := StartNew[cpu.PipelineTrace]("riscv-simulator.RiscVSimulator.Step", cpu.PipelineTrace{},
		func(op *Operation[cpu.PipelineTrace], rf *ResultFactory[cpu.PipelineTrace]) *OperationResult[cpu.PipelineTrace] {
			return rf.Generate(true, false, s.CPU.Step())
		}).GetResult()
	return result
}
