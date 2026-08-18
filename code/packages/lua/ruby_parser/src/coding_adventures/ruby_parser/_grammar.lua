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
        { type="rule_reference", name="method_call", is_token=false },
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
      line_number=71,
    },
    {
      name="mlhs_target",
      body={ type="sequence", elements={
        { type="optional", element={ type="literal", value="*" } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=72,
    },
    {
      name="modifier_statement",
      body={ type="sequence", elements={
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="assignment", is_token=false },
            { type="rule_reference", name="method_call_no_paren", is_token=false },
            { type="rule_reference", name="method_call", is_token=false },
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
      line_number=108,
    },
    {
      name="def_statement",
      body={ type="sequence", elements={
        { type="literal", value="def" },
        { type="optional", element={ type="rule_reference", name="def_receiver", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
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
      line_number=132,
    },
    {
      name="def_receiver",
      body={ type="sequence", elements={
        { type="rule_reference", name="singleton_receiver", is_token=false },
        { type="literal", value="." },
      } },
      line_number=138,
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
      line_number=147,
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
      line_number=168,
    },
    {
      name="singleton_receiver",
      body={ type="alternation", choices={
        { type="literal", value="self" },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=170,
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
      line_number=171,
    },
    {
      name="method_with_block",
      body={ type="sequence", elements={
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="KEYWORD", is_token=true },
          } } },
        { type="optional", element={ type="sequence", elements={
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
      line_number=173,
    },
    {
      name="block",
      body={ type="alternation", choices={
        { type="rule_reference", name="do_block", is_token=false },
        { type="rule_reference", name="brace_block", is_token=false },
      } },
      line_number=174,
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
      line_number=175,
    },
    {
      name="brace_block",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="rule_reference", name="block_params", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=176,
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
      line_number=186,
    },
    {
      name="return_statement",
      body={ type="sequence", elements={
        { type="literal", value="return" },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
      } },
      line_number=188,
    },
    {
      name="break_statement",
      body={ type="sequence", elements={
        { type="literal", value="break" },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
      } },
      line_number=189,
    },
    {
      name="next_statement",
      body={ type="sequence", elements={
        { type="literal", value="next" },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
      } },
      line_number=190,
    },
    {
      name="redo_statement",
      body={ type="literal", value="redo" },
      line_number=194,
    },
    {
      name="retry_statement",
      body={ type="literal", value="retry" },
      line_number=198,
    },
    {
      name="alias_statement",
      body={ type="sequence", elements={
        { type="literal", value="alias" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=209,
    },
    {
      name="undef_statement",
      body={ type="sequence", elements={
        { type="literal", value="undef" },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=221,
    },
    {
      name="yield_statement",
      body={ type="sequence", elements={
        { type="literal", value="yield" },
        { type="optional", element={ type="rule_reference", name="yield_args", is_token=false } },
      } },
      line_number=243,
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
      line_number=244,
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
      line_number=271,
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
      line_number=300,
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
      line_number=345,
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
      line_number=346,
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
      line_number=347,
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
      line_number=348,
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
      line_number=349,
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
      line_number=350,
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
      line_number=351,
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
      line_number=374,
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
      line_number=375,
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
      line_number=397,
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
      line_number=398,
    },
    {
      name="literal_pattern",
      body={ type="alternation", choices={
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="symbol_literal", is_token=false },
        { type="rule_reference", name="KEYWORD", is_token=true },
      } },
      line_number=399,
    },
    {
      name="binding_pattern",
      body={ type="rule_reference", name="NAME", is_token=true },
      line_number=400,
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
      line_number=401,
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
      line_number=402,
    },
    {
      name="hash_pattern_pair",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="optional", element={ type="rule_reference", name="pattern", is_token=false } },
      } },
      line_number=403,
    },
    {
      name="splat_pattern",
      body={ type="sequence", elements={
        { type="literal", value="*" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
      } },
      line_number=410,
    },
    {
      name="pin_pattern",
      body={ type="sequence", elements={
        { type="literal", value="^" },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=415,
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
      line_number=421,
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
      line_number=442,
    },
    {
      name="rescue_clause",
      body={ type="sequence", elements={
        { type="literal", value="rescue" },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="exception_list", is_token=false },
            { type="literal", value="=>" },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
        { type="repetition", element={ type="sequence", elements={
            { type="negative_lookahead", element={ type="literal", value="rescue" } },
            { type="negative_lookahead", element={ type="literal", value="ensure" } },
            { type="negative_lookahead", element={ type="literal", value="end" } },
            { type="rule_reference", name="statement", is_token=false },
          } } },
      } },
      line_number=451,
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
      line_number=452,
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
      line_number=453,
    },
    {
      name="index_write_receiver_postfix",
      body={ type="alternation", choices={
        { type="rule_reference", name="dot_call", is_token=false },
        { type="rule_reference", name="scope_resolution", is_token=false },
        { type="rule_reference", name="index_suffix", is_token=false },
      } },
      line_number=506,
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
      line_number=507,
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
      line_number=508,
    },
    {
      name="rightward_assignment",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="literal", value="=>" },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=527,
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
      line_number=544,
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
      line_number=545,
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
      line_number=553,
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
      line_number=608,
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
        { type="rule_reference", name="expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
          } } },
      } },
      line_number=656,
    },
    {
      name="expression_stmt",
      body={ type="rule_reference", name="expression", is_token=false },
      line_number=659,
    },
    {
      name="expression",
      body={ type="rule_reference", name="ternary", is_token=false },
      line_number=766,
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
      line_number=767,
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
      line_number=768,
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
      line_number=769,
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
      line_number=770,
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
      line_number=777,
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
      line_number=793,
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
      line_number=794,
    },
    {
      name="sum",
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
      line_number=795,
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
      line_number=796,
    },
    {
      name="super_expr",
      body={ type="sequence", elements={
        { type="literal", value="super" },
        { type="optional", element={ type="rule_reference", name="super_args", is_token=false } },
      } },
      line_number=865,
    },
    {
      name="index_suffix",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=877,
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
          } } },
        { type="repetition", element={ type="alternation", choices={
            { type="rule_reference", name="dot_call", is_token=false },
            { type="rule_reference", name="scope_resolution", is_token=false },
            { type="rule_reference", name="index_suffix", is_token=false },
          } } },
      } },
      line_number=878,
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
      line_number=897,
    },
    {
      name="unary_minus",
      body={ type="sequence", elements={
        { type="rule_reference", name="MINUS", is_token=true },
        { type="rule_reference", name="factor", is_token=false },
      } },
      line_number=898,
    },
    {
      name="defined_expression",
      body={ type="sequence", elements={
        { type="literal", value="defined?" },
        { type="rule_reference", name="factor", is_token=false },
      } },
      line_number=909,
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
      line_number=916,
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
      line_number=917,
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
      line_number=918,
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
      line_number=919,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
