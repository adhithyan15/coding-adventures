# AUTO-GENERATED FILE - DO NOT EDIT
defmodule JsonTokens do
  alias CodingAdventures.GrammarTools.TokenGrammar

  def grammar do
    %TokenGrammar{
      version: 1,
      case_insensitive: false,
      case_sensitive: true,
      mode: nil,
      escape_mode: nil,
      keywords: [],
      reserved_keywords: [],
      definitions: [
        %{name: "STRING", pattern: "\"([^\"\\\\]|\\\\[\"\\\\\\x2fbfnrt]|\\\\u[0-9a-fA-F]{4})*\"", is_regex: true, line_number: 25, alias: nil},
        %{name: "NUMBER", pattern: "-?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?", is_regex: true, line_number: 31, alias: nil},
        %{name: "TRUE", pattern: "true", is_regex: false, line_number: 35, alias: nil},
        %{name: "FALSE", pattern: "false", is_regex: false, line_number: 36, alias: nil},
        %{name: "NULL", pattern: "null", is_regex: false, line_number: 37, alias: nil},
        %{name: "LBRACE", pattern: "{", is_regex: false, line_number: 43, alias: nil},
        %{name: "RBRACE", pattern: "}", is_regex: false, line_number: 44, alias: nil},
        %{name: "LBRACKET", pattern: "[", is_regex: false, line_number: 45, alias: nil},
        %{name: "RBRACKET", pattern: "]", is_regex: false, line_number: 46, alias: nil},
        %{name: "COLON", pattern: ":", is_regex: false, line_number: 47, alias: nil},
        %{name: "COMMA", pattern: ",", is_regex: false, line_number: 48, alias: nil}
      ],
      skip_definitions: [
        %{name: "WHITESPACE", pattern: "[ \\t\\r\\n]+", is_regex: true, line_number: 59, alias: nil}
      ],
      error_definitions: [

      ],
      groups: %{

      }
    }
  end
end
