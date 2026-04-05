# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_wasm_execution"

# ==========================================================================
# Tests for WASM Instruction Handlers
# ==========================================================================
#
# Each WASM instruction is registered as a handler on the GenericVM.
# To test them in isolation, we:
#   1. Create a GenericVM instance
#   2. Register the handlers
#   3. Push operands onto the typed stack
#   4. Execute a single instruction via a minimal CodeObject
#   5. Verify the result on the stack
#
# Categories tested:
#   - i32 numeric (arithmetic, comparison, bitwise, shifts, rotations)
#   - i64 numeric (same categories)
#   - f32 numeric (arithmetic, comparison, unary)
#   - f64 numeric (arithmetic, comparison, unary)
#   - Conversion instructions (wrap, extend, trunc, convert, reinterpret)
#   - Parametric instructions (drop, select)
#   - Variable instructions (local.get, local.set, local.tee, global.get/set)
#   - Memory instructions (load, store, memory.size, memory.grow)
#   - Utility helpers (clz, ctz, popcnt for both i32 and i64)
# ==========================================================================

class TestInstructions < Minitest::Test
  WE = CodingAdventures::WasmExecution
  VM = CodingAdventures::VirtualMachine
  TrapError = CodingAdventures::WasmExecution::TrapError

  def setup
    @vm = VM::GenericVM.new
    WE::Instructions::Dispatch.register_all(@vm)
    WE::Instructions::Control.register(@vm)
  end

  # Helper: execute a single instruction with given opcode and operand.
  # Calls the handler directly on the VM without resetting state,
  # so callers can push operands onto the typed stack beforehand.
  def exec_instr(opcode, operand = nil, ctx = nil)
    instr = VM::Instruction.new(opcode: opcode, operand: operand)
    code = VM::CodeObject.new(instructions: [instr], constants: [], names: [])
    ctx ||= default_context
    handler = @vm.instance_variable_get(:@context_handlers)[opcode]
    raise "No handler for opcode 0x#{opcode.to_s(16)}" unless handler
    handler.call(@vm, instr, code, ctx)
  end

  def default_context
    {
      memory: nil,
      tables: [],
      globals: [],
      global_types: [],
      func_types: [],
      func_bodies: [],
      host_functions: [],
      typed_locals: [],
      label_stack: [],
      control_flow_map: {},
      saved_frames: [],
      returned: false,
      return_values: [],
      current_instructions: []
    }
  end

  # ── i32 Utility Helpers ────────────────────────────────────────────

  def test_clz32_zero
    assert_equal 32, WE::Instructions::NumericI32.clz32(0)
  end

  def test_clz32_one
    assert_equal 31, WE::Instructions::NumericI32.clz32(1)
  end

  def test_clz32_high_bit_set
    assert_equal 0, WE::Instructions::NumericI32.clz32(0x80000000)
  end

  def test_clz32_all_ones
    assert_equal 0, WE::Instructions::NumericI32.clz32(0xFFFFFFFF)
  end

  def test_ctz32_zero
    assert_equal 32, WE::Instructions::NumericI32.ctz32(0)
  end

  def test_ctz32_one
    assert_equal 0, WE::Instructions::NumericI32.ctz32(1)
  end

  def test_ctz32_powers_of_two
    assert_equal 4, WE::Instructions::NumericI32.ctz32(16)
  end

  def test_popcnt32_zero
    assert_equal 0, WE::Instructions::NumericI32.popcnt32(0)
  end

  def test_popcnt32_all_ones
    assert_equal 32, WE::Instructions::NumericI32.popcnt32(0xFFFFFFFF)
  end

  def test_popcnt32_alternating
    assert_equal 16, WE::Instructions::NumericI32.popcnt32(0xAAAAAAAA)
  end

  # ── i64 Utility Helpers ────────────────────────────────────────────

  def test_clz64_zero
    assert_equal 64, WE::Instructions::NumericI64.clz64(0)
  end

  def test_clz64_one
    assert_equal 63, WE::Instructions::NumericI64.clz64(1)
  end

  def test_ctz64_zero
    assert_equal 64, WE::Instructions::NumericI64.ctz64(0)
  end

  def test_ctz64_one
    assert_equal 0, WE::Instructions::NumericI64.ctz64(1)
  end

  def test_popcnt64_zero
    assert_equal 0, WE::Instructions::NumericI64.popcnt64(0)
  end

  def test_popcnt64_all_ones
    assert_equal 64, WE::Instructions::NumericI64.popcnt64(0xFFFFFFFFFFFFFFFF)
  end

  # ── i32 Arithmetic Instructions ────────────────────────────────────

  def test_i32_const
    @vm.push_typed(WE.i32(0)) # dummy to check const pushes a new value
    exec_instr(0x41, 42)
    assert_equal 42, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_add
    @vm.push_typed(WE.i32(10))
    @vm.push_typed(WE.i32(20))
    exec_instr(0x6A)
    assert_equal 30, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_add_wraps
    @vm.push_typed(WE.i32(2_147_483_647))
    @vm.push_typed(WE.i32(1))
    exec_instr(0x6A)
    assert_equal(-2_147_483_648, WE.as_i32(@vm.pop_typed))
  end

  def test_i32_sub
    @vm.push_typed(WE.i32(30))
    @vm.push_typed(WE.i32(10))
    exec_instr(0x6B)
    assert_equal 20, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_mul
    @vm.push_typed(WE.i32(6))
    @vm.push_typed(WE.i32(7))
    exec_instr(0x6C)
    assert_equal 42, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_div_s
    @vm.push_typed(WE.i32(10))
    @vm.push_typed(WE.i32(3))
    exec_instr(0x6D)
    assert_equal 3, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_div_s_negative
    @vm.push_typed(WE.i32(-7))
    @vm.push_typed(WE.i32(2))
    exec_instr(0x6D)
    assert_equal(-3, WE.as_i32(@vm.pop_typed))
  end

  def test_i32_div_s_by_zero_traps
    @vm.push_typed(WE.i32(1))
    @vm.push_typed(WE.i32(0))
    assert_raises(TrapError) { exec_instr(0x6D) }
  end

  def test_i32_div_s_overflow_traps
    @vm.push_typed(WE.i32(-2_147_483_648))
    @vm.push_typed(WE.i32(-1))
    assert_raises(TrapError) { exec_instr(0x6D) }
  end

  def test_i32_div_u
    @vm.push_typed(WE.i32(-1)) # 0xFFFFFFFF unsigned
    @vm.push_typed(WE.i32(2))
    exec_instr(0x6E)
    # 4294967295 / 2 = 2147483647
    assert_equal 2_147_483_647, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_div_u_by_zero_traps
    @vm.push_typed(WE.i32(1))
    @vm.push_typed(WE.i32(0))
    assert_raises(TrapError) { exec_instr(0x6E) }
  end

  def test_i32_rem_s
    @vm.push_typed(WE.i32(7))
    @vm.push_typed(WE.i32(3))
    exec_instr(0x6F)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_rem_s_negative_dividend
    @vm.push_typed(WE.i32(-7))
    @vm.push_typed(WE.i32(3))
    exec_instr(0x6F)
    assert_equal(-1, WE.as_i32(@vm.pop_typed))
  end

  def test_i32_rem_s_overflow_returns_zero
    @vm.push_typed(WE.i32(-2_147_483_648))
    @vm.push_typed(WE.i32(-1))
    exec_instr(0x6F)
    assert_equal 0, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_rem_s_by_zero_traps
    @vm.push_typed(WE.i32(1))
    @vm.push_typed(WE.i32(0))
    assert_raises(TrapError) { exec_instr(0x6F) }
  end

  def test_i32_rem_u
    @vm.push_typed(WE.i32(7))
    @vm.push_typed(WE.i32(3))
    exec_instr(0x70)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  # ── i32 Comparison Instructions ────────────────────────────────────

  def test_i32_eqz_true
    @vm.push_typed(WE.i32(0))
    exec_instr(0x45)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_eqz_false
    @vm.push_typed(WE.i32(42))
    exec_instr(0x45)
    assert_equal 0, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_eq_true
    @vm.push_typed(WE.i32(5))
    @vm.push_typed(WE.i32(5))
    exec_instr(0x46)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_eq_false
    @vm.push_typed(WE.i32(5))
    @vm.push_typed(WE.i32(6))
    exec_instr(0x46)
    assert_equal 0, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_ne_true
    @vm.push_typed(WE.i32(5))
    @vm.push_typed(WE.i32(6))
    exec_instr(0x47)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_lt_s_true
    @vm.push_typed(WE.i32(-1))
    @vm.push_typed(WE.i32(0))
    exec_instr(0x48)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_lt_s_false
    @vm.push_typed(WE.i32(0))
    @vm.push_typed(WE.i32(-1))
    exec_instr(0x48)
    assert_equal 0, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_lt_u
    # -1 as unsigned is 0xFFFFFFFF which is greater than 0
    @vm.push_typed(WE.i32(-1))
    @vm.push_typed(WE.i32(0))
    exec_instr(0x49)
    assert_equal 0, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_gt_s
    @vm.push_typed(WE.i32(5))
    @vm.push_typed(WE.i32(3))
    exec_instr(0x4A)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_gt_u
    @vm.push_typed(WE.i32(-1)) # max unsigned
    @vm.push_typed(WE.i32(1))
    exec_instr(0x4B)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_le_s_equal
    @vm.push_typed(WE.i32(5))
    @vm.push_typed(WE.i32(5))
    exec_instr(0x4C)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_le_u
    @vm.push_typed(WE.i32(0))
    @vm.push_typed(WE.i32(-1)) # max unsigned
    exec_instr(0x4D)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_ge_s
    @vm.push_typed(WE.i32(5))
    @vm.push_typed(WE.i32(5))
    exec_instr(0x4E)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_ge_u
    @vm.push_typed(WE.i32(-1)) # max unsigned
    @vm.push_typed(WE.i32(0))
    exec_instr(0x4F)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  # ── i32 Bitwise Instructions ───────────────────────────────────────

  def test_i32_and
    @vm.push_typed(WE.i32(0xFF00))
    @vm.push_typed(WE.i32(0x0FF0))
    exec_instr(0x71)
    assert_equal 0x0F00, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_or
    @vm.push_typed(WE.i32(0xFF00))
    @vm.push_typed(WE.i32(0x00FF))
    exec_instr(0x72)
    assert_equal 0xFFFF, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_xor
    @vm.push_typed(WE.i32(0xFF))
    @vm.push_typed(WE.i32(0xFF))
    exec_instr(0x73)
    assert_equal 0, WE.as_i32(@vm.pop_typed)
  end

  # ── i32 Shift and Rotate ──────────────────────────────────────────

  def test_i32_shl
    @vm.push_typed(WE.i32(1))
    @vm.push_typed(WE.i32(4))
    exec_instr(0x74)
    assert_equal 16, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_shl_wraps_shift_amount
    @vm.push_typed(WE.i32(1))
    @vm.push_typed(WE.i32(32)) # 32 & 31 = 0
    exec_instr(0x74)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_shr_s
    @vm.push_typed(WE.i32(-16))
    @vm.push_typed(WE.i32(2))
    exec_instr(0x75)
    assert_equal(-4, WE.as_i32(@vm.pop_typed))
  end

  def test_i32_shr_u
    @vm.push_typed(WE.i32(-1))
    @vm.push_typed(WE.i32(1))
    exec_instr(0x76)
    assert_equal 2_147_483_647, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_rotl
    @vm.push_typed(WE.i32(0x80000000))
    @vm.push_typed(WE.i32(1))
    exec_instr(0x77)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_rotr
    @vm.push_typed(WE.i32(1))
    @vm.push_typed(WE.i32(1))
    exec_instr(0x78)
    result = WE.as_i32(@vm.pop_typed)
    assert_equal(-2_147_483_648, result) # 0x80000000
  end

  # ── i32 clz/ctz/popcnt via instruction handlers ───────────────────

  def test_i32_clz_instruction
    @vm.push_typed(WE.i32(1))
    exec_instr(0x67)
    assert_equal 31, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_ctz_instruction
    @vm.push_typed(WE.i32(16))
    exec_instr(0x68)
    assert_equal 4, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_popcnt_instruction
    @vm.push_typed(WE.i32(0xFF))
    exec_instr(0x69)
    assert_equal 8, WE.as_i32(@vm.pop_typed)
  end

  # ── i64 Arithmetic Instructions ────────────────────────────────────

  def test_i64_const
    exec_instr(0x42, 999)
    assert_equal 999, WE.as_i64(@vm.pop_typed)
  end

  def test_i64_add
    @vm.push_typed(WE.i64(100))
    @vm.push_typed(WE.i64(200))
    exec_instr(0x7C)
    assert_equal 300, WE.as_i64(@vm.pop_typed)
  end

  def test_i64_sub
    @vm.push_typed(WE.i64(300))
    @vm.push_typed(WE.i64(100))
    exec_instr(0x7D)
    assert_equal 200, WE.as_i64(@vm.pop_typed)
  end

  def test_i64_mul
    @vm.push_typed(WE.i64(6))
    @vm.push_typed(WE.i64(7))
    exec_instr(0x7E)
    assert_equal 42, WE.as_i64(@vm.pop_typed)
  end

  def test_i64_div_s_by_zero_traps
    @vm.push_typed(WE.i64(1))
    @vm.push_typed(WE.i64(0))
    assert_raises(TrapError) { exec_instr(0x7F) }
  end

  def test_i64_div_u
    @vm.push_typed(WE.i64(10))
    @vm.push_typed(WE.i64(3))
    exec_instr(0x80)
    assert_equal 3, WE.as_i64(@vm.pop_typed)
  end

  def test_i64_rem_s
    @vm.push_typed(WE.i64(7))
    @vm.push_typed(WE.i64(3))
    exec_instr(0x81)
    assert_equal 1, WE.as_i64(@vm.pop_typed)
  end

  def test_i64_rem_u
    @vm.push_typed(WE.i64(7))
    @vm.push_typed(WE.i64(3))
    exec_instr(0x82)
    assert_equal 1, WE.as_i64(@vm.pop_typed)
  end

  # ── i64 Comparison Instructions ────────────────────────────────────

  def test_i64_eqz_true
    @vm.push_typed(WE.i64(0))
    exec_instr(0x50)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_i64_eqz_false
    @vm.push_typed(WE.i64(1))
    exec_instr(0x50)
    assert_equal 0, WE.as_i32(@vm.pop_typed)
  end

  def test_i64_eq
    @vm.push_typed(WE.i64(42))
    @vm.push_typed(WE.i64(42))
    exec_instr(0x51)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_i64_ne
    @vm.push_typed(WE.i64(1))
    @vm.push_typed(WE.i64(2))
    exec_instr(0x52)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_i64_lt_s
    @vm.push_typed(WE.i64(-1))
    @vm.push_typed(WE.i64(0))
    exec_instr(0x53)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_i64_gt_s
    @vm.push_typed(WE.i64(5))
    @vm.push_typed(WE.i64(3))
    exec_instr(0x55)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  # ── i64 Bitwise Instructions ──────────────────────────────────────

  def test_i64_and
    @vm.push_typed(WE.i64(0xFF00))
    @vm.push_typed(WE.i64(0x0FF0))
    exec_instr(0x83)
    assert_equal 0x0F00, WE.as_i64(@vm.pop_typed)
  end

  def test_i64_or
    @vm.push_typed(WE.i64(0xFF00))
    @vm.push_typed(WE.i64(0x00FF))
    exec_instr(0x84)
    assert_equal 0xFFFF, WE.as_i64(@vm.pop_typed)
  end

  def test_i64_xor
    @vm.push_typed(WE.i64(0xFF))
    @vm.push_typed(WE.i64(0xFF))
    exec_instr(0x85)
    assert_equal 0, WE.as_i64(@vm.pop_typed)
  end

  def test_i64_shl
    @vm.push_typed(WE.i64(1))
    @vm.push_typed(WE.i64(10))
    exec_instr(0x86)
    assert_equal 1024, WE.as_i64(@vm.pop_typed)
  end

  def test_i64_shr_s
    @vm.push_typed(WE.i64(-16))
    @vm.push_typed(WE.i64(2))
    exec_instr(0x87)
    assert_equal(-4, WE.as_i64(@vm.pop_typed))
  end

  def test_i64_shr_u
    @vm.push_typed(WE.i64(-1))
    @vm.push_typed(WE.i64(1))
    exec_instr(0x88)
    result = WE.as_i64(@vm.pop_typed)
    assert_equal 9223372036854775807, result # 0x7FFFFFFFFFFFFFFF
  end

  # ── f32 Instructions ───────────────────────────────────────────────

  def test_f32_const
    exec_instr(0x43, 3.14)
    assert_in_delta 3.14, WE.as_f32(@vm.pop_typed), 0.01
  end

  def test_f32_add
    @vm.push_typed(WE.f32(1.5))
    @vm.push_typed(WE.f32(2.5))
    exec_instr(0x92)
    assert_in_delta 4.0, WE.as_f32(@vm.pop_typed), 1e-6
  end

  def test_f32_sub
    @vm.push_typed(WE.f32(5.0))
    @vm.push_typed(WE.f32(2.0))
    exec_instr(0x93)
    assert_in_delta 3.0, WE.as_f32(@vm.pop_typed), 1e-6
  end

  def test_f32_mul
    @vm.push_typed(WE.f32(3.0))
    @vm.push_typed(WE.f32(4.0))
    exec_instr(0x94)
    assert_in_delta 12.0, WE.as_f32(@vm.pop_typed), 1e-6
  end

  def test_f32_div
    @vm.push_typed(WE.f32(10.0))
    @vm.push_typed(WE.f32(4.0))
    exec_instr(0x95)
    assert_in_delta 2.5, WE.as_f32(@vm.pop_typed), 1e-6
  end

  def test_f32_eq_true
    @vm.push_typed(WE.f32(1.0))
    @vm.push_typed(WE.f32(1.0))
    exec_instr(0x5B)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_f32_ne_with_nan
    @vm.push_typed(WE.f32(Float::NAN))
    @vm.push_typed(WE.f32(Float::NAN))
    exec_instr(0x5C)
    assert_equal 1, WE.as_i32(@vm.pop_typed) # NaN != NaN
  end

  def test_f32_lt
    @vm.push_typed(WE.f32(1.0))
    @vm.push_typed(WE.f32(2.0))
    exec_instr(0x5D)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_f32_abs
    @vm.push_typed(WE.f32(-5.0))
    exec_instr(0x8B)
    assert_in_delta 5.0, WE.as_f32(@vm.pop_typed), 1e-6
  end

  def test_f32_neg
    @vm.push_typed(WE.f32(5.0))
    exec_instr(0x8C)
    assert_in_delta(-5.0, WE.as_f32(@vm.pop_typed), 1e-6)
  end

  def test_f32_ceil
    @vm.push_typed(WE.f32(1.3))
    exec_instr(0x8D)
    assert_in_delta 2.0, WE.as_f32(@vm.pop_typed), 1e-6
  end

  def test_f32_floor
    @vm.push_typed(WE.f32(1.7))
    exec_instr(0x8E)
    assert_in_delta 1.0, WE.as_f32(@vm.pop_typed), 1e-6
  end

  def test_f32_trunc
    @vm.push_typed(WE.f32(-1.7))
    exec_instr(0x8F)
    assert_in_delta(-1.0, WE.as_f32(@vm.pop_typed), 1e-6)
  end

  def test_f32_nearest
    @vm.push_typed(WE.f32(2.5))
    exec_instr(0x90)
    assert_in_delta 2.0, WE.as_f32(@vm.pop_typed), 1e-6 # banker's rounding
  end

  def test_f32_sqrt
    @vm.push_typed(WE.f32(9.0))
    exec_instr(0x91)
    assert_in_delta 3.0, WE.as_f32(@vm.pop_typed), 1e-6
  end

  def test_f32_min
    @vm.push_typed(WE.f32(3.0))
    @vm.push_typed(WE.f32(1.0))
    exec_instr(0x96)
    assert_in_delta 1.0, WE.as_f32(@vm.pop_typed), 1e-6
  end

  def test_f32_max
    @vm.push_typed(WE.f32(3.0))
    @vm.push_typed(WE.f32(1.0))
    exec_instr(0x97)
    assert_in_delta 3.0, WE.as_f32(@vm.pop_typed), 1e-6
  end

  def test_f32_min_with_nan
    @vm.push_typed(WE.f32(Float::NAN))
    @vm.push_typed(WE.f32(1.0))
    exec_instr(0x96)
    assert_predicate WE.as_f32(@vm.pop_typed), :nan?
  end

  def test_f32_copysign
    @vm.push_typed(WE.f32(5.0))
    @vm.push_typed(WE.f32(-1.0))
    exec_instr(0x98)
    assert_in_delta(-5.0, WE.as_f32(@vm.pop_typed), 1e-6)
  end

  # ── f64 Instructions ───────────────────────────────────────────────

  def test_f64_const
    exec_instr(0x44, 3.14159)
    assert_in_delta 3.14159, WE.as_f64(@vm.pop_typed), 1e-10
  end

  def test_f64_add
    @vm.push_typed(WE.f64(1.5))
    @vm.push_typed(WE.f64(2.5))
    exec_instr(0xA0)
    assert_in_delta 4.0, WE.as_f64(@vm.pop_typed), 1e-14
  end

  def test_f64_sub
    @vm.push_typed(WE.f64(5.0))
    @vm.push_typed(WE.f64(2.0))
    exec_instr(0xA1)
    assert_in_delta 3.0, WE.as_f64(@vm.pop_typed), 1e-14
  end

  def test_f64_mul
    @vm.push_typed(WE.f64(3.0))
    @vm.push_typed(WE.f64(4.0))
    exec_instr(0xA2)
    assert_in_delta 12.0, WE.as_f64(@vm.pop_typed), 1e-14
  end

  def test_f64_div
    @vm.push_typed(WE.f64(10.0))
    @vm.push_typed(WE.f64(4.0))
    exec_instr(0xA3)
    assert_in_delta 2.5, WE.as_f64(@vm.pop_typed), 1e-14
  end

  def test_f64_eq_true
    @vm.push_typed(WE.f64(1.0))
    @vm.push_typed(WE.f64(1.0))
    exec_instr(0x61)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_f64_abs
    @vm.push_typed(WE.f64(-7.5))
    exec_instr(0x99)
    assert_in_delta 7.5, WE.as_f64(@vm.pop_typed), 1e-14
  end

  def test_f64_neg
    @vm.push_typed(WE.f64(7.5))
    exec_instr(0x9A)
    assert_in_delta(-7.5, WE.as_f64(@vm.pop_typed), 1e-14)
  end

  def test_f64_ceil
    @vm.push_typed(WE.f64(1.1))
    exec_instr(0x9B)
    assert_in_delta 2.0, WE.as_f64(@vm.pop_typed), 1e-14
  end

  def test_f64_floor
    @vm.push_typed(WE.f64(1.9))
    exec_instr(0x9C)
    assert_in_delta 1.0, WE.as_f64(@vm.pop_typed), 1e-14
  end

  def test_f64_sqrt
    @vm.push_typed(WE.f64(25.0))
    exec_instr(0x9F)
    assert_in_delta 5.0, WE.as_f64(@vm.pop_typed), 1e-14
  end

  def test_f64_min
    @vm.push_typed(WE.f64(3.0))
    @vm.push_typed(WE.f64(1.0))
    exec_instr(0xA4)
    assert_in_delta 1.0, WE.as_f64(@vm.pop_typed), 1e-14
  end

  def test_f64_max
    @vm.push_typed(WE.f64(3.0))
    @vm.push_typed(WE.f64(1.0))
    exec_instr(0xA5)
    assert_in_delta 3.0, WE.as_f64(@vm.pop_typed), 1e-14
  end

  def test_f64_copysign
    @vm.push_typed(WE.f64(5.0))
    @vm.push_typed(WE.f64(-1.0))
    exec_instr(0xA6)
    assert_in_delta(-5.0, WE.as_f64(@vm.pop_typed), 1e-14)
  end

  # ── Conversion Instructions ────────────────────────────────────────

  def test_i32_wrap_i64
    @vm.push_typed(WE.i64(0x100000001)) # 4294967297
    exec_instr(0xA7)
    assert_equal 1, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_trunc_f32_s
    @vm.push_typed(WE.f32(3.7))
    exec_instr(0xA8)
    assert_equal 3, WE.as_i32(@vm.pop_typed)
  end

  def test_i32_trunc_f32_s_nan_traps
    @vm.push_typed(WE.f32(Float::NAN))
    assert_raises(TrapError) { exec_instr(0xA8) }
  end

  def test_i32_trunc_f64_s
    @vm.push_typed(WE.f64(-2.9))
    exec_instr(0xAA)
    assert_equal(-2, WE.as_i32(@vm.pop_typed))
  end

  def test_i32_trunc_f64_u_negative_traps
    @vm.push_typed(WE.f64(-1.0))
    assert_raises(TrapError) { exec_instr(0xAB) }
  end

  def test_i64_extend_i32_s
    @vm.push_typed(WE.i32(-1))
    exec_instr(0xAC)
    assert_equal(-1, WE.as_i64(@vm.pop_typed))
  end

  def test_i64_extend_i32_u
    @vm.push_typed(WE.i32(-1))
    exec_instr(0xAD)
    assert_equal 4_294_967_295, WE.as_i64(@vm.pop_typed)
  end

  def test_f32_convert_i32_s
    @vm.push_typed(WE.i32(-5))
    exec_instr(0xB2)
    assert_in_delta(-5.0, WE.as_f32(@vm.pop_typed), 1e-6)
  end

  def test_f32_convert_i32_u
    @vm.push_typed(WE.i32(-1)) # 0xFFFFFFFF unsigned
    exec_instr(0xB3)
    assert_in_delta 4_294_967_295.0, WE.as_f32(@vm.pop_typed), 1000.0
  end

  def test_f64_convert_i32_s
    @vm.push_typed(WE.i32(-5))
    exec_instr(0xB7)
    assert_in_delta(-5.0, WE.as_f64(@vm.pop_typed), 1e-14)
  end

  def test_f64_promote_f32
    @vm.push_typed(WE.f32(1.5))
    exec_instr(0xBB)
    assert_in_delta 1.5, WE.as_f64(@vm.pop_typed), 1e-6
  end

  def test_f32_demote_f64
    @vm.push_typed(WE.f64(1.5))
    exec_instr(0xB6)
    assert_in_delta 1.5, WE.as_f32(@vm.pop_typed), 1e-6
  end

  def test_i32_reinterpret_f32
    @vm.push_typed(WE.f32(1.0))
    exec_instr(0xBC)
    # IEEE 754: 1.0f = 0x3F800000
    assert_equal 0x3F800000, WE.to_u32(WE.as_i32(@vm.pop_typed))
  end

  def test_f32_reinterpret_i32
    @vm.push_typed(WE.i32(0x3F800000))
    exec_instr(0xBE)
    assert_in_delta 1.0, WE.as_f32(@vm.pop_typed), 1e-6
  end

  def test_i64_reinterpret_f64
    @vm.push_typed(WE.f64(1.0))
    exec_instr(0xBD)
    # IEEE 754: 1.0d = 0x3FF0000000000000
    assert_equal 0x3FF0000000000000, WE.to_u64(WE.as_i64(@vm.pop_typed))
  end

  def test_f64_reinterpret_i64
    @vm.push_typed(WE.i64(0x3FF0000000000000))
    exec_instr(0xBF)
    assert_in_delta 1.0, WE.as_f64(@vm.pop_typed), 1e-14
  end

  # ── Parametric Instructions ────────────────────────────────────────

  def test_drop
    @vm.push_typed(WE.i32(42))
    @vm.push_typed(WE.i32(99))
    exec_instr(0x1A)
    assert_equal 42, WE.as_i32(@vm.pop_typed)
  end

  def test_select_true
    @vm.push_typed(WE.i32(10))  # val1
    @vm.push_typed(WE.i32(20))  # val2
    @vm.push_typed(WE.i32(1))   # condition (true)
    exec_instr(0x1B)
    assert_equal 10, WE.as_i32(@vm.pop_typed)
  end

  def test_select_false
    @vm.push_typed(WE.i32(10))  # val1
    @vm.push_typed(WE.i32(20))  # val2
    @vm.push_typed(WE.i32(0))   # condition (false)
    exec_instr(0x1B)
    assert_equal 20, WE.as_i32(@vm.pop_typed)
  end

  # ── Variable Instructions ──────────────────────────────────────────

  def test_local_get
    ctx = default_context
    ctx[:typed_locals] = [WE.i32(42), WE.i32(99)]
    exec_instr(0x20, 1, ctx)
    assert_equal 99, WE.as_i32(@vm.pop_typed)
  end

  def test_local_set
    ctx = default_context
    ctx[:typed_locals] = [WE.i32(0), WE.i32(0)]
    @vm.push_typed(WE.i32(42))
    exec_instr(0x21, 0, ctx)
    assert_equal 42, WE.as_i32(ctx[:typed_locals][0])
  end

  def test_local_tee
    ctx = default_context
    ctx[:typed_locals] = [WE.i32(0)]
    @vm.push_typed(WE.i32(42))
    exec_instr(0x22, 0, ctx)
    # Value should still be on the stack
    assert_equal 42, WE.as_i32(@vm.peek_typed)
    # And stored in the local
    assert_equal 42, WE.as_i32(ctx[:typed_locals][0])
  end

  def test_global_get
    ctx = default_context
    ctx[:globals] = [WE.i32(100)]
    exec_instr(0x23, 0, ctx)
    assert_equal 100, WE.as_i32(@vm.pop_typed)
  end

  def test_global_set
    ctx = default_context
    ctx[:globals] = [WE.i32(0)]
    @vm.push_typed(WE.i32(55))
    exec_instr(0x24, 0, ctx)
    assert_equal 55, WE.as_i32(ctx[:globals][0])
  end

  # ── Memory Instructions ────────────────────────────────────────────

  def test_memory_size
    mem = WE::LinearMemory.new(3)
    ctx = default_context
    ctx[:memory] = mem
    exec_instr(0x3F, nil, ctx)
    assert_equal 3, WE.as_i32(@vm.pop_typed)
  end

  def test_memory_grow
    mem = WE::LinearMemory.new(1, 4)
    ctx = default_context
    ctx[:memory] = mem
    @vm.push_typed(WE.i32(2))
    exec_instr(0x40, nil, ctx)
    assert_equal 1, WE.as_i32(@vm.pop_typed) # old page count
    assert_equal 3, mem.page_count
  end

  def test_memory_no_memory_traps
    ctx = default_context
    ctx[:memory] = nil
    @vm.push_typed(WE.i32(0))
    assert_raises(TrapError) { exec_instr(0x3F, nil, ctx) }
  end

  # ── Control: unreachable and nop ───────────────────────────────────

  def test_unreachable_traps
    assert_raises(TrapError) { exec_instr(0x00) }
  end

  def test_nop_does_nothing
    stack_size = @vm.typed_stack.length
    exec_instr(0x01)
    assert_equal stack_size, @vm.typed_stack.length
  end
end
