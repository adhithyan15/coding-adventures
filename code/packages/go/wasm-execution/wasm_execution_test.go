package wasmexecution

import (
	"math"
	"testing"

	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
)

// ════════════════════════════════════════════════════════════════════════
// HELPER: Build and run a single-function WASM module
// ════════════════════════════════════════════════════════════════════════

// makeEngine creates a single-function engine with the given bytecodes,
// function type, and optional memory/tables/globals.
func makeEngine(code []byte, ft wasmtypes.FuncType, mem *LinearMemory) *WasmExecutionEngine {
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	return NewWasmExecutionEngine(EngineConfig{
		Memory:        mem,
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
}

// voidToI32 is a convenience FuncType: () -> i32.
var voidToI32 = wasmtypes.FuncType{
	Params:  nil,
	Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
}

// i32ToI32 is a convenience FuncType: (i32) -> i32.
var i32ToI32 = wasmtypes.FuncType{
	Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
	Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
}

// expectTrap runs engine.CallFunction and asserts it returns a TrapError
// containing the given substring.
func expectTrap(t *testing.T, engine *WasmExecutionEngine, funcIdx int, args []WasmValue, substr string) {
	t.Helper()
	_, err := engine.CallFunction(funcIdx, args)
	if err == nil {
		t.Fatalf("expected trap containing %q, but got no error", substr)
	}
	te, ok := err.(*TrapError)
	if !ok {
		t.Fatalf("expected *TrapError, got %T: %v", err, err)
	}
	if substr != "" {
		if te.Message == "" || !contains(te.Message, substr) {
			t.Fatalf("expected trap message containing %q, got %q", substr, te.Message)
		}
	}
}

func contains(s, sub string) bool {
	return len(s) >= len(sub) && (s == sub || len(sub) == 0 || findSubstring(s, sub))
}

func findSubstring(s, sub string) bool {
	for i := 0; i <= len(s)-len(sub); i++ {
		if s[i:i+len(sub)] == sub {
			return true
		}
	}
	return false
}

// ════════════════════════════════════════════════════════════════════════
// VALUE CONSTRUCTORS AND TYPE EXTRACTORS
// ════════════════════════════════════════════════════════════════════════

func TestValueConstructors(t *testing.T) {
	v := I32(42)
	if v.Type != int(wasmtypes.ValueTypeI32) || v.Value.(int32) != 42 {
		t.Fatalf("I32(42) failed: %v", v)
	}

	v = I64(100)
	if v.Type != int(wasmtypes.ValueTypeI64) || v.Value.(int64) != 100 {
		t.Fatalf("I64(100) failed: %v", v)
	}

	v = F32(3.14)
	if v.Type != int(wasmtypes.ValueTypeF32) {
		t.Fatalf("F32 type wrong: %v", v)
	}

	v = F64(2.718)
	if v.Type != int(wasmtypes.ValueTypeF64) || v.Value.(float64) != 2.718 {
		t.Fatalf("F64(2.718) failed: %v", v)
	}
}

func TestDefaultValue(t *testing.T) {
	tests := []struct {
		vt   wasmtypes.ValueType
		zero interface{}
	}{
		{wasmtypes.ValueTypeI32, int32(0)},
		{wasmtypes.ValueTypeI64, int64(0)},
		{wasmtypes.ValueTypeF32, float32(0)},
		{wasmtypes.ValueTypeF64, float64(0)},
	}
	for _, tt := range tests {
		v := DefaultValue(tt.vt)
		if v.Value != tt.zero {
			t.Fatalf("DefaultValue(0x%02x) = %v, want %v", tt.vt, v.Value, tt.zero)
		}
	}
}

func TestDefaultValuePanicsOnUnknown(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Fatal("expected panic for unknown value type")
		}
	}()
	DefaultValue(wasmtypes.ValueType(0xFF))
}

func TestAsI32TypeMismatch(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Fatal("expected panic")
		}
	}()
	AsI32(I64(1))
}

func TestAsI64TypeMismatch(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Fatal("expected panic")
		}
	}()
	AsI64(I32(1))
}

func TestAsF32TypeMismatch(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Fatal("expected panic")
		}
	}()
	AsF32(I32(1))
}

func TestAsF64TypeMismatch(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Fatal("expected panic")
		}
	}()
	AsF64(I32(1))
}

func TestAsExtractors(t *testing.T) {
	if AsI32(I32(-7)) != -7 {
		t.Fatal("AsI32 roundtrip failed")
	}
	if AsI64(I64(math.MaxInt64)) != math.MaxInt64 {
		t.Fatal("AsI64 roundtrip failed")
	}
	if AsF32(F32(1.5)) != 1.5 {
		t.Fatal("AsF32 roundtrip failed")
	}
	if AsF64(F64(1.5)) != 1.5 {
		t.Fatal("AsF64 roundtrip failed")
	}
}

// ════════════════════════════════════════════════════════════════════════
// LINEAR MEMORY — FULL-WIDTH LOADS/STORES
// ════════════════════════════════════════════════════════════════════════

func TestLinearMemoryI64(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	mem.StoreI64(0, 0x123456789ABCDEF0)
	if mem.LoadI64(0) != 0x123456789ABCDEF0 {
		t.Fatal("i64 roundtrip failed")
	}
}

func TestLinearMemoryF32(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	mem.StoreF32(0, 3.14)
	if mem.LoadF32(0) != 3.14 {
		t.Fatalf("f32 roundtrip: got %v", mem.LoadF32(0))
	}
}

func TestLinearMemoryF64(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	mem.StoreF64(0, 2.718281828)
	if mem.LoadF64(0) != 2.718281828 {
		t.Fatalf("f64 roundtrip: got %v", mem.LoadF64(0))
	}
}

// ════════════════════════════════════════════════════════════════════════
// LINEAR MEMORY — NARROW LOADS/STORES
// ════════════════════════════════════════════════════════════════════════

func TestLinearMemoryNarrowI32(t *testing.T) {
	mem := NewLinearMemory(1, -1)

	// 8-bit sign/zero extension
	mem.StoreI32_8(0, 0x80) // 128 unsigned = -128 signed
	if mem.LoadI32_8s(0) != -128 {
		t.Fatalf("i32_8s: got %d", mem.LoadI32_8s(0))
	}
	if mem.LoadI32_8u(0) != 128 {
		t.Fatalf("i32_8u: got %d", mem.LoadI32_8u(0))
	}

	// 16-bit sign/zero extension
	mem.StoreI32_16(10, int32(0x8000)) // 32768 unsigned = -32768 signed
	if mem.LoadI32_16s(10) != -32768 {
		t.Fatalf("i32_16s: got %d", mem.LoadI32_16s(10))
	}
	if mem.LoadI32_16u(10) != 32768 {
		t.Fatalf("i32_16u: got %d", mem.LoadI32_16u(10))
	}
}

func TestLinearMemoryNarrowI64(t *testing.T) {
	mem := NewLinearMemory(1, -1)

	// i64 8-bit
	mem.StoreI64_8(0, 0xFF)
	if mem.LoadI64_8s(0) != -1 {
		t.Fatalf("i64_8s: got %d", mem.LoadI64_8s(0))
	}
	if mem.LoadI64_8u(0) != 255 {
		t.Fatalf("i64_8u: got %d", mem.LoadI64_8u(0))
	}

	// i64 16-bit
	mem.StoreI64_16(10, 0xFFFF)
	if mem.LoadI64_16s(10) != -1 {
		t.Fatalf("i64_16s: got %d", mem.LoadI64_16s(10))
	}
	if mem.LoadI64_16u(10) != 65535 {
		t.Fatalf("i64_16u: got %d", mem.LoadI64_16u(10))
	}

	// i64 32-bit
	mem.StoreI64_32(20, 0xFFFFFFFF)
	if mem.LoadI64_32s(20) != -1 {
		t.Fatalf("i64_32s: got %d", mem.LoadI64_32s(20))
	}
	if mem.LoadI64_32u(20) != 0xFFFFFFFF {
		t.Fatalf("i64_32u: got %d", mem.LoadI64_32u(20))
	}
}

// ════════════════════════════════════════════════════════════════════════
// LINEAR MEMORY — GROW AND OOB
// ════════════════════════════════════════════════════════════════════════

func TestLinearMemoryGrow(t *testing.T) {
	mem := NewLinearMemory(1, 3)

	if mem.Size() != 1 {
		t.Fatalf("initial size: got %d", mem.Size())
	}
	if mem.ByteLength() != PageSize {
		t.Fatalf("initial byte length: got %d", mem.ByteLength())
	}

	old := mem.Grow(1)
	if old != 1 {
		t.Fatalf("grow should return old page count 1, got %d", old)
	}
	if mem.Size() != 2 {
		t.Fatalf("after grow: size = %d", mem.Size())
	}

	// Grow beyond max should fail.
	result := mem.Grow(5)
	if result != -1 {
		t.Fatal("grow beyond max should return -1")
	}
}

func TestLinearMemoryGrowNoMax(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	old := mem.Grow(2)
	if old != 1 {
		t.Fatalf("expected old=1, got %d", old)
	}
	if mem.Size() != 3 {
		t.Fatalf("expected size=3, got %d", mem.Size())
	}
}

func TestLinearMemoryGrowBeyondSpecMax(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	// Trying to grow beyond 65536 total pages should fail.
	result := mem.Grow(65536)
	if result != -1 {
		t.Fatal("grow beyond 65536 pages should return -1")
	}
}

func TestLinearMemoryOOB(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	defer func() {
		if r := recover(); r == nil {
			t.Fatal("expected panic for OOB access")
		}
	}()
	mem.LoadI32(PageSize - 2) // only 2 bytes available, need 4
}

func TestLinearMemoryOOBStore(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	defer func() {
		if r := recover(); r == nil {
			t.Fatal("expected panic for OOB store")
		}
	}()
	mem.StoreI64(PageSize-4, 1)
}

func TestLinearMemoryWriteBytes(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	mem.WriteBytes(0, []byte{0x01, 0x02, 0x03, 0x04})
	if mem.LoadI32_8u(0) != 1 || mem.LoadI32_8u(3) != 4 {
		t.Fatal("WriteBytes did not write correctly")
	}
}

func TestLinearMemoryWriteBytesOOB(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	defer func() {
		if r := recover(); r == nil {
			t.Fatal("expected panic for OOB WriteBytes")
		}
	}()
	mem.WriteBytes(PageSize-2, []byte{1, 2, 3, 4})
}

// ════════════════════════════════════════════════════════════════════════
// TABLE — GET, SET, GROW, OOB
// ════════════════════════════════════════════════════════════════════════

