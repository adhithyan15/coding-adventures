defmodule CodingAdventures.PythonLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.PythonLexer

  defp token_types(source) do
    {:ok, tokens} = PythonLexer.tokenize(source)
    Enum.map(tokens, & &1.type)
  end

  defp token_values(source) do
    {:ok, tokens} = PythonLexer.tokenize(source)
    Enum.map(tokens, & &1.value)
  end

  describe "create_lexer/0" do
    test "returns the parsed python token grammar" do
      grammar = PythonLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)

      assert "NAME" in names
      assert "NUMBER" in names
      assert "STRING" in names
      assert "EQUALS_EQUALS" in names
      assert "COLON" in names
      assert "if" in grammar.keywords
      assert "def" in grammar.keywords
      assert "True" in grammar.keywords
    end
  end

  describe "tokenize/1" do
    test "tokenizes a basic assignment expression" do
      assert token_types("x = 1 + 2") == ["NAME", "EQUALS", "NUMBER", "PLUS", "NUMBER", "EOF"]
      assert token_values("x = 1 + 2") == ["x", "=", "1", "+", "2", ""]
    end

    test "maps Python reserved words to KEYWORD tokens" do
      {:ok, tokens} = PythonLexer.tokenize("if def True False None")

      assert Enum.map(tokens, & &1.type) == ["KEYWORD", "KEYWORD", "KEYWORD", "KEYWORD", "KEYWORD", "EOF"]
      assert Enum.map(tokens, & &1.value) == ["if", "def", "True", "False", "None", ""]
    end

    test "does not classify regular identifiers as keywords" do
      {:ok, [token, eof]} = PythonLexer.tokenize("foobar")

      assert token.type == "NAME"
      assert token.value == "foobar"
      assert eof.type == "EOF"
    end

    test "prefers equals-equals over equals" do
      assert token_types("x = y == z") == ["NAME", "EQUALS", "NAME", "EQUALS_EQUALS", "NAME", "EOF"]
    end

    test "tokenizes string literals and delimiters" do
      {:ok, tokens} = PythonLexer.tokenize("def foo(x): \"hello\"")

      assert Enum.map(tokens, & &1.type) == ["KEYWORD", "NAME", "LPAREN", "NAME", "RPAREN", "COLON", "STRING", "EOF"]
      assert Enum.at(tokens, 6).value == "hello"
    end

    test "tracks line and column positions" do
      {:ok, tokens} = PythonLexer.tokenize("x = 1")
      [name, equals, number, eof] = tokens

      assert {name.line, name.column} == {1, 1}
      assert {equals.line, equals.column} == {1, 3}
      assert {number.line, number.column} == {1, 5}
      assert {eof.line, eof.column} == {1, 6}
    end

    test "returns an error on unexpected characters" do
      assert {:error, message} = PythonLexer.tokenize("@")
      assert message =~ "Unexpected character"
    end
  end
end
