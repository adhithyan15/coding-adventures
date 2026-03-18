# frozen_string_literal: true

require_relative "test_helper"

# Tests for the pipeline orchestrator.
#
# These tests verify that the full pipeline -- lexer, parser, compiler, VM --
# works end-to-end. Each test feeds source code into the pipeline and checks
# that every stage produced the expected output.

class TestPipelineBasic < Minitest::Test
  def setup
    @pipeline = CodingAdventures::Pipeline::Orchestrator.new
  end

  def test_simple_assignment_returns_pipeline_result
    result = @pipeline.run("x = 1 + 2")
    assert_instance_of CodingAdventures::Pipeline::PipelineResult, result
  end

  def test_source_is_preserved
    result = @pipeline.run("x = 1 + 2")
    assert_equal "x = 1 + 2", result.source
  end

  def test_lexer_stage_has_tokens
    result = @pipeline.run("x = 1 + 2")
    assert_instance_of CodingAdventures::Pipeline::LexerStage, result.lexer_stage
    assert result.lexer_stage.token_count >= 6
  end

  def test_lexer_stage_source
    result = @pipeline.run("x = 1 + 2")
    assert_equal "x = 1 + 2", result.lexer_stage.source
  end

  def test_parser_stage_has_ast
    result = @pipeline.run("x = 1 + 2")
    assert_instance_of CodingAdventures::Pipeline::ParserStage, result.parser_stage
    ast_dict = result.parser_stage.ast_dict
    assert_equal "Program", ast_dict["type"]
    assert_equal 1, ast_dict["statements"].length
  end

  def test_parser_stage_assignment
    result = @pipeline.run("x = 1 + 2")
    stmt = result.parser_stage.ast_dict["statements"][0]
    assert_equal "Assignment", stmt["type"]
    assert_equal({"type" => "Name", "name" => "x"}, stmt["target"])
  end

  def test_compiler_stage_has_instructions
    result = @pipeline.run("x = 1 + 2")
    assert_instance_of CodingAdventures::Pipeline::CompilerStage, result.compiler_stage
    refute_empty result.compiler_stage.instructions_text
  end

  def test_compiler_stage_constants
    result = @pipeline.run("x = 1 + 2")
    assert_equal [1, 2], result.compiler_stage.constants
  end

  def test_compiler_stage_names
    result = @pipeline.run("x = 1 + 2")
    assert_equal ["x"], result.compiler_stage.names
  end

  def test_vm_stage_final_variables
    result = @pipeline.run("x = 1 + 2")
    assert_instance_of CodingAdventures::Pipeline::VMStage, result.vm_stage
    assert_equal({"x" => 3}, result.vm_stage.final_variables)
  end

  def test_vm_stage_has_traces
    result = @pipeline.run("x = 1 + 2")
    refute_empty result.vm_stage.traces
  end

  def test_vm_stage_output_is_array
    result = @pipeline.run("x = 1 + 2")
    assert_instance_of Array, result.vm_stage.output
  end
end

class TestPipelineComplex < Minitest::Test
  def setup
    @pipeline = CodingAdventures::Pipeline::Orchestrator.new
  end

  def test_multiple_assignments
    result = @pipeline.run("a = 10\nb = 20\nc = a + b")
    assert_equal({"a" => 10, "b" => 20, "c" => 30}, result.vm_stage.final_variables)
  end

  def test_operator_precedence
    result = @pipeline.run("x = 1 + 2 * 3")
    assert_equal({"x" => 7}, result.vm_stage.final_variables)
  end

  def test_parentheses
    result = @pipeline.run("x = (1 + 2) * 3")
    assert_equal({"x" => 9}, result.vm_stage.final_variables)
  end

  def test_string_assignment
    result = @pipeline.run('x = "hello"')
    assert_equal({"x" => "hello"}, result.vm_stage.final_variables)
  end

  def test_subtraction
    result = @pipeline.run("x = 10 - 3")
    assert_equal({"x" => 7}, result.vm_stage.final_variables)
  end

  def test_division
    result = @pipeline.run("x = 10 / 2")
    assert_equal({"x" => 5.0}, result.vm_stage.final_variables)
  end

  def test_complex_expression
    result = @pipeline.run("x = (10 + 20) * (3 - 1)")
    assert_equal({"x" => 60}, result.vm_stage.final_variables)
  end

  def test_variable_reuse
    result = @pipeline.run("x = 5\ny = x * 2")
    assert_equal({"x" => 5, "y" => 10}, result.vm_stage.final_variables)
  end

  def test_multiple_statements_have_multiple_ast_nodes
    result = @pipeline.run("a = 1\nb = 2")
    assert_equal 2, result.parser_stage.ast_dict["statements"].length
  end

  def test_traces_count_increases_with_complexity
    simple = @pipeline.run("x = 1")
    complex_result = @pipeline.run("x = 1 + 2 * 3")
    assert complex_result.vm_stage.traces.length > simple.vm_stage.traces.length
  end
end

