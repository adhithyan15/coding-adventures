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
      lineNumber: 30,
    },
    {
      name: "NUMBER",
      pattern: "-?[0-9]+\\.?[0-9]*[eE]?[-+]?[0-9]*",
      isRegex: true,
      lineNumber: 37,
    },
    {
      name: "TRUE",
      pattern: "true",
      isRegex: false,
      lineNumber: 41,
    },
    {
      name: "FALSE",
      pattern: "false",
      isRegex: false,
      lineNumber: 42,
    },
    {
      name: "NULL",
      pattern: "null",
      isRegex: false,
      lineNumber: 43,
    },
    {
      name: "LBRACE",
      pattern: "{",
      isRegex: false,
      lineNumber: 49,
    },
    {
      name: "RBRACE",
      pattern: "}",
      isRegex: false,
      lineNumber: 50,
    },
    {
      name: "LBRACKET",
      pattern: "[",
      isRegex: false,
      lineNumber: 51,
    },
    {
      name: "RBRACKET",
      pattern: "]",
      isRegex: false,
      lineNumber: 52,
    },
    {
      name: "COLON",
      pattern: ":",
      isRegex: false,
      lineNumber: 53,
    },
    {
      name: "COMMA",
      pattern: ",",
      isRegex: false,
      lineNumber: 54,
    },
  ],
  keywords: [],
  mode: undefined,
  escapeMode: "none",
  skipDefinitions: [
    {
      name: "WHITESPACE",
      pattern: "[ \\t\\r\\n]+",
      isRegex: true,
      lineNumber: 65,
    },
  ],
  reservedKeywords: [],
  layoutKeywords: [],
  contextKeywords: [],
  errorDefinitions: [],
  groups: {},
};
