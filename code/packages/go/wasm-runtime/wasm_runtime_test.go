package wasmruntime

import (
	"testing"

	wasmexecution "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-execution"
	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
)

// ════════════════════════════════════════════════════════════════════════
// RUNTIME — FULL PIPELINE
// ════════════════════════════════════════════════════════════════════════

func TestRuntimeSquare(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types: []wasmtypes.FuncType{
			{Params: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}},
		},
		Functions: []uint32{0},
		Code: []wasmtypes.FunctionBody{
			{
				Locals: nil,
				Code:   []byte{0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B},
			},
		},
		Exports: []wasmtypes.Export{
			{Name: "square", Kind: wasmtypes.ExternalKindFunction, Index: 0},
		},
	}

	runtime := New(nil)

	_, err := runtime.Validate(module)
	if err != nil {
		t.Fatalf("validation failed: %v", err)
	}

	instance, err := runtime.Instantiate(module)
	if err != nil {
		t.Fatalf("instantiation failed: %v", err)
	}

	results, err := runtime.Call(instance, "square", []int{5})
	if err != nil {
		t.Fatalf("call failed: %v", err)
	}
	if len(results) != 1 || results[0] != 25 {
		t.Fatalf("expected [25], got %v", results)
	}
}

func TestRuntimeWithMemory(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types: []wasmtypes.FuncType{
			{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}},
		},
		Functions: []uint32{0},
		Code: []wasmtypes.FunctionBody{
			{
				Locals: nil,
				Code: []byte{
					0x41, 0x00,
					0x41, 0x2A,
					0x36, 0x02, 0x00,
					0x41, 0x00,
					0x28, 0x02, 0x00,
					0x0B,
				},
			},
		},
		Memories: []wasmtypes.MemoryType{
			{Limits: wasmtypes.Limits{Min: 1}},
		},
		Exports: []wasmtypes.Export{
			{Name: "store_and_load", Kind: wasmtypes.ExternalKindFunction, Index: 0},
		},
	}

	runtime := New(nil)
	instance, err := runtime.Instantiate(module)
	if err != nil {
		t.Fatalf("instantiation failed: %v", err)
	}

	results, err := runtime.Call(instance, "store_and_load", nil)
	if err != nil {
		t.Fatalf("call failed: %v", err)
	}
	if len(results) != 1 || results[0] != 42 {
		t.Fatalf("expected [42], got %v", results)
	}
}

// ════════════════════════════════════════════════════════════════════════
// RUNTIME — LOAD AND RUN
// ════════════════════════════════════════════════════════════════════════

func TestLoadAndRunInvalidWasm(t *testing.T) {
	runtime := New(nil)
	_, err := runtime.LoadAndRun([]byte{0x00, 0x01, 0x02}, "test", nil)
	if err == nil {
		t.Fatal("expected error for invalid wasm bytes")
	}
}

// ════════════════════════════════════════════════════════════════════════
// RUNTIME — INSTANTIATE WITH GLOBALS
// ════════════════════════════════════════════════════════════════════════

func TestInstantiateWithGlobals(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types: []wasmtypes.FuncType{
			{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}},
		},
		Functions: []uint32{0},
		Code: []wasmtypes.FunctionBody{
			{
				Locals: nil,
				Code:   []byte{0x23, 0x00, 0x0B}, // global.get 0; end
			},
		},
		Globals: []wasmtypes.Global{
			{
				GlobalType: wasmtypes.GlobalType{ValueType: wasmtypes.ValueTypeI32, Mutable: false},
				InitExpr:   []byte{0x41, 0x2A, 0x0B}, // i32.const 42; end
			},
		},
		Exports: []wasmtypes.Export{
			{Name: "get_global", Kind: wasmtypes.ExternalKindFunction, Index: 0},
		},
	}

	runtime := New(nil)
	instance, err := runtime.Instantiate(module)
	if err != nil {
		t.Fatalf("instantiation failed: %v", err)
	}
	results, err := runtime.Call(instance, "get_global", nil)
	if err != nil {
		t.Fatalf("call failed: %v", err)
	}
	if len(results) != 1 || results[0] != 42 {
		t.Fatalf("expected [42], got %v", results)
	}
}

