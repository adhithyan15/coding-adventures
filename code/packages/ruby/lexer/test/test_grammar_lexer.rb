# frozen_string_literal: true

require_relative "test_helper"

class TestGrammarLexer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType
  GL = CodingAdventures::Lexer::GrammarLexer
  GT = CodingAdventures::GrammarTools

  def make_grammar(source)
    GT.parse_token_grammar(source)
  end

  def simple_grammar
    make_grammar(<<~TOKENS)
      NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
      NUMBER = /[0-9]+/
      STRING = /"([^"\\\\]|\\\\.)*"/
      EQUALS_EQUALS = "=="
      EQUALS = "="
      PLUS   = "+"
      MINUS  = "-"
      STAR   = "*"
      SLASH  = "/"
      LPAREN = "("
      RPAREN = ")"
      COMMA  = ","
      COLON  = ":"

      keywords:
        if
        else
    TOKENS
  end

  # -----------------------------------------------------------------------
  # Basic tokenization
  # -----------------------------------------------------------------------

  def test_empty_source
    tokens = GL.new("", simple_grammar).tokenize
    assert_equal 1, tokens.length
    assert_equal TT::EOF, tokens[0].type
  end

  def test_single_number
    tokens = GL.new("42", simple_grammar).tokenize
    assert_equal TT::NUMBER, tokens[0].type
    assert_equal "42", tokens[0].value
  end

  def test_single_name
    tokens = GL.new("hello", simple_grammar).tokenize
    assert_equal TT::NAME, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  def test_operators
    tokens = GL.new("+ - * /", simple_grammar).tokenize
    types = tokens.map(&:type)
    assert_equal [TT::PLUS, TT::MINUS, TT::STAR, TT::SLASH, TT::EOF], types
  end

  def test_equals_vs_equals_equals
    tokens = GL.new("= ==", simple_grammar).tokenize
    assert_equal TT::EQUALS, tokens[0].type
    assert_equal TT::EQUALS_EQUALS, tokens[1].type
  end

  def test_string_literal
    tokens = GL.new('"hello"', simple_grammar).tokenize
    assert_equal TT::STRING, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  def test_string_with_escapes
    tokens = GL.new('"line1\\nline2"', simple_grammar).tokenize
    assert_equal "line1\nline2", tokens[0].value
  end

  # -----------------------------------------------------------------------
  # Keywords
  # -----------------------------------------------------------------------

  def test_keyword_detection
    tokens = GL.new("if", simple_grammar).tokenize
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "if", tokens[0].value
  end

  def test_non_keyword
    tokens = GL.new("iffy", simple_grammar).tokenize
    assert_equal TT::NAME, tokens[0].type
  end

  # -----------------------------------------------------------------------
  # Newlines
  # -----------------------------------------------------------------------

  def test_newline_handling
    tokens = GL.new("x\ny", simple_grammar).tokenize
    assert_equal TT::NAME, tokens[0].type
    assert_equal TT::NEWLINE, tokens[1].type
    assert_equal TT::NAME, tokens[2].type
  end

  # -----------------------------------------------------------------------
  # Position tracking
  # -----------------------------------------------------------------------

  def test_position_tracking
    tokens = GL.new("x = 1", simple_grammar).tokenize
    assert_equal 1, tokens[0].line
    assert_equal 1, tokens[0].column
    assert_equal 1, tokens[1].line
    assert_equal 3, tokens[1].column
  end

  # -----------------------------------------------------------------------
  # Complete expressions
  # -----------------------------------------------------------------------

  def test_assignment
    tokens = GL.new("x = 1 + 2", simple_grammar).tokenize
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS, TT::NUMBER, TT::PLUS, TT::NUMBER, TT::EOF], types
  end

  # -----------------------------------------------------------------------
  # Error handling
  # -----------------------------------------------------------------------

  def test_unexpected_character
    error = assert_raises(CodingAdventures::Lexer::LexerError) do
      GL.new("@", simple_grammar).tokenize
    end
    assert_includes error.message, "Unexpected"
  end

  # -----------------------------------------------------------------------
  # Fallback token type
  # -----------------------------------------------------------------------

  def test_unknown_token_name_uses_string_type
    grammar = make_grammar('CUSTOM = /[a-z]+/')
    tokens = GL.new("hello", grammar).tokenize
    # CUSTOM is not in TokenType::ALL so uses the grammar name as a string
    assert_equal "CUSTOM", tokens[0].type
  end

  # -----------------------------------------------------------------------
  # Delimiters
  # -----------------------------------------------------------------------

  def test_delimiters
    tokens = GL.new("( ) , :", simple_grammar).tokenize
    types = tokens.map(&:type)
    assert_equal [TT::LPAREN, TT::RPAREN, TT::COMMA, TT::COLON, TT::EOF], types
  end

  # -----------------------------------------------------------------------
  # Skip patterns
  # -----------------------------------------------------------------------

  def test_skip_whitespace
    grammar = GT.parse_token_grammar(<<~TOKENS)
      NAME = /[a-z]+/
      skip:
        WHITESPACE = /[ \\t]+/
    TOKENS
    tokens = GL.new("hello world", grammar).tokenize
    types = tokens.map(&:type)
    assert_equal %w[NAME NAME EOF], types
  end

  def test_skip_comments
    grammar = GT.parse_token_grammar(<<~TOKENS)
      NAME = /[a-z]+/
      skip:
        WHITESPACE = /[ \\t]+/
        COMMENT = /#[^\\n]*/
    TOKENS
    tokens = GL.new("hello # a comment\nworld", grammar).tokenize
    names = tokens.reject { |t| t.type == TT::NEWLINE || t.type == TT::EOF }
    assert_equal %w[hello world], names.map(&:value)
  end

  # -----------------------------------------------------------------------
  # Aliases
  # -----------------------------------------------------------------------

  def test_alias_regex
    grammar = GT.parse_token_grammar('NUM = /[0-9]+/ -> INT')
    tokens = GL.new("42", grammar).tokenize
    assert_equal "INT", tokens[0].type
  end

  def test_alias_multiple
    grammar = GT.parse_token_grammar(<<~TOKENS)
      STRING_DQ = /"[^"]*"/ -> STRING
      STRING_SQ = /'[^']*'/ -> STRING
    TOKENS
    tokens = GL.new('"hello"', grammar).tokenize
    assert_equal TT::STRING, tokens[0].type
  end

  # -----------------------------------------------------------------------
  # Reserved keywords
  # -----------------------------------------------------------------------

  def test_reserved_keyword_raises
    grammar = GT.parse_token_grammar(<<~TOKENS)
      NAME = /[a-zA-Z_][a-zA-Z0-9_]*/
      reserved:
        class
        import
    TOKENS
    error = assert_raises(CodingAdventures::Lexer::LexerError) do
      GL.new("class", grammar).tokenize
    end
    assert_includes error.message, "Reserved keyword"
  end

  def test_reserved_non_reserved_passes
    grammar = GT.parse_token_grammar(<<~TOKENS)
      NAME = /[a-zA-Z_][a-zA-Z0-9_]*/
      reserved:
        class
    TOKENS
    tokens = GL.new("hello", grammar).tokenize
    assert_equal TT::NAME, tokens[0].type
  end

  # -----------------------------------------------------------------------
  # Indentation mode
  # -----------------------------------------------------------------------

  def starlark_grammar
    GT.parse_token_grammar(<<~TOKENS)
      mode: indentation
      NAME = /[a-zA-Z_][a-zA-Z0-9_]*/
      INT = /[0-9]+/
      EQUALS = "="
      PLUS = "+"
      COLON = ":"
      LPAREN = "("
      RPAREN = ")"
      LBRACKET = "["
      RBRACKET = "]"
      COMMA = ","

      keywords:
        def
        return
        if
        else
        for
        in
        pass

      skip:
        WHITESPACE = /[ \\t]+/
        COMMENT = /#[^\\n]*/
    TOKENS
  end

  def test_indentation_simple
    tokens = GL.new("x = 1\n", starlark_grammar).tokenize
    types = tokens.map(&:type)
    assert_includes types, TT::NEWLINE
    assert_includes types, TT::EOF
    refute_includes types, "INDENT"
  end

  def test_indentation_indent_dedent
    source = "if x:\n    y = 1\n"
    tokens = GL.new(source, starlark_grammar).tokenize
    types = tokens.map(&:type)
    assert_includes types, "INDENT"
    assert_includes types, "DEDENT"
  end

  def test_indentation_nested
    source = "if x:\n    if y:\n        z = 1\n"
    tokens = GL.new(source, starlark_grammar).tokenize
    types = tokens.map(&:type)
    indent_count = types.count("INDENT")
    dedent_count = types.count("DEDENT")
    assert_equal 2, indent_count
    assert_equal 2, dedent_count
  end

  def test_indentation_blank_lines_skipped
    source = "if x:\n    y = 1\n\n    z = 2\n"
    tokens = GL.new(source, starlark_grammar).tokenize
    # Blank line should not produce extra NEWLINE/DEDENT
    indent_count = tokens.map(&:type).count("INDENT")
    assert_equal 1, indent_count
  end

  def test_indentation_comment_lines_skipped
    source = "if x:\n    # comment\n    y = 1\n"
    tokens = GL.new(source, starlark_grammar).tokenize
    # Comment line should not affect indentation
    indent_count = tokens.map(&:type).count("INDENT")
    assert_equal 1, indent_count
  end

  def test_indentation_implicit_line_joining
    source = "x = (1 +\n    2)\n"
    tokens = GL.new(source, starlark_grammar).tokenize
    types = tokens.map(&:type)
    # Inside brackets, NEWLINE should be suppressed
    refute_includes types, "INDENT"
  end

  def test_indentation_tab_raises
    error = assert_raises(CodingAdventures::Lexer::LexerError) do
      GL.new("if x:\n\ty = 1\n", starlark_grammar).tokenize
    end
    assert_includes error.message, "Tab"
  end

  def test_indentation_empty_input
    tokens = GL.new("", starlark_grammar).tokenize
    types = tokens.map(&:type)
    assert_includes types, TT::EOF
  end

  def test_indentation_function_def
    source = "def add(x, y):\n    return x + y\n"
    tokens = GL.new(source, starlark_grammar).tokenize
    types = tokens.map(&:type)
    assert_includes types, "INDENT"
    assert_includes types, "DEDENT"
    values = tokens.map(&:value)
    assert_includes values, "def"
    assert_includes values, "return"
  end

  # -----------------------------------------------------------------------
  # Starlark integration
  # -----------------------------------------------------------------------

  def test_starlark_reserved_keyword
    grammar = GT.parse_token_grammar(<<~TOKENS)
      mode: indentation
      NAME = /[a-zA-Z_][a-zA-Z0-9_]*/
      reserved:
        class
      skip:
        WHITESPACE = /[ \\t]+/
    TOKENS
    error = assert_raises(CodingAdventures::Lexer::LexerError) do
      GL.new("class\n", grammar).tokenize
    end
    assert_includes error.message, "Reserved"
  end
end
