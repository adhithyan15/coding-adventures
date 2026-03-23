package wasmtypes

// Tests for the WASM 1.0 type system data structures.
//
// These tests verify:
//   - ValueType and ExternalKind constants have the correct byte values
//     matching the WASM binary encoding specification
//   - BlockType.Empty == 0x40
//   - All struct types can be constructed and exhibit correct field values
//   - WasmModule starts with nil (zero-value) slices and can be populated
//
// Coverage goal: 100% — every exported constant and struct field is exercised.

import (
	"testing"
)

// ---------------------------------------------------------------------------
// Test 1 — ValueType byte values
//
// These exact values come from the WASM binary encoding spec. They are
// designed to be in the negative signed LEB128 range (-1 through -4) so
// the binary format can distinguish them from non-negative type indices.
// ---------------------------------------------------------------------------

func TestValueTypeI32(t *testing.T) {
	if ValueTypeI32 != 0x7F {
		t.Errorf("ValueTypeI32 = 0x%02X; want 0x7F", ValueTypeI32)
	}
}

func TestValueTypeI64(t *testing.T) {
	if ValueTypeI64 != 0x7E {
		t.Errorf("ValueTypeI64 = 0x%02X; want 0x7E", ValueTypeI64)
	}
}

func TestValueTypeF32(t *testing.T) {
	if ValueTypeF32 != 0x7D {
		t.Errorf("ValueTypeF32 = 0x%02X; want 0x7D", ValueTypeF32)
	}
}

func TestValueTypeF64(t *testing.T) {
	if ValueTypeF64 != 0x7C {
		t.Errorf("ValueTypeF64 = 0x%02X; want 0x7C", ValueTypeF64)
	}
}

func TestValueTypeIsEmbeddableInSlice(t *testing.T) {
	// ValueType is a byte, so it can be directly collected as a byte slice
	// and the byte values match the WASM encoding.
	encoded := []byte{byte(ValueTypeI32), byte(ValueTypeI64), byte(ValueTypeF32), byte(ValueTypeF64)}
	want := []byte{0x7F, 0x7E, 0x7D, 0x7C}
	for i, b := range want {
		if encoded[i] != b {
			t.Errorf("index %d: got 0x%02X; want 0x%02X", i, encoded[i], b)
		}
	}
}

// ---------------------------------------------------------------------------
// Test 2 — ExternalKind byte values (0–3)
// ---------------------------------------------------------------------------

func TestExternalKindFunction(t *testing.T) {
	if ExternalKindFunction != 0x00 {
		t.Errorf("ExternalKindFunction = 0x%02X; want 0x00", ExternalKindFunction)
	}
}

func TestExternalKindTable(t *testing.T) {
	if ExternalKindTable != 0x01 {
		t.Errorf("ExternalKindTable = 0x%02X; want 0x01", ExternalKindTable)
	}
}

func TestExternalKindMemory(t *testing.T) {
	if ExternalKindMemory != 0x02 {
		t.Errorf("ExternalKindMemory = 0x%02X; want 0x02", ExternalKindMemory)
	}
}

func TestExternalKindGlobal(t *testing.T) {
	if ExternalKindGlobal != 0x03 {
		t.Errorf("ExternalKindGlobal = 0x%02X; want 0x03", ExternalKindGlobal)
	}
}

func TestExternalKindSequential(t *testing.T) {
	// The four kinds are 0,1,2,3 — no gaps, no duplicates.
	kinds := []ExternalKind{ExternalKindFunction, ExternalKindTable, ExternalKindMemory, ExternalKindGlobal}
	for i, k := range kinds {
		if int(k) != i {
			t.Errorf("kinds[%d] = %d; want %d", i, k, i)
		}
	}
}

// ---------------------------------------------------------------------------
// Test 3 — BlockType.EMPTY == 0x40
//
// 0x40 is chosen to be outside the ValueType range (0x7C–0x7F) so the
// binary format can unambiguously identify an empty-result block.
// ---------------------------------------------------------------------------

func TestBlockTypeEmpty(t *testing.T) {
	if BlockTypeEmpty != 0x40 {
		t.Errorf("BlockTypeEmpty = 0x%02X; want 0x40", BlockTypeEmpty)
	}
}