// ════════════════════════════════════════════════════════════════════════
// RUNTIME — INSTANTIATE WITH DATA SEGMENTS
// ════════════════════════════════════════════════════════════════════════

func TestInstantiateWithDataSegments(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types: []wasmtypes.FuncType{
			{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}},
		},
		Functions: []uint32{0},
		Code: []wasmtypes.FunctionBody{
			{
				Locals: nil,
				Code: []byte{
					0x41, 0x00, // i32.const 0
					0x28, 0x02, 0x00, // i32.load offset=0
					0x0B, // end
				},
			},
		},
		Memories: []wasmtypes.MemoryType{
			{Limits: wasmtypes.Limits{Min: 1}},
		},
		Data: []wasmtypes.DataSegment{
			{
				MemoryIndex: 0,
				OffsetExpr:  []byte{0x41, 0x00, 0x0B},       // i32.const 0; end
				Data:        []byte{0x63, 0x00, 0x00, 0x00}, // 99 in little-endian
			},
		},
		Exports: []wasmtypes.Export{
			{Name: "read_data", Kind: wasmtypes.ExternalKindFunction, Index: 0},
		},
	}

	runtime := New(nil)
	instance, err := runtime.Instantiate(module)
	if err != nil {
		t.Fatalf("instantiation failed: %v", err)
	}
	results, err := runtime.Call(instance, "read_data", nil)
	if err != nil {
		t.Fatalf("call failed: %v", err)
	}
	if len(results) != 1 || results[0] != 99 {
		t.Fatalf("expected [99], got %v", results)
	}
}

// ════════════════════════════════════════════════════════════════════════
// RUNTIME — INSTANTIATE WITH TABLES AND ELEMENTS
// ════════════════════════════════════════════════════════════════════════

func TestInstantiateWithTableAndElements(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types: []wasmtypes.FuncType{
			{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}},
		},
		Functions: []uint32{0},
		Code: []wasmtypes.FunctionBody{
			{
				Locals: nil,
				Code:   []byte{0x41, 0x2A, 0x0B}, // i32.const 42; end
			},
		},
		Tables: []wasmtypes.TableType{
			{ElementType: 0x70, Limits: wasmtypes.Limits{Min: 4}},
		},
		Elements: []wasmtypes.Element{
			{
				TableIndex:      0,
				OffsetExpr:      []byte{0x41, 0x00, 0x0B}, // i32.const 0; end
				FunctionIndices: []uint32{0},
			},
		},
		Exports: []wasmtypes.Export{
			{Name: "func0", Kind: wasmtypes.ExternalKindFunction, Index: 0},
		},
	}

	runtime := New(nil)
	instance, err := runtime.Instantiate(module)
	if err != nil {
		t.Fatalf("instantiation failed: %v", err)
	}

	// Verify table was populated.
	if len(instance.Tables) != 1 {
		t.Fatalf("expected 1 table, got %d", len(instance.Tables))
	}
	if instance.Tables[0].Get(0) != 0 {
		t.Fatalf("expected func index 0 at table[0], got %d", instance.Tables[0].Get(0))
	}
}

// ════════════════════════════════════════════════════════════════════════
// RUNTIME — INSTANTIATE WITH MEMORY MAX
// ════════════════════════════════════════════════════════════════════════

