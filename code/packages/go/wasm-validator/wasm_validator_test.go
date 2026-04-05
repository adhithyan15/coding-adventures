package wasmvalidator

import (
	"testing"

	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
)

// ════════════════════════════════════════════════════════════════════════
// VALID MODULES
// ════════════════════════════════════════════════════════════════════════

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
	if validated.Module != module {
		t.Fatal("validated module should reference original")
	}
	if validated.IndexSpaces == nil {
		t.Fatal("index spaces should not be nil")
	}
}

func TestValidateEmptyModule(t *testing.T) {
	module := &wasmtypes.WasmModule{}
	_, err := Validate(module)
	if err != nil {
		t.Fatalf("empty module should validate, got: %v", err)
	}
}

func TestValidateModuleWithMemory(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Memories: []wasmtypes.MemoryType{
			{Limits: wasmtypes.Limits{Min: 1, Max: 10, HasMax: true}},
		},
	}
	_, err := Validate(module)
	if err != nil {
		t.Fatalf("should pass: %v", err)
	}
}

func TestValidateModuleWithTable(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Tables: []wasmtypes.TableType{
			{ElementType: 0x70, Limits: wasmtypes.Limits{Min: 1, Max: 10, HasMax: true}},
		},
	}
	_, err := Validate(module)
	if err != nil {
		t.Fatalf("should pass: %v", err)
	}
}

func TestValidateModuleWithStartFunction(t *testing.T) {
	startIdx := uint32(0)
	module := &wasmtypes.WasmModule{
		Types:     []wasmtypes.FuncType{{Params: nil, Results: nil}},
		Functions: []uint32{0},
		Code:      []wasmtypes.FunctionBody{{Locals: nil, Code: []byte{0x0B}}},
		Start:     &startIdx,
	}
	_, err := Validate(module)
	if err != nil {
		t.Fatalf("valid start function should pass: %v", err)
	}
}

// ════════════════════════════════════════════════════════════════════════
// MULTIPLE MEMORIES / TABLES
// ════════════════════════════════════════════════════════════════════════

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

func TestValidateMultipleTables(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Tables: []wasmtypes.TableType{
			{ElementType: 0x70, Limits: wasmtypes.Limits{Min: 1}},
			{ElementType: 0x70, Limits: wasmtypes.Limits{Min: 1}},
		},
	}
	_, err := Validate(module)
	if err == nil {
		t.Fatal("expected validation error for multiple tables")
	}
	ve, ok := err.(*ValidationError)
	if !ok || ve.Kind != ErrMultipleTables {
		t.Fatalf("expected ErrMultipleTables, got: %v", err)
	}
}

// ════════════════════════════════════════════════════════════════════════
// MEMORY LIMITS
// ════════════════════════════════════════════════════════════════════════

func TestValidateMemoryLimits(t *testing.T) {
	// min > max
	module := &wasmtypes.WasmModule{
		Memories: []wasmtypes.MemoryType{
			{Limits: wasmtypes.Limits{Min: 10, Max: 5, HasMax: true}},
		},
	}
	_, err := Validate(module)
	if err == nil {
		t.Fatal("expected validation error for memory min > max")
	}
	ve, ok := err.(*ValidationError)
	if !ok || ve.Kind != ErrMemoryLimitOrder {
		t.Fatalf("expected ErrMemoryLimitOrder, got: %v", err)
	}
}

func TestValidateMemoryMinExceedsMax(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Memories: []wasmtypes.MemoryType{
			{Limits: wasmtypes.Limits{Min: MaxMemoryPages + 1}},
		},
	}
	_, err := Validate(module)
	if err == nil {
		t.Fatal("expected validation error for min exceeding max pages")
	}
	ve, ok := err.(*ValidationError)
	if !ok || ve.Kind != ErrMemoryLimitExceeded {
		t.Fatalf("expected ErrMemoryLimitExceeded, got: %v", err)
	}
}

func TestValidateMemoryMaxExceedsSpecLimit(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Memories: []wasmtypes.MemoryType{
			{Limits: wasmtypes.Limits{Min: 1, Max: MaxMemoryPages + 1, HasMax: true}},
		},
	}
	_, err := Validate(module)
	if err == nil {
		t.Fatal("expected validation error for max exceeding spec limit")
	}
	ve, ok := err.(*ValidationError)
	if !ok || ve.Kind != ErrMemoryLimitExceeded {
		t.Fatalf("expected ErrMemoryLimitExceeded, got: %v", err)
	}
}

