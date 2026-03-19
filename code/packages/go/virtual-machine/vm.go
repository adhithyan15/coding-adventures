// Package virtualmachine provides a dynamic, untyped stack-based interpreter defining Layer 5.
package virtualmachine

import (
	"fmt"
)

type OpCode int

const (
	OpLoadConst   OpCode = 0x01
	OpPop         OpCode = 0x02
	OpDup         OpCode = 0x03
	OpStoreName   OpCode = 0x10
	OpLoadName    OpCode = 0x11
	OpStoreLocal  OpCode = 0x12
	OpLoadLocal   OpCode = 0x13
	OpAdd         OpCode = 0x20
	OpSub         OpCode = 0x21
	OpMul         OpCode = 0x22
	OpDiv         OpCode = 0x23
	OpCmpEq       OpCode = 0x30
	OpCmpLt       OpCode = 0x31
	OpCmpGt       OpCode = 0x32
	OpJump        OpCode = 0x40
	OpJumpIfFalse OpCode = 0x41
	OpJumpIfTrue  OpCode = 0x42
	OpCall        OpCode = 0x50
	OpReturn      OpCode = 0x51
	OpPrint       OpCode = 0x60
	OpHalt        OpCode = 0xFF
)

type Instruction struct {
	Opcode  OpCode
	Operand interface{}
}

func (i Instruction) String() string {
	if i.Operand != nil {
		return fmt.Sprintf("Instruction(%d, %v)", i.Opcode, i.Operand)
	}
	return fmt.Sprintf("Instruction(%d)", i.Opcode)
}

type CodeObject struct {
	Instructions []Instruction
	Constants    []interface{}
	Names        []string
}

type CallFrame struct {
	ReturnAddress  int
	SavedVariables map[string]interface{}
	SavedLocals    []interface{}
}

type VMTrace struct {
	PC          int
	Instruction Instruction
	StackBefore []interface{}
	StackAfter  []interface{}
	Variables   map[string]interface{}
	Output      *string
	Description string
}

type VirtualMachine struct {
	Stack     []interface{}
	Variables map[string]interface{}
	Locals    []interface{}
	PC        int
	Halted    bool
	Output    []string
	CallStack []CallFrame
}

func NewVirtualMachine() *VirtualMachine {
	return &VirtualMachine{
		Stack:     []interface{}{},
		Variables: make(map[string]interface{}),
		Locals:    []interface{}{},
		PC:        0,
		Halted:    false,
		Output:    []string{},
		CallStack: []CallFrame{},
	}
}

func (vm *VirtualMachine) Execute(code CodeObject) []VMTrace {
	var traces []VMTrace
	for !vm.Halted && vm.PC < len(code.Instructions) {
		traces = append(traces, vm.Step(code))
	}
	return traces
}

