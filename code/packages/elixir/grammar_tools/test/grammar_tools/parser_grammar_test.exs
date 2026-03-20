defmodule CodingAdventures.GrammarTools.ParserGrammarTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.GrammarTools.ParserGrammar

  describe "parse/1 — basic rules" do
    test "parses a simple rule with a token reference" do
      {:ok, grammar} = ParserGrammar.parse("value = NUMBER ;")
      assert length(grammar.rules) == 1
      [rule] = grammar.rules
      assert rule.name == "value"
      assert rule.body == {:rule_reference, "NUMBER", true}
    end

    test "parses a rule with a rule reference" do
      {:ok, grammar} = ParserGrammar.parse("program = value ;")
      [rule] = grammar.rules
      assert rule.body == {:rule_reference, "value", false}
    end

    test "parses multiple rules" do
      source = """
      value = NUMBER ;
      program = value ;
      """

      {:ok, grammar} = ParserGrammar.parse(source)
      assert length(grammar.rules) == 2
    end
  end

  describe "parse/1 — alternation" do
    test "parses alternation with pipe" do
      {:ok, grammar} = ParserGrammar.parse("value = NUMBER | STRING ;")
      [rule] = grammar.rules

      assert {:alternation, [
               {:rule_reference, "NUMBER", true},
               {:rule_reference, "STRING", true}
             ]} = rule.body
    end

    test "parses three-way alternation" do
      {:ok, grammar} = ParserGrammar.parse("value = NUMBER | STRING | NAME ;")
      [rule] = grammar.rules
      {:alternation, choices} = rule.body
      assert length(choices) == 3
    end
  end

  describe "parse/1 — sequence" do
    test "parses sequence of elements" do
      {:ok, grammar} = ParserGrammar.parse("pair = STRING COLON NUMBER ;")
      [rule] = grammar.rules

      assert {:sequence, [
               {:rule_reference, "STRING", true},
               {:rule_reference, "COLON", true},
               {:rule_reference, "NUMBER", true}
             ]} = rule.body
    end
  end

  describe "parse/1 — repetition" do
    test "parses zero-or-more repetition" do
      {:ok, grammar} = ParserGrammar.parse("items = { NUMBER } ;")
      [rule] = grammar.rules
      assert {:repetition, {:rule_reference, "NUMBER", true}} = rule.body
    end
  end

  describe "parse/1 — optional" do
    test "parses optional element" do
      {:ok, grammar} = ParserGrammar.parse("maybe = [ NUMBER ] ;")
      [rule] = grammar.rules
      assert {:optional, {:rule_reference, "NUMBER", true}} = rule.body
    end
  end

  describe "parse/1 — group" do
    test "parses grouped alternation" do
      {:ok, grammar} = ParserGrammar.parse("op = ( PLUS | MINUS ) ;")
      [rule] = grammar.rules

      assert {:group, {:alternation, [
               {:rule_reference, "PLUS", true},
               {:rule_reference, "MINUS", true}
             ]}} = rule.body
    end
  end

  describe "parse/1 — literals" do
    test "parses literal string" do
      {:ok, grammar} = ParserGrammar.parse(~s(stmt = "return" NUMBER ;))
      [rule] = grammar.rules

      assert {:sequence, [
               {:literal, "return"},
               {:rule_reference, "NUMBER", true}
             ]} = rule.body
    end
  end

  describe "parse/1 — complex rules" do
    test "parses the JSON grammar" do
      grammar_dir =
        Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"])
        |> Path.expand()

      json_grammar = File.read!(Path.join(grammar_dir, "json.grammar"))
      {:ok, grammar} = ParserGrammar.parse(json_grammar)

      rule_names = Enum.map(grammar.rules, & &1.name)
      assert "value" in rule_names
      assert "object" in rule_names
      assert "pair" in rule_names
      assert "array" in rule_names
      assert length(grammar.rules) == 4
    end

    test "parses comma-separated list pattern" do
      source = "list = LBRACKET [ value { COMMA value } ] RBRACKET ;"
      {:ok, grammar} = ParserGrammar.parse(source)
      assert length(grammar.rules) == 1
    end
  end

  describe "parse/1 — comments" do
    test "skips comment lines" do
      source = """
      # This is a comment
      value = NUMBER ;
      # Another comment
      """

      {:ok, grammar} = ParserGrammar.parse(source)
      assert length(grammar.rules) == 1
    end
  end

  describe "parse/1 — error cases" do
    test "error on missing semicolon" do
      {:error, msg} = ParserGrammar.parse("value = NUMBER")
      assert msg =~ "Expected ';'"
    end

    test "error on missing equals" do
      {:error, msg} = ParserGrammar.parse("value NUMBER ;")
      assert msg =~ "Expected '='"
    end
  end

  describe "rule_names/1" do
    test "returns all rule names" do
      source = """
      value = NUMBER ;
      pair = STRING COLON value ;
      """

      {:ok, grammar} = ParserGrammar.parse(source)
      names = ParserGrammar.rule_names(grammar)
      assert MapSet.member?(names, "value")
      assert MapSet.member?(names, "pair")
    end
  end

  describe "token_references/1" do
    test "returns all UPPERCASE references" do
      source = "pair = STRING COLON value ;"
      {:ok, grammar} = ParserGrammar.parse(source)
      refs = ParserGrammar.token_references(grammar)
      assert MapSet.member?(refs, "STRING")
      assert MapSet.member?(refs, "COLON")
      refute MapSet.member?(refs, "value")
    end
  end

  describe "rule_references/1" do
    test "returns all lowercase references" do
      source = "object = LBRACE pair RBRACE ;"
      {:ok, grammar} = ParserGrammar.parse(source)
      refs = ParserGrammar.rule_references(grammar)
      assert MapSet.member?(refs, "pair")
      refute MapSet.member?(refs, "LBRACE")
    end
  end
end
