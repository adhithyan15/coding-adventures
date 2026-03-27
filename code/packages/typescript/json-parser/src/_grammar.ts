// AUTO-GENERATED FILE — DO NOT EDIT
// Source: json.grammar
// Regenerate with: grammar-tools compile-grammar json.grammar
//
// This file embeds a ParserGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .grammar file at
// runtime.

import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const PARSER_GRAMMAR: ParserGrammar = {
  version: 1,
  rules: [
  {
    name: "value",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "object" },
      { type: "rule_reference", name: "array" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "TRUE" },
      { type: "token_reference", name: "FALSE" },
      { type: "token_reference", name: "NULL" },
    ] },
    lineNumber: 28,
  },
  {
    name: "object",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "pair" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "pair" },
            ] } },
        ] } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 34,
  },
  {
    name: "pair",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "value" },
    ] },
    lineNumber: 38,
  },
  {
    name: "array",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "value" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "value" },
            ] } },
        ] } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 42,
  },
],
};
