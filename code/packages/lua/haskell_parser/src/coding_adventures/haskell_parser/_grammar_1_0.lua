-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: haskell1.0.grammar
-- Regenerate with: grammar-tools compile-grammar haskell1.0.grammar
--
-- This file embeds a ParserGrammar as native Lua data structures.
-- Call parser_grammar() instead of reading and parsing the .grammar file.

local gt = require("coding_adventures.grammar_tools")

local function parser_grammar()
  local g = gt.ParserGrammar.new()
  g.rules = {
    {
      name="file",
      body={ type="repetition", element={ type="sequence", elements={
          { type="rule_reference", name="declaration", is_token=false },
          { type="optional", element={ type="rule_reference", name="layout_sep", is_token=false } },
        } } },
      line_number=10,
    },
    {
      name="declaration",
      body={ type="alternation", choices={
        { type="rule_reference", name="module_decl", is_token=false },
        { type="rule_reference", name="let_decl", is_token=false },
        { type="rule_reference", name="do_decl", is_token=false },
        { type="rule_reference", name="expr_decl", is_token=false },
      } },
      line_number=11,
    },
    {
      name="layout_open",
      body={ type="alternation", choices={
        { type="rule_reference", name="VIRTUAL_LBRACE", is_token=true },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="literal", value="{" },
      } },
      line_number=18,
    },
    {
      name="layout_close",
      body={ type="alternation", choices={
        { type="rule_reference", name="VIRTUAL_RBRACE", is_token=true },
        { type="rule_reference", name="RBRACE", is_token=true },
        { type="literal", value="}" },
      } },
      line_number=19,
    },
    {
      name="layout_sep",
      body={ type="alternation", choices={
        { type="rule_reference", name="VIRTUAL_SEMICOLON", is_token=true },
        { type="rule_reference", name="SEMICOLON", is_token=true },
        { type="rule_reference", name="NEWLINE", is_token=true },
      } },
      line_number=20,
    },
    {
      name="module_decl",
      body={ type="sequence", elements={
        { type="literal", value="module" },
        { type="rule_reference", name="module_name", is_token=false },
        { type="literal", value="where" },
        { type="rule_reference", name="layout_open", is_token=false },
        { type="rule_reference", name="module_body", is_token=false },
        { type="rule_reference", name="layout_close", is_token=false },
      } },
      line_number=22,
    },
    {
      name="module_name",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="DOT", is_token=true },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
      } },
      line_number=23,
    },
    {
      name="module_body",
      body={ type="repetition", element={ type="sequence", elements={
          { type="rule_reference", name="declaration", is_token=false },
          { type="optional", element={ type="rule_reference", name="layout_sep", is_token=false } },
        } } },
      line_number=24,
    },
    {
      name="let_decl",
      body={ type="sequence", elements={
        { type="literal", value="let" },
        { type="rule_reference", name="layout_open", is_token=false },
        { type="rule_reference", name="let_bindings", is_token=false },
        { type="rule_reference", name="layout_close", is_token=false },
        { type="literal", value="in" },
        { type="rule_reference", name="expr_decl", is_token=false },
      } },
      line_number=26,
    },
    {
      name="let_bindings",
      body={ type="repetition", element={ type="sequence", elements={
          { type="rule_reference", name="binding", is_token=false },
          { type="optional", element={ type="rule_reference", name="layout_sep", is_token=false } },
        } } },
      line_number=27,
    },
    {
      name="binding",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="expr_decl", is_token=false },
      } },
      line_number=28,
    },
    {
      name="do_decl",
      body={ type="sequence", elements={
        { type="literal", value="do" },
        { type="rule_reference", name="layout_open", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="expr_decl", is_token=false },
            { type="optional", element={ type="rule_reference", name="layout_sep", is_token=false } },
          } } },
        { type="rule_reference", name="layout_close", is_token=false },
      } },
      line_number=30,
    },
    {
      name="expr_decl",
      body={ type="alternation", choices={
        { type="rule_reference", name="lambda_expr", is_token=false },
        { type="rule_reference", name="app_expr", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="INTEGER", is_token=true },
        { type="rule_reference", name="FLOAT", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="CHARACTER", is_token=true },
      } },
      line_number=32,
    },
    {
      name="lambda_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="LAMBDA", is_token=true },
        { type="repetition", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="RARROW", is_token=true },
        { type="rule_reference", name="expr_decl", is_token=false },
      } },
      line_number=34,
    },
    {
      name="app_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="atom_expr", is_token=false },
        { type="repetition", element={ type="rule_reference", name="atom_expr", is_token=false } },
      } },
      line_number=35,
    },
    {
      name="atom_expr",
      body={ type="alternation", choices={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="INTEGER", is_token=true },
        { type="rule_reference", name="FLOAT", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="CHARACTER", is_token=true },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="expr_decl", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="expr_list", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LBRACKET", is_token=true },
          { type="optional", element={ type="rule_reference", name="expr_list", is_token=false } },
          { type="rule_reference", name="RBRACKET", is_token=true },
        } },
      } },
      line_number=36,
    },
    {
      name="expr_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="expr_decl", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="expr_decl", is_token=false },
          } } },
      } },
      line_number=45,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