// ════════════════════════════════════════════════════════════════════════
// TABLE LIMITS
// ════════════════════════════════════════════════════════════════════════

func TestValidateTableLimitOrder(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Tables: []wasmtypes.TableType{
			{ElementType: 0x70, Limits: wasmtypes.Limits{Min: 10, Max: 5, HasMax: true}},
		},
	}
	_, err := Validate(module)
	if err == nil {
		t.Fatal("expected validation error for table min > max")
	}
	ve, ok := err.(*ValidationError)
	if !ok || ve.Kind != ErrTableLimitOrder {
		t.Fatalf("expected ErrTableLimitOrder, got: %v", err)
	}
}

// ════════════════════════════════════════════════════════════════════════
// DUPLICATE EXPORTS
// ════════════════════════════════════════════════════════════════════════

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

// ════════════════════════════════════════════════════════════════════════
// EXPORT INDEX OUT OF RANGE
// ════════════════════════════════════════════════════════════════════════

func TestValidateExportFuncIndexOOB(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types:     []wasmtypes.FuncType{{Params: nil, Results: nil}},
		Functions: []uint32{0},
		Code:      []wasmtypes.FunctionBody{{Locals: nil, Code: []byte{0x0B}}},
		Exports:   []wasmtypes.Export{{Name: "foo", Kind: wasmtypes.ExternalKindFunction, Index: 99}},
	}
	_, err := Validate(module)
	if err == nil {
		t.Fatal("expected error for func export OOB")
	}
	ve, ok := err.(*ValidationError)
	if !ok || ve.Kind != ErrExportIndexOutOfRange {
		t.Fatalf("expected ErrExportIndexOutOfRange, got: %v", err)
	}
}

func TestValidateExportTableIndexOOB(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Tables:  []wasmtypes.TableType{{ElementType: 0x70, Limits: wasmtypes.Limits{Min: 1}}},
		Exports: []wasmtypes.Export{{Name: "tbl", Kind: wasmtypes.ExternalKindTable, Index: 5}},
	}
	_, err := Validate(module)
	if err == nil {
		t.Fatal("expected error for table export OOB")
	}
	ve, ok := err.(*ValidationError)
	if !ok || ve.Kind != ErrExportIndexOutOfRange {
		t.Fatalf("expected ErrExportIndexOutOfRange, got: %v", err)
	}
}

func TestValidateExportMemoryIndexOOB(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Memories: []wasmtypes.MemoryType{{Limits: wasmtypes.Limits{Min: 1}}},
		Exports:  []wasmtypes.Export{{Name: "mem", Kind: wasmtypes.ExternalKindMemory, Index: 5}},
	}
	_, err := Validate(module)
	if err == nil {
		t.Fatal("expected error for memory export OOB")
	}
	ve, ok := err.(*ValidationError)
	if !ok || ve.Kind != ErrExportIndexOutOfRange {
		t.Fatalf("expected ErrExportIndexOutOfRange, got: %v", err)
	}
}

func TestValidateExportGlobalIndexOOB(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Globals: []wasmtypes.Global{
			{GlobalType: wasmtypes.GlobalType{ValueType: wasmtypes.ValueTypeI32, Mutable: false}, InitExpr: []byte{0x41, 0x00, 0x0B}},
		},
		Exports: []wasmtypes.Export{{Name: "g", Kind: wasmtypes.ExternalKindGlobal, Index: 5}},
	}
	_, err := Validate(module)
	if err == nil {
		t.Fatal("expected error for global export OOB")
	}
	ve, ok := err.(*ValidationError)
	if !ok || ve.Kind != ErrExportIndexOutOfRange {
		t.Fatalf("expected ErrExportIndexOutOfRange, got: %v", err)
	}
}

// ════════════════════════════════════════════════════════════════════════
// START FUNCTION VALIDATION
// ════════════════════════════════════════════════════════════════════════

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