func (vm *VirtualMachine) Step(code CodeObject) VMTrace {
	instr := code.Instructions[vm.PC]
	pcBefore := vm.PC
	stackBefore := vm.copyStack()

	var outputVal *string
	desc := ""

	switch instr.Opcode {
	case OpLoadConst:
		idx := instr.Operand.(int)
		if idx < 0 || idx >= len(code.Constants) {
			panic(fmt.Sprintf("InvalidOperandError: LOAD_CONST %d out of bounds", idx))
		}
		val := code.Constants[idx]
		vm.Stack = append(vm.Stack, val)
		vm.PC++
		desc = fmt.Sprintf("Push constant %v onto the stack", val)

	case OpPop:
		vm.pop()
		vm.PC++
		desc = "Discard top of stack"

	case OpDup:
		if len(vm.Stack) == 0 {
			panic("StackUnderflowError: DUP empty stack")
		}
		vm.Stack = append(vm.Stack, vm.Stack[len(vm.Stack)-1])
		vm.PC++
		desc = "Duplicate top of stack"

	case OpStoreName:
		idx := instr.Operand.(int)
		name := code.Names[idx]
		val := vm.pop()
		vm.Variables[name] = val
		vm.PC++
		desc = fmt.Sprintf("Store %v into variable '%s'", val, name)

	case OpLoadName:
		idx := instr.Operand.(int)
		name := code.Names[idx]
		val, ok := vm.Variables[name]
		if !ok {
			panic(fmt.Sprintf("UndefinedNameError: Variable '%s' is not defined", name))
		}
		vm.Stack = append(vm.Stack, val)
		vm.PC++
		desc = fmt.Sprintf("Push variable '%s' onto the stack", name)

	case OpStoreLocal:
		idx := instr.Operand.(int)
		val := vm.pop()
		for len(vm.Locals) <= idx {
			vm.Locals = append(vm.Locals, nil)
		}
		vm.Locals[idx] = val
		vm.PC++
		desc = fmt.Sprintf("Store %v into local slot %d", val, idx)

	case OpLoadLocal:
		idx := instr.Operand.(int)
		if idx < 0 || idx >= len(vm.Locals) {
			panic(fmt.Sprintf("InvalidOperandError: LOAD_LOCAL %d uninitialized", idx))
		}
		vm.Stack = append(vm.Stack, vm.Locals[idx])
		vm.PC++
		desc = fmt.Sprintf("Push local slot %d onto the stack", idx)

	case OpAdd:
		b := vm.pop()
		a := vm.pop()
		res := vm.add(a, b)
		vm.Stack = append(vm.Stack, res)
		vm.PC++
		desc = fmt.Sprintf("Pop %v and %v, push sum %v", b, a, res)

	case OpSub:
		b := vm.pop().(int)
		a := vm.pop().(int)
		res := a - b
		vm.Stack = append(vm.Stack, res)
		vm.PC++
		desc = fmt.Sprintf("Pop %v and %v, push difference %v", b, a, res)

	case OpMul:
		b := vm.pop().(int)
		a := vm.pop().(int)
		res := a * b
		vm.Stack = append(vm.Stack, res)
		vm.PC++
		desc = fmt.Sprintf("Pop %v and %v, push product %v", b, a, res)

	case OpDiv:
		b := vm.pop().(int)
		a := vm.pop().(int)
		if b == 0 {
			panic("DivisionByZeroError: Division by zero")
		}
		res := a / b
		vm.Stack = append(vm.Stack, res)
		vm.PC++
		desc = fmt.Sprintf("Pop %v and %v, push quotient %v", b, a, res)

	case OpCmpEq:
		b := vm.pop()
		a := vm.pop()
		res := 0
		if a == b {
			res = 1
		}
		vm.Stack = append(vm.Stack, res)
		vm.PC++
		desc = fmt.Sprintf("Compare %v == %v", a, b)

	case OpCmpLt:
		b := vm.pop().(int)
		a := vm.pop().(int)
		res := 0
		if a < b {
			res = 1
		}
		vm.Stack = append(vm.Stack, res)
		vm.PC++
		desc = fmt.Sprintf("Compare %v < %v", a, b)

	case OpCmpGt:
		b := vm.pop().(int)
		a := vm.pop().(int)
		res := 0
		if a > b {
			res = 1
		}
		vm.Stack = append(vm.Stack, res)
		vm.PC++
		desc = fmt.Sprintf("Compare %v > %v", a, b)

	case OpJump:
		target := instr.Operand.(int)
		vm.PC = target
		desc = fmt.Sprintf("Jump to instruction %d", target)

	case OpJumpIfFalse:
		target := instr.Operand.(int)
		val := vm.pop()
		if vm.isFalsy(val) {
			vm.PC = target
		} else {
			vm.PC++
		}
		desc = fmt.Sprintf("Pop %v, conditional jump", val)

	case OpJumpIfTrue:
		target := instr.Operand.(int)
		val := vm.pop()
		if !vm.isFalsy(val) {
			vm.PC = target
		} else {
			vm.PC++
		}
		desc = fmt.Sprintf("Pop %v, conditional jump", val)

	case OpCall:
		idx := instr.Operand.(int)
		funcName := code.Names[idx]
		funcObj, ok := vm.Variables[funcName]
		if !ok {
			panic(fmt.Sprintf("UndefinedNameError: Function '%s' is not defined", funcName))
		}
		funcCode, ok := funcObj.(CodeObject)
		if !ok {
			panic("VMError: Object is not callable")
		}

		frame := CallFrame{
			ReturnAddress:  vm.PC + 1,
			SavedVariables: vm.copyMap(vm.Variables),
			SavedLocals:    vm.copyStackArr(vm.Locals),
		}
		vm.CallStack = append(vm.CallStack, frame)

		vm.Locals = []interface{}{}
		savedPc := vm.PC
		vm.PC = 0
		
		for !vm.Halted && vm.PC < len(funcCode.Instructions) {
			if funcCode.Instructions[vm.PC].Opcode == OpReturn {
				break
			}
			vm.Step(funcCode)
		}

		popped := vm.CallStack[len(vm.CallStack)-1]
		vm.CallStack = vm.CallStack[:len(vm.CallStack)-1]
		vm.PC = popped.ReturnAddress
		vm.Locals = popped.SavedLocals
		desc = fmt.Sprintf("Call function '%s'", funcName)
		_ = savedPc

	case OpReturn:
		if len(vm.CallStack) > 0 {
			popped := vm.CallStack[len(vm.CallStack)-1]
			vm.CallStack = vm.CallStack[:len(vm.CallStack)-1]
			vm.PC = popped.ReturnAddress
			vm.Locals = popped.SavedLocals
		} else {
			vm.Halted = true
		}
		desc = "Return from function"

	case OpPrint:
		val := vm.pop()
		strVal := fmt.Sprintf("%v", val)
		vm.Output = append(vm.Output, strVal)
		outputVal = &strVal
		vm.PC++
		desc = fmt.Sprintf("Print %v", val)

	case OpHalt:
		vm.Halted = true
		desc = "Halt execution"

	default:
		panic(fmt.Sprintf("InvalidOpcodeError: Unknown opcode %d", instr.Opcode))
	}

	return VMTrace{
		PC:          pcBefore,
		Instruction: instr,
		StackBefore: stackBefore,
		StackAfter:  vm.copyStack(),
		Variables:   vm.copyMap(vm.Variables),
		Output:      outputVal,
		Description: desc,
	}
}

