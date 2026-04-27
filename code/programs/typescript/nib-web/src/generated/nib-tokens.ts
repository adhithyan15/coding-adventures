// AUTO-GENERATED FILE — DO NOT EDIT
// Source: nib.tokens
// Regenerate with: grammar-tools compile-tokens nib.tokens
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
      name: "WRAP_ADD",
      pattern: "+%",
      isRegex: false,
      lineNumber: 54,
    },
    {
      name: "SAT_ADD",
      pattern: "+?",
      isRegex: false,
      lineNumber: 61,
    },
    {
      name: "RANGE",
      pattern: "..",
      isRegex: false,
      lineNumber: 71,
    },
    {
      name: "ARROW",
      pattern: "->",
      isRegex: false,
      lineNumber: 77,
    },
    {
      name: "EQ_EQ",
      pattern: "==",
      isRegex: false,
      lineNumber: 85,
    },
    {
      name: "NEQ",
      pattern: "!=",
      isRegex: false,
      lineNumber: 91,
    },
    {
      name: "LEQ",
      pattern: "<=",
      isRegex: false,
      lineNumber: 95,
    },
    {
      name: "GEQ",
      pattern: ">=",
      isRegex: false,
      lineNumber: 99,
    },
    {
      name: "LAND",
      pattern: "&&",
      isRegex: false,
      lineNumber: 106,
    },
    {
      name: "LOR",
      pattern: "||",
      isRegex: false,
      lineNumber: 113,
    },
    {
      name: "PLUS",
      pattern: "+",
      isRegex: false,
      lineNumber: 121,
    },
    {
      name: "MINUS",
      pattern: "-",
      isRegex: false,
      lineNumber: 124,
    },
    {
      name: "STAR",
      pattern: "*",
      isRegex: false,
      lineNumber: 129,
    },
    {
      name: "SLASH",
      pattern: "/",
      isRegex: false,
      lineNumber: 132,
    },
    {
      name: "AMP",
      pattern: "&",
      isRegex: false,
      lineNumber: 140,
    },
    {
      name: "PIPE",
      pattern: "|",
      isRegex: false,
      lineNumber: 144,
    },
    {
      name: "CARET",
      pattern: "^",
      isRegex: false,
      lineNumber: 147,
    },
    {
      name: "TILDE",
      pattern: "~",
      isRegex: false,
      lineNumber: 151,
    },
    {
      name: "BANG",
      pattern: "!",
      isRegex: false,
      lineNumber: 160,
    },
    {
      name: "LT",
      pattern: "<",
      isRegex: false,
      lineNumber: 163,
    },
    {
      name: "GT",
      pattern: ">",
      isRegex: false,
      lineNumber: 166,
    },
    {
      name: "EQ",
      pattern: "=",
      isRegex: false,
      lineNumber: 176,
    },
    {
      name: "LBRACE",
      pattern: "{",
      isRegex: false,
      lineNumber: 183,
    },
    {
      name: "RBRACE",
      pattern: "}",
      isRegex: false,
      lineNumber: 184,
    },
    {
      name: "LPAREN",
      pattern: "(",
      isRegex: false,
      lineNumber: 187,
    },
    {
      name: "RPAREN",
      pattern: ")",
      isRegex: false,
      lineNumber: 188,
    },
    {
      name: "COLON",
      pattern: ":",
      isRegex: false,
      lineNumber: 192,
    },
    {
      name: "SEMICOLON",
      pattern: ";",
      isRegex: false,
      lineNumber: 197,
    },
    {
      name: "COMMA",
      pattern: ",",
      isRegex: false,
      lineNumber: 200,
    },
    {
      name: "HEX_LIT",
      pattern: "0x[0-9A-Fa-f]+",
      isRegex: true,
      lineNumber: 213,
    },
    {
      name: "INT_LIT",
      pattern: "[0-9]+",
      isRegex: true,
      lineNumber: 217,
    },
    {
      name: "NAME",
      pattern: "[a-zA-Z_][a-zA-Z0-9_]*",
      isRegex: true,
      lineNumber: 225,
    },
  ],
  keywords: ["fn","let","static","const","return","for","in","if","else","true","false"],
  mode: undefined,
  escapeMode: undefined,
  skipDefinitions: [
    {
      name: "WHITESPACE",
      pattern: "[ \\t\\r\\n]+",
      isRegex: true,
      lineNumber: 289,
    },
    {
      name: "LINE_COMMENT",
      pattern: "\\/\\/[^\\n]*",
      isRegex: true,
      lineNumber: 297,
    },
  ],
  reservedKeywords: [],
  contextKeywords: [],
  errorDefinitions: [],
  groups: {},
};
