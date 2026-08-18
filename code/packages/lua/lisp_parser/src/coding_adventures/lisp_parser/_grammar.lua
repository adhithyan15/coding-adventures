-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: lisp.grammar
-- Regenerate with: grammar-tools compile-grammar lisp.grammar
--
-- This file embeds a ParserGrammar as native Lua data structures.
-- Call parser_grammar() instead of reading and parsing the .grammar file.

local gt = require("coding_adventures.grammar_tools")

local function parser_grammar()
  local g = gt.ParserGrammar.new()
  g.rules = {
    {
      name="program",
      body={ type="repetition", element={ type="rule_reference", name="sexpr", is_token=false } },
      line_number=2,
    },
    {
      name="sexpr",
      body={ type="alternation", choices={
        { type="rule_reference", name="atom", is_token=false },
        { type="rule_reference", name="list", is_token=false },
        { type="rule_reference", name="quoted", is_token=false },
      } },
      line_number=3,
    },
    {
      name="atom",
      body={ type="alternation", choices={
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="rule_reference", name="SYMBOL", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
      } },
      line_number=4,
    },
    {
      name="list",
      body={ type="sequence", elements={
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="list_body", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=5,
    },
    {
      name="list_body",
      body={ type="optional", element={ type="sequence", elements={
          { type="rule_reference", name="sexpr", is_token=false },
          { type="repetition", element={ type="rule_reference", name="sexpr", is_token=false } },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="DOT", is_token=true },
              { type="rule_reference", name="sexpr", is_token=false },
            } } },
        } } },
      line_number=6,
    },
    {
      name="quoted",
      body={ type="sequence", elements={
        { type="rule_reference", name="QUOTE", is_token=true },
        { type="rule_reference", name="sexpr", is_token=false },
      } },
      line_number=7,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
