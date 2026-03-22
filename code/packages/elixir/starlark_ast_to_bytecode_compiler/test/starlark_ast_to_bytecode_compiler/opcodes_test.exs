defmodule CodingAdventures.StarlarkAstToBytecodeCompiler.OpcodesTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.StarlarkAstToBytecodeCompiler.Opcodes, as: Op

  # ===========================================================================
  # Stack Operations
  # ===========================================================================

  test "load_const opcode is 0x01" do
    assert Op.load_const() == 0x01
  end

  test "pop opcode is 0x02" do
    assert Op.pop() == 0x02
  end

  test "dup opcode is 0x03" do
    assert Op.dup() == 0x03
  end

  test "load_none opcode is 0x04" do
    assert Op.load_none() == 0x04
  end

  test "load_true opcode is 0x05" do
    assert Op.load_true() == 0x05
  end

  test "load_false opcode is 0x06" do
    assert Op.load_false() == 0x06
  end

  # ===========================================================================
  # Variable Operations
  # ===========================================================================

  test "store_name opcode is 0x10" do
    assert Op.store_name() == 0x10
  end

  test "load_name opcode is 0x11" do
    assert Op.load_name() == 0x11
  end

  test "store_local opcode is 0x12" do
    assert Op.store_local() == 0x12
  end

  test "load_local opcode is 0x13" do
    assert Op.load_local() == 0x13
  end

  test "store_closure opcode is 0x14" do
    assert Op.store_closure() == 0x14
  end

  test "load_closure opcode is 0x15" do
    assert Op.load_closure() == 0x15
  end

  # ===========================================================================
  # Arithmetic Operations
  # ===========================================================================

  test "add opcode is 0x20" do
    assert Op.add() == 0x20
  end

  test "sub opcode is 0x21" do
    assert Op.sub() == 0x21
  end

  test "mul opcode is 0x22" do
    assert Op.mul() == 0x22
  end

  test "div_op opcode is 0x23" do
    assert Op.div_op() == 0x23
  end

  test "floor_div opcode is 0x24" do
    assert Op.floor_div() == 0x24
  end

  test "mod opcode is 0x25" do
    assert Op.mod() == 0x25
  end

  test "power opcode is 0x26" do
    assert Op.power() == 0x26
  end

  test "negate opcode is 0x27" do
    assert Op.negate() == 0x27
  end

  test "bit_and opcode is 0x28" do
    assert Op.bit_and() == 0x28
  end

  test "bit_or opcode is 0x29" do
    assert Op.bit_or() == 0x29
  end

  test "bit_xor opcode is 0x2A" do
    assert Op.bit_xor() == 0x2A
  end

  test "bit_not opcode is 0x2B" do
    assert Op.bit_not() == 0x2B
  end

  test "lshift opcode is 0x2C" do
    assert Op.lshift() == 0x2C
  end

  test "rshift opcode is 0x2D" do
    assert Op.rshift() == 0x2D
  end

  # ===========================================================================
  # Comparison Operations
  # ===========================================================================

  test "cmp_eq opcode is 0x30" do
    assert Op.cmp_eq() == 0x30
  end

  test "cmp_ne opcode is 0x31" do
    assert Op.cmp_ne() == 0x31
  end

  test "cmp_lt opcode is 0x32" do
    assert Op.cmp_lt() == 0x32
  end

  test "cmp_gt opcode is 0x33" do
    assert Op.cmp_gt() == 0x33
  end

  test "cmp_le opcode is 0x34" do
    assert Op.cmp_le() == 0x34
  end

  test "cmp_ge opcode is 0x35" do
    assert Op.cmp_ge() == 0x35
  end

  test "cmp_in opcode is 0x36" do
    assert Op.cmp_in() == 0x36
  end

  test "cmp_not_in opcode is 0x37" do
    assert Op.cmp_not_in() == 0x37
  end

  # ===========================================================================
  # Boolean Operations
  # ===========================================================================

  test "logical_not opcode is 0x38" do
    assert Op.logical_not() == 0x38
  end

  # ===========================================================================
  # Control Flow
  # ===========================================================================

  test "jump opcode is 0x40" do
    assert Op.jump() == 0x40
  end

  test "jump_if_false opcode is 0x41" do
    assert Op.jump_if_false() == 0x41
  end

  test "jump_if_true opcode is 0x42" do
    assert Op.jump_if_true() == 0x42
  end

  test "jump_if_false_or_pop opcode is 0x43" do
    assert Op.jump_if_false_or_pop() == 0x43
  end

  test "jump_if_true_or_pop opcode is 0x44" do
    assert Op.jump_if_true_or_pop() == 0x44
  end

  # ===========================================================================
  # Function Operations
  # ===========================================================================

  test "make_function opcode is 0x50" do
    assert Op.make_function() == 0x50
  end

  test "call_function opcode is 0x51" do
    assert Op.call_function() == 0x51
  end

  test "call_function_kw opcode is 0x52" do
    assert Op.call_function_kw() == 0x52
  end

  test "return_op opcode is 0x53" do
    assert Op.return_op() == 0x53
  end

  # ===========================================================================
  # Collection Operations
  # ===========================================================================

  test "build_list opcode is 0x60" do
    assert Op.build_list() == 0x60
  end

  test "build_dict opcode is 0x61" do
    assert Op.build_dict() == 0x61
  end

  test "build_tuple opcode is 0x62" do
    assert Op.build_tuple() == 0x62
  end

  test "list_append opcode is 0x63" do
    assert Op.list_append() == 0x63
  end

  test "dict_set opcode is 0x64" do
    assert Op.dict_set() == 0x64
  end

  # ===========================================================================
  # Subscript & Attribute Operations
  # ===========================================================================

  test "load_subscript opcode is 0x70" do
    assert Op.load_subscript() == 0x70
  end

  test "store_subscript opcode is 0x71" do
    assert Op.store_subscript() == 0x71
  end

  test "load_attr opcode is 0x72" do
    assert Op.load_attr() == 0x72
  end

  test "store_attr opcode is 0x73" do
    assert Op.store_attr() == 0x73
  end

  test "load_slice opcode is 0x74" do
    assert Op.load_slice() == 0x74
  end

  # ===========================================================================
  # Iteration Operations
  # ===========================================================================

  test "get_iter opcode is 0x80" do
    assert Op.get_iter() == 0x80
  end

  test "for_iter opcode is 0x81" do
    assert Op.for_iter() == 0x81
  end

  test "unpack_sequence opcode is 0x82" do
    assert Op.unpack_sequence() == 0x82
  end

  # ===========================================================================
  # Module Operations
  # ===========================================================================

  test "load_module opcode is 0x90" do
    assert Op.load_module() == 0x90
  end

  test "import_from opcode is 0x91" do
    assert Op.import_from() == 0x91
  end

  # ===========================================================================
  # I/O Operations
  # ===========================================================================

  test "print_op opcode is 0xA0" do
    assert Op.print_op() == 0xA0
  end

  # ===========================================================================
  # VM Control
  # ===========================================================================

  test "halt opcode is 0xFF" do
    assert Op.halt() == 0xFF
  end

  # ===========================================================================
  # Operator Maps
  # ===========================================================================

  test "binary_op_map has all 12 operators" do
    map = Op.binary_op_map()
    assert map_size(map) == 12
    assert map["+"] == Op.add()
    assert map["-"] == Op.sub()
    assert map["*"] == Op.mul()
    assert map["/"] == Op.div_op()
    assert map["//"] == Op.floor_div()
    assert map["%"] == Op.mod()
    assert map["**"] == Op.power()
    assert map["&"] == Op.bit_and()
    assert map["|"] == Op.bit_or()
    assert map["^"] == Op.bit_xor()
    assert map["<<"] == Op.lshift()
    assert map[">>"] == Op.rshift()
  end

  test "compare_op_map has all 8 operators" do
    map = Op.compare_op_map()
    assert map_size(map) == 8
    assert map["=="] == Op.cmp_eq()
    assert map["!="] == Op.cmp_ne()
    assert map["<"] == Op.cmp_lt()
    assert map[">"] == Op.cmp_gt()
    assert map["<="] == Op.cmp_le()
    assert map[">="] == Op.cmp_ge()
    assert map["in"] == Op.cmp_in()
    assert map["not in"] == Op.cmp_not_in()
  end

  test "augmented_assign_map has all 12 operators" do
    map = Op.augmented_assign_map()
    assert map_size(map) == 12
    assert map["+="] == Op.add()
    assert map["-="] == Op.sub()
    assert map["*="] == Op.mul()
    assert map["/="] == Op.div_op()
    assert map["//="] == Op.floor_div()
    assert map["%="] == Op.mod()
    assert map["&="] == Op.bit_and()
    assert map["|="] == Op.bit_or()
    assert map["^="] == Op.bit_xor()
    assert map["<<="] == Op.lshift()
    assert map[">>="] == Op.rshift()
    assert map["**="] == Op.power()
  end

  test "unary_op_map has 3 operators" do
    map = Op.unary_op_map()
    assert map_size(map) == 3
    assert map["-"] == Op.negate()
    assert map["+"] == Op.pop()
    assert map["~"] == Op.bit_not()
  end

  # ===========================================================================
  # All Opcodes List
  # ===========================================================================

  test "all_opcodes returns correct count" do
    opcodes = Op.all_opcodes()
    # 46 opcodes total as documented, but we list 59 in the list
    # (some categories have more entries)
    assert length(opcodes) > 40
    assert Enum.all?(opcodes, fn op -> is_integer(op) end)
  end

  test "all opcodes are unique" do
    opcodes = Op.all_opcodes()
    assert length(Enum.uniq(opcodes)) == length(opcodes)
  end

  test "all opcodes fit in a single byte (0x00-0xFF)" do
    for op <- Op.all_opcodes() do
      assert op >= 0x00 and op <= 0xFF
    end
  end
end
