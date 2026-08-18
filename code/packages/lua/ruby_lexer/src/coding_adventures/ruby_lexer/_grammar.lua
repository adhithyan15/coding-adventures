-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: ruby.tokens
-- Regenerate with: grammar-tools compile-tokens ruby.tokens
--
-- This file embeds a TokenGrammar as native Lua data structures.
-- Call token_grammar() instead of reading and parsing the .tokens file.

local gt = require("coding_adventures.grammar_tools")

local function token_grammar()
  local g = gt.TokenGrammar.new()
  g.definitions = {
        {
          name="NAME",
          pattern="[a-zA-Z_][a-zA-Z0-9_]*",
          is_regex=true,
          line_number=23,
          alias="",
        },
        {
          name="NUMBER",
          pattern="[0-9]+",
          is_regex=true,
          line_number=24,
          alias="",
        },
        {
          name="STRING",
          pattern="\"([^\"\\\\]|\\\\.)*\"",
          is_regex=true,
          line_number=25,
          alias="",
        },
        {
          name="EQUALS_EQUALS",
          pattern="==",
          is_regex=false,
          line_number=28,
          alias="",
        },
        {
          name="DOT_DOT",
          pattern="..",
          is_regex=false,
          line_number=29,
          alias="",
        },
        {
          name="HASH_ROCKET",
          pattern="=>",
          is_regex=false,
          line_number=30,
          alias="",
        },
        {
          name="NOT_EQUALS",
          pattern="!=",
          is_regex=false,
          line_number=31,
          alias="",
        },
        {
          name="LESS_EQUALS",
          pattern="<=",
          is_regex=false,
          line_number=32,
          alias="",
        },
        {
          name="GREATER_EQUALS",
          pattern=">=",
          is_regex=false,
          line_number=33,
          alias="",
        },
        {
          name="EQUALS",
          pattern="=",
          is_regex=false,
          line_number=36,
          alias="",
        },
        {
          name="PLUS",
          pattern="+",
          is_regex=false,
          line_number=37,
          alias="",
        },
        {
          name="MINUS",
          pattern="-",
          is_regex=false,
          line_number=38,
          alias="",
        },
        {
          name="STAR",
          pattern="*",
          is_regex=false,
          line_number=39,
          alias="",
        },
        {
          name="SLASH",
          pattern="/",
          is_regex=false,
          line_number=40,
          alias="",
        },
        {
          name="LESS_THAN",
          pattern="<",
          is_regex=false,
          line_number=43,
          alias="",
        },
        {
          name="GREATER_THAN",
          pattern=">",
          is_regex=false,
          line_number=44,
          alias="",
        },
        {
          name="LPAREN",
          pattern="(",
          is_regex=false,
          line_number=47,
          alias="",
        },
        {
          name="RPAREN",
          pattern=")",
          is_regex=false,
          line_number=48,
          alias="",
        },
        {
          name="COMMA",
          pattern=",",
          is_regex=false,
          line_number=49,
          alias="",
        },
        {
          name="COLON",
          pattern=":",
          is_regex=false,
          line_number=50,
          alias="",
        },
      }
  g.keywords = {"if", "else", "elsif", "end", "while", "for", "do", "def", "return", "class", "module", "require", "puts", "true", "false", "nil", "and", "or", "not", "then", "unless", "until", "yield", "begin", "rescue", "ensure"}
  g.mode = ""
  g.escape_mode = ""
  g.skip_definitions = {}
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
