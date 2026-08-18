-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: lisp.tokens
-- Regenerate with: grammar-tools compile-tokens lisp.tokens
--
-- This file embeds a TokenGrammar as native Lua data structures.
-- Call token_grammar() instead of reading and parsing the .tokens file.

local gt = require("coding_adventures.grammar_tools")

local function token_grammar()
  local g = gt.TokenGrammar.new()
  g.definitions = {
        {
          name="NUMBER",
          pattern="-?[0-9]+",
          is_regex=true,
          line_number=11,
          alias="",
        },
        {
          name="SYMBOL",
          pattern="[a-zA-Z_+\\-*\\/=<>!?&][a-zA-Z0-9_+\\-*\\/=<>!?&]*",
          is_regex=true,
          line_number=12,
          alias="",
        },
        {
          name="STRING",
          pattern="\"([^\"\\\\]|\\\\.)*\"",
          is_regex=true,
          line_number=13,
          alias="",
        },
        {
          name="LPAREN",
          pattern="(",
          is_regex=false,
          line_number=14,
          alias="",
        },
        {
          name="RPAREN",
          pattern=")",
          is_regex=false,
          line_number=15,
          alias="",
        },
        {
          name="QUOTE",
          pattern="'",
          is_regex=false,
          line_number=16,
          alias="",
        },
        {
          name="DOT",
          pattern=".",
          is_regex=false,
          line_number=17,
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
          line_number=8,
          alias="",
        },
        {
          name="COMMENT",
          pattern=";[^\\n]*",
          is_regex=true,
          line_number=9,
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
