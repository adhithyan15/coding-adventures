// AUTO-GENERATED FILE - DO NOT EDIT
// Source: algol60.tokens
// Regenerate with: grammar-tools compile-tokens algol60.tokens
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
      name: "REAL_LIT",
      pattern: "[0-9]+\\.[0-9]*([eE][+-]?[0-9]+)?|[0-9]+[eE][+-]?[0-9]+",
      isRegex: true,
      lineNumber: 38,
    },
    {
      name: "INTEGER_LIT",
      pattern: "[0-9]+",
      isRegex: true,
      lineNumber: 41,
    },
    {
      name: "STRING_LIT",
      pattern: "'[^']*'|\"[^\"]*\"",
      isRegex: true,
      lineNumber: 46,
    },
    {
      name: "NAME",
      pattern: "[a-zA-Z][a-zA-Z0-9]*",
      isRegex: true,
      lineNumber: 53,
    },
    {
      name: "ASSIGN",
      pattern: ":=",
      isRegex: false,
      lineNumber: 61,
    },
    {
      name: "POWER",
      pattern: "**",
      isRegex: false,
      lineNumber: 66,
    },
    {
      name: "LEQ",
      pattern: "<=|≤",
      isRegex: true,
      lineNumber: 70,
    },
    {
      name: "GEQ",
      pattern: ">=|≥",
      isRegex: true,
      lineNumber: 71,
    },
    {
      name: "NEQ",
      pattern: "!=|<>|≠",
      isRegex: true,
      lineNumber: 72,
    },
    {
      name: "NOT_SYM",
      pattern: "¬",
      isRegex: false,
      lineNumber: 76,
    },
    {
      name: "AND_SYM",
      pattern: "∧",
      isRegex: false,
      lineNumber: 77,
    },
    {
      name: "OR_SYM",
      pattern: "∨",
      isRegex: false,
      lineNumber: 78,
    },
    {
      name: "IMPL_SYM",
      pattern: "⊃",
      isRegex: false,
      lineNumber: 79,
    },
    {
      name: "EQV_SYM",
      pattern: "≡",
      isRegex: false,
      lineNumber: 80,
    },
    {
      name: "PLUS",
      pattern: "+",
      isRegex: false,
      lineNumber: 86,
    },
    {
      name: "MINUS",
      pattern: "-",
      isRegex: false,
      lineNumber: 87,
    },
    {
      name: "STAR",
      pattern: "\\*|×",
      isRegex: true,
      lineNumber: 90,
    },
    {
      name: "SLASH",
      pattern: "\\/|÷",
      isRegex: true,
      lineNumber: 91,
    },
    {
      name: "CARET",
      pattern: "\\^|↑",
      isRegex: true,
      lineNumber: 95,
    },
    {
      name: "EQ",
      pattern: "=",
      isRegex: false,
      lineNumber: 98,
    },
    {
      name: "LT",
      pattern: "<",
      isRegex: false,
      lineNumber: 100,
    },
    {
      name: "GT",
      pattern: ">",
      isRegex: false,
      lineNumber: 101,
    },
    {
      name: "LPAREN",
      pattern: "(",
      isRegex: false,
      lineNumber: 107,
    },
    {
      name: "RPAREN",
      pattern: ")",
      isRegex: false,
      lineNumber: 108,
    },
    {
      name: "LBRACKET",
      pattern: "[",
      isRegex: false,
      lineNumber: 109,
    },
    {
      name: "RBRACKET",
      pattern: "]",
      isRegex: false,
      lineNumber: 110,
    },
    {
      name: "SEMICOLON",
      pattern: ";",
      isRegex: false,
      lineNumber: 111,
    },
    {
      name: "COMMA",
      pattern: ",",
      isRegex: false,
      lineNumber: 112,
    },
    {
      name: "COLON",
      pattern: ":",
      isRegex: false,
      lineNumber: 116,
    },
  ],
  keywords: ["begin","end","if","then","else","for","do","step","until","while","goto","switch","procedure","own","array","label","value","integer","real","boolean","string","true","false","not","and","or","impl","eqv","div","mod","comment"],
  mode: undefined,
  escapeMode: undefined,
  skipDefinitions: [
    {
      name: "WHITESPACE",
      pattern: "[ \\t\\r\\n]+",
      isRegex: true,
      lineNumber: 183,
    },
    {
      name: "COMMENT",
      pattern: "comment[^a-zA-Z0-9_][^;]*;",
      isRegex: true,
      lineNumber: 192,
    },
  ],
  reservedKeywords: [],
  layoutKeywords: [],
  contextKeywords: [],
  errorDefinitions: [],
  groups: {},
  softKeywords: [],
};
