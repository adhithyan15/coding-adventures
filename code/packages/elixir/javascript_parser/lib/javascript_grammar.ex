# AUTO-GENERATED FILE - DO NOT EDIT
defmodule JavascriptGrammar do
  alias CodingAdventures.GrammarTools.ParserGrammar

  def grammar do
    %ParserGrammar{
      version: 1,
      rules: [
        %{name: "program", line_number: 23, body: {:repetition, {:rule_reference, "statement", false}}},
        %{name: "statement", line_number: 24, body: {:alternation, [{:rule_reference, "var_declaration", false}, {:rule_reference, "assignment", false}, {:rule_reference, "expression_stmt", false}]}},
        %{name: "var_declaration", line_number: 25, body: {:sequence, [{:rule_reference, "KEYWORD", true}, {:rule_reference, "NAME", true}, {:rule_reference, "EQUALS", true}, {:rule_reference, "expression", false}, {:rule_reference, "SEMICOLON", true}]}},
        %{name: "assignment", line_number: 26, body: {:sequence, [{:rule_reference, "NAME", true}, {:rule_reference, "EQUALS", true}, {:rule_reference, "expression", false}, {:rule_reference, "SEMICOLON", true}]}},
        %{name: "expression_stmt", line_number: 27, body: {:sequence, [{:rule_reference, "expression", false}, {:rule_reference, "SEMICOLON", true}]}},
        %{name: "expression", line_number: 28, body: {:sequence, [{:rule_reference, "term", false}, {:repetition, {:sequence, [{:group, {:alternation, [{:rule_reference, "PLUS", true}, {:rule_reference, "MINUS", true}]}}, {:rule_reference, "term", false}]}}]}},
        %{name: "term", line_number: 29, body: {:sequence, [{:rule_reference, "factor", false}, {:repetition, {:sequence, [{:group, {:alternation, [{:rule_reference, "STAR", true}, {:rule_reference, "SLASH", true}]}}, {:rule_reference, "factor", false}]}}]}},
        %{name: "factor", line_number: 30, body: {:alternation, [{:rule_reference, "NUMBER", true}, {:rule_reference, "STRING", true}, {:rule_reference, "NAME", true}, {:rule_reference, "KEYWORD", true}, {:sequence, [{:rule_reference, "LPAREN", true}, {:rule_reference, "expression", false}, {:rule_reference, "RPAREN", true}]}]}}
      ]
    }
  end
end
