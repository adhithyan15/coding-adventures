# AUTO-GENERATED FILE - DO NOT EDIT
defmodule PythonTokens do
  alias CodingAdventures.GrammarTools.TokenGrammar

  def grammar do
    %TokenGrammar{
      version: 1,
      case_insensitive: false,
      case_sensitive: true,
      mode: nil,
      escape_mode: nil,
      keywords: ["if", "else", "elif", "while", "for", "def", "return", "class", "import", "from", "as", "True", "False", "None"],
      reserved_keywords: [],
      definitions: [
        %{name: "NAME", pattern: "[a-zA-Z_][a-zA-Z0-9_]*", is_regex: true, line_number: 13, alias: nil},
        %{name: "NUMBER", pattern: "[0-9]+", is_regex: true, line_number: 14, alias: nil},
        %{name: "STRING", pattern: "\"([^\"\\\\]|\\\\.)*\"", is_regex: true, line_number: 15, alias: nil},
        %{name: "EQUALS_EQUALS", pattern: "==", is_regex: false, line_number: 18, alias: nil},
        %{name: "EQUALS", pattern: "=", is_regex: false, line_number: 21, alias: nil},
        %{name: "PLUS", pattern: "+", is_regex: false, line_number: 22, alias: nil},
        %{name: "MINUS", pattern: "-", is_regex: false, line_number: 23, alias: nil},
        %{name: "STAR", pattern: "*", is_regex: false, line_number: 24, alias: nil},
        %{name: "SLASH", pattern: "/", is_regex: false, line_number: 25, alias: nil},
        %{name: "LPAREN", pattern: "(", is_regex: false, line_number: 28, alias: nil},
        %{name: "RPAREN", pattern: ")", is_regex: false, line_number: 29, alias: nil},
        %{name: "COMMA", pattern: ",", is_regex: false, line_number: 30, alias: nil},
        %{name: "COLON", pattern: ":", is_regex: false, line_number: 31, alias: nil}
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