func TestValidateStartFunctionWithResults(t *testing.T) {
	startIdx := uint32(0)
	module := &wasmtypes.WasmModule{
		Types:     []wasmtypes.FuncType{{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}}},
		Functions: []uint32{0},
		Code:      []wasmtypes.FunctionBody{{Locals: nil, Code: []byte{0x41, 0x00, 0x0B}}},
		Start:     &startIdx,
	}
	_, err := Validate(module)
	if err == nil {
		t.Fatal("expected validation error for start function with results")
	}
	ve, ok := err.(*ValidationError)
	if !ok || ve.Kind != ErrStartFunctionBadType {
		t.Fatalf("expected ErrStartFunctionBadType, got: %v", err)
	}
}

func TestValidateStartFunctionIndexOOB(t *testing.T) {
	startIdx := uint32(99)
	module := &wasmtypes.WasmModule{
		Types:     []wasmtypes.FuncType{{Params: nil, Results: nil}},
		Functions: []uint32{0},
		Code:      []wasmtypes.FunctionBody{{Locals: nil, Code: []byte{0x0B}}},
		Start:     &startIdx,
	}
	_, err := Validate(module)
	if err == nil {
		t.Fatal("expected validation error for start function index OOB")
	}
	ve, ok := err.(*ValidationError)
	if !ok || ve.Kind != ErrInvalidFuncIndex {
		t.Fatalf("expected ErrInvalidFuncIndex, got: %v", err)
	}
}

// ════════════════════════════════════════════════════════════════════════
// INVALID TYPE INDEX
// ════════════════════════════════════════════════════════════════════════

func TestValidateInvalidTypeIndex(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types:     []wasmtypes.FuncType{{Params: nil, Results: nil}},
		Functions: []uint32{5}, // type index 5 is OOB
		Code:      []wasmtypes.FunctionBody{{Locals: nil, Code: []byte{0x0B}}},
	}
	_, err := Validate(module)
	if err == nil {
		t.Fatal("expected validation error for invalid type index")
	}
	ve, ok := err.(*ValidationError)
	if !ok || ve.Kind != ErrInvalidTypeIndex {
		t.Fatalf("expected ErrInvalidTypeIndex, got: %v", err)
	}
}

// ════════════════════════════════════════════════════════════════════════
// DATA SEGMENT VALIDATION
// ════════════════════════════════════════════════════════════════════════

func TestValidateDataSegmentInvalidMemory(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Data: []wasmtypes.DataSegment{
			{MemoryIndex: 0, OffsetExpr: []byte{0x41, 0x00, 0x0B}, Data: []byte{1}},
		},
	}
	_, err := Validate(module)
	if err == nil {
		t.Fatal("expected validation error for data segment referencing non-existent memory")
	}
	ve, ok := err.(*ValidationError)
	if !ok || ve.Kind != ErrInvalidMemoryIndex {
		t.Fatalf("expected ErrInvalidMemoryIndex, got: %v", err)
	}
}

func TestValidateDataSegmentValidMemory(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Memories: []wasmtypes.MemoryType{{Limits: wasmtypes.Limits{Min: 1}}},
		Data: []wasmtypes.DataSegment{
			{MemoryIndex: 0, OffsetExpr: []byte{0x41, 0x00, 0x0B}, Data: []byte{1, 2, 3}},
		},
	}
	_, err := Validate(module)
	if err != nil {
		t.Fatalf("valid data segment should pass: %v", err)
	}
}

// ════════════════════════════════════════════════════════════════════════
// ELEMENT SEGMENT VALIDATION
// ════════════════════════════════════════════════════════════════════════

func TestValidateElementInvalidTable(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types:     []wasmtypes.FuncType{{Params: nil, Results: nil}},
		Functions: []uint32{0},
		Code:      []wasmtypes.FunctionBody{{Locals: nil, Code: []byte{0x0B}}},
		Elements: []wasmtypes.Element{
			{TableIndex: 0, OffsetExpr: []byte{0x41, 0x00, 0x0B}, FunctionIndices: []uint32{0}},
		},
	}
	_, err := Validate(module)
	if err == nil {
		t.Fatal("expected validation error for element referencing non-existent table")
	}
	ve, ok := err.(*ValidationError)
	if !ok || ve.Kind != ErrInvalidTableIndex {
		t.Fatalf("expected ErrInvalidTableIndex, got: %v", err)
	}
}