func TestTableOperations(t *testing.T) {
	table := NewTable(4, 8)

	// Initially all null (-1).
	for i := 0; i < 4; i++ {
		if table.Get(i) != -1 {
			t.Fatalf("table[%d] should be -1", i)
		}
	}

	table.Set(0, 10)
	table.Set(3, 42)
	if table.Get(0) != 10 || table.Get(3) != 42 {
		t.Fatal("set/get failed")
	}
	if table.Size() != 4 {
		t.Fatalf("size should be 4, got %d", table.Size())
	}
}

func TestTableGrow(t *testing.T) {
	table := NewTable(2, 5)
	old := table.Grow(2)
	if old != 2 {
		t.Fatalf("expected old=2, got %d", old)
	}
	if table.Size() != 4 {
		t.Fatalf("expected size=4, got %d", table.Size())
	}
	// New entries should be null.
	if table.Get(2) != -1 || table.Get(3) != -1 {
		t.Fatal("grown entries should be -1")
	}

	// Grow beyond max.
	result := table.Grow(5)
	if result != -1 {
		t.Fatal("grow beyond max should return -1")
	}
}

func TestTableOOBGet(t *testing.T) {
	table := NewTable(2, -1)
	defer func() {
		if r := recover(); r == nil {
			t.Fatal("expected panic for OOB get")
		}
	}()
	table.Get(5)
}

func TestTableOOBSet(t *testing.T) {
	table := NewTable(2, -1)
	defer func() {
		if r := recover(); r == nil {
			t.Fatal("expected panic for OOB set")
		}
	}()
	table.Set(5, 1)
}

func TestTableOOBNegativeGet(t *testing.T) {
	table := NewTable(2, -1)
	defer func() {
		if r := recover(); r == nil {
			t.Fatal("expected panic for negative index")
		}
	}()
	table.Get(-1)
}

// ════════════════════════════════════════════════════════════════════════
// TRAP ERROR
// ════════════════════════════════════════════════════════════════════════

func TestTrapError(t *testing.T) {
	te := NewTrapError("bad thing")
	if te.Error() != "TrapError: bad thing" {
		t.Fatalf("unexpected error string: %s", te.Error())
	}
}

// ════════════════════════════════════════════════════════════════════════
// I32 ARITHMETIC (via engine)
// ════════════════════════════════════════════════════════════════════════

func TestI32Arithmetic(t *testing.T) {
	// Build a simple function: (i32.const 3) (i32.const 4) (i32.add) (end)
	body := &wasmtypes.FunctionBody{
		Locals: nil,
		Code:   []byte{0x41, 0x03, 0x41, 0x04, 0x6A, 0x0B},
	}

	engine := NewWasmExecutionEngine(EngineConfig{
		Memory:        nil,
		Tables:        nil,
		Globals:       nil,
		GlobalTypes:   nil,
		FuncTypes:     []wasmtypes.FuncType{{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}}},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})

	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(results))
	}
	if AsI32(results[0]) != 7 {
		t.Fatalf("expected 7, got %d", AsI32(results[0]))
	}
}

func TestI32Sub(t *testing.T) {
	// i32.const 10; i32.const 3; i32.sub; end
	engine := makeEngine([]byte{0x41, 0x0A, 0x41, 0x03, 0x6B, 0x0B}, voidToI32, nil)
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 7 {
		t.Fatalf("expected 7, got %d", AsI32(results[0]))
	}
}

func TestI32Mul(t *testing.T) {
	// i32.const 6; i32.const 7; i32.mul; end
	engine := makeEngine([]byte{0x41, 0x06, 0x41, 0x07, 0x6C, 0x0B}, voidToI32, nil)
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 42 {
		t.Fatalf("expected 42, got %d", AsI32(results[0]))
	}
}

func TestI32DivS(t *testing.T) {
	// i32.const 10; i32.const 3; i32.div_s; end
	engine := makeEngine([]byte{0x41, 0x0A, 0x41, 0x03, 0x6D, 0x0B}, voidToI32, nil)
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 3 {
		t.Fatalf("expected 3, got %d", AsI32(results[0]))
	}
}

func TestI32DivU(t *testing.T) {
	// i32.const -1 (=0xFFFFFFFF); i32.const 2; i32.div_u; end
	// -1 encoded as signed LEB128: 0x7F
	engine := makeEngine([]byte{0x41, 0x7F, 0x41, 0x02, 0x6E, 0x0B}, voidToI32, nil)
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	// uint32(0xFFFFFFFF) / 2 = 2147483647
	if AsI32(results[0]) != int32(uint32(0xFFFFFFFF)/2) {
		t.Fatalf("expected %d, got %d", int32(uint32(0xFFFFFFFF)/2), AsI32(results[0]))
	}
}

func TestI32DivSOverflow(t *testing.T) {
	// INT32_MIN / -1 should trap with "integer overflow"
	// i32.const INT32_MIN (encoded as LEB128: 0x80 0x80 0x80 0x80 0x78)
	// i32.const -1 (encoded as 0x7F)
	// i32.div_s; end
	engine := makeEngine([]byte{
		0x41, 0x80, 0x80, 0x80, 0x80, 0x78, // i32.const INT32_MIN
		0x41, 0x7F, // i32.const -1
		0x6D, // i32.div_s
		0x0B, // end
	}, voidToI32, nil)
	expectTrap(t, engine, 0, nil, "integer overflow")
}

func TestI32RemS(t *testing.T) {
	// i32.const 10; i32.const 3; i32.rem_s; end
	engine := makeEngine([]byte{0x41, 0x0A, 0x41, 0x03, 0x6F, 0x0B}, voidToI32, nil)
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 1 {
		t.Fatalf("expected 1, got %d", AsI32(results[0]))
	}
}

func TestI32RemSOverflow(t *testing.T) {
	// INT32_MIN % -1 should return 0 (not trap)
	engine := makeEngine([]byte{
		0x41, 0x80, 0x80, 0x80, 0x80, 0x78, // i32.const INT32_MIN
		0x41, 0x7F, // i32.const -1
		0x6F, // i32.rem_s
		0x0B, // end
	}, voidToI32, nil)
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 0 {
		t.Fatalf("expected 0, got %d", AsI32(results[0]))
	}
}

func TestI32RemU(t *testing.T) {
	// i32.const 10; i32.const 3; i32.rem_u; end
	engine := makeEngine([]byte{0x41, 0x0A, 0x41, 0x03, 0x70, 0x0B}, voidToI32, nil)
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 1 {
		t.Fatalf("expected 1, got %d", AsI32(results[0]))
	}
}

func TestI32RemUDivByZero(t *testing.T) {
	engine := makeEngine([]byte{0x41, 0x0A, 0x41, 0x00, 0x70, 0x0B}, voidToI32, nil)
	expectTrap(t, engine, 0, nil, "integer divide by zero")
}

func TestI32RemSDivByZero(t *testing.T) {
	engine := makeEngine([]byte{0x41, 0x0A, 0x41, 0x00, 0x6F, 0x0B}, voidToI32, nil)
	expectTrap(t, engine, 0, nil, "integer divide by zero")
}

func TestI32DivUDivByZero(t *testing.T) {
	engine := makeEngine([]byte{0x41, 0x0A, 0x41, 0x00, 0x6E, 0x0B}, voidToI32, nil)
	expectTrap(t, engine, 0, nil, "integer divide by zero")
}

// ════════════════════════════════════════════════════════════════════════
// I32 COMPARISONS
// ════════════════════════════════════════════════════════════════════════

func TestI32Comparisons(t *testing.T) {
	tests := []struct {
		name   string
		opcode byte
		a, b   int
		expect int32
	}{
		// i32.eqz: only takes one operand, tested separately
		{"eq_true", 0x46, 5, 5, 1},
		{"eq_false", 0x46, 5, 6, 0},
		{"ne_true", 0x47, 5, 6, 1},
		{"ne_false", 0x47, 5, 5, 0},
		{"lt_s_true", 0x48, -1, 0, 1},
		{"lt_s_false", 0x48, 0, -1, 0},
		{"lt_u_true", 0x49, 1, 2, 1},
		{"lt_u_false", 0x49, 2, 1, 0},
		{"gt_s_true", 0x4A, 1, 0, 1},
		{"gt_s_false", 0x4A, 0, 1, 0},
		{"gt_u_true", 0x4B, 2, 1, 1},
		{"gt_u_false", 0x4B, 1, 2, 0},
		{"le_s_true", 0x4C, 5, 5, 1},
		{"le_s_false", 0x4C, 6, 5, 0},
		{"le_u_true", 0x4D, 5, 5, 1},
		{"le_u_false", 0x4D, 6, 5, 0},
		{"ge_s_true", 0x4E, 5, 5, 1},
		{"ge_s_false", 0x4E, 4, 5, 0},
		{"ge_u_true", 0x4F, 5, 5, 1},
		{"ge_u_false", 0x4F, 4, 5, 0},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// local.get 0; local.get 1; <cmp>; end
			code := []byte{0x20, 0x00, 0x20, 0x01, tt.opcode, 0x0B}
			ft := wasmtypes.FuncType{
				Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32, wasmtypes.ValueTypeI32},
				Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
			}
			body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
			engine := NewWasmExecutionEngine(EngineConfig{
				FuncTypes:     []wasmtypes.FuncType{ft},
				FuncBodies:    []*wasmtypes.FunctionBody{body},
				HostFunctions: []*HostFunction{nil},
			})
			results, err := engine.CallFunction(0, []WasmValue{I32(int32(tt.a)), I32(int32(tt.b))})
			if err != nil {
				t.Fatal(err)
			}
			if AsI32(results[0]) != tt.expect {
				t.Fatalf("expected %d, got %d", tt.expect, AsI32(results[0]))
			}
		})
	}
}

func TestI32Eqz(t *testing.T) {
	// local.get 0; i32.eqz; end
	code := []byte{0x20, 0x00, 0x45, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{i32ToI32},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})

	results, _ := engine.CallFunction(0, []WasmValue{I32(0)})
	if AsI32(results[0]) != 1 {
		t.Fatalf("eqz(0) should be 1")
	}
	results, _ = engine.CallFunction(0, []WasmValue{I32(5)})
	if AsI32(results[0]) != 0 {
		t.Fatalf("eqz(5) should be 0")
	}
}

// ════════════════════════════════════════════════════════════════════════
// I32 BITWISE, SHIFTS, ROTATIONS, CLZ/CTZ/POPCNT
// ════════════════════════════════════════════════════════════════════════

