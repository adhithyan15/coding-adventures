// typed_stack.go — Typed value stack and context execution for WASM support.
//
// ═���══════════════════════════════════════════════════════════════════════
// WHY A TYPED STACK?
// ═════════════════════════════════════════���══════════════════════════════
//
// The original GenericVM stack stores untyped interface{} values.  This
// works great for dynamically-typed languages like Python, where a
// variable can hold any type at any time.
//
// WebAssembly, however, is *statically typed* at the value level.  Every
// value on the stack carries a type tag (i32, i64, f32, f64).  A value
// of type i32(42) is fundamentally different from f64(42.0) — using one
// where the other is expected would be a type error.
//
// TypedVMValue pairs a type tag (an integer) with a raw payload.  The
// TypedStack is a separate LIFO stack of these typed values, running in
// parallel with the untyped Stack.  WASM instruction handlers use the
// typed stack exclusively; classic bytecode handlers use the untyped one.
//
// ���════════════════════════════════════��══════════════════════════════════
// CONTEXT EXECUTION
// ════════════════���═══════════════════════════════════════════════════════
//
// WASM instruction handlers need access to runtime state beyond what the
// generic VM provides: linear memory, tables, globals, local variables,
// and control flow labels.  Rather than bloating the GenericVM struct
// with WASM-specific fields, we use a *context* pattern:
//
//   - RegisterContextOpcode binds an opcode to a ContextOpcodeHandler,
//     which receives an additional "context" argument (interface{}).
//   - ExecuteWithContext runs a CodeObject with a context object that is
//     passed to every context handler during execution.
//   - The context is opaque to the VM — it's just an interface{}.  The
//     WASM execution engine passes a WasmExecutionContext struct.
//
// This keeps the GenericVM truly generic while supporting the richer
// needs of typed language runtimes like WASM.
//
// ════���═══════════════════════════════════════════════════════════════════
// PRE/POST INSTRUCTION HOOKS
// ════════════════════════════════════════════════════════════════════════
//
// Hooks allow external code to observe or transform execution at two
// points in the fetch-decode-execute cycle:
//
//   - PreInstructionHook:  called BEFORE the handler, after fetch.
//   - PostInstructionHook: called AFTER the handler completes.
//
// The WASM execution engine uses hooks for bytecode decoding (translating
// variable-length WASM bytecodes into fixed-format instructions) and for
// detecting function call/return boundaries.
package virtualmachine

import "fmt"

// ��═══════════════════════════════════════════════════════════════════════
// TYPED VALUE
// ════════════════════════��═══════════════════════════════════════════════

// TypedVMValue carries both a type tag and a raw payload.
//
// In WASM, the type tag is one of:
//
//	0x7F = i32 (32-bit integer)
//	0x7E = i64 (64-bit integer)
//	0x7D = f32 (32-bit float)
//	0x7C = f64 (64-bit float)
//
// The Value field holds the Go representation:
//
//	i32 → int32
//	i64 → int64
//	f32 → float32
//	f64 → float64
//
// Example:
//
//	v := TypedVMValue{Type: 0x7F, Value: int32(42)}  // i32(42)
type TypedVMValue struct {
	Type  int         // type tag (e.g., 0x7F for i32)
	Value interface{} // raw payload (int32, int64, float32, float64)
}

// ══���═══��═══════════════════════════════════���═════════════════════════════
// CONTEXT OPCODE HANDLER
// ════════════════════════════════════════════════════════════════════════

// ContextOpcodeHandler is like OpcodeHandler but receives an additional
// context argument.  This is how WASM instruction handlers access the
// execution context (memory, tables, globals, locals, etc.).
//
// The context is an interface{} — the handler casts it to the expected
// type (e.g., *WasmExecutionContext).
//
// Returns a *string for trace output, just like OpcodeHandler.
type ContextOpcodeHandler func(vm *GenericVM, instr Instruction, code CodeObject, ctx interface{}) *string

// ���══════════════════════════════════���════════════════════════════════════
// TYPED STACK OPERATIONS
// ════════════════════════════════════════════════════════════════════════

// PushTyped places a TypedVMValue on top of the typed stack.
//
// This is the primary way WASM instruction handlers push results.
// Each value carries its type tag, enabling runtime type checking.
//
// Example:
//
//	vm.PushTyped(TypedVMValue{Type: 0x7F, Value: int32(42)})
func (vm *GenericVM) PushTyped(value TypedVMValue) {
	vm.TypedStack = append(vm.TypedStack, value)
}

// PopTyped removes and returns the top TypedVMValue from the typed stack.
// Panics with "TypedStackUnderflowError" if the typed stack is empty.
//
// Example:
//
//	v := vm.PopTyped()  // TypedVMValue{Type: 0x7F, Value: int32(42)}
func (vm *GenericVM) PopTyped() TypedVMValue {
	if len(vm.TypedStack) == 0 {
		panic("TypedStackUnderflowError")
	}
	val := vm.TypedStack[len(vm.TypedStack)-1]
	vm.TypedStack = vm.TypedStack[:len(vm.TypedStack)-1]
	return val
}

