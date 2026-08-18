-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: dartmouth_basic.grammar
-- Regenerate with: grammar-tools compile-grammar dartmouth_basic.grammar
--
-- This file embeds a ParserGrammar as native Lua data structures.
-- Call parser_grammar() instead of reading and parsing the .grammar file.

local gt = require("coding_adventures.grammar_tools")

local function parser_grammar()
  local g = gt.ParserGrammar.new()
  g.rules = {
    {
      name="program",
      body={ type="repetition", element={ type="rule_reference", name="line", is_token=false } },
      line_number=70,
    },
    {
      name="line",
      body={ type="sequence", elements={
        { type="rule_reference", name="LINE_NUM", is_token=true },
        { type="optional", element={ type="rule_reference", name="statement", is_token=false } },
        { type="rule_reference", name="NEWLINE", is_token=true },
      } },
      line_number=81,
    },
    {
      name="statement",
      body={ type="alternation", choices={
        { type="rule_reference", name="let_stmt", is_token=false },
        { type="rule_reference", name="print_stmt", is_token=false },
        { type="rule_reference", name="input_stmt", is_token=false },
        { type="rule_reference", name="if_stmt", is_token=false },
        { type="rule_reference", name="goto_stmt", is_token=false },
        { type="rule_reference", name="gosub_stmt", is_token=false },
        { type="rule_reference", name="return_stmt", is_token=false },
        { type="rule_reference", name="for_stmt", is_token=false },
        { type="rule_reference", name="next_stmt", is_token=false },
        { type="rule_reference", name="end_stmt", is_token=false },
        { type="rule_reference", name="stop_stmt", is_token=false },
        { type="rule_reference", name="rem_stmt", is_token=false },
        { type="rule_reference", name="read_stmt", is_token=false },
        { type="rule_reference", name="data_stmt", is_token=false },
        { type="rule_reference", name="restore_stmt", is_token=false },
        { type="rule_reference", name="dim_stmt", is_token=false },
        { type="rule_reference", name="def_stmt", is_token=false },
      } },
      line_number=91,
    },
    {
      name="let_stmt",
      body={ type="sequence", elements={
        { type="literal", value="LET" },
        { type="rule_reference", name="variable", is_token=false },
        { type="rule_reference", name="EQ", is_token=true },
        { type="rule_reference", name="expr", is_token=false },
      } },
      line_number=121,
    },
    {
      name="print_stmt",
      body={ type="sequence", elements={
        { type="literal", value="PRINT" },
        { type="optional", element={ type="rule_reference", name="print_list", is_token=false } },
      } },
      line_number=137,
    },
    {
      name="print_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="print_item", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="print_sep", is_token=false },
            { type="rule_reference", name="print_item", is_token=false },
          } } },
        { type="optional", element={ type="rule_reference", name="print_sep", is_token=false } },
      } },
      line_number=139,
    },
    {
      name="print_item",
      body={ type="alternation", choices={
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="expr", is_token=false },
      } },
      line_number=141,
    },
    {
      name="print_sep",
      body={ type="alternation", choices={
        { type="rule_reference", name="COMMA", is_token=true },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=143,
    },
    {
      name="input_stmt",
      body={ type="sequence", elements={
        { type="literal", value="INPUT" },
        { type="rule_reference", name="variable", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="variable", is_token=false },
          } } },
      } },
      line_number=155,
    },
    {
      name="if_stmt",
      body={ type="sequence", elements={
        { type="literal", value="IF" },
        { type="rule_reference", name="expr", is_token=false },
        { type="rule_reference", name="relop", is_token=false },
        { type="rule_reference", name="expr", is_token=false },
        { type="literal", value="THEN" },
        { type="rule_reference", name="NUMBER", is_token=true },
      } },
      line_number=170,
    },
    {
      name="relop",
      body={ type="alternation", choices={
        { type="rule_reference", name="EQ", is_token=true },
        { type="rule_reference", name="LT", is_token=true },
        { type="rule_reference", name="GT", is_token=true },
        { type="rule_reference", name="LE", is_token=true },
        { type="rule_reference", name="GE", is_token=true },
        { type="rule_reference", name="NE", is_token=true },
      } },
      line_number=172,
    },
    {
      name="goto_stmt",
      body={ type="sequence", elements={
        { type="literal", value="GOTO" },
        { type="rule_reference", name="NUMBER", is_token=true },
      } },
      line_number=183,
    },
    {
      name="gosub_stmt",
      body={ type="sequence", elements={
        { type="literal", value="GOSUB" },
        { type="rule_reference", name="NUMBER", is_token=true },
      } },
      line_number=198,
    },
    {
      name="return_stmt",
      body={ type="literal", value="RETURN" },
      line_number=200,
    },
    {
      name="for_stmt",
      body={ type="sequence", elements={
        { type="literal", value="FOR" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="EQ", is_token=true },
        { type="rule_reference", name="expr", is_token=false },
        { type="literal", value="TO" },
        { type="rule_reference", name="expr", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="STEP" },
            { type="rule_reference", name="expr", is_token=false },
          } } },
      } },
      line_number=222,
    },
    {
      name="next_stmt",
      body={ type="sequence", elements={
        { type="literal", value="NEXT" },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=224,
    },
    {
      name="end_stmt",
      body={ type="literal", value="END" },
      line_number=233,
    },
    {
      name="stop_stmt",
      body={ type="literal", value="STOP" },
      line_number=234,
    },
    {
      name="rem_stmt",
      body={ type="literal", value="REM" },
      line_number=247,
    },
    {
      name="read_stmt",
      body={ type="sequence", elements={
        { type="literal", value="READ" },
        { type="rule_reference", name="variable", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="variable", is_token=false },
          } } },
      } },
      line_number=263,
    },
    {
      name="data_stmt",
      body={ type="sequence", elements={
        { type="literal", value="DATA" },
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="NUMBER", is_token=true },
          } } },
      } },
      line_number=265,
    },
    {
      name="restore_stmt",
      body={ type="literal", value="RESTORE" },
      line_number=267,
    },
    {
      name="dim_stmt",
      body={ type="sequence", elements={
        { type="literal", value="DIM" },
        { type="rule_reference", name="dim_decl", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="dim_decl", is_token=false },
          } } },
      } },
      line_number=280,
    },
    {
      name="dim_decl",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="NUMBER", is_token=true },
          } } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=282,
    },
    {
      name="def_stmt",
      body={ type="sequence", elements={
        { type="literal", value="DEF" },
        { type="rule_reference", name="USER_FN", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="EQ", is_token=true },
        { type="rule_reference", name="expr", is_token=false },
      } },
      line_number=295,
    },
    {
      name="variable",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="NAME", is_token=true },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="expr", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="expr", is_token=false },
            } } },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=312,
    },
    {
      name="expr",
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
      line_number=335,
    },
    {
      name="term",
      body={ type="sequence", elements={
        { type="rule_reference", name="power", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="STAR", is_token=true },
                { type="rule_reference", name="SLASH", is_token=true },
              } } },
            { type="rule_reference", name="power", is_token=false },
          } } },
      } },
      line_number=337,
    },
    {
      name="power",
      body={ type="sequence", elements={
        { type="rule_reference", name="unary", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="CARET", is_token=true },
            { type="rule_reference", name="power", is_token=false },
          } } },
      } },
      line_number=343,
    },
    {
      name="unary",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="MINUS", is_token=true },
          { type="rule_reference", name="primary", is_token=false },
        } },
        { type="rule_reference", name="primary", is_token=false },
      } },
      line_number=348,
    },
    {
      name="primary",
      body={ type="alternation", choices={
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="sequence", elements={
          { type="rule_reference", name="BUILTIN_FN", is_token=true },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="expr", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="USER_FN", is_token=true },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="expr", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="rule_reference", name="variable", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="expr", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
      } },
      line_number=366,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
