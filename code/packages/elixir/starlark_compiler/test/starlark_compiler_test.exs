defmodule CodingAdventures.StarlarkCompilerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.StarlarkCompiler

  test "keeps Rust-compatible byte values" do
    assert StarlarkCompiler.op_byte(:load_const) == 0x01
    assert StarlarkCompiler.op_byte(:add) == 0x20
    assert StarlarkCompiler.op_byte(:cmp_eq) == 0x30
    assert StarlarkCompiler.op_byte(:jump) == 0x40
    assert StarlarkCompiler.op_byte(:make_function) == 0x50
    assert StarlarkCompiler.op_byte(:build_list) == 0x60
    assert StarlarkCompiler.op_byte(:load_subscript) == 0x70
    assert StarlarkCompiler.op_byte(:get_iter) == 0x80
    assert StarlarkCompiler.op_byte(:load_module) == 0x90
    assert StarlarkCompiler.op_byte(:print) == 0xA0
    assert StarlarkCompiler.op_byte(:halt) == 0xFF
    assert StarlarkCompiler.op_byte(:missing) == nil
  end

  test "round trips every defined opcode" do
    for op <- StarlarkCompiler.all_ops() do
      assert StarlarkCompiler.op_from_byte(StarlarkCompiler.op_byte(op)) == op
    end

    assert StarlarkCompiler.op_from_byte(0xEE) == nil
  end

  test "classifies opcodes by high nibble" do
    assert StarlarkCompiler.op_category(:load_const) == :stack
    assert StarlarkCompiler.op_category(:load_name) == :variable
    assert StarlarkCompiler.op_category(:r_shift) == :arithmetic
    assert StarlarkCompiler.op_category(:not) == :comparison
    assert StarlarkCompiler.op_category(:jump_if_true) == :control_flow
    assert StarlarkCompiler.op_category(:return) == :function
    assert StarlarkCompiler.op_category(:dict_set) == :collection
    assert StarlarkCompiler.op_category(:load_slice) == :subscript_attribute
    assert StarlarkCompiler.op_category(:unpack_sequence) == :iteration
    assert StarlarkCompiler.op_category(:import_from) == :module
    assert StarlarkCompiler.op_category(:print) == :io
    assert StarlarkCompiler.op_category(:halt) == :vm_control
    assert StarlarkCompiler.op_category(:missing) == nil
  end

  test "maps binary operators" do
    map = StarlarkCompiler.binary_op_map()

    assert map["+"] == :add
    assert map["-"] == :sub
    assert map["*"] == :mul
    assert map["/"] == :div
    assert map["//"] == :floor_div
    assert map["%"] == :mod
    assert map["**"] == :power
    assert map["&"] == :bit_and
    assert map["|"] == :bit_or
    assert map["^"] == :bit_xor
    assert map["<<"] == :l_shift
    assert map[">>"] == :r_shift
    assert StarlarkCompiler.binary_opcode("+") == :add
    assert StarlarkCompiler.binary_opcode("???") == nil
    assert map_size(map) == 12
  end

  test "maps comparison operators" do
    map = StarlarkCompiler.compare_op_map()

    assert map["=="] == :cmp_eq
    assert map["!="] == :cmp_ne
    assert map["<"] == :cmp_lt
    assert map[">"] == :cmp_gt
    assert map["<="] == :cmp_le
    assert map[">="] == :cmp_ge
    assert map["in"] == :cmp_in
    assert map["not in"] == :cmp_not_in
    assert StarlarkCompiler.compare_opcode("in") == :cmp_in
    assert StarlarkCompiler.compare_opcode("contains") == nil
    assert map_size(map) == 8
  end

  test "maps augmented assignment and unary operators" do
    augmented = StarlarkCompiler.augmented_assign_map()

    assert augmented["+="] == :add
    assert augmented["-="] == :sub
    assert augmented["*="] == :mul
    assert augmented["/="] == :div
    assert augmented["//="] == :floor_div
    assert augmented["%="] == :mod
    assert augmented["&="] == :bit_and
    assert augmented["|="] == :bit_or
    assert augmented["^="] == :bit_xor
    assert augmented["<<="] == :l_shift
    assert augmented[">>="] == :r_shift
    assert augmented["**="] == :power
    assert StarlarkCompiler.augmented_assign_opcode("**=") == :power
    assert StarlarkCompiler.augmented_assign_opcode("=") == nil
    assert map_size(augmented) == 12

    unary = StarlarkCompiler.unary_op_map()
    assert unary["-"] == :negate
    assert unary["~"] == :bit_not
    assert StarlarkCompiler.unary_opcode("~") == :bit_not
    assert StarlarkCompiler.unary_opcode("not") == nil
    assert map_size(unary) == 2
  end
end
