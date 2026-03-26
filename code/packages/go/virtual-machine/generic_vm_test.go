package virtualmachine

// ════════════════════════════════════════════════════════════════════════
// generic_vm_test.go — Tests for the pluggable GenericVM
// ════════════════════════════════════════════════════════════════════════
//
// These tests verify every aspect of GenericVM: stack operations, call
// stack management, program counter control, handler registration,
// built-in functions, configuration, reset, error handling, and the
// execution loop.
//
// Each test function focuses on one logical area.  Subtests (t.Run)
// cover specific scenarios within that area.
//
// We define a small set of reusable test handlers at the top of the file
// so the tests read like executable specifications rather than
// implementation details.
//
// ════════════════════════════════════════════════════════════════════════

import (
	"fmt"
	"strings"
	"testing"
)

// ════════════════════════════════════════════════════════════════════════
// TEST HELPERS — Reusable opcode handlers for testing
// ════════════════════════════════════════════════════════════════════════
//
// These are minimal handlers that do just enough to test the VM's
// dispatch mechanism.  Real handlers would be more sophisticated.

// loadConstHandler pushes a constant from the CodeObject's Constants
// slice onto the stack.  The operand is the index into Constants.
func loadConstHandler(vm *GenericVM, instr Instruction, code CodeObject) *string {
	idx := instr.Operand.(int)
	vm.Push(code.Constants[idx])
	vm.AdvancePC()
	return nil
}

// addHandler pops two integers, pushes their sum.
func addHandler(vm *GenericVM, instr Instruction, code CodeObject) *string {
	b := vm.Pop().(int)
	a := vm.Pop().(int)
	vm.Push(a + b)
	vm.AdvancePC()
	return nil
}

// printHandler pops a value and returns it as output.
func printHandler(vm *GenericVM, instr Instruction, code CodeObject) *string {
	val := vm.Pop()
	s := fmt.Sprintf("%v", val)
	vm.AdvancePC()
	return &s
}

// haltHandler stops the VM.
func haltHandler(vm *GenericVM, instr Instruction, code CodeObject) *string {
	vm.Halted = true
	return nil
}

// storeNameHandler pops a value and stores it in Variables under
// the name found at Names[operand].
func storeNameHandler(vm *GenericVM, instr Instruction, code CodeObject) *string {
	idx := instr.Operand.(int)
	name := code.Names[idx]
	val := vm.Pop()
	vm.Variables[name] = val
	vm.AdvancePC()
	return nil
}

// loadNameHandler pushes the value of a named variable onto the stack.
func loadNameHandler(vm *GenericVM, instr Instruction, code CodeObject) *string {
	idx := instr.Operand.(int)
	name := code.Names[idx]
	vm.Push(vm.Variables[name])
	vm.AdvancePC()
	return nil
}

// jumpHandler sets the PC to the operand value (unconditional jump).
func jumpHandler(vm *GenericVM, instr Instruction, code CodeObject) *string {
	target := instr.Operand.(int)
	vm.JumpTo(target)
	return nil
}

// noopHandler does nothing except advance the PC.
func noopHandler(vm *GenericVM, instr Instruction, code CodeObject) *string {
	vm.AdvancePC()
	return nil
}

// registerStandardHandlers sets up the common handlers used by most tests.
// This avoids repetition and makes tests focus on what they're actually testing.
func registerStandardHandlers(vm *GenericVM) {
	vm.RegisterOpcode(OpLoadConst, loadConstHandler)
	vm.RegisterOpcode(OpAdd, addHandler)
	vm.RegisterOpcode(OpPrint, printHandler)
	vm.RegisterOpcode(OpHalt, haltHandler)
	vm.RegisterOpcode(OpStoreName, storeNameHandler)
	vm.RegisterOpcode(OpLoadName, loadNameHandler)
	vm.RegisterOpcode(OpJump, jumpHandler)
}

// assertPanic is a test helper that verifies a function panics with a
// message containing the expected substring.
//
// Usage:
//
//	assertPanic(t, "StackUnderflow", func() {
//	    vm.Pop()
//	})
func assertPanic(t *testing.T, expectedSubstring string, fn func()) {
	t.Helper()
	defer func() {
		r := recover()
		if r == nil {
			t.Fatalf("expected panic containing %q, but no panic occurred", expectedSubstring)
		}
		msg := fmt.Sprintf("%v", r)
		if !strings.Contains(msg, expectedSubstring) {
			t.Fatalf("expected panic containing %q, got: %q", expectedSubstring, msg)
		}
	}()
	fn()
}

// ════════════════════════════════════════════════════════════════════════
// TEST: Basic Execution
// ════════════════════════════════════════════════════════════════════════
//
// The simplest possible program: push two constants, add them, print
// the result, halt.  This exercises the full execution loop and verifies
// that handlers are called in sequence.

