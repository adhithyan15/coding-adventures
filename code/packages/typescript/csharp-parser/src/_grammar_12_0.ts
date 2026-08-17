// AUTO-GENERATED FILE - DO NOT EDIT
// Source: csharp12.0.grammar
// Regenerate with: grammar-tools compile-grammar csharp12.0.grammar
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
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "top_level_statements" },
          { type: "repetition", element: { type: "rule_reference", name: "namespace_member_declaration" } },
        ] } },
    ] },
    lineNumber: 153,
  },
  {
    name: "top_level_statements",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "statement" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
      { type: "repetition", element: { type: "rule_reference", name: "type_declaration" } },
    ] },
    lineNumber: 159,
  },
  {
    name: "extern_alias_directive",
    body: { type: "sequence", elements: [
      { type: "literal", value: "extern" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 165,
  },
  {
    name: "using_directive",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "optional", element: { type: "literal", value: "global" } },
        { type: "literal", value: "using" },
        { type: "literal", value: "static" },
        { type: "rule_reference", name: "qualified_name" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "optional", element: { type: "literal", value: "global" } },
        { type: "literal", value: "using" },
        { type: "rule_reference", name: "qualified_name" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "optional", element: { type: "literal", value: "global" } },
        { type: "literal", value: "using" },
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "EQUALS" },
        { type: "rule_reference", name: "type" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 186,
  },
  {
    name: "qualified_name",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "name_part" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "DOT" },
          { type: "rule_reference", name: "name_part" },
        ] } },
    ] },
    lineNumber: 194,
  },
  {
    name: "name_part",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON_COLON" },
          { type: "token_reference", name: "NAME" },
        ] } },
      { type: "optional", element: { type: "rule_reference", name: "type_argument_list" } },
    ] },
    lineNumber: 196,
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
    lineNumber: 198,
  },
  {
    name: "type_argument",
    body: { type: "rule_reference", name: "type" },
    lineNumber: 200,
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
    lineNumber: 206,
  },
  {
    name: "global_attribute_target",
    body: { type: "alternation", choices: [
      { type: "literal", value: "assembly" },
      { type: "literal", value: "module" },
    ] },
    lineNumber: 208,
  },
  {
    name: "namespace_declaration",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "namespace" },
        { type: "rule_reference", name: "qualified_name" },
        { type: "token_reference", name: "LBRACE" },
        { type: "repetition", element: { type: "rule_reference", name: "extern_alias_directive" } },
        { type: "repetition", element: { type: "rule_reference", name: "using_directive" } },
        { type: "repetition", element: { type: "rule_reference", name: "namespace_member_declaration" } },
        { type: "token_reference", name: "RBRACE" },
        { type: "optional", element: { type: "token_reference", name: "SEMICOLON" } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "namespace" },
        { type: "rule_reference", name: "qualified_name" },
        { type: "token_reference", name: "SEMICOLON" },
        { type: "repetition", element: { type: "rule_reference", name: "extern_alias_directive" } },
        { type: "repetition", element: { type: "rule_reference", name: "using_directive" } },
        { type: "repetition", element: { type: "rule_reference", name: "namespace_member_declaration" } },
      ] },
    ] },
    lineNumber: 218,
  },
  {
    name: "namespace_member_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "namespace_declaration" },
      { type: "rule_reference", name: "type_declaration" },
    ] },
    lineNumber: 232,
  },
  {
    name: "type_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "class_declaration" },
      { type: "rule_reference", name: "struct_declaration" },
      { type: "rule_reference", name: "interface_declaration" },
      { type: "rule_reference", name: "enum_declaration" },
      { type: "rule_reference", name: "delegate_declaration" },
      { type: "rule_reference", name: "record_declaration" },
      { type: "rule_reference", name: "record_struct_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 235,
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
    lineNumber: 251,
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
    lineNumber: 253,
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
    lineNumber: 261,
  },
  {
    name: "attribute",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "qualified_name" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "optional", element: { type: "rule_reference", name: "attribute_arguments" } },
          { type: "token_reference", name: "RPAREN" },
        ] } },
    ] },
    lineNumber: 263,
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
    lineNumber: 265,
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
    lineNumber: 267,
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
          { type: "token_reference", name: "LPAREN" },
          { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "class_base_list" },
        ] } },
      { type: "repetition", element: { type: "rule_reference", name: "type_parameter_constraint_clause" } },
      { type: "rule_reference", name: "class_body" },
      { type: "optional", element: { type: "token_reference", name: "SEMICOLON" } },
    ] },
    lineNumber: 340,
  },
  {
    name: "class_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "file" },
      { type: "literal", value: "new" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "sealed" },
      { type: "literal", value: "static" },
    ] },
    lineNumber: 347,
  },
  {
    name: "class_base_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "type" },
        ] } },
    ] },
    lineNumber: 357,
  },
  {
    name: "class_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "class_member_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 359,
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
    lineNumber: 365,
  },
  {
    name: "type_parameter",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "alternation", choices: [
          { type: "literal", value: "in" },
          { type: "literal", value: "out" },
        ] } },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 367,
  },
  {
    name: "type_parameter_constraint_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "where" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "type_parameter_constraints" },
    ] },
    lineNumber: 369,
  },
  {
    name: "type_parameter_constraints",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type_parameter_constraint" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "type_parameter_constraint" },
        ] } },
    ] },
    lineNumber: 371,
  },
  {
    name: "type_parameter_constraint",
    body: { type: "alternation", choices: [
      { type: "literal", value: "class" },
      { type: "literal", value: "struct" },
      { type: "literal", value: "unmanaged" },
      { type: "literal", value: "notnull" },
      { type: "sequence", elements: [
        { type: "literal", value: "new" },
        { type: "token_reference", name: "LPAREN" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "rule_reference", name: "type" },
    ] },
    lineNumber: 374,
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
      { type: "rule_reference", name: "checked_operator_declaration" },
      { type: "rule_reference", name: "conversion_operator_declaration" },
      { type: "rule_reference", name: "constructor_declaration" },
      { type: "rule_reference", name: "destructor_declaration" },
      { type: "rule_reference", name: "static_constructor_declaration" },
      { type: "rule_reference", name: "type_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 385,
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
    lineNumber: 404,
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
    lineNumber: 407,
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
    lineNumber: 413,
  },
  {
    name: "constant_declarator",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 415,
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
    lineNumber: 425,
  },
  {
    name: "field_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "file" },
      { type: "literal", value: "new" },
      { type: "literal", value: "static" },
      { type: "literal", value: "readonly" },
      { type: "literal", value: "volatile" },
      { type: "literal", value: "required" },
      { type: "literal", value: "ref" },
    ] },
    lineNumber: 428,
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
    lineNumber: 440,
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
    lineNumber: 442,
  },
  {
    name: "variable_initializer",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "expression" },
      { type: "rule_reference", name: "array_initializer" },
    ] },
    lineNumber: 444,
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
    lineNumber: 447,
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
      { type: "repetition", element: { type: "rule_reference", name: "type_parameter_constraint_clause" } },
      { type: "rule_reference", name: "method_body" },
    ] },
    lineNumber: 453,
  },
  {
    name: "method_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "file" },
      { type: "literal", value: "new" },
      { type: "literal", value: "static" },
      { type: "literal", value: "virtual" },
      { type: "literal", value: "sealed" },
      { type: "literal", value: "override" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "extern" },
      { type: "literal", value: "async" },
    ] },
    lineNumber: 459,
  },
  {
    name: "return_type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "void" },
      { type: "sequence", elements: [
        { type: "optional", element: { type: "sequence", elements: [
            { type: "literal", value: "ref" },
            { type: "optional", element: { type: "literal", value: "readonly" } },
          ] } },
        { type: "rule_reference", name: "type" },
      ] },
    ] },
    lineNumber: 473,
  },
  {
    name: "method_body",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "block" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LAMBDA_ARROW" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 476,
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
    lineNumber: 494,
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
    lineNumber: 497,
  },
  {
    name: "fixed_parameter",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "literal", value: "scoped" } },
      { type: "optional", element: { type: "rule_reference", name: "parameter_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 499,
  },
  {
    name: "parameter_modifier",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "ref" },
        { type: "literal", value: "readonly" },
      ] },
      { type: "literal", value: "ref" },
      { type: "literal", value: "out" },
      { type: "literal", value: "in" },
      { type: "literal", value: "this" },
    ] },
    lineNumber: 503,
  },
  {
    name: "parameter_array",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "literal", value: "params" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 509,
  },
  {
    name: "property_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "property_modifier" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "ref" },
          { type: "optional", element: { type: "literal", value: "readonly" } },
        ] } },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "qualified_name" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "token_reference", name: "LBRACE" },
            { type: "rule_reference", name: "accessor_declarations" },
            { type: "token_reference", name: "RBRACE" },
            { type: "optional", element: { type: "sequence", elements: [
                { type: "token_reference", name: "EQUALS" },
                { type: "rule_reference", name: "expression" },
                { type: "token_reference", name: "SEMICOLON" },
              ] } },
          ] },
          { type: "sequence", elements: [
            { type: "token_reference", name: "LAMBDA_ARROW" },
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "SEMICOLON" },
          ] },
        ] } },
    ] },
    lineNumber: 518,
  },
  {
    name: "property_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "file" },
      { type: "literal", value: "new" },
      { type: "literal", value: "static" },
      { type: "literal", value: "virtual" },
      { type: "literal", value: "sealed" },
      { type: "literal", value: "override" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "extern" },
      { type: "literal", value: "required" },
    ] },
    lineNumber: 523,
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
    lineNumber: 537,
  },
  {
    name: "get_accessor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "rule_reference", name: "accessor_modifier" } },
      { type: "literal", value: "get" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "block" },
          { type: "sequence", elements: [
            { type: "token_reference", name: "LAMBDA_ARROW" },
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "SEMICOLON" },
          ] },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
    ] },
    lineNumber: 540,
  },
  {
    name: "set_accessor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "rule_reference", name: "accessor_modifier" } },
      { type: "group", element: { type: "alternation", choices: [
          { type: "literal", value: "set" },
          { type: "literal", value: "init" },
        ] } },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "block" },
          { type: "sequence", elements: [
            { type: "token_reference", name: "LAMBDA_ARROW" },
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "SEMICOLON" },
          ] },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
    ] },
    lineNumber: 543,
  },
  {
    name: "accessor_modifier",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "protected" },
        { type: "literal", value: "internal" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "internal" },
        { type: "literal", value: "protected" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "private" },
        { type: "literal", value: "protected" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "protected" },
        { type: "literal", value: "private" },
      ] },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
    ] },
    lineNumber: 546,
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
    lineNumber: 558,
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
    lineNumber: 562,
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
    lineNumber: 574,
  },
  {
    name: "add_accessor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "literal", value: "add" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 577,
  },
  {
    name: "remove_accessor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "literal", value: "remove" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 578,
  },
  {
    name: "indexer_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "indexer_modifier" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "ref" },
          { type: "optional", element: { type: "literal", value: "readonly" } },
        ] } },
      { type: "rule_reference", name: "type" },
      { type: "literal", value: "this" },
      { type: "token_reference", name: "LBRACKET" },
      { type: "rule_reference", name: "formal_parameter_list" },
      { type: "token_reference", name: "RBRACKET" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "token_reference", name: "LBRACE" },
            { type: "rule_reference", name: "accessor_declarations" },
            { type: "token_reference", name: "RBRACE" },
          ] },
          { type: "sequence", elements: [
            { type: "token_reference", name: "LAMBDA_ARROW" },
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "SEMICOLON" },
          ] },
        ] } },
    ] },
    lineNumber: 584,
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
    lineNumber: 589,
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
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "block" },
          { type: "sequence", elements: [
            { type: "token_reference", name: "LAMBDA_ARROW" },
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "SEMICOLON" },
          ] },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
    ] },
    lineNumber: 606,
  },
  {
    name: "operator_modifiers",
    body: { type: "sequence", elements: [
      { type: "literal", value: "public" },
      { type: "literal", value: "static" },
      { type: "optional", element: { type: "literal", value: "extern" } },
    ] },
    lineNumber: 611,
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
      { type: "token_reference", name: "UNSIGNED_RIGHT_SHIFT" },
      { type: "token_reference", name: "EQUALS_EQUALS" },
      { type: "token_reference", name: "NOT_EQUALS" },
      { type: "token_reference", name: "LESS_THAN" },
      { type: "token_reference", name: "GREATER_THAN" },
      { type: "token_reference", name: "LESS_EQUALS" },
      { type: "token_reference", name: "GREATER_EQUALS" },
    ] },
    lineNumber: 613,
  },
  {
    name: "checked_operator_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "rule_reference", name: "operator_modifiers" },
      { type: "rule_reference", name: "type" },
      { type: "literal", value: "operator" },
      { type: "literal", value: "checked" },
      { type: "rule_reference", name: "checked_overloadable_operator" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "type" },
          { type: "token_reference", name: "NAME" },
        ] } },
      { type: "token_reference", name: "RPAREN" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "block" },
          { type: "sequence", elements: [
            { type: "token_reference", name: "LAMBDA_ARROW" },
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "SEMICOLON" },
          ] },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
    ] },
    lineNumber: 641,
  },
  {
    name: "checked_overloadable_operator",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "PLUS" },
      { type: "token_reference", name: "MINUS" },
      { type: "token_reference", name: "STAR" },
      { type: "token_reference", name: "SLASH" },
      { type: "token_reference", name: "PLUS_PLUS" },
      { type: "token_reference", name: "MINUS_MINUS" },
    ] },
    lineNumber: 646,
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
      { type: "optional", element: { type: "literal", value: "checked" } },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "RPAREN" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "block" },
          { type: "sequence", elements: [
            { type: "token_reference", name: "LAMBDA_ARROW" },
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "SEMICOLON" },
          ] },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
    ] },
    lineNumber: 657,
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
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "block" },
          { type: "sequence", elements: [
            { type: "token_reference", name: "LAMBDA_ARROW" },
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "SEMICOLON" },
          ] },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
    ] },
    lineNumber: 667,
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
    lineNumber: 672,
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
    lineNumber: 678,
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
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "block" },
          { type: "sequence", elements: [
            { type: "token_reference", name: "LAMBDA_ARROW" },
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "SEMICOLON" },
          ] },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
    ] },
    lineNumber: 685,
  },
  {
    name: "static_constructor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "rule_reference", name: "static_constructor_modifiers" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "token_reference", name: "RPAREN" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "block" },
          { type: "sequence", elements: [
            { type: "token_reference", name: "LAMBDA_ARROW" },
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "SEMICOLON" },
          ] },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
    ] },
    lineNumber: 693,
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
    lineNumber: 697,
  },
  {
    name: "struct_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "struct_modifier" } },
      { type: "optional", element: { type: "literal", value: "partial" } },
      { type: "optional", element: { type: "literal", value: "ref" } },
      { type: "literal", value: "struct" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_list" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "interface_type_list" },
        ] } },
      { type: "repetition", element: { type: "rule_reference", name: "type_parameter_constraint_clause" } },
      { type: "rule_reference", name: "struct_body" },
      { type: "optional", element: { type: "token_reference", name: "SEMICOLON" } },
    ] },
    lineNumber: 726,
  },
  {
    name: "struct_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "file" },
      { type: "literal", value: "new" },
      { type: "literal", value: "readonly" },
    ] },
    lineNumber: 734,
  },
  {
    name: "interface_type_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "type" },
        ] } },
    ] },
    lineNumber: 742,
  },
  {
    name: "struct_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "struct_member_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 744,
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
      { type: "rule_reference", name: "checked_operator_declaration" },
      { type: "rule_reference", name: "conversion_operator_declaration" },
      { type: "rule_reference", name: "constructor_declaration" },
      { type: "rule_reference", name: "static_constructor_declaration" },
      { type: "rule_reference", name: "type_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 746,
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
      { type: "repetition", element: { type: "rule_reference", name: "type_parameter_constraint_clause" } },
      { type: "rule_reference", name: "interface_body" },
      { type: "optional", element: { type: "token_reference", name: "SEMICOLON" } },
    ] },
    lineNumber: 766,
  },
  {
    name: "interface_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "file" },
      { type: "literal", value: "new" },
    ] },
    lineNumber: 773,
  },
  {
    name: "interface_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "interface_member_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 780,
  },
  {
    name: "interface_member_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "interface_method_declaration" },
      { type: "rule_reference", name: "interface_property_declaration" },
      { type: "rule_reference", name: "interface_event_declaration" },
      { type: "rule_reference", name: "interface_indexer_declaration" },
      { type: "rule_reference", name: "interface_constant_declaration" },
      { type: "rule_reference", name: "interface_operator_declaration" },
      { type: "rule_reference", name: "type_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 782,
  },
  {
    name: "interface_method_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "interface_method_modifier" } },
      { type: "rule_reference", name: "return_type" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_list" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "repetition", element: { type: "rule_reference", name: "type_parameter_constraint_clause" } },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "block" },
          { type: "sequence", elements: [
            { type: "token_reference", name: "LAMBDA_ARROW" },
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "SEMICOLON" },
          ] },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
    ] },
    lineNumber: 791,
  },
  {
    name: "interface_method_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "new" },
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "static" },
      { type: "literal", value: "virtual" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "sealed" },
      { type: "literal", value: "override" },
      { type: "literal", value: "extern" },
      { type: "literal", value: "async" },
    ] },
    lineNumber: 797,
  },
  {
    name: "interface_property_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "interface_method_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "accessor_declarations" },
      { type: "token_reference", name: "RBRACE" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "expression" },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
    ] },
    lineNumber: 810,
  },
  {
    name: "interface_event_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "interface_method_modifier" } },
      { type: "literal", value: "event" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "SEMICOLON" },
          { type: "sequence", elements: [
            { type: "token_reference", name: "LBRACE" },
            { type: "rule_reference", name: "event_accessor_declarations" },
            { type: "token_reference", name: "RBRACE" },
          ] },
        ] } },
    ] },
    lineNumber: 815,
  },
  {
    name: "interface_indexer_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "interface_method_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "literal", value: "this" },
      { type: "token_reference", name: "LBRACKET" },
      { type: "rule_reference", name: "formal_parameter_list" },
      { type: "token_reference", name: "RBRACKET" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "accessor_declarations" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 820,
  },
  {
    name: "interface_constant_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "interface_method_modifier" } },
      { type: "literal", value: "const" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 824,
  },
  {
    name: "interface_operator_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "literal", value: "public" },
      { type: "literal", value: "static" },
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
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "block" },
          { type: "sequence", elements: [
            { type: "token_reference", name: "LAMBDA_ARROW" },
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "SEMICOLON" },
          ] },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
    ] },
    lineNumber: 827,
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
    lineNumber: 836,
  },
  {
    name: "enum_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "file" },
      { type: "literal", value: "new" },
    ] },
    lineNumber: 840,
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
    lineNumber: 847,
  },
  {
    name: "enum_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "rule_reference", name: "enum_member_declarations" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 856,
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
    lineNumber: 858,
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
    lineNumber: 861,
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
      { type: "repetition", element: { type: "rule_reference", name: "type_parameter_constraint_clause" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 867,
  },
  {
    name: "delegate_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "file" },
      { type: "literal", value: "new" },
    ] },
    lineNumber: 873,
  },
  {
    name: "record_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "record_modifier" } },
      { type: "literal", value: "record" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_list" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "class_base_list" },
        ] } },
      { type: "repetition", element: { type: "rule_reference", name: "type_parameter_constraint_clause" } },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "class_body" },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
    ] },
    lineNumber: 888,
  },
  {
    name: "record_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "file" },
      { type: "literal", value: "new" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "sealed" },
    ] },
    lineNumber: 895,
  },
  {
    name: "record_struct_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "record_struct_modifier" } },
      { type: "literal", value: "record" },
      { type: "literal", value: "struct" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_list" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "interface_type_list" },
        ] } },
      { type: "repetition", element: { type: "rule_reference", name: "type_parameter_constraint_clause" } },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "struct_body" },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
    ] },
    lineNumber: 911,
  },
  {
    name: "record_struct_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "file" },
      { type: "literal", value: "new" },
      { type: "literal", value: "readonly" },
    ] },
    lineNumber: 919,
  },
  {
    name: "type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "nullable_type" },
      { type: "rule_reference", name: "non_nullable_type" },
    ] },
    lineNumber: 934,
  },
  {
    name: "non_nullable_type",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "tuple_type" },
        { type: "repetition", element: { type: "rule_reference", name: "rank_specifier" } },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "value_type" },
        { type: "repetition", element: { type: "rule_reference", name: "rank_specifier" } },
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
    lineNumber: 937,
  },
  {
    name: "nullable_type",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "non_nullable_type" },
      { type: "token_reference", name: "QUESTION" },
    ] },
    lineNumber: 942,
  },
  {
    name: "value_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "primitive_type" },
      { type: "rule_reference", name: "qualified_name" },
    ] },
    lineNumber: 944,
  },
  {
    name: "reference_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "qualified_name" },
      { type: "literal", value: "object" },
      { type: "literal", value: "string" },
      { type: "literal", value: "dynamic" },
    ] },
    lineNumber: 947,
  },
  {
    name: "primitive_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "numeric_type" },
      { type: "literal", value: "bool" },
    ] },
    lineNumber: 952,
  },
  {
    name: "numeric_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "integral_type" },
      { type: "rule_reference", name: "floating_point_type" },
      { type: "literal", value: "decimal" },
    ] },
    lineNumber: 955,
  },
  {
    name: "floating_point_type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "float" },
      { type: "literal", value: "double" },
    ] },
    lineNumber: 959,
  },
  {
    name: "rank_specifier",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "repetition", element: { type: "token_reference", name: "COMMA" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 962,
  },
  {
    name: "pointer_type",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "STAR" },
    ] },
    lineNumber: 964,
  },
  {
    name: "tuple_type",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "tuple_element" },
      { type: "token_reference", name: "COMMA" },
      { type: "rule_reference", name: "tuple_element" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "tuple_element" },
        ] } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 966,
  },
  {
    name: "tuple_element",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
    ] },
    lineNumber: 968,
  },
  {
    name: "block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 976,
  },
  {
    name: "statement",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "block" },
      { type: "rule_reference", name: "local_variable_declaration_statement" },
      { type: "rule_reference", name: "local_constant_declaration_statement" },
      { type: "rule_reference", name: "using_declaration_statement" },
      { type: "rule_reference", name: "empty_statement" },
      { type: "rule_reference", name: "expression_statement" },
      { type: "rule_reference", name: "if_statement" },
      { type: "rule_reference", name: "while_statement" },
      { type: "rule_reference", name: "do_while_statement" },
      { type: "rule_reference", name: "for_statement" },
      { type: "rule_reference", name: "foreach_statement" },
      { type: "rule_reference", name: "await_foreach_statement" },
      { type: "rule_reference", name: "switch_statement" },
      { type: "rule_reference", name: "try_statement" },
      { type: "rule_reference", name: "throw_statement" },
      { type: "rule_reference", name: "return_statement" },
      { type: "rule_reference", name: "break_statement" },
      { type: "rule_reference", name: "continue_statement" },
      { type: "rule_reference", name: "goto_statement" },
      { type: "rule_reference", name: "lock_statement" },
      { type: "rule_reference", name: "using_statement" },
      { type: "rule_reference", name: "await_using_statement" },
      { type: "rule_reference", name: "checked_statement" },
      { type: "rule_reference", name: "unchecked_statement" },
      { type: "rule_reference", name: "labelled_statement" },
      { type: "rule_reference", name: "unsafe_statement" },
      { type: "rule_reference", name: "fixed_statement" },
      { type: "rule_reference", name: "yield_statement" },
      { type: "rule_reference", name: "local_function_declaration" },
    ] },
    lineNumber: 978,
  },
  {
    name: "local_variable_declaration_statement",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "local_variable_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1012,
  },
  {
    name: "local_variable_declaration",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "optional", element: { type: "literal", value: "scoped" } },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "literal", value: "ref" },
            { type: "optional", element: { type: "literal", value: "readonly" } },
          ] } },
        { type: "rule_reference", name: "type" },
        { type: "rule_reference", name: "variable_declarators" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "var" },
        { type: "rule_reference", name: "variable_declarators" },
      ] },
      { type: "rule_reference", name: "deconstruction_declaration" },
    ] },
    lineNumber: 1014,
  },
  {
    name: "deconstruction_declaration",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "var" },
        { type: "rule_reference", name: "deconstruction_tuple" },
        { type: "token_reference", name: "EQUALS" },
        { type: "rule_reference", name: "expression" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "deconstruction_element" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "deconstruction_element" },
          ] } },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "EQUALS" },
        { type: "rule_reference", name: "expression" },
      ] },
    ] },
    lineNumber: 1018,
  },
  {
    name: "deconstruction_tuple",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "token_reference", name: "NAME" },
        ] } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 1022,
  },
  {
    name: "deconstruction_element",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 1024,
  },
  {
    name: "local_constant_declaration_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "const" },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "constant_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1026,
  },
  {
    name: "using_declaration_statement",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "literal", value: "await" } },
      { type: "literal", value: "using" },
      { type: "optional", element: { type: "literal", value: "ref" } },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "variable_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1028,
  },
  {
    name: "empty_statement",
    body: { type: "token_reference", name: "SEMICOLON" },
    lineNumber: 1030,
  },
  {
    name: "expression_statement",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1031,
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
    lineNumber: 1033,
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
    lineNumber: 1034,
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
    lineNumber: 1035,
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
    lineNumber: 1037,
  },
  {
    name: "for_initializer",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "local_variable_declaration" },
      { type: "rule_reference", name: "expression_list" },
    ] },
    lineNumber: 1040,
  },
  {
    name: "for_iterator",
    body: { type: "rule_reference", name: "expression_list" },
    lineNumber: 1041,
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
    lineNumber: 1042,
  },
  {
    name: "foreach_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "foreach" },
      { type: "token_reference", name: "LPAREN" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "rule_reference", name: "type" },
            { type: "token_reference", name: "NAME" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "var" },
            { type: "token_reference", name: "NAME" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "var" },
            { type: "rule_reference", name: "deconstruction_tuple" },
          ] },
        ] } },
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 1044,
  },
  {
    name: "await_foreach_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "await" },
      { type: "literal", value: "foreach" },
      { type: "token_reference", name: "LPAREN" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "rule_reference", name: "type" },
            { type: "token_reference", name: "NAME" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "var" },
            { type: "token_reference", name: "NAME" },
          ] },
        ] } },
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 1047,
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
    lineNumber: 1050,
  },
  {
    name: "switch_block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "switch_section" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 1051,
  },
  {
    name: "switch_section",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "switch_label" } },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
    ] },
    lineNumber: 1052,
  },
  {
    name: "switch_label",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "case" },
        { type: "rule_reference", name: "pattern" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "literal", value: "when" },
            { type: "rule_reference", name: "expression" },
          ] } },
        { type: "token_reference", name: "COLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "default" },
        { type: "token_reference", name: "COLON" },
      ] },
    ] },
    lineNumber: 1053,
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
    lineNumber: 1056,
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
    lineNumber: 1058,
  },
  {
    name: "specific_catch_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "catch" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "when" },
          { type: "token_reference", name: "LPAREN" },
          { type: "rule_reference", name: "expression" },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1061,
  },
  {
    name: "general_catch_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "catch" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1064,
  },
  {
    name: "finally_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "finally" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1065,
  },
  {
    name: "throw_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "throw" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1067,
  },
  {
    name: "return_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "return" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "optional", element: { type: "literal", value: "ref" } },
          { type: "rule_reference", name: "expression" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1068,
  },
  {
    name: "break_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "break" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1069,
  },
  {
    name: "continue_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "continue" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1070,
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
    lineNumber: 1072,
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
    lineNumber: 1076,
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
    lineNumber: 1078,
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
    lineNumber: 1079,
  },
  {
    name: "await_using_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "await" },
      { type: "literal", value: "using" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "resource_acquisition" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 1081,
  },
  {
    name: "checked_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "checked" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1083,
  },
  {
    name: "unchecked_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "unchecked" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1084,
  },
  {
    name: "labelled_statement",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 1085,
  },
  {
    name: "unsafe_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "unsafe" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1086,
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
    lineNumber: 1087,
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
    lineNumber: 1089,
  },
  {
    name: "local_function_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "local_function_modifier" } },
      { type: "rule_reference", name: "return_type" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameter_list" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "repetition", element: { type: "rule_reference", name: "type_parameter_constraint_clause" } },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "block" },
          { type: "sequence", elements: [
            { type: "token_reference", name: "LAMBDA_ARROW" },
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "SEMICOLON" },
          ] },
        ] } },
    ] },
    lineNumber: 1092,
  },
  {
    name: "local_function_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "static" },
      { type: "literal", value: "async" },
      { type: "literal", value: "unsafe" },
    ] },
    lineNumber: 1098,
  },
  {
    name: "pattern",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "list_pattern" },
      { type: "rule_reference", name: "relational_pattern" },
      { type: "rule_reference", name: "logical_not_pattern" },
      { type: "rule_reference", name: "logical_and_pattern" },
      { type: "rule_reference", name: "logical_or_pattern" },
      { type: "rule_reference", name: "discard_pattern" },
      { type: "rule_reference", name: "constant_pattern" },
      { type: "rule_reference", name: "var_pattern" },
      { type: "rule_reference", name: "declaration_pattern" },
      { type: "rule_reference", name: "property_pattern" },
      { type: "rule_reference", name: "tuple_pattern" },
      { type: "rule_reference", name: "positional_pattern" },
    ] },
    lineNumber: 1115,
  },
  {
    name: "constant_pattern",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "literal" },
      { type: "rule_reference", name: "qualified_name" },
    ] },
    lineNumber: 1128,
  },
  {
    name: "relational_pattern",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "GREATER_THAN" },
          { type: "token_reference", name: "LESS_THAN" },
          { type: "token_reference", name: "GREATER_EQUALS" },
          { type: "token_reference", name: "LESS_EQUALS" },
        ] } },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 1131,
  },
  {
    name: "logical_not_pattern",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "rule_reference", name: "pattern" },
    ] },
    lineNumber: 1135,
  },
  {
    name: "logical_and_pattern",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "pattern" },
      { type: "token_reference", name: "NAME" },
      { type: "rule_reference", name: "pattern" },
    ] },
    lineNumber: 1136,
  },
  {
    name: "logical_or_pattern",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "pattern" },
      { type: "token_reference", name: "NAME" },
      { type: "rule_reference", name: "pattern" },
    ] },
    lineNumber: 1137,
  },
  {
    name: "declaration_pattern",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 1139,
  },
  {
    name: "var_pattern",
    body: { type: "sequence", elements: [
      { type: "literal", value: "var" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 1140,
  },
  {
    name: "discard_pattern",
    body: { type: "token_reference", name: "NAME" },
    lineNumber: 1141,
  },
  {
    name: "property_pattern",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "rule_reference", name: "type" } },
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "rule_reference", name: "property_subpattern_list" } },
      { type: "token_reference", name: "RBRACE" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
    ] },
    lineNumber: 1144,
  },
  {
    name: "property_subpattern_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "property_subpattern" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "property_subpattern" },
        ] } },
    ] },
    lineNumber: 1145,
  },
  {
    name: "property_subpattern",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "name_chain" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "pattern" },
    ] },
    lineNumber: 1146,
  },
  {
    name: "name_chain",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "DOT" },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 1147,
  },
  {
    name: "tuple_pattern",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "subpattern" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "subpattern" },
        ] } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 1150,
  },
  {
    name: "positional_pattern",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "subpattern" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "subpattern" },
            ] } },
        ] } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "property_pattern" } },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
    ] },
    lineNumber: 1153,
  },
  {
    name: "subpattern",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "COLON" },
        ] } },
      { type: "rule_reference", name: "pattern" },
    ] },
    lineNumber: 1156,
  },
  {
    name: "list_pattern",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "list_pattern_element" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "list_pattern_element" },
            ] } },
        ] } },
      { type: "token_reference", name: "RBRACKET" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
    ] },
    lineNumber: 1159,
  },
  {
    name: "list_pattern_element",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "slice_pattern" },
      { type: "rule_reference", name: "pattern" },
    ] },
    lineNumber: 1162,
  },
  {
    name: "slice_pattern",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "DOT_DOT" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "var" },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 1163,
  },
  {
    name: "expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "lambda_expression" },
      { type: "rule_reference", name: "assignment_expression" },
    ] },
    lineNumber: 1209,
  },
  {
    name: "lambda_expression",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "literal", value: "async" } },
      { type: "rule_reference", name: "lambda_parameters" },
      { type: "token_reference", name: "LAMBDA_ARROW" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "expression" },
          { type: "rule_reference", name: "block" },
        ] } },
    ] },
    lineNumber: 1227,
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
    lineNumber: 1230,
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
    lineNumber: 1233,
  },
  {
    name: "lambda_parameter",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "literal", value: "scoped" } },
      { type: "optional", element: { type: "alternation", choices: [
          { type: "literal", value: "ref" },
          { type: "literal", value: "out" },
          { type: "literal", value: "in" },
        ] } },
      { type: "optional", element: { type: "rule_reference", name: "type" } },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 1236,
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
      { type: "rule_reference", name: "throw_expression" },
    ] },
    lineNumber: 1240,
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
      { type: "token_reference", name: "UNSIGNED_RIGHT_SHIFT_EQUALS" },
      { type: "token_reference", name: "QUESTION_QUESTION_EQUALS" },
    ] },
    lineNumber: 1244,
  },
  {
    name: "throw_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "throw" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 1258,
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
    lineNumber: 1262,
  },
  {
    name: "null_coalescing_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "logical_or_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "NULL_COALESCING" },
          { type: "rule_reference", name: "logical_or_expression" },
        ] } },
    ] },
    lineNumber: 1265,
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
    lineNumber: 1268,
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
    lineNumber: 1269,
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
    lineNumber: 1270,
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
    lineNumber: 1271,
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
    lineNumber: 1272,
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
    lineNumber: 1274,
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
            { type: "rule_reference", name: "pattern" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "as" },
            { type: "rule_reference", name: "type" },
          ] },
        ] } },
    ] },
    lineNumber: 1277,
  },
  {
    name: "shift_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "additive_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "LEFT_SHIFT" },
              { type: "token_reference", name: "RIGHT_SHIFT" },
              { type: "token_reference", name: "UNSIGNED_RIGHT_SHIFT" },
            ] } },
          { type: "rule_reference", name: "additive_expression" },
        ] } },
    ] },
    lineNumber: 1283,
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
    lineNumber: 1286,
  },
  {
    name: "multiplicative_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "range_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "STAR" },
              { type: "token_reference", name: "SLASH" },
              { type: "token_reference", name: "PERCENT" },
            ] } },
          { type: "rule_reference", name: "range_expression" },
        ] } },
    ] },
    lineNumber: 1289,
  },
  {
    name: "range_expression",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "unary_expression" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "DOT_DOT" },
            { type: "optional", element: { type: "rule_reference", name: "unary_expression" } },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "DOT_DOT" },
        { type: "optional", element: { type: "rule_reference", name: "unary_expression" } },
      ] },
    ] },
    lineNumber: 1292,
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
      { type: "sequence", elements: [
        { type: "token_reference", name: "CARET" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "await" },
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
    lineNumber: 1295,
  },
  {
    name: "cast_expression",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "unary_expression" },
    ] },
    lineNumber: 1308,
  },
  {
    name: "postfix_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "primary_expression" },
      { type: "repetition", element: { type: "rule_reference", name: "postfix_operator" } },
    ] },
    lineNumber: 1310,
  },
  {
    name: "postfix_operator",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "PLUS_PLUS" },
      { type: "token_reference", name: "MINUS_MINUS" },
      { type: "token_reference", name: "BANG" },
    ] },
    lineNumber: 1312,
  },
  {
    name: "primary_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "primary" },
      { type: "repetition", element: { type: "rule_reference", name: "primary_suffix" } },
    ] },
    lineNumber: 1325,
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
        { type: "token_reference", name: "NULL_CONDITIONAL_DOT" },
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
        { type: "rule_reference", name: "argument_list" },
        { type: "token_reference", name: "RBRACKET" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "NULL_CONDITIONAL_BRACKET" },
        { type: "rule_reference", name: "argument_list" },
        { type: "token_reference", name: "RBRACKET" },
      ] },
      { type: "token_reference", name: "BANG" },
      { type: "sequence", elements: [
        { type: "literal", value: "with" },
        { type: "token_reference", name: "LBRACE" },
        { type: "optional", element: { type: "rule_reference", name: "with_initializer_list" } },
        { type: "token_reference", name: "RBRACE" },
      ] },
    ] },
    lineNumber: 1327,
  },
  {
    name: "primary",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "literal" },
      { type: "rule_reference", name: "raw_string_literal" },
      { type: "rule_reference", name: "interpolated_string" },
      { type: "rule_reference", name: "collection_expression" },
      { type: "literal", value: "this" },
      { type: "sequence", elements: [
        { type: "literal", value: "base" },
        { type: "token_reference", name: "DOT" },
        { type: "token_reference", name: "NAME" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "base" },
        { type: "token_reference", name: "LBRACKET" },
        { type: "rule_reference", name: "argument_list" },
        { type: "token_reference", name: "RBRACKET" },
      ] },
      { type: "rule_reference", name: "typeof_expression" },
      { type: "rule_reference", name: "sizeof_expression" },
      { type: "rule_reference", name: "checked_expression" },
      { type: "rule_reference", name: "unchecked_expression" },
      { type: "rule_reference", name: "default_value_expression" },
      { type: "rule_reference", name: "nameof_expression" },
      { type: "rule_reference", name: "new_expression" },
      { type: "rule_reference", name: "stackalloc_expression" },
      { type: "rule_reference", name: "switch_expression" },
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
    lineNumber: 1335,
  },
  {
    name: "collection_expression",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "collection_element" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "collection_element" },
            ] } },
          { type: "optional", element: { type: "token_reference", name: "COMMA" } },
        ] } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 1393,
  },
  {
    name: "collection_element",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "spread_element" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 1395,
  },
  {
    name: "spread_element",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "DOT_DOT" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 1399,
  },
  {
    name: "raw_string_literal",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "RAW_STRING" },
      { type: "token_reference", name: "RAW_INTERPOLATED_STRING" },
    ] },
    lineNumber: 1403,
  },
  {
    name: "interpolated_string",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "INTERPOLATED_STRING" },
      { type: "token_reference", name: "INTERPOLATED_VERBATIM" },
    ] },
    lineNumber: 1408,
  },
  {
    name: "with_initializer_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "with_initializer" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "with_initializer" },
        ] } },
    ] },
    lineNumber: 1413,
  },
  {
    name: "with_initializer",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 1414,
  },
  {
    name: "typeof_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "typeof" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type_or_void" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 1418,
  },
  {
    name: "type_or_void",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "type" },
      { type: "literal", value: "void" },
    ] },
    lineNumber: 1419,
  },
  {
    name: "sizeof_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "sizeof" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 1420,
  },
  {
    name: "checked_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "checked" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 1421,
  },
  {
    name: "unchecked_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "unchecked" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 1422,
  },
  {
    name: "default_value_expression",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "default" },
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "type" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "literal", value: "default" },
    ] },
    lineNumber: 1424,
  },
  {
    name: "nameof_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "nameof" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "nameof_member_access" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 1427,
  },
  {
    name: "nameof_member_access",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "DOT" },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 1428,
  },
  {
    name: "new_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "new" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "anonymous_object_creation" },
          { type: "rule_reference", name: "new_array_expression" },
          { type: "rule_reference", name: "new_object_expression" },
          { type: "rule_reference", name: "target_typed_new" },
        ] } },
    ] },
    lineNumber: 1432,
  },
  {
    name: "new_object_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "object_or_collection_initializer" } },
    ] },
    lineNumber: 1437,
  },
  {
    name: "target_typed_new",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "object_or_collection_initializer" } },
    ] },
    lineNumber: 1440,
  },
  {
    name: "object_or_collection_initializer",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "object_initializer" },
      { type: "rule_reference", name: "collection_initializer" },
    ] },
    lineNumber: 1442,
  },
  {
    name: "object_initializer",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "member_initializer" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "member_initializer" },
            ] } },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 1444,
  },
  {
    name: "member_initializer",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "EQUALS" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "expression" },
          { type: "rule_reference", name: "object_initializer" },
        ] } },
    ] },
    lineNumber: 1445,
  },
  {
    name: "collection_initializer",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "element_initializer" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "element_initializer" },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 1447,
  },
  {
    name: "element_initializer",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "expression" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "expression_list" },
        { type: "token_reference", name: "RBRACE" },
      ] },
    ] },
    lineNumber: 1448,
  },
  {
    name: "anonymous_object_creation",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "anonymous_member" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "anonymous_member" },
            ] } },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 1450,
  },
  {
    name: "anonymous_member",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "EQUALS" },
        ] } },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 1451,
  },
  {
    name: "new_array_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "array_type" },
      { type: "rule_reference", name: "array_creation_suffix" },
    ] },
    lineNumber: 1453,
  },
  {
    name: "array_type",
    body: { type: "group", element: { type: "alternation", choices: [
        { type: "rule_reference", name: "primitive_type" },
        { type: "rule_reference", name: "qualified_name" },
      ] } },
    lineNumber: 1454,
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
    lineNumber: 1455,
  },
  {
    name: "stackalloc_expression",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "stackalloc" },
        { type: "rule_reference", name: "type" },
        { type: "token_reference", name: "LBRACKET" },
        { type: "optional", element: { type: "rule_reference", name: "expression" } },
        { type: "token_reference", name: "RBRACKET" },
        { type: "optional", element: { type: "rule_reference", name: "array_initializer" } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "stackalloc" },
        { type: "token_reference", name: "LBRACKET" },
        { type: "token_reference", name: "RBRACKET" },
        { type: "rule_reference", name: "array_initializer" },
      ] },
    ] },
    lineNumber: 1459,
  },
  {
    name: "switch_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "primary_expression" },
      { type: "literal", value: "switch" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "switch_expression_arm" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "switch_expression_arm" },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 1465,
  },
  {
    name: "switch_expression_arm",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "pattern" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "when" },
          { type: "rule_reference", name: "expression" },
        ] } },
      { type: "token_reference", name: "LAMBDA_ARROW" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 1469,
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
    lineNumber: 1473,
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
          { type: "literal", value: "in" },
          { type: "literal", value: "scoped" },
        ] } },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 1475,
  },
  {
    name: "query_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "from_clause" },
      { type: "rule_reference", name: "query_body" },
    ] },
    lineNumber: 1483,
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
    lineNumber: 1484,
  },
  {
    name: "query_body",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "query_body_clause" } },
      { type: "rule_reference", name: "select_or_group_clause" },
      { type: "optional", element: { type: "rule_reference", name: "query_continuation" } },
    ] },
    lineNumber: 1485,
  },
  {
    name: "query_body_clause",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "from_clause" },
      { type: "rule_reference", name: "let_clause" },
      { type: "rule_reference", name: "where_clause" },
      { type: "rule_reference", name: "join_clause" },
      { type: "rule_reference", name: "join_into_clause" },
      { type: "rule_reference", name: "orderby_clause" },
    ] },
    lineNumber: 1487,
  },
  {
    name: "let_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "let" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 1494,
  },
  {
    name: "where_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "where" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 1495,
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
    ] },
    lineNumber: 1496,
  },
  {
    name: "join_into_clause",
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
      { type: "literal", value: "into" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 1497,
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
    lineNumber: 1499,
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
    lineNumber: 1500,
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
    lineNumber: 1502,
  },
  {
    name: "query_continuation",
    body: { type: "sequence", elements: [
      { type: "literal", value: "into" },
      { type: "token_reference", name: "NAME" },
      { type: "rule_reference", name: "query_body" },
    ] },
    lineNumber: 1505,
  },
  {
    name: "literal",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "CHAR" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "VERBATIM_STRING" },
      { type: "token_reference", name: "RAW_STRING" },
      { type: "literal", value: "true" },
      { type: "literal", value: "false" },
      { type: "literal", value: "null" },
    ] },
    lineNumber: 1511,
  },
],
};
