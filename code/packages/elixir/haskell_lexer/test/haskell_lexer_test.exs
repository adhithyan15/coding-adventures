defmodule CodingAdventures.HaskellLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.HaskellLexer

  test "default version tokenizes with haskell2010" do
    {:ok, tokens} = HaskellLexer.tokenize("x")
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
end
