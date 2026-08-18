-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: toml.grammar
-- Regenerate with: grammar-tools compile-grammar toml.grammar
--
-- This file embeds a ParserGrammar as native Lua data structures.
-- Call parser_grammar() instead of reading and parsing the .grammar file.

local gt = require("coding_adventures.grammar_tools")

local function parser_grammar()
  local g = gt.ParserGrammar.new()
  g.rules = {
    {
      name="document",
      body={ type="repetition", element={ type="alternation", choices={
          { type="rule_reference", name="NEWLINE", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
        } } },
      line_number=38,
    },
    {
      name="expression",
      body={ type="alternation", choices={
        { type="rule_reference", name="array_table_header", is_token=false },
        { type="rule_reference", name="table_header", is_token=false },
        { type="rule_reference", name="keyval", is_token=false },
      } },
      line_number=49,
    },
    {
      name="keyval",
      body={ type="sequence", elements={
        { type="rule_reference", name="key", is_token=false },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="value", is_token=false },
      } },
      line_number=57,
    },
    {
      name="key",
      body={ type="sequence", elements={
        { type="rule_reference", name="simple_key", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="DOT", is_token=true },
            { type="rule_reference", name="simple_key", is_token=false },
          } } },
      } },
      line_number=65,
    },
    {
      name="simple_key",
      body={ type="alternation", choices={
        { type="rule_reference", name="BARE_KEY", is_token=true },
        { type="rule_reference", name="BASIC_STRING", is_token=true },
        { type="rule_reference", name="LITERAL_STRING", is_token=true },
        { type="rule_reference", name="TRUE", is_token=true },
        { type="rule_reference", name="FALSE", is_token=true },
        { type="rule_reference", name="INTEGER", is_token=true },
        { type="rule_reference", name="FLOAT", is_token=true },
        { type="rule_reference", name="OFFSET_DATETIME", is_token=true },
        { type="rule_reference", name="LOCAL_DATETIME", is_token=true },
        { type="rule_reference", name="LOCAL_DATE", is_token=true },
        { type="rule_reference", name="LOCAL_TIME", is_token=true },
      } },
      line_number=82,
    },
    {
      name="table_header",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="rule_reference", name="key", is_token=false },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=92,
    },
    {
      name="array_table_header",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="rule_reference", name="key", is_token=false },
        { type="rule_reference", name="RBRACKET", is_token=true },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=104,
    },
    {
      name="value",
      body={ type="alternation", choices={
        { type="rule_reference", name="BASIC_STRING", is_token=true },
        { type="rule_reference", name="ML_BASIC_STRING", is_token=true },
        { type="rule_reference", name="LITERAL_STRING", is_token=true },
        { type="rule_reference", name="ML_LITERAL_STRING", is_token=true },
        { type="rule_reference", name="INTEGER", is_token=true },
        { type="rule_reference", name="FLOAT", is_token=true },
        { type="rule_reference", name="TRUE", is_token=true },
        { type="rule_reference", name="FALSE", is_token=true },
        { type="rule_reference", name="OFFSET_DATETIME", is_token=true },
        { type="rule_reference", name="LOCAL_DATETIME", is_token=true },
        { type="rule_reference", name="LOCAL_DATE", is_token=true },
        { type="rule_reference", name="LOCAL_TIME", is_token=true },
        { type="rule_reference", name="array", is_token=false },
        { type="rule_reference", name="inline_table", is_token=false },
      } },
      line_number=121,
    },
    {
      name="array",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="rule_reference", name="array_values", is_token=false },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=140,
    },
    {
      name="array_values",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="NEWLINE", is_token=true } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="value", is_token=false },
            { type="repetition", element={ type="rule_reference", name="NEWLINE", is_token=true } },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="repetition", element={ type="rule_reference", name="NEWLINE", is_token=true } },
                { type="rule_reference", name="value", is_token=false },
                { type="repetition", element={ type="rule_reference", name="NEWLINE", is_token=true } },
              } } },
            { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
            { type="repetition", element={ type="rule_reference", name="NEWLINE", is_token=true } },
          } } },
      } },
      line_number=142,
    },
    {
      name="inline_table",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="keyval", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="keyval", is_token=false },
              } } },
          } } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=162,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
