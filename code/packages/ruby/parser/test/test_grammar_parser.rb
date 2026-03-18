# frozen_string_literal: true

require_relative "test_helper"

class TestGrammarParser < Minitest::Test
  P = CodingAdventures::Parser
  GDP = P::GrammarDrivenParser
  GT = CodingAdventures::GrammarTools
  TT = CodingAdventures::Lexer::TokenType
  Tokenizer = CodingAdventures::Lexer::Tokenizer

  PYTHON_GRAMMAR_SOURCE = <<~GRAMMAR
    program      = { statement } ;
    statement    = assignment | expression_stmt ;
    assignment   = NAME EQUALS expression ;
    expression_stmt = expression ;
    expression   = term { ( PLUS | MINUS ) term } ;
    term         = factor { ( STAR | SLASH ) factor } ;
    factor       = NUMBER | STRING | NAME | LPAREN expression RPAREN ;
  GRAMMAR

  def grammar
    GT.parse_parser_grammar(PYTHON_GRAMMAR_SOURCE)
  end

  def parse(source)
    tokens = Tokenizer.new(source).tokenize
    GDP.new(tokens, grammar).parse
  end

  # -----------------------------------------------------------------------
  # Basic parsing
  # -----------------------------------------------------------------------

  def test_single_number
    ast = parse("42")
    assert_kind_of P::ASTNode, ast
    assert_equal "program", ast.rule_name
    assert ast.children.length >= 1
  end

  def test_addition
    ast = parse("1 + 2")
    assert_equal "program", ast.rule_name
    # Should produce a tree with tokens for 1, +, and 2
  end

  def test_assignment
    ast = parse("x = 42\n")
    assert_equal "program", ast.rule_name
    # Should contain an assignment node
    statement = ast.children.find { |c| c.is_a?(P::ASTNode) && c.rule_name == "statement" }
    refute_nil statement
  end

  def test_multiple_statements
    ast = parse("x = 1\ny = 2\n")
    assert_equal "program", ast.rule_name
    statements = ast.children.select { |c| c.is_a?(P::ASTNode) }
    assert_equal 2, statements.length
  end

  def test_complex_expression
    ast = parse("1 + 2 * 3")
    assert_equal "program", ast.rule_name
  end

  def test_parenthesized_expression
    ast = parse("(1 + 2) * 3")
    assert_equal "program", ast.rule_name
  end

  def test_string_expression
    ast = parse('"hello"')
    assert_equal "program", ast.rule_name
  end

  def test_name_expression
    ast = parse("x")
    assert_equal "program", ast.rule_name
  end

  # -----------------------------------------------------------------------
  # ASTNode properties
  # -----------------------------------------------------------------------

  def test_ast_node_leaf
    token = CodingAdventures::Lexer::Token.new(type: TT::NUMBER, value: "42", line: 1, column: 1)
    node = P::ASTNode.new(rule_name: "factor", children: [token])
    assert node.leaf?
    assert_equal token, node.token
  end

  def test_ast_node_non_leaf
    inner = P::ASTNode.new(rule_name: "factor", children: [])
    node = P::ASTNode.new(rule_name: "expression", children: [inner])
    refute node.leaf?
    assert_nil node.token
  end

  # -----------------------------------------------------------------------
  # Error handling
  # -----------------------------------------------------------------------

  def test_no_rules
    empty_grammar = GT::ParserGrammar.new(rules: [])
    tokens = Tokenizer.new("42").tokenize
    error = assert_raises(P::GrammarParseError) do
      GDP.new(tokens, empty_grammar).parse
    end
    assert_includes error.message, "no rules"
  end

  def test_undefined_rule
    bad_grammar = GT.parse_parser_grammar("program = nonexistent ;")
    tokens = Tokenizer.new("42").tokenize
    assert_raises(P::GrammarParseError) do
      GDP.new(tokens, bad_grammar).parse
    end
  end

  def test_unconsumed_tokens
    # Parse a grammar that only handles one expression, then give it two
    # separated by something the grammar can't handle
    simple_grammar = GT.parse_parser_grammar("program = NUMBER ;")
    tokens = Tokenizer.new("42 99").tokenize
    assert_raises(P::GrammarParseError) do
      GDP.new(tokens, simple_grammar).parse
    end
  end

  # -----------------------------------------------------------------------
  # Empty program
  # -----------------------------------------------------------------------

  def test_empty_program
    ast = parse("")
    assert_equal "program", ast.rule_name
    assert_equal 0, ast.children.length
  end
end
