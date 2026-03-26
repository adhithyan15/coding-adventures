# AUTO-GENERATED FILE - DO NOT EDIT
defmodule CssTokens do
  alias CodingAdventures.GrammarTools.TokenGrammar

  def grammar do
    %TokenGrammar{
      version: 1,
      case_insensitive: false,
      case_sensitive: true,
      mode: nil,
      escape_mode: "none",
      keywords: [],
      reserved_keywords: [],
      definitions: [
        %{name: "STRING_DQ", pattern: "\"([^\"\\\\\\n]|\\\\.)*\"", is_regex: true, line_number: 67, alias: "STRING"},
        %{name: "STRING_SQ", pattern: "'([^'\\\\\\n]|\\\\.)*'", is_regex: true, line_number: 68, alias: "STRING"},
        %{name: "DIMENSION", pattern: "-?[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?[a-zA-Z]+", is_regex: true, line_number: 94, alias: nil},
        %{name: "PERCENTAGE", pattern: "-?[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?%", is_regex: true, line_number: 95, alias: nil},
        %{name: "NUMBER", pattern: "-?[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?", is_regex: true, line_number: 96, alias: nil},
        %{name: "HASH", pattern: "#[a-zA-Z0-9_-]+", is_regex: true, line_number: 109, alias: nil},
        %{name: "AT_KEYWORD", pattern: "@-?[a-zA-Z][a-zA-Z0-9-]*", is_regex: true, line_number: 123, alias: nil},
        %{name: "URL_TOKEN", pattern: "url\\([^)'\"]*\\)", is_regex: true, line_number: 136, alias: nil},
        %{name: "FUNCTION", pattern: "-?[a-zA-Z_][a-zA-Z0-9_-]*\\(", is_regex: true, line_number: 149, alias: nil},
        %{name: "CDO", pattern: "<!--", is_regex: false, line_number: 162, alias: nil},
        %{name: "CDC", pattern: "-->", is_regex: false, line_number: 163, alias: nil},
        %{name: "UNICODE_RANGE", pattern: "[Uu]\\+[0-9a-fA-F?]{1,6}(-[0-9a-fA-F]{1,6})?", is_regex: true, line_number: 190, alias: nil},
        %{name: "CUSTOM_PROPERTY", pattern: "--[a-zA-Z_][a-zA-Z0-9_-]*", is_regex: true, line_number: 192, alias: nil},
        %{name: "IDENT", pattern: "-?[a-zA-Z_][a-zA-Z0-9_-]*", is_regex: true, line_number: 193, alias: nil},
        %{name: "COLON_COLON", pattern: "::", is_regex: false, line_number: 202, alias: nil},
        %{name: "TILDE_EQUALS", pattern: "~=", is_regex: false, line_number: 203, alias: nil},
        %{name: "PIPE_EQUALS", pattern: "|=", is_regex: false, line_number: 204, alias: nil},
        %{name: "CARET_EQUALS", pattern: "^=", is_regex: false, line_number: 205, alias: nil},
        %{name: "DOLLAR_EQUALS", pattern: "$=", is_regex: false, line_number: 206, alias: nil},
        %{name: "STAR_EQUALS", pattern: "*=", is_regex: false, line_number: 207, alias: nil},
        %{name: "LBRACE", pattern: "{", is_regex: false, line_number: 216, alias: nil},
        %{name: "RBRACE", pattern: "}", is_regex: false, line_number: 217, alias: nil},
        %{name: "LPAREN", pattern: "(", is_regex: false, line_number: 218, alias: nil},
        %{name: "RPAREN", pattern: ")", is_regex: false, line_number: 219, alias: nil},
        %{name: "LBRACKET", pattern: "[", is_regex: false, line_number: 220, alias: nil},
        %{name: "RBRACKET", pattern: "]", is_regex: false, line_number: 221, alias: nil},
        %{name: "SEMICOLON", pattern: ";", is_regex: false, line_number: 222, alias: nil},
        %{name: "COLON", pattern: ":", is_regex: false, line_number: 223, alias: nil},
        %{name: "COMMA", pattern: ",", is_regex: false, line_number: 224, alias: nil},
        %{name: "DOT", pattern: ".", is_regex: false, line_number: 225, alias: nil},
        %{name: "PLUS", pattern: "+", is_regex: false, line_number: 226, alias: nil},
        %{name: "GREATER", pattern: ">", is_regex: false, line_number: 227, alias: nil},
        %{name: "TILDE", pattern: "~", is_regex: false, line_number: 228, alias: nil},
        %{name: "STAR", pattern: "*", is_regex: false, line_number: 229, alias: nil},
        %{name: "PIPE", pattern: "|", is_regex: false, line_number: 230, alias: nil},
        %{name: "BANG", pattern: "!", is_regex: false, line_number: 231, alias: nil},
        %{name: "SLASH", pattern: "/", is_regex: false, line_number: 232, alias: nil},
        %{name: "EQUALS", pattern: "=", is_regex: false, line_number: 233, alias: nil},
        %{name: "AMPERSAND", pattern: "&", is_regex: false, line_number: 234, alias: nil},
        %{name: "MINUS", pattern: "-", is_regex: false, line_number: 235, alias: nil}
      ],
      skip_definitions: [
        %{name: "COMMENT", pattern: "\\/\\*[\\s\\S]*?\\*\\/", is_regex: true, line_number: 51, alias: nil},
        %{name: "WHITESPACE", pattern: "[ \\t\\r\\n]+", is_regex: true, line_number: 52, alias: nil}
      ],
      error_definitions: [
        %{name: "BAD_STRING", pattern: "\"[^\"]*$", is_regex: true, line_number: 251, alias: nil},
        %{name: "BAD_URL", pattern: "url\\([^)]*$", is_regex: true, line_number: 252, alias: nil}
      ],
      groups: %{

      }
    }
  end
end