func TestI32Bitwise(t *testing.T) {
	tests := []struct {
		name   string
		opcode byte
		a, b   int32
		expect int32
	}{
		{"and", 0x71, 0xFF, 0x0F, 0x0F},
		{"or", 0x72, 0xF0, 0x0F, 0xFF},
		{"xor", 0x73, 0xFF, 0x0F, 0xF0},
		{"shl", 0x74, 1, 4, 16},
		{"shr_s", 0x75, -16, 2, -4},
		{"shr_u", 0x76, -1, 1, int32(uint32(0xFFFFFFFF) >> 1)},
		{"rotl", 0x77, 1, 1, 2},
		{"rotr", 0x78, 1, 1, -2147483648},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// local.get 0; local.get 1; <op>; end
			code := []byte{0x20, 0x00, 0x20, 0x01, tt.opcode, 0x0B}
			ft := wasmtypes.FuncType{
				Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32, wasmtypes.ValueTypeI32},
				Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
			}
			body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
			engine := NewWasmExecutionEngine(EngineConfig{
				FuncTypes:     []wasmtypes.FuncType{ft},
				FuncBodies:    []*wasmtypes.FunctionBody{body},
				HostFunctions: []*HostFunction{nil},
			})
			results, err := engine.CallFunction(0, []WasmValue{I32(tt.a), I32(tt.b)})
			if err != nil {
				t.Fatal(err)
			}
			if AsI32(results[0]) != tt.expect {
				t.Fatalf("expected %d, got %d", tt.expect, AsI32(results[0]))
			}
		})
	}
}

func TestI32ClzCtzPopcnt(t *testing.T) {
	tests := []struct {
		name   string
		opcode byte
		input  int32
		expect int32
	}{
		{"clz_1", 0x67, 1, 31},
		{"clz_max", 0x67, -1, 0},
		{"clz_zero", 0x67, 0, 32},
		{"ctz_1", 0x68, 1, 0},
		{"ctz_2", 0x68, 8, 3},
		{"ctz_zero", 0x68, 0, 32},
		{"popcnt", 0x69, 0xFF, 8},
		{"popcnt_zero", 0x69, 0, 0},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// local.get 0; <op>; end
			code := []byte{0x20, 0x00, tt.opcode, 0x0B}
			body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
			engine := NewWasmExecutionEngine(EngineConfig{
				FuncTypes:     []wasmtypes.FuncType{i32ToI32},
				FuncBodies:    []*wasmtypes.FunctionBody{body},
				HostFunctions: []*HostFunction{nil},
			})
			results, err := engine.CallFunction(0, []WasmValue{I32(tt.input)})
			if err != nil {
				t.Fatal(err)
			}
			if AsI32(results[0]) != tt.expect {
				t.Fatalf("expected %d, got %d", tt.expect, AsI32(results[0]))
			}
		})
	}
}

// ════════════════════════════════════════════════════════════════════════
// I64 OPERATIONS
// ════════════════════════════════════════════════════════════════════════

func TestI64Operations(t *testing.T) {
	voidToI64 := wasmtypes.FuncType{
		Params:  nil,
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
	}

	// i64.const 10; i64.const 20; i64.add; end
	engine := makeEngine([]byte{0x42, 0x0A, 0x42, 0x14, 0x7C, 0x0B}, voidToI64, nil)
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI64(results[0]) != 30 {
		t.Fatalf("i64 add: expected 30, got %d", AsI64(results[0]))
	}
}

func TestI64Eqz(t *testing.T) {
	// local.get 0; i64.eqz; end (returns i32)
	i64ToI32 := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
	}
	code := []byte{0x20, 0x00, 0x50, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{i64ToI32},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})

	results, _ := engine.CallFunction(0, []WasmValue{I64(0)})
	if AsI32(results[0]) != 1 {
		t.Fatal("i64.eqz(0) should be 1")
	}
	results, _ = engine.CallFunction(0, []WasmValue{I64(42)})
	if AsI32(results[0]) != 0 {
		t.Fatal("i64.eqz(42) should be 0")
	}
}

func TestI64Comparisons(t *testing.T) {
	i64i64ToI32 := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI64, wasmtypes.ValueTypeI64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
	}

	tests := []struct {
		name   string
		opcode byte
		a, b   int64
		expect int32
	}{
		{"eq", 0x51, 5, 5, 1},
		{"ne", 0x52, 5, 6, 1},
		{"lt_s", 0x53, -1, 0, 1},
		{"lt_u", 0x54, 1, 2, 1},
		{"gt_s", 0x55, 1, 0, 1},
		{"gt_u", 0x56, 2, 1, 1},
		{"le_s", 0x57, 5, 5, 1},
		{"le_u", 0x58, 5, 5, 1},
		{"ge_s", 0x59, 5, 5, 1},
		{"ge_u", 0x5A, 5, 5, 1},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			code := []byte{0x20, 0x00, 0x20, 0x01, tt.opcode, 0x0B}
			body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
			engine := NewWasmExecutionEngine(EngineConfig{
				FuncTypes:     []wasmtypes.FuncType{i64i64ToI32},
				FuncBodies:    []*wasmtypes.FunctionBody{body},
				HostFunctions: []*HostFunction{nil},
			})
			results, err := engine.CallFunction(0, []WasmValue{I64(tt.a), I64(tt.b)})
			if err != nil {
				t.Fatal(err)
			}
			if AsI32(results[0]) != tt.expect {
				t.Fatalf("expected %d, got %d", tt.expect, AsI32(results[0]))
			}
		})
	}
}

func TestI64Arithmetic(t *testing.T) {
	i64i64ToI64 := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI64, wasmtypes.ValueTypeI64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
	}

	tests := []struct {
		name   string
		opcode byte
		a, b   int64
		expect int64
	}{
		{"sub", 0x7D, 10, 3, 7},
		{"mul", 0x7E, 6, 7, 42},
		{"and", 0x83, 0xFF, 0x0F, 0x0F},
		{"or", 0x84, 0xF0, 0x0F, 0xFF},
		{"xor", 0x85, 0xFF, 0x0F, 0xF0},
		{"shl", 0x86, 1, 4, 16},
		{"shr_s", 0x87, -16, 2, -4},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			code := []byte{0x20, 0x00, 0x20, 0x01, tt.opcode, 0x0B}
			body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
			engine := NewWasmExecutionEngine(EngineConfig{
				FuncTypes:     []wasmtypes.FuncType{i64i64ToI64},
				FuncBodies:    []*wasmtypes.FunctionBody{body},
				HostFunctions: []*HostFunction{nil},
			})
			results, err := engine.CallFunction(0, []WasmValue{I64(tt.a), I64(tt.b)})
			if err != nil {
				t.Fatal(err)
			}
			if AsI64(results[0]) != tt.expect {
				t.Fatalf("expected %d, got %d", tt.expect, AsI64(results[0]))
			}
		})
	}
}

func TestI64Unary(t *testing.T) {
	i64ToI64 := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
	}

	tests := []struct {
		name   string
		opcode byte
		input  int64
		expect int64
	}{
		{"clz", 0x79, 1, 63},
		{"ctz", 0x7A, 8, 3},
		{"popcnt", 0x7B, 0xFF, 8},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			code := []byte{0x20, 0x00, tt.opcode, 0x0B}
			body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
			engine := NewWasmExecutionEngine(EngineConfig{
				FuncTypes:     []wasmtypes.FuncType{i64ToI64},
				FuncBodies:    []*wasmtypes.FunctionBody{body},
				HostFunctions: []*HostFunction{nil},
			})
			results, err := engine.CallFunction(0, []WasmValue{I64(tt.input)})
			if err != nil {
				t.Fatal(err)
			}
			if AsI64(results[0]) != tt.expect {
				t.Fatalf("expected %d, got %d", tt.expect, AsI64(results[0]))
			}
		})
	}
}

func TestI64DivByZero(t *testing.T) {
	i64i64ToI64 := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI64, wasmtypes.ValueTypeI64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
	}
	// i64.div_s by zero
	code := []byte{0x20, 0x00, 0x20, 0x01, 0x7F, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{i64i64ToI64},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	expectTrap(t, engine, 0, []WasmValue{I64(10), I64(0)}, "integer divide by zero")
}

func TestI64DivSOverflow(t *testing.T) {
	i64i64ToI64 := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI64, wasmtypes.ValueTypeI64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
	}
	code := []byte{0x20, 0x00, 0x20, 0x01, 0x7F, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{i64i64ToI64},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	expectTrap(t, engine, 0, []WasmValue{I64(math.MinInt64), I64(-1)}, "integer overflow")
}

func TestI64DivU(t *testing.T) {
	i64i64ToI64 := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI64, wasmtypes.ValueTypeI64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
	}
	// i64.div_u
	code := []byte{0x20, 0x00, 0x20, 0x01, 0x80, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{i64i64ToI64},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{I64(100), I64(3)})
	if err != nil {
		t.Fatal(err)
	}
	if AsI64(results[0]) != 33 {
		t.Fatalf("expected 33, got %d", AsI64(results[0]))
	}
}

func TestI64DivUByZero(t *testing.T) {
	i64i64ToI64 := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI64, wasmtypes.ValueTypeI64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
	}
	code := []byte{0x20, 0x00, 0x20, 0x01, 0x80, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{i64i64ToI64},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	expectTrap(t, engine, 0, []WasmValue{I64(10), I64(0)}, "integer divide by zero")
}

func TestI64RemS(t *testing.T) {
	i64i64ToI64 := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI64, wasmtypes.ValueTypeI64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
	}
	// i64.rem_s
	code := []byte{0x20, 0x00, 0x20, 0x01, 0x81, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{i64i64ToI64},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{I64(10), I64(3)})
	if err != nil {
		t.Fatal(err)
	}
	if AsI64(results[0]) != 1 {
		t.Fatalf("expected 1, got %d", AsI64(results[0]))
	}
}

func TestI64RemU(t *testing.T) {
	i64i64ToI64 := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI64, wasmtypes.ValueTypeI64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
	}
	// i64.rem_u
	code := []byte{0x20, 0x00, 0x20, 0x01, 0x82, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{i64i64ToI64},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{I64(10), I64(3)})
	if err != nil {
		t.Fatal(err)
	}
	if AsI64(results[0]) != 1 {
		t.Fatalf("expected 1, got %d", AsI64(results[0]))
	}
}

// ════════════════════════════════════════════════════════════════════════
// F32 OPERATIONS
// ════════════════════════════════════════════════════════════════════════

