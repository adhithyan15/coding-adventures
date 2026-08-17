defmodule CodingAdventures.XmlLexer.Grammar do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: xml.tokens
  # Regenerate with: grammar-tools compile-tokens xml.tokens
  #
  # This file embeds a TokenGrammar as native Elixir data structures.
  # Call token_grammar/0 instead of reading and parsing the .tokens file.
  
  alias CodingAdventures.GrammarTools.TokenGrammar
  
  def token_grammar do
    %TokenGrammar{
      definitions: [
          %{
            name: "TEXT",
            pattern: "[^<&]+",
            is_regex: true,
            line_number: 77,
            alias: nil,
          },
          %{
            name: "ENTITY_REF",
            pattern: "&[a-zA-Z][a-zA-Z0-9]*;",
            is_regex: true,
            line_number: 78,
            alias: nil,
          },
          %{
            name: "CHAR_REF_HEX",
            pattern: "&#x[0-9a-fA-F]+;",
            is_regex: true,
            line_number: 85,
            alias: "CHAR_REF",
          },
          %{
            name: "CHAR_REF_DEC",
            pattern: "&#[0-9]+;",
            is_regex: true,
            line_number: 86,
            alias: "CHAR_REF",
          },
          %{
            name: "COMMENT_START",
            pattern: "<!--",
            is_regex: false,
            line_number: 88,
            alias: nil,
          },
          %{
            name: "CDATA_START",
            pattern: "<![CDATA[",
            is_regex: false,
            line_number: 89,
            alias: nil,
          },
          %{
            name: "PI_START",
            pattern: "<?",
            is_regex: false,
            line_number: 90,
            alias: nil,
          },
          %{
            name: "CLOSE_TAG_START",
            pattern: "</",
            is_regex: false,
            line_number: 91,
            alias: nil,
          },
          %{
            name: "OPEN_TAG_START",
            pattern: "<",
            is_regex: false,
            line_number: 92,
            alias: nil,
          },
        ],
      keywords: [],
      mode: nil,
      escape_mode: "none",
      skip_definitions: [
          %{
            name: "WHITESPACE",
            pattern: "[ \\t\\r\\n]+",
            is_regex: true,
            line_number: 62,
            alias: nil,
          },
        ],
      reserved_keywords: [],
      error_definitions: [],
      groups: %{
          "cdata" => %{
            name: "cdata",
            definitions: [
                %{
                  name: "CDATA_END",
                  pattern: "]]>",
                  is_regex: false,
                  line_number: 150,
                  alias: nil,
                },
                %{
                  name: "CDATA_TEXT",
                  pattern: "[^\\]]+",
                  is_regex: true,
                  line_number: 151,
                  alias: nil,
                },
                %{
                  name: "CDATA_BRACK",
                  pattern: "]",
                  is_regex: true,
                  line_number: 152,
                  alias: "CDATA_TEXT",
                },
              ],
          },
          "comment" => %{
            name: "comment",
            definitions: [
                %{
                  name: "COMMENT_END",
                  pattern: "-->",
                  is_regex: false,
                  line_number: 133,
                  alias: nil,
                },
                %{
                  name: "COMMENT_TEXT",
                  pattern: "[^-]+",
                  is_regex: true,
                  line_number: 134,
                  alias: nil,
                },
                %{
                  name: "COMMENT_DASH",
                  pattern: "-",
                  is_regex: true,
                  line_number: 135,
                  alias: "COMMENT_TEXT",
                },
              ],
          },
          "pi" => %{
            name: "pi",
            definitions: [
                %{
                  name: "PI_END",
                  pattern: "?>",
                  is_regex: false,
                  line_number: 184,
                  alias: nil,
                },
                %{
                  name: "PI_TARGET",
                  pattern: "[a-zA-Z_][a-zA-Z0-9_:.-]*",
                  is_regex: true,
                  line_number: 185,
                  alias: nil,
                },
              ],
          },
          "pi_body" => %{
            name: "pi_body",
            definitions: [
                %{
                  name: "PI_END",
                  pattern: "?>",
                  is_regex: false,
                  line_number: 188,
                  alias: nil,
                },
                %{
                  name: "PI_TEXT",
                  pattern: "[^?]+",
                  is_regex: true,
                  line_number: 189,
                  alias: nil,
                },
                %{
                  name: "PI_QMARK",
                  pattern: "\\?",
                  is_regex: true,
                  line_number: 190,
                  alias: "PI_TEXT",
                },
              ],
          },
          "tag" => %{
            name: "tag",
            definitions: [
                %{
                  name: "TAG_NAME",
                  pattern: "[a-zA-Z_][a-zA-Z0-9_:.-]*",
                  is_regex: true,
                  line_number: 107,
                  alias: nil,
                },
                %{
                  name: "ATTR_EQUALS",
                  pattern: "=",
                  is_regex: false,
                  line_number: 108,
                  alias: nil,
                },
                %{
                  name: "ATTR_VALUE_DQ",
                  pattern: "\"[^\"]*\"",
                  is_regex: true,
                  line_number: 109,
                  alias: "ATTR_VALUE",
                },
                %{
                  name: "ATTR_VALUE_SQ",
                  pattern: "'[^']*'",
                  is_regex: true,
                  line_number: 110,
                  alias: "ATTR_VALUE",
                },
                %{
                  name: "TAG_CLOSE",
                  pattern: ">",
                  is_regex: false,
                  line_number: 111,
                  alias: nil,
                },
                %{
                  name: "SELF_CLOSE",
                  pattern: "/>",
                  is_regex: false,
                  line_number: 112,
                  alias: nil,
                },
                %{
                  name: "SLASH",
                  pattern: "/",
                  is_regex: false,
                  line_number: 113,
                  alias: nil,
                },
              ],
          },
        },
      layout_keywords: [],
      case_sensitive: true,
      version: 1,
      case_insensitive: false,
      start_mode: nil,
      transitions: [],
    }
  end
end
