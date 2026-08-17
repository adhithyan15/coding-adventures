// AUTO-GENERATED FILE - DO NOT EDIT
// Source: haskell2010.tokens
// Regenerate with: grammar-tools compile-tokens haskell2010.tokens
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
      name: "FLOAT",
      pattern: "[0-9]+\\.[0-9]+",
      isRegex: true,
      lineNumber: 29,
    },
    {
      name: "INTEGER",
      pattern: "[0-9]+",
      isRegex: true,
      lineNumber: 30,
    },
    {
      name: "CHARACTER",
      pattern: "'(?:[^'\\\\]|\\\\.)'",
      isRegex: true,
      lineNumber: 31,
    },
    {
      name: "STRING",
      pattern: "\"(?:[^\"\\\\]|\\\\.)*\"",
      isRegex: true,
      lineNumber: 32,
    },
    {
      name: "LAMBDA",
      pattern: "\\\\",
      isRegex: false,
      lineNumber: 37,
    },
    {
      name: "RARROW",
      pattern: "->",
      isRegex: false,
      lineNumber: 38,
    },
    {
      name: "LARROW",
      pattern: "<-",
      isRegex: false,
      lineNumber: 39,
    },
    {
      name: "DARROW",
      pattern: "=>",
      isRegex: false,
      lineNumber: 40,
    },
    {
      name: "DOUBLE_COLON",
      pattern: "::",
      isRegex: false,
      lineNumber: 41,
    },
    {
      name: "DOUBLE_DOT",
      pattern: "..",
      isRegex: false,
      lineNumber: 42,
    },
    {
      name: "EQUALS",
      pattern: "=",
      isRegex: false,
      lineNumber: 43,
    },
    {
      name: "EQ",
      pattern: "==",
      isRegex: false,
      lineNumber: 44,
    },
    {
      name: "PLUS",
      pattern: "+",
      isRegex: false,
      lineNumber: 45,
    },
    {
      name: "MINUS",
      pattern: "-",
      isRegex: false,
      lineNumber: 46,
    },
    {
      name: "STAR",
      pattern: "*",
      isRegex: false,
      lineNumber: 47,
    },
    {
      name: "SLASH",
      pattern: "/",
      isRegex: false,
      lineNumber: 48,
    },
    {
      name: "PIPE",
      pattern: "|",
      isRegex: false,
      lineNumber: 49,
    },
    {
      name: "AMPERSAND",
      pattern: "&",
      isRegex: false,
      lineNumber: 50,
    },
    {
      name: "CARET",
      pattern: "^",
      isRegex: false,
      lineNumber: 51,
    },
    {
      name: "TILDE",
      pattern: "~",
      isRegex: false,
      lineNumber: 52,
    },
    {
      name: "BANG",
      pattern: "!",
      isRegex: false,
      lineNumber: 53,
    },
    {
      name: "LESSTHAN",
      pattern: "<",
      isRegex: false,
      lineNumber: 54,
    },
    {
      name: "GREATERTHAN",
      pattern: ">",
      isRegex: false,
      lineNumber: 55,
    },
    {
      name: "COLON",
      pattern: ":",
      isRegex: false,
      lineNumber: 56,
    },
    {
      name: "COMMA",
      pattern: ",",
      isRegex: false,
      lineNumber: 57,
    },
    {
      name: "SEMICOLON",
      pattern: ";",
      isRegex: false,
      lineNumber: 58,
    },
    {
      name: "DOT",
      pattern: ".",
      isRegex: false,
      lineNumber: 59,
    },
    {
      name: "LPAREN",
      pattern: "(",
      isRegex: false,
      lineNumber: 60,
    },
    {
      name: "RPAREN",
      pattern: ")",
      isRegex: false,
      lineNumber: 61,
    },
    {
      name: "LBRACKET",
      pattern: "[",
      isRegex: false,
      lineNumber: 62,
    },
    {
      name: "RBRACKET",
      pattern: "]",
      isRegex: false,
      lineNumber: 63,
    },
    {
      name: "LBRACE",
      pattern: "{",
      isRegex: false,
      lineNumber: 64,
    },
    {
      name: "RBRACE",
      pattern: "}",
      isRegex: false,
      lineNumber: 65,
    },
    {
      name: "NAME",
      pattern: "[A-Za-z_][A-Za-z0-9_']*",
      isRegex: true,
      lineNumber: 70,
    },
  ],
  keywords: ["as","case","class","data","do","else","foreign","if","import","in","infix","infixl","infixr","let","module","of","then","type","where"],
  mode: "layout",
  escapeMode: undefined,
  skipDefinitions: [
    {
      name: "LINE_COMMENT",
      pattern: "--[^\\n]*",
      isRegex: true,
      lineNumber: 22,
    },
    {
      name: "BLOCK_COMMENT",
      pattern: "\\{\\-[\\s\\S]*?\\-\\}",
      isRegex: true,
      lineNumber: 23,
    },
    {
      name: "WHITESPACE",
      pattern: "[ \\t\\r]+",
      isRegex: true,
      lineNumber: 24,
    },
  ],
  reservedKeywords: [],
  layoutKeywords: ["let","where","do","of"],
  contextKeywords: [],
  errorDefinitions: [],
  groups: {},
  softKeywords: [],
};
