// Package clrsimulator embodies Microsoft's Common Language Runtime.
//
// === CLR vs JVM: Two philosophies of stack machines ===
//
// Both the JVM and CLR are stack-based virtual machines, but they take different
// approaches to type information:
//
//     JVM approach — type in the opcode:
//         iadd        ← "i" means int32 addition
//
//     CLR approach — type inferred from the stack:
//         add         ← type inferred! works for int32, int64, float...
//
// # Operations
//
// Every public method is wrapped in an Operation, giving each call
// automatic timing, structured logging, and panic recovery.
package clrsimulator

import (
	"encoding/binary"
	"fmt"
)

const (
	OpNop       = 0x00
	OpLdnull    = 0x01
	OpLdloc0    = 0x06
	OpLdloc1    = 0x07
	OpLdloc2    = 0x08
	OpLdloc3    = 0x09
	OpStloc0    = 0x0A
	OpStloc1    = 0x0B
	OpStloc2    = 0x0C
	OpStloc3    = 0x0D
	OpLdlocS    = 0x11
	OpStlocS    = 0x13
	OpLdcI4_0   = 0x16
	OpLdcI4_1   = 0x17
	OpLdcI4_2   = 0x18
	OpLdcI4_3   = 0x19
	OpLdcI4_4   = 0x1A
	OpLdcI4_5   = 0x1B
	OpLdcI4_6   = 0x1C
	OpLdcI4_7   = 0x1D
	OpLdcI4_8   = 0x1E
	OpLdcI4S    = 0x1F
	OpLdcI4     = 0x20
	OpRet       = 0x2A
	OpBrS       = 0x2B
	OpBrfalseS  = 0x2C
	OpBrtrueS   = 0x2D
	OpAdd       = 0x58
	OpSub       = 0x59
	OpMul       = 0x5A
	OpDiv       = 0x5B
	OpPrefixFE  = 0xFE
)

const (
	CeqByte = 0x01
	CgtByte = 0x02
	CltByte = 0x04
)

type CLRTrace struct {
	PC             int
	Opcode         string
	StackBefore    []*int
	StackAfter     []*int
	LocalsSnapshot []*int
	Description    string
}

type CLRSimulator struct {
	Stack    []*int
	Locals   []*int
	PC       int
	Bytecode []byte
	Halted   bool
}

// NewCLRSimulator creates a fresh CLR simulator with an empty stack and 16 local variable slots.
func NewCLRSimulator() *CLRSimulator {
	result, _ := StartNew[*CLRSimulator]("clr-simulator.NewCLRSimulator", nil,
		func(op *Operation[*CLRSimulator], rf *ResultFactory[*CLRSimulator]) *OperationResult[*CLRSimulator] {
			return rf.Generate(true, false, &CLRSimulator{
				Stack:  []*int{},
				Locals: make([]*int, 16),
				PC:     0,
			})
		}).GetResult()
	return result
}