func TestInstantiateWithMemoryMax(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types: []wasmtypes.FuncType{
			{Params: nil, Results: nil},
		},
		Functions: []uint32{0},
		Code: []wasmtypes.FunctionBody{
			{Locals: nil, Code: []byte{0x0B}},
		},
		Memories: []wasmtypes.MemoryType{
			{Limits: wasmtypes.Limits{Min: 1, Max: 5, HasMax: true}},
		},
		Exports: []wasmtypes.Export{
			{Name: "noop", Kind: wasmtypes.ExternalKindFunction, Index: 0},
		},
	}

	runtime := New(nil)
	instance, err := runtime.Instantiate(module)
	if err != nil {
		t.Fatalf("instantiation failed: %v", err)
	}
	if instance.Memory == nil {
		t.Fatal("expected memory to be allocated")
	}
	if instance.Memory.Size() != 1 {
		t.Fatalf("expected 1 page, got %d", instance.Memory.Size())
	}
}

// ════════════════════════════════════════════════════════════════════════
// RUNTIME — CALL ERROR PATHS
// ════════════════════════════════════════════════════════════════════════

func TestInstantiateBindsMemoryIntoWasiStub(t *testing.T) {
	wasi := NewWasiStub(nil, nil)
	module := &wasmtypes.WasmModule{
		Types: []wasmtypes.FuncType{
			{Params: nil, Results: nil},
		},
		Functions: []uint32{0},
		Code: []wasmtypes.FunctionBody{
			{Locals: nil, Code: []byte{0x0B}},
		},
		Memories: []wasmtypes.MemoryType{
			{Limits: wasmtypes.Limits{Min: 1}},
		},
		Exports: []wasmtypes.Export{
			{Name: "noop", Kind: wasmtypes.ExternalKindFunction, Index: 0},
		},
	}

	runtime := New(wasi)
	instance, err := runtime.Instantiate(module)
	if err != nil {
		t.Fatalf("instantiation failed: %v", err)
	}
	if instance.Memory == nil {
		t.Fatal("expected instance memory to be allocated")
	}
	if wasi.instanceMemory != instance.Memory {
		t.Fatal("expected runtime to bind instance memory into WasiStub")
	}
}

func TestCallExportNotFound(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types:     []wasmtypes.FuncType{{Params: nil, Results: nil}},
		Functions: []uint32{0},
		Code:      []wasmtypes.FunctionBody{{Locals: nil, Code: []byte{0x0B}}},
		Exports:   []wasmtypes.Export{{Name: "foo", Kind: wasmtypes.ExternalKindFunction, Index: 0}},
	}

	runtime := New(nil)
	instance, _ := runtime.Instantiate(module)

	_, err := runtime.Call(instance, "bar", nil)
	if err == nil {
		t.Fatal("expected error for export not found")
	}
}

func TestCallExportNotAFunction(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types:     []wasmtypes.FuncType{{Params: nil, Results: nil}},
		Functions: []uint32{0},
		Code:      []wasmtypes.FunctionBody{{Locals: nil, Code: []byte{0x0B}}},
		Memories:  []wasmtypes.MemoryType{{Limits: wasmtypes.Limits{Min: 1}}},
		Exports: []wasmtypes.Export{
			{Name: "memory", Kind: wasmtypes.ExternalKindMemory, Index: 0},
		},
	}

	runtime := New(nil)
	instance, _ := runtime.Instantiate(module)

	_, err := runtime.Call(instance, "memory", nil)
	if err == nil {
		t.Fatal("expected error for non-function export")
	}
}

// ════════════════════════════════════════════════════════════════════════
// RUNTIME — CALL WITH DIFFERENT VALUE TYPES
// ════════════════════════════════════════════════════════════════════════

func TestCallWithI64Return(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types: []wasmtypes.FuncType{
			{Params: []wasmtypes.ValueType{wasmtypes.ValueTypeI64}, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI64}},
		},
		Functions: []uint32{0},
		Code: []wasmtypes.FunctionBody{
			{Locals: nil, Code: []byte{0x20, 0x00, 0x0B}}, // local.get 0; end
		},
		Exports: []wasmtypes.Export{
			{Name: "identity", Kind: wasmtypes.ExternalKindFunction, Index: 0},
		},
	}

	runtime := New(nil)
	instance, err := runtime.Instantiate(module)
	if err != nil {
		t.Fatal(err)
	}
	results, err := runtime.Call(instance, "identity", []int{42})
	if err != nil {
		t.Fatal(err)
	}
	if results[0] != 42 {
		t.Fatalf("expected 42, got %d", results[0])
	}
}

