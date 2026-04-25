defmodule CodingAdventures.LispLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.LispLexer

  test "tokenizes a basic definition" do
    {:ok, tokens} = LispLexer.tokenize("(define x 42)")

    assert Enum.map(tokens, & &1.type) == [
             "LPAREN",
             "SYMBOL",
             "SYMBOL",
             "NUMBER",
             "RPAREN",
             "EOF"
           ]

    assert Enum.map(tokens, & &1.value) == ["(", "define", "x", "42", ")", ""]
  end

  test "keeps operator names as symbols and negative numbers as numbers" do
    {:ok, tokens} = LispLexer.tokenize("(+ -42 (* x 2))")
    assert Enum.at(tokens, 1).type == "SYMBOL"
    assert Enum.at(tokens, 1).value == "+"
    assert Enum.at(tokens, 2).type == "NUMBER"
    assert Enum.at(tokens, 2).value == "-42"
    assert Enum.at(tokens, 4).value == "*"
  end

  test "skips comments and tokenizes quoted dotted pairs" do
    {:ok, tokens} = LispLexer.tokenize("; ignore\n'(a . b)")

    assert Enum.map(tokens, & &1.type) == [
             "QUOTE",
             "LPAREN",
             "SYMBOL",
             "DOT",
             "SYMBOL",
             "RPAREN",
             "EOF"
           ]
  end

  test "create_lexer returns the parsed token grammar" do
    grammar = LispLexer.create_lexer()
    assert is_map(grammar)
    assert Enum.any?(grammar.definitions, &(&1.name == "SYMBOL"))
  end

  test "invalid characters return a lexer error" do
    assert {:error, message} = LispLexer.tokenize("@")
    assert message =~ "Unexpected character"
  end
end