func TestBlockTypeDistinctFromValueTypes(t *testing.T) {
	vts := []ValueType{ValueTypeI32, ValueTypeI64, ValueTypeF32, ValueTypeF64}
	for _, vt := range vts {
		if byte(BlockTypeEmpty) == byte(vt) {
			t.Errorf("BlockTypeEmpty (0x%02X) collides with ValueType (0x%02X)", BlockTypeEmpty, vt)
		}
	}
}

// ---------------------------------------------------------------------------
// Tests 4–6 — FuncType construction and equality
// ---------------------------------------------------------------------------

func TestFuncTypeBasicConstruction(t *testing.T) {
	// Test 4 — basic construction and equality
	ft := FuncType{
		Params:  []ValueType{ValueTypeI32},
		Results: []ValueType{ValueTypeI64},
	}
	if len(ft.Params) != 1 || ft.Params[0] != ValueTypeI32 {
		t.Errorf("Params = %v; want [I32]", ft.Params)
	}
	if len(ft.Results) != 1 || ft.Results[0] != ValueTypeI64 {
		t.Errorf("Results = %v; want [I64]", ft.Results)
	}
}

func TestFuncTypeEmptyParamsAndResults(t *testing.T) {
	// Test 5 — void->void function type
	ft := FuncType{Params: nil, Results: nil}
	if len(ft.Params) != 0 {
		t.Errorf("Params length = %d; want 0", len(ft.Params))
	}
	if len(ft.Results) != 0 {
		t.Errorf("Results length = %d; want 0", len(ft.Results))
	}
}

func TestFuncTypeMultipleParamsAndResults(t *testing.T) {
	// Test 6 — multiple params and results
	ft := FuncType{
		Params:  []ValueType{ValueTypeI32, ValueTypeI64, ValueTypeF32},
		Results: []ValueType{ValueTypeF64, ValueTypeI32},
	}
	if len(ft.Params) != 3 {
		t.Errorf("len(Params) = %d; want 3", len(ft.Params))
	}
	if len(ft.Results) != 2 {
		t.Errorf("len(Results) = %d; want 2", len(ft.Results))
	}
	if ft.Params[0] != ValueTypeI32 {
		t.Errorf("Params[0] = 0x%02X; want I32 (0x7F)", ft.Params[0])
	}
	if ft.Results[1] != ValueTypeI32 {
		t.Errorf("Results[1] = 0x%02X; want I32 (0x7F)", ft.Results[1])
	}
}

// ---------------------------------------------------------------------------
// Tests 7–8 — Limits
// ---------------------------------------------------------------------------

func TestLimitsMinOnly(t *testing.T) {
	// Test 7 — min with no max
	lim := Limits{Min: 1}
	if lim.Min != 1 {
		t.Errorf("Min = %d; want 1", lim.Min)
	}
	if lim.HasMax {
		t.Error("HasMax = true; want false for min-only limits")
	}
	if lim.Max != 0 {
		t.Errorf("Max = %d; want 0 (zero value when HasMax is false)", lim.Max)
	}
}

func TestLimitsMinAndMax(t *testing.T) {
	// Test 8 — min and max
	lim := Limits{Min: 0, Max: 10, HasMax: true}
	if lim.Min != 0 {
		t.Errorf("Min = %d; want 0", lim.Min)
	}
	if lim.Max != 10 {
		t.Errorf("Max = %d; want 10", lim.Max)
	}
	if !lim.HasMax {
		t.Error("HasMax = false; want true")
	}
}

// ---------------------------------------------------------------------------
// Test 9 — MemoryType construction
// ---------------------------------------------------------------------------

func TestMemoryTypeConstruction(t *testing.T) {
	mt := MemoryType{Limits: Limits{Min: 1, Max: 4, HasMax: true}}
	if mt.Limits.Min != 1 {
		t.Errorf("Limits.Min = %d; want 1", mt.Limits.Min)
	}
	if mt.Limits.Max != 4 {
		t.Errorf("Limits.Max = %d; want 4", mt.Limits.Max)
	}
}

func TestMemoryTypeUnbounded(t *testing.T) {
	mt := MemoryType{Limits: Limits{Min: 1}}
	if mt.Limits.HasMax {
		t.Error("HasMax = true; want false for unbounded memory")
	}
}

