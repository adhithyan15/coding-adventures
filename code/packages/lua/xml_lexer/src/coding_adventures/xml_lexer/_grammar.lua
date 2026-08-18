-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: xml.tokens
-- Regenerate with: grammar-tools compile-tokens xml.tokens
--
-- This file embeds a TokenGrammar as native Lua data structures.
-- Call token_grammar() instead of reading and parsing the .tokens file.

local gt = require("coding_adventures.grammar_tools")

local function token_grammar()
  local g = gt.TokenGrammar.new()
  g.definitions = {
        {
          name="TEXT",
          pattern="[^<&]+",
          is_regex=true,
          line_number=77,
          alias="",
        },
        {
          name="ENTITY_REF",
          pattern="&[a-zA-Z][a-zA-Z0-9]*;",
          is_regex=true,
          line_number=78,
          alias="",
        },
        {
          name="CHAR_REF_HEX",
          pattern="&#x[0-9a-fA-F]+;",
          is_regex=true,
          line_number=85,
          alias="CHAR_REF",
        },
        {
          name="CHAR_REF_DEC",
          pattern="&#[0-9]+;",
          is_regex=true,
          line_number=86,
          alias="CHAR_REF",
        },
        {
          name="COMMENT_START",
          pattern="<!--",
          is_regex=false,
          line_number=88,
          alias="",
        },
        {
          name="CDATA_START",
          pattern="<![CDATA[",
          is_regex=false,
          line_number=89,
          alias="",
        },
        {
          name="PI_START",
          pattern="<?",
          is_regex=false,
          line_number=90,
          alias="",
        },
        {
          name="CLOSE_TAG_START",
          pattern="</",
          is_regex=false,
          line_number=91,
          alias="",
        },
        {
          name="OPEN_TAG_START",
          pattern="<",
          is_regex=false,
          line_number=92,
          alias="",
        },
      }
  g.keywords = {}
  g.mode = ""
  g.escape_mode = "none"
  g.skip_definitions = {
        {
          name="WHITESPACE",
          pattern="[ \\t\\r\\n]+",
          is_regex=true,
          line_number=62,
          alias="",
        },
      }
  g.reserved_keywords = {}
  g.context_keywords = {}
  g.layout_keywords = {}
  g.soft_keywords = {}
  g.error_definitions = {}
  g.groups = {
        ["cdata"] = {
          name="cdata",
          definitions={
              {
                name="CDATA_END",
                pattern="]]>",
                is_regex=false,
                line_number=150,
                alias="",
              },
              {
                name="CDATA_TEXT",
                pattern="[^\\]]+",
                is_regex=true,
                line_number=151,
                alias="",
              },
              {
                name="CDATA_BRACK",
                pattern="]",
                is_regex=true,
                line_number=152,
                alias="CDATA_TEXT",
              },
            },
        },
        ["comment"] = {
          name="comment",
          definitions={
              {
                name="COMMENT_END",
                pattern="-->",
                is_regex=false,
                line_number=133,
                alias="",
              },
              {
                name="COMMENT_TEXT",
                pattern="[^-]+",
                is_regex=true,
                line_number=134,
                alias="",
              },
              {
                name="COMMENT_DASH",
                pattern="-",
                is_regex=true,
                line_number=135,
                alias="COMMENT_TEXT",
              },
            },
        },
        ["pi"] = {
          name="pi",
          definitions={
              {
                name="PI_END",
                pattern="?>",
                is_regex=false,
                line_number=184,
                alias="",
              },
              {
                name="PI_TARGET",
                pattern="[a-zA-Z_][a-zA-Z0-9_:.-]*",
                is_regex=true,
                line_number=185,
                alias="",
              },
            },
        },
        ["pi_body"] = {
          name="pi_body",
          definitions={
              {
                name="PI_END",
                pattern="?>",
                is_regex=false,
                line_number=188,
                alias="",
              },
              {
                name="PI_TEXT",
                pattern="[^?]+",
                is_regex=true,
                line_number=189,
                alias="",
              },
              {
                name="PI_QMARK",
                pattern="\\?",
                is_regex=true,
                line_number=190,
                alias="PI_TEXT",
              },
            },
        },
        ["tag"] = {
          name="tag",
          definitions={
              {
                name="TAG_NAME",
                pattern="[a-zA-Z_][a-zA-Z0-9_:.-]*",
                is_regex=true,
                line_number=107,
                alias="",
              },
              {
                name="ATTR_EQUALS",
                pattern="=",
                is_regex=false,
                line_number=108,
                alias="",
              },
              {
                name="ATTR_VALUE_DQ",
                pattern="\"[^\"]*\"",
                is_regex=true,
                line_number=109,
                alias="ATTR_VALUE",
              },
              {
                name="ATTR_VALUE_SQ",
                pattern="'[^']*'",
                is_regex=true,
                line_number=110,
                alias="ATTR_VALUE",
              },
              {
                name="TAG_CLOSE",
                pattern=">",
                is_regex=false,
                line_number=111,
                alias="",
              },
              {
                name="SELF_CLOSE",
                pattern="/>",
                is_regex=false,
                line_number=112,
                alias="",
              },
              {
                name="SLASH",
                pattern="/",
                is_regex=false,
                line_number=113,
                alias="",
              },
            },
        },
      }
  g.case_sensitive = true
  g.version = 0
  g.case_insensitive = false
  return g
end

return { token_grammar = token_grammar }
