// AUTO-GENERATED FILE - DO NOT EDIT
// Source: es1.grammar
// Regenerate with: grammar-tools compile-grammar es1.grammar
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
    lineNumber: 40,
  },
  {
    name: "source_element",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "function_declaration" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 42,
  },
  {
    name: "function_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "function" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 52,
  },
  {
    name: "formal_parameter_list",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 55,
  },
  {
    name: "function_body",
    body: { type: "repetition", element: { type: "rule_reference", name: "source_element" } },
    lineNumber: 57,
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
      { type: "rule_reference", name: "continue_statement" },
      { type: "rule_reference", name: "break_statement" },
      { type: "rule_reference", name: "return_statement" },
      { type: "rule_reference", name: "with_statement" },
      { type: "rule_reference", name: "switch_statement" },
      { type: "rule_reference", name: "labelled_statement" },
      { type: "rule_reference", name: "expression_statement" },
    ] },
    lineNumber: 70,
  },
  {
    name: "block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 89,
  },
  {
    name: "variable_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "var" },
      { type: "rule_reference", name: "variable_declaration_list" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 94,
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
    lineNumber: 96,
  },
  {
    name: "variable_declaration",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "assignment_expression" },
        ] } },
    ] },
    lineNumber: 98,
  },
  {
    name: "empty_statement",
    body: { type: "token_reference", name: "SEMICOLON" },
    lineNumber: 102,
  },
  {
    name: "expression_statement",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 107,
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
    lineNumber: 112,
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
    lineNumber: 116,
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
    lineNumber: 120,
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
          { type: "optional", element: { type: "rule_reference", name: "expression" } },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 124,
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
          { type: "rule_reference", name: "left_hand_side_expression" },
        ] } },
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 132,
  },
  {
    name: "continue_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "continue" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 138,
  },
  {
    name: "break_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "break" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 140,
  },
  {
    name: "return_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "return" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 144,
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
    lineNumber: 149,
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
    lineNumber: 153,
  },
  {
    name: "case_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "case" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "COLON" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
    ] },
    lineNumber: 156,
  },
  {
    name: "default_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "default" },
      { type: "token_reference", name: "COLON" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
    ] },
    lineNumber: 158,
  },
  {
    name: "labelled_statement",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 162,
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
    lineNumber: 180,
  },
  {
    name: "assignment_expression",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "left_hand_side_expression" },
        { type: "rule_reference", name: "assignment_operator" },
        { type: "rule_reference", name: "assignment_expression" },
      ] },
      { type: "rule_reference", name: "conditional_expression" },
    ] },
    lineNumber: 184,
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
    lineNumber: 187,
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
    lineNumber: 194,
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
    lineNumber: 199,
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
    lineNumber: 201,
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
    lineNumber: 205,
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
    lineNumber: 207,
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
    lineNumber: 209,
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
    lineNumber: 213,
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
              { type: "literal", value: "in" },
            ] } },
          { type: "rule_reference", name: "shift_expression" },
        ] } },
    ] },
    lineNumber: 219,
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
    lineNumber: 225,
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
    lineNumber: 230,
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
    lineNumber: 235,
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
    lineNumber: 240,
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
    lineNumber: 253,
  },
  {
    name: "left_hand_side_expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "call_expression" },
      { type: "rule_reference", name: "new_expression" },
    ] },
    lineNumber: 258,
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
        ] } },
    ] },
    lineNumber: 263,
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
    lineNumber: 268,
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
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "new" },
        { type: "rule_reference", name: "member_expression" },
        { type: "rule_reference", name: "arguments" },
      ] },
    ] },
    lineNumber: 273,
  },
  {
    name: "arguments",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "assignment_expression" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "assignment_expression" },
            ] } },
        ] } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 278,
  },
  {
    name: "primary_expression",
    body: { type: "alternation", choices: [
      { type: "literal", value: "this" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "STRING" },
      { type: "literal", value: "true" },
      { type: "literal", value: "false" },
      { type: "literal", value: "null" },
      { type: "rule_reference", name: "array_literal" },
      { type: "rule_reference", name: "object_literal" },
      { type: "rule_reference", name: "function_expression" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 282,
  },
  {
    name: "array_literal",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "rule_reference", name: "element_list" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 296,
  },
  {
    name: "element_list",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "rule_reference", name: "assignment_expression" } },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "optional", element: { type: "rule_reference", name: "assignment_expression" } },
        ] } },
    ] },
    lineNumber: 298,
  },
  {
    name: "object_literal",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "property_assignment" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "property_assignment" },
            ] } },
          { type: "optional", element: { type: "token_reference", name: "COMMA" } },
        ] } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 303,
  },
  {
    name: "property_assignment",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "property_name" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "assignment_expression" },
    ] },
    lineNumber: 305,
  },
  {
    name: "property_name",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "NUMBER" },
    ] },
    lineNumber: 307,
  },
  {
    name: "function_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "function" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "function_body" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 311,
  },
],
};
