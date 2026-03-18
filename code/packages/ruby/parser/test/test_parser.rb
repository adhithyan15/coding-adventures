# frozen_string_literal: true

require_relative "test_helper"

class TestParser < Minitest::Test
  P = CodingAdventures::Parser
  RDP = P::RecursiveDescentParser
  TT = CodingAdventures::Lexer::TokenType
  Tokenizer = CodingAdventures::Lexer::Tokenizer

  def parse(source)
    tokens = Tokenizer.new(source).tokenize
    RDP.new(tokens).parse
  end

  # -----------------------------------------------------------------------
  # Number literals
  # -----------------------------------------------------------------------

  def test_single_number
    ast = parse("42")
    assert_equal 1, ast.statements.length
    assert_kind_of P::NumberLiteral, ast.statements[0]
    assert_equal 42, ast.statements[0].value
  end

  def test_zero
    ast = parse("0")
    assert_equal 0, ast.statements[0].value
  end

  # -----------------------------------------------------------------------
  # String literals
  # -----------------------------------------------------------------------

  def test_string_literal
    ast = parse('"hello"')
    assert_kind_of P::StringLiteral, ast.statements[0]
    assert_equal "hello", ast.statements[0].value
  end

  # -----------------------------------------------------------------------
  # Name (variable reference)
  # -----------------------------------------------------------------------

  def test_name
    ast = parse("x")
    assert_kind_of P::Name, ast.statements[0]
    assert_equal "x", ast.statements[0].name
  end

  # -----------------------------------------------------------------------
  # Binary operations
  # -----------------------------------------------------------------------

  def test_addition
    ast = parse("1 + 2")
    expr = ast.statements[0]
    assert_kind_of P::BinaryOp, expr
    assert_equal "+", expr.op
    assert_equal 1, expr.left.value
    assert_equal 2, expr.right.value
  end

  def test_subtraction
    ast = parse("5 - 3")
    expr = ast.statements[0]
    assert_equal "-", expr.op
  end

  def test_multiplication
    ast = parse("2 * 3")
    expr = ast.statements[0]
    assert_equal "*", expr.op
  end

  def test_division
    ast = parse("10 / 2")
    expr = ast.statements[0]
    assert_equal "/", expr.op
  end

  # -----------------------------------------------------------------------
  # Operator precedence
  # -----------------------------------------------------------------------

  def test_mul_before_add
    # 1 + 2 * 3 should parse as 1 + (2 * 3)
    ast = parse("1 + 2 * 3")
    expr = ast.statements[0]
    assert_kind_of P::BinaryOp, expr
    assert_equal "+", expr.op
    assert_equal 1, expr.left.value
    assert_kind_of P::BinaryOp, expr.right
    assert_equal "*", expr.right.op
    assert_equal 2, expr.right.left.value
    assert_equal 3, expr.right.right.value
  end

  def test_div_before_sub
    # 10 - 6 / 2 should parse as 10 - (6 / 2)
    ast = parse("10 - 6 / 2")
    expr = ast.statements[0]
    assert_equal "-", expr.op
    assert_equal "/", expr.right.op
  end

  def test_parentheses_override_precedence
    # (1 + 2) * 3 should parse as (1 + 2) * 3
    ast = parse("(1 + 2) * 3")
    expr = ast.statements[0]
    assert_equal "*", expr.op
    assert_kind_of P::BinaryOp, expr.left
    assert_equal "+", expr.left.op
  end

  # -----------------------------------------------------------------------
  # Left associativity
  # -----------------------------------------------------------------------

  def test_left_associativity_addition
    # 1 + 2 + 3 should parse as (1 + 2) + 3
    ast = parse("1 + 2 + 3")
    expr = ast.statements[0]
    assert_equal "+", expr.op
    assert_kind_of P::BinaryOp, expr.left
    assert_equal "+", expr.left.op
    assert_equal 1, expr.left.left.value
    assert_equal 2, expr.left.right.value
    assert_equal 3, expr.right.value
  end

  def test_left_associativity_multiplication
    # 2 * 3 * 4 should parse as (2 * 3) * 4
    ast = parse("2 * 3 * 4")
    expr = ast.statements[0]
    assert_equal "*", expr.op
    assert_kind_of P::BinaryOp, expr.left
  end

  # -----------------------------------------------------------------------
  # Assignment
  # -----------------------------------------------------------------------

  def test_simple_assignment
    ast = parse("x = 42\n")
    stmt = ast.statements[0]
    assert_kind_of P::Assignment, stmt
    assert_equal "x", stmt.target.name
    assert_equal 42, stmt.value.value
  end

  def test_assignment_with_expression
    ast = parse("result = 1 + 2\n")
    stmt = ast.statements[0]
    assert_kind_of P::Assignment, stmt
    assert_kind_of P::BinaryOp, stmt.value
  end

  # -----------------------------------------------------------------------
  # Program (multiple statements)
  # -----------------------------------------------------------------------

  def test_multiple_statements
    ast = parse("x = 1\ny = 2\n")
    assert_equal 2, ast.statements.length
    assert_kind_of P::Assignment, ast.statements[0]
    assert_kind_of P::Assignment, ast.statements[1]
  end

  def test_blank_lines_between_statements
    ast = parse("x = 1\n\n\ny = 2\n")
    assert_equal 2, ast.statements.length
  end

  def test_mixed_statements
    ast = parse("x = 1\n2 + 3\n")
    assert_equal 2, ast.statements.length
    assert_kind_of P::Assignment, ast.statements[0]
    assert_kind_of P::BinaryOp, ast.statements[1]
  end

  def test_empty_program
    ast = parse("")
    assert_equal 0, ast.statements.length
  end

  # -----------------------------------------------------------------------
  # Nested parentheses
  # -----------------------------------------------------------------------

  def test_nested_parens
    ast = parse("((1 + 2))")
    expr = ast.statements[0]
    assert_kind_of P::BinaryOp, expr
    assert_equal "+", expr.op
  end

  # -----------------------------------------------------------------------
  # Error handling
  # -----------------------------------------------------------------------

  def test_unexpected_token_in_factor
    error = assert_raises(P::ParseError) do
      parse("+ 1")
    end
    assert_includes error.message, "Unexpected"
  end

  def test_missing_rparen
    error = assert_raises(P::ParseError) do
      parse("(1 + 2")
    end
    assert_includes error.message, "Expected RPAREN"
  end

  def test_parse_error_includes_token
    error = assert_raises(P::ParseError) do
      parse("+ 1")
    end
    assert_respond_to error, :token
  end
end
