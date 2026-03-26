# AUTO-GENERATED FILE - DO NOT EDIT
defmodule JsonGrammar do
  alias CodingAdventures.GrammarTools.ParserGrammar

  def grammar do
    %ParserGrammar{
      version: 1,
      rules: [
        %{name: "value", line_number: 28, body: {:alternation, [{:rule_reference, "object", false}, {:rule_reference, "array", false}, {:rule_reference, "STRING", true}, {:rule_reference, "NUMBER", true}, {:rule_reference, "TRUE", true}, {:rule_reference, "FALSE", true}, {:rule_reference, "NULL", true}]}},
        %{name: "object", line_number: 34, body: {:sequence, [{:rule_reference, "LBRACE", true}, {:optional, {:sequence, [{:rule_reference, "pair", false}, {:repetition, {:sequence, [{:rule_reference, "COMMA", true}, {:rule_reference, "pair", false}]}}]}}, {:rule_reference, "RBRACE", true}]}},
        %{name: "pair", line_number: 38, body: {:sequence, [{:rule_reference, "STRING", true}, {:rule_reference, "COLON", true}, {:rule_reference, "value", false}]}},
        %{name: "array", line_number: 42, body: {:sequence, [{:rule_reference, "LBRACKET", true}, {:optional, {:sequence, [{:rule_reference, "value", false}, {:repetition, {:sequence, [{:rule_reference, "COMMA", true}, {:rule_reference, "value", false}]}}]}}, {:rule_reference, "RBRACKET", true}]}}
      ]
    }
  end
end
