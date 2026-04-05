defmodule CodingAdventures.EcmascriptEs3LexerTest do
  use ExUnit.Case

  alias CodingAdventures.EcmascriptEs3Lexer

  # ===========================================================================
  # Module loading
  # ===========================================================================

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.EcmascriptEs3Lexer)
  end

  # ===========================================================================
  # Grammar loading
  # ===========================================================================

  test "grammar_path returns a path ending in es3.tokens" do
    path = EcmascriptEs3Lexer.grammar_path()
    assert String.ends_with?(path, "es3.tokens")
  end

  test "load_grammar succeeds" do
    assert {:ok, grammar} = EcmascriptEs3Lexer.load_grammar()
    assert length(grammar.definitions) > 0
    assert length(grammar.keywords) > 0
  end

  # ===========================================================================
  # Basic tokenization
  # ===========================================================================

  test "tokenize empty string produces EOF" do
    assert {:ok, tokens} = EcmascriptEs3Lexer.tokenize("")
    assert List.last(tokens).type == "EOF"
  end

  test "tokenize var declaration" do
    assert {:ok, tokens} = EcmascriptEs3Lexer.tokenize("var x = 42;")
    types = Enum.map(tokens, & &1.type)
    assert "KEYWORD" in types
    assert "NUMBER" in types
    assert "SEMICOLON" in types
  end

  # ===========================================================================
  # ES3-specific: Strict equality
  # ===========================================================================

  test "tokenize strict equality ===" do
    assert {:ok, tokens} = EcmascriptEs3Lexer.tokenize("a === b")
    types = Enum.map(tokens, & &1.type)
    assert "STRICT_EQUALS" in types
  end

  test "tokenize strict not-equals !==" do
    assert {:ok, tokens} = EcmascriptEs3Lexer.tokenize("a !== b")
    types = Enum.map(tokens, & &1.type)
    assert "STRICT_NOT_EQUALS" in types
  end

  test "tokenize abstract equality still works" do
    assert {:ok, tokens} = EcmascriptEs3Lexer.tokenize("a == b != c")
    types = Enum.map(tokens, & &1.type)
    assert "EQUALS_EQUALS" in types
    assert "NOT_EQUALS" in types
  end

  # ===========================================================================
  # ES3-specific: try/catch/finally/throw keywords
  # ===========================================================================

  test "tokenize try/catch keywords" do
    source = "try { x() } catch (e) { }"
    assert {:ok, tokens} = EcmascriptEs3Lexer.tokenize(source)
    keyword_tokens = Enum.filter(tokens, &(&1.type == "KEYWORD"))
    keyword_values = Enum.map(keyword_tokens, & &1.value)
    assert "try" in keyword_values
    assert "catch" in keyword_values
  end

  test "tokenize finally keyword" do
    source = "try { } finally { }"
    assert {:ok, tokens} = EcmascriptEs3Lexer.tokenize(source)
    keyword_tokens = Enum.filter(tokens, &(&1.type == "KEYWORD"))
    keyword_values = Enum.map(keyword_tokens, & &1.value)
    assert "finally" in keyword_values
  end

  test "tokenize throw keyword" do
    source = "throw new Error();"
    assert {:ok, tokens} = EcmascriptEs3Lexer.tokenize(source)
    keyword_tokens = Enum.filter(tokens, &(&1.type == "KEYWORD"))
    keyword_values = Enum.map(keyword_tokens, & &1.value)
    assert "throw" in keyword_values
    assert "new" in keyword_values
  end

  test "tokenize instanceof keyword" do
    source = "x instanceof Array"
    assert {:ok, tokens} = EcmascriptEs3Lexer.tokenize(source)
    keyword_tokens = Enum.filter(tokens, &(&1.type == "KEYWORD"))
    keyword_values = Enum.map(keyword_tokens, & &1.value)
    assert "instanceof" in keyword_values
  end

  # ===========================================================================
  # ES3-specific: Regex literals
  # ===========================================================================

  test "tokenize regex literal" do
    assert {:ok, tokens} = EcmascriptEs3Lexer.tokenize("/pattern/gi")
    regex_tokens = Enum.filter(tokens, &(&1.type == "REGEX"))
    assert length(regex_tokens) == 1
  end

  # ===========================================================================
  # Operators and delimiters (inherited from ES1)
  # ===========================================================================

  test "tokenize all operator types" do
    source = "a + b - c * d / e % f"
    assert {:ok, tokens} = EcmascriptEs3Lexer.tokenize(source)
    types = Enum.map(tokens, & &1.type)
    assert "PLUS" in types
    assert "MINUS" in types
    assert "STAR" in types
    assert "SLASH" in types
    assert "PERCENT" in types
  end

  test "tokenize shift operators" do
    assert {:ok, tokens} = EcmascriptEs3Lexer.tokenize("a << b >> c >>> d")
    types = Enum.map(tokens, & &1.type)
    assert "LEFT_SHIFT" in types
    assert "RIGHT_SHIFT" in types
    assert "UNSIGNED_RIGHT_SHIFT" in types
  end

  test "tokenize logical operators" do
    assert {:ok, tokens} = EcmascriptEs3Lexer.tokenize("a && b || !c")
    types = Enum.map(tokens, & &1.type)
    assert "AND_AND" in types
    assert "OR_OR" in types
    assert "BANG" in types
  end

  test "tokenize string literals" do
    assert {:ok, tokens} = EcmascriptEs3Lexer.tokenize(~s("hello" 'world'))
    string_tokens = Enum.filter(tokens, &(&1.type == "STRING"))
    assert length(string_tokens) == 2
  end

  test "tokenize numeric literals" do
    assert {:ok, tokens} = EcmascriptEs3Lexer.tokenize("42 3.14 0xFF .5 1e10")
    number_tokens = Enum.filter(tokens, &(&1.type == "NUMBER"))
    assert length(number_tokens) == 5
  end

  # ===========================================================================
  # Whitespace and comments
  # ===========================================================================

  test "skips whitespace and comments" do
    source = "x /* comment */ + // line\ny"
    assert {:ok, tokens} = EcmascriptEs3Lexer.tokenize(source)
    name_tokens = Enum.filter(tokens, &(&1.type == "NAME"))
    assert length(name_tokens) == 2
  end

  # ===========================================================================
  # Complex expression
  # ===========================================================================

  test "tokenize function with try/catch" do
    source = """
    function safe(fn) {
      try {
        return fn();
      } catch (err) {
        return null;
      }
    }
    """
    assert {:ok, tokens} = EcmascriptEs3Lexer.tokenize(source)
    keyword_tokens = Enum.filter(tokens, &(&1.type == "KEYWORD"))
    keyword_values = Enum.map(keyword_tokens, & &1.value)
    assert "function" in keyword_values
    assert "try" in keyword_values
    assert "return" in keyword_values
    assert "catch" in keyword_values
    assert "null" in keyword_values
  end
end
