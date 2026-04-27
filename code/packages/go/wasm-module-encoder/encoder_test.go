package wasmmoduleencoder

import (
	"bytes"
	"testing"

	wasmleb128 "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-leb128"
	wasmmoduleparser "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-module-parser"
	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
	wasmvalidator "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-validator"
)

func TestEncodeModuleProducesValidBinary(t *testing.T) {
	module := &wasmtypes.WasmModule{
		Types: []wasmtypes.FuncType{
			{
				Params:  nil,
				Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
			},
		},
		Functions: []uint32{0},
		Exports: []wasmtypes.Export{
			{
				Name:  "answer",
				Kind:  wasmtypes.ExternalKindFunction,
				Index: 0,
			},
		},
		Code: []wasmtypes.FunctionBody{
			{
				Locals: nil,
				Code:   append([]byte{0x41}, append(wasmleb128.EncodeSigned(42), 0x0B)...),
			},
		},
	}

	binary, err := EncodeModule(module)
	if err != nil {
		t.Fatalf("encode failed: %v", err)
	}
	if !bytes.Equal(binary[:4], WASMMagic) {
		t.Fatalf("expected wasm magic %v, got %v", WASMMagic, binary[:4])
	}
	if !bytes.Equal(binary[4:8], WASMVersion) {
		t.Fatalf("expected wasm version %v, got %v", WASMVersion, binary[4:8])
	}

	parsed, err := wasmmoduleparser.New().Parse(binary)
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}
	if len(parsed.Exports) != 1 || parsed.Exports[0].Name != "answer" {
		t.Fatalf("expected exported function 'answer', got %+v", parsed.Exports)
	}
	if _, err := wasmvalidator.Validate(parsed); err != nil {
		t.Fatalf("validate failed: %v", err)
	}
}
