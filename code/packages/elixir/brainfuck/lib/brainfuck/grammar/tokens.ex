defmodule CodingAdventures.Brainfuck.Grammar.Tokens do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: brainfuck.tokens
  # Regenerate with: grammar-tools compile-tokens brainfuck.tokens
  #
  # This file embeds a TokenGrammar as native Elixir data structures.
  # Call token_grammar/0 instead of reading and parsing the .tokens file.
  
  alias CodingAdventures.GrammarTools.TokenGrammar
  
  def token_grammar do
    %TokenGrammar{
      definitions: [
          %{
            name: "RIGHT",
            pattern: ">",
            is_regex: false,
            line_number: 23,
            alias: nil,
          },
          %{
            name: "LEFT",
            pattern: "<",
            is_regex: false,
            line_number: 24,
            alias: nil,
          },
          %{
            name: "INC",
            pattern: "+",
            is_regex: false,
            line_number: 29,
            alias: nil,
          },
          %{
            name: "DEC",
            pattern: "-",
            is_regex: false,
            line_number: 30,
            alias: nil,
          },
          %{
            name: "OUTPUT",
            pattern: ".",
            is_regex: false,
            line_number: 35,
            alias: nil,
          },
          %{
            name: "INPUT",
            pattern: ",",
            is_regex: false,
            line_number: 36,
            alias: nil,
          },
          %{
            name: "LOOP_START",
            pattern: "[",
            is_regex: false,
            line_number: 41,
            alias: nil,
          },
          %{
            name: "LOOP_END",
            pattern: "]",
            is_regex: false,
            line_number: 42,
            alias: nil,
          },
        ],
      keywords: [],
      mode: nil,
      escape_mode: nil,
      skip_definitions: [
          %{
            name: "WHITESPACE",
            pattern: "[ \\t\\r\\n]+",
            is_regex: true,
            line_number: 65,
            alias: nil,
          },
          %{
            name: "COMMENT",
            pattern: "[^><+\\-.,\\[\\] \\t\\r\\n]+",
            is_regex: true,
            line_number: 66,
            alias: nil,
          },
        ],
      reserved_keywords: [],
      error_definitions: [],
      groups: %{},
      layout_keywords: [],
      case_sensitive: true,
      version: 1,
      case_insensitive: false,
      start_mode: nil,
      transitions: [],
    }
  end
end