func (vm *VirtualMachine) pop() interface{} {
	if len(vm.Stack) == 0 {
		panic("StackUnderflowError")
	}
	val := vm.Stack[len(vm.Stack)-1]
	vm.Stack = vm.Stack[:len(vm.Stack)-1]
	return val
}

func (vm *VirtualMachine) add(a, b interface{}) interface{} {
	aInt, aIsInt := a.(int)
	bInt, bIsInt := b.(int)
	if aIsInt && bIsInt {
		return aInt + bInt
	}
	aStr, aIsStr := a.(string)
	bStr, bIsStr := b.(string)
	if aIsStr && bIsStr {
		return aStr + bStr
	}
	panic("TypeError: Cannot add differing formats generically without overloading in Go native structures")
}

func (vm *VirtualMachine) isFalsy(val interface{}) bool {
	if val == nil {
		return true
	}
	if v, ok := val.(int); ok && v == 0 {
		return true
	}
	if v, ok := val.(string); ok && v == "" {
		return true
	}
	return false
}

func (vm *VirtualMachine) copyStack() []interface{} {
	return vm.copyStackArr(vm.Stack)
}

func (vm *VirtualMachine) copyStackArr(arr []interface{}) []interface{} {
	c := make([]interface{}, len(arr))
	copy(c, arr)
	return c
}

func (vm *VirtualMachine) copyMap(m map[string]interface{}) map[string]interface{} {
	c := make(map[string]interface{})
	for k, v := range m {
		c[k] = v
	}
	return c
}

func AssembleCode(instructions []Instruction, constants []interface{}, names []string) CodeObject {
	if constants == nil {
		constants = []interface{}{}
	}
	if names == nil {
		names = []string{}
	}
	return CodeObject{
		Instructions: instructions,
		Constants:    constants,
		Names:        names,
	}
}