func TestBasicExecution(t *testing.T) {
	t.Run("push + add + halt produces correct output", func(t *testing.T) {
		vm := NewGenericVM()
		registerStandardHandlers(vm)

		// Program: push 10, push 20, add, print, halt
		// Expected output: "30"
		code := AssembleCode(
			[]Instruction{
				{Opcode: OpLoadConst, Operand: 0}, // push 10
				{Opcode: OpLoadConst, Operand: 1}, // push 20
				{Opcode: OpAdd},                   // pop 20 and 10, push 30
				{Opcode: OpPrint},                 // pop 30, output "30"
				{Opcode: OpHalt},                  // stop
			},
			[]interface{}{10, 20},
			nil,
		)

		traces := vm.Execute(code)

		// Verify output
		if len(vm.Output) != 1 {
			t.Fatalf("expected 1 output, got %d: %v", len(vm.Output), vm.Output)
		}
		if vm.Output[0] != "30" {
			t.Errorf("expected output '30', got %q", vm.Output[0])
		}

		// Verify traces
		if len(traces) != 5 {
			t.Fatalf("expected 5 traces (one per instruction), got %d", len(traces))
		}

		// Verify VM halted
		if !vm.Halted {
			t.Error("expected VM to be halted after OpHalt")
		}
	})

	t.Run("store and load variables", func(t *testing.T) {
		vm := NewGenericVM()
		registerStandardHandlers(vm)

		// Program: x = 42; print x
		code := AssembleCode(
			[]Instruction{
				{Opcode: OpLoadConst, Operand: 0},  // push 42
				{Opcode: OpStoreName, Operand: 0},   // x = 42
				{Opcode: OpLoadName, Operand: 0},    // push x
				{Opcode: OpPrint},                   // print
				{Opcode: OpHalt},
			},
			[]interface{}{42},
			[]string{"x"},
		)

		vm.Execute(code)

		if vm.Variables["x"] != 42 {
			t.Errorf("expected x=42, got x=%v", vm.Variables["x"])
		}
		if len(vm.Output) != 1 || vm.Output[0] != "42" {
			t.Errorf("expected output ['42'], got %v", vm.Output)
		}
	})
}

// ════════════════════════════════════════════════════════════════════════
// TEST: Trace Recording
// ════════════════════════════════════════════════════════════════════════
//
// Every call to Step() returns a VMTrace.  We verify that traces
// contain correct PC values, stack snapshots, and output pointers.

func TestTraceRecording(t *testing.T) {
	t.Run("traces capture PC and stack snapshots", func(t *testing.T) {
		vm := NewGenericVM()
		registerStandardHandlers(vm)

		code := AssembleCode(
			[]Instruction{
				{Opcode: OpLoadConst, Operand: 0}, // PC=0: push 5
				{Opcode: OpLoadConst, Operand: 1}, // PC=1: push 3
				{Opcode: OpAdd},                   // PC=2: add
				{Opcode: OpHalt},                  // PC=3: halt
			},
			[]interface{}{5, 3},
			nil,
		)

		traces := vm.Execute(code)

		// Trace 0: LOAD_CONST 5 at PC=0, stack was empty, becomes [5]
		if traces[0].PC != 0 {
			t.Errorf("trace[0].PC: expected 0, got %d", traces[0].PC)
		}
		if len(traces[0].StackBefore) != 0 {
			t.Errorf("trace[0].StackBefore: expected empty, got %v", traces[0].StackBefore)
		}
		if len(traces[0].StackAfter) != 1 || traces[0].StackAfter[0] != 5 {
			t.Errorf("trace[0].StackAfter: expected [5], got %v", traces[0].StackAfter)
		}

		// Trace 2: ADD at PC=2, stack was [5, 3], becomes [8]
		if traces[2].PC != 2 {
			t.Errorf("trace[2].PC: expected 2, got %d", traces[2].PC)
		}
		if len(traces[2].StackBefore) != 2 {
			t.Errorf("trace[2].StackBefore: expected 2 elements, got %d", len(traces[2].StackBefore))
		}
		if len(traces[2].StackAfter) != 1 || traces[2].StackAfter[0] != 8 {
			t.Errorf("trace[2].StackAfter: expected [8], got %v", traces[2].StackAfter)
		}
	})

	t.Run("traces capture output for print instructions", func(t *testing.T) {
		vm := NewGenericVM()
		registerStandardHandlers(vm)

		code := AssembleCode(
			[]Instruction{
				{Opcode: OpLoadConst, Operand: 0},
				{Opcode: OpPrint},
				{Opcode: OpHalt},
			},
			[]interface{}{"hello"},
			nil,
		)

		traces := vm.Execute(code)

		// Trace 0 (LOAD_CONST): no output
		if traces[0].Output != nil {
			t.Errorf("trace[0].Output: expected nil, got %v", *traces[0].Output)
		}

		// Trace 1 (PRINT): output is "hello"
		if traces[1].Output == nil {
			t.Fatal("trace[1].Output: expected non-nil")
		}
		if *traces[1].Output != "hello" {
			t.Errorf("trace[1].Output: expected 'hello', got %q", *traces[1].Output)
		}
	})

	t.Run("trace descriptions use hex opcode format", func(t *testing.T) {
		vm := NewGenericVM()
		registerStandardHandlers(vm)

		code := AssembleCode(
			[]Instruction{
				{Opcode: OpLoadConst, Operand: 0}, // OpLoadConst = 0x01
				{Opcode: OpHalt},                  // OpHalt = 0xFF
			},
			[]interface{}{1},
			nil,
		)

		traces := vm.Execute(code)

		if traces[0].Description != "Executed opcode 0x01" {
			t.Errorf("expected 'Executed opcode 0x01', got %q", traces[0].Description)
		}
		if traces[1].Description != "Executed opcode 0xff" {
			t.Errorf("expected 'Executed opcode 0xff', got %q", traces[1].Description)
		}
	})
}

