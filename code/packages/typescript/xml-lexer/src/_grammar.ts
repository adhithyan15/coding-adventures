// AUTO-GENERATED FILE — DO NOT EDIT
// Source: xml.tokens
// Regenerate with: grammar-tools compile-tokens xml.tokens
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
      name: "TEXT",
      pattern: "[^<&]+",
      isRegex: true,
      lineNumber: 57,
    },
    {
      name: "ENTITY_REF",
      pattern: "&[a-zA-Z][a-zA-Z0-9]*;",
      isRegex: true,
      lineNumber: 58,
    },
    {
      name: "CHAR_REF",
      pattern: "&#[0-9]+;|&#x[0-9a-fA-F]+;",
      isRegex: true,
      lineNumber: 59,
    },
    {
      name: "COMMENT_START",
      pattern: "<!--",
      isRegex: false,
      lineNumber: 60,
    },
    {
      name: "CDATA_START",
      pattern: "<![CDATA[",
      isRegex: false,
      lineNumber: 61,
    },
    {
      name: "PI_START",
      pattern: "<?",
      isRegex: false,
      lineNumber: 62,
    },
    {
      name: "CLOSE_TAG_START",
      pattern: "</",
      isRegex: false,
      lineNumber: 63,
    },
    {
      name: "OPEN_TAG_START",
      pattern: "<",
      isRegex: false,
      lineNumber: 64,
    },
  ],
  keywords: [],
  mode: undefined,
  escapeMode: "none",
  skipDefinitions: [
    {
      name: "WHITESPACE",
      pattern: "[ \\t\\r\\n]+",
      isRegex: true,
      lineNumber: 42,
    },
  ],
  reservedKeywords: [],
  layoutKeywords: [],
  contextKeywords: [],
  errorDefinitions: [],
  groups: {
    "tag": {
      name: "tag",
      definitions: [
        {
          name: "TAG_NAME",
          pattern: "[a-zA-Z_][a-zA-Z0-9_:.-]*",
          isRegex: true,
          lineNumber: 79,
        },
        {
          name: "ATTR_EQUALS",
          pattern: "=",
          isRegex: false,
          lineNumber: 80,
        },
        {
          name: "ATTR_VALUE_DQ",
          pattern: "\"[^\"]*\"",
          isRegex: true,
          lineNumber: 81,
          alias: "ATTR_VALUE",
        },
        {
          name: "ATTR_VALUE_SQ",
          pattern: "'[^']*'",
          isRegex: true,
          lineNumber: 82,
          alias: "ATTR_VALUE",
        },
        {
          name: "TAG_CLOSE",
          pattern: ">",
          isRegex: false,
          lineNumber: 83,
        },
        {
          name: "SELF_CLOSE",
          pattern: "/>",
          isRegex: false,
          lineNumber: 84,
        },
        {
          name: "SLASH",
          pattern: "/",
          isRegex: false,
          lineNumber: 85,
        },
      ],
    },
    "comment": {
      name: "comment",
      definitions: [
        {
          name: "COMMENT_TEXT",
          pattern: "([^-]|-(?!->))+",
          isRegex: true,
          lineNumber: 99,
        },
        {
          name: "COMMENT_END",
          pattern: "-->",
          isRegex: false,
          lineNumber: 100,
        },
      ],
    },
    "cdata": {
      name: "cdata",
      definitions: [
        {
          name: "CDATA_TEXT",
          pattern: "([^\\]]|\\](?!\\]>))+",
          isRegex: true,
          lineNumber: 113,
        },
        {
          name: "CDATA_END",
          pattern: "]]>",
          isRegex: false,
          lineNumber: 114,
        },
      ],
    },
    "pi": {
      name: "pi",
      definitions: [
        {
          name: "PI_TARGET",
          pattern: "[a-zA-Z_][a-zA-Z0-9_:.-]*",
          isRegex: true,
          lineNumber: 128,
        },
        {
          name: "PI_TEXT",
          pattern: "([^?]|\\?(?!>))+",
          isRegex: true,
          lineNumber: 129,
        },
        {
          name: "PI_END",
          pattern: "?>",
          isRegex: false,
          lineNumber: 130,
        },
      ],
    },
  },
};
