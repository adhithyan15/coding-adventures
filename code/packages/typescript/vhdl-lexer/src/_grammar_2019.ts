// AUTO-GENERATED FILE — DO NOT EDIT
// Source: vhdl.tokens
// Regenerate with: grammar-tools compile-tokens vhdl.tokens
//
// This file embeds a TokenGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .tokens file at
// runtime.

import type { TokenGrammar } from "@coding-adventures/grammar-tools";

export const TOKEN_GRAMMAR: TokenGrammar = {
  version: 0,
  caseInsensitive: false,
  caseSensitive: false,
  definitions: [
    {
      name: "STRING",
      pattern: "\"([^\"]|\"\")*\"",
      isRegex: true,
      lineNumber: 63,
    },
    {
      name: "BIT_STRING",
      pattern: "[bBoOxXdD]\"[0-9a-fA-F_]+\"",
      isRegex: true,
      lineNumber: 82,
    },
    {
      name: "CHAR_LITERAL",
      pattern: "'[^']'",
      isRegex: true,
      lineNumber: 100,
    },
    {
      name: "BASED_LITERAL",
      pattern: "[0-9]+#[0-9a-fA-F_]+(\\.[0-9a-fA-F_]+)?#([eE][+-]?[0-9_]+)?",
      isRegex: true,
      lineNumber: 116,
    },
    {
      name: "REAL_NUMBER",
      pattern: "[0-9][0-9_]*\\.[0-9_]+([eE][+-]?[0-9_]+)?",
      isRegex: true,
      lineNumber: 120,
    },
    {
      name: "NUMBER",
      pattern: "[0-9][0-9_]*",
      isRegex: true,
      lineNumber: 124,
    },
    {
      name: "EXTENDED_IDENT",
      pattern: "\\\\[^\\\\]+\\\\",
      isRegex: true,
      lineNumber: 143,
    },
    {
      name: "NAME",
      pattern: "[a-zA-Z][a-zA-Z0-9_]*",
      isRegex: true,
      lineNumber: 144,
    },
    {
      name: "VAR_ASSIGN",
      pattern: ":=",
      isRegex: false,
      lineNumber: 165,
    },
    {
      name: "LESS_EQUALS",
      pattern: "<=",
      isRegex: false,
      lineNumber: 166,
    },
    {
      name: "GREATER_EQUALS",
      pattern: ">=",
      isRegex: false,
      lineNumber: 167,
    },
    {
      name: "ARROW",
      pattern: "=>",
      isRegex: false,
      lineNumber: 168,
    },
    {
      name: "NOT_EQUALS",
      pattern: "/=",
      isRegex: false,
      lineNumber: 169,
    },
    {
      name: "POWER",
      pattern: "**",
      isRegex: false,
      lineNumber: 170,
    },
    {
      name: "BOX",
      pattern: "<>",
      isRegex: false,
      lineNumber: 171,
    },
    {
      name: "PLUS",
      pattern: "+",
      isRegex: false,
      lineNumber: 184,
    },
    {
      name: "MINUS",
      pattern: "-",
      isRegex: false,
      lineNumber: 185,
    },
    {
      name: "STAR",
      pattern: "*",
      isRegex: false,
      lineNumber: 186,
    },
    {
      name: "SLASH",
      pattern: "/",
      isRegex: false,
      lineNumber: 187,
    },
    {
      name: "AMPERSAND",
      pattern: "&",
      isRegex: false,
      lineNumber: 188,
    },
    {
      name: "LESS_THAN",
      pattern: "<",
      isRegex: false,
      lineNumber: 189,
    },
    {
      name: "GREATER_THAN",
      pattern: ">",
      isRegex: false,
      lineNumber: 190,
    },
    {
      name: "EQUALS",
      pattern: "=",
      isRegex: false,
      lineNumber: 191,
    },
    {
      name: "TICK",
      pattern: "'",
      isRegex: false,
      lineNumber: 192,
    },
    {
      name: "PIPE",
      pattern: "|",
      isRegex: false,
      lineNumber: 193,
    },
    {
      name: "LPAREN",
      pattern: "(",
      isRegex: false,
      lineNumber: 199,
    },
    {
      name: "RPAREN",
      pattern: ")",
      isRegex: false,
      lineNumber: 200,
    },
    {
      name: "LBRACKET",
      pattern: "[",
      isRegex: false,
      lineNumber: 201,
    },
    {
      name: "RBRACKET",
      pattern: "]",
      isRegex: false,
      lineNumber: 202,
    },
    {
      name: "SEMICOLON",
      pattern: ";",
      isRegex: false,
      lineNumber: 203,
    },
    {
      name: "COMMA",
      pattern: ",",
      isRegex: false,
      lineNumber: 204,
    },
    {
      name: "DOT",
      pattern: ".",
      isRegex: false,
      lineNumber: 205,
    },
    {
      name: "COLON",
      pattern: ":",
      isRegex: false,
      lineNumber: 206,
    },
  ],
  keywords: ["abs","access","after","alias","all","and","architecture","array","assert","attribute","begin","block","body","buffer","bus","case","component","configuration","constant","disconnect","downto","else","elsif","end","entity","exit","file","for","function","generate","generic","group","guarded","if","impure","in","inout","is","label","library","linkage","literal","loop","map","mod","nand","new","next","nor","not","null","of","on","open","or","others","out","package","port","postponed","procedure","process","pure","range","record","register","reject","rem","report","return","rol","ror","select","severity","signal","shared","sla","sll","sra","srl","subtype","then","to","transport","type","unaffected","units","until","use","variable","wait","when","while","with","xnor","xor"],
  mode: undefined,
  escapeMode: "none",
  skipDefinitions: [
    {
      name: "COMMENT",
      pattern: "--[^\\n]*",
      isRegex: true,
      lineNumber: 50,
    },
    {
      name: "WHITESPACE",
      pattern: "[ \\t\\r\\n]+",
      isRegex: true,
      lineNumber: 51,
    },
  ],
  reservedKeywords: [],
  errorDefinitions: [],
  groups: {},
};
