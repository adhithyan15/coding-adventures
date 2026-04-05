// instructions.go --- All WASM instruction handlers for the GenericVM.
//
// ════════════════════════════════════════════════════════════════════════
// ARCHITECTURE
// ════════════════════════════════════════════════════════════════════════
//
// Each WASM instruction is registered as a ContextOpcodeHandler on the
// GenericVM.  Handlers receive the VM, the decoded instruction, the code
// object, and the WasmExecutionContext.
//
// The handler pattern:
//   1. Pop operands from the typed stack.
//   2. Perform the operation (with trap checks).
//   3. Push the result.
//   4. Advance the PC.
//
// ════════════════════════════════════════════════════════════════════════
// i32 WRAPPING IN GO
// ════════════════════════════════════════════════════════════════════════
//
// Go's int32 arithmetic naturally wraps on overflow, unlike JavaScript
// where you need `| 0` tricks.  For example:
//
//	int32(math.MaxInt32) + 1 == math.MinInt32  (wraps automatically)
//
// For unsigned operations, we cast to uint32, operate, then cast back.
package wasmexecution

import (
	"math"
	"math/bits"

	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
)

// RegisterAllInstructions registers all non-control WASM instruction handlers.
func RegisterAllInstructions(genVM *vm.GenericVM) {
	registerNumericI32(genVM)
	registerNumericI64(genVM)
	registerNumericF32(genVM)
	registerNumericF64(genVM)
	registerConversion(genVM)
	registerVariable(genVM)
	registerParametric(genVM)
	registerMemory(genVM)
}

// ════════════════════════════════════════════════════════════════════════
// NUMERIC I32 (33 instructions)
// ════════════════════════════════════════════════════════════════════════

func registerNumericI32(v *vm.GenericVM) {
	// 0x41: i32.const
	v.RegisterContextOpcode(0x41, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		val := toInt32(instr.Operand)
		gvm.PushTyped(I32(val))
		gvm.AdvancePC()
		return nil
	})

	// 0x45: i32.eqz
	v.RegisterContextOpcode(0x45, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsI32(gvm.PopTyped())
		if a == 0 {
			gvm.PushTyped(I32(1))
		} else {
			gvm.PushTyped(I32(0))
		}
		gvm.AdvancePC()
		return nil
	})

	// 0x46: i32.eq
	v.RegisterContextOpcode(0x46, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(boolToI32(a == b)))
		gvm.AdvancePC()
		return nil
	})

	// 0x47: i32.ne
	v.RegisterContextOpcode(0x47, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(boolToI32(a != b)))
		gvm.AdvancePC()
		return nil
	})

	// 0x48: i32.lt_s
	v.RegisterContextOpcode(0x48, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(boolToI32(a < b)))
		gvm.AdvancePC()
		return nil
	})

	// 0x49: i32.lt_u
	v.RegisterContextOpcode(0x49, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(boolToI32(uint32(a) < uint32(b))))
		gvm.AdvancePC()
		return nil
	})

	// 0x4A: i32.gt_s
	v.RegisterContextOpcode(0x4A, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(boolToI32(a > b)))
		gvm.AdvancePC()
		return nil
	})

	// 0x4B: i32.gt_u
	v.RegisterContextOpcode(0x4B, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(boolToI32(uint32(a) > uint32(b))))
		gvm.AdvancePC()
		return nil
	})

	// 0x4C: i32.le_s
	v.RegisterContextOpcode(0x4C, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(boolToI32(a <= b)))
		gvm.AdvancePC()
		return nil
	})

	// 0x4D: i32.le_u
	v.RegisterContextOpcode(0x4D, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(boolToI32(uint32(a) <= uint32(b))))
		gvm.AdvancePC()
		return nil
	})

	// 0x4E: i32.ge_s
	v.RegisterContextOpcode(0x4E, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(boolToI32(a >= b)))
		gvm.AdvancePC()
		return nil
	})

	// 0x4F: i32.ge_u
	v.RegisterContextOpcode(0x4F, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(boolToI32(uint32(a) >= uint32(b))))
		gvm.AdvancePC()
		return nil
	})

	// 0x67: i32.clz — count leading zeros
	v.RegisterContextOpcode(0x67, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(int32(bits.LeadingZeros32(uint32(a)))))
		gvm.AdvancePC()
		return nil
	})

	// 0x68: i32.ctz — count trailing zeros
	v.RegisterContextOpcode(0x68, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(int32(bits.TrailingZeros32(uint32(a)))))
		gvm.AdvancePC()
		return nil
	})

	// 0x69: i32.popcnt
	v.RegisterContextOpcode(0x69, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(int32(bits.OnesCount32(uint32(a)))))
		gvm.AdvancePC()
		return nil
	})

	// 0x6A: i32.add
	v.RegisterContextOpcode(0x6A, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(a + b))
		gvm.AdvancePC()
		return nil
	})

	// 0x6B: i32.sub
	v.RegisterContextOpcode(0x6B, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(a - b))
		gvm.AdvancePC()
		return nil
	})

	// 0x6C: i32.mul
	v.RegisterContextOpcode(0x6C, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(a * b))
		gvm.AdvancePC()
		return nil
	})

	// 0x6D: i32.div_s — signed division (trapping)
	v.RegisterContextOpcode(0x6D, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		if b == 0 {
			panic(NewTrapError("integer divide by zero"))
		}
		if a == math.MinInt32 && b == -1 {
			panic(NewTrapError("integer overflow"))
		}
		gvm.PushTyped(I32(a / b))
		gvm.AdvancePC()
		return nil
	})

	// 0x6E: i32.div_u
	v.RegisterContextOpcode(0x6E, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		if uint32(b) == 0 {
			panic(NewTrapError("integer divide by zero"))
		}
		gvm.PushTyped(I32(int32(uint32(a) / uint32(b))))
		gvm.AdvancePC()
		return nil
	})

	// 0x6F: i32.rem_s
	v.RegisterContextOpcode(0x6F, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		if b == 0 {
			panic(NewTrapError("integer divide by zero"))
		}
		if a == math.MinInt32 && b == -1 {
			gvm.PushTyped(I32(0))
		} else {
			gvm.PushTyped(I32(a % b))
		}
		gvm.AdvancePC()
		return nil
	})

	// 0x70: i32.rem_u
	v.RegisterContextOpcode(0x70, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		if uint32(b) == 0 {
			panic(NewTrapError("integer divide by zero"))
		}
		gvm.PushTyped(I32(int32(uint32(a) % uint32(b))))
		gvm.AdvancePC()
		return nil
	})

	// 0x71: i32.and
	v.RegisterContextOpcode(0x71, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(a & b))
		gvm.AdvancePC()
		return nil
	})

	// 0x72: i32.or
	v.RegisterContextOpcode(0x72, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(a | b))
		gvm.AdvancePC()
		return nil
	})

	// 0x73: i32.xor
	v.RegisterContextOpcode(0x73, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(a ^ b))
		gvm.AdvancePC()
		return nil
	})

	// 0x74: i32.shl — shift amounts are taken modulo 32
	v.RegisterContextOpcode(0x74, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(a << (uint32(b) & 31)))
		gvm.AdvancePC()
		return nil
	})

	// 0x75: i32.shr_s — arithmetic shift right (sign-preserving)
	v.RegisterContextOpcode(0x75, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(a >> (uint32(b) & 31)))
		gvm.AdvancePC()
		return nil
	})

	// 0x76: i32.shr_u — logical shift right (zero-filling)
	v.RegisterContextOpcode(0x76, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(int32(uint32(a) >> (uint32(b) & 31))))
		gvm.AdvancePC()
		return nil
	})

	// 0x77: i32.rotl
	v.RegisterContextOpcode(0x77, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(int32(bits.RotateLeft32(uint32(a), int(b)))))
		gvm.AdvancePC()
		return nil
	})

	// 0x78: i32.rotr
	v.RegisterContextOpcode(0x78, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI32(gvm.PopTyped())
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I32(int32(bits.RotateLeft32(uint32(a), -int(b)))))
		gvm.AdvancePC()
		return nil
	})
}

