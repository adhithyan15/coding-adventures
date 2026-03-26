# AUTO-GENERATED FILE - DO NOT EDIT
defmodule StarlarkTokens do
  alias CodingAdventures.GrammarTools.TokenGrammar

  def grammar do
    %TokenGrammar{
      version: 1,
      case_insensitive: false,
      case_sensitive: true,
      mode: "indentation",
      escape_mode: nil,
      keywords: ["and", "break", "continue", "def", "elif", "else", "for", "if", "in", "lambda", "load", "not", "or", "pass", "return", "True", "False", "None"],
      reserved_keywords: ["as", "assert", "async", "await", "class", "del", "except", "finally", "from", "global", "import", "is", "nonlocal", "raise", "try", "while", "with", "yield"],
      definitions: [
        %{name: "STRING_RAW_TRIPLE_DQ", pattern: "[rR][bB]?\"\"\"([^\"\\\\]|\\\\.|\\n)*\"\"\"|[bB][rR]\"\"\"([^\"\\\\]|\\\\.|\\n)*\"\"\"", is_regex: true, line_number: 70, alias: "STRING"},
        %{name: "STRING_RAW_TRIPLE_SQ", pattern: "[rR][bB]?'''([^'\\\\]|\\\\.|\\n)*'''|[bB][rR]'''([^'\\\\]|\\\\.|\\n)*'''", is_regex: true, line_number: 71, alias: "STRING"},
        %{name: "STRING_TRIPLE_DQ", pattern: "[bB]?\"\"\"([^\"\\\\]|\\\\.|\\n)*\"\"\"", is_regex: true, line_number: 72, alias: "STRING"},
        %{name: "STRING_TRIPLE_SQ", pattern: "[bB]?'''([^'\\\\]|\\\\.|\\n)*'''", is_regex: true, line_number: 73, alias: "STRING"},
        %{name: "STRING_RAW_DQ", pattern: "[rR][bB]?\"([^\"\\\\]|\\\\.)*\"|[bB][rR]\"([^\"\\\\]|\\\\.)*\"", is_regex: true, line_number: 76, alias: "STRING"},
        %{name: "STRING_RAW_SQ", pattern: "[rR][bB]?'([^'\\\\]|\\\\.)*'|[bB][rR]'([^'\\\\]|\\\\.)*'", is_regex: true, line_number: 77, alias: "STRING"},
        %{name: "STRING_DQ", pattern: "[bB]?\"([^\"\\\\]|\\\\.)*\"", is_regex: true, line_number: 78, alias: "STRING"},
        %{name: "STRING_SQ", pattern: "[bB]?'([^'\\\\]|\\\\.)*'", is_regex: true, line_number: 79, alias: "STRING"},
        %{name: "FLOAT", pattern: "[0-9]+\\.[0-9]*([eE][+-]?[0-9]+)?|\\.[0-9]+([eE][+-]?[0-9]+)?|[0-9]+[eE][+-]?[0-9]+", is_regex: true, line_number: 92, alias: nil},
        %{name: "INT_HEX", pattern: "0[xX][0-9a-fA-F]+", is_regex: true, line_number: 95, alias: "INT"},
        %{name: "INT_OCT", pattern: "0[oO][0-7]+", is_regex: true, line_number: 96, alias: "INT"},
        %{name: "INT", pattern: "[0-9]+", is_regex: true, line_number: 97, alias: nil},
        %{name: "NAME", pattern: "[a-zA-Z_][a-zA-Z0-9_]*", is_regex: true, line_number: 107, alias: nil},
        %{name: "DOUBLE_STAR_EQUALS", pattern: "**=", is_regex: false, line_number: 116, alias: nil},
        %{name: "LEFT_SHIFT_EQUALS", pattern: "<<=", is_regex: false, line_number: 117, alias: nil},
        %{name: "RIGHT_SHIFT_EQUALS", pattern: ">>=", is_regex: false, line_number: 118, alias: nil},
        %{name: "FLOOR_DIV_EQUALS", pattern: "//=", is_regex: false, line_number: 119, alias: nil},
        %{name: "DOUBLE_STAR", pattern: "**", is_regex: false, line_number: 128, alias: nil},
        %{name: "FLOOR_DIV", pattern: "//", is_regex: false, line_number: 129, alias: nil},
        %{name: "LEFT_SHIFT", pattern: "<<", is_regex: false, line_number: 130, alias: nil},
        %{name: "RIGHT_SHIFT", pattern: ">>", is_regex: false, line_number: 131, alias: nil},
        %{name: "EQUALS_EQUALS", pattern: "==", is_regex: false, line_number: 132, alias: nil},
        %{name: "NOT_EQUALS", pattern: "!=", is_regex: false, line_number: 133, alias: nil},
        %{name: "LESS_EQUALS", pattern: "<=", is_regex: false, line_number: 134, alias: nil},
        %{name: "GREATER_EQUALS", pattern: ">=", is_regex: false, line_number: 135, alias: nil},
        %{name: "PLUS_EQUALS", pattern: "+=", is_regex: false, line_number: 136, alias: nil},
        %{name: "MINUS_EQUALS", pattern: "-=", is_regex: false, line_number: 137, alias: nil},
        %{name: "STAR_EQUALS", pattern: "*=", is_regex: false, line_number: 138, alias: nil},
        %{name: "SLASH_EQUALS", pattern: "/=", is_regex: false, line_number: 139, alias: nil},
        %{name: "PERCENT_EQUALS", pattern: "%=", is_regex: false, line_number: 140, alias: nil},
        %{name: "AMP_EQUALS", pattern: "&=", is_regex: false, line_number: 141, alias: nil},
        %{name: "PIPE_EQUALS", pattern: "|=", is_regex: false, line_number: 142, alias: nil},
        %{name: "CARET_EQUALS", pattern: "^=", is_regex: false, line_number: 143, alias: nil},
        %{name: "PLUS", pattern: "+", is_regex: false, line_number: 149, alias: nil},
        %{name: "MINUS", pattern: "-", is_regex: false, line_number: 150, alias: nil},
        %{name: "STAR", pattern: "*", is_regex: false, line_number: 151, alias: nil},
        %{name: "SLASH", pattern: "/", is_regex: false, line_number: 152, alias: nil},
        %{name: "PERCENT", pattern: "%", is_regex: false, line_number: 153, alias: nil},
        %{name: "EQUALS", pattern: "=", is_regex: false, line_number: 154, alias: nil},
        %{name: "LESS_THAN", pattern: "<", is_regex: false, line_number: 155, alias: nil},
        %{name: "GREATER_THAN", pattern: ">", is_regex: false, line_number: 156, alias: nil},
        %{name: "AMP", pattern: "&", is_regex: false, line_number: 157, alias: nil},
        %{name: "PIPE", pattern: "|", is_regex: false, line_number: 158, alias: nil},
        %{name: "CARET", pattern: "^", is_regex: false, line_number: 159, alias: nil},
        %{name: "TILDE", pattern: "~", is_regex: false, line_number: 160, alias: nil},
        %{name: "LPAREN", pattern: "(", is_regex: false, line_number: 166, alias: nil},
        %{name: "RPAREN", pattern: ")", is_regex: false, line_number: 167, alias: nil},
        %{name: "LBRACKET", pattern: "[", is_regex: false, line_number: 168, alias: nil},
        %{name: "RBRACKET", pattern: "]", is_regex: false, line_number: 169, alias: nil},
        %{name: "LBRACE", pattern: "{", is_regex: false, line_number: 170, alias: nil},
        %{name: "RBRACE", pattern: "}", is_regex: false, line_number: 171, alias: nil},
        %{name: "COMMA", pattern: ",", is_regex: false, line_number: 172, alias: nil},
        %{name: "COLON", pattern: ":", is_regex: false, line_number: 173, alias: nil},
        %{name: "SEMICOLON", pattern: ";", is_regex: false, line_number: 174, alias: nil},
        %{name: "DOT", pattern: ".", is_regex: false, line_number: 175, alias: nil}
      ],
      skip_definitions: [
        %{name: "COMMENT", pattern: "#[^\\n]*", is_regex: true, line_number: 51, alias: nil},
        %{name: "WHITESPACE", pattern: "[ \\t]+", is_regex: true, line_number: 52, alias: nil}
      ],
      error_definitions: [

      ],
      groups: %{

      }
    }
  end
end
