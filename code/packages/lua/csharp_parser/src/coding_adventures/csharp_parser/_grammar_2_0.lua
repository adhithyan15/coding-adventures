-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: csharp2.0.grammar
-- Regenerate with: grammar-tools compile-grammar csharp2.0.grammar
--
-- This file embeds a ParserGrammar as native Lua data structures.
-- Call parser_grammar() instead of reading and parsing the .grammar file.

local gt = require("coding_adventures.grammar_tools")

local function parser_grammar()
  local g = gt.ParserGrammar.new()
  g.rules = {
    {
      name="compilation_unit",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="extern_alias_directive", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="using_directive", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="global_attribute_section", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="namespace_member_declaration", is_token=false } },
      } },
      line_number=73,
    },
    {
      name="extern_alias_directive",
      body={ type="sequence", elements={
        { type="literal", value="extern" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=87,
    },
    {
      name="using_directive",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="using" },
          { type="rule_reference", name="qualified_name", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="using" },
          { type="rule_reference", name="NAME", is_token=true },
          { type="rule_reference", name="EQUALS", is_token=true },
          { type="rule_reference", name="namespace_or_type_name", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
      } },
      line_number=102,
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
      line_number=131,
    },
    {
      name="namespace_or_type_name",
      body={ type="sequence", elements={
        { type="rule_reference", name="namespace_or_type_part", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="DOT", is_token=true },
            { type="rule_reference", name="namespace_or_type_part", is_token=false },
          } } },
      } },
      line_number=136,
    },
    {
      name="namespace_or_type_part",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="NAME", is_token=true },
          { type="optional", element={ type="rule_reference", name="type_argument_list", is_token=false } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="NAME", is_token=true },
          { type="rule_reference", name="COLON_COLON", is_token=true },
          { type="rule_reference", name="NAME", is_token=true },
          { type="optional", element={ type="rule_reference", name="type_argument_list", is_token=false } },
        } },
      } },
      line_number=138,
    },
    {
      name="type_parameter_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="LESS_THAN", is_token=true },
        { type="rule_reference", name="type_parameter", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="type_parameter", is_token=false },
          } } },
        { type="rule_reference", name="GREATER_THAN", is_token=true },
      } },
      line_number=166,
    },
    {
      name="type_parameter",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=168,
    },
    {
      name="type_argument_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="LESS_THAN", is_token=true },
        { type="rule_reference", name="type", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="type", is_token=false },
          } } },
        { type="rule_reference", name="GREATER_THAN", is_token=true },
      } },
      line_number=170,
    },
    {
      name="type_parameter_constraints_clauses",
      body={ type="sequence", elements={
        { type="rule_reference", name="type_parameter_constraints_clause", is_token=false },
        { type="repetition", element={ type="rule_reference", name="type_parameter_constraints_clause", is_token=false } },
      } },
      line_number=208,
    },
    {
      name="type_parameter_constraints_clause",
      body={ type="sequence", elements={
        { type="literal", value="where" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="type_parameter_constraints", is_token=false },
      } },
      line_number=211,
    },
    {
      name="type_parameter_constraints",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="primary_constraint", is_token=false },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="secondary_constraints", is_token=false },
            } } },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="constructor_constraint", is_token=false },
            } } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="secondary_constraints", is_token=false },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="constructor_constraint", is_token=false },
            } } },
        } },
        { type="rule_reference", name="constructor_constraint", is_token=false },
      } },
      line_number=213,
    },
    {
      name="primary_constraint",
      body={ type="alternation", choices={
        { type="literal", value="class" },
        { type="literal", value="struct" },
        { type="rule_reference", name="namespace_or_type_name", is_token=false },
      } },
      line_number=217,
    },
    {
      name="secondary_constraints",
      body={ type="sequence", elements={
        { type="rule_reference", name="namespace_or_type_name", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="namespace_or_type_name", is_token=false },
          } } },
      } },
      line_number=221,
    },
    {
      name="constructor_constraint",
      body={ type="sequence", elements={
        { type="literal", value="new" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=223,
    },
    {
      name="global_attribute_section",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="rule_reference", name="global_attribute_target", is_token=false },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="attribute_list", is_token=false },
        { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=231,
    },
    {
      name="global_attribute_target",
      body={ type="alternation", choices={
        { type="literal", value="assembly" },
        { type="literal", value="module" },
      } },
      line_number=233,
    },
    {
      name="namespace_declaration",
      body={ type="sequence", elements={
        { type="literal", value="namespace" },
        { type="rule_reference", name="qualified_name", is_token=false },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="extern_alias_directive", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="using_directive", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="namespace_member_declaration", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
        { type="optional", element={ type="rule_reference", name="SEMICOLON", is_token=true } },
      } },
      line_number=242,
    },
    {
      name="namespace_member_declaration",
      body={ type="alternation", choices={
        { type="rule_reference", name="namespace_declaration", is_token=false },
        { type="rule_reference", name="type_declaration", is_token=false },
      } },
      line_number=252,
    },
    {
      name="type_declaration",
      body={ type="alternation", choices={
        { type="rule_reference", name="class_declaration", is_token=false },
        { type="rule_reference", name="struct_declaration", is_token=false },
        { type="rule_reference", name="interface_declaration", is_token=false },
        { type="rule_reference", name="enum_declaration", is_token=false },
        { type="rule_reference", name="delegate_declaration", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=255,
    },
    {
      name="attribute_section",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="attribute_target", is_token=false },
            { type="rule_reference", name="COLON", is_token=true },
          } } },
        { type="rule_reference", name="attribute_list", is_token=false },
        { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=269,
    },
    {
      name="attribute_target",
      body={ type="alternation", choices={
        { type="literal", value="field" },
        { type="literal", value="event" },
        { type="literal", value="method" },
        { type="literal", value="param" },
        { type="literal", value="property" },
        { type="literal", value="return" },
        { type="literal", value="type" },
      } },
      line_number=271,
    },
    {
      name="attribute_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="attribute", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="attribute", is_token=false },
          } } },
      } },
      line_number=279,
    },
    {
      name="attribute",
      body={ type="sequence", elements={
        { type="rule_reference", name="namespace_or_type_name", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="optional", element={ type="rule_reference", name="attribute_arguments", is_token=false } },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
      } },
      line_number=281,
    },
    {
      name="attribute_arguments",
      body={ type="sequence", elements={
        { type="rule_reference", name="attribute_argument", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="attribute_argument", is_token=false },
          } } },
      } },
      line_number=283,
    },
    {
      name="attribute_argument",
      body={ type="sequence", elements={
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="EQUALS", is_token=true },
          } } },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=285,
    },
    {
      name="class_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="class_modifier", is_token=false } },
        { type="literal", value="class" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameter_list", is_token=false } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="class_base_list", is_token=false },
          } } },
        { type="optional", element={ type="rule_reference", name="type_parameter_constraints_clauses", is_token=false } },
        { type="rule_reference", name="class_body", is_token=false },
        { type="optional", element={ type="rule_reference", name="SEMICOLON", is_token=true } },
      } },
      line_number=324,
    },
    {
      name="class_modifier",
      body={ type="alternation", choices={
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="internal" },
        { type="literal", value="private" },
        { type="literal", value="new" },
        { type="literal", value="abstract" },
        { type="literal", value="sealed" },
        { type="literal", value="static" },
        { type="literal", value="partial" },
      } },
      line_number=330,
    },
    {
      name="class_base_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="namespace_or_type_name", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="namespace_or_type_name", is_token=false },
          } } },
      } },
      line_number=340,
    },
    {
      name="class_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="class_member_declaration", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=342,
    },
    {
      name="class_member_declaration",
      body={ type="alternation", choices={
        { type="rule_reference", name="constant_declaration", is_token=false },
        { type="rule_reference", name="field_declaration", is_token=false },
        { type="rule_reference", name="method_declaration", is_token=false },
        { type="rule_reference", name="property_declaration", is_token=false },
        { type="rule_reference", name="event_declaration", is_token=false },
        { type="rule_reference", name="indexer_declaration", is_token=false },
        { type="rule_reference", name="operator_declaration", is_token=false },
        { type="rule_reference", name="conversion_operator_declaration", is_token=false },
        { type="rule_reference", name="constructor_declaration", is_token=false },
        { type="rule_reference", name="destructor_declaration", is_token=false },
        { type="rule_reference", name="static_constructor_declaration", is_token=false },
        { type="rule_reference", name="type_declaration", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=351,
    },
    {
      name="constant_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="constant_modifier", is_token=false } },
        { type="literal", value="const" },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="constant_declarators", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=371,
    },
    {
      name="constant_modifier",
      body={ type="alternation", choices={
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="internal" },
        { type="literal", value="private" },
        { type="literal", value="new" },
      } },
      line_number=374,
    },
    {
      name="constant_declarators",
      body={ type="sequence", elements={
        { type="rule_reference", name="constant_declarator", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="constant_declarator", is_token=false },
          } } },
      } },
      line_number=380,
    },
    {
      name="constant_declarator",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=382,
    },
    {
      name="field_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="field_modifier", is_token=false } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="variable_declarators", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=391,
    },
    {
      name="field_modifier",
      body={ type="alternation", choices={
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="internal" },
        { type="literal", value="private" },
        { type="literal", value="new" },
        { type="literal", value="static" },
        { type="literal", value="readonly" },
        { type="literal", value="volatile" },
      } },
      line_number=394,
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
      line_number=403,
    },
    {
      name="variable_declarator",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="EQUALS", is_token=true },
            { type="rule_reference", name="variable_initializer", is_token=false },
          } } },
      } },
      line_number=405,
    },
    {
      name="variable_initializer",
      body={ type="alternation", choices={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="array_initializer", is_token=false },
      } },
      line_number=407,
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
      line_number=410,
    },
    {
      name="method_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="method_modifier", is_token=false } },
        { type="rule_reference", name="return_type", is_token=false },
        { type="rule_reference", name="namespace_or_type_name", is_token=false },
        { type="optional", element={ type="rule_reference", name="type_parameter_list", is_token=false } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="formal_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameter_constraints_clauses", is_token=false } },
        { type="rule_reference", name="method_body", is_token=false },
      } },
      line_number=436,
    },
    {
      name="method_modifier",
      body={ type="alternation", choices={
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="internal" },
        { type="literal", value="private" },
        { type="literal", value="new" },
        { type="literal", value="static" },
        { type="literal", value="virtual" },
        { type="literal", value="sealed" },
        { type="literal", value="override" },
        { type="literal", value="abstract" },
        { type="literal", value="extern" },
      } },
      line_number=442,
    },
    {
      name="return_type",
      body={ type="alternation", choices={
        { type="literal", value="void" },
        { type="rule_reference", name="type", is_token=false },
      } },
      line_number=454,
    },
    {
      name="method_body",
      body={ type="alternation", choices={
        { type="rule_reference", name="block", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=457,
    },
    {
      name="formal_parameter_list",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="fixed_parameters", is_token=false },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="parameter_array", is_token=false },
            } } },
        } },
        { type="rule_reference", name="parameter_array", is_token=false },
      } },
      line_number=467,
    },
    {
      name="fixed_parameters",
      body={ type="sequence", elements={
        { type="rule_reference", name="fixed_parameter", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="fixed_parameter", is_token=false },
          } } },
      } },
      line_number=470,
    },
    {
      name="fixed_parameter",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="optional", element={ type="rule_reference", name="parameter_modifier", is_token=false } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=472,
    },
    {
      name="parameter_modifier",
      body={ type="alternation", choices={
        { type="literal", value="ref" },
        { type="literal", value="out" },
      } },
      line_number=474,
    },
    {
      name="parameter_array",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="literal", value="params" },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=477,
    },
    {
      name="property_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="property_modifier", is_token=false } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="namespace_or_type_name", is_token=false },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="accessor_declarations", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=496,
    },
    {
      name="property_modifier",
      body={ type="alternation", choices={
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="internal" },
        { type="literal", value="private" },
        { type="literal", value="new" },
        { type="literal", value="static" },
        { type="literal", value="virtual" },
        { type="literal", value="sealed" },
        { type="literal", value="override" },
        { type="literal", value="abstract" },
        { type="literal", value="extern" },
      } },
      line_number=499,
    },
    {
      name="accessor_declarations",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="get_accessor_declaration", is_token=false },
          { type="optional", element={ type="rule_reference", name="set_accessor_declaration", is_token=false } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="set_accessor_declaration", is_token=false },
          { type="optional", element={ type="rule_reference", name="get_accessor_declaration", is_token=false } },
        } },
      } },
      line_number=511,
    },
    {
      name="get_accessor_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="optional", element={ type="rule_reference", name="accessor_modifier", is_token=false } },
        { type="literal", value="get" },
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="block", is_token=false },
            { type="rule_reference", name="SEMICOLON", is_token=true },
          } } },
      } },
      line_number=514,
    },
    {
      name="set_accessor_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="optional", element={ type="rule_reference", name="accessor_modifier", is_token=false } },
        { type="literal", value="set" },
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="block", is_token=false },
            { type="rule_reference", name="SEMICOLON", is_token=true },
          } } },
      } },
      line_number=517,
    },
    {
      name="accessor_modifier",
      body={ type="alternation", choices={
        { type="literal", value="protected" },
        { type="literal", value="internal" },
        { type="literal", value="private" },
        { type="sequence", elements={
          { type="literal", value="protected" },
          { type="literal", value="internal" },
        } },
        { type="sequence", elements={
          { type="literal", value="internal" },
          { type="literal", value="protected" },
        } },
      } },
      line_number=520,
    },
    {
      name="event_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="event_modifier", is_token=false } },
        { type="literal", value="event" },
        { type="rule_reference", name="type", is_token=false },
        { type="group", element={ type="alternation", choices={
            { type="sequence", elements={
              { type="rule_reference", name="variable_declarators", is_token=false },
              { type="rule_reference", name="SEMICOLON", is_token=true },
            } },
            { type="sequence", elements={
              { type="rule_reference", name="namespace_or_type_name", is_token=false },
              { type="rule_reference", name="LBRACE", is_token=true },
              { type="rule_reference", name="event_accessor_declarations", is_token=false },
              { type="rule_reference", name="RBRACE", is_token=true },
            } },
          } } },
      } },
      line_number=532,
    },
    {
      name="event_modifier",
      body={ type="alternation", choices={
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="internal" },
        { type="literal", value="private" },
        { type="literal", value="new" },
        { type="literal", value="static" },
        { type="literal", value="virtual" },
        { type="literal", value="sealed" },
        { type="literal", value="override" },
        { type="literal", value="abstract" },
        { type="literal", value="extern" },
      } },
      line_number=536,
    },
    {
      name="event_accessor_declarations",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="add_accessor_declaration", is_token=false },
          { type="rule_reference", name="remove_accessor_declaration", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="remove_accessor_declaration", is_token=false },
          { type="rule_reference", name="add_accessor_declaration", is_token=false },
        } },
      } },
      line_number=548,
    },
    {
      name="add_accessor_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="literal", value="add" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=551,
    },
    {
      name="remove_accessor_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="literal", value="remove" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=553,
    },
    {
      name="indexer_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="indexer_modifier", is_token=false } },
        { type="rule_reference", name="type", is_token=false },
        { type="literal", value="this" },
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="rule_reference", name="formal_parameter_list", is_token=false },
        { type="rule_reference", name="RBRACKET", is_token=true },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="accessor_declarations", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=561,
    },
    {
      name="indexer_modifier",
      body={ type="alternation", choices={
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="internal" },
        { type="literal", value="private" },
        { type="literal", value="new" },
        { type="literal", value="virtual" },
        { type="literal", value="sealed" },
        { type="literal", value="override" },
        { type="literal", value="abstract" },
        { type="literal", value="extern" },
      } },
      line_number=565,
    },
    {
      name="operator_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="rule_reference", name="operator_modifiers", is_token=false },
        { type="rule_reference", name="type", is_token=false },
        { type="literal", value="operator" },
        { type="rule_reference", name="overloadable_operator", is_token=false },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="type", is_token=false },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="method_body", is_token=false },
      } },
      line_number=582,
    },
    {
      name="operator_modifiers",
      body={ type="sequence", elements={
        { type="literal", value="public" },
        { type="literal", value="static" },
        { type="optional", element={ type="literal", value="extern" } },
      } },
      line_number=587,
    },
    {
      name="overloadable_operator",
      body={ type="alternation", choices={
        { type="rule_reference", name="PLUS", is_token=true },
        { type="rule_reference", name="MINUS", is_token=true },
        { type="rule_reference", name="BANG", is_token=true },
        { type="rule_reference", name="TILDE", is_token=true },
        { type="rule_reference", name="PLUS_PLUS", is_token=true },
        { type="rule_reference", name="MINUS_MINUS", is_token=true },
        { type="literal", value="true" },
        { type="literal", value="false" },
        { type="rule_reference", name="STAR", is_token=true },
        { type="rule_reference", name="SLASH", is_token=true },
        { type="rule_reference", name="PERCENT", is_token=true },
        { type="rule_reference", name="AMPERSAND", is_token=true },
        { type="rule_reference", name="PIPE", is_token=true },
        { type="rule_reference", name="CARET", is_token=true },
        { type="rule_reference", name="LEFT_SHIFT", is_token=true },
        { type="rule_reference", name="RIGHT_SHIFT", is_token=true },
        { type="rule_reference", name="EQUALS_EQUALS", is_token=true },
        { type="rule_reference", name="NOT_EQUALS", is_token=true },
        { type="rule_reference", name="LESS_THAN", is_token=true },
        { type="rule_reference", name="GREATER_THAN", is_token=true },
        { type="rule_reference", name="LESS_EQUALS", is_token=true },
        { type="rule_reference", name="GREATER_EQUALS", is_token=true },
      } },
      line_number=589,
    },
    {
      name="conversion_operator_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="rule_reference", name="operator_modifiers", is_token=false },
        { type="group", element={ type="alternation", choices={
            { type="literal", value="implicit" },
            { type="literal", value="explicit" },
          } } },
        { type="literal", value="operator" },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="method_body", is_token=false },
      } },
      line_number=618,
    },
    {
      name="constructor_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="constructor_modifier", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="formal_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="constructor_initializer", is_token=false } },
        { type="rule_reference", name="method_body", is_token=false },
      } },
      line_number=628,
    },
    {
      name="constructor_modifier",
      body={ type="alternation", choices={
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="internal" },
        { type="literal", value="private" },
        { type="literal", value="extern" },
      } },
      line_number=632,
    },
    {
      name="constructor_initializer",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="COLON", is_token=true },
          { type="literal", value="base" },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="argument_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="COLON", is_token=true },
          { type="literal", value="this" },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="argument_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
      } },
      line_number=638,
    },
    {
      name="destructor_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="optional", element={ type="literal", value="extern" } },
        { type="rule_reference", name="TILDE", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="method_body", is_token=false },
      } },
      line_number=647,
    },
    {
      name="static_constructor_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="rule_reference", name="static_constructor_modifiers", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="method_body", is_token=false },
      } },
      line_number=656,
    },
    {
      name="static_constructor_modifiers",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="static" },
          { type="optional", element={ type="literal", value="extern" } },
        } },
        { type="sequence", elements={
          { type="literal", value="extern" },
          { type="literal", value="static" },
        } },
      } },
      line_number=659,
    },
    {
      name="struct_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="struct_modifier", is_token=false } },
        { type="literal", value="struct" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameter_list", is_token=false } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="interface_type_list", is_token=false },
          } } },
        { type="optional", element={ type="rule_reference", name="type_parameter_constraints_clauses", is_token=false } },
        { type="rule_reference", name="struct_body", is_token=false },
        { type="optional", element={ type="rule_reference", name="SEMICOLON", is_token=true } },
      } },
      line_number=676,
    },
    {
      name="struct_modifier",
      body={ type="alternation", choices={
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="internal" },
        { type="literal", value="private" },
        { type="literal", value="new" },
        { type="literal", value="partial" },
      } },
      line_number=682,
    },
    {
      name="interface_type_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="namespace_or_type_name", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="namespace_or_type_name", is_token=false },
          } } },
      } },
      line_number=689,
    },
    {
      name="struct_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="struct_member_declaration", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=691,
    },
    {
      name="struct_member_declaration",
      body={ type="alternation", choices={
        { type="rule_reference", name="constant_declaration", is_token=false },
        { type="rule_reference", name="field_declaration", is_token=false },
        { type="rule_reference", name="method_declaration", is_token=false },
        { type="rule_reference", name="property_declaration", is_token=false },
        { type="rule_reference", name="event_declaration", is_token=false },
        { type="rule_reference", name="indexer_declaration", is_token=false },
        { type="rule_reference", name="operator_declaration", is_token=false },
        { type="rule_reference", name="conversion_operator_declaration", is_token=false },
        { type="rule_reference", name="constructor_declaration", is_token=false },
        { type="rule_reference", name="static_constructor_declaration", is_token=false },
        { type="rule_reference", name="type_declaration", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=693,
    },
    {
      name="interface_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="interface_modifier", is_token=false } },
        { type="literal", value="interface" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameter_list", is_token=false } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="interface_type_list", is_token=false },
          } } },
        { type="optional", element={ type="rule_reference", name="type_parameter_constraints_clauses", is_token=false } },
        { type="rule_reference", name="interface_body", is_token=false },
        { type="optional", element={ type="rule_reference", name="SEMICOLON", is_token=true } },
      } },
      line_number=727,
    },
    {
      name="interface_modifier",
      body={ type="alternation", choices={
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="internal" },
        { type="literal", value="private" },
        { type="literal", value="new" },
        { type="literal", value="partial" },
      } },
      line_number=733,
    },
    {
      name="interface_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="interface_member_declaration", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=740,
    },
    {
      name="interface_member_declaration",
      body={ type="alternation", choices={
        { type="rule_reference", name="interface_method_declaration", is_token=false },
        { type="rule_reference", name="interface_property_declaration", is_token=false },
        { type="rule_reference", name="interface_event_declaration", is_token=false },
        { type="rule_reference", name="interface_indexer_declaration", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=742,
    },
    {
      name="interface_method_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="optional", element={ type="literal", value="new" } },
        { type="rule_reference", name="return_type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameter_list", is_token=false } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="formal_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameter_constraints_clauses", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=750,
    },
    {
      name="interface_property_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="optional", element={ type="literal", value="new" } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="interface_accessors", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=756,
    },
    {
      name="interface_accessors",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="get" },
          { type="rule_reference", name="SEMICOLON", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="literal", value="set" },
              { type="rule_reference", name="SEMICOLON", is_token=true },
            } } },
        } },
        { type="sequence", elements={
          { type="literal", value="set" },
          { type="rule_reference", name="SEMICOLON", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="literal", value="get" },
              { type="rule_reference", name="SEMICOLON", is_token=true },
            } } },
        } },
      } },
      line_number=759,
    },
    {
      name="interface_event_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="optional", element={ type="literal", value="new" } },
        { type="literal", value="event" },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=762,
    },
    {
      name="interface_indexer_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="optional", element={ type="literal", value="new" } },
        { type="rule_reference", name="type", is_token=false },
        { type="literal", value="this" },
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="rule_reference", name="formal_parameter_list", is_token=false },
        { type="rule_reference", name="RBRACKET", is_token=true },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="interface_accessors", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=764,
    },
    {
      name="enum_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="enum_modifier", is_token=false } },
        { type="literal", value="enum" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="integral_type", is_token=false },
          } } },
        { type="rule_reference", name="enum_body", is_token=false },
        { type="optional", element={ type="rule_reference", name="SEMICOLON", is_token=true } },
      } },
      line_number=774,
    },
    {
      name="enum_modifier",
      body={ type="alternation", choices={
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="internal" },
        { type="literal", value="private" },
        { type="literal", value="new" },
      } },
      line_number=778,
    },
    {
      name="integral_type",
      body={ type="alternation", choices={
        { type="literal", value="byte" },
        { type="literal", value="sbyte" },
        { type="literal", value="short" },
        { type="literal", value="ushort" },
        { type="literal", value="int" },
        { type="literal", value="uint" },
        { type="literal", value="long" },
        { type="literal", value="ulong" },
      } },
      line_number=784,
    },
    {
      name="enum_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="rule_reference", name="enum_member_declarations", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=793,
    },
    {
      name="enum_member_declarations",
      body={ type="sequence", elements={
        { type="rule_reference", name="enum_member_declaration", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="enum_member_declaration", is_token=false },
          } } },
        { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
      } },
      line_number=795,
    },
    {
      name="enum_member_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="EQUALS", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
          } } },
      } },
      line_number=798,
    },
    {
      name="delegate_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="delegate_modifier", is_token=false } },
        { type="literal", value="delegate" },
        { type="rule_reference", name="return_type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameter_list", is_token=false } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="formal_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameter_constraints_clauses", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=817,
    },
    {
      name="delegate_modifier",
      body={ type="alternation", choices={
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="internal" },
        { type="literal", value="private" },
        { type="literal", value="new" },
      } },
      line_number=823,
    },
    {
      name="type",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="non_array_type", is_token=false },
          { type="repetition", element={ type="rule_reference", name="rank_specifier", is_token=false } },
        } },
        { type="sequence", elements={
          { type="literal", value="void" },
          { type="rule_reference", name="STAR", is_token=true },
        } },
      } },
      line_number=859,
    },
    {
      name="non_array_type",
      body={ type="alternation", choices={
        { type="rule_reference", name="value_type", is_token=false },
        { type="rule_reference", name="reference_type", is_token=false },
      } },
      line_number=862,
    },
    {
      name="value_type",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="primitive_type", is_token=false },
          { type="optional", element={ type="rule_reference", name="QUESTION", is_token=true } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="namespace_or_type_name", is_token=false },
          { type="optional", element={ type="rule_reference", name="QUESTION", is_token=true } },
        } },
      } },
      line_number=865,
    },
    {
      name="reference_type",
      body={ type="alternation", choices={
        { type="rule_reference", name="namespace_or_type_name", is_token=false },
        { type="literal", value="object" },
        { type="literal", value="string" },
      } },
      line_number=868,
    },
    {
      name="primitive_type",
      body={ type="alternation", choices={
        { type="rule_reference", name="numeric_type", is_token=false },
        { type="literal", value="bool" },
      } },
      line_number=872,
    },
    {
      name="numeric_type",
      body={ type="alternation", choices={
        { type="rule_reference", name="integral_type", is_token=false },
        { type="rule_reference", name="floating_point_type", is_token=false },
        { type="literal", value="decimal" },
      } },
      line_number=875,
    },
    {
      name="floating_point_type",
      body={ type="alternation", choices={
        { type="literal", value="float" },
        { type="literal", value="double" },
      } },
      line_number=879,
    },
    {
      name="rank_specifier",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="repetition", element={ type="rule_reference", name="COMMA", is_token=true } },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=882,
    },
    {
      name="pointer_type",
      body={ type="sequence", elements={
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="STAR", is_token=true },
      } },
      line_number=884,
    },
    {
      name="block",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=926,
    },
    {
      name="statement",
      body={ type="alternation", choices={
        { type="rule_reference", name="block", is_token=false },
        { type="rule_reference", name="local_variable_declaration_statement", is_token=false },
        { type="rule_reference", name="local_constant_declaration_statement", is_token=false },
        { type="rule_reference", name="empty_statement", is_token=false },
        { type="rule_reference", name="expression_statement", is_token=false },
        { type="rule_reference", name="if_statement", is_token=false },
        { type="rule_reference", name="while_statement", is_token=false },
        { type="rule_reference", name="do_while_statement", is_token=false },
        { type="rule_reference", name="for_statement", is_token=false },
        { type="rule_reference", name="foreach_statement", is_token=false },
        { type="rule_reference", name="switch_statement", is_token=false },
        { type="rule_reference", name="try_statement", is_token=false },
        { type="rule_reference", name="throw_statement", is_token=false },
        { type="rule_reference", name="return_statement", is_token=false },
        { type="rule_reference", name="break_statement", is_token=false },
        { type="rule_reference", name="continue_statement", is_token=false },
        { type="rule_reference", name="goto_statement", is_token=false },
        { type="rule_reference", name="lock_statement", is_token=false },
        { type="rule_reference", name="using_statement", is_token=false },
        { type="rule_reference", name="checked_statement", is_token=false },
        { type="rule_reference", name="unchecked_statement", is_token=false },
        { type="rule_reference", name="labelled_statement", is_token=false },
        { type="rule_reference", name="unsafe_statement", is_token=false },
        { type="rule_reference", name="fixed_statement", is_token=false },
        { type="rule_reference", name="yield_statement", is_token=false },
      } },
      line_number=928,
    },
    {
      name="local_variable_declaration_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="variable_declarators", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=954,
    },
    {
      name="local_constant_declaration_statement",
      body={ type="sequence", elements={
        { type="literal", value="const" },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="constant_declarators", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=956,
    },
    {
      name="empty_statement",
      body={ type="rule_reference", name="SEMICOLON", is_token=true },
      line_number=958,
    },
    {
      name="expression_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=960,
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
      line_number=962,
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
      line_number=964,
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
      line_number=966,
    },
    {
      name="for_statement",
      body={ type="sequence", elements={
        { type="literal", value="for" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="for_initializer", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
        { type="optional", element={ type="rule_reference", name="for_iterator", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=968,
    },
    {
      name="for_initializer",
      body={ type="alternation", choices={
        { type="rule_reference", name="local_variable_declaration", is_token=false },
        { type="rule_reference", name="expression_list", is_token=false },
      } },
      line_number=971,
    },
    {
      name="for_iterator",
      body={ type="rule_reference", name="expression_list", is_token=false },
      line_number=974,
    },
    {
      name="local_variable_declaration",
      body={ type="sequence", elements={
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="variable_declarators", is_token=false },
      } },
      line_number=976,
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
      line_number=978,
    },
    {
      name="foreach_statement",
      body={ type="sequence", elements={
        { type="literal", value="foreach" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="in" },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=980,
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
      line_number=982,
    },
    {
      name="switch_block",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="switch_section", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=984,
    },
    {
      name="switch_section",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="switch_label", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
      } },
      line_number=986,
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
      line_number=988,
    },
    {
      name="try_statement",
      body={ type="sequence", elements={
        { type="literal", value="try" },
        { type="rule_reference", name="block", is_token=false },
        { type="group", element={ type="alternation", choices={
            { type="sequence", elements={
              { type="rule_reference", name="catch_clauses", is_token=false },
              { type="optional", element={ type="rule_reference", name="finally_clause", is_token=false } },
            } },
            { type="rule_reference", name="finally_clause", is_token=false },
          } } },
      } },
      line_number=991,
    },
    {
      name="catch_clauses",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="specific_catch_clause", is_token=false },
          { type="repetition", element={ type="rule_reference", name="specific_catch_clause", is_token=false } },
          { type="optional", element={ type="rule_reference", name="general_catch_clause", is_token=false } },
        } },
        { type="rule_reference", name="general_catch_clause", is_token=false },
      } },
      line_number=994,
    },
    {
      name="specific_catch_clause",
      body={ type="sequence", elements={
        { type="literal", value="catch" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="namespace_or_type_name", is_token=false },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=997,
    },
    {
      name="general_catch_clause",
      body={ type="sequence", elements={
        { type="literal", value="catch" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=999,
    },
    {
      name="finally_clause",
      body={ type="sequence", elements={
        { type="literal", value="finally" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=1001,
    },
    {
      name="throw_statement",
      body={ type="sequence", elements={
        { type="literal", value="throw" },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1003,
    },
    {
      name="return_statement",
      body={ type="sequence", elements={
        { type="literal", value="return" },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1005,
    },
    {
      name="break_statement",
      body={ type="sequence", elements={
        { type="literal", value="break" },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1007,
    },
    {
      name="continue_statement",
      body={ type="sequence", elements={
        { type="literal", value="continue" },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=1009,
    },
    {
      name="goto_statement",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="goto" },
          { type="rule_reference", name="NAME", is_token=true },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="goto" },
          { type="literal", value="case" },
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="goto" },
          { type="literal", value="default" },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
      } },
      line_number=1011,
    },
    {
      name="lock_statement",
      body={ type="sequence", elements={
        { type="literal", value="lock" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=1015,
    },
    {
      name="using_statement",
      body={ type="sequence", elements={
        { type="literal", value="using" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="resource_acquisition", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=1017,
    },
    {
      name="resource_acquisition",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="type", is_token=false },
          { type="rule_reference", name="variable_declarators", is_token=false },
        } },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=1019,
    },
    {
      name="checked_statement",
      body={ type="sequence", elements={
        { type="literal", value="checked" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=1022,
    },
    {
      name="unchecked_statement",
      body={ type="sequence", elements={
        { type="literal", value="unchecked" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=1024,
    },
    {
      name="labelled_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=1026,
    },
    {
      name="unsafe_statement",
      body={ type="sequence", elements={
        { type="literal", value="unsafe" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=1028,
    },
    {
      name="fixed_statement",
      body={ type="sequence", elements={
        { type="literal", value="fixed" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="variable_declarators", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=1030,
    },
    {
      name="yield_statement",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="yield" },
          { type="literal", value="return" },
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="yield" },
          { type="literal", value="break" },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
      } },
      line_number=1044,
    },
    {
      name="expression",
      body={ type="rule_reference", name="assignment_expression", is_token=false },
      line_number=1074,
    },
    {
      name="assignment_expression",
      body={ type="alternation", choices={
        { type="rule_reference", name="conditional_expression", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="unary_expression", is_token=false },
          { type="rule_reference", name="assignment_operator", is_token=false },
          { type="rule_reference", name="assignment_expression", is_token=false },
        } },
      } },
      line_number=1076,
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
      } },
      line_number=1079,
    },
    {
      name="conditional_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="null_coalescing_expression", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="QUESTION", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
          } } },
      } },
      line_number=1093,
    },
    {
      name="null_coalescing_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="logical_or_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="QUESTION_QUESTION", is_token=true },
            { type="rule_reference", name="logical_or_expression", is_token=false },
          } } },
      } },
      line_number=1107,
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
      line_number=1112,
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
      line_number=1116,
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
      line_number=1120,
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
      line_number=1124,
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
      line_number=1128,
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
      line_number=1132,
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
              { type="literal", value="is" },
              { type="rule_reference", name="type", is_token=false },
            } },
            { type="sequence", elements={
              { type="literal", value="as" },
              { type="rule_reference", name="type", is_token=false },
            } },
          } } },
      } },
      line_number=1142,
    },
    {
      name="shift_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="additive_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="LEFT_SHIFT", is_token=true },
                { type="rule_reference", name="RIGHT_SHIFT", is_token=true },
              } } },
            { type="rule_reference", name="additive_expression", is_token=false },
          } } },
      } },
      line_number=1150,
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
      line_number=1155,
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
      line_number=1160,
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
        { type="sequence", elements={
          { type="rule_reference", name="BANG", is_token=true },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="TILDE", is_token=true },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
        { type="rule_reference", name="cast_expression", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="AMPERSAND", is_token=true },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="STAR", is_token=true },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
        { type="rule_reference", name="postfix_expression", is_token=false },
      } },
      line_number=1165,
    },
    {
      name="cast_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="unary_expression", is_token=false },
      } },
      line_number=1176,
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
      line_number=1180,
    },
    {
      name="primary_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="primary", is_token=false },
        { type="repetition", element={ type="rule_reference", name="primary_suffix", is_token=false } },
      } },
      line_number=1191,
    },
    {
      name="primary_suffix",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="DOT", is_token=true },
          { type="rule_reference", name="NAME", is_token=true },
          { type="optional", element={ type="rule_reference", name="type_argument_list", is_token=false } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="argument_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LBRACKET", is_token=true },
          { type="rule_reference", name="expression_list", is_token=false },
          { type="rule_reference", name="RBRACKET", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="ARROW", is_token=true },
          { type="rule_reference", name="NAME", is_token=true },
        } },
      } },
      line_number=1193,
    },
    {
      name="primary",
      body={ type="alternation", choices={
        { type="rule_reference", name="literal", is_token=false },
        { type="literal", value="this" },
        { type="sequence", elements={
          { type="literal", value="base" },
          { type="rule_reference", name="DOT", is_token=true },
          { type="rule_reference", name="NAME", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="base" },
          { type="rule_reference", name="LBRACKET", is_token=true },
          { type="rule_reference", name="expression_list", is_token=false },
          { type="rule_reference", name="RBRACKET", is_token=true },
        } },
        { type="rule_reference", name="typeof_expression", is_token=false },
        { type="rule_reference", name="sizeof_expression", is_token=false },
        { type="rule_reference", name="checked_expression", is_token=false },
        { type="rule_reference", name="unchecked_expression", is_token=false },
        { type="rule_reference", name="default_value_expression", is_token=false },
        { type="rule_reference", name="new_expression", is_token=false },
        { type="rule_reference", name="anonymous_method_expression", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="NAME", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COLON_COLON", is_token=true },
              { type="rule_reference", name="NAME", is_token=true },
            } } },
          { type="optional", element={ type="rule_reference", name="type_argument_list", is_token=false } },
        } },
      } },
      line_number=1198,
    },
    {
      name="typeof_expression",
      body={ type="sequence", elements={
        { type="literal", value="typeof" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="type_or_void", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=1219,
    },
    {
      name="type_or_void",
      body={ type="alternation", choices={
        { type="rule_reference", name="type", is_token=false },
        { type="literal", value="void" },
        { type="sequence", elements={
          { type="rule_reference", name="namespace_or_type_name", is_token=false },
          { type="rule_reference", name="LESS_THAN", is_token=true },
          { type="repetition", element={ type="rule_reference", name="COMMA", is_token=true } },
          { type="rule_reference", name="GREATER_THAN", is_token=true },
        } },
      } },
      line_number=1221,
    },
    {
      name="sizeof_expression",
      body={ type="sequence", elements={
        { type="literal", value="sizeof" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=1227,
    },
    {
      name="checked_expression",
      body={ type="sequence", elements={
        { type="literal", value="checked" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=1231,
    },
    {
      name="unchecked_expression",
      body={ type="sequence", elements={
        { type="literal", value="unchecked" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=1233,
    },
    {
      name="default_value_expression",
      body={ type="sequence", elements={
        { type="literal", value="default" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=1242,
    },
    {
      name="new_expression",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="new" },
          { type="rule_reference", name="new_object_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="literal", value="new" },
          { type="rule_reference", name="new_array_expression", is_token=false },
        } },
      } },
      line_number=1251,
    },
    {
      name="new_object_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="namespace_or_type_name", is_token=false },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="argument_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=1254,
    },
    {
      name="new_array_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="array_type", is_token=false },
        { type="rule_reference", name="array_creation_suffix", is_token=false },
      } },
      line_number=1256,
    },
    {
      name="array_type",
      body={ type="group", element={ type="alternation", choices={
          { type="rule_reference", name="primitive_type", is_token=false },
          { type="rule_reference", name="namespace_or_type_name", is_token=false },
        } } },
      line_number=1258,
    },
    {
      name="array_creation_suffix",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="rank_specifier", is_token=false },
          { type="repetition", element={ type="rule_reference", name="rank_specifier", is_token=false } },
          { type="rule_reference", name="array_initializer", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LBRACKET", is_token=true },
          { type="rule_reference", name="expression_list", is_token=false },
          { type="rule_reference", name="RBRACKET", is_token=true },
          { type="repetition", element={ type="rule_reference", name="rank_specifier", is_token=false } },
          { type="optional", element={ type="rule_reference", name="array_initializer", is_token=false } },
        } },
      } },
      line_number=1260,
    },
    {
      name="anonymous_method_expression",
      body={ type="sequence", elements={
        { type="literal", value="delegate" },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="optional", element={ type="rule_reference", name="anonymous_method_parameter_list", is_token=false } },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=1292,
    },
    {
      name="anonymous_method_parameter_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="anonymous_method_parameter", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="anonymous_method_parameter", is_token=false },
          } } },
      } },
      line_number=1295,
    },
    {
      name="anonymous_method_parameter",
      body={ type="sequence", elements={
        { type="optional", element={ type="rule_reference", name="parameter_modifier", is_token=false } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=1298,
    },
    {
      name="argument_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="argument", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="argument", is_token=false },
          } } },
      } },
      line_number=1302,
    },
    {
      name="argument",
      body={ type="sequence", elements={
        { type="optional", element={ type="alternation", choices={
            { type="literal", value="ref" },
            { type="literal", value="out" },
          } } },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=1304,
    },
    {
      name="literal",
      body={ type="alternation", choices={
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="rule_reference", name="CHAR", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="VERBATIM_STRING", is_token=true },
        { type="literal", value="true" },
        { type="literal", value="false" },
        { type="literal", value="null" },
      } },
      line_number=1312,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
