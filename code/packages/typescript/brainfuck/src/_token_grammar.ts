// AUTO-GENERATED FILE - DO NOT EDIT
// Source: brainfuck.tokens
// Regenerate with: grammar-tools compile-tokens brainfuck.tokens
//
// This file embeds a TokenGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .tokens file at
// runtime.

import type { TokenGrammar } from "@coding-adventures/grammar-tools";

export const TOKEN_GRAMMAR: TokenGrammar = {
  version: 1,
  caseInsensitive: false,
  caseSensitive: true,
  definitions: [
    {
      name: "RIGHT",
      pattern: ">",
      isRegex: false,
      lineNumber: 23,
    },
    {
      name: "LEFT",
      pattern: "<",
      isRegex: false,
      lineNumber: 24,
    },
    {
      name: "INC",
      pattern: "+",
      isRegex: false,
      lineNumber: 29,
    },
    {
      name: "DEC",
      pattern: "-",
      isRegex: false,
      lineNumber: 30,
    },
    {
      name: "OUTPUT",
      pattern: ".",
      isRegex: false,
      lineNumber: 35,
    },
    {
      name: "INPUT",
      pattern: ",",
      isRegex: false,
      lineNumber: 36,
    },
    {
      name: "LOOP_START",
      pattern: "[",
      isRegex: false,
      lineNumber: 41,
    },
    {
      name: "LOOP_END",
      pattern: "]",
      isRegex: false,
      lineNumber: 42,
    },
  ],
  keywords: [],
  mode: undefined,
  escapeMode: undefined,
  skipDefinitions: [
    {
      name: "WHITESPACE",
      pattern: "[ \\t\\r\\n]+",
      isRegex: true,
      lineNumber: 65,
    },
    {
      name: "COMMENT",
      pattern: "[^><+\\-.,\\[\\] \\t\\r\\n]+",
      isRegex: true,
      lineNumber: 66,
    },
  ],
  reservedKeywords: [],
  layoutKeywords: [],
  contextKeywords: [],
  errorDefinitions: [],
  groups: {},
  softKeywords: [],
};
