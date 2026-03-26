# frozen_string_literal: true

# ==========================================================================
# Tests for the Brainfuck Translator (source → CodeObject)
# ==========================================================================

require_relative "test_helper"

class TestBasicTranslation < Minitest::Test
  Op = CodingAdventures::Brainfuck::Op

  def translate(source)
    CodingAdventures::Brainfuck.translate(source)
  end

  # Each BF character maps to one instruction.

  def test_empty_program
    code = translate("")
    assert_equal 1, code.instructions.length  # just HALT
    assert_equal Op::HALT, code.instructions[0].opcode
  end

  def test_single_right
    code = translate(">")
    assert_equal Op::RIGHT, code.instructions[0].opcode
    assert_equal Op::HALT, code.instructions[1].opcode
  end

  def test_single_left
    code = translate("<")
    assert_equal Op::LEFT, code.instructions[0].opcode
  end

  def test_single_inc
    code = translate("+")
    assert_equal Op::INC, code.instructions[0].opcode
  end

  def test_single_dec
    code = translate("-")
    assert_equal Op::DEC, code.instructions[0].opcode
  end

  def test_single_output
    code = translate(".")
    assert_equal Op::OUTPUT, code.instructions[0].opcode
  end

  def test_single_input
    code = translate(",")
    assert_equal Op::INPUT, code.instructions[0].opcode
  end

  def test_multiple_commands
    code = translate("+++>.")
    ops = code.instructions.map(&:opcode)
    assert_equal [Op::INC, Op::INC, Op::INC, Op::RIGHT, Op::OUTPUT, Op::HALT], ops
  end

  def test_comments_ignored
    code = translate("hello + world - !")
    ops = code.instructions.map(&:opcode)
    assert_equal [Op::INC, Op::DEC, Op::HALT], ops
  end

  def test_whitespace_ignored
    code = translate("  +  +  +  ")
    ops = code.instructions.map(&:opcode)
    assert_equal [Op::INC, Op::INC, Op::INC, Op::HALT], ops
  end

  def test_empty_constant_pool
    code = translate("+++")
    assert_equal [], code.constants
  end

  def test_empty_name_pool
    code = translate("+++")
    assert_equal [], code.names
  end
end

class TestBracketMatching < Minitest::Test
  Op = CodingAdventures::Brainfuck::Op

  def translate(source)
    CodingAdventures::Brainfuck.translate(source)
  end

  def test_simple_loop
    # [>+<-] — the simplest loop.
    code = translate("[>+<-]")
    # Instructions: LOOP_START, RIGHT, INC, LEFT, DEC, LOOP_END, HALT
    assert_equal 7, code.instructions.length

    loop_start = code.instructions[0]
    loop_end = code.instructions[5]

    assert_equal Op::LOOP_START, loop_start.opcode
    assert_equal 6, loop_start.operand  # jump past LOOP_END to HALT

    assert_equal Op::LOOP_END, loop_end.opcode
    assert_equal 0, loop_end.operand  # jump back to LOOP_START
  end

  def test_nested_loops
    # ++[>++[>+<-]<-]
    code = translate("++[>++[>+<-]<-]")

    # Outer [ at index 2, inner [ at index 6
    outer_start = code.instructions[2]
    inner_start = code.instructions[6]

    assert_equal Op::LOOP_START, outer_start.opcode
    assert_equal Op::LOOP_START, inner_start.opcode

    # Inner ] at index 11, outer ] at index 14
    inner_end = code.instructions[11]
    outer_end = code.instructions[14]

    # Inner loop: [ at 6 jumps to 12 (past ] at 11), ] at 11 jumps back to 6
    assert_equal 12, inner_start.operand
    assert_equal 6, inner_end.operand

    # Outer loop: [ at 2 jumps to 15 (past ] at 14), ] at 14 jumps back to 2
    assert_equal 15, outer_start.operand
    assert_equal 2, outer_end.operand
  end

  def test_empty_loop
    # [] — an empty loop
    code = translate("[]")
    assert_equal Op::LOOP_START, code.instructions[0].opcode
    assert_equal 2, code.instructions[0].operand  # past LOOP_END
    assert_equal Op::LOOP_END, code.instructions[1].opcode
    assert_equal 0, code.instructions[1].operand  # back to LOOP_START
  end

  def test_adjacent_loops
    # [][] — two loops side by side
    code = translate("[][]")
    assert_equal 2, code.instructions[0].operand
    assert_equal 0, code.instructions[1].operand
    assert_equal 4, code.instructions[2].operand
    assert_equal 2, code.instructions[3].operand
  end
end

class TestBracketErrors < Minitest::Test
  def translate(source)
    CodingAdventures::Brainfuck.translate(source)
  end

  def test_unmatched_open_bracket
    assert_raises(CodingAdventures::Brainfuck::TranslationError) do
      translate("[")
    end
  end

  def test_unmatched_close_bracket
    assert_raises(CodingAdventures::Brainfuck::TranslationError) do
      translate("]")
    end
  end

  def test_extra_open_bracket
    assert_raises(CodingAdventures::Brainfuck::TranslationError) do
      translate("[[]")
    end
  end

  def test_extra_close_bracket
    assert_raises(CodingAdventures::Brainfuck::TranslationError) do
      translate("[]]")
    end
  end

  def test_multiple_unmatched
    error = assert_raises(CodingAdventures::Brainfuck::TranslationError) do
      translate("[[")
    end
    assert_match(/2 unclosed/, error.message)
  end
end