// ════════════════════════════════════════════════════════════════════════
// NUMERIC I64 (stubs — register const + basic arithmetic)
// ════════════════════════════════════════════════════════════════════════

func registerNumericI64(v *vm.GenericVM) {
	// 0x42: i64.const
	v.RegisterContextOpcode(0x42, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		val := toInt64(instr.Operand)
		gvm.PushTyped(I64(val))
		gvm.AdvancePC()
		return nil
	})

	// 0x50: i64.eqz
	v.RegisterContextOpcode(0x50, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsI64(gvm.PopTyped())
		gvm.PushTyped(I32(boolToI32(a == 0)))
		gvm.AdvancePC()
		return nil
	})

	// 0x51-0x5A: i64 comparisons
	registerI64Cmp(v, 0x51, func(a, b int64) bool { return a == b })   // eq
	registerI64Cmp(v, 0x52, func(a, b int64) bool { return a != b })   // ne
	registerI64Cmp(v, 0x53, func(a, b int64) bool { return a < b })    // lt_s
	registerI64Cmp(v, 0x54, func(a, b int64) bool { return uint64(a) < uint64(b) })  // lt_u
	registerI64Cmp(v, 0x55, func(a, b int64) bool { return a > b })    // gt_s
	registerI64Cmp(v, 0x56, func(a, b int64) bool { return uint64(a) > uint64(b) })  // gt_u
	registerI64Cmp(v, 0x57, func(a, b int64) bool { return a <= b })   // le_s
	registerI64Cmp(v, 0x58, func(a, b int64) bool { return uint64(a) <= uint64(b) }) // le_u
	registerI64Cmp(v, 0x59, func(a, b int64) bool { return a >= b })   // ge_s
	registerI64Cmp(v, 0x5A, func(a, b int64) bool { return uint64(a) >= uint64(b) }) // ge_u

	// 0x79-0x8A: i64 arithmetic
	registerI64Unary(v, 0x79, func(a int64) int64 { return int64(bits.LeadingZeros64(uint64(a))) })
	registerI64Unary(v, 0x7A, func(a int64) int64 { return int64(bits.TrailingZeros64(uint64(a))) })
	registerI64Unary(v, 0x7B, func(a int64) int64 { return int64(bits.OnesCount64(uint64(a))) })
	registerI64Binary(v, 0x7C, func(a, b int64) int64 { return a + b })
	registerI64Binary(v, 0x7D, func(a, b int64) int64 { return a - b })
	registerI64Binary(v, 0x7E, func(a, b int64) int64 { return a * b })

	// 0x7F: i64.div_s
	v.RegisterContextOpcode(0x7F, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI64(gvm.PopTyped())
		a := AsI64(gvm.PopTyped())
		if b == 0 { panic(NewTrapError("integer divide by zero")) }
		if a == math.MinInt64 && b == -1 { panic(NewTrapError("integer overflow")) }
		gvm.PushTyped(I64(a / b))
		gvm.AdvancePC()
		return nil
	})

	// 0x80: i64.div_u
	v.RegisterContextOpcode(0x80, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI64(gvm.PopTyped())
		a := AsI64(gvm.PopTyped())
		if uint64(b) == 0 { panic(NewTrapError("integer divide by zero")) }
		gvm.PushTyped(I64(int64(uint64(a) / uint64(b))))
		gvm.AdvancePC()
		return nil
	})

	// 0x81: i64.rem_s
	v.RegisterContextOpcode(0x81, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI64(gvm.PopTyped())
		a := AsI64(gvm.PopTyped())
		if b == 0 { panic(NewTrapError("integer divide by zero")) }
		if a == math.MinInt64 && b == -1 { gvm.PushTyped(I64(0)) } else { gvm.PushTyped(I64(a % b)) }
		gvm.AdvancePC()
		return nil
	})

	// 0x82: i64.rem_u
	v.RegisterContextOpcode(0x82, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI64(gvm.PopTyped())
		a := AsI64(gvm.PopTyped())
		if uint64(b) == 0 { panic(NewTrapError("integer divide by zero")) }
		gvm.PushTyped(I64(int64(uint64(a) % uint64(b))))
		gvm.AdvancePC()
		return nil
	})

	registerI64Binary(v, 0x83, func(a, b int64) int64 { return a & b })  // and
	registerI64Binary(v, 0x84, func(a, b int64) int64 { return a | b })  // or
	registerI64Binary(v, 0x85, func(a, b int64) int64 { return a ^ b })  // xor
	registerI64Binary(v, 0x86, func(a, b int64) int64 { return a << (uint64(b) & 63) })      // shl
	registerI64Binary(v, 0x87, func(a, b int64) int64 { return a >> (uint64(b) & 63) })      // shr_s
	registerI64Binary(v, 0x88, func(a, b int64) int64 { return int64(uint64(a) >> (uint64(b) & 63)) }) // shr_u
	registerI64Binary(v, 0x89, func(a, b int64) int64 { return int64(bits.RotateLeft64(uint64(a), int(b))) })  // rotl
	registerI64Binary(v, 0x8A, func(a, b int64) int64 { return int64(bits.RotateLeft64(uint64(a), -int(b))) }) // rotr
}

