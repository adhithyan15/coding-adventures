// AUTO-GENERATED FILE - DO NOT EDIT
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
      lineNumber: 56,
    },
    {
      name: "SAT_ADD",
      pattern: "+?",
      isRegex: false,
      lineNumber: 63,
    },
    {
      name: "RANGE",
      pattern: "..",
      isRegex: false,
      lineNumber: 73,
    },
    {
      name: "ARROW",
      pattern: "->",
      isRegex: false,
      lineNumber: 79,
    },
    {
      name: "EQ_EQ",
      pattern: "==",
      isRegex: false,
      lineNumber: 87,
    },
    {
      name: "NEQ",
      pattern: "!=",
      isRegex: false,
      lineNumber: 93,
    },
    {
      name: "LEQ",
      pattern: "<=",
      isRegex: false,
      lineNumber: 97,
    },
    {
      name: "GEQ",
      pattern: ">=",
      isRegex: false,
      lineNumber: 101,
    },
    {
      name: "LAND",
      pattern: "&&",
      isRegex: false,
      lineNumber: 108,
    },
    {
      name: "LOR",
      pattern: "||",
      isRegex: false,
      lineNumber: 115,
    },
    {
      name: "SHL",
      pattern: "<<",
      isRegex: false,
      lineNumber: 119,
    },
    {
      name: "SHR",
      pattern: ">>",
      isRegex: false,
      lineNumber: 120,
    },
    {
      name: "PLUS",
      pattern: "+",
      isRegex: false,
      lineNumber: 128,
    },
    {
      name: "MINUS",
      pattern: "-",
      isRegex: false,
      lineNumber: 131,
    },
    {
      name: "STAR",
      pattern: "*",
      isRegex: false,
      lineNumber: 137,
    },
    {
      name: "SLASH",
      pattern: "/",
      isRegex: false,
      lineNumber: 141,
    },
    {
      name: "PERCENT",
      pattern: "%",
      isRegex: false,
      lineNumber: 146,
    },
    {
      name: "AMP",
      pattern: "&",
      isRegex: false,
      lineNumber: 154,
    },
    {
      name: "PIPE",
      pattern: "|",
      isRegex: false,
      lineNumber: 158,
    },
    {
      name: "CARET",
      pattern: "^",
      isRegex: false,
      lineNumber: 161,
    },
    {
      name: "TILDE",
      pattern: "~",
      isRegex: false,
      lineNumber: 165,
    },
    {
      name: "BANG",
      pattern: "!",
      isRegex: false,
      lineNumber: 174,
    },
    {
      name: "LT",
      pattern: "<",
      isRegex: false,
      lineNumber: 177,
    },
    {
      name: "GT",
      pattern: ">",
      isRegex: false,
      lineNumber: 180,
    },
    {
      name: "EQ",
      pattern: "=",
      isRegex: false,
      lineNumber: 190,
    },
    {
      name: "LBRACE",
      pattern: "{",
      isRegex: false,
      lineNumber: 197,
    },
    {
      name: "RBRACE",
      pattern: "}",
      isRegex: false,
      lineNumber: 198,
    },
    {
      name: "LPAREN",
      pattern: "(",
      isRegex: false,
      lineNumber: 201,
    },
    {
      name: "RPAREN",
      pattern: ")",
      isRegex: false,
      lineNumber: 202,
    },
    {
      name: "COLON",
      pattern: ":",
      isRegex: false,
      lineNumber: 206,
    },
    {
      name: "SEMICOLON",
      pattern: ";",
      isRegex: false,
      lineNumber: 211,
    },
    {
      name: "COMMA",
      pattern: ",",
      isRegex: false,
      lineNumber: 214,
    },
    {
      name: "HEX_LIT",
      pattern: "0x[0-9A-Fa-f]+",
      isRegex: true,
      lineNumber: 227,
    },
    {
      name: "INT_LIT",
      pattern: "[0-9]+",
      isRegex: true,
      lineNumber: 231,
    },
    {
      name: "NAME",
      pattern: "[a-zA-Z_][a-zA-Z0-9_]*",
      isRegex: true,
      lineNumber: 239,
    },
  ],
  keywords: ["fn","let","static","const","return","for","while","in","if","else","true","false"],
  mode: undefined,
  escapeMode: undefined,
  skipDefinitions: [
    {
      name: "WHITESPACE",
      pattern: "[ \\t\\r\\n]+",
      isRegex: true,
      lineNumber: 309,
    },
    {
      name: "LINE_COMMENT",
      pattern: "\\/\\/[^\\n]*",
      isRegex: true,
      lineNumber: 317,
    },
  ],
  reservedKeywords: [],
  layoutKeywords: [],
  contextKeywords: [],
  errorDefinitions: [],
  groups: {},
  softKeywords: [],
};
