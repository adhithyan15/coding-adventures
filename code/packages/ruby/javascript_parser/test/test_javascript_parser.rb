# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the JavaScript Parser
# ================================================================
#
# These tests verify that the grammar-driven parser, when loaded
# with javascript.grammar, correctly builds Abstract Syntax Trees
# from JavaScript source code.
#
# Version-aware tests verify that the `version:` keyword argument
# selects the correct versioned ECMAScript grammar from
# code/grammars/ecmascript/.
# ================================================================

class TestJavascriptParser < Minitest::Test
  ASTNode = CodingAdventures::Parser::ASTNode
  TT = CodingAdventures::Lexer::TokenType

  def parse(source, version: nil)
    CodingAdventures::JavascriptParser.parse(source, version: version)
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
  # Grammar path resolution (generic)
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::JavascriptParser::JS_GRAMMAR_PATH),
      "javascript.grammar file should exist at #{CodingAdventures::JavascriptParser::JS_GRAMMAR_PATH}"
  end

  # ------------------------------------------------------------------
  # Version-aware: nil and empty string both use generic grammar
  # ------------------------------------------------------------------

  def test_no_version_uses_generic_grammar
    path = CodingAdventures::JavascriptParser.resolve_grammar_path(nil)
    assert_match(/javascript\.grammar$/, path)
    assert File.exist?(path), "Generic javascript.grammar should exist"
  end

  def test_empty_string_version_uses_generic_grammar
    path = CodingAdventures::JavascriptParser.resolve_grammar_path("")
    assert_match(/javascript\.grammar$/, path)
  end

  # ------------------------------------------------------------------
  # Version-aware: valid version strings resolve to versioned paths
  # ------------------------------------------------------------------

  def test_resolve_grammar_path_es1
    path = CodingAdventures::JavascriptParser.resolve_grammar_path("es1")
    assert_match(%r{ecmascript/es1\.grammar$}, path)
    assert File.exist?(path), "es1.grammar should exist at #{path}"
  end

  def test_resolve_grammar_path_es5
    path = CodingAdventures::JavascriptParser.resolve_grammar_path("es5")
    assert_match(%r{ecmascript/es5\.grammar$}, path)
    assert File.exist?(path), "es5.grammar should exist at #{path}"
  end

  def test_resolve_grammar_path_es2020
    path = CodingAdventures::JavascriptParser.resolve_grammar_path("es2020")
    assert_match(%r{ecmascript/es2020\.grammar$}, path)
    assert File.exist?(path), "es2020.grammar should exist at #{path}"
  end

  def test_resolve_grammar_path_es2025
    path = CodingAdventures::JavascriptParser.resolve_grammar_path("es2025")
    assert_match(%r{ecmascript/es2025\.grammar$}, path)
    assert File.exist?(path), "es2025.grammar should exist at #{path}"
  end

  # ------------------------------------------------------------------
  # Version-aware: all valid versions have grammar files on disk
  # ------------------------------------------------------------------

  def test_all_valid_versions_have_grammar_files
    CodingAdventures::JavascriptParser::VALID_VERSIONS.each do |version|
      path = CodingAdventures::JavascriptParser.resolve_grammar_path(version)
      assert File.exist?(path),
        "Grammar file for version #{version.inspect} should exist at #{path}"
    end
  end

  # ------------------------------------------------------------------
  # Version-aware: parse with an explicit version produces a program node
  # ------------------------------------------------------------------

  def test_parse_with_es2020_version
    ast = parse("let x = 1;", version: "es2020")
    assert_equal "program", ast.rule_name
  end

  def test_parse_with_es5_version
    ast = parse("var x = 1;", version: "es5")
    assert_equal "program", ast.rule_name
  end

  def test_parse_with_es1_version
    ast = parse("var x = 1;", version: "es1")
    assert_equal "program", ast.rule_name
  end

  # ------------------------------------------------------------------
  # Version-aware: unknown version raises ArgumentError
  # ------------------------------------------------------------------

  def test_unknown_version_raises_argument_error
    err = assert_raises(ArgumentError) do
      parse("let x = 1;", version: "es9999")
    end
    assert_match(/es9999/, err.message)
  end

  # ------------------------------------------------------------------
  # Backward compatibility: parse with no version arg still works
  # ------------------------------------------------------------------

  def test_backward_compatible_no_version
    ast = CodingAdventures::JavascriptParser.parse("let x = 1;")
    assert_equal "program", ast.rule_name
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