// ════════════════════════════════════════════════════════════════════════
// NUMERIC F32 (23 instructions)
// ════════════════════════════════════════════════════════════════════════

func registerNumericF32(v *vm.GenericVM) {
	// 0x43: f32.const
	v.RegisterContextOpcode(0x43, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		val := toFloat32(instr.Operand)
		gvm.PushTyped(F32(val))
		gvm.AdvancePC()
		return nil
	})

	// F32 comparisons (0x5B-0x60)
	registerF32Cmp(v, 0x5B, func(a, b float32) bool { return a == b })
	registerF32Cmp(v, 0x5C, func(a, b float32) bool { return a != b })
	registerF32Cmp(v, 0x5D, func(a, b float32) bool { return a < b })
	registerF32Cmp(v, 0x5E, func(a, b float32) bool { return a > b })
	registerF32Cmp(v, 0x5F, func(a, b float32) bool { return a <= b })
	registerF32Cmp(v, 0x60, func(a, b float32) bool { return a >= b })

	// F32 arithmetic (0x8B-0x98)
	registerF32Unary(v, 0x8B, func(a float32) float32 { return float32(math.Abs(float64(a))) })
	registerF32Unary(v, 0x8C, func(a float32) float32 { return -a })
	registerF32Unary(v, 0x8D, func(a float32) float32 { return float32(math.Ceil(float64(a))) })
	registerF32Unary(v, 0x8E, func(a float32) float32 { return float32(math.Floor(float64(a))) })
	registerF32Unary(v, 0x8F, func(a float32) float32 { return float32(math.Trunc(float64(a))) })
	registerF32Unary(v, 0x90, func(a float32) float32 { return float32(math.RoundToEven(float64(a))) })
	registerF32Unary(v, 0x91, func(a float32) float32 { return float32(math.Sqrt(float64(a))) })
	registerF32Binary(v, 0x92, func(a, b float32) float32 { return a + b })
	registerF32Binary(v, 0x93, func(a, b float32) float32 { return a - b })
	registerF32Binary(v, 0x94, func(a, b float32) float32 { return a * b })
	registerF32Binary(v, 0x95, func(a, b float32) float32 { return a / b })
	registerF32Binary(v, 0x96, func(a, b float32) float32 { return float32(math.Min(float64(a), float64(b))) })
	registerF32Binary(v, 0x97, func(a, b float32) float32 { return float32(math.Max(float64(a), float64(b))) })
	registerF32Binary(v, 0x98, func(a, b float32) float32 { return float32(math.Copysign(float64(a), float64(b))) })
}

// ════════════════════════════════════════════════════════════════════════
// NUMERIC F64 (23 instructions)
// ════════════════════════════════════════════════════════════════════════

func registerNumericF64(v *vm.GenericVM) {
	// 0x44: f64.const
	v.RegisterContextOpcode(0x44, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		val := toFloat64(instr.Operand)
		gvm.PushTyped(F64(val))
		gvm.AdvancePC()
		return nil
	})

	registerF64Cmp(v, 0x61, func(a, b float64) bool { return a == b })
	registerF64Cmp(v, 0x62, func(a, b float64) bool { return a != b })
	registerF64Cmp(v, 0x63, func(a, b float64) bool { return a < b })
	registerF64Cmp(v, 0x64, func(a, b float64) bool { return a > b })
	registerF64Cmp(v, 0x65, func(a, b float64) bool { return a <= b })
	registerF64Cmp(v, 0x66, func(a, b float64) bool { return a >= b })

	registerF64Unary(v, 0x99, func(a float64) float64 { return math.Abs(a) })
	registerF64Unary(v, 0x9A, func(a float64) float64 { return -a })
	registerF64Unary(v, 0x9B, func(a float64) float64 { return math.Ceil(a) })
	registerF64Unary(v, 0x9C, func(a float64) float64 { return math.Floor(a) })
	registerF64Unary(v, 0x9D, func(a float64) float64 { return math.Trunc(a) })
	registerF64Unary(v, 0x9E, func(a float64) float64 { return math.RoundToEven(a) })
	registerF64Unary(v, 0x9F, func(a float64) float64 { return math.Sqrt(a) })
	registerF64Binary(v, 0xA0, func(a, b float64) float64 { return a + b })
	registerF64Binary(v, 0xA1, func(a, b float64) float64 { return a - b })
	registerF64Binary(v, 0xA2, func(a, b float64) float64 { return a * b })
	registerF64Binary(v, 0xA3, func(a, b float64) float64 { return a / b })
	registerF64Binary(v, 0xA4, func(a, b float64) float64 { return math.Min(a, b) })
	registerF64Binary(v, 0xA5, func(a, b float64) float64 { return math.Max(a, b) })
	registerF64Binary(v, 0xA6, func(a, b float64) float64 { return math.Copysign(a, b) })
}

// ════════════════════════════════════════════════════════════════════════
// CONVERSION (27 instructions: 0xA7-0xBF)
// ════════════════════════════════════════════════════════════════════════

