# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the ECMAScript 5 (ES5) Parser
# ================================================================
#
# These tests verify ES5-specific grammar features:
#   - debugger statement
#   - All ES3 features (try/catch/finally/throw) still work
#   - All ES1 features still work
# ================================================================

class TestEcmascriptEs5Parser < Minitest::Test
  ASTNode = CodingAdventures::Parser::ASTNode
  TT = CodingAdventures::Lexer::TokenType

  def parse(source)
    CodingAdventures::EcmascriptEs5Parser.parse(source)
  end

  # ------------------------------------------------------------------
  # Basic: var declaration (inherited from ES1)
  # ------------------------------------------------------------------

  def test_var_declaration
    ast = parse("var x = 1 + 2;")

    assert_equal "program", ast.rule_name
    source_el = ast.children.find { |c| c.is_a?(ASTNode) }
    refute_nil source_el
  end

  # ------------------------------------------------------------------
  # ES5-specific: debugger statement
  # ------------------------------------------------------------------

  def test_debugger_statement
    ast = parse("debugger;")

    assert_equal "program", ast.rule_name
    all_tokens = collect_tokens(ast)
    debugger_kw = all_tokens.find { |t| t.type == TT::KEYWORD && t.value == "debugger" }
    refute_nil debugger_kw, "Expected 'debugger' keyword in AST"
  end

  # ------------------------------------------------------------------
  # ES3 features retained: try/catch
  # ------------------------------------------------------------------

  def test_try_catch
    ast = parse("try { var x = 1; } catch (e) { var y = 2; }")

    all_tokens = collect_tokens(ast)
    try_kw = all_tokens.find { |t| t.type == TT::KEYWORD && t.value == "try" }
    refute_nil try_kw, "Expected 'try' keyword in AST"
    catch_kw = all_tokens.find { |t| t.type == TT::KEYWORD && t.value == "catch" }
    refute_nil catch_kw, "Expected 'catch' keyword in AST"
  end

  def test_throw_statement
    ast = parse("throw x;")

    all_tokens = collect_tokens(ast)
    throw_kw = all_tokens.find { |t| t.type == TT::KEYWORD && t.value == "throw" }
    refute_nil throw_kw, "Expected 'throw' keyword in AST"
  end

  # ------------------------------------------------------------------
  # Expression statement
  # ------------------------------------------------------------------

  def test_expression_statement
    ast = parse("1 + 2;")

    assert_equal "program", ast.rule_name
    source_el = ast.children.find { |c| c.is_a?(ASTNode) }
    refute_nil source_el
  end

  # ------------------------------------------------------------------
  # Operator precedence
  # ------------------------------------------------------------------

  def test_operator_precedence
    ast = parse("1 + 2 * 3;")

    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.type == TT::PLUS }
    assert all_tokens.any? { |t| t.type == TT::STAR }
  end

  # ------------------------------------------------------------------
  # Multiple statements
  # ------------------------------------------------------------------

  def test_multiple_statements
    ast = parse("var x = 1;var y = 2;")

    assert_equal "program", ast.rule_name
    source_elements = ast.children.select { |c| c.is_a?(ASTNode) && c.rule_name == "source_element" }
    assert_equal 2, source_elements.length
  end

  # ------------------------------------------------------------------
  # Root is always 'program'
  # ------------------------------------------------------------------

  def test_root_is_program
    ast = parse("1;")
    assert_equal "program", ast.rule_name
  end

  # ------------------------------------------------------------------
  # Empty program
  # ------------------------------------------------------------------

  def test_empty_program
    ast = parse("")
    assert_equal "program", ast.rule_name
  end

  # ------------------------------------------------------------------
  # Function declaration
  # ------------------------------------------------------------------

  def test_function_declaration
    ast = parse("function foo(a) { return a; }")

    all_tokens = collect_tokens(ast)
    fn_kw = all_tokens.find { |t| t.type == TT::KEYWORD && t.value == "function" }
    refute_nil fn_kw
  end

  # ------------------------------------------------------------------
  # Grammar path resolution
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::EcmascriptEs5Parser::ES5_GRAMMAR_PATH),
      "es5.grammar file should exist at #{CodingAdventures::EcmascriptEs5Parser::ES5_GRAMMAR_PATH}"
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
