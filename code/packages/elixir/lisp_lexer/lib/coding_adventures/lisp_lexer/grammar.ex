defmodule CodingAdventures.LispLexer.Grammar do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: lisp.tokens
  # Regenerate with: grammar-tools compile-tokens lisp.tokens
  #
  # This file embeds a TokenGrammar as native Elixir data structures.
  # Call token_grammar/0 instead of reading and parsing the .tokens file.

  alias CodingAdventures.GrammarTools.TokenGrammar

  def token_grammar do
    %TokenGrammar{
      definitions: [
          %{
            name: "NUMBER",
            pattern: "-?[0-9]+",
            is_regex: true,
            line_number: 11,
            alias: nil,
          },
          %{
            name: "SYMBOL",
            pattern: "[a-zA-Z_+\\-*\\/=<>!?&][a-zA-Z0-9_+\\-*\\/=<>!?&]*",
            is_regex: true,
            line_number: 12,
            alias: nil,
          },
          %{
            name: "STRING",
            pattern: "\"([^\"\\\\]|\\\\.)*\"",
            is_regex: true,
            line_number: 13,
            alias: nil,
          },
          %{
            name: "LPAREN",
            pattern: "(",
            is_regex: false,
            line_number: 14,
            alias: nil,
          },
          %{
            name: "RPAREN",
            pattern: ")",
            is_regex: false,
            line_number: 15,
            alias: nil,
          },
          %{
            name: "QUOTE",
            pattern: "'",
            is_regex: false,
            line_number: 16,
            alias: nil,
          },
          %{
            name: "DOT",
            pattern: ".",
            is_regex: false,
            line_number: 17,
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
            line_number: 8,
            alias: nil,
          },
          %{
            name: "COMMENT",
            pattern: ";[^\\n]*",
            is_regex: true,
            line_number: 9,
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
