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
