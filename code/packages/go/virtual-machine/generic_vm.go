// generic_vm.go — A pluggable, handler-based stack machine.
//
// ════════════════════════════════════════════════════════════════════════
// WHY A GENERIC VM?
// ════════════════════════════════════════════════════════════════════════
//
// The original VirtualMachine (vm.go) hard-codes every opcode in a giant
// switch statement.  That's fine for one language — but what if you want
// the *same* bytecode interpreter core to run Python bytecode, Ruby
// bytecode, or some future language you invent next weekend?
//
// GenericVM solves this with the *handler pattern*:
//
//   1. The VM itself knows NOTHING about individual opcodes.
//   2. Each opcode is registered as a callback: OpCode → OpcodeHandler.
//   3. At execution time, the VM fetches the instruction, looks up the
//      handler in a map, and calls it.  That's it.
//
// Think of it like a telephone switchboard.  The VM is the operator who
// connects calls, but doesn't know what the callers are saying.  The
// handlers are the callers who do the actual work.
//
// This separation makes it trivial to:
//   - Add new opcodes without touching the VM core.
//   - Swap opcode sets to emulate different bytecode formats.
//   - Test individual opcodes in isolation.
//   - Freeze a VM configuration so no more opcodes can be registered
//     (useful for sandboxing).
//
// ════════════════════════════════════════════════════════════════════════
// ARCHITECTURE DIAGRAM
// ════════════════════════════════════════════════════════════════════════
//
//   ┌──────────────────────────────────────────────┐
//   │                  GenericVM                    │
//   │                                              │
//   │  ┌──────────┐   ┌───────────┐   ┌────────┐  │
//   │  │  Stack    │   │ Variables │   │ Locals │  │
//   │  │ (LIFO)   │   │ (map)     │   │ (slot) │  │
//   │  └──────────┘   └───────────┘   └────────┘  │
//   │                                              │
//   │  ┌──────────────────────────────────────┐    │
//   │  │  Handler Table                       │    │
//   │  │  0x01 → LoadConst handler            │    │
//   │  │  0x20 → Add handler                  │    │
//   │  │  0x60 → Print handler                │    │
//   │  │  0xFF → Halt handler                 │    │
//   │  │  ...                                 │    │
//   │  └──────────────────────────────────────┘    │
//   │                                              │
//   │  ┌──────────────────────────────────────┐    │
//   │  │  Builtins Table                      │    │
//   │  │  "len"  → length function            │    │
//   │  │  "abs"  → absolute value function    │    │
//   │  │  ...                                 │    │
//   │  └──────────────────────────────────────┘    │
//   │                                              │
//   │  PC ──▶ fetch instruction ──▶ lookup handler │
//   │         ──▶ call handler ──▶ record trace    │
//   └──────────────────────────────────────────────┘
//
// ════════════════════════════════════════════════════════════════════════
// THE EXECUTION CYCLE (Fetch–Decode–Execute)
// ════════════════════════════════════════════════════════════════════════
//
// Every real CPU and every virtual machine follows the same basic loop:
//
//   1. FETCH:   Read the instruction at the current Program Counter (PC).
//   2. DECODE:  Figure out what the instruction means (look up handler).
//   3. EXECUTE: Do the work (the handler modifies VM state).
//   4. REPEAT:  Go back to step 1 unless halted.
//
// The GenericVM's Execute() method is exactly this loop.  The Step()
// method performs one iteration of fetch-decode-execute and returns a
// VMTrace so you can inspect what happened.
//
// ════════════════════════════════════════════════════════════════════════
// CALL STACK AND RECURSION
// ════════════════════════════════════════════════════════════════════════
//
// When a function calls another function, the VM needs to remember
// where to return to.  This is the call stack — a stack of "frames",
// each holding the saved state of the caller.
//
// GenericVM uses a simple []map[string]interface{} for frames.  Each
// frame is just a bag of key-value pairs — the handler decides what to
// save and restore.  This keeps the VM generic.
//
// To prevent infinite recursion (e.g., a function that calls itself
// forever), you can set MaxRecursionDepth.  If the call stack exceeds
// this depth, PushFrame panics with "MaxRecursionError".
//
//   depth = nil   → unlimited recursion (default)
//   depth = 0     → no function calls allowed at all
//   depth = 100   → up to 100 nested calls
//
package virtualmachine

