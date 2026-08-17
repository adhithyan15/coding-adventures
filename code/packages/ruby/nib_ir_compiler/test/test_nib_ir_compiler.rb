# frozen_string_literal: true

require_relative "test_helper"

class NibIrCompilerTest < Minitest::Test
  def compile_source(source)
    ast = CodingAdventures::NibParser.parse_nib(source)
    typed = CodingAdventures::NibTypeChecker.check(ast)
    assert typed.ok, typed.errors.map(&:message).join("\n")
    CodingAdventures::NibIrCompiler.compile_nib(typed.typed_ast).program
  end

  def test_emits_program_entry_and_halt
    program = compile_source("fn main() -> u4 { return 7; }")
    opcodes = program.instructions.map(&:opcode)

    assert_includes opcodes, CodingAdventures::CompilerIr::IrOp::LABEL
    assert_includes opcodes, CodingAdventures::CompilerIr::IrOp::CALL
    assert_includes opcodes, CodingAdventures::CompilerIr::IrOp::HALT
  end

  def test_emits_call_and_add_shapes
    program = compile_source("fn add(a: u4, b: u4) -> u4 { return a +% b; } fn main() -> u4 { return add(3, 4); }")
    opcodes = program.instructions.map(&:opcode)

    assert_includes opcodes, CodingAdventures::CompilerIr::IrOp::ADD
    assert_includes opcodes, CodingAdventures::CompilerIr::IrOp::CALL
  end

  # Regression test for #11257: see the matching test in nib_type_checker's
  # suite for the full root-cause writeup. The bug also broke code
  # generation here -- compile_add couldn't find its add_expr operands
  # either, so no ADD/ADD_IMM instruction was ever emitted for plain
  # additive expressions like `a + b` (using the PLUS operator, as opposed
  # to `+%`/WRAP_ADD used by the other tests in this file, which happens to
  # exercise the same bug already).
  def test_emits_add_for_plain_additive_expression
    program = compile_source("fn add(a: u4, b: u4) -> u4 { return a + b; } fn main() -> u4 { return add(3, 4); }")
    opcodes = program.instructions.map(&:opcode)

    assert_includes opcodes, CodingAdventures::CompilerIr::IrOp::ADD
  end

  def test_emits_loop_branches
    program = compile_source(<<~NIB)
      fn count_to(n: u4) -> u4 {
        let acc: u4 = 0;
        for i: u4 in 0..n {
          acc = acc +% 1;
        }
        return acc;
      }
    NIB

    opcodes = program.instructions.map(&:opcode)
    assert_includes opcodes, CodingAdventures::CompilerIr::IrOp::BRANCH_Z
    assert_includes opcodes, CodingAdventures::CompilerIr::IrOp::JUMP
  end
end
