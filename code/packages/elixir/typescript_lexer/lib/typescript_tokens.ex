# AUTO-GENERATED FILE - DO NOT EDIT
defmodule TypescriptTokens do
  alias CodingAdventures.GrammarTools.TokenGrammar

  def grammar do
    %TokenGrammar{
      version: 1,
      case_insensitive: false,
      case_sensitive: true,
      mode: nil,
      escape_mode: nil,
      keywords: ["let", "const", "var", "if", "else", "while", "for", "do", "function", "return", "class", "import", "export", "from", "as", "new", "this", "typeof", "instanceof", "true", "false", "null", "undefined", "interface", "type", "enum", "namespace", "declare", "readonly", "public", "private", "protected", "abstract", "implements", "extends", "keyof", "infer", "never", "unknown", "any", "void", "number", "string", "boolean", "object", "symbol", "bigint"],
      reserved_keywords: [],
      definitions: [
        %{name: "NAME", pattern: "[a-zA-Z_$][a-zA-Z0-9_$]*", is_regex: true, line_number: 25, alias: nil},
        %{name: "NUMBER", pattern: "[0-9]+", is_regex: true, line_number: 26, alias: nil},
        %{name: "STRING", pattern: "\"([^\"\\\\]|\\\\.)*\"", is_regex: true, line_number: 27, alias: nil},
        %{name: "STRICT_EQUALS", pattern: "===", is_regex: false, line_number: 30, alias: nil},
        %{name: "STRICT_NOT_EQUALS", pattern: "!==", is_regex: false, line_number: 31, alias: nil},
        %{name: "EQUALS_EQUALS", pattern: "==", is_regex: false, line_number: 32, alias: nil},
        %{name: "NOT_EQUALS", pattern: "!=", is_regex: false, line_number: 33, alias: nil},
        %{name: "LESS_EQUALS", pattern: "<=", is_regex: false, line_number: 34, alias: nil},
        %{name: "GREATER_EQUALS", pattern: ">=", is_regex: false, line_number: 35, alias: nil},
        %{name: "ARROW", pattern: "=>", is_regex: false, line_number: 36, alias: nil},
        %{name: "EQUALS", pattern: "=", is_regex: false, line_number: 39, alias: nil},
        %{name: "PLUS", pattern: "+", is_regex: false, line_number: 40, alias: nil},
        %{name: "MINUS", pattern: "-", is_regex: false, line_number: 41, alias: nil},
        %{name: "STAR", pattern: "*", is_regex: false, line_number: 42, alias: nil},
        %{name: "SLASH", pattern: "/", is_regex: false, line_number: 43, alias: nil},
        %{name: "LESS_THAN", pattern: "<", is_regex: false, line_number: 44, alias: nil},
        %{name: "GREATER_THAN", pattern: ">", is_regex: false, line_number: 45, alias: nil},
        %{name: "BANG", pattern: "!", is_regex: false, line_number: 46, alias: nil},
        %{name: "LPAREN", pattern: "(", is_regex: false, line_number: 49, alias: nil},
        %{name: "RPAREN", pattern: ")", is_regex: false, line_number: 50, alias: nil},
        %{name: "LBRACE", pattern: "{", is_regex: false, line_number: 51, alias: nil},
        %{name: "RBRACE", pattern: "}", is_regex: false, line_number: 52, alias: nil},
        %{name: "LBRACKET", pattern: "[", is_regex: false, line_number: 53, alias: nil},
        %{name: "RBRACKET", pattern: "]", is_regex: false, line_number: 54, alias: nil},
        %{name: "COMMA", pattern: ",", is_regex: false, line_number: 55, alias: nil},
        %{name: "COLON", pattern: ":", is_regex: false, line_number: 56, alias: nil},
        %{name: "SEMICOLON", pattern: ";", is_regex: false, line_number: 57, alias: nil},
        %{name: "DOT", pattern: ".", is_regex: false, line_number: 58, alias: nil}
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
