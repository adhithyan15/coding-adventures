// AUTO-GENERATED FILE - DO NOT EDIT
// Source: java14.grammar
// Regenerate with: grammar-tools compile-grammar java14.grammar
//
// This file embeds a ParserGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .grammar file at
// runtime.

import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const PARSER_GRAMMAR: ParserGrammar = {
  version: 1,
  rules: [
  {
    name: "program",
    body: { type: "repetition", element: { type: "rule_reference", name: "program_item" } },
    lineNumber: 108,
  },
  {
    name: "program_item",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "package_declaration" },
      { type: "rule_reference", name: "import_declaration" },
      { type: "rule_reference", name: "type_declaration" },
      { type: "rule_reference", name: "method_declaration" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 109,
  },
  {
    name: "compilation_unit",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
      { type: "optional", element: { type: "rule_reference", name: "package_declaration" } },
      { type: "repetition", element: { type: "rule_reference", name: "import_declaration" } },
      { type: "repetition", element: { type: "rule_reference", name: "type_declaration" } },
    ] },
    lineNumber: 110,
  },
  {
    name: "package_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "package" },
      { type: "rule_reference", name: "qualified_name" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 126,
  },
  {
    name: "import_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "import" },
      { type: "optional", element: { type: "literal", value: "static" } },
      { type: "rule_reference", name: "qualified_name" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "DOT" },
          { type: "token_reference", name: "STAR" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 141,
  },
  {
    name: "type_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "class_declaration" },
      { type: "rule_reference", name: "interface_declaration" },
      { type: "rule_reference", name: "enum_declaration" },
      { type: "rule_reference", name: "annotation_type_declaration" },
      { type: "rule_reference", name: "record_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 164,
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
    lineNumber: 183,
  },
  {
    name: "annotation",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "AT" },
      { type: "rule_reference", name: "qualified_name" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "optional", element: { type: "alternation", choices: [
              { type: "rule_reference", name: "element_value_pairs" },
              { type: "rule_reference", name: "element_value" },
            ] } },
          { type: "token_reference", name: "RPAREN" },
        ] } },
    ] },
    lineNumber: 206,
  },
  {
    name: "annotations",
    body: { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
    lineNumber: 208,
  },
  {
    name: "element_value_pairs",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "element_value_pair" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "element_value_pair" },
        ] } },
    ] },
    lineNumber: 210,
  },
  {
    name: "element_value_pair",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "element_value" },
    ] },
    lineNumber: 212,
  },
  {
    name: "element_value",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "annotation" },
      { type: "rule_reference", name: "element_value_array" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 220,
  },
  {
    name: "element_value_array",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "element_value" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "element_value" },
            ] } },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 224,
  },
  {
    name: "annotation_type_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "class_modifier" } },
      { type: "token_reference", name: "AT" },
      { type: "literal", value: "interface" },
      { type: "token_reference", name: "NAME" },
      { type: "rule_reference", name: "annotation_type_body" },
    ] },
    lineNumber: 240,
  },
  {
    name: "annotation_type_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "annotation_type_element_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 243,
  },
  {
    name: "annotation_type_element_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "annotation_element_declaration" },
      { type: "rule_reference", name: "field_declaration" },
      { type: "rule_reference", name: "class_declaration" },
      { type: "rule_reference", name: "interface_declaration" },
      { type: "rule_reference", name: "enum_declaration" },
      { type: "rule_reference", name: "annotation_type_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 245,
  },
  {
    name: "annotation_element_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "method_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "default" },
          { type: "rule_reference", name: "element_value" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 253,
  },
  {
    name: "class_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "class_modifier" } },
      { type: "literal", value: "class" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "extends" },
          { type: "rule_reference", name: "class_type" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "implements" },
          { type: "rule_reference", name: "interface_type_list" },
        ] } },
      { type: "rule_reference", name: "class_body" },
    ] },
    lineNumber: 274,
  },
  {
    name: "class_modifier",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "annotation" },
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "private" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "final" },
      { type: "literal", value: "static" },
      { type: "literal", value: "strictfp" },
    ] },
    lineNumber: 279,
  },
  {
    name: "class_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "class_body_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 288,
  },
  {
    name: "class_body_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "static_initializer" },
      { type: "rule_reference", name: "instance_initializer" },
      { type: "rule_reference", name: "constructor_declaration" },
      { type: "rule_reference", name: "method_declaration" },
      { type: "rule_reference", name: "field_declaration" },
      { type: "rule_reference", name: "class_declaration" },
      { type: "rule_reference", name: "interface_declaration" },
      { type: "rule_reference", name: "enum_declaration" },
      { type: "rule_reference", name: "annotation_type_declaration" },
      { type: "rule_reference", name: "record_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 294,
  },
  {
    name: "interface_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "interface_modifier" } },
      { type: "literal", value: "interface" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "extends" },
          { type: "rule_reference", name: "interface_type_list" },
        ] } },
      { type: "rule_reference", name: "interface_body" },
    ] },
    lineNumber: 340,
  },
  {
    name: "interface_modifier",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "annotation" },
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "private" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "static" },
      { type: "literal", value: "strictfp" },
    ] },
    lineNumber: 344,
  },
  {
    name: "interface_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "interface_body_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 352,
  },
  {
    name: "interface_body_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "interface_method_declaration" },
      { type: "rule_reference", name: "interface_field_declaration" },
      { type: "rule_reference", name: "class_declaration" },
      { type: "rule_reference", name: "interface_declaration" },
      { type: "rule_reference", name: "enum_declaration" },
      { type: "rule_reference", name: "annotation_type_declaration" },
      { type: "rule_reference", name: "record_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 357,
  },
  {
    name: "interface_field_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "field_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "variable_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 366,
  },
  {
    name: "interface_method_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "interface_method_modifier" } },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "rule_reference", name: "result_type" },
      { type: "rule_reference", name: "method_declarator" },
      { type: "optional", element: { type: "rule_reference", name: "throws_clause" } },
      { type: "rule_reference", name: "method_body" },
    ] },
    lineNumber: 381,
  },
  {
    name: "interface_method_modifier",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "annotation" },
      { type: "literal", value: "public" },
      { type: "literal", value: "private" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "default" },
      { type: "literal", value: "static" },
      { type: "literal", value: "strictfp" },
    ] },
    lineNumber: 384,
  },
  {
    name: "interface_type_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "class_type" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "class_type" },
        ] } },
    ] },
    lineNumber: 392,
  },
  {
    name: "enum_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "class_modifier" } },
      { type: "literal", value: "enum" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "implements" },
          { type: "rule_reference", name: "interface_type_list" },
        ] } },
      { type: "rule_reference", name: "enum_body" },
    ] },
    lineNumber: 412,
  },
  {
    name: "enum_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "rule_reference", name: "enum_constant_list" } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "SEMICOLON" },
          { type: "repetition", element: { type: "rule_reference", name: "class_body_declaration" } },
        ] } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 416,
  },
  {
    name: "enum_constant_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "enum_constant" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "enum_constant" },
        ] } },
    ] },
    lineNumber: 418,
  },
  {
    name: "enum_constant",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "annotations" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "optional", element: { type: "rule_reference", name: "class_body" } },
    ] },
    lineNumber: 420,
  },
  {
    name: "record_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "class_modifier" } },
      { type: "literal", value: "record" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "rule_reference", name: "record_header" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "implements" },
          { type: "rule_reference", name: "interface_type_list" },
        ] } },
      { type: "rule_reference", name: "record_body" },
    ] },
    lineNumber: 498,
  },
  {
    name: "record_header",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "record_component_list" } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 516,
  },
  {
    name: "record_component_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "record_component" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "record_component" },
        ] } },
    ] },
    lineNumber: 518,
  },
  {
    name: "record_component",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 520,
  },
  {
    name: "record_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "record_body_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 543,
  },
  {
    name: "record_body_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "compact_constructor" },
      { type: "rule_reference", name: "class_body_declaration" },
    ] },
    lineNumber: 545,
  },
  {
    name: "compact_constructor",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "constructor_modifier" } },
      { type: "token_reference", name: "NAME" },
      { type: "rule_reference", name: "constructor_body" },
    ] },
    lineNumber: 561,
  },
  {
    name: "type_parameters",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LESS_THAN" },
      { type: "rule_reference", name: "type_parameter" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "type_parameter" },
        ] } },
      { type: "token_reference", name: "GREATER_THAN" },
    ] },
    lineNumber: 587,
  },
  {
    name: "type_parameter",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "extends" },
          { type: "rule_reference", name: "bound" },
        ] } },
    ] },
    lineNumber: 589,
  },
  {
    name: "bound",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "class_type" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "AMPERSAND" },
          { type: "rule_reference", name: "class_type" },
        ] } },
    ] },
    lineNumber: 591,
  },
  {
    name: "type_arguments",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LESS_THAN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "type_argument" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "type_argument" },
            ] } },
        ] } },
      { type: "token_reference", name: "GREATER_THAN" },
    ] },
    lineNumber: 602,
  },
  {
    name: "type_argument",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
        { type: "rule_reference", name: "type" },
      ] },
      { type: "sequence", elements: [
        { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
        { type: "token_reference", name: "QUESTION" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "group", element: { type: "alternation", choices: [
                { type: "literal", value: "extends" },
                { type: "literal", value: "super" },
              ] } },
            { type: "rule_reference", name: "type" },
          ] } },
      ] },
    ] },
    lineNumber: 604,
  },
  {
    name: "field_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "field_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "variable_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 622,
  },
  {
    name: "field_modifier",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "annotation" },
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "private" },
      { type: "literal", value: "static" },
      { type: "literal", value: "final" },
      { type: "literal", value: "transient" },
      { type: "literal", value: "volatile" },
    ] },
    lineNumber: 624,
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
    lineNumber: 633,
  },
  {
    name: "variable_declarator",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LBRACKET" },
          { type: "token_reference", name: "RBRACKET" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "variable_initializer" },
        ] } },
    ] },
    lineNumber: 635,
  },
  {
    name: "variable_initializer",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "expression" },
      { type: "rule_reference", name: "array_initializer" },
    ] },
    lineNumber: 637,
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
    lineNumber: 640,
  },
  {
    name: "method_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "method_modifier" } },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "rule_reference", name: "result_type" },
      { type: "rule_reference", name: "method_declarator" },
      { type: "optional", element: { type: "rule_reference", name: "throws_clause" } },
      { type: "rule_reference", name: "method_body" },
    ] },
    lineNumber: 659,
  },
  {
    name: "method_modifier",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "annotation" },
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "private" },
      { type: "literal", value: "static" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "final" },
      { type: "literal", value: "synchronized" },
      { type: "literal", value: "native" },
      { type: "literal", value: "strictfp" },
    ] },
    lineNumber: 662,
  },
  {
    name: "result_type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "void" },
      { type: "rule_reference", name: "type" },
    ] },
    lineNumber: 673,
  },
  {
    name: "method_declarator",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LBRACKET" },
          { type: "token_reference", name: "RBRACKET" },
        ] } },
    ] },
    lineNumber: 676,
  },
  {
    name: "formal_parameter_list",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "formal_parameter" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "formal_parameter" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "formal_parameter" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "formal_parameter" },
          ] } },
        { type: "token_reference", name: "COMMA" },
        { type: "rule_reference", name: "varargs_parameter" },
      ] },
      { type: "rule_reference", name: "varargs_parameter" },
    ] },
    lineNumber: 693,
  },
  {
    name: "formal_parameter",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
      { type: "optional", element: { type: "literal", value: "final" } },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LBRACKET" },
          { type: "token_reference", name: "RBRACKET" },
        ] } },
    ] },
    lineNumber: 697,
  },
  {
    name: "varargs_parameter",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
      { type: "optional", element: { type: "literal", value: "final" } },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "ELLIPSIS" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 699,
  },
  {
    name: "throws_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "throws" },
      { type: "rule_reference", name: "class_type" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "class_type" },
        ] } },
    ] },
    lineNumber: 701,
  },
  {
    name: "method_body",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "block" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 703,
  },
  {
    name: "constructor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "constructor_modifier" } },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "throws_clause" } },
      { type: "rule_reference", name: "constructor_body" },
    ] },
    lineNumber: 720,
  },
  {
    name: "constructor_modifier",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "annotation" },
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "private" },
    ] },
    lineNumber: 724,
  },
  {
    name: "constructor_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "rule_reference", name: "explicit_constructor_invocation" } },
      { type: "repetition", element: { type: "rule_reference", name: "block_statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 729,
  },
  {
    name: "explicit_constructor_invocation",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "optional", element: { type: "rule_reference", name: "type_arguments" } },
        { type: "literal", value: "this" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "optional", element: { type: "rule_reference", name: "type_arguments" } },
        { type: "literal", value: "super" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 737,
  },
  {
    name: "static_initializer",
    body: { type: "sequence", elements: [
      { type: "literal", value: "static" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 749,
  },
  {
    name: "instance_initializer",
    body: { type: "rule_reference", name: "block" },
    lineNumber: 751,
  },
  {
    name: "type",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
        { type: "rule_reference", name: "primitive_type" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "LBRACKET" },
            { type: "token_reference", name: "RBRACKET" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
        { type: "rule_reference", name: "class_type" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "LBRACKET" },
            { type: "token_reference", name: "RBRACKET" },
          ] } },
      ] },
    ] },
    lineNumber: 777,
  },
  {
    name: "primitive_type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "boolean" },
      { type: "literal", value: "byte" },
      { type: "literal", value: "short" },
      { type: "literal", value: "int" },
      { type: "literal", value: "long" },
      { type: "literal", value: "char" },
      { type: "literal", value: "float" },
      { type: "literal", value: "double" },
    ] },
    lineNumber: 780,
  },
  {
    name: "class_type",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
      { type: "rule_reference", name: "qualified_name" },
      { type: "optional", element: { type: "rule_reference", name: "type_arguments" } },
    ] },
    lineNumber: 794,
  },
  {
    name: "local_var_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "type" },
      { type: "literal", value: "var" },
    ] },
    lineNumber: 812,
  },
  {
    name: "statement",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "block" },
      { type: "rule_reference", name: "var_declaration" },
      { type: "rule_reference", name: "empty_statement" },
      { type: "rule_reference", name: "expression_statement" },
      { type: "rule_reference", name: "if_statement" },
      { type: "rule_reference", name: "while_statement" },
      { type: "rule_reference", name: "do_while_statement" },
      { type: "rule_reference", name: "for_statement" },
      { type: "rule_reference", name: "enhanced_for_statement" },
      { type: "rule_reference", name: "switch_statement" },
      { type: "rule_reference", name: "try_statement" },
      { type: "rule_reference", name: "throw_statement" },
      { type: "rule_reference", name: "return_statement" },
      { type: "rule_reference", name: "break_statement" },
      { type: "rule_reference", name: "continue_statement" },
      { type: "rule_reference", name: "yield_statement" },
      { type: "rule_reference", name: "synchronized_statement" },
      { type: "rule_reference", name: "assert_statement" },
      { type: "rule_reference", name: "labelled_statement" },
    ] },
    lineNumber: 859,
  },
  {
    name: "block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "block_statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 891,
  },
  {
    name: "block_statement",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "var_declaration" },
      { type: "rule_reference", name: "class_declaration" },
      { type: "rule_reference", name: "record_declaration" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 893,
  },
  {
    name: "var_declaration",
    body: { type: "rule_reference", name: "local_variable_declaration_statement" },
    lineNumber: 907,
  },
  {
    name: "local_variable_declaration_statement",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
      { type: "optional", element: { type: "literal", value: "final" } },
      { type: "rule_reference", name: "local_var_type" },
      { type: "rule_reference", name: "variable_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 909,
  },
  {
    name: "empty_statement",
    body: { type: "token_reference", name: "SEMICOLON" },
    lineNumber: 914,
  },
  {
    name: "expression_statement",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 922,
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
    lineNumber: 929,
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
    lineNumber: 933,
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
    lineNumber: 937,
  },
  {
    name: "for_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "for" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "for_init" },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "optional", element: { type: "rule_reference", name: "for_update" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 950,
  },
  {
    name: "for_init",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
        { type: "optional", element: { type: "literal", value: "final" } },
        { type: "rule_reference", name: "local_var_type" },
        { type: "rule_reference", name: "variable_declarators" },
      ] },
      { type: "optional", element: { type: "rule_reference", name: "expression_list" } },
    ] },
    lineNumber: 953,
  },
  {
    name: "for_update",
    body: { type: "rule_reference", name: "expression_list" },
    lineNumber: 956,
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
    lineNumber: 958,
  },
  {
    name: "enhanced_for_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "for" },
      { type: "token_reference", name: "LPAREN" },
      { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
      { type: "optional", element: { type: "literal", value: "final" } },
      { type: "rule_reference", name: "local_var_type" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 970,
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
    lineNumber: 1060,
  },
  {
    name: "switch_block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "switch_rule" },
          { type: "rule_reference", name: "switch_block_statement_group" },
        ] } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 1074,
  },
  {
    name: "switch_rule",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "switch_label" },
      { type: "token_reference", name: "ARROW" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "SEMICOLON" },
          ] },
          { type: "rule_reference", name: "block" },
          { type: "sequence", elements: [
            { type: "literal", value: "throw" },
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "SEMICOLON" },
          ] },
        ] } },
    ] },
    lineNumber: 1092,
  },
  {
    name: "switch_block_statement_group",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "switch_label" },
      { type: "token_reference", name: "COLON" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "switch_label" },
          { type: "token_reference", name: "COLON" },
        ] } },
      { type: "repetition", element: { type: "rule_reference", name: "block_statement" } },
    ] },
    lineNumber: 1101,
  },
  {
    name: "switch_label",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "case" },
        { type: "rule_reference", name: "case_constants" },
      ] },
      { type: "literal", value: "default" },
    ] },
    lineNumber: 1116,
  },
  {
    name: "case_constants",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 1125,
  },
  {
    name: "yield_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "yield" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1165,
  },
  {
    name: "try_statement",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "try" },
        { type: "rule_reference", name: "resource_specification" },
        { type: "rule_reference", name: "block" },
        { type: "repetition", element: { type: "rule_reference", name: "catch_clause" } },
        { type: "optional", element: { type: "rule_reference", name: "finally_clause" } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "try" },
        { type: "rule_reference", name: "block" },
        { type: "group", element: { type: "alternation", choices: [
            { type: "sequence", elements: [
              { type: "rule_reference", name: "catch_clause" },
              { type: "repetition", element: { type: "rule_reference", name: "catch_clause" } },
              { type: "optional", element: { type: "rule_reference", name: "finally_clause" } },
            ] },
            { type: "rule_reference", name: "finally_clause" },
          ] } },
      ] },
    ] },
    lineNumber: 1191,
  },
  {
    name: "resource_specification",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "resource" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "SEMICOLON" },
          { type: "rule_reference", name: "resource" },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "SEMICOLON" } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 1197,
  },
  {
    name: "resource",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
        { type: "optional", element: { type: "literal", value: "final" } },
        { type: "rule_reference", name: "local_var_type" },
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "EQUALS" },
        { type: "rule_reference", name: "expression" },
      ] },
      { type: "rule_reference", name: "qualified_name" },
    ] },
    lineNumber: 1199,
  },
  {
    name: "catch_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "catch" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "catch_formal_parameter" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1204,
  },
  {
    name: "catch_formal_parameter",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
      { type: "optional", element: { type: "literal", value: "final" } },
      { type: "rule_reference", name: "catch_type" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 1206,
  },
  {
    name: "catch_type",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "class_type" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "PIPE" },
          { type: "rule_reference", name: "class_type" },
        ] } },
    ] },
    lineNumber: 1208,
  },
  {
    name: "finally_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "finally" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1210,
  },
  {
    name: "throw_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "throw" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1214,
  },
  {
    name: "return_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "return" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1218,
  },
  {
    name: "break_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "break" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1231,
  },
  {
    name: "continue_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "continue" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1233,
  },
  {
    name: "synchronized_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "synchronized" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1237,
  },
  {
    name: "assert_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "assert" },
      { type: "rule_reference", name: "expression" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "expression" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1244,
  },
  {
    name: "labelled_statement",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 1248,
  },
  {
    name: "expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "lambda_expression" },
      { type: "rule_reference", name: "assignment_expression" },
    ] },
    lineNumber: 1297,
  },
  {
    name: "lambda_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "lambda_parameters" },
      { type: "token_reference", name: "ARROW" },
      { type: "rule_reference", name: "lambda_body" },
    ] },
    lineNumber: 1300,
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
    lineNumber: 1302,
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
    lineNumber: 1305,
  },
  {
    name: "lambda_parameter",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
        { type: "optional", element: { type: "literal", value: "final" } },
        { type: "rule_reference", name: "type" },
        { type: "token_reference", name: "NAME" },
      ] },
      { type: "sequence", elements: [
        { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
        { type: "optional", element: { type: "literal", value: "final" } },
        { type: "token_reference", name: "NAME" },
      ] },
    ] },
    lineNumber: 1307,
  },
  {
    name: "lambda_body",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "expression" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1310,
  },
  {
    name: "assignment_expression",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "unary_expression" },
        { type: "rule_reference", name: "assignment_operator" },
        { type: "rule_reference", name: "assignment_expression" },
      ] },
      { type: "rule_reference", name: "conditional_expression" },
    ] },
    lineNumber: 1317,
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
    ] },
    lineNumber: 1320,
  },
  {
    name: "conditional_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "logical_or_expression" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "QUESTION" },
          { type: "rule_reference", name: "assignment_expression" },
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "assignment_expression" },
        ] } },
    ] },
    lineNumber: 1335,
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
    lineNumber: 1342,
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
    lineNumber: 1348,
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
    lineNumber: 1352,
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
    lineNumber: 1356,
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
    lineNumber: 1360,
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
    lineNumber: 1364,
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
            { type: "literal", value: "instanceof" },
            { type: "rule_reference", name: "type" },
          ] },
        ] } },
    ] },
    lineNumber: 1372,
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
    lineNumber: 1379,
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
    lineNumber: 1384,
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
    lineNumber: 1389,
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
      { type: "rule_reference", name: "unary_expression_not_plus_minus" },
    ] },
    lineNumber: 1394,
  },
  {
    name: "unary_expression_not_plus_minus",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "TILDE" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "BANG" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "rule_reference", name: "cast_expression" },
      { type: "rule_reference", name: "postfix_expression" },
    ] },
    lineNumber: 1400,
  },
  {
    name: "cast_expression",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "primitive_type" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "LBRACKET" },
            { type: "token_reference", name: "RBRACKET" },
          ] } },
        { type: "token_reference", name: "RPAREN" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "class_type" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "AMPERSAND" },
            { type: "rule_reference", name: "class_type" },
          ] } },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "LBRACKET" },
            { type: "token_reference", name: "RBRACKET" },
          ] } },
        { type: "token_reference", name: "RPAREN" },
        { type: "rule_reference", name: "unary_expression_not_plus_minus" },
      ] },
    ] },
    lineNumber: 1414,
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
    lineNumber: 1420,
  },
  {
    name: "primary_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "primary" },
      { type: "repetition", element: { type: "rule_reference", name: "primary_suffix" } },
    ] },
    lineNumber: 1442,
  },
  {
    name: "primary_suffix",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "DOUBLE_COLON" },
        { type: "optional", element: { type: "rule_reference", name: "type_arguments" } },
        { type: "group", element: { type: "alternation", choices: [
            { type: "token_reference", name: "NAME" },
            { type: "literal", value: "new" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "DOT" },
        { type: "optional", element: { type: "rule_reference", name: "type_arguments" } },
        { type: "token_reference", name: "NAME" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "DOT" },
        { type: "literal", value: "class" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "DOT" },
        { type: "literal", value: "this" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "DOT" },
        { type: "literal", value: "super" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "DOT" },
        { type: "literal", value: "new" },
        { type: "optional", element: { type: "rule_reference", name: "type_arguments" } },
        { type: "token_reference", name: "NAME" },
        { type: "optional", element: { type: "rule_reference", name: "type_arguments" } },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "class_body" } },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LBRACKET" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "RBRACKET" },
      ] },
    ] },
    lineNumber: 1448,
  },
  {
    name: "primary",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "switch_expression" },
      { type: "rule_reference", name: "literal" },
      { type: "literal", value: "this" },
      { type: "sequence", elements: [
        { type: "literal", value: "super" },
        { type: "token_reference", name: "DOUBLE_COLON" },
        { type: "optional", element: { type: "rule_reference", name: "type_arguments" } },
        { type: "group", element: { type: "alternation", choices: [
            { type: "token_reference", name: "NAME" },
            { type: "literal", value: "new" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "super" },
        { type: "token_reference", name: "DOT" },
        { type: "optional", element: { type: "rule_reference", name: "type_arguments" } },
        { type: "token_reference", name: "NAME" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "super" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "new" },
        { type: "optional", element: { type: "rule_reference", name: "type_arguments" } },
        { type: "rule_reference", name: "class_type" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "class_body" } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "new" },
        { type: "rule_reference", name: "array_creation_type" },
        { type: "rule_reference", name: "array_dimension_exprs" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "LBRACKET" },
            { type: "token_reference", name: "RBRACKET" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "new" },
        { type: "rule_reference", name: "array_creation_type" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "LBRACKET" },
            { type: "token_reference", name: "RBRACKET" },
          ] } },
        { type: "rule_reference", name: "array_initializer" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 1466,
  },
  {
    name: "switch_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "switch" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "switch_block" },
    ] },
    lineNumber: 1509,
  },
  {
    name: "argument_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 1511,
  },
  {
    name: "array_creation_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "primitive_type" },
      { type: "rule_reference", name: "class_type" },
    ] },
    lineNumber: 1519,
  },
  {
    name: "array_dimension_exprs",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RBRACKET" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LBRACKET" },
          { type: "rule_reference", name: "expression" },
          { type: "token_reference", name: "RBRACKET" },
        ] } },
    ] },
    lineNumber: 1522,
  },
  {
    name: "literal",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "CHAR" },
      { type: "token_reference", name: "STRING" },
      { type: "literal", value: "true" },
      { type: "literal", value: "false" },
      { type: "literal", value: "null" },
    ] },
    lineNumber: 1551,
  },
],
};