// PeekTyped returns the top TypedVMValue without removing it.
// Panics with "TypedStackUnderflowError" if the typed stack is empty.
func (vm *GenericVM) PeekTyped() TypedVMValue {
	if len(vm.TypedStack) == 0 {
		panic("TypedStackUnderflowError")
	}
	return vm.TypedStack[len(vm.TypedStack)-1]
}

// ════════════════════════════════════════════════════════════════════════
// CONTEXT OPCODE REGISTRATION
// ════════════════════════════════════════════════════════════════════════

// RegisterContextOpcode binds an opcode to a ContextOpcodeHandler.
//
// During context execution (ExecuteWithContext), the VM first checks
// the context handler table before falling back to the regular handler
// table.  This allows WASM opcodes to coexist with classic opcodes.
//
// If the VM is frozen, this panics with "FrozenVMError".
//
// Example:
//
//	vm.RegisterContextOpcode(0x41, func(vm *GenericVM, instr Instruction,
//	    code CodeObject, ctx interface{}) *string {
//	    // i32.const handler
//	    wasmCtx := ctx.(*WasmExecutionContext)
//	    vm.PushTyped(TypedVMValue{Type: 0x7F, Value: int32(instr.Operand.(int))})
//	    vm.AdvancePC()
//	    return nil
//	})
func (vm *GenericVM) RegisterContextOpcode(opcode OpCode, handler ContextOpcodeHandler) {
	if vm.frozen {
		panic("FrozenVMError: cannot register opcodes on a frozen VM")
	}
	if vm.contextHandlers == nil {
		vm.contextHandlers = make(map[OpCode]ContextOpcodeHandler)
	}
	vm.contextHandlers[opcode] = handler
}

// ═════��══════════════════════════��═══════════════════════════════════════
// HOOKS
// ��═══════════════════════════════════════════════════════════════════════

// SetPreInstructionHook installs a function that is called before each
// instruction is dispatched.  The hook receives the VM, the current
// instruction, and the code object.  It can modify state (e.g., decode
// variable-length bytecodes into the instruction's operand field).
//
// Pass nil to remove the hook.
func (vm *GenericVM) SetPreInstructionHook(hook func(vm *GenericVM, instr *Instruction, code CodeObject)) {
	vm.preInstructionHook = hook
}

// SetPostInstructionHook installs a function that is called after each
// instruction handler completes.  Useful for debugging, tracing, or
// detecting function call/return boundaries.
//
// Pass nil to remove the hook.
func (vm *GenericVM) SetPostInstructionHook(hook func(vm *GenericVM, instr Instruction, code CodeObject)) {
	vm.postInstructionHook = hook
}

// ═════════���══════════════════════════════��═══════════════════════════════
// CONTEXT EXECUTION
// ════════════════════════════════════════════════════════════════════════

// ExecuteWithContext runs a CodeObject with a context object.
//
// This is the WASM execution entry point.  Unlike Execute(), it:
//   - Passes the context to ContextOpcodeHandlers.
//   - Calls pre/post instruction hooks if installed.
//   - Prefers context handlers over regular handlers.
//
// The execution loop continues until vm.Halted is true or the PC
// exceeds the instruction count.
//
// Example:
//
//	ctx := &WasmExecutionContext{memory: mem, tables: tables, ...}
//	vm.ExecuteWithContext(code, ctx)
func (vm *GenericVM) ExecuteWithContext(code CodeObject, ctx interface{}) {
	vm.ExecutionContext = ctx
	for !vm.Halted && vm.PC < len(code.Instructions) {
		vm.StepWithContext(code, ctx)
	}
}

// StepWithContext executes one instruction with context support.
//
// The fetch-decode-execute cycle with hooks:
//  1. FETCH: Read the instruction at vm.PC.
//  2. PRE-HOOK: Call preInstructionHook if set.
//  3. DECODE: Look up context handler first, then regular handler.
//  4. EXECUTE: Call the handler with the context.
//  5. POST-HOOK: Call postInstructionHook if set.
func (vm *GenericVM) StepWithContext(code CodeObject, ctx interface{}) {
	// 1. FETCH
	instr := code.Instructions[vm.PC]

	// 2. PRE-HOOK — allows bytecode decoding, instruction transformation, etc.
	if vm.preInstructionHook != nil {
		vm.preInstructionHook(vm, &instr, code)
	}

	// 3. DECODE — prefer context handlers, fall back to regular handlers.
	if ctxHandler, ok := vm.contextHandlers[instr.Opcode]; ok {
		outputVal := ctxHandler(vm, instr, code, ctx)
		if outputVal != nil {
			vm.Output = append(vm.Output, *outputVal)
		}
	} else if handler, ok := vm.handlers[instr.Opcode]; ok {
		outputVal := handler(vm, instr, code)
		if outputVal != nil {
			vm.Output = append(vm.Output, *outputVal)
		}
	} else {
		panic(fmt.Sprintf("InvalidOpcodeError: no handler registered for opcode 0x%02x", instr.Opcode))
	}

	// 5. POST-HOOK — allows tracing, call/return detection, etc.
	if vm.postInstructionHook != nil {
		vm.postInstructionHook(vm, instr, code)
	}
}
