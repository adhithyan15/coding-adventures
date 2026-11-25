// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/reduce/reduce.tokens
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
        lineNumber: 51,
        alias: null),
    TokenDefinition(
        name: "LE", pattern: "<=", isRegex: false, lineNumber: 53, alias: null),
    TokenDefinition(
        name: "GE", pattern: ">=", isRegex: false, lineNumber: 54, alias: null),
    TokenDefinition(
        name: "POW",
        pattern: "**",
        isRegex: false,
        lineNumber: 62,
        alias: null),
    TokenDefinition(
        name: "GROUP_OPEN",
        pattern: "<<",
        isRegex: false,
        lineNumber: 67,
        alias: null),
    TokenDefinition(
        name: "GROUP_CLOSE",
        pattern: ">>",
        isRegex: false,
        lineNumber: 68,
        alias: null),
    TokenDefinition(
        name: "PLUS",
        pattern: "+",
        isRegex: false,
        lineNumber: 74,
        alias: null),
    TokenDefinition(
        name: "MINUS",
        pattern: "-",
        isRegex: false,
        lineNumber: 75,
        alias: null),
    TokenDefinition(
        name: "TIMES",
        pattern: "*",
        isRegex: false,
        lineNumber: 76,
        alias: null),
    TokenDefinition(
        name: "SLASH",
        pattern: "/",
        isRegex: false,
        lineNumber: 77,
        alias: null),
    TokenDefinition(
        name: "CARET",
        pattern: "^",
        isRegex: false,
        lineNumber: 78,
        alias: null),
    TokenDefinition(
        name: "EQ", pattern: "=", isRegex: false, lineNumber: 85, alias: null),
    TokenDefinition(
        name: "LESS",
        pattern: "<",
        isRegex: false,
        lineNumber: 86,
        alias: null),
    TokenDefinition(
        name: "GREATER",
        pattern: ">",
        isRegex: false,
        lineNumber: 87,
        alias: null),
    TokenDefinition(
        name: "LPAREN",
        pattern: "(",
        isRegex: false,
        lineNumber: 89,
        alias: null),
    TokenDefinition(
        name: "RPAREN",
        pattern: ")",
        isRegex: false,
        lineNumber: 90,
        alias: null),
    TokenDefinition(
        name: "LBRACE",
        pattern: "{",
        isRegex: false,
        lineNumber: 98,
        alias: null),
    TokenDefinition(
        name: "RBRACE",
        pattern: "}",
        isRegex: false,
        lineNumber: 99,
        alias: null),
    TokenDefinition(
        name: "COMMA",
        pattern: ",",
        isRegex: false,
        lineNumber: 101,
        alias: null),
    TokenDefinition(
        name: "SEMI",
        pattern: ";",
        isRegex: false,
        lineNumber: 107,
        alias: null),
    TokenDefinition(
        name: "DOLLAR",
        pattern: "\$",
        isRegex: false,
        lineNumber: 114,
        alias: null),
    TokenDefinition(
        name: "DOT",
        pattern: ".",
        isRegex: false,
        lineNumber: 123,
        alias: null),
    TokenDefinition(
        name: "NUMBER",
        pattern: "[0-9]+\\.?[0-9]*([eE][+-]?[0-9]+)?",
        isRegex: true,
        lineNumber: 137,
        alias: null),
    TokenDefinition(
        name: "NAME",
        pattern: "[a-zA-Z][a-zA-Z0-9]*",
        isRegex: true,
        lineNumber: 138,
        alias: null),
  ],
  keywords: ["and", "or", "not", "neq", "if", "then", "else"],
  mode: null,
  skipDefinitions: [
    TokenDefinition(
        name: "WHITESPACE",
        pattern: "[ \\t\\r\\n]+",
        isRegex: true,
        lineNumber: 151,
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
