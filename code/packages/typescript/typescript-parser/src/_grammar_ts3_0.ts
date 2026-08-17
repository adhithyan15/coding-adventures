// AUTO-GENERATED FILE - DO NOT EDIT
// Source: ts3.0.grammar
// Regenerate with: grammar-tools compile-grammar ts3.0.grammar
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
    body: { type: "repetition", element: { type: "rule_reference", name: "source_element" } },
    lineNumber: 66,
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
    lineNumber: 77,
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
    lineNumber: 99,
  },
  {
    name: "function_body",
    body: { type: "repetition", element: { type: "rule_reference", name: "source_element" } },
    lineNumber: 103,
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
    lineNumber: 107,
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
    lineNumber: 111,
  },
  {
    name: "yield_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "yield" },
      { type: "optional", element: { type: "token_reference", name: "STAR" } },
      { type: "rule_reference", name: "assignment_expression" },
    ] },
    lineNumber: 115,
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
    lineNumber: 119,
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
    lineNumber: 123,
  },
  {
    name: "async_arrow_function",
    body: { type: "sequence", elements: [
      { type: "literal", value: "async" },
      { type: "rule_reference", name: "arrow_parameters" },
      { type: "token_reference", name: "ARROW" },
      { type: "rule_reference", name: "concise_body" },
    ] },
    lineNumber: 127,
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
    lineNumber: 129,
  },
  {
    name: "await_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "await" },
      { type: "rule_reference", name: "unary_expression" },
    ] },
    lineNumber: 133,
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
    lineNumber: 141,
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
    lineNumber: 146,
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
    lineNumber: 160,
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
    lineNumber: 162,
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
    lineNumber: 164,
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
      { type: "sequence", elements: [
        { type: "literal", value: "import" },
        { type: "literal", value: "type" },
        { type: "rule_reference", name: "import_clause" },
        { type: "rule_reference", name: "from_clause" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 175,
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
    lineNumber: 179,
  },
  {
    name: "default_import",
    body: { type: "token_reference", name: "NAME" },
    lineNumber: 184,
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
    lineNumber: 186,
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
    lineNumber: 188,
  },
  {
    name: "namespace_import",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "STAR" },
      { type: "literal", value: "as" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 190,
  },
  {
    name: "from_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "from" },
      { type: "token_reference", name: "STRING" },
    ] },
    lineNumber: 192,
  },
  {
    name: "module_specifier",
    body: { type: "token_reference", name: "STRING" },
    lineNumber: 194,
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
        { type: "optional", element: { type: "rule_reference", name: "from_clause" } },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "export" },
        { type: "token_reference", name: "STAR" },
        { type: "rule_reference", name: "from_clause" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "export" },
        { type: "literal", value: "type" },
        { type: "rule_reference", name: "named_exports" },
        { type: "optional", element: { type: "rule_reference", name: "from_clause" } },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 196,
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
    lineNumber: 214,
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
    lineNumber: 216,
  },
  {
    name: "binding_pattern",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "object_binding_pattern" },
      { type: "rule_reference", name: "array_binding_pattern" },
    ] },
    lineNumber: 222,
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
    lineNumber: 226,
  },
  {
    name: "object_rest_property",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "ELLIPSIS" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 229,
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
    lineNumber: 231,
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
    lineNumber: 233,
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
    lineNumber: 236,
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
    lineNumber: 243,
  },
  {
    name: "block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 265,
  },
  {
    name: "variable_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "var" },
      { type: "rule_reference", name: "variable_declaration_list" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 267,
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
    lineNumber: 269,
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
    lineNumber: 271,
  },
  {
    name: "empty_statement",
    body: { type: "token_reference", name: "SEMICOLON" },
    lineNumber: 273,
  },
  {
    name: "expression_statement",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 275,
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
    lineNumber: 277,
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
    lineNumber: 279,
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
    lineNumber: 281,
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
    lineNumber: 283,
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
    lineNumber: 292,
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
    lineNumber: 299,
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
    lineNumber: 307,
  },
  {
    name: "continue_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "continue" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 314,
  },
  {
    name: "break_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "break" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 316,
  },
  {
    name: "return_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "return" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 318,
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
    lineNumber: 320,
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
    lineNumber: 322,
  },
  {
    name: "case_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "case" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "COLON" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
    ] },
    lineNumber: 325,
  },
  {
    name: "default_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "default" },
      { type: "token_reference", name: "COLON" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
    ] },
    lineNumber: 327,
  },
  {
    name: "labelled_statement",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 329,
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
    lineNumber: 331,
  },
  {
    name: "catch_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "catch" },
      { type: "token_reference", name: "LPAREN" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 333,
  },
  {
    name: "finally_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "finally" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 335,
  },
  {
    name: "throw_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "throw" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 337,
  },
  {
    name: "debugger_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "debugger" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 339,
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
    lineNumber: 360,
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
    lineNumber: 362,
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
    ] },
    lineNumber: 370,
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
    lineNumber: 376,
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
    lineNumber: 379,
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
    lineNumber: 381,
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
    lineNumber: 383,
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
    lineNumber: 385,
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
    lineNumber: 387,
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
    lineNumber: 389,
  },
  {
    name: "relational_expression",
    body: { type: "sequence", elements: [
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
    lineNumber: 393,
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
    lineNumber: 397,
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
    lineNumber: 400,
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
    lineNumber: 403,
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
    lineNumber: 406,
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
    lineNumber: 408,
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
    lineNumber: 428,
  },
  {
    name: "left_hand_side_expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "call_expression" },
      { type: "rule_reference", name: "new_expression" },
    ] },
    lineNumber: 430,
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
            { type: "token_reference", name: "LBRACKET" },
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "RBRACKET" },
          ] },
          { type: "rule_reference", name: "template_literal" },
        ] } },
    ] },
    lineNumber: 432,
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
    lineNumber: 436,
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
    lineNumber: 439,
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
    lineNumber: 446,
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
    lineNumber: 448,
  },
  {
    name: "spread_element",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "ELLIPSIS" },
      { type: "rule_reference", name: "assignment_expression" },
    ] },
    lineNumber: 451,
  },
  {
    name: "arrow_function",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "arrow_parameters" },
      { type: "token_reference", name: "ARROW" },
      { type: "rule_reference", name: "concise_body" },
    ] },
    lineNumber: 453,
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
    lineNumber: 455,
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
    lineNumber: 459,
  },
  {
    name: "primary_expression",
    body: { type: "alternation", choices: [
      { type: "literal", value: "this" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "NUMBER" },
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
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 462,
  },
  {
    name: "array_literal",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "rule_reference", name: "element_list" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 479,
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
    lineNumber: 481,
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
    lineNumber: 485,
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
    lineNumber: 487,
  },
  {
    name: "object_spread_property",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "ELLIPSIS" },
      { type: "rule_reference", name: "assignment_expression" },
    ] },
    lineNumber: 493,
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
    lineNumber: 495,
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
    lineNumber: 500,
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
    lineNumber: 504,
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
    lineNumber: 513,
  },
  {
    name: "template_span",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "TEMPLATE_MIDDLE" },
    ] },
    lineNumber: 516,
  },
  {
    name: "type_annotation",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "type_expression" },
    ] },
    lineNumber: 541,
  },
  {
    name: "type_expression",
    body: { type: "rule_reference", name: "conditional_type" },
    lineNumber: 554,
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
    lineNumber: 567,
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
    lineNumber: 574,
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
    lineNumber: 581,
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
    lineNumber: 587,
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
    lineNumber: 593,
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
    lineNumber: 630,
  },
  {
    name: "literal_type",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "STRING" },
      { type: "literal", value: "true" },
      { type: "literal", value: "false" },
    ] },
    lineNumber: 638,
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
    lineNumber: 643,
  },
  {
    name: "type_arguments",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LESS_THAN" },
      { type: "rule_reference", name: "type_argument_list" },
      { type: "token_reference", name: "GREATER_THAN" },
    ] },
    lineNumber: 644,
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
    lineNumber: 645,
  },
  {
    name: "type_parameters",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LESS_THAN" },
      { type: "rule_reference", name: "type_parameter_list" },
      { type: "token_reference", name: "GREATER_THAN" },
    ] },
    lineNumber: 651,
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
    lineNumber: 652,
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
    lineNumber: 653,
  },
  {
    name: "object_type",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "type_member" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 673,
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
    lineNumber: 675,
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
    lineNumber: 684,
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
    lineNumber: 690,
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
    lineNumber: 692,
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
    lineNumber: 695,
  },
  {
    name: "construct_signature",
    body: { type: "sequence", elements: [
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
    lineNumber: 697,
  },
  {
    name: "tuple_type",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "rule_reference", name: "tuple_element_list" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 709,
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
    lineNumber: 710,
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
    lineNumber: 711,
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
    lineNumber: 725,
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
    lineNumber: 727,
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
    lineNumber: 745,
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
    lineNumber: 749,
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
    lineNumber: 750,
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
    lineNumber: 765,
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
    lineNumber: 783,
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
    lineNumber: 786,
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
    lineNumber: 790,
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
    lineNumber: 806,
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
    lineNumber: 808,
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
    lineNumber: 819,
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
    lineNumber: 834,
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
    lineNumber: 836,
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
    lineNumber: 838,
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
    lineNumber: 855,
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
    lineNumber: 857,
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
    lineNumber: 859,
  },
  {
    name: "export_assignment",
    body: { type: "sequence", elements: [
      { type: "literal", value: "export" },
      { type: "token_reference", name: "EQUALS" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 864,
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
    lineNumber: 866,
  },
  {
    name: "ambient_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "declare" },
      { type: "rule_reference", name: "ambient_declaration_body" },
    ] },
    lineNumber: 882,
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
    lineNumber: 884,
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
    lineNumber: 889,
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
    lineNumber: 893,
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
    lineNumber: 917,
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
    lineNumber: 920,
  },
  {
    name: "ts_class_modifiers",
    body: { type: "alternation", choices: [
      { type: "literal", value: "abstract" },
      { type: "literal", value: "declare" },
    ] },
    lineNumber: 923,
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
    lineNumber: 925,
  },
  {
    name: "ts_class_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "ts_class_element" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 928,
  },
  {
    name: "ts_class_element",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "ts_class_member" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 930,
  },
  {
    name: "ts_class_member",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "ts_constructor_declaration" },
      { type: "rule_reference", name: "ts_method_declaration" },
      { type: "rule_reference", name: "ts_property_declaration" },
      { type: "rule_reference", name: "ts_accessor_declaration" },
      { type: "rule_reference", name: "index_signature" },
    ] },
    lineNumber: 933,
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
    lineNumber: 939,
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
    lineNumber: 943,
  },
  {
    name: "ts_constructor_param",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "rule_reference", name: "accessibility_modifier" } },
      { type: "optional", element: { type: "literal", value: "override" } },
      { type: "optional", element: { type: "literal", value: "readonly" } },
      { type: "rule_reference", name: "typed_parameter" },
    ] },
    lineNumber: 946,
  },
  {
    name: "accessibility_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "private" },
      { type: "literal", value: "protected" },
    ] },
    lineNumber: 948,
  },
  {
    name: "ts_method_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "decorator" } },
      { type: "repetition", element: { type: "rule_reference", name: "ts_member_modifier" } },
      { type: "rule_reference", name: "ts_method_body" },
    ] },
    lineNumber: 950,
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
    lineNumber: 952,
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
    lineNumber: 955,
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
    lineNumber: 969,
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
    lineNumber: 974,
  },
  {
    name: "decorator",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "AT" },
      { type: "rule_reference", name: "decorator_expression" },
    ] },
    lineNumber: 991,
  },
  {
    name: "decorator_expression",
    body: { type: "rule_reference", name: "left_hand_side_expression" },
    lineNumber: 993,
  },
  {
    name: "ts_as_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "conditional_expression" },
      { type: "literal", value: "as" },
      { type: "rule_reference", name: "type_expression" },
    ] },
    lineNumber: 1010,
  },
  {
    name: "ts_satisfies_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "conditional_expression" },
      { type: "literal", value: "satisfies" },
      { type: "rule_reference", name: "type_expression" },
    ] },
    lineNumber: 1021,
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
    lineNumber: 1031,
  },
],
};
