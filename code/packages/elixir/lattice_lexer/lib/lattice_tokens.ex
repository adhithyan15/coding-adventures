# AUTO-GENERATED FILE - DO NOT EDIT
defmodule LatticeTokens do
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
        %{name: "STRING_DQ", pattern: "\"([^\"\\\\\\n]|\\\\.)*\"", is_regex: true, line_number: 66, alias: "STRING"},
        %{name: "STRING_SQ", pattern: "'([^'\\\\\\n]|\\\\.)*'", is_regex: true, line_number: 67, alias: "STRING"},
        %{name: "VARIABLE", pattern: "\\$[a-zA-Z_][a-zA-Z0-9_-]*", is_regex: true, line_number: 83, alias: nil},
        %{name: "PLACEHOLDER", pattern: "%[a-zA-Z_][a-zA-Z0-9_-]*", is_regex: true, line_number: 93, alias: nil},
        %{name: "DIMENSION", pattern: "-?[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?[a-zA-Z]+", is_regex: true, line_number: 102, alias: nil},
        %{name: "PERCENTAGE", pattern: "-?[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?%", is_regex: true, line_number: 103, alias: nil},
        %{name: "NUMBER", pattern: "-?[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?", is_regex: true, line_number: 104, alias: nil},
        %{name: "HASH", pattern: "#[a-zA-Z0-9_-]+", is_regex: true, line_number: 110, alias: nil},
        %{name: "AT_KEYWORD", pattern: "@-?[a-zA-Z][a-zA-Z0-9-]*", is_regex: true, line_number: 127, alias: nil},
        %{name: "URL_TOKEN", pattern: "url\\([^)'\"]*\\)", is_regex: true, line_number: 133, alias: nil},
        %{name: "FUNCTION", pattern: "-?[a-zA-Z_][a-zA-Z0-9_-]*\\(", is_regex: true, line_number: 139, alias: nil},
        %{name: "CDO", pattern: "<!--", is_regex: false, line_number: 145, alias: nil},
        %{name: "CDC", pattern: "-->", is_regex: false, line_number: 146, alias: nil},
        %{name: "UNICODE_RANGE", pattern: "[Uu]\\+[0-9a-fA-F?]{1,6}(-[0-9a-fA-F]{1,6})?", is_regex: true, line_number: 152, alias: nil},
        %{name: "CUSTOM_PROPERTY", pattern: "--[a-zA-Z_][a-zA-Z0-9_-]*", is_regex: true, line_number: 153, alias: nil},
        %{name: "IDENT", pattern: "-?[a-zA-Z_][a-zA-Z0-9_-]*", is_regex: true, line_number: 154, alias: nil},
        %{name: "COLON_COLON", pattern: "::", is_regex: false, line_number: 163, alias: nil},
        %{name: "TILDE_EQUALS", pattern: "~=", is_regex: false, line_number: 164, alias: nil},
        %{name: "PIPE_EQUALS", pattern: "|=", is_regex: false, line_number: 165, alias: nil},
        %{name: "CARET_EQUALS", pattern: "^=", is_regex: false, line_number: 166, alias: nil},
        %{name: "DOLLAR_EQUALS", pattern: "$=", is_regex: false, line_number: 167, alias: nil},
        %{name: "STAR_EQUALS", pattern: "*=", is_regex: false, line_number: 168, alias: nil},
        %{name: "EQUALS_EQUALS", pattern: "==", is_regex: false, line_number: 171, alias: nil},
        %{name: "NOT_EQUALS", pattern: "!=", is_regex: false, line_number: 172, alias: nil},
        %{name: "GREATER_EQUALS", pattern: ">=", is_regex: false, line_number: 173, alias: nil},
        %{name: "LESS_EQUALS", pattern: "<=", is_regex: false, line_number: 174, alias: nil},
        %{name: "LBRACE", pattern: "{", is_regex: false, line_number: 180, alias: nil},
        %{name: "RBRACE", pattern: "}", is_regex: false, line_number: 181, alias: nil},
        %{name: "LPAREN", pattern: "(", is_regex: false, line_number: 182, alias: nil},
        %{name: "RPAREN", pattern: ")", is_regex: false, line_number: 183, alias: nil},
        %{name: "LBRACKET", pattern: "[", is_regex: false, line_number: 184, alias: nil},
        %{name: "RBRACKET", pattern: "]", is_regex: false, line_number: 185, alias: nil},
        %{name: "SEMICOLON", pattern: ";", is_regex: false, line_number: 186, alias: nil},
        %{name: "COLON", pattern: ":", is_regex: false, line_number: 187, alias: nil},
        %{name: "COMMA", pattern: ",", is_regex: false, line_number: 188, alias: nil},
        %{name: "DOT", pattern: ".", is_regex: false, line_number: 189, alias: nil},
        %{name: "PLUS", pattern: "+", is_regex: false, line_number: 190, alias: nil},
        %{name: "GREATER", pattern: ">", is_regex: false, line_number: 191, alias: nil},
        %{name: "LESS", pattern: "<", is_regex: false, line_number: 192, alias: nil},
        %{name: "TILDE", pattern: "~", is_regex: false, line_number: 193, alias: nil},
        %{name: "STAR", pattern: "*", is_regex: false, line_number: 194, alias: nil},
        %{name: "PIPE", pattern: "|", is_regex: false, line_number: 195, alias: nil},
        %{name: "BANG_DEFAULT", pattern: "!default", is_regex: false, line_number: 198, alias: nil},
        %{name: "BANG_GLOBAL", pattern: "!global", is_regex: false, line_number: 199, alias: nil},
        %{name: "BANG", pattern: "!", is_regex: false, line_number: 200, alias: nil},
        %{name: "SLASH", pattern: "/", is_regex: false, line_number: 201, alias: nil},
        %{name: "EQUALS", pattern: "=", is_regex: false, line_number: 202, alias: nil},
        %{name: "AMPERSAND", pattern: "&", is_regex: false, line_number: 203, alias: nil},
        %{name: "MINUS", pattern: "-", is_regex: false, line_number: 204, alias: nil}
      ],
      skip_definitions: [
        %{name: "LINE_COMMENT", pattern: "\\/\\/[^\\n]*", is_regex: true, line_number: 55, alias: nil},
        %{name: "COMMENT", pattern: "\\/\\*[\\s\\S]*?\\*\\/", is_regex: true, line_number: 56, alias: nil},
        %{name: "WHITESPACE", pattern: "[ \\t\\r\\n]+", is_regex: true, line_number: 57, alias: nil}
      ],
      error_definitions: [

      ],
      groups: %{

      }
    }
  end
end