import (
	"fmt"
)

// ════════════════════════════════════════════════════════════════════════
// TYPE DEFINITIONS
// ════════════════════════════════════════════════════════════════════════

// OpcodeHandler is a function that the GenericVM calls when it encounters
// a registered opcode.  The handler receives:
//   - vm:    the GenericVM itself, so the handler can push/pop/jump/etc.
//   - instr: the current instruction (opcode + operand).
//   - code:  the full CodeObject, in case the handler needs constants or names.
//
// The handler returns a *string.  If non-nil, that string is treated as
// output (like a print statement) and recorded in the VMTrace.  Most
// handlers return nil.
//
// Example handler for a hypothetical "NOP" opcode:
//
//	func nopHandler(vm *GenericVM, instr Instruction, code CodeObject) *string {
//	    vm.AdvancePC()  // move to next instruction
//	    return nil      // no output
//	}
type OpcodeHandler func(vm *GenericVM, instr Instruction, code CodeObject) *string

// BuiltinFunction represents a built-in callable that opcode handlers
// can look up by name.  Think of these as the "standard library" of
// your VM — functions like len(), abs(), print() that are always available.
//
// The Implementation field takes variadic arguments and returns a single
// value, keeping the interface maximally flexible.
type BuiltinFunction struct {
	Name           string
	Implementation func(args ...interface{}) interface{}
}

// GenericVM is a pluggable stack-based bytecode interpreter.
//
// Unlike VirtualMachine (which hard-codes opcodes in a switch), GenericVM
// dispatches every opcode through a registered handler function.  This
// makes it language-agnostic: register Python opcodes for Python,
// Ruby opcodes for Ruby, or mix and match.
//
// Fields:
//   - Stack:     the operand stack (LIFO).  Handlers push/pop values here.
//   - Variables: named variables (like global scope).
//   - Locals:    slot-indexed local variables (like function locals).
//   - PC:        program counter — index of the next instruction to execute.
//   - Halted:    when true, the execution loop stops.
//   - Output:    accumulated print output.
//   - CallStack: saved frames for function call/return.
//
// Private fields:
//   - handlers:          opcode → handler mapping.
//   - builtins:          name → BuiltinFunction mapping.
//   - maxRecursionDepth: nil means unlimited; otherwise, max call stack depth.
//   - frozen:            if true, RegisterOpcode and RegisterBuiltin panic.
type GenericVM struct {
	Stack     []interface{}
	Variables map[string]interface{}
	Locals    []interface{}
	PC        int
	Halted    bool
	Output    []string
	CallStack []map[string]interface{}

	// TypedStack holds typed values for statically-typed languages like WASM.
	// Each value carries a type tag (e.g., i32=0x7F) and a payload.
	// WASM handlers use PushTyped/PopTyped/PeekTyped instead of Push/Pop/Peek.
	TypedStack []TypedVMValue

	// ExecutionContext is an opaque context object passed to ContextOpcodeHandlers
	// during ExecuteWithContext.  For WASM, this is a *WasmExecutionContext.
	ExecutionContext interface{}

	handlers          map[OpCode]OpcodeHandler
	contextHandlers   map[OpCode]ContextOpcodeHandler
	builtins          map[string]BuiltinFunction
	maxRecursionDepth *int
	frozen            bool

	// Instruction hooks — called before/after each instruction dispatch.
	preInstructionHook  func(vm *GenericVM, instr *Instruction, code CodeObject)
	postInstructionHook func(vm *GenericVM, instr Instruction, code CodeObject)
}

// ════════════════════════════════════════════════════════════════════════
// CONSTRUCTOR
// ════════════════════════════════════════════════════════════════════════