class TestAstToDict < Minitest::Test
  def test_number_literal
    node = CodingAdventures::Parser::NumberLiteral.new(value: 42)
    assert_equal({"type" => "NumberLiteral", "value" => 42},
      CodingAdventures::Pipeline.ast_to_dict(node))
  end

  def test_string_literal
    node = CodingAdventures::Parser::StringLiteral.new(value: "hello")
    assert_equal({"type" => "StringLiteral", "value" => "hello"},
      CodingAdventures::Pipeline.ast_to_dict(node))
  end

  def test_name
    node = CodingAdventures::Parser::Name.new(name: "x")
    assert_equal({"type" => "Name", "name" => "x"},
      CodingAdventures::Pipeline.ast_to_dict(node))
  end

  def test_binary_op
    node = CodingAdventures::Parser::BinaryOp.new(
      left: CodingAdventures::Parser::NumberLiteral.new(value: 1),
      op: "+",
      right: CodingAdventures::Parser::NumberLiteral.new(value: 2)
    )
    d = CodingAdventures::Pipeline.ast_to_dict(node)
    assert_equal "BinaryOp", d["type"]
    assert_equal "+", d["op"]
    assert_equal({"type" => "NumberLiteral", "value" => 1}, d["left"])
    assert_equal({"type" => "NumberLiteral", "value" => 2}, d["right"])
  end

  def test_assignment
    node = CodingAdventures::Parser::Assignment.new(
      target: CodingAdventures::Parser::Name.new(name: "x"),
      value: CodingAdventures::Parser::NumberLiteral.new(value: 42)
    )
    d = CodingAdventures::Pipeline.ast_to_dict(node)
    assert_equal "Assignment", d["type"]
    assert_equal({"type" => "Name", "name" => "x"}, d["target"])
    assert_equal({"type" => "NumberLiteral", "value" => 42}, d["value"])
  end

  def test_program
    stmt = CodingAdventures::Parser::Assignment.new(
      target: CodingAdventures::Parser::Name.new(name: "x"),
      value: CodingAdventures::Parser::NumberLiteral.new(value: 1)
    )
    prog = CodingAdventures::Parser::Program.new(statements: [stmt])
    d = CodingAdventures::Pipeline.ast_to_dict(prog)
    assert_equal "Program", d["type"]
    assert_equal 1, d["statements"].length
  end

  def test_unknown_type_fallback
    d = CodingAdventures::Pipeline.ast_to_dict("something else")
    assert_equal "String", d["type"]
    assert d.key?("repr")
  end
end

class TestInstructionToText < Minitest::Test
  def test_load_const_with_resolution
    code = CodingAdventures::VirtualMachine::CodeObject.new(
      instructions: [CodingAdventures::VirtualMachine::Instruction.new(:LOAD_CONST, 0)],
      constants: [42],
      names: []
    )
    text = CodingAdventures::Pipeline.instruction_to_text(code.instructions[0], code)
    assert_equal "LOAD_CONST 0 (42)", text
  end

  def test_store_name_with_resolution
    code = CodingAdventures::VirtualMachine::CodeObject.new(
      instructions: [CodingAdventures::VirtualMachine::Instruction.new(:STORE_NAME, 0)],
      constants: [],
      names: ["x"]
    )
    text = CodingAdventures::Pipeline.instruction_to_text(code.instructions[0], code)
    assert_equal 'STORE_NAME 0 ("x")', text
  end

  def test_load_name_with_resolution
    code = CodingAdventures::VirtualMachine::CodeObject.new(
      instructions: [CodingAdventures::VirtualMachine::Instruction.new(:LOAD_NAME, 0)],
      constants: [],
      names: ["y"]
    )
    text = CodingAdventures::Pipeline.instruction_to_text(code.instructions[0], code)
    assert_equal 'LOAD_NAME 0 ("y")', text
  end

  def test_add_no_operand
    code = CodingAdventures::VirtualMachine::CodeObject.new(
      instructions: [CodingAdventures::VirtualMachine::Instruction.new(:ADD)],
      constants: [],
      names: []
    )
    text = CodingAdventures::Pipeline.instruction_to_text(code.instructions[0], code)
    assert_equal "ADD", text
  end

  def test_halt_no_operand
    code = CodingAdventures::VirtualMachine::CodeObject.new(
      instructions: [CodingAdventures::VirtualMachine::Instruction.new(:HALT)],
      constants: [],
      names: []
    )
    text = CodingAdventures::Pipeline.instruction_to_text(code.instructions[0], code)
    assert_equal "HALT", text
  end

  def test_out_of_bounds_operand
    code = CodingAdventures::VirtualMachine::CodeObject.new(
      instructions: [CodingAdventures::VirtualMachine::Instruction.new(:LOAD_CONST, 99)],
      constants: [42],
      names: []
    )
    text = CodingAdventures::Pipeline.instruction_to_text(code.instructions[0], code)
    assert_equal "LOAD_CONST 99", text
  end
end

class TestStageDataclasses < Minitest::Test
  def setup
    @pipeline = CodingAdventures::Pipeline::Orchestrator.new
  end

  def test_lexer_stage_tokens_are_array
    result = @pipeline.run("x = 1")
    assert_instance_of Array, result.lexer_stage.tokens
  end

  def test_parser_stage_ast_dict_is_hash
    result = @pipeline.run("x = 1")
    assert_instance_of Hash, result.parser_stage.ast_dict
  end

  def test_compiler_stage_code_has_instructions
    result = @pipeline.run("x = 1")
    assert_respond_to result.compiler_stage.code, :instructions
  end

  def test_vm_stage_traces_are_array
    result = @pipeline.run("x = 1")
    assert_instance_of Array, result.vm_stage.traces
  end

  def test_vm_stage_final_variables_is_hash
    result = @pipeline.run("x = 1")
    assert_instance_of Hash, result.vm_stage.final_variables
  end
end
