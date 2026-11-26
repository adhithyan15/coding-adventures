// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/axiom/axiom.tokens
// Regenerate with: grammar-tools compile-tokens <source.tokens>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final tokenGrammar = TokenGrammar(
  version: 1,
  caseInsensitive: false,
  definitions: [
    TokenDefinition(
        name: "ASSIGN",
        pattern: ":=",
        isRegex: false,
        lineNumber: 79,
        alias: null),
    TokenDefinition(
        name: "COERCE",
        pattern: "::",
        isRegex: false,
        lineNumber: 88,
        alias: null),
    TokenDefinition(
        name: "DEFINE",
        pattern: "==",
        isRegex: false,
        lineNumber: 97,
        alias: null),
    TokenDefinition(
        name: "NE",
        pattern: "~=",
        isRegex: false,
        lineNumber: 105,
        alias: null),
    TokenDefinition(
        name: "LE",
        pattern: "<=",
        isRegex: false,
        lineNumber: 107,
        alias: null),
    TokenDefinition(
        name: "GE",
        pattern: ">=",
        isRegex: false,
        lineNumber: 108,
        alias: null),
    TokenDefinition(
        name: "POW",
        pattern: "**",
        isRegex: false,
        lineNumber: 115,
        alias: null),
    TokenDefinition(
        name: "PLUS",
        pattern: "+",
        isRegex: false,
        lineNumber: 121,
        alias: null),
    TokenDefinition(
        name: "MINUS",
        pattern: "-",
        isRegex: false,
        lineNumber: 122,
        alias: null),
    TokenDefinition(
        name: "TIMES",
        pattern: "*",
        isRegex: false,
        lineNumber: 123,
        alias: null),
    TokenDefinition(
        name: "SLASH",
        pattern: "/",
        isRegex: false,
        lineNumber: 124,
        alias: null),
    TokenDefinition(
        name: "CARET",
        pattern: "^",
        isRegex: false,
        lineNumber: 131,
        alias: null),
    TokenDefinition(
        name: "EQ", pattern: "=", isRegex: false, lineNumber: 142, alias: null),
    TokenDefinition(
        name: "LESS",
        pattern: "<",
        isRegex: false,
        lineNumber: 143,
        alias: null),
    TokenDefinition(
        name: "GREATER",
        pattern: ">",
        isRegex: false,
        lineNumber: 144,
        alias: null),
    TokenDefinition(
        name: "LPAREN",
        pattern: "(",
        isRegex: false,
        lineNumber: 146,
        alias: null),
    TokenDefinition(
        name: "RPAREN",
        pattern: ")",
        isRegex: false,
        lineNumber: 147,
        alias: null),
    TokenDefinition(
        name: "LBRACKET",
        pattern: "[",
        isRegex: false,
        lineNumber: 153,
        alias: null),
    TokenDefinition(
        name: "RBRACKET",
        pattern: "]",
        isRegex: false,
        lineNumber: 154,
        alias: null),
    TokenDefinition(
        name: "COMMA",
        pattern: ",",
        isRegex: false,
        lineNumber: 156,
        alias: null),
    TokenDefinition(
        name: "SEMI",
        pattern: ";",
        isRegex: false,
        lineNumber: 162,
        alias: null),
    TokenDefinition(
        name: "COLON",
        pattern: ":",
        isRegex: false,
        lineNumber: 177,
        alias: null),
    TokenDefinition(
        name: "NUMBER",
        pattern: "[0-9]+\\.?[0-9]*([eE][+-]?[0-9]+)?",
        isRegex: true,
        lineNumber: 214,
        alias: null),
    TokenDefinition(
        name: "STRING",
        pattern: "\"[^\"]*\"",
        isRegex: true,
        lineNumber: 215,
        alias: null),
    TokenDefinition(
        name: "NAME",
        pattern: "[a-zA-Z][a-zA-Z0-9]*",
        isRegex: true,
        lineNumber: 216,
        alias: null),
  ],
  keywords: ["if", "then", "else", "has"],
  mode: null,
  skipDefinitions: [
    TokenDefinition(
        name: "WHITESPACE",
        pattern: "[ \\t\\r\\n]+",
        isRegex: true,
        lineNumber: 248,
        alias: null),
    TokenDefinition(
        name: "COMMENT",
        pattern: "--[^\\n]*",
        isRegex: true,
        lineNumber: 249,
        alias: null),
  ],
  reservedKeywords: const [],
  escapeMode: "none",
  errorDefinitions: [
    TokenDefinition(
        name: "UNKNOWN",
        pattern: ".",
        isRegex: true,
        lineNumber: 293,
        alias: null),
  ],
  groups: const {},
  caseSensitive: true,
  layoutKeywords: const [],
  contextKeywords: const [],
  softKeywords: const [],
);
