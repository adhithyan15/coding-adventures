// AUTO-GENERATED FILE — DO NOT EDIT
// Source: python.tokens
// Regenerate with: grammar-tools compile-tokens python.tokens
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
      name: "NAME",
      pattern: "[a-zA-Z_][a-zA-Z0-9_]*",
      isRegex: true,
      lineNumber: 13,
    },
    {
      name: "NUMBER",
      pattern: "[0-9]+",
      isRegex: true,
      lineNumber: 14,
    },
    {
      name: "STRING",
      pattern: "\"([^\"\\\\]|\\\\.)*\"",
      isRegex: true,
      lineNumber: 15,
    },
    {
      name: "EQUALS_EQUALS",
      pattern: "==",
      isRegex: false,
      lineNumber: 18,
    },
    {
      name: "EQUALS",
      pattern: "=",
      isRegex: false,
      lineNumber: 21,
    },
    {
      name: "PLUS",
      pattern: "+",
      isRegex: false,
      lineNumber: 22,
    },
    {
      name: "MINUS",
      pattern: "-",
      isRegex: false,
      lineNumber: 23,
    },
    {
      name: "STAR",
      pattern: "*",
      isRegex: false,
      lineNumber: 24,
    },
    {
      name: "SLASH",
      pattern: "/",
      isRegex: false,
      lineNumber: 25,
    },
    {
      name: "LPAREN",
      pattern: "(",
      isRegex: false,
      lineNumber: 28,
    },
    {
      name: "RPAREN",
      pattern: ")",
      isRegex: false,
      lineNumber: 29,
    },
    {
      name: "COMMA",
      pattern: ",",
      isRegex: false,
      lineNumber: 30,
    },
    {
      name: "COLON",
      pattern: ":",
      isRegex: false,
      lineNumber: 31,
    },
  ],
  keywords: ["if","else","elif","while","for","def","return","class","import","from","as","True","False","None"],
  mode: undefined,
  escapeMode: undefined,
  skipDefinitions: [],
  reservedKeywords: [],
  layoutKeywords: [],
  contextKeywords: [],
  errorDefinitions: [],
  groups: {},
};
