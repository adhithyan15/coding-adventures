# AUTO-GENERATED FILE - DO NOT EDIT
defmodule XmlTokens do
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
        %{name: "TEXT", pattern: "[^<&]+", is_regex: true, line_number: 57, alias: nil},
        %{name: "ENTITY_REF", pattern: "&[a-zA-Z][a-zA-Z0-9]*;", is_regex: true, line_number: 58, alias: nil},
        %{name: "CHAR_REF", pattern: "&#[0-9]+;|&#x[0-9a-fA-F]+;", is_regex: true, line_number: 59, alias: nil},
        %{name: "COMMENT_START", pattern: "<!--", is_regex: false, line_number: 60, alias: nil},
        %{name: "CDATA_START", pattern: "<![CDATA[", is_regex: false, line_number: 61, alias: nil},
        %{name: "PI_START", pattern: "<?", is_regex: false, line_number: 62, alias: nil},
        %{name: "CLOSE_TAG_START", pattern: "</", is_regex: false, line_number: 63, alias: nil},
        %{name: "OPEN_TAG_START", pattern: "<", is_regex: false, line_number: 64, alias: nil}
      ],
      skip_definitions: [
        %{name: "WHITESPACE", pattern: "[ \\t\\r\\n]+", is_regex: true, line_number: 42, alias: nil}
      ],
      error_definitions: [

      ],
      groups: %{
        "cdata" => %{
          name: "cdata",
          definitions: [
            %{name: "CDATA_TEXT", pattern: "([^\\]]|\\](?!\\]>))+", is_regex: true, line_number: 113, alias: nil},
            %{name: "CDATA_END", pattern: "]]>", is_regex: false, line_number: 114, alias: nil}
          ]
        },
        "comment" => %{
          name: "comment",
          definitions: [
            %{name: "COMMENT_TEXT", pattern: "([^-]|-(?!->))+", is_regex: true, line_number: 99, alias: nil},
            %{name: "COMMENT_END", pattern: "-->", is_regex: false, line_number: 100, alias: nil}
          ]
        },
        "pi" => %{
          name: "pi",
          definitions: [
            %{name: "PI_TARGET", pattern: "[a-zA-Z_][a-zA-Z0-9_:.-]*", is_regex: true, line_number: 128, alias: nil},
            %{name: "PI_TEXT", pattern: "([^?]|\\?(?!>))+", is_regex: true, line_number: 129, alias: nil},
            %{name: "PI_END", pattern: "?>", is_regex: false, line_number: 130, alias: nil}
          ]
        },
        "tag" => %{
          name: "tag",
          definitions: [
            %{name: "TAG_NAME", pattern: "[a-zA-Z_][a-zA-Z0-9_:.-]*", is_regex: true, line_number: 79, alias: nil},
            %{name: "ATTR_EQUALS", pattern: "=", is_regex: false, line_number: 80, alias: nil},
            %{name: "ATTR_VALUE_DQ", pattern: "\"[^\"]*\"", is_regex: true, line_number: 81, alias: "ATTR_VALUE"},
            %{name: "ATTR_VALUE_SQ", pattern: "'[^']*'", is_regex: true, line_number: 82, alias: "ATTR_VALUE"},
            %{name: "TAG_CLOSE", pattern: ">", is_regex: false, line_number: 83, alias: nil},
            %{name: "SELF_CLOSE", pattern: "/>", is_regex: false, line_number: 84, alias: nil},
            %{name: "SLASH", pattern: "/", is_regex: false, line_number: 85, alias: nil}
          ]
        }
      }
    }
  end
end