// ════════════════════════════════════════════════════════════════════════
// TEST: Stack Operations
// ════════════════════════════════════════════════════════════════════════
//
// Push, Pop, and Peek are the fundamental stack operations.  We test
// normal operation and the error case (underflow).

func TestStackOperations(t *testing.T) {
	t.Run("push and pop", func(t *testing.T) {
		vm := NewGenericVM()
		vm.Push(1)
		vm.Push(2)
		vm.Push(3)

		if vm.Pop() != 3 {
			t.Error("expected 3")
		}
		if vm.Pop() != 2 {
			t.Error("expected 2")
		}
		if vm.Pop() != 1 {
			t.Error("expected 1")
		}
	})

	t.Run("peek returns top without removing", func(t *testing.T) {
		vm := NewGenericVM()
		vm.Push("hello")
		vm.Push("world")

		if vm.Peek() != "world" {
			t.Error("peek should return 'world'")
		}
		// Stack should still have 2 elements
		if len(vm.Stack) != 2 {
			t.Errorf("stack should still have 2 elements, got %d", len(vm.Stack))
		}
	})

	t.Run("pop on empty stack panics with StackUnderflowError", func(t *testing.T) {
		vm := NewGenericVM()
		assertPanic(t, "StackUnderflowError", func() {
			vm.Pop()
		})
	})

	t.Run("peek on empty stack panics with StackUnderflowError", func(t *testing.T) {
		vm := NewGenericVM()
		assertPanic(t, "StackUnderflowError", func() {
			vm.Peek()
		})
	})

	t.Run("push various types", func(t *testing.T) {
		vm := NewGenericVM()
		vm.Push(42)
		vm.Push("hello")
		vm.Push(3.14)
		vm.Push(nil)
		vm.Push(true)

		if vm.Pop() != true {
			t.Error("expected true")
		}
		if vm.Pop() != nil {
			t.Error("expected nil")
		}
		if vm.Pop() != 3.14 {
			t.Error("expected 3.14")
		}
		if vm.Pop() != "hello" {
			t.Error("expected 'hello'")
		}
		if vm.Pop() != 42 {
			t.Error("expected 42")
		}
	})
}

// ════════════════════════════════════════════════════════════════════════
// TEST: Call Stack
// ════════════════════════════════════════════════════════════════════════
//
// The call stack stores frames when functions are called.  We test
// push/pop, max recursion depth (including zero depth and unlimited).

