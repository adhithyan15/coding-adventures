defmodule CodingAdventures.JsonParser.Grammar do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: json.grammar
  # Regenerate with: grammar-tools compile-grammar json.grammar
  #
  # This file embeds a ParserGrammar as native Elixir data structures.
  # Call parser_grammar/0 instead of reading and parsing the .grammar file.
  
  alias CodingAdventures.GrammarTools.ParserGrammar
  
  def parser_grammar do
    %ParserGrammar{
      rules: [
        %{
          name: "value",
          body: {:alternation, [
            {:rule_reference, "object", false},
            {:rule_reference, "array", false},
            {:rule_reference, "STRING", true},
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "TRUE", true},
            {:rule_reference, "FALSE", true},
            {:rule_reference, "NULL", true},
          ]},
          line_number: 28,
        },
        %{
          name: "object",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:optional, {:sequence, [
                {:rule_reference, "pair", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "COMMA", true},
                    {:rule_reference, "pair", false},
                  ]}},
              ]}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 34,
        },
        %{
          name: "pair",
          body: {:sequence, [
            {:rule_reference, "STRING", true},
            {:rule_reference, "COLON", true},
            {:rule_reference, "value", false},
          ]},
          line_number: 38,
        },
        %{
          name: "array",
          body: {:sequence, [
            {:rule_reference, "LBRACKET", true},
            {:optional, {:sequence, [
                {:rule_reference, "value", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "COMMA", true},
                    {:rule_reference, "value", false},
                  ]}},
              ]}},
            {:rule_reference, "RBRACKET", true},
          ]},
          line_number: 42,
        },
      ],
      version: 1,
    }
  end
end
