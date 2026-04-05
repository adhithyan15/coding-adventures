// const_expr.go --- Evaluate WASM constant expressions.
//
// Constant expressions are tiny programs used to initialize globals,
// data segment offsets, and element segment offsets.  They consist of
// only a few allowed opcodes: i32.const, i64.const, f32.const, f64.const,
// global.get, and end.
package wasmexecution

import (
	"encoding/binary"
	"fmt"
	"math"

	wasmleb128 "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-leb128"
)

// EvaluateConstExpr evaluates a WASM constant expression and returns
// the single WasmValue it produces.
//
// The expr is a byte slice of opcodes + immediates ending with 0x0B (end).
// The globals slice provides values for global.get instructions.
func EvaluateConstExpr(expr []byte, globals []WasmValue) (WasmValue, error) {
	var result *WasmValue
	pos := 0

	for pos < len(expr) {
		opcode := expr[pos]
		pos++

		switch opcode {
		case 0x41: // i32.const
			value, consumed, err := wasmleb128.DecodeSigned(expr, pos)
			if err != nil {
				return WasmValue{}, fmt.Errorf("const_expr: i32.const decode error: %w", err)
			}
			pos += consumed
			v := I32(int32(value))
			result = &v

		case 0x42: // i64.const
			value, consumed, err := decodeSigned64(expr, pos)
			if err != nil {
				return WasmValue{}, fmt.Errorf("const_expr: i64.const decode error: %w", err)
			}
			pos += consumed
			v := I64(value)
			result = &v

		case 0x43: // f32.const
			if pos+4 > len(expr) {
				return WasmValue{}, fmt.Errorf("const_expr: f32.const: not enough bytes")
			}
			bits := binary.LittleEndian.Uint32(expr[pos:])
			pos += 4
			v := F32(math.Float32frombits(bits))
			result = &v

		case 0x44: // f64.const
			if pos+8 > len(expr) {
				return WasmValue{}, fmt.Errorf("const_expr: f64.const: not enough bytes")
			}
			bits := binary.LittleEndian.Uint64(expr[pos:])
			pos += 8
			v := F64(math.Float64frombits(bits))
			result = &v

		case 0x23: // global.get
			idx, consumed, err := wasmleb128.DecodeUnsigned(expr, pos)
			if err != nil {
				return WasmValue{}, fmt.Errorf("const_expr: global.get decode error: %w", err)
			}
			pos += consumed
			if int(idx) >= len(globals) {
				return WasmValue{}, fmt.Errorf("const_expr: global.get index %d out of bounds (%d globals)", idx, len(globals))
			}
			v := globals[idx]
			result = &v

		case 0x0B: // end
			if result == nil {
				return WasmValue{}, fmt.Errorf("const_expr: expression produced no value")
			}
			return *result, nil

		default:
			return WasmValue{}, fmt.Errorf("const_expr: illegal opcode 0x%02x", opcode)
		}
	}

	return WasmValue{}, fmt.Errorf("const_expr: missing end opcode (0x0B)")
}

// decodeSigned64 decodes a signed 64-bit LEB128 value (up to 10 bytes).
func decodeSigned64(data []byte, offset int) (int64, int, error) {
	var result int64
	var shift uint
	consumed := 0
	const maxBytes = 10

	for consumed < maxBytes {
		if offset+consumed >= len(data) {
			return 0, 0, fmt.Errorf("unterminated LEB128 at offset %d", offset)
		}
		b := data[offset+consumed]
		consumed++

		result |= int64(b&0x7F) << shift
		shift += 7

		if b&0x80 == 0 {
			// Sign extension.
			if shift < 64 && b&0x40 != 0 {
				result |= -(int64(1) << shift)
			}
			return result, consumed, nil
		}
	}
	return 0, 0, fmt.Errorf("LEB128 sequence too long for i64 at offset %d", offset)
}
