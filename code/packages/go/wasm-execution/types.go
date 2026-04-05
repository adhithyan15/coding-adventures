// types.go --- Shared type definitions for the WASM execution engine.
//
// These types are used across the execution engine: the execution context,
// labels for structured control flow, and control flow map entries.
package wasmexecution

import (
	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
)

// ════════════════════════════════════════════════════════════════════════
// CONTROL FLOW STRUCTURES
// ════════════════════════════════════════════════════════════════════════

// Label tracks one level of structured control flow (block/loop/if).
//
// When br N executes, it unwinds to the Nth label from the top of the
// label stack:
//   - block/if labels: branch jumps to END (forward)
//   - loop labels: branch jumps to LOOP START (backward)
type Label struct {
	Arity       int  // how many values this block produces
	TargetPC    int  // where to jump on branch
	StackHeight int  // typed stack height when block started
	IsLoop      bool // loops branch backward; blocks branch forward
}

// ControlTarget maps a block/loop/if start to its end (and else for if).
type ControlTarget struct {
	EndPC  int // instruction index of the matching end
	ElsePC int // instruction index of else, or -1 if no else
}

// ════════════════════════════════════════════════════════════════════════
// SAVED FRAME
// ════════════════════════════════════════════════════════════════════════

// SavedFrame is a snapshot of the caller's state before a function call.
type SavedFrame struct {
	Locals         []WasmValue
	LabelStack     []Label
	StackHeight    int
	ControlFlowMap map[int]ControlTarget
	ReturnPC       int
	ReturnArity    int
}

// ════════════════════════════════════════════════════════════════════════
// EXECUTION CONTEXT
// ════════════════════════════════════════════════════════════════════════

// WasmExecutionContext is the per-execution context passed to all WASM
// instruction handlers via ExecuteWithContext.
//
// It carries all runtime state that WASM instructions need: linear
// memory, tables, globals, locals, and control flow structures.
type WasmExecutionContext struct {
	Memory         *LinearMemory
	Tables         []*Table
	Globals        []WasmValue
	GlobalTypes    []wasmtypes.GlobalType
	FuncTypes      []wasmtypes.FuncType
	FuncBodies     []*wasmtypes.FunctionBody // nil for imported functions
	HostFunctions  []*HostFunction           // nil for module-defined functions
	TypedLocals    []WasmValue
	LabelStack     []Label
	ControlFlowMap map[int]ControlTarget
	SavedFrames    []SavedFrame
	Returned       bool
	ReturnValues   []WasmValue
}
