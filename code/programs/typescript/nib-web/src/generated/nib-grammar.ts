// AUTO-GENERATED FILE — DO NOT EDIT
// Source: nib.grammar
// Regenerate with: grammar-tools compile-grammar nib.grammar
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
    body: { type: "repetition", element: { type: "rule_reference", name: "top_decl" } },
    lineNumber: 42,
  },
  {
    name: "top_decl",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "const_decl" },
      { type: "rule_reference", name: "static_decl" },
      { type: "rule_reference", name: "fn_decl" },
    ] },
    lineNumber: 47,
  },
  {
    name: "const_decl",
    body: { type: "sequence", elements: [
      { type: "literal", value: "const" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "EQ" },
      { type: "rule_reference", name: "expr" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 60,
  },
  {
    name: "static_decl",
    body: { type: "sequence", elements: [
      { type: "literal", value: "static" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "EQ" },
      { type: "rule_reference", name: "expr" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 66,
  },
  {
    name: "fn_decl",
    body: { type: "sequence", elements: [
      { type: "literal", value: "fn" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "param_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "ARROW" },
          { type: "rule_reference", name: "type" },
        ] } },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 77,
  },
  {
    name: "param_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "param" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "param" },
        ] } },
    ] },
    lineNumber: 80,
  },
  {
    name: "param",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "type" },
    ] },
    lineNumber: 87,
  },
  {
    name: "block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "stmt" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 98,
  },
  {
    name: "stmt",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "let_stmt" },
      { type: "rule_reference", name: "assign_stmt" },
      { type: "rule_reference", name: "return_stmt" },
      { type: "rule_reference", name: "for_stmt" },
      { type: "rule_reference", name: "if_stmt" },
      { type: "rule_reference", name: "expr_stmt" },
    ] },
    lineNumber: 109,
  },
  {
    name: "let_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "let" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "EQ" },
      { type: "rule_reference", name: "expr" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 121,
  },
  {
    name: "assign_stmt",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "EQ" },
      { type: "rule_reference", name: "expr" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 126,
  },
  {
    name: "return_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "return" },
      { type: "rule_reference", name: "expr" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 131,
  },
  {
    name: "for_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "for" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "type" },
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "expr" },
      { type: "token_reference", name: "RANGE" },
      { type: "rule_reference", name: "expr" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 154,
  },
  {
    name: "if_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "if" },
      { type: "rule_reference", name: "expr" },
      { type: "rule_reference", name: "block" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "else" },
          { type: "rule_reference", name: "block" },
        ] } },
    ] },
    lineNumber: 160,
  },
  {
    name: "expr_stmt",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expr" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 167,
  },
  {
    name: "type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "u4" },
      { type: "literal", value: "u8" },
      { type: "literal", value: "bcd" },
      { type: "literal", value: "bool" },
    ] },
    lineNumber: 202,
  },
  {
    name: "expr",
    body: { type: "rule_reference", name: "or_expr" },
    lineNumber: 242,
  },
  {
    name: "or_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "and_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LOR" },
          { type: "rule_reference", name: "and_expr" },
        ] } },
    ] },
    lineNumber: 248,
  },
  {
    name: "and_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "eq_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LAND" },
          { type: "rule_reference", name: "eq_expr" },
        ] } },
    ] },
    lineNumber: 252,
  },
  {
    name: "eq_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "cmp_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "EQ_EQ" },
              { type: "token_reference", name: "NEQ" },
            ] } },
          { type: "rule_reference", name: "cmp_expr" },
        ] } },
    ] },
    lineNumber: 257,
  },
  {
    name: "cmp_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "add_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "LT" },
              { type: "token_reference", name: "GT" },
              { type: "token_reference", name: "LEQ" },
              { type: "token_reference", name: "GEQ" },
            ] } },
          { type: "rule_reference", name: "add_expr" },
        ] } },
    ] },
    lineNumber: 263,
  },
  {
    name: "add_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "bitwise_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "PLUS" },
              { type: "token_reference", name: "MINUS" },
              { type: "token_reference", name: "WRAP_ADD" },
              { type: "token_reference", name: "SAT_ADD" },
            ] } },
          { type: "rule_reference", name: "bitwise_expr" },
        ] } },
    ] },
    lineNumber: 276,
  },
  {
    name: "bitwise_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "unary_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "AMP" },
              { type: "token_reference", name: "PIPE" },
              { type: "token_reference", name: "CARET" },
            ] } },
          { type: "rule_reference", name: "unary_expr" },
        ] } },
    ] },
    lineNumber: 282,
  },
  {
    name: "unary_expr",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "group", element: { type: "alternation", choices: [
            { type: "token_reference", name: "BANG" },
            { type: "token_reference", name: "TILDE" },
          ] } },
        { type: "rule_reference", name: "unary_expr" },
      ] },
      { type: "rule_reference", name: "primary" },
    ] },
    lineNumber: 290,
  },
  {
    name: "primary",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "INT_LIT" },
      { type: "token_reference", name: "HEX_LIT" },
      { type: "literal", value: "true" },
      { type: "literal", value: "false" },
      { type: "rule_reference", name: "call_expr" },
      { type: "token_reference", name: "NAME" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expr" },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 298,
  },
  {
    name: "call_expr",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "arg_list" } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 321,
  },
  {
    name: "arg_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "expr" },
        ] } },
    ] },
    lineNumber: 324,
  },
],
};
