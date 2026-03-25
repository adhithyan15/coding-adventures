defmodule CodingAdventures.RubyLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.RubyLexer

  defp token_types(source) do
    {:ok, tokens} = RubyLexer.tokenize(source)
    Enum.map(tokens, & &1.type)
  end

  defp token_values(source) do
    {:ok, tokens} = RubyLexer.tokenize(source)
    Enum.map(tokens, & &1.value)
  end

  describe "create_lexer/0" do
    test "returns the parsed ruby token grammar" do
      grammar = RubyLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)

      assert "NAME" in names
      assert "STRING" in names
      assert "DOT_DOT" in names
      assert "HASH_ROCKET" in names
      assert "NOT_EQUALS" in names
      assert "def" in grammar.keywords
      assert "end" in grammar.keywords
      assert "puts" in grammar.keywords
    end
  end

  describe "tokenize/1" do
    test "tokenizes a basic assignment expression" do
      assert token_types("x = 1 + 2") == ["NAME", "EQUALS", "NUMBER", "PLUS", "NUMBER", "EOF"]
      assert token_values("x = 1 + 2") == ["x", "=", "1", "+", "2", ""]
    end

    test "maps Ruby keywords to KEYWORD tokens" do
      {:ok, tokens} = RubyLexer.tokenize("def end puts true false nil")

      assert Enum.map(tokens, & &1.type) == ["KEYWORD", "KEYWORD", "KEYWORD", "KEYWORD", "KEYWORD", "KEYWORD", "EOF"]
      assert Enum.map(tokens, & &1.value) == ["def", "end", "puts", "true", "false", "nil", ""]
    end

    test "tokenizes Ruby-specific operators" do
      assert token_values("1..10") == ["1", "..", "10", ""]
      assert token_values("key => value") == ["key", "=>", "value", ""]
      assert token_values("x != 1") == ["x", "!=", "1", ""]
      assert token_values("a <= b >= c") == ["a", "<=", "b", ">=", "c", ""]
    end

    test "tokenizes string literals and delimiters" do
      {:ok, tokens} = RubyLexer.tokenize("def greet(name): \"hello\"")

      assert Enum.map(tokens, & &1.type) == ["KEYWORD", "NAME", "LPAREN", "NAME", "RPAREN", "COLON", "STRING", "EOF"]
      assert Enum.at(tokens, 6).value == "hello"
    end

    test "tracks line and column positions" do
      {:ok, tokens} = RubyLexer.tokenize("x = 1")
      [name, equals, number, eof] = tokens

      assert {name.line, name.column} == {1, 1}
      assert {equals.line, equals.column} == {1, 3}
      assert {number.line, number.column} == {1, 5}
      assert {eof.line, eof.column} == {1, 6}
    end

    test "returns an error on unexpected characters" do
      assert {:error, message} = RubyLexer.tokenize("@")
      assert message =~ "Unexpected character"
    end
  end
end
