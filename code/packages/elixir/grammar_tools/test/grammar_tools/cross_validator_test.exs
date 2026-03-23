defmodule CodingAdventures.GrammarTools.CrossValidatorTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.GrammarTools.{TokenGrammar, ParserGrammar, CrossValidator}

  test "no issues when all references match" do
    {:ok, tg} = TokenGrammar.parse("NUMBER = /[0-9]+/")
    {:ok, pg} = ParserGrammar.parse("value = NUMBER ;")
    assert CrossValidator.validate(tg, pg) == []
  end

  test "reports undefined token references" do
    {:ok, tg} = TokenGrammar.parse("NUMBER = /[0-9]+/")
    {:ok, pg} = ParserGrammar.parse("value = NUMBER | STRING ;")
    issues = CrossValidator.validate(tg, pg)
    assert Enum.any?(issues, &(&1 =~ "Undefined token reference: 'STRING'"))
  end

  test "reports unused token definitions" do
    {:ok, tg} =
      TokenGrammar.parse("""
      NUMBER = /[0-9]+/
      STRING = /"[^"]*"/
      """)

    {:ok, pg} = ParserGrammar.parse("value = NUMBER ;")
    issues = CrossValidator.validate(tg, pg)
    assert Enum.any?(issues, &(&1 =~ "Token 'STRING' is defined but never referenced"))
  end

  test "NEWLINE is always valid, INDENT/DEDENT only in indentation mode" do
    # NEWLINE is always a valid synthetic token because the lexer emits it
    # whenever a bare newline is encountered and no skip pattern consumed it.
    # INDENT/DEDENT are only valid in indentation mode.
    {:ok, tg} = TokenGrammar.parse("NAME = /[a-z]+/")
    {:ok, pg} = ParserGrammar.parse("file = NAME NEWLINE INDENT NAME DEDENT ;")
    issues = CrossValidator.validate(tg, pg)
    undefined = Enum.filter(issues, &String.starts_with?(&1, "Undefined"))
    assert Enum.any?(undefined, &(&1 =~ "INDENT"))
    assert Enum.any?(undefined, &(&1 =~ "DEDENT"))
    refute Enum.any?(undefined, &(&1 =~ "NEWLINE"))
  end

  test "EOF is always implicitly available" do
    {:ok, tg} = TokenGrammar.parse("NAME = /[a-z]+/")
    {:ok, pg} = ParserGrammar.parse("file = NAME EOF ;")
    issues = CrossValidator.validate(tg, pg)
    undefined = Enum.filter(issues, &String.starts_with?(&1, "Undefined"))
    refute Enum.any?(undefined, &(&1 =~ "EOF"))
  end

  test "validates json.tokens against json.grammar" do
    grammar_dir =
      Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"])
      |> Path.expand()

    {:ok, tg} = TokenGrammar.parse(File.read!(Path.join(grammar_dir, "json.tokens")))
    {:ok, pg} = ParserGrammar.parse(File.read!(Path.join(grammar_dir, "json.grammar")))
    issues = CrossValidator.validate(tg, pg)
    # No undefined references — all tokens used in grammar should exist
    undefined = Enum.filter(issues, &String.starts_with?(&1, "Undefined"))
    assert undefined == []
  end
end