// Load resets the simulator state and loads new bytecode with a given number of local variable slots.
func (s *CLRSimulator) Load(bytecode []byte, numLocals int) {
	_, _ = StartNew[struct{}]("clr-simulator.Load", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("bytecodeLen", len(bytecode))
			op.AddProperty("numLocals", numLocals)
			s.Bytecode = bytecode
			s.Stack = []*int{}
			s.Locals = make([]*int, numLocals)
			s.PC = 0
			s.Halted = false
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Step executes a single CLR instruction and returns a trace of what happened.
func (s *CLRSimulator) Step() CLRTrace {
	result, _ := StartNew[CLRTrace]("clr-simulator.Step", CLRTrace{},
		func(op *Operation[CLRTrace], rf *ResultFactory[CLRTrace]) *OperationResult[CLRTrace] {
			op.AddProperty("pc", s.PC)
			if s.Halted {
				panic("CLR simulator has halted")
			}

			pc := s.PC
			if pc >= len(s.Bytecode) {
				panic(fmt.Sprintf("PC (%d) beyond bytecode length", pc))
			}

			stackBefore := s.copyArr(s.Stack)
			opcodeByte := s.Bytecode[pc]

			if opcodeByte == OpPrefixFE {
				return rf.Generate(true, false, s.executeTwoByteOpcode(stackBefore))
			}

			if opcodeByte == OpNop {
				s.PC++
				return rf.Generate(true, false, CLRTrace{PC: pc, Opcode: "nop", StackBefore: stackBefore, StackAfter: s.copyArr(s.Stack), LocalsSnapshot: s.copyArr(s.Locals), Description: "no operation"})
			}

			if opcodeByte == OpLdnull {
				s.Stack = append(s.Stack, nil)
				s.PC++
				return rf.Generate(true, false, CLRTrace{PC: pc, Opcode: "ldnull", StackBefore: stackBefore, StackAfter: s.copyArr(s.Stack), LocalsSnapshot: s.copyArr(s.Locals), Description: "push null"})
			}

			if opcodeByte >= OpLdcI4_0 && opcodeByte <= OpLdcI4_8 {
				value := int(opcodeByte - OpLdcI4_0)
				valPtr := &value
				s.Stack = append(s.Stack, valPtr)
				s.PC++
				return rf.Generate(true, false, CLRTrace{PC: pc, Opcode: fmt.Sprintf("ldc.i4.%d", value), StackBefore: stackBefore, StackAfter: s.copyArr(s.Stack), LocalsSnapshot: s.copyArr(s.Locals), Description: fmt.Sprintf("push %d", value)})
			}

			if opcodeByte == OpLdcI4S {
				raw := int(s.Bytecode[pc+1])
				if raw >= 128 {
					raw -= 256
				}
				valPtr := &raw
				s.Stack = append(s.Stack, valPtr)
				s.PC += 2
				return rf.Generate(true, false, CLRTrace{PC: pc, Opcode: "ldc.i4.s", StackBefore: stackBefore, StackAfter: s.copyArr(s.Stack), LocalsSnapshot: s.copyArr(s.Locals), Description: fmt.Sprintf("push %d", raw)})
			}

			if opcodeByte == OpLdcI4 {
				rawBytes := s.Bytecode[pc+1 : pc+5]
				val := int(int32(binary.LittleEndian.Uint32(rawBytes)))
				valPtr := &val
				s.Stack = append(s.Stack, valPtr)
				s.PC += 5
				return rf.Generate(true, false, CLRTrace{PC: pc, Opcode: "ldc.i4", StackBefore: stackBefore, StackAfter: s.copyArr(s.Stack), LocalsSnapshot: s.copyArr(s.Locals), Description: fmt.Sprintf("push %d", val)})
			}

			if opcodeByte >= OpLdloc0 && opcodeByte <= OpLdloc3 {
				slot := int(opcodeByte - OpLdloc0)
				val := s.Locals[slot]
				if val == nil {
					panic(fmt.Sprintf("Local %d uninitialized", slot))
				}
				s.Stack = append(s.Stack, val)
				s.PC++
				return rf.Generate(true, false, CLRTrace{PC: pc, Opcode: fmt.Sprintf("ldloc.%d", slot), StackBefore: stackBefore, StackAfter: s.copyArr(s.Stack), LocalsSnapshot: s.copyArr(s.Locals), Description: fmt.Sprintf("push locals[%d] = %d", slot, *val)})
			}

			if opcodeByte >= OpStloc0 && opcodeByte <= OpStloc3 {
				slot := int(opcodeByte - OpStloc0)
				val := s.Stack[len(s.Stack)-1]
				s.Stack = s.Stack[:len(s.Stack)-1]
				s.Locals[slot] = val
				s.PC++
				desc := "pop null"
				if val != nil {
					desc = fmt.Sprintf("pop %d", *val)
				}
				return rf.Generate(true, false, CLRTrace{PC: pc, Opcode: fmt.Sprintf("stloc.%d", slot), StackBefore: stackBefore, StackAfter: s.copyArr(s.Stack), LocalsSnapshot: s.copyArr(s.Locals), Description: fmt.Sprintf("%s, store in locals[%d]", desc, slot)})
			}

			if opcodeByte == OpLdlocS {
				slot := int(s.Bytecode[pc+1])
				val := s.Locals[slot]
				if val == nil {
					panic(fmt.Sprintf("Local %d uninitialized", slot))
				}
				s.Stack = append(s.Stack, val)
				s.PC += 2
				return rf.Generate(true, false, CLRTrace{PC: pc, Opcode: "ldloc.s", StackBefore: stackBefore, StackAfter: s.copyArr(s.Stack), LocalsSnapshot: s.copyArr(s.Locals), Description: fmt.Sprintf("push locals[%d] = %d", slot, *val)})
			}

			if opcodeByte == OpStlocS {
				slot := int(s.Bytecode[pc+1])
				val := s.Stack[len(s.Stack)-1]
				s.Stack = s.Stack[:len(s.Stack)-1]
				s.Locals[slot] = val
				s.PC += 2
				desc := "pop null"
				if val != nil {
					desc = fmt.Sprintf("pop %d", *val)
				}
				return rf.Generate(true, false, CLRTrace{PC: pc, Opcode: "stloc.s", StackBefore: stackBefore, StackAfter: s.copyArr(s.Stack), LocalsSnapshot: s.copyArr(s.Locals), Description: fmt.Sprintf("%s, store in locals[%d]", desc, slot)})
			}

			if opcodeByte == OpAdd {
				return rf.Generate(true, false, s.executeArithmetic(stackBefore, "add", func(a, b int) int { return a + b }))
			}
			if opcodeByte == OpSub {
				return rf.Generate(true, false, s.executeArithmetic(stackBefore, "sub", func(a, b int) int { return a - b }))
			}
			if opcodeByte == OpMul {
				return rf.Generate(true, false, s.executeArithmetic(stackBefore, "mul", func(a, b int) int { return a * b }))
			}
			if opcodeByte == OpDiv {
				b := s.Stack[len(s.Stack)-1]
				a := s.Stack[len(s.Stack)-2]
				if b == nil || a == nil {
					panic("Math ops require valid ints")
				}
				if *b == 0 {
					panic("System.DivideByZeroException: division by zero")
				}
				result := *a / *b
				resPtr := &result
				s.Stack[len(s.Stack)-2] = resPtr
				s.Stack = s.Stack[:len(s.Stack)-1]
				s.PC++
				return rf.Generate(true, false, CLRTrace{PC: pc, Opcode: "div", StackBefore: stackBefore, StackAfter: s.copyArr(s.Stack), LocalsSnapshot: s.copyArr(s.Locals), Description: fmt.Sprintf("pop %d and %d, push %d", *b, *a, result)})
			}

			if opcodeByte == OpRet {
				s.PC++
				s.Halted = true
				return rf.Generate(true, false, CLRTrace{PC: pc, Opcode: "ret", StackBefore: stackBefore, StackAfter: s.copyArr(s.Stack), LocalsSnapshot: s.copyArr(s.Locals), Description: "return"})
			}

			if opcodeByte == OpBrS {
				return rf.Generate(true, false, s.executeBranchS(stackBefore, "br.s", true, false))
			}
			if opcodeByte == OpBrfalseS {
				return rf.Generate(true, false, s.executeBranchS(stackBefore, "brfalse.s", false, true))
			}
			if opcodeByte == OpBrtrueS {
				return rf.Generate(true, false, s.executeBranchS(stackBefore, "brtrue.s", false, false))
			}

			panic(fmt.Sprintf("Unknown CLR opcode: 0x%02X at PC=%d", opcodeByte, pc))
		}).GetResult()
	return result
}

func (s *CLRSimulator) executeTwoByteOpcode(stackBefore []*int) CLRTrace {
	pc := s.PC
	secondByte := s.Bytecode[pc+1]

	bStr := s.Stack[len(s.Stack)-1]
	aStr := s.Stack[len(s.Stack)-2]

	if bStr == nil || aStr == nil {
		panic("Cannot compare nulls logically here")
	}
	b := *bStr
	a := *aStr
	s.Stack = s.Stack[:len(s.Stack)-2]

	var result int
	var mnemonic string
	var op string
	if secondByte == CeqByte {
		mnemonic = "ceq"
		op = "=="
		if a == b {
			result = 1
		}
	} else if secondByte == CgtByte {
		mnemonic = "cgt"
		op = ">"
		if a > b {
			result = 1
		}
	} else if secondByte == CltByte {
		mnemonic = "clt"
		op = "<"
		if a < b {
			result = 1
		}
	} else {
		panic(fmt.Sprintf("Unknown two-byte opcode: 0xFE 0x%02X", secondByte))
	}
	resPtr := &result
	s.Stack = append(s.Stack, resPtr)
	s.PC += 2
	return CLRTrace{PC: pc, Opcode: mnemonic, StackBefore: stackBefore, StackAfter: s.copyArr(s.Stack), LocalsSnapshot: s.copyArr(s.Locals), Description: fmt.Sprintf("pop %d and %d, push %d (%d %s %d)", b, a, result, a, op, b)}
}

func (s *CLRSimulator) executeArithmetic(stackBefore []*int, mnemonic string, op func(int, int) int) CLRTrace {
	b := s.Stack[len(s.Stack)-1]
	a := s.Stack[len(s.Stack)-2]
	if b == nil || a == nil {
		panic("Math mapping nulls")
	}
	result := op(*a, *b)
	resPtr := &result
	s.Stack[len(s.Stack)-2] = resPtr
	s.Stack = s.Stack[:len(s.Stack)-1]
	s.PC++
	return CLRTrace{PC: s.PC - 1, Opcode: mnemonic, StackBefore: stackBefore, StackAfter: s.copyArr(s.Stack), LocalsSnapshot: s.copyArr(s.Locals), Description: fmt.Sprintf("pop %d and %d, push %d", *b, *a, result)}
}

func (s *CLRSimulator) executeBranchS(stackBefore []*int, mnemonic string, always bool, takeIfZero bool) CLRTrace {
	pc := s.PC
	raw := int(s.Bytecode[pc+1])
	if raw >= 128 {
		raw -= 256
	}
	offset := raw
	nextPc := pc + 2
	target := nextPc + offset

	if always {
		s.PC = target
		return CLRTrace{PC: pc, Opcode: mnemonic, StackBefore: stackBefore, StackAfter: s.copyArr(s.Stack), LocalsSnapshot: s.copyArr(s.Locals), Description: fmt.Sprintf("branch to PC=%d (offset %d)", target, offset)}
	}

	valPtr := s.Stack[len(s.Stack)-1]
	s.Stack = s.Stack[:len(s.Stack)-1]
	valConv := 0
	if valPtr != nil {
		valConv = *valPtr
	}

	shouldBranch := false
	if takeIfZero {
		shouldBranch = (valConv == 0)
	} else {
		shouldBranch = (valConv != 0)
	}

	descVal := "null"
	if valPtr != nil {
		descVal = fmt.Sprintf("%d", *valPtr)
	}

	if shouldBranch {
		s.PC = target
		return CLRTrace{PC: pc, Opcode: mnemonic, StackBefore: stackBefore, StackAfter: s.copyArr(s.Stack), LocalsSnapshot: s.copyArr(s.Locals), Description: fmt.Sprintf("pop %s, branch taken to PC=%d", descVal, target)}
	}

	s.PC = nextPc
	return CLRTrace{PC: pc, Opcode: mnemonic, StackBefore: stackBefore, StackAfter: s.copyArr(s.Stack), LocalsSnapshot: s.copyArr(s.Locals), Description: fmt.Sprintf("pop %s, branch not taken", descVal)}
}

func (s *CLRSimulator) copyArr(arr []*int) []*int {
	c := make([]*int, len(arr))
	for i, v := range arr {
		if v != nil {
			vp := *v
			c[i] = &vp
		}
	}
	return c
}

// Run executes instructions until the simulator halts or maxSteps is reached, returning all traces.
func (s *CLRSimulator) Run(maxSteps int) []CLRTrace {
	result, _ := StartNew[[]CLRTrace]("clr-simulator.Run", nil,
		func(op *Operation[[]CLRTrace], rf *ResultFactory[[]CLRTrace]) *OperationResult[[]CLRTrace] {
			op.AddProperty("maxSteps", maxSteps)
			var traces []CLRTrace
			for i := 0; i < maxSteps; i++ {
				if s.Halted {
					break
				}
				traces = append(traces, s.Step())
			}
			return rf.Generate(true, false, traces)
		}).GetResult()
	return result
}

// EncodeLdcI4 encodes an integer constant push instruction using the most compact encoding.
func EncodeLdcI4(n int) []byte {
	result, _ := StartNew[[]byte]("clr-simulator.EncodeLdcI4", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			op.AddProperty("n", n)
			if n >= 0 && n <= 8 {
				return rf.Generate(true, false, []byte{byte(OpLdcI4_0 + n)})
			}
			if n >= -128 && n <= 127 {
				raw := n
				if n < 0 {
					raw += 256
				}
				return rf.Generate(true, false, []byte{OpLdcI4S, byte(raw)})
			}
			res := make([]byte, 5)
			res[0] = OpLdcI4
			binary.LittleEndian.PutUint32(res[1:], uint32(n))
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}

// EncodeStloc encodes a store-local instruction using the most compact form for slots 0–3.
func EncodeStloc(slot int) []byte {
	result, _ := StartNew[[]byte]("clr-simulator.EncodeStloc", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			op.AddProperty("slot", slot)
			if slot >= 0 && slot <= 3 {
				return rf.Generate(true, false, []byte{byte(OpStloc0 + slot)})
			}
			return rf.Generate(true, false, []byte{OpStlocS, byte(slot)})
		}).GetResult()
	return result
}

// EncodeLdloc encodes a load-local instruction using the most compact form for slots 0–3.
func EncodeLdloc(slot int) []byte {
	result, _ := StartNew[[]byte]("clr-simulator.EncodeLdloc", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			op.AddProperty("slot", slot)
			if slot >= 0 && slot <= 3 {
				return rf.Generate(true, false, []byte{byte(OpLdloc0 + slot)})
			}
			return rf.Generate(true, false, []byte{OpLdlocS, byte(slot)})
		}).GetResult()
	return result
}

// AssembleClr concatenates encoded instruction byte slices into a single bytecode program.
func AssembleClr(instructions [][]byte) []byte {
	result, _ := StartNew[[]byte]("clr-simulator.AssembleClr", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			op.AddProperty("instructionCount", len(instructions))
			var res []byte
			for _, inst := range instructions {
				res = append(res, inst...)
			}
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}
