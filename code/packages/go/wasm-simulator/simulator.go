// Package wasmsimulator implements a minimalistic stack-based virtual machine compliant with WebAssembly principles.
//
// === Stack machines vs register machines ===
//
// Previous simulators (RISC-V, ARM) are *register machines*: instructions name specific
// registers as explicit operands (e.g., "add R2, R1, R0" — read R1 and R0, write R2).
//
// WASM is a *stack machine*: instructions don't name their operands. Instead,
// operands live on an implicit *operand stack*. Push values onto the stack,
// then invoke an operation — it autonomously pops its inputs and pushes the evaluated result.
package wasmsimulator

import (
	"encoding/binary"
	"fmt"
)

// Standard WASM Instruction Bytecodes
const (
	OpEnd       = 0x0B // End of block / function / halt
	OpLocalGet  = 0x20 // Push a local variable onto the stack
	OpLocalSet  = 0x21 // Pop the stack into a local variable
	OpI32Const  = 0x41 // Push a 32-bit explicit integer constant
	OpI32Add    = 0x6A // Pop two i32s, execute Add, push their sum
	OpI32Sub    = 0x6B // Pop two i32s, execute Sub, push their difference
)

// WasmInstruction characterizes a dynamically parsed variable-length instruction block.
type WasmInstruction struct {
	Opcode   byte
	Mnemonic string
	Operand  *int // Used optionally dependent on if instruction consumes a parameter.
	Size     int  // Number of bytes the cursor should sweep forward.
}

// WasmDecoder is dedicated towards abstracting the variable lengths of raw bytecode arrays.
type WasmDecoder struct{}

func (d *WasmDecoder) Decode(bytecode []byte, pc int) WasmInstruction {
	opcode := bytecode[pc]

	switch opcode {
	case OpI32Const:
		valBytes := bytecode[pc+1 : pc+5]
		val := int(int32(binary.LittleEndian.Uint32(valBytes)))
		return WasmInstruction{Opcode: opcode, Mnemonic: "i32.const", Operand: &val, Size: 5}
	case OpI32Add:
		return WasmInstruction{Opcode: opcode, Mnemonic: "i32.add", Size: 1}
	case OpI32Sub:
		return WasmInstruction{Opcode: opcode, Mnemonic: "i32.sub", Size: 1}
	case OpLocalGet:
		val := int(bytecode[pc+1])
		return WasmInstruction{Opcode: opcode, Mnemonic: "local.get", Operand: &val, Size: 2}
	case OpLocalSet:
		val := int(bytecode[pc+1])
		return WasmInstruction{Opcode: opcode, Mnemonic: "local.set", Operand: &val, Size: 2}
	case OpEnd:
		return WasmInstruction{Opcode: opcode, Mnemonic: "end", Size: 1}
	default:
		panic(fmt.Sprintf("Unknown WASM opcode: 0x%02X at PC=%d", opcode, pc))
	}
}

// WasmStepTrace outlines an exact historical footprint mapping the Stack logic sequentially.
type WasmStepTrace struct {
	PC             int
	Instruction    WasmInstruction
	StackBefore    []int
	StackAfter     []int
	LocalsSnapshot []int
	Description    string
	Halted         bool
}

// WasmExecutor implements mutation mechanics against raw Stack arrays strictly linearly.
type WasmExecutor struct{}

