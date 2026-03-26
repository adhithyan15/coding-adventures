# AUTO-GENERATED FILE - DO NOT EDIT
defmodule TomlGrammar do
  alias CodingAdventures.GrammarTools.ParserGrammar

  def grammar do
    %ParserGrammar{
      version: 1,
      rules: [
        %{name: "document", line_number: 38, body: {:repetition, {:alternation, [{:rule_reference, "NEWLINE", true}, {:rule_reference, "expression", false}]}}},
        %{name: "expression", line_number: 49, body: {:alternation, [{:rule_reference, "array_table_header", false}, {:rule_reference, "table_header", false}, {:rule_reference, "keyval", false}]}},
        %{name: "keyval", line_number: 57, body: {:sequence, [{:rule_reference, "key", false}, {:rule_reference, "EQUALS", true}, {:rule_reference, "value", false}]}},
        %{name: "key", line_number: 65, body: {:sequence, [{:rule_reference, "simple_key", false}, {:repetition, {:sequence, [{:rule_reference, "DOT", true}, {:rule_reference, "simple_key", false}]}}]}},
        %{name: "simple_key", line_number: 82, body: {:alternation, [{:rule_reference, "BARE_KEY", true}, {:rule_reference, "BASIC_STRING", true}, {:rule_reference, "LITERAL_STRING", true}, {:rule_reference, "TRUE", true}, {:rule_reference, "FALSE", true}, {:rule_reference, "INTEGER", true}, {:rule_reference, "FLOAT", true}, {:rule_reference, "OFFSET_DATETIME", true}, {:rule_reference, "LOCAL_DATETIME", true}, {:rule_reference, "LOCAL_DATE", true}, {:rule_reference, "LOCAL_TIME", true}]}},
        %{name: "table_header", line_number: 92, body: {:sequence, [{:rule_reference, "LBRACKET", true}, {:rule_reference, "key", false}, {:rule_reference, "RBRACKET", true}]}},
        %{name: "array_table_header", line_number: 104, body: {:sequence, [{:rule_reference, "LBRACKET", true}, {:rule_reference, "LBRACKET", true}, {:rule_reference, "key", false}, {:rule_reference, "RBRACKET", true}, {:rule_reference, "RBRACKET", true}]}},
        %{name: "value", line_number: 121, body: {:alternation, [{:rule_reference, "BASIC_STRING", true}, {:rule_reference, "ML_BASIC_STRING", true}, {:rule_reference, "LITERAL_STRING", true}, {:rule_reference, "ML_LITERAL_STRING", true}, {:rule_reference, "INTEGER", true}, {:rule_reference, "FLOAT", true}, {:rule_reference, "TRUE", true}, {:rule_reference, "FALSE", true}, {:rule_reference, "OFFSET_DATETIME", true}, {:rule_reference, "LOCAL_DATETIME", true}, {:rule_reference, "LOCAL_DATE", true}, {:rule_reference, "LOCAL_TIME", true}, {:rule_reference, "array", false}, {:rule_reference, "inline_table", false}]}},
        %{name: "array", line_number: 140, body: {:sequence, [{:rule_reference, "LBRACKET", true}, {:rule_reference, "array_values", false}, {:rule_reference, "RBRACKET", true}]}},
        %{name: "array_values", line_number: 142, body: {:sequence, [{:repetition, {:rule_reference, "NEWLINE", true}}, {:optional, {:sequence, [{:rule_reference, "value", false}, {:repetition, {:rule_reference, "NEWLINE", true}}, {:repetition, {:sequence, [{:rule_reference, "COMMA", true}, {:repetition, {:rule_reference, "NEWLINE", true}}, {:rule_reference, "value", false}, {:repetition, {:rule_reference, "NEWLINE", true}}]}}, {:optional, {:rule_reference, "COMMA", true}}, {:repetition, {:rule_reference, "NEWLINE", true}}]}}]}},
        %{name: "inline_table", line_number: 162, body: {:sequence, [{:rule_reference, "LBRACE", true}, {:optional, {:sequence, [{:rule_reference, "keyval", false}, {:repetition, {:sequence, [{:rule_reference, "COMMA", true}, {:rule_reference, "keyval", false}]}}]}}, {:rule_reference, "RBRACE", true}]}}
      ]
    }
  end
end