func registerConversion(v *vm.GenericVM) {
	// 0xA7: i32.wrap_i64
	v.RegisterContextOpcode(0xA7, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsI64(gvm.PopTyped())
		gvm.PushTyped(I32(int32(a)))
		gvm.AdvancePC()
		return nil
	})

	// 0xA8: i32.trunc_f32_s
	v.RegisterContextOpcode(0xA8, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsF32(gvm.PopTyped())
		if math.IsNaN(float64(a)) || math.IsInf(float64(a), 0) || float64(a) >= math.MaxInt32+1 || float64(a) < math.MinInt32 {
			panic(NewTrapError("integer overflow"))
		}
		gvm.PushTyped(I32(int32(a)))
		gvm.AdvancePC()
		return nil
	})

	// 0xA9: i32.trunc_f32_u
	v.RegisterContextOpcode(0xA9, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsF32(gvm.PopTyped())
		if math.IsNaN(float64(a)) || math.IsInf(float64(a), 0) || float64(a) >= math.MaxUint32+1 || float64(a) < 0 {
			panic(NewTrapError("integer overflow"))
		}
		gvm.PushTyped(I32(int32(uint32(a))))
		gvm.AdvancePC()
		return nil
	})

	// 0xAA: i32.trunc_f64_s
	v.RegisterContextOpcode(0xAA, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsF64(gvm.PopTyped())
		if math.IsNaN(a) || math.IsInf(a, 0) || a >= math.MaxInt32+1 || a < math.MinInt32 {
			panic(NewTrapError("integer overflow"))
		}
		gvm.PushTyped(I32(int32(a)))
		gvm.AdvancePC()
		return nil
	})

	// 0xAB: i32.trunc_f64_u
	v.RegisterContextOpcode(0xAB, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsF64(gvm.PopTyped())
		if math.IsNaN(a) || math.IsInf(a, 0) || a >= math.MaxUint32+1 || a < 0 {
			panic(NewTrapError("integer overflow"))
		}
		gvm.PushTyped(I32(int32(uint32(a))))
		gvm.AdvancePC()
		return nil
	})

	// 0xAC: i64.extend_i32_s
	v.RegisterContextOpcode(0xAC, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I64(int64(a)))
		gvm.AdvancePC()
		return nil
	})

	// 0xAD: i64.extend_i32_u
	v.RegisterContextOpcode(0xAD, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsI32(gvm.PopTyped())
		gvm.PushTyped(I64(int64(uint32(a))))
		gvm.AdvancePC()
		return nil
	})

	// 0xAE-0xB1: i64.trunc_f32/f64 (s/u) — simplified implementations
	for _, entry := range []struct{ opcode byte; isF32 bool; signed bool }{
		{0xAE, true, true}, {0xAF, true, false}, {0xB0, false, true}, {0xB1, false, false},
	} {
		e := entry
		v.RegisterContextOpcode(vm.OpCode(e.opcode), func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
			var fval float64
			if e.isF32 { fval = float64(AsF32(gvm.PopTyped())) } else { fval = AsF64(gvm.PopTyped()) }
			if math.IsNaN(fval) || math.IsInf(fval, 0) { panic(NewTrapError("integer overflow")) }
			if e.signed {
				gvm.PushTyped(I64(int64(fval)))
			} else {
				if fval < 0 { panic(NewTrapError("integer overflow")) }
				gvm.PushTyped(I64(int64(uint64(fval))))
			}
			gvm.AdvancePC()
			return nil
		})
	}

	// 0xB2: f32.convert_i32_s
	v.RegisterContextOpcode(0xB2, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsI32(gvm.PopTyped()); gvm.PushTyped(F32(float32(a))); gvm.AdvancePC(); return nil
	})
	// 0xB3: f32.convert_i32_u
	v.RegisterContextOpcode(0xB3, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsI32(gvm.PopTyped()); gvm.PushTyped(F32(float32(uint32(a)))); gvm.AdvancePC(); return nil
	})
	// 0xB4: f32.convert_i64_s
	v.RegisterContextOpcode(0xB4, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsI64(gvm.PopTyped()); gvm.PushTyped(F32(float32(a))); gvm.AdvancePC(); return nil
	})
	// 0xB5: f32.convert_i64_u
	v.RegisterContextOpcode(0xB5, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsI64(gvm.PopTyped()); gvm.PushTyped(F32(float32(uint64(a)))); gvm.AdvancePC(); return nil
	})
	// 0xB6: f32.demote_f64
	v.RegisterContextOpcode(0xB6, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsF64(gvm.PopTyped()); gvm.PushTyped(F32(float32(a))); gvm.AdvancePC(); return nil
	})
	// 0xB7: f64.convert_i32_s
	v.RegisterContextOpcode(0xB7, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsI32(gvm.PopTyped()); gvm.PushTyped(F64(float64(a))); gvm.AdvancePC(); return nil
	})
	// 0xB8: f64.convert_i32_u
	v.RegisterContextOpcode(0xB8, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsI32(gvm.PopTyped()); gvm.PushTyped(F64(float64(uint32(a)))); gvm.AdvancePC(); return nil
	})
	// 0xB9: f64.convert_i64_s
	v.RegisterContextOpcode(0xB9, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsI64(gvm.PopTyped()); gvm.PushTyped(F64(float64(a))); gvm.AdvancePC(); return nil
	})
	// 0xBA: f64.convert_i64_u
	v.RegisterContextOpcode(0xBA, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsI64(gvm.PopTyped()); gvm.PushTyped(F64(float64(uint64(a)))); gvm.AdvancePC(); return nil
	})
	// 0xBB: f64.promote_f32
	v.RegisterContextOpcode(0xBB, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsF32(gvm.PopTyped()); gvm.PushTyped(F64(float64(a))); gvm.AdvancePC(); return nil
	})
	// 0xBC: i32.reinterpret_f32
	v.RegisterContextOpcode(0xBC, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsF32(gvm.PopTyped()); gvm.PushTyped(I32(int32(math.Float32bits(a)))); gvm.AdvancePC(); return nil
	})
	// 0xBD: i64.reinterpret_f64
	v.RegisterContextOpcode(0xBD, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsF64(gvm.PopTyped()); gvm.PushTyped(I64(int64(math.Float64bits(a)))); gvm.AdvancePC(); return nil
	})
	// 0xBE: f32.reinterpret_i32
	v.RegisterContextOpcode(0xBE, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsI32(gvm.PopTyped()); gvm.PushTyped(F32(math.Float32frombits(uint32(a)))); gvm.AdvancePC(); return nil
	})
	// 0xBF: f64.reinterpret_i64
	v.RegisterContextOpcode(0xBF, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsI64(gvm.PopTyped()); gvm.PushTyped(F64(math.Float64frombits(uint64(a)))); gvm.AdvancePC(); return nil
	})
}

// ════════════════════════════════════════════════════════════════════════
// VARIABLE (5 instructions: 0x20-0x24)
// ════════════════════════════════════════════════════════════════════════

func registerVariable(v *vm.GenericVM) {
	// 0x20: local.get
	v.RegisterContextOpcode(0x20, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		wctx := ctx.(*WasmExecutionContext)
		idx := toInt(instr.Operand)
		gvm.PushTyped(wctx.TypedLocals[idx])
		gvm.AdvancePC()
		return nil
	})

	// 0x21: local.set
	v.RegisterContextOpcode(0x21, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		wctx := ctx.(*WasmExecutionContext)
		idx := toInt(instr.Operand)
		wctx.TypedLocals[idx] = gvm.PopTyped()
		gvm.AdvancePC()
		return nil
	})

	// 0x22: local.tee
	v.RegisterContextOpcode(0x22, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		wctx := ctx.(*WasmExecutionContext)
		idx := toInt(instr.Operand)
		wctx.TypedLocals[idx] = gvm.PeekTyped()
		gvm.AdvancePC()
		return nil
	})

	// 0x23: global.get
	v.RegisterContextOpcode(0x23, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		wctx := ctx.(*WasmExecutionContext)
		idx := toInt(instr.Operand)
		gvm.PushTyped(wctx.Globals[idx])
		gvm.AdvancePC()
		return nil
	})

	// 0x24: global.set
	v.RegisterContextOpcode(0x24, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		wctx := ctx.(*WasmExecutionContext)
		idx := toInt(instr.Operand)
		wctx.Globals[idx] = gvm.PopTyped()
		gvm.AdvancePC()
		return nil
	})
}

