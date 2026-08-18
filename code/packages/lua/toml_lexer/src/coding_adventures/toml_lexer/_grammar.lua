-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: toml.tokens
-- Regenerate with: grammar-tools compile-tokens toml.tokens
--
-- This file embeds a TokenGrammar as native Lua data structures.
-- Call token_grammar() instead of reading and parsing the .tokens file.

local gt = require("coding_adventures.grammar_tools")

local function token_grammar()
  local g = gt.TokenGrammar.new()
  g.definitions = {
        {
          name="ML_BASIC_STRING",
          pattern="\"\"\"([^\\\\]|\\\\(.|\\n)|\\n)*?\"\"\"",
          is_regex=true,
          line_number=60,
          alias="",
        },
        {
          name="ML_LITERAL_STRING",
          pattern="'''[\\s\\S]*?'''",
          is_regex=true,
          line_number=61,
          alias="",
        },
        {
          name="BASIC_STRING",
          pattern="\"([^\"\\\\\\n]|\\\\.)*\"",
          is_regex=true,
          line_number=70,
          alias="",
        },
        {
          name="LITERAL_STRING",
          pattern="'[^'\\n]*'",
          is_regex=true,
          line_number=71,
          alias="",
        },
        {
          name="OFFSET_DATETIME_FRAC_TZ",
          pattern="\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}\\.\\d+[+-]\\d{2}:\\d{2}",
          is_regex=true,
          line_number=91,
          alias="OFFSET_DATETIME",
        },
        {
          name="OFFSET_DATETIME_FRAC_Z",
          pattern="\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}\\.\\d+Z",
          is_regex=true,
          line_number=92,
          alias="OFFSET_DATETIME",
        },
        {
          name="OFFSET_DATETIME_TZ",
          pattern="\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}[+-]\\d{2}:\\d{2}",
          is_regex=true,
          line_number=93,
          alias="OFFSET_DATETIME",
        },
        {
          name="OFFSET_DATETIME_Z",
          pattern="\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}Z",
          is_regex=true,
          line_number=94,
          alias="OFFSET_DATETIME",
        },
        {
          name="LOCAL_DATETIME_FRAC",
          pattern="\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}\\.\\d+",
          is_regex=true,
          line_number=95,
          alias="LOCAL_DATETIME",
        },
        {
          name="LOCAL_DATETIME",
          pattern="\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}",
          is_regex=true,
          line_number=96,
          alias="",
        },
        {
          name="LOCAL_DATE",
          pattern="\\d{4}-\\d{2}-\\d{2}",
          is_regex=true,
          line_number=97,
          alias="",
        },
        {
          name="LOCAL_TIME_FRAC",
          pattern="\\d{2}:\\d{2}:\\d{2}\\.\\d+",
          is_regex=true,
          line_number=98,
          alias="LOCAL_TIME",
        },
        {
          name="LOCAL_TIME",
          pattern="\\d{2}:\\d{2}:\\d{2}",
          is_regex=true,
          line_number=99,
          alias="",
        },
        {
          name="FLOAT_INF",
          pattern="[+-]?inf",
          is_regex=true,
          line_number=114,
          alias="FLOAT",
        },
        {
          name="FLOAT_NAN",
          pattern="[+-]?nan",
          is_regex=true,
          line_number=115,
          alias="FLOAT",
        },
        {
          name="FLOAT_EXP",
          pattern="[+-]?[0-9][0-9_]*\\.?[0-9_]*[eE][+-]?[0-9][0-9_]*",
          is_regex=true,
          line_number=116,
          alias="FLOAT",
        },
        {
          name="FLOAT_DEC",
          pattern="[+-]?[0-9][0-9_]*\\.[0-9][0-9_]*",
          is_regex=true,
          line_number=117,
          alias="FLOAT",
        },
        {
          name="HEX_INTEGER",
          pattern="0x[0-9a-fA-F][0-9a-fA-F_]*",
          is_regex=true,
          line_number=129,
          alias="INTEGER",
        },
        {
          name="OCT_INTEGER",
          pattern="0o[0-7][0-7_]*",
          is_regex=true,
          line_number=130,
          alias="INTEGER",
        },
        {
          name="BIN_INTEGER",
          pattern="0b[01][01_]*",
          is_regex=true,
          line_number=131,
          alias="INTEGER",
        },
        {
          name="INTEGER",
          pattern="[+-]?[0-9][0-9_]*",
          is_regex=true,
          line_number=132,
          alias="",
        },
        {
          name="TRUE",
          pattern="true",
          is_regex=false,
          line_number=143,
          alias="",
        },
        {
          name="FALSE",
          pattern="false",
          is_regex=false,
          line_number=144,
          alias="",
        },
        {
          name="BARE_KEY",
          pattern="[A-Za-z0-9_-]+",
          is_regex=true,
          line_number=158,
          alias="",
        },
        {
          name="EQUALS",
          pattern="=",
          is_regex=false,
          line_number=168,
          alias="",
        },
        {
          name="DOT",
          pattern=".",
          is_regex=false,
          line_number=169,
          alias="",
        },
        {
          name="COMMA",
          pattern=",",
          is_regex=false,
          line_number=170,
          alias="",
        },
        {
          name="LBRACKET",
          pattern="[",
          is_regex=false,
          line_number=171,
          alias="",
        },
        {
          name="RBRACKET",
          pattern="]",
          is_regex=false,
          line_number=172,
          alias="",
        },
        {
          name="LBRACE",
          pattern="{",
          is_regex=false,
          line_number=173,
          alias="",
        },
        {
          name="RBRACE",
          pattern="}",
          is_regex=false,
          line_number=174,
          alias="",
        },
        {
          name="NEWLINE",
          pattern="\\r?\\n",
          is_regex=true,
          line_number=175,
          alias="",
        },
      }
  g.keywords = {}
  g.mode = ""
  g.escape_mode = "none"
  g.skip_definitions = {
        {
          name="COMMENT",
          pattern="#[^\\n]*",
          is_regex=true,
          line_number=28,
          alias="",
        },
        {
          name="WHITESPACE",
          pattern="[ \\t]+",
          is_regex=true,
          line_number=29,
          alias="",
        },
      }
  g.reserved_keywords = {}
  g.context_keywords = {}
  g.layout_keywords = {}
  g.soft_keywords = {}
  g.error_definitions = {}
  g.groups = {}
  g.case_sensitive = true
  g.version = 0
  g.case_insensitive = false
  return g
end

return { token_grammar = token_grammar }
