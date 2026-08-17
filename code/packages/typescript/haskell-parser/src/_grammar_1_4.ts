// AUTO-GENERATED FILE - DO NOT EDIT
// Source: haskell1.4.grammar
// Regenerate with: grammar-tools compile-grammar haskell1.4.grammar
//
// This file embeds a ParserGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .grammar file at
// runtime.

import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const PARSER_GRAMMAR: ParserGrammar = {
  version: 1,
  rules: [
  {
    name: "file",
    body: { type: "repetition", element: { type: "sequence", elements: [
        { type: "rule_reference", name: "declaration" },
        { type: "optional", element: { type: "rule_reference", name: "layout_sep" } },
      ] } },
    lineNumber: 10,
  },
  {
    name: "declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "module_decl" },
      { type: "rule_reference", name: "let_decl" },
      { type: "rule_reference", name: "do_decl" },
      { type: "rule_reference", name: "expr_decl" },
    ] },
    lineNumber: 11,
  },
  {
    name: "layout_open",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "VIRTUAL_LBRACE" },
      { type: "token_reference", name: "LBRACE" },
      { type: "literal", value: "{" },
    ] },
    lineNumber: 18,
  },
  {
    name: "layout_close",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "VIRTUAL_RBRACE" },
      { type: "token_reference", name: "RBRACE" },
      { type: "literal", value: "}" },
    ] },
    lineNumber: 19,
  },
  {
    name: "layout_sep",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "VIRTUAL_SEMICOLON" },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "token_reference", name: "NEWLINE" },
    ] },
    lineNumber: 20,
  },
  {
    name: "module_decl",
    body: { type: "sequence", elements: [
      { type: "literal", value: "module" },
      { type: "rule_reference", name: "module_name" },
      { type: "literal", value: "where" },
      { type: "rule_reference", name: "layout_open" },
      { type: "rule_reference", name: "module_body" },
      { type: "rule_reference", name: "layout_close" },
    ] },
    lineNumber: 22,
  },
  {
    name: "module_name",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "DOT" },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 23,
  },
  {
    name: "module_body",
    body: { type: "repetition", element: { type: "sequence", elements: [
        { type: "rule_reference", name: "declaration" },
        { type: "optional", element: { type: "rule_reference", name: "layout_sep" } },
      ] } },
    lineNumber: 24,
  },
  {
    name: "let_decl",
    body: { type: "sequence", elements: [
      { type: "literal", value: "let" },
      { type: "rule_reference", name: "layout_open" },
      { type: "rule_reference", name: "let_bindings" },
      { type: "rule_reference", name: "layout_close" },
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "expr_decl" },
    ] },
    lineNumber: 26,
  },
  {
    name: "let_bindings",
    body: { type: "repetition", element: { type: "sequence", elements: [
        { type: "rule_reference", name: "binding" },
        { type: "optional", element: { type: "rule_reference", name: "layout_sep" } },
      ] } },
    lineNumber: 27,
  },
  {
    name: "binding",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "expr_decl" },
    ] },
    lineNumber: 28,
  },
  {
    name: "do_decl",
    body: { type: "sequence", elements: [
      { type: "literal", value: "do" },
      { type: "rule_reference", name: "layout_open" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "expr_decl" },
          { type: "optional", element: { type: "rule_reference", name: "layout_sep" } },
        ] } },
      { type: "rule_reference", name: "layout_close" },
    ] },
    lineNumber: 30,
  },
  {
    name: "expr_decl",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "lambda_expr" },
      { type: "rule_reference", name: "app_expr" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "INTEGER" },
      { type: "token_reference", name: "FLOAT" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "CHARACTER" },
    ] },
    lineNumber: 32,
  },
  {
    name: "lambda_expr",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LAMBDA" },
      { type: "repetition", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "RARROW" },
      { type: "rule_reference", name: "expr_decl" },
    ] },
    lineNumber: 34,
  },
  {
    name: "app_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "atom_expr" },
      { type: "repetition", element: { type: "rule_reference", name: "atom_expr" } },
    ] },
    lineNumber: 35,
  },
  {
    name: "atom_expr",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "INTEGER" },
      { type: "token_reference", name: "FLOAT" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "CHARACTER" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expr_decl" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expr_list" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LBRACKET" },
        { type: "optional", element: { type: "rule_reference", name: "expr_list" } },
        { type: "token_reference", name: "RBRACKET" },
      ] },
    ] },
    lineNumber: 36,
  },
  {
    name: "expr_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expr_decl" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "expr_decl" },
        ] } },
    ] },
    lineNumber: 45,
  },
],
};