// ════════════════════════════════════════════════════════════════════════
// PARAMETRIC (2 instructions: 0x1A-0x1B)
// ════════════════════════════════════════════════════════════════════════

func registerParametric(v *vm.GenericVM) {
	// 0x1A: drop
	v.RegisterContextOpcode(0x1A, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		gvm.PopTyped()
		gvm.AdvancePC()
		return nil
	})

	// 0x1B: select
	v.RegisterContextOpcode(0x1B, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		condition := gvm.PopTyped()
		val2 := gvm.PopTyped()
		val1 := gvm.PopTyped()
		if condition.Value.(int32) != 0 {
			gvm.PushTyped(val1)
		} else {
			gvm.PushTyped(val2)
		}
		gvm.AdvancePC()
		return nil
	})
}

// ════════════════════════════════════════════════════════════════════════
// MEMORY (27 instructions: 0x28-0x40)
// ════════════════════════════════════════════════════════════════════════

func registerMemory(v *vm.GenericVM) {
	// Helper to get memory from context.
	getMem := func(ctx interface{}) *LinearMemory {
		wctx := ctx.(*WasmExecutionContext)
		if wctx.Memory == nil {
			panic(NewTrapError("no memory"))
		}
		return wctx.Memory
	}

	// Helper to compute effective address.
	effectiveAddr := func(base int32, operand interface{}) int {
		m, ok := operand.(map[string]interface{})
		if !ok {
			return int(uint32(base))
		}
		offset := 0
		if v, ok := m["offset"]; ok {
			offset = toInt(v)
		}
		return int(uint32(base)) + offset
	}

	// Loads
	v.RegisterContextOpcode(0x28, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		mem := getMem(ctx); base := AsI32(gvm.PopTyped()); addr := effectiveAddr(base, instr.Operand)
		gvm.PushTyped(I32(mem.LoadI32(addr))); gvm.AdvancePC(); return nil
	})
	v.RegisterContextOpcode(0x29, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		mem := getMem(ctx); base := AsI32(gvm.PopTyped()); addr := effectiveAddr(base, instr.Operand)
		gvm.PushTyped(I64(mem.LoadI64(addr))); gvm.AdvancePC(); return nil
	})
	v.RegisterContextOpcode(0x2A, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		mem := getMem(ctx); base := AsI32(gvm.PopTyped()); addr := effectiveAddr(base, instr.Operand)
		gvm.PushTyped(F32(mem.LoadF32(addr))); gvm.AdvancePC(); return nil
	})
	v.RegisterContextOpcode(0x2B, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		mem := getMem(ctx); base := AsI32(gvm.PopTyped()); addr := effectiveAddr(base, instr.Operand)
		gvm.PushTyped(F64(mem.LoadF64(addr))); gvm.AdvancePC(); return nil
	})
	// Narrow i32 loads
	v.RegisterContextOpcode(0x2C, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		mem := getMem(ctx); base := AsI32(gvm.PopTyped()); addr := effectiveAddr(base, instr.Operand)
		gvm.PushTyped(I32(mem.LoadI32_8s(addr))); gvm.AdvancePC(); return nil
	})
	v.RegisterContextOpcode(0x2D, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		mem := getMem(ctx); base := AsI32(gvm.PopTyped()); addr := effectiveAddr(base, instr.Operand)
		gvm.PushTyped(I32(mem.LoadI32_8u(addr))); gvm.AdvancePC(); return nil
	})
	v.RegisterContextOpcode(0x2E, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		mem := getMem(ctx); base := AsI32(gvm.PopTyped()); addr := effectiveAddr(base, instr.Operand)
		gvm.PushTyped(I32(mem.LoadI32_16s(addr))); gvm.AdvancePC(); return nil
	})
	v.RegisterContextOpcode(0x2F, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		mem := getMem(ctx); base := AsI32(gvm.PopTyped()); addr := effectiveAddr(base, instr.Operand)
		gvm.PushTyped(I32(mem.LoadI32_16u(addr))); gvm.AdvancePC(); return nil
	})
	// Narrow i64 loads (0x30-0x35)
	v.RegisterContextOpcode(0x30, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string { mem := getMem(ctx); base := AsI32(gvm.PopTyped()); addr := effectiveAddr(base, instr.Operand); gvm.PushTyped(I64(mem.LoadI64_8s(addr))); gvm.AdvancePC(); return nil })
	v.RegisterContextOpcode(0x31, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string { mem := getMem(ctx); base := AsI32(gvm.PopTyped()); addr := effectiveAddr(base, instr.Operand); gvm.PushTyped(I64(mem.LoadI64_8u(addr))); gvm.AdvancePC(); return nil })
	v.RegisterContextOpcode(0x32, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string { mem := getMem(ctx); base := AsI32(gvm.PopTyped()); addr := effectiveAddr(base, instr.Operand); gvm.PushTyped(I64(mem.LoadI64_16s(addr))); gvm.AdvancePC(); return nil })
	v.RegisterContextOpcode(0x33, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string { mem := getMem(ctx); base := AsI32(gvm.PopTyped()); addr := effectiveAddr(base, instr.Operand); gvm.PushTyped(I64(mem.LoadI64_16u(addr))); gvm.AdvancePC(); return nil })
	v.RegisterContextOpcode(0x34, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string { mem := getMem(ctx); base := AsI32(gvm.PopTyped()); addr := effectiveAddr(base, instr.Operand); gvm.PushTyped(I64(mem.LoadI64_32s(addr))); gvm.AdvancePC(); return nil })
	v.RegisterContextOpcode(0x35, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string { mem := getMem(ctx); base := AsI32(gvm.PopTyped()); addr := effectiveAddr(base, instr.Operand); gvm.PushTyped(I64(mem.LoadI64_32u(addr))); gvm.AdvancePC(); return nil })

	// Stores (0x36-0x3E)
	v.RegisterContextOpcode(0x36, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string { mem := getMem(ctx); val := AsI32(gvm.PopTyped()); base := AsI32(gvm.PopTyped()); mem.StoreI32(effectiveAddr(base, instr.Operand), val); gvm.AdvancePC(); return nil })
	v.RegisterContextOpcode(0x37, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string { mem := getMem(ctx); val := AsI64(gvm.PopTyped()); base := AsI32(gvm.PopTyped()); mem.StoreI64(effectiveAddr(base, instr.Operand), val); gvm.AdvancePC(); return nil })
	v.RegisterContextOpcode(0x38, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string { mem := getMem(ctx); val := AsF32(gvm.PopTyped()); base := AsI32(gvm.PopTyped()); mem.StoreF32(effectiveAddr(base, instr.Operand), val); gvm.AdvancePC(); return nil })
	v.RegisterContextOpcode(0x39, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string { mem := getMem(ctx); val := AsF64(gvm.PopTyped()); base := AsI32(gvm.PopTyped()); mem.StoreF64(effectiveAddr(base, instr.Operand), val); gvm.AdvancePC(); return nil })
	v.RegisterContextOpcode(0x3A, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string { mem := getMem(ctx); val := AsI32(gvm.PopTyped()); base := AsI32(gvm.PopTyped()); mem.StoreI32_8(effectiveAddr(base, instr.Operand), val); gvm.AdvancePC(); return nil })
	v.RegisterContextOpcode(0x3B, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string { mem := getMem(ctx); val := AsI32(gvm.PopTyped()); base := AsI32(gvm.PopTyped()); mem.StoreI32_16(effectiveAddr(base, instr.Operand), val); gvm.AdvancePC(); return nil })
	v.RegisterContextOpcode(0x3C, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string { mem := getMem(ctx); val := AsI64(gvm.PopTyped()); base := AsI32(gvm.PopTyped()); mem.StoreI64_8(effectiveAddr(base, instr.Operand), val); gvm.AdvancePC(); return nil })
	v.RegisterContextOpcode(0x3D, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string { mem := getMem(ctx); val := AsI64(gvm.PopTyped()); base := AsI32(gvm.PopTyped()); mem.StoreI64_16(effectiveAddr(base, instr.Operand), val); gvm.AdvancePC(); return nil })
	v.RegisterContextOpcode(0x3E, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string { mem := getMem(ctx); val := AsI64(gvm.PopTyped()); base := AsI32(gvm.PopTyped()); mem.StoreI64_32(effectiveAddr(base, instr.Operand), val); gvm.AdvancePC(); return nil })

	// 0x3F: memory.size
	v.RegisterContextOpcode(0x3F, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		mem := getMem(ctx)
		gvm.PushTyped(I32(int32(mem.Size())))
		gvm.AdvancePC()
		return nil
	})

	// 0x40: memory.grow
	v.RegisterContextOpcode(0x40, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		mem := getMem(ctx)
		delta := AsI32(gvm.PopTyped())
		result := mem.Grow(int(delta))
		gvm.PushTyped(I32(int32(result)))
		gvm.AdvancePC()
		return nil
	})
}

