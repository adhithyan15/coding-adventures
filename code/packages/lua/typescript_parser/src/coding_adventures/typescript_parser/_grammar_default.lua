-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: typescript.grammar
-- Regenerate with: grammar-tools compile-grammar typescript.grammar
--
-- This file embeds a ParserGrammar as native Lua data structures.
-- Call parser_grammar() instead of reading and parsing the .grammar file.

local gt = require("coding_adventures.grammar_tools")

local function parser_grammar()
  local g = gt.ParserGrammar.new()
  g.rules = {
    {
      name="program",
      body={ type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
      line_number=34,
    },
    {
      name="statement",
      body={ type="alternation", choices={
        { type="rule_reference", name="var_declaration", is_token=false },
        { type="rule_reference", name="assignment", is_token=false },
        { type="rule_reference", name="expression_stmt", is_token=false },
      } },
      line_number=35,
    },
    {
      name="var_declaration",
      body={ type="sequence", elements={
        { type="rule_reference", name="KEYWORD", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=36,
    },
    {
      name="assignment",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=37,
    },
    {
      name="expression_stmt",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=38,
    },
    {
      name="expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="term", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="PLUS", is_token=true },
                { type="rule_reference", name="MINUS", is_token=true },
              } } },
            { type="rule_reference", name="term", is_token=false },
          } } },
      } },
      line_number=39,
    },
    {
      name="term",
      body={ type="sequence", elements={
        { type="rule_reference", name="factor", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="STAR", is_token=true },
                { type="rule_reference", name="SLASH", is_token=true },
              } } },
            { type="rule_reference", name="factor", is_token=false },
          } } },
      } },
      line_number=40,
    },
    {
      name="factor",
      body={ type="alternation", choices={
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="KEYWORD", is_token=true },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
      } },
      line_number=41,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
