# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_pipeline"

Orch  = CodingAdventures::Pipeline::Orchestrator
Res   = CodingAdventures::Pipeline::PipelineResult
LS    = CodingAdventures::Pipeline::LexerStage
PS    = CodingAdventures::Pipeline::ParserStage

class TestPipelineVersion < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::Pipeline::VERSION
  end
end

class TestPipelineOrchestratorBasic < Minitest::Test
  def setup
    @pipeline = Orch.new
  end

  def test_run_returns_pipeline_result
    result = @pipeline.run("x = 1")
    assert_instance_of Res, result
  end

  def test_result_has_lexer_stage
    result = @pipeline.run("x = 1")
    assert_instance_of LS, result.lexer_stage
  end

  def test_result_has_parser_stage
    result = @pipeline.run("x = 1")
    assert_instance_of PS, result.parser_stage
  end

  def test_result_preserves_source
    source = "x = 42"
    result = @pipeline.run(source)
    assert_equal source, result.source
  end
end

class TestPipelineLexerStage < Minitest::Test
  def setup
    @pipeline = Orch.new
  end

  def test_lexer_stage_has_tokens
    result = @pipeline.run("x = 1")
    assert_kind_of Array, result.lexer_stage.tokens
    refute_empty result.lexer_stage.tokens
  end

  def test_lexer_stage_token_count_matches_tokens
    result = @pipeline.run("x = 1 + 2")
    assert_equal result.lexer_stage.tokens.size, result.lexer_stage.token_count
  end

  def test_lexer_stage_preserves_source
    source = "x = 99"
    result = @pipeline.run(source)
    assert_equal source, result.lexer_stage.source
  end

  def test_simple_assignment_token_count
    # "x = 1\n" produces: NAME EQUALS NUMBER NEWLINE EOF = 5 tokens
    result = @pipeline.run("x = 1\n")
    assert result.lexer_stage.token_count >= 4
  end

  def test_arithmetic_expression_tokens
    result = @pipeline.run("y = 3 + 4\n")
    # Should have NAME, EQUALS, NUMBER, PLUS, NUMBER, NEWLINE, EOF
    assert result.lexer_stage.token_count >= 5
  end

  def test_tokens_include_eof
    result = @pipeline.run("x = 1")
    last_token = result.lexer_stage.tokens.last
    refute_nil last_token
    require "coding_adventures_lexer"
    assert_equal CodingAdventures::Lexer::TokenType::EOF, last_token.type
  end
end

class TestPipelineParserStage < Minitest::Test
  def setup
    @pipeline = Orch.new
  end

  def test_parser_stage_has_ast
    result = @pipeline.run("x = 1\n")
    refute_nil result.parser_stage.ast
  end

  def test_parser_stage_ast_is_program
    result = @pipeline.run("x = 1\n")
    require "coding_adventures_parser"
    assert_instance_of CodingAdventures::Parser::Program, result.parser_stage.ast
  end

  def test_parser_stage_node_count_positive
    result = @pipeline.run("x = 1\n")
    assert result.parser_stage.node_count > 0
  end

  def test_assignment_ast_structure
    result = @pipeline.run("x = 42\n")
    require "coding_adventures_parser"
    prog = result.parser_stage.ast
    assert_equal 1, prog.statements.size
    stmt = prog.statements.first
    assert_instance_of CodingAdventures::Parser::Assignment, stmt
    assert_equal "x", stmt.target.name
    assert_instance_of CodingAdventures::Parser::NumberLiteral, stmt.value
    assert_equal 42, stmt.value.value
  end

  def test_arithmetic_ast_structure
    result = @pipeline.run("z = 3 + 4\n")
    require "coding_adventures_parser"
    prog = result.parser_stage.ast
    stmt = prog.statements.first
    assert_instance_of CodingAdventures::Parser::Assignment, stmt
    rhs = stmt.value
    assert_instance_of CodingAdventures::Parser::BinaryOp, rhs
    assert_equal "+", rhs.op
    assert_equal 3, rhs.left.value
    assert_equal 4, rhs.right.value
  end

  def test_multiple_statements
    result = @pipeline.run("x = 1\ny = 2\n")
    prog = result.parser_stage.ast
    assert_equal 2, prog.statements.size
  end

  def test_empty_source_gives_empty_program
    result = @pipeline.run("")
    require "coding_adventures_parser"
    prog = result.parser_stage.ast
    assert_instance_of CodingAdventures::Parser::Program, prog
    assert_equal 0, prog.statements.size
  end
end

class TestPipelineMultipleRuns < Minitest::Test
  def test_pipeline_is_reusable
    pipeline = Orch.new
    result1 = pipeline.run("x = 1\n")
    result2 = pipeline.run("y = 2\n")
    # Each call is independent
    assert_equal "x = 1\n", result1.source
    assert_equal "y = 2\n", result2.source
  end
end
