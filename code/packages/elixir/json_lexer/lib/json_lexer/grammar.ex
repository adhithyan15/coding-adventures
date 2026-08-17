defmodule CodingAdventures.JsonLexer.Grammar do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: json.tokens
  # Regenerate with: grammar-tools compile-tokens json.tokens
  #
  # This file embeds a TokenGrammar as native Elixir data structures.
  # Call token_grammar/0 instead of reading and parsing the .tokens file.
  
  alias CodingAdventures.GrammarTools.TokenGrammar
  
  def token_grammar do
    %TokenGrammar{
      definitions: [
          %{
            name: "STRING",
            pattern: "\"([^\"\\\\]|\\\\[\"\\\\\\x2fbfnrt]|\\\\u[0-9a-fA-F]{4})*\"",
            is_regex: true,
            line_number: 30,
            alias: nil,
          },
          %{
            name: "NUMBER",
            pattern: "-?[0-9]+\\.?[0-9]*[eE]?[-+]?[0-9]*",
            is_regex: true,
            line_number: 37,
            alias: nil,
          },
          %{
            name: "TRUE",
            pattern: "true",
            is_regex: false,
            line_number: 41,
            alias: nil,
          },
          %{
            name: "FALSE",
            pattern: "false",
            is_regex: false,
            line_number: 42,
            alias: nil,
          },
          %{
            name: "NULL",
            pattern: "null",
            is_regex: false,
            line_number: 43,
            alias: nil,
          },
          %{
            name: "LBRACE",
            pattern: "{",
            is_regex: false,
            line_number: 49,
            alias: nil,
          },
          %{
            name: "RBRACE",
            pattern: "}",
            is_regex: false,
            line_number: 50,
            alias: nil,
          },
          %{
            name: "LBRACKET",
            pattern: "[",
            is_regex: false,
            line_number: 51,
            alias: nil,
          },
          %{
            name: "RBRACKET",
            pattern: "]",
            is_regex: false,
            line_number: 52,
            alias: nil,
          },
          %{
            name: "COLON",
            pattern: ":",
            is_regex: false,
            line_number: 53,
            alias: nil,
          },
          %{
            name: "COMMA",
            pattern: ",",
            is_regex: false,
            line_number: 54,
            alias: nil,
          },
        ],
      keywords: [],
      mode: nil,
      escape_mode: "none",
      skip_definitions: [
          %{
            name: "WHITESPACE",
            pattern: "[ \\t\\r\\n]+",
            is_regex: true,
            line_number: 65,
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