func TestF32Arithmetic(t *testing.T) {
	f32f32ToF32 := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeF32, wasmtypes.ValueTypeF32},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF32},
	}

	tests := []struct {
		name   string
		opcode byte
		a, b   float32
		expect float32
	}{
		{"add", 0x92, 1.5, 2.5, 4.0},
		{"sub", 0x93, 5.0, 2.0, 3.0},
		{"mul", 0x94, 3.0, 4.0, 12.0},
		{"div", 0x95, 10.0, 4.0, 2.5},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			code := []byte{0x20, 0x00, 0x20, 0x01, tt.opcode, 0x0B}
			body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
			engine := NewWasmExecutionEngine(EngineConfig{
				FuncTypes:     []wasmtypes.FuncType{f32f32ToF32},
				FuncBodies:    []*wasmtypes.FunctionBody{body},
				HostFunctions: []*HostFunction{nil},
			})
			results, err := engine.CallFunction(0, []WasmValue{F32(tt.a), F32(tt.b)})
			if err != nil {
				t.Fatal(err)
			}
			if AsF32(results[0]) != tt.expect {
				t.Fatalf("expected %v, got %v", tt.expect, AsF32(results[0]))
			}
		})
	}
}

func TestF32Unary(t *testing.T) {
	f32ToF32 := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeF32},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF32},
	}

	tests := []struct {
		name   string
		opcode byte
		input  float32
		expect float32
	}{
		{"abs", 0x8B, -5.0, 5.0},
		{"neg", 0x8C, 3.0, -3.0},
		{"ceil", 0x8D, 1.3, 2.0},
		{"floor", 0x8E, 1.7, 1.0},
		{"trunc", 0x8F, 1.7, 1.0},
		{"sqrt", 0x91, 9.0, 3.0},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			code := []byte{0x20, 0x00, tt.opcode, 0x0B}
			body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
			engine := NewWasmExecutionEngine(EngineConfig{
				FuncTypes:     []wasmtypes.FuncType{f32ToF32},
				FuncBodies:    []*wasmtypes.FunctionBody{body},
				HostFunctions: []*HostFunction{nil},
			})
			results, err := engine.CallFunction(0, []WasmValue{F32(tt.input)})
			if err != nil {
				t.Fatal(err)
			}
			if AsF32(results[0]) != tt.expect {
				t.Fatalf("expected %v, got %v", tt.expect, AsF32(results[0]))
			}
		})
	}
}

func TestF32Comparisons(t *testing.T) {
	f32f32ToI32 := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeF32, wasmtypes.ValueTypeF32},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
	}

	tests := []struct {
		name   string
		opcode byte
		a, b   float32
		expect int32
	}{
		{"eq", 0x5B, 1.0, 1.0, 1},
		{"ne", 0x5C, 1.0, 2.0, 1},
		{"lt", 0x5D, 1.0, 2.0, 1},
		{"gt", 0x5E, 2.0, 1.0, 1},
		{"le", 0x5F, 1.0, 1.0, 1},
		{"ge", 0x60, 2.0, 1.0, 1},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			code := []byte{0x20, 0x00, 0x20, 0x01, tt.opcode, 0x0B}
			body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
			engine := NewWasmExecutionEngine(EngineConfig{
				FuncTypes:     []wasmtypes.FuncType{f32f32ToI32},
				FuncBodies:    []*wasmtypes.FunctionBody{body},
				HostFunctions: []*HostFunction{nil},
			})
			results, err := engine.CallFunction(0, []WasmValue{F32(tt.a), F32(tt.b)})
			if err != nil {
				t.Fatal(err)
			}
			if AsI32(results[0]) != tt.expect {
				t.Fatalf("expected %d, got %d", tt.expect, AsI32(results[0]))
			}
		})
	}
}

// ════════════════════════════════════════════════════════════════════════
// F64 OPERATIONS
// ════════════════════════════════════════════════════════════════════════

func TestF64Arithmetic(t *testing.T) {
	f64f64ToF64 := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeF64, wasmtypes.ValueTypeF64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF64},
	}

	tests := []struct {
		name   string
		opcode byte
		a, b   float64
		expect float64
	}{
		{"add", 0xA0, 1.5, 2.5, 4.0},
		{"sub", 0xA1, 5.0, 2.0, 3.0},
		{"mul", 0xA2, 3.0, 4.0, 12.0},
		{"div", 0xA3, 10.0, 4.0, 2.5},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			code := []byte{0x20, 0x00, 0x20, 0x01, tt.opcode, 0x0B}
			body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
			engine := NewWasmExecutionEngine(EngineConfig{
				FuncTypes:     []wasmtypes.FuncType{f64f64ToF64},
				FuncBodies:    []*wasmtypes.FunctionBody{body},
				HostFunctions: []*HostFunction{nil},
			})
			results, err := engine.CallFunction(0, []WasmValue{F64(tt.a), F64(tt.b)})
			if err != nil {
				t.Fatal(err)
			}
			if AsF64(results[0]) != tt.expect {
				t.Fatalf("expected %v, got %v", tt.expect, AsF64(results[0]))
			}
		})
	}
}

func TestF64Unary(t *testing.T) {
	f64ToF64 := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeF64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF64},
	}

	tests := []struct {
		name   string
		opcode byte
		input  float64
		expect float64
	}{
		{"abs", 0x99, -5.0, 5.0},
		{"neg", 0x9A, 3.0, -3.0},
		{"ceil", 0x9B, 1.3, 2.0},
		{"floor", 0x9C, 1.7, 1.0},
		{"trunc", 0x9D, 1.7, 1.0},
		{"sqrt", 0x9F, 16.0, 4.0},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			code := []byte{0x20, 0x00, tt.opcode, 0x0B}
			body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
			engine := NewWasmExecutionEngine(EngineConfig{
				FuncTypes:     []wasmtypes.FuncType{f64ToF64},
				FuncBodies:    []*wasmtypes.FunctionBody{body},
				HostFunctions: []*HostFunction{nil},
			})
			results, err := engine.CallFunction(0, []WasmValue{F64(tt.input)})
			if err != nil {
				t.Fatal(err)
			}
			if AsF64(results[0]) != tt.expect {
				t.Fatalf("expected %v, got %v", tt.expect, AsF64(results[0]))
			}
		})
	}
}

func TestF64Comparisons(t *testing.T) {
	f64f64ToI32 := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeF64, wasmtypes.ValueTypeF64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
	}

	tests := []struct {
		name   string
		opcode byte
		a, b   float64
		expect int32
	}{
		{"eq", 0x61, 1.0, 1.0, 1},
		{"ne", 0x62, 1.0, 2.0, 1},
		{"lt", 0x63, 1.0, 2.0, 1},
		{"gt", 0x64, 2.0, 1.0, 1},
		{"le", 0x65, 1.0, 1.0, 1},
		{"ge", 0x66, 2.0, 1.0, 1},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			code := []byte{0x20, 0x00, 0x20, 0x01, tt.opcode, 0x0B}
			body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
			engine := NewWasmExecutionEngine(EngineConfig{
				FuncTypes:     []wasmtypes.FuncType{f64f64ToI32},
				FuncBodies:    []*wasmtypes.FunctionBody{body},
				HostFunctions: []*HostFunction{nil},
			})
			results, err := engine.CallFunction(0, []WasmValue{F64(tt.a), F64(tt.b)})
			if err != nil {
				t.Fatal(err)
			}
			if AsI32(results[0]) != tt.expect {
				t.Fatalf("expected %d, got %d", tt.expect, AsI32(results[0]))
			}
		})
	}
}

// ════════════════════════════════════════════════════════════════════════
// CONVERSION INSTRUCTIONS
// ════════════════════════════════════════════════════════════════════════

func TestI32WrapI64(t *testing.T) {
	// local.get 0; i32.wrap_i64; end
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
	}
	code := []byte{0x20, 0x00, 0xA7, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{I64(0x100000000 + 42)})
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 42 {
		t.Fatalf("expected 42, got %d", AsI32(results[0]))
	}
}

func TestI64ExtendI32S(t *testing.T) {
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
	}
	code := []byte{0x20, 0x00, 0xAC, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{I32(-1)})
	if err != nil {
		t.Fatal(err)
	}
	if AsI64(results[0]) != -1 {
		t.Fatalf("expected -1, got %d", AsI64(results[0]))
	}
}

func TestI64ExtendI32U(t *testing.T) {
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
	}
	code := []byte{0x20, 0x00, 0xAD, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{I32(-1)})
	if err != nil {
		t.Fatal(err)
	}
	if AsI64(results[0]) != int64(uint32(0xFFFFFFFF)) {
		t.Fatalf("expected %d, got %d", int64(uint32(0xFFFFFFFF)), AsI64(results[0]))
	}
}

func TestI32TruncF32S(t *testing.T) {
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeF32},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
	}
	code := []byte{0x20, 0x00, 0xA8, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{F32(3.7)})
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 3 {
		t.Fatalf("expected 3, got %d", AsI32(results[0]))
	}
}

func TestI32TruncF32STrapNaN(t *testing.T) {
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeF32},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
	}
	code := []byte{0x20, 0x00, 0xA8, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	expectTrap(t, engine, 0, []WasmValue{F32(float32(math.NaN()))}, "integer overflow")
}

func TestI32TruncF64S(t *testing.T) {
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeF64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
	}
	code := []byte{0x20, 0x00, 0xAA, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{F64(-3.7)})
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != -3 {
		t.Fatalf("expected -3, got %d", AsI32(results[0]))
	}
}

