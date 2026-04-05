package wasmvalidator

import (
	"testing"

	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
)

// TestValidateMinimalModule tests validation of a minimal valid module.
func TestValidateMinimalModule(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types:     []wasmtypes.FuncType{{Params: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}}},
		Functions: []uint32{0},
		Code:      []wasmtypes.FunctionBody{{Locals: nil, Code: []byte{0x20, 0x00, 0x0B}}},
		Exports:   []wasmtypes.Export{{Name: "test", Kind: wasmtypes.ExternalKindFunction, Index: 0}},
	}

	validated, err := Validate(module)
	if err != nil {
		t.Fatalf("expected validation to pass, got: %v", err)
	}
	if len(validated.FuncTypes) != 1 {
		t.Fatalf("expected 1 func type, got %d", len(validated.FuncTypes))
	}
}

// TestValidateMultipleMemories tests that multiple memories are rejected.
func TestValidateMultipleMemories(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Memories: []wasmtypes.MemoryType{
			{Limits: wasmtypes.Limits{Min: 1}},
			{Limits: wasmtypes.Limits{Min: 1}},
		},
	}
	_, err := Validate(module)
	if err == nil {
		t.Fatal("expected validation error for multiple memories")
	}
	ve, ok := err.(*ValidationError)
	if !ok || ve.Kind != ErrMultipleMemories {
		t.Fatalf("expected ErrMultipleMemories, got: %v", err)
	}
}

// TestValidateDuplicateExport tests that duplicate export names are rejected.
func TestValidateDuplicateExport(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types:     []wasmtypes.FuncType{{Params: nil, Results: nil}},
		Functions: []uint32{0, 0},
		Code: []wasmtypes.FunctionBody{
			{Locals: nil, Code: []byte{0x0B}},
			{Locals: nil, Code: []byte{0x0B}},
		},
		Exports: []wasmtypes.Export{
			{Name: "foo", Kind: wasmtypes.ExternalKindFunction, Index: 0},
			{Name: "foo", Kind: wasmtypes.ExternalKindFunction, Index: 1},
		},
	}
	_, err := Validate(module)
	if err == nil {
		t.Fatal("expected validation error for duplicate exports")
	}
	ve, ok := err.(*ValidationError)
	if !ok || ve.Kind != ErrDuplicateExportName {
		t.Fatalf("expected ErrDuplicateExportName, got: %v", err)
	}
}

// TestValidateStartFunctionBadType tests that a start function with params is rejected.
func TestValidateStartFunctionBadType(t *testing.T) {
	startIdx := uint32(0)
	module := &wasmtypes.WasmModule{
		Types:     []wasmtypes.FuncType{{Params: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}, Results: nil}},
		Functions: []uint32{0},
		Code:      []wasmtypes.FunctionBody{{Locals: nil, Code: []byte{0x0B}}},
		Start:     &startIdx,
	}
	_, err := Validate(module)
	if err == nil {
		t.Fatal("expected validation error for start function with params")
	}
	ve, ok := err.(*ValidationError)
	if !ok || ve.Kind != ErrStartFunctionBadType {
		t.Fatalf("expected ErrStartFunctionBadType, got: %v", err)
	}
}

// TestValidateMemoryLimits tests memory limit validation.
func TestValidateMemoryLimits(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Memories: []wasmtypes.MemoryType{
			{Limits: wasmtypes.Limits{Min: 10, Max: 5, HasMax: true}},
		},
	}
	_, err := Validate(module)
	if err == nil {
		t.Fatal("expected validation error for memory min > max")
	}
}