func TestCallWithF32Params(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types: []wasmtypes.FuncType{
			{Params: []wasmtypes.ValueType{wasmtypes.ValueTypeF32}, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF32}},
		},
		Functions: []uint32{0},
		Code: []wasmtypes.FunctionBody{
			{Locals: nil, Code: []byte{0x20, 0x00, 0x0B}},
		},
		Exports: []wasmtypes.Export{
			{Name: "identity", Kind: wasmtypes.ExternalKindFunction, Index: 0},
		},
	}

	runtime := New(nil)
	instance, err := runtime.Instantiate(module)
	if err != nil {
		t.Fatal(err)
	}
	results, err := runtime.Call(instance, "identity", []int{5})
	if err != nil {
		t.Fatal(err)
	}
	if results[0] != 5 {
		t.Fatalf("expected 5, got %d", results[0])
	}
}

func TestCallWithF64Params(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types: []wasmtypes.FuncType{
			{Params: []wasmtypes.ValueType{wasmtypes.ValueTypeF64}, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeF64}},
		},
		Functions: []uint32{0},
		Code: []wasmtypes.FunctionBody{
			{Locals: nil, Code: []byte{0x20, 0x00, 0x0B}},
		},
		Exports: []wasmtypes.Export{
			{Name: "identity", Kind: wasmtypes.ExternalKindFunction, Index: 0},
		},
	}

	runtime := New(nil)
	instance, err := runtime.Instantiate(module)
	if err != nil {
		t.Fatal(err)
	}
	results, err := runtime.Call(instance, "identity", []int{7})
	if err != nil {
		t.Fatal(err)
	}
	if results[0] != 7 {
		t.Fatalf("expected 7, got %d", results[0])
	}
}

// ════════════════════════════════════════════════════════════════════════
// RUNTIME — START FUNCTION
// ════════════════════════════════════════════════════════════════════════

func TestInstantiateWithStartFunction(t *testing.T) {
	startIdx := uint32(0)
	module := &wasmtypes.WasmModule{
		Types: []wasmtypes.FuncType{
			{Params: nil, Results: nil}, // start fn type
			{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}}, // getter type
		},
		Functions: []uint32{0, 1},
		Code: []wasmtypes.FunctionBody{
			{
				Locals: nil,
				Code: []byte{
					0x41, 0x2A, // i32.const 42
					0x24, 0x00, // global.set 0
					0x0B, // end
				},
			},
			{
				Locals: nil,
				Code:   []byte{0x23, 0x00, 0x0B}, // global.get 0; end
			},
		},
		Globals: []wasmtypes.Global{
			{
				GlobalType: wasmtypes.GlobalType{ValueType: wasmtypes.ValueTypeI32, Mutable: true},
				InitExpr:   []byte{0x41, 0x00, 0x0B}, // i32.const 0; end
			},
		},
		Start: &startIdx,
		Exports: []wasmtypes.Export{
			{Name: "get_val", Kind: wasmtypes.ExternalKindFunction, Index: 1},
		},
	}

	runtime := New(nil)
	instance, err := runtime.Instantiate(module)
	if err != nil {
		t.Fatalf("instantiation failed: %v", err)
	}

	// Start function should have set global to 42.
	results, err := runtime.Call(instance, "get_val", nil)
	if err != nil {
		t.Fatal(err)
	}
	if results[0] != 42 {
		t.Fatalf("expected 42 from start fn, got %d", results[0])
	}
}

// ════════════════════════════════════════════════════════════════════════
// RUNTIME — HOST INTERFACE (IMPORTS)
// ════════════════════════════════════════════════════════════════════════

// simpleHost implements HostInterface with a single function.
type simpleHost struct {
	fn *wasmexecution.HostFunction
}

