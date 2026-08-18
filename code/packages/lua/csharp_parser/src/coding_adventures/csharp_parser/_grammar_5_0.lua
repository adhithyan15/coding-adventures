-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: csharp5.0.grammar
-- Regenerate with: grammar-tools compile-grammar csharp5.0.grammar
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
      line_number=89,
    },
    {
      name="extern_alias_directive",
      body={ type="sequence", elements={
        { type="literal", value="extern" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=103,
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
          { type="rule_reference", name="qualified_name", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
      } },
      line_number=117,
    },
    {
      name="qualified_name",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="NAME", is_token=true },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="DOT", is_token=true },
              { type="rule_reference", name="NAME", is_token=true },
            } } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="NAME", is_token=true },
          { type="rule_reference", name="NAMESPACE_ALIAS", is_token=true },
          { type="rule_reference", name="NAME", is_token=true },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="DOT", is_token=true },
              { type="rule_reference", name="NAME", is_token=true },
            } } },
        } },
      } },
      line_number=131,
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
      line_number=143,
    },
    {
      name="global_attribute_target",
      body={ type="alternation", choices={
        { type="literal", value="assembly" },
        { type="literal", value="module" },
      } },
      line_number=145,
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
      line_number=160,
    },
    {
      name="namespace_member_declaration",
      body={ type="alternation", choices={
        { type="rule_reference", name="namespace_declaration", is_token=false },
        { type="rule_reference", name="type_declaration", is_token=false },
      } },
      line_number=170,
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
      line_number=173,
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
      line_number=191,
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
      line_number=193,
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
      line_number=201,
    },
    {
      name="attribute",
      body={ type="sequence", elements={
        { type="rule_reference", name="qualified_name", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="optional", element={ type="rule_reference", name="attribute_arguments", is_token=false } },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
      } },
      line_number=203,
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
      line_number=205,
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
      line_number=207,
    },
    {
      name="class_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="class_modifier", is_token=false } },
        { type="optional", element={ type="literal", value="partial" } },
        { type="literal", value="class" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameter_list", is_token=false } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="class_base_list", is_token=false },
          } } },
        { type="repetition", element={ type="rule_reference", name="type_parameter_constraints_clause", is_token=false } },
        { type="rule_reference", name="class_body", is_token=false },
        { type="optional", element={ type="rule_reference", name="SEMICOLON", is_token=true } },
      } },
      line_number=229,
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
      } },
      line_number=235,
    },
    {
      name="class_base_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="type_name", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="type_name", is_token=false },
          } } },
      } },
      line_number=244,
    },
    {
      name="class_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="class_member_declaration", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=246,
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
      line_number=270,
    },
    {
      name="type_parameter",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="optional", element={ type="rule_reference", name="variance_annotation", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=272,
    },
    {
      name="variance_annotation",
      body={ type="alternation", choices={
        { type="literal", value="in" },
        { type="literal", value="out" },
      } },
      line_number=274,
    },
    {
      name="type_parameter_constraints_clause",
      body={ type="sequence", elements={
        { type="literal", value="where" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="type_parameter_constraints", is_token=false },
      } },
      line_number=289,
    },
    {
      name="type_parameter_constraints",
      body={ type="sequence", elements={
        { type="rule_reference", name="type_parameter_constraint", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="type_parameter_constraint", is_token=false },
          } } },
      } },
      line_number=291,
    },
    {
      name="type_parameter_constraint",
      body={ type="alternation", choices={
        { type="literal", value="class" },
        { type="literal", value="struct" },
        { type="sequence", elements={
          { type="literal", value="new" },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
        { type="rule_reference", name="type_name", is_token=false },
      } },
      line_number=293,
    },
    {
      name="type_name",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="DOT", is_token=true },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
        { type="optional", element={ type="rule_reference", name="type_argument_list", is_token=false } },
      } },
      line_number=311,
    },
    {
      name="type_argument_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="LESS_THAN", is_token=true },
        { type="rule_reference", name="type_argument", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="type_argument", is_token=false },
          } } },
        { type="rule_reference", name="GREATER_THAN", is_token=true },
      } },
      line_number=313,
    },
    {
      name="type_argument",
      body={ type="rule_reference", name="type", is_token=false },
      line_number=315,
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
      line_number=326,
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
      line_number=348,
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
      line_number=351,
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
      line_number=357,
    },
    {
      name="constant_declarator",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=359,
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
      line_number=370,
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
      line_number=373,
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
      line_number=382,
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
      line_number=384,
    },
    {
      name="variable_initializer",
      body={ type="alternation", choices={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="array_initializer", is_token=false },
      } },
      line_number=386,
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
      line_number=389,
    },
    {
      name="method_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="method_modifier", is_token=false } },
        { type="optional", element={ type="literal", value="partial" } },
        { type="rule_reference", name="return_type", is_token=false },
        { type="rule_reference", name="qualified_name", is_token=false },
        { type="optional", element={ type="rule_reference", name="type_parameter_list", is_token=false } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="formal_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="repetition", element={ type="rule_reference", name="type_parameter_constraints_clause", is_token=false } },
        { type="rule_reference", name="method_body", is_token=false },
      } },
      line_number=422,
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
        { type="literal", value="async" },
      } },
      line_number=428,
    },
    {
      name="return_type",
      body={ type="alternation", choices={
        { type="literal", value="void" },
        { type="rule_reference", name="type", is_token=false },
      } },
      line_number=441,
    },
    {
      name="method_body",
      body={ type="alternation", choices={
        { type="rule_reference", name="block", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=444,
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
      line_number=463,
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
      line_number=466,
    },
    {
      name="fixed_parameter",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="optional", element={ type="rule_reference", name="parameter_modifier", is_token=false } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="EQUALS", is_token=true },
            { type="rule_reference", name="expression", is_token=false },
          } } },
      } },
      line_number=475,
    },
    {
      name="parameter_modifier",
      body={ type="alternation", choices={
        { type="literal", value="ref" },
        { type="literal", value="out" },
        { type="literal", value="this" },
      } },
      line_number=477,
    },
    {
      name="parameter_array",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="literal", value="params" },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=490,
    },
    {
      name="property_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="property_modifier", is_token=false } },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="qualified_name", is_token=false },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="accessor_declarations", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=508,
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
      line_number=511,
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
      line_number=523,
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
      line_number=526,
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
      line_number=529,
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
      line_number=532,
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
              { type="rule_reference", name="qualified_name", is_token=false },
              { type="rule_reference", name="LBRACE", is_token=true },
              { type="rule_reference", name="event_accessor_declarations", is_token=false },
              { type="rule_reference", name="RBRACE", is_token=true },
            } },
          } } },
      } },
      line_number=550,
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
      line_number=554,
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
      line_number=566,
    },
    {
      name="add_accessor_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="literal", value="add" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=569,
    },
    {
      name="remove_accessor_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="literal", value="remove" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=571,
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
      line_number=582,
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
      line_number=586,
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
      line_number=603,
    },
    {
      name="operator_modifiers",
      body={ type="sequence", elements={
        { type="literal", value="public" },
        { type="literal", value="static" },
        { type="optional", element={ type="literal", value="extern" } },
      } },
      line_number=608,
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
      line_number=610,
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
      line_number=637,
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
      line_number=647,
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
      line_number=651,
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
      line_number=657,
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
      line_number=667,
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
      line_number=676,
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
      line_number=679,
    },
    {
      name="struct_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="struct_modifier", is_token=false } },
        { type="optional", element={ type="literal", value="partial" } },
        { type="literal", value="struct" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameter_list", is_token=false } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="interface_type_list", is_token=false },
          } } },
        { type="repetition", element={ type="rule_reference", name="type_parameter_constraints_clause", is_token=false } },
        { type="rule_reference", name="struct_body", is_token=false },
        { type="optional", element={ type="rule_reference", name="SEMICOLON", is_token=true } },
      } },
      line_number=689,
    },
    {
      name="struct_modifier",
      body={ type="alternation", choices={
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="internal" },
        { type="literal", value="private" },
        { type="literal", value="new" },
      } },
      line_number=695,
    },
    {
      name="interface_type_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="type_name", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="type_name", is_token=false },
          } } },
      } },
      line_number=701,
    },
    {
      name="struct_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="struct_member_declaration", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=703,
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
      line_number=705,
    },
    {
      name="interface_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="attribute_section", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="interface_modifier", is_token=false } },
        { type="optional", element={ type="literal", value="partial" } },
        { type="literal", value="interface" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameter_list", is_token=false } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="interface_type_list", is_token=false },
          } } },
        { type="repetition", element={ type="rule_reference", name="type_parameter_constraints_clause", is_token=false } },
        { type="rule_reference", name="interface_body", is_token=false },
        { type="optional", element={ type="rule_reference", name="SEMICOLON", is_token=true } },
      } },
      line_number=733,
    },
    {
      name="interface_modifier",
      body={ type="alternation", choices={
        { type="literal", value="public" },
        { type="literal", value="protected" },
        { type="literal", value="internal" },
        { type="literal", value="private" },
        { type="literal", value="new" },
      } },
      line_number=739,
    },
    {
      name="interface_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="interface_member_declaration", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=745,
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
      line_number=747,
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
        { type="repetition", element={ type="rule_reference", name="type_parameter_constraints_clause", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=753,
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
      line_number=759,
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
      line_number=762,
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
      line_number=765,
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
      line_number=767,
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
      line_number=780,
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
      line_number=784,
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
      line_number=790,
    },
    {
      name="enum_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="rule_reference", name="enum_member_declarations", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=799,
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
      line_number=801,
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
      line_number=804,
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
        { type="repetition", element={ type="rule_reference", name="type_parameter_constraints_clause", is_token=false } },
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
          { type="rule_reference", name="value_type", is_token=false },
          { type="repetition", element={ type="rule_reference", name="rank_specifier", is_token=false } },
          { type="optional", element={ type="rule_reference", name="QUESTION", is_token=true } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="reference_type", is_token=false },
          { type="repetition", element={ type="rule_reference", name="rank_specifier", is_token=false } },
        } },
        { type="sequence", elements={
          { type="literal", value="dynamic" },
          { type="repetition", element={ type="rule_reference", name="rank_specifier", is_token=false } },
        } },
        { type="sequence", elements={
          { type="literal", value="void" },
          { type="rule_reference", name="STAR", is_token=true },
        } },
      } },
      line_number=850,
    },
    {
      name="value_type",
      body={ type="alternation", choices={
        { type="rule_reference", name="primitive_type", is_token=false },
        { type="rule_reference", name="type_name", is_token=false },
      } },
      line_number=855,
    },
    {
      name="reference_type",
      body={ type="alternation", choices={
        { type="rule_reference", name="type_name", is_token=false },
        { type="literal", value="object" },
        { type="literal", value="string" },
      } },
      line_number=858,
    },
    {
      name="primitive_type",
      body={ type="alternation", choices={
        { type="rule_reference", name="numeric_type", is_token=false },
        { type="literal", value="bool" },
      } },
      line_number=862,
    },
    {
      name="numeric_type",
      body={ type="alternation", choices={
        { type="rule_reference", name="integral_type", is_token=false },
        { type="rule_reference", name="floating_point_type", is_token=false },
        { type="literal", value="decimal" },
      } },
      line_number=865,
    },
    {
      name="floating_point_type",
      body={ type="alternation", choices={
        { type="literal", value="float" },
        { type="literal", value="double" },
      } },
      line_number=869,
    },
    {
      name="rank_specifier",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="repetition", element={ type="rule_reference", name="COMMA", is_token=true } },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=872,
    },
    {
      name="pointer_type",
      body={ type="sequence", elements={
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="STAR", is_token=true },
      } },
      line_number=874,
    },
    {
      name="block",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=893,
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
      line_number=895,
    },
    {
      name="local_variable_declaration_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="variable_declarators", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=928,
    },
    {
      name="local_constant_declaration_statement",
      body={ type="sequence", elements={
        { type="literal", value="const" },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="constant_declarators", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=930,
    },
    {
      name="empty_statement",
      body={ type="rule_reference", name="SEMICOLON", is_token=true },
      line_number=932,
    },
    {
      name="expression_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=934,
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
      line_number=936,
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
      line_number=938,
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
      line_number=940,
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
      line_number=942,
    },
    {
      name="for_initializer",
      body={ type="alternation", choices={
        { type="rule_reference", name="local_variable_declaration", is_token=false },
        { type="rule_reference", name="expression_list", is_token=false },
      } },
      line_number=945,
    },
    {
      name="for_iterator",
      body={ type="rule_reference", name="expression_list", is_token=false },
      line_number=948,
    },
    {
      name="local_variable_declaration",
      body={ type="sequence", elements={
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="variable_declarators", is_token=false },
      } },
      line_number=950,
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
      line_number=952,
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
      line_number=954,
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
      line_number=956,
    },
    {
      name="switch_block",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="switch_section", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=958,
    },
    {
      name="switch_section",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="switch_label", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
      } },
      line_number=960,
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
      line_number=962,
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
      line_number=969,
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
      line_number=972,
    },
    {
      name="specific_catch_clause",
      body={ type="sequence", elements={
        { type="literal", value="catch" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="type_name", is_token=false },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=975,
    },
    {
      name="general_catch_clause",
      body={ type="sequence", elements={
        { type="literal", value="catch" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=977,
    },
    {
      name="finally_clause",
      body={ type="sequence", elements={
        { type="literal", value="finally" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=979,
    },
    {
      name="throw_statement",
      body={ type="sequence", elements={
        { type="literal", value="throw" },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=981,
    },
    {
      name="return_statement",
      body={ type="sequence", elements={
        { type="literal", value="return" },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=983,
    },
    {
      name="break_statement",
      body={ type="sequence", elements={
        { type="literal", value="break" },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=985,
    },
    {
      name="continue_statement",
      body={ type="sequence", elements={
        { type="literal", value="continue" },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=987,
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
      line_number=989,
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
      line_number=993,
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
      line_number=995,
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
      line_number=997,
    },
    {
      name="checked_statement",
      body={ type="sequence", elements={
        { type="literal", value="checked" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=1000,
    },
    {
      name="unchecked_statement",
      body={ type="sequence", elements={
        { type="literal", value="unchecked" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=1002,
    },
    {
      name="labelled_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=1004,
    },
    {
      name="unsafe_statement",
      body={ type="sequence", elements={
        { type="literal", value="unsafe" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=1006,
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
      line_number=1008,
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
      line_number=1037,
    },
    {
      name="expression",
      body={ type="alternation", choices={
        { type="rule_reference", name="assignment_expression", is_token=false },
        { type="rule_reference", name="lambda_expression", is_token=false },
        { type="rule_reference", name="query_expression", is_token=false },
      } },
      line_number=1078,
    },
    {
      name="lambda_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="lambda_parameters", is_token=false },
        { type="rule_reference", name="LAMBDA", is_token=true },
        { type="rule_reference", name="lambda_body", is_token=false },
      } },
      line_number=1101,
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
      line_number=1103,
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
      line_number=1106,
    },
    {
      name="lambda_parameter",
      body={ type="sequence", elements={
        { type="optional", element={ type="rule_reference", name="type", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=1108,
    },
    {
      name="lambda_body",
      body={ type="alternation", choices={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=1110,
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
      line_number=1115,
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
      line_number=1118,
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
      line_number=1132,
    },
    {
      name="null_coalescing_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="logical_or_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="NULL_COALESCE", is_token=true },
            { type="rule_reference", name="logical_or_expression", is_token=false },
          } } },
      } },
      line_number=1146,
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
      line_number=1150,
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
      line_number=1154,
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
      line_number=1158,
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
      line_number=1162,
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
      line_number=1166,
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
      line_number=1170,
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
      line_number=1175,
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
      line_number=1183,
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
      line_number=1188,
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
      line_number=1193,
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
        { type="sequence", elements={
          { type="literal", value="await" },
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
      line_number=1207,
    },
    {
      name="cast_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="unary_expression", is_token=false },
      } },
      line_number=1219,
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
      line_number=1223,
    },
    {
      name="primary_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="primary", is_token=false },
        { type="repetition", element={ type="rule_reference", name="primary_suffix", is_token=false } },
      } },
      line_number=1236,
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
      line_number=1238,
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
          { type="optional", element={ type="rule_reference", name="type_argument_list", is_token=false } },
        } },
      } },
      line_number=1243,
    },
    {
      name="typeof_expression",
      body={ type="sequence", elements={
        { type="literal", value="typeof" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="type_or_void", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=1259,
    },
    {
      name="type_or_void",
      body={ type="alternation", choices={
        { type="rule_reference", name="type", is_token=false },
        { type="literal", value="void" },
      } },
      line_number=1261,
    },
    {
      name="sizeof_expression",
      body={ type="sequence", elements={
        { type="literal", value="sizeof" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=1263,
    },
    {
      name="checked_expression",
      body={ type="sequence", elements={
        { type="literal", value="checked" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=1265,
    },
    {
      name="unchecked_expression",
      body={ type="sequence", elements={
        { type="literal", value="unchecked" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=1267,
    },
    {
      name="default_value_expression",
      body={ type="sequence", elements={
        { type="literal", value="default" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="type", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=1269,
    },
    {
      name="anonymous_method_expression",
      body={ type="sequence", elements={
        { type="literal", value="delegate" },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="optional", element={ type="rule_reference", name="formal_parameter_list", is_token=false } },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=1284,
    },
    {
      name="new_expression",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="new" },
          { type="rule_reference", name="new_anonymous_type", is_token=false },
        } },
        { type="sequence", elements={
          { type="literal", value="new" },
          { type="rule_reference", name="new_implicitly_typed_array", is_token=false },
        } },
        { type="sequence", elements={
          { type="literal", value="new" },
          { type="rule_reference", name="new_object_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="literal", value="new" },
          { type="rule_reference", name="new_array_expression", is_token=false },
        } },
      } },
      line_number=1296,
    },
    {
      name="new_anonymous_type",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="anonymous_type_member", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="anonymous_type_member", is_token=false },
              } } },
          } } },
        { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=1313,
    },
    {
      name="anonymous_type_member",
      body={ type="sequence", elements={
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="EQUALS", is_token=true },
          } } },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=1315,
    },
    {
      name="new_implicitly_typed_array",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="rule_reference", name="RBRACKET", is_token=true },
        { type="rule_reference", name="array_initializer", is_token=false },
      } },
      line_number=1321,
    },
    {
      name="new_object_expression",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="type_name", is_token=false },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="argument_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="object_or_collection_initializer", is_token=false } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="type_name", is_token=false },
          { type="rule_reference", name="object_or_collection_initializer", is_token=false },
        } },
      } },
      line_number=1329,
    },
    {
      name="object_or_collection_initializer",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="rule_reference", name="initializer_list", is_token=false } },
        { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=1339,
    },
    {
      name="initializer_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="initializer_item", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="initializer_item", is_token=false },
          } } },
      } },
      line_number=1341,
    },
    {
      name="initializer_item",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="NAME", is_token=true },
          { type="rule_reference", name="EQUALS", is_token=true },
          { type="group", element={ type="alternation", choices={
              { type="rule_reference", name="expression", is_token=false },
              { type="rule_reference", name="object_or_collection_initializer", is_token=false },
            } } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="rule_reference", name="expression_list", is_token=false },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=1345,
    },
    {
      name="new_array_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="array_type", is_token=false },
        { type="rule_reference", name="array_creation_suffix", is_token=false },
      } },
      line_number=1349,
    },
    {
      name="array_type",
      body={ type="group", element={ type="alternation", choices={
          { type="rule_reference", name="primitive_type", is_token=false },
          { type="rule_reference", name="type_name", is_token=false },
        } } },
      line_number=1351,
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
      line_number=1353,
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
      line_number=1362,
    },
    {
      name="argument",
      body={ type="sequence", elements={
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="COLON", is_token=true },
          } } },
        { type="optional", element={ type="alternation", choices={
            { type="literal", value="ref" },
            { type="literal", value="out" },
          } } },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=1364,
    },
    {
      name="query_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="from_clause", is_token=false },
        { type="rule_reference", name="query_body", is_token=false },
      } },
      line_number=1390,
    },
    {
      name="from_clause",
      body={ type="sequence", elements={
        { type="literal", value="from" },
        { type="optional", element={ type="rule_reference", name="type", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="in" },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=1392,
    },
    {
      name="query_body",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="query_body_clause", is_token=false } },
        { type="rule_reference", name="select_or_group_clause", is_token=false },
        { type="optional", element={ type="rule_reference", name="query_continuation", is_token=false } },
      } },
      line_number=1394,
    },
    {
      name="query_body_clause",
      body={ type="alternation", choices={
        { type="rule_reference", name="from_clause", is_token=false },
        { type="rule_reference", name="let_clause", is_token=false },
        { type="rule_reference", name="where_clause", is_token=false },
        { type="rule_reference", name="join_clause", is_token=false },
        { type="rule_reference", name="orderby_clause", is_token=false },
      } },
      line_number=1396,
    },
    {
      name="let_clause",
      body={ type="sequence", elements={
        { type="literal", value="let" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=1402,
    },
    {
      name="where_clause",
      body={ type="sequence", elements={
        { type="literal", value="where" },
        { type="rule_reference", name="expression", is_token=false },
      } },
      line_number=1404,
    },
    {
      name="join_clause",
      body={ type="sequence", elements={
        { type="literal", value="join" },
        { type="optional", element={ type="rule_reference", name="type", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="in" },
        { type="rule_reference", name="expression", is_token=false },
        { type="literal", value="on" },
        { type="rule_reference", name="expression", is_token=false },
        { type="literal", value="equals" },
        { type="rule_reference", name="expression", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="into" },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
      } },
      line_number=1406,
    },
    {
      name="orderby_clause",
      body={ type="sequence", elements={
        { type="literal", value="orderby" },
        { type="rule_reference", name="ordering", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="ordering", is_token=false },
          } } },
      } },
      line_number=1410,
    },
    {
      name="ordering",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="optional", element={ type="alternation", choices={
            { type="literal", value="ascending" },
            { type="literal", value="descending" },
          } } },
      } },
      line_number=1412,
    },
    {
      name="select_or_group_clause",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="select" },
          { type="rule_reference", name="expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="literal", value="group" },
          { type="rule_reference", name="expression", is_token=false },
          { type="literal", value="by" },
          { type="rule_reference", name="expression", is_token=false },
        } },
      } },
      line_number=1414,
    },
    {
      name="query_continuation",
      body={ type="sequence", elements={
        { type="literal", value="into" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="query_body", is_token=false },
      } },
      line_number=1417,
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
      line_number=1425,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