func TestCallStack(t *testing.T) {
	t.Run("push and pop frames", func(t *testing.T) {
		vm := NewGenericVM()

		frame1 := map[string]interface{}{"returnPC": 5}
		frame2 := map[string]interface{}{"returnPC": 10}

		vm.PushFrame(frame1)
		vm.PushFrame(frame2)

		popped := vm.PopFrame()
		if popped["returnPC"] != 10 {
			t.Errorf("expected returnPC=10, got %v", popped["returnPC"])
		}

		popped = vm.PopFrame()
		if popped["returnPC"] != 5 {
			t.Errorf("expected returnPC=5, got %v", popped["returnPC"])
		}
	})

	t.Run("pop empty call stack panics", func(t *testing.T) {
		vm := NewGenericVM()
		assertPanic(t, "CallStackUnderflowError", func() {
			vm.PopFrame()
		})
	})

	t.Run("max recursion depth enforced", func(t *testing.T) {
		vm := NewGenericVM()
		depth := 3
		vm.SetMaxRecursionDepth(&depth)

		// Push 3 frames (at the limit)
		vm.PushFrame(map[string]interface{}{"level": 1})
		vm.PushFrame(map[string]interface{}{"level": 2})
		vm.PushFrame(map[string]interface{}{"level": 3})

		// Fourth push should panic
		assertPanic(t, "MaxRecursionError", func() {
			vm.PushFrame(map[string]interface{}{"level": 4})
		})
	})

	t.Run("zero depth means no calls allowed", func(t *testing.T) {
		vm := NewGenericVM()
		depth := 0
		vm.SetMaxRecursionDepth(&depth)

		// Even a single push should fail
		assertPanic(t, "MaxRecursionError", func() {
			vm.PushFrame(map[string]interface{}{"level": 1})
		})
	})

	t.Run("unlimited recursion when depth is nil", func(t *testing.T) {
		vm := NewGenericVM()
		vm.SetMaxRecursionDepth(nil) // explicitly set to nil (unlimited)

		// Should be able to push many frames without panic
		for i := 0; i < 1000; i++ {
			vm.PushFrame(map[string]interface{}{"level": i})
		}

		if len(vm.CallStack) != 1000 {
			t.Errorf("expected 1000 frames, got %d", len(vm.CallStack))
		}
	})
}

// ════════════════════════════════════════════════════════════════════════
// TEST: Program Counter
// ════════════════════════════════════════════════════════════════════════
//
// AdvancePC and JumpTo control the flow of execution.

func TestProgramCounter(t *testing.T) {
	t.Run("advance PC increments by 1", func(t *testing.T) {
		vm := NewGenericVM()
		if vm.PC != 0 {
			t.Fatalf("initial PC should be 0, got %d", vm.PC)
		}

		vm.AdvancePC()
		if vm.PC != 1 {
			t.Errorf("after AdvancePC: expected PC=1, got %d", vm.PC)
		}

		vm.AdvancePC()
		if vm.PC != 2 {
			t.Errorf("after second AdvancePC: expected PC=2, got %d", vm.PC)
		}
	})

	t.Run("jump to sets PC to target", func(t *testing.T) {
		vm := NewGenericVM()
		vm.JumpTo(42)
		if vm.PC != 42 {
			t.Errorf("after JumpTo(42): expected PC=42, got %d", vm.PC)
		}

		vm.JumpTo(0)
		if vm.PC != 0 {
			t.Errorf("after JumpTo(0): expected PC=0, got %d", vm.PC)
		}
	})

	t.Run("jump handler changes execution flow", func(t *testing.T) {
		vm := NewGenericVM()
		registerStandardHandlers(vm)

		// Program: push 1, jump to PC=3 (skip push 2), push 3, add, print, halt
		// Without jump: 1+2 = 3.  With jump: 1+3 = 4.
		code := AssembleCode(
			[]Instruction{
				{Opcode: OpLoadConst, Operand: 0}, // PC=0: push 1
				{Opcode: OpJump, Operand: 3},      // PC=1: jump to PC=3
				{Opcode: OpLoadConst, Operand: 1}, // PC=2: push 2 (SKIPPED)
				{Opcode: OpLoadConst, Operand: 2}, // PC=3: push 3
				{Opcode: OpAdd},                   // PC=4: add
				{Opcode: OpPrint},                 // PC=5: print
				{Opcode: OpHalt},                  // PC=6: halt
			},
			[]interface{}{1, 2, 3},
			nil,
		)

		vm.Execute(code)

		if len(vm.Output) != 1 || vm.Output[0] != "4" {
			t.Errorf("expected output ['4'], got %v", vm.Output)
		}
	})
}

// ════════════════════════════════════════════════════════════════════════
// TEST: Built-in Functions
// ════════════════════════════════════════════════════════════════════════
//
// Built-in functions are registered by name and retrieved by handlers.