func (h *simpleHost) ResolveFunction(moduleName, name string) *wasmexecution.HostFunction {
	if moduleName == "env" && name == "double" {
		return h.fn
	}
	return nil
}
func (h *simpleHost) ResolveGlobal(moduleName, name string) *wasmexecution.HostGlobal {
	if moduleName == "env" && name == "g1" {
		return &wasmexecution.HostGlobal{
			Type:  wasmtypes.GlobalType{ValueType: wasmtypes.ValueTypeI32, Mutable: false},
			Value: wasmexecution.I32(100),
		}
	}
	return nil
}
func (h *simpleHost) ResolveMemory(moduleName, name string) *wasmexecution.LinearMemory {
	return nil
}
func (h *simpleHost) ResolveTable(moduleName, name string) *wasmexecution.Table {
	return nil
}

func TestInstantiateWithImports(t *testing.T) {
	host := &simpleHost{
		fn: &wasmexecution.HostFunction{
			Type: wasmtypes.FuncType{
				Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
				Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
			},
			Call: func(args []wasmexecution.WasmValue) []wasmexecution.WasmValue {
				v := wasmexecution.AsI32(args[0])
				return []wasmexecution.WasmValue{wasmexecution.I32(v * 2)}
			},
		},
	}

	module := &wasmtypes.WasmModule{
		Types: []wasmtypes.FuncType{
			{Params: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}},
		},
		Imports: []wasmtypes.Import{
			{ModuleName: "env", Name: "double", Kind: wasmtypes.ExternalKindFunction, TypeInfo: uint32(0)},
		},
		Functions: []uint32{0}, // local function also uses type 0
		Code: []wasmtypes.FunctionBody{
			{
				Locals: nil,
				Code: []byte{
					0x20, 0x00, // local.get 0
					0x10, 0x00, // call 0 (the imported "double" function)
					0x0B, // end
				},
			},
		},
		Exports: []wasmtypes.Export{
			{Name: "call_double", Kind: wasmtypes.ExternalKindFunction, Index: 1},
		},
	}

	runtime := New(host)
	instance, err := runtime.Instantiate(module)
	if err != nil {
		t.Fatalf("instantiation failed: %v", err)
	}

	results, err := runtime.Call(instance, "call_double", []int{5})
	if err != nil {
		t.Fatalf("call failed: %v", err)
	}
	if results[0] != 10 {
		t.Fatalf("expected 10, got %d", results[0])
	}
}

func TestInstantiateWithImportedGlobal(t *testing.T) {
	host := &simpleHost{fn: nil}

	module := &wasmtypes.WasmModule{
		Types: []wasmtypes.FuncType{
			{Params: nil, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}},
		},
		Imports: []wasmtypes.Import{
			{ModuleName: "env", Name: "g1", Kind: wasmtypes.ExternalKindGlobal, TypeInfo: wasmtypes.GlobalType{ValueType: wasmtypes.ValueTypeI32, Mutable: false}},
		},
		Functions: []uint32{0},
		Code: []wasmtypes.FunctionBody{
			{
				Locals: nil,
				Code:   []byte{0x23, 0x00, 0x0B}, // global.get 0; end
			},
		},
		Exports: []wasmtypes.Export{
			{Name: "get_g", Kind: wasmtypes.ExternalKindFunction, Index: 0},
		},
	}

	runtime := New(host)
	instance, err := runtime.Instantiate(module)
	if err != nil {
		t.Fatalf("instantiation failed: %v", err)
	}
	results, err := runtime.Call(instance, "get_g", nil)
	if err != nil {
		t.Fatalf("call failed: %v", err)
	}
	if results[0] != 100 {
		t.Fatalf("expected 100, got %d", results[0])
	}
}

// ════════════════════════════════════════════════════════════════════════
// WASI STUB
// ════════════════════════════════════════════════════════════════════════

