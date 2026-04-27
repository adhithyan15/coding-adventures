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
  # Layout mode (Haskell-style prototype)
  # -----------------------------------------------------------------------

  def haskell_layout_grammar
    GT.parse_token_grammar(<<~TOKENS)
      mode: layout
      NAME = /[a-zA-Z_][a-zA-Z0-9_]*/
      NUMBER = /[0-9]+/
      EQUALS = "="
      SEMICOLON = ";"
      LBRACE = "{"
      RBRACE = "}"
      LPAREN = "("
      RPAREN = ")"

      keywords:
        let
        in
        where
        do
        of

      layout_keywords:
        let
        where
        do
        of

      skip:
        WHITESPACE = /[ \\t\\r]+/
    TOKENS
  end

  def filtered_layout_tokens(source)
    GL.new(source, haskell_layout_grammar).tokenize.reject do |token|
      %w[NEWLINE EOF].include?(token.type_name)
    end
  end

  def test_layout_mode_inserts_virtual_tokens_for_let_block
    tokens = filtered_layout_tokens(<<~HS)
      let
        x = 1
        y = 2
      in x
    HS

    assert_equal [
      "KEYWORD:let",
      "VIRTUAL_LBRACE:{",
      "NAME:x",
      "EQUALS:=",
      "NUMBER:1",
      "VIRTUAL_SEMICOLON:;",
      "NAME:y",
      "EQUALS:=",
      "NUMBER:2",
      "VIRTUAL_RBRACE:}",
      "KEYWORD:in",
      "NAME:x"
    ], tokens.map { |token| "#{token.type_name}:#{token.value}" }
  end

  def test_layout_mode_skips_virtual_open_when_explicit_brace_present
    tokens = filtered_layout_tokens("let { x = 1; y = 2 } in x\n")
    types = tokens.map(&:type_name)

    refute_includes types, "VIRTUAL_LBRACE"
    refute_includes types, "VIRTUAL_SEMICOLON"
    refute_includes types, "VIRTUAL_RBRACE"
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

  # -----------------------------------------------------------------------
  # Case-insensitive keyword matching
  # -----------------------------------------------------------------------
  #
  # When the grammar declares `# @case_insensitive true`, keywords are
  # stored in uppercase and incoming NAME tokens are compared against the
  # uppercase set. The emitted KEYWORD token always carries the uppercased
  # value, regardless of how the identifier was written in the source.
  #
  # Default grammars (no magic comment) are case-sensitive: "SELECT" is
  # just a NAME when "select" is the declared keyword.
  # -----------------------------------------------------------------------

  # Helper that builds a case-insensitive grammar with "select" as a keyword.
  def case_insensitive_grammar
    make_grammar(<<~TOKENS)
      # @case_insensitive true
      NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
      NUMBER = /[0-9]+/

      keywords:
        select
        from
    TOKENS
  end

  # Lowercase input matches the keyword and is normalised to uppercase.
  def test_case_insensitive_lowercase_keyword
    tokens = GL.new("select", case_insensitive_grammar).tokenize
    kw = tokens[0]
    assert_equal TT::KEYWORD, kw.type,
      "expected KEYWORD but got #{kw.type}"
    assert_equal "SELECT", kw.value,
      "expected value 'SELECT' but got '#{kw.value}'"
  end

  # Uppercase input also matches and is emitted as uppercase.
  def test_case_insensitive_uppercase_keyword
    tokens = GL.new("SELECT", case_insensitive_grammar).tokenize
    kw = tokens[0]
    assert_equal TT::KEYWORD, kw.type,
      "expected KEYWORD but got #{kw.type}"
    assert_equal "SELECT", kw.value,
      "expected value 'SELECT' but got '#{kw.value}'"
  end

  # Mixed-case input matches and is normalised to uppercase.
  def test_case_insensitive_mixed_case_keyword
    tokens = GL.new("Select", case_insensitive_grammar).tokenize
    kw = tokens[0]
    assert_equal TT::KEYWORD, kw.type,
      "expected KEYWORD but got #{kw.type}"
    assert_equal "SELECT", kw.value,
      "expected value 'SELECT' but got '#{kw.value}'"
  end

  # Without the magic comment, keyword matching is case-sensitive.
  # "SELECT" (all-caps) does not match the lowercase keyword "select".
  def test_case_sensitive_default_no_match
    grammar = make_grammar(<<~TOKENS)
      NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
      keywords:
        select
    TOKENS
    tokens = GL.new("SELECT", grammar).tokenize
    assert_equal TT::NAME, tokens[0].type,
      "expected NAME (case-sensitive mode) but got #{tokens[0].type}"
  end

  # Non-keyword identifiers in a case-insensitive grammar retain their
  # original casing in the emitted NAME token.
  def test_case_insensitive_non_keyword_preserves_case
    tokens = GL.new("myTable", case_insensitive_grammar).tokenize
    tok = tokens[0]
    assert_equal TT::NAME, tok.type,
      "expected NAME but got #{tok.type}"
    assert_equal "myTable", tok.value,
      "expected original value 'myTable' but got '#{tok.value}'"
  end
