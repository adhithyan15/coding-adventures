# frozen_string_literal: true

require_relative "test_helper"

# ==========================================================================
# Tests for the Source Map Chain — all four segments and composite queries
# ==========================================================================

class TestSourcePosition < Minitest::Test
  include CodingAdventures::CompilerSourceMap

  def test_to_s
    pos = SourcePosition.new(file: "hello.bf", line: 1, column: 3, length: 1)
    assert_equal "hello.bf:1:3 (len=1)", pos.to_s
  end

  def test_fields
    pos = SourcePosition.new(file: "test.bf", line: 5, column: 2, length: 3)
    assert_equal "test.bf", pos.file
    assert_equal 5, pos.line
    assert_equal 2, pos.column
    assert_equal 3, pos.length
  end
end

class TestSourceToAst < Minitest::Test
  include CodingAdventures::CompilerSourceMap

  def pos(file, line, col, len = 1)
    SourcePosition.new(file: file, line: line, column: col, length: len)
  end

  def test_add_and_lookup
    s = SourceToAst.new
    p = pos("test.bf", 1, 1)
    s.add(p, 42)
    result = s.lookup_by_node_id(42)
    assert_equal p, result
  end

  def test_lookup_missing_returns_nil
    s = SourceToAst.new
    assert_nil s.lookup_by_node_id(999)
  end

  def test_multiple_entries
    s = SourceToAst.new
    s.add(pos("a.bf", 1, 1), 0)
    s.add(pos("a.bf", 1, 2), 1)
    assert_equal pos("a.bf", 1, 1), s.lookup_by_node_id(0)
    assert_equal pos("a.bf", 1, 2), s.lookup_by_node_id(1)
  end

  def test_entries_array
    s = SourceToAst.new
    s.add(pos("x.bf", 1, 1), 7)
    assert_equal 1, s.entries.length
    assert_equal 7, s.entries[0].ast_node_id
  end
end

class TestAstToIr < Minitest::Test
  include CodingAdventures::CompilerSourceMap

  def test_add_and_lookup_by_ast
    a = AstToIr.new
    a.add(42, [10, 11, 12])
    assert_equal [10, 11, 12], a.lookup_by_ast_node_id(42)
  end

  def test_lookup_by_ast_missing_returns_nil
    a = AstToIr.new
    assert_nil a.lookup_by_ast_node_id(999)
  end

  def test_lookup_by_ir_id
    a = AstToIr.new
    a.add(42, [10, 11, 12])
    assert_equal 42, a.lookup_by_ir_id(10)
    assert_equal 42, a.lookup_by_ir_id(11)
    assert_equal 42, a.lookup_by_ir_id(12)
  end

  def test_lookup_by_ir_id_missing
    a = AstToIr.new
    a.add(42, [10, 11])
    assert_equal(-1, a.lookup_by_ir_id(99))
  end
end

class TestIrToIr < Minitest::Test
  include CodingAdventures::CompilerSourceMap

  def test_add_mapping_and_lookup
    m = IrToIr.new("contraction")
    m.add_mapping(7, [100])
    m.add_mapping(8, [100])
    assert_equal [100], m.lookup_by_original_id(7)
    assert_equal [100], m.lookup_by_original_id(8)
  end

  def test_add_deletion
    m = IrToIr.new("dead_store")
    m.add_deletion(5)
    assert_nil m.lookup_by_original_id(5)
    assert m.deleted[5]
  end

  def test_lookup_by_new_id
    m = IrToIr.new("contraction")
    m.add_mapping(7, [100])
    m.add_mapping(8, [100])
    original = m.lookup_by_new_id(100)
    assert [7, 8].include?(original), "expected 7 or 8, got #{original}"
  end

  def test_lookup_by_new_id_missing
    m = IrToIr.new("identity")
    assert_equal(-1, m.lookup_by_new_id(999))
  end

  def test_pass_name
    m = IrToIr.new("clear_loop")
    assert_equal "clear_loop", m.pass_name
  end
end

class TestIrToMachineCode < Minitest::Test
  include CodingAdventures::CompilerSourceMap

  def test_add_and_lookup_by_ir_id
    mc = IrToMachineCode.new
    mc.add(5, 0x14, 4)
    offset, length = mc.lookup_by_ir_id(5)
    assert_equal 0x14, offset
    assert_equal 4, length
  end

  def test_lookup_by_ir_id_missing
    mc = IrToMachineCode.new
    offset, length = mc.lookup_by_ir_id(999)
    assert_equal(-1, offset)
    assert_equal 0, length
  end

  def test_lookup_by_mc_offset_exact
    mc = IrToMachineCode.new
    mc.add(5, 0x10, 4)
    assert_equal 5, mc.lookup_by_mc_offset(0x10)
    assert_equal 5, mc.lookup_by_mc_offset(0x13)
  end

  def test_lookup_by_mc_offset_out_of_range
    mc = IrToMachineCode.new
    mc.add(5, 0x10, 4)
    assert_equal(-1, mc.lookup_by_mc_offset(0x14))  # one past end
    assert_equal(-1, mc.lookup_by_mc_offset(0x0F))  # one before start
  end

  def test_lookup_by_mc_offset_missing
    mc = IrToMachineCode.new
    assert_equal(-1, mc.lookup_by_mc_offset(0))
  end