// ════════════════════════════════════════════════════════════════════════
// CONTROL FLOW (13 instructions: 0x00-0x11)
// ════════════════════════════════════════════════════════════════════════

// RegisterControl registers all control flow instruction handlers.
func RegisterControl(genVM *vm.GenericVM) {
	// 0x00: unreachable
	genVM.RegisterContextOpcode(0x00, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		panic(NewTrapError("unreachable instruction executed"))
	})

	// 0x01: nop
	genVM.RegisterContextOpcode(0x01, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		gvm.AdvancePC()
		return nil
	})

	// 0x02: block
	genVM.RegisterContextOpcode(0x02, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		wctx := ctx.(*WasmExecutionContext)
		blockType := toIntOrDefault(instr.Operand, 0x40)
		arity := blockArity(blockType, wctx.FuncTypes)
		target, ok := wctx.ControlFlowMap[gvm.PC]
		endPc := gvm.PC + 1
		if ok { endPc = target.EndPC }

		wctx.LabelStack = append(wctx.LabelStack, Label{
			Arity: arity, TargetPC: endPc,
			StackHeight: len(gvm.TypedStack), IsLoop: false,
		})
		gvm.AdvancePC()
		return nil
	})

	// 0x03: loop
	genVM.RegisterContextOpcode(0x03, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		wctx := ctx.(*WasmExecutionContext)
		blockType := toIntOrDefault(instr.Operand, 0x40)
		arity := blockArity(blockType, wctx.FuncTypes)

		wctx.LabelStack = append(wctx.LabelStack, Label{
			Arity: arity, TargetPC: gvm.PC,
			StackHeight: len(gvm.TypedStack), IsLoop: true,
		})
		gvm.AdvancePC()
		return nil
	})

	// 0x04: if
	genVM.RegisterContextOpcode(0x04, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		wctx := ctx.(*WasmExecutionContext)
		blockType := toIntOrDefault(instr.Operand, 0x40)
		arity := blockArity(blockType, wctx.FuncTypes)
		condition := AsI32(gvm.PopTyped())

		target, ok := wctx.ControlFlowMap[gvm.PC]
		endPc := gvm.PC + 1
		elsePc := -1
		if ok { endPc = target.EndPC; elsePc = target.ElsePC }

		wctx.LabelStack = append(wctx.LabelStack, Label{
			Arity: arity, TargetPC: endPc,
			StackHeight: len(gvm.TypedStack), IsLoop: false,
		})

		if condition != 0 {
			gvm.AdvancePC()
		} else {
			if elsePc >= 0 {
				gvm.JumpTo(elsePc + 1)
			} else {
				gvm.JumpTo(endPc)
			}
		}
		return nil
	})

	// 0x05: else
	genVM.RegisterContextOpcode(0x05, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		wctx := ctx.(*WasmExecutionContext)
		label := wctx.LabelStack[len(wctx.LabelStack)-1]
		gvm.JumpTo(label.TargetPC)
		return nil
	})

	// 0x0B: end
	genVM.RegisterContextOpcode(0x0B, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		wctx := ctx.(*WasmExecutionContext)
		if len(wctx.LabelStack) > 0 {
			wctx.LabelStack = wctx.LabelStack[:len(wctx.LabelStack)-1]
			gvm.AdvancePC()
		} else {
			wctx.Returned = true
			gvm.Halted = true
		}
		return nil
	})

	// 0x0C: br
	genVM.RegisterContextOpcode(0x0C, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		wctx := ctx.(*WasmExecutionContext)
		labelIndex := toInt(instr.Operand)
		executeBranch(gvm, wctx, labelIndex)
		return nil
	})

	// 0x0D: br_if
	genVM.RegisterContextOpcode(0x0D, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		wctx := ctx.(*WasmExecutionContext)
		labelIndex := toInt(instr.Operand)
		condition := AsI32(gvm.PopTyped())
		if condition != 0 {
			executeBranch(gvm, wctx, labelIndex)
		} else {
			gvm.AdvancePC()
		}
		return nil
	})

	// 0x0E: br_table
	genVM.RegisterContextOpcode(0x0E, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		wctx := ctx.(*WasmExecutionContext)
		m := instr.Operand.(map[string]interface{})
		labels := m["labels"].([]int)
		defaultLabel := toInt(m["defaultLabel"])
		index := AsI32(gvm.PopTyped())

		targetLabel := defaultLabel
		if int(index) >= 0 && int(index) < len(labels) {
			targetLabel = labels[index]
		}
		executeBranch(gvm, wctx, targetLabel)
		return nil
	})

	// 0x0F: return
	genVM.RegisterContextOpcode(0x0F, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		wctx := ctx.(*WasmExecutionContext)
		wctx.Returned = true
		gvm.Halted = true
		return nil
	})

	// 0x10: call
	genVM.RegisterContextOpcode(0x10, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		wctx := ctx.(*WasmExecutionContext)
		funcIndex := toInt(instr.Operand)
		callFunction(gvm, wctx, funcIndex)
		return nil
	})

	// 0x11: call_indirect
	genVM.RegisterContextOpcode(0x11, func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		wctx := ctx.(*WasmExecutionContext)
		m := instr.Operand.(map[string]interface{})
		typeIdx := toInt(m["typeidx"])
		tableIdx := 0
		if v, ok := m["tableidx"]; ok { tableIdx = toInt(v) }

		elemIndex := AsI32(gvm.PopTyped())
		table := wctx.Tables[tableIdx]
		if table == nil { panic(NewTrapError("undefined table")) }

		funcIndex := table.Get(int(elemIndex))
		if funcIndex < 0 { panic(NewTrapError("uninitialized table element")) }

		// Type check.
		expectedType := wctx.FuncTypes[typeIdx]
		actualType := wctx.FuncTypes[funcIndex]
		if !funcTypesEqual(expectedType, actualType) {
			panic(NewTrapError("indirect call type mismatch"))
		}

		callFunction(gvm, wctx, funcIndex)
		return nil
	})
}