// ---------------------------------------------------------------------------
// Test 10 — TableType with default element_type 0x70
// ---------------------------------------------------------------------------

func TestTableTypeDefaultElementType(t *testing.T) {
	// Default zero-value would be 0, so we always set ElementTypeFuncRef.
	tt := TableType{ElementType: ElementTypeFuncRef, Limits: Limits{Min: 0}}
	if tt.ElementType != 0x70 {
		t.Errorf("ElementType = 0x%02X; want 0x70 (funcref)", tt.ElementType)
	}
}

func TestElementTypeFuncRefConstant(t *testing.T) {
	if ElementTypeFuncRef != 0x70 {
		t.Errorf("ElementTypeFuncRef = 0x%02X; want 0x70", ElementTypeFuncRef)
	}
}

func TestTableTypeWithLimits(t *testing.T) {
	tt := TableType{ElementType: ElementTypeFuncRef, Limits: Limits{Min: 0, Max: 100, HasMax: true}}
	if tt.Limits.Max != 100 {
		t.Errorf("Limits.Max = %d; want 100", tt.Limits.Max)
	}
}

// ---------------------------------------------------------------------------
// Test 11 — GlobalType mutable and immutable
// ---------------------------------------------------------------------------

func TestGlobalTypeImmutable(t *testing.T) {
	gt := GlobalType{ValueType: ValueTypeI32, Mutable: false}
	if gt.ValueType != ValueTypeI32 {
		t.Errorf("ValueType = 0x%02X; want I32 (0x7F)", gt.ValueType)
	}
	if gt.Mutable {
		t.Error("Mutable = true; want false")
	}
}

func TestGlobalTypeMutable(t *testing.T) {
	gt := GlobalType{ValueType: ValueTypeF64, Mutable: true}
	if !gt.Mutable {
		t.Error("Mutable = false; want true")
	}
}

// ---------------------------------------------------------------------------
// Test 12 — Import construction for each ExternalKind
// ---------------------------------------------------------------------------

func TestImportFunctionKind(t *testing.T) {
	imp := Import{
		ModuleName: "wasi_snapshot_preview1",
		Name:       "fd_write",
		Kind:       ExternalKindFunction,
		TypeInfo:   uint32(3),
	}
	if imp.Kind != ExternalKindFunction {
		t.Errorf("Kind = %d; want FUNCTION (0)", imp.Kind)
	}
	if idx, ok := imp.TypeInfo.(uint32); !ok || idx != 3 {
		t.Errorf("TypeInfo = %v; want uint32(3)", imp.TypeInfo)
	}
}

func TestImportTableKind(t *testing.T) {
	tt := TableType{ElementType: ElementTypeFuncRef, Limits: Limits{Min: 0}}
	imp := Import{
		ModuleName: "env",
		Name:       "table",
		Kind:       ExternalKindTable,
		TypeInfo:   tt,
	}
	if imp.Kind != ExternalKindTable {
		t.Errorf("Kind = %d; want TABLE (1)", imp.Kind)
	}
	if _, ok := imp.TypeInfo.(TableType); !ok {
		t.Errorf("TypeInfo is not TableType; got %T", imp.TypeInfo)
	}
}

func TestImportMemoryKind(t *testing.T) {
	mt := MemoryType{Limits: Limits{Min: 1}}
	imp := Import{
		ModuleName: "env",
		Name:       "memory",
		Kind:       ExternalKindMemory,
		TypeInfo:   mt,
	}
	if imp.Kind != ExternalKindMemory {
		t.Errorf("Kind = %d; want MEMORY (2)", imp.Kind)
	}
	if _, ok := imp.TypeInfo.(MemoryType); !ok {
		t.Errorf("TypeInfo is not MemoryType; got %T", imp.TypeInfo)
	}
}

func TestImportGlobalKind(t *testing.T) {
	gt := GlobalType{ValueType: ValueTypeI32, Mutable: true}
	imp := Import{
		ModuleName: "env",
		Name:       "__stack_pointer",
		Kind:       ExternalKindGlobal,
		TypeInfo:   gt,
	}
	if imp.Kind != ExternalKindGlobal {
		t.Errorf("Kind = %d; want GLOBAL (3)", imp.Kind)
	}
	got, ok := imp.TypeInfo.(GlobalType)
	if !ok {
		t.Errorf("TypeInfo is not GlobalType; got %T", imp.TypeInfo)
	}
	if !got.Mutable {
		t.Error("GlobalType.Mutable = false; want true")
	}
}

