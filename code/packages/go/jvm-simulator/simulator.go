// Package jvmsimulator provides a stack-based virtual machine explicitly bound by Typed operations.
// 
// === Stack machine with typed opcodes ===
// 
// Like WASM, the JVM is a stack-based machine. But the JVM is typed.
// Where conventional VMs have a generic ADD instruction, the JVM forces variable integrity:
//     iadd          <-- integer add
//     ladd          <-- long add
//     fadd          <-- float add
package jvmsimulator

import "fmt"

const (
	OpIconst0  = 0x03
	OpIconst1  = 0x04
	OpIconst2  = 0x05
	OpIconst3  = 0x06
	OpIconst4  = 0x07
	OpIconst5  = 0x08
	OpBipush   = 0x10
	OpLdc      = 0x12
	OpIload    = 0x15
	OpIload0   = 0x1A
	OpIload1   = 0x1B
	OpIload2   = 0x1C
	OpIload3   = 0x1D
	OpIstore   = 0x36
	OpIstore0  = 0x3B
	OpIstore1  = 0x3C
	OpIstore2  = 0x3D
	OpIstore3  = 0x3E
	OpIadd     = 0x60
	OpIsub     = 0x64
	OpImul     = 0x68
	OpIdiv     = 0x6C
	OpIfIcmpeq = 0x9F
	OpIfIcmpgt = 0xA3
	OpGoto     = 0xA7
	OpIreturn  = 0xAC
	OpReturn   = 0xB1
)

type JVMTrace struct {
	PC             int
	Opcode         string
	StackBefore    []int
	StackAfter     []int
	LocalsSnapshot []*int
	Description    string
}

type JVMSimulator struct {
	Stack       []int
	Locals      []*int
	Constants   []interface{}
	PC          int
	Halted      bool
	ReturnValue *int
	bytecode    []byte
	numLocals   int
}

func NewJVMSimulator() *JVMSimulator {
	return &JVMSimulator{
		numLocals: 16,
		Locals:    make([]*int, 16),
	}
}

func (s *JVMSimulator) Load(bytecode []byte, constants []interface{}, numLocals int) {
	s.bytecode = bytecode
	if constants == nil {
		s.Constants = []interface{}{}
	} else {
		s.Constants = constants
	}
	s.numLocals = numLocals
	s.Stack = []int{}
	s.Locals = make([]*int, numLocals)
	s.PC = 0
	s.Halted = false
	s.ReturnValue = nil
}

func (s *JVMSimulator) Step() JVMTrace {
	if s.Halted {
		panic("JVM simulator has halted")
	}

	pc := s.PC
	if pc >= len(s.bytecode) {
		panic(fmt.Sprintf("PC (%d) past end of bytecode", pc))
	}

	stackBefore := make([]int, len(s.Stack))
	copy(stackBefore, s.Stack)

	opcodeByte := s.bytecode[pc]

	return s.executeOpcode(opcodeByte, stackBefore, pc)
}

func (s *JVMSimulator) Run(maxSteps int) []JVMTrace {
	var traces []JVMTrace
	for i := 0; i < maxSteps; i++ {
		if s.Halted {
			break
		}
		traces = append(traces, s.Step())
	}
	return traces
}

func (s *JVMSimulator) copyLocals() []*int {
	c := make([]*int, len(s.Locals))
	for i, v := range s.Locals {
		if v != nil {
			vCopy := *v
			c[i] = &vCopy
		}
	}
	return c
}

func copyStack(st []int) []int {
	c := make([]int, len(st))
	copy(c, st)
	return c
}

func toI32(val int) int {
	// Equivalent to Python's manual wrapping
	val = val & 0xFFFFFFFF
	if val >= 0x80000000 {
		val -= 0x100000000
	}
	return val
}

