# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the ECMAScript 1 (ES1) Parser
# ================================================================
#
# These tests verify that the grammar-driven parser, when loaded
# with es1.grammar, correctly builds Abstract Syntax Trees from
# ES1 JavaScript source code.
#
# ES1 supports: var declarations, function declarations/expressions,
# if/else, while, do-while, for, for-in, switch/case, with,
# break, continue, return, labelled statements, and the full
# expression precedence chain.
# ================================================================

class TestEcmascriptEs1Parser < Minitest::Test
  ASTNode = CodingAdventures::Parser::ASTNode
  TT = CodingAdventures::Lexer::TokenType

  def parse(source)
    CodingAdventures::EcmascriptEs1Parser.parse(source)
  end

  # ------------------------------------------------------------------
  # Variable declaration: var x = 1 + 2;
  # ------------------------------------------------------------------

  def test_var_declaration
    ast = parse("var x = 1 + 2;")

    assert_equal "program", ast.rule_name

    stmt = ast.children.find { |c| c.is_a?(ASTNode) }
    assert_equal "source_element", stmt.rule_name
  end

  def test_var_without_initializer
    ast = parse("var x;")

    assert_equal "program", ast.rule_name
    source_el = ast.children.find { |c| c.is_a?(ASTNode) }
    refute_nil source_el
  end

  def test_multiple_var_declarations
    ast = parse("var x = 1, y = 2;")

    assert_equal "program", ast.rule_name
  end

  # ------------------------------------------------------------------
  # Assignment: x = 5;
  # ------------------------------------------------------------------

  def test_expression_with_name
    ast = parse("x;")

    assert_equal "program", ast.rule_name
    source_el = ast.children.find { |c| c.is_a?(ASTNode) }
    refute_nil source_el
  end

  # ------------------------------------------------------------------
  # Expression statement: 1 + 2;
  # ------------------------------------------------------------------

  def test_expression_statement
    ast = parse("1 + 2;")

    assert_equal "program", ast.rule_name
    source_el = ast.children.find { |c| c.is_a?(ASTNode) }
    refute_nil source_el
  end

  # ------------------------------------------------------------------
  # Operator precedence: 1 + 2 * 3;
  # ------------------------------------------------------------------

  def test_operator_precedence
    ast = parse("1 + 2 * 3;")

    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.type == TT::PLUS }, "Expected PLUS token"
    assert all_tokens.any? { |t| t.type == TT::STAR }, "Expected STAR token"
  end

  # ------------------------------------------------------------------
  # Multiple statements
  # ------------------------------------------------------------------

  def test_multiple_statements
    ast = parse("var x = 1;var y = 2;")

    assert_equal "program", ast.rule_name
    source_elements = ast.children.select { |c| c.is_a?(ASTNode) && c.rule_name == "source_element" }
    assert_equal 2, source_elements.length, "Expected 2 source elements"
  end

  # ------------------------------------------------------------------
  # Root is always 'program'
  # ------------------------------------------------------------------

  def test_root_is_program
    ast = parse("1;")
    assert_equal "program", ast.rule_name
  end

  # ------------------------------------------------------------------
  # Function declaration
  # ------------------------------------------------------------------

  def test_function_declaration
    ast = parse("function foo(a, b) { return a + b; }")

    all_tokens = collect_tokens(ast)
    fn_kw = all_tokens.find { |t| t.type == TT::KEYWORD && t.value == "function" }
    refute_nil fn_kw, "Expected 'function' keyword"
  end

  # ------------------------------------------------------------------
  # If statement
  # ------------------------------------------------------------------

  def test_if_statement
    ast = parse("if (x) { var y = 1; }")

    all_tokens = collect_tokens(ast)
    if_kw = all_tokens.find { |t| t.type == TT::KEYWORD && t.value == "if" }
    refute_nil if_kw, "Expected 'if' keyword"
  end

  # ------------------------------------------------------------------
  # While loop
  # ------------------------------------------------------------------

  def test_while_statement
    ast = parse("while (x) { 1; }")

    all_tokens = collect_tokens(ast)
    while_kw = all_tokens.find { |t| t.type == TT::KEYWORD && t.value == "while" }
    refute_nil while_kw, "Expected 'while' keyword"
  end

  # ------------------------------------------------------------------
  # For loop
  # ------------------------------------------------------------------

  def test_for_statement
    ast = parse("for (var i = 0; i; i) { }")

    all_tokens = collect_tokens(ast)
    for_kw = all_tokens.find { |t| t.type == TT::KEYWORD && t.value == "for" }
    refute_nil for_kw, "Expected 'for' keyword"
  end

  # ------------------------------------------------------------------
  # Empty program
  # ------------------------------------------------------------------

  def test_empty_program
    ast = parse("")
    assert_equal "program", ast.rule_name
  end

  # ------------------------------------------------------------------
  # Grammar path resolution
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::EcmascriptEs1Parser::ES1_GRAMMAR_PATH),
      "es1.grammar file should exist at #{CodingAdventures::EcmascriptEs1Parser::ES1_GRAMMAR_PATH}"
  end

  private

  def collect_tokens(node)
    tokens = []
    return tokens unless node.is_a?(ASTNode)

    node.children.each do |child|
      if child.is_a?(CodingAdventures::Lexer::Token)
        tokens << child
      elsif child.is_a?(ASTNode)
        tokens.concat(collect_tokens(child))
      end
    end
    tokens
  end
end
