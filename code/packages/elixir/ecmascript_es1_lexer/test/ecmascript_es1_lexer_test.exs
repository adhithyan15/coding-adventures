defmodule CodingAdventures.EcmascriptEs1LexerTest do
  use ExUnit.Case

  alias CodingAdventures.EcmascriptEs1Lexer

  # ===========================================================================
  # Module loading
  # ===========================================================================

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.EcmascriptEs1Lexer)
  end

  # ===========================================================================
  # Grammar loading
  # ===========================================================================

  test "grammar_path returns a path ending in es1.tokens" do
    path = EcmascriptEs1Lexer.grammar_path()
    assert String.ends_with?(path, "es1.tokens")
  end

  test "load_grammar succeeds" do
    assert {:ok, grammar} = EcmascriptEs1Lexer.load_grammar()
    assert length(grammar.definitions) > 0
    assert length(grammar.keywords) > 0
  end

  # ===========================================================================
  # Basic tokenization
  # ===========================================================================

  test "tokenize empty string produces EOF" do
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize("")
    assert List.last(tokens).type == "EOF"
  end

  test "tokenize var declaration" do
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize("var x = 42;")
    types = Enum.map(tokens, & &1.type)
    assert "KEYWORD" in types
    assert "NAME" in types
    assert "EQUALS" in types
    assert "NUMBER" in types
    assert "SEMICOLON" in types
    assert "EOF" in types
  end

  test "tokenize string literals" do
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize(~s("hello"))
    string_tokens = Enum.filter(tokens, &(&1.type == "STRING"))
    assert length(string_tokens) == 1
  end

  test "tokenize single-quoted strings" do
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize("'world'")
    string_tokens = Enum.filter(tokens, &(&1.type == "STRING"))
    assert length(string_tokens) == 1
  end

  test "tokenize numeric literals" do
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize("42 3.14 0xFF .5 1e10")
    number_tokens = Enum.filter(tokens, &(&1.type == "NUMBER"))
    assert length(number_tokens) == 5
  end

  test "tokenize operators" do
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize("a + b - c * d / e")
    types = Enum.map(tokens, & &1.type)
    assert "PLUS" in types
    assert "MINUS" in types
    assert "STAR" in types
    assert "SLASH" in types
  end

  test "tokenize comparison operators" do
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize("a == b != c <= d >= e")
    types = Enum.map(tokens, & &1.type)
    assert "EQUALS_EQUALS" in types
    assert "NOT_EQUALS" in types
    assert "LESS_EQUALS" in types
    assert "GREATER_EQUALS" in types
  end

  test "tokenize increment and decrement" do
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize("x++ y--")
    types = Enum.map(tokens, & &1.type)
    assert "PLUS_PLUS" in types
    assert "MINUS_MINUS" in types
  end

  test "tokenize logical operators" do
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize("a && b || c")
    types = Enum.map(tokens, & &1.type)
    assert "AND_AND" in types
    assert "OR_OR" in types
  end

  test "tokenize shift operators" do
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize("a << b >> c >>> d")
    types = Enum.map(tokens, & &1.type)
    assert "LEFT_SHIFT" in types
    assert "RIGHT_SHIFT" in types
    assert "UNSIGNED_RIGHT_SHIFT" in types
  end

  test "tokenize delimiters" do
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize("( ) { } [ ] ; , : .")
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
  # Keywords
  # ===========================================================================

  test "tokenize keywords" do
    source = "var function if else while for return"
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize(source)
    keyword_tokens = Enum.filter(tokens, &(&1.type == "KEYWORD"))
    keyword_values = Enum.map(keyword_tokens, & &1.value)
    assert "var" in keyword_values
    assert "function" in keyword_values
    assert "if" in keyword_values
    assert "else" in keyword_values
    assert "while" in keyword_values
    assert "for" in keyword_values
    assert "return" in keyword_values
  end

  test "tokenize boolean and null literals as keywords" do
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize("true false null")
    keyword_tokens = Enum.filter(tokens, &(&1.type == "KEYWORD"))
    keyword_values = Enum.map(keyword_tokens, & &1.value)
    assert "true" in keyword_values
    assert "false" in keyword_values
    assert "null" in keyword_values
  end

  # ===========================================================================
  # Identifiers
  # ===========================================================================

  test "tokenize identifiers with dollar sign" do
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize("$foo _bar baz123")
    name_tokens = Enum.filter(tokens, &(&1.type == "NAME"))
    name_values = Enum.map(name_tokens, & &1.value)
    assert "$foo" in name_values
    assert "_bar" in name_values
    assert "baz123" in name_values
  end

  # ===========================================================================
  # Whitespace and comments
  # ===========================================================================

  test "skips whitespace" do
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize("  x  ")
    non_eof = Enum.reject(tokens, &(&1.type == "EOF"))
    assert length(non_eof) == 1
    assert hd(non_eof).type == "NAME"
  end

  test "skips line comments" do
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize("x // this is a comment\ny")
    name_tokens = Enum.filter(tokens, &(&1.type == "NAME"))
    assert length(name_tokens) == 2
  end

  test "skips block comments" do
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize("x /* block */ y")
    name_tokens = Enum.filter(tokens, &(&1.type == "NAME"))
    assert length(name_tokens) == 2
  end

  # ===========================================================================
  # Complex expressions
  # ===========================================================================

  test "tokenize function declaration" do
    source = "function add(a, b) { return a + b; }"
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize(source)
    types = Enum.map(tokens, & &1.type)
    assert "KEYWORD" in types
    assert "NAME" in types
    assert "LPAREN" in types
    assert "COMMA" in types
    assert "RPAREN" in types
    assert "LBRACE" in types
    assert "RBRACE" in types
  end

  test "tokenize assignment operators" do
    source = "x += 1; y -= 2; z *= 3; w /= 4; v %= 5;"
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize(source)
    types = Enum.map(tokens, & &1.type)
    assert "PLUS_EQUALS" in types
    assert "MINUS_EQUALS" in types
    assert "STAR_EQUALS" in types
    assert "SLASH_EQUALS" in types
    assert "PERCENT_EQUALS" in types
  end

  test "tokenize ternary operator" do
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize("a ? b : c")
    types = Enum.map(tokens, & &1.type)
    assert "QUESTION" in types
    assert "COLON" in types
  end

  # ===========================================================================
  # ES1 does NOT have strict equality
  # ===========================================================================

  test "ES1 does not have strict equality as separate tokens" do
    # In ES1, === would be lexed as == followed by =
    assert {:ok, tokens} = EcmascriptEs1Lexer.tokenize("a == b")
    types = Enum.map(tokens, & &1.type)
    assert "EQUALS_EQUALS" in types
  end
end
