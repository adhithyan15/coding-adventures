-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: brainfuck.tokens
-- Regenerate with: grammar-tools compile-tokens brainfuck.tokens
--
-- This file embeds a TokenGrammar as native Lua data structures.
-- Call token_grammar() instead of reading and parsing the .tokens file.

local gt = require("coding_adventures.grammar_tools")

local function token_grammar()
  local g = gt.TokenGrammar.new()
  g.definitions = {
        {
          name="RIGHT",
          pattern=">",
          is_regex=false,
          line_number=23,
          alias="",
        },
        {
          name="LEFT",
          pattern="<",
          is_regex=false,
          line_number=24,
          alias="",
        },
        {
          name="INC",
          pattern="+",
          is_regex=false,
          line_number=29,
          alias="",
        },
        {
          name="DEC",
          pattern="-",
          is_regex=false,
          line_number=30,
          alias="",
        },
        {
          name="OUTPUT",
          pattern=".",
          is_regex=false,
          line_number=35,
          alias="",
        },
        {
          name="INPUT",
          pattern=",",
          is_regex=false,
          line_number=36,
          alias="",
        },
        {
          name="LOOP_START",
          pattern="[",
          is_regex=false,
          line_number=41,
          alias="",
        },
        {
          name="LOOP_END",
          pattern="]",
          is_regex=false,
          line_number=42,
          alias="",
        },
      }
  g.keywords = {}
  g.mode = ""
  g.escape_mode = ""
  g.skip_definitions = {
        {
          name="WHITESPACE",
          pattern="[ \\t\\r\\n]+",
          is_regex=true,
          line_number=65,
          alias="",
        },
        {
          name="COMMENT",
          pattern="[^><+\\-.,\\[\\] \\t\\r\\n]+",
          is_regex=true,
          line_number=66,
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
