// AUTO-GENERATED FILE - DO NOT EDIT
// Source: lisp.grammar
// Regenerate with: grammar-tools compile-grammar lisp.grammar
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
    body: { type: "repetition", element: { type: "rule_reference", name: "sexpr" } },
    lineNumber: 2,
  },
  {
    name: "sexpr",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "atom" },
      { type: "rule_reference", name: "list" },
      { type: "rule_reference", name: "quoted" },
    ] },
    lineNumber: 3,
  },
  {
    name: "atom",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "SYMBOL" },
      { type: "token_reference", name: "STRING" },
    ] },
    lineNumber: 4,
  },
  {
    name: "list",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "list_body" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 5,
  },
  {
    name: "list_body",
    body: { type: "optional", element: { type: "sequence", elements: [
        { type: "rule_reference", name: "sexpr" },
        { type: "repetition", element: { type: "rule_reference", name: "sexpr" } },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "DOT" },
            { type: "rule_reference", name: "sexpr" },
          ] } },
      ] } },
    lineNumber: 6,
  },
  {
    name: "quoted",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "QUOTE" },
      { type: "rule_reference", name: "sexpr" },
    ] },
    lineNumber: 7,
  },
],
};