func TestReinterpretInstructions(t *testing.T) {
	// i32.reinterpret_f32: f32 -> i32
	{
		ft := wasmtypes.FuncType{
			Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeF32},
			Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		}
		code := []byte{0x20, 0x00, 0xBC, 0x0B}
		body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
		engine := NewWasmExecutionEngine(EngineConfig{
			FuncTypes:     []wasmtypes.FuncType{ft},
			FuncBodies:    []*wasmtypes.FunctionBody{body},
			HostFunctions: []*HostFunction{nil},
		})
		results, err := engine.CallFunction(0, []WasmValue{F32(1.0)})
		if err != nil {
			t.Fatal(err)
		}
		expected := int32(math.Float32bits(1.0))
		if AsI32(results[0]) != expected {
			t.Fatalf("expected %d, got %d", expected, AsI32(results[0]))
		}
	}

	// f32.reinterpret_i32: i32 -> f32
	{
		ft := wasmtypes.FuncType{
			Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
			Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF32},
		}
		code := []byte{0x20, 0x00, 0xBE, 0x0B}
		body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
		engine := NewWasmExecutionEngine(EngineConfig{
			FuncTypes:     []wasmtypes.FuncType{ft},
			FuncBodies:    []*wasmtypes.FunctionBody{body},
			HostFunctions: []*HostFunction{nil},
		})
		input := int32(math.Float32bits(1.0))
		results, err := engine.CallFunction(0, []WasmValue{I32(input)})
		if err != nil {
			t.Fatal(err)
		}
		if AsF32(results[0]) != 1.0 {
			t.Fatalf("expected 1.0, got %v", AsF32(results[0]))
		}
	}

	// i64.reinterpret_f64
	{
		ft := wasmtypes.FuncType{
			Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeF64},
			Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
		}
		code := []byte{0x20, 0x00, 0xBD, 0x0B}
		body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
		engine := NewWasmExecutionEngine(EngineConfig{
			FuncTypes:     []wasmtypes.FuncType{ft},
			FuncBodies:    []*wasmtypes.FunctionBody{body},
			HostFunctions: []*HostFunction{nil},
		})
		results, err := engine.CallFunction(0, []WasmValue{F64(1.0)})
		if err != nil {
			t.Fatal(err)
		}
		expected := int64(math.Float64bits(1.0))
		if AsI64(results[0]) != expected {
			t.Fatalf("expected %d, got %d", expected, AsI64(results[0]))
		}
	}

	// f64.reinterpret_i64
	{
		ft := wasmtypes.FuncType{
			Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
			Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF64},
		}
		code := []byte{0x20, 0x00, 0xBF, 0x0B}
		body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
		engine := NewWasmExecutionEngine(EngineConfig{
			FuncTypes:     []wasmtypes.FuncType{ft},
			FuncBodies:    []*wasmtypes.FunctionBody{body},
			HostFunctions: []*HostFunction{nil},
		})
		input := int64(math.Float64bits(1.0))
		results, err := engine.CallFunction(0, []WasmValue{I64(input)})
		if err != nil {
			t.Fatal(err)
		}
		if AsF64(results[0]) != 1.0 {
			t.Fatalf("expected 1.0, got %v", AsF64(results[0]))
		}
	}
}

func TestF32ConvertI32S(t *testing.T) {
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF32},
	}
	code := []byte{0x20, 0x00, 0xB2, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{I32(-5)})
	if err != nil {
		t.Fatal(err)
	}
	if AsF32(results[0]) != -5.0 {
		t.Fatalf("expected -5.0, got %v", AsF32(results[0]))
	}
}

func TestF64ConvertI32S(t *testing.T) {
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF64},
	}
	code := []byte{0x20, 0x00, 0xB7, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{I32(42)})
	if err != nil {
		t.Fatal(err)
	}
	if AsF64(results[0]) != 42.0 {
		t.Fatalf("expected 42.0, got %v", AsF64(results[0]))
	}
}

func TestF32DemoteF64(t *testing.T) {
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeF64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF32},
	}
	code := []byte{0x20, 0x00, 0xB6, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{F64(3.0)})
	if err != nil {
		t.Fatal(err)
	}
	if AsF32(results[0]) != 3.0 {
		t.Fatalf("expected 3.0, got %v", AsF32(results[0]))
	}
}

func TestF64PromoteF32(t *testing.T) {
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeF32},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF64},
	}
	code := []byte{0x20, 0x00, 0xBB, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{F32(3.0)})
	if err != nil {
		t.Fatal(err)
	}
	if AsF64(results[0]) != 3.0 {
		t.Fatalf("expected 3.0, got %v", AsF64(results[0]))
	}
}

// ════════════════════════════════════════════════════════════════════════
// CONTROL FLOW
// ════════════════════════════════════════════════════════════════════════

func TestSquareFunction(t *testing.T) {
	body := &wasmtypes.FunctionBody{
		Locals: nil,
		Code:   []byte{0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B},
	}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{{Params: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}}},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{I32(5)})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(results) != 1 || AsI32(results[0]) != 25 {
		t.Fatalf("expected [25], got %v", results)
	}
}

func TestBranchAndLoop(t *testing.T) {
	code := []byte{
		0x02, 0x40, // block (empty)
		0x03, 0x40, // loop (empty)
		0x20, 0x00, // local.get 0
		0x41, 0x0A, // i32.const 10
		0x4E,       // i32.ge_s
		0x0D, 0x01, // br_if 1
		0x20, 0x00, // local.get 0
		0x41, 0x01, // i32.const 1
		0x6A,       // i32.add
		0x21, 0x00, // local.set 0
		0x0C, 0x00, // br 0
		0x0B, // end loop
		0x0B, // end block
		0x20, 0x00, // local.get 0
		0x0B, // end function
	}

	body := &wasmtypes.FunctionBody{
		Locals: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		Code:   code,
	}

	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}}},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})

	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(results) != 1 || AsI32(results[0]) != 10 {
		t.Fatalf("expected [10], got %v", results)
	}
}

func TestIfElse(t *testing.T) {
	// if condition true: return 1, else: return 2
	// (func (param i32) (result i32)
	//   local.get 0
	//   if (result i32)
	//     i32.const 1
	//   else
	//     i32.const 2
	//   end)
	code := []byte{
		0x20, 0x00, // local.get 0
		0x04, 0x7F, // if (result i32)
		0x41, 0x01, // i32.const 1
		0x05,       // else
		0x41, 0x02, // i32.const 2
		0x0B,       // end
		0x0B,       // end function
	}

	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{i32ToI32},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})

	// True branch
	results, err := engine.CallFunction(0, []WasmValue{I32(1)})
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 1 {
		t.Fatalf("true branch: expected 1, got %d", AsI32(results[0]))
	}

	// False branch
	results, err = engine.CallFunction(0, []WasmValue{I32(0)})
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 2 {
		t.Fatalf("false branch: expected 2, got %d", AsI32(results[0]))
	}
}

func TestIfWithoutElse(t *testing.T) {
	// if condition is false and there's no else, skip to end
	// (func (param i32) (result i32)
	//   i32.const 55
	//   local.get 0
	//   if
	//     drop
	//     i32.const 42
	//   end)
	code := []byte{
		0x41, 0x37, // i32.const 55 (stays on stack if condition false)
		0x20, 0x00, // local.get 0
		0x04, 0x40, // if (empty block type)
		0x1A,       // drop (the 55)
		0x41, 0x2A, // i32.const 42
		0x0B,       // end if
		0x0B,       // end function
	}

	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{i32ToI32},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})

	// True branch: drops 99, pushes 42
	results, err := engine.CallFunction(0, []WasmValue{I32(1)})
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 42 {
		t.Fatalf("true branch: expected 42, got %d", AsI32(results[0]))
	}

	// False branch: 55 stays
	results, err = engine.CallFunction(0, []WasmValue{I32(0)})
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 55 {
		t.Fatalf("false branch: expected 55, got %d", AsI32(results[0]))
	}
}

func TestUnreachable(t *testing.T) {
	engine := makeEngine([]byte{0x00, 0x0B}, voidToI32, nil)
	expectTrap(t, engine, 0, nil, "unreachable")
}

func TestNop(t *testing.T) {
	// nop; i32.const 42; end
	engine := makeEngine([]byte{0x01, 0x41, 0x2A, 0x0B}, voidToI32, nil)
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 42 {
		t.Fatalf("expected 42, got %d", AsI32(results[0]))
	}
}

func TestReturn(t *testing.T) {
	// i32.const 5; return; i32.const 99; end
	engine := makeEngine([]byte{0x41, 0x05, 0x0F, 0x41, 0x63, 0x0B}, voidToI32, nil)
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 5 {
		t.Fatalf("expected 5, got %d", AsI32(results[0]))
	}
}

// ════════════════════════════════════════════════════════════════════════
// VARIABLE INSTRUCTIONS
// ════════════════════════════════════════════════════════════════════════

func TestLocalGetSet(t *testing.T) {
	// (func (param i32) (result i32)
	//   local.get 0; i32.const 10; i32.add; local.set 0; local.get 0; end)
	code := []byte{
		0x20, 0x00, // local.get 0
		0x41, 0x0A, // i32.const 10
		0x6A,       // i32.add
		0x21, 0x00, // local.set 0
		0x20, 0x00, // local.get 0
		0x0B,       // end
	}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{i32ToI32},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{I32(5)})
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 15 {
		t.Fatalf("expected 15, got %d", AsI32(results[0]))
	}
}

func TestLocalTee(t *testing.T) {
	// (func (param i32) (result i32)
	//   i32.const 42; local.tee 0; end)
	// local.tee sets the local AND leaves the value on the stack
	code := []byte{
		0x41, 0x2A, // i32.const 42
		0x22, 0x00, // local.tee 0
		0x0B, // end
	}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{i32ToI32},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{I32(0)})
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 42 {
		t.Fatalf("expected 42, got %d", AsI32(results[0]))
	}
}

func TestGlobalGetSet(t *testing.T) {
	// global.get 0; i32.const 1; i32.add; global.set 0; global.get 0; end
	code := []byte{
		0x23, 0x00, // global.get 0
		0x41, 0x01, // i32.const 1
		0x6A,       // i32.add
		0x24, 0x00, // global.set 0
		0x23, 0x00, // global.get 0
		0x0B,       // end
	}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	globals := []WasmValue{I32(10)}
	globalTypes := []wasmtypes.GlobalType{
		{ValueType: wasmtypes.ValueTypeI32, Mutable: true},
	}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{voidToI32},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
		Globals:       globals,
		GlobalTypes:   globalTypes,
	})
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 11 {
		t.Fatalf("expected 11, got %d", AsI32(results[0]))
	}
}

// ════════════════════════════════════════════════════════════════════════
// PARAMETRIC INSTRUCTIONS (DROP, SELECT)
// ════════════════════════════════════════════════════════════════════════

func TestDrop(t *testing.T) {
	// i32.const 55; i32.const 42; drop; end
	engine := makeEngine([]byte{0x41, 0x37, 0x41, 0x2A, 0x1A, 0x0B}, voidToI32, nil)
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 55 {
		t.Fatalf("expected 55 after drop, got %d", AsI32(results[0]))
	}
}

func TestSelect(t *testing.T) {
	// select: (val1, val2, condition) -> val1 if condition != 0, else val2
	// i32.const 10; i32.const 20; i32.const 1; select; end
	engine := makeEngine([]byte{
		0x41, 0x0A, // i32.const 10
		0x41, 0x14, // i32.const 20
		0x41, 0x01, // i32.const 1 (true)
		0x1B,       // select
		0x0B,       // end
	}, voidToI32, nil)
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 10 {
		t.Fatalf("select(true): expected 10, got %d", AsI32(results[0]))
	}

	// Condition = 0 => val2
	engine2 := makeEngine([]byte{
		0x41, 0x0A, 0x41, 0x14, 0x41, 0x00, 0x1B, 0x0B,
	}, voidToI32, nil)
	results, err = engine2.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 20 {
		t.Fatalf("select(false): expected 20, got %d", AsI32(results[0]))
	}
}

