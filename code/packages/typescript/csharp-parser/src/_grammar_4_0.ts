// AUTO-GENERATED FILE - DO NOT EDIT
// Source: csharp4.0.grammar
// Regenerate with: grammar-tools compile-grammar csharp4.0.grammar
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
    lineNumber: 130,
  },
  {
    name: "extern_alias_directive",
    body: { type: "sequence", elements: [
      { type: "literal", value: "extern" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 139,
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
        { type: "rule_reference", name: "qualified_name" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 147,
  },
  {
    name: "qualified_name",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "DOT" },
            { type: "token_reference", name: "NAME" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "COLON_COLON" },
        { type: "token_reference", name: "NAME" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "DOT" },
            { type: "token_reference", name: "NAME" },
          ] } },
      ] },
    ] },
    lineNumber: 154,
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
    lineNumber: 157,
  },
  {
    name: "namespace_or_type_part",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "COLON_COLON" },
        { type: "token_reference", name: "NAME" },
        { type: "optional", element: { type: "rule_reference", name: "type_argument_list" } },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "optional", element: { type: "rule_reference", name: "type_argument_list" } },
      ] },
    ] },
    lineNumber: 159,
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
    lineNumber: 197,
  },
  {
    name: "type_parameter",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "rule_reference", name: "variance_annotation" } },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 201,
  },
  {
    name: "variance_annotation",
    body: { type: "alternation", choices: [
      { type: "literal", value: "in" },
      { type: "literal", value: "out" },
    ] },
    lineNumber: 203,
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
    lineNumber: 206,
  },
  {
    name: "type_parameter_constraints_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "where" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "type_parameter_constraints" },
    ] },
    lineNumber: 212,
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
    lineNumber: 214,
  },
  {
    name: "primary_constraint",
    body: { type: "alternation", choices: [
      { type: "literal", value: "class" },
      { type: "literal", value: "struct" },
      { type: "rule_reference", name: "namespace_or_type_name" },
    ] },
    lineNumber: 219,
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
    lineNumber: 223,
  },
  {
    name: "constructor_constraint",
    body: { type: "sequence", elements: [
      { type: "literal", value: "new" },
      { type: "token_reference", name: "LPAREN" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 225,
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
    lineNumber: 240,
  },
  {
    name: "namespace_member_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "namespace_declaration" },
      { type: "rule_reference", name: "type_declaration" },
    ] },
    lineNumber: 250,
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
    lineNumber: 253,
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
    lineNumber: 264,
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
    lineNumber: 266,
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
    lineNumber: 274,
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
    lineNumber: 276,
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
    lineNumber: 278,
  },
  {
    name: "attribute_argument",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "COLON" },
        ] } },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 280,
  },
  {
    name: "class_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "class_modifier" } },
      { type: "optional", element: { type: "literal", value: "partial" } },
      { type: "literal", value: "class" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_list" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "class_base_list" },
        ] } },
      { type: "repetition", element: { type: "rule_reference", name: "type_parameter_constraints_clause" } },
      { type: "rule_reference", name: "class_body" },
      { type: "optional", element: { type: "token_reference", name: "SEMICOLON" } },
    ] },
    lineNumber: 290,
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
    ] },
    lineNumber: 296,
  },
  {
    name: "class_base_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type_name" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "type_name" },
        ] } },
    ] },
    lineNumber: 305,
  },
  {
    name: "class_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "class_member_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 307,
  },
  {
    name: "type_name",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "DOT" },
          { type: "token_reference", name: "NAME" },
        ] } },
      { type: "optional", element: { type: "rule_reference", name: "type_argument_list" } },
    ] },
    lineNumber: 322,
  },
  {
    name: "type_argument_list",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LESS_THAN" },
      { type: "rule_reference", name: "type_argument" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "type_argument" },
        ] } },
      { type: "token_reference", name: "GREATER_THAN" },
    ] },
    lineNumber: 324,
  },
  {
    name: "type_argument",
    body: { type: "rule_reference", name: "type" },
    lineNumber: 326,
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
    lineNumber: 332,
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
    lineNumber: 350,
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
    lineNumber: 353,
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
    lineNumber: 359,
  },
  {
    name: "constant_declarator",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 361,
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
    lineNumber: 367,
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
    lineNumber: 370,
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
    lineNumber: 379,
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
    lineNumber: 381,
  },
  {
    name: "variable_initializer",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "expression" },
      { type: "rule_reference", name: "array_initializer" },
    ] },
    lineNumber: 383,
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
    lineNumber: 386,
  },
  {
    name: "method_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "method_modifier" } },
      { type: "optional", element: { type: "literal", value: "partial" } },
      { type: "rule_reference", name: "return_type" },
      { type: "rule_reference", name: "qualified_name" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_list" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "repetition", element: { type: "rule_reference", name: "type_parameter_constraints_clause" } },
      { type: "rule_reference", name: "method_body" },
    ] },
    lineNumber: 395,
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
    lineNumber: 401,
  },
  {
    name: "return_type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "void" },
      { type: "rule_reference", name: "type" },
    ] },
    lineNumber: 413,
  },
  {
    name: "method_body",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "block" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 416,
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
    lineNumber: 451,
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
    lineNumber: 454,
  },
  {
    name: "fixed_parameter",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "rule_reference", name: "parameter_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 457,
  },
  {
    name: "parameter_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "ref" },
      { type: "literal", value: "out" },
      { type: "literal", value: "this" },
    ] },
    lineNumber: 460,
  },
  {
    name: "parameter_array",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "literal", value: "params" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 464,
  },
  {
    name: "property_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "property_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "qualified_name" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "accessor_declarations" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 472,
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
    lineNumber: 475,
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
    lineNumber: 487,
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
    lineNumber: 490,
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
    lineNumber: 493,
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
    lineNumber: 496,
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
            { type: "rule_reference", name: "qualified_name" },
            { type: "token_reference", name: "LBRACE" },
            { type: "rule_reference", name: "event_accessor_declarations" },
            { type: "token_reference", name: "RBRACE" },
          ] },
        ] } },
    ] },
    lineNumber: 506,
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
    lineNumber: 510,
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
    lineNumber: 522,
  },
  {
    name: "add_accessor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "literal", value: "add" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 525,
  },
  {
    name: "remove_accessor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "literal", value: "remove" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 527,
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
    lineNumber: 533,
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
    lineNumber: 537,
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
    lineNumber: 552,
  },
  {
    name: "operator_modifiers",
    body: { type: "sequence", elements: [
      { type: "literal", value: "public" },
      { type: "literal", value: "static" },
      { type: "optional", element: { type: "literal", value: "extern" } },
    ] },
    lineNumber: 557,
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
    lineNumber: 559,
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
    lineNumber: 586,
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
    lineNumber: 594,
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
    lineNumber: 598,
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
    lineNumber: 604,
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
    lineNumber: 611,
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
    lineNumber: 618,
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
    lineNumber: 621,
  },
  {
    name: "struct_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "struct_modifier" } },
      { type: "optional", element: { type: "literal", value: "partial" } },
      { type: "literal", value: "struct" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_list" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "interface_type_list" },
        ] } },
      { type: "repetition", element: { type: "rule_reference", name: "type_parameter_constraints_clause" } },
      { type: "rule_reference", name: "struct_body" },
      { type: "optional", element: { type: "token_reference", name: "SEMICOLON" } },
    ] },
    lineNumber: 628,
  },
  {
    name: "struct_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
    ] },
    lineNumber: 634,
  },
  {
    name: "interface_type_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type_name" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "type_name" },
        ] } },
    ] },
    lineNumber: 640,
  },
  {
    name: "struct_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "struct_member_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 642,
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
    lineNumber: 644,
  },
  {
    name: "interface_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "interface_modifier" } },
      { type: "optional", element: { type: "literal", value: "partial" } },
      { type: "literal", value: "interface" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_list" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "interface_type_list" },
        ] } },
      { type: "repetition", element: { type: "rule_reference", name: "type_parameter_constraints_clause" } },
      { type: "rule_reference", name: "interface_body" },
      { type: "optional", element: { type: "token_reference", name: "SEMICOLON" } },
    ] },
    lineNumber: 678,
  },
  {
    name: "interface_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
    ] },
    lineNumber: 684,
  },
  {
    name: "interface_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "interface_member_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 690,
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
    lineNumber: 692,
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
      { type: "repetition", element: { type: "rule_reference", name: "type_parameter_constraints_clause" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 698,
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
    lineNumber: 704,
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
    lineNumber: 707,
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
    lineNumber: 710,
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
    lineNumber: 712,
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
    lineNumber: 720,
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
    lineNumber: 724,
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
    lineNumber: 730,
  },
  {
    name: "enum_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "rule_reference", name: "enum_member_declarations" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 739,
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
    lineNumber: 741,
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
    lineNumber: 744,
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
      { type: "repetition", element: { type: "rule_reference", name: "type_parameter_constraints_clause" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 759,
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
    lineNumber: 765,
  },
  {
    name: "type",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "dynamic" },
        { type: "repetition", element: { type: "rule_reference", name: "rank_specifier" } },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "value_type" },
        { type: "repetition", element: { type: "rule_reference", name: "rank_specifier" } },
        { type: "optional", element: { type: "token_reference", name: "QUESTION" } },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "reference_type" },
        { type: "repetition", element: { type: "rule_reference", name: "rank_specifier" } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "void" },
        { type: "token_reference", name: "STAR" },
      ] },
    ] },
    lineNumber: 797,
  },
  {
    name: "value_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "primitive_type" },
      { type: "rule_reference", name: "type_name" },
    ] },
    lineNumber: 802,
  },
  {
    name: "reference_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "type_name" },
      { type: "literal", value: "object" },
      { type: "literal", value: "string" },
    ] },
    lineNumber: 805,
  },
  {
    name: "primitive_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "numeric_type" },
      { type: "literal", value: "bool" },
    ] },
    lineNumber: 809,
  },
  {
    name: "numeric_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "integral_type" },
      { type: "rule_reference", name: "floating_point_type" },
      { type: "literal", value: "decimal" },
    ] },
    lineNumber: 812,
  },
  {
    name: "floating_point_type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "float" },
      { type: "literal", value: "double" },
    ] },
    lineNumber: 816,
  },
  {
    name: "rank_specifier",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "repetition", element: { type: "token_reference", name: "COMMA" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 819,
  },
  {
    name: "pointer_type",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "STAR" },
    ] },
    lineNumber: 821,
  },
  {
    name: "block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 829,
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
    lineNumber: 831,
  },
  {
    name: "local_variable_declaration_statement",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "variable_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 857,
  },
  {
    name: "local_constant_declaration_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "const" },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "constant_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 859,
  },
  {
    name: "empty_statement",
    body: { type: "token_reference", name: "SEMICOLON" },
    lineNumber: 861,
  },
  {
    name: "expression_statement",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 863,
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
    lineNumber: 865,
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
    lineNumber: 867,
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
    lineNumber: 869,
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
    lineNumber: 871,
  },
  {
    name: "for_initializer",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "local_variable_declaration" },
      { type: "rule_reference", name: "expression_list" },
    ] },
    lineNumber: 874,
  },
  {
    name: "for_iterator",
    body: { type: "rule_reference", name: "expression_list" },
    lineNumber: 877,
  },
  {
    name: "local_variable_declaration",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "variable_declarators" },
    ] },
    lineNumber: 879,
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
    lineNumber: 881,
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
    lineNumber: 883,
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
    lineNumber: 885,
  },
  {
    name: "switch_block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "switch_section" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 887,
  },
  {
    name: "switch_section",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "switch_label" } },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
    ] },
    lineNumber: 889,
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
    lineNumber: 891,
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
    lineNumber: 894,
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
    lineNumber: 897,
  },
  {
    name: "specific_catch_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "catch" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type_name" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 900,
  },
  {
    name: "general_catch_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "catch" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 902,
  },
  {
    name: "finally_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "finally" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 904,
  },
  {
    name: "throw_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "throw" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 906,
  },
  {
    name: "return_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "return" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 908,
  },
  {
    name: "break_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "break" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 910,
  },
  {
    name: "continue_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "continue" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 912,
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
    lineNumber: 914,
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
    lineNumber: 918,
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
    lineNumber: 920,
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
    lineNumber: 922,
  },
  {
    name: "checked_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "checked" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 925,
  },
  {
    name: "unchecked_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "unchecked" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 927,
  },
  {
    name: "labelled_statement",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 929,
  },
  {
    name: "unsafe_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "unsafe" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 931,
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
    lineNumber: 933,
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
    lineNumber: 935,
  },
  {
    name: "expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "assignment_expression" },
      { type: "rule_reference", name: "lambda_expression" },
      { type: "rule_reference", name: "query_expression" },
    ] },
    lineNumber: 954,
  },
  {
    name: "lambda_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "lambda_parameters" },
      { type: "token_reference", name: "LAMBDA" },
      { type: "rule_reference", name: "lambda_body" },
    ] },
    lineNumber: 962,
  },
  {
    name: "lambda_parameters",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NAME" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "lambda_parameter_list" } },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 964,
  },
  {
    name: "lambda_parameter_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "lambda_parameter" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "lambda_parameter" },
        ] } },
    ] },
    lineNumber: 967,
  },
  {
    name: "lambda_parameter",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "rule_reference", name: "type" } },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 969,
  },
  {
    name: "lambda_body",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "expression" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 971,
  },
  {
    name: "query_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "from_clause" },
      { type: "rule_reference", name: "query_body" },
    ] },
    lineNumber: 978,
  },
  {
    name: "from_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "from" },
      { type: "optional", element: { type: "rule_reference", name: "type" } },
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 980,
  },
  {
    name: "query_body",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "query_body_clause" } },
      { type: "rule_reference", name: "select_or_group_clause" },
      { type: "optional", element: { type: "rule_reference", name: "query_continuation" } },
    ] },
    lineNumber: 982,
  },
  {
    name: "query_body_clause",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "from_clause" },
      { type: "rule_reference", name: "let_clause" },
      { type: "rule_reference", name: "where_clause" },
      { type: "rule_reference", name: "join_clause" },
      { type: "rule_reference", name: "orderby_clause" },
    ] },
    lineNumber: 984,
  },
  {
    name: "let_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "let" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 990,
  },
  {
    name: "where_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "where" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 992,
  },
  {
    name: "join_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "join" },
      { type: "optional", element: { type: "rule_reference", name: "type" } },
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "expression" },
      { type: "literal", value: "on" },
      { type: "rule_reference", name: "expression" },
      { type: "literal", value: "equals" },
      { type: "rule_reference", name: "expression" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "into" },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 994,
  },
  {
    name: "orderby_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "orderby" },
      { type: "rule_reference", name: "ordering" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "ordering" },
        ] } },
    ] },
    lineNumber: 998,
  },
  {
    name: "ordering",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "optional", element: { type: "alternation", choices: [
          { type: "literal", value: "ascending" },
          { type: "literal", value: "descending" },
        ] } },
    ] },
    lineNumber: 1000,
  },
  {
    name: "select_or_group_clause",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "select" },
        { type: "rule_reference", name: "expression" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "group" },
        { type: "rule_reference", name: "expression" },
        { type: "literal", value: "by" },
        { type: "rule_reference", name: "expression" },
      ] },
    ] },
    lineNumber: 1002,
  },
  {
    name: "query_continuation",
    body: { type: "sequence", elements: [
      { type: "literal", value: "into" },
      { type: "token_reference", name: "NAME" },
      { type: "rule_reference", name: "query_body" },
    ] },
    lineNumber: 1005,
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
    lineNumber: 1011,
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
    lineNumber: 1014,
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
    lineNumber: 1026,
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
    lineNumber: 1029,
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
    lineNumber: 1031,
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
    lineNumber: 1033,
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
    lineNumber: 1035,
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
    lineNumber: 1037,
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
    lineNumber: 1039,
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
    lineNumber: 1041,
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
    lineNumber: 1044,
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
    lineNumber: 1050,
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
    lineNumber: 1053,
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
    lineNumber: 1056,
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
    lineNumber: 1065,
  },
  {
    name: "cast_expression",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "unary_expression" },
    ] },
    lineNumber: 1076,
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
    lineNumber: 1078,
  },
  {
    name: "primary_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "primary" },
      { type: "repetition", element: { type: "rule_reference", name: "primary_suffix" } },
    ] },
    lineNumber: 1088,
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
    lineNumber: 1090,
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
        { type: "optional", element: { type: "rule_reference", name: "type_argument_list" } },
      ] },
    ] },
    lineNumber: 1095,
  },
  {
    name: "typeof_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "typeof" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type_or_void" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 1109,
  },
  {
    name: "type_or_void",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "type" },
      { type: "literal", value: "void" },
    ] },
    lineNumber: 1111,
  },
  {
    name: "sizeof_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "sizeof" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 1113,
  },
  {
    name: "checked_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "checked" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 1115,
  },
  {
    name: "unchecked_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "unchecked" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 1117,
  },
  {
    name: "default_value_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "default" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 1119,
  },
  {
    name: "anonymous_method_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "delegate" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1121,
  },
  {
    name: "new_expression",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "new" },
        { type: "rule_reference", name: "new_anonymous_type" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "new" },
        { type: "rule_reference", name: "new_implicitly_typed_array" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "new" },
        { type: "rule_reference", name: "new_object_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "new" },
        { type: "rule_reference", name: "new_array_expression" },
      ] },
    ] },
    lineNumber: 1127,
  },
  {
    name: "new_anonymous_type",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "anonymous_type_member" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "anonymous_type_member" },
            ] } },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 1132,
  },
  {
    name: "anonymous_type_member",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "EQUALS" },
        ] } },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 1134,
  },
  {
    name: "new_implicitly_typed_array",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "token_reference", name: "RBRACKET" },
      { type: "rule_reference", name: "array_initializer" },
    ] },
    lineNumber: 1136,
  },
  {
    name: "new_object_expression",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "type_name" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "object_or_collection_initializer" } },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "type_name" },
        { type: "rule_reference", name: "object_or_collection_initializer" },
      ] },
    ] },
    lineNumber: 1138,
  },
  {
    name: "object_or_collection_initializer",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "rule_reference", name: "initializer_list" } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 1141,
  },
  {
    name: "initializer_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "initializer_item" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "initializer_item" },
        ] } },
    ] },
    lineNumber: 1143,
  },
  {
    name: "initializer_item",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "EQUALS" },
        { type: "group", element: { type: "alternation", choices: [
            { type: "rule_reference", name: "expression" },
            { type: "rule_reference", name: "object_or_collection_initializer" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "expression_list" },
        { type: "token_reference", name: "RBRACE" },
      ] },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 1145,
  },
  {
    name: "new_array_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "array_type" },
      { type: "rule_reference", name: "array_creation_suffix" },
    ] },
    lineNumber: 1149,
  },
  {
    name: "array_type",
    body: { type: "group", element: { type: "alternation", choices: [
        { type: "rule_reference", name: "primitive_type" },
        { type: "rule_reference", name: "type_name" },
      ] } },
    lineNumber: 1151,
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
    lineNumber: 1153,
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
    lineNumber: 1191,
  },
  {
    name: "argument",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "COLON" },
        ] } },
      { type: "optional", element: { type: "alternation", choices: [
          { type: "literal", value: "ref" },
          { type: "literal", value: "out" },
        ] } },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 1193,
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
    lineNumber: 1199,
  },
],
};
