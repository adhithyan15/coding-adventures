// AUTO-GENERATED FILE — DO NOT EDIT
// Source: python.grammar
// Regenerate with: grammar-tools compile-grammar python.grammar
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
    body: { type: "repetition", element: { type: "alternation", choices: [
        { type: "token_reference", name: "NEWLINE" },
        { type: "rule_reference", name: "statement" },
      ] } },
    lineNumber: 17,
  },
  {
    name: "statement",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "assignment" },
          { type: "rule_reference", name: "expression_stmt" },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "NEWLINE" } },
    ] },
    lineNumber: 18,
  },
  {
    name: "assignment",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 19,
  },
  {
    name: "expression_stmt",
    body: { type: "rule_reference", name: "expression" },
    lineNumber: 20,
  },
  {
    name: "expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "term" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "PLUS" },
              { type: "token_reference", name: "MINUS" },
            ] } },
          { type: "rule_reference", name: "term" },
        ] } },
    ] },
    lineNumber: 21,
  },
  {
    name: "term",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "factor" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "STAR" },
              { type: "token_reference", name: "SLASH" },
            ] } },
          { type: "rule_reference", name: "factor" },
        ] } },
    ] },
    lineNumber: 22,
  },
  {
    name: "factor",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "INT" },
      { type: "token_reference", name: "FLOAT" },
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "NAME" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 23,
  },
],
};
