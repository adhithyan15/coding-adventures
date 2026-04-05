// engine.go --- WasmExecutionEngine: the core WASM interpreter.
//
// ════════════════════════════════════════════════════════════════════════
// HOW IT FITS TOGETHER
// ════════════════════════════════════════════════════════════════════════
//
// The engine takes a module's runtime state (memory, tables, globals,
// functions) and executes function calls using the GenericVM.
//
// Flow for calling a function:
//
//	callFunction(funcIndex, args)
//	  1. Look up the function body.
//	  2. Decode bytecodes into Instruction[].
//	  3. Build the control flow map.
//	  4. Initialize locals (args + zero-initialized declared locals).
//	  5. Create WasmExecutionContext.
//	  6. Run GenericVM.ExecuteWithContext(code, ctx).
//	  7. Collect return values from the typed stack.
package wasmexecution

import (
	"fmt"

	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
)

// MaxCallDepth limits recursive function calls.
const MaxCallDepth = 1024

// EngineConfig holds the configuration for creating a WasmExecutionEngine.
type EngineConfig struct {
	Memory        *LinearMemory
	Tables        []*Table
	Globals       []WasmValue
	GlobalTypes   []wasmtypes.GlobalType
	FuncTypes     []wasmtypes.FuncType
	FuncBodies    []*wasmtypes.FunctionBody
	HostFunctions []*HostFunction
}

// WasmExecutionEngine interprets validated WASM modules.
type WasmExecutionEngine struct {
	vm            *vm.GenericVM
	memory        *LinearMemory
	tables        []*Table
	globals       []WasmValue
	globalTypes   []wasmtypes.GlobalType
	funcTypes     []wasmtypes.FuncType
	funcBodies    []*wasmtypes.FunctionBody
	hostFunctions []*HostFunction
	decodedCache  map[int][]DecodedInstruction
}

// NewWasmExecutionEngine creates a new engine with the given configuration.
func NewWasmExecutionEngine(config EngineConfig) *WasmExecutionEngine {
	genVM := vm.NewGenericVM()
	depth := MaxCallDepth
	genVM.SetMaxRecursionDepth(&depth)

	// Register all WASM instruction handlers.
	RegisterAllInstructions(genVM)
	RegisterControl(genVM)

	return &WasmExecutionEngine{
		vm:            genVM,
		memory:        config.Memory,
		tables:        config.Tables,
		globals:       config.Globals,
		globalTypes:   config.GlobalTypes,
		funcTypes:     config.FuncTypes,
		funcBodies:    config.FuncBodies,
		hostFunctions: config.HostFunctions,
		decodedCache:  make(map[int][]DecodedInstruction),
	}
}

// CallFunction calls a WASM function by index.
//
// Returns the function's return values as WasmValues, or an error on trap.
func (e *WasmExecutionEngine) CallFunction(funcIndex int, args []WasmValue) (results []WasmValue, err error) {
	// Recover from panics (TrapErrors) and return them as errors.
	defer func() {
		if r := recover(); r != nil {
			if te, ok := r.(*TrapError); ok {
				err = te
			} else {
				err = fmt.Errorf("unexpected panic: %v", r)
			}
		}
	}()

	if funcIndex >= len(e.funcTypes) {
		return nil, NewTrapError(fmt.Sprintf("undefined function index %d", funcIndex))
	}

	funcType := e.funcTypes[funcIndex]
	if len(args) != len(funcType.Params) {
		return nil, NewTrapError(fmt.Sprintf(
			"function %d expects %d arguments, got %d",
			funcIndex, len(funcType.Params), len(args)))
	}

	// Host function?
	if funcIndex < len(e.hostFunctions) && e.hostFunctions[funcIndex] != nil {
		return e.hostFunctions[funcIndex].Call(args), nil
	}

	// Module-defined function.
	if funcIndex >= len(e.funcBodies) || e.funcBodies[funcIndex] == nil {
		return nil, NewTrapError(fmt.Sprintf("no body for function %d", funcIndex))
	}
	body := e.funcBodies[funcIndex]

	// Decode the function body (cached).
	decoded, ok := e.decodedCache[funcIndex]
	if !ok {
		decoded = DecodeFunctionBody(body)
		e.decodedCache[funcIndex] = decoded
	}

	// Build control flow map.
	controlFlowMap := BuildControlFlowMap(decoded)

	// Convert to GenericVM instructions.
	vmInstructions := ToVMInstructions(decoded)

	// Initialize locals: arguments + zero-initialized declared locals.
	typedLocals := make([]WasmValue, 0, len(args)+len(body.Locals))
	typedLocals = append(typedLocals, args...)
	for _, lt := range body.Locals {
		typedLocals = append(typedLocals, DefaultValue(lt))
	}

	// Build execution context.
	ctx := &WasmExecutionContext{
		Memory:         e.memory,
		Tables:         e.tables,
		Globals:        e.globals,
		GlobalTypes:    e.globalTypes,
		FuncTypes:      e.funcTypes,
		FuncBodies:     e.funcBodies,
		HostFunctions:  e.hostFunctions,
		TypedLocals:    typedLocals,
		LabelStack:     nil,
		ControlFlowMap: controlFlowMap,
		SavedFrames:    nil,
		Returned:       false,
		ReturnValues:   nil,
	}

	// Build the CodeObject.
	code := vm.CodeObject{
		Instructions: vmInstructions,
		Constants:    nil,
		Names:        nil,
	}

	// Reset and execute.
	e.vm.Reset()
	e.vm.ExecuteWithContext(code, ctx)

	// Collect return values.
	resultCount := len(funcType.Results)
	results = make([]WasmValue, resultCount)
	for i := resultCount - 1; i >= 0; i-- {
		if len(e.vm.TypedStack) > 0 {
			results[i] = e.vm.PopTyped()
		}
	}

	return results, nil
}
