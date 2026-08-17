// AUTO-GENERATED FILE - DO NOT EDIT
// Source: java1.0.grammar
// Regenerate with: grammar-tools compile-grammar java1.0.grammar
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
    lineNumber: 76,
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
    lineNumber: 78,
  },
  {
    name: "compilation_unit",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "rule_reference", name: "package_declaration" } },
      { type: "repetition", element: { type: "rule_reference", name: "import_declaration" } },
      { type: "repetition", element: { type: "rule_reference", name: "type_declaration" } },
    ] },
    lineNumber: 84,
  },
  {
    name: "package_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "package" },
      { type: "rule_reference", name: "qualified_name" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 103,
  },
  {
    name: "import_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "import" },
      { type: "rule_reference", name: "qualified_name" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "DOT" },
          { type: "token_reference", name: "STAR" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 120,
  },
  {
    name: "type_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "class_declaration" },
      { type: "rule_reference", name: "interface_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 132,
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
    lineNumber: 152,
  },
  {
    name: "class_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "class_modifier" } },
      { type: "literal", value: "class" },
      { type: "token_reference", name: "NAME" },
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
    lineNumber: 176,
  },
  {
    name: "class_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "final" },
    ] },
    lineNumber: 181,
  },
  {
    name: "class_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "class_body_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 191,
  },
  {
    name: "class_body_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "static_initializer" },
      { type: "rule_reference", name: "constructor_declaration" },
      { type: "rule_reference", name: "method_declaration" },
      { type: "rule_reference", name: "field_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 193,
  },
  {
    name: "interface_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "interface_modifier" } },
      { type: "literal", value: "interface" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "extends" },
          { type: "rule_reference", name: "interface_type_list" },
        ] } },
      { type: "rule_reference", name: "interface_body" },
    ] },
    lineNumber: 221,
  },
  {
    name: "interface_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "abstract" },
    ] },
    lineNumber: 225,
  },
  {
    name: "interface_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "interface_body_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 228,
  },
  {
    name: "interface_body_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "interface_method_declaration" },
      { type: "rule_reference", name: "interface_field_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 230,
  },
  {
    name: "interface_field_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "constant_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "variable_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 237,
  },
  {
    name: "constant_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "static" },
      { type: "literal", value: "final" },
    ] },
    lineNumber: 239,
  },
  {
    name: "interface_method_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "interface_method_modifier" } },
      { type: "rule_reference", name: "result_type" },
      { type: "rule_reference", name: "method_declarator" },
      { type: "optional", element: { type: "rule_reference", name: "throws_clause" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 246,
  },
  {
    name: "interface_method_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "abstract" },
    ] },
    lineNumber: 249,
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
    lineNumber: 255,
  },
  {
    name: "field_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "field_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "variable_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 275,
  },
  {
    name: "field_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "private" },
      { type: "literal", value: "static" },
      { type: "literal", value: "final" },
      { type: "literal", value: "transient" },
      { type: "literal", value: "volatile" },
    ] },
    lineNumber: 277,
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
    lineNumber: 294,
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
    lineNumber: 296,
  },
  {
    name: "variable_initializer",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "expression" },
      { type: "rule_reference", name: "array_initializer" },
    ] },
    lineNumber: 301,
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
    lineNumber: 311,
  },
  {
    name: "method_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "method_modifier" } },
      { type: "rule_reference", name: "result_type" },
      { type: "rule_reference", name: "method_declarator" },
      { type: "optional", element: { type: "rule_reference", name: "throws_clause" } },
      { type: "rule_reference", name: "method_body" },
    ] },
    lineNumber: 336,
  },
  {
    name: "method_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "private" },
      { type: "literal", value: "static" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "final" },
      { type: "literal", value: "synchronized" },
      { type: "literal", value: "native" },
    ] },
    lineNumber: 339,
  },
  {
    name: "result_type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "void" },
      { type: "rule_reference", name: "type" },
    ] },
    lineNumber: 350,
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
    lineNumber: 360,
  },
  {
    name: "formal_parameter_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "formal_parameter" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "formal_parameter" },
        ] } },
    ] },
    lineNumber: 367,
  },
  {
    name: "formal_parameter",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LBRACKET" },
          { type: "token_reference", name: "RBRACKET" },
        ] } },
    ] },
    lineNumber: 369,
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
    lineNumber: 377,
  },
  {
    name: "method_body",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "block" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 382,
  },
  {
    name: "constructor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "constructor_modifier" } },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "throws_clause" } },
      { type: "rule_reference", name: "constructor_body" },
    ] },
    lineNumber: 403,
  },
  {
    name: "constructor_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "private" },
    ] },
    lineNumber: 407,
  },
  {
    name: "constructor_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "rule_reference", name: "explicit_constructor_invocation" } },
      { type: "repetition", element: { type: "rule_reference", name: "block_statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 414,
  },
  {
    name: "explicit_constructor_invocation",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "this" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "super" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 419,
  },
  {
    name: "static_initializer",
    body: { type: "sequence", elements: [
      { type: "literal", value: "static" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 438,
  },
  {
    name: "type",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "primitive_type" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "LBRACKET" },
            { type: "token_reference", name: "RBRACKET" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "class_type" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "LBRACKET" },
            { type: "token_reference", name: "RBRACKET" },
          ] } },
      ] },
    ] },
    lineNumber: 459,
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
    lineNumber: 467,
  },
  {
    name: "class_type",
    body: { type: "rule_reference", name: "qualified_name" },
    lineNumber: 480,
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
      { type: "rule_reference", name: "switch_statement" },
      { type: "rule_reference", name: "try_statement" },
      { type: "rule_reference", name: "throw_statement" },
      { type: "rule_reference", name: "return_statement" },
      { type: "rule_reference", name: "break_statement" },
      { type: "rule_reference", name: "continue_statement" },
      { type: "rule_reference", name: "synchronized_statement" },
      { type: "rule_reference", name: "labelled_statement" },
    ] },
    lineNumber: 499,
  },
  {
    name: "block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "block_statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 527,
  },
  {
    name: "block_statement",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "var_declaration" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 533,
  },
  {
    name: "var_declaration",
    body: { type: "rule_reference", name: "local_variable_declaration_statement" },
    lineNumber: 548,
  },
  {
    name: "local_variable_declaration_statement",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "literal", value: "final" } },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "variable_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 550,
  },
  {
    name: "empty_statement",
    body: { type: "token_reference", name: "SEMICOLON" },
    lineNumber: 557,
  },
  {
    name: "expression_statement",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 571,
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
    lineNumber: 588,
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
    lineNumber: 599,
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
    lineNumber: 611,
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
    lineNumber: 632,
  },
  {
    name: "for_init",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "optional", element: { type: "literal", value: "final" } },
        { type: "rule_reference", name: "type" },
        { type: "rule_reference", name: "variable_declarators" },
      ] },
      { type: "optional", element: { type: "rule_reference", name: "expression_list" } },
    ] },
    lineNumber: 638,
  },
  {
    name: "for_update",
    body: { type: "rule_reference", name: "expression_list" },
    lineNumber: 644,
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
    lineNumber: 650,
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
    lineNumber: 668,
  },
  {
    name: "switch_block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "switch_block_statement_group" } },
      { type: "repetition", element: { type: "rule_reference", name: "switch_label" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 670,
  },
  {
    name: "switch_block_statement_group",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "switch_label" },
      { type: "repetition", element: { type: "rule_reference", name: "switch_label" } },
      { type: "repetition", element: { type: "rule_reference", name: "block_statement" } },
    ] },
    lineNumber: 672,
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
    lineNumber: 674,
  },
  {
    name: "try_statement",
    body: { type: "sequence", elements: [
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
    lineNumber: 696,
  },
  {
    name: "catch_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "catch" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "formal_parameter" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 699,
  },
  {
    name: "finally_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "finally" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 701,
  },
  {
    name: "throw_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "throw" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 712,
  },
  {
    name: "return_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "return" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 719,
  },
  {
    name: "break_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "break" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 734,
  },
  {
    name: "continue_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "continue" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 736,
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
    lineNumber: 755,
  },
  {
    name: "labelled_statement",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 769,
  },
  {
    name: "expression",
    body: { type: "rule_reference", name: "assignment_expression" },
    lineNumber: 820,
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
    lineNumber: 822,
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
    lineNumber: 825,
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
    lineNumber: 845,
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
    lineNumber: 855,
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
    lineNumber: 863,
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
    lineNumber: 871,
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
    lineNumber: 879,
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
    lineNumber: 887,
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
    lineNumber: 898,
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
    lineNumber: 913,
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
    lineNumber: 931,
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
    lineNumber: 942,
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
    lineNumber: 954,
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
    lineNumber: 976,
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
    lineNumber: 982,
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
            { type: "token_reference", name: "LBRACKET" },
            { type: "token_reference", name: "RBRACKET" },
          ] } },
        { type: "token_reference", name: "RPAREN" },
        { type: "rule_reference", name: "unary_expression_not_plus_minus" },
      ] },
    ] },
    lineNumber: 1006,
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
    lineNumber: 1017,
  },
  {
    name: "primary_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "primary" },
      { type: "repetition", element: { type: "rule_reference", name: "primary_suffix" } },
    ] },
    lineNumber: 1041,
  },
  {
    name: "primary_suffix",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "DOT" },
        { type: "token_reference", name: "NAME" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "DOT" },
        { type: "literal", value: "class" },
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
    lineNumber: 1043,
  },
  {
    name: "primary",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "literal" },
      { type: "literal", value: "this" },
      { type: "sequence", elements: [
        { type: "literal", value: "super" },
        { type: "token_reference", name: "DOT" },
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
        { type: "rule_reference", name: "class_type" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
        { type: "token_reference", name: "RPAREN" },
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
    lineNumber: 1051,
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
    lineNumber: 1065,
  },
  {
    name: "array_creation_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "primitive_type" },
      { type: "rule_reference", name: "class_type" },
    ] },
    lineNumber: 1079,
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
    lineNumber: 1084,
  },
  {
    name: "literal",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "CHARACTER" },
      { type: "token_reference", name: "STRING" },
      { type: "literal", value: "true" },
      { type: "literal", value: "false" },
      { type: "literal", value: "null" },
    ] },
    lineNumber: 1105,
  },
],
};
