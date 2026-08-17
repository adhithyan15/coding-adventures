// AUTO-GENERATED FILE - DO NOT EDIT
// Source: dot.tokens
// Regenerate with: grammar-tools compile-tokens dot.tokens
//
// This file embeds a TokenGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .tokens file at
// runtime.

import type { TokenGrammar } from "@coding-adventures/grammar-tools";

export const TOKEN_GRAMMAR: TokenGrammar = {
  version: 1,
  caseInsensitive: true,
  caseSensitive: true,
  definitions: [
    {
      name: "EDGEOP",
      pattern: "->",
      isRegex: false,
      lineNumber: 20,
    },
    {
      name: "LBRACE",
      pattern: "{",
      isRegex: false,
      lineNumber: 21,
    },
    {
      name: "RBRACE",
      pattern: "}",
      isRegex: false,
      lineNumber: 22,
    },
    {
      name: "LBRACKET",
      pattern: "[",
      isRegex: false,
      lineNumber: 23,
    },
    {
      name: "RBRACKET",
      pattern: "]",
      isRegex: false,
      lineNumber: 24,
    },
    {
      name: "EQUALS",
      pattern: "=",
      isRegex: false,
      lineNumber: 25,
    },
    {
      name: "COMMA",
      pattern: ",",
      isRegex: false,
      lineNumber: 26,
    },
    {
      name: "SEMICOLON",
      pattern: ";",
      isRegex: false,
      lineNumber: 27,
    },
    {
      name: "STRING",
      pattern: "\"([^\"\\\\]|\\\\.)*\"",
      isRegex: true,
      lineNumber: 29,
    },
    {
      name: "NUMBER",
      pattern: "-?(?:\\.[0-9]+|[0-9]+(?:\\.[0-9]*)?)(?:[eE][+-]?[0-9]+)?",
      isRegex: true,
      lineNumber: 30,
    },
    {
      name: "NAME",
      pattern: "[A-Za-z_\\x80-\\xff][A-Za-z0-9_\\x80-\\xff]*",
      isRegex: true,
      lineNumber: 31,
    },
  ],
  keywords: ["strict","digraph","graph","node","edge"],
  mode: undefined,
  escapeMode: undefined,
  skipDefinitions: [
    {
      name: "WHITESPACE",
      pattern: "[ \\t\\r\\n]+",
      isRegex: true,
      lineNumber: 15,
    },
    {
      name: "LINE_COMMENT",
      pattern: "\\x2f\\x2f[^\\n]*",
      isRegex: true,
      lineNumber: 16,
    },
    {
      name: "BLOCK_COMMENT",
      pattern: "\\x2f\\*[\\s\\S]*?\\*\\x2f",
      isRegex: true,
      lineNumber: 17,
    },
    {
      name: "PREPROCESSOR",
      pattern: "#[^\\n]*",
      isRegex: true,
      lineNumber: 18,
    },
  ],
  reservedKeywords: [],
  layoutKeywords: [],
  contextKeywords: [],
  errorDefinitions: [],
  groups: {},
  softKeywords: [],
};
