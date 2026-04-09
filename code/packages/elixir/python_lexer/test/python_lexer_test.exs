defmodule CodingAdventures.PythonLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.PythonLexer

  defp token_types(source, version \\ nil) do
    {:ok, tokens} = PythonLexer.tokenize(source, version)
    Enum.map(tokens, & &1.type)
  end

  defp token_values(source, version \\ nil) do
    {:ok, tokens} = PythonLexer.tokenize(source, version)
    Enum.map(tokens, & &1.value)
  end

  describe "version constants" do
    test "default_version is 3.12" do
      assert PythonLexer.default_version() == "3.12"
    end

    test "supported_versions includes expected versions" do
      versions = PythonLexer.supported_versions()
      assert "2.7" in versions
      assert "3.0" in versions
      assert "3.6" in versions
      assert "3.8" in versions
      assert "3.10" in versions
      assert "3.12" in versions
    end
  end

  describe "create_lexer/1" do
    test "returns the parsed python token grammar for default version" do
      grammar = PythonLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)

      assert "NAME" in names
      assert "EQUALS_EQUALS" in names
      assert "COLON" in names
      assert "if" in grammar.keywords
      assert "def" in grammar.keywords
      assert "True" in grammar.keywords
    end

    test "accepts an explicit version string" do
      grammar = PythonLexer.create_lexer("3.12")
      names = Enum.map(grammar.definitions, & &1.name)
      assert "NAME" in names
    end
  end

  describe "tokenize/2" do
    test "tokenizes a basic assignment expression" do
      types = token_types("x = 1 + 2\n")
      assert "NAME" in types
      assert "EQUALS" in types
      assert "INT" in types
      assert "PLUS" in types
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

    test "tokenizes @ as AT token (decorator operator)" do
      {:ok, tokens} = PythonLexer.tokenize("@\n")
      types = Enum.map(tokens, & &1.type)
      assert "AT" in types
    end

    test "accepts an explicit version parameter" do
      types = token_types("x = 1\n", "3.12")
      assert "NAME" in types
      assert "INT" in types
    end

    test "nil version defaults to 3.12" do
      types = token_types("x = 1\n", nil)
      assert "NAME" in types
      assert "INT" in types
    end

    test "empty string version defaults to 3.12" do
      types = token_types("x = 1\n", "")
      assert "NAME" in types
      assert "INT" in types
    end
  end
end
