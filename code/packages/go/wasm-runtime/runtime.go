// Package wasmruntime provides the complete WebAssembly 1.0 runtime.
//
// ════════════════════════════════════════════════════════════════════════
// WHAT IS A RUNTIME?
// ════════════════════════════════════════════════════════════════════════
//
// A WASM runtime composes the parser, validator, and execution engine
// into a single, user-facing API.  It handles the full pipeline:
//
//	.wasm bytes  →  Parse  →  Validate  →  Instantiate  →  Execute
//
// The convenience method LoadAndRun does all four steps in one call:
//
//	runtime := wasmruntime.New(nil)
//	result, err := runtime.LoadAndRun(squareWasm, "square", []int{5})
//	// result = [25]
package wasmruntime

import (
	"fmt"

	wasmexecution "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-execution"
	wasmmoduleparser "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-module-parser"
	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
	wasmvalidator "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-validator"
)

// ════════════════════════════════════════════════════════════════════════
// WASM INSTANCE
// ════════════════════════════════════════════════════════════════════════

// WasmInstance is a live, executable instance of a WASM module.
type WasmInstance struct {
	Module        *wasmtypes.WasmModule
	Memory        *wasmexecution.LinearMemory
	Tables        []*wasmexecution.Table
	Globals       []wasmexecution.WasmValue
	GlobalTypes   []wasmtypes.GlobalType
	FuncTypes     []wasmtypes.FuncType
	FuncBodies    []*wasmtypes.FunctionBody
	HostFunctions []*wasmexecution.HostFunction
	Exports       map[string]Export
	Host          wasmexecution.HostInterface
}

// Export describes an exported entity.
type Export struct {
	Kind  wasmtypes.ExternalKind
	Index uint32
}

// ════════════════════════════════════════════════════════════════════════
// WASM RUNTIME
// ════════════════════════════════════════════════════════════════════════

// WasmRuntime composes the parser, validator, and execution engine.
type WasmRuntime struct {
	parser *wasmmoduleparser.Parser
	host   wasmexecution.HostInterface
}

type memoryBinder interface {
	SetMemory(*wasmexecution.LinearMemory)
}

// New creates a new WasmRuntime with an optional host interface.
func New(host wasmexecution.HostInterface) *WasmRuntime {
	return &WasmRuntime{
		parser: wasmmoduleparser.New(),
		host:   host,
	}
}

// Load parses a .wasm binary into a WasmModule.
func (r *WasmRuntime) Load(wasmBytes []byte) (*wasmtypes.WasmModule, error) {
	return r.parser.Parse(wasmBytes)
}

// Validate checks a parsed module for structural correctness.
func (r *WasmRuntime) Validate(module *wasmtypes.WasmModule) (*wasmvalidator.ValidatedModule, error) {
	return wasmvalidator.Validate(module)
}

