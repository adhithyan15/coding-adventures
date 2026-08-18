defmodule CodingAdventures.MosaicLexer.Grammar do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: mosaic.tokens
  # Regenerate with: grammar-tools compile-tokens mosaic.tokens
  #
  # This file embeds a TokenGrammar as native Elixir data structures.
  # Call token_grammar/0 instead of reading and parsing the .tokens file.

  alias CodingAdventures.GrammarTools.TokenGrammar

  def token_grammar do
    %TokenGrammar{
      definitions: [
          %{
            name: "STRING",
            pattern: "\"([^\"\\\\\\n]|\\\\.)*\"",
            is_regex: true,
            line_number: 23,
            alias: nil,
          },
          %{
            name: "DIMENSION",
            pattern: "-?[0-9]*\\.?[0-9]+[a-zA-Z%]+",
            is_regex: true,
            line_number: 31,
            alias: nil,
          },
          %{
            name: "NUMBER",
            pattern: "-?[0-9]*\\.?[0-9]+",
            is_regex: true,
            line_number: 32,
            alias: nil,
          },
          %{
            name: "COLOR_HEX",
            pattern: "#[0-9a-fA-F]{3,8}",
            is_regex: true,
            line_number: 39,
            alias: nil,
          },
          %{
            name: "NAME",
            pattern: "[a-zA-Z_][a-zA-Z0-9_-]*",
            is_regex: true,
            line_number: 70,
            alias: nil,
          },
          %{
            name: "LBRACE",
            pattern: "{",
            is_regex: false,
            line_number: 76,
            alias: nil,
          },
          %{
            name: "RBRACE",
            pattern: "}",
            is_regex: false,
            line_number: 77,
            alias: nil,
          },
          %{
            name: "LANGLE",
            pattern: "<",
            is_regex: false,
            line_number: 78,
            alias: nil,
          },
          %{
            name: "RANGLE",
            pattern: ">",
            is_regex: false,
            line_number: 79,
            alias: nil,
          },
          %{
            name: "COLON",
            pattern: ":",
            is_regex: false,
            line_number: 80,
            alias: nil,
          },
          %{
            name: "SEMICOLON",
            pattern: ";",
            is_regex: false,
            line_number: 81,
            alias: nil,
          },
          %{
            name: "COMMA",
            pattern: ",",
            is_regex: false,
            line_number: 82,
            alias: nil,
          },
          %{
            name: "DOT",
            pattern: ".",
            is_regex: false,
            line_number: 83,
            alias: nil,
          },
          %{
            name: "EQUALS",
            pattern: "=",
            is_regex: false,
            line_number: 84,
            alias: nil,
          },
          %{
            name: "AT",
            pattern: "@",
            is_regex: false,
            line_number: 85,
            alias: nil,
          },
        ],
      keywords: ["component", "slot", "import", "from", "as", "text", "number", "bool", "image", "color", "node", "list", "true", "false", "when", "each"],
      mode: nil,
      escape_mode: "standard",
      skip_definitions: [
          %{
            name: "LINE_COMMENT",
            pattern: "\\/\\/[^\\n]*",
            is_regex: true,
            line_number: 15,
            alias: nil,
          },
          %{
            name: "BLOCK_COMMENT",
            pattern: "\\/\\*[\\s\\S]*?\\*\\/",
            is_regex: true,
            line_number: 16,
            alias: nil,
          },
          %{
            name: "WHITESPACE",
            pattern: "[ \\t\\r\\n]+",
            is_regex: true,
            line_number: 17,
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
