// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/macsyma/macsyma.tokens
// Regenerate with: grammar-tools compile-tokens code/grammars/macsyma/macsyma.tokens
//
// This file embeds a TokenGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .tokens file at
// runtime.

import type { TokenGrammar } from "@coding-adventures/grammar-tools";

export const TOKEN_GRAMMAR = {
  version: 1,
  caseInsensitive: false,
  caseSensitive: true,
  definitions: [
    {
      name: "COLONEQ",
      pattern: ":=",
      isRegex: false,
      lineNumber: 47,
    },
    {
      name: "STAREQ",
      pattern: "**",
      isRegex: false,
      lineNumber: 48,
    },
    {
      name: "LEQ",
      pattern: "<=",
      isRegex: false,
      lineNumber: 49,
    },
    {
      name: "GEQ",
      pattern: ">=",
      isRegex: false,
      lineNumber: 50,
    },
    {
      name: "ARROW",
      pattern: "->",
      isRegex: false,
      lineNumber: 51,
    },
    {
      name: "PLUS",
      pattern: "+",
      isRegex: false,
      lineNumber: 57,
    },
    {
      name: "MINUS",
      pattern: "-",
      isRegex: false,
      lineNumber: 58,
    },
    {
      name: "STAR",
      pattern: "*",
      isRegex: false,
      lineNumber: 59,
    },
    {
      name: "SLASH",
      pattern: "/",
      isRegex: false,
      lineNumber: 60,
    },
    {
      name: "CARET",
      pattern: "^",
      isRegex: false,
      lineNumber: 61,
    },
    {
      name: "COLON",
      pattern: ":",
      isRegex: false,
      lineNumber: 62,
    },
    {
      name: "EQ",
      pattern: "=",
      isRegex: false,
      lineNumber: 63,
    },
    {
      name: "HASH",
      pattern: "#",
      isRegex: false,
      lineNumber: 64,
    },
    {
      name: "LT",
      pattern: "<",
      isRegex: false,
      lineNumber: 65,
    },
    {
      name: "GT",
      pattern: ">",
      isRegex: false,
      lineNumber: 66,
    },
    {
      name: "BANG",
      pattern: "!",
      isRegex: false,
      lineNumber: 67,
    },
    {
      name: "LPAREN",
      pattern: "(",
      isRegex: false,
      lineNumber: 69,
    },
    {
      name: "RPAREN",
      pattern: ")",
      isRegex: false,
      lineNumber: 70,
    },
    {
      name: "LBRACKET",
      pattern: "[",
      isRegex: false,
      lineNumber: 71,
    },
    {
      name: "RBRACKET",
      pattern: "]",
      isRegex: false,
      lineNumber: 72,
    },
    {
      name: "LBRACE",
      pattern: "{",
      isRegex: false,
      lineNumber: 73,
    },
    {
      name: "RBRACE",
      pattern: "}",
      isRegex: false,
      lineNumber: 74,
    },
    {
      name: "COMMA",
      pattern: ",",
      isRegex: false,
      lineNumber: 76,
    },
    {
      name: "SEMI",
      pattern: ";",
      isRegex: false,
      lineNumber: 77,
    },
    {
      name: "DOLLAR",
      pattern: "$",
      isRegex: false,
      lineNumber: 78,
    },
    {
      name: "NUMBER",
      pattern: "[0-9]+\\.?[0-9]*([eE][+-]?[0-9]+)?",
      isRegex: true,
      lineNumber: 95,
    },
    {
      name: "NAME",
      pattern: "%?[a-zA-Z_][a-zA-Z0-9_]*",
      isRegex: true,
      lineNumber: 96,
    },
    {
      name: "STRING",
      pattern: "\"([^\"\\\\]|\\\\.)*\"",
      isRegex: true,
      lineNumber: 97,
    },
  ],
  keywords: ["and","or","not","if","then","else","elseif","true","false","do","for","while","unless","in","step","thru","block","return"],
  mode: undefined,
  escapeMode: "none",
  skipDefinitions: [
    {
      name: "WHITESPACE",
      pattern: "[ \\t\\r\\n]+",
      isRegex: true,
      lineNumber: 136,
    },
    {
      name: "LINECOMMENT",
      pattern: "\\/\\*([^*]|\\*[^\\/])*\\*\\/",
      isRegex: true,
      lineNumber: 137,
    },
  ],
  reservedKeywords: [],
  layoutKeywords: [],
  contextKeywords: [],
  errorDefinitions: [],
  groups: {},
} as unknown as TokenGrammar;
