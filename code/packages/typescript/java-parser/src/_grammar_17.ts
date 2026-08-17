// AUTO-GENERATED FILE - DO NOT EDIT
// Source: java17.grammar
// Regenerate with: grammar-tools compile-grammar java17.grammar
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
    lineNumber: 231,
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
    lineNumber: 232,
  },
  {
    name: "compilation_unit",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
      { type: "optional", element: { type: "rule_reference", name: "package_declaration" } },
      { type: "repetition", element: { type: "rule_reference", name: "import_declaration" } },
      { type: "repetition", element: { type: "rule_reference", name: "type_declaration" } },
    ] },
    lineNumber: 233,
  },
  {
    name: "package_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "package" },
      { type: "rule_reference", name: "qualified_name" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 249,
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
    lineNumber: 264,
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
    lineNumber: 280,
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
    lineNumber: 299,
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
    lineNumber: 322,
  },
  {
    name: "annotations",
    body: { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
    lineNumber: 324,
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
    lineNumber: 326,
  },
  {
    name: "element_value_pair",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "element_value" },
    ] },
    lineNumber: 328,
  },
  {
    name: "element_value",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "annotation" },
      { type: "rule_reference", name: "element_value_array" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 336,
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
    lineNumber: 340,
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
    lineNumber: 356,
  },
  {
    name: "annotation_type_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "annotation_type_element_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 359,
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
    lineNumber: 361,
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
    lineNumber: 369,
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
      { type: "optional", element: { type: "rule_reference", name: "permits_clause" } },
      { type: "rule_reference", name: "class_body" },
    ] },
    lineNumber: 406,
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
      { type: "rule_reference", name: "non_sealed_modifier" },
      { type: "literal", value: "sealed" },
    ] },
    lineNumber: 431,
  },
  {
    name: "non_sealed_modifier",
    body: { type: "sequence", elements: [
      { type: "literal", value: "non" },
      { type: "token_reference", name: "MINUS" },
      { type: "literal", value: "sealed" },
    ] },
    lineNumber: 458,
  },
  {
    name: "permits_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "permits" },
      { type: "rule_reference", name: "class_type" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "class_type" },
        ] } },
    ] },
    lineNumber: 471,
  },
  {
    name: "class_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "class_body_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 473,
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
    lineNumber: 475,
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
      { type: "optional", element: { type: "rule_reference", name: "permits_clause" } },
      { type: "rule_reference", name: "interface_body" },
    ] },
    lineNumber: 523,
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
      { type: "rule_reference", name: "non_sealed_modifier" },
      { type: "literal", value: "sealed" },
    ] },
    lineNumber: 528,
  },
  {
    name: "interface_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "interface_body_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 538,
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
    lineNumber: 544,
  },
  {
    name: "interface_field_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "field_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "variable_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 553,
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
    lineNumber: 576,
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
    lineNumber: 579,
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
    lineNumber: 587,
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
    lineNumber: 607,
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
    lineNumber: 611,
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
    lineNumber: 613,
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
    lineNumber: 615,
  },
  {
    name: "record_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "class_modifier" } },
      { type: "literal", value: "record" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "rule_reference", name: "record_components" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "implements" },
          { type: "rule_reference", name: "interface_type_list" },
        ] } },
      { type: "rule_reference", name: "record_body" },
    ] },
    lineNumber: 674,
  },
  {
    name: "record_components",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "record_component_list" } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 689,
  },
  {
    name: "record_component_list",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "record_component" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "record_component" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "record_component" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "record_component" },
          ] } },
        { type: "token_reference", name: "COMMA" },
        { type: "rule_reference", name: "varargs_record_component" },
      ] },
      { type: "rule_reference", name: "varargs_record_component" },
    ] },
    lineNumber: 691,
  },
  {
    name: "record_component",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 695,
  },
  {
    name: "varargs_record_component",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "ELLIPSIS" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 697,
  },
  {
    name: "record_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "record_body_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 718,
  },
  {
    name: "record_body_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "compact_constructor_declaration" },
      { type: "rule_reference", name: "constructor_declaration" },
      { type: "rule_reference", name: "method_declaration" },
      { type: "rule_reference", name: "field_declaration" },
      { type: "rule_reference", name: "class_declaration" },
      { type: "rule_reference", name: "interface_declaration" },
      { type: "rule_reference", name: "enum_declaration" },
      { type: "rule_reference", name: "annotation_type_declaration" },
      { type: "rule_reference", name: "record_declaration" },
      { type: "rule_reference", name: "static_initializer" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 720,
  },
  {
    name: "compact_constructor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "constructor_modifier" } },
      { type: "token_reference", name: "NAME" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 740,
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
    lineNumber: 770,
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
    lineNumber: 772,
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
    lineNumber: 774,
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
    lineNumber: 788,
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
    lineNumber: 790,
  },
  {
    name: "field_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "field_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "variable_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 808,
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
    lineNumber: 810,
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
    lineNumber: 819,
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
    lineNumber: 821,
  },
  {
    name: "variable_initializer",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "expression" },
      { type: "rule_reference", name: "array_initializer" },
    ] },
    lineNumber: 823,
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
    lineNumber: 826,
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
    lineNumber: 843,
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
    lineNumber: 846,
  },
  {
    name: "result_type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "void" },
      { type: "rule_reference", name: "type" },
    ] },
    lineNumber: 857,
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
    lineNumber: 860,
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
    lineNumber: 877,
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
    lineNumber: 881,
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
    lineNumber: 883,
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
    lineNumber: 885,
  },
  {
    name: "method_body",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "block" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 887,
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
    lineNumber: 904,
  },
  {
    name: "constructor_modifier",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "annotation" },
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "private" },
    ] },
    lineNumber: 908,
  },
  {
    name: "constructor_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "rule_reference", name: "explicit_constructor_invocation" } },
      { type: "repetition", element: { type: "rule_reference", name: "block_statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 913,
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
    lineNumber: 921,
  },
  {
    name: "static_initializer",
    body: { type: "sequence", elements: [
      { type: "literal", value: "static" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 933,
  },
  {
    name: "instance_initializer",
    body: { type: "rule_reference", name: "block" },
    lineNumber: 935,
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
    lineNumber: 965,
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
    lineNumber: 968,
  },
  {
    name: "class_type",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
      { type: "rule_reference", name: "qualified_name" },
      { type: "optional", element: { type: "rule_reference", name: "type_arguments" } },
    ] },
    lineNumber: 982,
  },
  {
    name: "local_var_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "type" },
      { type: "literal", value: "var" },
    ] },
    lineNumber: 1003,
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
    lineNumber: 1041,
  },
  {
    name: "block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "block_statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 1063,
  },
  {
    name: "block_statement",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "var_declaration" },
      { type: "rule_reference", name: "class_declaration" },
      { type: "rule_reference", name: "record_declaration" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 1065,
  },
  {
    name: "var_declaration",
    body: { type: "rule_reference", name: "local_variable_declaration_statement" },
    lineNumber: 1083,
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
    lineNumber: 1085,
  },
  {
    name: "empty_statement",
    body: { type: "token_reference", name: "SEMICOLON" },
    lineNumber: 1090,
  },
  {
    name: "expression_statement",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1094,
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
    lineNumber: 1101,
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
    lineNumber: 1105,
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
    lineNumber: 1109,
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
    lineNumber: 1113,
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
    lineNumber: 1116,
  },
  {
    name: "for_update",
    body: { type: "rule_reference", name: "expression_list" },
    lineNumber: 1119,
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
    lineNumber: 1121,
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
    lineNumber: 1129,
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
    lineNumber: 1182,
  },
  {
    name: "switch_block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "switch_rule" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 1188,
  },
  {
    name: "switch_rule",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "switch_label" },
        { type: "token_reference", name: "COLON" },
        { type: "repetition", element: { type: "rule_reference", name: "block_statement" } },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "switch_label" },
        { type: "token_reference", name: "ARROW" },
        { type: "rule_reference", name: "switch_rule_body" },
      ] },
    ] },
    lineNumber: 1199,
  },
  {
    name: "switch_rule_body",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "block" },
      { type: "sequence", elements: [
        { type: "literal", value: "throw" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 1202,
  },
  {
    name: "switch_label",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "case" },
        { type: "rule_reference", name: "case_constant" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "case_constant" },
          ] } },
      ] },
      { type: "literal", value: "default" },
    ] },
    lineNumber: 1215,
  },
  {
    name: "case_constant",
    body: { type: "rule_reference", name: "expression" },
    lineNumber: 1218,
  },
  {
    name: "yield_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "yield" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1240,
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
    lineNumber: 1256,
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
    lineNumber: 1260,
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
    lineNumber: 1262,
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
    lineNumber: 1267,
  },
  {
    name: "catch_formal_parameter",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
      { type: "optional", element: { type: "literal", value: "final" } },
      { type: "rule_reference", name: "catch_type" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 1269,
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
    lineNumber: 1271,
  },
  {
    name: "finally_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "finally" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1273,
  },
  {
    name: "throw_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "throw" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1277,
  },
  {
    name: "return_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "return" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1281,
  },
  {
    name: "break_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "break" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1285,
  },
  {
    name: "continue_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "continue" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1287,
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
    lineNumber: 1291,
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
    lineNumber: 1295,
  },
  {
    name: "labelled_statement",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 1299,
  },
  {
    name: "expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "lambda_expression" },
      { type: "rule_reference", name: "assignment_expression" },
    ] },
    lineNumber: 1346,
  },
  {
    name: "lambda_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "lambda_parameters" },
      { type: "token_reference", name: "ARROW" },
      { type: "rule_reference", name: "lambda_body" },
    ] },
    lineNumber: 1349,
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
    lineNumber: 1351,
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
    lineNumber: 1361,
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
        { type: "literal", value: "var" },
        { type: "token_reference", name: "NAME" },
      ] },
      { type: "sequence", elements: [
        { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
        { type: "optional", element: { type: "literal", value: "final" } },
        { type: "token_reference", name: "NAME" },
      ] },
    ] },
    lineNumber: 1363,
  },
  {
    name: "lambda_body",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "expression" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1367,
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
    lineNumber: 1374,
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
    lineNumber: 1377,
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
    lineNumber: 1392,
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
    lineNumber: 1397,
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
    lineNumber: 1401,
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
    lineNumber: 1405,
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
    lineNumber: 1409,
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
    lineNumber: 1413,
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
    lineNumber: 1417,
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
            { type: "rule_reference", name: "pattern" },
          ] },
        ] } },
    ] },
    lineNumber: 1461,
  },
  {
    name: "pattern",
    body: { type: "rule_reference", name: "type_pattern" },
    lineNumber: 1492,
  },
  {
    name: "type_pattern",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "optional", element: { type: "literal", value: "final" } },
        { type: "repetition", element: { type: "rule_reference", name: "annotation" } },
        { type: "rule_reference", name: "type" },
        { type: "token_reference", name: "NAME" },
      ] },
      { type: "rule_reference", name: "type" },
    ] },
    lineNumber: 1494,
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
    lineNumber: 1499,
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
    lineNumber: 1504,
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
    lineNumber: 1509,
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
    lineNumber: 1514,
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
    lineNumber: 1520,
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
    lineNumber: 1533,
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
    lineNumber: 1539,
  },
  {
    name: "primary_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "primary" },
      { type: "repetition", element: { type: "rule_reference", name: "primary_suffix" } },
    ] },
    lineNumber: 1560,
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
    lineNumber: 1566,
  },
  {
    name: "primary",
    body: { type: "alternation", choices: [
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
      { type: "rule_reference", name: "switch_expression" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 1596,
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
    lineNumber: 1608,
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
    lineNumber: 1620,
  },
  {
    name: "array_creation_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "primitive_type" },
      { type: "rule_reference", name: "class_type" },
    ] },
    lineNumber: 1624,
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
    lineNumber: 1627,
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
    lineNumber: 1653,
  },
],
};
