// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/derive/derive.tokens
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
        lineNumber: 43,
        alias: null),
    TokenDefinition(
        name: "LE", pattern: "<=", isRegex: false, lineNumber: 45, alias: null),
    TokenDefinition(
        name: "GE", pattern: ">=", isRegex: false, lineNumber: 46, alias: null),
    TokenDefinition(
        name: "PLUS",
        pattern: "+",
        isRegex: false,
        lineNumber: 52,
        alias: null),
    TokenDefinition(
        name: "MINUS",
        pattern: "-",
        isRegex: false,
        lineNumber: 53,
        alias: null),
    TokenDefinition(
        name: "TIMES",
        pattern: "*",
        isRegex: false,
        lineNumber: 54,
        alias: null),
    TokenDefinition(
        name: "SLASH",
        pattern: "/",
        isRegex: false,
        lineNumber: 55,
        alias: null),
    TokenDefinition(
        name: "POWER",
        pattern: "^",
        isRegex: false,
        lineNumber: 56,
        alias: null),
    TokenDefinition(
        name: "EQ", pattern: "=", isRegex: false, lineNumber: 63, alias: null),
    TokenDefinition(
        name: "LESS",
        pattern: "<",
        isRegex: false,
        lineNumber: 64,
        alias: null),
    TokenDefinition(
        name: "GREATER",
        pattern: ">",
        isRegex: false,
        lineNumber: 65,
        alias: null),
    TokenDefinition(
        name: "LPAREN",
        pattern: "(",
        isRegex: false,
        lineNumber: 67,
        alias: null),
    TokenDefinition(
        name: "RPAREN",
        pattern: ")",
        isRegex: false,
        lineNumber: 68,
        alias: null),
    TokenDefinition(
        name: "LBRACKET",
        pattern: "[",
        isRegex: false,
        lineNumber: 69,
        alias: null),
    TokenDefinition(
        name: "RBRACKET",
        pattern: "]",
        isRegex: false,
        lineNumber: 70,
        alias: null),
    TokenDefinition(
        name: "COMMA",
        pattern: ",",
        isRegex: false,
        lineNumber: 72,
        alias: null),
    TokenDefinition(
        name: "SEMI",
        pattern: ";",
        isRegex: false,
        lineNumber: 77,
        alias: null),
    TokenDefinition(
        name: "NUMBER",
        pattern: "[0-9]+\\.?[0-9]*([eE][+-]?[0-9]+)?",
        isRegex: true,
        lineNumber: 90,
        alias: null),
    TokenDefinition(
        name: "NAME",
        pattern: "[a-zA-Z][a-zA-Z0-9]*",
        isRegex: true,
        lineNumber: 91,
        alias: null),
    TokenDefinition(
        name: "NEWLINE",
        pattern: "\\r?\\n",
        isRegex: true,
        lineNumber: 104,
        alias: null),
  ],
  keywords: ["AND", "OR", "NOT"],
  mode: null,
  skipDefinitions: [
    TokenDefinition(
        name: "WHITESPACE",
        pattern: "[ \\t]+",
        isRegex: true,
        lineNumber: 107,
        alias: null),
  ],
  reservedKeywords: const [],
  escapeMode: "none",
  errorDefinitions: const [],
  groups: const {},
  caseSensitive: true,
  layoutKeywords: const [],
  contextKeywords: const [],
  softKeywords: const [],
);