// ════════════════════════════════════════════════════════════════════════
// MEMORY INSTRUCTIONS
// ════════════════════════════════════════════════════════════════════════

func TestMemorySize(t *testing.T) {
	mem := NewLinearMemory(2, -1)
	// memory.size; end
	engine := makeEngine([]byte{0x3F, 0x00, 0x0B}, voidToI32, mem)
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 2 {
		t.Fatalf("expected 2 pages, got %d", AsI32(results[0]))
	}
}

func TestMemoryGrowInstruction(t *testing.T) {
	mem := NewLinearMemory(1, 5)
	// i32.const 2; memory.grow; end (should return old size = 1)
	engine := makeEngine([]byte{0x41, 0x02, 0x40, 0x00, 0x0B}, voidToI32, mem)
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 1 {
		t.Fatalf("expected old size 1, got %d", AsI32(results[0]))
	}
	if mem.Size() != 3 {
		t.Fatalf("expected 3 pages after grow, got %d", mem.Size())
	}
}

// ════════════════════════════════════════════════════════════════════════
// CONST EXPR
// ════════════════════════════════════════════════════════════════════════

func TestConstExpr(t *testing.T) {
	// i32.const 42; end
	result, err := EvaluateConstExpr([]byte{0x41, 0x2A, 0x0B}, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if AsI32(result) != 42 {
		t.Fatalf("expected 42, got %d", AsI32(result))
	}
}

func TestConstExprI64(t *testing.T) {
	// i64.const 10; end (10 = 0x0A in signed LEB128)
	result, err := EvaluateConstExpr([]byte{0x42, 0x0A, 0x0B}, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI64(result) != 10 {
		t.Fatalf("expected 10, got %d", AsI64(result))
	}
}

func TestConstExprF32(t *testing.T) {
	// f32.const 0.0; end
	result, err := EvaluateConstExpr([]byte{0x43, 0x00, 0x00, 0x00, 0x00, 0x0B}, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsF32(result) != 0.0 {
		t.Fatalf("expected 0.0, got %v", AsF32(result))
	}
}

func TestConstExprF64(t *testing.T) {
	// f64.const 0.0; end
	result, err := EvaluateConstExpr([]byte{0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0B}, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsF64(result) != 0.0 {
		t.Fatalf("expected 0.0, got %v", AsF64(result))
	}
}

func TestConstExprGlobalGet(t *testing.T) {
	globals := []WasmValue{I32(99)}
	// global.get 0; end
	result, err := EvaluateConstExpr([]byte{0x23, 0x00, 0x0B}, globals)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(result) != 99 {
		t.Fatalf("expected 99, got %d", AsI32(result))
	}
}

func TestConstExprGlobalGetOOB(t *testing.T) {
	_, err := EvaluateConstExpr([]byte{0x23, 0x05, 0x0B}, nil)
	if err == nil {
		t.Fatal("expected error for global.get OOB")
	}
}

func TestConstExprIllegalOpcode(t *testing.T) {
	_, err := EvaluateConstExpr([]byte{0x6A, 0x0B}, nil)
	if err == nil {
		t.Fatal("expected error for illegal opcode in const expr")
	}
}

func TestConstExprNoValue(t *testing.T) {
	_, err := EvaluateConstExpr([]byte{0x0B}, nil)
	if err == nil {
		t.Fatal("expected error for no-value const expr")
	}
}

func TestConstExprMissingEnd(t *testing.T) {
	_, err := EvaluateConstExpr([]byte{0x41, 0x2A}, nil)
	if err == nil {
		t.Fatal("expected error for missing end opcode")
	}
}

func TestConstExprF32TooShort(t *testing.T) {
	_, err := EvaluateConstExpr([]byte{0x43, 0x00, 0x00, 0x0B}, nil)
	if err == nil {
		t.Fatal("expected error for f32.const with not enough bytes")
	}
}

func TestConstExprF64TooShort(t *testing.T) {
	_, err := EvaluateConstExpr([]byte{0x44, 0x00, 0x00, 0x0B}, nil)
	if err == nil {
		t.Fatal("expected error for f64.const with not enough bytes")
	}
}

// ════════════════════════════════════════════════════════════════════════
// DECODER
// ════════════════════════════════════════════════════════════════════════

func TestDecodeFunctionBody(t *testing.T) {
	body := &wasmtypes.FunctionBody{
		Locals: nil,
		Code:   []byte{0x41, 0x03, 0x41, 0x04, 0x6A, 0x0B},
	}
	decoded := DecodeFunctionBody(body)
	if len(decoded) != 4 {
		t.Fatalf("expected 4 instructions, got %d", len(decoded))
	}
	if decoded[0].Opcode != 0x41 || decoded[2].Opcode != 0x6A || decoded[3].Opcode != 0x0B {
		t.Fatal("decoded opcodes don't match")
	}
}

func TestBuildControlFlowMap(t *testing.T) {
	// block; nop; end
	decoded := []DecodedInstruction{
		{Opcode: 0x02},
		{Opcode: 0x01},
		{Opcode: 0x0B},
	}
	cfMap := BuildControlFlowMap(decoded)
	target, ok := cfMap[0]
	if !ok {
		t.Fatal("expected control flow entry for block at index 0")
	}
	if target.EndPC != 2 || target.ElsePC != -1 {
		t.Fatalf("expected EndPC=2 ElsePC=-1, got %+v", target)
	}
}

func TestBuildControlFlowMapIfElse(t *testing.T) {
	// if; nop; else; nop; end
	decoded := []DecodedInstruction{
		{Opcode: 0x04},
		{Opcode: 0x01},
		{Opcode: 0x05},
		{Opcode: 0x01},
		{Opcode: 0x0B},
	}
	cfMap := BuildControlFlowMap(decoded)
	target, ok := cfMap[0]
	if !ok {
		t.Fatal("expected control flow entry for if at index 0")
	}
	if target.EndPC != 4 || target.ElsePC != 2 {
		t.Fatalf("expected EndPC=4 ElsePC=2, got %+v", target)
	}
}

func TestToVMInstructions(t *testing.T) {
	decoded := []DecodedInstruction{
		{Opcode: 0x41, Operand: 42},
		{Opcode: 0x0B, Operand: nil},
	}
	vmInstrs := ToVMInstructions(decoded)
	if len(vmInstrs) != 2 {
		t.Fatalf("expected 2, got %d", len(vmInstrs))
	}
	if vmInstrs[0].Opcode != vm.OpCode(0x41) || vmInstrs[0].Operand != 42 {
		t.Fatal("conversion failed")
	}
}

// ════════════════════════════════════════════════════════════════════════
// HOST FUNCTION
// ════════════════════════════════════════════════════════════════════════

func TestHostFunction(t *testing.T) {
	hf := &HostFunction{
		Type: wasmtypes.FuncType{
			Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
			Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		},
		Call: func(args []WasmValue) []WasmValue {
			v := AsI32(args[0])
			return []WasmValue{I32(v * 10)}
		},
	}

	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{hf.Type},
		FuncBodies:    []*wasmtypes.FunctionBody{nil},
		HostFunctions: []*HostFunction{hf},
	})

	results, err := engine.CallFunction(0, []WasmValue{I32(5)})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(results) != 1 || AsI32(results[0]) != 50 {
		t.Fatalf("expected [50], got %v", results)
	}
}

// ════════════════════════════════════════════════════════════════════════
// ENGINE ERROR PATHS
// ════════════════════════════════════════════════════════════════════════

func TestEngineUndefinedFunction(t *testing.T) {
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{voidToI32},
		FuncBodies:    []*wasmtypes.FunctionBody{nil},
		HostFunctions: []*HostFunction{nil},
	})
	_, err := engine.CallFunction(5, nil)
	if err == nil {
		t.Fatal("expected error for undefined function")
	}
}

func TestEngineArgCountMismatch(t *testing.T) {
	body := &wasmtypes.FunctionBody{Locals: nil, Code: []byte{0x0B}}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{i32ToI32},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	_, err := engine.CallFunction(0, nil) // expects 1 arg, got 0
	if err == nil {
		t.Fatal("expected error for arg count mismatch")
	}
}

func TestEngineNoBody(t *testing.T) {
	voidFt := wasmtypes.FuncType{Params: nil, Results: nil}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{voidFt},
		FuncBodies:    []*wasmtypes.FunctionBody{nil},
		HostFunctions: []*HostFunction{nil},
	})
	_, err := engine.CallFunction(0, nil)
	if err == nil {
		t.Fatal("expected error for no body")
	}
}

func TestDivByZeroTraps(t *testing.T) {
	body := &wasmtypes.FunctionBody{
		Locals: nil,
		Code:   []byte{0x41, 0x0A, 0x41, 0x00, 0x6D, 0x0B},
	}

	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}}},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})

	expectTrap(t, engine, 0, nil, "integer divide by zero")
}

// ════════════════════════════════════════════════════════════════════════
// TYPED STACK OPERATIONS
// ════════════════════════════════════════════════════════════════════════

func TestTypedStackOperations(t *testing.T) {
	genVM := vm.NewGenericVM()

	genVM.PushTyped(I32(42))
	genVM.PushTyped(I64(100))

	v := genVM.PopTyped()
	if v.Type != int(wasmtypes.ValueTypeI64) || v.Value.(int64) != 100 {
		t.Fatalf("expected i64(100), got %v", v)
	}

	v = genVM.PopTyped()
	if v.Type != int(wasmtypes.ValueTypeI32) || v.Value.(int32) != 42 {
		t.Fatalf("expected i32(42), got %v", v)
	}
}

// ════════════════════════════════════════════════════════════════════════
// LINEAR MEMORY — BASIC ROUNDTRIP (original test)
// ════════════════════════════════════════════════════════════════════════

func TestLinearMemory(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	mem.StoreI32(0, 42)
	if mem.LoadI32(0) != 42 {
		t.Fatalf("expected 42, got %d", mem.LoadI32(0))
	}

	mem.StoreI32_8(4, 0xFF)
	if mem.LoadI32_8s(4) != -1 {
		t.Fatalf("expected -1, got %d", mem.LoadI32_8s(4))
	}
	if mem.LoadI32_8u(4) != 255 {
		t.Fatalf("expected 255, got %d", mem.LoadI32_8u(4))
	}
}

// ════════════════════════════════════════════════════════════════════════
// TABLE — BASIC (original test)
// ════════════════════════════════════════════════════════════════════════

func TestTable(t *testing.T) {
	table := NewTable(4, -1)
	table.Set(0, 10)
	table.Set(1, 20)

	if table.Get(0) != 10 {
		t.Fatalf("expected 10, got %d", table.Get(0))
	}
	if table.Get(2) != -1 {
		t.Fatalf("expected -1 (null), got %d", table.Get(2))
	}
}

