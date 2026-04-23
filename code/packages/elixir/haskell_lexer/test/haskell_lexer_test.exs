defmodule CodingAdventures.HaskellLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.HaskellLexer

  test "default_version exposes 2010" do
    assert HaskellLexer.default_version() == "2010"
  end

  test "supported_versions includes all versioned grammars" do
    assert HaskellLexer.supported_versions() == ~w(1.0 1.1 1.2 1.3 1.4 98 2010)
  end

  test "default version tokenizes with haskell2010" do
    {:ok, tokens} = HaskellLexer.tokenize("x")
    assert hd(tokens).type == "NAME"
  end

  test "empty string version falls back to the default grammar" do
    {:ok, tokens} = HaskellLexer.tokenize("x", "")
    assert hd(tokens).type == "NAME"
  end

  test "layout mode injects virtual braces" do
    {:ok, tokens} = HaskellLexer.tokenize("let\n  x = y\nin x")
    types = Enum.map(tokens, & &1.type)
    assert "VIRTUAL_LBRACE" in types
    assert "VIRTUAL_RBRACE" in types
  end

  test "historical versions are routable" do
    {:ok, tokens} = HaskellLexer.tokenize("x", "98")
    assert hd(tokens).type == "NAME"
  end

  test "create_lexer returns a token grammar for a historical version" do
    grammar = HaskellLexer.create_lexer("98")
    assert is_map(grammar)
    assert Enum.member?(grammar.keywords, "let")
    assert grammar.mode == "layout"
  end

  test "unknown versions raise a helpful error" do
    assert_raise ArgumentError, ~r/Unknown Haskell version/, fn ->
      HaskellLexer.create_lexer("99")
    end
  end
end
