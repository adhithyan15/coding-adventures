// AUTO-GENERATED FILE - DO NOT EDIT
// Source: ts2.0.grammar
// Regenerate with: grammar-tools compile-grammar ts2.0.grammar
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
    lineNumber: 61,
  },
  {
    name: "source_element",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "import_declaration" },
      { type: "rule_reference", name: "export_declaration" },
      { type: "rule_reference", name: "interface_declaration" },
      { type: "rule_reference", name: "type_alias_declaration" },
      { type: "rule_reference", name: "enum_declaration" },
      { type: "rule_reference", name: "namespace_declaration" },
      { type: "rule_reference", name: "ambient_declaration" },
      { type: "rule_reference", name: "function_declaration" },
      { type: "rule_reference", name: "generator_declaration" },
      { type: "rule_reference", name: "ts_class_declaration" },
      { type: "rule_reference", name: "lexical_declaration" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 66,
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
    lineNumber: 115,
  },
  {
    name: "typed_parameter",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "rule_reference", name: "accessibility_modifier" } },
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
    lineNumber: 118,
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
    lineNumber: 120,
  },
  {
    name: "lexical_declaration",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "literal", value: "let" },
          { type: "literal", value: "const" },
        ] } },
      { type: "rule_reference", name: "typed_binding_list" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 136,
  },
  {
    name: "typed_binding_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "typed_lexical_binding" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "typed_lexical_binding" },
        ] } },
    ] },
    lineNumber: 138,
  },
  {
    name: "typed_lexical_binding",
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
    lineNumber: 140,
  },
  {
    name: "lexical_declaration_no_semi",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "literal", value: "let" },
          { type: "literal", value: "const" },
        ] } },
      { type: "rule_reference", name: "typed_binding_list" },
    ] },
    lineNumber: 143,
  },
  {
    name: "variable_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "var" },
      { type: "rule_reference", name: "variable_declaration_list" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 150,
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
    lineNumber: 152,
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
    lineNumber: 154,
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
    lineNumber: 165,
  },
  {
    name: "ts_class_declaration",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "literal", value: "abstract" } },
      { type: "literal", value: "class" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "optional", element: { type: "rule_reference", name: "ts_class_heritage" } },
      { type: "rule_reference", name: "ts_class_body" },
    ] },
    lineNumber: 190,
  },
  {
    name: "ts_class_heritage",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "extends" },
          { type: "rule_reference", name: "type_reference" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "implements" },
          { type: "rule_reference", name: "type_reference" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "type_reference" },
            ] } },
        ] } },
    ] },
    lineNumber: 192,
  },
  {
    name: "ts_class_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "ts_class_element" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 194,
  },
  {
    name: "ts_class_element",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "ts_method_definition" },
      { type: "rule_reference", name: "ts_property_declaration" },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "index_signature" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 196,
  },
  {
    name: "ts_method_definition",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "rule_reference", name: "accessibility_modifier" } },
      { type: "optional", element: { type: "literal", value: "abstract" } },
      { type: "optional", element: { type: "literal", value: "static" } },
      { type: "optional", element: { type: "literal", value: "readonly" } },
      { type: "rule_reference", name: "ts_method_definition_body" },
    ] },
    lineNumber: 201,
  },
  {
    name: "accessibility_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "private" },
      { type: "literal", value: "protected" },
    ] },
    lineNumber: 203,
  },
  {
    name: "ts_method_definition_body",
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
    ] },
    lineNumber: 205,
  },
  {
    name: "ts_property_declaration",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "rule_reference", name: "accessibility_modifier" } },
      { type: "optional", element: { type: "literal", value: "static" } },
      { type: "optional", element: { type: "literal", value: "abstract" } },
      { type: "optional", element: { type: "literal", value: "readonly" } },
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
    lineNumber: 210,
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
        { type: "token_reference", name: "STRING" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "import" },
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "EQUALS" },
        { type: "literal", value: "require" },
        { type: "token_reference", name: "LPAREN" },
        { type: "token_reference", name: "STRING" },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 232,
  },
  {
    name: "import_clause",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "default_import" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "group", element: { type: "alternation", choices: [
                { type: "rule_reference", name: "named_imports" },
                { type: "rule_reference", name: "namespace_import" },
              ] } },
          ] } },
      ] },
      { type: "rule_reference", name: "named_imports" },
      { type: "rule_reference", name: "namespace_import" },
    ] },
    lineNumber: 239,
  },
  {
    name: "default_import",
    body: { type: "token_reference", name: "NAME" },
    lineNumber: 243,
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
    lineNumber: 245,
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
    lineNumber: 247,
  },
  {
    name: "namespace_import",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "STAR" },
      { type: "literal", value: "as" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 249,
  },
  {
    name: "from_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "from" },
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
            { type: "rule_reference", name: "ts_class_declaration" },
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
            { type: "rule_reference", name: "ts_class_declaration" },
            { type: "rule_reference", name: "lexical_declaration" },
            { type: "rule_reference", name: "variable_statement" },
            { type: "rule_reference", name: "interface_declaration" },
            { type: "rule_reference", name: "type_alias_declaration" },
            { type: "rule_reference", name: "enum_declaration" },
            { type: "rule_reference", name: "namespace_declaration" },
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
        { type: "token_reference", name: "EQUALS" },
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 253,
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
    lineNumber: 269,
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
    lineNumber: 271,
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
      { type: "rule_reference", name: "continue_statement" },
      { type: "rule_reference", name: "break_statement" },
      { type: "rule_reference", name: "return_statement" },
      { type: "rule_reference", name: "with_statement" },
      { type: "rule_reference", name: "switch_statement" },
      { type: "rule_reference", name: "labelled_statement" },
      { type: "rule_reference", name: "try_statement" },
      { type: "rule_reference", name: "throw_statement" },
      { type: "rule_reference", name: "debugger_statement" },
      { type: "rule_reference", name: "expression_statement" },
    ] },
    lineNumber: 280,
  },
  {
    name: "block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 300,
  },
  {
    name: "empty_statement",
    body: { type: "token_reference", name: "SEMICOLON" },
    lineNumber: 302,
  },
  {
    name: "expression_statement",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 304,
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
    lineNumber: 306,
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
    lineNumber: 308,
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
    lineNumber: 310,
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
          { type: "rule_reference", name: "lexical_declaration_no_semi" },
          { type: "optional", element: { type: "rule_reference", name: "expression" } },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 312,
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
            { type: "group", element: { type: "alternation", choices: [
                { type: "token_reference", name: "NAME" },
                { type: "rule_reference", name: "binding_pattern" },
              ] } },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "const" },
            { type: "group", element: { type: "alternation", choices: [
                { type: "token_reference", name: "NAME" },
                { type: "rule_reference", name: "binding_pattern" },
              ] } },
          ] },
          { type: "rule_reference", name: "left_hand_side_expression" },
        ] } },
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 320,
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
            { type: "group", element: { type: "alternation", choices: [
                { type: "token_reference", name: "NAME" },
                { type: "rule_reference", name: "binding_pattern" },
              ] } },
            { type: "optional", element: { type: "sequence", elements: [
                { type: "token_reference", name: "COLON" },
                { type: "rule_reference", name: "type_expression" },
              ] } },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "const" },
            { type: "group", element: { type: "alternation", choices: [
                { type: "token_reference", name: "NAME" },
                { type: "rule_reference", name: "binding_pattern" },
              ] } },
            { type: "optional", element: { type: "sequence", elements: [
                { type: "token_reference", name: "COLON" },
                { type: "rule_reference", name: "type_expression" },
              ] } },
          ] },
          { type: "rule_reference", name: "left_hand_side_expression" },
        ] } },
      { type: "literal", value: "of" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 329,
  },
  {
    name: "continue_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "continue" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 336,
  },
  {
    name: "break_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "break" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 338,
  },
  {
    name: "return_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "return" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 340,
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
    lineNumber: 342,
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
    lineNumber: 344,
  },
  {
    name: "case_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "case" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "COLON" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
    ] },
    lineNumber: 347,
  },
  {
    name: "default_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "default" },
      { type: "token_reference", name: "COLON" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
    ] },
    lineNumber: 349,
  },
  {
    name: "labelled_statement",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 351,
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
    lineNumber: 353,
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
    lineNumber: 355,
  },
  {
    name: "finally_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "finally" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 357,
  },
  {
    name: "throw_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "throw" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 359,
  },
  {
    name: "debugger_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "debugger" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 361,
  },
  {
    name: "binding_pattern",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "object_binding_pattern" },
      { type: "rule_reference", name: "array_binding_pattern" },
    ] },
    lineNumber: 375,
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
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 377,
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
    lineNumber: 379,
  },
  {
    name: "binding_element",
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
    lineNumber: 382,
  },
  {
    name: "array_binding_pattern",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "array_binding_element" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "array_binding_element" },
            ] } },
          { type: "optional", element: { type: "token_reference", name: "COMMA" } },
        ] } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 384,
  },
  {
    name: "array_binding_element",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "binding_element" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "ELLIPSIS" },
        { type: "group", element: { type: "alternation", choices: [
            { type: "token_reference", name: "NAME" },
            { type: "rule_reference", name: "binding_pattern" },
          ] } },
      ] },
    ] },
    lineNumber: 386,
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
    lineNumber: 396,
  },
  {
    name: "assignment_expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "arrow_function" },
      { type: "rule_reference", name: "ts_as_expression" },
      { type: "rule_reference", name: "ts_angle_bracket_assertion" },
      { type: "sequence", elements: [
        { type: "literal", value: "yield" },
        { type: "optional", element: { type: "token_reference", name: "STAR" } },
        { type: "rule_reference", name: "assignment_expression" },
      ] },
      { type: "rule_reference", name: "conditional_expression" },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "left_hand_side_expression" },
        { type: "rule_reference", name: "assignment_operator" },
        { type: "rule_reference", name: "assignment_expression" },
      ] },
    ] },
    lineNumber: 403,
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
    lineNumber: 410,
  },
  {
    name: "arrow_function",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "arrow_parameters" },
      { type: "token_reference", name: "ARROW" },
      { type: "rule_reference", name: "concise_body" },
    ] },
    lineNumber: 426,
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
    lineNumber: 428,
  },
  {
    name: "concise_body",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "LBRACE" },
        { type: "rule_reference", name: "function_body" },
        { type: "token_reference", name: "RBRACE" },
      ] },
      { type: "rule_reference", name: "assignment_expression" },
    ] },
    lineNumber: 432,
  },
  {
    name: "ts_as_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "ts_non_null_expression" },
      { type: "literal", value: "as" },
      { type: "rule_reference", name: "type_expression" },
    ] },
    lineNumber: 438,
  },
  {
    name: "ts_non_null_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "left_hand_side_expression" },
      { type: "token_reference", name: "BANG" },
    ] },
    lineNumber: 442,
  },
  {
    name: "ts_angle_bracket_assertion",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LESS_THAN" },
      { type: "rule_reference", name: "type_expression" },
      { type: "token_reference", name: "GREATER_THAN" },
      { type: "rule_reference", name: "assignment_expression" },
    ] },
    lineNumber: 445,
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
    lineNumber: 449,
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
    lineNumber: 454,
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
    lineNumber: 456,
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
    lineNumber: 460,
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
    lineNumber: 462,
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
    lineNumber: 464,
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
    lineNumber: 468,
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
    lineNumber: 474,
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
    lineNumber: 480,
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
    lineNumber: 485,
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
    lineNumber: 488,
  },
  {
    name: "unary_expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "postfix_expression" },
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
    lineNumber: 493,
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
    lineNumber: 506,
  },
  {
    name: "left_hand_side_expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "call_expression" },
      { type: "rule_reference", name: "new_expression" },
    ] },
    lineNumber: 510,
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
    lineNumber: 514,
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
    lineNumber: 518,
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
        { type: "rule_reference", name: "member_expression" },
        { type: "rule_reference", name: "arguments" },
      ] },
    ] },
    lineNumber: 521,
  },
  {
    name: "arguments",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 527,
  },
  {
    name: "argument_list",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "token_reference", name: "ELLIPSIS" },
            { type: "rule_reference", name: "assignment_expression" },
          ] },
          { type: "rule_reference", name: "assignment_expression" },
        ] } },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "group", element: { type: "alternation", choices: [
              { type: "sequence", elements: [
                { type: "token_reference", name: "ELLIPSIS" },
                { type: "rule_reference", name: "assignment_expression" },
              ] },
              { type: "rule_reference", name: "assignment_expression" },
            ] } },
        ] } },
    ] },
    lineNumber: 529,
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
      { type: "rule_reference", name: "template_literal" },
      { type: "rule_reference", name: "array_literal" },
      { type: "rule_reference", name: "object_literal" },
      { type: "rule_reference", name: "function_expression" },
      { type: "rule_reference", name: "generator_expression" },
      { type: "rule_reference", name: "ts_class_expression" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 534,
  },
  {
    name: "template_literal",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "TEMPLATE_NO_SUB" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "TEMPLATE_HEAD" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "TEMPLATE_MIDDLE" },
          ] } },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "TEMPLATE_TAIL" },
      ] },
    ] },
    lineNumber: 556,
  },
  {
    name: "array_literal",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "rule_reference", name: "array_element_list" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 561,
  },
  {
    name: "array_element_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "array_element" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "array_element" },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
    ] },
    lineNumber: 563,
  },
  {
    name: "array_element",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "ELLIPSIS" },
        { type: "rule_reference", name: "assignment_expression" },
      ] },
      { type: "rule_reference", name: "assignment_expression" },
    ] },
    lineNumber: 565,
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
    lineNumber: 570,
  },
  {
    name: "property_definition",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "property_name" },
        { type: "token_reference", name: "COLON" },
        { type: "rule_reference", name: "assignment_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "property_name" },
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
        { type: "token_reference", name: "ELLIPSIS" },
        { type: "rule_reference", name: "assignment_expression" },
      ] },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 572,
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
    lineNumber: 581,
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
    lineNumber: 586,
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
    lineNumber: 590,
  },
  {
    name: "ts_class_expression",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "literal", value: "abstract" } },
      { type: "literal", value: "class" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "optional", element: { type: "rule_reference", name: "ts_class_heritage" } },
      { type: "rule_reference", name: "ts_class_body" },
    ] },
    lineNumber: 596,
  },
  {
    name: "type_expression",
    body: { type: "rule_reference", name: "conditional_type" },
    lineNumber: 622,
  },
  {
    name: "conditional_type",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "union_type" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "extends" },
          { type: "rule_reference", name: "type_expression" },
          { type: "token_reference", name: "QUESTION" },
          { type: "rule_reference", name: "type_expression" },
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "type_expression" },
        ] } },
    ] },
    lineNumber: 650,
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
    lineNumber: 655,
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
    lineNumber: 660,
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
    lineNumber: 665,
  },
  {
    name: "primary_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "predefined_type" },
      { type: "rule_reference", name: "type_reference" },
      { type: "rule_reference", name: "literal_type" },
      { type: "rule_reference", name: "object_type" },
      { type: "rule_reference", name: "tuple_type" },
      { type: "rule_reference", name: "mapped_type" },
      { type: "rule_reference", name: "function_type" },
      { type: "rule_reference", name: "constructor_type" },
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
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "type_expression" },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 670,
  },
  {
    name: "predefined_type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "any" },
      { type: "literal", value: "string" },
      { type: "literal", value: "number" },
      { type: "literal", value: "boolean" },
      { type: "literal", value: "void" },
      { type: "literal", value: "object" },
      { type: "literal", value: "symbol" },
      { type: "literal", value: "undefined" },
      { type: "literal", value: "null" },
      { type: "literal", value: "never" },
    ] },
    lineNumber: 714,
  },
  {
    name: "literal_type",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "STRING" },
      { type: "literal", value: "true" },
      { type: "literal", value: "false" },
    ] },
    lineNumber: 719,
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
    lineNumber: 723,
  },
  {
    name: "type_arguments",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LESS_THAN" },
      { type: "rule_reference", name: "type_argument_list" },
      { type: "token_reference", name: "GREATER_THAN" },
    ] },
    lineNumber: 725,
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
    lineNumber: 726,
  },
  {
    name: "type_parameters",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LESS_THAN" },
      { type: "rule_reference", name: "type_parameter_list" },
      { type: "token_reference", name: "GREATER_THAN" },
    ] },
    lineNumber: 730,
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
    lineNumber: 731,
  },
  {
    name: "type_parameter",
    body: { type: "sequence", elements: [
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
    lineNumber: 732,
  },
  {
    name: "object_type",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "type_member_semicolon" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 736,
  },
  {
    name: "type_member_semicolon",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type_member" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 737,
  },
  {
    name: "type_member",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "construct_signature" },
      { type: "rule_reference", name: "call_signature" },
      { type: "rule_reference", name: "index_signature" },
      { type: "rule_reference", name: "method_signature" },
      { type: "rule_reference", name: "property_signature" },
    ] },
    lineNumber: 738,
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
    lineNumber: 744,
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
    lineNumber: 745,
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
    lineNumber: 746,
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
    lineNumber: 747,
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
    lineNumber: 748,
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
      { type: "token_reference", name: "SEMICOLON" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 790,
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
    lineNumber: 793,
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
    lineNumber: 798,
  },
  {
    name: "tuple_type",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "rule_reference", name: "tuple_element_list" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 804,
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
    lineNumber: 805,
  },
  {
    name: "tuple_element",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "token_reference", name: "ELLIPSIS" } },
      { type: "rule_reference", name: "type_expression" },
      { type: "optional", element: { type: "token_reference", name: "QUESTION" } },
    ] },
    lineNumber: 806,
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
    lineNumber: 810,
  },
  {
    name: "constructor_type",
    body: { type: "sequence", elements: [
      { type: "literal", value: "new" },
      { type: "optional", element: { type: "rule_reference", name: "type_parameters" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "typed_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "ARROW" },
      { type: "rule_reference", name: "type_expression" },
    ] },
    lineNumber: 814,
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
    lineNumber: 834,
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
    lineNumber: 835,
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
    lineNumber: 850,
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
    lineNumber: 860,
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
    lineNumber: 861,
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
    lineNumber: 862,
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
    lineNumber: 874,
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
    lineNumber: 875,
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
    lineNumber: 877,
  },
  {
    name: "export_assignment",
    body: { type: "sequence", elements: [
      { type: "literal", value: "export" },
      { type: "token_reference", name: "EQUALS" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 890,
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
          { type: "rule_reference", name: "generator_declaration" },
          { type: "rule_reference", name: "enum_declaration" },
          { type: "rule_reference", name: "lexical_declaration" },
          { type: "rule_reference", name: "variable_statement" },
        ] } },
    ] },
    lineNumber: 892,
  },
  {
    name: "ambient_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "declare" },
      { type: "rule_reference", name: "ambient_declaration_body" },
    ] },
    lineNumber: 912,
  },
  {
    name: "ambient_declaration_body",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "variable_statement" },
      { type: "rule_reference", name: "ambient_function_declaration" },
      { type: "rule_reference", name: "generator_declaration" },
      { type: "rule_reference", name: "ts_class_declaration" },
      { type: "rule_reference", name: "interface_declaration" },
      { type: "rule_reference", name: "type_alias_declaration" },
      { type: "rule_reference", name: "enum_declaration" },
      { type: "rule_reference", name: "namespace_declaration" },
      { type: "rule_reference", name: "ambient_module_declaration" },
      { type: "rule_reference", name: "ambient_global_augmentation" },
    ] },
    lineNumber: 913,
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
    lineNumber: 924,
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
    lineNumber: 928,
  },
  {
    name: "ambient_global_augmentation",
    body: { type: "sequence", elements: [
      { type: "literal", value: "global" },
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "namespace_element" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 932,
  },
  {
    name: "type_predicate",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "is" },
      { type: "rule_reference", name: "type_expression" },
    ] },
    lineNumber: 955,
  },
  {
    name: "type_annotation",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "type_expression" },
    ] },
    lineNumber: 961,
  },
],
};
