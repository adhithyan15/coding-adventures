defmodule CodingAdventures.AlgolLexer.Grammar.Algol60 do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: algol60.tokens
  # Regenerate with: grammar-tools compile-tokens algol60.tokens
  #
  # This file embeds a TokenGrammar as native Elixir data structures.
  # Call token_grammar/0 instead of reading and parsing the .tokens file.

  alias CodingAdventures.GrammarTools.TokenGrammar

  def token_grammar do
    %TokenGrammar{
      definitions: [
          %{
            name: "REAL_LIT",
            pattern: "[0-9]+\\.[0-9]*([eE][+-]?[0-9]+)?|[0-9]+[eE][+-]?[0-9]+",
            is_regex: true,
            line_number: 38,
            alias: nil,
          },
          %{
            name: "INTEGER_LIT",
            pattern: "[0-9]+",
            is_regex: true,
            line_number: 41,
            alias: nil,
          },
          %{
            name: "STRING_LIT",
            pattern: "'[^']*'|\"[^\"]*\"",
            is_regex: true,
            line_number: 46,
            alias: nil,
          },
          %{
            name: "NAME",
            pattern: "[a-zA-Z][a-zA-Z0-9]*",
            is_regex: true,
            line_number: 53,
            alias: nil,
          },
          %{
            name: "ASSIGN",
            pattern: ":=",
            is_regex: false,
            line_number: 61,
            alias: nil,
          },
          %{
            name: "POWER",
            pattern: "**",
            is_regex: false,
            line_number: 66,
            alias: nil,
          },
          %{
            name: "LEQ",
            pattern: "<=|≤",
            is_regex: true,
            line_number: 70,
            alias: nil,
          },
          %{
            name: "GEQ",
            pattern: ">=|≥",
            is_regex: true,
            line_number: 71,
            alias: nil,
          },
          %{
            name: "NEQ",
            pattern: "!=|<>|≠",
            is_regex: true,
            line_number: 72,
            alias: nil,
          },
          %{
            name: "NOT_SYM",
            pattern: "¬",
            is_regex: false,
            line_number: 76,
            alias: nil,
          },
          %{
            name: "AND_SYM",
            pattern: "∧",
            is_regex: false,
            line_number: 77,
            alias: nil,
          },
          %{
            name: "OR_SYM",
            pattern: "∨",
            is_regex: false,
            line_number: 78,
            alias: nil,
          },
          %{
            name: "IMPL_SYM",
            pattern: "⊃",
            is_regex: false,
            line_number: 79,
            alias: nil,
          },
          %{
            name: "EQV_SYM",
            pattern: "≡",
            is_regex: false,
            line_number: 80,
            alias: nil,
          },
          %{
            name: "PLUS",
            pattern: "+",
            is_regex: false,
            line_number: 86,
            alias: nil,
          },
          %{
            name: "MINUS",
            pattern: "-",
            is_regex: false,
            line_number: 87,
            alias: nil,
          },
          %{
            name: "STAR",
            pattern: "\\*|×",
            is_regex: true,
            line_number: 90,
            alias: nil,
          },
          %{
            name: "SLASH",
            pattern: "\\/|÷",
            is_regex: true,
            line_number: 91,
            alias: nil,
          },
          %{
            name: "CARET",
            pattern: "\\^|↑",
            is_regex: true,
            line_number: 95,
            alias: nil,
          },
          %{
            name: "EQ",
            pattern: "=",
            is_regex: false,
            line_number: 98,
            alias: nil,
          },
          %{
            name: "LT",
            pattern: "<",
            is_regex: false,
            line_number: 100,
            alias: nil,
          },
          %{
            name: "GT",
            pattern: ">",
            is_regex: false,
            line_number: 101,
            alias: nil,
          },
          %{
            name: "LPAREN",
            pattern: "(",
            is_regex: false,
            line_number: 107,
            alias: nil,
          },
          %{
            name: "RPAREN",
            pattern: ")",
            is_regex: false,
            line_number: 108,
            alias: nil,
          },
          %{
            name: "LBRACKET",
            pattern: "[",
            is_regex: false,
            line_number: 109,
            alias: nil,
          },
          %{
            name: "RBRACKET",
            pattern: "]",
            is_regex: false,
            line_number: 110,
            alias: nil,
          },
          %{
            name: "SEMICOLON",
            pattern: ";",
            is_regex: false,
            line_number: 111,
            alias: nil,
          },
          %{
            name: "COMMA",
            pattern: ",",
            is_regex: false,
            line_number: 112,
            alias: nil,
          },
          %{
            name: "COLON",
            pattern: ":",
            is_regex: false,
            line_number: 116,
            alias: nil,
          },
        ],
      keywords: ["begin", "end", "if", "then", "else", "for", "do", "step", "until", "while", "goto", "switch", "procedure", "own", "array", "label", "value", "integer", "real", "boolean", "string", "true", "false", "not", "and", "or", "impl", "eqv", "div", "mod", "comment"],
      mode: nil,
      escape_mode: nil,
      skip_definitions: [
          %{
            name: "WHITESPACE",
            pattern: "[ \\t\\r\\n]+",
            is_regex: true,
            line_number: 183,
            alias: nil,
          },
          %{
            name: "COMMENT",
            pattern: "comment[^a-zA-Z0-9_][^;]*;",
            is_regex: true,
            line_number: 192,
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
