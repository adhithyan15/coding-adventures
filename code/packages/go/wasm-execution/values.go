// values.go --- Typed WASM values and constructor/assertion helpers.
//
// ════════════════════════════════════════════════════════════════════════
// WHAT ARE WASM VALUES?
// ════════════════════════════════════════════════════════════════════════
//
// Every value in WebAssembly is *typed* — it carries both a raw payload
// and a type tag that identifies which of the four numeric types it belongs
// to.  This is unlike dynamically-typed languages where 42 and 42.0 could
// be the same thing.  In WASM:
//
//   - i32(42) is a 32-bit integer with value 42.
//   - f64(42.0) is a 64-bit float with value 42.0.
//   - They are DIFFERENT values.
//
// ════════════════════════════════════════════════════════════════════════
// GO TYPE MAPPING
// ════════════════════════════════════════════════════════════════════════
//
//	WASM Type   Go Type    Why?
//	─────────   ───────    ────
//	i32         int32      Native 32-bit wrapping arithmetic
//	i64         int64      Native 64-bit wrapping arithmetic
//	f32         float32    IEEE 754 single precision
//	f64         float64    IEEE 754 double precision
//
// Go's int32 arithmetic naturally wraps on overflow, just like WASM
// requires.  No bit tricks needed (unlike JavaScript's `| 0`).
package wasmexecution

import (
	"fmt"

	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
)

// WasmValue is a typed WASM value.  We reuse the GenericVM's TypedVMValue
// so that values can be pushed/popped directly on the typed stack.
type WasmValue = vm.TypedVMValue

// ════════════════════════════════════════════════════════════════════════
// CONSTRUCTOR FUNCTIONS
// ════════════════════════════════════════════════════════════════════════

// I32 creates a WasmValue of type i32.
//
// Go's int32 conversion naturally truncates to 32 bits, giving us the
// wrapping behavior that WASM requires.
//
//	I32(42)           => i32(42)
//	I32(0xFFFFFFFF)   => i32(-1)   (two's complement wrapping)
//	I32(0x100000000)  => i32(0)    (truncated to 32 bits)
func I32(v int32) WasmValue {
	return WasmValue{Type: int(wasmtypes.ValueTypeI32), Value: v}
}

// I64 creates a WasmValue of type i64.
func I64(v int64) WasmValue {
	return WasmValue{Type: int(wasmtypes.ValueTypeI64), Value: v}
}

// F32 creates a WasmValue of type f32.
func F32(v float32) WasmValue {
	return WasmValue{Type: int(wasmtypes.ValueTypeF32), Value: v}
}

// F64 creates a WasmValue of type f64.
func F64(v float64) WasmValue {
	return WasmValue{Type: int(wasmtypes.ValueTypeF64), Value: v}
}

// ════════════════════════════════════════════════════════════════════════
// DEFAULT VALUES
// ════════════════════════════════════════════════════════════════════════

// DefaultValue returns the zero-initialized WasmValue for a given type.
//
// WASM spec section 4.2.1: "The default value of a value type is the
// respective zero."
//
//	i32 → 0
//	i64 → 0
//	f32 → 0.0
//	f64 → 0.0
func DefaultValue(vt wasmtypes.ValueType) WasmValue {
	switch vt {
	case wasmtypes.ValueTypeI32:
		return I32(0)
	case wasmtypes.ValueTypeI64:
		return I64(0)
	case wasmtypes.ValueTypeF32:
		return F32(0)
	case wasmtypes.ValueTypeF64:
		return F64(0)
	default:
		panic(fmt.Sprintf("unknown value type: 0x%02x", vt))
	}
}

// ════════════════════════════════════════════════════════════════════════
// TYPE-SAFE EXTRACTION
// ════════════════════════════════════════════════════════════════════════

// typeNames maps type codes to human-readable names for error messages.
var typeNames = map[int]string{
	int(wasmtypes.ValueTypeI32): "i32",
	int(wasmtypes.ValueTypeI64): "i64",
	int(wasmtypes.ValueTypeF32): "f32",
	int(wasmtypes.ValueTypeF64): "f64",
}

// AsI32 extracts the int32 payload from a WasmValue.
// Panics with a TrapError if the type tag is not i32.
func AsI32(v WasmValue) int32 {
	if v.Type != int(wasmtypes.ValueTypeI32) {
		panic(NewTrapError(fmt.Sprintf("type mismatch: expected i32, got %s", typeNames[v.Type])))
	}
	return v.Value.(int32)
}

// AsI64 extracts the int64 payload from a WasmValue.
func AsI64(v WasmValue) int64 {
	if v.Type != int(wasmtypes.ValueTypeI64) {
		panic(NewTrapError(fmt.Sprintf("type mismatch: expected i64, got %s", typeNames[v.Type])))
	}
	return v.Value.(int64)
}

// AsF32 extracts the float32 payload from a WasmValue.
func AsF32(v WasmValue) float32 {
	if v.Type != int(wasmtypes.ValueTypeF32) {
		panic(NewTrapError(fmt.Sprintf("type mismatch: expected f32, got %s", typeNames[v.Type])))
	}
	return v.Value.(float32)
}

// AsF64 extracts the float64 payload from a WasmValue.
func AsF64(v WasmValue) float64 {
	if v.Type != int(wasmtypes.ValueTypeF64) {
		panic(NewTrapError(fmt.Sprintf("type mismatch: expected f64, got %s", typeNames[v.Type])))
	}
	return v.Value.(float64)
}
