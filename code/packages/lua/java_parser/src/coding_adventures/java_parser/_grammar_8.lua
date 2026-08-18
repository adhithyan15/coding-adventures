-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: java8.grammar
-- Regenerate with: grammar-tools compile-grammar java8.grammar
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
      line_number=167,
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
      line_number=168,
    },
    {
      name="compilation_unit",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="optional", element={ type="rule_reference", name="package_declaration", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="import_declaration", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="type_declaration", is_token=false } },
      } },
      line_number=169,
    },
    {
      name="package_declaration",
      body={ type="sequence", elements={
        { type="literal", value="package" },
        { type="rule_reference", name="qualified_name", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=192,
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
      line_number=211,
    },
    {
      name="type_declaration",
      body={ type="alternation", choices={
        { type="rule_reference", name="class_declaration", is_token=false },
        { type="rule_reference", name="interface_declaration", is_token=false },
        { type="rule_reference", name="enum_declaration", is_token=false },
        { type="rule_reference", name="annotation_type_declaration", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=231,
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
      line_number=251,
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
      line_number=288,
    },
    {
      name="annotations",
      body={ type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
      line_number=293,
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
      line_number=298,
    },
    {
      name="element_value_pair",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="element_value", is_token=false },
      } },
      line_number=300,
    },
    {
      name="element_value",
      body={ type="alternation", choices={
        { type="rule_reference", name="annotation", is_token=false },
        { type="rule_reference", name="element_value_array", is_token=false },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=314,
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
      line_number=320,
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
      line_number=340,
    },
    {
      name="annotation_type_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="annotation_type_element_declaration", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=343,
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
      line_number=345,
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
      line_number=353,
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
      line_number=375,
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
      line_number=380,
    },
    {
      name="class_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="class_body_declaration", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=392,
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
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=394,
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
      line_number=467,
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
      line_number=471,
    },
    {
      name="interface_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="interface_body_declaration", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=479,
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
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=488,
    },
    {
      name="interface_field_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="field_modifier", is_token=false } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="variable_declarators", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=499,
    },
    {
      name="interface_method_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="interface_method_modifier", is_token=false } },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="result_type", is_token=false },
        { type="rule_reference", name="method_declarator", is_token=false },
        { type="optional", element={ type="rule_reference", name="throws_clause", is_token=false } },
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="block", is_token=false },
            { type="rule_reference", name="SEMICOLON", is_token=true },
          } } },
      } },
      line_number=528,
    },
    {
      name="interface_method_modifier",
      body={ type="alternation", choices={
        { type="rule_reference", name="annotation", is_token=false },
        { type="literal", value="public" },
        { type="literal", value="abstract" },
        { type="literal", value="default" },
        { type="literal", value="static" },
        { type="literal", value="strictfp" },
      } },
      line_number=532,
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
      line_number=543,
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
      line_number=586,
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
      line_number=590,
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
      line_number=592,
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
      line_number=594,
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
      line_number=649,
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
      line_number=654,
    },
    {
      name="bound",
      body={ type="sequence", elements={
        { type="rule_reference", name="annotated_type", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="AMPERSAND", is_token=true },
            { type="rule_reference", name="annotated_type", is_token=false },
          } } },
      } },
      line_number=659,
    },
    {
      name="type_arguments",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="LESS_THAN", is_token=true },
          { type="rule_reference", name="GREATER_THAN", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LESS_THAN", is_token=true },
          { type="rule_reference", name="type_argument", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="type_argument", is_token=false },
            } } },
          { type="rule_reference", name="GREATER_THAN", is_token=true },
        } },
      } },
      line_number=666,
    },
    {
      name="type_argument",
      body={ type="alternation", choices={
        { type="rule_reference", name="annotated_type", is_token=false },
        { type="sequence", elements={
          { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
          { type="rule_reference", name="QUESTION", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="group", element={ type="alternation", choices={
                  { type="literal", value="extends" },
                  { type="literal", value="super" },
                } } },
              { type="rule_reference", name="annotated_type", is_token=false },
            } } },
        } },
      } },
      line_number=672,
    },
    {
      name="annotated_type",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="rule_reference", name="type", is_token=false },
      } },
      line_number=717,
    },
    {
      name="field_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="field_modifier", is_token=false } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="variable_declarators", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=737,
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
      line_number=739,
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
      line_number=753,
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
      line_number=755,
    },
    {
      name="variable_initializer",
      body={ type="alternation", choices={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="array_initializer", is_token=false },
      } },
      line_number=757,
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
      line_number=763,
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
      line_number=788,
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
      line_number=791,
    },
    {
      name="result_type",
      body={ type="alternation", choices={
        { type="literal", value="void" },
        { type="rule_reference", name="type", is_token=false },
      } },
      line_number=802,
    },
    {
      name="method_declarator",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="receiver_parameter", is_token=false },
            { type="rule_reference", name="COMMA", is_token=true },
          } } },
        { type="optional", element={ type="rule_reference", name="formal_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="LBRACKET", is_token=true },
            { type="rule_reference", name="RBRACKET", is_token=true },
          } } },
      } },
      line_number=813,
    },
    {
      name="receiver_parameter",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="rule_reference", name="type", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="DOT", is_token=true },
          } } },
        { type="literal", value="this" },
      } },
      line_number=824,
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
      line_number=841,
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
      line_number=845,
    },
    {
      name="varargs_parameter",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="optional", element={ type="literal", value="final" } },
        { type="rule_reference", name="type", is_token=false },
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="rule_reference", name="ELLIPSIS", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=847,
    },
    {
      name="throws_clause",
      body={ type="sequence", elements={
        { type="literal", value="throws" },
        { type="rule_reference", name="annotated_type", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="annotated_type", is_token=false },
          } } },
      } },
      line_number=854,
    },
    {
      name="method_body",
      body={ type="alternation", choices={
        { type="rule_reference", name="block", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=858,
    },
    {
      name="constructor_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="constructor_modifier", is_token=false } },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="receiver_parameter", is_token=false },
            { type="rule_reference", name="COMMA", is_token=true },
          } } },
        { type="optional", element={ type="rule_reference", name="formal_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="throws_clause", is_token=false } },
        { type="rule_reference", name="constructor_body", is_token=false },
      } },
      line_number=879,
    },
    {
      name="constructor_modifier",
      body={ type="alternation", choices={
        { type="rule_reference", name="annotation", is_token=false },
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="private" },
      } },
      line_number=883,
    },
    {
      name="constructor_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="rule_reference", name="explicit_constructor_invocation", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="block_statement", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=888,
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
      line_number=895,
    },
    {
      name="static_initializer",
      body={ type="sequence", elements={
        { type="literal", value="static" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=924,
    },
    {
      name="instance_initializer",
      body={ type="rule_reference", name="block", is_token=false },
      line_number=926,
    },
    {
      name="type",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="primitive_type", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="LBRACKET", is_token=true },
              { type="rule_reference", name="RBRACKET", is_token=true },
            } } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="class_type", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="LBRACKET", is_token=true },
              { type="rule_reference", name="RBRACKET", is_token=true },
            } } },
        } },
      } },
      line_number=955,
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
      line_number=964,
    },
    {
      name="class_type",
      body={ type="sequence", elements={
        { type="rule_reference", name="qualified_name", is_token=false },
        { type="optional", element={ type="rule_reference", name="type_arguments", is_token=false } },
      } },
      line_number=985,
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
        { type="rule_reference", name="try_with_resources_statement", is_token=false },
        { type="rule_reference", name="throw_statement", is_token=false },
        { type="rule_reference", name="return_statement", is_token=false },
        { type="rule_reference", name="break_statement", is_token=false },
        { type="rule_reference", name="continue_statement", is_token=false },
        { type="rule_reference", name="synchronized_statement", is_token=false },
        { type="rule_reference", name="assert_statement", is_token=false },
        { type="rule_reference", name="labelled_statement", is_token=false },
      } },
      line_number=1007,
    },
    {
      name="block",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="block_statement", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=1031,
    },
    {
      name="block_statement",
      body={ type="alternation", choices={
        { type="rule_reference", name="var_declaration", is_token=false },
        { type="rule_reference", name="class_declaration", is_token=false },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=1033,
    },
    {
      name="var_declaration",
      body={ type="rule_reference", name="local_variable_declaration_statement", is_token=false },
      line_number=1047,
    },
    {
      name="local_variable_declaration_statement",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="optional", element={ type="literal", value="final" } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="variable_declarators", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1049,
    },
    {
      name="empty_statement",
      body={ type="rule_reference", name="SEMICOLON", is_token=true },
      line_number=1053,
    },
    {
      name="expression_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1060,
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
      line_number=1066,
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
      line_number=1070,
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
      line_number=1074,
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
      line_number=1082,
    },
    {
      name="for_init",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
          { type="optional", element={ type="literal", value="final" } },
          { type="rule_reference", name="type", is_token=false },
          { type="rule_reference", name="variable_declarators", is_token=false },
        } },
        { type="optional", element={ type="rule_reference", name="expression_list", is_token=false } },
      } },
      line_number=1085,
    },
    {
      name="for_update",
      body={ type="rule_reference", name="expression_list", is_token=false },
      line_number=1088,
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
      line_number=1090,
    },
    {
      name="enhanced_for_statement",
      body={ type="sequence", elements={
        { type="literal", value="for" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="optional", element={ type="literal", value="final" } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=1109,
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
      line_number=1132,
    },
    {
      name="switch_block",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="switch_block_statement_group", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="switch_label", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=1134,
    },
    {
      name="switch_block_statement_group",
      body={ type="sequence", elements={
        { type="rule_reference", name="switch_label", is_token=false },
        { type="repetition", element={ type="rule_reference", name="switch_label", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="block_statement", is_token=false } },
      } },
      line_number=1136,
    },
    {
      name="switch_label",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="case" },
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="COLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="default" },
          { type="rule_reference", name="COLON", is_token=true },
        } },
      } },
      line_number=1138,
    },
    {
      name="try_statement",
      body={ type="sequence", elements={
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
      line_number=1174,
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
      line_number=1180,
    },
    {
      name="catch_formal_parameter",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="optional", element={ type="literal", value="final" } },
        { type="rule_reference", name="catch_type", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="PIPE", is_token=true },
            { type="rule_reference", name="catch_type", is_token=false },
          } } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=1187,
    },
    {
      name="catch_type",
      body={ type="rule_reference", name="class_type", is_token=false },
      line_number=1189,
    },
    {
      name="finally_clause",
      body={ type="sequence", elements={
        { type="literal", value="finally" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=1191,
    },
    {
      name="try_with_resources_statement",
      body={ type="sequence", elements={
        { type="literal", value="try" },
        { type="rule_reference", name="resource_specification", is_token=false },
        { type="rule_reference", name="block", is_token=false },
        { type="repetition", element={ type="rule_reference", name="catch_clause", is_token=false } },
        { type="optional", element={ type="rule_reference", name="finally_clause", is_token=false } },
      } },
      line_number=1233,
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
      line_number=1246,
    },
    {
      name="resource",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="annotation", is_token=false } },
        { type="optional", element={ type="literal", value="final" } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=1248,
    },
    {
      name="throw_statement",
      body={ type="sequence", elements={
        { type="literal", value="throw" },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1252,
    },
    {
      name="return_statement",
      body={ type="sequence", elements={
        { type="literal", value="return" },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1256,
    },
    {
      name="break_statement",
      body={ type="sequence", elements={
        { type="literal", value="break" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1269,
    },
    {
      name="continue_statement",
      body={ type="sequence", elements={
        { type="literal", value="continue" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1271,
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
      line_number=1275,
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
      line_number=1282,
    },
    {
      name="labelled_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=1286,
    },
    {
      name="expression",
      body={ type="rule_reference", name="assignment_expression", is_token=false },
      line_number=1402,
    },
    {
      name="assignment_expression",
      body={ type="alternation", choices={
        { type="rule_reference", name="lambda_expression", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="conditional_expression", is_token=false },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="assignment_operator", is_token=false },
              { type="rule_reference", name="assignment_expression", is_token=false },
            } } },
        } },
      } },
      line_number=1404,
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
      line_number=1408,
    },
    {
      name="lambda_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="lambda_parameters", is_token=false },
        { type="rule_reference", name="ARROW", is_token=true },
        { type="rule_reference", name="lambda_body", is_token=false },
      } },
      line_number=1506,
    },
    {
      name="lambda_parameters",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="formal_parameter_list", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="inferred_parameter_list", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=1520,
    },
    {
      name="inferred_parameter_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="COMMA", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
      } },
      line_number=1534,
    },
    {
      name="lambda_body",
      body={ type="alternation", choices={
        { type="rule_reference", name="block", is_token=false },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=1546,
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
      line_number=1615,
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
      line_number=1623,
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
      line_number=1629,
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
      line_number=1633,
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
      line_number=1637,
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
      line_number=1641,
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
      line_number=1647,
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
              { type="rule_reference", name="annotated_type", is_token=false },
            } },
          } } },
      } },
      line_number=1658,
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
      line_number=1665,
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
      line_number=1670,
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
      line_number=1675,
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
      line_number=1682,
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
      line_number=1688,
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
          { type="rule_reference", name="annotated_type", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="AMPERSAND", is_token=true },
              { type="rule_reference", name="annotated_type", is_token=false },
            } } },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="LBRACKET", is_token=true },
              { type="rule_reference", name="RBRACKET", is_token=true },
            } } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="rule_reference", name="unary_expression_not_plus_minus", is_token=false },
        } },
      } },
      line_number=1712,
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
      line_number=1718,
    },
    {
      name="primary_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="primary", is_token=false },
        { type="repetition", element={ type="rule_reference", name="primary_suffix", is_token=false } },
      } },
      line_number=1737,
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
      line_number=1756,
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
        { type="sequence", elements={
          { type="rule_reference", name="primitive_type", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="LBRACKET", is_token=true },
              { type="rule_reference", name="RBRACKET", is_token=true },
            } } },
          { type="rule_reference", name="DOT", is_token=true },
          { type="literal", value="class" },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="primitive_type", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="LBRACKET", is_token=true },
              { type="rule_reference", name="RBRACKET", is_token=true },
            } } },
          { type="rule_reference", name="DOUBLE_COLON", is_token=true },
          { type="literal", value="new" },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=1789,
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
      line_number=1804,
    },
    {
      name="array_creation_type",
      body={ type="alternation", choices={
        { type="rule_reference", name="primitive_type", is_token=false },
        { type="rule_reference", name="class_type", is_token=false },
      } },
      line_number=1814,
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
      line_number=1817,
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
      line_number=1837,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
