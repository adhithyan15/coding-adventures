# frozen_string_literal: true
# AUTO-GENERATED FILE - DO NOT EDIT
require "coding_adventures_grammar_tools"

module CodingAdventures
  module XmlTokens
    def self.grammar
      @grammar ||= CodingAdventures::GrammarTools::TokenGrammar.new(
        definitions: [
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "TEXT", pattern: "[^<&]+", is_regex: true, line_number: 57, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "ENTITY_REF", pattern: "&[a-zA-Z][a-zA-Z0-9]*;", is_regex: true, line_number: 58, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "CHAR_REF", pattern: "&#[0-9]+;|&#x[0-9a-fA-F]+;", is_regex: true, line_number: 59, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "COMMENT_START", pattern: "<!--", is_regex: false, line_number: 60, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "CDATA_START", pattern: "<![CDATA[", is_regex: false, line_number: 61, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "PI_START", pattern: "<?", is_regex: false, line_number: 62, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "CLOSE_TAG_START", pattern: "</", is_regex: false, line_number: 63, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "OPEN_TAG_START", pattern: "<", is_regex: false, line_number: 64, alias_name: nil),
        ],
        keywords: [],
        mode: nil,
        skip_definitions: [
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "WHITESPACE", pattern: "[ \\t\\r\\n]+", is_regex: true, line_number: 42, alias_name: nil),
        ],
        error_definitions: [
        ],
        reserved_keywords: [],
        escape_mode: "none",
        groups: {
          "tag" => CodingAdventures::GrammarTools::PatternGroup.new(
            name: "tag",
            definitions: [
              CodingAdventures::GrammarTools::TokenDefinition.new(name: "TAG_NAME", pattern: "[a-zA-Z_][a-zA-Z0-9_:.-]*", is_regex: true, line_number: 79, alias_name: nil),
              CodingAdventures::GrammarTools::TokenDefinition.new(name: "ATTR_EQUALS", pattern: "=", is_regex: false, line_number: 80, alias_name: nil),
              CodingAdventures::GrammarTools::TokenDefinition.new(name: "ATTR_VALUE_DQ", pattern: "\"[^\"]*\"", is_regex: true, line_number: 81, alias_name: "ATTR_VALUE"),
              CodingAdventures::GrammarTools::TokenDefinition.new(name: "ATTR_VALUE_SQ", pattern: "'[^']*'", is_regex: true, line_number: 82, alias_name: "ATTR_VALUE"),
              CodingAdventures::GrammarTools::TokenDefinition.new(name: "TAG_CLOSE", pattern: ">", is_regex: false, line_number: 83, alias_name: nil),
              CodingAdventures::GrammarTools::TokenDefinition.new(name: "SELF_CLOSE", pattern: "/>", is_regex: false, line_number: 84, alias_name: nil),
              CodingAdventures::GrammarTools::TokenDefinition.new(name: "SLASH", pattern: "/", is_regex: false, line_number: 85, alias_name: nil),
            ]
          ),
          "comment" => CodingAdventures::GrammarTools::PatternGroup.new(
            name: "comment",
            definitions: [
              CodingAdventures::GrammarTools::TokenDefinition.new(name: "COMMENT_TEXT", pattern: "([^-]|-(?!->))+", is_regex: true, line_number: 99, alias_name: nil),
              CodingAdventures::GrammarTools::TokenDefinition.new(name: "COMMENT_END", pattern: "-->", is_regex: false, line_number: 100, alias_name: nil),
            ]
          ),
          "cdata" => CodingAdventures::GrammarTools::PatternGroup.new(
            name: "cdata",
            definitions: [
              CodingAdventures::GrammarTools::TokenDefinition.new(name: "CDATA_TEXT", pattern: "([^\\]]|\\](?!\\]>))+", is_regex: true, line_number: 113, alias_name: nil),
              CodingAdventures::GrammarTools::TokenDefinition.new(name: "CDATA_END", pattern: "]]>", is_regex: false, line_number: 114, alias_name: nil),
            ]
          ),
          "pi" => CodingAdventures::GrammarTools::PatternGroup.new(
            name: "pi",
            definitions: [
              CodingAdventures::GrammarTools::TokenDefinition.new(name: "PI_TARGET", pattern: "[a-zA-Z_][a-zA-Z0-9_:.-]*", is_regex: true, line_number: 128, alias_name: nil),
              CodingAdventures::GrammarTools::TokenDefinition.new(name: "PI_TEXT", pattern: "([^?]|\\?(?!>))+", is_regex: true, line_number: 129, alias_name: nil),
              CodingAdventures::GrammarTools::TokenDefinition.new(name: "PI_END", pattern: "?>", is_regex: false, line_number: 130, alias_name: nil),
            ]
          ),
        },
        case_sensitive: true,
        version: 1,
        case_insensitive: false
      )
    end
  end
end
