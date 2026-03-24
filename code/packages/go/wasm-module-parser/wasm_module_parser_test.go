package wasmmoduleparser

// Tests for wasm-module-parser.
//
// These tests construct minimal .wasm binaries by hand, parse them with
// Parser.Parse, and verify the resulting WasmModule fields.
//
// Binary construction helpers
// ----------------------------
// makeWasm assembles a complete .wasm binary from (sectionID, payload) pairs.
// leb128 encodes an unsigned integer as LEB128 bytes.
// These helpers exactly mirror the binary format, making tests easy to read
// and debug.

import (
	"errors"
	"testing"

	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
)

// ---------------------------------------------------------------------------
// BINARY CONSTRUCTION HELPERS
// ---------------------------------------------------------------------------

// wasmHeader is the 8-byte WASM module header.
var wasmHeader = []byte{0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00}

// leb128 encodes a non-negative integer as unsigned LEB128.
//
// This mirrors wasm_leb128.EncodeUnsigned but is inlined here so tests
// don't need a separate import just for binary construction.
//
// Example:
//
//	leb128(0)    → [0x00]
//	leb128(128)  → [0x80, 0x01]
//	leb128(300)  → [0xAC, 0x02]
func leb128(value uint64) []byte {
	var out []byte
	for {
		payload := byte(value & 0x7F)
		value >>= 7
		if value != 0 {
			out = append(out, payload|0x80)
		} else {
			out = append(out, payload)
			break
		}
	}
	return out
}

// makeName encodes a WASM name: LEB128-length + UTF-8 bytes.
func makeName(s string) []byte {
	b := []byte(s)
	return append(leb128(uint64(len(b))), b...)
}

// makeLimits encodes a WASM Limits struct.
// If max < 0, no maximum is encoded (flags = 0x00).
// If max >= 0, maximum is encoded (flags = 0x01).
func makeLimits(min uint32, max int64) []byte {
	if max < 0 {
		return append([]byte{0x00}, leb128(uint64(min))...)
	}
	out := []byte{0x01}
	out = append(out, leb128(uint64(min))...)
	out = append(out, leb128(uint64(max))...)
	return out
}

// makeFuncType encodes one FuncType entry: 0x60 + params + results.
func makeFuncType(params, results []byte) []byte {
	out := []byte{0x60}
	out = append(out, leb128(uint64(len(params)))...)
	out = append(out, params...)
	out = append(out, leb128(uint64(len(results)))...)
	out = append(out, results...)
	return out
}

// makeInitExprI32 encodes an i32.const N init_expr: 0x41 + leb128(N) + 0x0B.
func makeInitExprI32(n uint32) []byte {
	out := []byte{0x41}
	out = append(out, leb128(uint64(n))...)
	out = append(out, 0x0B)
	return out
}

// makeSection assembles a section envelope: [id][leb128(size)][payload].
func makeSection(id byte, payload []byte) []byte {
	out := []byte{id}
	out = append(out, leb128(uint64(len(payload)))...)
	out = append(out, payload...)
	return out
}

// makeWasm assembles a complete .wasm binary from section envelopes.
func makeWasm(sections ...[]byte) []byte {
	out := make([]byte, len(wasmHeader))
	copy(out, wasmHeader)
	for _, s := range sections {
		out = append(out, s...)
	}
	return out
}

// ---------------------------------------------------------------------------
// HELPER: assert ParseError
// ---------------------------------------------------------------------------

func assertParseError(t *testing.T, err error) *ParseError {
	t.Helper()
	if err == nil {
		t.Fatal("expected ParseError, got nil")
	}
	var pe *ParseError
	if !errors.As(err, &pe) {
		t.Fatalf("expected *ParseError, got %T: %v", err, err)
	}
	return pe
}

// ---------------------------------------------------------------------------
// TESTS
// ---------------------------------------------------------------------------

// TestPackageLoads verifies the package compiles and can be imported.
func TestPackageLoads(t *testing.T) {
	t.Log("wasm-module-parser package loaded successfully")
}

// TestParserNew verifies New() returns a non-nil parser.
func TestParserNew(t *testing.T) {
	p := New()
	if p == nil {
		t.Fatal("New() returned nil")
	}
}

// ---------------------------------------------------------------------------
// Test 1: Minimal module (header only)
// ---------------------------------------------------------------------------

// TestMinimalModule verifies parsing a module with just the 8-byte header.
// All WasmModule fields should be nil/zero.
func TestMinimalModule(t *testing.T) {
	module, err := New().Parse(wasmHeader)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if module == nil {
		t.Fatal("module is nil")
	}
	if len(module.Types) != 0 {
		t.Errorf("expected 0 types, got %d", len(module.Types))
	}
	if len(module.Imports) != 0 {
		t.Errorf("expected 0 imports, got %d", len(module.Imports))
	}
	if len(module.Functions) != 0 {
		t.Errorf("expected 0 functions, got %d", len(module.Functions))
	}
	if len(module.Tables) != 0 {
		t.Errorf("expected 0 tables, got %d", len(module.Tables))
	}
	if len(module.Memories) != 0 {
		t.Errorf("expected 0 memories, got %d", len(module.Memories))
	}
	if len(module.Globals) != 0 {
		t.Errorf("expected 0 globals, got %d", len(module.Globals))
	}
	if len(module.Exports) != 0 {
		t.Errorf("expected 0 exports, got %d", len(module.Exports))
	}
	if module.Start != nil {
		t.Errorf("expected nil start, got %d", *module.Start)
	}
	if len(module.Elements) != 0 {
		t.Errorf("expected 0 elements, got %d", len(module.Elements))
	}
	if len(module.Code) != 0 {
		t.Errorf("expected 0 code bodies, got %d", len(module.Code))
	}
	if len(module.Data) != 0 {
		t.Errorf("expected 0 data segments, got %d", len(module.Data))
	}
	if len(module.Customs) != 0 {
		t.Errorf("expected 0 customs, got %d", len(module.Customs))
	}
}

