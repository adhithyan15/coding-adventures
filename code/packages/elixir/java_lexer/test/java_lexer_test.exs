defmodule CodingAdventures.JavaLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.JavaLexer

  defp token_types(tokens) do
    Enum.map(tokens, & &1.type)
  end

  describe "version helpers" do
    test "default_version is Java 21" do
      assert JavaLexer.default_version() == "21"
    end

    test "supported_versions includes all expected releases" do
      versions = JavaLexer.supported_versions()

      assert "1.0" in versions
      assert "1.1" in versions
      assert "1.4" in versions
      assert "5" in versions
      assert "7" in versions
      assert "8" in versions
      assert "10" in versions
      assert "14" in versions
      assert "17" in versions
      assert "21" in versions
    end
  end

  describe "tokenize/2" do
    test "tokenizes a simple class declaration with the default grammar" do
      assert {:ok, tokens} = JavaLexer.tokenize("public class Hello { }")
      assert token_types(tokens) == ["KEYWORD", "KEYWORD", "NAME", "LBRACE", "RBRACE", "EOF"]
      assert Enum.map(tokens, & &1.value) == ["public", "class", "Hello", "{", "}", ""]
    end

    test "treats nil and empty version as Java 21" do
      assert {:ok, nil_version_tokens} = JavaLexer.tokenize("class Hello { }", nil)
      assert {:ok, empty_version_tokens} = JavaLexer.tokenize("class Hello { }", "")

      assert token_types(nil_version_tokens) == token_types(empty_version_tokens)
    end

    test "accepts all declared version strings" do
      for version <- JavaLexer.supported_versions() do
        assert {:ok, tokens} = JavaLexer.tokenize("class Hello { }", version)
        assert hd(token_types(tokens)) == "KEYWORD"
      end
    end

    test "tokenizes Java 1.0 style declarations" do
      assert {:ok, tokens} = JavaLexer.tokenize("int x = 1;", "1.0")
      assert token_types(tokens) == ["KEYWORD", "NAME", "EQUALS", "NUMBER", "SEMICOLON", "EOF"]
    end

    test "returns only EOF for empty input" do
      assert {:ok, tokens} = JavaLexer.tokenize("")
      assert token_types(tokens) == ["EOF"]
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown Java version "99"/, fn ->
        JavaLexer.tokenize("class Hello { }", "99")
      end
    end
  end

  describe "create_lexer/1" do
    test "returns the parsed token grammar for the default version" do
      grammar = JavaLexer.create_lexer()

      assert is_map(grammar)
      assert grammar.version == 1
      assert Enum.member?(grammar.keywords, "class")
      assert Enum.member?(grammar.keywords, "public")
    end

    test "returns the parsed token grammar for a specific version" do
      grammar = JavaLexer.create_lexer("8")

      assert is_map(grammar)
      assert grammar.version == 1
      assert Enum.member?(grammar.keywords, "class")
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown Java version/, fn ->
        JavaLexer.create_lexer("latest")
      end
    end
  end
end
