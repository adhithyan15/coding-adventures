// AUTO-GENERATED FILE — DO NOT EDIT
// Source: json.tokens
// Regenerate with: grammar-tools compile-tokens json.tokens
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
      name: "STRING",
      pattern: "\"([^\"\\\\]|\\\\[\"\\\\\\x2fbfnrt]|\\\\u[0-9a-fA-F]{4})*\"",
      isRegex: true,
      lineNumber: 25,
    },
    {
      name: "NUMBER",
      pattern: "-?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?",
      isRegex: true,
      lineNumber: 31,
    },
    {
      name: "TRUE",
      pattern: "true",
      isRegex: false,
      lineNumber: 35,
    },
    {
      name: "FALSE",
      pattern: "false",
      isRegex: false,
      lineNumber: 36,
    },
    {
      name: "NULL",
      pattern: "null",
      isRegex: false,
      lineNumber: 37,
    },
    {
      name: "LBRACE",
      pattern: "{",
      isRegex: false,
      lineNumber: 43,
    },
    {
      name: "RBRACE",
      pattern: "}",
      isRegex: false,
      lineNumber: 44,
    },
    {
      name: "LBRACKET",
      pattern: "[",
      isRegex: false,
      lineNumber: 45,
    },
    {
      name: "RBRACKET",
      pattern: "]",
      isRegex: false,
      lineNumber: 46,
    },
    {
      name: "COLON",
      pattern: ":",
      isRegex: false,
      lineNumber: 47,
    },
    {
      name: "COMMA",
      pattern: ",",
      isRegex: false,
      lineNumber: 48,
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
      lineNumber: 59,
    },
  ],
  reservedKeywords: [],
  errorDefinitions: [],
  groups: {},
};
