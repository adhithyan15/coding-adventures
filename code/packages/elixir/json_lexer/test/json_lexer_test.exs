defmodule CodingAdventures.JsonLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.JsonLexer

  describe "create_lexer/0" do
    test "returns a TokenGrammar" do
      grammar = JsonLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)
      assert "STRING" in names
      assert "NUMBER" in names
      assert "TRUE" in names
      assert "FALSE" in names
      assert "NULL" in names
      assert "LBRACE" in names
      assert "RBRACE" in names
      assert "COLON" in names
      assert "COMMA" in names
    end
  end

  describe "tokenize/1 — primitives" do
    test "tokenizes number" do
      {:ok, tokens} = JsonLexer.tokenize("42")
      [num, eof] = tokens
      assert num.type == "NUMBER"
      assert num.value == "42"
      assert eof.type == "EOF"
    end

    test "tokenizes negative number" do
      {:ok, tokens} = JsonLexer.tokenize("-3.14")
      [num, _eof] = tokens
      assert num.type == "NUMBER"
      assert num.value == "-3.14"
    end

    test "tokenizes exponent number" do
      {:ok, tokens} = JsonLexer.tokenize("1.5e10")
      [num, _eof] = tokens
      assert num.value == "1.5e10"
    end

    test "tokenizes string" do
      {:ok, tokens} = JsonLexer.tokenize(~s("hello"))
      [str, _eof] = tokens
      assert str.type == "STRING"
      assert str.value == "hello"
    end

    test "tokenizes string with escapes" do
      # The JSON lexer uses `escapes: none` — escape sequences are passed
      # through raw and decoded by the parser, not the lexer. So \n in the
      # source remains as the two-character sequence \n in the token value.
      {:ok, tokens} = JsonLexer.tokenize(~S("hello\nworld"))
      [str, _eof] = tokens
      assert str.value == "hello\\nworld"
    end

    test "tokenizes true" do
      {:ok, tokens} = JsonLexer.tokenize("true")
      [tok, _eof] = tokens
      assert tok.type == "TRUE"
      assert tok.value == "true"
    end

    test "tokenizes false" do
      {:ok, tokens} = JsonLexer.tokenize("false")
      [tok, _eof] = tokens
      assert tok.type == "FALSE"
    end

    test "tokenizes null" do
      {:ok, tokens} = JsonLexer.tokenize("null")
      [tok, _eof] = tokens
      assert tok.type == "NULL"
    end
  end

  describe "tokenize/1 — structural tokens" do
    test "tokenizes all delimiters" do
      {:ok, tokens} = JsonLexer.tokenize("{[]:,}")
      types = Enum.map(tokens, & &1.type)
      assert types == ["LBRACE", "LBRACKET", "RBRACKET", "COLON", "COMMA", "RBRACE", "EOF"]
    end
  end

  describe "tokenize/1 — compound structures" do
    test "tokenizes a simple object" do
      {:ok, tokens} = JsonLexer.tokenize(~s({"key": "value"}))
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["LBRACE", "STRING", "COLON", "STRING", "RBRACE"]
    end

    test "tokenizes a simple array" do
      {:ok, tokens} = JsonLexer.tokenize("[1, 2, 3]")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["LBRACKET", "NUMBER", "COMMA", "NUMBER", "COMMA", "NUMBER", "RBRACKET"]
    end

    test "tokenizes nested structures" do
      source = ~s({"users": [{"name": "Alice"}]})
      {:ok, tokens} = JsonLexer.tokenize(source)
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))

      assert types == [
               "LBRACE", "STRING", "COLON",
               "LBRACKET",
               "LBRACE", "STRING", "COLON", "STRING", "RBRACE",
               "RBRACKET",
               "RBRACE"
             ]
    end
  end

  describe "tokenize/1 — whitespace handling" do
    test "skips whitespace including newlines" do
      source = """
      {
        "a" : 1
      }
      """

      {:ok, tokens} = JsonLexer.tokenize(source)
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["LBRACE", "STRING", "COLON", "NUMBER", "RBRACE"]
    end
  end

  describe "tokenize/1 — position tracking" do
    test "tracks line and column" do
      {:ok, tokens} = JsonLexer.tokenize(~s({"a": 1}))
      [lbrace | _] = tokens
      assert lbrace.line == 1
      assert lbrace.column == 1
    end
  end

  describe "tokenize/1 — error cases" do
    test "errors on unexpected character" do
      {:error, msg} = JsonLexer.tokenize("@")
      assert msg =~ "Unexpected character"
    end
  end
end
