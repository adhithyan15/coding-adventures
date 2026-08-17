# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: xml.tokens
# Regenerate with: grammar-tools compile-tokens xml.tokens
#
# This file embeds a TokenGrammar as native Ruby data structures.
# Downstream packages require this file directly instead of reading
# and parsing the .tokens file at runtime.

require "coding_adventures_grammar_tools"

GT = CodingAdventures::GrammarTools unless defined?(GT)

TOKEN_GRAMMAR = GT::TokenGrammar.new(
  version: 1,
  case_insensitive: false,
  case_sensitive: true,
  definitions: [
      GT::TokenDefinition.new(
        name: "TEXT",
        pattern: "[^<&]+",
        is_regex: true,
        line_number: 77,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "ENTITY_REF",
        pattern: "&[a-zA-Z][a-zA-Z0-9]*;",
        is_regex: true,
        line_number: 78,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "CHAR_REF_HEX",
        pattern: "&#x[0-9a-fA-F]+;",
        is_regex: true,
        line_number: 85,
        alias_name: "CHAR_REF",
      ),
      GT::TokenDefinition.new(
        name: "CHAR_REF_DEC",
        pattern: "&#[0-9]+;",
        is_regex: true,
        line_number: 86,
        alias_name: "CHAR_REF",
      ),
      GT::TokenDefinition.new(
        name: "COMMENT_START",
        pattern: "<!--",
        is_regex: false,
        line_number: 88,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "CDATA_START",
        pattern: "<![CDATA[",
        is_regex: false,
        line_number: 89,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "PI_START",
        pattern: "<?",
        is_regex: false,
        line_number: 90,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "CLOSE_TAG_START",
        pattern: "</",
        is_regex: false,
        line_number: 91,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "OPEN_TAG_START",
        pattern: "<",
        is_regex: false,
        line_number: 92,
        alias_name: nil,
      ),
    ],
  keywords: [],
  mode: nil,
  escape_mode: "none",
  skip_definitions: [
      GT::TokenDefinition.new(
        name: "WHITESPACE",
        pattern: "[ \\t\\r\\n]+",
        is_regex: true,
        line_number: 62,
        alias_name: nil,
      ),
    ],
  reserved_keywords: [],
  error_definitions: [],
  groups: {
      "tag" => GT::PatternGroup.new(
        name: "tag",
        definitions: [
          GT::TokenDefinition.new(
            name: "TAG_NAME",
            pattern: "[a-zA-Z_][a-zA-Z0-9_:.-]*",
            is_regex: true,
            line_number: 107,
            alias_name: nil,
          ),
          GT::TokenDefinition.new(
            name: "ATTR_EQUALS",
            pattern: "=",
            is_regex: false,
            line_number: 108,
            alias_name: nil,
          ),
          GT::TokenDefinition.new(
            name: "ATTR_VALUE_DQ",
            pattern: "\"[^\"]*\"",
            is_regex: true,
            line_number: 109,
            alias_name: "ATTR_VALUE",
          ),
          GT::TokenDefinition.new(
            name: "ATTR_VALUE_SQ",
            pattern: "'[^']*'",
            is_regex: true,
            line_number: 110,
            alias_name: "ATTR_VALUE",
          ),
          GT::TokenDefinition.new(
            name: "TAG_CLOSE",
            pattern: ">",
            is_regex: false,
            line_number: 111,
            alias_name: nil,
          ),
          GT::TokenDefinition.new(
            name: "SELF_CLOSE",
            pattern: "/>",
            is_regex: false,
            line_number: 112,
            alias_name: nil,
          ),
          GT::TokenDefinition.new(
            name: "SLASH",
            pattern: "/",
            is_regex: false,
            line_number: 113,
            alias_name: nil,
          ),
        ],
      ),
      "comment" => GT::PatternGroup.new(
        name: "comment",
        definitions: [
          GT::TokenDefinition.new(
            name: "COMMENT_END",
            pattern: "-->",
            is_regex: false,
            line_number: 133,
            alias_name: nil,
          ),
          GT::TokenDefinition.new(
            name: "COMMENT_TEXT",
            pattern: "[^-]+",
            is_regex: true,
            line_number: 134,
            alias_name: nil,
          ),
          GT::TokenDefinition.new(
            name: "COMMENT_DASH",
            pattern: "-",
            is_regex: true,
            line_number: 135,
            alias_name: "COMMENT_TEXT",
          ),
        ],
      ),
      "cdata" => GT::PatternGroup.new(
        name: "cdata",
        definitions: [
          GT::TokenDefinition.new(
            name: "CDATA_END",
            pattern: "]]>",
            is_regex: false,
            line_number: 150,
            alias_name: nil,
          ),
          GT::TokenDefinition.new(
            name: "CDATA_TEXT",
            pattern: "[^\\]]+",
            is_regex: true,
            line_number: 151,
            alias_name: nil,
          ),
          GT::TokenDefinition.new(
            name: "CDATA_BRACK",
            pattern: "]",
            is_regex: true,
            line_number: 152,
            alias_name: "CDATA_TEXT",
          ),
        ],
      ),
      "pi" => GT::PatternGroup.new(
        name: "pi",
        definitions: [
          GT::TokenDefinition.new(
            name: "PI_END",
            pattern: "?>",
            is_regex: false,
            line_number: 184,
            alias_name: nil,
          ),
          GT::TokenDefinition.new(
            name: "PI_TARGET",
            pattern: "[a-zA-Z_][a-zA-Z0-9_:.-]*",
            is_regex: true,
            line_number: 185,
            alias_name: nil,
          ),
        ],
      ),
      "pi_body" => GT::PatternGroup.new(
        name: "pi_body",
        definitions: [
          GT::TokenDefinition.new(
            name: "PI_END",
            pattern: "?>",
            is_regex: false,
            line_number: 188,
            alias_name: nil,
          ),
          GT::TokenDefinition.new(
            name: "PI_TEXT",
            pattern: "[^?]+",
            is_regex: true,
            line_number: 189,
            alias_name: nil,
          ),
          GT::TokenDefinition.new(
            name: "PI_QMARK",
            pattern: "\\?",
            is_regex: true,
            line_number: 190,
            alias_name: "PI_TEXT",
          ),
        ],
      ),
    },
  layout_keywords: [],
  context_keywords: [],
  soft_keywords: [],
)