// ---------------------------------------------------------------------------
// Test 13 — Export construction
// ---------------------------------------------------------------------------

func TestExportConstruction(t *testing.T) {
	exp := Export{Name: "main", Kind: ExternalKindFunction, Index: 0}
	if exp.Name != "main" {
		t.Errorf("Name = %q; want \"main\"", exp.Name)
	}
	if exp.Kind != ExternalKindFunction {
		t.Errorf("Kind = %d; want FUNCTION (0)", exp.Kind)
	}
	if exp.Index != 0 {
		t.Errorf("Index = %d; want 0", exp.Index)
	}
}

func TestExportMemory(t *testing.T) {
	exp := Export{Name: "memory", Kind: ExternalKindMemory, Index: 0}
	if exp.Kind != ExternalKindMemory {
		t.Errorf("Kind = %d; want MEMORY (2)", exp.Kind)
	}
}

// ---------------------------------------------------------------------------
// Test 14 — Global with init_expr bytes
// ---------------------------------------------------------------------------

func TestGlobalConstruction(t *testing.T) {
	// i32.const 42 = 0x41 0x2A 0x0B
	g := Global{
		GlobalType: GlobalType{ValueType: ValueTypeI32, Mutable: true},
		InitExpr:   []byte{0x41, 0x2A, 0x0B},
	}
	if g.GlobalType.ValueType != ValueTypeI32 {
		t.Errorf("GlobalType.ValueType = 0x%02X; want I32 (0x7F)", g.GlobalType.ValueType)
	}
	if !g.GlobalType.Mutable {
		t.Error("GlobalType.Mutable = false; want true")
	}
	if len(g.InitExpr) != 3 || g.InitExpr[0] != 0x41 {
		t.Errorf("InitExpr = %v; want [0x41 0x2A 0x0B]", g.InitExpr)
	}
}

func TestGlobalImmutable(t *testing.T) {
	g := Global{
		GlobalType: GlobalType{ValueType: ValueTypeF64, Mutable: false},
		InitExpr:   []byte{0x44, 0, 0, 0, 0, 0, 0, 0, 0, 0x0B},
	}
	if g.GlobalType.Mutable {
		t.Error("Mutable = true; want false")
	}
}

// ---------------------------------------------------------------------------
// Test 15 — Element with function_indices slice
// ---------------------------------------------------------------------------

func TestElementConstruction(t *testing.T) {
	elem := Element{
		TableIndex:      0,
		OffsetExpr:      []byte{0x41, 0x00, 0x0B},
		FunctionIndices: []uint32{1, 2, 3},
	}
	if elem.TableIndex != 0 {
		t.Errorf("TableIndex = %d; want 0", elem.TableIndex)
	}
	if len(elem.FunctionIndices) != 3 {
		t.Errorf("len(FunctionIndices) = %d; want 3", len(elem.FunctionIndices))
	}
	if elem.FunctionIndices[0] != 1 || elem.FunctionIndices[2] != 3 {
		t.Errorf("FunctionIndices = %v; want [1 2 3]", elem.FunctionIndices)
	}
}

func TestElementEmptyIndices(t *testing.T) {
	elem := Element{
		TableIndex:      0,
		OffsetExpr:      []byte{0x41, 0x00, 0x0B},
		FunctionIndices: nil,
	}
	if len(elem.FunctionIndices) != 0 {
		t.Errorf("len(FunctionIndices) = %d; want 0", len(elem.FunctionIndices))
	}
}

// ---------------------------------------------------------------------------
// Test 16 — DataSegment construction
// ---------------------------------------------------------------------------

func TestDataSegmentConstruction(t *testing.T) {
	ds := DataSegment{
		MemoryIndex: 0,
		OffsetExpr:  []byte{0x41, 0x00, 0x0B},
		Data:        []byte("hello, wasm"),
	}
	if ds.MemoryIndex != 0 {
		t.Errorf("MemoryIndex = %d; want 0", ds.MemoryIndex)
	}
	if string(ds.Data) != "hello, wasm" {
		t.Errorf("Data = %q; want \"hello, wasm\"", ds.Data)
	}
}

