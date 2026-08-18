-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: java17.grammar
-- Regenerate with: grammar-tools compile-grammar java17.grammar
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
      line_number=231,
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
      line_number=232,
    },
    {
      name="compilation_unit",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="optional", element={ type="rule_reference", name="package_declaration", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="import_declaration", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="type_declaration", is_token=false } },
      } },
      line_number=233,
    },
    {
      name="package_declaration",
      body={ type="sequence", elements={
        { type="literal", value="package" },
        { type="rule_reference", name="qualified_name", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=249,
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
      line_number=264,
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
      line_number=280,
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
      line_number=299,
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
      line_number=322,
    },
    {
      name="annotations",
      body={ type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
      line_number=324,
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
      line_number=326,
    },
    {
      name="element_value_pair",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="element_value", is_token=false },
      } },
      line_number=328,
    },
    {
      name="element_value",
      body={ type="alternation", choices={
        { type="rule_reference", name="annotation", is_token=false },
        { type="rule_reference", name="element_value_array", is_token=false },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=336,
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
      line_number=340,
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
      line_number=356,
    },
    {
      name="annotation_type_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="annotation_type_element_declaration", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=359,
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
      line_number=361,
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
      line_number=369,
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
        { type="optional", element={ type="rule_reference", name="permits_clause", is_token=false } },
        { type="rule_reference", name="class_body", is_token=false },
      } },
      line_number=406,
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
        { type="rule_reference", name="non_sealed_modifier", is_token=false },
        { type="literal", value="sealed" },
      } },
      line_number=431,
    },
    {
      name="non_sealed_modifier",
      body={ type="sequence", elements={
        { type="literal", value="non" },
        { type="rule_reference", name="MINUS", is_token=true },
        { type="literal", value="sealed" },
      } },
      line_number=458,
    },
    {
      name="permits_clause",
      body={ type="sequence", elements={
        { type="literal", value="permits" },
        { type="rule_reference", name="class_type", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="class_type", is_token=false },
          } } },
      } },
      line_number=471,
    },
    {
      name="class_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="class_body_declaration", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=473,
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
      line_number=475,
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
        { type="optional", element={ type="rule_reference", name="permits_clause", is_token=false } },
        { type="rule_reference", name="interface_body", is_token=false },
      } },
      line_number=523,
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
        { type="rule_reference", name="non_sealed_modifier", is_token=false },
        { type="literal", value="sealed" },
      } },
      line_number=528,
    },
    {
      name="interface_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="interface_body_declaration", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=538,
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
      line_number=544,
    },
    {
      name="interface_field_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="field_modifier", is_token=false } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="variable_declarators", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=553,
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
      line_number=576,
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
      line_number=579,
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
      line_number=587,
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
      line_number=607,
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
      line_number=611,
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
      line_number=613,
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
      line_number=615,
    },
    {
      name="record_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="class_modifier", is_token=false } },
        { type="literal", value="record" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="record_components", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="implements" },
            { type="rule_reference", name="interface_type_list", is_token=false },
          } } },
        { type="rule_reference", name="record_body", is_token=false },
      } },
      line_number=674,
    },
    {
      name="record_components",
      body={ type="sequence", elements={
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="record_component_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=689,
    },
    {
      name="record_component_list",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="record_component", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="record_component", is_token=false },
            } } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="record_component", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="record_component", is_token=false },
            } } },
          { type="rule_reference", name="COMMA", is_token=true },
          { type="rule_reference", name="varargs_record_component", is_token=false },
        } },
        { type="rule_reference", name="varargs_record_component", is_token=false },
      } },
      line_number=691,
    },
    {
      name="record_component",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=695,
    },
    {
      name="varargs_record_component",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="ELLIPSIS", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=697,
    },
    {
      name="record_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="record_body_declaration", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=718,
    },
    {
      name="record_body_declaration",
      body={ type="alternation", choices={
        { type="rule_reference", name="compact_constructor_declaration", is_token=false },
        { type="rule_reference", name="constructor_declaration", is_token=false },
        { type="rule_reference", name="method_declaration", is_token=false },
        { type="rule_reference", name="field_declaration", is_token=false },
        { type="rule_reference", name="class_declaration", is_token=false },
        { type="rule_reference", name="interface_declaration", is_token=false },
        { type="rule_reference", name="enum_declaration", is_token=false },
        { type="rule_reference", name="annotation_type_declaration", is_token=false },
        { type="rule_reference", name="record_declaration", is_token=false },
        { type="rule_reference", name="static_initializer", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=720,
    },
    {
      name="compact_constructor_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="constructor_modifier", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=740,
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
      line_number=770,
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
      line_number=772,
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
      line_number=774,
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
      line_number=788,
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
      line_number=790,
    },
    {
      name="field_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="field_modifier", is_token=false } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="variable_declarators", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=808,
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
      line_number=810,
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
      line_number=819,
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
      line_number=821,
    },
    {
      name="variable_initializer",
      body={ type="alternation", choices={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="array_initializer", is_token=false },
      } },
      line_number=823,
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
      line_number=826,
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
      line_number=843,
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
      line_number=846,
    },
    {
      name="result_type",
      body={ type="alternation", choices={
        { type="literal", value="void" },
        { type="rule_reference", name="type", is_token=false },
      } },
      line_number=857,
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
      line_number=860,
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
      line_number=877,
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
      line_number=881,
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
      line_number=883,
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
      line_number=885,
    },
    {
      name="method_body",
      body={ type="alternation", choices={
        { type="rule_reference", name="block", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=887,
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
      line_number=904,
    },
    {
      name="constructor_modifier",
      body={ type="alternation", choices={
        { type="rule_reference", name="annotation", is_token=false },
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="private" },
      } },
      line_number=908,
    },
    {
      name="constructor_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="rule_reference", name="explicit_constructor_invocation", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="block_statement", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=913,
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
      line_number=921,
    },
    {
      name="static_initializer",
      body={ type="sequence", elements={
        { type="literal", value="static" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=933,
    },
    {
      name="instance_initializer",
      body={ type="rule_reference", name="block", is_token=false },
      line_number=935,
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
      line_number=965,
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
      line_number=968,
    },
    {
      name="class_type",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="rule_reference", name="qualified_name", is_token=false },
        { type="optional", element={ type="rule_reference", name="type_arguments", is_token=false } },
      } },
      line_number=982,
    },
    {
      name="local_var_type",
      body={ type="alternation", choices={
        { type="rule_reference", name="type", is_token=false },
        { type="literal", value="var" },
      } },
      line_number=1003,
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
      line_number=1041,
    },
    {
      name="block",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="block_statement", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=1063,
    },
    {
      name="block_statement",
      body={ type="alternation", choices={
        { type="rule_reference", name="var_declaration", is_token=false },
        { type="rule_reference", name="class_declaration", is_token=false },
        { type="rule_reference", name="record_declaration", is_token=false },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=1065,
    },
    {
      name="var_declaration",
      body={ type="rule_reference", name="local_variable_declaration_statement", is_token=false },
      line_number=1083,
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
      line_number=1085,
    },
    {
      name="empty_statement",
      body={ type="rule_reference", name="SEMICOLON", is_token=true },
      line_number=1090,
    },
    {
      name="expression_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1094,
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
      line_number=1101,
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
      line_number=1105,
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
      line_number=1109,
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
      line_number=1113,
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
      line_number=1116,
    },
    {
      name="for_update",
      body={ type="rule_reference", name="expression_list", is_token=false },
      line_number=1119,
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
      line_number=1121,
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
      line_number=1129,
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
      line_number=1182,
    },
    {
      name="switch_block",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="switch_rule", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=1188,
    },
    {
      name="switch_rule",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="switch_label", is_token=false },
          { type="rule_reference", name="COLON", is_token=true },
          { type="repetition", element={ type="rule_reference", name="block_statement", is_token=false } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="switch_label", is_token=false },
          { type="rule_reference", name="ARROW", is_token=true },
          { type="rule_reference", name="switch_rule_body", is_token=false },
        } },
      } },
      line_number=1199,
    },
    {
      name="switch_rule_body",
      body={ type="alternation", choices={
        { type="rule_reference", name="block", is_token=false },
        { type="sequence", elements={
          { type="literal", value="throw" },
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
      } },
      line_number=1202,
    },
    {
      name="switch_label",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="case" },
          { type="rule_reference", name="case_constant", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="case_constant", is_token=false },
            } } },
        } },
        { type="literal", value="default" },
      } },
      line_number=1215,
    },
    {
      name="case_constant",
      body={ type="rule_reference", name="expression", is_token=false },
      line_number=1218,
    },
    {
      name="yield_statement",
      body={ type="sequence", elements={
        { type="literal", value="yield" },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1240,
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
      line_number=1256,
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
      line_number=1260,
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
      line_number=1262,
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
      line_number=1267,
    },
    {
      name="catch_formal_parameter",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="optional", element={ type="literal", value="final" } },
        { type="rule_reference", name="catch_type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=1269,
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
      line_number=1271,
    },
    {
      name="finally_clause",
      body={ type="sequence", elements={
        { type="literal", value="finally" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=1273,
    },
    {
      name="throw_statement",
      body={ type="sequence", elements={
        { type="literal", value="throw" },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1277,
    },
    {
      name="return_statement",
      body={ type="sequence", elements={
        { type="literal", value="return" },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1281,
    },
    {
      name="break_statement",
      body={ type="sequence", elements={
        { type="literal", value="break" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1285,
    },
    {
      name="continue_statement",
      body={ type="sequence", elements={
        { type="literal", value="continue" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1287,
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
      line_number=1291,
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
      line_number=1295,
    },
    {
      name="labelled_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=1299,
    },
    {
      name="expression",
      body={ type="alternation", choices={
        { type="rule_reference", name="lambda_expression", is_token=false },
        { type="rule_reference", name="assignment_expression", is_token=false },
      } },
      line_number=1346,
    },
    {
      name="lambda_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="lambda_parameters", is_token=false },
        { type="rule_reference", name="ARROW", is_token=true },
        { type="rule_reference", name="lambda_body", is_token=false },
      } },
      line_number=1349,
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
      line_number=1351,
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
      line_number=1361,
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
          { type="literal", value="var" },
          { type="rule_reference", name="NAME", is_token=true },
        } },
        { type="sequence", elements={
          { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
          { type="optional", element={ type="literal", value="final" } },
          { type="rule_reference", name="NAME", is_token=true },
        } },
      } },
      line_number=1363,
    },
    {
      name="lambda_body",
      body={ type="alternation", choices={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=1367,
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
      line_number=1374,
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
      line_number=1377,
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
      line_number=1392,
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
      line_number=1397,
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
      line_number=1401,
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
      line_number=1405,
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
      line_number=1409,
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
      line_number=1413,
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
      line_number=1417,
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
              { type="rule_reference", name="pattern", is_token=false },
            } },
          } } },
      } },
      line_number=1461,
    },
    {
      name="pattern",
      body={ type="rule_reference", name="type_pattern", is_token=false },
      line_number=1492,
    },
    {
      name="type_pattern",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="optional", element={ type="literal", value="final" } },
          { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
          { type="rule_reference", name="type", is_token=false },
          { type="rule_reference", name="NAME", is_token=true },
        } },
        { type="rule_reference", name="type", is_token=false },
      } },
      line_number=1494,
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
      line_number=1499,
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
      line_number=1504,
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
      line_number=1509,
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
      line_number=1514,
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
      line_number=1520,
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
      line_number=1533,
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
      line_number=1539,
    },
    {
      name="primary_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="primary", is_token=false },
        { type="repetition", element={ type="rule_reference", name="primary_suffix", is_token=false } },
      } },
      line_number=1560,
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
      line_number=1566,
    },
    {
      name="primary",
      body={ type="alternation", choices={
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
        { type="rule_reference", name="switch_expression", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=1596,
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
      line_number=1608,
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
      line_number=1620,
    },
    {
      name="array_creation_type",
      body={ type="alternation", choices={
        { type="rule_reference", name="primitive_type", is_token=false },
        { type="rule_reference", name="class_type", is_token=false },
      } },
      line_number=1624,
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
      line_number=1627,
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
      line_number=1653,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
