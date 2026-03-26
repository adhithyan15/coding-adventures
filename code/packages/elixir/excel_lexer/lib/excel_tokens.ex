# AUTO-GENERATED FILE - DO NOT EDIT
defmodule ExcelTokens do
  alias CodingAdventures.GrammarTools.TokenGrammar

  def grammar do
    %TokenGrammar{
      version: 1,
      case_insensitive: true,
      case_sensitive: true,
      mode: nil,
      escape_mode: nil,
      keywords: ["TRUE", "FALSE"],
      reserved_keywords: [],
      definitions: [
        %{name: "SPACE", pattern: " +", is_regex: true, line_number: 13, alias: nil},
        %{name: "REF_PREFIX_QUOTED", pattern: "(\\[[^\\]]+\\])?'([^']|'')*'!", is_regex: true, line_number: 15, alias: "REF_PREFIX"},
        %{name: "REF_PREFIX_BARE", pattern: "(\\[[^\\]]+\\])?[A-Za-z_\\\\][A-Za-z0-9_\\.]*(?::[A-Za-z_\\\\][A-Za-z0-9_\\.]*)*!", is_regex: true, line_number: 16, alias: "REF_PREFIX"},
        %{name: "STRING", pattern: "\"([^\"]|\"\")*\"", is_regex: true, line_number: 18, alias: nil},
        %{name: "ERROR_CONSTANT", pattern: "#NULL!|#DIV\\/0!|#VALUE!|#REF!|#NAME\\?|#NUM!|#N\\/A|#SPILL!|#CALC!|#GETTING_DATA", is_regex: true, line_number: 19, alias: nil},
        %{name: "NUMBER", pattern: "[0-9]+\\.[0-9]*([eE][+-]?[0-9]+)?|\\.[0-9]+([eE][+-]?[0-9]+)?|[0-9]+([eE][+-]?[0-9]+)?", is_regex: true, line_number: 20, alias: nil},
        %{name: "STRUCTURED_KEYWORD", pattern: "\\[#(?:All|Data|Headers|Totals|This Row)\\]", is_regex: true, line_number: 22, alias: nil},
        %{name: "STRUCTURED_COLUMN", pattern: "\\[(?:[^\\[\\]]|\\]\\])+\\]", is_regex: true, line_number: 23, alias: nil},
        %{name: "CELL", pattern: "\\$?[A-Za-z]{1,3}\\$?[0-9]{1,7}", is_regex: true, line_number: 26, alias: nil},
        %{name: "NOT_EQUALS", pattern: "<>", is_regex: false, line_number: 28, alias: nil},
        %{name: "LESS_EQUALS", pattern: "<=", is_regex: false, line_number: 29, alias: nil},
        %{name: "GREATER_EQUALS", pattern: ">=", is_regex: false, line_number: 30, alias: nil},
        %{name: "PLUS", pattern: "+", is_regex: false, line_number: 32, alias: nil},
        %{name: "MINUS", pattern: "-", is_regex: false, line_number: 33, alias: nil},
        %{name: "STAR", pattern: "*", is_regex: false, line_number: 34, alias: nil},
        %{name: "SLASH", pattern: "/", is_regex: false, line_number: 35, alias: nil},
        %{name: "CARET", pattern: "^", is_regex: false, line_number: 36, alias: nil},
        %{name: "AMP", pattern: "&", is_regex: false, line_number: 37, alias: nil},
        %{name: "PERCENT", pattern: "%", is_regex: false, line_number: 38, alias: nil},
        %{name: "EQUALS", pattern: "=", is_regex: false, line_number: 39, alias: nil},
        %{name: "LESS_THAN", pattern: "<", is_regex: false, line_number: 40, alias: nil},
        %{name: "GREATER_THAN", pattern: ">", is_regex: false, line_number: 41, alias: nil},
        %{name: "BANG", pattern: "!", is_regex: false, line_number: 42, alias: nil},
        %{name: "DOLLAR", pattern: "$", is_regex: false, line_number: 43, alias: nil},
        %{name: "LPAREN", pattern: "(", is_regex: false, line_number: 44, alias: nil},
        %{name: "RPAREN", pattern: ")", is_regex: false, line_number: 45, alias: nil},
        %{name: "LBRACE", pattern: "{", is_regex: false, line_number: 46, alias: nil},
        %{name: "RBRACE", pattern: "}", is_regex: false, line_number: 47, alias: nil},
        %{name: "LBRACKET", pattern: "[", is_regex: false, line_number: 48, alias: nil},
        %{name: "RBRACKET", pattern: "]", is_regex: false, line_number: 49, alias: nil},
        %{name: "COMMA", pattern: ",", is_regex: false, line_number: 50, alias: nil},
        %{name: "SEMICOLON", pattern: ";", is_regex: false, line_number: 51, alias: nil},
        %{name: "COLON", pattern: ":", is_regex: false, line_number: 52, alias: nil},
        %{name: "AT", pattern: "@", is_regex: false, line_number: 53, alias: nil},
        %{name: "NAME", pattern: "[A-Za-z_\\\\][A-Za-z0-9_\\.]*", is_regex: true, line_number: 55, alias: nil},
        %{name: "FUNCTION_NAME", pattern: "(?!.)", is_regex: true, line_number: 61, alias: nil},
        %{name: "TABLE_NAME", pattern: "(?!.)", is_regex: true, line_number: 62, alias: nil},
        %{name: "COLUMN_REF", pattern: "(?!.)", is_regex: true, line_number: 63, alias: nil},
        %{name: "ROW_REF", pattern: "(?!.)", is_regex: true, line_number: 64, alias: nil}
      ],
      skip_definitions: [
        %{name: "NONSPACE_WHITESPACE", pattern: "[\\t\\r\\n]+", is_regex: true, line_number: 11, alias: nil}
      ],
      error_definitions: [

      ],
      groups: %{

      }
    }
  end
end
