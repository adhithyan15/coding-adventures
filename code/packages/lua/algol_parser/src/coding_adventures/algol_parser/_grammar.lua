-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: algol60.grammar
-- Regenerate with: grammar-tools compile-grammar algol60.grammar
--
-- This file embeds a ParserGrammar as native Lua data structures.
-- Call parser_grammar() instead of reading and parsing the .grammar file.

local gt = require("coding_adventures.grammar_tools")

local function parser_grammar()
  local g = gt.ParserGrammar.new()
  g.rules = {
    {
      name="program",
      body={ type="rule_reference", name="block", is_token=false },
      line_number=47,
    },
    {
      name="block",
      body={ type="sequence", elements={
        { type="literal", value="begin" },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="declaration", is_token=false },
            { type="rule_reference", name="SEMICOLON", is_token=true },
          } } },
        { type="repetition", element={ type="rule_reference", name="SEMICOLON", is_token=true } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="statement", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="SEMICOLON", is_token=true },
                { type="optional", element={ type="rule_reference", name="statement", is_token=false } },
              } } },
          } } },
        { type="literal", value="end" },
      } },
      line_number=53,
    },
    {
      name="declaration",
      body={ type="alternation", choices={
        { type="rule_reference", name="type_decl", is_token=false },
        { type="rule_reference", name="own_decl", is_token=false },
        { type="rule_reference", name="own_array_decl", is_token=false },
        { type="rule_reference", name="array_decl", is_token=false },
        { type="rule_reference", name="switch_decl", is_token=false },
        { type="rule_reference", name="procedure_decl", is_token=false },
      } },
      line_number=60,
    },
    {
      name="type_decl",
      body={ type="sequence", elements={
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="ident_list", is_token=false },
      } },
      line_number=71,
    },
    {
      name="own_decl",
      body={ type="sequence", elements={
        { type="literal", value="own" },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="ident_list", is_token=false },
      } },
      line_number=76,
    },
    {
      name="own_array_decl",
      body={ type="sequence", elements={
        { type="literal", value="own" },
        { type="optional", element={ type="rule_reference", name="type", is_token=false } },
        { type="literal", value="array" },
        { type="rule_reference", name="array_segment", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="array_segment", is_token=false },
          } } },
      } },
      line_number=81,
    },
    {
      name="type",
      body={ type="alternation", choices={
        { type="literal", value="integer" },
        { type="literal", value="real" },
        { type="literal", value="boolean" },
        { type="literal", value="string" },
      } },
      line_number=83,
    },
    {
      name="ident_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
      } },
      line_number=85,
    },
    {
      name="array_decl",
      body={ type="sequence", elements={
        { type="optional", element={ type="rule_reference", name="type", is_token=false } },
        { type="literal", value="array" },
        { type="rule_reference", name="array_segment", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="array_segment", is_token=false },
          } } },
      } },
      line_number=93,
    },
    {
      name="array_segment",
      body={ type="sequence", elements={
        { type="rule_reference", name="ident_list", is_token=false },
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="rule_reference", name="bound_pair", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="bound_pair", is_token=false },
          } } },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=95,
    },
    {
      name="bound_pair",
      body={ type="sequence", elements={
        { type="rule_reference", name="arith_expr", is_token=false },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="arith_expr", is_token=false },
      } },
      line_number=99,
    },
    {
      name="switch_decl",
      body={ type="sequence", elements={
        { type="literal", value="switch" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="ASSIGN", is_token=true },
        { type="rule_reference", name="switch_list", is_token=false },
      } },
      line_number=104,
    },
    {
      name="switch_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="desig_expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="desig_expr", is_token=false },
          } } },
      } },
      line_number=106,
    },
    {
      name="procedure_decl",
      body={ type="sequence", elements={
        { type="optional", element={ type="rule_reference", name="type", is_token=false } },
        { type="literal", value="procedure" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="formal_params", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
        { type="optional", element={ type="rule_reference", name="value_part", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="spec_part", is_token=false } },
        { type="rule_reference", name="proc_body", is_token=false },
      } },
      line_number=113,
    },
    {
      name="formal_params",
      body={ type="sequence", elements={
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="ident_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=118,
    },
    {
      name="value_part",
      body={ type="sequence", elements={
        { type="literal", value="value" },
        { type="rule_reference", name="ident_list", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=123,
    },
    {
      name="spec_part",
      body={ type="sequence", elements={
        { type="rule_reference", name="specifier", is_token=false },
        { type="rule_reference", name="ident_list", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=130,
    },
    {
      name="specifier",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="type", is_token=false },
          { type="literal", value="array" },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="type", is_token=false },
          { type="literal", value="procedure" },
        } },
        { type="literal", value="array" },
        { type="literal", value="label" },
        { type="literal", value="switch" },
        { type="literal", value="procedure" },
        { type="rule_reference", name="type", is_token=false },
      } },
      line_number=132,
    },
    {
      name="proc_body",
      body={ type="alternation", choices={
        { type="rule_reference", name="block", is_token=false },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=140,
    },
    {
      name="statement",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="label", is_token=false },
              { type="rule_reference", name="COLON", is_token=true },
            } } },
          { type="rule_reference", name="unlabeled_stmt", is_token=false },
        } },
        { type="sequence", elements={
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="label", is_token=false },
              { type="rule_reference", name="COLON", is_token=true },
            } } },
          { type="rule_reference", name="cond_stmt", is_token=false },
        } },
      } },
      line_number=152,
    },
    {
      name="label",
      body={ type="alternation", choices={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="INTEGER_LIT", is_token=true },
      } },
      line_number=155,
    },
    {
      name="unlabeled_stmt",
      body={ type="alternation", choices={
        { type="rule_reference", name="assign_stmt", is_token=false },
        { type="rule_reference", name="dummy_stmt", is_token=false },
        { type="rule_reference", name="goto_stmt", is_token=false },
        { type="rule_reference", name="proc_stmt", is_token=false },
        { type="rule_reference", name="compound_stmt", is_token=false },
        { type="rule_reference", name="block", is_token=false },
        { type="rule_reference", name="for_stmt", is_token=false },
      } },
      line_number=165,
    },
    {
      name="dummy_stmt",
      body={ type="alternation", choices={
        { type="positive_lookahead", element={ type="rule_reference", name="SEMICOLON", is_token=true } },
        { type="positive_lookahead", element={ type="literal", value="end" } },
        { type="positive_lookahead", element={ type="literal", value="else" } },
      } },
      line_number=175,
    },
    {
      name="cond_stmt",
      body={ type="sequence", elements={
        { type="literal", value="if" },
        { type="rule_reference", name="bool_expr", is_token=false },
        { type="literal", value="then" },
        { type="rule_reference", name="unlabeled_stmt", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="else" },
            { type="rule_reference", name="statement", is_token=false },
          } } },
      } },
      line_number=181,
    },
    {
      name="compound_stmt",
      body={ type="sequence", elements={
        { type="literal", value="begin" },
        { type="repetition", element={ type="rule_reference", name="SEMICOLON", is_token=true } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="statement", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="SEMICOLON", is_token=true },
                { type="optional", element={ type="rule_reference", name="statement", is_token=false } },
              } } },
          } } },
        { type="literal", value="end" },
      } },
      line_number=185,
    },
    {
      name="assign_stmt",
      body={ type="sequence", elements={
        { type="rule_reference", name="left_part", is_token=false },
        { type="repetition", element={ type="rule_reference", name="left_part", is_token=false } },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=191,
    },
    {
      name="left_part",
      body={ type="sequence", elements={
        { type="rule_reference", name="variable", is_token=false },
        { type="rule_reference", name="ASSIGN", is_token=true },
      } },
      line_number=193,
    },
    {
      name="goto_stmt",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="goto" },
          { type="rule_reference", name="desig_expr", is_token=false },
        } },
        { type="sequence", elements={
          { type="literal", value="go" },
          { type="literal", value="to" },
          { type="rule_reference", name="desig_expr", is_token=false },
        } },
      } },
      line_number=197,
    },
    {
      name="proc_stmt",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="optional", element={ type="rule_reference", name="actual_params", is_token=false } },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
      } },
      line_number=202,
    },
    {
      name="actual_params",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
          } } },
      } },
      line_number=204,
    },
    {
      name="for_stmt",
      body={ type="sequence", elements={
        { type="literal", value="for" },
        { type="rule_reference", name="variable", is_token=false },
        { type="rule_reference", name="ASSIGN", is_token=true },
        { type="rule_reference", name="for_list", is_token=false },
        { type="literal", value="do" },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=212,
    },
    {
      name="for_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="for_elem", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="for_elem", is_token=false },
          } } },
      } },
      line_number=214,
    },
    {
      name="for_elem",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="arith_expr", is_token=false },
          { type="literal", value="step" },
          { type="rule_reference", name="arith_expr", is_token=false },
          { type="literal", value="until" },
          { type="rule_reference", name="arith_expr", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="arith_expr", is_token=false },
          { type="literal", value="while" },
          { type="rule_reference", name="bool_expr", is_token=false },
        } },
        { type="rule_reference", name="arith_expr", is_token=false },
      } },
      line_number=218,
    },
    {
      name="expression",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="if" },
          { type="rule_reference", name="bool_expr", is_token=false },
          { type="literal", value="then" },
          { type="rule_reference", name="expression", is_token=false },
          { type="literal", value="else" },
          { type="rule_reference", name="expression", is_token=false },
        } },
        { type="rule_reference", name="expr_eqv", is_token=false },
      } },
      line_number=250,
    },
    {
      name="expr_eqv",
      body={ type="sequence", elements={
        { type="rule_reference", name="expr_impl", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="eqv" },
            { type="rule_reference", name="expr_impl", is_token=false },
          } } },
      } },
      line_number=253,
    },
    {
      name="expr_impl",
      body={ type="sequence", elements={
        { type="rule_reference", name="expr_or", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="impl" },
            { type="rule_reference", name="expr_or", is_token=false },
          } } },
      } },
      line_number=254,
    },
    {
      name="expr_or",
      body={ type="sequence", elements={
        { type="rule_reference", name="expr_and", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="or" },
            { type="rule_reference", name="expr_and", is_token=false },
          } } },
      } },
      line_number=255,
    },
    {
      name="expr_and",
      body={ type="sequence", elements={
        { type="rule_reference", name="expr_not", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="and" },
            { type="rule_reference", name="expr_not", is_token=false },
          } } },
      } },
      line_number=256,
    },
    {
      name="expr_not",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="not" },
          { type="rule_reference", name="expr_not", is_token=false },
        } },
        { type="rule_reference", name="expr_cmp", is_token=false },
      } },
      line_number=257,
    },
    {
      name="expr_cmp",
      body={ type="sequence", elements={
        { type="rule_reference", name="expr_add", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="EQ", is_token=true },
                { type="rule_reference", name="NEQ", is_token=true },
                { type="rule_reference", name="LT", is_token=true },
                { type="rule_reference", name="LEQ", is_token=true },
                { type="rule_reference", name="GT", is_token=true },
                { type="rule_reference", name="GEQ", is_token=true },
              } } },
            { type="rule_reference", name="expr_add", is_token=false },
          } } },
      } },
      line_number=258,
    },
    {
      name="expr_add",
      body={ type="sequence", elements={
        { type="optional", element={ type="alternation", choices={
            { type="rule_reference", name="PLUS", is_token=true },
            { type="rule_reference", name="MINUS", is_token=true },
          } } },
        { type="rule_reference", name="expr_mul", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="PLUS", is_token=true },
                { type="rule_reference", name="MINUS", is_token=true },
              } } },
            { type="rule_reference", name="expr_mul", is_token=false },
          } } },
      } },
      line_number=259,
    },
    {
      name="expr_mul",
      body={ type="sequence", elements={
        { type="rule_reference", name="expr_pow", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="STAR", is_token=true },
                { type="rule_reference", name="SLASH", is_token=true },
                { type="literal", value="div" },
                { type="literal", value="mod" },
              } } },
            { type="rule_reference", name="expr_pow", is_token=false },
          } } },
      } },
      line_number=260,
    },
    {
      name="expr_pow",
      body={ type="sequence", elements={
        { type="rule_reference", name="expr_atom", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="CARET", is_token=true },
                { type="rule_reference", name="POWER", is_token=true },
              } } },
            { type="rule_reference", name="expr_atom", is_token=false },
          } } },
      } },
      line_number=261,
    },
    {
      name="expr_atom",
      body={ type="alternation", choices={
        { type="rule_reference", name="INTEGER_LIT", is_token=true },
        { type="rule_reference", name="REAL_LIT", is_token=true },
        { type="rule_reference", name="STRING_LIT", is_token=true },
        { type="literal", value="true" },
        { type="literal", value="false" },
        { type="rule_reference", name="proc_call", is_token=false },
        { type="rule_reference", name="variable", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
      } },
      line_number=262,
    },
    {
      name="arith_expr",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="if" },
          { type="rule_reference", name="bool_expr", is_token=false },
          { type="literal", value="then" },
          { type="rule_reference", name="arith_expr", is_token=false },
          { type="literal", value="else" },
          { type="rule_reference", name="arith_expr", is_token=false },
        } },
        { type="rule_reference", name="simple_arith", is_token=false },
      } },
      line_number=274,
    },
    {
      name="simple_arith",
      body={ type="sequence", elements={
        { type="optional", element={ type="alternation", choices={
            { type="rule_reference", name="PLUS", is_token=true },
            { type="rule_reference", name="MINUS", is_token=true },
          } } },
        { type="rule_reference", name="term", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="PLUS", is_token=true },
                { type="rule_reference", name="MINUS", is_token=true },
              } } },
            { type="rule_reference", name="term", is_token=false },
          } } },
      } },
      line_number=278,
    },
    {
      name="term",
      body={ type="sequence", elements={
        { type="rule_reference", name="factor", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="STAR", is_token=true },
                { type="rule_reference", name="SLASH", is_token=true },
                { type="literal", value="div" },
                { type="literal", value="mod" },
              } } },
            { type="rule_reference", name="factor", is_token=false },
          } } },
      } },
      line_number=283,
    },
    {
      name="factor",
      body={ type="sequence", elements={
        { type="rule_reference", name="primary", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="CARET", is_token=true },
                { type="rule_reference", name="POWER", is_token=true },
              } } },
            { type="rule_reference", name="primary", is_token=false },
          } } },
      } },
      line_number=289,
    },
    {
      name="primary",
      body={ type="alternation", choices={
        { type="rule_reference", name="INTEGER_LIT", is_token=true },
        { type="rule_reference", name="REAL_LIT", is_token=true },
        { type="rule_reference", name="STRING_LIT", is_token=true },
        { type="literal", value="true" },
        { type="literal", value="false" },
        { type="rule_reference", name="proc_call", is_token=false },
        { type="rule_reference", name="variable", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="arith_expr", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
      } },
      line_number=291,
    },
    {
      name="bool_expr",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="if" },
          { type="rule_reference", name="bool_expr", is_token=false },
          { type="literal", value="then" },
          { type="rule_reference", name="bool_expr", is_token=false },
          { type="literal", value="else" },
          { type="rule_reference", name="bool_expr", is_token=false },
        } },
        { type="rule_reference", name="simple_bool", is_token=false },
      } },
      line_number=309,
    },
    {
      name="simple_bool",
      body={ type="sequence", elements={
        { type="rule_reference", name="implication", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="eqv" },
            { type="rule_reference", name="implication", is_token=false },
          } } },
      } },
      line_number=312,
    },
    {
      name="implication",
      body={ type="sequence", elements={
        { type="rule_reference", name="bool_term", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="impl" },
            { type="rule_reference", name="bool_term", is_token=false },
          } } },
      } },
      line_number=314,
    },
    {
      name="bool_term",
      body={ type="sequence", elements={
        { type="rule_reference", name="bool_factor", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="or" },
            { type="rule_reference", name="bool_factor", is_token=false },
          } } },
      } },
      line_number=316,
    },
    {
      name="bool_factor",
      body={ type="sequence", elements={
        { type="rule_reference", name="bool_secondary", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="and" },
            { type="rule_reference", name="bool_secondary", is_token=false },
          } } },
      } },
      line_number=318,
    },
    {
      name="bool_secondary",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="not" },
          { type="rule_reference", name="bool_secondary", is_token=false },
        } },
        { type="rule_reference", name="bool_primary", is_token=false },
      } },
      line_number=320,
    },
    {
      name="bool_primary",
      body={ type="alternation", choices={
        { type="rule_reference", name="relation", is_token=false },
        { type="literal", value="true" },
        { type="literal", value="false" },
        { type="rule_reference", name="proc_call", is_token=false },
        { type="rule_reference", name="variable", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="bool_expr", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
      } },
      line_number=322,
    },
    {
      name="relation",
      body={ type="sequence", elements={
        { type="rule_reference", name="simple_arith", is_token=false },
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="EQ", is_token=true },
            { type="rule_reference", name="NEQ", is_token=true },
            { type="rule_reference", name="LT", is_token=true },
            { type="rule_reference", name="LEQ", is_token=true },
            { type="rule_reference", name="GT", is_token=true },
            { type="rule_reference", name="GEQ", is_token=true },
          } } },
        { type="rule_reference", name="simple_arith", is_token=false },
      } },
      line_number=332,
    },
    {
      name="desig_expr",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="if" },
          { type="rule_reference", name="bool_expr", is_token=false },
          { type="literal", value="then" },
          { type="rule_reference", name="desig_expr", is_token=false },
          { type="literal", value="else" },
          { type="rule_reference", name="desig_expr", is_token=false },
        } },
        { type="rule_reference", name="simple_desig", is_token=false },
      } },
      line_number=337,
    },
    {
      name="simple_desig",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="NAME", is_token=true },
          { type="rule_reference", name="LBRACKET", is_token=true },
          { type="rule_reference", name="arith_expr", is_token=false },
          { type="rule_reference", name="RBRACKET", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="desig_expr", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="rule_reference", name="label", is_token=false },
      } },
      line_number=340,
    },
    {
      name="variable",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="LBRACKET", is_token=true },
            { type="rule_reference", name="subscripts", is_token=false },
            { type="rule_reference", name="RBRACKET", is_token=true },
          } } },
      } },
      line_number=352,
    },
    {
      name="subscripts",
      body={ type="sequence", elements={
        { type="rule_reference", name="arith_expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="arith_expr", is_token=false },
          } } },
      } },
      line_number=354,
    },
    {
      name="proc_call",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="actual_params", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=359,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
