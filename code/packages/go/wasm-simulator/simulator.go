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
	result, _ := StartNew[WasmInstruction]("wasm-simulator.WasmDecoder.Decode", WasmInstruction{},
		func(op *Operation[WasmInstruction], rf *ResultFactory[WasmInstruction]) *OperationResult[WasmInstruction] {
			op.AddProperty("pc", pc)
			opcode := bytecode[pc]

			switch opcode {
			case OpI32Const:
				valBytes := bytecode[pc+1 : pc+5]
				val := int(int32(binary.LittleEndian.Uint32(valBytes)))
				return rf.Generate(true, false, WasmInstruction{Opcode: opcode, Mnemonic: "i32.const", Operand: &val, Size: 5})
			case OpI32Add:
				return rf.Generate(true, false, WasmInstruction{Opcode: opcode, Mnemonic: "i32.add", Size: 1})
			case OpI32Sub:
				return rf.Generate(true, false, WasmInstruction{Opcode: opcode, Mnemonic: "i32.sub", Size: 1})
			case OpLocalGet:
				val := int(bytecode[pc+1])
				return rf.Generate(true, false, WasmInstruction{Opcode: opcode, Mnemonic: "local.get", Operand: &val, Size: 2})
			case OpLocalSet:
				val := int(bytecode[pc+1])
				return rf.Generate(true, false, WasmInstruction{Opcode: opcode, Mnemonic: "local.set", Operand: &val, Size: 2})
			case OpEnd:
				return rf.Generate(true, false, WasmInstruction{Opcode: opcode, Mnemonic: "end", Size: 1})
			default:
				panic(fmt.Sprintf("Unknown WASM opcode: 0x%02X at PC=%d", opcode, pc))
			}
		}).PanicOnUnexpected().GetResult()
	return result
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
	result, _ := StartNew[WasmStepTrace]("wasm-simulator.WasmExecutor.Execute", WasmStepTrace{},
		func(op *Operation[WasmStepTrace], rf *ResultFactory[WasmStepTrace]) *OperationResult[WasmStepTrace] {
			op.AddProperty("pc", pc)
			op.AddProperty("mnemonic", instruction.Mnemonic)
			stackBefore := make([]int, len(*stack))
			copy(stackBefore, *stack)

			mnemonic := instruction.Mnemonic
			switch mnemonic {
			case "i32.const":
				val := *instruction.Operand
				*stack = append(*stack, val)
				return rf.Generate(true, false, WasmStepTrace{PC: pc, Instruction: instruction, StackBefore: stackBefore, StackAfter: copyStack(*stack), LocalsSnapshot: copyStack(locals), Description: fmt.Sprintf("push %d", val)})
			case "i32.add":
				b := (*stack)[len(*stack)-1]
				a := (*stack)[len(*stack)-2]
				*stack = (*stack)[:len(*stack)-2]

				res := int(uint32(a + b)) // Masking to simulate 32-bit truncation universally
				*stack = append(*stack, res)
				return rf.Generate(true, false, WasmStepTrace{PC: pc, Instruction: instruction, StackBefore: stackBefore, StackAfter: copyStack(*stack), LocalsSnapshot: copyStack(locals), Description: fmt.Sprintf("pop %d and %d, push %d", b, a, res)})
			case "i32.sub":
				b := (*stack)[len(*stack)-1]
				a := (*stack)[len(*stack)-2]
				*stack = (*stack)[:len(*stack)-2]

				res := int(uint32(a - b))
				*stack = append(*stack, res)
				return rf.Generate(true, false, WasmStepTrace{PC: pc, Instruction: instruction, StackBefore: stackBefore, StackAfter: copyStack(*stack), LocalsSnapshot: copyStack(locals), Description: fmt.Sprintf("pop %d and %d, push %d", b, a, res)})
			case "local.get":
				idx := *instruction.Operand
				val := locals[idx]
				*stack = append(*stack, val)
				return rf.Generate(true, false, WasmStepTrace{PC: pc, Instruction: instruction, StackBefore: stackBefore, StackAfter: copyStack(*stack), LocalsSnapshot: copyStack(locals), Description: fmt.Sprintf("push locals[%d] = %d", idx, val)})
			case "local.set":
				idx := *instruction.Operand
				val := (*stack)[len(*stack)-1]
				*stack = (*stack)[:len(*stack)-1]
				locals[idx] = val
				return rf.Generate(true, false, WasmStepTrace{PC: pc, Instruction: instruction, StackBefore: stackBefore, StackAfter: copyStack(*stack), LocalsSnapshot: copyStack(locals), Description: fmt.Sprintf("pop %d, store in locals[%d]", val, idx)})
			case "end":
				return rf.Generate(true, false, WasmStepTrace{PC: pc, Instruction: instruction, StackBefore: stackBefore, StackAfter: copyStack(*stack), LocalsSnapshot: copyStack(locals), Description: "halt", Halted: true})
			default:
				panic(fmt.Sprintf("Cannot execute: %s", mnemonic))
			}
		}).PanicOnUnexpected().GetResult()
	return result
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
	result, _ := StartNew[*WasmSimulator]("wasm-simulator.NewWasmSimulator", nil,
		func(op *Operation[*WasmSimulator], rf *ResultFactory[*WasmSimulator]) *OperationResult[*WasmSimulator] {
			op.AddProperty("numLocals", numLocals)
			return rf.Generate(true, false, &WasmSimulator{
				Stack:    []int{},
				Locals:   make([]int, numLocals),
				decoder:  &WasmDecoder{},
				executor: &WasmExecutor{},
			})
		}).GetResult()
	return result
}