func TestBuiltinFunctions(t *testing.T) {
	t.Run("register and retrieve builtin", func(t *testing.T) {
		vm := NewGenericVM()
		vm.RegisterBuiltin("double", func(args ...interface{}) interface{} {
			return args[0].(int) * 2
		})

		builtin := vm.GetBuiltin("double")
		if builtin == nil {
			t.Fatal("expected non-nil builtin")
		}
		if builtin.Name != "double" {
			t.Errorf("expected name 'double', got %q", builtin.Name)
		}

		result := builtin.Implementation(21)
		if result != 42 {
			t.Errorf("expected 42, got %v", result)
		}
	})

	t.Run("get nonexistent builtin returns nil", func(t *testing.T) {
		vm := NewGenericVM()
		if vm.GetBuiltin("nonexistent") != nil {
			t.Error("expected nil for nonexistent builtin")
		}
	})

	t.Run("builtin used from opcode handler", func(t *testing.T) {
		vm := NewGenericVM()

		// Register a "double" builtin
		vm.RegisterBuiltin("double", func(args ...interface{}) interface{} {
			return args[0].(int) * 2
		})

		// Register a custom opcode that calls the "double" builtin.
		// We use OpDup (0x03) as our custom opcode for this test.
		vm.RegisterOpcode(OpDup, func(vm *GenericVM, instr Instruction, code CodeObject) *string {
			val := vm.Pop()
			builtin := vm.GetBuiltin("double")
			result := builtin.Implementation(val)
			vm.Push(result)
			vm.AdvancePC()
			return nil
		})
		vm.RegisterOpcode(OpLoadConst, loadConstHandler)
		vm.RegisterOpcode(OpPrint, printHandler)
		vm.RegisterOpcode(OpHalt, haltHandler)

		code := AssembleCode(
			[]Instruction{
				{Opcode: OpLoadConst, Operand: 0}, // push 5
				{Opcode: OpDup},                   // double it -> 10
				{Opcode: OpPrint},                 // print
				{Opcode: OpHalt},
			},
			[]interface{}{5},
			nil,
		)

		vm.Execute(code)

		if len(vm.Output) != 1 || vm.Output[0] != "10" {
			t.Errorf("expected output ['10'], got %v", vm.Output)
		}
	})

	t.Run("multiple builtins", func(t *testing.T) {
		vm := NewGenericVM()
		vm.RegisterBuiltin("add", func(args ...interface{}) interface{} {
			return args[0].(int) + args[1].(int)
		})
		vm.RegisterBuiltin("negate", func(args ...interface{}) interface{} {
			return -args[0].(int)
		})

		addBuiltin := vm.GetBuiltin("add")
		negBuiltin := vm.GetBuiltin("negate")

		if addBuiltin.Implementation(3, 4) != 7 {
			t.Error("add builtin failed")
		}
		if negBuiltin.Implementation(5) != -5 {
			t.Error("negate builtin failed")
		}
	})
}

// ════════════════════════════════════════════════════════════════════════
// TEST: InjectGlobals
// ════════════════════════════════════════════════════════════════════════
//
// InjectGlobals pre-seeds variables into the VM's global scope before
// execution begins.  This is the mechanism for passing build context
// (like _ctx with OS info) into Starlark programs.

func TestInjectGlobals(t *testing.T) {
	t.Run("injected globals are accessible as variables", func(t *testing.T) {
		vm := NewGenericVM()
		registerStandardHandlers(vm)

		vm.InjectGlobals(map[string]interface{}{
			"greeting": "hello",
		})

		// LOAD_NAME "greeting" should find the injected value
		code := AssembleCode(
			[]Instruction{
				{Opcode: OpLoadName, Operand: 0}, // load "greeting"
				{Opcode: OpPrint},
				{Opcode: OpHalt},
			},
			nil,
			[]string{"greeting"},
		)

		vm.Execute(code)

		if len(vm.Output) != 1 || vm.Output[0] != "hello" {
			t.Errorf("expected output ['hello'], got %v", vm.Output)
		}
	})

	t.Run("injected dict is accessible via subscript", func(t *testing.T) {
		vm := NewGenericVM()

		// Inject a nested dict (like _ctx)
		vm.InjectGlobals(map[string]interface{}{
			"_ctx": map[string]interface{}{
				"os":   "darwin",
				"arch": "arm64",
			},
		})

		// Verify the injected dict exists and has the right structure
		ctx, ok := vm.Variables["_ctx"]
		if !ok {
			t.Fatal("expected _ctx to be in Variables")
		}
		ctxMap, ok := ctx.(map[string]interface{})
		if !ok {
			t.Fatal("expected _ctx to be a map")
		}
		if ctxMap["os"] != "darwin" {
			t.Errorf("expected os=darwin, got %v", ctxMap["os"])
		}
		if ctxMap["arch"] != "arm64" {
			t.Errorf("expected arch=arm64, got %v", ctxMap["arch"])
		}
	})

	t.Run("inject overwrites existing variable", func(t *testing.T) {
		vm := NewGenericVM()
		vm.Variables["x"] = 1

		vm.InjectGlobals(map[string]interface{}{
			"x": 2,
		})

		if vm.Variables["x"] != 2 {
			t.Errorf("expected x=2 after overwrite, got %v", vm.Variables["x"])
		}
	})

	t.Run("inject does not remove unrelated variables", func(t *testing.T) {
		vm := NewGenericVM()
		vm.Variables["existing"] = "keep me"

		vm.InjectGlobals(map[string]interface{}{
			"new_var": "added",
		})

		if vm.Variables["existing"] != "keep me" {
			t.Error("existing variable should not be removed by InjectGlobals")
		}
		if vm.Variables["new_var"] != "added" {
			t.Error("new variable should be added by InjectGlobals")
		}
	})

	t.Run("inject nil globals is a no-op", func(t *testing.T) {
		vm := NewGenericVM()
		vm.Variables["x"] = 1

		vm.InjectGlobals(nil)

		if vm.Variables["x"] != 1 {
			t.Error("nil globals should not affect existing variables")
		}
	})

	t.Run("inject multiple globals at once", func(t *testing.T) {
		vm := NewGenericVM()

		vm.InjectGlobals(map[string]interface{}{
			"a": 1,
			"b": "two",
			"c": true,
		})

		if vm.Variables["a"] != 1 || vm.Variables["b"] != "two" || vm.Variables["c"] != true {
			t.Errorf("expected a=1, b=two, c=true, got %v", vm.Variables)
		}
	})
}

