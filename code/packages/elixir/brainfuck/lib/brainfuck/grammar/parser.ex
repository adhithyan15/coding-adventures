defmodule CodingAdventures.Brainfuck.Grammar.Parser do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: brainfuck.grammar
  # Regenerate with: grammar-tools compile-grammar brainfuck.grammar
  #
  # This file embeds a ParserGrammar as native Elixir data structures.
  # Call parser_grammar/0 instead of reading and parsing the .grammar file.
  
  alias CodingAdventures.GrammarTools.ParserGrammar
  
  def parser_grammar do
    %ParserGrammar{
      rules: [
        %{
          name: "program",
          body: {:repetition, {:rule_reference, "instruction", false}},
          line_number: 15,
        },
        %{
          name: "instruction",
          body: {:alternation, [
            {:rule_reference, "loop", false},
            {:rule_reference, "command", false},
          ]},
          line_number: 21,
        },
        %{
          name: "loop",
          body: {:sequence, [
            {:rule_reference, "LOOP_START", true},
            {:repetition, {:rule_reference, "instruction", false}},
            {:rule_reference, "LOOP_END", true},
          ]},
          line_number: 27,
        },
        %{
          name: "command",
          body: {:alternation, [
            {:rule_reference, "RIGHT", true},
            {:rule_reference, "LEFT", true},
            {:rule_reference, "INC", true},
            {:rule_reference, "DEC", true},
            {:rule_reference, "OUTPUT", true},
            {:rule_reference, "INPUT", true},
          ]},
          line_number: 32,
        },
      ],
      version: 1,
    }
  end
end