// Instantiate creates a live instance from a parsed module.
func (r *WasmRuntime) Instantiate(module *wasmtypes.WasmModule) (*WasmInstance, error) {
	var funcTypes []wasmtypes.FuncType
	var funcBodies []*wasmtypes.FunctionBody
	var hostFunctions []*wasmexecution.HostFunction
	var globalTypes []wasmtypes.GlobalType
	var globals []wasmexecution.WasmValue
	var memory *wasmexecution.LinearMemory
	var tables []*wasmexecution.Table

	// Step 1: Resolve imports.
	for _, imp := range module.Imports {
		switch imp.Kind {
		case wasmtypes.ExternalKindFunction:
			typeIdx := imp.TypeInfo.(uint32)
			funcTypes = append(funcTypes, module.Types[typeIdx])
			funcBodies = append(funcBodies, nil)
			var hf *wasmexecution.HostFunction
			if r.host != nil {
				hf = r.host.ResolveFunction(imp.ModuleName, imp.Name)
			}
			hostFunctions = append(hostFunctions, hf)

		case wasmtypes.ExternalKindMemory:
			if r.host != nil {
				if m := r.host.ResolveMemory(imp.ModuleName, imp.Name); m != nil {
					memory = m
				}
			}

		case wasmtypes.ExternalKindTable:
			if r.host != nil {
				if t := r.host.ResolveTable(imp.ModuleName, imp.Name); t != nil {
					tables = append(tables, t)
				}
			}

		case wasmtypes.ExternalKindGlobal:
			if r.host != nil {
				if g := r.host.ResolveGlobal(imp.ModuleName, imp.Name); g != nil {
					globalTypes = append(globalTypes, g.Type)
					globals = append(globals, g.Value)
				}
			}
		}
	}

	// Step 2: Add module-defined functions.
	for i, typeIdx := range module.Functions {
		funcTypes = append(funcTypes, module.Types[typeIdx])
		if i < len(module.Code) {
			body := module.Code[i]
			funcBodies = append(funcBodies, &body)
		} else {
			funcBodies = append(funcBodies, nil)
		}
		hostFunctions = append(hostFunctions, nil)
	}

	// Step 3: Allocate memory.
	if memory == nil && len(module.Memories) > 0 {
		memType := module.Memories[0]
		maxPages := -1
		if memType.Limits.HasMax {
			maxPages = int(memType.Limits.Max)
		}
		memory = wasmexecution.NewLinearMemory(int(memType.Limits.Min), maxPages)
	}

	// Step 4: Allocate tables.
	for _, tableType := range module.Tables {
		maxSize := -1
		if tableType.Limits.HasMax {
			maxSize = int(tableType.Limits.Max)
		}
		tables = append(tables, wasmexecution.NewTable(int(tableType.Limits.Min), maxSize))
	}

	// Step 5: Initialize globals.
	for _, g := range module.Globals {
		globalTypes = append(globalTypes, g.GlobalType)
		val, err := wasmexecution.EvaluateConstExpr(g.InitExpr, globals)
		if err != nil {
			return nil, fmt.Errorf("global init: %w", err)
		}
		globals = append(globals, val)
	}

	// Step 6: Apply data segments.
	if memory != nil {
		for _, seg := range module.Data {
			offset, err := wasmexecution.EvaluateConstExpr(seg.OffsetExpr, globals)
			if err != nil {
				return nil, fmt.Errorf("data segment offset: %w", err)
			}
			memory.WriteBytes(int(offset.Value.(int32)), seg.Data)
		}
	}

	// Step 7: Apply element segments.
	for _, elem := range module.Elements {
		if int(elem.TableIndex) < len(tables) {
			table := tables[elem.TableIndex]
			offset, err := wasmexecution.EvaluateConstExpr(elem.OffsetExpr, globals)
			if err != nil {
				return nil, fmt.Errorf("element segment offset: %w", err)
			}
			offsetNum := int(offset.Value.(int32))
			for j, funcIdx := range elem.FunctionIndices {
				table.Set(offsetNum+j, int(funcIdx))
			}
		}
	}

	// Build export map.
	exports := make(map[string]Export)
	for _, exp := range module.Exports {
		exports[exp.Name] = Export{Kind: exp.Kind, Index: exp.Index}
	}

	instance := &WasmInstance{
		Module:        module,
		Memory:        memory,
		Tables:        tables,
		Globals:       globals,
		GlobalTypes:   globalTypes,
		FuncTypes:     funcTypes,
		FuncBodies:    funcBodies,
		HostFunctions: hostFunctions,
		Exports:       exports,
		Host:          r.host,
	}

	if binder, ok := r.host.(memoryBinder); ok && instance.Memory != nil {
		binder.SetMemory(instance.Memory)
	}

	// Step 8: Call start function.
	if module.Start != nil {
		engine := wasmexecution.NewWasmExecutionEngine(wasmexecution.EngineConfig{
			Memory:        instance.Memory,
			Tables:        instance.Tables,
			Globals:       instance.Globals,
			GlobalTypes:   instance.GlobalTypes,
			FuncTypes:     instance.FuncTypes,
			FuncBodies:    instance.FuncBodies,
			HostFunctions: instance.HostFunctions,
		})
		if _, err := engine.CallFunction(int(*module.Start), nil); err != nil {
			return nil, fmt.Errorf("start function error: %w", err)
		}
	}

	return instance, nil
}

// Call invokes an exported function by name.
func (r *WasmRuntime) Call(instance *WasmInstance, name string, args []int) ([]int, error) {
	exp, ok := instance.Exports[name]
	if !ok {
		return nil, wasmexecution.NewTrapError(fmt.Sprintf("export %q not found", name))
	}
	if exp.Kind != wasmtypes.ExternalKindFunction {
		return nil, wasmexecution.NewTrapError(fmt.Sprintf("export %q is not a function", name))
	}

	funcType := instance.FuncTypes[exp.Index]

	// Convert plain ints to WasmValues.
	wasmArgs := make([]wasmexecution.WasmValue, len(args))
	for i, arg := range args {
		if i < len(funcType.Params) {
			switch funcType.Params[i] {
			case wasmtypes.ValueTypeI32:
				wasmArgs[i] = wasmexecution.I32(int32(arg))
			case wasmtypes.ValueTypeI64:
				wasmArgs[i] = wasmexecution.I64(int64(arg))
			case wasmtypes.ValueTypeF32:
				wasmArgs[i] = wasmexecution.F32(float32(arg))
			case wasmtypes.ValueTypeF64:
				wasmArgs[i] = wasmexecution.F64(float64(arg))
			default:
				wasmArgs[i] = wasmexecution.I32(int32(arg))
			}
		}
	}

	engine := wasmexecution.NewWasmExecutionEngine(wasmexecution.EngineConfig{
		Memory:        instance.Memory,
		Tables:        instance.Tables,
		Globals:       instance.Globals,
		GlobalTypes:   instance.GlobalTypes,
		FuncTypes:     instance.FuncTypes,
		FuncBodies:    instance.FuncBodies,
		HostFunctions: instance.HostFunctions,
	})

	results, err := engine.CallFunction(int(exp.Index), wasmArgs)
	if err != nil {
		return nil, err
	}

	// Convert back to ints.
	intResults := make([]int, len(results))
	for i, r := range results {
		switch v := r.Value.(type) {
		case int32:
			intResults[i] = int(v)
		case int64:
			intResults[i] = int(v)
		case float32:
			intResults[i] = int(v)
		case float64:
			intResults[i] = int(v)
		}
	}

	return intResults, nil
}

// LoadAndRun does parse, validate, instantiate, and call in one step.
func (r *WasmRuntime) LoadAndRun(wasmBytes []byte, entry string, args []int) ([]int, error) {
	module, err := r.Load(wasmBytes)
	if err != nil {
		return nil, err
	}
	if _, err := r.Validate(module); err != nil {
		return nil, err
	}
	instance, err := r.Instantiate(module)
	if err != nil {
		return nil, err
	}
	return r.Call(instance, entry, args)
}