// ════════════════════════════════════════════════════════════════════════
// HELPERS
// ════════════════════════════════════════════════════════════════════════

func boolToI32(b bool) int32 {
	if b { return 1 }
	return 0
}

// toInt converts various numeric types to int.
func toInt(v interface{}) int {
	switch val := v.(type) {
	case int:
		return val
	case int32:
		return int(val)
	case int64:
		return int(val)
	case uint64:
		return int(val)
	case float64:
		return int(val)
	default:
		return 0
	}
}

func toInt32(v interface{}) int32 {
	switch val := v.(type) {
	case int:
		return int32(val)
	case int32:
		return val
	case int64:
		return int32(val)
	case float64:
		return int32(val)
	default:
		return 0
	}
}

func toInt64(v interface{}) int64 {
	switch val := v.(type) {
	case int64:
		return val
	case int:
		return int64(val)
	case int32:
		return int64(val)
	default:
		return 0
	}
}

func toFloat32(v interface{}) float32 {
	switch val := v.(type) {
	case float32:
		return val
	case float64:
		return float32(val)
	default:
		return 0
	}
}

func toFloat64(v interface{}) float64 {
	switch val := v.(type) {
	case float64:
		return val
	case float32:
		return float64(val)
	default:
		return 0
	}
}

func toIntOrDefault(v interface{}, def int) int {
	if v == nil { return def }
	return toInt(v)
}

// blockArity determines how many values a block type produces.
func blockArity(blockType int, funcTypes []wasmtypes.FuncType) int {
	if blockType == 0x40 { return 0 }
	if blockType == int(wasmtypes.ValueTypeI32) || blockType == int(wasmtypes.ValueTypeI64) ||
		blockType == int(wasmtypes.ValueTypeF32) || blockType == int(wasmtypes.ValueTypeF64) {
		return 1
	}
	if blockType >= 0 && blockType < len(funcTypes) {
		return len(funcTypes[blockType].Results)
	}
	return 0
}

func funcTypesEqual(a, b wasmtypes.FuncType) bool {
	if len(a.Params) != len(b.Params) || len(a.Results) != len(b.Results) {
		return false
	}
	for i := range a.Params {
		if a.Params[i] != b.Params[i] { return false }
	}
	for i := range a.Results {
		if a.Results[i] != b.Results[i] { return false }
	}
	return true
}

// executeBranch implements the branch-to-label-N logic.
func executeBranch(gvm *vm.GenericVM, wctx *WasmExecutionContext, labelIndex int) {
	labelStackIndex := len(wctx.LabelStack) - 1 - labelIndex
	if labelStackIndex < 0 {
		panic(NewTrapError("branch target out of range"))
	}
	label := wctx.LabelStack[labelStackIndex]
	arity := label.Arity
	if label.IsLoop { arity = 0 }

	// Save result values.
	var results []WasmValue
	for j := 0; j < arity; j++ {
		results = append([]WasmValue{gvm.PopTyped()}, results...)
	}

	// Unwind the stack.
	for len(gvm.TypedStack) > label.StackHeight {
		gvm.PopTyped()
	}

	// Push results back.
	for _, r := range results {
		gvm.PushTyped(r)
	}

	// Pop labels.
	wctx.LabelStack = wctx.LabelStack[:labelStackIndex]

	// For blocks/if: jump past the end instruction (targetPC points to end,
	// so +1 to skip it).  For loops: jump back to loop start (targetPC is
	// already the loop instruction, and the loop handler advances PC).
	if label.IsLoop {
		gvm.JumpTo(label.TargetPC)
	} else {
		gvm.JumpTo(label.TargetPC + 1)
	}
}