func TestDataSegmentEmptyData(t *testing.T) {
	ds := DataSegment{MemoryIndex: 0, OffsetExpr: nil, Data: nil}
	if len(ds.Data) != 0 {
		t.Errorf("len(Data) = %d; want 0", len(ds.Data))
	}
}

// ---------------------------------------------------------------------------
// Test 17 — FunctionBody with locals and code
// ---------------------------------------------------------------------------

func TestFunctionBodyConstruction(t *testing.T) {
	fb := FunctionBody{
		Locals: []ValueType{ValueTypeI32, ValueTypeI32},
		Code:   []byte{0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B},
	}
	if len(fb.Locals) != 2 {
		t.Errorf("len(Locals) = %d; want 2", len(fb.Locals))
	}
	if fb.Locals[0] != ValueTypeI32 {
		t.Errorf("Locals[0] = 0x%02X; want I32 (0x7F)", fb.Locals[0])
	}
	if fb.Code[len(fb.Code)-1] != 0x0B {
		t.Errorf("last Code byte = 0x%02X; want 0x0B (end)", fb.Code[len(fb.Code)-1])
	}
}

func TestFunctionBodyNoLocals(t *testing.T) {
	fb := FunctionBody{Locals: nil, Code: []byte{0x0B}}
	if len(fb.Locals) != 0 {
		t.Errorf("len(Locals) = %d; want 0", len(fb.Locals))
	}
}

// ---------------------------------------------------------------------------
// Test 18 — CustomSection construction
// ---------------------------------------------------------------------------

func TestCustomSectionConstruction(t *testing.T) {
	cs := CustomSection{Name: "name", Data: []byte{0x00, 0x04, 'm', 'a', 'i', 'n'}}
	if cs.Name != "name" {
		t.Errorf("Name = %q; want \"name\"", cs.Name)
	}
	if len(cs.Data) != 6 {
		t.Errorf("len(Data) = %d; want 6", len(cs.Data))
	}
}

func TestCustomSectionEmptyData(t *testing.T) {
	cs := CustomSection{Name: "producers", Data: nil}
	if cs.Name != "producers" {
		t.Errorf("Name = %q; want \"producers\"", cs.Name)
	}
}

// ---------------------------------------------------------------------------
// Tests 19–20 — WasmModule
// ---------------------------------------------------------------------------

func TestWasmModuleStartsEmpty(t *testing.T) {
	// Test 19 — zero value has nil slices and nil Start
	m := WasmModule{}
	if len(m.Types) != 0 {
		t.Errorf("len(Types) = %d; want 0", len(m.Types))
	}
	if len(m.Imports) != 0 {
		t.Errorf("len(Imports) = %d; want 0", len(m.Imports))
	}
	if len(m.Functions) != 0 {
		t.Errorf("len(Functions) = %d; want 0", len(m.Functions))
	}
	if len(m.Tables) != 0 {
		t.Errorf("len(Tables) = %d; want 0", len(m.Tables))
	}
	if len(m.Memories) != 0 {
		t.Errorf("len(Memories) = %d; want 0", len(m.Memories))
	}
	if len(m.Globals) != 0 {
		t.Errorf("len(Globals) = %d; want 0", len(m.Globals))
	}
	if len(m.Exports) != 0 {
		t.Errorf("len(Exports) = %d; want 0", len(m.Exports))
	}
	if m.Start != nil {
		t.Errorf("Start = %v; want nil", m.Start)
	}
	if len(m.Elements) != 0 {
		t.Errorf("len(Elements) = %d; want 0", len(m.Elements))
	}
	if len(m.Code) != 0 {
		t.Errorf("len(Code) = %d; want 0", len(m.Code))
	}
	if len(m.Data) != 0 {
		t.Errorf("len(Data) = %d; want 0", len(m.Data))
	}
	if len(m.Customs) != 0 {
		t.Errorf("len(Customs) = %d; want 0", len(m.Customs))
	}
}

