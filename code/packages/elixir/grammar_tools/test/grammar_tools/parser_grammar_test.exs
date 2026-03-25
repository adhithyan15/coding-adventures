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

  # ---------------------------------------------------------------------------
  # Magic Comments
  # ---------------------------------------------------------------------------
  #
  # Magic comments (`# @key value`) are extracted in a pre-pass before
  # tokenisation, so they never interfere with EBNF rule parsing.
  #
  # Currently supported keys:
  #   @version N  — integer schema version (default 0)

  describe "parse/1 — magic comments" do
    test "# @version 1 sets version to 1" do
      source = """
      # @version 1
      value = NUMBER ;
      """

      {:ok, grammar} = ParserGrammar.parse(source)
      assert grammar.version == 1
    end

    test "missing version comment defaults to 0" do
      {:ok, grammar} = ParserGrammar.parse("value = NUMBER ;")
      assert grammar.version == 0
    end

    test "magic comment does not appear as a rule" do
      source = """
      # @version 5
      value = NUMBER ;
      """

      {:ok, grammar} = ParserGrammar.parse(source)
      assert length(grammar.rules) == 1
    end

    test "unknown magic comment key is silently ignored" do
      source = """
      # @future_feature on
      value = NUMBER ;
      """

      {:ok, grammar} = ParserGrammar.parse(source)
      assert grammar.version == 0
      assert length(grammar.rules) == 1
    end

    test "magic comment coexists with ordinary comments and rules" do
      source = """
      # @version 7
      # Plain comment — not a magic comment
      value = NUMBER | STRING ;
      program = value ;
      """

      {:ok, grammar} = ParserGrammar.parse(source)
      assert grammar.version == 7
      assert length(grammar.rules) == 2
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

  # ---------------------------------------------------------------------------
  # validate_parser_grammar/2
  # ---------------------------------------------------------------------------
  #
  # The validator runs a lint pass over a parsed grammar, catching semantic
  # issues such as undefined rule references, duplicate rule names, and
  # unreachable rules.

  describe "validate_parser_grammar/2 — valid grammar" do
    test "returns empty list for a valid grammar" do
      # array is the start rule; it references value which is defined.
      # Neither rule is unreachable (array is start, value is referenced by array).
      source = """
      array = LBRACKET value RBRACKET ;
      value = NUMBER ;
      """

      {:ok, grammar} = ParserGrammar.parse(source)
      assert ParserGrammar.validate_parser_grammar(grammar) == []
    end
  end

  describe "validate_parser_grammar/2 — duplicate rule names" do
    test "reports duplicate rule name" do
      # Build a grammar struct directly to simulate duplicate names
      rule1 = %{name: "value", body: {:rule_reference, "NUMBER", true}, line_number: 1}
      rule2 = %{name: "value", body: {:rule_reference, "STRING", true}, line_number: 2}
      grammar = %ParserGrammar{rules: [rule1, rule2]}
      issues = ParserGrammar.validate_parser_grammar(grammar)
      assert Enum.any?(issues, &(&1 =~ "Duplicate rule name 'value'"))
    end
  end

  describe "validate_parser_grammar/2 — non-lowercase rule names" do
    test "reports UPPERCASE rule name" do
      rule = %{name: "Value", body: {:rule_reference, "NUMBER", true}, line_number: 1}
      grammar = %ParserGrammar{rules: [rule]}
      issues = ParserGrammar.validate_parser_grammar(grammar)
      assert Enum.any?(issues, &(&1 =~ "should be lowercase"))
    end
  end

  describe "validate_parser_grammar/2 — undefined rule references" do
    test "reports undefined rule reference" do
      source = "program = expression ;"
      {:ok, grammar} = ParserGrammar.parse(source)
      issues = ParserGrammar.validate_parser_grammar(grammar)
      assert Enum.any?(issues, &(&1 =~ "Undefined rule reference: 'expression'"))
    end

    test "no issue when all rule references are defined" do
      source = """
      program = expression ;
      expression = NUMBER ;
      """

      {:ok, grammar} = ParserGrammar.parse(source)
      issues = ParserGrammar.validate_parser_grammar(grammar)
      refute Enum.any?(issues, &(&1 =~ "Undefined rule reference"))
    end
  end

  describe "validate_parser_grammar/2 — undefined token references" do
    test "reports undefined token reference when token_names provided" do
      source = "value = NUMBER | STRING ;"
      {:ok, grammar} = ParserGrammar.parse(source)
      token_names = MapSet.new(["NUMBER"])
      issues = ParserGrammar.validate_parser_grammar(grammar, token_names)
      assert Enum.any?(issues, &(&1 =~ "Undefined token reference: 'STRING'"))
      refute Enum.any?(issues, &(&1 =~ "Undefined token reference: 'NUMBER'"))
    end

    test "no undefined token check when token_names is nil" do
      source = "value = TOTALLY_UNDEFINED ;"
      {:ok, grammar} = ParserGrammar.parse(source)
      issues = ParserGrammar.validate_parser_grammar(grammar, nil)
      refute Enum.any?(issues, &(&1 =~ "Undefined token reference"))
    end

    test "synthetic tokens are always valid" do
      source = "program = NEWLINE INDENT NAME DEDENT EOF ;"
      {:ok, grammar} = ParserGrammar.parse(source)
      # Only NAME is undefined; synthetic tokens should not trigger an error
      token_names = MapSet.new([])
      issues = ParserGrammar.validate_parser_grammar(grammar, token_names)
      refute Enum.any?(issues, &(&1 =~ "Undefined token reference: 'NEWLINE'"))
      refute Enum.any?(issues, &(&1 =~ "Undefined token reference: 'INDENT'"))
      refute Enum.any?(issues, &(&1 =~ "Undefined token reference: 'DEDENT'"))
      refute Enum.any?(issues, &(&1 =~ "Undefined token reference: 'EOF'"))
      assert Enum.any?(issues, &(&1 =~ "Undefined token reference: 'NAME'"))
    end
  end

  describe "validate_parser_grammar/2 — unreachable rules" do
    test "reports unreachable rule" do
      source = """
      program = NUMBER ;
      orphan = STRING ;
      """

      {:ok, grammar} = ParserGrammar.parse(source)
      issues = ParserGrammar.validate_parser_grammar(grammar)
      assert Enum.any?(issues, &(&1 =~ "Rule 'orphan' is defined but never referenced"))
    end

    test "start rule is exempt from unreachable check" do
      source = "program = NUMBER ;"
      {:ok, grammar} = ParserGrammar.parse(source)
      issues = ParserGrammar.validate_parser_grammar(grammar)
      refute Enum.any?(issues, &(&1 =~ "Rule 'program'"))
    end

    test "referenced rules are not flagged as unreachable" do
      source = """
      program = expression ;
      expression = NUMBER ;
      """

      {:ok, grammar} = ParserGrammar.parse(source)
      issues = ParserGrammar.validate_parser_grammar(grammar)
      refute Enum.any?(issues, &(&1 =~ "unreachable"))
    end
  end
end
