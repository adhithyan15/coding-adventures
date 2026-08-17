// AUTO-GENERATED FILE - DO NOT EDIT
// Source: dartmouth_basic.tokens
// Regenerate with: grammar-tools compile-tokens dartmouth_basic.tokens
//
// This file embeds a TokenGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .tokens file at
// runtime.

import type { TokenGrammar } from "@coding-adventures/grammar-tools";

export const TOKEN_GRAMMAR: TokenGrammar = {
  version: 1,
  caseInsensitive: true,
  caseSensitive: false,
  definitions: [
    {
      name: "LE",
      pattern: "<=",
      isRegex: false,
      lineNumber: 50,
    },
    {
      name: "GE",
      pattern: ">=",
      isRegex: false,
      lineNumber: 51,
    },
    {
      name: "NE",
      pattern: "<>",
      isRegex: false,
      lineNumber: 52,
    },
    {
      name: "NUMBER",
      pattern: "[0-9]*\\.?[0-9]+([Ee][+-]?[0-9]+)?",
      isRegex: true,
      lineNumber: 85,
    },
    {
      name: "LINE_NUM",
      pattern: "[0-9]+",
      isRegex: true,
      lineNumber: 86,
    },
    {
      name: "STRING_BODY",
      pattern: "\"[^\"]*\"",
      isRegex: true,
      lineNumber: 112,
      alias: "STRING",
    },
    {
      name: "BUILTIN_FN",
      pattern: "(?:sin|cos|tan|atn|exp|log|abs|sqr|int|rnd|sgn)",
      isRegex: true,
      lineNumber: 168,
    },
    {
      name: "USER_FN",
      pattern: "fn[a-z]",
      isRegex: true,
      lineNumber: 169,
    },
    {
      name: "NAME",
      pattern: "[a-z][a-z0-9]*\\$?",
      isRegex: true,
      lineNumber: 204,
    },
    {
      name: "PLUS",
      pattern: "+",
      isRegex: false,
      lineNumber: 244,
    },
    {
      name: "MINUS",
      pattern: "-",
      isRegex: false,
      lineNumber: 245,
    },
    {
      name: "STAR",
      pattern: "*",
      isRegex: false,
      lineNumber: 246,
    },
    {
      name: "SLASH",
      pattern: "/",
      isRegex: false,
      lineNumber: 247,
    },
    {
      name: "CARET",
      pattern: "^",
      isRegex: false,
      lineNumber: 248,
    },
    {
      name: "EQ",
      pattern: "=",
      isRegex: false,
      lineNumber: 249,
    },
    {
      name: "LT",
      pattern: "<",
      isRegex: false,
      lineNumber: 250,
    },
    {
      name: "GT",
      pattern: ">",
      isRegex: false,
      lineNumber: 251,
    },
    {
      name: "LPAREN",
      pattern: "(",
      isRegex: false,
      lineNumber: 252,
    },
    {
      name: "RPAREN",
      pattern: ")",
      isRegex: false,
      lineNumber: 253,
    },
    {
      name: "COMMA",
      pattern: ",",
      isRegex: false,
      lineNumber: 254,
    },
    {
      name: "SEMICOLON",
      pattern: ";",
      isRegex: false,
      lineNumber: 255,
    },
    {
      name: "NEWLINE",
      pattern: "\\r?\\n",
      isRegex: true,
      lineNumber: 276,
    },
  ],
  keywords: ["LET","PRINT","INPUT","IF","THEN","GOTO","GOSUB","RETURN","FOR","TO","STEP","NEXT","END","STOP","REM","READ","DATA","RESTORE","DIM","DEF"],
  mode: undefined,
  escapeMode: undefined,
  skipDefinitions: [
    {
      name: "WHITESPACE",
      pattern: "[ \\t]+",
      isRegex: true,
      lineNumber: 288,
    },
  ],
  reservedKeywords: [],
  layoutKeywords: [],
  contextKeywords: [],
  errorDefinitions: [],
  groups: {},
  softKeywords: [],
};