func (s *JVMSimulator) executeOpcode(opcode byte, stackBefore []int, pc int) JVMTrace {
	// iconst_N
	if opcode >= OpIconst0 && opcode <= OpIconst5 {
		val := int(opcode - OpIconst0)
		s.Stack = append(s.Stack, val)
		s.PC++
		return JVMTrace{PC: pc, Opcode: fmt.Sprintf("iconst_%d", val), StackBefore: stackBefore, StackAfter: copyStack(s.Stack), LocalsSnapshot: s.copyLocals(), Description: fmt.Sprintf("push %d", val)}
	}

	switch opcode {
	case OpBipush:
		raw := int(s.bytecode[pc+1])
		val := raw
		if val >= 128 {
			val -= 256
		}
		s.Stack = append(s.Stack, val)
		s.PC += 2
		return JVMTrace{PC: pc, Opcode: "bipush", StackBefore: stackBefore, StackAfter: copyStack(s.Stack), LocalsSnapshot: s.copyLocals(), Description: fmt.Sprintf("push %d", val)}
	case OpLdc:
		idx := int(s.bytecode[pc+1])
		if idx >= len(s.Constants) {
			panic(fmt.Sprintf("Constant pool index %d out of range", idx))
		}
		valInterface := s.Constants[idx]
		val, ok := valInterface.(int)
		if !ok {
			panic(fmt.Sprintf("ldc: constant pool %d is not int", idx))
		}
		s.Stack = append(s.Stack, val)
		s.PC += 2
		return JVMTrace{PC: pc, Opcode: "ldc", StackBefore: stackBefore, StackAfter: copyStack(s.Stack), LocalsSnapshot: s.copyLocals(), Description: fmt.Sprintf("push constant[%d] = %d", idx, val)}
	}

	// iload_N
	if opcode >= OpIload0 && opcode <= OpIload3 {
		slot := int(opcode - OpIload0)
		return s.doIload(pc, slot, fmt.Sprintf("iload_%d", slot), stackBefore, 1)
	}
	if opcode == OpIload {
		slot := int(s.bytecode[pc+1])
		return s.doIload(pc, slot, "iload", stackBefore, 2)
	}

	// istore_N
	if opcode >= OpIstore0 && opcode <= OpIstore3 {
		slot := int(opcode - OpIstore0)
		return s.doIstore(pc, slot, fmt.Sprintf("istore_%d", slot), stackBefore, 1)
	}
	if opcode == OpIstore {
		slot := int(s.bytecode[pc+1])
		return s.doIstore(pc, slot, "istore", stackBefore, 2)
	}

	// Operations
	switch opcode {
	case OpIadd:
		return s.doBinaryOp(pc, "iadd", func(a, b int) int { return a + b }, stackBefore)
	case OpIsub:
		return s.doBinaryOp(pc, "isub", func(a, b int) int { return a - b }, stackBefore)
	case OpImul:
		return s.doBinaryOp(pc, "imul", func(a, b int) int { return a * b }, stackBefore)
	case OpIdiv:
		if len(s.Stack) < 2 {
			panic("Stack underflow")
		}
		if s.Stack[len(s.Stack)-1] == 0 {
			panic("ArithmeticException: division by zero")
		}
		return s.doBinaryOp(pc, "idiv", func(a, b int) int { return a / b }, stackBefore)
	case OpGoto:
		b1 := s.bytecode[pc+1]
		b2 := s.bytecode[pc+2]
		raw := int((uint16(b1) << 8) | uint16(b2))
		offset := raw
		if raw >= 0x8000 {
			offset -= 0x10000
		}
		target := pc + offset
		s.PC = target
		return JVMTrace{PC: pc, Opcode: "goto", StackBefore: stackBefore, StackAfter: copyStack(s.Stack), LocalsSnapshot: s.copyLocals(), Description: fmt.Sprintf("jump to PC=%d", target)}
	case OpIfIcmpeq:
		return s.doIfIcmp(pc, "if_icmpeq", stackBefore, func(a, b int) bool { return a == b }, "==")
	case OpIfIcmpgt:
		return s.doIfIcmp(pc, "if_icmpgt", stackBefore, func(a, b int) bool { return a > b }, ">")
	case OpIreturn:
		if len(s.Stack) < 1 {
			panic("Stack underflow")
		}
		val := s.Stack[len(s.Stack)-1]
		s.Stack = s.Stack[:len(s.Stack)-1]
		s.ReturnValue = &val
		s.Halted = true
		s.PC++
		return JVMTrace{PC: pc, Opcode: "ireturn", StackBefore: stackBefore, StackAfter: copyStack(s.Stack), LocalsSnapshot: s.copyLocals(), Description: fmt.Sprintf("return %d", val)}
	case OpReturn:
		s.Halted = true
		s.PC++
		return JVMTrace{PC: pc, Opcode: "return", StackBefore: stackBefore, StackAfter: copyStack(s.Stack), LocalsSnapshot: s.copyLocals(), Description: "return void"}
	}
	panic(fmt.Sprintf("Unimplemented opcode: 0x%02X", opcode))
}