// ════════════════════════════════════════════════════════════════════════
// TEST: Configuration (Frozen, Max Recursion Depth)
// ════════════════════════════════════════════════════════════════════════

func TestConfiguration(t *testing.T) {
	t.Run("frozen VM rejects opcode registration", func(t *testing.T) {
		vm := NewGenericVM()
		vm.SetFrozen(true)

		if !vm.IsFrozen() {
			t.Error("expected IsFrozen() = true")
		}

		assertPanic(t, "FrozenVMError", func() {
			vm.RegisterOpcode(OpAdd, addHandler)
		})
	})

	t.Run("frozen VM rejects builtin registration", func(t *testing.T) {
		vm := NewGenericVM()
		vm.SetFrozen(true)

		assertPanic(t, "FrozenVMError", func() {
			vm.RegisterBuiltin("foo", func(args ...interface{}) interface{} { return nil })
		})
	})

	t.Run("unfreezing allows registration again", func(t *testing.T) {
		vm := NewGenericVM()
		vm.SetFrozen(true)
		vm.SetFrozen(false)

		if vm.IsFrozen() {
			t.Error("expected IsFrozen() = false after unfreezing")
		}

		// Should not panic
		vm.RegisterOpcode(OpAdd, addHandler)
		vm.RegisterBuiltin("test", func(args ...interface{}) interface{} { return nil })
	})

	t.Run("max recursion depth getter/setter", func(t *testing.T) {
		vm := NewGenericVM()

		// Default is nil (unlimited)
		if vm.MaxRecursionDepth() != nil {
			t.Error("default max recursion depth should be nil")
		}

		depth := 10
		vm.SetMaxRecursionDepth(&depth)
		got := vm.MaxRecursionDepth()
		if got == nil || *got != 10 {
			t.Errorf("expected 10, got %v", got)
		}

		// Reset to unlimited
		vm.SetMaxRecursionDepth(nil)
		if vm.MaxRecursionDepth() != nil {
			t.Error("expected nil after reset")
		}
	})
}

// ════════════════════════════════════════════════════════════════════════
// TEST: Reset
// ════════════════════════════════════════════════════════════════════════
//
// Reset clears runtime state but preserves handlers and configuration.

func TestReset(t *testing.T) {
	t.Run("reset clears runtime state", func(t *testing.T) {
		vm := NewGenericVM()
		registerStandardHandlers(vm)

		// Run a program to populate state
		code := AssembleCode(
			[]Instruction{
				{Opcode: OpLoadConst, Operand: 0},
				{Opcode: OpStoreName, Operand: 0},
				{Opcode: OpLoadConst, Operand: 1},
				{Opcode: OpPrint},
				{Opcode: OpHalt},
			},
			[]interface{}{99, "printed"},
			[]string{"myvar"},
		)
		vm.Execute(code)

		// Verify state is populated
		if len(vm.Output) == 0 {
			t.Fatal("expected output before reset")
		}
		if !vm.Halted {
			t.Fatal("expected halted before reset")
		}

		// Reset
		vm.Reset()

		// Verify state is cleared
		if len(vm.Stack) != 0 {
			t.Error("stack should be empty after reset")
		}
		if len(vm.Variables) != 0 {
			t.Error("variables should be empty after reset")
		}
		if len(vm.Locals) != 0 {
			t.Error("locals should be empty after reset")
		}
		if vm.PC != 0 {
			t.Error("PC should be 0 after reset")
		}
		if vm.Halted {
			t.Error("halted should be false after reset")
		}
		if len(vm.Output) != 0 {
			t.Error("output should be empty after reset")
		}
		if len(vm.CallStack) != 0 {
			t.Error("call stack should be empty after reset")
		}
	})

	t.Run("reset preserves handlers", func(t *testing.T) {
		vm := NewGenericVM()
		registerStandardHandlers(vm)

		vm.Execute(AssembleCode(
			[]Instruction{
				{Opcode: OpLoadConst, Operand: 0},
				{Opcode: OpPrint},
				{Opcode: OpHalt},
			},
			[]interface{}{"first run"},
			nil,
		))

		vm.Reset()

		// Should be able to run another program with the same handlers
		vm.Execute(AssembleCode(
			[]Instruction{
				{Opcode: OpLoadConst, Operand: 0},
				{Opcode: OpPrint},
				{Opcode: OpHalt},
			},
			[]interface{}{"second run"},
			nil,
		))

		if len(vm.Output) != 1 || vm.Output[0] != "second run" {
			t.Errorf("expected ['second run'], got %v", vm.Output)
		}
	})

	t.Run("reset preserves builtins and configuration", func(t *testing.T) {
		vm := NewGenericVM()
		vm.RegisterBuiltin("test", func(args ...interface{}) interface{} { return "ok" })
		depth := 5
		vm.SetMaxRecursionDepth(&depth)
		vm.SetFrozen(true)

		vm.Reset()

		// Builtins preserved
		if vm.GetBuiltin("test") == nil {
			t.Error("builtin should be preserved after reset")
		}

		// Config preserved
		if vm.MaxRecursionDepth() == nil || *vm.MaxRecursionDepth() != 5 {
			t.Error("max recursion depth should be preserved after reset")
		}
		if !vm.IsFrozen() {
			t.Error("frozen state should be preserved after reset")
		}
	})
}