end

class TestSourceMapChain < Minitest::Test
  include CodingAdventures::CompilerSourceMap

  # Build a complete chain for use across tests
  def build_chain
    chain = SourceMapChain.new
    pos = SourcePosition.new(file: "test.bf", line: 1, column: 1, length: 1)
    chain.source_to_ast.add(pos, 0)
    chain.ast_to_ir.add(0, [5, 6, 7, 8])

    mc = IrToMachineCode.new
    mc.add(5, 0, 4)
    mc.add(6, 4, 4)
    mc.add(7, 8, 4)
    mc.add(8, 12, 4)
    chain.ir_to_machine_code = mc

    chain
  end

  def test_source_to_mc_basic
    chain = build_chain
    pos = SourcePosition.new(file: "test.bf", line: 1, column: 1, length: 1)
    results = chain.source_to_mc(pos)
    assert_equal 4, results.length
    offsets = results.map(&:mc_offset).sort
    assert_equal [0, 4, 8, 12], offsets
  end

  def test_source_to_mc_no_machine_code
    chain = SourceMapChain.new
    pos = SourcePosition.new(file: "test.bf", line: 1, column: 1, length: 1)
    assert_equal [], chain.source_to_mc(pos)
  end

  def test_source_to_mc_no_match
    chain = build_chain
    pos = SourcePosition.new(file: "other.bf", line: 99, column: 99, length: 1)
    assert_equal [], chain.source_to_mc(pos)
  end

  def test_mc_to_source_basic
    chain = build_chain
    result = chain.mc_to_source(0)
    refute_nil result
    assert_equal "test.bf", result.file
    assert_equal 1, result.line
    assert_equal 1, result.column
  end

  def test_mc_to_source_no_machine_code
    chain = SourceMapChain.new
    assert_nil chain.mc_to_source(0)
  end

  def test_mc_to_source_no_match
    chain = build_chain
    assert_nil chain.mc_to_source(9999)
  end

  def test_add_optimizer_pass
    chain = SourceMapChain.new
    pass = IrToIr.new("identity")
    chain.add_optimizer_pass(pass)
    assert_equal 1, chain.ir_to_ir.length
  end

  def test_source_to_mc_through_optimizer_pass
    chain = SourceMapChain.new
    pos = SourcePosition.new(file: "test.bf", line: 1, column: 1, length: 1)
    chain.source_to_ast.add(pos, 0)
    chain.ast_to_ir.add(0, [5])

    # Optimiser pass: 5 → 100
    pass = IrToIr.new("contraction")
    pass.add_mapping(5, [100])
    chain.add_optimizer_pass(pass)

    mc = IrToMachineCode.new
    mc.add(100, 0, 4)
    chain.ir_to_machine_code = mc

    results = chain.source_to_mc(pos)
    assert_equal 1, results.length
    assert_equal 0, results[0].mc_offset
  end

  def test_mc_to_source_through_optimizer_pass
    chain = SourceMapChain.new
    pos = SourcePosition.new(file: "test.bf", line: 1, column: 1, length: 1)
    chain.source_to_ast.add(pos, 0)
    chain.ast_to_ir.add(0, [5])

    pass = IrToIr.new("contraction")
    pass.add_mapping(5, [100])
    chain.add_optimizer_pass(pass)

    mc = IrToMachineCode.new
    mc.add(100, 0, 4)
    chain.ir_to_machine_code = mc

    result = chain.mc_to_source(2)  # offset 2 is inside the 4-byte block at 0
    refute_nil result
    assert_equal "test.bf", result.file
  end

  def test_source_to_mc_optimizer_deleted_instruction
    chain = SourceMapChain.new
    pos = SourcePosition.new(file: "test.bf", line: 1, column: 1, length: 1)
    chain.source_to_ast.add(pos, 0)
    chain.ast_to_ir.add(0, [5])

    pass = IrToIr.new("dead_store")
    pass.add_deletion(5)
    chain.add_optimizer_pass(pass)

    mc = IrToMachineCode.new
    chain.ir_to_machine_code = mc

    results = chain.source_to_mc(pos)
    assert_equal [], results
  end
end