func TestWasiStub(t *testing.T) {
	var stdout []string
	wasi := NewWasiHost(
		func(text string) { stdout = append(stdout, text) },
		nil,
	)

	fdWrite := wasi.ResolveFunction("wasi_snapshot_preview1", "fd_write")
	if fdWrite == nil {
		t.Fatal("fd_write should be resolvable")
	}

	if wasi.ResolveFunction("env", "something") != nil {
		t.Fatal("non-WASI module should return nil")
	}

	procExit := wasi.ResolveFunction("wasi_snapshot_preview1", "proc_exit")
	if procExit == nil {
		t.Fatal("proc_exit should be resolvable")
	}
	func() {
		defer func() {
			r := recover()
			if r == nil {
				t.Fatal("proc_exit should panic")
			}
			pe, ok := r.(*ProcExitError)
			if !ok {
				t.Fatalf("expected *ProcExitError, got %T", r)
			}
			if pe.ExitCode != 0 {
				t.Fatalf("expected exit code 0, got %d", pe.ExitCode)
			}
		}()
		procExit.Call([]wasmexecution.WasmValue{wasmexecution.I32(0)})
	}()
}

func TestWasiStubResolvers(t *testing.T) {
	wasi := NewWasiHost(nil, nil)

	// These should all return nil.
	if wasi.ResolveGlobal("wasi_snapshot_preview1", "anything") != nil {
		t.Fatal("expected nil for global")
	}
	if wasi.ResolveMemory("wasi_snapshot_preview1", "anything") != nil {
		t.Fatal("expected nil for memory")
	}
	if wasi.ResolveTable("wasi_snapshot_preview1", "anything") != nil {
		t.Fatal("expected nil for table")
	}
}

func TestWasiStubUnknownFunction(t *testing.T) {
	wasi := NewWasiHost(nil, nil)
	// Use a function name that will never be implemented — the WASI spec
	// does not define "totally_not_a_real_wasi_function".
	stub := wasi.ResolveFunction("wasi_snapshot_preview1", "totally_not_a_real_wasi_function")
	if stub == nil {
		t.Fatal("unknown WASI functions should return a stub")
	}
	// Calling the stub should return ENOSYS (52).
	result := stub.Call(nil)
	if len(result) != 1 {
		t.Fatalf("expected 1 result, got %d", len(result))
	}
	if wasmexecution.AsI32(result[0]) != 52 {
		t.Fatalf("expected ENOSYS(52), got %d", wasmexecution.AsI32(result[0]))
	}
}

func TestWasiStubFdWriteNoMemory(t *testing.T) {
	wasi := NewWasiHost(nil, nil)
	fdWrite := wasi.ResolveFunction("wasi_snapshot_preview1", "fd_write")

	// fd_write without memory set should return ENOSYS.
	result := fdWrite.Call([]wasmexecution.WasmValue{
		wasmexecution.I32(1),
		wasmexecution.I32(0),
		wasmexecution.I32(0),
		wasmexecution.I32(0),
	})
	if wasmexecution.AsI32(result[0]) != 52 {
		t.Fatalf("expected ENOSYS(52), got %d", wasmexecution.AsI32(result[0]))
	}
}

func TestWasiStubFdWriteStdout(t *testing.T) {
	var output string
	wasi := NewWasiHost(func(text string) { output += text }, nil)

	mem := wasmexecution.NewLinearMemory(1, -1)
	wasi.SetMemory(mem)

	// Set up an iov at offset 100: pointer=200, length=5
	mem.StoreI32(100, 200)               // buf_ptr = 200
	mem.StoreI32(104, 5)                 // buf_len = 5
	mem.WriteBytes(200, []byte("Hello")) // The actual string data

	fdWrite := wasi.ResolveFunction("wasi_snapshot_preview1", "fd_write")
	result := fdWrite.Call([]wasmexecution.WasmValue{
		wasmexecution.I32(1),   // fd = stdout
		wasmexecution.I32(100), // iovs_ptr
		wasmexecution.I32(1),   // iovs_len = 1
		wasmexecution.I32(300), // nwritten_ptr
	})

	if wasmexecution.AsI32(result[0]) != 0 {
		t.Fatalf("expected success(0), got %d", wasmexecution.AsI32(result[0]))
	}
	if output != "Hello" {
		t.Fatalf("expected 'Hello', got %q", output)
	}
	// Check nwritten.
	nwritten := mem.LoadI32(300)
	if nwritten != 5 {
		t.Fatalf("expected nwritten=5, got %d", nwritten)
	}
}

