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

  # -----------------------------------------------------------------------
  # Packrat memoization
  # -----------------------------------------------------------------------

  def test_memoization_same_result
    ast1 = parse("1 + 2 * 3")
    ast2 = parse("1 + 2 * 3")
    assert_equal ast1.rule_name, ast2.rule_name
    assert_equal ast1.children.length, ast2.children.length
  end

  def test_memoization_with_backtracking
    # This requires trying assignment first, failing, then expression_stmt.
    ast = parse("1 + 2")
    assert_equal "program", ast.rule_name
  end

  # -----------------------------------------------------------------------
  # String-based token types
  # -----------------------------------------------------------------------

  def test_string_type_token_matching
    grammar = GT.parse_parser_grammar("expr = INT ;")
    tokens = [
      CodingAdventures::Lexer::Token.new(type: "INT", value: "42", line: 1, column: 1),
      CodingAdventures::Lexer::Token.new(type: "EOF", value: "", line: 1, column: 3)
    ]
    parser = GDP.new(tokens, grammar)
    ast = parser.parse
    assert_equal "expr", ast.rule_name
    assert_equal 1, ast.children.length
  end

  def test_mixed_enum_and_string_types
    grammar = GT.parse_parser_grammar("expr = NAME INT ;")
    tokens = [
      CodingAdventures::Lexer::Token.new(type: TT::NAME, value: "x", line: 1, column: 1),
      CodingAdventures::Lexer::Token.new(type: "INT", value: "1", line: 1, column: 3),
      CodingAdventures::Lexer::Token.new(type: TT::EOF, value: "", line: 1, column: 4)
    ]
    parser = GDP.new(tokens, grammar)
    ast = parser.parse
    assert_equal 2, ast.children.length
  end

  # -----------------------------------------------------------------------
  # Significant newlines
  # -----------------------------------------------------------------------

  def test_grammar_with_newlines_significant
    grammar = GT.parse_parser_grammar("file = { NAME NEWLINE } ;")
    tokens = [
      CodingAdventures::Lexer::Token.new(type: TT::NAME, value: "x", line: 1, column: 1),
      CodingAdventures::Lexer::Token.new(type: TT::NEWLINE, value: "\\n", line: 1, column: 2),
      CodingAdventures::Lexer::Token.new(type: TT::EOF, value: "", line: 2, column: 1)
    ]
    parser = GDP.new(tokens, grammar)
    assert parser.newlines_significant
    ast = parser.parse
    assert_equal "file", ast.rule_name
  end

  def test_grammar_without_newlines_insignificant
    grammar = GT.parse_parser_grammar("expr = NUMBER ;")
    tokens = [
      CodingAdventures::Lexer::Token.new(type: TT::NEWLINE, value: "\\n", line: 1, column: 1),
      CodingAdventures::Lexer::Token.new(type: TT::NUMBER, value: "42", line: 2, column: 1),
      CodingAdventures::Lexer::Token.new(type: TT::EOF, value: "", line: 2, column: 3)
    ]
    parser = GDP.new(tokens, grammar)
    refute parser.newlines_significant
    ast = parser.parse
    assert_equal "expr", ast.rule_name
  end

  # -----------------------------------------------------------------------
  # Furthest failure tracking
  # -----------------------------------------------------------------------

  def test_furthest_failure_error_message
    # Test that furthest failure tracking gives useful errors. When the
    # parser gets partway through a rule before failing, the error should
    # mention what was expected at the furthest position reached.
    grammar = GT.parse_parser_grammar(<<~GRAMMAR)
      program = { statement } ;
      statement = assignment | expr_stmt ;
      assignment = NAME EQUALS NUMBER NEWLINE ;
      expr_stmt = NUMBER NEWLINE ;
    GRAMMAR
    tokens = [
      CodingAdventures::Lexer::Token.new(type: TT::NAME, value: "x", line: 1, column: 1),
      CodingAdventures::Lexer::Token.new(type: TT::NUMBER, value: "1", line: 1, column: 3),
      CodingAdventures::Lexer::Token.new(type: TT::EOF, value: "", line: 1, column: 4)
    ]
    error = assert_raises(P::GrammarParseError) do
      GDP.new(tokens, grammar).parse
    end
    # Furthest failure tracking should provide a useful error message.
    # The parser gets past NAME and then fails at position 1 (expecting
    # EQUALS), which is the furthest point reached.
    assert_includes error.message, "Expected"
  end

  # -----------------------------------------------------------------------
  # Starlark full pipeline
  # -----------------------------------------------------------------------

  GRAMMARS_DIR = File.join(__dir__, "..", "..", "..", "..", "..", "grammars")

  def starlark_grammar_obj
    path = File.join(GRAMMARS_DIR, "starlark.grammar")
    skip("starlark.grammar not found") unless File.exist?(path)
    GT.parse_parser_grammar(File.read(path))
  end

  def starlark_tokens_obj
    path = File.join(GRAMMARS_DIR, "starlark.tokens")
    skip("starlark.tokens not found") unless File.exist?(path)
    GT.parse_token_grammar(File.read(path))
  end

  def test_starlark_simple_assignment
    sg = starlark_grammar_obj
    st = starlark_tokens_obj
    tokens = CodingAdventures::Lexer::GrammarLexer.new("x = 1\n", st).tokenize
    ast = GDP.new(tokens, sg).parse
    assert_equal "file", ast.rule_name
  end

  def test_starlark_function_definition
    sg = starlark_grammar_obj
    st = starlark_tokens_obj
    source = "def add(x, y):\n    return x + y\n"
    tokens = CodingAdventures::Lexer::GrammarLexer.new(source, st).tokenize
    ast = GDP.new(tokens, sg).parse
    assert_equal "file", ast.rule_name
  end

  def test_starlark_if_else
    sg = starlark_grammar_obj
    st = starlark_tokens_obj
    source = "if x:\n    y = 1\nelse:\n    y = 2\n"
    tokens = CodingAdventures::Lexer::GrammarLexer.new(source, st).tokenize
    ast = GDP.new(tokens, sg).parse
    assert_equal "file", ast.rule_name
  end

  def test_starlark_list_literal
    sg = starlark_grammar_obj
    st = starlark_tokens_obj
    source = "x = [1, 2, 3]\n"
    tokens = CodingAdventures::Lexer::GrammarLexer.new(source, st).tokenize
    ast = GDP.new(tokens, sg).parse
    assert_equal "file", ast.rule_name
  end

  def test_starlark_for_loop
    sg = starlark_grammar_obj
    st = starlark_tokens_obj
    source = "for x in items:\n    pass\n"
    tokens = CodingAdventures::Lexer::GrammarLexer.new(source, st).tokenize
    ast = GDP.new(tokens, sg).parse
    assert_equal "file", ast.rule_name
  end
end
