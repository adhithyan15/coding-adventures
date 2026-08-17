// AUTO-GENERATED FILE - DO NOT EDIT
// Source: ts5.8.grammar
// Regenerate with: grammar-tools compile-grammar ts5.8.grammar
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
    lineNumber: 71,
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
      { type: "rule_reference", name: "ts_class_declaration" },
      { type: "rule_reference", name: "interface_declaration" },
      { type: "rule_reference", name: "type_alias_declaration" },
      { type: "rule_reference", name: "enum_declaration" },
      { type: "rule_reference", name: "namespace_declaration" },
      { type: "rule_reference", name: "ambient_declaration" },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "decorator" },
        { type: "rule_reference", name: "ts_class_declaration" },
      ] },
      { type: "rule_reference", name: "lexical_declaration" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 73,
  },
  {
    name: "function_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "function" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 93,
  },
  {
    name: "function_body",
    body: { type: "repetition", element: { type: "rule_reference", name: "source_element" } },
    lineNumber: 97,
  },
  {
    name: "generator_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "function" },
      { type: "token_reference", name: "STAR" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 99,
  },
  {
    name: "generator_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "function" },
      { type: "token_reference", name: "STAR" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 103,
  },
  {
    name: "yield_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "yield" },
      { type: "optional", element: { type: "token_reference", name: "STAR" } },
      { type: "rule_reference", name: "assignment_expression" },
    ] },
    lineNumber: 107,
  },
  {
    name: "async_function_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "async" },
      { type: "literal", value: "function" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 109,
  },
  {
    name: "async_function_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "async" },
      { type: "literal", value: "function" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 113,
  },
  {
    name: "async_arrow_function",
    body: { type: "sequence", elements: [
      { type: "literal", value: "async" },
      { type: "rule_reference", name: "arrow_parameters" },
      { type: "token_reference", name: "ARROW" },
      { type: "rule_reference", name: "concise_body" },
    ] },
    lineNumber: 117,
  },
  {
    name: "async_method",
    body: { type: "sequence", elements: [
      { type: "literal", value: "async" },
      { type: "optional", element: { type: "token_reference", name: "STAR" } },
      { type: "rule_reference", name: "property_name" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 119,
  },
  {
    name: "await_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "await" },
      { type: "rule_reference", name: "unary_expression" },
    ] },
    lineNumber: 123,
  },
  {
    name: "async_generator_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "async" },
      { type: "literal", value: "function" },
      { type: "token_reference", name: "STAR" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 125,
  },
  {
    name: "async_generator_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "async" },
      { type: "literal", value: "function" },
      { type: "token_reference", name: "STAR" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 130,
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
    lineNumber: 135,
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
    lineNumber: 137,
  },
  {
    name: "lexical_binding",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "NAME" },
          { type: "rule_reference", name: "binding_pattern" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "assignment_expression" },
        ] } },
    ] },
    lineNumber: 139,
  },
  {
    name: "using_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "using" },
      { type: "rule_reference", name: "binding_list" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 169,
  },
  {
    name: "await_using_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "await" },
      { type: "literal", value: "using" },
      { type: "rule_reference", name: "binding_list" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 171,
  },
  {
    name: "decorator",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "AT" },
      { type: "rule_reference", name: "decorator_expression" },
    ] },
    lineNumber: 192,
  },
  {
    name: "decorator_expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "decorator_member_expression" },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "decorator_member_expression" },
        { type: "rule_reference", name: "arguments" },
      ] },
    ] },
    lineNumber: 197,
  },
  {
    name: "decorator_member_expression",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "DOT" },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 200,
  },
  {
    name: "decorated_class_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "decorator" } },
      { type: "rule_reference", name: "ts_class_declaration" },
    ] },
    lineNumber: 209,
  },
  {
    name: "import_declaration",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "import" },
        { type: "rule_reference", name: "import_clause" },
        { type: "rule_reference", name: "from_clause" },
        { type: "optional", element: { type: "rule_reference", name: "import_attributes" } },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "import" },
        { type: "rule_reference", name: "module_specifier" },
        { type: "optional", element: { type: "rule_reference", name: "import_attributes" } },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "import" },
        { type: "literal", value: "type" },
        { type: "rule_reference", name: "import_clause" },
        { type: "rule_reference", name: "from_clause" },
        { type: "optional", element: { type: "rule_reference", name: "import_attributes" } },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 225,
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
    lineNumber: 229,
  },
  {
    name: "default_import",
    body: { type: "token_reference", name: "NAME" },
    lineNumber: 234,
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
    lineNumber: 236,
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
    lineNumber: 238,
  },
  {
    name: "namespace_import",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "STAR" },
      { type: "literal", value: "as" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 240,
  },
  {
    name: "from_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "from" },
      { type: "token_reference", name: "STRING" },
    ] },
    lineNumber: 242,
  },
  {
    name: "module_specifier",
    body: { type: "token_reference", name: "STRING" },
    lineNumber: 244,
  },
  {
    name: "import_attributes",
    body: { type: "sequence", elements: [
      { type: "literal", value: "with" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "attribute_list" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 247,
  },
  {
    name: "attribute_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "import_attribute" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "import_attribute" },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
    ] },
    lineNumber: 249,
  },
  {
    name: "import_attribute",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "STRING" },
        ] } },
      { type: "token_reference", name: "COLON" },
      { type: "token_reference", name: "STRING" },
    ] },
    lineNumber: 251,
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
            { type: "rule_reference", name: "decorated_class_declaration" },
            { type: "rule_reference", name: "ts_class_declaration" },
            { type: "rule_reference", name: "interface_declaration" },
            { type: "rule_reference", name: "type_alias_declaration" },
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
            { type: "rule_reference", name: "decorated_class_declaration" },
            { type: "rule_reference", name: "ts_class_declaration" },
            { type: "rule_reference", name: "interface_declaration" },
            { type: "rule_reference", name: "type_alias_declaration" },
            { type: "rule_reference", name: "enum_declaration" },
            { type: "rule_reference", name: "namespace_declaration" },
            { type: "rule_reference", name: "lexical_declaration" },
            { type: "rule_reference", name: "variable_statement" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "export" },
        { type: "rule_reference", name: "named_exports" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "from_clause" },
            { type: "optional", element: { type: "rule_reference", name: "import_attributes" } },
          ] } },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "export" },
        { type: "token_reference", name: "STAR" },
        { type: "rule_reference", name: "from_clause" },
        { type: "optional", element: { type: "rule_reference", name: "import_attributes" } },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "export" },
        { type: "literal", value: "type" },
        { type: "rule_reference", name: "named_exports" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "from_clause" },
            { type: "optional", element: { type: "rule_reference", name: "import_attributes" } },
          ] } },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "export" },
        { type: "literal", value: "type" },
        { type: "token_reference", name: "STAR" },
        { type: "rule_reference", name: "from_clause" },
        { type: "optional", element: { type: "rule_reference", name: "import_attributes" } },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "export" },
        { type: "literal", value: "type" },
        { type: "token_reference", name: "STAR" },
        { type: "literal", value: "as" },
        { type: "token_reference", name: "NAME" },
        { type: "rule_reference", name: "from_clause" },
        { type: "optional", element: { type: "rule_reference", name: "import_attributes" } },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 265,
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
    lineNumber: 287,
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
    lineNumber: 289,
  },
  {
    name: "binding_pattern",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "object_binding_pattern" },
      { type: "rule_reference", name: "array_binding_pattern" },
    ] },
    lineNumber: 295,
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
    lineNumber: 298,
  },
  {
    name: "object_rest_property",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "ELLIPSIS" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 301,
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
    lineNumber: 303,
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
    lineNumber: 305,
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
    lineNumber: 308,
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
      { type: "rule_reference", name: "using_declaration" },
      { type: "rule_reference", name: "await_using_declaration" },
      { type: "rule_reference", name: "expression_statement" },
    ] },
    lineNumber: 318,
  },
  {
    name: "block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 342,
  },
  {
    name: "variable_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "var" },
      { type: "rule_reference", name: "variable_declaration_list" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 344,
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
    lineNumber: 346,
  },
  {
    name: "variable_declaration",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "NAME" },
          { type: "rule_reference", name: "binding_pattern" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "assignment_expression" },
        ] } },
    ] },
    lineNumber: 348,
  },
  {
    name: "empty_statement",
    body: { type: "token_reference", name: "SEMICOLON" },
    lineNumber: 350,
  },
  {
    name: "expression_statement",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 352,
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
    lineNumber: 354,
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
    lineNumber: 356,
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
    lineNumber: 358,
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
    lineNumber: 360,
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
    lineNumber: 369,
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
          { type: "sequence", elements: [
            { type: "literal", value: "using" },
            { type: "rule_reference", name: "binding_element" },
          ] },
          { type: "rule_reference", name: "left_hand_side_expression" },
        ] } },
      { type: "literal", value: "of" },
      { type: "rule_reference", name: "assignment_expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 378,
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
          { type: "sequence", elements: [
            { type: "literal", value: "using" },
            { type: "rule_reference", name: "binding_element" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "await" },
            { type: "literal", value: "using" },
            { type: "rule_reference", name: "binding_element" },
          ] },
          { type: "rule_reference", name: "left_hand_side_expression" },
        ] } },
      { type: "literal", value: "of" },
      { type: "rule_reference", name: "assignment_expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 387,
  },
  {
    name: "continue_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "continue" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 396,
  },
  {
    name: "break_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "break" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 398,
  },
  {
    name: "return_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "return" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 400,
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
    lineNumber: 402,
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
    lineNumber: 404,
  },
  {
    name: "case_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "case" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "COLON" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
    ] },
    lineNumber: 407,
  },
  {
    name: "default_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "default" },
      { type: "token_reference", name: "COLON" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
    ] },
    lineNumber: 409,
  },
  {
    name: "labelled_statement",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 411,
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
    lineNumber: 413,
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
    lineNumber: 415,
  },
  {
    name: "finally_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "finally" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 417,
  },
  {
    name: "throw_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "throw" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 419,
  },
  {
    name: "debugger_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "debugger" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 421,
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
    lineNumber: 427,
  },
  {
    name: "assignment_expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "conditional_expression" },
      { type: "rule_reference", name: "arrow_function" },
      { type: "rule_reference", name: "async_arrow_function" },
      { type: "rule_reference", name: "yield_expression" },
      { type: "rule_reference", name: "ts_as_expression" },
      { type: "rule_reference", name: "ts_satisfies_expression" },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "left_hand_side_expression" },
        { type: "rule_reference", name: "assignment_operator" },
        { type: "rule_reference", name: "assignment_expression" },
      ] },
    ] },
    lineNumber: 429,
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
    lineNumber: 437,
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
    lineNumber: 444,
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
    lineNumber: 447,
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
    lineNumber: 450,
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
    lineNumber: 452,
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
    lineNumber: 454,
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
    lineNumber: 456,
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
    lineNumber: 458,
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
    lineNumber: 460,
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
    lineNumber: 465,
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
    lineNumber: 470,
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
    lineNumber: 473,
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
    lineNumber: 476,
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
    lineNumber: 479,
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
    lineNumber: 481,
  },
  {
    name: "postfix_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "left_hand_side_expression" },
      { type: "optional", element: { type: "alternation", choices: [
          { type: "token_reference", name: "PLUS_PLUS" },
          { type: "token_reference", name: "MINUS_MINUS" },
          { type: "token_reference", name: "BANG" },
        ] } },
    ] },
    lineNumber: 494,
  },
  {
    name: "left_hand_side_expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "call_expression" },
      { type: "rule_reference", name: "optional_chain_expression" },
      { type: "rule_reference", name: "new_expression" },
    ] },
    lineNumber: 496,
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
    lineNumber: 500,
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
    lineNumber: 504,
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
    lineNumber: 515,
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
    lineNumber: 518,
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
    lineNumber: 526,
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
    lineNumber: 528,
  },
  {
    name: "spread_element",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "ELLIPSIS" },
      { type: "rule_reference", name: "assignment_expression" },
    ] },
    lineNumber: 531,
  },
  {
    name: "arrow_function",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "arrow_parameters" },
      { type: "token_reference", name: "ARROW" },
      { type: "rule_reference", name: "concise_body" },
    ] },
    lineNumber: 533,
  },
  {
    name: "arrow_parameters",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NAME" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COLON" },
            { type: "rule_reference", name: "type_expression" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LESS_THAN" },
        { type: "rule_reference", name: "type_parameter_list" },
        { type: "token_reference", name: "GREATER_THAN" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COLON" },
            { type: "rule_reference", name: "type_expression" },
          ] } },
      ] },
    ] },
    lineNumber: 535,
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
    lineNumber: 539,
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
      { type: "rule_reference", name: "ts_class_expression" },
      { type: "rule_reference", name: "template_literal" },
      { type: "rule_reference", name: "dynamic_import" },
      { type: "rule_reference", name: "import_meta" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 542,
  },
  {
    name: "dynamic_import",
    body: { type: "sequence", elements: [
      { type: "literal", value: "import" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "assignment_expression" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "token_reference", name: "LBRACE" },
          { type: "literal", value: "with" },
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "import_attributes" },
          { type: "token_reference", name: "RBRACE" },
        ] } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 563,
  },
  {
    name: "import_meta",
    body: { type: "sequence", elements: [
      { type: "literal", value: "import" },
      { type: "token_reference", name: "DOT" },
      { type: "literal", value: "meta" },
    ] },
    lineNumber: 565,
  },
  {
    name: "array_literal",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "rule_reference", name: "element_list" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 567,
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
    lineNumber: 569,
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
    lineNumber: 572,
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
    lineNumber: 574,
  },
  {
    name: "object_spread_property",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "ELLIPSIS" },
      { type: "rule_reference", name: "assignment_expression" },
    ] },
    lineNumber: 580,
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
    lineNumber: 582,
  },
  {
    name: "function_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "function" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 587,
  },
  {
    name: "method_definition",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "property_name" },
        { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COLON" },
            { type: "rule_reference", name: "type_expression" },
          ] } },
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "get" },
        { type: "rule_reference", name: "property_name" },
        { type: "token_reference", name: "LPAREN" },
        { type: "token_reference", name: "RPAREN" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COLON" },
            { type: "rule_reference", name: "type_expression" },
          ] } },
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "set" },
        { type: "rule_reference", name: "property_name" },
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "typed_parameter" },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "STAR" },
        { type: "rule_reference", name: "property_name" },
        { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COLON" },
            { type: "rule_reference", name: "type_expression" },
          ] } },
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
    ] },
    lineNumber: 591,
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
    lineNumber: 600,
  },
  {
    name: "template_span",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "TEMPLATE_MIDDLE" },
    ] },
    lineNumber: 603,
  },
  {
    name: "type_annotation",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "type_expression" },
    ] },
    lineNumber: 609,
  },
  {
    name: "type_expression",
    body: { type: "rule_reference", name: "conditional_type" },
    lineNumber: 611,
  },
  {
    name: "conditional_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "union_type" },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "union_type" },
        { type: "literal", value: "extends" },
        { type: "rule_reference", name: "type_expression" },
        { type: "token_reference", name: "QUESTION" },
        { type: "rule_reference", name: "type_expression" },
        { type: "token_reference", name: "COLON" },
        { type: "rule_reference", name: "type_expression" },
      ] },
    ] },
    lineNumber: 620,
  },
  {
    name: "union_type",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "intersection_type" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "PIPE" },
          { type: "rule_reference", name: "intersection_type" },
        ] } },
    ] },
    lineNumber: 623,
  },
  {
    name: "intersection_type",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "array_type" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "AMPERSAND" },
          { type: "rule_reference", name: "array_type" },
        ] } },
    ] },
    lineNumber: 625,
  },
  {
    name: "array_type",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "primary_type" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LBRACKET" },
          { type: "token_reference", name: "RBRACKET" },
        ] } },
    ] },
    lineNumber: 627,
  },
  {
    name: "primary_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "predefined_type" },
      { type: "rule_reference", name: "type_reference" },
      { type: "rule_reference", name: "literal_type" },
      { type: "rule_reference", name: "object_type" },
      { type: "rule_reference", name: "tuple_type" },
      { type: "rule_reference", name: "function_type" },
      { type: "rule_reference", name: "constructor_type" },
      { type: "rule_reference", name: "mapped_type" },
      { type: "sequence", elements: [
        { type: "literal", value: "typeof" },
        { type: "rule_reference", name: "left_hand_side_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "keyof" },
        { type: "rule_reference", name: "type_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "unique" },
        { type: "literal", value: "symbol" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "infer" },
        { type: "token_reference", name: "NAME" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "literal", value: "extends" },
            { type: "rule_reference", name: "type_expression" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "readonly" },
        { type: "rule_reference", name: "array_type" },
      ] },
      { type: "rule_reference", name: "template_literal_type" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "type_expression" },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 633,
  },
  {
    name: "predefined_type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "any" },
      { type: "literal", value: "string" },
      { type: "literal", value: "number" },
      { type: "literal", value: "boolean" },
      { type: "literal", value: "void" },
      { type: "literal", value: "never" },
      { type: "literal", value: "object" },
      { type: "literal", value: "symbol" },
      { type: "literal", value: "bigint" },
      { type: "literal", value: "undefined" },
      { type: "literal", value: "null" },
      { type: "literal", value: "unknown" },
    ] },
    lineNumber: 649,
  },
  {
    name: "literal_type",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "BIGINT" },
      { type: "token_reference", name: "STRING" },
      { type: "literal", value: "true" },
      { type: "literal", value: "false" },
    ] },
    lineNumber: 652,
  },
  {
    name: "type_reference",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "DOT" },
          { type: "token_reference", name: "NAME" },
        ] } },
      { type: "optional", element: { type: "rule_reference", name: "type_arguments" } },
    ] },
    lineNumber: 654,
  },
  {
    name: "type_arguments",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LESS_THAN" },
      { type: "rule_reference", name: "type_argument_list" },
      { type: "token_reference", name: "GREATER_THAN" },
    ] },
    lineNumber: 655,
  },
  {
    name: "type_argument_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
    ] },
    lineNumber: 656,
  },
  {
    name: "type_parameters",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LESS_THAN" },
      { type: "rule_reference", name: "type_parameter_list" },
      { type: "token_reference", name: "GREATER_THAN" },
    ] },
    lineNumber: 658,
  },
  {
    name: "type_parameter_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type_parameter" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "type_parameter" },
        ] } },
    ] },
    lineNumber: 659,
  },
  {
    name: "type_parameter",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "alternation", choices: [
          { type: "literal", value: "in" },
          { type: "literal", value: "out" },
          { type: "sequence", elements: [
            { type: "literal", value: "in" },
            { type: "literal", value: "out" },
          ] },
          { type: "literal", value: "const" },
        ] } },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "extends" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
    ] },
    lineNumber: 660,
  },
  {
    name: "object_type",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "type_member" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 667,
  },
  {
    name: "type_member",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "construct_signature" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "call_signature" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "index_signature" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "method_signature" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "property_signature" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 669,
  },
  {
    name: "property_signature",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "literal", value: "readonly" } },
      { type: "rule_reference", name: "property_name" },
      { type: "optional", element: { type: "token_reference", name: "QUESTION" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
    ] },
    lineNumber: 675,
  },
  {
    name: "index_signature",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "type_expression" },
      { type: "token_reference", name: "RBRACKET" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "type_expression" },
    ] },
    lineNumber: 677,
  },
  {
    name: "method_signature",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "property_name" },
      { type: "optional", element: { type: "token_reference", name: "QUESTION" } },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
    ] },
    lineNumber: 679,
  },
  {
    name: "call_signature",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
    ] },
    lineNumber: 682,
  },
  {
    name: "construct_signature",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "abstract" },
        { type: "literal", value: "new" },
        { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "ARROW" },
        { type: "rule_reference", name: "type_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "new" },
        { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COLON" },
            { type: "rule_reference", name: "type_expression" },
          ] } },
      ] },
    ] },
    lineNumber: 684,
  },
  {
    name: "tuple_type",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "rule_reference", name: "tuple_element_list" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 698,
  },
  {
    name: "tuple_element_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "tuple_element" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "tuple_element" },
        ] } },
    ] },
    lineNumber: 699,
  },
  {
    name: "tuple_element",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "COLON" },
        ] } },
      { type: "optional", element: { type: "literal", value: "readonly" } },
      { type: "optional", element: { type: "token_reference", name: "ELLIPSIS" } },
      { type: "rule_reference", name: "type_expression" },
      { type: "optional", element: { type: "token_reference", name: "QUESTION" } },
    ] },
    lineNumber: 700,
  },
  {
    name: "function_type",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "ARROW" },
      { type: "rule_reference", name: "type_expression" },
    ] },
    lineNumber: 706,
  },
  {
    name: "constructor_type",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "abstract" },
        { type: "literal", value: "new" },
        { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "ARROW" },
        { type: "rule_reference", name: "type_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "new" },
        { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "ARROW" },
        { type: "rule_reference", name: "type_expression" },
      ] },
    ] },
    lineNumber: 708,
  },
  {
    name: "mapped_type",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "rule_reference", name: "readonly_modifier" } },
      { type: "token_reference", name: "LBRACKET" },
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "type_expression" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "as" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
      { type: "token_reference", name: "RBRACKET" },
      { type: "optional", element: { type: "rule_reference", name: "question_modifier" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 715,
  },
  {
    name: "readonly_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "readonly" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "PLUS" },
        { type: "literal", value: "readonly" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "MINUS" },
        { type: "literal", value: "readonly" },
      ] },
    ] },
    lineNumber: 719,
  },
  {
    name: "question_modifier",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "QUESTION" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "PLUS" },
        { type: "token_reference", name: "QUESTION" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "MINUS" },
        { type: "token_reference", name: "QUESTION" },
      ] },
    ] },
    lineNumber: 720,
  },
  {
    name: "template_literal_type",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "TEMPLATE_NO_SUB" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "TEMPLATE_HEAD" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "type_expression" },
            { type: "token_reference", name: "TEMPLATE_MIDDLE" },
          ] } },
        { type: "rule_reference", name: "type_expression" },
        { type: "token_reference", name: "TEMPLATE_TAIL" },
      ] },
    ] },
    lineNumber: 726,
  },
  {
    name: "typed_parameter_list",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "typed_parameter" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "typed_parameter" },
          ] } },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "rest_typed_parameter" },
          ] } },
      ] },
      { type: "rule_reference", name: "rest_typed_parameter" },
    ] },
    lineNumber: 733,
  },
  {
    name: "typed_parameter",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "alternation", choices: [
          { type: "literal", value: "public" },
          { type: "literal", value: "private" },
          { type: "literal", value: "protected" },
        ] } },
      { type: "optional", element: { type: "literal", value: "override" } },
      { type: "optional", element: { type: "literal", value: "readonly" } },
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "NAME" },
          { type: "rule_reference", name: "binding_pattern" },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "QUESTION" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "assignment_expression" },
        ] } },
    ] },
    lineNumber: 736,
  },
  {
    name: "rest_typed_parameter",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "ELLIPSIS" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
    ] },
    lineNumber: 740,
  },
  {
    name: "interface_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "interface" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "optional", element: { type: "rule_reference", name: "interface_heritage" } },
      { type: "rule_reference", name: "object_type" },
    ] },
    lineNumber: 746,
  },
  {
    name: "interface_heritage",
    body: { type: "sequence", elements: [
      { type: "literal", value: "extends" },
      { type: "rule_reference", name: "type_reference" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "type_reference" },
        ] } },
    ] },
    lineNumber: 748,
  },
  {
    name: "type_alias_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "type_expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 750,
  },
  {
    name: "enum_declaration",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "literal", value: "const" } },
      { type: "literal", value: "enum" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "rule_reference", name: "enum_body" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 752,
  },
  {
    name: "enum_body",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "enum_member" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "enum_member" },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
    ] },
    lineNumber: 754,
  },
  {
    name: "enum_member",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "property_name" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "assignment_expression" },
        ] } },
    ] },
    lineNumber: 756,
  },
  {
    name: "namespace_declaration",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "literal", value: "namespace" },
          { type: "literal", value: "module" },
        ] } },
      { type: "rule_reference", name: "qualified_name" },
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "namespace_element" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 758,
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
    lineNumber: 760,
  },
  {
    name: "namespace_element",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "namespace_declaration" },
      { type: "rule_reference", name: "interface_declaration" },
      { type: "rule_reference", name: "type_alias_declaration" },
      { type: "rule_reference", name: "ts_class_declaration" },
      { type: "rule_reference", name: "function_declaration" },
      { type: "rule_reference", name: "generator_declaration" },
      { type: "rule_reference", name: "enum_declaration" },
      { type: "rule_reference", name: "lexical_declaration" },
      { type: "rule_reference", name: "variable_statement" },
      { type: "rule_reference", name: "export_assignment" },
      { type: "rule_reference", name: "export_namespace_element" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 762,
  },
  {
    name: "export_assignment",
    body: { type: "sequence", elements: [
      { type: "literal", value: "export" },
      { type: "token_reference", name: "EQUALS" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 767,
  },
  {
    name: "export_namespace_element",
    body: { type: "sequence", elements: [
      { type: "literal", value: "export" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "namespace_declaration" },
          { type: "rule_reference", name: "interface_declaration" },
          { type: "rule_reference", name: "type_alias_declaration" },
          { type: "rule_reference", name: "ts_class_declaration" },
          { type: "rule_reference", name: "function_declaration" },
          { type: "rule_reference", name: "enum_declaration" },
          { type: "rule_reference", name: "lexical_declaration" },
          { type: "rule_reference", name: "variable_statement" },
        ] } },
    ] },
    lineNumber: 769,
  },
  {
    name: "ambient_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "declare" },
      { type: "rule_reference", name: "ambient_declaration_body" },
    ] },
    lineNumber: 773,
  },
  {
    name: "ambient_declaration_body",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "variable_statement" },
      { type: "rule_reference", name: "ambient_function_declaration" },
      { type: "rule_reference", name: "ts_class_declaration" },
      { type: "rule_reference", name: "interface_declaration" },
      { type: "rule_reference", name: "type_alias_declaration" },
      { type: "rule_reference", name: "enum_declaration" },
      { type: "rule_reference", name: "namespace_declaration" },
      { type: "rule_reference", name: "ambient_module_declaration" },
      { type: "sequence", elements: [
        { type: "literal", value: "global" },
        { type: "token_reference", name: "LBRACE" },
        { type: "repetition", element: { type: "rule_reference", name: "namespace_element" } },
        { type: "token_reference", name: "RBRACE" },
      ] },
    ] },
    lineNumber: 775,
  },
  {
    name: "ambient_module_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "module" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "namespace_element" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 780,
  },
  {
    name: "ambient_function_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "function" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 784,
  },
  {
    name: "ts_class_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "decorator" } },
      { type: "optional", element: { type: "rule_reference", name: "ts_class_modifiers" } },
      { type: "literal", value: "class" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "optional", element: { type: "rule_reference", name: "ts_class_heritage" } },
      { type: "rule_reference", name: "ts_class_body" },
    ] },
    lineNumber: 800,
  },
  {
    name: "ts_class_expression",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "decorator" } },
      { type: "optional", element: { type: "rule_reference", name: "ts_class_modifiers" } },
      { type: "literal", value: "class" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "optional", element: { type: "rule_reference", name: "ts_class_heritage" } },
      { type: "rule_reference", name: "ts_class_body" },
    ] },
    lineNumber: 803,
  },
  {
    name: "ts_class_modifiers",
    body: { type: "alternation", choices: [
      { type: "literal", value: "abstract" },
      { type: "literal", value: "declare" },
    ] },
    lineNumber: 806,
  },
  {
    name: "ts_class_heritage",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "extends" },
        { type: "rule_reference", name: "type_reference" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "literal", value: "implements" },
            { type: "rule_reference", name: "type_reference" },
            { type: "repetition", element: { type: "sequence", elements: [
                { type: "token_reference", name: "COMMA" },
                { type: "rule_reference", name: "type_reference" },
              ] } },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "implements" },
        { type: "rule_reference", name: "type_reference" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "type_reference" },
          ] } },
      ] },
    ] },
    lineNumber: 808,
  },
  {
    name: "ts_class_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "ts_class_element" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 811,
  },
  {
    name: "ts_class_element",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "ts_class_member" },
      { type: "rule_reference", name: "static_block" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 813,
  },
  {
    name: "ts_class_member",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "ts_constructor_declaration" },
      { type: "rule_reference", name: "ts_method_declaration" },
      { type: "rule_reference", name: "ts_property_declaration" },
      { type: "rule_reference", name: "ts_private_field_declaration" },
      { type: "rule_reference", name: "ts_private_method_declaration" },
      { type: "rule_reference", name: "ts_accessor_declaration" },
      { type: "rule_reference", name: "index_signature" },
    ] },
    lineNumber: 817,
  },
  {
    name: "static_block",
    body: { type: "sequence", elements: [
      { type: "literal", value: "static" },
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 825,
  },
  {
    name: "ts_private_field_declaration",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "literal", value: "static" } },
      { type: "token_reference", name: "PRIVATE_NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "assignment_expression" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 827,
  },
  {
    name: "ts_private_method_declaration",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "optional", element: { type: "literal", value: "static" } },
        { type: "token_reference", name: "PRIVATE_NAME" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "formal_parameters" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COLON" },
            { type: "rule_reference", name: "type_expression" },
          ] } },
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
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COLON" },
            { type: "rule_reference", name: "type_expression" },
          ] } },
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
        { type: "literal", value: "async" },
        { type: "optional", element: { type: "token_reference", name: "STAR" } },
        { type: "token_reference", name: "PRIVATE_NAME" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "formal_parameters" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COLON" },
            { type: "rule_reference", name: "type_expression" },
          ] } },
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
    ] },
    lineNumber: 831,
  },
  {
    name: "ts_constructor_declaration",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "rule_reference", name: "accessibility_modifier" } },
      { type: "literal", value: "constructor" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "ts_constructor_params" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "literal", value: "void" },
        ] } },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 846,
  },
  {
    name: "ts_constructor_params",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "ts_constructor_param" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "ts_constructor_param" },
          ] } },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "rest_typed_parameter" },
          ] } },
      ] },
      { type: "rule_reference", name: "rest_typed_parameter" },
    ] },
    lineNumber: 850,
  },
  {
    name: "ts_constructor_param",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "rule_reference", name: "accessibility_modifier" } },
      { type: "optional", element: { type: "literal", value: "override" } },
      { type: "optional", element: { type: "literal", value: "readonly" } },
      { type: "rule_reference", name: "typed_parameter" },
    ] },
    lineNumber: 853,
  },
  {
    name: "accessibility_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "private" },
      { type: "literal", value: "protected" },
    ] },
    lineNumber: 855,
  },
  {
    name: "ts_method_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "decorator" } },
      { type: "repetition", element: { type: "rule_reference", name: "ts_member_modifier" } },
      { type: "rule_reference", name: "ts_method_body" },
    ] },
    lineNumber: 857,
  },
  {
    name: "ts_member_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "private" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "static" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "readonly" },
      { type: "literal", value: "override" },
      { type: "literal", value: "declare" },
    ] },
    lineNumber: 859,
  },
  {
    name: "ts_method_body",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "property_name" },
        { type: "optional", element: { type: "token_reference", name: "QUESTION" } },
        { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COLON" },
            { type: "rule_reference", name: "type_expression" },
          ] } },
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "get" },
        { type: "rule_reference", name: "property_name" },
        { type: "token_reference", name: "LPAREN" },
        { type: "token_reference", name: "RPAREN" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COLON" },
            { type: "rule_reference", name: "type_expression" },
          ] } },
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "set" },
        { type: "rule_reference", name: "property_name" },
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "typed_parameter" },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "STAR" },
        { type: "rule_reference", name: "property_name" },
        { type: "optional", element: { type: "token_reference", name: "QUESTION" } },
        { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COLON" },
            { type: "rule_reference", name: "type_expression" },
          ] } },
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "async" },
        { type: "optional", element: { type: "token_reference", name: "STAR" } },
        { type: "rule_reference", name: "property_name" },
        { type: "optional", element: { type: "token_reference", name: "QUESTION" } },
        { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COLON" },
            { type: "rule_reference", name: "type_expression" },
          ] } },
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
    ] },
    lineNumber: 862,
  },
  {
    name: "ts_property_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "decorator" } },
      { type: "repetition", element: { type: "rule_reference", name: "ts_member_modifier" } },
      { type: "rule_reference", name: "property_name" },
      { type: "optional", element: { type: "token_reference", name: "QUESTION" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "assignment_expression" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 876,
  },
  {
    name: "ts_accessor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "decorator" } },
      { type: "repetition", element: { type: "rule_reference", name: "ts_member_modifier" } },
      { type: "literal", value: "accessor" },
      { type: "rule_reference", name: "property_name" },
      { type: "optional", element: { type: "token_reference", name: "QUESTION" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "assignment_expression" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 879,
  },
  {
    name: "ts_as_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "conditional_expression" },
      { type: "literal", value: "as" },
      { type: "rule_reference", name: "type_expression" },
    ] },
    lineNumber: 886,
  },
  {
    name: "ts_satisfies_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "conditional_expression" },
      { type: "literal", value: "satisfies" },
      { type: "rule_reference", name: "type_expression" },
    ] },
    lineNumber: 888,
  },
  {
    name: "type_predicate",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "optional", element: { type: "literal", value: "asserts" } },
        { type: "token_reference", name: "NAME" },
        { type: "literal", value: "is" },
        { type: "rule_reference", name: "type_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "asserts" },
        { type: "token_reference", name: "NAME" },
      ] },
    ] },
    lineNumber: 893,
  },
],
};
