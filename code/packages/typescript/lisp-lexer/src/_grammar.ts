// AUTO-GENERATED FILE - DO NOT EDIT
// Source: lisp.tokens
// Regenerate with: grammar-tools compile-tokens lisp.tokens
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
      name: "NUMBER",
      pattern: "-?[0-9]+",
      isRegex: true,
      lineNumber: 11,
    },
    {
      name: "SYMBOL",
      pattern: "[a-zA-Z_+\\-*\\/=<>!?&][a-zA-Z0-9_+\\-*\\/=<>!?&]*",
      isRegex: true,
      lineNumber: 12,
    },
    {
      name: "STRING",
      pattern: "\"([^\"\\\\]|\\\\.)*\"",
      isRegex: true,
      lineNumber: 13,
    },
    {
      name: "LPAREN",
      pattern: "(",
      isRegex: false,
      lineNumber: 14,
    },
    {
      name: "RPAREN",
      pattern: ")",
      isRegex: false,
      lineNumber: 15,
    },
    {
      name: "QUOTE",
      pattern: "'",
      isRegex: false,
      lineNumber: 16,
    },
    {
      name: "DOT",
      pattern: ".",
      isRegex: false,
      lineNumber: 17,
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
      lineNumber: 8,
    },
    {
      name: "COMMENT",
      pattern: ";[^\\n]*",
      isRegex: true,
      lineNumber: 9,
    },
  ],
  reservedKeywords: [],
  layoutKeywords: [],
  contextKeywords: [],
  errorDefinitions: [],
  groups: {},
  softKeywords: [],
};
