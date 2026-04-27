defmodule CodingAdventures.NibLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.NibLexer

  test "tokenizes simple function" do
    {:ok, tokens} = NibLexer.tokenize_nib("fn main() { return 0; }")

    assert Enum.map(tokens, & &1.type) ==
             ["KEYWORD", "NAME", "LPAREN", "RPAREN", "LBRACE", "KEYWORD", "INT_LIT", "SEMICOLON", "RBRACE", "EOF"]
  end

  test "keeps multicharacter operators intact" do
    {:ok, tokens} = NibLexer.tokenize_nib("1 +% 2 +? 3")
    assert Enum.map(tokens, & &1.value) == ["1", "+%", "2", "+?", "3", ""]
  end
end