func TestWasmModuleCanBePopulated(t *testing.T) {
	// Test 20 — append to lists
	m := WasmModule{}

	// Types
	ft := FuncType{Params: []ValueType{ValueTypeI32}, Results: []ValueType{ValueTypeI32}}
	m.Types = append(m.Types, ft)
	if len(m.Types) != 1 {
		t.Errorf("len(Types) = %d; want 1", len(m.Types))
	}

	// Imports
	m.Imports = append(m.Imports, Import{
		ModuleName: "env",
		Name:       "mem",
		Kind:       ExternalKindMemory,
		TypeInfo:   MemoryType{Limits: Limits{Min: 1}},
	})
	if len(m.Imports) != 1 {
		t.Errorf("len(Imports) = %d; want 1", len(m.Imports))
	}

	// Functions
	m.Functions = append(m.Functions, 0)
	m.Functions = append(m.Functions, 1)
	if len(m.Functions) != 2 {
		t.Errorf("len(Functions) = %d; want 2", len(m.Functions))
	}

	// Tables
	m.Tables = append(m.Tables, TableType{ElementType: ElementTypeFuncRef, Limits: Limits{Min: 0}})
	if len(m.Tables) != 1 {
		t.Errorf("len(Tables) = %d; want 1", len(m.Tables))
	}

	// Memories
	m.Memories = append(m.Memories, MemoryType{Limits: Limits{Min: 1}})
	if len(m.Memories) != 1 {
		t.Errorf("len(Memories) = %d; want 1", len(m.Memories))
	}

	// Globals
	m.Globals = append(m.Globals, Global{
		GlobalType: GlobalType{ValueType: ValueTypeI32, Mutable: true},
		InitExpr:   []byte{0x41, 0x00, 0x0B},
	})
	if len(m.Globals) != 1 {
		t.Errorf("len(Globals) = %d; want 1", len(m.Globals))
	}

	// Exports
	m.Exports = append(m.Exports, Export{Name: "main", Kind: ExternalKindFunction, Index: 1})
	if len(m.Exports) != 1 {
		t.Errorf("len(Exports) = %d; want 1", len(m.Exports))
	}

	// Start
	startIdx := uint32(1)
	m.Start = &startIdx
	if m.Start == nil || *m.Start != 1 {
		t.Errorf("Start = %v; want pointer to 1", m.Start)
	}

	// Elements
	m.Elements = append(m.Elements, Element{
		TableIndex:      0,
		OffsetExpr:      []byte{0x41, 0x00, 0x0B},
		FunctionIndices: []uint32{1},
	})
	if len(m.Elements) != 1 {
		t.Errorf("len(Elements) = %d; want 1", len(m.Elements))
	}

	// Code
	m.Code = append(m.Code, FunctionBody{Locals: nil, Code: []byte{0x0B}})
	if len(m.Code) != 1 {
		t.Errorf("len(Code) = %d; want 1", len(m.Code))
	}

	// Data
	m.Data = append(m.Data, DataSegment{
		MemoryIndex: 0,
		OffsetExpr:  []byte{0x41, 0x00, 0x0B},
		Data:        []byte("hello"),
	})
	if len(m.Data) != 1 {
		t.Errorf("len(Data) = %d; want 1", len(m.Data))
	}

	// Customs
	m.Customs = append(m.Customs, CustomSection{Name: "name", Data: nil})
	if len(m.Customs) != 1 {
		t.Errorf("len(Customs) = %d; want 1", len(m.Customs))
	}
}

func TestWasmModuleIndependentInstances(t *testing.T) {
	// Two WasmModule instances must not share slice backing arrays.
	m1 := WasmModule{}
	m2 := WasmModule{}
	m1.Functions = append(m1.Functions, 0)
	if len(m2.Functions) != 0 {
		t.Error("m2.Functions was modified when m1.Functions was appended to")
	}
}

func TestWasmModuleStartNilByDefault(t *testing.T) {
	m := WasmModule{}
	if m.Start != nil {
		t.Error("Start should be nil by default (no Start section)")
	}
}

func TestWasmModuleStartCanBeSet(t *testing.T) {
	m := WasmModule{}
	idx := uint32(5)
	m.Start = &idx
	if m.Start == nil || *m.Start != 5 {
		t.Errorf("Start = %v; want pointer to 5", m.Start)
	}
}
