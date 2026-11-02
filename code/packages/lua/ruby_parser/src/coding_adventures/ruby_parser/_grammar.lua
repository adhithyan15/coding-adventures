-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: ruby.grammar
-- Regenerate with: grammar-tools compile-grammar ruby.grammar
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
      line_number=27,
    },
    {
      name="statement",
      body={ type="alternation", choices={
        { type="rule_reference", name="endless_def_statement", is_token=false },
        { type="rule_reference", name="def_statement", is_token=false },
        { type="rule_reference", name="class_statement", is_token=false },
        { type="rule_reference", name="module_statement", is_token=false },
        { type="rule_reference", name="if_statement", is_token=false },
        { type="rule_reference", name="unless_statement", is_token=false },
        { type="rule_reference", name="while_statement", is_token=false },
        { type="rule_reference", name="until_statement", is_token=false },
        { type="rule_reference", name="for_statement", is_token=false },
        { type="rule_reference", name="case_statement", is_token=false },
        { type="rule_reference", name="begin_statement", is_token=false },
        { type="rule_reference", name="return_statement", is_token=false },
        { type="rule_reference", name="break_statement", is_token=false },
        { type="rule_reference", name="next_statement", is_token=false },
        { type="rule_reference", name="redo_statement", is_token=false },
        { type="rule_reference", name="retry_statement", is_token=false },
        { type="rule_reference", name="yield_statement", is_token=false },
        { type="rule_reference", name="alias_statement", is_token=false },
        { type="rule_reference", name="undef_statement", is_token=false },
        { type="rule_reference", name="multi_assignment", is_token=false },
        { type="rule_reference", name="modifier_statement", is_token=false },
        { type="rule_reference", name="rightward_assignment", is_token=false },
        { type="rule_reference", name="index_assignment", is_token=false },
        { type="rule_reference", name="assignment", is_token=false },
        { type="rule_reference", name="defined_expression", is_token=false },
        { type="rule_reference", name="method_with_block", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="method_call", is_token=false },
          { type="negative_lookahead", element={ type="literal", value="+" } },
          { type="negative_lookahead", element={ type="literal", value="-" } },
          { type="negative_lookahead", element={ type="rule_reference", name="STAR", is_token=true } },
          { type="negative_lookahead", element={ type="rule_reference", name="SLASH", is_token=true } },
          { type="negative_lookahead", element={ type="literal", value="<<" } },
          { type="negative_lookahead", element={ type="literal", value="==" } },
          { type="negative_lookahead", element={ type="literal", value="!=" } },
          { type="negative_lookahead", element={ type="literal", value="<=" } },
          { type="negative_lookahead", element={ type="literal", value=">=" } },
          { type="negative_lookahead", element={ type="literal", value="<" } },
          { type="negative_lookahead", element={ type="literal", value=">" } },
          { type="negative_lookahead", element={ type="literal", value="&&" } },
          { type="negative_lookahead", element={ type="literal", value="||" } },
          { type="negative_lookahead", element={ type="literal", value="and" } },
          { type="negative_lookahead", element={ type="literal", value="or" } },
          { type="negative_lookahead", element={ type="literal", value="..." } },
          { type="negative_lookahead", element={ type="literal", value=".." } },
          { type="negative_lookahead", element={ type="literal", value="?" } },
        } },
        { type="rule_reference", name="method_call_no_paren", is_token=false },
        { type="rule_reference", name="expression_stmt", is_token=false },
      } },
      line_number=28,
    },
    {
      name="multi_assignment",
      body={ type="sequence", elements={
        { type="rule_reference", name="mlhs_target", is_token=false },
        { type="rule_reference", name="COMMA", is_token=true },
        { type="rule_reference", name="mlhs_target", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="mlhs_target", is_token=false },
          } } },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
          } } },
      } },
      line_number=87,
    },
    {
      name="mlhs_target",
      body={ type="sequence", elements={
        { type="optional", element={ type="literal", value="*" } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=88,
    },
    {
      name="modifier_statement",
      body={ type="sequence", elements={
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="assignment", is_token=false },
            { type="rule_reference", name="method_call_no_paren", is_token=false },
            { type="sequence", elements={
              { type="rule_reference", name="method_call", is_token=false },
              { type="negative_lookahead", element={ type="literal", value="+" } },
              { type="negative_lookahead", element={ type="literal", value="-" } },
              { type="negative_lookahead", element={ type="rule_reference", name="STAR", is_token=true } },
              { type="negative_lookahead", element={ type="rule_reference", name="SLASH", is_token=true } },
              { type="negative_lookahead", element={ type="literal", value="<<" } },
              { type="negative_lookahead", element={ type="literal", value="==" } },
              { type="negative_lookahead", element={ type="literal", value="!=" } },
              { type="negative_lookahead", element={ type="literal", value="<=" } },
              { type="negative_lookahead", element={ type="literal", value=">=" } },
              { type="negative_lookahead", element={ type="literal", value="<" } },
              { type="negative_lookahead", element={ type="literal", value=">" } },
              { type="negative_lookahead", element={ type="literal", value="&&" } },
              { type="negative_lookahead", element={ type="literal", value="||" } },
              { type="negative_lookahead", element={ type="literal", value="and" } },
              { type="negative_lookahead", element={ type="literal", value="or" } },
              { type="negative_lookahead", element={ type="literal", value="..." } },
              { type="negative_lookahead", element={ type="literal", value=".." } },
              { type="negative_lookahead", element={ type="literal", value="?" } },
            } },
            { type="rule_reference", name="expression_stmt", is_token=false },
          } } },
        { type="group", element={ type="alternation", choices={
            { type="literal", value="if_modifier" },
            { type="literal", value="unless_modifier" },
            { type="literal", value="while_modifier" },
            { type="literal", value="until_modifier" },
          } } },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=124,
    },
    {
      name="def_statement",
      body={ type="sequence", elements={
        { type="literal", value="def" },
        { type="optional", element={ type="rule_reference", name="def_receiver", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="( line start" } },
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="optional", element={ type="rule_reference", name="params", is_token=false } },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
        { type="repetition", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="rescue" } },
            { type="negative_lookahead", element={ type="literal", value="ensure" } },
            { type="negative_lookahead", element={ type="literal", value="end" } },
            { type="rule_reference", name="statement", is_token=false },
          } } },
        { type="repetition", element={ type="rule_reference", name="rescue_clause", is_token=false } },
        { type="optional", element={ type="rule_reference", name="ensure_clause", is_token=false } },
        { type="literal", value="end" },
      } },
      line_number=148,
    },
    {
      name="def_receiver",
      body={ type="sequence", elements={
        { type="rule_reference", name="singleton_receiver", is_token=false },
        { type="literal", value="." },
      } },
      line_number=154,
    },
    {
      name="endless_def_statement",
      body={ type="sequence", elements={
        { type="literal", value="def" },
        { type="optional", element={ type="rule_reference", name="def_receiver", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="optional", element={ type="rule_reference", name="params", is_token=false } },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=163,
    },
    {
      name="class_statement",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="class" },
          { type="literal", value="<<" },
          { type="rule_reference", name="singleton_receiver", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="negative_lookahead", element={ type="literal", value="end" } },
              { type="rule_reference", name="statement", is_token=false },
            } } },
          { type="literal", value="end" },
        } },
        { type="sequence", elements={
          { type="literal", value="class" },
          { type="rule_reference", name="NAME", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="literal", value="<" },
              { type="rule_reference", name="NAME", is_token=true },
            } } },
          { type="repetition", element={ type="sequence", elements={
              { type="negative_lookahead", element={ type="literal", value="end" } },
              { type="rule_reference", name="statement", is_token=false },
            } } },
          { type="literal", value="end" },
        } },
      } },
      line_number=184,
    },
    {
      name="singleton_receiver",
      body={ type="alternation", choices={
        { type="literal", value="self" },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=186,
    },
    {
      name="module_statement",
      body={ type="sequence", elements={
        { type="literal", value="module" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="end" } },
            { type="rule_reference", name="statement", is_token=false },
          } } },
        { type="literal", value="end" },
      } },
      line_number=187,
    },
    {
      name="method_with_block",
      body={ type="sequence", elements={
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="KEYWORD", is_token=true },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="( line start" } },
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="optional", element={ type="sequence", elements={
                { type="rule_reference", name="expression", is_token=false },
                { type="repetition", element={ type="sequence", elements={
                    { type="rule_reference", name="COMMA", is_token=true },
                    { type="rule_reference", name="expression", is_token=false },
                  } } },
              } } },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=189,
    },
    {
      name="block",
      body={ type="alternation", choices={
        { type="rule_reference", name="do_block", is_token=false },
        { type="rule_reference", name="brace_block", is_token=false },
      } },
      line_number=190,
    },
    {
      name="do_block",
      body={ type="sequence", elements={
        { type="literal", value="do" },
        { type="optional", element={ type="rule_reference", name="block_params", is_token=false } },
        { type="repetition", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="end" } },
            { type="rule_reference", name="statement", is_token=false },
          } } },
        { type="literal", value="end" },
      } },
      line_number=191,
    },
    {
      name="brace_block",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="rule_reference", name="block_params", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=192,
    },
    {
      name="block_params",
      body={ type="sequence", elements={
        { type="literal", value="|" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value=";" },
            { type="rule_reference", name="NAME", is_token=true },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="NAME", is_token=true },
              } } },
          } } },
        { type="literal", value="|" },
      } },
      line_number=202,
    },
    {
      name="return_statement",
      body={ type="sequence", elements={
        { type="literal", value="return" },
        { type="group", element={ type="alternation", choices={
            { type="literal", value=";" },
            { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
          } } },
      } },
      line_number=209,
    },
    {
      name="break_statement",
      body={ type="sequence", elements={
        { type="literal", value="break" },
        { type="group", element={ type="alternation", choices={
            { type="literal", value=";" },
            { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
          } } },
      } },
      line_number=210,
    },
    {
      name="next_statement",
      body={ type="sequence", elements={
        { type="literal", value="next" },
        { type="group", element={ type="alternation", choices={
            { type="literal", value=";" },
            { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
          } } },
      } },
      line_number=211,
    },
    {
      name="redo_statement",
      body={ type="literal", value="redo" },
      line_number=215,
    },
    {
      name="retry_statement",
      body={ type="literal", value="retry" },
      line_number=219,
    },
    {
      name="alias_statement",
      body={ type="sequence", elements={
        { type="literal", value="alias" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=230,
    },
    {
      name="undef_statement",
      body={ type="sequence", elements={
        { type="literal", value="undef" },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=242,
    },
    {
      name="yield_statement",
      body={ type="sequence", elements={
        { type="literal", value="yield" },
        { type="group", element={ type="alternation", choices={
            { type="literal", value=";" },
            { type="optional", element={ type="rule_reference", name="yield_args", is_token=false } },
          } } },
      } },
      line_number=264,
    },
    {
      name="yield_args",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="call_arg", is_token=false },
              { type="repetition", element={ type="sequence", elements={
                  { type="rule_reference", name="COMMA", is_token=true },
                  { type="rule_reference", name="call_arg", is_token=false },
                } } },
            } } },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="call_arg", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="call_arg", is_token=false },
            } } },
        } },
      } },
      line_number=265,
    },
    {
      name="super_args",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="call_arg", is_token=false },
              { type="repetition", element={ type="sequence", elements={
                  { type="rule_reference", name="COMMA", is_token=true },
                  { type="rule_reference", name="call_arg", is_token=false },
                } } },
            } } },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="call_arg", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="call_arg", is_token=false },
            } } },
        } },
      } },
      line_number=292,
    },
    {
      name="params",
      body={ type="alternation", choices={
        { type="literal", value="..." },
        { type="sequence", elements={
          { type="rule_reference", name="param", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="param", is_token=false },
            } } },
        } },
      } },
      line_number=321,
    },
    {
      name="param",
      body={ type="sequence", elements={
        { type="optional", element={ type="alternation", choices={
            { type="literal", value="*" },
            { type="literal", value="**" },
          } } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="alternation", choices={
            { type="sequence", elements={
              { type="rule_reference", name="COLON", is_token=true },
              { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
            } },
            { type="sequence", elements={
              { type="rule_reference", name="EQUALS", is_token=true },
              { type="rule_reference", name="expression", is_token=false },
            } },
          } } },
      } },
      line_number=366,
    },
    {
      name="if_statement",
      body={ type="sequence", elements={
        { type="literal", value="if" },
        { type="rule_reference", name="expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="else" } },
            { type="negative_lookahead", element={ type="literal", value="elsif" } },
            { type="negative_lookahead", element={ type="literal", value="end" } },
            { type="rule_reference", name="statement", is_token=false },
          } } },
        { type="repetition", element={ type="rule_reference", name="elsif_clause", is_token=false } },
        { type="optional", element={ type="rule_reference", name="else_clause", is_token=false } },
        { type="literal", value="end" },
      } },
      line_number=367,
    },
    {
      name="elsif_clause",
      body={ type="sequence", elements={
        { type="literal", value="elsif" },
        { type="rule_reference", name="expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="else" } },
            { type="negative_lookahead", element={ type="literal", value="elsif" } },
            { type="negative_lookahead", element={ type="literal", value="end" } },
            { type="rule_reference", name="statement", is_token=false },
          } } },
      } },
      line_number=368,
    },
    {
      name="else_clause",
      body={ type="sequence", elements={
        { type="literal", value="else" },
        { type="repetition", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="end" } },
            { type="rule_reference", name="statement", is_token=false },
          } } },
      } },
      line_number=369,
    },
    {
      name="unless_statement",
      body={ type="sequence", elements={
        { type="literal", value="unless" },
        { type="rule_reference", name="expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="else" } },
            { type="negative_lookahead", element={ type="literal", value="end" } },
            { type="rule_reference", name="statement", is_token=false },
          } } },
        { type="optional", element={ type="rule_reference", name="else_clause", is_token=false } },
        { type="literal", value="end" },
      } },
      line_number=370,
    },
    {
      name="while_statement",
      body={ type="sequence", elements={
        { type="literal", value="while" },
        { type="rule_reference", name="expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="end" } },
            { type="rule_reference", name="statement", is_token=false },
          } } },
        { type="literal", value="end" },
      } },
      line_number=371,
    },
    {
      name="until_statement",
      body={ type="sequence", elements={
        { type="literal", value="until" },
        { type="rule_reference", name="expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="end" } },
            { type="rule_reference", name="statement", is_token=false },
          } } },
        { type="literal", value="end" },
      } },
      line_number=372,
    },
    {
      name="for_statement",
      body={ type="sequence", elements={
        { type="literal", value="for" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="in" },
        { type="rule_reference", name="expression", is_token=false },
        { type="optional", element={ type="literal", value="do" } },
        { type="repetition", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="end" } },
            { type="rule_reference", name="statement", is_token=false },
          } } },
        { type="literal", value="end" },
      } },
      line_number=380,
    },
    {
      name="case_statement",
      body={ type="sequence", elements={
        { type="literal", value="case" },
        { type="rule_reference", name="expression", is_token=false },
        { type="repetition", element={ type="alternation", choices={
            { type="rule_reference", name="when_clause", is_token=false },
            { type="rule_reference", name="in_clause", is_token=false },
          } } },
        { type="optional", element={ type="rule_reference", name="else_clause", is_token=false } },
        { type="literal", value="end" },
      } },
      line_number=403,
    },
    {
      name="when_clause",
      body={ type="sequence", elements={
        { type="literal", value="when" },
        { type="rule_reference", name="expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
          } } },
        { type="repetition", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="when" } },
            { type="negative_lookahead", element={ type="literal", value="in" } },
            { type="negative_lookahead", element={ type="literal", value="else" } },
            { type="negative_lookahead", element={ type="literal", value="end" } },
            { type="rule_reference", name="statement", is_token=false },
          } } },
      } },
      line_number=404,
    },
    {
      name="in_clause",
      body={ type="sequence", elements={
        { type="literal", value="in" },
        { type="rule_reference", name="pattern", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="when" } },
            { type="negative_lookahead", element={ type="literal", value="in" } },
            { type="negative_lookahead", element={ type="literal", value="else" } },
            { type="negative_lookahead", element={ type="literal", value="end" } },
            { type="rule_reference", name="statement", is_token=false },
          } } },
      } },
      line_number=426,
    },
    {
      name="pattern",
      body={ type="alternation", choices={
        { type="rule_reference", name="array_pattern", is_token=false },
        { type="rule_reference", name="hash_pattern", is_token=false },
        { type="rule_reference", name="class_pattern", is_token=false },
        { type="rule_reference", name="pin_pattern", is_token=false },
        { type="rule_reference", name="literal_pattern", is_token=false },
        { type="rule_reference", name="binding_pattern", is_token=false },
      } },
      line_number=427,
    },
    {
      name="literal_pattern",
      body={ type="alternation", choices={
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="symbol_literal", is_token=false },
        { type="rule_reference", name="KEYWORD", is_token=true },
      } },
      line_number=428,
    },
    {
      name="binding_pattern",
      body={ type="rule_reference", name="NAME", is_token=true },
      line_number=429,
    },
    {
      name="array_pattern",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="splat_pattern", is_token=false },
                { type="rule_reference", name="pattern", is_token=false },
              } } },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="group", element={ type="alternation", choices={
                    { type="rule_reference", name="splat_pattern", is_token=false },
                    { type="rule_reference", name="pattern", is_token=false },
                  } } },
              } } },
          } } },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=430,
    },
    {
      name="hash_pattern",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="hash_pattern_pair", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="hash_pattern_pair", is_token=false },
              } } },
          } } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=431,
    },
    {
      name="hash_pattern_pair",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="optional", element={ type="rule_reference", name="pattern", is_token=false } },
      } },
      line_number=432,
    },
    {
      name="splat_pattern",
      body={ type="sequence", elements={
        { type="literal", value="*" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
      } },
      line_number=439,
    },
    {
      name="pin_pattern",
      body={ type="sequence", elements={
        { type="literal", value="^" },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=444,
    },
    {
      name="class_pattern",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="pattern", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="pattern", is_token=false },
              } } },
          } } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=450,
    },
    {
      name="begin_statement",
      body={ type="sequence", elements={
        { type="literal", value="begin" },
        { type="repetition", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="rescue" } },
            { type="negative_lookahead", element={ type="literal", value="ensure" } },
            { type="negative_lookahead", element={ type="literal", value="end" } },
            { type="rule_reference", name="statement", is_token=false },
          } } },
        { type="repetition", element={ type="rule_reference", name="rescue_clause", is_token=false } },
        { type="optional", element={ type="rule_reference", name="ensure_clause", is_token=false } },
        { type="literal", value="end" },
      } },
      line_number=471,
    },
    {
      name="rescue_clause",
      body={ type="sequence", elements={
        { type="literal", value="rescue" },
        { type="optional", element={ type="alternation", choices={
            { type="sequence", elements={
              { type="literal", value="=>" },
              { type="rule_reference", name="NAME", is_token=true },
            } },
            { type="sequence", elements={
              { type="rule_reference", name="exception_list", is_token=false },
              { type="group", element={ type="alternation", choices={
                  { type="sequence", elements={
                    { type="literal", value="=>" },
                    { type="rule_reference", name="NAME", is_token=true },
                  } },
                  { type="literal", value=";" },
                } } },
            } },
            { type="literal", value=";" },
          } } },
        { type="repetition", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="rescue" } },
            { type="negative_lookahead", element={ type="literal", value="ensure" } },
            { type="negative_lookahead", element={ type="literal", value="end" } },
            { type="rule_reference", name="statement", is_token=false },
          } } },
      } },
      line_number=491,
    },
    {
      name="exception_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
      } },
      line_number=492,
    },
    {
      name="ensure_clause",
      body={ type="sequence", elements={
        { type="literal", value="ensure" },
        { type="repetition", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="end" } },
            { type="rule_reference", name="statement", is_token=false },
          } } },
      } },
      line_number=493,
    },
    {
      name="index_write_receiver_postfix",
      body={ type="alternation", choices={
        { type="rule_reference", name="dot_call", is_token=false },
        { type="rule_reference", name="scope_resolution", is_token=false },
        { type="rule_reference", name="index_suffix", is_token=false },
      } },
      line_number=546,
    },
    {
      name="index_assignment",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="index_write_receiver_postfix", is_token=false },
            { type="positive_lookahead", element={ type="rule_reference", name="index_write_receiver_postfix", is_token=false } },
          } } },
        { type="rule_reference", name="index_suffix", is_token=false },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=547,
    },
    {
      name="assignment",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="EQUALS", is_token=true },
            { type="literal", value="+=" },
            { type="literal", value="-=" },
            { type="literal", value="*=" },
            { type="literal", value="/=" },
            { type="literal", value="%=" },
            { type="literal", value="**=" },
            { type="literal", value="<<=" },
            { type="literal", value=">>=" },
            { type="literal", value="&=" },
            { type="literal", value="|=" },
            { type="literal", value="^=" },
            { type="literal", value="||=" },
            { type="literal", value="&&=" },
          } } },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=548,
    },
    {
      name="rightward_assignment",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="literal", value="=>" },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=567,
    },
    {
      name="method_call",
      body={ type="sequence", elements={
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="NAME", is_token=true },
            { type="sequence", elements={
              { type="negative_lookahead", element={ type="literal", value="super" } },
              { type="rule_reference", name="KEYWORD", is_token=true },
            } },
          } } },
        { type="negative_lookahead", element={ type="literal", value="( line start" } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="call_arg", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="call_arg", is_token=false },
              } } },
          } } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="repetition", element={ type="rule_reference", name="dot_call", is_token=false } },
      } },
      line_number=588,
    },
    {
      name="dot_call",
      body={ type="sequence", elements={
        { type="literal", value="." },
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="KEYWORD", is_token=true },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="( line start" } },
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="optional", element={ type="sequence", elements={
                { type="rule_reference", name="call_arg", is_token=false },
                { type="repetition", element={ type="sequence", elements={
                    { type="rule_reference", name="COMMA", is_token=true },
                    { type="rule_reference", name="call_arg", is_token=false },
                  } } },
              } } },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
        { type="optional", element={ type="rule_reference", name="block", is_token=false } },
      } },
      line_number=589,
    },
    {
      name="scope_resolution",
      body={ type="sequence", elements={
        { type="literal", value="::" },
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="KEYWORD", is_token=true },
          } } },
      } },
      line_number=597,
    },
    {
      name="call_arg",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="NAME", is_token=true },
          { type="rule_reference", name="COLON", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="optional", element={ type="alternation", choices={
              { type="literal", value="*" },
              { type="literal", value="**" },
              { type="literal", value="&" },
            } } },
          { type="rule_reference", name="expression", is_token=false },
        } },
      } },
      line_number=652,
    },
    {
      name="method_call_no_paren",
      body={ type="sequence", elements={
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="NAME", is_token=true },
            { type="sequence", elements={
              { type="negative_lookahead", element={ type="literal", value="super" } },
              { type="rule_reference", name="KEYWORD", is_token=true },
            } },
          } } },
        { type="negative_lookahead", element={ type="literal", value="<" } },
        { type="negative_lookahead", element={ type="literal", value=">" } },
        { type="negative_lookahead", element={ type="literal", value="<=" } },
        { type="negative_lookahead", element={ type="literal", value=">=" } },
        { type="negative_lookahead", element={ type="literal", value="!=" } },
        { type="negative_lookahead", element={ type="literal", value="&&" } },
        { type="negative_lookahead", element={ type="literal", value="||" } },
        { type="negative_lookahead", element={ type="literal", value="<<" } },
        { type="negative_lookahead", element={ type="literal", value="[ line start" } },
        { type="negative_lookahead", element={ type="literal", value="( line start" } },
        { type="negative_lookahead", element={ type="literal", value="- line start" } },
        { type="negative_lookahead", element={ type="literal", value="+ line start" } },
        { type="rule_reference", name="expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
          } } },
      } },
      line_number=706,
    },
    {
      name="expression_stmt",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="optional", element={ type="literal", value=";" } },
      } },
      line_number=710,
    },
    {
      name="expression",
      body={ type="rule_reference", name="ternary", is_token=false },
      line_number=817,
    },
    {
      name="ternary",
      body={ type="sequence", elements={
        { type="rule_reference", name="range", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="?" },
            { type="rule_reference", name="expression", is_token=false },
            { type="literal", value=":" },
            { type="rule_reference", name="expression", is_token=false },
          } } },
      } },
      line_number=818,
    },
    {
      name="range",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="group", element={ type="alternation", choices={
              { type="literal", value="..." },
              { type="literal", value=".." },
            } } },
          { type="rule_reference", name="logical_or", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="logical_or", is_token=false },
          { type="optional", element={ type="sequence", elements={
              { type="group", element={ type="alternation", choices={
                  { type="literal", value="..." },
                  { type="literal", value=".." },
                } } },
              { type="optional", element={ type="rule_reference", name="logical_or", is_token=false } },
            } } },
        } },
      } },
      line_number=819,
    },
    {
      name="logical_or",
      body={ type="sequence", elements={
        { type="rule_reference", name="logical_and", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="literal", value="||" },
                { type="literal", value="or" },
              } } },
            { type="rule_reference", name="logical_and", is_token=false },
          } } },
      } },
      line_number=820,
    },
    {
      name="logical_and",
      body={ type="sequence", elements={
        { type="rule_reference", name="logical_not", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="literal", value="&&" },
                { type="literal", value="and" },
              } } },
            { type="rule_reference", name="logical_not", is_token=false },
          } } },
      } },
      line_number=821,
    },
    {
      name="logical_not",
      body={ type="sequence", elements={
        { type="repetition", element={ type="group", element={ type="alternation", choices={
              { type="literal", value="!" },
              { type="literal", value="not" },
            } } } },
        { type="rule_reference", name="comparison", is_token=false },
      } },
      line_number=828,
    },
    {
      name="comparison",
      body={ type="sequence", elements={
        { type="rule_reference", name="shift", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="literal", value="==" },
                { type="literal", value="!=" },
                { type="literal", value="<=" },
                { type="literal", value=">=" },
                { type="literal", value="<" },
                { type="literal", value=">" },
              } } },
            { type="rule_reference", name="shift", is_token=false },
          } } },
      } },
      line_number=844,
    },
    {
      name="shift",
      body={ type="sequence", elements={
        { type="rule_reference", name="sum", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="<<" },
            { type="rule_reference", name="sum", is_token=false },
          } } },
      } },
      line_number=845,
    },
    {
      name="sum",
      body={ type="sequence", elements={
        { type="rule_reference", name="term", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="- line start" } },
            { type="negative_lookahead", element={ type="literal", value="+ line start" } },
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="PLUS", is_token=true },
                { type="rule_reference", name="MINUS", is_token=true },
              } } },
            { type="rule_reference", name="term", is_token=false },
          } } },
      } },
      line_number=850,
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
      line_number=851,
    },
    {
      name="super_expr",
      body={ type="sequence", elements={
        { type="literal", value="super" },
        { type="group", element={ type="alternation", choices={
            { type="literal", value=";" },
            { type="optional", element={ type="rule_reference", name="super_args", is_token=false } },
          } } },
      } },
      line_number=923,
    },
    {
      name="index_suffix",
      body={ type="sequence", elements={
        { type="negative_lookahead", element={ type="literal", value="[ line start" } },
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=951,
    },
    {
      name="factor",
      body={ type="sequence", elements={
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="defined_expression", is_token=false },
            { type="rule_reference", name="lambda_literal", is_token=false },
            { type="rule_reference", name="super_expr", is_token=false },
            { type="rule_reference", name="method_call", is_token=false },
            { type="rule_reference", name="NUMBER", is_token=true },
            { type="rule_reference", name="STRING", is_token=true },
            { type="rule_reference", name="NAME", is_token=true },
            { type="group", element={ type="sequence", elements={
                { type="negative_lookahead", element={ type="literal", value="end" } },
                { type="negative_lookahead", element={ type="literal", value="rescue" } },
                { type="negative_lookahead", element={ type="literal", value="ensure" } },
                { type="negative_lookahead", element={ type="literal", value="else" } },
                { type="negative_lookahead", element={ type="literal", value="elsif" } },
                { type="negative_lookahead", element={ type="literal", value="when" } },
                { type="negative_lookahead", element={ type="literal", value="then" } },
                { type="negative_lookahead", element={ type="literal", value="in" } },
                { type="negative_lookahead", element={ type="literal", value="do" } },
                { type="negative_lookahead", element={ type="literal", value="if_modifier" } },
                { type="negative_lookahead", element={ type="literal", value="unless_modifier" } },
                { type="negative_lookahead", element={ type="literal", value="while_modifier" } },
                { type="negative_lookahead", element={ type="literal", value="until_modifier" } },
                { type="rule_reference", name="KEYWORD", is_token=true },
              } } },
            { type="rule_reference", name="symbol_literal", is_token=false },
            { type="rule_reference", name="array_literal", is_token=false },
            { type="rule_reference", name="hash_literal", is_token=false },
            { type="sequence", elements={
              { type="rule_reference", name="LPAREN", is_token=true },
              { type="rule_reference", name="expression", is_token=false },
              { type="rule_reference", name="RPAREN", is_token=true },
            } },
            { type="rule_reference", name="unary_minus", is_token=false },
            { type="rule_reference", name="unary_plus", is_token=false },
          } } },
        { type="repetition", element={ type="alternation", choices={
            { type="rule_reference", name="dot_call", is_token=false },
            { type="rule_reference", name="scope_resolution", is_token=false },
            { type="rule_reference", name="index_suffix", is_token=false },
          } } },
      } },
      line_number=955,
    },
    {
      name="lambda_literal",
      body={ type="sequence", elements={
        { type="literal", value="->" },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="optional", element={ type="rule_reference", name="params", is_token=false } },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=974,
    },
    {
      name="unary_minus",
      body={ type="sequence", elements={
        { type="rule_reference", name="MINUS", is_token=true },
        { type="rule_reference", name="factor", is_token=false },
      } },
      line_number=975,
    },
    {
      name="unary_plus",
      body={ type="sequence", elements={
        { type="literal", value="+ line start" },
        { type="rule_reference", name="factor", is_token=false },
      } },
      line_number=982,
    },
    {
      name="defined_expression",
      body={ type="sequence", elements={
        { type="literal", value="defined?" },
        { type="rule_reference", name="factor", is_token=false },
      } },
      line_number=993,
    },
    {
      name="symbol_literal",
      body={ type="sequence", elements={
        { type="literal", value=":" },
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="KEYWORD", is_token=true },
            { type="rule_reference", name="STRING", is_token=true },
          } } },
      } },
      line_number=1000,
    },
    {
      name="array_literal",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="expression", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="expression", is_token=false },
              } } },
          } } },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=1001,
    },
    {
      name="hash_literal",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="hash_entry", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="hash_entry", is_token=false },
              } } },
          } } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=1002,
    },
    {
      name="hash_entry",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="NAME", is_token=true },
          { type="rule_reference", name="COLON", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="NAME", is_token=true },
          { type="rule_reference", name="COLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="expression", is_token=false },
          { type="literal", value="=>" },
          { type="rule_reference", name="expression", is_token=false },
        } },
      } },
      line_number=1003,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