func (s *WasmSimulator) Load(bytecode []byte) {
	_, _ = StartNew[struct{}]("wasm-simulator.WasmSimulator.Load", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			s.Bytecode = bytecode
			s.PC = 0
			s.Halted = false
			s.Cycle = 0
			s.Stack = []int{}
			for i := range s.Locals {
				s.Locals[i] = 0
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

func (s *WasmSimulator) Step() WasmStepTrace {
	result, _ := StartNew[WasmStepTrace]("wasm-simulator.WasmSimulator.Step", WasmStepTrace{},
		func(op *Operation[WasmStepTrace], rf *ResultFactory[WasmStepTrace]) *OperationResult[WasmStepTrace] {
			op.AddProperty("pc", s.PC)
			if s.Halted {
				panic("WASM simulator has halted — no more instructions to execute")
			}
			instruction := s.decoder.Decode(s.Bytecode, s.PC)
			trace := s.executor.Execute(instruction, &s.Stack, s.Locals, s.PC)
			s.PC += instruction.Size
			s.Halted = trace.Halted
			s.Cycle++
			return rf.Generate(true, false, trace)
		}).PanicOnUnexpected().GetResult()
	return result
}

func (s *WasmSimulator) Run(program []byte, maxSteps int) []WasmStepTrace {
	result, _ := StartNew[[]WasmStepTrace]("wasm-simulator.WasmSimulator.Run", nil,
		func(op *Operation[[]WasmStepTrace], rf *ResultFactory[[]WasmStepTrace]) *OperationResult[[]WasmStepTrace] {
			op.AddProperty("maxSteps", maxSteps)
			s.Load(program)
			var traces []WasmStepTrace
			for i := 0; i < maxSteps; i++ {
				if s.Halted {
					break
				}
				traces = append(traces, s.Step())
			}
			return rf.Generate(true, false, traces)
		}).PanicOnUnexpected().GetResult()
	return result
}

// Bytecode Assembly Generators

func EncodeI32Const(val int) []byte {
	result, _ := StartNew[[]byte]("wasm-simulator.EncodeI32Const", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			op.AddProperty("val", val)
			b := make([]byte, 5)
			b[0] = OpI32Const
			binary.LittleEndian.PutUint32(b[1:], uint32(val))
			return rf.Generate(true, false, b)
		}).GetResult()
	return result
}

func EncodeI32Add() []byte {
	result, _ := StartNew[[]byte]("wasm-simulator.EncodeI32Add", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			return rf.Generate(true, false, []byte{OpI32Add})
		}).GetResult()
	return result
}

func EncodeI32Sub() []byte {
	result, _ := StartNew[[]byte]("wasm-simulator.EncodeI32Sub", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			return rf.Generate(true, false, []byte{OpI32Sub})
		}).GetResult()
	return result
}

func EncodeLocalGet(idx int) []byte {
	result, _ := StartNew[[]byte]("wasm-simulator.EncodeLocalGet", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			op.AddProperty("idx", idx)
			return rf.Generate(true, false, []byte{OpLocalGet, byte(idx)})
		}).GetResult()
	return result
}

func EncodeLocalSet(idx int) []byte {
	result, _ := StartNew[[]byte]("wasm-simulator.EncodeLocalSet", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			op.AddProperty("idx", idx)
			return rf.Generate(true, false, []byte{OpLocalSet, byte(idx)})
		}).GetResult()
	return result
}

func EncodeEnd() []byte {
	result, _ := StartNew[[]byte]("wasm-simulator.EncodeEnd", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			return rf.Generate(true, false, []byte{OpEnd})
		}).GetResult()
	return result
}

func AssembleWasm(instructions [][]byte) []byte {
	result, _ := StartNew[[]byte]("wasm-simulator.AssembleWasm", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			var res []byte
			for _, inst := range instructions {
				res = append(res, inst...)
			}
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}
