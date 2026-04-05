package wasmexecution

import (
	"testing"

	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
)

// TestI32Arithmetic tests basic i32 arithmetic via the engine.
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

// TestSquareFunction tests a square(n) = n * n function.
//
// WASM bytecodes for square(x: i32) -> i32:
//   local.get 0    (0x20 0x00)
//   local.get 0    (0x20 0x00)
//   i32.mul        (0x6C)
//   end            (0x0B)
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

// TestLinearMemory tests basic memory load/store operations.
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

// TestTable tests basic table operations.
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

// TestConstExpr tests constant expression evaluation.
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

// TestHostFunction tests calling a host function.
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

// TestDivByZeroTraps tests that division by zero produces a TrapError.
func TestDivByZeroTraps(t *testing.T) {
	// i32.const 10; i32.const 0; i32.div_s; end
	body := &wasmtypes.FunctionBody{
		Locals: nil,
		Code:   []byte{0x41, 0x0A, 0x41, 0x00, 0x6D, 0x0B},
	}

	engine := NewWasmExecutionEngine(EngineConfig{
		FuncTypes:     []wasmtypes.FuncType{{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}}},
		FuncBodies:    []*wasmtypes.FunctionBody{body},
		HostFunctions: []*HostFunction{nil},
	})

	_, err := engine.CallFunction(0, nil)
	if err == nil {
		t.Fatal("expected trap error for division by zero")
	}
	te, ok := err.(*TrapError)
	if !ok {
		t.Fatalf("expected *TrapError, got %T: %v", err, err)
	}
	if te.Message != "integer divide by zero" {
		t.Fatalf("expected 'integer divide by zero', got %q", te.Message)
	}
}

// TestBranchAndLoop tests a simple counting loop using block/loop/br_if.
//
// This implements: let mut i = 0; while (i < 10) { i += 1; }; return i;
func TestBranchAndLoop(t *testing.T) {
	// (func (result i32)
	//   (local i32)               ;; local 0 = counter
	//   (block                    ;; label 1
	//     (loop                   ;; label 0
	//       local.get 0
	//       i32.const 10
	//       i32.ge_s              ;; i >= 10?
	//       br_if 1               ;; if so, break (branch to block end)
	//       local.get 0
	//       i32.const 1
	//       i32.add
	//       local.set 0
	//       br 0                  ;; continue loop
	//     )
	//   )
	//   local.get 0
	// )
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
		0x0B,       // end loop
		0x0B,       // end block
		0x20, 0x00, // local.get 0
		0x0B,       // end function
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
		t.Fatalf("expected [10], got %v (val=%d)", results, AsI32(results[0]))
	}
}

// TestTypedStackOperations tests push/pop on the typed stack.
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
