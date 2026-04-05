package wasmruntime

import (
	"testing"

	wasmexecution "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-execution"
	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
)

// TestRuntimeSquare tests the full runtime pipeline with a square function.
func TestRuntimeSquare(t *testing.T) {
	// Build a minimal module with a square function.
	module := &wasmtypes.WasmModule{
		Types: []wasmtypes.FuncType{
			{Params: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}, Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32}},
		},
		Functions: []uint32{0},
		Code: []wasmtypes.FunctionBody{
			{
				Locals: nil,
				Code:   []byte{0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B}, // local.get 0; local.get 0; i32.mul; end
			},
		},
		Exports: []wasmtypes.Export{
			{Name: "square", Kind: wasmtypes.ExternalKindFunction, Index: 0},
		},
	}

	runtime := New(nil)

	// Validate.
	_, err := runtime.Validate(module)
	if err != nil {
		t.Fatalf("validation failed: %v", err)
	}

	// Instantiate.
	instance, err := runtime.Instantiate(module)
	if err != nil {
		t.Fatalf("instantiation failed: %v", err)
	}

	// Call.
	results, err := runtime.Call(instance, "square", []int{5})
	if err != nil {
		t.Fatalf("call failed: %v", err)
	}
	if len(results) != 1 || results[0] != 25 {
		t.Fatalf("expected [25], got %v", results)
	}
}

// TestRuntimeWithMemory tests a module that uses linear memory.
func TestRuntimeWithMemory(t *testing.T) {
	// Module: store 42 at offset 0, then load it back.
	// store_and_load() -> i32:
	//   i32.const 0     (0x41 0x00)
	//   i32.const 42    (0x41 0x2A)
	//   i32.store       (0x36 0x02 0x00)  align=2, offset=0
	//   i32.const 0     (0x41 0x00)
	//   i32.load        (0x28 0x02 0x00)
	//   end             (0x0B)
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
					0x41, 0x2A, // i32.const 42
					0x36, 0x02, 0x00, // i32.store align=2 offset=0
					0x41, 0x00, // i32.const 0
					0x28, 0x02, 0x00, // i32.load align=2 offset=0
					0x0B, // end
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

// TestWasiStub tests the WASI stub host interface.
func TestWasiStub(t *testing.T) {
	var stdout []string
	wasi := NewWasiStub(
		func(text string) { stdout = append(stdout, text) },
		nil,
	)

	// Test that resolving a WASI function works.
	fdWrite := wasi.ResolveFunction("wasi_snapshot_preview1", "fd_write")
	if fdWrite == nil {
		t.Fatal("fd_write should be resolvable")
	}

	// Test that non-WASI modules return nil.
	if wasi.ResolveFunction("env", "something") != nil {
		t.Fatal("non-WASI module should return nil")
	}

	// Test proc_exit.
	procExit := wasi.ResolveFunction("wasi_snapshot_preview1", "proc_exit")
	if procExit == nil {
		t.Fatal("proc_exit should be resolvable")
	}
	// Calling proc_exit should panic with ProcExitError.
	func() {
		defer func() {
			r := recover()
			if r == nil {
				t.Fatal("proc_exit should panic")
			}
			if _, ok := r.(*ProcExitError); !ok {
				t.Fatalf("expected *ProcExitError, got %T", r)
			}
		}()
		procExit.Call([]wasmexecution.WasmValue{wasmexecution.I32(0)})
	}()
}