// ════════════════════════════════════════════════════════════════════════
// TEST: Error Handling
// ════════════════════════════════════════════════════════════════════════
//
// The VM should panic with clear error messages for invalid states.

func TestErrorHandling(t *testing.T) {
	t.Run("unknown opcode panics with InvalidOpcodeError", func(t *testing.T) {
		vm := NewGenericVM()
		// Register only HALT, not LOAD_CONST
		vm.RegisterOpcode(OpHalt, haltHandler)

		code := AssembleCode(
			[]Instruction{
				{Opcode: OpLoadConst, Operand: 0}, // no handler for this!
			},
			[]interface{}{1},
			nil,
		)

		assertPanic(t, "InvalidOpcodeError", func() {
			vm.Execute(code)
		})
	})

	t.Run("error message includes hex opcode", func(t *testing.T) {
		vm := NewGenericVM()

		code := AssembleCode(
			[]Instruction{
				{Opcode: OpCode(0xAB)}, // unregistered opcode
			},
			nil,
			nil,
		)

		assertPanic(t, "0xab", func() {
			vm.Execute(code)
		})
	})

	t.Run("no handlers registered at all", func(t *testing.T) {
		vm := NewGenericVM()

		code := AssembleCode(
			[]Instruction{
				{Opcode: OpHalt},
			},
			nil,
			nil,
		)

		assertPanic(t, "InvalidOpcodeError", func() {
			vm.Execute(code)
		})
	})
}

// ════════════════════════════════════════════════════════════════════════
// TEST: Step-by-Step Execution
// ════════════════════════════════════════════════════════════════════════
//
// Step() lets you execute one instruction at a time, which is useful
// for debuggers and interactive exploration.

func TestStepByStepExecution(t *testing.T) {
	t.Run("manual stepping produces same result as Execute", func(t *testing.T) {
		vm := NewGenericVM()
		registerStandardHandlers(vm)

		code := AssembleCode(
			[]Instruction{
				{Opcode: OpLoadConst, Operand: 0}, // push 7
				{Opcode: OpLoadConst, Operand: 1}, // push 3
				{Opcode: OpAdd},                   // 7+3 = 10
				{Opcode: OpPrint},                 // print 10
				{Opcode: OpHalt},
			},
			[]interface{}{7, 3},
			nil,
		)

		var traces []VMTrace
		for !vm.Halted && vm.PC < len(code.Instructions) {
			trace := vm.Step(code)
			traces = append(traces, trace)
		}

		if len(traces) != 5 {
			t.Fatalf("expected 5 traces, got %d", len(traces))
		}
		if len(vm.Output) != 1 || vm.Output[0] != "10" {
			t.Errorf("expected output ['10'], got %v", vm.Output)
		}
	})

	t.Run("step returns correct trace for each instruction", func(t *testing.T) {
		vm := NewGenericVM()
		registerStandardHandlers(vm)

		code := AssembleCode(
			[]Instruction{
				{Opcode: OpLoadConst, Operand: 0},
				{Opcode: OpHalt},
			},
			[]interface{}{99},
			nil,
		)

		// Step 1: LOAD_CONST
		trace1 := vm.Step(code)
		if trace1.PC != 0 {
			t.Errorf("expected PC=0, got %d", trace1.PC)
		}
		if trace1.Instruction.Opcode != OpLoadConst {
			t.Errorf("expected OpLoadConst, got %v", trace1.Instruction.Opcode)
		}
		if len(trace1.StackAfter) != 1 || trace1.StackAfter[0] != 99 {
			t.Errorf("expected stack [99], got %v", trace1.StackAfter)
		}

		// Step 2: HALT
		trace2 := vm.Step(code)
		if trace2.PC != 1 {
			t.Errorf("expected PC=1, got %d", trace2.PC)
		}
		if !vm.Halted {
			t.Error("expected halted after OpHalt step")
		}
	})
}