end

# =========================================================================
# Helper -- Build a Grammar with Pattern Groups for Testing
# =========================================================================
#
# This creates a simplified XML-like grammar with two pattern groups:
#
# - Default group: TEXT (any non-< characters) and OPEN_TAG (<)
# - tag group: TAG_NAME (identifiers), EQUALS (=), VALUE (quoted),
#   TAG_CLOSE (>)
#
# Skip patterns handle whitespace globally. The "escapes: none"
# directive disables escape processing in STRING values, since
# VALUE patterns here use quotes for attribute values, not strings.
# =========================================================================

def make_group_grammar
  CodingAdventures::GrammarTools.parse_token_grammar(<<~TOKENS)
    escapes: none

    skip:
      WS = /[ \\t\\r\\n]+/

    TEXT      = /[^<]+/
    OPEN_TAG  = "<"

    group tag:
      TAG_NAME  = /[a-zA-Z_][a-zA-Z0-9_]*/
      EQUALS    = "="
      VALUE     = /"[^"]*"/
      TAG_CLOSE = ">"
  TOKENS
end

# Helper to normalize a token type to its string name.
#
# Token types can be either a string (e.g., "TAG_NAME") or a constant
# from TokenType (e.g., TokenType::EQUALS). This normalizes both to a
# plain string for easy assertion comparison.
def token_type_name(token)
  token.type.to_s
end

# =========================================================================
# TestLexerContext -- Unit Tests for the LexerContext API
# =========================================================================
#
# These tests verify that each LexerContext method correctly records
# actions without immediately mutating lexer state. The actions are
# applied by the tokenizer's main loop after the callback returns.
# =========================================================================

class TestLexerContext < Minitest::Test
  LC = CodingAdventures::Lexer::LexerContext
  GL = CodingAdventures::Lexer::GrammarLexer
  TK = CodingAdventures::Lexer::Token

  # push_group records a push action in the group_actions buffer.
  # The action is not applied until the tokenizer processes it.
  def test_push_group_records_action
    grammar = make_group_grammar
    lexer = GL.new("x", grammar)
    ctx = LC.new(lexer, "x", 1)
    ctx.push_group("tag")
    assert_equal [[:push, "tag"]], ctx.group_actions
  end

  # push_group with an unknown group name raises ArgumentError.
  # This catches typos in callback code at runtime.
  def test_push_unknown_group_raises
    grammar = make_group_grammar
    lexer = GL.new("x", grammar)
    ctx = LC.new(lexer, "x", 1)
    assert_raises(ArgumentError) do
      ctx.push_group("nonexistent")
    end
  end

  # pop_group records a pop action. The actual stack pop happens
  # after the callback returns.
  def test_pop_group_records_action
    grammar = make_group_grammar
    lexer = GL.new("x", grammar)
    ctx = LC.new(lexer, "x", 1)
    ctx.pop_group
    assert_equal [[:pop, ""]], ctx.group_actions
  end

  # active_group reads the top of the lexer's group stack.
  # At initialization, this is always "default".
  def test_active_group_reads_stack
    grammar = make_group_grammar
    lexer = GL.new("x", grammar)
    ctx = LC.new(lexer, "x", 1)
    assert_equal "default", ctx.active_group
  end

  # group_stack_depth returns the number of entries in the stack.
  # At initialization, this is 1 (the default group).
  def test_group_stack_depth
    grammar = make_group_grammar
    lexer = GL.new("x", grammar)
    ctx = LC.new(lexer, "x", 1)
    assert_equal 1, ctx.group_stack_depth
  end

  # emit appends a synthetic token to the emitted buffer.
  # These tokens are injected after the current token in the output.
  def test_emit_appends_token
    grammar = make_group_grammar
    lexer = GL.new("x", grammar)
    ctx = LC.new(lexer, "x", 1)
    synthetic = TK.new(type: "SYNTHETIC", value: "!", line: 1, column: 1)
    ctx.emit(synthetic)
    assert_equal [synthetic], ctx.emitted
  end

  # suppress sets the suppressed flag, which tells the tokenizer
  # to exclude the current token from the output.
  def test_suppress_sets_flag
    grammar = make_group_grammar
    lexer = GL.new("x", grammar)
    ctx = LC.new(lexer, "x", 1)
    assert_equal false, ctx.suppressed
    ctx.suppress
    assert_equal true, ctx.suppressed
  end

  # peek reads characters from the source after the current token.
  # offset=1 is the first character after the token.
  def test_peek_reads_source
    grammar = make_group_grammar
    lexer = GL.new("hello", grammar)
    # Suppose token ended at position 3 (consumed "hel")
    ctx = LC.new(lexer, "hello", 3)
    assert_equal "l", ctx.peek(1)
    assert_equal "o", ctx.peek(2)
    assert_equal "", ctx.peek(3) # past EOF
  end

  # peek_str reads a substring from the source after the current token.
  def test_peek_str_reads_source
    grammar = make_group_grammar
    lexer = GL.new("hello world", grammar)
    ctx = LC.new(lexer, "hello world", 5)
    assert_equal " world", ctx.peek_str(6)
  end

  # set_skip_enabled records the new skip state. nil means no change
  # was requested (the default).
  def test_set_skip_enabled
    grammar = make_group_grammar
    lexer = GL.new("x", grammar)
    ctx = LC.new(lexer, "x", 1)
    assert_nil ctx.skip_enabled # no change by default
    ctx.set_skip_enabled(false)
    assert_equal false, ctx.skip_enabled
  end

  # Multiple push_group calls are recorded in order. The tokenizer
  # applies them sequentially after the callback returns.
  def test_multiple_pushes
    grammar = make_group_grammar
    lexer = GL.new("x", grammar)
    ctx = LC.new(lexer, "x", 1)
    ctx.push_group("tag")
    ctx.push_group("tag")
    assert_equal [[:push, "tag"], [:push, "tag"]], ctx.group_actions
  end
