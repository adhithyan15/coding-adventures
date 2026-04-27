// AUTO-GENERATED FILE — DO NOT EDIT
// Source: ruby.tokens
// Regenerate with: grammar-tools compile-tokens ruby.tokens
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
      name: "EQUALS_EQUALS",
      pattern: "==",
      isRegex: false,
      lineNumber: 28,
    },
    {
      name: "DOT_DOT",
      pattern: "..",
      isRegex: false,
      lineNumber: 29,
    },
    {
      name: "HASH_ROCKET",
      pattern: "=>",
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
      name: "EQUALS",
      pattern: "=",
      isRegex: false,
      lineNumber: 36,
    },
    {
      name: "PLUS",
      pattern: "+",
      isRegex: false,
      lineNumber: 37,
    },
    {
      name: "MINUS",
      pattern: "-",
      isRegex: false,
      lineNumber: 38,
    },
    {
      name: "STAR",
      pattern: "*",
      isRegex: false,
      lineNumber: 39,
    },
    {
      name: "SLASH",
      pattern: "/",
      isRegex: false,
      lineNumber: 40,
    },
    {
      name: "LESS_THAN",
      pattern: "<",
      isRegex: false,
      lineNumber: 43,
    },
    {
      name: "GREATER_THAN",
      pattern: ">",
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
      name: "COMMA",
      pattern: ",",
      isRegex: false,
      lineNumber: 49,
    },
    {
      name: "COLON",
      pattern: ":",
      isRegex: false,
      lineNumber: 50,
    },
  ],
  keywords: ["if","else","elsif","end","while","for","do","def","return","class","module","require","puts","true","false","nil","and","or","not","then","unless","until","yield","begin","rescue","ensure"],
  mode: undefined,
  escapeMode: undefined,
  skipDefinitions: [],
  reservedKeywords: [],
  layoutKeywords: [],
  contextKeywords: [],
  errorDefinitions: [],
  groups: {},
};
