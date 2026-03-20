# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the Starlark Parser
# ================================================================
#
# These tests verify that the grammar-driven parser, when loaded
# with starlark.grammar, correctly builds Abstract Syntax Trees from
# Starlark source code.
#
# The grammar-driven parser produces generic ASTNode objects:
#
#   ASTNode(rule_name: "file", children: [...])
#
# Each node records which grammar rule produced it and its matched
# children (which can be tokens or other ASTNodes). This is different
# from a hand-written parser's typed nodes (NumberLiteral, BinaryOp),
# but it captures the same structural information.
#
# Starlark's grammar (starlark.grammar) is a full EBNF grammar with
# rules for:
#   - file-level structure (file = { NEWLINE | statement })
#   - compound statements (if/elif/else, for, def)
#   - simple statements (assignments, return, break, continue, pass, load)
#   - 15 levels of expression precedence
#   - list/dict literals and comprehensions
#   - function call arguments
# ================================================================

class TestStarlarkParser < Minitest::Test
  ASTNode = CodingAdventures::Parser::ASTNode
  TT = CodingAdventures::Lexer::TokenType
  INT_TYPE = "INT"

  # ------------------------------------------------------------------
  # Helper
  # ------------------------------------------------------------------

  def parse(source)
    CodingAdventures::StarlarkParser.parse(source)
  end

  # ------------------------------------------------------------------
  # Basic assignment: x = 1
  # ------------------------------------------------------------------
  # The simplest Starlark statement. The grammar path is:
  #   file -> statement -> simple_stmt -> small_stmt -> assign_stmt
  #   assign_stmt -> expression_list EQUALS expression_list

  def test_simple_assignment
    ast = parse("x = 1\n")

    # Root should be a 'file' node (Starlark's top-level rule is 'file')
    assert_equal "file", ast.rule_name

    # Should contain at least one statement
    statements = find_nodes_by_rule(ast, "statement")
    assert statements.length >= 1, "Expected at least one statement"

    # Should contain an assign_stmt somewhere in the tree
    assign_stmts = find_nodes_by_rule(ast, "assign_stmt")
    assert assign_stmts.length >= 1, "Expected an assign_stmt node"

    # The NAME token 'x' and NUMBER token '1' should both appear
    all_tokens = collect_tokens(ast)
    name_token = all_tokens.find { |t| t.type == TT::NAME && t.value == "x" }
    refute_nil name_token, "Expected NAME token 'x' in AST"
    number_token = all_tokens.find { |t| t.type == INT_TYPE && t.value == "1" }
    refute_nil number_token, "Expected INT token '1' in AST"
  end

  # ------------------------------------------------------------------
  # Expression: 1 + 2 * 3
  # ------------------------------------------------------------------
  # Tests operator precedence. The grammar encodes precedence via
  # rule hierarchy:
  #   arith = term { (PLUS | MINUS) term }
  #   term  = factor { (STAR | SLASH | FLOOR_DIV | PERCENT) factor }
  #
  # So 1 + 2 * 3 should parse as 1 + (2 * 3), with STAR nested
  # deeper in the AST than PLUS.

  def test_expression
    ast = parse("1 + 2 * 3\n")

    assert_equal "file", ast.rule_name

    # Should have a PLUS and a STAR token in the tree
    all_tokens = collect_tokens(ast)
    plus_token = all_tokens.find { |t| t.type == TT::PLUS }
    star_token = all_tokens.find { |t| t.type == TT::STAR }
    refute_nil plus_token, "Expected PLUS token"
    refute_nil star_token, "Expected STAR token"

    # Should have arith and term nodes
    arith_nodes = find_nodes_by_rule(ast, "arith")
    term_nodes = find_nodes_by_rule(ast, "term")
    assert arith_nodes.length >= 1, "Expected arith node for addition"
    assert term_nodes.length >= 1, "Expected term node for multiplication"
  end

  # ------------------------------------------------------------------
  # Function definition: def greet(name):
  # ------------------------------------------------------------------
  # Tests compound statement parsing. The grammar rule is:
  #   def_stmt = "def" NAME LPAREN [ parameters ] RPAREN COLON suite
  #   suite = simple_stmt | NEWLINE INDENT { statement } DEDENT

  def test_function_def
    source = "def greet(name):\n    return name\n"
    ast = parse(source)

    assert_equal "file", ast.rule_name

    # Should contain a def_stmt node
    def_stmts = find_nodes_by_rule(ast, "def_stmt")
    assert def_stmts.length >= 1, "Expected a def_stmt node"

    # The function name 'greet' should appear as a NAME token
    all_tokens = collect_tokens(ast)
    name_token = all_tokens.find { |t| t.type == TT::NAME && t.value == "greet" }
    refute_nil name_token, "Expected NAME token 'greet'"

    # The parameter 'name' should also appear
    param_token = all_tokens.find { |t| t.type == TT::NAME && t.value == "name" }
    refute_nil param_token, "Expected NAME token 'name' (parameter)"

    # The 'def' and 'return' keywords should appear
    keywords = all_tokens.select { |t| t.type == TT::KEYWORD }
    keyword_values = keywords.map(&:value)
    assert_includes keyword_values, "def"
    assert_includes keyword_values, "return"
  end

  # ------------------------------------------------------------------
  # If/else: conditional execution
  # ------------------------------------------------------------------
  # Tests the if_stmt grammar rule:
  #   if_stmt = "if" expression COLON suite
  #             { "elif" expression COLON suite }
  #             [ "else" COLON suite ]

  def test_if_else
    source = "if x:\n    y = 1\nelse:\n    y = 2\n"
    ast = parse(source)

    assert_equal "file", ast.rule_name

    # Should contain an if_stmt node
    if_stmts = find_nodes_by_rule(ast, "if_stmt")
    assert if_stmts.length >= 1, "Expected an if_stmt node"

    # The 'if' and 'else' keywords should appear
    all_tokens = collect_tokens(ast)
    keywords = all_tokens.select { |t| t.type == TT::KEYWORD }
    keyword_values = keywords.map(&:value)
    assert_includes keyword_values, "if"
    assert_includes keyword_values, "else"

    # Both 'y' assignments should appear (two NAME tokens with value 'y')
    y_tokens = all_tokens.select { |t| t.type == TT::NAME && t.value == "y" }
    assert_equal 2, y_tokens.length, "Expected 2 NAME tokens 'y' (one in each branch)"
  end

  # ------------------------------------------------------------------
  # For loop: iteration over a collection
  # ------------------------------------------------------------------
  # Tests the for_stmt grammar rule:
  #   for_stmt = "for" loop_vars "in" expression COLON suite
  #
  # Starlark only has for-loops (no while loops), which guarantees
  # termination when iterating over finite collections.

  def test_for_loop
    source = "for x in items:\n    pass\n"
    ast = parse(source)

    assert_equal "file", ast.rule_name

    # Should contain a for_stmt node
    for_stmts = find_nodes_by_rule(ast, "for_stmt")
    assert for_stmts.length >= 1, "Expected a for_stmt node"

    # The 'for', 'in', and 'pass' keywords should appear
    all_tokens = collect_tokens(ast)
    keywords = all_tokens.select { |t| t.type == TT::KEYWORD }
    keyword_values = keywords.map(&:value)
    assert_includes keyword_values, "for"
    assert_includes keyword_values, "in"
    assert_includes keyword_values, "pass"

    # The loop variable 'x' and iterable 'items' should appear
    name_values = all_tokens.select { |t| t.type == TT::NAME }.map(&:value)
    assert_includes name_values, "x"
    assert_includes name_values, "items"
  end

  # ------------------------------------------------------------------
  # Multiple statements: x = 1\ny = 2\nz = 3
  # ------------------------------------------------------------------
  # Tests that the parser handles multiple sequential statements,
  # separated by NEWLINE tokens.

  def test_multiple_statements
    source = "x = 1\ny = 2\nz = 3\n"
    ast = parse(source)

    assert_equal "file", ast.rule_name

    # Should have three statements
    statements = find_nodes_by_rule(ast, "statement")
    assert_equal 3, statements.length, "Expected 3 statements"

    # Each should contain an assign_stmt
    statements.each do |stmt|
      assign_stmts = find_nodes_by_rule(stmt, "assign_stmt")
      assert assign_stmts.length >= 1, "Expected assign_stmt in each statement"
    end
  end

  # ------------------------------------------------------------------
  # String assignment: name = "world"
  # ------------------------------------------------------------------

  def test_string_assignment
    ast = parse("name = \"world\"\n")

    all_tokens = collect_tokens(ast)
    string_token = all_tokens.find { |t| t.type == TT::STRING }
    refute_nil string_token, "Expected STRING token in assignment"
    assert_equal "world", string_token.value
  end

  # ------------------------------------------------------------------
  # Parenthesized expression: (1 + 2) * 3
  # ------------------------------------------------------------------

  def test_parenthesized_expression
    ast = parse("(1 + 2) * 3\n")
    refute_nil ast, "Expected successful parse with parentheses"

    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.type == TT::STAR }, "Expected STAR token"
    assert all_tokens.any? { |t| t.type == TT::PLUS }, "Expected PLUS token"
    assert all_tokens.any? { |t| t.type == TT::LPAREN }, "Expected LPAREN token"
    assert all_tokens.any? { |t| t.type == TT::RPAREN }, "Expected RPAREN token"
  end

  # ------------------------------------------------------------------
  # Grammar path resolution
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::StarlarkParser::STARLARK_GRAMMAR_PATH),
      "starlark.grammar file should exist at #{CodingAdventures::StarlarkParser::STARLARK_GRAMMAR_PATH}"
  end

  # ------------------------------------------------------------------
  # Root node is always 'file'
  # ------------------------------------------------------------------

  def test_root_is_file
    ast = parse("1\n")
    assert_equal "file", ast.rule_name
  end

  private

  # Recursively collect all Token objects from an AST.
  # Walks the entire tree depth-first, gathering every leaf token.
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

  # Find all ASTNode descendants with a given rule_name.
  # Useful for asserting that specific grammar rules were matched.
  def find_nodes_by_rule(node, rule_name)
    results = []
    return results unless node.is_a?(ASTNode)

    results << node if node.rule_name == rule_name
    node.children.each do |child|
      results.concat(find_nodes_by_rule(child, rule_name)) if child.is_a?(ASTNode)
    end
    results
  end
end
