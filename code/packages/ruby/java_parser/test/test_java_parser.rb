# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the Java Parser
# ================================================================
#
# These tests verify that the grammar-driven parser, when loaded
# with java/java<version>.grammar, correctly builds Abstract Syntax Trees
# from Java source code.
#
# Version-aware tests verify that the `version:` keyword argument
# selects the correct versioned Java grammar from
# code/grammars/java/.
# ================================================================

class TestJavaParser < Minitest::Test
  ASTNode = CodingAdventures::Parser::ASTNode
  TT = CodingAdventures::Lexer::TokenType

  def find_rule(node, rule_name)
    return nil unless node.is_a?(ASTNode)
    return node if node.rule_name == rule_name

    node.children.each do |child|
      found = find_rule(child, rule_name)
      return found if found
    end

    nil
  end

  def find_rules(node, rule_name, results = [])
    return results unless node.is_a?(ASTNode)

    results << node if node.rule_name == rule_name
    node.children.each do |child|
      find_rules(child, rule_name, results)
    end

    results
  end

  def parse(source, version: nil)
    CodingAdventures::JavaParser.parse(source, version: version)
  end

  # ------------------------------------------------------------------
  # Variable declaration: int x = 1 + 2;
  # ------------------------------------------------------------------

  def test_int_declaration
    ast = parse("int x = 1 + 2;")

    assert_equal "program", ast.rule_name

    stmt = find_rule(ast, "statement")
    assert_equal "statement", stmt.rule_name

    var_decl = find_rule(ast, "var_declaration")
    refute_nil var_decl, "Expected a var_declaration node"
  end

  # ------------------------------------------------------------------
  # Assignment: x = 5;
  # ------------------------------------------------------------------

  def test_assignment
    ast = parse("x = 5;")

    assignment = find_rule(ast, "assignment_expression")
    refute_nil assignment, "Expected an assignment_expression node"
  end

  # ------------------------------------------------------------------
  # Expression statement: 1 + 2;
  # ------------------------------------------------------------------

  def test_expression_statement
    ast = parse("1 + 2;")

    expr_stmt = find_rule(ast, "expression_statement")
    refute_nil expr_stmt, "Expected expression_statement for bare expression"
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
    ast = parse("int x = 1;int y = 2;")

    assert_equal "program", ast.rule_name
    statements = find_rules(ast, "statement")
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
  # Grammar path resolution (default)
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    path = CodingAdventures::JavaParser.resolve_grammar_path(nil)
    assert File.exist?(path),
      "java21.grammar file should exist at #{path}"
  end

  # ------------------------------------------------------------------
  # Version-aware: nil and empty string both use default grammar
  # ------------------------------------------------------------------

  def test_no_version_uses_default_grammar
    path = CodingAdventures::JavaParser.resolve_grammar_path(nil)
    assert_match(%r{java/java21\.grammar$}, path)
    assert File.exist?(path), "Default java21.grammar should exist"
  end

  def test_empty_string_version_uses_default_grammar
    path = CodingAdventures::JavaParser.resolve_grammar_path("")
    assert_match(%r{java/java21\.grammar$}, path)
  end

  # ------------------------------------------------------------------
  # Version-aware: valid version strings resolve to versioned paths
  # ------------------------------------------------------------------

  def test_resolve_grammar_path_1_0
    path = CodingAdventures::JavaParser.resolve_grammar_path("1.0")
    assert_match(%r{java/java1\.0\.grammar$}, path)
    assert File.exist?(path), "java1.0.grammar should exist at #{path}"
  end

  def test_resolve_grammar_path_8
    path = CodingAdventures::JavaParser.resolve_grammar_path("8")
    assert_match(%r{java/java8\.grammar$}, path)
    assert File.exist?(path), "java8.grammar should exist at #{path}"
  end

  def test_resolve_grammar_path_17
    path = CodingAdventures::JavaParser.resolve_grammar_path("17")
    assert_match(%r{java/java17\.grammar$}, path)
    assert File.exist?(path), "java17.grammar should exist at #{path}"
  end

  def test_resolve_grammar_path_21
    path = CodingAdventures::JavaParser.resolve_grammar_path("21")
    assert_match(%r{java/java21\.grammar$}, path)
    assert File.exist?(path), "java21.grammar should exist at #{path}"
  end

  # ------------------------------------------------------------------
  # Version-aware: all valid versions have grammar files on disk
  # ------------------------------------------------------------------

  def test_all_valid_versions_have_grammar_files
    CodingAdventures::JavaParser::VALID_VERSIONS.each do |version|
      path = CodingAdventures::JavaParser.resolve_grammar_path(version)
      assert File.exist?(path),
        "Grammar file for version #{version.inspect} should exist at #{path}"
    end
  end

  # ------------------------------------------------------------------
  # Version-aware: parse with an explicit version produces a program node
  # ------------------------------------------------------------------

  def test_parse_with_java_8_version
    ast = parse("int x = 1;", version: "8")
    assert_equal "program", ast.rule_name
  end

  def test_parse_with_java_17_version
    ast = parse("int x = 1;", version: "17")
    assert_equal "program", ast.rule_name
  end

  def test_parse_with_java_1_0_version
    ast = parse("int x = 1;", version: "1.0")
    assert_equal "program", ast.rule_name
  end

  # ------------------------------------------------------------------
  # Version-aware: unknown version raises ArgumentError
  # ------------------------------------------------------------------

  def test_unknown_version_raises_argument_error
    err = assert_raises(ArgumentError) do
      parse("int x = 1;", version: "99")
    end
    assert_match(/99/, err.message)
  end

  # ------------------------------------------------------------------
  # Backward compatibility: parse with no version arg still works
  # ------------------------------------------------------------------

  def test_backward_compatible_no_version
    ast = CodingAdventures::JavaParser.parse("int x = 1;")
    assert_equal "program", ast.rule_name
  end

  # ------------------------------------------------------------------
  # create_parser factory method
  # ------------------------------------------------------------------

  def test_create_parser_returns_hash
    result = CodingAdventures::JavaParser.create_parser("int x = 1;")
    assert_instance_of Hash, result
  end

  def test_create_parser_stores_source
    result = CodingAdventures::JavaParser.create_parser("int x = 1;")
    assert_equal "int x = 1;", result[:source]
  end

  def test_create_parser_stores_nil_version
    result = CodingAdventures::JavaParser.create_parser("int x = 1;")
    assert_nil result[:version]
  end

  def test_create_parser_stores_language
    result = CodingAdventures::JavaParser.create_parser("int x = 1;")
    assert_equal :java, result[:language]
  end

  def test_create_parser_with_version
    result = CodingAdventures::JavaParser.create_parser("int x = 1;", version: "8")
    assert_equal "8", result[:version]
  end

  def test_create_parser_raises_for_unknown_version
    assert_raises(ArgumentError) do
      CodingAdventures::JavaParser.create_parser("int x = 1;", version: "bogus")
    end
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
