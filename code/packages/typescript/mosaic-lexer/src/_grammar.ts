// AUTO-GENERATED FILE — DO NOT EDIT
// Source: mosaic.tokens
// Regenerate with: grammar-tools compile-tokens mosaic.tokens
//
// This file embeds a TokenGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .tokens file at runtime.

import type { TokenGrammar } from "@coding-adventures/grammar-tools";

export const TOKEN_GRAMMAR: TokenGrammar = {
  version: 1,
  caseInsensitive: false,
  caseSensitive: true,
  definitions: [
    {
      name: "STRING",
      pattern: '"([^"\\\\\\n]|\\\\.)*"',
      isRegex: true,
      lineNumber: 23,
    },
    // DIMENSION must come before NUMBER — same digit prefix, longer match wins.
    {
      name: "DIMENSION",
      pattern: "-?[0-9]*\\.?[0-9]+[a-zA-Z%]+",
      isRegex: true,
      lineNumber: 31,
    },
    {
      name: "NUMBER",
      pattern: "-?[0-9]*\\.?[0-9]+",
      isRegex: true,
      lineNumber: 32,
    },
    {
      name: "COLOR_HEX",
      pattern: "#[0-9a-fA-F]{3,8}",
      isRegex: true,
      lineNumber: 39,
    },
    {
      name: "LBRACE",
      pattern: "{",
      isRegex: false,
      lineNumber: 76,
    },
    {
      name: "RBRACE",
      pattern: "}",
      isRegex: false,
      lineNumber: 77,
    },
    {
      name: "LANGLE",
      pattern: "<",
      isRegex: false,
      lineNumber: 78,
    },
    {
      name: "RANGLE",
      pattern: ">",
      isRegex: false,
      lineNumber: 79,
    },
    {
      name: "COLON",
      pattern: ":",
      isRegex: false,
      lineNumber: 80,
    },
    {
      name: "SEMICOLON",
      pattern: ";",
      isRegex: false,
      lineNumber: 81,
    },
    {
      name: "COMMA",
      pattern: ",",
      isRegex: false,
      lineNumber: 82,
    },
    {
      name: "DOT",
      pattern: ".",
      isRegex: false,
      lineNumber: 83,
    },
    {
      name: "EQUALS",
      pattern: "=",
      isRegex: false,
      lineNumber: 84,
    },
    {
      name: "AT",
      pattern: "@",
      isRegex: false,
      lineNumber: 85,
    },
    // NAME comes after delimiters but is listed before keywords so that
    // the keyword list can override matching text.
    // Named "NAME" (not "IDENT") so the grammar engine performs keyword
    // reclassification: when a NAME value matches a keyword, the token
    // type is promoted to "KEYWORD".
    {
      name: "NAME",
      pattern: "[a-zA-Z_][a-zA-Z0-9_-]*",
      isRegex: true,
      lineNumber: 70,
    },
  ],
  keywords: [
    "component",
    "slot",
    "import",
    "from",
    "as",
    "text",
    "number",
    "bool",
    "image",
    "color",
    "node",
    "list",
    "true",
    "false",
    "when",
    "each",
  ],
  mode: undefined,
  escapeMode: "standard",
  skipDefinitions: [
    {
      name: "LINE_COMMENT",
      pattern: "\\/\\/[^\\n]*",
      isRegex: true,
      lineNumber: 15,
    },
    {
      name: "BLOCK_COMMENT",
      pattern: "\\/\\*[\\s\\S]*?\\*\\/",
      isRegex: true,
      lineNumber: 16,
    },
    {
      name: "WHITESPACE",
      pattern: "[ \\t\\r\\n]+",
      isRegex: true,
      lineNumber: 17,
    },
  ],
  reservedKeywords: [],
  errorDefinitions: [],
  groups: {},
};