// ════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS (toInt, toInt32, etc.)
// ════════════════════════════════════════════════════════════════════════

func TestToIntVariants(t *testing.T) {
	// Test toInt with different types
	if toInt(int(5)) != 5 {
		t.Fatal("toInt(int) failed")
	}
	if toInt(int32(5)) != 5 {
		t.Fatal("toInt(int32) failed")
	}
	if toInt(int64(5)) != 5 {
		t.Fatal("toInt(int64) failed")
	}
	if toInt(uint64(5)) != 5 {
		t.Fatal("toInt(uint64) failed")
	}
	if toInt(float64(5)) != 5 {
		t.Fatal("toInt(float64) failed")
	}
	if toInt("nope") != 0 {
		t.Fatal("toInt(string) should return 0")
	}
}

func TestToInt32Variants(t *testing.T) {
	if toInt32(int(5)) != 5 {
		t.Fatal("toInt32(int) failed")
	}
	if toInt32(int64(5)) != 5 {
		t.Fatal("toInt32(int64) failed")
	}
	if toInt32(float64(5)) != 5 {
		t.Fatal("toInt32(float64) failed")
	}
	if toInt32("nope") != 0 {
		t.Fatal("toInt32(string) should return 0")
	}
}

func TestToInt64Variants(t *testing.T) {
	if toInt64(int64(5)) != 5 {
		t.Fatal("toInt64(int64) failed")
	}
	if toInt64(int(5)) != 5 {
		t.Fatal("toInt64(int) failed")
	}
	if toInt64(int32(5)) != 5 {
		t.Fatal("toInt64(int32) failed")
	}
	if toInt64("nope") != 0 {
		t.Fatal("toInt64(string) should return 0")
	}
}

func TestToFloat32Variants(t *testing.T) {
	if toFloat32(float32(3.0)) != 3.0 {
		t.Fatal("toFloat32(float32) failed")
	}
	if toFloat32(float64(3.0)) != 3.0 {
		t.Fatal("toFloat32(float64) failed")
	}
	if toFloat32("nope") != 0 {
		t.Fatal("toFloat32(string) should return 0")
	}
}

func TestToFloat64Variants(t *testing.T) {
	if toFloat64(float64(3.0)) != 3.0 {
		t.Fatal("toFloat64(float64) failed")
	}
	if toFloat64(float32(3.0)) != float64(float32(3.0)) {
		t.Fatal("toFloat64(float32) failed")
	}
	if toFloat64("nope") != 0 {
		t.Fatal("toFloat64(string) should return 0")
	}
}

func TestToIntOrDefault(t *testing.T) {
	if toIntOrDefault(nil, 99) != 99 {
		t.Fatal("toIntOrDefault(nil) should return default")
	}
	if toIntOrDefault(5, 99) != 5 {
		t.Fatal("toIntOrDefault(5) should return 5")
	}
}

func TestBlockArity(t *testing.T) {
	funcTypes := []wasmtypes.FuncType{
		{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32, wasmtypes.ValueTypeI32}},
	}
	if blockArity(0x40, funcTypes) != 0 {
		t.Fatal("empty block should have arity 0")
	}
	if blockArity(int(wasmtypes.ValueTypeI32), funcTypes) != 1 {
		t.Fatal("i32 block should have arity 1")
	}
	if blockArity(0, funcTypes) != 2 {
		t.Fatal("func type index 0 should have arity 2")
	}
}

func TestFuncTypesEqual(t *testing.T) {
	a := wasmtypes.FuncType{Params: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}}
	b := wasmtypes.FuncType{Params: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}}
	c := wasmtypes.FuncType{Params: []wasmtypes.ValueType{wasmtypes.ValueTypeI64}, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}}
	d := wasmtypes.FuncType{Params: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}, Results: nil}

	if !funcTypesEqual(a, b) {
		t.Fatal("should be equal")
	}
	if funcTypesEqual(a, c) {
		t.Fatal("different params should not be equal")
	}
	if funcTypesEqual(a, d) {
		t.Fatal("different results should not be equal")
	}
}

// ════════════════════════════════════════════════════════════════════════
// DECODESIGNED64
// ���═══════════════════════════════════════════════════════════════════════

func TestDecodeSigned64(t *testing.T) {
	// Encode 0 as LEB128
	val, consumed, err := decodeSigned64([]byte{0x00}, 0)
	if err != nil || val != 0 || consumed != 1 {
		t.Fatalf("decodeSigned64(0) = %d, %d, %v", val, consumed, err)
	}

	// Encode -1
	val, consumed, err = decodeSigned64([]byte{0x7F}, 0)
	if err != nil || val != -1 || consumed != 1 {
		t.Fatalf("decodeSigned64(-1) = %d, %d, %v", val, consumed, err)
	}
}

func TestDecodeSigned64Unterminated(t *testing.T) {
	_, _, err := decodeSigned64([]byte{0x80}, 0)
	if err == nil {
		t.Fatal("expected error for unterminated LEB128")
	}
}

func TestDecodeSigned64TooLong(t *testing.T) {
	// 11 continuation bytes (all with high bit set)
	data := make([]byte, 11)
	for i := range data {
		data[i] = 0x80
	}
	data[10] = 0x00
	_, _, err := decodeSigned64(data, 0)
	if err == nil {
		t.Fatal("expected error for too-long LEB128")
	}
}

// ════════════════════════════════════════════════════════════════════════
// MEMORY INSTRUCTIONS VIA ENGINE (i32.store/load, narrow variants)
// ════════════════════════════════════════════════════════════════════════

func TestMemoryStoreLoadI32ViaEngine(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	// i32.const 0; i32.const 42; i32.store offset=0 align=2; i32.const 0; i32.load offset=0 align=2; end
	code := []byte{
		0x41, 0x00, 0x41, 0x2A, 0x36, 0x02, 0x00, // store
		0x41, 0x00, 0x28, 0x02, 0x00, // load
		0x0B,
	}
	engine := makeEngine(code, voidToI32, mem)
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 42 {
		t.Fatalf("expected 42, got %d", AsI32(results[0]))
	}
}

func TestMemoryStoreLoadI64ViaEngine(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	voidToI64 := wasmtypes.FuncType{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64}}
	// i32.const 0; i64.const 10; i64.store offset=0 align=3; i32.const 0; i64.load offset=0 align=3; end
	code := []byte{
		0x41, 0x00, 0x42, 0x0A, 0x37, 0x03, 0x00, // store i64
		0x41, 0x00, 0x29, 0x03, 0x00, // load i64
		0x0B,
	}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		Memory:        mem,
		FuncTypes:     []wasmtypes.FuncType{voidToI64},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI64(results[0]) != 10 {
		t.Fatalf("expected 10, got %d", AsI64(results[0]))
	}
}

func TestMemoryStoreLoadF32ViaEngine(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	voidToF32 := wasmtypes.FuncType{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF32}}
	// i32.const 0; f32.const 0.0; f32.store; i32.const 0; f32.load; end
	// We need f32.const which reads 4 bytes of IEEE754. 0.0 = 00 00 00 00
	code := []byte{
		0x41, 0x00, // i32.const 0
		0x43, 0x00, 0x00, 0x00, 0x00, // f32.const 0.0
		0x38, 0x02, 0x00, // f32.store
		0x41, 0x00, // i32.const 0
		0x2A, 0x02, 0x00, // f32.load
		0x0B,
	}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		Memory:        mem,
		FuncTypes:     []wasmtypes.FuncType{voidToF32},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsF32(results[0]) != 0.0 {
		t.Fatalf("expected 0.0, got %v", AsF32(results[0]))
	}
}

func TestMemoryStoreLoadF64ViaEngine(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	voidToF64 := wasmtypes.FuncType{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF64}}
	code := []byte{
		0x41, 0x00, // i32.const 0
		0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // f64.const 0.0
		0x39, 0x03, 0x00, // f64.store
		0x41, 0x00, // i32.const 0
		0x2B, 0x03, 0x00, // f64.load
		0x0B,
	}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		Memory:        mem,
		FuncTypes:     []wasmtypes.FuncType{voidToF64},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsF64(results[0]) != 0.0 {
		t.Fatalf("expected 0.0, got %v", AsF64(results[0]))
	}
}

func TestMemoryNarrowLoadsViaEngine(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	mem.StoreI32_8(0, 0x80) // -128 signed, 128 unsigned

	// i32.const 0; i32.load8_s offset=0; end
	code_8s := []byte{0x41, 0x00, 0x2C, 0x00, 0x00, 0x0B}
	engine := makeEngine(code_8s, voidToI32, mem)
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != -128 {
		t.Fatalf("i32.load8_s: expected -128, got %d", AsI32(results[0]))
	}

	// i32.const 0; i32.load8_u offset=0; end
	code_8u := []byte{0x41, 0x00, 0x2D, 0x00, 0x00, 0x0B}
	engine2 := makeEngine(code_8u, voidToI32, mem)
	results, err = engine2.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 128 {
		t.Fatalf("i32.load8_u: expected 128, got %d", AsI32(results[0]))
	}
}

func TestMemoryNarrow16LoadsViaEngine(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	mem.StoreI32_16(0, int32(0x8000))

	// i32.const 0; i32.load16_s; end
	code := []byte{0x41, 0x00, 0x2E, 0x01, 0x00, 0x0B}
	engine := makeEngine(code, voidToI32, mem)
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != -32768 {
		t.Fatalf("i32.load16_s: expected -32768, got %d", AsI32(results[0]))
	}

	// i32.load16_u
	code2 := []byte{0x41, 0x00, 0x2F, 0x01, 0x00, 0x0B}
	engine2 := makeEngine(code2, voidToI32, mem)
	results, err = engine2.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 32768 {
		t.Fatalf("i32.load16_u: expected 32768, got %d", AsI32(results[0]))
	}
}

func TestMemoryNarrowStoresViaEngine(t *testing.T) {
	mem := NewLinearMemory(1, -1)

	// i32.const 0; i32.const 0xFF; i32.store8; end; then load back
	code := []byte{
		0x41, 0x00, // i32.const 0
		0x41, 0x7F, // i32.const -1 (= 0xFF as byte)
		0x3A, 0x00, 0x00, // i32.store8
		0x41, 0x00, // i32.const 0
		0x2D, 0x00, 0x00, // i32.load8_u
		0x0B,
	}
	engine := makeEngine(code, voidToI32, mem)
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	// -1 truncated to 8 bits = 0xFF = 255 unsigned
	if AsI32(results[0]) != 255 {
		t.Fatalf("expected 255, got %d", AsI32(results[0]))
	}
}

