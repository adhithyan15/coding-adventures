defmodule CodingAdventures.TomlParser.Grammar do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: toml.grammar
  # Regenerate with: grammar-tools compile-grammar toml.grammar
  #
  # This file embeds a ParserGrammar as native Elixir data structures.
  # Call parser_grammar/0 instead of reading and parsing the .grammar file.
  
  alias CodingAdventures.GrammarTools.ParserGrammar
  
  def parser_grammar do
    %ParserGrammar{
      rules: [
        %{
          name: "document",
          body: {:repetition, {:alternation, [
              {:rule_reference, "NEWLINE", true},
              {:rule_reference, "expression", false},
            ]}},
          line_number: 38,
        },
        %{
          name: "expression",
          body: {:alternation, [
            {:rule_reference, "array_table_header", false},
            {:rule_reference, "table_header", false},
            {:rule_reference, "keyval", false},
          ]},
          line_number: 49,
        },
        %{
          name: "keyval",
          body: {:sequence, [
            {:rule_reference, "key", false},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "value", false},
          ]},
          line_number: 57,
        },
        %{
          name: "key",
          body: {:sequence, [
            {:rule_reference, "simple_key", false},
            {:repetition, {:sequence, [
                {:rule_reference, "DOT", true},
                {:rule_reference, "simple_key", false},
              ]}},
          ]},
          line_number: 65,
        },
        %{
          name: "simple_key",
          body: {:alternation, [
            {:rule_reference, "BARE_KEY", true},
            {:rule_reference, "BASIC_STRING", true},
            {:rule_reference, "LITERAL_STRING", true},
            {:rule_reference, "TRUE", true},
            {:rule_reference, "FALSE", true},
            {:rule_reference, "INTEGER", true},
            {:rule_reference, "FLOAT", true},
            {:rule_reference, "OFFSET_DATETIME", true},
            {:rule_reference, "LOCAL_DATETIME", true},
            {:rule_reference, "LOCAL_DATE", true},
            {:rule_reference, "LOCAL_TIME", true},
          ]},
          line_number: 82,
        },
        %{
          name: "table_header",
          body: {:sequence, [
            {:rule_reference, "LBRACKET", true},
            {:rule_reference, "key", false},
            {:rule_reference, "RBRACKET", true},
          ]},
          line_number: 92,
        },
        %{
          name: "array_table_header",
          body: {:sequence, [
            {:rule_reference, "LBRACKET", true},
            {:rule_reference, "LBRACKET", true},
            {:rule_reference, "key", false},
            {:rule_reference, "RBRACKET", true},
            {:rule_reference, "RBRACKET", true},
          ]},
          line_number: 104,
        },
        %{
          name: "value",
          body: {:alternation, [
            {:rule_reference, "BASIC_STRING", true},
            {:rule_reference, "ML_BASIC_STRING", true},
            {:rule_reference, "LITERAL_STRING", true},
            {:rule_reference, "ML_LITERAL_STRING", true},
            {:rule_reference, "INTEGER", true},
            {:rule_reference, "FLOAT", true},
            {:rule_reference, "TRUE", true},
            {:rule_reference, "FALSE", true},
            {:rule_reference, "OFFSET_DATETIME", true},
            {:rule_reference, "LOCAL_DATETIME", true},
            {:rule_reference, "LOCAL_DATE", true},
            {:rule_reference, "LOCAL_TIME", true},
            {:rule_reference, "array", false},
            {:rule_reference, "inline_table", false},
          ]},
          line_number: 121,
        },
        %{
          name: "array",
          body: {:sequence, [
            {:rule_reference, "LBRACKET", true},
            {:rule_reference, "array_values", false},
            {:rule_reference, "RBRACKET", true},
          ]},
          line_number: 140,
        },
        %{
          name: "array_values",
          body: {:sequence, [
            {:repetition, {:rule_reference, "NEWLINE", true}},
            {:optional, {:sequence, [
                {:rule_reference, "value", false},
                {:repetition, {:rule_reference, "NEWLINE", true}},
                {:repetition, {:sequence, [
                    {:rule_reference, "COMMA", true},
                    {:repetition, {:rule_reference, "NEWLINE", true}},
                    {:rule_reference, "value", false},
                    {:repetition, {:rule_reference, "NEWLINE", true}},
                  ]}},
                {:optional, {:rule_reference, "COMMA", true}},
                {:repetition, {:rule_reference, "NEWLINE", true}},
              ]}},
          ]},
          line_number: 142,
        },
        %{
          name: "inline_table",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:optional, {:sequence, [
                {:rule_reference, "keyval", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "COMMA", true},
                    {:rule_reference, "keyval", false},
                  ]}},
              ]}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 162,
        },
      ],
      version: 1,
    }
  end
end