func TestWasiStubFdWriteStderr(t *testing.T) {
	var errOutput string
	wasi := NewWasiHost(nil, func(text string) { errOutput += text })

	mem := wasmexecution.NewLinearMemory(1, -1)
	wasi.SetMemory(mem)

	mem.StoreI32(100, 200)
	mem.StoreI32(104, 3)
	mem.WriteBytes(200, []byte("Err"))

	fdWrite := wasi.ResolveFunction("wasi_snapshot_preview1", "fd_write")
	result := fdWrite.Call([]wasmexecution.WasmValue{
		wasmexecution.I32(2), // fd = stderr
		wasmexecution.I32(100),
		wasmexecution.I32(1),
		wasmexecution.I32(300),
	})

	if wasmexecution.AsI32(result[0]) != 0 {
		t.Fatalf("expected success(0), got %d", wasmexecution.AsI32(result[0]))
	}
	if errOutput != "Err" {
		t.Fatalf("expected 'Err', got %q", errOutput)
	}
}

func TestProcExitError(t *testing.T) {
	e := &ProcExitError{ExitCode: 42}
	if e.Error() != "proc_exit(42)" {
		t.Fatalf("unexpected error string: %s", e.Error())
	}
}

func TestWasiProcExitNonZero(t *testing.T) {
	wasi := NewWasiHost(nil, nil)
	procExit := wasi.ResolveFunction("wasi_snapshot_preview1", "proc_exit")

	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("proc_exit should panic")
		}
		pe, ok := r.(*ProcExitError)
		if !ok {
			t.Fatalf("expected *ProcExitError, got %T", r)
		}
		if pe.ExitCode != 1 {
			t.Fatalf("expected exit code 1, got %d", pe.ExitCode)
		}
	}()
	procExit.Call([]wasmexecution.WasmValue{wasmexecution.I32(1)})
}

func TestWasiStubFdReadStdin(t *testing.T) {
	wasi := NewWasiHostFromConfig(WasiConfig{
		StdinCallback: func(n int) []byte {
			if n > 3 {
				n = 3
			}
			return []byte("hey")[:n]
		},
	})

	mem := wasmexecution.NewLinearMemory(1, -1)
	wasi.SetMemory(mem)
	mem.StoreI32(100, 200)
	mem.StoreI32(104, 3)

	fdRead := wasi.ResolveFunction("wasi_snapshot_preview1", "fd_read")
	result := fdRead.Call([]wasmexecution.WasmValue{
		wasmexecution.I32(0),
		wasmexecution.I32(100),
		wasmexecution.I32(1),
		wasmexecution.I32(300),
	})

	if wasmexecution.AsI32(result[0]) != 0 {
		t.Fatalf("expected success(0), got %d", wasmexecution.AsI32(result[0]))
	}
	bytes := []byte{
		byte(mem.LoadI32_8u(200)),
		byte(mem.LoadI32_8u(201)),
		byte(mem.LoadI32_8u(202)),
	}
	if string(bytes) != "hey" {
		t.Fatalf("expected stdin bytes at buffer, got %q", string(bytes))
	}
	if nread := mem.LoadI32(300); nread != 3 {
		t.Fatalf("expected nread=3, got %d", nread)
	}
}

