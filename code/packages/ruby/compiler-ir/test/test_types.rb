# frozen_string_literal: true

require_relative "test_helper"

# ==========================================================================
# Tests for IR Types — IrRegister, IrImmediate, IrLabel, IrInstruction,
#                      IrDataDecl, IrProgram, IDGenerator
# ==========================================================================

class TestIrRegister < Minitest::Test
  include CodingAdventures::CompilerIr

  def test_to_s_zero
    assert_equal "v0", IrRegister.new(0).to_s
  end

  def test_to_s_nonzero
    assert_equal "v5", IrRegister.new(5).to_s
  end

  def test_to_s_large_index
    assert_equal "v65535", IrRegister.new(65535).to_s
  end

  def test_immutable
    # Data.define objects are frozen
    r = IrRegister.new(0)
    assert r.frozen?
  end

  def test_equality
    assert_equal IrRegister.new(3), IrRegister.new(3)
    refute_equal IrRegister.new(3), IrRegister.new(4)
  end
end

class TestIrImmediate < Minitest::Test
  include CodingAdventures::CompilerIr

  def test_to_s_positive
    assert_equal "42", IrImmediate.new(42).to_s
  end

  def test_to_s_negative
    assert_equal "-1", IrImmediate.new(-1).to_s
  end

  def test_to_s_zero
    assert_equal "0", IrImmediate.new(0).to_s
  end

  def test_to_s_255
    assert_equal "255", IrImmediate.new(255).to_s
  end

  def test_immutable
    assert IrImmediate.new(0).frozen?
  end

  def test_equality
    assert_equal IrImmediate.new(42), IrImmediate.new(42)
    refute_equal IrImmediate.new(42), IrImmediate.new(43)
  end
end

class TestIrLabel < Minitest::Test
  include CodingAdventures::CompilerIr

  def test_to_s
    assert_equal "_start", IrLabel.new("_start").to_s
  end

  def test_to_s_loop_label
    assert_equal "loop_0_end", IrLabel.new("loop_0_end").to_s
  end

  def test_immutable
    assert IrLabel.new("x").frozen?
  end

  def test_equality
    assert_equal IrLabel.new("tape"), IrLabel.new("tape")
    refute_equal IrLabel.new("tape"), IrLabel.new("other")
  end
end

class TestIrInstruction < Minitest::Test
  include CodingAdventures::CompilerIr

  def test_fields
    instr = IrInstruction.new(IrOp::ADD_IMM,
                              [IrRegister.new(1), IrRegister.new(1), IrImmediate.new(1)],
                              3)
    assert_equal IrOp::ADD_IMM, instr.opcode
    assert_equal 3, instr.operands.length
    assert_equal 3, instr.id
  end

  def test_empty_operands
    instr = IrInstruction.new(IrOp::HALT, [], 0)
    assert_equal [], instr.operands
  end
end

class TestIrDataDecl < Minitest::Test
  include CodingAdventures::CompilerIr

  def test_fields
    decl = IrDataDecl.new("tape", 30_000, 0)
    assert_equal "tape", decl.label
    assert_equal 30_000, decl.size
    assert_equal 0, decl.init
  end
end

class TestIrProgram < Minitest::Test
  include CodingAdventures::CompilerIr

  def test_default_version
    prog = IrProgram.new("_start")
    assert_equal 1, prog.version
  end

  def test_entry_label
    prog = IrProgram.new("my_entry")
    assert_equal "my_entry", prog.entry_label
  end

  def test_empty_instructions
    prog = IrProgram.new("_start")
    assert_equal [], prog.instructions
  end

  def test_add_instruction
    prog = IrProgram.new("_start")
    instr = IrInstruction.new(IrOp::HALT, [], 0)
    prog.add_instruction(instr)
    assert_equal 1, prog.instructions.length
    assert_equal IrOp::HALT, prog.instructions[0].opcode
  end

  def test_add_data
    prog = IrProgram.new("_start")
    decl = IrDataDecl.new("tape", 30_000, 0)
    prog.add_data(decl)
    assert_equal 1, prog.data.length
    assert_equal "tape", prog.data[0].label
  end

  def test_multiple_instructions_ordered
    prog = IrProgram.new("_start")
    prog.add_instruction(IrInstruction.new(IrOp::LOAD_IMM, [IrRegister.new(0), IrImmediate.new(0)], 0))
    prog.add_instruction(IrInstruction.new(IrOp::HALT, [], 1))
    assert_equal IrOp::LOAD_IMM, prog.instructions[0].opcode
    assert_equal IrOp::HALT, prog.instructions[1].opcode
  end
end

class TestIDGenerator < Minitest::Test
  include CodingAdventures::CompilerIr

  def test_starts_at_zero
    gen = IDGenerator.new
    assert_equal 0, gen.next
  end

  def test_increments
    gen = IDGenerator.new
    assert_equal 0, gen.next
    assert_equal 1, gen.next
    assert_equal 2, gen.next
  end

  def test_current_before_next
    gen = IDGenerator.new
    assert_equal 0, gen.current
    gen.next
    assert_equal 1, gen.current
  end

  def test_start_from
    gen = IDGenerator.new(10)
    assert_equal 10, gen.next
    assert_equal 11, gen.next
  end

  def test_current_does_not_advance
    gen = IDGenerator.new
    assert_equal 0, gen.current
    assert_equal 0, gen.current
    assert_equal 0, gen.next
    assert_equal 1, gen.current
  end
end
