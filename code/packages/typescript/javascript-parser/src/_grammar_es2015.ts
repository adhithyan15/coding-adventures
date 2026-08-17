// AUTO-GENERATED FILE - DO NOT EDIT
// Source: es2015.grammar
// Regenerate with: grammar-tools compile-grammar es2015.grammar
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
    lineNumber: 31,
  },
  {
    name: "source_element",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "import_declaration" },
      { type: "rule_reference", name: "export_declaration" },
      { type: "rule_reference", name: "function_declaration" },
      { type: "rule_reference", name: "generator_declaration" },
      { type: "rule_reference", name: "class_declaration" },
      { type: "rule_reference", name: "lexical_declaration" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 33,
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
    lineNumber: 47,
  },
  {
    name: "function_body",
    body: { type: "repetition", element: { type: "rule_reference", name: "source_element" } },
    lineNumber: 50,
  },
  {
    name: "formal_parameters",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "formal_parameter" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "formal_parameter" },
          ] } },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "rest_parameter" },
          ] } },
      ] },
      { type: "rule_reference", name: "rest_parameter" },
    ] },
    lineNumber: 59,
  },
  {
    name: "formal_parameter",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "binding_element" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "assignment_expression" },
        ] } },
    ] },
    lineNumber: 62,
  },
  {
    name: "rest_parameter",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "ELLIPSIS" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "NAME" },
          { type: "rule_reference", name: "binding_pattern" },
        ] } },
    ] },
    lineNumber: 64,
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
    lineNumber: 72,
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
    lineNumber: 74,
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
    lineNumber: 76,
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
    lineNumber: 87,
  },
  {
    name: "class_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "class" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "class_heritage" } },
      { type: "rule_reference", name: "class_body" },
    ] },
    lineNumber: 104,
  },
  {
    name: "class_heritage",
    body: { type: "sequence", elements: [
      { type: "literal", value: "extends" },
      { type: "rule_reference", name: "left_hand_side_expression" },
    ] },
    lineNumber: 106,
  },
  {
    name: "class_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "class_element" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 108,
  },
  {
    name: "class_element",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "optional", element: { type: "literal", value: "static" } },
        { type: "rule_reference", name: "method_definition" },
      ] },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 110,
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
    lineNumber: 113,
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
    ] },
    lineNumber: 143,
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
    lineNumber: 146,
  },
  {
    name: "default_import",
    body: { type: "token_reference", name: "NAME" },
    lineNumber: 150,
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
    lineNumber: 152,
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
    lineNumber: 154,
  },
  {
    name: "namespace_import",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "STAR" },
      { type: "literal", value: "as" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 156,
  },
  {
    name: "from_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "from" },
      { type: "token_reference", name: "STRING" },
    ] },
    lineNumber: 158,
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
    ] },
    lineNumber: 160,
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
    lineNumber: 171,
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
    lineNumber: 173,
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
    lineNumber: 179,
  },
  {
    name: "block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 199,
  },
  {
    name: "variable_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "var" },
      { type: "rule_reference", name: "variable_declaration_list" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 201,
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
    lineNumber: 203,
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
    lineNumber: 205,
  },
  {
    name: "empty_statement",
    body: { type: "token_reference", name: "SEMICOLON" },
    lineNumber: 207,
  },
  {
    name: "expression_statement",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 209,
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
    lineNumber: 211,
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
    lineNumber: 213,
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
    lineNumber: 215,
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
    lineNumber: 217,
  },
  {
    name: "lexical_declaration_no_semi",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "literal", value: "let" },
          { type: "literal", value: "const" },
        ] } },
      { type: "rule_reference", name: "binding_list" },
    ] },
    lineNumber: 226,
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
    lineNumber: 228,
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
      { type: "literal", value: "of" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 243,
  },
  {
    name: "continue_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "continue" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 250,
  },
  {
    name: "break_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "break" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 252,
  },
  {
    name: "return_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "return" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 254,
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
    lineNumber: 256,
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
    lineNumber: 258,
  },
  {
    name: "case_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "case" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "COLON" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
    ] },
    lineNumber: 261,
  },
  {
    name: "default_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "default" },
      { type: "token_reference", name: "COLON" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
    ] },
    lineNumber: 263,
  },
  {
    name: "labelled_statement",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 265,
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
    lineNumber: 267,
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
    lineNumber: 269,
  },
  {
    name: "finally_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "finally" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 271,
  },
  {
    name: "throw_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "throw" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 273,
  },
  {
    name: "debugger_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "debugger" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 275,
  },
  {
    name: "binding_pattern",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "object_binding_pattern" },
      { type: "rule_reference", name: "array_binding_pattern" },
    ] },
    lineNumber: 293,
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
    lineNumber: 295,
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
    lineNumber: 297,
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
    lineNumber: 300,
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
    lineNumber: 302,
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
    lineNumber: 304,
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
    lineNumber: 314,
  },
  {
    name: "assignment_expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "arrow_function" },
      { type: "sequence", elements: [
        { type: "literal", value: "yield" },
        { type: "optional", element: { type: "token_reference", name: "STAR" } },
        { type: "rule_reference", name: "assignment_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "left_hand_side_expression" },
        { type: "rule_reference", name: "assignment_operator" },
        { type: "rule_reference", name: "assignment_expression" },
      ] },
      { type: "rule_reference", name: "conditional_expression" },
    ] },
    lineNumber: 319,
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
    lineNumber: 324,
  },
  {
    name: "arrow_function",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "arrow_parameters" },
      { type: "token_reference", name: "ARROW" },
      { type: "rule_reference", name: "concise_body" },
    ] },
    lineNumber: 338,
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
    lineNumber: 340,
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
    lineNumber: 343,
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
    lineNumber: 348,
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
    lineNumber: 353,
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
    lineNumber: 355,
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
    lineNumber: 359,
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
    lineNumber: 361,
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
    lineNumber: 363,
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
    lineNumber: 367,
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
    lineNumber: 373,
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
    lineNumber: 379,
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
    lineNumber: 384,
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
    lineNumber: 387,
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
    lineNumber: 392,
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
    lineNumber: 405,
  },
  {
    name: "left_hand_side_expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "call_expression" },
      { type: "rule_reference", name: "new_expression" },
    ] },
    lineNumber: 409,
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
    lineNumber: 411,
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
    lineNumber: 415,
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
    lineNumber: 418,
  },
  {
    name: "arguments",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 424,
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
    lineNumber: 426,
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
      { type: "rule_reference", name: "class_expression" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 432,
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
    lineNumber: 460,
  },
  {
    name: "array_literal",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "rule_reference", name: "array_element_list" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 465,
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
    lineNumber: 467,
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
    lineNumber: 469,
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
    lineNumber: 476,
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
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 478,
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
    lineNumber: 487,
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
    lineNumber: 492,
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
    lineNumber: 495,
  },
  {
    name: "class_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "class" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "optional", element: { type: "rule_reference", name: "class_heritage" } },
      { type: "rule_reference", name: "class_body" },
    ] },
    lineNumber: 500,
  },
],
};