func TestWasiStubFdReadRejectsNonStdinFd(t *testing.T) {
	wasi := NewWasiHostFromConfig(WasiConfig{
		StdinCallback: func(n int) []byte { return make([]byte, n) },
	})
	wasi.SetMemory(wasmexecution.NewLinearMemory(1, -1))
	fdRead := wasi.ResolveFunction("wasi_snapshot_preview1", "fd_read")
	result := fdRead.Call([]wasmexecution.WasmValue{
		wasmexecution.I32(1),
		wasmexecution.I32(0),
		wasmexecution.I32(0),
		wasmexecution.I32(0),
	})
	if wasmexecution.AsI32(result[0]) != wasiEBadf {
		t.Fatalf("expected EBADF(%d), got %d", wasiEBadf, wasmexecution.AsI32(result[0]))
	}
}

// ════════════════════════════════════════════════════════════════════════
// RUNTIME — INSTANTIATE WITH IMPORTS (table, memory)
// ════════════════════════════════════════════════════════════════════════

// fullHost implements HostInterface with memory and table resolution.
type fullHost struct{}

func (h *fullHost) ResolveFunction(moduleName, name string) *wasmexecution.HostFunction {
	return nil
}
func (h *fullHost) ResolveGlobal(moduleName, name string) *wasmexecution.HostGlobal {
	return nil
}
func (h *fullHost) ResolveMemory(moduleName, name string) *wasmexecution.LinearMemory {
	if moduleName == "env" && name == "memory" {
		return wasmexecution.NewLinearMemory(2, -1)
	}
	return nil
}
func (h *fullHost) ResolveTable(moduleName, name string) *wasmexecution.Table {
	if moduleName == "env" && name == "table" {
		return wasmexecution.NewTable(4, -1)
	}
	return nil
}

func TestInstantiateWithImportedMemory(t *testing.T) {
	host := &fullHost{}

	module := &wasmtypes.WasmModule{
		Types: []wasmtypes.FuncType{
			{Params: nil, Results: nil},
		},
		Imports: []wasmtypes.Import{
			{ModuleName: "env", Name: "memory", Kind: wasmtypes.ExternalKindMemory, TypeInfo: wasmtypes.MemoryType{Limits: wasmtypes.Limits{Min: 1}}},
		},
		Functions: []uint32{0},
		Code:      []wasmtypes.FunctionBody{{Locals: nil, Code: []byte{0x0B}}},
		Exports:   []wasmtypes.Export{{Name: "noop", Kind: wasmtypes.ExternalKindFunction, Index: 0}},
	}

	runtime := New(host)
	instance, err := runtime.Instantiate(module)
	if err != nil {
		t.Fatalf("instantiation failed: %v", err)
	}
	if instance.Memory == nil {
		t.Fatal("expected imported memory")
	}
	if instance.Memory.Size() != 2 {
		t.Fatalf("expected 2 pages from imported memory, got %d", instance.Memory.Size())
	}
}

func TestInstantiateWithImportedTable(t *testing.T) {
	host := &fullHost{}

	module := &wasmtypes.WasmModule{
		Types: []wasmtypes.FuncType{
			{Params: nil, Results: nil},
		},
		Imports: []wasmtypes.Import{
			{ModuleName: "env", Name: "table", Kind: wasmtypes.ExternalKindTable, TypeInfo: wasmtypes.TableType{Limits: wasmtypes.Limits{Min: 2}}},
		},
		Functions: []uint32{0},
		Code:      []wasmtypes.FunctionBody{{Locals: nil, Code: []byte{0x0B}}},
		Exports:   []wasmtypes.Export{{Name: "noop", Kind: wasmtypes.ExternalKindFunction, Index: 0}},
	}

	runtime := New(host)
	instance, err := runtime.Instantiate(module)
	if err != nil {
		t.Fatalf("instantiation failed: %v", err)
	}
	if len(instance.Tables) != 1 {
		t.Fatalf("expected 1 table, got %d", len(instance.Tables))
	}
}

// ════════════════════════════════════════════════════════════════════════
// RUNTIME — LOAD
// ════════════════════════════════════════════════════════════════════════

func TestLoadInvalidBytes(t *testing.T) {
	runtime := New(nil)
	_, err := runtime.Load([]byte{0xFF, 0xFF})
	if err == nil {
		t.Fatal("expected error for invalid wasm bytes")
	}
}
