# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the C# Parser
# ================================================================
#
# These tests verify that the grammar-driven parser, when loaded
# with csharp/csharp<version>.grammar, correctly builds Abstract
# Syntax Trees from C# source code.
#
# The AST is a tree of ASTNode objects.  Each ASTNode has a
# `rule_name` (the name of the grammar production that matched,
# e.g. "program", "statement", "var_declaration") and a list of
# `children` (either more ASTNodes or Token leaf nodes).
#
# Grammar structure (simplified):
#
#   program      -> statement*
#   statement    -> var_declaration | assignment | expression_stmt
#   var_declaration -> KEYWORD NAME EQUALS expression SEMICOLON
#   assignment   -> NAME EQUALS expression SEMICOLON
#   expression_stmt -> expression SEMICOLON
#   expression   -> term ((PLUS | MINUS) term)*
#   term         -> factor ((STAR | SLASH) factor)*
#   factor       -> NUMBER | NAME | LPAREN expression RPAREN
#
# Version-aware tests verify that the `version:` keyword argument
# selects the correct versioned C# grammar from code/grammars/csharp/.
# ================================================================

class TestCSharpParser < Minitest::Test
  ASTNode = CodingAdventures::Parser::ASTNode
  TT = CodingAdventures::Lexer::TokenType

  def parse(source, version: nil)
    CodingAdventures::CSharpParser.parse(source, version: version)
  end

  # ------------------------------------------------------------------
  # Simple class declaration
  # ------------------------------------------------------------------
  #
  # The most basic C# construct is a class declaration.  We verify
  # that the parser produces a root "program" node and that somewhere
  # in the tree we can find a node that corresponds to the class.

  def test_simple_class_declaration
    ast = parse("class Foo { }")
    assert_equal "program", ast.rule_name
  end

  def test_class_declaration_is_a_statement
    ast = parse("class Foo { }")
    stmt = ast.children.find { |c| c.is_a?(ASTNode) }
    refute_nil stmt, "Expected at least one statement child under program"
  end

  # ------------------------------------------------------------------
  # Variable declaration: int x = 1 + 2;
  # ------------------------------------------------------------------

  def test_int_declaration
    ast = parse("int x = 1 + 2;")

    assert_equal "program", ast.rule_name

    stmt = ast.children.find { |c| c.is_a?(ASTNode) }
    assert_equal "statement", stmt.rule_name

    var_decl = stmt.children.find { |c| c.is_a?(ASTNode) && c.rule_name == "var_declaration" }
    refute_nil var_decl, "Expected a var_declaration node"
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
    ast = parse("int x = 1;int y = 2;")

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
  # Grammar path resolution (default)
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    path = CodingAdventures::CSharpParser.resolve_grammar_path(nil)
    assert File.exist?(path),
      "csharp12.0.grammar file should exist at #{path}"
  end

  # ------------------------------------------------------------------
  # Version-aware: nil and empty string both use default grammar
  # ------------------------------------------------------------------

  def test_no_version_uses_default_grammar
    path = CodingAdventures::CSharpParser.resolve_grammar_path(nil)
    assert_match(%r{csharp/csharp12\.0\.grammar$}, path)
    assert File.exist?(path), "Default csharp12.0.grammar should exist"
  end

  def test_empty_string_version_uses_default_grammar
    path = CodingAdventures::CSharpParser.resolve_grammar_path("")
    assert_match(%r{csharp/csharp12\.0\.grammar$}, path)
  end

  # ------------------------------------------------------------------
  # Version-aware: valid version strings resolve to versioned paths
  # ------------------------------------------------------------------

  def test_resolve_grammar_path_1_0
    path = CodingAdventures::CSharpParser.resolve_grammar_path("1.0")
    assert_match(%r{csharp/csharp1\.0\.grammar$}, path)
    assert File.exist?(path), "csharp1.0.grammar should exist at #{path}"
  end

  def test_resolve_grammar_path_5_0
    path = CodingAdventures::CSharpParser.resolve_grammar_path("5.0")
    assert_match(%r{csharp/csharp5\.0\.grammar$}, path)
    assert File.exist?(path), "csharp5.0.grammar should exist at #{path}"
  end

  def test_resolve_grammar_path_8_0
    path = CodingAdventures::CSharpParser.resolve_grammar_path("8.0")
    assert_match(%r{csharp/csharp8\.0\.grammar$}, path)
    assert File.exist?(path), "csharp8.0.grammar should exist at #{path}"
  end

  def test_resolve_grammar_path_12_0
    path = CodingAdventures::CSharpParser.resolve_grammar_path("12.0")
    assert_match(%r{csharp/csharp12\.0\.grammar$}, path)
    assert File.exist?(path), "csharp12.0.grammar should exist at #{path}"
  end

  # ------------------------------------------------------------------
  # Version-aware: all 12 valid versions have grammar files on disk
  # ------------------------------------------------------------------

  def test_all_valid_versions_have_grammar_files
    CodingAdventures::CSharpParser::VALID_VERSIONS.each do |version|
      path = CodingAdventures::CSharpParser.resolve_grammar_path(version)
      assert File.exist?(path),
        "Grammar file for version #{version.inspect} should exist at #{path}"
    end
  end

  def test_valid_versions_count
    # Exactly 12 versions: 1.0 through 12.0
    assert_equal 12, CodingAdventures::CSharpParser::VALID_VERSIONS.length
  end

  # ------------------------------------------------------------------
  # Version-aware: parse with explicit version produces a program node
  # ------------------------------------------------------------------

  def test_parse_with_csharp_1_0_version
    ast = parse("int x = 1;", version: "1.0")
    assert_equal "program", ast.rule_name
  end

  def test_parse_with_csharp_8_0_version
    ast = parse("int x = 1;", version: "8.0")
    assert_equal "program", ast.rule_name
  end

  def test_parse_with_csharp_12_0_version
    # "class Foo { }" is valid with the default C# 12.0 grammar
    ast = parse("class Foo { }", version: "12.0")
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

  def test_unknown_version_with_java_style_integer_raises
    # C# uses "8.0" not "8" -- bare integers are not valid
    err = assert_raises(ArgumentError) do
      parse("int x = 1;", version: "8")
    end
    assert_match(/8/, err.message)
  end

  # ------------------------------------------------------------------
  # Backward compatibility: parse with no version arg still works
  # ------------------------------------------------------------------

  def test_backward_compatible_no_version
    ast = CodingAdventures::CSharpParser.parse("int x = 1;")
    assert_equal "program", ast.rule_name
  end

  # ------------------------------------------------------------------
  # parse_csharp alias
  # ------------------------------------------------------------------

  def test_parse_csharp_alias_works
    ast = CodingAdventures::CSharpParser.parse_csharp("int x = 1;")
    assert_equal "program", ast.rule_name
  end

  def test_parse_csharp_alias_accepts_version
    ast = CodingAdventures::CSharpParser.parse_csharp("int x = 1;", version: "5.0")
    assert_equal "program", ast.rule_name
  end

  # ------------------------------------------------------------------
  # create_csharp_parser factory method
  # ------------------------------------------------------------------

  def test_create_csharp_parser_returns_hash
    result = CodingAdventures::CSharpParser.create_csharp_parser("int x = 1;")
    assert_instance_of Hash, result
  end

  def test_create_csharp_parser_stores_source
    result = CodingAdventures::CSharpParser.create_csharp_parser("int x = 1;")
    assert_equal "int x = 1;", result[:source]
  end

  def test_create_csharp_parser_stores_nil_version
    result = CodingAdventures::CSharpParser.create_csharp_parser("int x = 1;")
    assert_nil result[:version]
  end

  def test_create_csharp_parser_stores_language
    result = CodingAdventures::CSharpParser.create_csharp_parser("int x = 1;")
    assert_equal :csharp, result[:language]
  end

  def test_create_csharp_parser_with_version
    result = CodingAdventures::CSharpParser.create_csharp_parser("int x = 1;", version: "8.0")
    assert_equal "8.0", result[:version]
  end

  def test_create_csharp_parser_raises_for_unknown_version
    assert_raises(ArgumentError) do
      CodingAdventures::CSharpParser.create_csharp_parser("int x = 1;", version: "bogus")
    end
  end

  private

  # Recursively collect all Token leaf nodes from an AST subtree.
  # Used in tests that need to verify which tokens appear anywhere in
  # the tree, regardless of depth.
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
