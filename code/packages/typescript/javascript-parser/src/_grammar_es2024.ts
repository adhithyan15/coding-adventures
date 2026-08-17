// AUTO-GENERATED FILE - DO NOT EDIT
// Source: es2024.grammar
// Regenerate with: grammar-tools compile-grammar es2024.grammar
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
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "token_reference", name: "HASHBANG" } },
      { type: "repetition", element: { type: "rule_reference", name: "source_element" } },
    ] },
    lineNumber: 25,
  },
  {
    name: "source_element",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "import_declaration" },
      { type: "rule_reference", name: "export_declaration" },
      { type: "rule_reference", name: "function_declaration" },
      { type: "rule_reference", name: "generator_declaration" },
      { type: "rule_reference", name: "async_function_declaration" },
      { type: "rule_reference", name: "async_generator_declaration" },
      { type: "rule_reference", name: "class_declaration" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 27,
  },
  {
    name: "function_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "function" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameters" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 40,
  },
  {
    name: "formal_parameters",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "formal_parameter" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "formal_parameter" },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
    ] },
    lineNumber: 43,
  },
  {
    name: "formal_parameter",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "group", element: { type: "alternation", choices: [
            { type: "token_reference", name: "NAME" },
            { type: "rule_reference", name: "binding_pattern" },
          ] } },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "EQUALS" },
            { type: "rule_reference", name: "assignment_expression" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "ELLIPSIS" },
        { type: "group", element: { type: "alternation", choices: [
            { type: "token_reference", name: "NAME" },
            { type: "rule_reference", name: "binding_pattern" },
          ] } },
      ] },
    ] },
    lineNumber: 45,
  },
  {
    name: "function_body",
    body: { type: "repetition", element: { type: "rule_reference", name: "source_element" } },
    lineNumber: 48,
  },
  {
    name: "generator_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "function" },
      { type: "token_reference", name: "STAR" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameters" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 50,
  },
  {
    name: "generator_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "function" },
      { type: "token_reference", name: "STAR" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameters" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 53,
  },
  {
    name: "yield_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "yield" },
      { type: "optional", element: { type: "token_reference", name: "STAR" } },
      { type: "rule_reference", name: "assignment_expression" },
    ] },
    lineNumber: 56,
  },
  {
    name: "async_function_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "async" },
      { type: "literal", value: "function" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameters" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 58,
  },
  {
    name: "async_function_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "async" },
      { type: "literal", value: "function" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameters" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 61,
  },
  {
    name: "async_arrow_function",
    body: { type: "sequence", elements: [
      { type: "literal", value: "async" },
      { type: "rule_reference", name: "arrow_parameters" },
      { type: "token_reference", name: "ARROW" },
      { type: "rule_reference", name: "concise_body" },
    ] },
    lineNumber: 64,
  },
  {
    name: "async_method",
    body: { type: "sequence", elements: [
      { type: "literal", value: "async" },
      { type: "optional", element: { type: "token_reference", name: "STAR" } },
      { type: "rule_reference", name: "property_name" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameters" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 66,
  },
  {
    name: "await_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "await" },
      { type: "rule_reference", name: "unary_expression" },
    ] },
    lineNumber: 69,
  },
  {
    name: "async_generator_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "async" },
      { type: "literal", value: "function" },
      { type: "token_reference", name: "STAR" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameters" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 71,
  },
  {
    name: "async_generator_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "async" },
      { type: "literal", value: "function" },
      { type: "token_reference", name: "STAR" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameters" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 75,
  },
  {
    name: "lexical_declaration",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "literal", value: "let" },
          { type: "literal", value: "const" },
        ] } },
      { type: "rule_reference", name: "binding_list" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 79,
  },
  {
    name: "binding_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "lexical_binding" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "lexical_binding" },
        ] } },
    ] },
    lineNumber: 81,
  },
  {
    name: "lexical_binding",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "NAME" },
          { type: "rule_reference", name: "binding_pattern" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "assignment_expression" },
        ] } },
    ] },
    lineNumber: 83,
  },
  {
    name: "class_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "class" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "class_heritage" } },
      { type: "rule_reference", name: "class_body" },
    ] },
    lineNumber: 87,
  },
  {
    name: "class_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "class" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "optional", element: { type: "rule_reference", name: "class_heritage" } },
      { type: "rule_reference", name: "class_body" },
    ] },
    lineNumber: 89,
  },
  {
    name: "class_heritage",
    body: { type: "sequence", elements: [
      { type: "literal", value: "extends" },
      { type: "rule_reference", name: "left_hand_side_expression" },
    ] },
    lineNumber: 91,
  },
  {
    name: "class_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "class_element" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 93,
  },
  {
    name: "class_element",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "optional", element: { type: "literal", value: "static" } },
        { type: "rule_reference", name: "method_definition" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "optional", element: { type: "literal", value: "static" } },
        { type: "rule_reference", name: "async_method" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "rule_reference", name: "class_field_declaration" },
      { type: "rule_reference", name: "private_method_definition" },
      { type: "rule_reference", name: "static_block" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 95,
  },
  {
    name: "class_field_declaration",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "literal", value: "static" } },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "property_name" },
          { type: "token_reference", name: "PRIVATE_NAME" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "assignment_expression" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 102,
  },
  {
    name: "private_method_definition",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "optional", element: { type: "literal", value: "static" } },
        { type: "token_reference", name: "PRIVATE_NAME" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "formal_parameters" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
      { type: "sequence", elements: [
        { type: "optional", element: { type: "literal", value: "static" } },
        { type: "literal", value: "get" },
        { type: "token_reference", name: "PRIVATE_NAME" },
        { type: "token_reference", name: "LPAREN" },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
      { type: "sequence", elements: [
        { type: "optional", element: { type: "literal", value: "static" } },
        { type: "literal", value: "set" },
        { type: "token_reference", name: "PRIVATE_NAME" },
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "formal_parameter" },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
      { type: "sequence", elements: [
        { type: "optional", element: { type: "literal", value: "static" } },
        { type: "token_reference", name: "STAR" },
        { type: "token_reference", name: "PRIVATE_NAME" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "formal_parameters" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
      { type: "sequence", elements: [
        { type: "optional", element: { type: "literal", value: "static" } },
        { type: "literal", value: "async" },
        { type: "optional", element: { type: "token_reference", name: "STAR" } },
        { type: "token_reference", name: "PRIVATE_NAME" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "formal_parameters" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
    ] },
    lineNumber: 105,
  },
  {
    name: "static_block",
    body: { type: "sequence", elements: [
      { type: "literal", value: "static" },
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 120,
  },
  {
    name: "method_definition",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "property_name" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "formal_parameters" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "get" },
        { type: "rule_reference", name: "property_name" },
        { type: "token_reference", name: "LPAREN" },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "set" },
        { type: "rule_reference", name: "property_name" },
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "formal_parameter" },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "STAR" },
        { type: "rule_reference", name: "property_name" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "formal_parameters" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
    ] },
    lineNumber: 122,
  },
  {
    name: "import_declaration",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "import" },
        { type: "rule_reference", name: "import_clause" },
        { type: "rule_reference", name: "from_clause" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "import" },
        { type: "rule_reference", name: "module_specifier" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 133,
  },
  {
    name: "import_clause",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "default_import" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "named_imports" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "default_import" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "namespace_import" },
          ] } },
      ] },
      { type: "rule_reference", name: "named_imports" },
      { type: "rule_reference", name: "namespace_import" },
    ] },
    lineNumber: 136,
  },
  {
    name: "default_import",
    body: { type: "token_reference", name: "NAME" },
    lineNumber: 141,
  },
  {
    name: "named_imports",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "import_specifier" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "import_specifier" },
            ] } },
          { type: "optional", element: { type: "token_reference", name: "COMMA" } },
        ] } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 143,
  },
  {
    name: "import_specifier",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "as" },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 145,
  },
  {
    name: "namespace_import",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "STAR" },
      { type: "literal", value: "as" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 147,
  },
  {
    name: "from_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "from" },
      { type: "token_reference", name: "STRING" },
    ] },
    lineNumber: 149,
  },
  {
    name: "module_specifier",
    body: { type: "token_reference", name: "STRING" },
    lineNumber: 151,
  },
  {
    name: "export_declaration",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "export" },
        { type: "literal", value: "default" },
        { type: "group", element: { type: "alternation", choices: [
            { type: "rule_reference", name: "function_declaration" },
            { type: "rule_reference", name: "generator_declaration" },
            { type: "rule_reference", name: "async_function_declaration" },
            { type: "rule_reference", name: "async_generator_declaration" },
            { type: "rule_reference", name: "class_declaration" },
            { type: "sequence", elements: [
              { type: "rule_reference", name: "assignment_expression" },
              { type: "token_reference", name: "SEMICOLON" },
            ] },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "export" },
        { type: "group", element: { type: "alternation", choices: [
            { type: "rule_reference", name: "function_declaration" },
            { type: "rule_reference", name: "generator_declaration" },
            { type: "rule_reference", name: "async_function_declaration" },
            { type: "rule_reference", name: "async_generator_declaration" },
            { type: "rule_reference", name: "class_declaration" },
            { type: "rule_reference", name: "lexical_declaration" },
            { type: "rule_reference", name: "variable_statement" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "export" },
        { type: "rule_reference", name: "named_exports" },
        { type: "optional", element: { type: "rule_reference", name: "from_clause" } },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "export" },
        { type: "token_reference", name: "STAR" },
        { type: "rule_reference", name: "from_clause" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 153,
  },
  {
    name: "named_exports",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "export_specifier" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "export_specifier" },
            ] } },
          { type: "optional", element: { type: "token_reference", name: "COMMA" } },
        ] } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 166,
  },
  {
    name: "export_specifier",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "as" },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 168,
  },
  {
    name: "binding_pattern",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "object_binding_pattern" },
      { type: "rule_reference", name: "array_binding_pattern" },
    ] },
    lineNumber: 174,
  },
  {
    name: "object_binding_pattern",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "binding_property" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "binding_property" },
            ] } },
          { type: "optional", element: { type: "token_reference", name: "COMMA" } },
        ] } },
      { type: "optional", element: { type: "rule_reference", name: "object_rest_property" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 177,
  },
  {
    name: "object_rest_property",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "ELLIPSIS" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 180,
  },
  {
    name: "array_binding_pattern",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "binding_element" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "binding_element" },
            ] } },
          { type: "optional", element: { type: "token_reference", name: "COMMA" } },
        ] } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 182,
  },
  {
    name: "binding_property",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "property_name" },
        { type: "token_reference", name: "COLON" },
        { type: "rule_reference", name: "binding_element" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "EQUALS" },
            { type: "rule_reference", name: "assignment_expression" },
          ] } },
      ] },
    ] },
    lineNumber: 184,
  },
  {
    name: "binding_element",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "group", element: { type: "alternation", choices: [
            { type: "token_reference", name: "NAME" },
            { type: "rule_reference", name: "binding_pattern" },
          ] } },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "EQUALS" },
            { type: "rule_reference", name: "assignment_expression" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "ELLIPSIS" },
        { type: "token_reference", name: "NAME" },
      ] },
    ] },
    lineNumber: 187,
  },
  {
    name: "statement",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "block" },
      { type: "rule_reference", name: "variable_statement" },
      { type: "rule_reference", name: "empty_statement" },
      { type: "rule_reference", name: "if_statement" },
      { type: "rule_reference", name: "while_statement" },
      { type: "rule_reference", name: "do_while_statement" },
      { type: "rule_reference", name: "for_statement" },
      { type: "rule_reference", name: "for_in_statement" },
      { type: "rule_reference", name: "for_of_statement" },
      { type: "rule_reference", name: "for_await_of_statement" },
      { type: "rule_reference", name: "continue_statement" },
      { type: "rule_reference", name: "break_statement" },
      { type: "rule_reference", name: "return_statement" },
      { type: "rule_reference", name: "with_statement" },
      { type: "rule_reference", name: "switch_statement" },
      { type: "rule_reference", name: "labelled_statement" },
      { type: "rule_reference", name: "try_statement" },
      { type: "rule_reference", name: "throw_statement" },
      { type: "rule_reference", name: "debugger_statement" },
      { type: "rule_reference", name: "lexical_declaration" },
      { type: "rule_reference", name: "expression_statement" },
    ] },
    lineNumber: 194,
  },
  {
    name: "block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 216,
  },
  {
    name: "variable_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "var" },
      { type: "rule_reference", name: "variable_declaration_list" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 218,
  },
  {
    name: "variable_declaration_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "variable_declaration" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "variable_declaration" },
        ] } },
    ] },
    lineNumber: 220,
  },
  {
    name: "variable_declaration",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "NAME" },
          { type: "rule_reference", name: "binding_pattern" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "assignment_expression" },
        ] } },
    ] },
    lineNumber: 222,
  },
  {
    name: "empty_statement",
    body: { type: "token_reference", name: "SEMICOLON" },
    lineNumber: 224,
  },
  {
    name: "expression_statement",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 226,
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
    lineNumber: 228,
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
    lineNumber: 230,
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
    lineNumber: 232,
  },
  {
    name: "for_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "for" },
      { type: "token_reference", name: "LPAREN" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "literal", value: "var" },
            { type: "rule_reference", name: "variable_declaration_list" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "let" },
            { type: "rule_reference", name: "binding_list" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "const" },
            { type: "rule_reference", name: "binding_list" },
          ] },
          { type: "optional", element: { type: "rule_reference", name: "expression" } },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 234,
  },
  {
    name: "for_in_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "for" },
      { type: "token_reference", name: "LPAREN" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "literal", value: "var" },
            { type: "rule_reference", name: "variable_declaration" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "let" },
            { type: "rule_reference", name: "binding_element" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "const" },
            { type: "rule_reference", name: "binding_element" },
          ] },
          { type: "rule_reference", name: "left_hand_side_expression" },
        ] } },
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 243,
  },
  {
    name: "for_of_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "for" },
      { type: "token_reference", name: "LPAREN" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "literal", value: "var" },
            { type: "rule_reference", name: "variable_declaration" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "let" },
            { type: "rule_reference", name: "binding_element" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "const" },
            { type: "rule_reference", name: "binding_element" },
          ] },
          { type: "rule_reference", name: "left_hand_side_expression" },
        ] } },
      { type: "literal", value: "of" },
      { type: "rule_reference", name: "assignment_expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 250,
  },
  {
    name: "for_await_of_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "for" },
      { type: "literal", value: "await" },
      { type: "token_reference", name: "LPAREN" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "literal", value: "var" },
            { type: "rule_reference", name: "variable_declaration" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "let" },
            { type: "rule_reference", name: "binding_element" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "const" },
            { type: "rule_reference", name: "binding_element" },
          ] },
          { type: "rule_reference", name: "left_hand_side_expression" },
        ] } },
      { type: "literal", value: "of" },
      { type: "rule_reference", name: "assignment_expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 257,
  },
  {
    name: "continue_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "continue" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 264,
  },
  {
    name: "break_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "break" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 266,
  },
  {
    name: "return_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "return" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 268,
  },
  {
    name: "with_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "with" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 270,
  },
  {
    name: "switch_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "switch" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "case_clause" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "default_clause" },
          { type: "repetition", element: { type: "rule_reference", name: "case_clause" } },
        ] } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 272,
  },
  {
    name: "case_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "case" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "COLON" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
    ] },
    lineNumber: 275,
  },
  {
    name: "default_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "default" },
      { type: "token_reference", name: "COLON" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
    ] },
    lineNumber: 277,
  },
  {
    name: "labelled_statement",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 279,
  },
  {
    name: "try_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "try" },
      { type: "rule_reference", name: "block" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "rule_reference", name: "catch_clause" },
            { type: "optional", element: { type: "rule_reference", name: "finally_clause" } },
          ] },
          { type: "rule_reference", name: "finally_clause" },
        ] } },
    ] },
    lineNumber: 281,
  },
  {
    name: "catch_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "catch" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 283,
  },
  {
    name: "finally_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "finally" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 285,
  },
  {
    name: "throw_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "throw" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 287,
  },
  {
    name: "debugger_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "debugger" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 289,
  },
  {
    name: "expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "assignment_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "assignment_expression" },
        ] } },
    ] },
    lineNumber: 295,
  },
  {
    name: "assignment_expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "arrow_function" },
      { type: "rule_reference", name: "async_arrow_function" },
      { type: "rule_reference", name: "yield_expression" },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "left_hand_side_expression" },
        { type: "rule_reference", name: "assignment_operator" },
        { type: "rule_reference", name: "assignment_expression" },
      ] },
      { type: "rule_reference", name: "conditional_expression" },
    ] },
    lineNumber: 297,
  },
  {
    name: "assignment_operator",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "EQUALS" },
      { type: "token_reference", name: "PLUS_EQUALS" },
      { type: "token_reference", name: "MINUS_EQUALS" },
      { type: "token_reference", name: "STAR_STAR_EQUALS" },
      { type: "token_reference", name: "STAR_EQUALS" },
      { type: "token_reference", name: "SLASH_EQUALS" },
      { type: "token_reference", name: "PERCENT_EQUALS" },
      { type: "token_reference", name: "AMPERSAND_EQUALS" },
      { type: "token_reference", name: "PIPE_EQUALS" },
      { type: "token_reference", name: "CARET_EQUALS" },
      { type: "token_reference", name: "LEFT_SHIFT_EQUALS" },
      { type: "token_reference", name: "RIGHT_SHIFT_EQUALS" },
      { type: "token_reference", name: "UNSIGNED_RIGHT_SHIFT_EQUALS" },
      { type: "token_reference", name: "OR_OR_EQUALS" },
      { type: "token_reference", name: "AND_AND_EQUALS" },
      { type: "token_reference", name: "NULLISH_COALESCE_EQUALS" },
    ] },
    lineNumber: 303,
  },
  {
    name: "conditional_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "nullish_coalescing_expression" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "QUESTION" },
          { type: "rule_reference", name: "assignment_expression" },
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "assignment_expression" },
        ] } },
    ] },
    lineNumber: 310,
  },
  {
    name: "nullish_coalescing_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "logical_or_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "NULLISH_COALESCE" },
          { type: "rule_reference", name: "logical_or_expression" },
        ] } },
    ] },
    lineNumber: 313,
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
    lineNumber: 316,
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
    lineNumber: 318,
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
    lineNumber: 320,
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
    lineNumber: 322,
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
    lineNumber: 324,
  },
  {
    name: "equality_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "relational_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "STRICT_EQUALS" },
              { type: "token_reference", name: "STRICT_NOT_EQUALS" },
              { type: "token_reference", name: "EQUALS_EQUALS" },
              { type: "token_reference", name: "NOT_EQUALS" },
            ] } },
          { type: "rule_reference", name: "relational_expression" },
        ] } },
    ] },
    lineNumber: 326,
  },
  {
    name: "relational_expression",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "shift_expression" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "group", element: { type: "alternation", choices: [
                { type: "token_reference", name: "LESS_THAN" },
                { type: "token_reference", name: "GREATER_THAN" },
                { type: "token_reference", name: "LESS_EQUALS" },
                { type: "token_reference", name: "GREATER_EQUALS" },
                { type: "literal", value: "instanceof" },
                { type: "literal", value: "in" },
              ] } },
            { type: "rule_reference", name: "shift_expression" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "PRIVATE_NAME" },
        { type: "literal", value: "in" },
        { type: "rule_reference", name: "shift_expression" },
      ] },
    ] },
    lineNumber: 330,
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
    lineNumber: 335,
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
    lineNumber: 338,
  },
  {
    name: "multiplicative_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "exponentiation_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "STAR" },
              { type: "token_reference", name: "SLASH" },
              { type: "token_reference", name: "PERCENT" },
            ] } },
          { type: "rule_reference", name: "exponentiation_expression" },
        ] } },
    ] },
    lineNumber: 341,
  },
  {
    name: "exponentiation_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "unary_expression" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "STAR_STAR" },
          { type: "rule_reference", name: "exponentiation_expression" },
        ] } },
    ] },
    lineNumber: 344,
  },
  {
    name: "unary_expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "postfix_expression" },
      { type: "rule_reference", name: "await_expression" },
      { type: "sequence", elements: [
        { type: "literal", value: "delete" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "void" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "typeof" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
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
        { type: "token_reference", name: "TILDE" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "BANG" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
    ] },
    lineNumber: 346,
  },
  {
    name: "postfix_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "left_hand_side_expression" },
      { type: "optional", element: { type: "alternation", choices: [
          { type: "token_reference", name: "PLUS_PLUS" },
          { type: "token_reference", name: "MINUS_MINUS" },
        ] } },
    ] },
    lineNumber: 358,
  },
  {
    name: "left_hand_side_expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "call_expression" },
      { type: "rule_reference", name: "optional_chain_expression" },
      { type: "rule_reference", name: "new_expression" },
    ] },
    lineNumber: 360,
  },
  {
    name: "call_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "member_expression" },
      { type: "rule_reference", name: "arguments" },
      { type: "repetition", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "arguments" },
          { type: "sequence", elements: [
            { type: "token_reference", name: "DOT" },
            { type: "token_reference", name: "NAME" },
          ] },
          { type: "sequence", elements: [
            { type: "token_reference", name: "DOT" },
            { type: "token_reference", name: "PRIVATE_NAME" },
          ] },
          { type: "sequence", elements: [
            { type: "token_reference", name: "LBRACKET" },
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "RBRACKET" },
          ] },
          { type: "rule_reference", name: "template_literal" },
        ] } },
    ] },
    lineNumber: 364,
  },
  {
    name: "optional_chain_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "member_expression" },
      { type: "repetition", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "token_reference", name: "OPTIONAL_CHAIN" },
            { type: "token_reference", name: "NAME" },
          ] },
          { type: "sequence", elements: [
            { type: "token_reference", name: "OPTIONAL_CHAIN" },
            { type: "token_reference", name: "PRIVATE_NAME" },
          ] },
          { type: "sequence", elements: [
            { type: "token_reference", name: "OPTIONAL_CHAIN" },
            { type: "token_reference", name: "LBRACKET" },
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "RBRACKET" },
          ] },
          { type: "sequence", elements: [
            { type: "token_reference", name: "OPTIONAL_CHAIN" },
            { type: "rule_reference", name: "arguments" },
          ] },
          { type: "sequence", elements: [
            { type: "token_reference", name: "DOT" },
            { type: "token_reference", name: "NAME" },
          ] },
          { type: "sequence", elements: [
            { type: "token_reference", name: "DOT" },
            { type: "token_reference", name: "PRIVATE_NAME" },
          ] },
          { type: "sequence", elements: [
            { type: "token_reference", name: "LBRACKET" },
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "RBRACKET" },
          ] },
          { type: "rule_reference", name: "arguments" },
          { type: "rule_reference", name: "template_literal" },
        ] } },
    ] },
    lineNumber: 368,
  },
  {
    name: "new_expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "member_expression" },
      { type: "sequence", elements: [
        { type: "literal", value: "new" },
        { type: "rule_reference", name: "new_expression" },
      ] },
    ] },
    lineNumber: 379,
  },
  {
    name: "member_expression",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "primary_expression" },
        { type: "repetition", element: { type: "alternation", choices: [
            { type: "sequence", elements: [
              { type: "token_reference", name: "DOT" },
              { type: "token_reference", name: "NAME" },
            ] },
            { type: "sequence", elements: [
              { type: "token_reference", name: "DOT" },
              { type: "token_reference", name: "PRIVATE_NAME" },
            ] },
            { type: "sequence", elements: [
              { type: "token_reference", name: "LBRACKET" },
              { type: "rule_reference", name: "expression" },
              { type: "token_reference", name: "RBRACKET" },
            ] },
            { type: "rule_reference", name: "template_literal" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "new" },
        { type: "rule_reference", name: "member_expression" },
        { type: "rule_reference", name: "arguments" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "super" },
        { type: "token_reference", name: "DOT" },
        { type: "token_reference", name: "NAME" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "super" },
        { type: "token_reference", name: "LBRACKET" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "RBRACKET" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "new" },
        { type: "token_reference", name: "DOT" },
        { type: "literal", value: "target" },
      ] },
    ] },
    lineNumber: 382,
  },
  {
    name: "arguments",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "argument_list" },
          { type: "optional", element: { type: "token_reference", name: "COMMA" } },
        ] } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 390,
  },
  {
    name: "argument_list",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "spread_element" },
          { type: "rule_reference", name: "assignment_expression" },
        ] } },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "group", element: { type: "alternation", choices: [
              { type: "rule_reference", name: "spread_element" },
              { type: "rule_reference", name: "assignment_expression" },
            ] } },
        ] } },
    ] },
    lineNumber: 392,
  },
  {
    name: "spread_element",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "ELLIPSIS" },
      { type: "rule_reference", name: "assignment_expression" },
    ] },
    lineNumber: 395,
  },
  {
    name: "arrow_function",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "arrow_parameters" },
      { type: "token_reference", name: "ARROW" },
      { type: "rule_reference", name: "concise_body" },
    ] },
    lineNumber: 397,
  },
  {
    name: "arrow_parameters",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NAME" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "formal_parameters" } },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 399,
  },
  {
    name: "concise_body",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "assignment_expression" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
    ] },
    lineNumber: 402,
  },
  {
    name: "primary_expression",
    body: { type: "alternation", choices: [
      { type: "literal", value: "this" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "BIGINT" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "REGEX" },
      { type: "literal", value: "true" },
      { type: "literal", value: "false" },
      { type: "literal", value: "null" },
      { type: "rule_reference", name: "array_literal" },
      { type: "rule_reference", name: "object_literal" },
      { type: "rule_reference", name: "function_expression" },
      { type: "rule_reference", name: "generator_expression" },
      { type: "rule_reference", name: "async_function_expression" },
      { type: "rule_reference", name: "class_expression" },
      { type: "rule_reference", name: "template_literal" },
      { type: "rule_reference", name: "dynamic_import" },
      { type: "rule_reference", name: "import_meta" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 405,
  },
  {
    name: "dynamic_import",
    body: { type: "sequence", elements: [
      { type: "literal", value: "import" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "assignment_expression" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 425,
  },
  {
    name: "import_meta",
    body: { type: "sequence", elements: [
      { type: "literal", value: "import" },
      { type: "token_reference", name: "DOT" },
      { type: "literal", value: "meta" },
    ] },
    lineNumber: 427,
  },
  {
    name: "array_literal",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "rule_reference", name: "element_list" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 429,
  },
  {
    name: "element_list",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "spread_element" },
          { type: "rule_reference", name: "assignment_expression" },
        ] } },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "optional", element: { type: "alternation", choices: [
              { type: "rule_reference", name: "spread_element" },
              { type: "rule_reference", name: "assignment_expression" },
            ] } },
        ] } },
    ] },
    lineNumber: 431,
  },
  {
    name: "object_literal",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "property_definition" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "property_definition" },
            ] } },
          { type: "optional", element: { type: "token_reference", name: "COMMA" } },
        ] } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 434,
  },
  {
    name: "property_definition",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "property_name" },
        { type: "token_reference", name: "COLON" },
        { type: "rule_reference", name: "assignment_expression" },
      ] },
      { type: "token_reference", name: "NAME" },
      { type: "rule_reference", name: "method_definition" },
      { type: "rule_reference", name: "async_method" },
      { type: "rule_reference", name: "object_spread_property" },
    ] },
    lineNumber: 436,
  },
  {
    name: "object_spread_property",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "ELLIPSIS" },
      { type: "rule_reference", name: "assignment_expression" },
    ] },
    lineNumber: 442,
  },
  {
    name: "property_name",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "NUMBER" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LBRACKET" },
        { type: "rule_reference", name: "assignment_expression" },
        { type: "token_reference", name: "RBRACKET" },
      ] },
    ] },
    lineNumber: 444,
  },
  {
    name: "function_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "function" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameters" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 449,
  },
  {
    name: "template_literal",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "TEMPLATE_NO_SUB" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "TEMPLATE_HEAD" },
        { type: "repetition", element: { type: "rule_reference", name: "template_span" } },
        { type: "token_reference", name: "TEMPLATE_TAIL" },
      ] },
    ] },
    lineNumber: 452,
  },
  {
    name: "template_span",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "TEMPLATE_MIDDLE" },
    ] },
    lineNumber: 455,
  },
],
};