func (s *JVMSimulator) doIload(pc int, slot int, mnemonic string, stackBefore []int, sz int) JVMTrace {
	valPtr := s.Locals[slot]
	if valPtr == nil {
		panic("Local variable uninitialized")
	}
	val := *valPtr
	s.Stack = append(s.Stack, val)
	s.PC += sz
	return JVMTrace{PC: pc, Opcode: mnemonic, StackBefore: stackBefore, StackAfter: copyStack(s.Stack), LocalsSnapshot: s.copyLocals(), Description: fmt.Sprintf("push locals[%d] = %d", slot, val)}
}

func (s *JVMSimulator) doIstore(pc int, slot int, mnemonic string, stackBefore []int, sz int) JVMTrace {
	if len(s.Stack) < 1 {
		panic("Stack underflow")
	}
	val := s.Stack[len(s.Stack)-1]
	s.Stack = s.Stack[:len(s.Stack)-1]
	s.Locals[slot] = &val
	s.PC += sz
	return JVMTrace{PC: pc, Opcode: mnemonic, StackBefore: stackBefore, StackAfter: copyStack(s.Stack), LocalsSnapshot: s.copyLocals(), Description: fmt.Sprintf("pop %d, store in locals[%d]", val, slot)}
}

func (s *JVMSimulator) doBinaryOp(pc int, mnemonic string, op func(int, int) int, stackBefore []int) JVMTrace {
	if len(s.Stack) < 2 {
		panic("Stack underflow")
	}
	b := s.Stack[len(s.Stack)-1]
	a := s.Stack[len(s.Stack)-2]
	s.Stack = s.Stack[:len(s.Stack)-2]
	result := toI32(op(a, b))
	s.Stack = append(s.Stack, result)
	s.PC++
	return JVMTrace{PC: pc, Opcode: mnemonic, StackBefore: stackBefore, StackAfter: copyStack(s.Stack), LocalsSnapshot: s.copyLocals(), Description: fmt.Sprintf("pop %d and %d, push %d", b, a, result)}
}

func (s *JVMSimulator) doIfIcmp(pc int, mnemonic string, stackBefore []int, op func(int, int) bool, operatorStr string) JVMTrace {
	if len(s.Stack) < 2 {
		panic("Stack underflow")
	}
	b1 := s.bytecode[pc+1]
	b2 := s.bytecode[pc+2]
	raw := int((uint16(b1) << 8) | uint16(b2))
	offset := raw
	if raw >= 0x8000 {
		offset -= 0x10000
	}
	b := s.Stack[len(s.Stack)-1]
	a := s.Stack[len(s.Stack)-2]
	s.Stack = s.Stack[:len(s.Stack)-2]
	taken := op(a, b)
	var desc string
	if taken {
		target := pc + offset
		s.PC = target
		desc = fmt.Sprintf("pop %d and %d, true, jump to PC=%d", b, a, target)
	} else {
		s.PC = pc + 3
		desc = fmt.Sprintf("pop %d and %d, false, fall through", b, a)
	}
	return JVMTrace{PC: pc, Opcode: mnemonic, StackBefore: stackBefore, StackAfter: copyStack(s.Stack), LocalsSnapshot: s.copyLocals(), Description: desc}
}

// Bytecode Assembly Generators
func EncodeIconst(n int) []byte {
	if n >= 0 && n <= 5 {
		return []byte{byte(OpIconst0 + n)}
	}
	if n >= -128 && n <= 127 {
		raw := n
		if n < 0 {
			raw += 256
		}
		return []byte{OpBipush, byte(raw)}
	}
	panic("Out of range")
}

func EncodeIstore(slot int) []byte {
	if slot >= 0 && slot <= 3 {
		return []byte{byte(OpIstore0 + slot)}
	}
	return []byte{OpIstore, byte(slot)}
}

func EncodeIload(slot int) []byte {
	if slot >= 0 && slot <= 3 {
		return []byte{byte(OpIload0 + slot)}
	}
	return []byte{OpIload, byte(slot)}
}

type Instr struct {
	Opcode byte
	Params []int
}
func AssembleJvm(instructions []Instr) []byte {
	var res []byte
	for _, inst := range instructions {
		res = append(res, inst.Opcode)
		switch inst.Opcode {
		case OpBipush, OpIload, OpIstore, OpLdc:
			res = append(res, byte(inst.Params[0]))
		case OpGoto, OpIfIcmpeq, OpIfIcmpgt:
			off := inst.Params[0]
			raw := uint16(off)
			res = append(res, byte((raw>>8)&0xFF), byte(raw&0xFF))
		}
	}
	return res
}
