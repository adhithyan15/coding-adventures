// AUTO-GENERATED FILE — DO NOT EDIT
// Source: javascript.tokens
// Regenerate with: grammar-tools compile-tokens javascript.tokens
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
      pattern: "[a-zA-Z_$][a-zA-Z0-9_$]*",
      isRegex: true,
      lineNumber: 23,
    },
    {
      name: "NUMBER",
      pattern: "[0-9]+",
      isRegex: true,
      lineNumber: 24,
    },
    {
      name: "STRING",
      pattern: "\"([^\"\\\\]|\\\\.)*\"",
      isRegex: true,
      lineNumber: 25,
    },
    {
      name: "STRICT_EQUALS",
      pattern: "===",
      isRegex: false,
      lineNumber: 28,
    },
    {
      name: "STRICT_NOT_EQUALS",
      pattern: "!==",
      isRegex: false,
      lineNumber: 29,
    },
    {
      name: "EQUALS_EQUALS",
      pattern: "==",
      isRegex: false,
      lineNumber: 30,
    },
    {
      name: "NOT_EQUALS",
      pattern: "!=",
      isRegex: false,
      lineNumber: 31,
    },
    {
      name: "LESS_EQUALS",
      pattern: "<=",
      isRegex: false,
      lineNumber: 32,
    },
    {
      name: "GREATER_EQUALS",
      pattern: ">=",
      isRegex: false,
      lineNumber: 33,
    },
    {
      name: "ARROW",
      pattern: "=>",
      isRegex: false,
      lineNumber: 34,
    },
    {
      name: "EQUALS",
      pattern: "=",
      isRegex: false,
      lineNumber: 37,
    },
    {
      name: "PLUS",
      pattern: "+",
      isRegex: false,
      lineNumber: 38,
    },
    {
      name: "MINUS",
      pattern: "-",
      isRegex: false,
      lineNumber: 39,
    },
    {
      name: "STAR",
      pattern: "*",
      isRegex: false,
      lineNumber: 40,
    },
    {
      name: "SLASH",
      pattern: "/",
      isRegex: false,
      lineNumber: 41,
    },
    {
      name: "LESS_THAN",
      pattern: "<",
      isRegex: false,
      lineNumber: 42,
    },
    {
      name: "GREATER_THAN",
      pattern: ">",
      isRegex: false,
      lineNumber: 43,
    },
    {
      name: "BANG",
      pattern: "!",
      isRegex: false,
      lineNumber: 44,
    },
    {
      name: "LPAREN",
      pattern: "(",
      isRegex: false,
      lineNumber: 47,
    },
    {
      name: "RPAREN",
      pattern: ")",
      isRegex: false,
      lineNumber: 48,
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
      name: "COMMA",
      pattern: ",",
      isRegex: false,
      lineNumber: 53,
    },
    {
      name: "COLON",
      pattern: ":",
      isRegex: false,
      lineNumber: 54,
    },
    {
      name: "SEMICOLON",
      pattern: ";",
      isRegex: false,
      lineNumber: 55,
    },
    {
      name: "DOT",
      pattern: ".",
      isRegex: false,
      lineNumber: 56,
    },
  ],
  keywords: ["let","const","var","if","else","while","for","do","function","return","class","import","export","from","as","new","this","typeof","instanceof","true","false","null","undefined"],
  mode: undefined,
  escapeMode: undefined,
  skipDefinitions: [],
  reservedKeywords: [],
  layoutKeywords: [],
  contextKeywords: [],
  errorDefinitions: [],
  groups: {},
};
