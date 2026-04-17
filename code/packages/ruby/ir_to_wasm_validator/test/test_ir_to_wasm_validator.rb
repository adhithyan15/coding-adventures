# frozen_string_literal: true

require_relative "test_helper"

class TestIrToWasmValidator < Minitest::Test
  def test_returns_empty_for_lowerable_program
    program = IR::IrProgram.new("_start")
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LABEL, [IR::IrLabel.new("_start")], -1))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::HALT, [], 0))

    issues = ITWV.validate(program, [ITWC::FunctionSignature.new(label: "_start", param_count: 0, export_name: "_start")])

    assert_empty issues
  end

  def test_returns_issue_for_unsupported_syscall
    program = IR::IrProgram.new("_start")
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LABEL, [IR::IrLabel.new("_start")], -1))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::SYSCALL, [IR::IrImmediate.new(99)], 0))

    issues = ITWV.validate(program, [ITWC::FunctionSignature.new(label: "_start", param_count: 0, export_name: "_start")])

    assert_equal 1, issues.length
    assert_equal "lowering", issues.first.rule
    assert_match(/unsupported SYSCALL/, issues.first.message)
  end
end
