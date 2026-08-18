-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: starlark.grammar
-- Regenerate with: grammar-tools compile-grammar starlark.grammar
--
-- This file embeds a ParserGrammar as native Lua data structures.
-- Call parser_grammar() instead of reading and parsing the .grammar file.

local gt = require("coding_adventures.grammar_tools")

local function parser_grammar()
  local g = gt.ParserGrammar.new()
  g.rules = {
    {
      name="file",
      body={ type="repetition", element={ type="alternation", choices={
          { type="rule_reference", name="NEWLINE", is_token=true },
          { type="rule_reference", name="statement", is_token=false },
        } } },
      line_number=48,
    },
    {
      name="statement",
      body={ type="alternation", choices={
        { type="rule_reference", name="compound_stmt", is_token=false },
        { type="rule_reference", name="simple_stmt", is_token=false },
      } },
      line_number=62,
    },
    {
      name="simple_stmt",
      body={ type="sequence", elements={
        { type="rule_reference", name="small_stmt", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="SEMICOLON", is_token=true },
            { type="rule_reference", name="small_stmt", is_token=false },
          } } },
        { type="rule_reference", name="NEWLINE", is_token=true },
      } },
      line_number=66,
    },
    {
      name="small_stmt",
      body={ type="alternation", choices={
        { type="rule_reference", name="return_stmt", is_token=false },
        { type="rule_reference", name="break_stmt", is_token=false },
        { type="rule_reference", name="continue_stmt", is_token=false },
        { type="rule_reference", name="pass_stmt", is_token=false },
        { type="rule_reference", name="load_stmt", is_token=false },
        { type="rule_reference", name="assign_stmt", is_token=false },
      } },
      line_number=68,
    },
    {
      name="return_stmt",
      body={ type="sequence", elements={
        { type="literal", value="return" },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
      } },
      line_number=82,
    },
    {
      name="break_stmt",
      body={ type="literal", value="break" },
      line_number=85,
    },
    {
      name="continue_stmt",
      body={ type="literal", value="continue" },
      line_number=88,
    },
    {
      name="pass_stmt",
      body={ type="literal", value="pass" },
      line_number=93,
    },
    {
      name="load_stmt",
      body={ type="sequence", elements={
        { type="literal", value="load" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="load_arg", is_token=false },
          } } },
        { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=102,
    },
    {
      name="load_arg",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="NAME", is_token=true },
          { type="rule_reference", name="EQUALS", is_token=true },
          { type="rule_reference", name="STRING", is_token=true },
        } },
        { type="rule_reference", name="STRING", is_token=true },
      } },
      line_number=103,
    },
    {
      name="assign_stmt",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression_list", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="assign_op", is_token=false },
                { type="rule_reference", name="augmented_assign_op", is_token=false },
              } } },
            { type="rule_reference", name="expression_list", is_token=false },
          } } },
      } },
      line_number=124,
    },
    {
      name="assign_op",
      body={ type="rule_reference", name="EQUALS", is_token=true },
      line_number=127,
    },
    {
      name="augmented_assign_op",
      body={ type="alternation", choices={
        { type="rule_reference", name="PLUS_EQUALS", is_token=true },
        { type="rule_reference", name="MINUS_EQUALS", is_token=true },
        { type="rule_reference", name="STAR_EQUALS", is_token=true },
        { type="rule_reference", name="SLASH_EQUALS", is_token=true },
        { type="rule_reference", name="FLOOR_DIV_EQUALS", is_token=true },
        { type="rule_reference", name="PERCENT_EQUALS", is_token=true },
        { type="rule_reference", name="AMP_EQUALS", is_token=true },
        { type="rule_reference", name="PIPE_EQUALS", is_token=true },
        { type="rule_reference", name="CARET_EQUALS", is_token=true },
        { type="rule_reference", name="LEFT_SHIFT_EQUALS", is_token=true },
        { type="rule_reference", name="RIGHT_SHIFT_EQUALS", is_token=true },
        { type="rule_reference", name="DOUBLE_STAR_EQUALS", is_token=true },
      } },
      line_number=129,
    },
    {
      name="compound_stmt",
      body={ type="alternation", choices={
        { type="rule_reference", name="if_stmt", is_token=false },
        { type="rule_reference", name="for_stmt", is_token=false },
        { type="rule_reference", name="def_stmt", is_token=false },
      } },
      line_number=138,
    },
    {
      name="if_stmt",
      body={ type="sequence", elements={
        { type="literal", value="if" },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="suite", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="elif" },
            { type="rule_reference", name="expression", is_token=false },
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="suite", is_token=false },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="else" },
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="suite", is_token=false },
          } } },
      } },
      line_number=150,
    },
    {
      name="for_stmt",
      body={ type="sequence", elements={
        { type="literal", value="for" },
        { type="rule_reference", name="loop_vars", is_token=false },
        { type="literal", value="in" },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="suite", is_token=false },
      } },
      line_number=164,
    },
    {
      name="loop_vars",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
      } },
      line_number=170,
    },
    {
      name="def_stmt",
      body={ type="sequence", elements={
        { type="literal", value="def" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="parameters", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="suite", is_token=false },
      } },
      line_number=180,
    },
    {
      name="suite",
      body={ type="alternation", choices={
        { type="rule_reference", name="simple_stmt", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="NEWLINE", is_token=true },
          { type="rule_reference", name="INDENT", is_token=true },
          { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
          { type="rule_reference", name="DEDENT", is_token=true },
        } },
      } },
      line_number=191,
    },
    {
      name="parameters",
      body={ type="sequence", elements={
        { type="rule_reference", name="parameter", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="parameter", is_token=false },
          } } },
        { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
      } },
      line_number=212,
    },
    {
      name="parameter",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="DOUBLE_STAR", is_token=true },
          { type="rule_reference", name="NAME", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="STAR", is_token=true },
          { type="rule_reference", name="NAME", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="NAME", is_token=true },
          { type="rule_reference", name="EQUALS", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
        } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=214,
    },
    {
      name="expression_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
          } } },
        { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
      } },
      line_number=248,
    },
    {
      name="expression",
      body={ type="alternation", choices={
        { type="rule_reference", name="lambda_expr", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="or_expr", is_token=false },
          { type="optional", element={ type="sequence", elements={
              { type="literal", value="if" },
              { type="rule_reference", name="or_expr", is_token=false },
              { type="literal", value="else" },
              { type="rule_reference", name="expression", is_token=false },
            } } },
        } },
      } },
      line_number=253,
    },
    {
      name="lambda_expr",
      body={ type="sequence", elements={
        { type="literal", value="lambda" },
        { type="optional", element={ type="rule_reference", name="lambda_params", is_token=false } },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=258,
    },
    {
      name="lambda_params",
      body={ type="sequence", elements={
        { type="rule_reference", name="lambda_param", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="lambda_param", is_token=false },
          } } },
        { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
      } },
      line_number=259,
    },
    {
      name="lambda_param",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="NAME", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="EQUALS", is_token=true },
              { type="rule_reference", name="expression", is_token=false },
            } } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="STAR", is_token=true },
          { type="rule_reference", name="NAME", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="DOUBLE_STAR", is_token=true },
          { type="rule_reference", name="NAME", is_token=true },
        } },
      } },
      line_number=260,
    },
    {
      name="or_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="and_expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="or" },
            { type="rule_reference", name="and_expr", is_token=false },
          } } },
      } },
      line_number=264,
    },
    {
      name="and_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="not_expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="and" },
            { type="rule_reference", name="not_expr", is_token=false },
          } } },
      } },
      line_number=268,
    },
    {
      name="not_expr",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="not" },
          { type="rule_reference", name="not_expr", is_token=false },
        } },
        { type="rule_reference", name="comparison", is_token=false },
      } },
      line_number=272,
    },
    {
      name="comparison",
      body={ type="sequence", elements={
        { type="rule_reference", name="bitwise_or", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="comp_op", is_token=false },
            { type="rule_reference", name="bitwise_or", is_token=false },
          } } },
      } },
      line_number=281,
    },
    {
      name="comp_op",
      body={ type="alternation", choices={
        { type="rule_reference", name="EQUALS_EQUALS", is_token=true },
        { type="rule_reference", name="NOT_EQUALS", is_token=true },
        { type="rule_reference", name="LESS_THAN", is_token=true },
        { type="rule_reference", name="GREATER_THAN", is_token=true },
        { type="rule_reference", name="LESS_EQUALS", is_token=true },
        { type="rule_reference", name="GREATER_EQUALS", is_token=true },
        { type="literal", value="in" },
        { type="sequence", elements={
          { type="literal", value="not" },
          { type="literal", value="in" },
        } },
      } },
      line_number=283,
    },
    {
      name="bitwise_or",
      body={ type="sequence", elements={
        { type="rule_reference", name="bitwise_xor", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="PIPE", is_token=true },
            { type="rule_reference", name="bitwise_xor", is_token=false },
          } } },
      } },
      line_number=289,
    },
    {
      name="bitwise_xor",
      body={ type="sequence", elements={
        { type="rule_reference", name="bitwise_and", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="CARET", is_token=true },
            { type="rule_reference", name="bitwise_and", is_token=false },
          } } },
      } },
      line_number=290,
    },
    {
      name="bitwise_and",
      body={ type="sequence", elements={
        { type="rule_reference", name="shift", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="AMP", is_token=true },
            { type="rule_reference", name="shift", is_token=false },
          } } },
      } },
      line_number=291,
    },
    {
      name="shift",
      body={ type="sequence", elements={
        { type="rule_reference", name="arith", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="LEFT_SHIFT", is_token=true },
                { type="rule_reference", name="RIGHT_SHIFT", is_token=true },
              } } },
            { type="rule_reference", name="arith", is_token=false },
          } } },
      } },
      line_number=294,
    },
    {
      name="arith",
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
      line_number=298,
    },
    {
      name="term",
      body={ type="sequence", elements={
        { type="rule_reference", name="factor", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="STAR", is_token=true },
                { type="rule_reference", name="SLASH", is_token=true },
                { type="rule_reference", name="FLOOR_DIV", is_token=true },
                { type="rule_reference", name="PERCENT", is_token=true },
              } } },
            { type="rule_reference", name="factor", is_token=false },
          } } },
      } },
      line_number=303,
    },
    {
      name="factor",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="group", element={ type="alternation", choices={
              { type="rule_reference", name="PLUS", is_token=true },
              { type="rule_reference", name="MINUS", is_token=true },
              { type="rule_reference", name="TILDE", is_token=true },
            } } },
          { type="rule_reference", name="factor", is_token=false },
        } },
        { type="rule_reference", name="power", is_token=false },
      } },
      line_number=309,
    },
    {
      name="power",
      body={ type="sequence", elements={
        { type="rule_reference", name="primary", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="DOUBLE_STAR", is_token=true },
            { type="rule_reference", name="factor", is_token=false },
          } } },
      } },
      line_number=317,
    },
    {
      name="primary",
      body={ type="sequence", elements={
        { type="rule_reference", name="atom", is_token=false },
        { type="repetition", element={ type="rule_reference", name="suffix", is_token=false } },
      } },
      line_number=334,
    },
    {
      name="suffix",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="DOT", is_token=true },
          { type="rule_reference", name="NAME", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LBRACKET", is_token=true },
          { type="rule_reference", name="subscript", is_token=false },
          { type="rule_reference", name="RBRACKET", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="arguments", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
      } },
      line_number=336,
    },
    {
      name="subscript",
      body={ type="alternation", choices={
        { type="rule_reference", name="expression", is_token=false },
        { type="sequence", elements={
          { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
          { type="rule_reference", name="COLON", is_token=true },
          { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COLON", is_token=true },
              { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
            } } },
        } },
      } },
      line_number=348,
    },
    {
      name="atom",
      body={ type="alternation", choices={
        { type="rule_reference", name="INT", is_token=true },
        { type="rule_reference", name="FLOAT", is_token=true },
        { type="sequence", elements={
          { type="rule_reference", name="STRING", is_token=true },
          { type="repetition", element={ type="rule_reference", name="STRING", is_token=true } },
        } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="True" },
        { type="literal", value="False" },
        { type="literal", value="None" },
        { type="rule_reference", name="list_expr", is_token=false },
        { type="rule_reference", name="dict_expr", is_token=false },
        { type="rule_reference", name="paren_expr", is_token=false },
      } },
      line_number=357,
    },
    {
      name="list_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="optional", element={ type="rule_reference", name="list_body", is_token=false } },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=373,
    },
    {
      name="list_body",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="comp_clause", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="expression", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="expression", is_token=false },
            } } },
          { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
        } },
      } },
      line_number=375,
    },
    {
      name="dict_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="rule_reference", name="dict_body", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=381,
    },
    {
      name="dict_body",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="dict_entry", is_token=false },
          { type="rule_reference", name="comp_clause", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="dict_entry", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="dict_entry", is_token=false },
            } } },
          { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
        } },
      } },
      line_number=383,
    },
    {
      name="dict_entry",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=386,
    },
    {
      name="paren_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="paren_body", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=393,
    },
    {
      name="paren_body",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="comp_clause", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="COMMA", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="expression", is_token=false },
              { type="repetition", element={ type="sequence", elements={
                  { type="rule_reference", name="COMMA", is_token=true },
                  { type="rule_reference", name="expression", is_token=false },
                } } },
              { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
            } } },
        } },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=395,
    },
    {
      name="comp_clause",
      body={ type="sequence", elements={
        { type="rule_reference", name="comp_for", is_token=false },
        { type="repetition", element={ type="alternation", choices={
            { type="rule_reference", name="comp_for", is_token=false },
            { type="rule_reference", name="comp_if", is_token=false },
          } } },
      } },
      line_number=411,
    },
    {
      name="comp_for",
      body={ type="sequence", elements={
        { type="literal", value="for" },
        { type="rule_reference", name="loop_vars", is_token=false },
        { type="literal", value="in" },
        { type="rule_reference", name="or_expr", is_token=false },
      } },
      line_number=413,
    },
    {
      name="comp_if",
      body={ type="sequence", elements={
        { type="literal", value="if" },
        { type="rule_reference", name="or_expr", is_token=false },
      } },
      line_number=415,
    },
    {
      name="arguments",
      body={ type="sequence", elements={
        { type="rule_reference", name="argument", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="argument", is_token=false },
          } } },
        { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
      } },
      line_number=434,
    },
    {
      name="argument",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="DOUBLE_STAR", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="STAR", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="NAME", is_token=true },
          { type="rule_reference", name="EQUALS", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
        } },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=436,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
