defmodule CodingAdventures.LispParser.Grammar do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: lisp.grammar
  # Regenerate with: grammar-tools compile-grammar lisp.grammar
  #
  # This file embeds a ParserGrammar as native Elixir data structures.
  # Call parser_grammar/0 instead of reading and parsing the .grammar file.

  alias CodingAdventures.GrammarTools.ParserGrammar

  def parser_grammar do
    %ParserGrammar{
      rules: [
        %{
          name: "program",
          body: {:repetition, {:rule_reference, "sexpr", false}},
          line_number: 2,
        },
        %{
          name: "sexpr",
          body: {:alternation, [
            {:rule_reference, "atom", false},
            {:rule_reference, "list", false},
            {:rule_reference, "quoted", false},
          ]},
          line_number: 3,
        },
        %{
          name: "atom",
          body: {:alternation, [
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "SYMBOL", true},
            {:rule_reference, "STRING", true},
          ]},
          line_number: 4,
        },
        %{
          name: "list",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "list_body", false},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 5,
        },
        %{
          name: "list_body",
          body: {:optional, {:sequence, [
              {:rule_reference, "sexpr", false},
              {:repetition, {:rule_reference, "sexpr", false}},
              {:optional, {:sequence, [
                  {:rule_reference, "DOT", true},
                  {:rule_reference, "sexpr", false},
                ]}},
            ]}},
          line_number: 6,
        },
        %{
          name: "quoted",
          body: {:sequence, [
            {:rule_reference, "QUOTE", true},
            {:rule_reference, "sexpr", false},
          ]},
          line_number: 7,
        },
      ],
      version: 1,
    }
  end

end
