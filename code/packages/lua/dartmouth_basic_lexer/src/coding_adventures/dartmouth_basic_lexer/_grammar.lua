-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: dartmouth_basic.tokens
-- Regenerate with: grammar-tools compile-tokens dartmouth_basic.tokens
--
-- This file embeds a TokenGrammar as native Lua data structures.
-- Call token_grammar() instead of reading and parsing the .tokens file.

local gt = require("coding_adventures.grammar_tools")

local function token_grammar()
  local g = gt.TokenGrammar.new()
  g.definitions = {
        {
          name="LE",
          pattern="<=",
          is_regex=false,
          line_number=50,
          alias="",
        },
        {
          name="GE",
          pattern=">=",
          is_regex=false,
          line_number=51,
          alias="",
        },
        {
          name="NE",
          pattern="<>",
          is_regex=false,
          line_number=52,
          alias="",
        },
        {
          name="NUMBER",
          pattern="[0-9]*\\.?[0-9]+([Ee][+-]?[0-9]+)?",
          is_regex=true,
          line_number=85,
          alias="",
        },
        {
          name="LINE_NUM",
          pattern="[0-9]+",
          is_regex=true,
          line_number=86,
          alias="",
        },
        {
          name="STRING_BODY",
          pattern="\"[^\"]*\"",
          is_regex=true,
          line_number=112,
          alias="STRING",
        },
        {
          name="BUILTIN_FN",
          pattern="sin|cos|tan|atn|exp|log|abs|sqr|int|rnd|sgn",
          is_regex=true,
          line_number=168,
          alias="",
        },
        {
          name="USER_FN",
          pattern="fn[a-z]",
          is_regex=true,
          line_number=169,
          alias="",
        },
        {
          name="NAME",
          pattern="[a-z][a-z0-9]*\\$?",
          is_regex=true,
          line_number=204,
          alias="",
        },
        {
          name="PLUS",
          pattern="+",
          is_regex=false,
          line_number=244,
          alias="",
        },
        {
          name="MINUS",
          pattern="-",
          is_regex=false,
          line_number=245,
          alias="",
        },
        {
          name="STAR",
          pattern="*",
          is_regex=false,
          line_number=246,
          alias="",
        },
        {
          name="SLASH",
          pattern="/",
          is_regex=false,
          line_number=247,
          alias="",
        },
        {
          name="CARET",
          pattern="^",
          is_regex=false,
          line_number=248,
          alias="",
        },
        {
          name="EQ",
          pattern="=",
          is_regex=false,
          line_number=249,
          alias="",
        },
        {
          name="LT",
          pattern="<",
          is_regex=false,
          line_number=250,
          alias="",
        },
        {
          name="GT",
          pattern=">",
          is_regex=false,
          line_number=251,
          alias="",
        },
        {
          name="LPAREN",
          pattern="(",
          is_regex=false,
          line_number=252,
          alias="",
        },
        {
          name="RPAREN",
          pattern=")",
          is_regex=false,
          line_number=253,
          alias="",
        },
        {
          name="COMMA",
          pattern=",",
          is_regex=false,
          line_number=254,
          alias="",
        },
        {
          name="SEMICOLON",
          pattern=";",
          is_regex=false,
          line_number=255,
          alias="",
        },
        {
          name="NEWLINE",
          pattern="\\r?\\n",
          is_regex=true,
          line_number=276,
          alias="",
        },
      }
  g.keywords = {"LET", "PRINT", "INPUT", "IF", "THEN", "GOTO", "GOSUB", "RETURN", "FOR", "TO", "STEP", "NEXT", "END", "STOP", "REM", "READ", "DATA", "RESTORE", "DIM", "DEF"}
  g.mode = ""
  g.escape_mode = ""
  g.skip_definitions = {
        {
          name="WHITESPACE",
          pattern="[ \\t]+",
          is_regex=true,
          line_number=288,
          alias="",
        },
      }
  g.reserved_keywords = {}
  g.context_keywords = {}
  g.layout_keywords = {}
  g.soft_keywords = {}
  g.error_definitions = {
        {
          name="UNKNOWN",
          pattern=".",
          is_regex=true,
          line_number=304,
          alias="",
        },
      }
  g.groups = {}
  g.case_sensitive = true
  g.version = 0
  g.case_insensitive = true
  return g
end

return { token_grammar = token_grammar }
