// AUTO-GENERATED FILE - DO NOT EDIT
// Source: csharp2.0.grammar
// Regenerate with: grammar-tools compile-grammar csharp2.0.grammar
//
// This file embeds a ParserGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .grammar file at
// runtime.

import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const PARSER_GRAMMAR: ParserGrammar = {
  version: 1,
  rules: [
  {
    name: "compilation_unit",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "extern_alias_directive" } },
      { type: "repetition", element: { type: "rule_reference", name: "using_directive" } },
      { type: "repetition", element: { type: "rule_reference", name: "global_attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "namespace_member_declaration" } },
    ] },
    lineNumber: 73,
  },
  {
    name: "extern_alias_directive",
    body: { type: "sequence", elements: [
      { type: "literal", value: "extern" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 87,
  },
  {
    name: "using_directive",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "using" },
        { type: "rule_reference", name: "qualified_name" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "using" },
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "EQUALS" },
        { type: "rule_reference", name: "namespace_or_type_name" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 102,
  },
  {
    name: "qualified_name",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "DOT" },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 131,
  },
  {
    name: "namespace_or_type_name",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "namespace_or_type_part" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "DOT" },
          { type: "rule_reference", name: "namespace_or_type_part" },
        ] } },
    ] },
    lineNumber: 136,
  },
  {
    name: "namespace_or_type_part",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "optional", element: { type: "rule_reference", name: "type_argument_list" } },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "COLON_COLON" },
        { type: "token_reference", name: "NAME" },
        { type: "optional", element: { type: "rule_reference", name: "type_argument_list" } },
      ] },
    ] },
    lineNumber: 138,
  },
  {
    name: "type_parameter_list",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LESS_THAN" },
      { type: "rule_reference", name: "type_parameter" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "type_parameter" },
        ] } },
      { type: "token_reference", name: "GREATER_THAN" },
    ] },
    lineNumber: 166,
  },
  {
    name: "type_parameter",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 168,
  },
  {
    name: "type_argument_list",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LESS_THAN" },
      { type: "rule_reference", name: "type" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "type" },
        ] } },
      { type: "token_reference", name: "GREATER_THAN" },
    ] },
    lineNumber: 170,
  },
  {
    name: "type_parameter_constraints_clauses",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type_parameter_constraints_clause" },
      { type: "repetition", element: { type: "rule_reference", name: "type_parameter_constraints_clause" } },
    ] },
    lineNumber: 208,
  },
  {
    name: "type_parameter_constraints_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "where" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "type_parameter_constraints" },
    ] },
    lineNumber: 211,
  },
  {
    name: "type_parameter_constraints",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "primary_constraint" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "secondary_constraints" },
          ] } },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "constructor_constraint" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "secondary_constraints" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "constructor_constraint" },
          ] } },
      ] },
      { type: "rule_reference", name: "constructor_constraint" },
    ] },
    lineNumber: 213,
  },
  {
    name: "primary_constraint",
    body: { type: "alternation", choices: [
      { type: "literal", value: "class" },
      { type: "literal", value: "struct" },
      { type: "rule_reference", name: "namespace_or_type_name" },
    ] },
    lineNumber: 217,
  },
  {
    name: "secondary_constraints",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "namespace_or_type_name" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "namespace_or_type_name" },
        ] } },
    ] },
    lineNumber: 221,
  },
  {
    name: "constructor_constraint",
    body: { type: "sequence", elements: [
      { type: "literal", value: "new" },
      { type: "token_reference", name: "LPAREN" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 223,
  },
  {
    name: "global_attribute_section",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "rule_reference", name: "global_attribute_target" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "attribute_list" },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 231,
  },
  {
    name: "global_attribute_target",
    body: { type: "alternation", choices: [
      { type: "literal", value: "assembly" },
      { type: "literal", value: "module" },
    ] },
    lineNumber: 233,
  },
  {
    name: "namespace_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "namespace" },
      { type: "rule_reference", name: "qualified_name" },
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "extern_alias_directive" } },
      { type: "repetition", element: { type: "rule_reference", name: "using_directive" } },
      { type: "repetition", element: { type: "rule_reference", name: "namespace_member_declaration" } },
      { type: "token_reference", name: "RBRACE" },
      { type: "optional", element: { type: "token_reference", name: "SEMICOLON" } },
    ] },
    lineNumber: 242,
  },
  {
    name: "namespace_member_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "namespace_declaration" },
      { type: "rule_reference", name: "type_declaration" },
    ] },
    lineNumber: 252,
  },
  {
    name: "type_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "class_declaration" },
      { type: "rule_reference", name: "struct_declaration" },
      { type: "rule_reference", name: "interface_declaration" },
      { type: "rule_reference", name: "enum_declaration" },
      { type: "rule_reference", name: "delegate_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 255,
  },
  {
    name: "attribute_section",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "attribute_target" },
          { type: "token_reference", name: "COLON" },
        ] } },
      { type: "rule_reference", name: "attribute_list" },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 269,
  },
  {
    name: "attribute_target",
    body: { type: "alternation", choices: [
      { type: "literal", value: "field" },
      { type: "literal", value: "event" },
      { type: "literal", value: "method" },
      { type: "literal", value: "param" },
      { type: "literal", value: "property" },
      { type: "literal", value: "return" },
      { type: "literal", value: "type" },
    ] },
    lineNumber: 271,
  },
  {
    name: "attribute_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "attribute" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "attribute" },
        ] } },
    ] },
    lineNumber: 279,
  },
  {
    name: "attribute",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "namespace_or_type_name" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "optional", element: { type: "rule_reference", name: "attribute_arguments" } },
          { type: "token_reference", name: "RPAREN" },
        ] } },
    ] },
    lineNumber: 281,
  },
  {
    name: "attribute_arguments",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "attribute_argument" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "attribute_argument" },
        ] } },
    ] },
    lineNumber: 283,
  },
  {
    name: "attribute_argument",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "EQUALS" },
        ] } },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 285,
  },
  {
    name: "class_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "class_modifier" } },
      { type: "literal", value: "class" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_list" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "class_base_list" },
        ] } },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_constraints_clauses" } },
      { type: "rule_reference", name: "class_body" },
      { type: "optional", element: { type: "token_reference", name: "SEMICOLON" } },
    ] },
    lineNumber: 324,
  },
  {
    name: "class_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "sealed" },
      { type: "literal", value: "static" },
      { type: "literal", value: "partial" },
    ] },
    lineNumber: 330,
  },
  {
    name: "class_base_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "namespace_or_type_name" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "namespace_or_type_name" },
        ] } },
    ] },
    lineNumber: 340,
  },
  {
    name: "class_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "class_member_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 342,
  },
  {
    name: "class_member_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "constant_declaration" },
      { type: "rule_reference", name: "field_declaration" },
      { type: "rule_reference", name: "method_declaration" },
      { type: "rule_reference", name: "property_declaration" },
      { type: "rule_reference", name: "event_declaration" },
      { type: "rule_reference", name: "indexer_declaration" },
      { type: "rule_reference", name: "operator_declaration" },
      { type: "rule_reference", name: "conversion_operator_declaration" },
      { type: "rule_reference", name: "constructor_declaration" },
      { type: "rule_reference", name: "destructor_declaration" },
      { type: "rule_reference", name: "static_constructor_declaration" },
      { type: "rule_reference", name: "type_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 351,
  },
  {
    name: "constant_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "constant_modifier" } },
      { type: "literal", value: "const" },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "constant_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 371,
  },
  {
    name: "constant_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
    ] },
    lineNumber: 374,
  },
  {
    name: "constant_declarators",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "constant_declarator" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "constant_declarator" },
        ] } },
    ] },
    lineNumber: 380,
  },
  {
    name: "constant_declarator",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 382,
  },
  {
    name: "field_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "field_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "variable_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 391,
  },
  {
    name: "field_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
      { type: "literal", value: "static" },
      { type: "literal", value: "readonly" },
      { type: "literal", value: "volatile" },
    ] },
    lineNumber: 394,
  },
  {
    name: "variable_declarators",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "variable_declarator" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "variable_declarator" },
        ] } },
    ] },
    lineNumber: 403,
  },
  {
    name: "variable_declarator",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "variable_initializer" },
        ] } },
    ] },
    lineNumber: 405,
  },
  {
    name: "variable_initializer",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "expression" },
      { type: "rule_reference", name: "array_initializer" },
    ] },
    lineNumber: 407,
  },
  {
    name: "array_initializer",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "variable_initializer" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "variable_initializer" },
            ] } },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 410,
  },
  {
    name: "method_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "method_modifier" } },
      { type: "rule_reference", name: "return_type" },
      { type: "rule_reference", name: "namespace_or_type_name" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_list" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_constraints_clauses" } },
      { type: "rule_reference", name: "method_body" },
    ] },
    lineNumber: 436,
  },
  {
    name: "method_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
      { type: "literal", value: "static" },
      { type: "literal", value: "virtual" },
      { type: "literal", value: "sealed" },
      { type: "literal", value: "override" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "extern" },
    ] },
    lineNumber: 442,
  },
  {
    name: "return_type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "void" },
      { type: "rule_reference", name: "type" },
    ] },
    lineNumber: 454,
  },
  {
    name: "method_body",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "block" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 457,
  },
  {
    name: "formal_parameter_list",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "fixed_parameters" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "parameter_array" },
          ] } },
      ] },
      { type: "rule_reference", name: "parameter_array" },
    ] },
    lineNumber: 467,
  },
  {
    name: "fixed_parameters",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "fixed_parameter" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "fixed_parameter" },
        ] } },
    ] },
    lineNumber: 470,
  },
  {
    name: "fixed_parameter",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "rule_reference", name: "parameter_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 472,
  },
  {
    name: "parameter_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "ref" },
      { type: "literal", value: "out" },
    ] },
    lineNumber: 474,
  },
  {
    name: "parameter_array",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "literal", value: "params" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 477,
  },
  {
    name: "property_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "property_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "namespace_or_type_name" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "accessor_declarations" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 496,
  },
  {
    name: "property_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
      { type: "literal", value: "static" },
      { type: "literal", value: "virtual" },
      { type: "literal", value: "sealed" },
      { type: "literal", value: "override" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "extern" },
    ] },
    lineNumber: 499,
  },
  {
    name: "accessor_declarations",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "get_accessor_declaration" },
        { type: "optional", element: { type: "rule_reference", name: "set_accessor_declaration" } },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "set_accessor_declaration" },
        { type: "optional", element: { type: "rule_reference", name: "get_accessor_declaration" } },
      ] },
    ] },
    lineNumber: 511,
  },
  {
    name: "get_accessor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "rule_reference", name: "accessor_modifier" } },
      { type: "literal", value: "get" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "block" },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
    ] },
    lineNumber: 514,
  },
  {
    name: "set_accessor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "rule_reference", name: "accessor_modifier" } },
      { type: "literal", value: "set" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "block" },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
    ] },
    lineNumber: 517,
  },
  {
    name: "accessor_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "sequence", elements: [
        { type: "literal", value: "protected" },
        { type: "literal", value: "internal" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "internal" },
        { type: "literal", value: "protected" },
      ] },
    ] },
    lineNumber: 520,
  },
  {
    name: "event_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "event_modifier" } },
      { type: "literal", value: "event" },
      { type: "rule_reference", name: "type" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "rule_reference", name: "variable_declarators" },
            { type: "token_reference", name: "SEMICOLON" },
          ] },
          { type: "sequence", elements: [
            { type: "rule_reference", name: "namespace_or_type_name" },
            { type: "token_reference", name: "LBRACE" },
            { type: "rule_reference", name: "event_accessor_declarations" },
            { type: "token_reference", name: "RBRACE" },
          ] },
        ] } },
    ] },
    lineNumber: 532,
  },
  {
    name: "event_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
      { type: "literal", value: "static" },
      { type: "literal", value: "virtual" },
      { type: "literal", value: "sealed" },
      { type: "literal", value: "override" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "extern" },
    ] },
    lineNumber: 536,
  },
  {
    name: "event_accessor_declarations",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "add_accessor_declaration" },
        { type: "rule_reference", name: "remove_accessor_declaration" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "remove_accessor_declaration" },
        { type: "rule_reference", name: "add_accessor_declaration" },
      ] },
    ] },
    lineNumber: 548,
  },
  {
    name: "add_accessor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "literal", value: "add" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 551,
  },
  {
    name: "remove_accessor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "literal", value: "remove" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 553,
  },
  {
    name: "indexer_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "indexer_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "literal", value: "this" },
      { type: "token_reference", name: "LBRACKET" },
      { type: "rule_reference", name: "formal_parameter_list" },
      { type: "token_reference", name: "RBRACKET" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "accessor_declarations" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 561,
  },
  {
    name: "indexer_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
      { type: "literal", value: "virtual" },
      { type: "literal", value: "sealed" },
      { type: "literal", value: "override" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "extern" },
    ] },
    lineNumber: 565,
  },
  {
    name: "operator_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "rule_reference", name: "operator_modifiers" },
      { type: "rule_reference", name: "type" },
      { type: "literal", value: "operator" },
      { type: "rule_reference", name: "overloadable_operator" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "type" },
          { type: "token_reference", name: "NAME" },
        ] } },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "method_body" },
    ] },
    lineNumber: 582,
  },
  {
    name: "operator_modifiers",
    body: { type: "sequence", elements: [
      { type: "literal", value: "public" },
      { type: "literal", value: "static" },
      { type: "optional", element: { type: "literal", value: "extern" } },
    ] },
    lineNumber: 587,
  },
  {
    name: "overloadable_operator",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "PLUS" },
      { type: "token_reference", name: "MINUS" },
      { type: "token_reference", name: "BANG" },
      { type: "token_reference", name: "TILDE" },
      { type: "token_reference", name: "PLUS_PLUS" },
      { type: "token_reference", name: "MINUS_MINUS" },
      { type: "literal", value: "true" },
      { type: "literal", value: "false" },
      { type: "token_reference", name: "STAR" },
      { type: "token_reference", name: "SLASH" },
      { type: "token_reference", name: "PERCENT" },
      { type: "token_reference", name: "AMPERSAND" },
      { type: "token_reference", name: "PIPE" },
      { type: "token_reference", name: "CARET" },
      { type: "token_reference", name: "LEFT_SHIFT" },
      { type: "token_reference", name: "RIGHT_SHIFT" },
      { type: "token_reference", name: "EQUALS_EQUALS" },
      { type: "token_reference", name: "NOT_EQUALS" },
      { type: "token_reference", name: "LESS_THAN" },
      { type: "token_reference", name: "GREATER_THAN" },
      { type: "token_reference", name: "LESS_EQUALS" },
      { type: "token_reference", name: "GREATER_EQUALS" },
    ] },
    lineNumber: 589,
  },
  {
    name: "conversion_operator_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "rule_reference", name: "operator_modifiers" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "literal", value: "implicit" },
          { type: "literal", value: "explicit" },
        ] } },
      { type: "literal", value: "operator" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "method_body" },
    ] },
    lineNumber: 618,
  },
  {
    name: "constructor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "constructor_modifier" } },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "constructor_initializer" } },
      { type: "rule_reference", name: "method_body" },
    ] },
    lineNumber: 628,
  },
  {
    name: "constructor_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "extern" },
    ] },
    lineNumber: 632,
  },
  {
    name: "constructor_initializer",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "COLON" },
        { type: "literal", value: "base" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "COLON" },
        { type: "literal", value: "this" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 638,
  },
  {
    name: "destructor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "literal", value: "extern" } },
      { type: "token_reference", name: "TILDE" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "method_body" },
    ] },
    lineNumber: 647,
  },
  {
    name: "static_constructor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "rule_reference", name: "static_constructor_modifiers" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "method_body" },
    ] },
    lineNumber: 656,
  },
  {
    name: "static_constructor_modifiers",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "static" },
        { type: "optional", element: { type: "literal", value: "extern" } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "extern" },
        { type: "literal", value: "static" },
      ] },
    ] },
    lineNumber: 659,
  },
  {
    name: "struct_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "struct_modifier" } },
      { type: "literal", value: "struct" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_list" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "interface_type_list" },
        ] } },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_constraints_clauses" } },
      { type: "rule_reference", name: "struct_body" },
      { type: "optional", element: { type: "token_reference", name: "SEMICOLON" } },
    ] },
    lineNumber: 676,
  },
  {
    name: "struct_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
      { type: "literal", value: "partial" },
    ] },
    lineNumber: 682,
  },
  {
    name: "interface_type_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "namespace_or_type_name" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "namespace_or_type_name" },
        ] } },
    ] },
    lineNumber: 689,
  },
  {
    name: "struct_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "struct_member_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 691,
  },
  {
    name: "struct_member_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "constant_declaration" },
      { type: "rule_reference", name: "field_declaration" },
      { type: "rule_reference", name: "method_declaration" },
      { type: "rule_reference", name: "property_declaration" },
      { type: "rule_reference", name: "event_declaration" },
      { type: "rule_reference", name: "indexer_declaration" },
      { type: "rule_reference", name: "operator_declaration" },
      { type: "rule_reference", name: "conversion_operator_declaration" },
      { type: "rule_reference", name: "constructor_declaration" },
      { type: "rule_reference", name: "static_constructor_declaration" },
      { type: "rule_reference", name: "type_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 693,
  },
  {
    name: "interface_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "interface_modifier" } },
      { type: "literal", value: "interface" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_list" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "interface_type_list" },
        ] } },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_constraints_clauses" } },
      { type: "rule_reference", name: "interface_body" },
      { type: "optional", element: { type: "token_reference", name: "SEMICOLON" } },
    ] },
    lineNumber: 727,
  },
  {
    name: "interface_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
      { type: "literal", value: "partial" },
    ] },
    lineNumber: 733,
  },
  {
    name: "interface_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "interface_member_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 740,
  },
  {
    name: "interface_member_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "interface_method_declaration" },
      { type: "rule_reference", name: "interface_property_declaration" },
      { type: "rule_reference", name: "interface_event_declaration" },
      { type: "rule_reference", name: "interface_indexer_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 742,
  },
  {
    name: "interface_method_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "literal", value: "new" } },
      { type: "rule_reference", name: "return_type" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_list" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_constraints_clauses" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 750,
  },
  {
    name: "interface_property_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "literal", value: "new" } },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "interface_accessors" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 756,
  },
  {
    name: "interface_accessors",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "get" },
        { type: "token_reference", name: "SEMICOLON" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "literal", value: "set" },
            { type: "token_reference", name: "SEMICOLON" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "set" },
        { type: "token_reference", name: "SEMICOLON" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "literal", value: "get" },
            { type: "token_reference", name: "SEMICOLON" },
          ] } },
      ] },
    ] },
    lineNumber: 759,
  },
  {
    name: "interface_event_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "literal", value: "new" } },
      { type: "literal", value: "event" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 762,
  },
  {
    name: "interface_indexer_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "literal", value: "new" } },
      { type: "rule_reference", name: "type" },
      { type: "literal", value: "this" },
      { type: "token_reference", name: "LBRACKET" },
      { type: "rule_reference", name: "formal_parameter_list" },
      { type: "token_reference", name: "RBRACKET" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "interface_accessors" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 764,
  },
  {
    name: "enum_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "enum_modifier" } },
      { type: "literal", value: "enum" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "integral_type" },
        ] } },
      { type: "rule_reference", name: "enum_body" },
      { type: "optional", element: { type: "token_reference", name: "SEMICOLON" } },
    ] },
    lineNumber: 774,
  },
  {
    name: "enum_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
    ] },
    lineNumber: 778,
  },
  {
    name: "integral_type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "byte" },
      { type: "literal", value: "sbyte" },
      { type: "literal", value: "short" },
      { type: "literal", value: "ushort" },
      { type: "literal", value: "int" },
      { type: "literal", value: "uint" },
      { type: "literal", value: "long" },
      { type: "literal", value: "ulong" },
    ] },
    lineNumber: 784,
  },
  {
    name: "enum_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "rule_reference", name: "enum_member_declarations" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 793,
  },
  {
    name: "enum_member_declarations",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "enum_member_declaration" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "enum_member_declaration" },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
    ] },
    lineNumber: 795,
  },
  {
    name: "enum_member_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 798,
  },
  {
    name: "delegate_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "delegate_modifier" } },
      { type: "literal", value: "delegate" },
      { type: "rule_reference", name: "return_type" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_list" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_constraints_clauses" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 817,
  },
  {
    name: "delegate_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
    ] },
    lineNumber: 823,
  },
  {
    name: "type",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "non_array_type" },
        { type: "repetition", element: { type: "rule_reference", name: "rank_specifier" } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "void" },
        { type: "token_reference", name: "STAR" },
      ] },
    ] },
    lineNumber: 859,
  },
  {
    name: "non_array_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "value_type" },
      { type: "rule_reference", name: "reference_type" },
    ] },
    lineNumber: 862,
  },
  {
    name: "value_type",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "primitive_type" },
        { type: "optional", element: { type: "token_reference", name: "QUESTION" } },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "namespace_or_type_name" },
        { type: "optional", element: { type: "token_reference", name: "QUESTION" } },
      ] },
    ] },
    lineNumber: 865,
  },
  {
    name: "reference_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "namespace_or_type_name" },
      { type: "literal", value: "object" },
      { type: "literal", value: "string" },
    ] },
    lineNumber: 868,
  },
  {
    name: "primitive_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "numeric_type" },
      { type: "literal", value: "bool" },
    ] },
    lineNumber: 872,
  },
  {
    name: "numeric_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "integral_type" },
      { type: "rule_reference", name: "floating_point_type" },
      { type: "literal", value: "decimal" },
    ] },
    lineNumber: 875,
  },
  {
    name: "floating_point_type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "float" },
      { type: "literal", value: "double" },
    ] },
    lineNumber: 879,
  },
  {
    name: "rank_specifier",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "repetition", element: { type: "token_reference", name: "COMMA" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 882,
  },
  {
    name: "pointer_type",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "STAR" },
    ] },
    lineNumber: 884,
  },
  {
    name: "block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 926,
  },
  {
    name: "statement",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "block" },
      { type: "rule_reference", name: "local_variable_declaration_statement" },
      { type: "rule_reference", name: "local_constant_declaration_statement" },
      { type: "rule_reference", name: "empty_statement" },
      { type: "rule_reference", name: "expression_statement" },
      { type: "rule_reference", name: "if_statement" },
      { type: "rule_reference", name: "while_statement" },
      { type: "rule_reference", name: "do_while_statement" },
      { type: "rule_reference", name: "for_statement" },
      { type: "rule_reference", name: "foreach_statement" },
      { type: "rule_reference", name: "switch_statement" },
      { type: "rule_reference", name: "try_statement" },
      { type: "rule_reference", name: "throw_statement" },
      { type: "rule_reference", name: "return_statement" },
      { type: "rule_reference", name: "break_statement" },
      { type: "rule_reference", name: "continue_statement" },
      { type: "rule_reference", name: "goto_statement" },
      { type: "rule_reference", name: "lock_statement" },
      { type: "rule_reference", name: "using_statement" },
      { type: "rule_reference", name: "checked_statement" },
      { type: "rule_reference", name: "unchecked_statement" },
      { type: "rule_reference", name: "labelled_statement" },
      { type: "rule_reference", name: "unsafe_statement" },
      { type: "rule_reference", name: "fixed_statement" },
      { type: "rule_reference", name: "yield_statement" },
    ] },
    lineNumber: 928,
  },
  {
    name: "local_variable_declaration_statement",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "variable_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 954,
  },
  {
    name: "local_constant_declaration_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "const" },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "constant_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 956,
  },
  {
    name: "empty_statement",
    body: { type: "token_reference", name: "SEMICOLON" },
    lineNumber: 958,
  },
  {
    name: "expression_statement",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 960,
  },
  {
    name: "if_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "if" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "else" },
          { type: "rule_reference", name: "statement" },
        ] } },
    ] },
    lineNumber: 962,
  },
  {
    name: "while_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "while" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 964,
  },
  {
    name: "do_while_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "do" },
      { type: "rule_reference", name: "statement" },
      { type: "literal", value: "while" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 966,
  },
  {
    name: "for_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "for" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "for_initializer" } },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "optional", element: { type: "rule_reference", name: "for_iterator" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 968,
  },
  {
    name: "for_initializer",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "local_variable_declaration" },
      { type: "rule_reference", name: "expression_list" },
    ] },
    lineNumber: 971,
  },
  {
    name: "for_iterator",
    body: { type: "rule_reference", name: "expression_list" },
    lineNumber: 974,
  },
  {
    name: "local_variable_declaration",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "variable_declarators" },
    ] },
    lineNumber: 976,
  },
  {
    name: "expression_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 978,
  },
  {
    name: "foreach_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "foreach" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 980,
  },
  {
    name: "switch_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "switch" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "switch_block" },
    ] },
    lineNumber: 982,
  },
  {
    name: "switch_block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "switch_section" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 984,
  },
  {
    name: "switch_section",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "switch_label" } },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
    ] },
    lineNumber: 986,
  },
  {
    name: "switch_label",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "case" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "COLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "default" },
        { type: "token_reference", name: "COLON" },
      ] },
    ] },
    lineNumber: 988,
  },
  {
    name: "try_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "try" },
      { type: "rule_reference", name: "block" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "rule_reference", name: "catch_clauses" },
            { type: "optional", element: { type: "rule_reference", name: "finally_clause" } },
          ] },
          { type: "rule_reference", name: "finally_clause" },
        ] } },
    ] },
    lineNumber: 991,
  },
  {
    name: "catch_clauses",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "specific_catch_clause" },
        { type: "repetition", element: { type: "rule_reference", name: "specific_catch_clause" } },
        { type: "optional", element: { type: "rule_reference", name: "general_catch_clause" } },
      ] },
      { type: "rule_reference", name: "general_catch_clause" },
    ] },
    lineNumber: 994,
  },
  {
    name: "specific_catch_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "catch" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "namespace_or_type_name" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 997,
  },
  {
    name: "general_catch_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "catch" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 999,
  },
  {
    name: "finally_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "finally" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1001,
  },
  {
    name: "throw_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "throw" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1003,
  },
  {
    name: "return_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "return" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1005,
  },
  {
    name: "break_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "break" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1007,
  },
  {
    name: "continue_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "continue" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1009,
  },
  {
    name: "goto_statement",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "goto" },
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "goto" },
        { type: "literal", value: "case" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "goto" },
        { type: "literal", value: "default" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 1011,
  },
  {
    name: "lock_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "lock" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 1015,
  },
  {
    name: "using_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "using" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "resource_acquisition" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 1017,
  },
  {
    name: "resource_acquisition",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "type" },
        { type: "rule_reference", name: "variable_declarators" },
      ] },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 1019,
  },
  {
    name: "checked_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "checked" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1022,
  },
  {
    name: "unchecked_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "unchecked" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1024,
  },
  {
    name: "labelled_statement",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 1026,
  },
  {
    name: "unsafe_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "unsafe" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1028,
  },
  {
    name: "fixed_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "fixed" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "variable_declarators" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 1030,
  },
  {
    name: "yield_statement",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "yield" },
        { type: "literal", value: "return" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "yield" },
        { type: "literal", value: "break" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 1044,
  },
  {
    name: "expression",
    body: { type: "rule_reference", name: "assignment_expression" },
    lineNumber: 1074,
  },
  {
    name: "assignment_expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "conditional_expression" },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "unary_expression" },
        { type: "rule_reference", name: "assignment_operator" },
        { type: "rule_reference", name: "assignment_expression" },
      ] },
    ] },
    lineNumber: 1076,
  },
  {
    name: "assignment_operator",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "EQUALS" },
      { type: "token_reference", name: "PLUS_EQUALS" },
      { type: "token_reference", name: "MINUS_EQUALS" },
      { type: "token_reference", name: "STAR_EQUALS" },
      { type: "token_reference", name: "SLASH_EQUALS" },
      { type: "token_reference", name: "PERCENT_EQUALS" },
      { type: "token_reference", name: "AMPERSAND_EQUALS" },
      { type: "token_reference", name: "PIPE_EQUALS" },
      { type: "token_reference", name: "CARET_EQUALS" },
      { type: "token_reference", name: "LEFT_SHIFT_EQUALS" },
      { type: "token_reference", name: "RIGHT_SHIFT_EQUALS" },
    ] },
    lineNumber: 1079,
  },
  {
    name: "conditional_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "null_coalescing_expression" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "QUESTION" },
          { type: "rule_reference", name: "expression" },
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 1093,
  },
  {
    name: "null_coalescing_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "logical_or_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "QUESTION_QUESTION" },
          { type: "rule_reference", name: "logical_or_expression" },
        ] } },
    ] },
    lineNumber: 1107,
  },
  {
    name: "logical_or_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "logical_and_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "OR_OR" },
          { type: "rule_reference", name: "logical_and_expression" },
        ] } },
    ] },
    lineNumber: 1112,
  },
  {
    name: "logical_and_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "bitwise_or_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "AND_AND" },
          { type: "rule_reference", name: "bitwise_or_expression" },
        ] } },
    ] },
    lineNumber: 1116,
  },
  {
    name: "bitwise_or_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "bitwise_xor_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "PIPE" },
          { type: "rule_reference", name: "bitwise_xor_expression" },
        ] } },
    ] },
    lineNumber: 1120,
  },
  {
    name: "bitwise_xor_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "bitwise_and_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "CARET" },
          { type: "rule_reference", name: "bitwise_and_expression" },
        ] } },
    ] },
    lineNumber: 1124,
  },
  {
    name: "bitwise_and_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "equality_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "AMPERSAND" },
          { type: "rule_reference", name: "equality_expression" },
        ] } },
    ] },
    lineNumber: 1128,
  },
  {
    name: "equality_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "relational_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "EQUALS_EQUALS" },
              { type: "token_reference", name: "NOT_EQUALS" },
            ] } },
          { type: "rule_reference", name: "relational_expression" },
        ] } },
    ] },
    lineNumber: 1132,
  },
  {
    name: "relational_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "shift_expression" },
      { type: "repetition", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "group", element: { type: "alternation", choices: [
                { type: "token_reference", name: "LESS_THAN" },
                { type: "token_reference", name: "GREATER_THAN" },
                { type: "token_reference", name: "LESS_EQUALS" },
                { type: "token_reference", name: "GREATER_EQUALS" },
              ] } },
            { type: "rule_reference", name: "shift_expression" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "is" },
            { type: "rule_reference", name: "type" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "as" },
            { type: "rule_reference", name: "type" },
          ] },
        ] } },
    ] },
    lineNumber: 1142,
  },
  {
    name: "shift_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "additive_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "LEFT_SHIFT" },
              { type: "token_reference", name: "RIGHT_SHIFT" },
            ] } },
          { type: "rule_reference", name: "additive_expression" },
        ] } },
    ] },
    lineNumber: 1150,
  },
  {
    name: "additive_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "multiplicative_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "PLUS" },
              { type: "token_reference", name: "MINUS" },
            ] } },
          { type: "rule_reference", name: "multiplicative_expression" },
        ] } },
    ] },
    lineNumber: 1155,
  },
  {
    name: "multiplicative_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "unary_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "STAR" },
              { type: "token_reference", name: "SLASH" },
              { type: "token_reference", name: "PERCENT" },
            ] } },
          { type: "rule_reference", name: "unary_expression" },
        ] } },
    ] },
    lineNumber: 1160,
  },
  {
    name: "unary_expression",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "PLUS_PLUS" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "MINUS_MINUS" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "PLUS" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "MINUS" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "BANG" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "TILDE" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "rule_reference", name: "cast_expression" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "AMPERSAND" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "STAR" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "rule_reference", name: "postfix_expression" },
    ] },
    lineNumber: 1165,
  },
  {
    name: "cast_expression",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "unary_expression" },
    ] },
    lineNumber: 1176,
  },
  {
    name: "postfix_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "primary_expression" },
      { type: "repetition", element: { type: "alternation", choices: [
          { type: "token_reference", name: "PLUS_PLUS" },
          { type: "token_reference", name: "MINUS_MINUS" },
        ] } },
    ] },
    lineNumber: 1180,
  },
  {
    name: "primary_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "primary" },
      { type: "repetition", element: { type: "rule_reference", name: "primary_suffix" } },
    ] },
    lineNumber: 1191,
  },
  {
    name: "primary_suffix",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "DOT" },
        { type: "token_reference", name: "NAME" },
        { type: "optional", element: { type: "rule_reference", name: "type_argument_list" } },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LBRACKET" },
        { type: "rule_reference", name: "expression_list" },
        { type: "token_reference", name: "RBRACKET" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "ARROW" },
        { type: "token_reference", name: "NAME" },
      ] },
    ] },
    lineNumber: 1193,
  },
  {
    name: "primary",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "literal" },
      { type: "literal", value: "this" },
      { type: "sequence", elements: [
        { type: "literal", value: "base" },
        { type: "token_reference", name: "DOT" },
        { type: "token_reference", name: "NAME" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "base" },
        { type: "token_reference", name: "LBRACKET" },
        { type: "rule_reference", name: "expression_list" },
        { type: "token_reference", name: "RBRACKET" },
      ] },
      { type: "rule_reference", name: "typeof_expression" },
      { type: "rule_reference", name: "sizeof_expression" },
      { type: "rule_reference", name: "checked_expression" },
      { type: "rule_reference", name: "unchecked_expression" },
      { type: "rule_reference", name: "default_value_expression" },
      { type: "rule_reference", name: "new_expression" },
      { type: "rule_reference", name: "anonymous_method_expression" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COLON_COLON" },
            { type: "token_reference", name: "NAME" },
          ] } },
        { type: "optional", element: { type: "rule_reference", name: "type_argument_list" } },
      ] },
    ] },
    lineNumber: 1198,
  },
  {
    name: "typeof_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "typeof" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type_or_void" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 1219,
  },
  {
    name: "type_or_void",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "type" },
      { type: "literal", value: "void" },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "namespace_or_type_name" },
        { type: "token_reference", name: "LESS_THAN" },
        { type: "repetition", element: { type: "token_reference", name: "COMMA" } },
        { type: "token_reference", name: "GREATER_THAN" },
      ] },
    ] },
    lineNumber: 1221,
  },
  {
    name: "sizeof_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "sizeof" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 1227,
  },
  {
    name: "checked_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "checked" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 1231,
  },
  {
    name: "unchecked_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "unchecked" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 1233,
  },
  {
    name: "default_value_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "default" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 1242,
  },
  {
    name: "new_expression",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "new" },
        { type: "rule_reference", name: "new_object_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "new" },
        { type: "rule_reference", name: "new_array_expression" },
      ] },
    ] },
    lineNumber: 1251,
  },
  {
    name: "new_object_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "namespace_or_type_name" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 1254,
  },
  {
    name: "new_array_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "array_type" },
      { type: "rule_reference", name: "array_creation_suffix" },
    ] },
    lineNumber: 1256,
  },
  {
    name: "array_type",
    body: { type: "group", element: { type: "alternation", choices: [
        { type: "rule_reference", name: "primitive_type" },
        { type: "rule_reference", name: "namespace_or_type_name" },
      ] } },
    lineNumber: 1258,
  },
  {
    name: "array_creation_suffix",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "rank_specifier" },
        { type: "repetition", element: { type: "rule_reference", name: "rank_specifier" } },
        { type: "rule_reference", name: "array_initializer" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LBRACKET" },
        { type: "rule_reference", name: "expression_list" },
        { type: "token_reference", name: "RBRACKET" },
        { type: "repetition", element: { type: "rule_reference", name: "rank_specifier" } },
        { type: "optional", element: { type: "rule_reference", name: "array_initializer" } },
      ] },
    ] },
    lineNumber: 1260,
  },
  {
    name: "anonymous_method_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "delegate" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "optional", element: { type: "rule_reference", name: "anonymous_method_parameter_list" } },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1292,
  },
  {
    name: "anonymous_method_parameter_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "anonymous_method_parameter" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "anonymous_method_parameter" },
        ] } },
    ] },
    lineNumber: 1295,
  },
  {
    name: "anonymous_method_parameter",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "rule_reference", name: "parameter_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 1298,
  },
  {
    name: "argument_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "argument" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "argument" },
        ] } },
    ] },
    lineNumber: 1302,
  },
  {
    name: "argument",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "alternation", choices: [
          { type: "literal", value: "ref" },
          { type: "literal", value: "out" },
        ] } },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 1304,
  },
  {
    name: "literal",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "CHAR" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "VERBATIM_STRING" },
      { type: "literal", value: "true" },
      { type: "literal", value: "false" },
      { type: "literal", value: "null" },
    ] },
    lineNumber: 1312,
  },
],
};
