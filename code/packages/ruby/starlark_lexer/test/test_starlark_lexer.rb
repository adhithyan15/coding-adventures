# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the Starlark Lexer
# ================================================================
#
# These tests verify that the grammar-driven lexer, when loaded
# with starlark.tokens, correctly tokenizes Starlark source code.
#
# Starlark is a deterministic subset of Python designed for build
# configuration files (Bazel BUILD files, etc.). The key differences
# from Python that affect lexing:
#
#   1. Different keyword set: Starlark has 'load', 'lambda', 'in',
#      'not', 'and', 'or', but lacks 'while', 'class', 'import', etc.
#   2. Reserved words: Python keywords not in Starlark cause errors.
#   3. Indentation mode: INDENT/DEDENT/NEWLINE tokens for blocks.
#
# We are not testing the lexer engine itself (that is tested in the
# lexer gem) -- we are testing that the Starlark grammar file
# correctly describes Starlark's lexical rules.
# ================================================================

class TestStarlarkLexer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType

  # ---------------------------------------------------------------
  # Starlark defines many token types beyond the base TokenType enum.
  # The GrammarLexer falls back to using raw string names for types
  # not in TokenType::ALL (e.g., "NOT_EQUALS", "INDENT", "FLOAT").
  # These constants make tests readable while matching the actual
  # string types emitted by the grammar-driven lexer.
  # ---------------------------------------------------------------
  NOT_EQUALS     = "NOT_EQUALS"
  LESS_EQUALS    = "LESS_EQUALS"
  GREATER_EQUALS = "GREATER_EQUALS"
  PLUS_EQUALS    = "PLUS_EQUALS"
  MINUS_EQUALS   = "MINUS_EQUALS"
  STAR_EQUALS    = "STAR_EQUALS"
  PERCENT        = "PERCENT"
  INT_TYPE       = "INT"
  FLOAT_TYPE     = "FLOAT"
  INDENT_TYPE    = "INDENT"
  DEDENT_TYPE    = "DEDENT"

  # ------------------------------------------------------------------
  # Helper: tokenize source and provide convenient accessors
  # ------------------------------------------------------------------

  def tokenize(source)
    CodingAdventures::StarlarkLexer.tokenize(source)
  end

  def token_types(source)
    tokenize(source).map(&:type)
  end

  def token_values(source)
    tokenize(source).map(&:value)
  end

  # ------------------------------------------------------------------
  # Basic arithmetic: x = 1 + 2
  # ------------------------------------------------------------------
  # This is the simplest possible Starlark statement: a variable
  # assignment with an arithmetic expression. The lexer should
  # produce NAME EQUALS NUMBER PLUS NUMBER NEWLINE EOF.

  def test_simple_expression
    tokens = tokenize("x = 1 + 2")
    types = tokens.map(&:type)
    # In indentation mode, every logical line ends with NEWLINE before EOF
    assert_equal [TT::NAME, TT::EQUALS, INT_TYPE, TT::PLUS, INT_TYPE, TT::NEWLINE, TT::EOF], types
  end

  def test_simple_expression_values
    tokens = tokenize("x = 1 + 2")
    values = tokens.map(&:value)
    assert_equal "x", values[0]
    assert_equal "=", values[1]
    assert_equal "1", values[2]
    assert_equal "+", values[3]
    assert_equal "2", values[4]
  end

  # ------------------------------------------------------------------
  # Starlark keywords
  # ------------------------------------------------------------------
  # Starlark has a specific set of keywords. These are NAME tokens
  # that get reclassified as KEYWORD by the lexer because they appear
  # in the keywords: section of starlark.tokens.
  #
  # The full keyword list: and, break, continue, def, elif, else,
  # for, if, in, lambda, load, not, or, pass, return, True, False, None

  def test_keywords
    keywords = %w[def return if else elif for in lambda load and or not break continue pass True False None]
    keywords.each do |kw|
      tokens = tokenize(kw)
      assert_equal TT::KEYWORD, tokens[0].type, "Expected '#{kw}' to be a KEYWORD"
      assert_equal kw, tokens[0].value
    end
  end

  def test_keyword_def
    tokens = tokenize("def")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "def", tokens[0].value
  end

  def test_keyword_return
    tokens = tokenize("return")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "return", tokens[0].value
  end

  def test_keyword_if
    tokens = tokenize("if")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "if", tokens[0].value
  end

  def test_keyword_else
    tokens = tokenize("else")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "else", tokens[0].value
  end

  def test_keyword_for
    tokens = tokenize("for")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "for", tokens[0].value
  end

  def test_keyword_lambda
    tokens = tokenize("lambda")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "lambda", tokens[0].value
  end

  def test_keyword_load
    tokens = tokenize("load")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "load", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Reserved keywords
  # ------------------------------------------------------------------
  # Starlark reserves several Python keywords that are not part of
  # the Starlark language. Using them should raise an error because
  # they would be confusing -- a user might think they're writing
  # valid Python when they're actually in Starlark.
  #
  # Reserved words: as, assert, async, await, class, del, except,
  # finally, from, global, import, is, nonlocal, raise, try, while,
  # with, yield

  def test_reserved_keywords
    reserved = %w[class import while try except finally raise with yield as assert async await del from global is nonlocal]
    reserved.each do |kw|
      assert_raises(StandardError, "Expected '#{kw}' to raise an error (it is reserved in Starlark)") do
        tokenize(kw)
      end
    end
  end

  # ------------------------------------------------------------------
  # Indentation: INDENT/DEDENT from blocks
  # ------------------------------------------------------------------
  # Starlark uses significant indentation like Python. When the lexer
  # encounters increased indentation, it emits an INDENT token. When
  # indentation decreases, it emits one or more DEDENT tokens.
  #
  # For example:
  #   def foo():        KEYWORD NAME LPAREN RPAREN COLON NEWLINE
  #       x = 1         INDENT NAME EQUALS NUMBER NEWLINE
  #   (end of block)    DEDENT

  def test_indentation
    source = "def foo():\n    x = 1\n"
    tokens = tokenize(source)
    types = tokens.map(&:type)

    # Should contain INDENT and DEDENT tokens
    assert_includes types, INDENT_TYPE, "Expected INDENT token for indented block"
    assert_includes types, DEDENT_TYPE, "Expected DEDENT token when block ends"

    # INDENT should come after the colon+newline
    indent_idx = types.index(INDENT_TYPE)
    colon_idx = types.index(TT::COLON)
    assert indent_idx > colon_idx, "INDENT should come after COLON"
  end

  # ------------------------------------------------------------------
  # Operators: multi-character operators
  # ------------------------------------------------------------------
  # Starlark supports a rich set of operators. Multi-character operators
  # like ==, !=, <=, >=, **, //, <<, >> must be matched before their
  # single-character prefixes. The starlark.tokens file achieves this
  # by listing multi-character operators first (first-match-wins).

  def test_operators
    # Test equality operator
    tokens = tokenize("x == y")
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS_EQUALS, TT::NAME, TT::NEWLINE, TT::EOF], types

    # Test not-equals operator
    tokens = tokenize("x != y")
    assert_equal NOT_EQUALS, tokens[1].type

    # Test less-than-or-equal
    tokens = tokenize("x <= y")
    assert_equal LESS_EQUALS, tokens[1].type

    # Test greater-than-or-equal
    tokens = tokenize("x >= y")
    assert_equal GREATER_EQUALS, tokens[1].type
  end

  def test_augmented_assignment_operators
    # Test += operator
    tokens = tokenize("x += 1")
    assert_equal PLUS_EQUALS, tokens[1].type

    # Test -= operator
    tokens = tokenize("x -= 1")
    assert_equal MINUS_EQUALS, tokens[1].type

    # Test *= operator
    tokens = tokenize("x *= 2")
    assert_equal STAR_EQUALS, tokens[1].type
  end

  def test_single_char_operators
    # Plus
    tokens = tokenize("a + b")
    assert_equal TT::PLUS, tokens[1].type

    # Minus
    tokens = tokenize("a - b")
    assert_equal TT::MINUS, tokens[1].type

    # Star (multiply)
    tokens = tokenize("a * b")
    assert_equal TT::STAR, tokens[1].type

    # Slash (divide)
    tokens = tokenize("a / b")
    assert_equal TT::SLASH, tokens[1].type

    # Percent (modulo)
    tokens = tokenize("a % b")
    assert_equal PERCENT, tokens[1].type
  end

  # ------------------------------------------------------------------
  # Strings: string literals
  # ------------------------------------------------------------------
  # Starlark supports both single-quoted and double-quoted strings,
  # as well as triple-quoted strings for multi-line content. The
  # lexer should produce STRING tokens for all of these.

  def test_strings
    tokens = tokenize('"hello"')
    assert_equal TT::STRING, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  def test_string_with_spaces
    tokens = tokenize('"Hello, World!"')
    assert_equal TT::STRING, tokens[0].type
    assert_equal "Hello, World!", tokens[0].value
  end

  def test_single_quoted_string
    tokens = tokenize("'hello'")
    assert_equal TT::STRING, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Comments: comments are skipped
  # ------------------------------------------------------------------
  # In Starlark, comments start with # and run to the end of the line.
  # The lexer should skip comments entirely -- they produce no tokens.
  # This is handled by the skip: section in starlark.tokens.

  def test_comments
    tokens = tokenize("x = 1  # this is a comment")
    types = tokens.map(&:type)
    # The comment should be completely absent from the token stream
    refute types.include?(:COMMENT), "Comments should be skipped, not tokenized"
    # The meaningful tokens should still be there
    assert_equal TT::NAME, tokens[0].type
    assert_equal TT::EQUALS, tokens[1].type
    assert_equal INT_TYPE, tokens[2].type
  end

  # ------------------------------------------------------------------
  # Numbers: integer and float literals
  # ------------------------------------------------------------------

  def test_integer
    tokens = tokenize("42")
    assert_equal INT_TYPE, tokens[0].type
    assert_equal "42", tokens[0].value
  end

  def test_float
    tokens = tokenize("3.14")
    # Starlark has FLOAT as a separate token type from INT
    # The token type depends on the grammar; it might be NUMBER or FLOAT
    assert [INT_TYPE, FLOAT_TYPE].include?(tokens[0].type),
      "Expected NUMBER or FLOAT token for 3.14"
  end

  # ------------------------------------------------------------------
  # Delimiters: parentheses, brackets, braces, comma, colon
  # ------------------------------------------------------------------

  def test_parentheses
    tokens = tokenize("(x)")
    types = tokens.map(&:type)
    assert_equal [TT::LPAREN, TT::NAME, TT::RPAREN, TT::NEWLINE, TT::EOF], types
  end

  def test_brackets
    tokens = tokenize("[x]")
    types = tokens.map(&:type)
    assert_equal [TT::LBRACKET, TT::NAME, TT::RBRACKET, TT::NEWLINE, TT::EOF], types
  end

  def test_braces
    tokens = tokenize("{}")
    types = tokens.map(&:type)
    assert_equal [TT::LBRACE, TT::RBRACE, TT::NEWLINE, TT::EOF], types
  end

  def test_comma
    tokens = tokenize("a, b")
    assert_equal TT::COMMA, tokens[1].type
  end

  def test_colon
    tokens = tokenize("if x:")
    assert_equal TT::COLON, tokens[2].type
  end

  # ------------------------------------------------------------------
  # Non-keyword names are NAME tokens, not KEYWORD
  # ------------------------------------------------------------------

  def test_name_not_keyword
    tokens = tokenize("foobar")
    assert_equal TT::NAME, tokens[0].type
    assert_equal "foobar", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Equals vs equals-equals disambiguation
  # ------------------------------------------------------------------
  # The lexer must correctly distinguish = (assignment) from ==
  # (comparison). This is a first-match-wins rule: == is listed
  # before = in starlark.tokens, so "==" matches first.

  def test_equals_vs_equals_equals
    source = "x = y == z"
    tokens = tokenize(source)
    types = tokens.map(&:type)
    expected = [TT::NAME, TT::EQUALS, TT::NAME, TT::EQUALS_EQUALS, TT::NAME, TT::NEWLINE, TT::EOF]
    assert_equal expected, types
  end

  # ------------------------------------------------------------------
  # Multi-line Starlark code
  # ------------------------------------------------------------------

  def test_multiline
    source = "x = 1\ny = 2"
    tokens = tokenize(source)
    types = tokens.map(&:type)
    # x = 1 NEWLINE y = 2 NEWLINE EOF
    expected = [
      TT::NAME, TT::EQUALS, INT_TYPE,
      TT::NEWLINE,
      TT::NAME, TT::EQUALS, INT_TYPE,
      TT::NEWLINE,
      TT::EOF
    ]
    assert_equal expected, types
  end

  # ------------------------------------------------------------------
  # Line tracking
  # ------------------------------------------------------------------

  def test_line_tracking
    source = "x = 1\ny = 2"
    tokens = tokenize(source)
    # 'y' should be on line 2
    y_token = tokens.find { |t| t.value == "y" }
    assert_equal 2, y_token.line
  end

  # ------------------------------------------------------------------
  # Grammar path resolution
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::StarlarkLexer::STARLARK_TOKENS_PATH),
      "starlark.tokens file should exist at #{CodingAdventures::StarlarkLexer::STARLARK_TOKENS_PATH}"
  end

  # ------------------------------------------------------------------
  # Function definition syntax
  # ------------------------------------------------------------------

  def test_def_function
    source = "def foo(x):"
    tokens = tokenize(source)
    types = tokens.map(&:type)
    expected = [
      TT::KEYWORD, TT::NAME, TT::LPAREN, TT::NAME, TT::RPAREN,
      TT::COLON, TT::NEWLINE, TT::EOF
    ]
    assert_equal expected, types
    assert_equal "def", tokens[0].value
    assert_equal "foo", tokens[1].value
  end
end