// callFunction calls a WASM function (host or module-defined).
//
// For module functions, it saves the caller's entire execution state
// (TypedStack, PC, Halted, TypedLocals, LabelStack, ControlFlowMap),
// executes the callee in a fresh context using the SAME GenericVM,
// collects the callee's results, restores the caller's state, pushes
// results back onto the caller's stack, and advances the caller's PC.
//
// This is correct recursive execution.  The previous approach of
// JumpTo(0) was broken: it reset the PC but left the CALLER's code
// object in place, causing the call instruction to re-execute forever.
func callFunction(gvm *vm.GenericVM, wctx *WasmExecutionContext, funcIndex int) {
	funcType := wctx.FuncTypes[funcIndex]

	// Pop arguments in reverse (last argument is on top of stack).
	args := make([]WasmValue, len(funcType.Params))
	for j := len(funcType.Params) - 1; j >= 0; j-- {
		args[j] = gvm.PopTyped()
	}

	// Host function — call directly and continue.
	if funcIndex < len(wctx.HostFunctions) && wctx.HostFunctions[funcIndex] != nil {
		results := wctx.HostFunctions[funcIndex].Call(args)
		for _, r := range results {
			gvm.PushTyped(r)
		}
		gvm.AdvancePC()
		return
	}

	// Module-defined function.
	body := wctx.FuncBodies[funcIndex]
	if body == nil {
		panic(NewTrapError("no body for function"))
	}

	// Guard against runaway recursion.
	wctx.CallDepth++
	if wctx.CallDepth > MaxCallDepth {
		wctx.CallDepth--
		panic(NewTrapError("call stack exhausted"))
	}

	// ── Save caller state ───────────────────────────────────────────────
	savedLocals := append([]WasmValue{}, wctx.TypedLocals...)
	savedLabels := append([]Label{}, wctx.LabelStack...)
	savedCFM := wctx.ControlFlowMap
	savedReturned := wctx.Returned
	savedStack := append([]WasmValue{}, gvm.TypedStack...)
	savedPC := gvm.PC
	savedHalted := gvm.Halted

	// ── Set up callee ────────────────────────────────────────────────────
	locals := make([]WasmValue, 0, len(args)+len(body.Locals))
	locals = append(locals, args...)
	for _, lt := range body.Locals {
		locals = append(locals, DefaultValue(lt))
	}
	wctx.TypedLocals = locals
	wctx.LabelStack = nil
	wctx.Returned = false

	decoded := DecodeFunctionBody(body)
	wctx.ControlFlowMap = BuildControlFlowMap(decoded)
	calleeCode := vm.CodeObject{Instructions: ToVMInstructions(decoded)}

	// Fresh VM state for callee execution.
	gvm.TypedStack = nil
	gvm.PC = 0
	gvm.Halted = false

	// ── Execute callee ───────────────────────────────────────────────────
	gvm.ExecuteWithContext(calleeCode, wctx)

	// ── Collect callee results ───────────────────────────────────────────
	results := make([]WasmValue, len(funcType.Results))
	for i := len(results) - 1; i >= 0; i-- {
		if len(gvm.TypedStack) > 0 {
			results[i] = gvm.PopTyped()
		}
	}

	// ── Restore caller state ─────────────────────────────────────────────
	wctx.CallDepth--
	wctx.TypedLocals = savedLocals
	wctx.LabelStack = savedLabels
	wctx.ControlFlowMap = savedCFM
	wctx.Returned = savedReturned
	gvm.TypedStack = savedStack
	gvm.PC = savedPC
	gvm.Halted = savedHalted

	// ── Push results to caller's stack and continue ──────────────────────
	for _, r := range results {
		gvm.PushTyped(r)
	}
	gvm.AdvancePC()
}

// ════════════════════════════════════════════════════════════════════════
// GENERIC HANDLER REGISTRATION HELPERS
// ════════════════════════════════════════════════════════════════════════

func registerI64Cmp(v *vm.GenericVM, opcode byte, cmp func(int64, int64) bool) {
	v.RegisterContextOpcode(vm.OpCode(opcode), func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI64(gvm.PopTyped())
		a := AsI64(gvm.PopTyped())
		gvm.PushTyped(I32(boolToI32(cmp(a, b))))
		gvm.AdvancePC()
		return nil
	})
}

func registerI64Unary(v *vm.GenericVM, opcode byte, op func(int64) int64) {
	v.RegisterContextOpcode(vm.OpCode(opcode), func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsI64(gvm.PopTyped())
		gvm.PushTyped(I64(op(a)))
		gvm.AdvancePC()
		return nil
	})
}

func registerI64Binary(v *vm.GenericVM, opcode byte, op func(int64, int64) int64) {
	v.RegisterContextOpcode(vm.OpCode(opcode), func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsI64(gvm.PopTyped())
		a := AsI64(gvm.PopTyped())
		gvm.PushTyped(I64(op(a, b)))
		gvm.AdvancePC()
		return nil
	})
}

func registerF32Cmp(v *vm.GenericVM, opcode byte, cmp func(float32, float32) bool) {
	v.RegisterContextOpcode(vm.OpCode(opcode), func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsF32(gvm.PopTyped())
		a := AsF32(gvm.PopTyped())
		gvm.PushTyped(I32(boolToI32(cmp(a, b))))
		gvm.AdvancePC()
		return nil
	})
}

func registerF32Unary(v *vm.GenericVM, opcode byte, op func(float32) float32) {
	v.RegisterContextOpcode(vm.OpCode(opcode), func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsF32(gvm.PopTyped())
		gvm.PushTyped(F32(op(a)))
		gvm.AdvancePC()
		return nil
	})
}

func registerF32Binary(v *vm.GenericVM, opcode byte, op func(float32, float32) float32) {
	v.RegisterContextOpcode(vm.OpCode(opcode), func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsF32(gvm.PopTyped())
		a := AsF32(gvm.PopTyped())
		gvm.PushTyped(F32(op(a, b)))
		gvm.AdvancePC()
		return nil
	})
}

func registerF64Cmp(v *vm.GenericVM, opcode byte, cmp func(float64, float64) bool) {
	v.RegisterContextOpcode(vm.OpCode(opcode), func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsF64(gvm.PopTyped())
		a := AsF64(gvm.PopTyped())
		gvm.PushTyped(I32(boolToI32(cmp(a, b))))
		gvm.AdvancePC()
		return nil
	})
}

func registerF64Unary(v *vm.GenericVM, opcode byte, op func(float64) float64) {
	v.RegisterContextOpcode(vm.OpCode(opcode), func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		a := AsF64(gvm.PopTyped())
		gvm.PushTyped(F64(op(a)))
		gvm.AdvancePC()
		return nil
	})
}

func registerF64Binary(v *vm.GenericVM, opcode byte, op func(float64, float64) float64) {
	v.RegisterContextOpcode(vm.OpCode(opcode), func(gvm *vm.GenericVM, instr vm.Instruction, code vm.CodeObject, ctx interface{}) *string {
		b := AsF64(gvm.PopTyped())
		a := AsF64(gvm.PopTyped())
		gvm.PushTyped(F64(op(a, b)))
		gvm.AdvancePC()
		return nil
	})
}