func TestValidateElementInvalidFuncIndex(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types:     []wasmtypes.FuncType{{Params: nil, Results: nil}},
		Functions: []uint32{0},
		Code:      []wasmtypes.FunctionBody{{Locals: nil, Code: []byte{0x0B}}},
		Tables:    []wasmtypes.TableType{{ElementType: 0x70, Limits: wasmtypes.Limits{Min: 4}}},
		Elements: []wasmtypes.Element{
			{TableIndex: 0, OffsetExpr: []byte{0x41, 0x00, 0x0B}, FunctionIndices: []uint32{99}},
		},
	}
	_, err := Validate(module)
	if err == nil {
		t.Fatal("expected validation error for element with invalid func index")
	}
	ve, ok := err.(*ValidationError)
	if !ok || ve.Kind != ErrInvalidFuncIndex {
		t.Fatalf("expected ErrInvalidFuncIndex, got: %v", err)
	}
}

func TestValidateElementValid(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types:     []wasmtypes.FuncType{{Params: nil, Results: nil}},
		Functions: []uint32{0},
		Code:      []wasmtypes.FunctionBody{{Locals: nil, Code: []byte{0x0B}}},
		Tables:    []wasmtypes.TableType{{ElementType: 0x70, Limits: wasmtypes.Limits{Min: 4}}},
		Elements: []wasmtypes.Element{
			{TableIndex: 0, OffsetExpr: []byte{0x41, 0x00, 0x0B}, FunctionIndices: []uint32{0}},
		},
	}
	_, err := Validate(module)
	if err != nil {
		t.Fatalf("valid element segment should pass: %v", err)
	}
}

// ════════════════════════════════════════════════════════════════════════
// INDEX SPACES WITH IMPORTS
// ════════════════════════════════════════════════════════════════════════

func TestBuildIndexSpacesWithImports(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types: []wasmtypes.FuncType{
			{Params: nil, Results: nil},
			{Params: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}},
		},
		Imports: []wasmtypes.Import{
			{ModuleName: "env", Name: "fn", Kind: wasmtypes.ExternalKindFunction, TypeInfo: uint32(0)},
			{ModuleName: "env", Name: "tbl", Kind: wasmtypes.ExternalKindTable, TypeInfo: wasmtypes.TableType{ElementType: 0x70, Limits: wasmtypes.Limits{Min: 1}}},
			{ModuleName: "env", Name: "mem", Kind: wasmtypes.ExternalKindMemory, TypeInfo: wasmtypes.MemoryType{Limits: wasmtypes.Limits{Min: 1}}},
			{ModuleName: "env", Name: "g1", Kind: wasmtypes.ExternalKindGlobal, TypeInfo: wasmtypes.GlobalType{ValueType: wasmtypes.ValueTypeI32, Mutable: false}},
		},
		Functions: []uint32{1},
		Code:      []wasmtypes.FunctionBody{{Locals: nil, Code: []byte{0x20, 0x00, 0x0B}}},
	}

	spaces := buildIndexSpaces(module)

	if spaces.NumImportedFuncs != 1 {
		t.Fatalf("expected 1 imported func, got %d", spaces.NumImportedFuncs)
	}
	if len(spaces.FuncTypes) != 2 {
		t.Fatalf("expected 2 total func types, got %d", len(spaces.FuncTypes))
	}
	if spaces.NumImportedTables != 1 {
		t.Fatalf("expected 1 imported table, got %d", spaces.NumImportedTables)
	}
	if spaces.NumImportedMemories != 1 {
		t.Fatalf("expected 1 imported memory, got %d", spaces.NumImportedMemories)
	}
	if spaces.NumImportedGlobals != 1 {
		t.Fatalf("expected 1 imported global, got %d", spaces.NumImportedGlobals)
	}
	if spaces.NumTypes != 2 {
		t.Fatalf("expected 2 types, got %d", spaces.NumTypes)
	}
}

// ════════════════════════════════════════════════════════════════════════
// VALIDATION ERROR TYPE
// ════════════════════════════════════════════════════════════════════════

func TestValidationErrorString(t *testing.T) {
	ve := &ValidationError{
		Kind:    ErrMultipleMemories,
		Message: "too many memories",
	}
	expected := "ValidationError(multiple_memories): too many memories"
	if ve.Error() != expected {
		t.Fatalf("expected %q, got %q", expected, ve.Error())
	}
}

