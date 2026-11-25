// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/j/j.tokens
// Regenerate with: grammar-tools compile-tokens <source.tokens>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final tokenGrammar = TokenGrammar(
  version: 1,
  caseInsensitive: false,
  definitions: [
    TokenDefinition(
        name: "FLOOR",
        pattern: "<.",
        isRegex: false,
        lineNumber: 65,
        alias: null),
    TokenDefinition(
        name: "CEILING",
        pattern: ">.",
        isRegex: false,
        lineNumber: 66,
        alias: null),
    TokenDefinition(
        name: "LE", pattern: "<:", isRegex: false, lineNumber: 70, alias: null),
    TokenDefinition(
        name: "GE", pattern: ">:", isRegex: false, lineNumber: 71, alias: null),
    TokenDefinition(
        name: "NE", pattern: "~:", isRegex: false, lineNumber: 72, alias: null),
    TokenDefinition(
        name: "IDOT",
        pattern: "i.",
        isRegex: false,
        lineNumber: 77,
        alias: null),
    TokenDefinition(
        name: "ASSIGN_LOCAL",
        pattern: "=.",
        isRegex: false,
        lineNumber: 82,
        alias: null),
    TokenDefinition(
        name: "ASSIGN_GLOBAL",
        pattern: "=:",
        isRegex: false,
        lineNumber: 83,
        alias: null),
    TokenDefinition(
        name: "PLUS",
        pattern: "+",
        isRegex: false,
        lineNumber: 96,
        alias: null),
    TokenDefinition(
        name: "MINUS",
        pattern: "-",
        isRegex: false,
        lineNumber: 97,
        alias: null),
    TokenDefinition(
        name: "STAR",
        pattern: "*",
        isRegex: false,
        lineNumber: 98,
        alias: null),
    TokenDefinition(
        name: "PERCENT",
        pattern: "%",
        isRegex: false,
        lineNumber: 99,
        alias: null),
    TokenDefinition(
        name: "CARET",
        pattern: "^",
        isRegex: false,
        lineNumber: 100,
        alias: null),
    TokenDefinition(
        name: "DOLLAR",
        pattern: "\$",
        isRegex: false,
        lineNumber: 101,
        alias: null),
    TokenDefinition(
        name: "RAVEL",
        pattern: ",",
        isRegex: false,
        lineNumber: 102,
        alias: null),
    TokenDefinition(
        name: "HASH",
        pattern: "#",
        isRegex: false,
        lineNumber: 103,
        alias: null),
    TokenDefinition(
        name: "EQ", pattern: "=", isRegex: false, lineNumber: 105, alias: null),
    TokenDefinition(
        name: "LT", pattern: "<", isRegex: false, lineNumber: 106, alias: null),
    TokenDefinition(
        name: "GT", pattern: ">", isRegex: false, lineNumber: 107, alias: null),
    TokenDefinition(
        name: "REDUCE",
        pattern: "/",
        isRegex: false,
        lineNumber: 119,
        alias: null),
    TokenDefinition(
        name: "SCAN",
        pattern: "\\",
        isRegex: false,
        lineNumber: 120,
        alias: null),
    TokenDefinition(
        name: "AT", pattern: "@", isRegex: false, lineNumber: 121, alias: null),
    TokenDefinition(
        name: "LPAREN",
        pattern: "(",
        isRegex: false,
        lineNumber: 123,
        alias: null),
    TokenDefinition(
        name: "RPAREN",
        pattern: ")",
        isRegex: false,
        lineNumber: 124,
        alias: null),
    TokenDefinition(
        name: "NUMBER",
        pattern: "_?[0-9]+(\\.[0-9]+)?([Ee]_?[0-9]+)?",
        isRegex: true,
        lineNumber: 139,
        alias: null),
    TokenDefinition(
        name: "NAME",
        pattern: "[A-Za-z][A-Za-z0-9]*",
        isRegex: true,
        lineNumber: 140,
        alias: null),
    TokenDefinition(
        name: "NEWLINE",
        pattern: "\\r?\\n",
        isRegex: true,
        lineNumber: 158,
        alias: null),
  ],
  keywords: const [],
  mode: null,
  skipDefinitions: [
    TokenDefinition(
        name: "WHITESPACE",
        pattern: "[ \\t]+",
        isRegex: true,
        lineNumber: 161,
        alias: null),
    TokenDefinition(
        name: "COMMENT",
        pattern: "NB\\.[^\\r\\n]*",
        isRegex: true,
        lineNumber: 162,
        alias: null),
  ],
  reservedKeywords: const [],
  escapeMode: null,
  errorDefinitions: const [],
  groups: const {},
  caseSensitive: true,
  layoutKeywords: const [],
  contextKeywords: const [],
  softKeywords: const [],
);
