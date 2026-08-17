// AUTO-GENERATED FILE - DO NOT EDIT
// Source: brainfuck.grammar
// Regenerate with: grammar-tools compile-grammar brainfuck.grammar
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
    body: { type: "repetition", element: { type: "rule_reference", name: "instruction" } },
    lineNumber: 15,
  },
  {
    name: "instruction",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "loop" },
      { type: "rule_reference", name: "command" },
    ] },
    lineNumber: 21,
  },
  {
    name: "loop",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LOOP_START" },
      { type: "repetition", element: { type: "rule_reference", name: "instruction" } },
      { type: "token_reference", name: "LOOP_END" },
    ] },
    lineNumber: 27,
  },
  {
    name: "command",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "RIGHT" },
      { type: "token_reference", name: "LEFT" },
      { type: "token_reference", name: "INC" },
      { type: "token_reference", name: "DEC" },
      { type: "token_reference", name: "OUTPUT" },
      { type: "token_reference", name: "INPUT" },
    ] },
    lineNumber: 32,
  },
],
};
