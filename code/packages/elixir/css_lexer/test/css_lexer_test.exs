defmodule CodingAdventures.CssLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CssLexer

  defp token_types(source) do
    {:ok, tokens} = CssLexer.tokenize(source)
    Enum.map(tokens, & &1.type)
  end

  defp token_values(source) do
    {:ok, tokens} = CssLexer.tokenize(source)
    Enum.map(tokens, & &1.value)
  end

  describe "create_lexer/0" do
    test "returns the parsed css token grammar" do
      grammar = CssLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)

      assert "DIMENSION" in names
      assert "PERCENTAGE" in names
      assert "URL_TOKEN" in names
      assert "FUNCTION" in names
      assert "CUSTOM_PROPERTY" in names
      assert grammar.escape_mode == "none"
      assert Enum.any?(grammar.skip_definitions, &(&1.name == "COMMENT"))
    end
  end

  describe "tokenize/1" do
    test "tokenizes a simple rule" do
      assert token_types("h1 { color: red; }") == [
               "IDENT",
               "LBRACE",
               "IDENT",
               "COLON",
               "IDENT",
               "SEMICOLON",
               "RBRACE",
               "EOF"
             ]
    end

    test "prefers compound numeric tokens over NUMBER" do
      assert token_values("10px 50% 42") == ["10px", "50%", "42", ""]
      assert token_types("10px 50% 42") == ["DIMENSION", "PERCENTAGE", "NUMBER", "EOF"]
    end

    test "tokenizes function-like css constructs" do
      assert token_values("rgb(") == ["rgb(", ""]
      assert token_types("url(image.png)") == ["URL_TOKEN", "EOF"]
    end

    test "tokenizes selectors and Ruby-style operators is not confused by CSS operators" do
      assert token_values("#header @media a~=b") == ["#header", "@media", "a", "~=", "b", ""]
      assert token_types("#header @media a~=b") == ["HASH", "AT_KEYWORD", "IDENT", "TILDE_EQUALS", "IDENT", "EOF"]
    end

    test "tokenizes custom properties and unicode ranges" do
      assert token_values("--main-color U+0025-00FF") == ["--main-color", "U+0025-00FF", ""]
      assert token_types("--main-color U+0025-00FF") == ["CUSTOM_PROPERTY", "UNICODE_RANGE", "EOF"]
    end

    test "tracks line and column positions" do
      {:ok, tokens} = CssLexer.tokenize("h1 { color: red; }")
      [ident, lbrace | _] = tokens

      assert {ident.line, ident.column} == {1, 1}
      assert {lbrace.line, lbrace.column} == {1, 4}
    end

    test "returns an error on unsupported characters" do
      assert {:error, message} = CssLexer.tokenize("`")
      assert message =~ "Unexpected character"
    end
  end
end
