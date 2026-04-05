defmodule CodingAdventures.EcmascriptEs5LexerTest do
  use ExUnit.Case

  alias CodingAdventures.EcmascriptEs5Lexer

  # ===========================================================================
  # Module loading
  # ===========================================================================

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.EcmascriptEs5Lexer)
  end

  # ===========================================================================
  # Grammar loading
  # ===========================================================================

  test "grammar_path returns a path ending in es5.tokens" do
    path = EcmascriptEs5Lexer.grammar_path()
    assert String.ends_with?(path, "es5.tokens")
  end

  test "load_grammar succeeds" do
    assert {:ok, grammar} = EcmascriptEs5Lexer.load_grammar()
    assert length(grammar.definitions) > 0
    assert length(grammar.keywords) > 0
  end

  # ===========================================================================
  # Basic tokenization
  # ===========================================================================

  test "tokenize empty string produces EOF" do
    assert {:ok, tokens} = EcmascriptEs5Lexer.tokenize("")
    assert List.last(tokens).type == "EOF"
  end

  test "tokenize var declaration" do
    assert {:ok, tokens} = EcmascriptEs5Lexer.tokenize("var x = 42;")
    types = Enum.map(tokens, & &1.type)
    assert "KEYWORD" in types
    assert "NUMBER" in types
    assert "SEMICOLON" in types
  end

  # ===========================================================================
  # ES5-specific: debugger keyword
  # ===========================================================================

  test "tokenize debugger keyword" do
    assert {:ok, tokens} = EcmascriptEs5Lexer.tokenize("debugger;")
    keyword_tokens = Enum.filter(tokens, &(&1.type == "KEYWORD"))
    keyword_values = Enum.map(keyword_tokens, & &1.value)
    assert "debugger" in keyword_values
  end

  # ===========================================================================
  # ES3 features still work in ES5
  # ===========================================================================

  test "tokenize strict equality ===" do
    assert {:ok, tokens} = EcmascriptEs5Lexer.tokenize("a === b")
    types = Enum.map(tokens, & &1.type)
    assert "STRICT_EQUALS" in types
  end

  test "tokenize strict not-equals !==" do
    assert {:ok, tokens} = EcmascriptEs5Lexer.tokenize("a !== b")
    types = Enum.map(tokens, & &1.type)
    assert "STRICT_NOT_EQUALS" in types
  end

  test "tokenize try/catch/finally keywords" do
    source = "try { } catch (e) { } finally { }"
    assert {:ok, tokens} = EcmascriptEs5Lexer.tokenize(source)
    keyword_tokens = Enum.filter(tokens, &(&1.type == "KEYWORD"))
    keyword_values = Enum.map(keyword_tokens, & &1.value)
    assert "try" in keyword_values
    assert "catch" in keyword_values
    assert "finally" in keyword_values
  end

  test "tokenize instanceof keyword" do
    assert {:ok, tokens} = EcmascriptEs5Lexer.tokenize("x instanceof Array")
    keyword_tokens = Enum.filter(tokens, &(&1.type == "KEYWORD"))
    keyword_values = Enum.map(keyword_tokens, & &1.value)
    assert "instanceof" in keyword_values
  end

  test "tokenize regex literal" do
    assert {:ok, tokens} = EcmascriptEs5Lexer.tokenize("/pattern/gim")
    regex_tokens = Enum.filter(tokens, &(&1.type == "REGEX"))
    assert length(regex_tokens) == 1
  end

  # ===========================================================================
  # Operators and delimiters
  # ===========================================================================

  test "tokenize all operator types" do
    source = "a + b - c * d / e % f"
    assert {:ok, tokens} = EcmascriptEs5Lexer.tokenize(source)
    types = Enum.map(tokens, & &1.type)
    assert "PLUS" in types
    assert "MINUS" in types
    assert "STAR" in types
    assert "SLASH" in types
    assert "PERCENT" in types
  end

  test "tokenize bitwise operators" do
    assert {:ok, tokens} = EcmascriptEs5Lexer.tokenize("a & b | c ^ d ~ e")
    types = Enum.map(tokens, & &1.type)
    assert "AMPERSAND" in types
    assert "PIPE" in types
    assert "CARET" in types
    assert "TILDE" in types
  end

  test "tokenize assignment operators" do
    source = "x += 1; y -= 2; z <<= 3; w >>= 4; v >>>= 5;"
    assert {:ok, tokens} = EcmascriptEs5Lexer.tokenize(source)
    types = Enum.map(tokens, & &1.type)
    assert "PLUS_EQUALS" in types
    assert "MINUS_EQUALS" in types
    assert "LEFT_SHIFT_EQUALS" in types
    assert "RIGHT_SHIFT_EQUALS" in types
    assert "UNSIGNED_RIGHT_SHIFT_EQUALS" in types
  end

  test "tokenize delimiters" do
    assert {:ok, tokens} = EcmascriptEs5Lexer.tokenize("( ) { } [ ] ; , : .")
    types = Enum.map(tokens, & &1.type)
    assert "LPAREN" in types
    assert "RPAREN" in types
    assert "LBRACE" in types
    assert "RBRACE" in types
    assert "LBRACKET" in types
    assert "RBRACKET" in types
    assert "SEMICOLON" in types
    assert "COMMA" in types
    assert "COLON" in types
    assert "DOT" in types
  end

  # ===========================================================================
  # Whitespace and comments
  # ===========================================================================

  test "skips whitespace and comments" do
    source = "x /* block */ + // line\ny"
    assert {:ok, tokens} = EcmascriptEs5Lexer.tokenize(source)
    name_tokens = Enum.filter(tokens, &(&1.type == "NAME"))
    assert length(name_tokens) == 2
  end

  # ===========================================================================
  # Complex expression
  # ===========================================================================

  test "tokenize getter/setter object literal" do
    source = """
    var obj = {
      get name() { return this._name; },
      set name(value) { this._name = value; }
    };
    """
    assert {:ok, tokens} = EcmascriptEs5Lexer.tokenize(source)
    types = Enum.map(tokens, & &1.type)
    assert "KEYWORD" in types
    assert "NAME" in types
    assert "LBRACE" in types
    assert "RBRACE" in types
  end
end
