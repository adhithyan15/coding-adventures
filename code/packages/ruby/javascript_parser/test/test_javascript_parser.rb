# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the JavaScript Parser
# ================================================================
#
# These tests verify that the grammar-driven parser, when loaded
# with javascript.grammar, correctly builds Abstract Syntax Trees
# from JavaScript source code.
# ================================================================

class TestJavascriptParser < Minitest::Test
  ASTNode = CodingAdventures::Parser::ASTNode
  TT = CodingAdventures::Lexer::TokenType

  def parse(source)
    CodingAdventures::JavascriptParser.parse(source)
  end

  # ------------------------------------------------------------------
  # Variable declaration: let x = 1 + 2;
  # ------------------------------------------------------------------

  def test_let_declaration
    ast = parse("let x = 1 + 2;")

    assert_equal "program", ast.rule_name

    stmt = ast.children.find { |c| c.is_a?(ASTNode) }
    assert_equal "statement", stmt.rule_name

    var_decl = stmt.children.find { |c| c.is_a?(ASTNode) && c.rule_name == "var_declaration" }
    refute_nil var_decl, "Expected a var_declaration node"
  end

  def test_const_declaration
    ast = parse("const y = 42;")

    stmt = ast.children.find { |c| c.is_a?(ASTNode) }
    var_decl = stmt.children.find { |c| c.is_a?(ASTNode) && c.rule_name == "var_declaration" }
    refute_nil var_decl, "Expected a var_declaration node"

    tokens = collect_tokens(var_decl)
    keyword = tokens.find { |t| t.type == TT::KEYWORD }
    assert_equal "const", keyword.value
  end

  # ------------------------------------------------------------------
  # Assignment: x = 5;
  # ------------------------------------------------------------------

  def test_assignment
    ast = parse("x = 5;")

    stmt = ast.children.find { |c| c.is_a?(ASTNode) }
    assignment = stmt.children.find { |c| c.is_a?(ASTNode) && c.rule_name == "assignment" }
    refute_nil assignment, "Expected an assignment node"
  end

  # ------------------------------------------------------------------
  # Expression statement: 1 + 2;
  # ------------------------------------------------------------------

  def test_expression_statement
    ast = parse("1 + 2;")

    stmt = ast.children.find { |c| c.is_a?(ASTNode) }
    expr_stmt = stmt.children.find { |c| c.is_a?(ASTNode) && c.rule_name == "expression_stmt" }
    refute_nil expr_stmt, "Expected expression_stmt for bare expression"
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
    ast = parse("let x = 1;let y = 2;")

    assert_equal "program", ast.rule_name
    statements = ast.children.select { |c| c.is_a?(ASTNode) && c.rule_name == "statement" }
    assert_equal 2, statements.length, "Expected 2 statements"
  end

  # ------------------------------------------------------------------
  # Root is always 'program'
  # ------------------------------------------------------------------

  def test_root_is_program
    ast = parse("1;")
    assert_equal "program", ast.rule_name
  end

  # ------------------------------------------------------------------
  # Grammar path resolution
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::JavascriptParser::JS_GRAMMAR_PATH),
      "javascript.grammar file should exist at #{CodingAdventures::JavascriptParser::JS_GRAMMAR_PATH}"
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
