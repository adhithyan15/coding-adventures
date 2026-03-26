# AUTO-GENERATED FILE - DO NOT EDIT
require "coding_adventures_grammar_tools"

module CodingAdventures
  module SqlTokens
    def self.grammar
      @grammar ||= CodingAdventures::GrammarTools::TokenGrammar.new(
        definitions: [
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "NAME", pattern: "[a-zA-Z_][a-zA-Z0-9_]*", is_regex: true, line_number: 12, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "NUMBER", pattern: "[0-9]+(\\.[0-9]+)?", is_regex: true, line_number: 13, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "STRING_SQ", pattern: "'([^'\\\\]|\\\\.)*'", is_regex: true, line_number: 14, alias_name: "STRING"),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "QUOTED_ID", pattern: "`[^`]+`", is_regex: true, line_number: 15, alias_name: "NAME"),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LESS_EQUALS", pattern: "<=", is_regex: false, line_number: 17, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "GREATER_EQUALS", pattern: ">=", is_regex: false, line_number: 18, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "NOT_EQUALS", pattern: "!=", is_regex: false, line_number: 19, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "NEQ_ANSI", pattern: "<>", is_regex: false, line_number: 20, alias_name: "NOT_EQUALS"),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "EQUALS", pattern: "=", is_regex: false, line_number: 22, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LESS_THAN", pattern: "<", is_regex: false, line_number: 23, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "GREATER_THAN", pattern: ">", is_regex: false, line_number: 24, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "PLUS", pattern: "+", is_regex: false, line_number: 25, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "MINUS", pattern: "-", is_regex: false, line_number: 26, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "STAR", pattern: "*", is_regex: false, line_number: 27, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "SLASH", pattern: "/", is_regex: false, line_number: 28, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "PERCENT", pattern: "%", is_regex: false, line_number: 29, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LPAREN", pattern: "(", is_regex: false, line_number: 31, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "RPAREN", pattern: ")", is_regex: false, line_number: 32, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "COMMA", pattern: ",", is_regex: false, line_number: 33, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "SEMICOLON", pattern: ";", is_regex: false, line_number: 34, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "DOT", pattern: ".", is_regex: false, line_number: 35, alias_name: nil),
        ],
        keywords: ["SELECT", "FROM", "WHERE", "GROUP", "BY", "HAVING", "ORDER", "LIMIT", "OFFSET", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "DROP", "TABLE", "IF", "EXISTS", "NOT", "AND", "OR", "NULL", "IS", "IN", "BETWEEN", "LIKE", "AS", "DISTINCT", "ALL", "UNION", "INTERSECT", "EXCEPT", "JOIN", "INNER", "LEFT", "RIGHT", "OUTER", "CROSS", "FULL", "ON", "ASC", "DESC", "TRUE", "FALSE", "CASE", "WHEN", "THEN", "ELSE", "END", "PRIMARY", "KEY", "UNIQUE", "DEFAULT"],
        mode: nil,
        skip_definitions: [
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "WHITESPACE", pattern: "[ \\t\\r\\n]+", is_regex: true, line_number: 95, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LINE_COMMENT", pattern: "--[^\\n]*", is_regex: true, line_number: 96, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "BLOCK_COMMENT", pattern: "\\x2f\\*([^*]|\\*[^\\x2f])*\\*\\x2f", is_regex: true, line_number: 97, alias_name: nil),
        ],
        error_definitions: [
        ],
        reserved_keywords: [],
        escape_mode: nil,
        groups: {
        },
        case_sensitive: true,
        version: 1,
        case_insensitive: true
      )
    end
  end
end
