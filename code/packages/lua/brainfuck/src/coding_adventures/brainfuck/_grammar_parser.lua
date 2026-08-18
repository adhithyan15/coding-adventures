-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: brainfuck.grammar
-- Regenerate with: grammar-tools compile-grammar brainfuck.grammar
--
-- This file embeds a ParserGrammar as native Lua data structures.
-- Call parser_grammar() instead of reading and parsing the .grammar file.

local gt = require("coding_adventures.grammar_tools")

local function parser_grammar()
  local g = gt.ParserGrammar.new()
  g.rules = {
    {
      name="program",
      body={ type="repetition", element={ type="rule_reference", name="instruction", is_token=false } },
      line_number=15,
    },
    {
      name="instruction",
      body={ type="alternation", choices={
        { type="rule_reference", name="loop", is_token=false },
        { type="rule_reference", name="command", is_token=false },
      } },
      line_number=21,
    },
    {
      name="loop",
      body={ type="sequence", elements={
        { type="rule_reference", name="LOOP_START", is_token=true },
        { type="repetition", element={ type="rule_reference", name="instruction", is_token=false } },
        { type="rule_reference", name="LOOP_END", is_token=true },
      } },
      line_number=27,
    },
    {
      name="command",
      body={ type="alternation", choices={
        { type="rule_reference", name="RIGHT", is_token=true },
        { type="rule_reference", name="LEFT", is_token=true },
        { type="rule_reference", name="INC", is_token=true },
        { type="rule_reference", name="DEC", is_token=true },
        { type="rule_reference", name="OUTPUT", is_token=true },
        { type="rule_reference", name="INPUT", is_token=true },
      } },
      line_number=32,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
