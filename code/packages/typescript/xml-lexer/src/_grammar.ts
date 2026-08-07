// AUTO-GENERATED FILE - DO NOT EDIT
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
      lineNumber: 77,
    },
    {
      name: "ENTITY_REF",
      pattern: "&[a-zA-Z][a-zA-Z0-9]*;",
      isRegex: true,
      lineNumber: 78,
    },
    {
      name: "CHAR_REF_HEX",
      pattern: "&#x[0-9a-fA-F]+;",
      isRegex: true,
      lineNumber: 85,
      alias: "CHAR_REF",
    },
    {
      name: "CHAR_REF_DEC",
      pattern: "&#[0-9]+;",
      isRegex: true,
      lineNumber: 86,
      alias: "CHAR_REF",
    },
    {
      name: "COMMENT_START",
      pattern: "<!--",
      isRegex: false,
      lineNumber: 88,
    },
    {
      name: "CDATA_START",
      pattern: "<![CDATA[",
      isRegex: false,
      lineNumber: 89,
    },
    {
      name: "PI_START",
      pattern: "<?",
      isRegex: false,
      lineNumber: 90,
    },
    {
      name: "CLOSE_TAG_START",
      pattern: "</",
      isRegex: false,
      lineNumber: 91,
    },
    {
      name: "OPEN_TAG_START",
      pattern: "<",
      isRegex: false,
      lineNumber: 92,
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
      lineNumber: 62,
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
          lineNumber: 107,
        },
        {
          name: "ATTR_EQUALS",
          pattern: "=",
          isRegex: false,
          lineNumber: 108,
        },
        {
          name: "ATTR_VALUE_DQ",
          pattern: "\"[^\"]*\"",
          isRegex: true,
          lineNumber: 109,
          alias: "ATTR_VALUE",
        },
        {
          name: "ATTR_VALUE_SQ",
          pattern: "'[^']*'",
          isRegex: true,
          lineNumber: 110,
          alias: "ATTR_VALUE",
        },
        {
          name: "TAG_CLOSE",
          pattern: ">",
          isRegex: false,
          lineNumber: 111,
        },
        {
          name: "SELF_CLOSE",
          pattern: "/>",
          isRegex: false,
          lineNumber: 112,
        },
        {
          name: "SLASH",
          pattern: "/",
          isRegex: false,
          lineNumber: 113,
        },
      ],
    },
    "comment": {
      name: "comment",
      definitions: [
        {
          name: "COMMENT_END",
          pattern: "-->",
          isRegex: false,
          lineNumber: 133,
        },
        {
          name: "COMMENT_TEXT",
          pattern: "[^-]+",
          isRegex: true,
          lineNumber: 134,
        },
        {
          name: "COMMENT_DASH",
          pattern: "-",
          isRegex: true,
          lineNumber: 135,
          alias: "COMMENT_TEXT",
        },
      ],
    },
    "cdata": {
      name: "cdata",
      definitions: [
        {
          name: "CDATA_END",
          pattern: "]]>",
          isRegex: false,
          lineNumber: 150,
        },
        {
          name: "CDATA_TEXT",
          pattern: "[^\\]]+",
          isRegex: true,
          lineNumber: 151,
        },
        {
          name: "CDATA_BRACK",
          pattern: "]",
          isRegex: true,
          lineNumber: 152,
          alias: "CDATA_TEXT",
        },
      ],
    },
    "pi": {
      name: "pi",
      definitions: [
        {
          name: "PI_END",
          pattern: "?>",
          isRegex: false,
          lineNumber: 184,
        },
        {
          name: "PI_TARGET",
          pattern: "[a-zA-Z_][a-zA-Z0-9_:.-]*",
          isRegex: true,
          lineNumber: 185,
        },
      ],
    },
    "pi_body": {
      name: "pi_body",
      definitions: [
        {
          name: "PI_END",
          pattern: "?>",
          isRegex: false,
          lineNumber: 188,
        },
        {
          name: "PI_TEXT",
          pattern: "[^?]+",
          isRegex: true,
          lineNumber: 189,
        },
        {
          name: "PI_QMARK",
          pattern: "\\?",
          isRegex: true,
          lineNumber: 190,
          alias: "PI_TEXT",
        },
      ],
    },
  },
  softKeywords: [],
};
