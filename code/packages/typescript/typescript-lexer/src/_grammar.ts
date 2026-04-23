// AUTO-GENERATED FILE — DO NOT EDIT
// Source: typescript.tokens
// Regenerate with: grammar-tools compile-tokens typescript.tokens
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
      lineNumber: 25,
    },
    {
      name: "NUMBER",
      pattern: "[0-9]+",
      isRegex: true,
      lineNumber: 26,
    },
    {
      name: "STRING",
      pattern: "\"([^\"\\\\]|\\\\.)*\"",
      isRegex: true,
      lineNumber: 27,
    },
    {
      name: "STRICT_EQUALS",
      pattern: "===",
      isRegex: false,
      lineNumber: 30,
    },
    {
      name: "STRICT_NOT_EQUALS",
      pattern: "!==",
      isRegex: false,
      lineNumber: 31,
    },
    {
      name: "EQUALS_EQUALS",
      pattern: "==",
      isRegex: false,
      lineNumber: 32,
    },
    {
      name: "NOT_EQUALS",
      pattern: "!=",
      isRegex: false,
      lineNumber: 33,
    },
    {
      name: "LESS_EQUALS",
      pattern: "<=",
      isRegex: false,
      lineNumber: 34,
    },
    {
      name: "GREATER_EQUALS",
      pattern: ">=",
      isRegex: false,
      lineNumber: 35,
    },
    {
      name: "ARROW",
      pattern: "=>",
      isRegex: false,
      lineNumber: 36,
    },
    {
      name: "EQUALS",
      pattern: "=",
      isRegex: false,
      lineNumber: 39,
    },
    {
      name: "PLUS",
      pattern: "+",
      isRegex: false,
      lineNumber: 40,
    },
    {
      name: "MINUS",
      pattern: "-",
      isRegex: false,
      lineNumber: 41,
    },
    {
      name: "STAR",
      pattern: "*",
      isRegex: false,
      lineNumber: 42,
    },
    {
      name: "SLASH",
      pattern: "/",
      isRegex: false,
      lineNumber: 43,
    },
    {
      name: "LESS_THAN",
      pattern: "<",
      isRegex: false,
      lineNumber: 44,
    },
    {
      name: "GREATER_THAN",
      pattern: ">",
      isRegex: false,
      lineNumber: 45,
    },
    {
      name: "BANG",
      pattern: "!",
      isRegex: false,
      lineNumber: 46,
    },
    {
      name: "LPAREN",
      pattern: "(",
      isRegex: false,
      lineNumber: 49,
    },
    {
      name: "RPAREN",
      pattern: ")",
      isRegex: false,
      lineNumber: 50,
    },
    {
      name: "LBRACE",
      pattern: "{",
      isRegex: false,
      lineNumber: 51,
    },
    {
      name: "RBRACE",
      pattern: "}",
      isRegex: false,
      lineNumber: 52,
    },
    {
      name: "LBRACKET",
      pattern: "[",
      isRegex: false,
      lineNumber: 53,
    },
    {
      name: "RBRACKET",
      pattern: "]",
      isRegex: false,
      lineNumber: 54,
    },
    {
      name: "COMMA",
      pattern: ",",
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
      name: "SEMICOLON",
      pattern: ";",
      isRegex: false,
      lineNumber: 57,
    },
    {
      name: "DOT",
      pattern: ".",
      isRegex: false,
      lineNumber: 58,
    },
  ],
  keywords: ["let","const","var","if","else","while","for","do","function","return","class","import","export","from","as","new","this","typeof","instanceof","true","false","null","undefined","interface","type","enum","namespace","declare","readonly","public","private","protected","abstract","implements","extends","keyof","infer","never","unknown","any","void","number","string","boolean","object","symbol","bigint"],
  mode: undefined,
  escapeMode: undefined,
  skipDefinitions: [],
  reservedKeywords: [],
  layoutKeywords: [],
  contextKeywords: [],
  errorDefinitions: [],
  groups: {},
};
