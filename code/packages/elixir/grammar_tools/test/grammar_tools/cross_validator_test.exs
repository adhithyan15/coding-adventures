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
