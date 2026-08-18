-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: css.grammar
-- Regenerate with: grammar-tools compile-grammar css.grammar
--
-- This file embeds a ParserGrammar as native Lua data structures.
-- Call parser_grammar() instead of reading and parsing the .grammar file.

local gt = require("coding_adventures.grammar_tools")

local function parser_grammar()
  local g = gt.ParserGrammar.new()
  g.rules = {
    {
      name="stylesheet",
      body={ type="repetition", element={ type="rule_reference", name="rule", is_token=false } },
      line_number=33,
    },
    {
      name="rule",
      body={ type="alternation", choices={
        { type="rule_reference", name="at_rule", is_token=false },
        { type="rule_reference", name="qualified_rule", is_token=false },
      } },
      line_number=35,
    },
    {
      name="at_rule",
      body={ type="sequence", elements={
        { type="rule_reference", name="AT_KEYWORD", is_token=true },
        { type="rule_reference", name="at_prelude", is_token=false },
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="SEMICOLON", is_token=true },
            { type="rule_reference", name="block", is_token=false },
          } } },
      } },
      line_number=55,
    },
    {
      name="at_prelude",
      body={ type="repetition", element={ type="rule_reference", name="at_prelude_token", is_token=false } },
      line_number=61,
    },
    {
      name="at_prelude_token",
      body={ type="alternation", choices={
        { type="rule_reference", name="IDENT", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="rule_reference", name="DIMENSION", is_token=true },
        { type="rule_reference", name="PERCENTAGE", is_token=true },
        { type="rule_reference", name="HASH", is_token=true },
        { type="rule_reference", name="CUSTOM_PROPERTY", is_token=true },
        { type="rule_reference", name="UNICODE_RANGE", is_token=true },
        { type="rule_reference", name="function_in_prelude", is_token=false },
        { type="rule_reference", name="paren_block", is_token=false },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="COMMA", is_token=true },
        { type="rule_reference", name="SLASH", is_token=true },
        { type="rule_reference", name="DOT", is_token=true },
        { type="rule_reference", name="STAR", is_token=true },
        { type="rule_reference", name="PLUS", is_token=true },
        { type="rule_reference", name="MINUS", is_token=true },
        { type="rule_reference", name="GREATER", is_token=true },
        { type="rule_reference", name="TILDE", is_token=true },
        { type="rule_reference", name="PIPE", is_token=true },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="AMPERSAND", is_token=true },
        { type="rule_reference", name="CDO", is_token=true },
        { type="rule_reference", name="CDC", is_token=true },
      } },
      line_number=63,
    },
    {
      name="function_in_prelude",
      body={ type="sequence", elements={
        { type="rule_reference", name="FUNCTION", is_token=true },
        { type="rule_reference", name="at_prelude_tokens", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=71,
    },
    {
      name="paren_block",
      body={ type="sequence", elements={
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="at_prelude_tokens", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=72,
    },
    {
      name="at_prelude_tokens",
      body={ type="repetition", element={ type="rule_reference", name="at_prelude_token", is_token=false } },
      line_number=73,
    },
    {
      name="qualified_rule",
      body={ type="sequence", elements={
        { type="rule_reference", name="selector_list", is_token=false },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=85,
    },
    {
      name="selector_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="complex_selector", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="complex_selector", is_token=false },
          } } },
      } },
      line_number=96,
    },
    {
      name="complex_selector",
      body={ type="sequence", elements={
        { type="rule_reference", name="compound_selector", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="optional", element={ type="rule_reference", name="combinator", is_token=false } },
            { type="rule_reference", name="compound_selector", is_token=false },
          } } },
      } },
      line_number=105,
    },
    {
      name="combinator",
      body={ type="alternation", choices={
        { type="rule_reference", name="GREATER", is_token=true },
        { type="rule_reference", name="PLUS", is_token=true },
        { type="rule_reference", name="TILDE", is_token=true },
      } },
      line_number=112,
    },
    {
      name="compound_selector",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="simple_selector", is_token=false },
          { type="repetition", element={ type="rule_reference", name="subclass_selector", is_token=false } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="subclass_selector", is_token=false },
          { type="repetition", element={ type="rule_reference", name="subclass_selector", is_token=false } },
        } },
      } },
      line_number=124,
    },
    {
      name="simple_selector",
      body={ type="alternation", choices={
        { type="rule_reference", name="IDENT", is_token=true },
        { type="rule_reference", name="STAR", is_token=true },
        { type="rule_reference", name="AMPERSAND", is_token=true },
      } },
      line_number=131,
    },
    {
      name="subclass_selector",
      body={ type="alternation", choices={
        { type="rule_reference", name="class_selector", is_token=false },
        { type="rule_reference", name="id_selector", is_token=false },
        { type="rule_reference", name="attribute_selector", is_token=false },
        { type="rule_reference", name="pseudo_class", is_token=false },
        { type="rule_reference", name="pseudo_element", is_token=false },
      } },
      line_number=139,
    },
    {
      name="class_selector",
      body={ type="sequence", elements={
        { type="rule_reference", name="DOT", is_token=true },
        { type="rule_reference", name="IDENT", is_token=true },
      } },
      line_number=145,
    },
    {
      name="id_selector",
      body={ type="rule_reference", name="HASH", is_token=true },
      line_number=150,
    },
    {
      name="attribute_selector",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="rule_reference", name="IDENT", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="attr_matcher", is_token=false },
            { type="rule_reference", name="attr_value", is_token=false },
            { type="optional", element={ type="rule_reference", name="IDENT", is_token=true } },
          } } },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=161,
    },
    {
      name="attr_matcher",
      body={ type="alternation", choices={
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="TILDE_EQUALS", is_token=true },
        { type="rule_reference", name="PIPE_EQUALS", is_token=true },
        { type="rule_reference", name="CARET_EQUALS", is_token=true },
        { type="rule_reference", name="DOLLAR_EQUALS", is_token=true },
        { type="rule_reference", name="STAR_EQUALS", is_token=true },
      } },
      line_number=163,
    },
    {
      name="attr_value",
      body={ type="alternation", choices={
        { type="rule_reference", name="IDENT", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
      } },
      line_number=166,
    },
    {
      name="pseudo_class",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="COLON", is_token=true },
          { type="rule_reference", name="FUNCTION", is_token=true },
          { type="rule_reference", name="pseudo_class_args", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="COLON", is_token=true },
          { type="rule_reference", name="IDENT", is_token=true },
        } },
      } },
      line_number=173,
    },
    {
      name="pseudo_class_args",
      body={ type="repetition", element={ type="rule_reference", name="pseudo_class_arg", is_token=false } },
      line_number=181,
    },
    {
      name="pseudo_class_arg",
      body={ type="alternation", choices={
        { type="rule_reference", name="IDENT", is_token=true },
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="rule_reference", name="DIMENSION", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="HASH", is_token=true },
        { type="rule_reference", name="PLUS", is_token=true },
        { type="rule_reference", name="COMMA", is_token=true },
        { type="rule_reference", name="DOT", is_token=true },
        { type="rule_reference", name="STAR", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="AMPERSAND", is_token=true },
        { type="sequence", elements={
          { type="rule_reference", name="FUNCTION", is_token=true },
          { type="rule_reference", name="pseudo_class_args", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LBRACKET", is_token=true },
          { type="rule_reference", name="pseudo_class_args", is_token=false },
          { type="rule_reference", name="RBRACKET", is_token=true },
        } },
      } },
      line_number=183,
    },
    {
      name="pseudo_element",
      body={ type="sequence", elements={
        { type="rule_reference", name="COLON_COLON", is_token=true },
        { type="rule_reference", name="IDENT", is_token=true },
      } },
      line_number=190,
    },
    {
      name="block",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="block_contents", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=200,
    },
    {
      name="block_contents",
      body={ type="repetition", element={ type="rule_reference", name="block_item", is_token=false } },
      line_number=202,
    },
    {
      name="block_item",
      body={ type="alternation", choices={
        { type="rule_reference", name="at_rule", is_token=false },
        { type="rule_reference", name="declaration_or_nested", is_token=false },
      } },
      line_number=211,
    },
    {
      name="declaration_or_nested",
      body={ type="alternation", choices={
        { type="rule_reference", name="declaration", is_token=false },
        { type="rule_reference", name="qualified_rule", is_token=false },
      } },
      line_number=217,
    },
    {
      name="declaration",
      body={ type="sequence", elements={
        { type="rule_reference", name="property", is_token=false },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="value_list", is_token=false },
        { type="optional", element={ type="rule_reference", name="priority", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=231,
    },
    {
      name="property",
      body={ type="alternation", choices={
        { type="rule_reference", name="IDENT", is_token=true },
        { type="rule_reference", name="CUSTOM_PROPERTY", is_token=true },
      } },
      line_number=233,
    },
    {
      name="priority",
      body={ type="sequence", elements={
        { type="rule_reference", name="BANG", is_token=true },
        { type="literal", value="important" },
      } },
      line_number=238,
    },
    {
      name="value_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="value", is_token=false },
        { type="repetition", element={ type="rule_reference", name="value", is_token=false } },
      } },
      line_number=251,
    },
    {
      name="value",
      body={ type="alternation", choices={
        { type="rule_reference", name="DIMENSION", is_token=true },
        { type="rule_reference", name="PERCENTAGE", is_token=true },
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="IDENT", is_token=true },
        { type="rule_reference", name="HASH", is_token=true },
        { type="rule_reference", name="CUSTOM_PROPERTY", is_token=true },
        { type="rule_reference", name="UNICODE_RANGE", is_token=true },
        { type="rule_reference", name="function_call", is_token=false },
        { type="rule_reference", name="SLASH", is_token=true },
        { type="rule_reference", name="COMMA", is_token=true },
        { type="rule_reference", name="PLUS", is_token=true },
        { type="rule_reference", name="MINUS", is_token=true },
      } },
      line_number=253,
    },
    {
      name="function_call",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="FUNCTION", is_token=true },
          { type="rule_reference", name="function_args", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="rule_reference", name="URL_TOKEN", is_token=true },
      } },
      line_number=267,
    },
    {
      name="function_args",
      body={ type="repetition", element={ type="rule_reference", name="function_arg", is_token=false } },
      line_number=272,
    },
    {
      name="function_arg",
      body={ type="alternation", choices={
        { type="rule_reference", name="DIMENSION", is_token=true },
        { type="rule_reference", name="PERCENTAGE", is_token=true },
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="IDENT", is_token=true },
        { type="rule_reference", name="HASH", is_token=true },
        { type="rule_reference", name="CUSTOM_PROPERTY", is_token=true },
        { type="rule_reference", name="COMMA", is_token=true },
        { type="rule_reference", name="SLASH", is_token=true },
        { type="rule_reference", name="PLUS", is_token=true },
        { type="rule_reference", name="MINUS", is_token=true },
        { type="rule_reference", name="STAR", is_token=true },
        { type="sequence", elements={
          { type="rule_reference", name="FUNCTION", is_token=true },
          { type="rule_reference", name="function_args", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
      } },
      line_number=274,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