end

# =========================================================================
# TestPatternGroupTokenization -- Group Switching During Tokenization
# =========================================================================
#
# These tests verify that the lexer correctly switches between pattern
# groups based on callback actions, producing the right tokens in the
# right order. This is the integration-level test for the callback +
# group mechanism.
# =========================================================================

class TestPatternGroupTokenization < Minitest::Test
  GL = CodingAdventures::Lexer::GrammarLexer
  GT = CodingAdventures::GrammarTools
  TK = CodingAdventures::Lexer::Token
  TT = CodingAdventures::Lexer::TokenType

  # Without a callback, only default group patterns are used.
  # This verifies backward compatibility -- no groups + no callback
  # behaves identically to the original GrammarLexer.
  def test_no_callback_uses_default_group
    grammar = make_group_grammar
    tokens = GL.new("hello", grammar).tokenize
    assert_equal "TEXT", tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  # Callback can push/pop groups to switch pattern sets.
  #
  # Simulates: <div> where < triggers push("tag"), > triggers pop.
  # Inside the tag group, TAG_NAME and TAG_CLOSE patterns are active.
  # After popping back to default, TEXT patterns are active again.
  def test_callback_push_pop_group
    grammar = make_group_grammar

    on_token = proc { |token, ctx|
      if token.type == "OPEN_TAG"
        ctx.push_group("tag")
      elsif token.type == "TAG_CLOSE"
        ctx.pop_group
      end
    }

    lexer = GL.new("<div>hello", grammar)
    lexer.set_on_token(on_token)
    tokens = lexer.tokenize

    types = tokens
      .reject { |t| token_type_name(t) == "EOF" }
      .map { |t| [token_type_name(t), t.value] }
    assert_equal [
      ["OPEN_TAG", "<"],
      ["TAG_NAME", "div"],
      ["TAG_CLOSE", ">"],
      ["TEXT", "hello"]
    ], types
  end

  # Callback handles tag with attributes.
  #
  # Simulates: <div class="main"> where the tag group lexes
  # TAG_NAME, EQUALS, and VALUE tokens.
  def test_callback_with_attributes
    grammar = make_group_grammar

    on_token = proc { |token, ctx|
      if token.type == "OPEN_TAG"
        ctx.push_group("tag")
      elsif token.type == "TAG_CLOSE"
        ctx.pop_group
      end
    }

    lexer = GL.new('<div class="main">', grammar)
    lexer.set_on_token(on_token)
    tokens = lexer.tokenize

    types = tokens
      .reject { |t| token_type_name(t) == "EOF" }
      .map { |t| [token_type_name(t), t.value] }
    assert_equal [
      ["OPEN_TAG", "<"],
      ["TAG_NAME", "div"],
      ["TAG_NAME", "class"],
      ["EQUALS", "="],
      ["VALUE", '"main"'],
      ["TAG_CLOSE", ">"]
    ], types
  end

  # Group stack handles nested structures.
  #
  # Simulates: <a>text<b>inner</b></a> with push/pop on < and >.
  # The group stack grows to depth 2 for the inner tag and correctly
  # pops back to default for the text between tags.
  def test_nested_tags
    grammar = GT.parse_token_grammar(<<~TOKENS)
      escapes: none

      skip:
        WS = /[ \\t\\r\\n]+/

      TEXT             = /[^<]+/
      CLOSE_TAG_START  = "</"
      OPEN_TAG         = "<"

      group tag:
        TAG_NAME  = /[a-zA-Z_][a-zA-Z0-9_]*/
        TAG_CLOSE = ">"
        SLASH     = "/"
    TOKENS

    on_token = proc { |token, ctx|
      if token.type == "OPEN_TAG" || token.type == "CLOSE_TAG_START"
        ctx.push_group("tag")
      elsif token.type == "TAG_CLOSE"
        ctx.pop_group
      end
    }

    lexer = GL.new("<a>text<b>inner</b></a>", grammar)
    lexer.set_on_token(on_token)
    tokens = lexer.tokenize

    types = tokens
      .reject { |t| token_type_name(t) == "EOF" }
      .map { |t| [token_type_name(t), t.value] }
    assert_equal [
      ["OPEN_TAG", "<"],
      ["TAG_NAME", "a"],
      ["TAG_CLOSE", ">"],
      ["TEXT", "text"],
      ["OPEN_TAG", "<"],
      ["TAG_NAME", "b"],
      ["TAG_CLOSE", ">"],
      ["TEXT", "inner"],
      ["CLOSE_TAG_START", "</"],
      ["TAG_NAME", "b"],
      ["TAG_CLOSE", ">"],
      ["CLOSE_TAG_START", "</"],
      ["TAG_NAME", "a"],
      ["TAG_CLOSE", ">"]
    ], types
  end

  # Callback can suppress tokens (remove from output).
  # The OPEN_TAG token is suppressed -- only TEXT appears.
  def test_suppress_token
    grammar = make_group_grammar

    on_token = proc { |token, ctx|
      ctx.suppress if token.type == "OPEN_TAG"
    }

    lexer = GL.new("<hello", grammar)
    lexer.set_on_token(on_token)
    tokens = lexer.tokenize

    types = tokens
      .reject { |t| token_type_name(t) == "EOF" }
      .map { |t| token_type_name(t) }
    assert_equal ["TEXT"], types
  end

  # Callback can emit synthetic tokens after the current one.
  # A MARKER token is injected after OPEN_TAG.
  def test_emit_synthetic_token
    grammar = make_group_grammar

    on_token = proc { |token, ctx|
      if token.type == "OPEN_TAG"
        ctx.emit(TK.new(
          type: "MARKER",
          value: "[start]",
          line: token.line,
          column: token.column
        ))
      end
    }

    lexer = GL.new("<hello", grammar)
    lexer.set_on_token(on_token)
    tokens = lexer.tokenize

    types = tokens
      .reject { |t| token_type_name(t) == "EOF" }
      .map { |t| [token_type_name(t), t.value] }
    assert_equal [
      ["OPEN_TAG", "<"],
      ["MARKER", "[start]"],
      ["TEXT", "hello"]
    ], types
  end

  # Suppress + emit = token replacement.
  #
  # The current token is swallowed, but emitted tokens still output.
  # This enables token rewriting (e.g., replacing OPEN_TAG with a
  # different token type).
  def test_suppress_and_emit
    grammar = make_group_grammar

    on_token = proc { |token, ctx|
      if token.type == "OPEN_TAG"
        ctx.suppress
        ctx.emit(TK.new(
          type: "REPLACED",
          value: "<",
          line: token.line,
          column: token.column
        ))
      end
    }

    lexer = GL.new("<hello", grammar)
    lexer.set_on_token(on_token)
    tokens = lexer.tokenize

    types = tokens
      .reject { |t| token_type_name(t) == "EOF" }
      .map { |t| [token_type_name(t), t.value] }
    assert_equal [
      ["REPLACED", "<"],
      ["TEXT", "hello"]
    ], types
  end

  # Popping when only default remains is a no-op (no crash).
  # The default group is the floor and cannot be popped.
  def test_pop_at_bottom_is_noop
    grammar = make_group_grammar

    on_token = proc { |_token, ctx|
      ctx.pop_group # Should be safe even at the bottom
    }

    lexer = GL.new("hello", grammar)
    lexer.set_on_token(on_token)
    tokens = lexer.tokenize

    # Should still produce TEXT token without crashing.
    assert_equal "TEXT", tokens[0].type
  end

  # Callback can disable skip patterns for significant whitespace.
  #
  # When skip is disabled, whitespace that would normally be consumed
  # silently is instead captured by the active group's patterns. Here,
  # RAW_TEXT captures " hello world " including the spaces.
  def test_set_skip_enabled_false
    grammar = GT.parse_token_grammar(<<~TOKENS)
      escapes: none

      skip:
        WS = /[ \\t]+/

      TEXT      = /[^<]+/
      START     = "<!"

      group raw:
        RAW_TEXT = /[^>]+/
        END      = ">"
    TOKENS

    on_token = proc { |token, ctx|
      if token.type == "START"
        ctx.push_group("raw")
        ctx.set_skip_enabled(false)
      elsif token.type == "END"
        ctx.pop_group
        ctx.set_skip_enabled(true)
      end
    }

    # The space in "hello world" should be preserved (not skipped)
    # because skip is disabled while in the raw group.
    lexer = GL.new("<! hello world >after", grammar)
    lexer.set_on_token(on_token)
    tokens = lexer.tokenize

    types = tokens
      .reject { |t| token_type_name(t) == "EOF" }
      .map { |t| [token_type_name(t), t.value] }
    assert_equal [
      ["START", "<!"],
      ["RAW_TEXT", " hello world "],
      ["END", ">"],
      ["TEXT", "after"]
    ], types
  end

  # A grammar with no groups behaves identically to before.
  # This verifies backward compatibility: no groups + no callback
  # = same behavior as the original GrammarLexer.
  def test_no_groups_backward_compat
    grammar = GT.parse_token_grammar(<<~TOKENS)
      NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
      NUMBER = /[0-9]+/
      PLUS   = "+"
    TOKENS
    tokens = GL.new("x + 1", grammar).tokenize

    types = tokens
      .reject { |t| [TT::NEWLINE, TT::EOF].include?(t.type) }
      .map { |t| [token_type_name(t), t.value] }
    assert_equal [
      ["NAME", "x"],
      ["PLUS", "+"],
      ["NUMBER", "1"]
    ], types
  end

  # Passing nil to set_on_token clears the callback.
  # After clearing, no callback fires during tokenization.
  def test_clear_callback
    grammar = make_group_grammar
    called = []

    on_token = proc { |token, _ctx|
      called << token.type
    }

    lexer = GL.new("hello", grammar)
    lexer.set_on_token(on_token)
    lexer.set_on_token(nil)
    lexer.tokenize

    assert_equal [], called
  end

  # The group stack resets when tokenize is called again.
  #
  # This ensures the lexer can be reused for multiple tokenize
  # calls without group state leaking between them.
  def test_group_stack_resets_between_calls
    grammar = make_group_grammar

    on_token = proc { |token, ctx|
      ctx.push_group("tag") if token.type == "OPEN_TAG"
    }

    lexer = GL.new("<div", grammar)
    lexer.set_on_token(on_token)

    # First call: pushes "tag" group.
    tokens1 = lexer.tokenize
    assert tokens1.any? { |t| t.type == "TAG_NAME" }

    # Second call: should start fresh from "default".
    lexer.source = "<div"
    lexer.pos = 0
    lexer.line = 1
    lexer.column = 1
    tokens2 = lexer.tokenize
    assert tokens2.any? { |t| t.type == "TAG_NAME" }
  end

  # Multiple push/pop in one callback are applied in order.
  # Double-push results in stack ["default", "tag", "tag"].
  def test_multiple_push_pop_sequence
    grammar = make_group_grammar

    on_token = proc { |token, ctx|
      if token.type == "OPEN_TAG"
        ctx.push_group("tag")
        ctx.push_group("tag")
      end
    }

    lexer = GL.new("<div", grammar)
    lexer.set_on_token(on_token)
    lexer.tokenize

    # Re-tokenize to confirm the stack handles double-push.
    lexer.source = "<div"
    lexer.pos = 0
    lexer.line = 1
    lexer.column = 1
    tokens = lexer.tokenize
    assert tokens.any? { |t| token_type_name(t) == "TAG_NAME" }
  end
end
