# AUTO-GENERATED FILE - DO NOT EDIT
defmodule RubyTokens do
  alias CodingAdventures.GrammarTools.TokenGrammar

  def grammar do
    %TokenGrammar{
      version: 1,
      case_insensitive: false,
      case_sensitive: true,
      mode: nil,
      escape_mode: nil,
      keywords: ["if", "else", "elsif", "end", "while", "for", "do", "def", "return", "class", "module", "require", "puts", "true", "false", "nil", "and", "or", "not", "then", "unless", "until", "yield", "begin", "rescue", "ensure"],
      reserved_keywords: [],
      definitions: [
        %{name: "NAME", pattern: "[a-zA-Z_][a-zA-Z0-9_]*", is_regex: true, line_number: 23, alias: nil},
        %{name: "NUMBER", pattern: "[0-9]+", is_regex: true, line_number: 24, alias: nil},
        %{name: "STRING", pattern: "\"([^\"\\\\]|\\\\.)*\"", is_regex: true, line_number: 25, alias: nil},
        %{name: "EQUALS_EQUALS", pattern: "==", is_regex: false, line_number: 28, alias: nil},
        %{name: "DOT_DOT", pattern: "..", is_regex: false, line_number: 29, alias: nil},
        %{name: "HASH_ROCKET", pattern: "=>", is_regex: false, line_number: 30, alias: nil},
        %{name: "NOT_EQUALS", pattern: "!=", is_regex: false, line_number: 31, alias: nil},
        %{name: "LESS_EQUALS", pattern: "<=", is_regex: false, line_number: 32, alias: nil},
        %{name: "GREATER_EQUALS", pattern: ">=", is_regex: false, line_number: 33, alias: nil},
        %{name: "EQUALS", pattern: "=", is_regex: false, line_number: 36, alias: nil},
        %{name: "PLUS", pattern: "+", is_regex: false, line_number: 37, alias: nil},
        %{name: "MINUS", pattern: "-", is_regex: false, line_number: 38, alias: nil},
        %{name: "STAR", pattern: "*", is_regex: false, line_number: 39, alias: nil},
        %{name: "SLASH", pattern: "/", is_regex: false, line_number: 40, alias: nil},
        %{name: "LESS_THAN", pattern: "<", is_regex: false, line_number: 43, alias: nil},
        %{name: "GREATER_THAN", pattern: ">", is_regex: false, line_number: 44, alias: nil},
        %{name: "LPAREN", pattern: "(", is_regex: false, line_number: 47, alias: nil},
        %{name: "RPAREN", pattern: ")", is_regex: false, line_number: 48, alias: nil},
        %{name: "COMMA", pattern: ",", is_regex: false, line_number: 49, alias: nil},
        %{name: "COLON", pattern: ":", is_regex: false, line_number: 50, alias: nil}
      ],
      skip_definitions: [

      ],
      error_definitions: [

      ],
      groups: %{

      }
    }
  end
end