// ---------------------------------------------------------------------------
// Test 2: Type section
// ---------------------------------------------------------------------------

// TestTypeSectionI32I32ToI32 parses a type section with (i32,i32)→i32.
func TestTypeSectionI32I32ToI32(t *testing.T) {
	// Payload: count=1, functype: 0x60, 2 params (i32,i32), 1 result (i32)
	typePayload := append(leb128(1), makeFuncType([]byte{0x7F, 0x7F}, []byte{0x7F})...)
	module, err := New().Parse(makeWasm(makeSection(1, typePayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(module.Types) != 1 {
		t.Fatalf("expected 1 type, got %d", len(module.Types))
	}
	ft := module.Types[0]
	if len(ft.Params) != 2 || ft.Params[0] != wasmtypes.ValueTypeI32 || ft.Params[1] != wasmtypes.ValueTypeI32 {
		t.Errorf("wrong params: %v", ft.Params)
	}
	if len(ft.Results) != 1 || ft.Results[0] != wasmtypes.ValueTypeI32 {
		t.Errorf("wrong results: %v", ft.Results)
	}
}

// TestTypeSectionEmpty parses a type section with 0 entries.
func TestTypeSectionEmpty(t *testing.T) {
	typePayload := leb128(0)
	module, err := New().Parse(makeWasm(makeSection(1, typePayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(module.Types) != 0 {
		t.Errorf("expected 0 types, got %d", len(module.Types))
	}
}

// TestTypeSectionVoidToVoid parses a void→void function type.
func TestTypeSectionVoidToVoid(t *testing.T) {
	typePayload := append(leb128(1), makeFuncType([]byte{}, []byte{})...)
	module, err := New().Parse(makeWasm(makeSection(1, typePayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(module.Types[0].Params) != 0 {
		t.Errorf("expected 0 params")
	}
	if len(module.Types[0].Results) != 0 {
		t.Errorf("expected 0 results")
	}
}

// TestTypeSectionMultipleTypes parses two function types.
func TestTypeSectionMultipleTypes(t *testing.T) {
	typePayload := append(leb128(2),
		makeFuncType([]byte{0x7F}, []byte{0x7E})...) // (i32)→i64
	typePayload = append(typePayload,
		makeFuncType([]byte{0x7D, 0x7C}, []byte{})...) // (f32,f64)→()
	module, err := New().Parse(makeWasm(makeSection(1, typePayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(module.Types) != 2 {
		t.Fatalf("expected 2 types, got %d", len(module.Types))
	}
	if module.Types[0].Params[0] != wasmtypes.ValueTypeI32 {
		t.Errorf("wrong first type params")
	}
	if module.Types[0].Results[0] != wasmtypes.ValueTypeI64 {
		t.Errorf("wrong first type results")
	}
}

// ---------------------------------------------------------------------------
// Test 3: Function section
// ---------------------------------------------------------------------------

// TestFunctionSection parses a function section with one type index.
func TestFunctionSection(t *testing.T) {
	funcPayload := append(leb128(1), leb128(0)...)
	module, err := New().Parse(makeWasm(makeSection(3, funcPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(module.Functions) != 1 || module.Functions[0] != 0 {
		t.Errorf("expected [0], got %v", module.Functions)
	}
}

// TestFunctionSectionMultiple parses three function type indices.
func TestFunctionSectionMultiple(t *testing.T) {
	funcPayload := leb128(3)
	funcPayload = append(funcPayload, leb128(0)...)
	funcPayload = append(funcPayload, leb128(1)...)
	funcPayload = append(funcPayload, leb128(0)...)
	module, err := New().Parse(makeWasm(makeSection(3, funcPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(module.Functions) != 3 {
		t.Fatalf("expected 3 functions, got %d", len(module.Functions))
	}
	if module.Functions[1] != 1 {
		t.Errorf("expected Functions[1]==1, got %d", module.Functions[1])
	}
}

// ---------------------------------------------------------------------------
// Test 4: Export section
// ---------------------------------------------------------------------------

// TestExportSectionFunction parses an export of function 'main' at index 0.
func TestExportSectionFunction(t *testing.T) {
	exportPayload := leb128(1)
	exportPayload = append(exportPayload, makeName("main")...)
	exportPayload = append(exportPayload, 0x00) // ExternalKindFunction
	exportPayload = append(exportPayload, leb128(0)...)
	module, err := New().Parse(makeWasm(makeSection(7, exportPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(module.Exports) != 1 {
		t.Fatalf("expected 1 export, got %d", len(module.Exports))
	}
	exp := module.Exports[0]
	if exp.Name != "main" {
		t.Errorf("expected name 'main', got %q", exp.Name)
	}
	if exp.Kind != wasmtypes.ExternalKindFunction {
		t.Errorf("expected FUNCTION kind, got %v", exp.Kind)
	}
	if exp.Index != 0 {
		t.Errorf("expected index 0, got %d", exp.Index)
	}
}

// TestExportSectionMemory parses an export of memory at index 0.
func TestExportSectionMemory(t *testing.T) {
	exportPayload := leb128(1)
	exportPayload = append(exportPayload, makeName("memory")...)
	exportPayload = append(exportPayload, 0x02) // ExternalKindMemory
	exportPayload = append(exportPayload, leb128(0)...)
	module, err := New().Parse(makeWasm(makeSection(7, exportPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if module.Exports[0].Kind != wasmtypes.ExternalKindMemory {
		t.Errorf("expected MEMORY kind")
	}
}

// ---------------------------------------------------------------------------
// Test 5: Code section
// ---------------------------------------------------------------------------

// TestCodeSectionNoLocals parses a code section with one body having no locals.
func TestCodeSectionNoLocals(t *testing.T) {
	// body: 0 local decls + end (0x0B)
	body := append(leb128(0), 0x0B)
	codePayload := leb128(1)
	codePayload = append(codePayload, leb128(uint64(len(body)))...)
	codePayload = append(codePayload, body...)

	module, err := New().Parse(makeWasm(makeSection(10, codePayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(module.Code) != 1 {
		t.Fatalf("expected 1 code body, got %d", len(module.Code))
	}
	fb := module.Code[0]
	if len(fb.Locals) != 0 {
		t.Errorf("expected 0 locals, got %d", len(fb.Locals))
	}
	if len(fb.Code) != 1 || fb.Code[0] != 0x0B {
		t.Errorf("expected code=[0x0B], got %v", fb.Code)
	}
}

// TestCodeSectionWithLocals parses a body with two i32 locals.
//
//	Local encoding: 1 group → (2, i32)
//	Code: local.get 0, local.get 1, i32.add, end
func TestCodeSectionWithLocals(t *testing.T) {
	codeBytes := []byte{0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B}
	// 1 local decl group: 2 × i32
	localsEnc := leb128(1)
	localsEnc = append(localsEnc, leb128(2)...)
	localsEnc = append(localsEnc, 0x7F) // i32
	body := append(localsEnc, codeBytes...)

	codePayload := leb128(1)
	codePayload = append(codePayload, leb128(uint64(len(body)))...)
	codePayload = append(codePayload, body...)

	module, err := New().Parse(makeWasm(makeSection(10, codePayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	fb := module.Code[0]
	if len(fb.Locals) != 2 {
		t.Fatalf("expected 2 locals, got %d", len(fb.Locals))
	}
	if fb.Locals[0] != wasmtypes.ValueTypeI32 || fb.Locals[1] != wasmtypes.ValueTypeI32 {
		t.Errorf("wrong local types: %v", fb.Locals)
	}
	if string(fb.Code) != string(codeBytes) {
		t.Errorf("wrong code: %v", fb.Code)
	}
}

// TestCodeSectionMultipleLocalGroups parses a body with two local-decl groups.
func TestCodeSectionMultipleLocalGroups(t *testing.T) {
	// 2 groups: (1, i32) and (1, f64)
	localsEnc := leb128(2)
	localsEnc = append(localsEnc, leb128(1)...)
	localsEnc = append(localsEnc, 0x7F) // i32
	localsEnc = append(localsEnc, leb128(1)...)
	localsEnc = append(localsEnc, 0x7C) // f64
	body := append(localsEnc, 0x0B)

	codePayload := leb128(1)
	codePayload = append(codePayload, leb128(uint64(len(body)))...)
	codePayload = append(codePayload, body...)

	module, err := New().Parse(makeWasm(makeSection(10, codePayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	fb := module.Code[0]
	if len(fb.Locals) != 2 {
		t.Fatalf("expected 2 locals, got %d", len(fb.Locals))
	}
	if fb.Locals[0] != wasmtypes.ValueTypeI32 || fb.Locals[1] != wasmtypes.ValueTypeF64 {
		t.Errorf("wrong local types: %v", fb.Locals)
	}
}

// ---------------------------------------------------------------------------
// Test 6: Import section
// ---------------------------------------------------------------------------

// TestImportSectionFunction parses a function import: env::add, type 0.
func TestImportSectionFunction(t *testing.T) {
	impPayload := leb128(1)
	impPayload = append(impPayload, makeName("env")...)
	impPayload = append(impPayload, makeName("add")...)
	impPayload = append(impPayload, 0x00) // ExternalKindFunction
	impPayload = append(impPayload, leb128(0)...)

	module, err := New().Parse(makeWasm(makeSection(2, impPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(module.Imports) != 1 {
		t.Fatalf("expected 1 import, got %d", len(module.Imports))
	}
	imp := module.Imports[0]
	if imp.ModuleName != "env" || imp.Name != "add" {
		t.Errorf("wrong names: %q %q", imp.ModuleName, imp.Name)
	}
	if imp.Kind != wasmtypes.ExternalKindFunction {
		t.Errorf("wrong kind: %v", imp.Kind)
	}
	if idx, ok := imp.TypeInfo.(uint32); !ok || idx != 0 {
		t.Errorf("wrong type_info: %v", imp.TypeInfo)
	}
}

// TestImportSectionMemory parses a memory import with min=1, no max.
func TestImportSectionMemory(t *testing.T) {
	impPayload := leb128(1)
	impPayload = append(impPayload, makeName("env")...)
	impPayload = append(impPayload, makeName("memory")...)
	impPayload = append(impPayload, 0x02)              // ExternalKindMemory
	impPayload = append(impPayload, makeLimits(1, -1)...) // min=1, no max

	module, err := New().Parse(makeWasm(makeSection(2, impPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	imp := module.Imports[0]
	if imp.Kind != wasmtypes.ExternalKindMemory {
		t.Errorf("expected MEMORY kind")
	}
	mt, ok := imp.TypeInfo.(wasmtypes.MemoryType)
	if !ok {
		t.Fatalf("TypeInfo is not MemoryType")
	}
	if mt.Limits.Min != 1 || mt.Limits.HasMax {
		t.Errorf("wrong limits: %+v", mt.Limits)
	}
}

// TestImportSectionGlobal parses an immutable i32 global import.
func TestImportSectionGlobal(t *testing.T) {
	impPayload := leb128(1)
	impPayload = append(impPayload, makeName("env")...)
	impPayload = append(impPayload, makeName("g")...)
	impPayload = append(impPayload, 0x03) // ExternalKindGlobal
	impPayload = append(impPayload, 0x7F) // ValueTypeI32
	impPayload = append(impPayload, 0x00) // immutable

	module, err := New().Parse(makeWasm(makeSection(2, impPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	gt, ok := module.Imports[0].TypeInfo.(wasmtypes.GlobalType)
	if !ok {
		t.Fatalf("TypeInfo is not GlobalType")
	}
	if gt.ValueType != wasmtypes.ValueTypeI32 || gt.Mutable {
		t.Errorf("wrong global type: %+v", gt)
	}
}

// TestImportSectionTable parses a funcref table import with min=0, no max.
func TestImportSectionTable(t *testing.T) {
	impPayload := leb128(1)
	impPayload = append(impPayload, makeName("env")...)
	impPayload = append(impPayload, makeName("tbl")...)
	impPayload = append(impPayload, 0x01)              // ExternalKindTable
	impPayload = append(impPayload, 0x70)              // funcref
	impPayload = append(impPayload, makeLimits(0, -1)...) // min=0, no max

	module, err := New().Parse(makeWasm(makeSection(2, impPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	tt, ok := module.Imports[0].TypeInfo.(wasmtypes.TableType)
	if !ok {
		t.Fatalf("TypeInfo is not TableType")
	}
	if tt.ElementType != 0x70 {
		t.Errorf("wrong element type: 0x%02X", tt.ElementType)
	}
	if tt.Limits.Min != 0 || tt.Limits.HasMax {
		t.Errorf("wrong limits: %+v", tt.Limits)
	}
}

// ---------------------------------------------------------------------------
// Test 7: Memory section
// ---------------------------------------------------------------------------

// TestMemorySectionNoMax parses a memory with min=1, no max.
func TestMemorySectionNoMax(t *testing.T) {
	memPayload := append(leb128(1), makeLimits(1, -1)...)
	module, err := New().Parse(makeWasm(makeSection(5, memPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(module.Memories) != 1 {
		t.Fatalf("expected 1 memory")
	}
	mem := module.Memories[0]
	if mem.Limits.Min != 1 || mem.Limits.HasMax {
		t.Errorf("wrong limits: %+v", mem.Limits)
	}
}

// TestMemorySectionWithMax parses a memory with min=1, max=4.
func TestMemorySectionWithMax(t *testing.T) {
	memPayload := append(leb128(1), makeLimits(1, 4)...)
	module, err := New().Parse(makeWasm(makeSection(5, memPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	mem := module.Memories[0]
	if mem.Limits.Min != 1 || !mem.Limits.HasMax || mem.Limits.Max != 4 {
		t.Errorf("wrong limits: %+v", mem.Limits)
	}
}

// ---------------------------------------------------------------------------
// Test 8: Table section
// ---------------------------------------------------------------------------

// TestTableSection parses a funcref table with min=0, max=10.
func TestTableSection(t *testing.T) {
	tblPayload := leb128(1)
	tblPayload = append(tblPayload, 0x70)              // funcref
	tblPayload = append(tblPayload, makeLimits(0, 10)...) // min=0, max=10

	module, err := New().Parse(makeWasm(makeSection(4, tblPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(module.Tables) != 1 {
		t.Fatalf("expected 1 table")
	}
	tbl := module.Tables[0]
	if tbl.ElementType != 0x70 {
		t.Errorf("wrong element type: 0x%02X", tbl.ElementType)
	}
	if tbl.Limits.Min != 0 || !tbl.Limits.HasMax || tbl.Limits.Max != 10 {
		t.Errorf("wrong limits: %+v", tbl.Limits)
	}
}

// ---------------------------------------------------------------------------
// Test 9: Global section
// ---------------------------------------------------------------------------

// TestGlobalSectionConstI32 parses an immutable i32 global initialized to 42.
func TestGlobalSectionConstI32(t *testing.T) {
	// init_expr: i32.const 42; end = 0x41 0x2A 0x0B
	globPayload := leb128(1)
	globPayload = append(globPayload, 0x7F, 0x00)          // i32, immutable
	globPayload = append(globPayload, 0x41, 0x2A, 0x0B)    // i32.const 42; end

	module, err := New().Parse(makeWasm(makeSection(6, globPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(module.Globals) != 1 {
		t.Fatalf("expected 1 global")
	}
	g := module.Globals[0]
	if g.GlobalType.ValueType != wasmtypes.ValueTypeI32 || g.GlobalType.Mutable {
		t.Errorf("wrong global type: %+v", g.GlobalType)
	}
	if string(g.InitExpr) != string([]byte{0x41, 0x2A, 0x0B}) {
		t.Errorf("wrong init_expr: %v", g.InitExpr)
	}
}

// TestGlobalSectionMutableI32 parses a mutable i32 global initialized to 0.
func TestGlobalSectionMutableI32(t *testing.T) {
	globPayload := leb128(1)
	globPayload = append(globPayload, 0x7F, 0x01) // i32, mutable
	globPayload = append(globPayload, makeInitExprI32(0)...)

	module, err := New().Parse(makeWasm(makeSection(6, globPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !module.Globals[0].GlobalType.Mutable {
		t.Errorf("expected mutable global")
	}
}

// ---------------------------------------------------------------------------
// Test 10: Data section
// ---------------------------------------------------------------------------

// TestDataSection parses a data segment writing b"hello" at offset 0.
func TestDataSection(t *testing.T) {
	dataPayload := leb128(1)
	dataPayload = append(dataPayload, leb128(0)...)         // memory_index = 0
	dataPayload = append(dataPayload, makeInitExprI32(0)...) // offset = 0
	dataPayload = append(dataPayload, leb128(5)...)         // 5 bytes
	dataPayload = append(dataPayload, []byte("hello")...)

	module, err := New().Parse(makeWasm(makeSection(11, dataPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(module.Data) != 1 {
		t.Fatalf("expected 1 data segment")
	}
	ds := module.Data[0]
	if ds.MemoryIndex != 0 {
		t.Errorf("wrong memory index: %d", ds.MemoryIndex)
	}
	if string(ds.Data) != "hello" {
		t.Errorf("wrong data: %v", ds.Data)
	}
}

// ---------------------------------------------------------------------------
// Test 11: Element section
// ---------------------------------------------------------------------------

// TestElementSection parses an element segment: table 0, offset 0, funcs [1,2,3].
func TestElementSection(t *testing.T) {
	elemPayload := leb128(1)
	elemPayload = append(elemPayload, leb128(0)...)          // table_index = 0
	elemPayload = append(elemPayload, makeInitExprI32(0)...) // offset = 0
	elemPayload = append(elemPayload, leb128(3)...)          // 3 function indices
	elemPayload = append(elemPayload, leb128(1)...)
	elemPayload = append(elemPayload, leb128(2)...)
	elemPayload = append(elemPayload, leb128(3)...)

	module, err := New().Parse(makeWasm(makeSection(9, elemPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(module.Elements) != 1 {
		t.Fatalf("expected 1 element segment")
	}
	elem := module.Elements[0]
	if elem.TableIndex != 0 {
		t.Errorf("wrong table index: %d", elem.TableIndex)
	}
	if len(elem.FunctionIndices) != 3 || elem.FunctionIndices[0] != 1 {
		t.Errorf("wrong function indices: %v", elem.FunctionIndices)
	}
}

// ---------------------------------------------------------------------------
// Test 12: Start section
// ---------------------------------------------------------------------------

// TestStartSection parses a Start section with function index 5.
func TestStartSection(t *testing.T) {
	startPayload := leb128(5)
	module, err := New().Parse(makeWasm(makeSection(8, startPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if module.Start == nil {
		t.Fatal("expected non-nil start")
	}
	if *module.Start != 5 {
		t.Errorf("expected start=5, got %d", *module.Start)
	}
}

// TestNoStartSection verifies Start is nil when the section is absent.
func TestNoStartSection(t *testing.T) {
	module, err := New().Parse(wasmHeader)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if module.Start != nil {
		t.Errorf("expected nil start, got %d", *module.Start)
	}
}

// ---------------------------------------------------------------------------
// Test 13: Custom section
// ---------------------------------------------------------------------------

// TestCustomSection parses a custom section named "name".
func TestCustomSection(t *testing.T) {
	data := []byte{0x00, 0x04, 'm', 'a', 'i', 'n'}
	customPayload := append(makeName("name"), data...)
	module, err := New().Parse(makeWasm(makeSection(0, customPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(module.Customs) != 1 {
		t.Fatalf("expected 1 custom section")
	}
	cs := module.Customs[0]
	if cs.Name != "name" {
		t.Errorf("expected name 'name', got %q", cs.Name)
	}
	if string(cs.Data) != string(data) {
		t.Errorf("wrong data: %v", cs.Data)
	}
}

// TestCustomSectionBeforeType verifies custom sections may precede non-custom ones.
func TestCustomSectionBeforeType(t *testing.T) {
	customPayload := append(makeName("pre"), []byte{0xDE, 0xAD}...)
	typePayload := leb128(0)

	module, err := New().Parse(makeWasm(
		makeSection(0, customPayload),
		makeSection(1, typePayload),
	))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(module.Customs) != 1 || module.Customs[0].Name != "pre" {
		t.Errorf("wrong customs: %v", module.Customs)
	}
	if len(module.Types) != 0 {
		t.Errorf("expected 0 types")
	}
}

// TestMultipleCustomSections verifies two custom sections are both collected.
func TestMultipleCustomSections(t *testing.T) {
	c1 := append(makeName("a"), []byte{0x01}...)
	c2 := append(makeName("b"), []byte{0x02, 0x03}...)
	module, err := New().Parse(makeWasm(
		makeSection(0, c1),
		makeSection(0, c2),
	))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(module.Customs) != 2 {
		t.Fatalf("expected 2 customs, got %d", len(module.Customs))
	}
	if module.Customs[0].Name != "a" || module.Customs[1].Name != "b" {
		t.Errorf("wrong custom names: %q %q", module.Customs[0].Name, module.Customs[1].Name)
	}
}

// TestCustomSectionEmptyData verifies a custom section with no data bytes.
func TestCustomSectionEmptyData(t *testing.T) {
	customPayload := makeName("empty")
	module, err := New().Parse(makeWasm(makeSection(0, customPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	cs := module.Customs[0]
	if cs.Name != "empty" || len(cs.Data) != 0 {
		t.Errorf("wrong custom section: %+v", cs)
	}
}

// ---------------------------------------------------------------------------
// Test 14: Multiple sections combined
// ---------------------------------------------------------------------------

// TestMultipleSections parses type + function + export + code sections together.
func TestMultipleSections(t *testing.T) {
	typePayload := append(leb128(1), makeFuncType([]byte{0x7F, 0x7F}, []byte{0x7F})...)
	funcPayload := append(leb128(1), leb128(0)...)

	exportPayload := leb128(1)
	exportPayload = append(exportPayload, makeName("add")...)
	exportPayload = append(exportPayload, 0x00)
	exportPayload = append(exportPayload, leb128(0)...)

	body := append(leb128(0), 0x0B)
	codePayload := leb128(1)
	codePayload = append(codePayload, leb128(uint64(len(body)))...)
	codePayload = append(codePayload, body...)

	module, err := New().Parse(makeWasm(
		makeSection(1, typePayload),
		makeSection(3, funcPayload),
		makeSection(7, exportPayload),
		makeSection(10, codePayload),
	))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(module.Types) != 1 {
		t.Errorf("expected 1 type")
	}
	if len(module.Functions) != 1 || module.Functions[0] != 0 {
		t.Errorf("wrong functions: %v", module.Functions)
	}
	if len(module.Exports) != 1 || module.Exports[0].Name != "add" {
		t.Errorf("wrong exports: %v", module.Exports)
	}
	if len(module.Code) != 1 {
		t.Errorf("expected 1 code body")
	}
}

// ---------------------------------------------------------------------------
// Test 15: Error — bad magic
// ---------------------------------------------------------------------------

// TestBadMagic verifies ParseError is returned for wrong magic bytes.
func TestBadMagic(t *testing.T) {
	bad := []byte{'W', 'A', 'S', 'M', 0x01, 0x00, 0x00, 0x00}
	_, err := New().Parse(bad)
	pe := assertParseError(t, err)
	if pe.Offset != 0 {
		t.Errorf("expected offset=0, got %d", pe.Offset)
	}
}

// TestEmptyInput verifies ParseError is returned for empty input.
func TestEmptyInput(t *testing.T) {
	_, err := New().Parse([]byte{})
	pe := assertParseError(t, err)
	if pe.Offset != 0 {
		t.Errorf("expected offset=0, got %d", pe.Offset)
	}
}

// ---------------------------------------------------------------------------
// Test 16: Error — wrong version
// ---------------------------------------------------------------------------

// TestWrongVersion verifies ParseError for a module with version != 1.
func TestWrongVersion(t *testing.T) {
	bad := []byte{0x00, 0x61, 0x73, 0x6D, 0x02, 0x00, 0x00, 0x00} // version 2
	_, err := New().Parse(bad)
	pe := assertParseError(t, err)
	if pe.Offset != 4 {
		t.Errorf("expected offset=4, got %d", pe.Offset)
	}
}

// ---------------------------------------------------------------------------
// Test 17: Error — truncated header
// ---------------------------------------------------------------------------

// TestTruncatedHeader verifies ParseError for headers shorter than 8 bytes.
func TestTruncatedHeader(t *testing.T) {
	cases := [][]byte{
		{},
		{0x00, 0x61, 0x73, 0x6D},
		{0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00},
	}
	for _, tc := range cases {
		_, err := New().Parse(tc)
		assertParseError(t, err)
	}
}

// ---------------------------------------------------------------------------
// Test 18: Error — truncated section payload
// ---------------------------------------------------------------------------

// TestTruncatedSectionPayload verifies ParseError when section payload is too short.
func TestTruncatedSectionPayload(t *testing.T) {
	// Section header says size=10 but only 2 bytes of payload are present.
	truncated := append([]byte(nil), wasmHeader...)
	truncated = append(truncated, 0x01)   // section ID = type
	truncated = append(truncated, 0x0A)   // size = 10 (non-LEB128 for clarity)
	truncated = append(truncated, 0x01, 0x60) // only 2 bytes payload
	_, err := New().Parse(truncated)
	assertParseError(t, err)
}

// TestTruncatedSectionSizeField verifies error when LEB128 size is truncated.
func TestTruncatedSectionSizeField(t *testing.T) {
	// Section ID present, but LEB128 has continuation bit set with no following byte.
	truncated := append([]byte(nil), wasmHeader...)
	truncated = append(truncated, 0x01, 0x80) // id=1, size starts but is incomplete
	_, err := New().Parse(truncated)
	assertParseError(t, err)
}

// ---------------------------------------------------------------------------
// Test 19: Round-trip
// ---------------------------------------------------------------------------

// TestFullRoundTrip builds a multi-section module and verifies every field.
func TestFullRoundTrip(t *testing.T) {
	// Type section: (i32, i32) → i32
	typePayload := append(leb128(1), makeFuncType([]byte{0x7F, 0x7F}, []byte{0x7F})...)

	// Import section: env::log, func, type 0
	impPayload := leb128(1)
	impPayload = append(impPayload, makeName("env")...)
	impPayload = append(impPayload, makeName("log")...)
	impPayload = append(impPayload, 0x00) // FUNCTION
	impPayload = append(impPayload, leb128(0)...)

	// Function section: local function uses type 0
	funcPayload := append(leb128(1), leb128(0)...)

	// Table section: funcref, min=0, max=5
	tblPayload := leb128(1)
	tblPayload = append(tblPayload, 0x70)
	tblPayload = append(tblPayload, makeLimits(0, 5)...)

	// Memory section: min=1, max=2
	memPayload := append(leb128(1), makeLimits(1, 2)...)

	// Global section: immutable i32 = 0
	globPayload := leb128(1)
	globPayload = append(globPayload, 0x7F, 0x00)       // i32, immutable
	globPayload = append(globPayload, makeInitExprI32(0)...)

	// Export section: add → function 1
	exportPayload := leb128(1)
	exportPayload = append(exportPayload, makeName("add")...)
	exportPayload = append(exportPayload, 0x00) // FUNCTION
	exportPayload = append(exportPayload, leb128(1)...)

	// Start section: function 0
	startPayload := leb128(0)

	// Element section: table 0, offset 0, [1]
	elemPayload := leb128(1)
	elemPayload = append(elemPayload, leb128(0)...)
	elemPayload = append(elemPayload, makeInitExprI32(0)...)
	elemPayload = append(elemPayload, leb128(1)...)
	elemPayload = append(elemPayload, leb128(1)...)

	// Code section: one body, no locals, end
	body := append(leb128(0), 0x0B)
	codePayload := leb128(1)
	codePayload = append(codePayload, leb128(uint64(len(body)))...)
	codePayload = append(codePayload, body...)

	// Data section: memory 0, offset 0, b"hi"
	dataPayload := leb128(1)
	dataPayload = append(dataPayload, leb128(0)...)
	dataPayload = append(dataPayload, makeInitExprI32(0)...)
	dataPayload = append(dataPayload, leb128(2)...)
	dataPayload = append(dataPayload, []byte("hi")...)

	// Custom section: name="test", data=[]byte{0xCA, 0xFE}
	customPayload := append(makeName("test"), 0xCA, 0xFE)

	binary := makeWasm(
		makeSection(1, typePayload),
		makeSection(2, impPayload),
		makeSection(3, funcPayload),
		makeSection(4, tblPayload),
		makeSection(5, memPayload),
		makeSection(6, globPayload),
		makeSection(7, exportPayload),
		makeSection(8, startPayload),
		makeSection(9, elemPayload),
		makeSection(10, codePayload),
		makeSection(11, dataPayload),
		makeSection(0, customPayload), // custom after data is valid
	)

	module, err := New().Parse(binary)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	// Verify types
	if len(module.Types) != 1 {
		t.Fatalf("expected 1 type, got %d", len(module.Types))
	}
	ft := module.Types[0]
	if len(ft.Params) != 2 || ft.Params[0] != wasmtypes.ValueTypeI32 {
		t.Errorf("wrong type params: %v", ft.Params)
	}

	// Verify imports
	if len(module.Imports) != 1 {
		t.Fatalf("expected 1 import")
	}
	imp := module.Imports[0]
	if imp.ModuleName != "env" || imp.Name != "log" {
		t.Errorf("wrong import names: %q %q", imp.ModuleName, imp.Name)
	}
	if idx, ok := imp.TypeInfo.(uint32); !ok || idx != 0 {
		t.Errorf("wrong import TypeInfo: %v", imp.TypeInfo)
	}

	// Verify functions
	if len(module.Functions) != 1 || module.Functions[0] != 0 {
		t.Errorf("wrong functions: %v", module.Functions)
	}

	// Verify tables
	if len(module.Tables) != 1 {
		t.Fatalf("expected 1 table")
	}
	if module.Tables[0].ElementType != 0x70 {
		t.Errorf("wrong element type")
	}
	if module.Tables[0].Limits.Min != 0 || module.Tables[0].Limits.Max != 5 {
		t.Errorf("wrong table limits: %+v", module.Tables[0].Limits)
	}

	// Verify memories
	if len(module.Memories) != 1 {
		t.Fatalf("expected 1 memory")
	}
	if module.Memories[0].Limits.Min != 1 || module.Memories[0].Limits.Max != 2 {
		t.Errorf("wrong memory limits: %+v", module.Memories[0].Limits)
	}

	// Verify globals
	if len(module.Globals) != 1 {
		t.Fatalf("expected 1 global")
	}
	g := module.Globals[0]
	if g.GlobalType.ValueType != wasmtypes.ValueTypeI32 || g.GlobalType.Mutable {
		t.Errorf("wrong global type: %+v", g.GlobalType)
	}

	// Verify exports
	if len(module.Exports) != 1 || module.Exports[0].Name != "add" {
		t.Errorf("wrong exports: %v", module.Exports)
	}
	if module.Exports[0].Index != 1 {
		t.Errorf("wrong export index: %d", module.Exports[0].Index)
	}

	// Verify start
	if module.Start == nil || *module.Start != 0 {
		t.Errorf("wrong start: %v", module.Start)
	}

	// Verify elements
	if len(module.Elements) != 1 {
		t.Fatalf("expected 1 element")
	}
	if module.Elements[0].TableIndex != 0 {
		t.Errorf("wrong table index")
	}
	if len(module.Elements[0].FunctionIndices) != 1 || module.Elements[0].FunctionIndices[0] != 1 {
		t.Errorf("wrong function indices: %v", module.Elements[0].FunctionIndices)
	}

	// Verify code
	if len(module.Code) != 1 {
		t.Fatalf("expected 1 code body")
	}
	fb := module.Code[0]
	if len(fb.Locals) != 0 || len(fb.Code) != 1 || fb.Code[0] != 0x0B {
		t.Errorf("wrong function body: %+v", fb)
	}

	// Verify data
	if len(module.Data) != 1 {
		t.Fatalf("expected 1 data segment")
	}
	ds := module.Data[0]
	if ds.MemoryIndex != 0 || string(ds.Data) != "hi" {
		t.Errorf("wrong data segment: %+v", ds)
	}

	// Verify customs
	if len(module.Customs) != 1 {
		t.Fatalf("expected 1 custom section")
	}
	cs := module.Customs[0]
	if cs.Name != "test" || len(cs.Data) != 2 || cs.Data[0] != 0xCA || cs.Data[1] != 0xFE {
		t.Errorf("wrong custom section: %+v", cs)
	}
}

// ---------------------------------------------------------------------------
// Additional edge-case tests for coverage
// ---------------------------------------------------------------------------

// TestParseErrorFields verifies ParseError.Message and .Offset fields.
func TestParseErrorFields(t *testing.T) {
	_, err := New().Parse([]byte("NOTW"))
	pe := assertParseError(t, err)
	if len(pe.Message) == 0 {
		t.Error("expected non-empty Message")
	}
	if pe.Offset < 0 {
		t.Error("expected non-negative Offset")
	}
	// Error() returns the Message string.
	if pe.Error() != pe.Message {
		t.Error("Error() should return Message")
	}
}

// TestTypeSectionBadMarker verifies error when functype marker is not 0x60.
func TestTypeSectionBadMarker(t *testing.T) {
	// Section with count=1 but marker 0x40 instead of 0x60.
	typePayload := append(leb128(1), 0x40, 0x00, 0x00)
	_, err := New().Parse(makeWasm(makeSection(1, typePayload)))
	assertParseError(t, err)
}

// TestLargeLEB128Index verifies a function section with a multi-byte type index (128).
func TestLargeLEB128Index(t *testing.T) {
	funcPayload := append(leb128(1), leb128(128)...)
	module, err := New().Parse(makeWasm(makeSection(3, funcPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(module.Functions) != 1 || module.Functions[0] != 128 {
		t.Errorf("expected [128], got %v", module.Functions)
	}
}

// TestGlobalImportMutable verifies a mutable i64 global import.
func TestGlobalImportMutable(t *testing.T) {
	impPayload := leb128(1)
	impPayload = append(impPayload, makeName("env")...)
	impPayload = append(impPayload, makeName("gv")...)
	impPayload = append(impPayload, 0x03)  // ExternalKindGlobal
	impPayload = append(impPayload, 0x7E)  // ValueTypeI64
	impPayload = append(impPayload, 0x01)  // mutable

	module, err := New().Parse(makeWasm(makeSection(2, impPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	gt, ok := module.Imports[0].TypeInfo.(wasmtypes.GlobalType)
	if !ok {
		t.Fatal("TypeInfo is not GlobalType")
	}
	if gt.ValueType != wasmtypes.ValueTypeI64 || !gt.Mutable {
		t.Errorf("wrong global type: %+v", gt)
	}
}

// TestMemoryImportWithMax verifies a memory import with explicit max.
func TestMemoryImportWithMax(t *testing.T) {
	impPayload := leb128(1)
	impPayload = append(impPayload, makeName("env")...)
	impPayload = append(impPayload, makeName("mem")...)
	impPayload = append(impPayload, 0x02)
	impPayload = append(impPayload, makeLimits(2, 8)...)

	module, err := New().Parse(makeWasm(makeSection(2, impPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	mt, ok := module.Imports[0].TypeInfo.(wasmtypes.MemoryType)
	if !ok {
		t.Fatal("TypeInfo is not MemoryType")
	}
	if mt.Limits.Min != 2 || mt.Limits.Max != 8 || !mt.Limits.HasMax {
		t.Errorf("wrong limits: %+v", mt.Limits)
	}
}

// TestTableImportNoMax verifies a table import without max.
func TestTableImportNoMax(t *testing.T) {
	impPayload := leb128(1)
	impPayload = append(impPayload, makeName("env")...)
	impPayload = append(impPayload, makeName("t")...)
	impPayload = append(impPayload, 0x01)              // ExternalKindTable
	impPayload = append(impPayload, 0x70)
	impPayload = append(impPayload, makeLimits(1, -1)...)

	module, err := New().Parse(makeWasm(makeSection(2, impPayload)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	tt, ok := module.Imports[0].TypeInfo.(wasmtypes.TableType)
	if !ok {
		t.Fatal("TypeInfo is not TableType")
	}
	if tt.Limits.Min != 1 || tt.Limits.HasMax {
		t.Errorf("wrong limits: %+v", tt.Limits)
	}
}
