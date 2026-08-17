defmodule CodingAdventures.PythonParser.Grammar do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: python.grammar
  # Regenerate with: grammar-tools compile-grammar python.grammar
  #
  # This file embeds a ParserGrammar as native Elixir data structures.
  # Call parser_grammar/0 instead of reading and parsing the .grammar file.
  
  alias CodingAdventures.GrammarTools.ParserGrammar
  
  def parser_grammar do
    %ParserGrammar{
      rules: [
        %{
          name: "program",
          body: {:repetition, {:alternation, [
              {:rule_reference, "NEWLINE", true},
              {:rule_reference, "statement", false},
            ]}},
          line_number: 17,
        },
        %{
          name: "statement",
          body: {:sequence, [
            {:group, {:alternation, [
                {:rule_reference, "assignment", false},
                {:rule_reference, "expression_stmt", false},
              ]}},
            {:optional, {:rule_reference, "NEWLINE", true}},
          ]},
          line_number: 18,
        },
        %{
          name: "assignment",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "expression", false},
          ]},
          line_number: 19,
        },
        %{
          name: "expression_stmt",
          body: {:rule_reference, "expression", false},
          line_number: 20,
        },
        %{
          name: "expression",
          body: {:sequence, [
            {:rule_reference, "term", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "PLUS", true},
                    {:rule_reference, "MINUS", true},
                  ]}},
                {:rule_reference, "term", false},
              ]}},
          ]},
          line_number: 21,
        },
        %{
          name: "term",
          body: {:sequence, [
            {:rule_reference, "factor", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "STAR", true},
                    {:rule_reference, "SLASH", true},
                  ]}},
                {:rule_reference, "factor", false},
              ]}},
          ]},
          line_number: 22,
        },
        %{
          name: "factor",
          body: {:alternation, [
            {:rule_reference, "INT", true},
            {:rule_reference, "FLOAT", true},
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "STRING", true},
            {:rule_reference, "NAME", true},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "expression", false},
              {:rule_reference, "RPAREN", true},
            ]},
          ]},
          line_number: 23,
        },
      ],
      version: 1,
    }
  end
  end
