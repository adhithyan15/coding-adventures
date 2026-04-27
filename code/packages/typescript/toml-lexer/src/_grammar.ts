// AUTO-GENERATED FILE — DO NOT EDIT
// Source: toml.tokens
// Regenerate with: grammar-tools compile-tokens toml.tokens
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
      name: "ML_BASIC_STRING",
      pattern: "\"\"\"([^\\\\]|\\\\(.|\\n)|\\n)*?\"\"\"",
      isRegex: true,
      lineNumber: 60,
    },
    {
      name: "ML_LITERAL_STRING",
      pattern: "'''[\\s\\S]*?'''",
      isRegex: true,
      lineNumber: 61,
    },
    {
      name: "BASIC_STRING",
      pattern: "\"([^\"\\\\\\n]|\\\\.)*\"",
      isRegex: true,
      lineNumber: 70,
    },
    {
      name: "LITERAL_STRING",
      pattern: "'[^'\\n]*'",
      isRegex: true,
      lineNumber: 71,
    },
    {
      name: "OFFSET_DATETIME_FRAC_TZ",
      pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}\\.\\d+[+-]\\d{2}:\\d{2}",
      isRegex: true,
      lineNumber: 91,
      alias: "OFFSET_DATETIME",
    },
    {
      name: "OFFSET_DATETIME_FRAC_Z",
      pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}\\.\\d+Z",
      isRegex: true,
      lineNumber: 92,
      alias: "OFFSET_DATETIME",
    },
    {
      name: "OFFSET_DATETIME_TZ",
      pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}[+-]\\d{2}:\\d{2}",
      isRegex: true,
      lineNumber: 93,
      alias: "OFFSET_DATETIME",
    },
    {
      name: "OFFSET_DATETIME_Z",
      pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}Z",
      isRegex: true,
      lineNumber: 94,
      alias: "OFFSET_DATETIME",
    },
    {
      name: "LOCAL_DATETIME_FRAC",
      pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}\\.\\d+",
      isRegex: true,
      lineNumber: 95,
      alias: "LOCAL_DATETIME",
    },
    {
      name: "LOCAL_DATETIME",
      pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}",
      isRegex: true,
      lineNumber: 96,
    },
    {
      name: "LOCAL_DATE",
      pattern: "\\d{4}-\\d{2}-\\d{2}",
      isRegex: true,
      lineNumber: 97,
    },
    {
      name: "LOCAL_TIME_FRAC",
      pattern: "\\d{2}:\\d{2}:\\d{2}\\.\\d+",
      isRegex: true,
      lineNumber: 98,
      alias: "LOCAL_TIME",
    },
    {
      name: "LOCAL_TIME",
      pattern: "\\d{2}:\\d{2}:\\d{2}",
      isRegex: true,
      lineNumber: 99,
    },
    {
      name: "FLOAT_INF",
      pattern: "[+-]?inf",
      isRegex: true,
      lineNumber: 114,
      alias: "FLOAT",
    },
    {
      name: "FLOAT_NAN",
      pattern: "[+-]?nan",
      isRegex: true,
      lineNumber: 115,
      alias: "FLOAT",
    },
    {
      name: "FLOAT_EXP",
      pattern: "[+-]?[0-9][0-9_]*\\.?[0-9_]*[eE][+-]?[0-9][0-9_]*",
      isRegex: true,
      lineNumber: 116,
      alias: "FLOAT",
    },
    {
      name: "FLOAT_DEC",
      pattern: "[+-]?[0-9][0-9_]*\\.[0-9][0-9_]*",
      isRegex: true,
      lineNumber: 117,
      alias: "FLOAT",
    },
    {
      name: "HEX_INTEGER",
      pattern: "0x[0-9a-fA-F][0-9a-fA-F_]*",
      isRegex: true,
      lineNumber: 129,
      alias: "INTEGER",
    },
    {
      name: "OCT_INTEGER",
      pattern: "0o[0-7][0-7_]*",
      isRegex: true,
      lineNumber: 130,
      alias: "INTEGER",
    },
    {
      name: "BIN_INTEGER",
      pattern: "0b[01][01_]*",
      isRegex: true,
      lineNumber: 131,
      alias: "INTEGER",
    },
    {
      name: "INTEGER",
      pattern: "[+-]?[0-9][0-9_]*",
      isRegex: true,
      lineNumber: 132,
    },
    {
      name: "TRUE",
      pattern: "true",
      isRegex: false,
      lineNumber: 143,
    },
    {
      name: "FALSE",
      pattern: "false",
      isRegex: false,
      lineNumber: 144,
    },
    {
      name: "BARE_KEY",
      pattern: "[A-Za-z0-9_-]+",
      isRegex: true,
      lineNumber: 158,
    },
    {
      name: "EQUALS",
      pattern: "=",
      isRegex: false,
      lineNumber: 168,
    },
    {
      name: "DOT",
      pattern: ".",
      isRegex: false,
      lineNumber: 169,
    },
    {
      name: "COMMA",
      pattern: ",",
      isRegex: false,
      lineNumber: 170,
    },
    {
      name: "LBRACKET",
      pattern: "[",
      isRegex: false,
      lineNumber: 171,
    },
    {
      name: "RBRACKET",
      pattern: "]",
      isRegex: false,
      lineNumber: 172,
    },
    {
      name: "LBRACE",
      pattern: "{",
      isRegex: false,
      lineNumber: 173,
    },
    {
      name: "RBRACE",
      pattern: "}",
      isRegex: false,
      lineNumber: 174,
    },
    {
      name: "NEWLINE",
      pattern: "\\r?\\n",
      isRegex: true,
      lineNumber: 175,
    },
  ],
  keywords: [],
  mode: undefined,
  escapeMode: "none",
  skipDefinitions: [
    {
      name: "COMMENT",
      pattern: "#[^\\n]*",
      isRegex: true,
      lineNumber: 28,
    },
    {
      name: "WHITESPACE",
      pattern: "[ \\t]+",
      isRegex: true,
      lineNumber: 29,
    },
  ],
  reservedKeywords: [],
  layoutKeywords: [],
  contextKeywords: [],
  errorDefinitions: [],
  groups: {},
};
