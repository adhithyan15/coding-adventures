// AUTO-GENERATED FILE - DO NOT EDIT
// Source: ../../../grammars/macsyma/macsyma.tokens
// Regenerate with: grammar-tools compile-tokens ../../../grammars/macsyma/macsyma.tokens
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
      name: "COLONEQ",
      pattern: ":=",
      isRegex: false,
      lineNumber: 48,
    },
    {
      name: "STAREQ",
      pattern: "**",
      isRegex: false,
      lineNumber: 49,
    },
    {
      name: "LEQ",
      pattern: "<=",
      isRegex: false,
      lineNumber: 50,
    },
    {
      name: "GEQ",
      pattern: ">=",
      isRegex: false,
      lineNumber: 51,
    },
    {
      name: "ARROW",
      pattern: "->",
      isRegex: false,
      lineNumber: 52,
    },
    {
      name: "PLUS",
      pattern: "+",
      isRegex: false,
      lineNumber: 58,
    },
    {
      name: "MINUS",
      pattern: "-",
      isRegex: false,
      lineNumber: 59,
    },
    {
      name: "STAR",
      pattern: "*",
      isRegex: false,
      lineNumber: 60,
    },
    {
      name: "SLASH",
      pattern: "/",
      isRegex: false,
      lineNumber: 61,
    },
    {
      name: "CARET",
      pattern: "^",
      isRegex: false,
      lineNumber: 62,
    },
    {
      name: "COLON",
      pattern: ":",
      isRegex: false,
      lineNumber: 63,
    },
    {
      name: "EQ",
      pattern: "=",
      isRegex: false,
      lineNumber: 64,
    },
    {
      name: "HASH",
      pattern: "#",
      isRegex: false,
      lineNumber: 65,
    },
    {
      name: "LT",
      pattern: "<",
      isRegex: false,
      lineNumber: 66,
    },
    {
      name: "GT",
      pattern: ">",
      isRegex: false,
      lineNumber: 67,
    },
    {
      name: "BANG",
      pattern: "!",
      isRegex: false,
      lineNumber: 68,
    },
    {
      name: "LPAREN",
      pattern: "(",
      isRegex: false,
      lineNumber: 70,
    },
    {
      name: "RPAREN",
      pattern: ")",
      isRegex: false,
      lineNumber: 71,
    },
    {
      name: "LBRACKET",
      pattern: "[",
      isRegex: false,
      lineNumber: 72,
    },
    {
      name: "RBRACKET",
      pattern: "]",
      isRegex: false,
      lineNumber: 73,
    },
    {
      name: "LBRACE",
      pattern: "{",
      isRegex: false,
      lineNumber: 74,
    },
    {
      name: "RBRACE",
      pattern: "}",
      isRegex: false,
      lineNumber: 75,
    },
    {
      name: "COMMA",
      pattern: ",",
      isRegex: false,
      lineNumber: 77,
    },
    {
      name: "SEMI",
      pattern: ";",
      isRegex: false,
      lineNumber: 78,
    },
    {
      name: "DOLLAR",
      pattern: "$",
      isRegex: false,
      lineNumber: 79,
    },
    {
      name: "NUMBER",
      pattern: "[0-9]+\\.?[0-9]*([eE][+-]?[0-9]+)?",
      isRegex: true,
      lineNumber: 96,
    },
    {
      name: "NAME",
      pattern: "%[a-zA-Z_][a-zA-Z0-9_]*|%|[a-zA-Z_][a-zA-Z0-9_]*",
      isRegex: true,
      lineNumber: 97,
    },
    {
      name: "STRING",
      pattern: "\"([^\"\\\\]|\\\\.)*\"",
      isRegex: true,
      lineNumber: 98,
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
      lineNumber: 137,
    },
    {
      name: "LINECOMMENT",
      pattern: "\\/\\*([^*]|\\*[^\\/])*\\*\\/",
      isRegex: true,
      lineNumber: 138,
    },
  ],
  reservedKeywords: [],
  layoutKeywords: [],
  contextKeywords: [],
  errorDefinitions: [],
  groups: {},
  softKeywords: [],
};