func (e *WasmExecutor) Execute(instruction WasmInstruction, stack *[]int, locals []int, pc int) WasmStepTrace {
	stackBefore := make([]int, len(*stack))
	copy(stackBefore, *stack)

	mnemonic := instruction.Mnemonic
	switch mnemonic {
	case "i32.const":
		val := *instruction.Operand
		*stack = append(*stack, val)
		return WasmStepTrace{PC: pc, Instruction: instruction, StackBefore: stackBefore, StackAfter: copyStack(*stack), LocalsSnapshot: copyStack(locals), Description: fmt.Sprintf("push %d", val)}
	case "i32.add":
		b := (*stack)[len(*stack)-1]
		a := (*stack)[len(*stack)-2]
		*stack = (*stack)[:len(*stack)-2]
		
		res := int(uint32(a + b)) // Masking to simulate 32-bit truncation universally
		*stack = append(*stack, res)
		return WasmStepTrace{PC: pc, Instruction: instruction, StackBefore: stackBefore, StackAfter: copyStack(*stack), LocalsSnapshot: copyStack(locals), Description: fmt.Sprintf("pop %d and %d, push %d", b, a, res)}
	case "i32.sub":
		b := (*stack)[len(*stack)-1]
		a := (*stack)[len(*stack)-2]
		*stack = (*stack)[:len(*stack)-2]
		
		res := int(uint32(a - b))
		*stack = append(*stack, res)
		return WasmStepTrace{PC: pc, Instruction: instruction, StackBefore: stackBefore, StackAfter: copyStack(*stack), LocalsSnapshot: copyStack(locals), Description: fmt.Sprintf("pop %d and %d, push %d", b, a, res)}
	case "local.get":
		idx := *instruction.Operand
		val := locals[idx]
		*stack = append(*stack, val)
		return WasmStepTrace{PC: pc, Instruction: instruction, StackBefore: stackBefore, StackAfter: copyStack(*stack), LocalsSnapshot: copyStack(locals), Description: fmt.Sprintf("push locals[%d] = %d", idx, val)}
	case "local.set":
		idx := *instruction.Operand
		val := (*stack)[len(*stack)-1]
		*stack = (*stack)[:len(*stack)-1]
		locals[idx] = val
		return WasmStepTrace{PC: pc, Instruction: instruction, StackBefore: stackBefore, StackAfter: copyStack(*stack), LocalsSnapshot: copyStack(locals), Description: fmt.Sprintf("pop %d, store in locals[%d]", val, idx)}
	case "end":
		return WasmStepTrace{PC: pc, Instruction: instruction, StackBefore: stackBefore, StackAfter: copyStack(*stack), LocalsSnapshot: copyStack(locals), Description: "halt", Halted: true}
	default:
		panic(fmt.Sprintf("Cannot execute: %s", mnemonic))
	}
}

func copyStack(s []int) []int {
	c := make([]int, len(s))
	copy(c, s)
	return c
}

// WasmSimulator manages native program allocations independently differing from generic CPU layers.
type WasmSimulator struct {
	Stack    []int
	Locals   []int
	PC       int
	Bytecode []byte
	Halted   bool
	Cycle    int
	decoder  *WasmDecoder
	executor *WasmExecutor
}

func NewWasmSimulator(numLocals int) *WasmSimulator {
	return &WasmSimulator{
		Stack:    []int{},
		Locals:   make([]int, numLocals),
		decoder:  &WasmDecoder{},
		executor: &WasmExecutor{},
	}
}

func (s *WasmSimulator) Load(bytecode []byte) {
	s.Bytecode = bytecode
	s.PC = 0
	s.Halted = false
	s.Cycle = 0
	s.Stack = []int{}
	for i := range s.Locals {
		s.Locals[i] = 0
	}
}

func (s *WasmSimulator) Step() WasmStepTrace {
	if s.Halted {
		panic("WASM simulator has halted — no more instructions to execute")
	}
	instruction := s.decoder.Decode(s.Bytecode, s.PC)
	trace := s.executor.Execute(instruction, &s.Stack, s.Locals, s.PC)
	s.PC += instruction.Size
	s.Halted = trace.Halted
	s.Cycle++
	return trace
}

func (s *WasmSimulator) Run(program []byte, maxSteps int) []WasmStepTrace {
	s.Load(program)
	var traces []WasmStepTrace
	for i := 0; i < maxSteps; i++ {
		if s.Halted {
			break
		}
		traces = append(traces, s.Step())
	}
	return traces
}

// Bytecode Assembly Generators

func EncodeI32Const(val int) []byte {
	b := make([]byte, 5)
	b[0] = OpI32Const
	binary.LittleEndian.PutUint32(b[1:], uint32(val))
	return b
}
func EncodeI32Add() []byte { return []byte{OpI32Add} }
func EncodeI32Sub() []byte { return []byte{OpI32Sub} }
func EncodeLocalGet(idx int) []byte { return []byte{OpLocalGet, byte(idx)} }
func EncodeLocalSet(idx int) []byte { return []byte{OpLocalSet, byte(idx)} }
func EncodeEnd() []byte { return []byte{OpEnd} }

func AssembleWasm(instructions [][]byte) []byte {
	var result []byte
	for _, inst := range instructions {
		result = append(result, inst...)
	}
	return result
}
