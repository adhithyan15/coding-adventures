# frozen_string_literal: true

require_relative "test_helper"

# ==========================================================================
# Tests for IrPrinter — IrProgram → canonical text
# ==========================================================================
#
# Every format rule is verified:
#   - .version N is first
#   - .data declarations appear with a blank line before
#   - .entry appears after .data
#   - Labels get their own unindented line with trailing ":"
#   - Regular instructions are indented and have ; #ID comments
#   - COMMENT instructions emit as "  ; text"
# ==========================================================================

class TestIrPrinter < Minitest::Test
  include CodingAdventures::CompilerIr

  # Build a minimal empty program for baseline tests
  def minimal_program
    prog = IrProgram.new("_start")
    prog.add_instruction(IrInstruction.new(IrOp::LABEL, [IrLabel.new("_start")], -1))
    prog.add_instruction(IrInstruction.new(IrOp::HALT, [], 0))
    prog
  end

  def test_contains_version
    text = IrPrinter.print(minimal_program)
    assert_match(/^\.version 1$/, text)
  end

  def test_version_is_first_line
    text = IrPrinter.print(minimal_program)
    first_line = text.lines.first.chomp
    assert_equal ".version 1", first_line
  end

  def test_contains_entry
    text = IrPrinter.print(minimal_program)
    assert_includes text, ".entry _start"
  end

  def test_label_on_own_line_with_colon
    text = IrPrinter.print(minimal_program)
    assert_match(/^_start:$/, text)
  end

  def test_halt_indented
    text = IrPrinter.print(minimal_program)
    assert_match(/^  HALT/, text)
  end

  def test_halt_has_id_comment
    text = IrPrinter.print(minimal_program)
    assert_match(/HALT.*; #0/, text)
  end

  def test_data_declaration
    prog = IrProgram.new("_start")
    prog.add_data(IrDataDecl.new("tape", 30_000, 0))
    prog.add_instruction(IrInstruction.new(IrOp::HALT, [], 0))
    text = IrPrinter.print(prog)
    assert_includes text, ".data tape 30000 0"
  end

  def test_data_before_entry
    prog = IrProgram.new("_start")
    prog.add_data(IrDataDecl.new("tape", 30_000, 0))
    prog.add_instruction(IrInstruction.new(IrOp::HALT, [], 0))
    text = IrPrinter.print(prog)
    data_pos = text.index(".data")
    entry_pos = text.index(".entry")
    assert data_pos < entry_pos, ".data must appear before .entry"
  end

  def test_instruction_with_operands
    prog = IrProgram.new("_start")
    prog.add_instruction(IrInstruction.new(
      IrOp::LOAD_IMM,
      [IrRegister.new(0), IrImmediate.new(42)],
      5
    ))
    text = IrPrinter.print(prog)
    assert_match(/LOAD_IMM.*v0.*42.*; #5/, text)
  end

  def test_comment_instruction
    prog = IrProgram.new("_start")
    prog.add_instruction(IrInstruction.new(IrOp::COMMENT, [IrLabel.new("load tape base")], -1))
    text = IrPrinter.print(prog)
    assert_match(/^  ; load tape base$/, text)
  end

  def test_trailing_newline
    text = IrPrinter.print(minimal_program)
    assert text.end_with?("\n"), "output must end with newline"
  end

  def test_multiple_data_decls
    prog = IrProgram.new("_start")
    prog.add_data(IrDataDecl.new("tape", 30_000, 0))
    prog.add_data(IrDataDecl.new("buf", 4096, 0))
    prog.add_instruction(IrInstruction.new(IrOp::HALT, [], 0))
    text = IrPrinter.print(prog)
    assert_includes text, ".data tape 30000 0"
    assert_includes text, ".data buf 4096 0"
  end

  def test_branch_z_instruction
    prog = IrProgram.new("_start")
    prog.add_instruction(IrInstruction.new(
      IrOp::BRANCH_Z,
      [IrRegister.new(2), IrLabel.new("loop_0_end")],
      7
    ))
    text = IrPrinter.print(prog)
    assert_match(/BRANCH_Z.*v2.*loop_0_end.*; #7/, text)
  end
end
