defmodule CodingAdventures.FSharpLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.FSharpLexer

  defp token_types(tokens) do
    Enum.map(tokens, & &1.type)
  end

  describe "version helpers" do
    test "default_version is F# 10" do
      assert FSharpLexer.default_version() == "10"
    end

    test "supported_versions includes all expected releases" do
      versions = FSharpLexer.supported_versions()

      assert "1.0" in versions
      assert "2.0" in versions
      assert "3.0" in versions
      assert "3.1" in versions
      assert "4.0" in versions
      assert "4.1" in versions
      assert "4.5" in versions
      assert "4.6" in versions
      assert "4.7" in versions
      assert "5" in versions
      assert "6" in versions
      assert "7" in versions
      assert "8" in versions
      assert "9" in versions
      assert "10" in versions
    end
  end

  describe "tokenize/2" do
    test "tokenizes a simple let binding with the default grammar" do
      assert {:ok, tokens} = FSharpLexer.tokenize("let value = 1")
      assert token_types(tokens) == ["KEYWORD", "NAME", "EQUALS", "NUMBER", "EOF"]
      assert Enum.map(tokens, & &1.value) == ["let", "value", "=", "1", ""]
    end

    test "tokenizes a small F# entry point with layout-sensitive newlines" do
      source = """
      [<EntryPoint>]
      let main _ =
          printfn "Hello, World!"
          0
      """

      assert {:ok, tokens} = FSharpLexer.tokenize(source)

      assert token_types(tokens) == [
               "LBRACKET",
               "LESS_THAN",
               "NAME",
               "GREATER_THAN",
               "RBRACKET",
               "NEWLINE",
               "KEYWORD",
               "NAME",
               "UNDERSCORE",
               "EQUALS",
               "NEWLINE",
               "NAME",
               "STRING",
               "NEWLINE",
               "NUMBER",
               "NEWLINE",
               "EOF"
             ]
    end

    test "treats nil and empty version as F# 10" do
      assert {:ok, nil_version_tokens} = FSharpLexer.tokenize("let value = 1", nil)
      assert {:ok, empty_version_tokens} = FSharpLexer.tokenize("let value = 1", "")

      assert token_types(nil_version_tokens) == token_types(empty_version_tokens)
    end

    test "accepts all declared version strings" do
      for version <- FSharpLexer.supported_versions() do
        assert {:ok, tokens} = FSharpLexer.tokenize("let value = 1", version)
        assert hd(token_types(tokens)) == "KEYWORD"
      end
    end

    test "returns only EOF for empty input" do
      assert {:ok, tokens} = FSharpLexer.tokenize("")
      assert token_types(tokens) == ["EOF"]
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown F# version "99"/, fn ->
        FSharpLexer.tokenize("let value = 1", "99")
      end
    end
  end

  describe "create_lexer/1" do
    test "returns the parsed token grammar for the default version" do
      grammar = FSharpLexer.create_lexer()

      assert is_map(grammar)
      assert grammar.version == 1
      assert Enum.member?(grammar.keywords, "let")
      assert Enum.member?(grammar.keywords, "module")
    end

    test "returns the parsed token grammar for a specific version" do
      grammar = FSharpLexer.create_lexer("4.0")

      assert is_map(grammar)
      assert grammar.version == 1
      assert Enum.member?(grammar.keywords, "let")
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown F# version/, fn ->
        FSharpLexer.create_lexer("latest")
      end
    end
  end
end
