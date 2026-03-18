# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the Python Parser
# ================================================================
#
# These tests verify that the grammar-driven parser, when loaded
# with python.grammar, correctly builds Abstract Syntax Trees from
# Python source code.
#
# The grammar-driven parser produces generic ASTNode objects:
#
#   ASTNode(rule_name: "program", children: [...])
#
# Each node records which grammar rule produced it and its matched
# children (which can be tokens or other ASTNodes). This is different
# from the hand-written parser's typed nodes (NumberLiteral, BinaryOp),
# but it captures the same structural information.
#
# The grammar being tested:
#
#   program      = { statement } ;
#   statement    = assignment | expression_stmt ;
#   assignment   = NAME EQUALS expression ;
#   expression_stmt = expression ;
#   expression   = term { ( PLUS | MINUS ) term } ;
#   term         = factor { ( STAR | SLASH ) factor } ;
#   factor       = NUMBER | STRING | NAME | LPAREN expression RPAREN ;
# ================================================================

class TestPythonParser < Minitest::Test
  ASTNode = CodingAdventures::Parser::ASTNode
  TT = CodingAdventures::Lexer::TokenType

  # ------------------------------------------------------------------
  # Helper
  # ------------------------------------------------------------------

  def parse(source)
    CodingAdventures::PythonParser.parse(source)
  end

  # ------------------------------------------------------------------
  # Basic assignment: x = 1 + 2
  # ------------------------------------------------------------------

  def test_simple_assignment
    ast = parse("x = 1 + 2")

    # Root should be a 'program' node
    assert_equal "program", ast.rule_name

    # Program has one statement child
    assert_equal 1, ast.children.count { |c| c.is_a?(ASTNode) }

    # The statement should be resolved as 'statement'
    stmt = ast.children.find { |c| c.is_a?(ASTNode) }
    assert_equal "statement", stmt.rule_name

    # Inside statement, find the assignment
    assignment = stmt.children.find { |c| c.is_a?(ASTNode) && c.rule_name == "assignment" }
    refute_nil assignment, "Expected an assignment node"
  end

  def test_assignment_has_name_and_expression
    ast = parse("x = 1 + 2")
    stmt = ast.children.find { |c| c.is_a?(ASTNode) }
    assignment = stmt.children.find { |c| c.is_a?(ASTNode) && c.rule_name == "assignment" }

    # Assignment children: NAME token, EQUALS token, expression node
    tokens = assignment.children.select { |c| c.is_a?(CodingAdventures::Lexer::Token) }
    name_token = tokens.find { |t| t.type == TT::NAME }
    equals_token = tokens.find { |t| t.type == TT::EQUALS }

    refute_nil name_token, "Expected NAME token in assignment"
    assert_equal "x", name_token.value
    refute_nil equals_token, "Expected EQUALS token in assignment"

    expr_node = assignment.children.find { |c| c.is_a?(ASTNode) && c.rule_name == "expression" }
    refute_nil expr_node, "Expected expression node in assignment"
  end

  # ------------------------------------------------------------------
  # Operator precedence: 1 + 2 * 3
  # ------------------------------------------------------------------

  def test_operator_precedence
    ast = parse("1 + 2 * 3")

    # Navigate to the expression
    stmt = ast.children.find { |c| c.is_a?(ASTNode) }
    expr_stmt = stmt.children.find { |c| c.is_a?(ASTNode) }
    expr = if expr_stmt.rule_name == "expression"
      expr_stmt
    else
      expr_stmt.children.find { |c| c.is_a?(ASTNode) && c.rule_name == "expression" }
    end

    refute_nil expr, "Expected an expression node"

    # The expression should contain a PLUS token (addition is at expression level)
    plus_token = expr.children.find { |c| c.is_a?(CodingAdventures::Lexer::Token) && c.type == TT::PLUS }
    refute_nil plus_token, "Expected PLUS at expression level"

    # There should be term nodes (multiplication is at term level)
    terms = expr.children.select { |c| c.is_a?(ASTNode) && c.rule_name == "term" }
    assert terms.length >= 2, "Expected at least 2 term nodes"

    # The second term should contain STAR (multiplication)
    second_term = terms[1]
    star_token = second_term.children.find { |c| c.is_a?(CodingAdventures::Lexer::Token) && c.type == TT::STAR }
    refute_nil star_token, "Expected STAR in second term (precedence)"
  end

  # ------------------------------------------------------------------
  # Parenthesized expression: (1 + 2) * 3
  # ------------------------------------------------------------------

  def test_parentheses
    ast = parse("(1 + 2) * 3")

    # Should parse without error -- the parentheses override precedence
    refute_nil ast, "Expected successful parse with parentheses"

    # Find the STAR token at the term level
    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.type == TT::STAR }, "Expected STAR token"
    assert all_tokens.any? { |t| t.type == TT::PLUS }, "Expected PLUS token"
    assert all_tokens.any? { |t| t.type == TT::LPAREN }, "Expected LPAREN token"
    assert all_tokens.any? { |t| t.type == TT::RPAREN }, "Expected RPAREN token"
  end

  # ------------------------------------------------------------------
  # Multiple statements: x = 1\ny = 2
  # ------------------------------------------------------------------

  def test_multiple_statements
    ast = parse("x = 1\ny = 2")

    assert_equal "program", ast.rule_name

    # Should have two statement children
    statements = ast.children.select { |c| c.is_a?(ASTNode) && c.rule_name == "statement" }
    assert_equal 2, statements.length, "Expected 2 statements"

    # Both should be assignments
    statements.each do |stmt|
      assignment = stmt.children.find { |c| c.is_a?(ASTNode) && c.rule_name == "assignment" }
      refute_nil assignment, "Expected assignment in each statement"
    end
  end

  # ------------------------------------------------------------------
  # Expression statement: just a number
  # ------------------------------------------------------------------

  def test_expression_statement
    ast = parse("42")

    stmt = ast.children.find { |c| c.is_a?(ASTNode) }
    assert_equal "statement", stmt.rule_name

    # Should be an expression_stmt, not an assignment
    expr_stmt = stmt.children.find { |c| c.is_a?(ASTNode) && c.rule_name == "expression_stmt" }
    refute_nil expr_stmt, "Expected expression_stmt for bare number"
  end

  # ------------------------------------------------------------------
  # Function call: print("hello")
  # ------------------------------------------------------------------
  # Note: The current grammar does not have a dedicated function_call rule.
  # print("hello") parses as: expression_stmt -> expression -> term -> factor
  # where factor matches NAME, then the LPAREN STRING RPAREN are separate
  # factors/terms in an expression. However, the grammar as written might
  # not parse this correctly since factor = NAME | ... | LPAREN expr RPAREN
  # and there's no call syntax. Let's test what actually happens.

  def test_function_call_syntax
    # The grammar treats print("hello") as: NAME followed by LPAREN expr RPAREN
    # Since there's no call rule, this will parse as two separate expression terms
    # or fail. Let's see what the grammar produces.
    # Actually the grammar is:
    #   expression = term { (PLUS|MINUS) term }
    #   term = factor { (STAR|SLASH) factor }
    #   factor = NUMBER | STRING | NAME | LPAREN expression RPAREN
    #
    # print("hello") would be: NAME LPAREN STRING RPAREN
    # factor matches NAME -> term done -> expression done -> statement done
    # Then LPAREN STRING RPAREN starts next statement:
    #   factor matches LPAREN expression RPAREN -> term -> expression -> statement
    #
    # So it should parse as two statements.
    ast = parse('print("hello")')
    assert_equal "program", ast.rule_name

    statements = ast.children.select { |c| c.is_a?(ASTNode) && c.rule_name == "statement" }
    assert statements.length >= 1, "Expected at least one statement"

    # Verify the name 'print' appears somewhere
    all_tokens = collect_tokens(ast)
    name_token = all_tokens.find { |t| t.value == "print" }
    refute_nil name_token, "Expected 'print' token in AST"

    # Verify the string 'hello' appears
    string_token = all_tokens.find { |t| t.value == "hello" }
    refute_nil string_token, "Expected 'hello' string token in AST"
  end

  # ------------------------------------------------------------------
  # Simple subtraction: 5 - 3
  # ------------------------------------------------------------------

  def test_subtraction
    ast = parse("5 - 3")

    stmt = ast.children.find { |c| c.is_a?(ASTNode) }
    refute_nil stmt

    all_tokens = collect_tokens(ast)
    minus_token = all_tokens.find { |t| t.type == TT::MINUS }
    refute_nil minus_token, "Expected MINUS token"
  end

  # ------------------------------------------------------------------
  # Division: 10 / 2
  # ------------------------------------------------------------------

  def test_division
    ast = parse("10 / 2")

    all_tokens = collect_tokens(ast)
    slash_token = all_tokens.find { |t| t.type == TT::SLASH }
    refute_nil slash_token, "Expected SLASH token"
  end

  # ------------------------------------------------------------------
  # String assignment: name = "world"
  # ------------------------------------------------------------------

  def test_string_assignment
    ast = parse('name = "world"')

    stmt = ast.children.find { |c| c.is_a?(ASTNode) }
    assignment = stmt.children.find { |c| c.is_a?(ASTNode) && c.rule_name == "assignment" }
    refute_nil assignment

    all_tokens = collect_tokens(assignment)
    string_token = all_tokens.find { |t| t.type == TT::STRING }
    refute_nil string_token, "Expected STRING token in assignment"
    assert_equal "world", string_token.value
  end

  # ------------------------------------------------------------------
  # Nested parentheses: ((1 + 2))
  # ------------------------------------------------------------------

  def test_nested_parentheses
    ast = parse("((1 + 2))")
    refute_nil ast, "Expected successful parse with nested parens"

    all_tokens = collect_tokens(ast)
    lparens = all_tokens.count { |t| t.type == TT::LPAREN }
    rparens = all_tokens.count { |t| t.type == TT::RPAREN }
    assert_equal 2, lparens, "Expected 2 LPAREN tokens"
    assert_equal 2, rparens, "Expected 2 RPAREN tokens"
  end

  # ------------------------------------------------------------------
  # Variable reference in expression: y = x
  # ------------------------------------------------------------------

  def test_variable_reference
    ast = parse("y = x")

    stmt = ast.children.find { |c| c.is_a?(ASTNode) }
    assignment = stmt.children.find { |c| c.is_a?(ASTNode) && c.rule_name == "assignment" }
    refute_nil assignment

    all_tokens = collect_tokens(assignment)
    names = all_tokens.select { |t| t.type == TT::NAME }
    assert_equal 2, names.length, "Expected 2 NAME tokens (target and value)"
    assert_equal "y", names[0].value
    assert_equal "x", names[1].value
  end

  # ------------------------------------------------------------------
  # Grammar path resolution
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::PythonParser::PYTHON_GRAMMAR_PATH),
      "python.grammar file should exist at #{CodingAdventures::PythonParser::PYTHON_GRAMMAR_PATH}"
  end

  # ------------------------------------------------------------------
  # Root node is always 'program'
  # ------------------------------------------------------------------

  def test_root_is_program
    ast = parse("1")
    assert_equal "program", ast.rule_name
  end

  # ------------------------------------------------------------------
  # Complex multi-line program
  # ------------------------------------------------------------------

  def test_multiline_program
    source = "x = 1\ny = 2\nz = x"
    ast = parse(source)

    statements = ast.children.select { |c| c.is_a?(ASTNode) && c.rule_name == "statement" }
    assert_equal 3, statements.length, "Expected 3 statements"
  end

  private

  # Recursively collect all Token objects from an AST
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
