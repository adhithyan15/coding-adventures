// host_interface.go --- TrapError and host function interfaces for WASM.
//
// ════════════════════════════════════════════════════════════════════════
// TRAPS
// ════════════════════════════════════════════════════════════════════════
//
// A WASM trap is an unrecoverable runtime error.  When a trap occurs,
// execution halts immediately and propagates to the host.  Go models
// traps as a TrapError type that implements the error interface.
//
// Common trap causes:
//   - Out-of-bounds memory access
//   - Division by zero
//   - Integer overflow in i32.div_s(INT32_MIN, -1)
//   - Unreachable instruction
//   - Uninitialized table element in call_indirect
//   - Type mismatch in call_indirect
//
// ════════════════════════════════════════════════════════════════════════
// HOST INTERFACE
// ════════════════════════════════════════════════════════════════════════
//
// WASM modules interact with the outside world through imports.  The
// HostInterface is the contract that any host environment must implement
// to provide imported functions, globals, memories, and tables.
package wasmexecution

import (
	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
)

// ════════════════════════════════════════════════════════════════════════
// TRAP ERROR
// ════════════════════════════════════════════════════════════════════════

// TrapError represents a WASM runtime trap — an unrecoverable error.
type TrapError struct {
	Message string
}

func (e *TrapError) Error() string {
	return "TrapError: " + e.Message
}

// NewTrapError creates a new TrapError with the given message.
func NewTrapError(msg string) *TrapError {
	return &TrapError{Message: msg}
}

// ════════════════════════════════════════════════════════════════════════
// HOST FUNCTION
// ════════════════════════════════════════════════════════════════════════

// HostFunction is a callable function provided by the host environment.
//
// When a WASM module imports a function, the host must provide an object
// implementing this interface.  The Type field describes the expected
// parameter and return types, and the Call method performs the actual work.
type HostFunction struct {
	Type wasmtypes.FuncType
	Call func(args []WasmValue) []WasmValue
}

// ════════════════════════════════════════════════════════════════════════
// HOST GLOBAL
// ════════════════════════════════════════════════════════════════════════

// HostGlobal is an imported global variable provided by the host.
type HostGlobal struct {
	Type  wasmtypes.GlobalType
	Value WasmValue
}

// ════════════════════════════════════════════════════════════════════════
// HOST INTERFACE
// ════════════════════════════════════════════════════════════════════════

// HostInterface resolves WASM imports from the host environment.
//
// Each resolve method takes a two-level namespace (module + name) and
// returns the imported definition, or nil if not found.
type HostInterface interface {
	ResolveFunction(moduleName, name string) *HostFunction
	ResolveGlobal(moduleName, name string) *HostGlobal
	ResolveMemory(moduleName, name string) *LinearMemory
	ResolveTable(moduleName, name string) *Table
}
