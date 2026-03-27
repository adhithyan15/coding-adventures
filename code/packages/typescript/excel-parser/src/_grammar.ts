// AUTO-GENERATED FILE — DO NOT EDIT
// Source: excel.grammar
// Regenerate with: grammar-tools compile-grammar excel.grammar
//
// This file embeds a ParserGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .grammar file at
// runtime.

import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const PARSER_GRAMMAR: ParserGrammar = {
  version: 1,
  rules: [
  {
    name: "formula",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "ws" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "ws" },
        ] } },
      { type: "rule_reference", name: "expression" },
      { type: "rule_reference", name: "ws" },
    ] },
    lineNumber: 15,
  },
  {
    name: "ws",
    body: { type: "repetition", element: { type: "token_reference", name: "SPACE" } },
    lineNumber: 17,
  },
  {
    name: "req_space",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "SPACE" },
      { type: "repetition", element: { type: "token_reference", name: "SPACE" } },
    ] },
    lineNumber: 18,
  },
  {
    name: "expression",
    body: { type: "rule_reference", name: "comparison_expr" },
    lineNumber: 20,
  },
  {
    name: "comparison_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "concat_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "ws" },
          { type: "rule_reference", name: "comparison_op" },
          { type: "rule_reference", name: "ws" },
          { type: "rule_reference", name: "concat_expr" },
        ] } },
    ] },
    lineNumber: 22,
  },
  {
    name: "comparison_op",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "EQUALS" },
      { type: "token_reference", name: "NOT_EQUALS" },
      { type: "token_reference", name: "LESS_THAN" },
      { type: "token_reference", name: "LESS_EQUALS" },
      { type: "token_reference", name: "GREATER_THAN" },
      { type: "token_reference", name: "GREATER_EQUALS" },
    ] },
    lineNumber: 23,
  },
  {
    name: "concat_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "additive_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "ws" },
          { type: "token_reference", name: "AMP" },
          { type: "rule_reference", name: "ws" },
          { type: "rule_reference", name: "additive_expr" },
        ] } },
    ] },
    lineNumber: 26,
  },
  {
    name: "additive_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "multiplicative_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "ws" },
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "PLUS" },
              { type: "token_reference", name: "MINUS" },
            ] } },
          { type: "rule_reference", name: "ws" },
          { type: "rule_reference", name: "multiplicative_expr" },
        ] } },
    ] },
    lineNumber: 27,
  },
  {
    name: "multiplicative_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "power_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "ws" },
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "STAR" },
              { type: "token_reference", name: "SLASH" },
            ] } },
          { type: "rule_reference", name: "ws" },
          { type: "rule_reference", name: "power_expr" },
        ] } },
    ] },
    lineNumber: 28,
  },
  {
    name: "power_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "unary_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "ws" },
          { type: "token_reference", name: "CARET" },
          { type: "rule_reference", name: "ws" },
          { type: "rule_reference", name: "unary_expr" },
        ] } },
    ] },
    lineNumber: 29,
  },
  {
    name: "unary_expr",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "prefix_op" },
          { type: "rule_reference", name: "ws" },
        ] } },
      { type: "rule_reference", name: "postfix_expr" },
    ] },
    lineNumber: 30,
  },
  {
    name: "prefix_op",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "PLUS" },
      { type: "token_reference", name: "MINUS" },
    ] },
    lineNumber: 31,
  },
  {
    name: "postfix_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "primary" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "ws" },
          { type: "token_reference", name: "PERCENT" },
        ] } },
    ] },
    lineNumber: 32,
  },
  {
    name: "primary",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "parenthesized_expression" },
      { type: "rule_reference", name: "constant" },
      { type: "rule_reference", name: "function_call" },
      { type: "rule_reference", name: "structure_reference" },
      { type: "rule_reference", name: "reference_expression" },
      { type: "rule_reference", name: "bang_reference" },
      { type: "rule_reference", name: "bang_name" },
      { type: "rule_reference", name: "name_reference" },
    ] },
    lineNumber: 34,
  },
  {
    name: "parenthesized_expression",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "ws" },
      { type: "rule_reference", name: "expression" },
      { type: "rule_reference", name: "ws" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 43,
  },
  {
    name: "constant",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "KEYWORD" },
      { type: "token_reference", name: "ERROR_CONSTANT" },
      { type: "rule_reference", name: "array_constant" },
    ] },
    lineNumber: 45,
  },
  {
    name: "array_constant",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "ws" },
      { type: "rule_reference", name: "array_row" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "ws" },
          { type: "token_reference", name: "SEMICOLON" },
          { type: "rule_reference", name: "ws" },
          { type: "rule_reference", name: "array_row" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "ws" },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
      { type: "rule_reference", name: "ws" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 47,
  },
  {
    name: "array_row",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "array_item" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "ws" },
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "ws" },
          { type: "rule_reference", name: "array_item" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "ws" },
          { type: "token_reference", name: "COMMA" },
        ] } },
    ] },
    lineNumber: 48,
  },
  {
    name: "array_item",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "KEYWORD" },
      { type: "token_reference", name: "ERROR_CONSTANT" },
    ] },
    lineNumber: 49,
  },
  {
    name: "function_call",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "function_name" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "ws" },
      { type: "optional", element: { type: "rule_reference", name: "function_argument_list" } },
      { type: "rule_reference", name: "ws" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 51,
  },
  {
    name: "function_name",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "FUNCTION_NAME" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 52,
  },
  {
    name: "function_argument_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "function_argument" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "ws" },
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "ws" },
          { type: "rule_reference", name: "function_argument" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "ws" },
          { type: "token_reference", name: "COMMA" },
        ] } },
    ] },
    lineNumber: 53,
  },
  {
    name: "function_argument",
    body: { type: "optional", element: { type: "rule_reference", name: "expression" } },
    lineNumber: 54,
  },
  {
    name: "reference_expression",
    body: { type: "rule_reference", name: "union_reference" },
    lineNumber: 56,
  },
  {
    name: "union_reference",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "intersection_reference" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "ws" },
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "ws" },
          { type: "rule_reference", name: "intersection_reference" },
        ] } },
    ] },
    lineNumber: 57,
  },
  {
    name: "intersection_reference",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "range_reference" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "req_space" },
          { type: "rule_reference", name: "range_reference" },
        ] } },
    ] },
    lineNumber: 58,
  },
  {
    name: "range_reference",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "reference_primary" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "ws" },
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "ws" },
          { type: "rule_reference", name: "reference_primary" },
        ] } },
    ] },
    lineNumber: 59,
  },
  {
    name: "reference_primary",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "parenthesized_reference" },
      { type: "rule_reference", name: "prefixed_reference" },
      { type: "rule_reference", name: "external_reference" },
      { type: "rule_reference", name: "structure_reference" },
      { type: "rule_reference", name: "a1_reference" },
      { type: "rule_reference", name: "bang_reference" },
      { type: "rule_reference", name: "bang_name" },
      { type: "rule_reference", name: "name_reference" },
    ] },
    lineNumber: 61,
  },
  {
    name: "parenthesized_reference",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "ws" },
      { type: "rule_reference", name: "reference_expression" },
      { type: "rule_reference", name: "ws" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 70,
  },
  {
    name: "prefixed_reference",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "REF_PREFIX" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "a1_reference" },
          { type: "rule_reference", name: "name_reference" },
          { type: "rule_reference", name: "structure_reference" },
        ] } },
    ] },
    lineNumber: 71,
  },
  {
    name: "external_reference",
    body: { type: "token_reference", name: "REF_PREFIX" },
    lineNumber: 72,
  },
  {
    name: "bang_reference",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "BANG" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "CELL" },
          { type: "token_reference", name: "COLUMN_REF" },
          { type: "token_reference", name: "ROW_REF" },
          { type: "token_reference", name: "NUMBER" },
        ] } },
    ] },
    lineNumber: 73,
  },
  {
    name: "bang_name",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "BANG" },
      { type: "rule_reference", name: "name_reference" },
    ] },
    lineNumber: 74,
  },
  {
    name: "name_reference",
    body: { type: "token_reference", name: "NAME" },
    lineNumber: 75,
  },
  {
    name: "column_reference",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "token_reference", name: "DOLLAR" } },
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "COLUMN_REF" },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 77,
  },
  {
    name: "row_reference",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "token_reference", name: "DOLLAR" } },
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "ROW_REF" },
          { type: "token_reference", name: "NUMBER" },
        ] } },
    ] },
    lineNumber: 78,
  },
  {
    name: "a1_reference",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "CELL" },
      { type: "rule_reference", name: "column_reference" },
      { type: "rule_reference", name: "row_reference" },
      { type: "token_reference", name: "COLUMN_REF" },
      { type: "token_reference", name: "ROW_REF" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "NUMBER" },
    ] },
    lineNumber: 80,
  },
  {
    name: "structure_reference",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "rule_reference", name: "table_name" } },
      { type: "rule_reference", name: "intra_table_reference" },
    ] },
    lineNumber: 82,
  },
  {
    name: "table_name",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "TABLE_NAME" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 83,
  },
  {
    name: "intra_table_reference",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "STRUCTURED_KEYWORD" },
      { type: "rule_reference", name: "structured_column_range" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LBRACKET" },
        { type: "rule_reference", name: "ws" },
        { type: "optional", element: { type: "rule_reference", name: "inner_structure_reference" } },
        { type: "rule_reference", name: "ws" },
        { type: "token_reference", name: "RBRACKET" },
      ] },
    ] },
    lineNumber: 84,
  },
  {
    name: "inner_structure_reference",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "structured_keyword_list" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "ws" },
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "ws" },
            { type: "rule_reference", name: "structured_column_range" },
          ] } },
      ] },
      { type: "rule_reference", name: "structured_column_range" },
    ] },
    lineNumber: 87,
  },
  {
    name: "structured_keyword_list",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "STRUCTURED_KEYWORD" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "ws" },
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "ws" },
          { type: "token_reference", name: "STRUCTURED_KEYWORD" },
        ] } },
    ] },
    lineNumber: 89,
  },
  {
    name: "structured_column_range",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "structured_column" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "ws" },
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "ws" },
          { type: "rule_reference", name: "structured_column" },
        ] } },
    ] },
    lineNumber: 90,
  },
  {
    name: "structured_column",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "STRUCTURED_COLUMN" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "AT" },
        { type: "token_reference", name: "STRUCTURED_COLUMN" },
      ] },
    ] },
    lineNumber: 91,
  },
],
};
