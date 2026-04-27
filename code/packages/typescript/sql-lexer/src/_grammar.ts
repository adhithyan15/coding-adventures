// AUTO-GENERATED FILE — DO NOT EDIT
// Source: sql.tokens
// Regenerate with: grammar-tools compile-tokens sql.tokens
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
      name: "NAME",
      pattern: "[a-zA-Z_][a-zA-Z0-9_]*",
      isRegex: true,
      lineNumber: 17,
    },
    {
      name: "NUMBER",
      pattern: "[0-9]+\\.?[0-9]*",
      isRegex: true,
      lineNumber: 18,
    },
    {
      name: "STRING_SQ",
      pattern: "'([^'\\\\]|\\\\.)*'",
      isRegex: true,
      lineNumber: 19,
      alias: "STRING",
    },
    {
      name: "QUOTED_ID",
      pattern: "`[^`]+`",
      isRegex: true,
      lineNumber: 20,
      alias: "NAME",
    },
    {
      name: "LESS_EQUALS",
      pattern: "<=",
      isRegex: false,
      lineNumber: 22,
    },
    {
      name: "GREATER_EQUALS",
      pattern: ">=",
      isRegex: false,
      lineNumber: 23,
    },
    {
      name: "NOT_EQUALS",
      pattern: "!=",
      isRegex: false,
      lineNumber: 24,
    },
    {
      name: "NEQ_ANSI",
      pattern: "<>",
      isRegex: false,
      lineNumber: 25,
      alias: "NOT_EQUALS",
    },
    {
      name: "EQUALS",
      pattern: "=",
      isRegex: false,
      lineNumber: 27,
    },
    {
      name: "LESS_THAN",
      pattern: "<",
      isRegex: false,
      lineNumber: 28,
    },
    {
      name: "GREATER_THAN",
      pattern: ">",
      isRegex: false,
      lineNumber: 29,
    },
    {
      name: "PLUS",
      pattern: "+",
      isRegex: false,
      lineNumber: 30,
    },
    {
      name: "MINUS",
      pattern: "-",
      isRegex: false,
      lineNumber: 31,
    },
    {
      name: "STAR",
      pattern: "*",
      isRegex: false,
      lineNumber: 32,
    },
    {
      name: "SLASH",
      pattern: "/",
      isRegex: false,
      lineNumber: 33,
    },
    {
      name: "PERCENT",
      pattern: "%",
      isRegex: false,
      lineNumber: 34,
    },
    {
      name: "LPAREN",
      pattern: "(",
      isRegex: false,
      lineNumber: 36,
    },
    {
      name: "RPAREN",
      pattern: ")",
      isRegex: false,
      lineNumber: 37,
    },
    {
      name: "COMMA",
      pattern: ",",
      isRegex: false,
      lineNumber: 38,
    },
    {
      name: "SEMICOLON",
      pattern: ";",
      isRegex: false,
      lineNumber: 39,
    },
    {
      name: "DOT",
      pattern: ".",
      isRegex: false,
      lineNumber: 40,
    },
  ],
  keywords: ["SELECT","FROM","WHERE","GROUP","BY","HAVING","ORDER","LIMIT","OFFSET","INSERT","INTO","VALUES","UPDATE","SET","DELETE","CREATE","DROP","TABLE","IF","EXISTS","NOT","AND","OR","NULL","IS","IN","BETWEEN","LIKE","AS","DISTINCT","ALL","UNION","INTERSECT","EXCEPT","JOIN","INNER","LEFT","RIGHT","OUTER","CROSS","FULL","ON","ASC","DESC","TRUE","FALSE","CASE","WHEN","THEN","ELSE","END","PRIMARY","KEY","UNIQUE","DEFAULT"],
  mode: undefined,
  escapeMode: undefined,
  skipDefinitions: [
    {
      name: "WHITESPACE",
      pattern: "[ \\t\\r\\n]+",
      isRegex: true,
      lineNumber: 100,
    },
    {
      name: "LINE_COMMENT",
      pattern: "--[^\\n]*",
      isRegex: true,
      lineNumber: 101,
    },
    {
      name: "BLOCK_COMMENT",
      pattern: "\\x2f\\*([^*]|\\*[^\\x2f])*\\*\\x2f",
      isRegex: true,
      lineNumber: 102,
    },
  ],
  reservedKeywords: [],
  layoutKeywords: [],
  contextKeywords: [],
  errorDefinitions: [],
  groups: {},
};
