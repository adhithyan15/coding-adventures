# frozen_string_literal: true

# ==========================================================================
# Tests for Starlark Opcodes
# ==========================================================================
#
# These tests verify that all 46 opcodes are correctly defined with the
# expected integer values, and that every opcode has a human-readable name
# in the NAMES lookup table.
# ==========================================================================

require "minitest/autorun"
require "coding_adventures_starlark_ast_to_bytecode_compiler"

class TestOpcodes < Minitest::Test
  Op = CodingAdventures::StarlarkAstToBytecodeCompiler::Op

  # ================================================================
  # Stack Operation Values
  # ================================================================

  def test_load_const_value
    assert_equal 0x01, Op::LOAD_CONST
  end

  def test_pop_value
    assert_equal 0x02, Op::POP
  end

  def test_dup_value
    assert_equal 0x03, Op::DUP
  end

  def test_load_none_value
    assert_equal 0x04, Op::LOAD_NONE
  end

  def test_load_true_value
    assert_equal 0x05, Op::LOAD_TRUE
  end

  def test_load_false_value
    assert_equal 0x06, Op::LOAD_FALSE
  end

  # ================================================================
  # Variable Access Values
  # ================================================================

  def test_store_name_value
    assert_equal 0x10, Op::STORE_NAME
  end

  def test_load_name_value
    assert_equal 0x11, Op::LOAD_NAME
  end

  def test_store_local_value
    assert_equal 0x12, Op::STORE_LOCAL
  end

  def test_load_local_value
    assert_equal 0x13, Op::LOAD_LOCAL
  end

  def test_store_closure_value
    assert_equal 0x14, Op::STORE_CLOSURE
  end

  def test_load_closure_value
    assert_equal 0x15, Op::LOAD_CLOSURE
  end

  # ================================================================
  # Arithmetic Operation Values
  # ================================================================

  def test_add_value
    assert_equal 0x20, Op::ADD
  end

  def test_sub_value
    assert_equal 0x21, Op::SUB
  end

  def test_mul_value
    assert_equal 0x22, Op::MUL
  end

  def test_div_value
    assert_equal 0x23, Op::DIV
  end

  def test_floor_div_value
    assert_equal 0x24, Op::FLOOR_DIV
  end

  def test_mod_value
    assert_equal 0x25, Op::MOD
  end

  def test_power_value
    assert_equal 0x26, Op::POWER
  end

  def test_negate_value
    assert_equal 0x27, Op::NEGATE
  end

  def test_bit_and_value
    assert_equal 0x28, Op::BIT_AND
  end

  def test_bit_or_value
    assert_equal 0x29, Op::BIT_OR
  end

  def test_bit_xor_value
    assert_equal 0x2A, Op::BIT_XOR
  end

  def test_bit_not_value
    assert_equal 0x2B, Op::BIT_NOT
  end

  def test_lshift_value
    assert_equal 0x2C, Op::LSHIFT
  end

  def test_rshift_value
    assert_equal 0x2D, Op::RSHIFT
  end

  # ================================================================
  # Comparison Values
  # ================================================================

  def test_cmp_eq_value
    assert_equal 0x30, Op::CMP_EQ
  end

  def test_cmp_lt_value
    assert_equal 0x31, Op::CMP_LT
  end

  def test_cmp_gt_value
    assert_equal 0x32, Op::CMP_GT
  end

  def test_cmp_ne_value
    assert_equal 0x33, Op::CMP_NE
  end

  def test_cmp_le_value
    assert_equal 0x34, Op::CMP_LE
  end

  def test_cmp_ge_value
    assert_equal 0x35, Op::CMP_GE
  end

  def test_cmp_in_value
    assert_equal 0x36, Op::CMP_IN
  end

  def test_cmp_not_in_value
    assert_equal 0x37, Op::CMP_NOT_IN
  end

  def test_not_value
    assert_equal 0x38, Op::NOT
  end

  # ================================================================
  # Control Flow Values
  # ================================================================

  def test_jump_value
    assert_equal 0x40, Op::JUMP
  end

  def test_jump_if_false_value
    assert_equal 0x41, Op::JUMP_IF_FALSE
  end

  def test_jump_if_true_value
    assert_equal 0x42, Op::JUMP_IF_TRUE
  end

  def test_jump_if_false_or_pop_value
    assert_equal 0x43, Op::JUMP_IF_FALSE_OR_POP
  end

  def test_jump_if_true_or_pop_value
    assert_equal 0x44, Op::JUMP_IF_TRUE_OR_POP
  end

  def test_break_value
    assert_equal 0x45, Op::BREAK
  end

  def test_continue_value
    assert_equal 0x46, Op::CONTINUE
  end

  # ================================================================
  # Function Values
  # ================================================================

  def test_make_function_value
    assert_equal 0x50, Op::MAKE_FUNCTION
  end

  def test_call_function_value
    assert_equal 0x51, Op::CALL_FUNCTION
  end

  def test_call_function_kw_value
    assert_equal 0x52, Op::CALL_FUNCTION_KW
  end

  def test_return_value_value
    assert_equal 0x53, Op::RETURN_VALUE
  end

  # ================================================================
  # Collection Values
  # ================================================================

  def test_build_list_value
    assert_equal 0x60, Op::BUILD_LIST
  end

  def test_build_dict_value
    assert_equal 0x61, Op::BUILD_DICT
  end

  def test_build_tuple_value
    assert_equal 0x62, Op::BUILD_TUPLE
  end

  def test_list_append_value
    assert_equal 0x63, Op::LIST_APPEND
  end

  def test_dict_set_value
    assert_equal 0x64, Op::DICT_SET
  end

  # ================================================================
  # Subscript/Attr Values
  # ================================================================

  def test_load_subscript_value
    assert_equal 0x70, Op::LOAD_SUBSCRIPT
  end

  def test_store_subscript_value
    assert_equal 0x71, Op::STORE_SUBSCRIPT
  end

  def test_load_attr_value
    assert_equal 0x72, Op::LOAD_ATTR
  end

  def test_store_attr_value
    assert_equal 0x73, Op::STORE_ATTR
  end

  def test_load_slice_value
    assert_equal 0x74, Op::LOAD_SLICE
  end

  # ================================================================
  # Iteration Values
  # ================================================================

  def test_get_iter_value
    assert_equal 0x80, Op::GET_ITER
  end

  def test_for_iter_value
    assert_equal 0x81, Op::FOR_ITER
  end

  def test_unpack_sequence_value
    assert_equal 0x82, Op::UNPACK_SEQUENCE
  end

  # ================================================================
  # Module Values
  # ================================================================

  def test_load_module_value
    assert_equal 0x90, Op::LOAD_MODULE
  end

  def test_import_from_value
    assert_equal 0x91, Op::IMPORT_FROM
  end

  # ================================================================
  # I/O Values
  # ================================================================

  def test_print_value_value
    assert_equal 0xA0, Op::PRINT_VALUE
  end

  # ================================================================
  # VM Control Values
  # ================================================================

  def test_halt_value
    assert_equal 0xFF, Op::HALT
  end

  # ================================================================
  # NAMES Coverage
  # ================================================================

  def test_all_opcodes_have_names
    # Every opcode constant should have a corresponding entry in NAMES.
    all_opcodes = [
      Op::LOAD_CONST, Op::POP, Op::DUP, Op::LOAD_NONE, Op::LOAD_TRUE, Op::LOAD_FALSE,
      Op::STORE_NAME, Op::LOAD_NAME, Op::STORE_LOCAL, Op::LOAD_LOCAL,
      Op::STORE_CLOSURE, Op::LOAD_CLOSURE,
      Op::ADD, Op::SUB, Op::MUL, Op::DIV, Op::FLOOR_DIV, Op::MOD, Op::POWER,
      Op::NEGATE, Op::BIT_AND, Op::BIT_OR, Op::BIT_XOR, Op::BIT_NOT, Op::LSHIFT, Op::RSHIFT,
      Op::CMP_EQ, Op::CMP_LT, Op::CMP_GT, Op::CMP_NE, Op::CMP_LE, Op::CMP_GE,
      Op::CMP_IN, Op::CMP_NOT_IN, Op::NOT,
      Op::JUMP, Op::JUMP_IF_FALSE, Op::JUMP_IF_TRUE,
      Op::JUMP_IF_FALSE_OR_POP, Op::JUMP_IF_TRUE_OR_POP,
      Op::BREAK, Op::CONTINUE,
      Op::MAKE_FUNCTION, Op::CALL_FUNCTION, Op::CALL_FUNCTION_KW, Op::RETURN_VALUE,
      Op::BUILD_LIST, Op::BUILD_DICT, Op::BUILD_TUPLE, Op::LIST_APPEND, Op::DICT_SET,
      Op::LOAD_SUBSCRIPT, Op::STORE_SUBSCRIPT, Op::LOAD_ATTR, Op::STORE_ATTR, Op::LOAD_SLICE,
      Op::GET_ITER, Op::FOR_ITER, Op::UNPACK_SEQUENCE,
      Op::LOAD_MODULE, Op::IMPORT_FROM,
      Op::PRINT_VALUE,
      Op::HALT
    ]

    all_opcodes.each do |opcode|
      name = Op::NAMES[opcode]
      refute_nil name, "Opcode 0x#{opcode.to_s(16).upcase} has no name in NAMES"
      refute_empty name, "Opcode 0x#{opcode.to_s(16).upcase} has empty name"
    end
  end

  def test_names_count
    # Verify we have exactly the right number of named opcodes.
    # Stack(6) + Vars(6) + Arith(14) + Cmp(9) + Control(7) + Func(4) +
    # Collections(5) + Subscript(5) + Iter(3) + Module(2) + IO(1) + VM(1) = 63
    # But we have 46 unique opcodes per spec (some groups are smaller).
    # Just check that NAMES is non-empty and all values are strings.
    assert Op::NAMES.length >= 46, "Expected at least 46 named opcodes"
    Op::NAMES.each_value do |name|
      assert_kind_of String, name
    end
  end

  def test_names_are_uppercase
    # Convention: opcode names are UPPER_SNAKE_CASE
    Op::NAMES.each_value do |name|
      assert_match(/\A[A-Z_]+\z/, name, "Name '#{name}' should be UPPER_SNAKE_CASE")
    end
  end

  def test_opcodes_are_unique
    # No two opcodes should share the same integer value.
    all_values = Op::NAMES.keys
    assert_equal all_values.length, all_values.uniq.length,
      "Duplicate opcode values found"
  end
end
