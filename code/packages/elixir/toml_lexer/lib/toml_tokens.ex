# AUTO-GENERATED FILE - DO NOT EDIT
defmodule TomlTokens do
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
        %{name: "ML_BASIC_STRING", pattern: "\"\"\"([^\\\\]|\\\\(.|\\n)|\\n)*?\"\"\"", is_regex: true, line_number: 60, alias: nil},
        %{name: "ML_LITERAL_STRING", pattern: "'''[\\s\\S]*?'''", is_regex: true, line_number: 61, alias: nil},
        %{name: "BASIC_STRING", pattern: "\"([^\"\\\\\\n]|\\\\.)*\"", is_regex: true, line_number: 70, alias: nil},
        %{name: "LITERAL_STRING", pattern: "'[^'\\n]*'", is_regex: true, line_number: 71, alias: nil},
        %{name: "OFFSET_DATETIME", pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}(\\.\\d+)?(Z|[+-]\\d{2}:\\d{2})", is_regex: true, line_number: 91, alias: nil},
        %{name: "LOCAL_DATETIME", pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}(\\.\\d+)?", is_regex: true, line_number: 92, alias: nil},
        %{name: "LOCAL_DATE", pattern: "\\d{4}-\\d{2}-\\d{2}", is_regex: true, line_number: 93, alias: nil},
        %{name: "LOCAL_TIME", pattern: "\\d{2}:\\d{2}:\\d{2}(\\.\\d+)?", is_regex: true, line_number: 94, alias: nil},
        %{name: "FLOAT_SPECIAL", pattern: "[+-]?(inf|nan)", is_regex: true, line_number: 109, alias: "FLOAT"},
        %{name: "FLOAT_EXP", pattern: "[+-]?([0-9](_?[0-9])*)(\\.[0-9](_?[0-9])*)?[eE][+-]?[0-9](_?[0-9])*", is_regex: true, line_number: 110, alias: "FLOAT"},
        %{name: "FLOAT_DEC", pattern: "[+-]?([0-9](_?[0-9])*)\\.([0-9](_?[0-9])*)", is_regex: true, line_number: 111, alias: "FLOAT"},
        %{name: "HEX_INTEGER", pattern: "0x[0-9a-fA-F](_?[0-9a-fA-F])*", is_regex: true, line_number: 123, alias: "INTEGER"},
        %{name: "OCT_INTEGER", pattern: "0o[0-7](_?[0-7])*", is_regex: true, line_number: 124, alias: "INTEGER"},
        %{name: "BIN_INTEGER", pattern: "0b[01](_?[01])*", is_regex: true, line_number: 125, alias: "INTEGER"},
        %{name: "INTEGER", pattern: "[+-]?[0-9](_?[0-9])*", is_regex: true, line_number: 126, alias: nil},
        %{name: "TRUE", pattern: "true", is_regex: false, line_number: 137, alias: nil},
        %{name: "FALSE", pattern: "false", is_regex: false, line_number: 138, alias: nil},
        %{name: "BARE_KEY", pattern: "[A-Za-z0-9_-]+", is_regex: true, line_number: 152, alias: nil},
        %{name: "EQUALS", pattern: "=", is_regex: false, line_number: 162, alias: nil},
        %{name: "DOT", pattern: ".", is_regex: false, line_number: 163, alias: nil},
        %{name: "COMMA", pattern: ",", is_regex: false, line_number: 164, alias: nil},
        %{name: "LBRACKET", pattern: "[", is_regex: false, line_number: 165, alias: nil},
        %{name: "RBRACKET", pattern: "]", is_regex: false, line_number: 166, alias: nil},
        %{name: "LBRACE", pattern: "{", is_regex: false, line_number: 167, alias: nil},
        %{name: "RBRACE", pattern: "}", is_regex: false, line_number: 168, alias: nil}
      ],
      skip_definitions: [
        %{name: "COMMENT", pattern: "#[^\\n]*", is_regex: true, line_number: 28, alias: nil},
        %{name: "WHITESPACE", pattern: "[ \\t]+", is_regex: true, line_number: 29, alias: nil}
      ],
      error_definitions: [

      ],
      groups: %{

      }
    }
  end
end