// NewGenericVM creates a fresh GenericVM with empty state and no
// registered handlers.  You must register opcode handlers before
// executing any code.
//
// Usage:
//
//	vm := NewGenericVM()
//	vm.RegisterOpcode(OpLoadConst, myLoadConstHandler)
//	vm.RegisterOpcode(OpHalt, myHaltHandler)
//	traces := vm.Execute(code)
func NewGenericVM() *GenericVM {
	result, _ := StartNew[*GenericVM]("virtual-machine.NewGenericVM", nil,
		func(op *Operation[*GenericVM], rf *ResultFactory[*GenericVM]) *OperationResult[*GenericVM] {
			vm := &GenericVM{
				Stack:           []interface{}{},
				Variables:       make(map[string]interface{}),
				Locals:          []interface{}{},
				PC:              0,
				Halted:          false,
				Output:          []string{},
				CallStack:       []map[string]interface{}{},
				TypedStack:      []TypedVMValue{},
				handlers:        make(map[OpCode]OpcodeHandler),
				contextHandlers: make(map[OpCode]ContextOpcodeHandler),
				builtins:        make(map[string]BuiltinFunction),
			}
			return rf.Generate(true, false, vm)
		}).GetResult()
	return result
}

// ════════════════════════════════════════════════════════════════════════
// REGISTRATION — Adding opcodes and builtins
// ════════════════════════════════════════════════════════════════════════

// RegisterOpcode binds an opcode to a handler function.  When the VM
// encounters this opcode during execution, it will call the handler.
//
// If the VM is frozen, this panics with "FrozenVMError".
// Registering the same opcode twice silently overwrites the previous handler.
//
// Example:
//
//	vm.RegisterOpcode(OpAdd, func(vm *GenericVM, instr Instruction, code CodeObject) *string {
//	    b := vm.Pop().(int)
//	    a := vm.Pop().(int)
//	    vm.Push(a + b)
//	    vm.AdvancePC()
//	    return nil
//	})
func (vm *GenericVM) RegisterOpcode(opcode OpCode, handler OpcodeHandler) {
	_, _ = StartNew[struct{}]("virtual-machine.RegisterOpcode", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("opcode", opcode)
			if vm.frozen {
				panic("FrozenVMError: cannot register opcodes on a frozen VM")
			}
			vm.handlers[opcode] = handler
			return rf.Generate(true, false, struct{}{})
		}).PanicOnUnexpected().GetResult()
}

// RegisterBuiltin adds a named built-in function that opcode handlers
// can retrieve via GetBuiltin.  Panics if the VM is frozen.
//
// Example:
//
//	vm.RegisterBuiltin("len", func(args ...interface{}) interface{} {
//	    s := args[0].(string)
//	    return len(s)
//	})
func (vm *GenericVM) RegisterBuiltin(name string, impl func(args ...interface{}) interface{}) {
	_, _ = StartNew[struct{}]("virtual-machine.RegisterBuiltin", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("name", name)
			if vm.frozen {
				panic("FrozenVMError: cannot register builtins on a frozen VM")
			}
			vm.builtins[name] = BuiltinFunction{
				Name:           name,
				Implementation: impl,
			}
			return rf.Generate(true, false, struct{}{})
		}).PanicOnUnexpected().GetResult()
}

// GetBuiltin retrieves a registered built-in function by name.
// Returns nil if no such built-in exists.
//
// This is typically called from within an opcode handler when it needs
// to invoke a built-in:
//
//	builtin := vm.GetBuiltin("len")
//	if builtin != nil {
//	    result := builtin.Implementation("hello")
//	}
func (vm *GenericVM) GetBuiltin(name string) *BuiltinFunction {
	result, _ := StartNew[*BuiltinFunction]("virtual-machine.GetBuiltin", nil,
		func(op *Operation[*BuiltinFunction], rf *ResultFactory[*BuiltinFunction]) *OperationResult[*BuiltinFunction] {
			op.AddProperty("name", name)
			b, ok := vm.builtins[name]
			if !ok {
				return rf.Generate(true, false, nil)
			}
			return rf.Generate(true, false, &b)
		}).GetResult()
	return result
}

// ════════════════════════════════════════════════════════════════════════
// STACK OPERATIONS
// ════════════════════════════════════════════════════════════════════════
//
// The stack is the heart of a stack-based VM.  Almost every operation
// works by pushing values onto the stack or popping them off.
//
// Analogy: think of a stack of plates in a cafeteria.
//   - Push = put a plate on top.
//   - Pop  = take the top plate off.
//   - Peek = look at the top plate without removing it.
//
// If you try to Pop or Peek when the stack is empty, that's an error
// (you can't take a plate from an empty stack), so we panic.