// ════════════════════════════════════════════════════════════════════════
// VALIDATE WITH GLOBALS
// ════════════════════════════════════════════════════════════════════════

func TestValidateModuleWithGlobals(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Globals: []wasmtypes.Global{
			{GlobalType: wasmtypes.GlobalType{ValueType: wasmtypes.ValueTypeI32, Mutable: true}, InitExpr: []byte{0x41, 0x00, 0x0B}},
			{GlobalType: wasmtypes.GlobalType{ValueType: wasmtypes.ValueTypeI64, Mutable: false}, InitExpr: []byte{0x42, 0x00, 0x0B}},
		},
	}
	validated, err := Validate(module)
	if err != nil {
		t.Fatalf("should pass: %v", err)
	}
	if len(validated.IndexSpaces.GlobalTypes) != 2 {
		t.Fatalf("expected 2 globals, got %d", len(validated.IndexSpaces.GlobalTypes))
	}
}

// ════════════════════════════════════════════════════════════════════════
// COMPLEX MODULE (multi-section)
// ════════════════════════════════════════════════════════════════════════

func TestValidateComplexModule(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types: []wasmtypes.FuncType{
			{Params: nil, Results: nil},
			{Params: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}},
		},
		Functions: []uint32{0, 1},
		Code: []wasmtypes.FunctionBody{
			{Locals: nil, Code: []byte{0x0B}},
			{Locals: nil, Code: []byte{0x20, 0x00, 0x0B}},
		},
		Memories: []wasmtypes.MemoryType{{Limits: wasmtypes.Limits{Min: 1, Max: 100, HasMax: true}}},
		Tables:   []wasmtypes.TableType{{ElementType: 0x70, Limits: wasmtypes.Limits{Min: 4}}},
		Globals: []wasmtypes.Global{
			{GlobalType: wasmtypes.GlobalType{ValueType: wasmtypes.ValueTypeI32, Mutable: true}, InitExpr: []byte{0x41, 0x00, 0x0B}},
		},
		Exports: []wasmtypes.Export{
			{Name: "init", Kind: wasmtypes.ExternalKindFunction, Index: 0},
			{Name: "add", Kind: wasmtypes.ExternalKindFunction, Index: 1},
			{Name: "mem", Kind: wasmtypes.ExternalKindMemory, Index: 0},
			{Name: "tbl", Kind: wasmtypes.ExternalKindTable, Index: 0},
			{Name: "g", Kind: wasmtypes.ExternalKindGlobal, Index: 0},
		},
		Data: []wasmtypes.DataSegment{
			{MemoryIndex: 0, OffsetExpr: []byte{0x41, 0x00, 0x0B}, Data: []byte{1, 2, 3}},
		},
		Elements: []wasmtypes.Element{
			{TableIndex: 0, OffsetExpr: []byte{0x41, 0x00, 0x0B}, FunctionIndices: []uint32{0}},
		},
	}

	validated, err := Validate(module)
	if err != nil {
		t.Fatalf("complex module should validate: %v", err)
	}
	if len(validated.FuncTypes) != 2 {
		t.Fatalf("expected 2 func types, got %d", len(validated.FuncTypes))
	}
}

// ════════════════════════════════════════════════════════════════════════
// TABLE LIMITS (valid, no max)
// ════════════════════════════════════════════════════════════════════════

func TestValidateTableLimitsNoMax(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Tables: []wasmtypes.TableType{
			{ElementType: 0x70, Limits: wasmtypes.Limits{Min: 10}},
		},
	}
	_, err := Validate(module)
	if err != nil {
		t.Fatalf("table without max should validate: %v", err)
	}
}

// ════════════════════════════════════════════════════════════════════════
// MEMORY LIMITS (valid, no max)
// ════════════════════════════════════════════════════════════════════════

func TestValidateMemoryLimitsNoMax(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Memories: []wasmtypes.MemoryType{
			{Limits: wasmtypes.Limits{Min: 1}},
		},
	}
	_, err := Validate(module)
	if err != nil {
		t.Fatalf("memory without max should validate: %v", err)
	}
}
