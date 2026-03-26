# AUTO-GENERATED FILE - DO NOT EDIT
defmodule SqlTokens do
  alias CodingAdventures.GrammarTools.TokenGrammar

  def grammar do
    %TokenGrammar{
      version: 1,
      case_insensitive: true,
      case_sensitive: true,
      mode: nil,
      escape_mode: nil,
      keywords: ["SELECT", "FROM", "WHERE", "GROUP", "BY", "HAVING", "ORDER", "LIMIT", "OFFSET", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "DROP", "TABLE", "IF", "EXISTS", "NOT", "AND", "OR", "NULL", "IS", "IN", "BETWEEN", "LIKE", "AS", "DISTINCT", "ALL", "UNION", "INTERSECT", "EXCEPT", "JOIN", "INNER", "LEFT", "RIGHT", "OUTER", "CROSS", "FULL", "ON", "ASC", "DESC", "TRUE", "FALSE", "CASE", "WHEN", "THEN", "ELSE", "END", "PRIMARY", "KEY", "UNIQUE", "DEFAULT"],
      reserved_keywords: [],
      definitions: [
        %{name: "NAME", pattern: "[a-zA-Z_][a-zA-Z0-9_]*", is_regex: true, line_number: 12, alias: nil},
        %{name: "NUMBER", pattern: "[0-9]+(\\.[0-9]+)?", is_regex: true, line_number: 13, alias: nil},
        %{name: "STRING_SQ", pattern: "'([^'\\\\]|\\\\.)*'", is_regex: true, line_number: 14, alias: "STRING"},
        %{name: "QUOTED_ID", pattern: "`[^`]+`", is_regex: true, line_number: 15, alias: "NAME"},
        %{name: "LESS_EQUALS", pattern: "<=", is_regex: false, line_number: 17, alias: nil},
        %{name: "GREATER_EQUALS", pattern: ">=", is_regex: false, line_number: 18, alias: nil},
        %{name: "NOT_EQUALS", pattern: "!=", is_regex: false, line_number: 19, alias: nil},
        %{name: "NEQ_ANSI", pattern: "<>", is_regex: false, line_number: 20, alias: "NOT_EQUALS"},
        %{name: "EQUALS", pattern: "=", is_regex: false, line_number: 22, alias: nil},
        %{name: "LESS_THAN", pattern: "<", is_regex: false, line_number: 23, alias: nil},
        %{name: "GREATER_THAN", pattern: ">", is_regex: false, line_number: 24, alias: nil},
        %{name: "PLUS", pattern: "+", is_regex: false, line_number: 25, alias: nil},
        %{name: "MINUS", pattern: "-", is_regex: false, line_number: 26, alias: nil},
        %{name: "STAR", pattern: "*", is_regex: false, line_number: 27, alias: nil},
        %{name: "SLASH", pattern: "/", is_regex: false, line_number: 28, alias: nil},
        %{name: "PERCENT", pattern: "%", is_regex: false, line_number: 29, alias: nil},
        %{name: "LPAREN", pattern: "(", is_regex: false, line_number: 31, alias: nil},
        %{name: "RPAREN", pattern: ")", is_regex: false, line_number: 32, alias: nil},
        %{name: "COMMA", pattern: ",", is_regex: false, line_number: 33, alias: nil},
        %{name: "SEMICOLON", pattern: ";", is_regex: false, line_number: 34, alias: nil},
        %{name: "DOT", pattern: ".", is_regex: false, line_number: 35, alias: nil}
      ],
      skip_definitions: [
        %{name: "WHITESPACE", pattern: "[ \\t\\r\\n]+", is_regex: true, line_number: 95, alias: nil},
        %{name: "LINE_COMMENT", pattern: "--[^\\n]*", is_regex: true, line_number: 96, alias: nil},
        %{name: "BLOCK_COMMENT", pattern: "\\x2f\\*([^*]|\\*[^\\x2f])*\\*\\x2f", is_regex: true, line_number: 97, alias: nil}
      ],
      error_definitions: [

      ],
      groups: %{

      }
    }
  end
end
