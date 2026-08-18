-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: java14.grammar
-- Regenerate with: grammar-tools compile-grammar java14.grammar
--
-- This file embeds a ParserGrammar as native Lua data structures.
-- Call parser_grammar() instead of reading and parsing the .grammar file.

local gt = require("coding_adventures.grammar_tools")

local function parser_grammar()
  local g = gt.ParserGrammar.new()
  g.rules = {
    {
      name="program",
      body={ type="repetition", element={ type="rule_reference", name="program_item", is_token=false } },
      line_number=108,
    },
    {
      name="program_item",
      body={ type="alternation", choices={
        { type="rule_reference", name="package_declaration", is_token=false },
        { type="rule_reference", name="import_declaration", is_token=false },
        { type="rule_reference", name="type_declaration", is_token=false },
        { type="rule_reference", name="method_declaration", is_token=false },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=109,
    },
    {
      name="compilation_unit",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="optional", element={ type="rule_reference", name="package_declaration", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="import_declaration", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="type_declaration", is_token=false } },
      } },
      line_number=110,
    },
    {
      name="package_declaration",
      body={ type="sequence", elements={
        { type="literal", value="package" },
        { type="rule_reference", name="qualified_name", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=126,
    },
    {
      name="import_declaration",
      body={ type="sequence", elements={
        { type="literal", value="import" },
        { type="optional", element={ type="literal", value="static" } },
        { type="rule_reference", name="qualified_name", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="DOT", is_token=true },
            { type="rule_reference", name="STAR", is_token=true },
          } } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=141,
    },
    {
      name="type_declaration",
      body={ type="alternation", choices={
        { type="rule_reference", name="class_declaration", is_token=false },
        { type="rule_reference", name="interface_declaration", is_token=false },
        { type="rule_reference", name="enum_declaration", is_token=false },
        { type="rule_reference", name="annotation_type_declaration", is_token=false },
        { type="rule_reference", name="record_declaration", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=164,
    },
    {
      name="qualified_name",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="DOT", is_token=true },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
      } },
      line_number=183,
    },
    {
      name="annotation",
      body={ type="sequence", elements={
        { type="rule_reference", name="AT", is_token=true },
        { type="rule_reference", name="qualified_name", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="optional", element={ type="alternation", choices={
                { type="rule_reference", name="element_value_pairs", is_token=false },
                { type="rule_reference", name="element_value", is_token=false },
              } } },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
      } },
      line_number=206,
    },
    {
      name="annotations",
      body={ type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
      line_number=208,
    },
    {
      name="element_value_pairs",
      body={ type="sequence", elements={
        { type="rule_reference", name="element_value_pair", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="element_value_pair", is_token=false },
          } } },
      } },
      line_number=210,
    },
    {
      name="element_value_pair",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="element_value", is_token=false },
      } },
      line_number=212,
    },
    {
      name="element_value",
      body={ type="alternation", choices={
        { type="rule_reference", name="annotation", is_token=false },
        { type="rule_reference", name="element_value_array", is_token=false },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=220,
    },
    {
      name="element_value_array",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="element_value", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="element_value", is_token=false },
              } } },
          } } },
        { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=224,
    },
    {
      name="annotation_type_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="class_modifier", is_token=false } },
        { type="rule_reference", name="AT", is_token=true },
        { type="literal", value="interface" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="annotation_type_body", is_token=false },
      } },
      line_number=240,
    },
    {
      name="annotation_type_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="annotation_type_element_declaration", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=243,
    },
    {
      name="annotation_type_element_declaration",
      body={ type="alternation", choices={
        { type="rule_reference", name="annotation_element_declaration", is_token=false },
        { type="rule_reference", name="field_declaration", is_token=false },
        { type="rule_reference", name="class_declaration", is_token=false },
        { type="rule_reference", name="interface_declaration", is_token=false },
        { type="rule_reference", name="enum_declaration", is_token=false },
        { type="rule_reference", name="annotation_type_declaration", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=245,
    },
    {
      name="annotation_element_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="method_modifier", is_token=false } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="default" },
            { type="rule_reference", name="element_value", is_token=false },
          } } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=253,
    },
    {
      name="class_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="class_modifier", is_token=false } },
        { type="literal", value="class" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="extends" },
            { type="rule_reference", name="class_type", is_token=false },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="implements" },
            { type="rule_reference", name="interface_type_list", is_token=false },
          } } },
        { type="rule_reference", name="class_body", is_token=false },
      } },
      line_number=274,
    },
    {
      name="class_modifier",
      body={ type="alternation", choices={
        { type="rule_reference", name="annotation", is_token=false },
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="private" },
        { type="literal", value="abstract" },
        { type="literal", value="final" },
        { type="literal", value="static" },
        { type="literal", value="strictfp" },
      } },
      line_number=279,
    },
    {
      name="class_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="class_body_declaration", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=288,
    },
    {
      name="class_body_declaration",
      body={ type="alternation", choices={
        { type="rule_reference", name="static_initializer", is_token=false },
        { type="rule_reference", name="instance_initializer", is_token=false },
        { type="rule_reference", name="constructor_declaration", is_token=false },
        { type="rule_reference", name="method_declaration", is_token=false },
        { type="rule_reference", name="field_declaration", is_token=false },
        { type="rule_reference", name="class_declaration", is_token=false },
        { type="rule_reference", name="interface_declaration", is_token=false },
        { type="rule_reference", name="enum_declaration", is_token=false },
        { type="rule_reference", name="annotation_type_declaration", is_token=false },
        { type="rule_reference", name="record_declaration", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=294,
    },
    {
      name="interface_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="interface_modifier", is_token=false } },
        { type="literal", value="interface" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="extends" },
            { type="rule_reference", name="interface_type_list", is_token=false },
          } } },
        { type="rule_reference", name="interface_body", is_token=false },
      } },
      line_number=340,
    },
    {
      name="interface_modifier",
      body={ type="alternation", choices={
        { type="rule_reference", name="annotation", is_token=false },
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="private" },
        { type="literal", value="abstract" },
        { type="literal", value="static" },
        { type="literal", value="strictfp" },
      } },
      line_number=344,
    },
    {
      name="interface_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="interface_body_declaration", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=352,
    },
    {
      name="interface_body_declaration",
      body={ type="alternation", choices={
        { type="rule_reference", name="interface_method_declaration", is_token=false },
        { type="rule_reference", name="interface_field_declaration", is_token=false },
        { type="rule_reference", name="class_declaration", is_token=false },
        { type="rule_reference", name="interface_declaration", is_token=false },
        { type="rule_reference", name="enum_declaration", is_token=false },
        { type="rule_reference", name="annotation_type_declaration", is_token=false },
        { type="rule_reference", name="record_declaration", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=357,
    },
    {
      name="interface_field_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="field_modifier", is_token=false } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="variable_declarators", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=366,
    },
    {
      name="interface_method_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="interface_method_modifier", is_token=false } },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="result_type", is_token=false },
        { type="rule_reference", name="method_declarator", is_token=false },
        { type="optional", element={ type="rule_reference", name="throws_clause", is_token=false } },
        { type="rule_reference", name="method_body", is_token=false },
      } },
      line_number=381,
    },
    {
      name="interface_method_modifier",
      body={ type="alternation", choices={
        { type="rule_reference", name="annotation", is_token=false },
        { type="literal", value="public" },
        { type="literal", value="private" },
        { type="literal", value="abstract" },
        { type="literal", value="default" },
        { type="literal", value="static" },
        { type="literal", value="strictfp" },
      } },
      line_number=384,
    },
    {
      name="interface_type_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="class_type", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="class_type", is_token=false },
          } } },
      } },
      line_number=392,
    },
    {
      name="enum_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="class_modifier", is_token=false } },
        { type="literal", value="enum" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="implements" },
            { type="rule_reference", name="interface_type_list", is_token=false },
          } } },
        { type="rule_reference", name="enum_body", is_token=false },
      } },
      line_number=412,
    },
    {
      name="enum_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="rule_reference", name="enum_constant_list", is_token=false } },
        { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="SEMICOLON", is_token=true },
            { type="repetition", element={ type="rule_reference", name="class_body_declaration", is_token=false } },
          } } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=416,
    },
    {
      name="enum_constant_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="enum_constant", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="enum_constant", is_token=false },
          } } },
      } },
      line_number=418,
    },
    {
      name="enum_constant",
      body={ type="sequence", elements={
        { type="rule_reference", name="annotations", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="optional", element={ type="rule_reference", name="argument_list", is_token=false } },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
        { type="optional", element={ type="rule_reference", name="class_body", is_token=false } },
      } },
      line_number=420,
    },
    {
      name="record_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="class_modifier", is_token=false } },
        { type="literal", value="record" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="record_header", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="implements" },
            { type="rule_reference", name="interface_type_list", is_token=false },
          } } },
        { type="rule_reference", name="record_body", is_token=false },
      } },
      line_number=498,
    },
    {
      name="record_header",
      body={ type="sequence", elements={
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="record_component_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=516,
    },
    {
      name="record_component_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="record_component", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="record_component", is_token=false },
          } } },
      } },
      line_number=518,
    },
    {
      name="record_component",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=520,
    },
    {
      name="record_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="record_body_declaration", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=543,
    },
    {
      name="record_body_declaration",
      body={ type="alternation", choices={
        { type="rule_reference", name="compact_constructor", is_token=false },
        { type="rule_reference", name="class_body_declaration", is_token=false },
      } },
      line_number=545,
    },
    {
      name="compact_constructor",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="constructor_modifier", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="constructor_body", is_token=false },
      } },
      line_number=561,
    },
    {
      name="type_parameters",
      body={ type="sequence", elements={
        { type="rule_reference", name="LESS_THAN", is_token=true },
        { type="rule_reference", name="type_parameter", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="type_parameter", is_token=false },
          } } },
        { type="rule_reference", name="GREATER_THAN", is_token=true },
      } },
      line_number=587,
    },
    {
      name="type_parameter",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="extends" },
            { type="rule_reference", name="bound", is_token=false },
          } } },
      } },
      line_number=589,
    },
    {
      name="bound",
      body={ type="sequence", elements={
        { type="rule_reference", name="class_type", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="AMPERSAND", is_token=true },
            { type="rule_reference", name="class_type", is_token=false },
          } } },
      } },
      line_number=591,
    },
    {
      name="type_arguments",
      body={ type="sequence", elements={
        { type="rule_reference", name="LESS_THAN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="type_argument", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="type_argument", is_token=false },
              } } },
          } } },
        { type="rule_reference", name="GREATER_THAN", is_token=true },
      } },
      line_number=602,
    },
    {
      name="type_argument",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
          { type="rule_reference", name="type", is_token=false },
        } },
        { type="sequence", elements={
          { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
          { type="rule_reference", name="QUESTION", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="group", element={ type="alternation", choices={
                  { type="literal", value="extends" },
                  { type="literal", value="super" },
                } } },
              { type="rule_reference", name="type", is_token=false },
            } } },
        } },
      } },
      line_number=604,
    },
    {
      name="field_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="field_modifier", is_token=false } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="variable_declarators", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=622,
    },
    {
      name="field_modifier",
      body={ type="alternation", choices={
        { type="rule_reference", name="annotation", is_token=false },
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="private" },
        { type="literal", value="static" },
        { type="literal", value="final" },
        { type="literal", value="transient" },
        { type="literal", value="volatile" },
      } },
      line_number=624,
    },
    {
      name="variable_declarators",
      body={ type="sequence", elements={
        { type="rule_reference", name="variable_declarator", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="variable_declarator", is_token=false },
          } } },
      } },
      line_number=633,
    },
    {
      name="variable_declarator",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="LBRACKET", is_token=true },
            { type="rule_reference", name="RBRACKET", is_token=true },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="EQUALS", is_token=true },
            { type="rule_reference", name="variable_initializer", is_token=false },
          } } },
      } },
      line_number=635,
    },
    {
      name="variable_initializer",
      body={ type="alternation", choices={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="array_initializer", is_token=false },
      } },
      line_number=637,
    },
    {
      name="array_initializer",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="variable_initializer", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="variable_initializer", is_token=false },
              } } },
          } } },
        { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=640,
    },
    {
      name="method_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="method_modifier", is_token=false } },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="result_type", is_token=false },
        { type="rule_reference", name="method_declarator", is_token=false },
        { type="optional", element={ type="rule_reference", name="throws_clause", is_token=false } },
        { type="rule_reference", name="method_body", is_token=false },
      } },
      line_number=659,
    },
    {
      name="method_modifier",
      body={ type="alternation", choices={
        { type="rule_reference", name="annotation", is_token=false },
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="private" },
        { type="literal", value="static" },
        { type="literal", value="abstract" },
        { type="literal", value="final" },
        { type="literal", value="synchronized" },
        { type="literal", value="native" },
        { type="literal", value="strictfp" },
      } },
      line_number=662,
    },
    {
      name="result_type",
      body={ type="alternation", choices={
        { type="literal", value="void" },
        { type="rule_reference", name="type", is_token=false },
      } },
      line_number=673,
    },
    {
      name="method_declarator",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="formal_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="LBRACKET", is_token=true },
            { type="rule_reference", name="RBRACKET", is_token=true },
          } } },
      } },
      line_number=676,
    },
    {
      name="formal_parameter_list",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="formal_parameter", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="formal_parameter", is_token=false },
            } } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="formal_parameter", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="formal_parameter", is_token=false },
            } } },
          { type="rule_reference", name="COMMA", is_token=true },
          { type="rule_reference", name="varargs_parameter", is_token=false },
        } },
        { type="rule_reference", name="varargs_parameter", is_token=false },
      } },
      line_number=693,
    },
    {
      name="formal_parameter",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="optional", element={ type="literal", value="final" } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="LBRACKET", is_token=true },
            { type="rule_reference", name="RBRACKET", is_token=true },
          } } },
      } },
      line_number=697,
    },
    {
      name="varargs_parameter",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="optional", element={ type="literal", value="final" } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="ELLIPSIS", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=699,
    },
    {
      name="throws_clause",
      body={ type="sequence", elements={
        { type="literal", value="throws" },
        { type="rule_reference", name="class_type", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="class_type", is_token=false },
          } } },
      } },
      line_number=701,
    },
    {
      name="method_body",
      body={ type="alternation", choices={
        { type="rule_reference", name="block", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=703,
    },
    {
      name="constructor_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="constructor_modifier", is_token=false } },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="formal_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="throws_clause", is_token=false } },
        { type="rule_reference", name="constructor_body", is_token=false },
      } },
      line_number=720,
    },
    {
      name="constructor_modifier",
      body={ type="alternation", choices={
        { type="rule_reference", name="annotation", is_token=false },
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="private" },
      } },
      line_number=724,
    },
    {
      name="constructor_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="rule_reference", name="explicit_constructor_invocation", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="block_statement", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=729,
    },
    {
      name="explicit_constructor_invocation",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="optional", element={ type="rule_reference", name="type_arguments", is_token=false } },
          { type="literal", value="this" },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="argument_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="optional", element={ type="rule_reference", name="type_arguments", is_token=false } },
          { type="literal", value="super" },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="argument_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
      } },
      line_number=737,
    },
    {
      name="static_initializer",
      body={ type="sequence", elements={
        { type="literal", value="static" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=749,
    },
    {
      name="instance_initializer",
      body={ type="rule_reference", name="block", is_token=false },
      line_number=751,
    },
    {
      name="type",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
          { type="rule_reference", name="primitive_type", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="LBRACKET", is_token=true },
              { type="rule_reference", name="RBRACKET", is_token=true },
            } } },
        } },
        { type="sequence", elements={
          { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
          { type="rule_reference", name="class_type", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="LBRACKET", is_token=true },
              { type="rule_reference", name="RBRACKET", is_token=true },
            } } },
        } },
      } },
      line_number=777,
    },
    {
      name="primitive_type",
      body={ type="alternation", choices={
        { type="literal", value="boolean" },
        { type="literal", value="byte" },
        { type="literal", value="short" },
        { type="literal", value="int" },
        { type="literal", value="long" },
        { type="literal", value="char" },
        { type="literal", value="float" },
        { type="literal", value="double" },
      } },
      line_number=780,
    },
    {
      name="class_type",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="rule_reference", name="qualified_name", is_token=false },
        { type="optional", element={ type="rule_reference", name="type_arguments", is_token=false } },
      } },
      line_number=794,
    },
    {
      name="local_var_type",
      body={ type="alternation", choices={
        { type="rule_reference", name="type", is_token=false },
        { type="literal", value="var" },
      } },
      line_number=812,
    },
    {
      name="statement",
      body={ type="alternation", choices={
        { type="rule_reference", name="block", is_token=false },
        { type="rule_reference", name="var_declaration", is_token=false },
        { type="rule_reference", name="empty_statement", is_token=false },
        { type="rule_reference", name="expression_statement", is_token=false },
        { type="rule_reference", name="if_statement", is_token=false },
        { type="rule_reference", name="while_statement", is_token=false },
        { type="rule_reference", name="do_while_statement", is_token=false },
        { type="rule_reference", name="for_statement", is_token=false },
        { type="rule_reference", name="enhanced_for_statement", is_token=false },
        { type="rule_reference", name="switch_statement", is_token=false },
        { type="rule_reference", name="try_statement", is_token=false },
        { type="rule_reference", name="throw_statement", is_token=false },
        { type="rule_reference", name="return_statement", is_token=false },
        { type="rule_reference", name="break_statement", is_token=false },
        { type="rule_reference", name="continue_statement", is_token=false },
        { type="rule_reference", name="yield_statement", is_token=false },
        { type="rule_reference", name="synchronized_statement", is_token=false },
        { type="rule_reference", name="assert_statement", is_token=false },
        { type="rule_reference", name="labelled_statement", is_token=false },
      } },
      line_number=859,
    },
    {
      name="block",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="block_statement", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=891,
    },
    {
      name="block_statement",
      body={ type="alternation", choices={
        { type="rule_reference", name="var_declaration", is_token=false },
        { type="rule_reference", name="class_declaration", is_token=false },
        { type="rule_reference", name="record_declaration", is_token=false },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=893,
    },
    {
      name="var_declaration",
      body={ type="rule_reference", name="local_variable_declaration_statement", is_token=false },
      line_number=907,
    },
    {
      name="local_variable_declaration_statement",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="optional", element={ type="literal", value="final" } },
        { type="rule_reference", name="local_var_type", is_token=false },
        { type="rule_reference", name="variable_declarators", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=909,
    },
    {
      name="empty_statement",
      body={ type="rule_reference", name="SEMICOLON", is_token=true },
      line_number=914,
    },
    {
      name="expression_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=922,
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
      line_number=929,
    },
    {
      name="while_statement",
      body={ type="sequence", elements={
        { type="literal", value="while" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=933,
    },
    {
      name="do_while_statement",
      body={ type="sequence", elements={
        { type="literal", value="do" },
        { type="rule_reference", name="statement", is_token=false },
        { type="literal", value="while" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=937,
    },
    {
      name="for_statement",
      body={ type="sequence", elements={
        { type="literal", value="for" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="for_init", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
        { type="optional", element={ type="rule_reference", name="for_update", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=950,
    },
    {
      name="for_init",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
          { type="optional", element={ type="literal", value="final" } },
          { type="rule_reference", name="local_var_type", is_token=false },
          { type="rule_reference", name="variable_declarators", is_token=false },
        } },
        { type="optional", element={ type="rule_reference", name="expression_list", is_token=false } },
      } },
      line_number=953,
    },
    {
      name="for_update",
      body={ type="rule_reference", name="expression_list", is_token=false },
      line_number=956,
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
      line_number=958,
    },
    {
      name="enhanced_for_statement",
      body={ type="sequence", elements={
        { type="literal", value="for" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="optional", element={ type="literal", value="final" } },
        { type="rule_reference", name="local_var_type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=970,
    },
    {
      name="switch_statement",
      body={ type="sequence", elements={
        { type="literal", value="switch" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="switch_block", is_token=false },
      } },
      line_number=1060,
    },
    {
      name="switch_block",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="alternation", choices={
            { type="rule_reference", name="switch_rule", is_token=false },
            { type="rule_reference", name="switch_block_statement_group", is_token=false },
          } } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=1074,
    },
    {
      name="switch_rule",
      body={ type="sequence", elements={
        { type="rule_reference", name="switch_label", is_token=false },
        { type="rule_reference", name="ARROW", is_token=true },
        { type="group", element={ type="alternation", choices={
            { type="sequence", elements={
              { type="rule_reference", name="expression", is_token=false },
              { type="rule_reference", name="SEMICOLON", is_token=true },
            } },
            { type="rule_reference", name="block", is_token=false },
            { type="sequence", elements={
              { type="literal", value="throw" },
              { type="rule_reference", name="expression", is_token=false },
              { type="rule_reference", name="SEMICOLON", is_token=true },
            } },
          } } },
      } },
      line_number=1092,
    },
    {
      name="switch_block_statement_group",
      body={ type="sequence", elements={
        { type="rule_reference", name="switch_label", is_token=false },
        { type="rule_reference", name="COLON", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="switch_label", is_token=false },
            { type="rule_reference", name="COLON", is_token=true },
          } } },
        { type="repetition", element={ type="rule_reference", name="block_statement", is_token=false } },
      } },
      line_number=1101,
    },
    {
      name="switch_label",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="case" },
          { type="rule_reference", name="case_constants", is_token=false },
        } },
        { type="literal", value="default" },
      } },
      line_number=1116,
    },
    {
      name="case_constants",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
          } } },
      } },
      line_number=1125,
    },
    {
      name="yield_statement",
      body={ type="sequence", elements={
        { type="literal", value="yield" },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1165,
    },
    {
      name="try_statement",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="try" },
          { type="rule_reference", name="resource_specification", is_token=false },
          { type="rule_reference", name="block", is_token=false },
          { type="repetition", element={ type="rule_reference", name="catch_clause", is_token=false } },
          { type="optional", element={ type="rule_reference", name="finally_clause", is_token=false } },
        } },
        { type="sequence", elements={
          { type="literal", value="try" },
          { type="rule_reference", name="block", is_token=false },
          { type="group", element={ type="alternation", choices={
              { type="sequence", elements={
                { type="rule_reference", name="catch_clause", is_token=false },
                { type="repetition", element={ type="rule_reference", name="catch_clause", is_token=false } },
                { type="optional", element={ type="rule_reference", name="finally_clause", is_token=false } },
              } },
              { type="rule_reference", name="finally_clause", is_token=false },
            } } },
        } },
      } },
      line_number=1191,
    },
    {
      name="resource_specification",
      body={ type="sequence", elements={
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="resource", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="SEMICOLON", is_token=true },
            { type="rule_reference", name="resource", is_token=false },
          } } },
        { type="optional", element={ type="rule_reference", name="SEMICOLON", is_token=true } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=1197,
    },
    {
      name="resource",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
          { type="optional", element={ type="literal", value="final" } },
          { type="rule_reference", name="local_var_type", is_token=false },
          { type="rule_reference", name="NAME", is_token=true },
          { type="rule_reference", name="EQUALS", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
        } },
        { type="rule_reference", name="qualified_name", is_token=false },
      } },
      line_number=1199,
    },
    {
      name="catch_clause",
      body={ type="sequence", elements={
        { type="literal", value="catch" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="catch_formal_parameter", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=1204,
    },
    {
      name="catch_formal_parameter",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="optional", element={ type="literal", value="final" } },
        { type="rule_reference", name="catch_type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=1206,
    },
    {
      name="catch_type",
      body={ type="sequence", elements={
        { type="rule_reference", name="class_type", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="PIPE", is_token=true },
            { type="rule_reference", name="class_type", is_token=false },
          } } },
      } },
      line_number=1208,
    },
    {
      name="finally_clause",
      body={ type="sequence", elements={
        { type="literal", value="finally" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=1210,
    },
    {
      name="throw_statement",
      body={ type="sequence", elements={
        { type="literal", value="throw" },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1214,
    },
    {
      name="return_statement",
      body={ type="sequence", elements={
        { type="literal", value="return" },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1218,
    },
    {
      name="break_statement",
      body={ type="sequence", elements={
        { type="literal", value="break" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1231,
    },
    {
      name="continue_statement",
      body={ type="sequence", elements={
        { type="literal", value="continue" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1233,
    },
    {
      name="synchronized_statement",
      body={ type="sequence", elements={
        { type="literal", value="synchronized" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=1237,
    },
    {
      name="assert_statement",
      body={ type="sequence", elements={
        { type="literal", value="assert" },
        { type="rule_reference", name="expression", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
          } } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1244,
    },
    {
      name="labelled_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=1248,
    },
    {
      name="expression",
      body={ type="alternation", choices={
        { type="rule_reference", name="lambda_expression", is_token=false },
        { type="rule_reference", name="assignment_expression", is_token=false },
      } },
      line_number=1297,
    },
    {
      name="lambda_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="lambda_parameters", is_token=false },
        { type="rule_reference", name="ARROW", is_token=true },
        { type="rule_reference", name="lambda_body", is_token=false },
      } },
      line_number=1300,
    },
    {
      name="lambda_parameters",
      body={ type="alternation", choices={
        { type="rule_reference", name="NAME", is_token=true },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="lambda_parameter_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
      } },
      line_number=1302,
    },
    {
      name="lambda_parameter_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="lambda_parameter", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="lambda_parameter", is_token=false },
          } } },
      } },
      line_number=1305,
    },
    {
      name="lambda_parameter",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
          { type="optional", element={ type="literal", value="final" } },
          { type="rule_reference", name="type", is_token=false },
          { type="rule_reference", name="NAME", is_token=true },
        } },
        { type="sequence", elements={
          { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
          { type="optional", element={ type="literal", value="final" } },
          { type="rule_reference", name="NAME", is_token=true },
        } },
      } },
      line_number=1307,
    },
    {
      name="lambda_body",
      body={ type="alternation", choices={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=1310,
    },
    {
      name="assignment_expression",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="unary_expression", is_token=false },
          { type="rule_reference", name="assignment_operator", is_token=false },
          { type="rule_reference", name="assignment_expression", is_token=false },
        } },
        { type="rule_reference", name="conditional_expression", is_token=false },
      } },
      line_number=1317,
    },
    {
      name="assignment_operator",
      body={ type="alternation", choices={
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="PLUS_EQUALS", is_token=true },
        { type="rule_reference", name="MINUS_EQUALS", is_token=true },
        { type="rule_reference", name="STAR_EQUALS", is_token=true },
        { type="rule_reference", name="SLASH_EQUALS", is_token=true },
        { type="rule_reference", name="PERCENT_EQUALS", is_token=true },
        { type="rule_reference", name="AMPERSAND_EQUALS", is_token=true },
        { type="rule_reference", name="PIPE_EQUALS", is_token=true },
        { type="rule_reference", name="CARET_EQUALS", is_token=true },
        { type="rule_reference", name="LEFT_SHIFT_EQUALS", is_token=true },
        { type="rule_reference", name="RIGHT_SHIFT_EQUALS", is_token=true },
        { type="rule_reference", name="UNSIGNED_RIGHT_SHIFT_EQUALS", is_token=true },
      } },
      line_number=1320,
    },
    {
      name="conditional_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="logical_or_expression", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="QUESTION", is_token=true },
            { type="rule_reference", name="assignment_expression", is_token=false },
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="assignment_expression", is_token=false },
          } } },
      } },
      line_number=1335,
    },
    {
      name="logical_or_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="logical_and_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="OR_OR", is_token=true },
            { type="rule_reference", name="logical_and_expression", is_token=false },
          } } },
      } },
      line_number=1342,
    },
    {
      name="logical_and_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="bitwise_or_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="AND_AND", is_token=true },
            { type="rule_reference", name="bitwise_or_expression", is_token=false },
          } } },
      } },
      line_number=1348,
    },
    {
      name="bitwise_or_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="bitwise_xor_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="PIPE", is_token=true },
            { type="rule_reference", name="bitwise_xor_expression", is_token=false },
          } } },
      } },
      line_number=1352,
    },
    {
      name="bitwise_xor_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="bitwise_and_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="CARET", is_token=true },
            { type="rule_reference", name="bitwise_and_expression", is_token=false },
          } } },
      } },
      line_number=1356,
    },
    {
      name="bitwise_and_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="equality_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="AMPERSAND", is_token=true },
            { type="rule_reference", name="equality_expression", is_token=false },
          } } },
      } },
      line_number=1360,
    },
    {
      name="equality_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="relational_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="EQUALS_EQUALS", is_token=true },
                { type="rule_reference", name="NOT_EQUALS", is_token=true },
              } } },
            { type="rule_reference", name="relational_expression", is_token=false },
          } } },
      } },
      line_number=1364,
    },
    {
      name="relational_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="shift_expression", is_token=false },
        { type="repetition", element={ type="alternation", choices={
            { type="sequence", elements={
              { type="group", element={ type="alternation", choices={
                  { type="rule_reference", name="LESS_THAN", is_token=true },
                  { type="rule_reference", name="GREATER_THAN", is_token=true },
                  { type="rule_reference", name="LESS_EQUALS", is_token=true },
                  { type="rule_reference", name="GREATER_EQUALS", is_token=true },
                } } },
              { type="rule_reference", name="shift_expression", is_token=false },
            } },
            { type="sequence", elements={
              { type="literal", value="instanceof" },
              { type="rule_reference", name="type", is_token=false },
            } },
          } } },
      } },
      line_number=1372,
    },
    {
      name="shift_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="additive_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="LEFT_SHIFT", is_token=true },
                { type="rule_reference", name="RIGHT_SHIFT", is_token=true },
                { type="rule_reference", name="UNSIGNED_RIGHT_SHIFT", is_token=true },
              } } },
            { type="rule_reference", name="additive_expression", is_token=false },
          } } },
      } },
      line_number=1379,
    },
    {
      name="additive_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="multiplicative_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="PLUS", is_token=true },
                { type="rule_reference", name="MINUS", is_token=true },
              } } },
            { type="rule_reference", name="multiplicative_expression", is_token=false },
          } } },
      } },
      line_number=1384,
    },
    {
      name="multiplicative_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="unary_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="STAR", is_token=true },
                { type="rule_reference", name="SLASH", is_token=true },
                { type="rule_reference", name="PERCENT", is_token=true },
              } } },
            { type="rule_reference", name="unary_expression", is_token=false },
          } } },
      } },
      line_number=1389,
    },
    {
      name="unary_expression",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="PLUS_PLUS", is_token=true },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="MINUS_MINUS", is_token=true },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="PLUS", is_token=true },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="MINUS", is_token=true },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
        { type="rule_reference", name="unary_expression_not_plus_minus", is_token=false },
      } },
      line_number=1394,
    },
    {
      name="unary_expression_not_plus_minus",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="TILDE", is_token=true },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="BANG", is_token=true },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
        { type="rule_reference", name="cast_expression", is_token=false },
        { type="rule_reference", name="postfix_expression", is_token=false },
      } },
      line_number=1400,
    },
    {
      name="cast_expression",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="primitive_type", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="LBRACKET", is_token=true },
              { type="rule_reference", name="RBRACKET", is_token=true },
            } } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="class_type", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="AMPERSAND", is_token=true },
              { type="rule_reference", name="class_type", is_token=false },
            } } },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="LBRACKET", is_token=true },
              { type="rule_reference", name="RBRACKET", is_token=true },
            } } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="rule_reference", name="unary_expression_not_plus_minus", is_token=false },
        } },
      } },
      line_number=1414,
    },
    {
      name="postfix_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="primary_expression", is_token=false },
        { type="repetition", element={ type="alternation", choices={
            { type="rule_reference", name="PLUS_PLUS", is_token=true },
            { type="rule_reference", name="MINUS_MINUS", is_token=true },
          } } },
      } },
      line_number=1420,
    },
    {
      name="primary_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="primary", is_token=false },
        { type="repetition", element={ type="rule_reference", name="primary_suffix", is_token=false } },
      } },
      line_number=1442,
    },
    {
      name="primary_suffix",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="DOUBLE_COLON", is_token=true },
          { type="optional", element={ type="rule_reference", name="type_arguments", is_token=false } },
          { type="group", element={ type="alternation", choices={
              { type="rule_reference", name="NAME", is_token=true },
              { type="literal", value="new" },
            } } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="DOT", is_token=true },
          { type="optional", element={ type="rule_reference", name="type_arguments", is_token=false } },
          { type="rule_reference", name="NAME", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="DOT", is_token=true },
          { type="literal", value="class" },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="DOT", is_token=true },
          { type="literal", value="this" },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="DOT", is_token=true },
          { type="literal", value="super" },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="DOT", is_token=true },
          { type="literal", value="new" },
          { type="optional", element={ type="rule_reference", name="type_arguments", is_token=false } },
          { type="rule_reference", name="NAME", is_token=true },
          { type="optional", element={ type="rule_reference", name="type_arguments", is_token=false } },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="argument_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="class_body", is_token=false } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="argument_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LBRACKET", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="RBRACKET", is_token=true },
        } },
      } },
      line_number=1448,
    },
    {
      name="primary",
      body={ type="alternation", choices={
        { type="rule_reference", name="switch_expression", is_token=false },
        { type="rule_reference", name="literal", is_token=false },
        { type="literal", value="this" },
        { type="sequence", elements={
          { type="literal", value="super" },
          { type="rule_reference", name="DOUBLE_COLON", is_token=true },
          { type="optional", element={ type="rule_reference", name="type_arguments", is_token=false } },
          { type="group", element={ type="alternation", choices={
              { type="rule_reference", name="NAME", is_token=true },
              { type="literal", value="new" },
            } } },
        } },
        { type="sequence", elements={
          { type="literal", value="super" },
          { type="rule_reference", name="DOT", is_token=true },
          { type="optional", element={ type="rule_reference", name="type_arguments", is_token=false } },
          { type="rule_reference", name="NAME", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="super" },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="argument_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="new" },
          { type="optional", element={ type="rule_reference", name="type_arguments", is_token=false } },
          { type="rule_reference", name="class_type", is_token=false },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="argument_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="class_body", is_token=false } },
        } },
        { type="sequence", elements={
          { type="literal", value="new" },
          { type="rule_reference", name="array_creation_type", is_token=false },
          { type="rule_reference", name="array_dimension_exprs", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="LBRACKET", is_token=true },
              { type="rule_reference", name="RBRACKET", is_token=true },
            } } },
        } },
        { type="sequence", elements={
          { type="literal", value="new" },
          { type="rule_reference", name="array_creation_type", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="LBRACKET", is_token=true },
              { type="rule_reference", name="RBRACKET", is_token=true },
            } } },
          { type="rule_reference", name="array_initializer", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=1466,
    },
    {
      name="switch_expression",
      body={ type="sequence", elements={
        { type="literal", value="switch" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="switch_block", is_token=false },
      } },
      line_number=1509,
    },
    {
      name="argument_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
          } } },
      } },
      line_number=1511,
    },
    {
      name="array_creation_type",
      body={ type="alternation", choices={
        { type="rule_reference", name="primitive_type", is_token=false },
        { type="rule_reference", name="class_type", is_token=false },
      } },
      line_number=1519,
    },
    {
      name="array_dimension_exprs",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RBRACKET", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="LBRACKET", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
            { type="rule_reference", name="RBRACKET", is_token=true },
          } } },
      } },
      line_number=1522,
    },
    {
      name="literal",
      body={ type="alternation", choices={
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="rule_reference", name="CHAR", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="literal", value="true" },
        { type="literal", value="false" },
        { type="literal", value="null" },
      } },
      line_number=1551,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