func TestMemoryI32Store16ViaEngine(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	// i32.const 0; i32.const 0x1234; i32.store16; i32.const 0; i32.load16_u; end
	// 0x1234 = 4660. In signed LEB128: 0xB4, 0x24
	code := []byte{
		0x41, 0x00,
		0x41, 0xB4, 0x24, // i32.const 4660
		0x3B, 0x01, 0x00, // i32.store16
		0x41, 0x00,
		0x2F, 0x01, 0x00, // i32.load16_u
		0x0B,
	}
	engine := makeEngine(code, voidToI32, mem)
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 4660 {
		t.Fatalf("expected 4660, got %d", AsI32(results[0]))
	}
}

func TestMemoryI64NarrowViaEngine(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	voidToI64 := wasmtypes.FuncType{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64}}

	// i32.const 0; i64.const 0xFF; i64.store8; i32.const 0; i64.load8_u; end
	// 0xFF signed LEB128 for i64 (255 unsigned, but signed LEB128 for i64.const 255 is 0xFF 0x01)
	code := []byte{
		0x41, 0x00,       // i32.const 0
		0x42, 0x01,       // i64.const 1
		0x3C, 0x00, 0x00, // i64.store8
		0x41, 0x00,       // i32.const 0
		0x31, 0x00, 0x00, // i64.load8_u
		0x0B,
	}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		Memory:        mem,
		FuncTypes:     []wasmtypes.FuncType{voidToI64},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI64(results[0]) != 1 {
		t.Fatalf("expected 1, got %d", AsI64(results[0]))
	}
}

func TestMemoryI64Store16ViaEngine(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	voidToI64 := wasmtypes.FuncType{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64}}
	code := []byte{
		0x41, 0x00,       // i32.const 0
		0x42, 0x01,       // i64.const 1
		0x3D, 0x01, 0x00, // i64.store16
		0x41, 0x00,       // i32.const 0
		0x33, 0x01, 0x00, // i64.load16_u
		0x0B,
	}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		Memory:        mem,
		FuncTypes:     []wasmtypes.FuncType{voidToI64},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI64(results[0]) != 1 {
		t.Fatalf("expected 1, got %d", AsI64(results[0]))
	}
}

func TestMemoryI64Store32ViaEngine(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	voidToI64 := wasmtypes.FuncType{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64}}
	code := []byte{
		0x41, 0x00,       // i32.const 0
		0x42, 0x2A,       // i64.const 42
		0x3E, 0x02, 0x00, // i64.store32
		0x41, 0x00,       // i32.const 0
		0x35, 0x02, 0x00, // i64.load32_u
		0x0B,
	}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		Memory:        mem,
		FuncTypes:     []wasmtypes.FuncType{voidToI64},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI64(results[0]) != 42 {
		t.Fatalf("expected 42, got %d", AsI64(results[0]))
	}
}

func TestMemoryI64Load8sViaEngine(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	mem.StoreI64_8(0, 0xFF) // 0xFF = -1 as signed byte
	voidToI64 := wasmtypes.FuncType{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64}}
	code := []byte{0x41, 0x00, 0x30, 0x00, 0x00, 0x0B} // i64.load8_s
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		Memory:        mem,
		FuncTypes:     []wasmtypes.FuncType{voidToI64},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI64(results[0]) != -1 {
		t.Fatalf("expected -1, got %d", AsI64(results[0]))
	}
}

func TestMemoryI64Load16sViaEngine(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	mem.StoreI64_16(0, 0xFFFF)
	voidToI64 := wasmtypes.FuncType{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64}}
	code := []byte{0x41, 0x00, 0x32, 0x01, 0x00, 0x0B} // i64.load16_s
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		Memory:        mem,
		FuncTypes:     []wasmtypes.FuncType{voidToI64},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI64(results[0]) != -1 {
		t.Fatalf("expected -1, got %d", AsI64(results[0]))
	}
}

func TestMemoryI64Load32sViaEngine(t *testing.T) {
	mem := NewLinearMemory(1, -1)
	mem.StoreI64_32(0, 0xFFFFFFFF)
	voidToI64 := wasmtypes.FuncType{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64}}
	code := []byte{0x41, 0x00, 0x34, 0x02, 0x00, 0x0B} // i64.load32_s
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		Memory:        mem,
		FuncTypes:     []wasmtypes.FuncType{voidToI64},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if AsI64(results[0]) != -1 {
		t.Fatalf("expected -1, got %d", AsI64(results[0]))
	}
}

// ════════════════════════════════════════════════════════════════════════
// CALL INSTRUCTION (internal function call)
// ════════════════════════════════════════════════════════════════════════

func TestCallInstruction(t *testing.T) {
	// Function 0: (i32) -> i32, doubles the input (local.get 0; local.get 0; i32.add; end)
	// Function 1: (i32) -> i32, calls function 0 (local.get 0; call 0; end)
	body0 := &wasmtypes.FunctionBody{
		Locals: nil,
		Code:   []byte{0x20, 0x00, 0x20, 0x00, 0x6A, 0x0B},
	}
	body1 := &wasmtypes.FunctionBody{
		Locals: nil,
		Code:   []byte{0x20, 0x00, 0x10, 0x00, 0x0B},
	}

	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes: []wasmtypes.FuncType{
			{Params: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}},
			{Params: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}},
		},
		FuncBodies:    []*wasmtypes.FunctionBody{body0, body1},
		HostFunctions: []*HostFunction{nil, nil},
	})

	results, err := engine.CallFunction(1, []WasmValue{I32(5)})
	if err != nil {
		t.Fatalf("call instruction failed: %v", err)
	}
	if AsI32(results[0]) != 10 {
		t.Fatalf("expected 10, got %d", AsI32(results[0]))
	}
}

func TestCallHostFromInternalCall(t *testing.T) {
	// Host function 0: doubles input
	// Module function 1: calls function 0
	hf := &HostFunction{
		Type: wasmtypes.FuncType{
			Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
			Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		},
		Call: func(args []WasmValue) []WasmValue {
			v := AsI32(args[0])
			return []WasmValue{I32(v * 3)}
		},
	}

	body := &wasmtypes.FunctionBody{
		Locals: nil,
		Code:   []byte{0x20, 0x00, 0x10, 0x00, 0x0B}, // local.get 0; call 0; end
	}

	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes: []wasmtypes.FuncType{
			hf.Type,
			{Params: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}},
		},
		FuncBodies:    []*wasmtypes.FunctionBody{nil, body},
		HostFunctions: []*HostFunction{hf, nil},
	})

	results, err := engine.CallFunction(1, []WasmValue{I32(7)})
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 21 {
		t.Fatalf("expected 21, got %d", AsI32(results[0]))
	}
}

// ════════════════════════════════════════════════════════════════════════
// MORE CONVERSION COVERAGE
// ════════════════════════════════════════════════════════════════════════

func TestI32TruncF32U(t *testing.T) {
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeF32},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
	}
	code := []byte{0x20, 0x00, 0xA9, 0x0B} // i32.trunc_f32_u
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{F32(3.7)})
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 3 {
		t.Fatalf("expected 3, got %d", AsI32(results[0]))
	}
}

func TestI32TruncF64U(t *testing.T) {
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeF64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
	}
	code := []byte{0x20, 0x00, 0xAB, 0x0B} // i32.trunc_f64_u
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{F64(3.7)})
	if err != nil {
		t.Fatal(err)
	}
	if AsI32(results[0]) != 3 {
		t.Fatalf("expected 3, got %d", AsI32(results[0]))
	}
}

func TestI64TruncF32S(t *testing.T) {
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeF32},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
	}
	code := []byte{0x20, 0x00, 0xAE, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{F32(-3.7)})
	if err != nil {
		t.Fatal(err)
	}
	if AsI64(results[0]) != -3 {
		t.Fatalf("expected -3, got %d", AsI64(results[0]))
	}
}

func TestI64TruncF64U(t *testing.T) {
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeF64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
	}
	code := []byte{0x20, 0x00, 0xB1, 0x0B} // i64.trunc_f64_u
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{F64(42.9)})
	if err != nil {
		t.Fatal(err)
	}
	if AsI64(results[0]) != 42 {
		t.Fatalf("expected 42, got %d", AsI64(results[0]))
	}
}

func TestF32ConvertI32U(t *testing.T) {
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF32},
	}
	code := []byte{0x20, 0x00, 0xB3, 0x0B} // f32.convert_i32_u
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{I32(5)})
	if err != nil {
		t.Fatal(err)
	}
	if AsF32(results[0]) != 5.0 {
		t.Fatalf("expected 5.0, got %v", AsF32(results[0]))
	}
}

func TestF32ConvertI64S(t *testing.T) {
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF32},
	}
	code := []byte{0x20, 0x00, 0xB4, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{I64(42)})
	if err != nil {
		t.Fatal(err)
	}
	if AsF32(results[0]) != 42.0 {
		t.Fatalf("expected 42.0, got %v", AsF32(results[0]))
	}
}

func TestF64ConvertI32U(t *testing.T) {
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF64},
	}
	code := []byte{0x20, 0x00, 0xB8, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{I32(5)})
	if err != nil {
		t.Fatal(err)
	}
	if AsF64(results[0]) != 5.0 {
		t.Fatalf("expected 5.0, got %v", AsF64(results[0]))
	}
}

func TestF64ConvertI64S(t *testing.T) {
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF64},
	}
	code := []byte{0x20, 0x00, 0xB9, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{I64(42)})
	if err != nil {
		t.Fatal(err)
	}
	if AsF64(results[0]) != 42.0 {
		t.Fatalf("expected 42.0, got %v", AsF64(results[0]))
	}
}

func TestF64ConvertI64U(t *testing.T) {
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF64},
	}
	code := []byte{0x20, 0x00, 0xBA, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{I64(42)})
	if err != nil {
		t.Fatal(err)
	}
	if AsF64(results[0]) != 42.0 {
		t.Fatalf("expected 42.0, got %v", AsF64(results[0]))
	}
}

func TestF32ConvertI64U(t *testing.T) {
	ft := wasmtypes.FuncType{
		Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI64},
		Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF32},
	}
	code := []byte{0x20, 0x00, 0xB5, 0x0B}
	body := &wasmtypes.FunctionBody{Locals: nil, Code: code}
	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{ft},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})
	results, err := engine.CallFunction(0, []WasmValue{I64(42)})
	if err != nil {
		t.Fatal(err)
	}
	if AsF32(results[0]) != 42.0 {
		t.Fatalf("expected 42.0, got %v", AsF32(results[0]))
	}
}