// ════════════════════════════════════════════════════════════════════════
// TEST: Program Ends Without Halt
// ════════════════════════════════════════════════════════════════════════
//
// A program doesn't have to end with HALT.  If the PC moves past the
// last instruction, Execute() should stop gracefully.

func TestProgramEndsWithoutHalt(t *testing.T) {
	t.Run("execution stops when PC exceeds instruction count", func(t *testing.T) {
		vm := NewGenericVM()
		registerStandardHandlers(vm)

		// Program with no HALT — just push and print
		code := AssembleCode(
			[]Instruction{
				{Opcode: OpLoadConst, Operand: 0},
				{Opcode: OpPrint},
				// No HALT instruction!
			},
			[]interface{}{"no halt"},
			nil,
		)

		traces := vm.Execute(code)

		if len(traces) != 2 {
			t.Fatalf("expected 2 traces, got %d", len(traces))
		}
		if vm.Halted {
			t.Error("VM should NOT be halted (no halt instruction was executed)")
		}
		if len(vm.Output) != 1 || vm.Output[0] != "no halt" {
			t.Errorf("expected output ['no halt'], got %v", vm.Output)
		}
	})

	t.Run("empty program produces no traces", func(t *testing.T) {
		vm := NewGenericVM()
		registerStandardHandlers(vm)

		code := AssembleCode([]Instruction{}, nil, nil)
		traces := vm.Execute(code)

		if len(traces) != 0 {
			t.Errorf("expected 0 traces for empty program, got %d", len(traces))
		}
	})
}

// ════════════════════════════════════════════════════════════════════════
// TEST: Handler Overwrite
// ════════════════════════════════════════════════════════════════════════
//
// Registering the same opcode twice should silently replace the handler.

func TestHandlerOverwrite(t *testing.T) {
	t.Run("second registration replaces first handler", func(t *testing.T) {
		vm := NewGenericVM()
		vm.RegisterOpcode(OpHalt, haltHandler)

		// First LOAD_CONST: pushes the constant normally
		vm.RegisterOpcode(OpLoadConst, loadConstHandler)

		// Override: new handler pushes constant * 10
		vm.RegisterOpcode(OpLoadConst, func(vm *GenericVM, instr Instruction, code CodeObject) *string {
			idx := instr.Operand.(int)
			val := code.Constants[idx].(int) * 10
			vm.Push(val)
			vm.AdvancePC()
			return nil
		})
		vm.RegisterOpcode(OpPrint, printHandler)

		code := AssembleCode(
			[]Instruction{
				{Opcode: OpLoadConst, Operand: 0}, // should push 50, not 5
				{Opcode: OpPrint},
				{Opcode: OpHalt},
			},
			[]interface{}{5},
			nil,
		)

		vm.Execute(code)

		if len(vm.Output) != 1 || vm.Output[0] != "50" {
			t.Errorf("expected output ['50'] (overridden handler), got %v", vm.Output)
		}
	})
}

// ════════════════════════════════════════════════════════════════════════
// TEST: Variables Snapshot in Trace
// ════════════════════════════════════════════════════════════════════════
//
// The trace should contain a copy of variables at the time the
// instruction executed, not a reference that changes later.

func TestVariablesSnapshot(t *testing.T) {
	t.Run("trace variables are independent copies", func(t *testing.T) {
		vm := NewGenericVM()
		registerStandardHandlers(vm)

		code := AssembleCode(
			[]Instruction{
				{Opcode: OpLoadConst, Operand: 0},  // push 1
				{Opcode: OpStoreName, Operand: 0},   // x = 1
				{Opcode: OpLoadConst, Operand: 1},  // push 2
				{Opcode: OpStoreName, Operand: 0},   // x = 2
				{Opcode: OpHalt},
			},
			[]interface{}{1, 2},
			[]string{"x"},
		)

		traces := vm.Execute(code)

		// After first STORE_NAME (trace[1]), x should be 1
		if traces[1].Variables["x"] != 1 {
			t.Errorf("after first store, expected x=1, got x=%v", traces[1].Variables["x"])
		}
		// After second STORE_NAME (trace[3]), x should be 2
		if traces[3].Variables["x"] != 2 {
			t.Errorf("after second store, expected x=2, got x=%v", traces[3].Variables["x"])
		}
	})
}
