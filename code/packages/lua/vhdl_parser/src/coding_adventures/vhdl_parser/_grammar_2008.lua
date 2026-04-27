-- AUTO-GENERATED FILE - DO NOT EDIT
-- Source: vhdl2008.grammar
-- Regenerate with: grammar-tools compile-grammar vhdl2008.grammar
--
-- This file embeds a ParserGrammar as native Lua data structures.
-- Call parser_grammar() instead of reading and parsing the .grammar file.

local gt = require("coding_adventures.grammar_tools")

local function parser_grammar()
  local g = gt.ParserGrammar.new()
  g.rules = {
    {
      name="design_file",
      body={ type="repetition", element={ type="rule_reference", name="design_unit", is_token=false } },
      line_number=64,
    },
    {
      name="design_unit",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="context_item", is_token=false } },
        { type="rule_reference", name="library_unit", is_token=false },
      } },
      line_number=66,
    },
    {
      name="context_item",
      body={ type="alternation", choices={
        { type="rule_reference", name="library_clause", is_token=false },
        { type="rule_reference", name="use_clause", is_token=false },
      } },
      line_number=68,
    },
    {
      name="library_clause",
      body={ type="sequence", elements={
        { type="literal", value="library" },
        { type="rule_reference", name="name_list", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=71,
    },
    {
      name="use_clause",
      body={ type="sequence", elements={
        { type="literal", value="use" },
        { type="rule_reference", name="selected_name", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=74,
    },
    {
      name="selected_name",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="DOT", is_token=true },
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="NAME", is_token=true },
                { type="literal", value="all" },
              } } },
          } } },
      } },
      line_number=77,
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
      line_number=79,
    },
    {
      name="library_unit",
      body={ type="alternation", choices={
        { type="rule_reference", name="entity_declaration", is_token=false },
        { type="rule_reference", name="architecture_body", is_token=false },
        { type="rule_reference", name="package_declaration", is_token=false },
        { type="rule_reference", name="package_body", is_token=false },
      } },
      line_number=81,
    },
    {
      name="entity_declaration",
      body={ type="sequence", elements={
        { type="literal", value="entity" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="is" },
        { type="optional", element={ type="rule_reference", name="generic_clause", is_token=false } },
        { type="optional", element={ type="rule_reference", name="port_clause", is_token=false } },
        { type="literal", value="end" },
        { type="optional", element={ type="literal", value="entity" } },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=112,
    },
    {
      name="generic_clause",
      body={ type="sequence", elements={
        { type="literal", value="generic" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="interface_list", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=117,
    },
    {
      name="port_clause",
      body={ type="sequence", elements={
        { type="literal", value="port" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="interface_list", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=118,
    },
    {
      name="interface_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="interface_element", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="SEMICOLON", is_token=true },
            { type="rule_reference", name="interface_element", is_token=false },
          } } },
      } },
      line_number=123,
    },
    {
      name="interface_element",
      body={ type="sequence", elements={
        { type="rule_reference", name="name_list", is_token=false },
        { type="rule_reference", name="COLON", is_token=true },
        { type="optional", element={ type="rule_reference", name="mode", is_token=false } },
        { type="rule_reference", name="subtype_indication", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="VAR_ASSIGN", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
          } } },
      } },
      line_number=124,
    },
    {
      name="mode",
      body={ type="alternation", choices={
        { type="literal", value="in" },
        { type="literal", value="out" },
        { type="literal", value="inout" },
        { type="literal", value="buffer" },
      } },
      line_number=132,
    },
    {
      name="architecture_body",
      body={ type="sequence", elements={
        { type="literal", value="architecture" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="of" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="is" },
        { type="repetition", element={ type="rule_reference", name="block_declarative_item", is_token=false } },
        { type="literal", value="begin" },
        { type="repetition", element={ type="rule_reference", name="concurrent_statement", is_token=false } },
        { type="literal", value="end" },
        { type="optional", element={ type="literal", value="architecture" } },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=154,
    },
    {
      name="block_declarative_item",
      body={ type="alternation", choices={
        { type="rule_reference", name="signal_declaration", is_token=false },
        { type="rule_reference", name="constant_declaration", is_token=false },
        { type="rule_reference", name="type_declaration", is_token=false },
        { type="rule_reference", name="subtype_declaration", is_token=false },
        { type="rule_reference", name="component_declaration", is_token=false },
        { type="rule_reference", name="function_declaration", is_token=false },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="procedure_declaration", is_token=false },
        { type="rule_reference", name="procedure_body", is_token=false },
      } },
      line_number=160,
    },
    {
      name="signal_declaration",
      body={ type="sequence", elements={
        { type="literal", value="signal" },
        { type="rule_reference", name="name_list", is_token=false },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="subtype_indication", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="VAR_ASSIGN", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
          } } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=189,
    },
    {
      name="constant_declaration",
      body={ type="sequence", elements={
        { type="literal", value="constant" },
        { type="rule_reference", name="name_list", is_token=false },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="subtype_indication", is_token=false },
        { type="rule_reference", name="VAR_ASSIGN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=191,
    },
    {
      name="variable_declaration",
      body={ type="sequence", elements={
        { type="literal", value="variable" },
        { type="rule_reference", name="name_list", is_token=false },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="subtype_indication", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="VAR_ASSIGN", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
          } } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=193,
    },
    {
      name="type_declaration",
      body={ type="sequence", elements={
        { type="literal", value="type" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="is" },
        { type="rule_reference", name="type_definition", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=218,
    },
    {
      name="subtype_declaration",
      body={ type="sequence", elements={
        { type="literal", value="subtype" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="is" },
        { type="rule_reference", name="subtype_indication", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=219,
    },
    {
      name="type_definition",
      body={ type="alternation", choices={
        { type="rule_reference", name="enumeration_type", is_token=false },
        { type="rule_reference", name="array_type", is_token=false },
        { type="rule_reference", name="record_type", is_token=false },
      } },
      line_number=221,
    },
    {
      name="enumeration_type",
      body={ type="sequence", elements={
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="CHAR_LITERAL", is_token=true },
          } } },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="NAME", is_token=true },
                { type="rule_reference", name="CHAR_LITERAL", is_token=true },
              } } },
          } } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=227,
    },
    {
      name="array_type",
      body={ type="sequence", elements={
        { type="literal", value="array" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="index_constraint", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="literal", value="of" },
        { type="rule_reference", name="subtype_indication", is_token=false },
      } },
      line_number=232,
    },
    {
      name="index_constraint",
      body={ type="sequence", elements={
        { type="rule_reference", name="discrete_range", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="discrete_range", is_token=false },
          } } },
      } },
      line_number=234,
    },
    {
      name="discrete_range",
      body={ type="alternation", choices={
        { type="rule_reference", name="subtype_indication", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="expression", is_token=false },
          { type="group", element={ type="alternation", choices={
              { type="literal", value="to" },
              { type="literal", value="downto" },
            } } },
          { type="rule_reference", name="expression", is_token=false },
        } },
      } },
      line_number=235,
    },
    {
      name="record_type",
      body={ type="sequence", elements={
        { type="literal", value="record" },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="subtype_indication", is_token=false },
            { type="rule_reference", name="SEMICOLON", is_token=true },
          } } },
        { type="literal", value="end" },
        { type="literal", value="record" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
      } },
      line_number=239,
    },
    {
      name="subtype_indication",
      body={ type="sequence", elements={
        { type="rule_reference", name="selected_name", is_token=false },
        { type="optional", element={ type="rule_reference", name="constraint", is_token=false } },
      } },
      line_number=247,
    },
    {
      name="constraint",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
          { type="group", element={ type="alternation", choices={
              { type="literal", value="to" },
              { type="literal", value="downto" },
            } } },
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="range" },
          { type="rule_reference", name="expression", is_token=false },
          { type="group", element={ type="alternation", choices={
              { type="literal", value="to" },
              { type="literal", value="downto" },
            } } },
          { type="rule_reference", name="expression", is_token=false },
        } },
      } },
      line_number=249,
    },
    {
      name="concurrent_statement",
      body={ type="alternation", choices={
        { type="rule_reference", name="process_statement", is_token=false },
        { type="rule_reference", name="signal_assignment_concurrent", is_token=false },
        { type="rule_reference", name="component_instantiation", is_token=false },
        { type="rule_reference", name="generate_statement", is_token=false },
      } },
      line_number=264,
    },
    {
      name="signal_assignment_concurrent",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LESS_EQUALS", is_token=true },
        { type="rule_reference", name="waveform", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=272,
    },
    {
      name="waveform",
      body={ type="sequence", elements={
        { type="rule_reference", name="waveform_element", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="waveform_element", is_token=false },
          } } },
      } },
      line_number=274,
    },
    {
      name="waveform_element",
      body={ type="rule_reference", name="expression", is_token=false },
      line_number=275,
    },
    {
      name="process_statement",
      body={ type="sequence", elements={
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="COLON", is_token=true },
          } } },
        { type="literal", value="process" },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="rule_reference", name="sensitivity_list", is_token=false },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
        { type="optional", element={ type="literal", value="is" } },
        { type="repetition", element={ type="rule_reference", name="process_declarative_item", is_token=false } },
        { type="literal", value="begin" },
        { type="repetition", element={ type="rule_reference", name="sequential_statement", is_token=false } },
        { type="literal", value="end" },
        { type="literal", value="process" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=307,
    },
    {
      name="sensitivity_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
      } },
      line_number=315,
    },
    {
      name="process_declarative_item",
      body={ type="alternation", choices={
        { type="rule_reference", name="variable_declaration", is_token=false },
        { type="rule_reference", name="constant_declaration", is_token=false },
        { type="rule_reference", name="type_declaration", is_token=false },
        { type="rule_reference", name="subtype_declaration", is_token=false },
      } },
      line_number=317,
    },
    {
      name="sequential_statement",
      body={ type="alternation", choices={
        { type="rule_reference", name="signal_assignment_seq", is_token=false },
        { type="rule_reference", name="variable_assignment", is_token=false },
        { type="rule_reference", name="if_statement", is_token=false },
        { type="rule_reference", name="case_statement", is_token=false },
        { type="rule_reference", name="loop_statement", is_token=false },
        { type="rule_reference", name="return_statement", is_token=false },
        { type="rule_reference", name="null_statement", is_token=false },
      } },
      line_number=329,
    },
    {
      name="signal_assignment_seq",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LESS_EQUALS", is_token=true },
        { type="rule_reference", name="waveform", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=342,
    },
    {
      name="variable_assignment",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="VAR_ASSIGN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=346,
    },
    {
      name="if_statement",
      body={ type="sequence", elements={
        { type="literal", value="if" },
        { type="rule_reference", name="expression", is_token=false },
        { type="literal", value="then" },
        { type="repetition", element={ type="rule_reference", name="sequential_statement", is_token=false } },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="elsif" },
            { type="rule_reference", name="expression", is_token=false },
            { type="literal", value="then" },
            { type="repetition", element={ type="rule_reference", name="sequential_statement", is_token=false } },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="else" },
            { type="repetition", element={ type="rule_reference", name="sequential_statement", is_token=false } },
          } } },
        { type="literal", value="end" },
        { type="literal", value="if" },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=356,
    },
    {
      name="case_statement",
      body={ type="sequence", elements={
        { type="literal", value="case" },
        { type="rule_reference", name="expression", is_token=false },
        { type="literal", value="is" },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="when" },
            { type="rule_reference", name="choices", is_token=false },
            { type="rule_reference", name="ARROW", is_token=true },
            { type="repetition", element={ type="rule_reference", name="sequential_statement", is_token=false } },
          } } },
        { type="literal", value="end" },
        { type="literal", value="case" },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=372,
    },
    {
      name="choices",
      body={ type="sequence", elements={
        { type="rule_reference", name="choice", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="PIPE", is_token=true },
            { type="rule_reference", name="choice", is_token=false },
          } } },
      } },
      line_number=376,
    },
    {
      name="choice",
      body={ type="alternation", choices={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="discrete_range", is_token=false },
        { type="literal", value="others" },
      } },
      line_number=377,
    },
    {
      name="loop_statement",
      body={ type="sequence", elements={
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="COLON", is_token=true },
          } } },
        { type="optional", element={ type="alternation", choices={
            { type="sequence", elements={
              { type="literal", value="for" },
              { type="rule_reference", name="NAME", is_token=true },
              { type="literal", value="in" },
              { type="rule_reference", name="discrete_range", is_token=false },
            } },
            { type="sequence", elements={
              { type="literal", value="while" },
              { type="rule_reference", name="expression", is_token=false },
            } },
          } } },
        { type="literal", value="loop" },
        { type="repetition", element={ type="rule_reference", name="sequential_statement", is_token=false } },
        { type="literal", value="end" },
        { type="literal", value="loop" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=391,
    },
    {
      name="return_statement",
      body={ type="sequence", elements={
        { type="literal", value="return" },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=398,
    },
    {
      name="null_statement",
      body={ type="sequence", elements={
        { type="literal", value="null" },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=399,
    },
    {
      name="component_declaration",
      body={ type="sequence", elements={
        { type="literal", value="component" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="literal", value="is" } },
        { type="optional", element={ type="rule_reference", name="generic_clause", is_token=false } },
        { type="optional", element={ type="rule_reference", name="port_clause", is_token=false } },
        { type="literal", value="end" },
        { type="literal", value="component" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=425,
    },
    {
      name="component_instantiation",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="NAME", is_token=true },
            { type="sequence", elements={
              { type="literal", value="entity" },
              { type="rule_reference", name="selected_name", is_token=false },
              { type="optional", element={ type="sequence", elements={
                  { type="rule_reference", name="LPAREN", is_token=true },
                  { type="rule_reference", name="NAME", is_token=true },
                  { type="rule_reference", name="RPAREN", is_token=true },
                } } },
            } },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="generic" },
            { type="literal", value="map" },
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="rule_reference", name="association_list", is_token=false },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="port" },
            { type="literal", value="map" },
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="rule_reference", name="association_list", is_token=false },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=430,
    },
    {
      name="association_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="association_element", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="association_element", is_token=false },
          } } },
      } },
      line_number=437,
    },
    {
      name="association_element",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="NAME", is_token=true },
              { type="rule_reference", name="ARROW", is_token=true },
            } } },
          { type="rule_reference", name="expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="NAME", is_token=true },
              { type="rule_reference", name="ARROW", is_token=true },
            } } },
          { type="literal", value="open" },
        } },
      } },
      line_number=438,
    },
    {
      name="generate_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="for_generate", is_token=false },
            { type="rule_reference", name="if_generate", is_token=false },
          } } },
      } },
      line_number=461,
    },
    {
      name="for_generate",
      body={ type="sequence", elements={
        { type="literal", value="for" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="in" },
        { type="rule_reference", name="discrete_range", is_token=false },
        { type="literal", value="generate" },
        { type="repetition", element={ type="rule_reference", name="concurrent_statement", is_token=false } },
        { type="literal", value="end" },
        { type="literal", value="generate" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=463,
    },
    {
      name="if_generate",
      body={ type="sequence", elements={
        { type="literal", value="if" },
        { type="rule_reference", name="expression", is_token=false },
        { type="literal", value="generate" },
        { type="repetition", element={ type="rule_reference", name="concurrent_statement", is_token=false } },
        { type="literal", value="end" },
        { type="literal", value="generate" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=467,
    },
    {
      name="package_declaration",
      body={ type="sequence", elements={
        { type="literal", value="package" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="is" },
        { type="repetition", element={ type="rule_reference", name="package_declarative_item", is_token=false } },
        { type="literal", value="end" },
        { type="optional", element={ type="literal", value="package" } },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=488,
    },
    {
      name="package_body",
      body={ type="sequence", elements={
        { type="literal", value="package" },
        { type="literal", value="body" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="is" },
        { type="repetition", element={ type="rule_reference", name="package_body_declarative_item", is_token=false } },
        { type="literal", value="end" },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="package" },
            { type="literal", value="body" },
          } } },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=492,
    },
    {
      name="package_declarative_item",
      body={ type="alternation", choices={
        { type="rule_reference", name="type_declaration", is_token=false },
        { type="rule_reference", name="subtype_declaration", is_token=false },
        { type="rule_reference", name="constant_declaration", is_token=false },
        { type="rule_reference", name="signal_declaration", is_token=false },
        { type="rule_reference", name="component_declaration", is_token=false },
        { type="rule_reference", name="function_declaration", is_token=false },
        { type="rule_reference", name="procedure_declaration", is_token=false },
      } },
      line_number=496,
    },
    {
      name="package_body_declarative_item",
      body={ type="alternation", choices={
        { type="rule_reference", name="type_declaration", is_token=false },
        { type="rule_reference", name="subtype_declaration", is_token=false },
        { type="rule_reference", name="constant_declaration", is_token=false },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="procedure_body", is_token=false },
      } },
      line_number=504,
    },
    {
      name="function_declaration",
      body={ type="sequence", elements={
        { type="optional", element={ type="alternation", choices={
            { type="literal", value="pure" },
            { type="literal", value="impure" },
          } } },
        { type="literal", value="function" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="rule_reference", name="interface_list", is_token=false },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
        { type="literal", value="return" },
        { type="rule_reference", name="subtype_indication", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=520,
    },
    {
      name="function_body",
      body={ type="sequence", elements={
        { type="optional", element={ type="alternation", choices={
            { type="literal", value="pure" },
            { type="literal", value="impure" },
          } } },
        { type="literal", value="function" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="rule_reference", name="interface_list", is_token=false },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
        { type="literal", value="return" },
        { type="rule_reference", name="subtype_indication", is_token=false },
        { type="literal", value="is" },
        { type="repetition", element={ type="rule_reference", name="process_declarative_item", is_token=false } },
        { type="literal", value="begin" },
        { type="repetition", element={ type="rule_reference", name="sequential_statement", is_token=false } },
        { type="literal", value="end" },
        { type="optional", element={ type="literal", value="function" } },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=525,
    },
    {
      name="procedure_declaration",
      body={ type="sequence", elements={
        { type="literal", value="procedure" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="rule_reference", name="interface_list", is_token=false },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=534,
    },
    {
      name="procedure_body",
      body={ type="sequence", elements={
        { type="literal", value="procedure" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="rule_reference", name="interface_list", is_token=false },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
        { type="literal", value="is" },
        { type="repetition", element={ type="rule_reference", name="process_declarative_item", is_token=false } },
        { type="literal", value="begin" },
        { type="repetition", element={ type="rule_reference", name="sequential_statement", is_token=false } },
        { type="literal", value="end" },
        { type="optional", element={ type="literal", value="procedure" } },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=537,
    },
    {
      name="expression",
      body={ type="rule_reference", name="logical_expr", is_token=false },
      line_number=574,
    },
    {
      name="logical_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="relation", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="logical_op", is_token=false },
            { type="rule_reference", name="relation", is_token=false },
          } } },
      } },
      line_number=581,
    },
    {
      name="logical_op",
      body={ type="alternation", choices={
        { type="literal", value="and" },
        { type="literal", value="or" },
        { type="literal", value="xor" },
        { type="literal", value="nand" },
        { type="literal", value="nor" },
        { type="literal", value="xnor" },
      } },
      line_number=582,
    },
    {
      name="relation",
      body={ type="sequence", elements={
        { type="rule_reference", name="shift_expr", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="relational_op", is_token=false },
            { type="rule_reference", name="shift_expr", is_token=false },
          } } },
      } },
      line_number=586,
    },
    {
      name="relational_op",
      body={ type="alternation", choices={
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="NOT_EQUALS", is_token=true },
        { type="rule_reference", name="LESS_THAN", is_token=true },
        { type="rule_reference", name="LESS_EQUALS", is_token=true },
        { type="rule_reference", name="GREATER_THAN", is_token=true },
        { type="rule_reference", name="GREATER_EQUALS", is_token=true },
      } },
      line_number=587,
    },
    {
      name="shift_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="adding_expr", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="shift_op", is_token=false },
            { type="rule_reference", name="adding_expr", is_token=false },
          } } },
      } },
      line_number=592,
    },
    {
      name="shift_op",
      body={ type="alternation", choices={
        { type="literal", value="sll" },
        { type="literal", value="srl" },
        { type="literal", value="sla" },
        { type="literal", value="sra" },
        { type="literal", value="rol" },
        { type="literal", value="ror" },
      } },
      line_number=593,
    },
    {
      name="adding_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="multiplying_expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="adding_op", is_token=false },
            { type="rule_reference", name="multiplying_expr", is_token=false },
          } } },
      } },
      line_number=597,
    },
    {
      name="adding_op",
      body={ type="alternation", choices={
        { type="rule_reference", name="PLUS", is_token=true },
        { type="rule_reference", name="MINUS", is_token=true },
        { type="rule_reference", name="AMPERSAND", is_token=true },
      } },
      line_number=598,
    },
    {
      name="multiplying_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="unary_expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="multiplying_op", is_token=false },
            { type="rule_reference", name="unary_expr", is_token=false },
          } } },
      } },
      line_number=601,
    },
    {
      name="multiplying_op",
      body={ type="alternation", choices={
        { type="rule_reference", name="STAR", is_token=true },
        { type="rule_reference", name="SLASH", is_token=true },
        { type="literal", value="mod" },
        { type="literal", value="rem" },
      } },
      line_number=602,
    },
    {
      name="unary_expr",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="abs" },
          { type="rule_reference", name="unary_expr", is_token=false },
        } },
        { type="sequence", elements={
          { type="literal", value="not" },
          { type="rule_reference", name="unary_expr", is_token=false },
        } },
        { type="sequence", elements={
          { type="group", element={ type="alternation", choices={
              { type="rule_reference", name="PLUS", is_token=true },
              { type="rule_reference", name="MINUS", is_token=true },
            } } },
          { type="rule_reference", name="unary_expr", is_token=false },
        } },
        { type="rule_reference", name="power_expr", is_token=false },
      } },
      line_number=605,
    },
    {
      name="power_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="primary", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="POWER", is_token=true },
            { type="rule_reference", name="primary", is_token=false },
          } } },
      } },
      line_number=611,
    },
    {
      name="primary",
      body={ type="alternation", choices={
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="rule_reference", name="REAL_NUMBER", is_token=true },
        { type="rule_reference", name="BASED_LITERAL", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="CHAR_LITERAL", is_token=true },
        { type="rule_reference", name="BIT_STRING", is_token=true },
        { type="sequence", elements={
          { type="rule_reference", name="NAME", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="TICK", is_token=true },
              { type="rule_reference", name="NAME", is_token=true },
            } } },
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
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="rule_reference", name="aggregate", is_token=false },
        { type="literal", value="null" },
      } },
      line_number=619,
    },
    {
      name="aggregate",
      body={ type="sequence", elements={
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="element_association", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="element_association", is_token=false },
          } } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=635,
    },
    {
      name="element_association",
      body={ type="sequence", elements={
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="choices", is_token=false },
            { type="rule_reference", name="ARROW", is_token=true },
          } } },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=636,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