// Push places a value on top of the operand stack.
func (vm *GenericVM) Push(value interface{}) {
	_, _ = StartNew[struct{}]("virtual-machine.Push", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			vm.Stack = append(vm.Stack, value)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Pop removes and returns the top value from the operand stack.
// Panics with "StackUnderflowError" if the stack is empty.
func (vm *GenericVM) Pop() interface{} {
	result, _ := StartNew[interface{}]("virtual-machine.Pop", nil,
		func(op *Operation[interface{}], rf *ResultFactory[interface{}]) *OperationResult[interface{}] {
			if len(vm.Stack) == 0 {
				panic("StackUnderflowError")
			}
			val := vm.Stack[len(vm.Stack)-1]
			vm.Stack = vm.Stack[:len(vm.Stack)-1]
			return rf.Generate(true, false, val)
		}).PanicOnUnexpected().GetResult()
	return result
}

// Peek returns the top value from the operand stack WITHOUT removing it.
// Panics with "StackUnderflowError" if the stack is empty.
//
// This is useful when a handler needs to inspect the top of stack
// without consuming it — for example, a DUP instruction.
func (vm *GenericVM) Peek() interface{} {
	result, _ := StartNew[interface{}]("virtual-machine.Peek", nil,
		func(op *Operation[interface{}], rf *ResultFactory[interface{}]) *OperationResult[interface{}] {
			if len(vm.Stack) == 0 {
				panic("StackUnderflowError")
			}
			return rf.Generate(true, false, vm.Stack[len(vm.Stack)-1])
		}).PanicOnUnexpected().GetResult()
	return result
}

// ════════════════════════════════════════════════════════════════════════
// CALL STACK OPERATIONS
// ════════════════════════════════════════════════════════════════════════
//
// The call stack tracks function invocations.  Each "frame" is a
// map[string]interface{} — a generic bag of saved state.  The handler
// for CALL decides what to put in the frame (return address, saved
// variables, etc.) and the handler for RETURN decides how to restore it.
//
// Why a map instead of a struct?  Because different languages save
// different things.  Python saves locals + globals.  Ruby saves self +
// binding.  A map lets each language's handlers store whatever they need.

// PushFrame saves a frame onto the call stack.
//
// If maxRecursionDepth is set and the call stack would exceed that
// depth, PushFrame panics with "MaxRecursionError".  This protects
// against infinite recursion.
//
// Truth table for recursion depth checking:
//
//	maxRecursionDepth | len(CallStack) | Action
//	──────────────────┼────────────────┼────────────────
//	nil               | any            | allow (unlimited)
//	ptr to 0          | 0              | panic (no calls allowed)
//	ptr to 3          | 2              | allow (2 < 3)
//	ptr to 3          | 3              | panic (3 >= 3)
func (vm *GenericVM) PushFrame(frame map[string]interface{}) {
	_, _ = StartNew[struct{}]("virtual-machine.PushFrame", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			if vm.maxRecursionDepth != nil {
				if len(vm.CallStack) >= *vm.maxRecursionDepth {
					panic("MaxRecursionError")
				}
			}
			vm.CallStack = append(vm.CallStack, frame)
			return rf.Generate(true, false, struct{}{})
		}).PanicOnUnexpected().GetResult()
}

// PopFrame removes and returns the top frame from the call stack.
// Panics with "CallStackUnderflowError" if the call stack is empty.
func (vm *GenericVM) PopFrame() map[string]interface{} {
	result, _ := StartNew[map[string]interface{}]("virtual-machine.PopFrame", nil,
		func(op *Operation[map[string]interface{}], rf *ResultFactory[map[string]interface{}]) *OperationResult[map[string]interface{}] {
			if len(vm.CallStack) == 0 {
				panic("CallStackUnderflowError")
			}
			frame := vm.CallStack[len(vm.CallStack)-1]
			vm.CallStack = vm.CallStack[:len(vm.CallStack)-1]
			return rf.Generate(true, false, frame)
		}).PanicOnUnexpected().GetResult()
	return result
}

// ════════════════════════════════════════════════════════════════════════
// PROGRAM COUNTER CONTROL
// ════════════════════════════════════════════════════════════════════════
//
// The Program Counter (PC) tells the VM which instruction to execute
// next.  Normally it advances by 1 after each instruction (sequential
// execution).  Jump instructions set it to an arbitrary target.

// AdvancePC increments the program counter by 1.
// This is the normal "move to next instruction" operation.
func (vm *GenericVM) AdvancePC() {
	_, _ = StartNew[struct{}]("virtual-machine.AdvancePC", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			vm.PC++
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// JumpTo sets the program counter to an arbitrary target address.
// Used by jump/branch instructions to implement loops, if/else, etc.
//
// Example: a conditional jump handler might do:
//
//	if condition {
//	    vm.JumpTo(target)
//	} else {
//	    vm.AdvancePC()
//	}
func (vm *GenericVM) JumpTo(target int) {
	_, _ = StartNew[struct{}]("virtual-machine.JumpTo", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("target", target)
			vm.PC = target
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// ════════════════════════════════════════════════════════════════════════
// CONFIGURATION
// ════════════════════════════════════════════════════════════════════════

// InjectGlobals pre-seeds named variables into the VM's global scope.
//
// These variables are available to the program as regular global variables,
// but they exist before execution begins.  This is the proper way to pass
// context from the host environment (e.g., build configuration, platform
// information) into a Starlark/bytecode program.
//
// Injected globals are merged into Variables — they don't replace the map.
// If a key already exists in Variables, the injected value overwrites it.
//
// This is analogous to Bazel's repository_ctx.os which provides platform
// information to Starlark code, except our mechanism is general-purpose:
// any key-value pair can be injected.
//
// Example:
//
//	vm := NewGenericVM()
//	vm.InjectGlobals(map[string]interface{}{
//	    "_ctx": map[string]interface{}{
//	        "os":   "darwin",
//	        "arch": "arm64",
//	    },
//	})
//	// Now the program can access _ctx["os"] as a regular variable.
func (vm *GenericVM) InjectGlobals(globals map[string]interface{}) {
	for k, v := range globals {
		vm.Variables[k] = v
	}
}

// SetMaxRecursionDepth configures the maximum call stack depth.
// Pass nil for unlimited recursion.  Pass a pointer to 0 to disallow
// any function calls.
func (vm *GenericVM) SetMaxRecursionDepth(depth *int) {
	_, _ = StartNew[struct{}]("virtual-machine.SetMaxRecursionDepth", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			vm.maxRecursionDepth = depth
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// MaxRecursionDepth returns the current max recursion depth setting.
// Returns nil if unlimited.
func (vm *GenericVM) MaxRecursionDepth() *int {
	result, _ := StartNew[*int]("virtual-machine.MaxRecursionDepth", nil,
		func(op *Operation[*int], rf *ResultFactory[*int]) *OperationResult[*int] {
			return rf.Generate(true, false, vm.maxRecursionDepth)
		}).GetResult()
	return result
}

// SetFrozen locks or unlocks the VM's handler/builtin registration.
// A frozen VM will panic if you try to register new opcodes or builtins.
// This is useful for sandboxing: configure the VM, freeze it, then hand
// it to untrusted code that can execute but not modify the instruction set.
func (vm *GenericVM) SetFrozen(frozen bool) {
	_, _ = StartNew[struct{}]("virtual-machine.SetFrozen", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("frozen", frozen)
			vm.frozen = frozen
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// IsFrozen returns whether the VM is currently frozen.
func (vm *GenericVM) IsFrozen() bool {
	result, _ := StartNew[bool]("virtual-machine.IsFrozen", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, vm.frozen)
		}).GetResult()
	return result
}

// ════════════════════════════════════════════════════════════════════════
// EXECUTION
// ════════════════════════════════════════════════════════════════════════

// Execute runs a CodeObject from start to finish, collecting a VMTrace
// for each instruction executed.  Execution stops when:
//   - The VM is halted (a handler set vm.Halted = true).
//   - The PC moves past the end of the instruction list.
//
// This is the main entry point for running bytecode programs.
//
//	vm := NewGenericVM()
//	// ... register handlers ...
//	traces := vm.Execute(code)
//	for _, t := range traces {
//	    fmt.Printf("PC=%d  %s\n", t.PC, t.Description)
//	}
func (vm *GenericVM) Execute(code CodeObject) []VMTrace {
	result, _ := StartNew[[]VMTrace]("virtual-machine.GenericVM.Execute", nil,
		func(op *Operation[[]VMTrace], rf *ResultFactory[[]VMTrace]) *OperationResult[[]VMTrace] {
			var traces []VMTrace
			for !vm.Halted && vm.PC < len(code.Instructions) {
				traces = append(traces, vm.Step(code))
			}
			return rf.Generate(true, false, traces)
		}).PanicOnUnexpected().GetResult()
	return result
}

// Step executes exactly one instruction and returns a VMTrace describing
// what happened.  This is the fetch-decode-execute cycle in one call.
//
// The process:
//  1. FETCH:   Read code.Instructions[vm.PC].
//  2. SNAPSHOT: Copy the stack before execution (for the trace).
//  3. DECODE:  Look up the handler for instr.Opcode.
//  4. EXECUTE: Call the handler.  The handler modifies VM state
//     (stack, variables, PC, etc.) and optionally returns output.
//  5. TRACE:   Build a VMTrace with before/after snapshots.
//
// If no handler is registered for the opcode, Step panics with
// "InvalidOpcodeError".
//
// The description field in the trace uses the hex format of the opcode:
// "Executed opcode 0x01" for OpLoadConst (0x01).
func (vm *GenericVM) Step(code CodeObject) VMTrace {
	result, _ := StartNew[VMTrace]("virtual-machine.GenericVM.Step", VMTrace{},
		func(op *Operation[VMTrace], rf *ResultFactory[VMTrace]) *OperationResult[VMTrace] {
			op.AddProperty("pc", vm.PC)
			// 1. FETCH
			instr := code.Instructions[vm.PC]
			pcBefore := vm.PC

			// 2. SNAPSHOT — capture stack state before the handler runs
			stackBefore := vm.copyStack()

			// 3. DECODE — find the handler for this opcode
			handler, ok := vm.handlers[instr.Opcode]
			if !ok {
				panic(fmt.Sprintf("InvalidOpcodeError: no handler registered for opcode 0x%02x", instr.Opcode))
			}

			// 4. EXECUTE — run the handler
			outputVal := handler(vm, instr, code)

			// If the handler produced output, record it
			if outputVal != nil {
				vm.Output = append(vm.Output, *outputVal)
			}

			// 5. TRACE — build the execution record
			return rf.Generate(true, false, VMTrace{
				PC:          pcBefore,
				Instruction: instr,
				StackBefore: stackBefore,
				StackAfter:  vm.copyStack(),
				Variables:   vm.copyMap(vm.Variables),
				Output:      outputVal,
				Description: fmt.Sprintf("Executed opcode 0x%02x", instr.Opcode),
			})
		}).PanicOnUnexpected().GetResult()
	return result
}

// ════════════════════════════════════════════════════════════════════════
// RESET
// ════════════════════════════════════════════════════════════════════════

// Reset clears all runtime state (stack, variables, locals, PC, output,
// call stack, halted flag) but PRESERVES registered handlers, builtins,
// configuration (maxRecursionDepth, frozen).
//
// This lets you reuse a configured VM to run multiple programs without
// re-registering all the opcodes.
//
//	vm := NewGenericVM()
//	// ... register handlers ...
//	vm.Execute(program1)
//	vm.Reset()
//	vm.Execute(program2)  // handlers are still registered
func (vm *GenericVM) Reset() {
	_, _ = StartNew[struct{}]("virtual-machine.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			vm.Stack = []interface{}{}
			vm.Variables = make(map[string]interface{})
			vm.Locals = []interface{}{}
			vm.PC = 0
			vm.Halted = false
			vm.Output = []string{}
			vm.CallStack = []map[string]interface{}{}
			vm.TypedStack = []TypedVMValue{}
			vm.ExecutionContext = nil
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// ════════════════════════════════════════════════════════════════════════
// INTERNAL HELPERS
// ════════════════════════════════════════════════════════════════════════
//
// These are private utility methods used by the VM internals.  They
// follow the same pattern as the original VirtualMachine in vm.go.

// copyStack returns a shallow copy of the operand stack.
func (vm *GenericVM) copyStack() []interface{} {
	c := make([]interface{}, len(vm.Stack))
	copy(c, vm.Stack)
	return c
}

// copyMap returns a shallow copy of a string-keyed map.
func (vm *GenericVM) copyMap(m map[string]interface{}) map[string]interface{} {
	c := make(map[string]interface{})
	for k, v := range m {
		c[k] = v
	}
	return c
}
