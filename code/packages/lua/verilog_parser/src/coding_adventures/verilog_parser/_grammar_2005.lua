-- AUTO-GENERATED FILE - DO NOT EDIT
-- Source: verilog2005.grammar
-- Regenerate with: grammar-tools compile-grammar verilog2005.grammar
--
-- This file embeds a ParserGrammar as native Lua data structures.
-- Call parser_grammar() instead of reading and parsing the .grammar file.

local gt = require("coding_adventures.grammar_tools")

local function parser_grammar()
  local g = gt.ParserGrammar.new()
  g.rules = {
    {
      name="source_text",
      body={ type="repetition", element={ type="rule_reference", name="description", is_token=false } },
      line_number=42,
    },
    {
      name="description",
      body={ type="rule_reference", name="module_declaration", is_token=false },
      line_number=44,
    },
    {
      name="module_declaration",
      body={ type="sequence", elements={
        { type="literal", value="module" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="parameter_port_list", is_token=false } },
        { type="optional", element={ type="rule_reference", name="port_list", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
        { type="repetition", element={ type="rule_reference", name="module_item", is_token=false } },
        { type="literal", value="endmodule" },
      } },
      line_number=73,
    },
    {
      name="parameter_port_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="HASH", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="parameter_declaration", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="parameter_declaration", is_token=false },
          } } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=91,
    },
    {
      name="parameter_declaration",
      body={ type="sequence", elements={
        { type="literal", value="parameter" },
        { type="optional", element={ type="rule_reference", name="range", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=94,
    },
    {
      name="localparam_declaration",
      body={ type="sequence", elements={
        { type="literal", value="localparam" },
        { type="optional", element={ type="rule_reference", name="range", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=95,
    },
    {
      name="port_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="port", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="port", is_token=false },
          } } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=115,
    },
    {
      name="port",
      body={ type="sequence", elements={
        { type="optional", element={ type="rule_reference", name="port_direction", is_token=false } },
        { type="optional", element={ type="rule_reference", name="net_type", is_token=false } },
        { type="optional", element={ type="literal", value="signed" } },
        { type="optional", element={ type="rule_reference", name="range", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=117,
    },
    {
      name="port_direction",
      body={ type="alternation", choices={
        { type="literal", value="input" },
        { type="literal", value="output" },
        { type="literal", value="inout" },
      } },
      line_number=119,
    },
    {
      name="net_type",
      body={ type="alternation", choices={
        { type="literal", value="wire" },
        { type="literal", value="reg" },
        { type="literal", value="tri" },
        { type="literal", value="supply0" },
        { type="literal", value="supply1" },
      } },
      line_number=120,
    },
    {
      name="range",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=122,
    },
    {
      name="module_item",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="port_declaration", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="net_declaration", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="reg_declaration", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="integer_declaration", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="parameter_declaration", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="localparam_declaration", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="rule_reference", name="continuous_assign", is_token=false },
        { type="rule_reference", name="always_construct", is_token=false },
        { type="rule_reference", name="initial_construct", is_token=false },
        { type="rule_reference", name="module_instantiation", is_token=false },
        { type="rule_reference", name="generate_region", is_token=false },
        { type="rule_reference", name="function_declaration", is_token=false },
        { type="rule_reference", name="task_declaration", is_token=false },
      } },
      line_number=139,
    },
    {
      name="port_declaration",
      body={ type="sequence", elements={
        { type="rule_reference", name="port_direction", is_token=false },
        { type="optional", element={ type="rule_reference", name="net_type", is_token=false } },
        { type="optional", element={ type="literal", value="signed" } },
        { type="optional", element={ type="rule_reference", name="range", is_token=false } },
        { type="rule_reference", name="name_list", is_token=false },
      } },
      line_number=174,
    },
    {
      name="net_declaration",
      body={ type="sequence", elements={
        { type="rule_reference", name="net_type", is_token=false },
        { type="optional", element={ type="literal", value="signed" } },
        { type="optional", element={ type="rule_reference", name="range", is_token=false } },
        { type="rule_reference", name="name_list", is_token=false },
      } },
      line_number=176,
    },
    {
      name="reg_declaration",
      body={ type="sequence", elements={
        { type="literal", value="reg" },
        { type="optional", element={ type="literal", value="signed" } },
        { type="optional", element={ type="rule_reference", name="range", is_token=false } },
        { type="rule_reference", name="name_list", is_token=false },
      } },
      line_number=177,
    },
    {
      name="integer_declaration",
      body={ type="sequence", elements={
        { type="literal", value="integer" },
        { type="rule_reference", name="name_list", is_token=false },
      } },
      line_number=178,
    },
    {
      name="name_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
      } },
      line_number=179,
    },
    {
      name="continuous_assign",
      body={ type="sequence", elements={
        { type="literal", value="assign" },
        { type="rule_reference", name="assignment", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="assignment", is_token=false },
          } } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=198,
    },
    {
      name="assignment",
      body={ type="sequence", elements={
        { type="rule_reference", name="lvalue", is_token=false },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=199,
    },
    {
      name="lvalue",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="NAME", is_token=true },
          { type="optional", element={ type="rule_reference", name="range_select", is_token=false } },
        } },
        { type="rule_reference", name="concatenation", is_token=false },
      } },
      line_number=203,
    },
    {
      name="range_select",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
          } } },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=206,
    },
    {
      name="always_construct",
      body={ type="sequence", elements={
        { type="literal", value="always" },
        { type="rule_reference", name="AT", is_token=true },
        { type="rule_reference", name="sensitivity_list", is_token=false },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=243,
    },
    {
      name="initial_construct",
      body={ type="sequence", elements={
        { type="literal", value="initial" },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=244,
    },
    {
      name="sensitivity_list",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="sensitivity_item", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="group", element={ type="alternation", choices={
                  { type="literal", value="or" },
                  { type="rule_reference", name="COMMA", is_token=true },
                } } },
              { type="rule_reference", name="sensitivity_item", is_token=false },
            } } },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="STAR", is_token=true },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
      } },
      line_number=246,
    },
    {
      name="sensitivity_item",
      body={ type="sequence", elements={
        { type="optional", element={ type="alternation", choices={
            { type="literal", value="posedge" },
            { type="literal", value="negedge" },
          } } },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=250,
    },
    {
      name="statement",
      body={ type="alternation", choices={
        { type="rule_reference", name="block_statement", is_token=false },
        { type="rule_reference", name="if_statement", is_token=false },
        { type="rule_reference", name="case_statement", is_token=false },
        { type="rule_reference", name="for_statement", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="blocking_assignment", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="nonblocking_assignment", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="task_call", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=259,
    },
    {
      name="block_statement",
      body={ type="sequence", elements={
        { type="literal", value="begin" },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
        { type="literal", value="end" },
      } },
      line_number=275,
    },
    {
      name="if_statement",
      body={ type="sequence", elements={
        { type="literal", value="if" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="else" },
            { type="rule_reference", name="statement", is_token=false },
          } } },
      } },
      line_number=286,
    },
    {
      name="case_statement",
      body={ type="sequence", elements={
        { type="group", element={ type="alternation", choices={
            { type="literal", value="case" },
            { type="literal", value="casex" },
            { type="literal", value="casez" },
          } } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="repetition", element={ type="rule_reference", name="case_item", is_token=false } },
        { type="literal", value="endcase" },
      } },
      line_number=301,
    },
    {
      name="case_item",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="expression_list", is_token=false },
          { type="rule_reference", name="COLON", is_token=true },
          { type="rule_reference", name="statement", is_token=false },
        } },
        { type="sequence", elements={
          { type="literal", value="default" },
          { type="optional", element={ type="rule_reference", name="COLON", is_token=true } },
          { type="rule_reference", name="statement", is_token=false },
        } },
      } },
      line_number=306,
    },
    {
      name="expression_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
          } } },
      } },
      line_number=309,
    },
    {
      name="for_statement",
      body={ type="sequence", elements={
        { type="literal", value="for" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="blocking_assignment", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
        { type="rule_reference", name="blocking_assignment", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=313,
    },
    {
      name="blocking_assignment",
      body={ type="sequence", elements={
        { type="rule_reference", name="lvalue", is_token=false },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=317,
    },
    {
      name="nonblocking_assignment",
      body={ type="sequence", elements={
        { type="rule_reference", name="lvalue", is_token=false },
        { type="rule_reference", name="LESS_EQUALS", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=318,
    },
    {
      name="task_call",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="expression", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="expression", is_token=false },
              } } },
          } } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=321,
    },
    {
      name="module_instantiation",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="parameter_value_assignment", is_token=false } },
        { type="rule_reference", name="instance", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="instance", is_token=false },
          } } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=340,
    },
    {
      name="parameter_value_assignment",
      body={ type="sequence", elements={
        { type="rule_reference", name="HASH", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
          } } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=343,
    },
    {
      name="instance",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="port_connections", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=345,
    },
    {
      name="port_connections",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="named_port_connection", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="named_port_connection", is_token=false },
            } } },
        } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="expression", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="expression", is_token=false },
              } } },
          } } },
      } },
      line_number=347,
    },
    {
      name="named_port_connection",
      body={ type="sequence", elements={
        { type="rule_reference", name="DOT", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=350,
    },
    {
      name="generate_region",
      body={ type="sequence", elements={
        { type="literal", value="generate" },
        { type="repetition", element={ type="rule_reference", name="generate_item", is_token=false } },
        { type="literal", value="endgenerate" },
      } },
      line_number=377,
    },
    {
      name="generate_item",
      body={ type="alternation", choices={
        { type="rule_reference", name="genvar_declaration", is_token=false },
        { type="rule_reference", name="generate_for", is_token=false },
        { type="rule_reference", name="generate_if", is_token=false },
        { type="rule_reference", name="module_item", is_token=false },
      } },
      line_number=379,
    },
    {
      name="genvar_declaration",
      body={ type="sequence", elements={
        { type="literal", value="genvar" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=384,
    },
    {
      name="generate_for",
      body={ type="sequence", elements={
        { type="literal", value="for" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="genvar_assignment", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
        { type="rule_reference", name="genvar_assignment", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="generate_block", is_token=false },
      } },
      line_number=386,
    },
    {
      name="generate_if",
      body={ type="sequence", elements={
        { type="literal", value="if" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="generate_block", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="else" },
            { type="rule_reference", name="generate_block", is_token=false },
          } } },
      } },
      line_number=390,
    },
    {
      name="generate_block",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="begin" },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COLON", is_token=true },
              { type="rule_reference", name="NAME", is_token=true },
            } } },
          { type="repetition", element={ type="rule_reference", name="generate_item", is_token=false } },
          { type="literal", value="end" },
        } },
        { type="rule_reference", name="generate_item", is_token=false },
      } },
      line_number=393,
    },
    {
      name="genvar_assignment",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=396,
    },
    {
      name="function_declaration",
      body={ type="sequence", elements={
        { type="literal", value="function" },
        { type="optional", element={ type="rule_reference", name="range", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="SEMICOLON", is_token=true },
        { type="repetition", element={ type="rule_reference", name="function_item", is_token=false } },
        { type="rule_reference", name="statement", is_token=false },
        { type="literal", value="endfunction" },
      } },
      line_number=415,
    },
    {
      name="function_item",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="port_declaration", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="reg_declaration", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="integer_declaration", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="parameter_declaration", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
      } },
      line_number=420,
    },
    {
      name="task_declaration",
      body={ type="sequence", elements={
        { type="literal", value="task" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="SEMICOLON", is_token=true },
        { type="repetition", element={ type="rule_reference", name="task_item", is_token=false } },
        { type="rule_reference", name="statement", is_token=false },
        { type="literal", value="endtask" },
      } },
      line_number=425,
    },
    {
      name="task_item",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="port_declaration", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="reg_declaration", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="integer_declaration", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
      } },
      line_number=430,
    },
    {
      name="expression",
      body={ type="rule_reference", name="ternary_expr", is_token=false },
      line_number=458,
    },
    {
      name="ternary_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="or_expr", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="QUESTION", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="ternary_expr", is_token=false },
          } } },
      } },
      line_number=464,
    },
    {
      name="or_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="and_expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="LOGIC_OR", is_token=true },
            { type="rule_reference", name="and_expr", is_token=false },
          } } },
      } },
      line_number=467,
    },
    {
      name="and_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="bit_or_expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="LOGIC_AND", is_token=true },
            { type="rule_reference", name="bit_or_expr", is_token=false },
          } } },
      } },
      line_number=468,
    },
    {
      name="bit_or_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="bit_xor_expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="PIPE", is_token=true },
            { type="rule_reference", name="bit_xor_expr", is_token=false },
          } } },
      } },
      line_number=471,
    },
    {
      name="bit_xor_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="bit_and_expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="CARET", is_token=true },
            { type="rule_reference", name="bit_and_expr", is_token=false },
          } } },
      } },
      line_number=472,
    },
    {
      name="bit_and_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="equality_expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="AMP", is_token=true },
            { type="rule_reference", name="equality_expr", is_token=false },
          } } },
      } },
      line_number=473,
    },
    {
      name="equality_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="relational_expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="EQUALS_EQUALS", is_token=true },
                { type="rule_reference", name="NOT_EQUALS", is_token=true },
                { type="rule_reference", name="CASE_EQ", is_token=true },
                { type="rule_reference", name="CASE_NEQ", is_token=true },
              } } },
            { type="rule_reference", name="relational_expr", is_token=false },
          } } },
      } },
      line_number=477,
    },
    {
      name="relational_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="shift_expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="LESS_THAN", is_token=true },
                { type="rule_reference", name="LESS_EQUALS", is_token=true },
                { type="rule_reference", name="GREATER_THAN", is_token=true },
                { type="rule_reference", name="GREATER_EQUALS", is_token=true },
              } } },
            { type="rule_reference", name="shift_expr", is_token=false },
          } } },
      } },
      line_number=484,
    },
    {
      name="shift_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="additive_expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="LEFT_SHIFT", is_token=true },
                { type="rule_reference", name="RIGHT_SHIFT", is_token=true },
                { type="rule_reference", name="ARITH_LEFT_SHIFT", is_token=true },
                { type="rule_reference", name="ARITH_RIGHT_SHIFT", is_token=true },
              } } },
            { type="rule_reference", name="additive_expr", is_token=false },
          } } },
      } },
      line_number=489,
    },
    {
      name="additive_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="multiplicative_expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="PLUS", is_token=true },
                { type="rule_reference", name="MINUS", is_token=true },
              } } },
            { type="rule_reference", name="multiplicative_expr", is_token=false },
          } } },
      } },
      line_number=494,
    },
    {
      name="multiplicative_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="power_expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="STAR", is_token=true },
                { type="rule_reference", name="SLASH", is_token=true },
                { type="rule_reference", name="PERCENT", is_token=true },
              } } },
            { type="rule_reference", name="power_expr", is_token=false },
          } } },
      } },
      line_number=495,
    },
    {
      name="power_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="unary_expr", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="POWER", is_token=true },
            { type="rule_reference", name="unary_expr", is_token=false },
          } } },
      } },
      line_number=496,
    },
    {
      name="unary_expr",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="group", element={ type="alternation", choices={
              { type="rule_reference", name="PLUS", is_token=true },
              { type="rule_reference", name="MINUS", is_token=true },
              { type="rule_reference", name="BANG", is_token=true },
              { type="rule_reference", name="TILDE", is_token=true },
              { type="rule_reference", name="AMP", is_token=true },
              { type="rule_reference", name="PIPE", is_token=true },
              { type="rule_reference", name="CARET", is_token=true },
              { type="sequence", elements={
                { type="rule_reference", name="TILDE", is_token=true },
                { type="rule_reference", name="AMP", is_token=true },
              } },
              { type="sequence", elements={
                { type="rule_reference", name="TILDE", is_token=true },
                { type="rule_reference", name="PIPE", is_token=true },
              } },
              { type="sequence", elements={
                { type="rule_reference", name="TILDE", is_token=true },
                { type="rule_reference", name="CARET", is_token=true },
              } },
            } } },
          { type="rule_reference", name="unary_expr", is_token=false },
        } },
        { type="rule_reference", name="primary", is_token=false },
      } },
      line_number=508,
    },
    {
      name="primary",
      body={ type="alternation", choices={
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="rule_reference", name="SIZED_NUMBER", is_token=true },
        { type="rule_reference", name="REAL_NUMBER", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="SYSTEM_ID", is_token=true },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="rule_reference", name="concatenation", is_token=false },
        { type="rule_reference", name="replication", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="primary", is_token=false },
          { type="rule_reference", name="LBRACKET", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COLON", is_token=true },
              { type="rule_reference", name="expression", is_token=false },
            } } },
          { type="rule_reference", name="RBRACKET", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="NAME", is_token=true },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="expression", is_token=false },
              { type="repetition", element={ type="sequence", elements={
                  { type="rule_reference", name="COMMA", is_token=true },
                  { type="rule_reference", name="expression", is_token=false },
                } } },
            } } },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
      } },
      line_number=518,
    },
    {
      name="concatenation",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
          } } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=534,
    },
    {
      name="replication",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="concatenation", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=540,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
