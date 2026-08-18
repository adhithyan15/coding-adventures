defmodule CodingAdventures.DartmouthBasicLexer.Grammar do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: dartmouth_basic.tokens
  # Regenerate with: grammar-tools compile-tokens dartmouth_basic.tokens
  #
  # This file embeds a TokenGrammar as native Elixir data structures.
  # Call token_grammar/0 instead of reading and parsing the .tokens file.
  
  alias CodingAdventures.GrammarTools.TokenGrammar
  
  def token_grammar do
    %TokenGrammar{
      definitions: [
          %{
            name: "LE",
            pattern: "<=",
            is_regex: false,
            line_number: 50,
            alias: nil,
          },
          %{
            name: "GE",
            pattern: ">=",
            is_regex: false,
            line_number: 51,
            alias: nil,
          },
          %{
            name: "NE",
            pattern: "<>",
            is_regex: false,
            line_number: 52,
            alias: nil,
          },
          %{
            name: "NUMBER",
            pattern: "[0-9]*\\.?[0-9]+([Ee][+-]?[0-9]+)?",
            is_regex: true,
            line_number: 85,
            alias: nil,
          },
          %{
            name: "LINE_NUM",
            pattern: "[0-9]+",
            is_regex: true,
            line_number: 86,
            alias: nil,
          },
          %{
            name: "STRING_BODY",
            pattern: "\"[^\"]*\"",
            is_regex: true,
            line_number: 112,
            alias: "STRING",
          },
          %{
            name: "BUILTIN_FN",
            pattern: "(?:sin|cos|tan|atn|exp|log|abs|sqr|int|rnd|sgn)",
            is_regex: true,
            line_number: 168,
            alias: nil,
          },
          %{
            name: "USER_FN",
            pattern: "fn[a-z]",
            is_regex: true,
            line_number: 169,
            alias: nil,
          },
          %{
            name: "NAME",
            pattern: "[a-z][a-z0-9]*\\$?",
            is_regex: true,
            line_number: 204,
            alias: nil,
          },
          %{
            name: "PLUS",
            pattern: "+",
            is_regex: false,
            line_number: 244,
            alias: nil,
          },
          %{
            name: "MINUS",
            pattern: "-",
            is_regex: false,
            line_number: 245,
            alias: nil,
          },
          %{
            name: "STAR",
            pattern: "*",
            is_regex: false,
            line_number: 246,
            alias: nil,
          },
          %{
            name: "SLASH",
            pattern: "/",
            is_regex: false,
            line_number: 247,
            alias: nil,
          },
          %{
            name: "CARET",
            pattern: "^",
            is_regex: false,
            line_number: 248,
            alias: nil,
          },
          %{
            name: "EQ",
            pattern: "=",
            is_regex: false,
            line_number: 249,
            alias: nil,
          },
          %{
            name: "LT",
            pattern: "<",
            is_regex: false,
            line_number: 250,
            alias: nil,
          },
          %{
            name: "GT",
            pattern: ">",
            is_regex: false,
            line_number: 251,
            alias: nil,
          },
          %{
            name: "LPAREN",
            pattern: "(",
            is_regex: false,
            line_number: 252,
            alias: nil,
          },
          %{
            name: "RPAREN",
            pattern: ")",
            is_regex: false,
            line_number: 253,
            alias: nil,
          },
          %{
            name: "COMMA",
            pattern: ",",
            is_regex: false,
            line_number: 254,
            alias: nil,
          },
          %{
            name: "SEMICOLON",
            pattern: ";",
            is_regex: false,
            line_number: 255,
            alias: nil,
          },
          %{
            name: "NEWLINE",
            pattern: "\\r?\\n",
            is_regex: true,
            line_number: 276,
            alias: nil,
          },
        ],
      keywords: ["LET", "PRINT", "INPUT", "IF", "THEN", "GOTO", "GOSUB", "RETURN", "FOR", "TO", "STEP", "NEXT", "END", "STOP", "REM", "READ", "DATA", "RESTORE", "DIM", "DEF"],
      mode: nil,
      escape_mode: nil,
      skip_definitions: [
          %{
            name: "WHITESPACE",
            pattern: "[ \\t]+",
            is_regex: true,
            line_number: 288,
            alias: nil,
          },
        ],
      reserved_keywords: [],
      error_definitions: [
          %{
            name: "UNKNOWN",
            pattern: ".",
            is_regex: true,
            line_number: 304,
            alias: nil,
          },
        ],
      groups: %{},
      layout_keywords: [],
      case_sensitive: false,
      version: 1,
      case_insensitive: true,
      start_mode: nil,
      transitions: [],
    }
  end
end
