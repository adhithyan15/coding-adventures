// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/maple/maple.tokens
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
        lineNumber: 69,
        alias: null),
    TokenDefinition(
        name: "ARROW",
        pattern: "->",
        isRegex: false,
        lineNumber: 78,
        alias: null),
    TokenDefinition(
        name: "NEQ",
        pattern: "<>",
        isRegex: false,
        lineNumber: 85,
        alias: null),
    TokenDefinition(
        name: "LE", pattern: "<=", isRegex: false, lineNumber: 87, alias: null),
    TokenDefinition(
        name: "GE", pattern: ">=", isRegex: false, lineNumber: 88, alias: null),
    TokenDefinition(
        name: "PLUS",
        pattern: "+",
        isRegex: false,
        lineNumber: 94,
        alias: null),
    TokenDefinition(
        name: "MINUS",
        pattern: "-",
        isRegex: false,
        lineNumber: 95,
        alias: null),
    TokenDefinition(
        name: "TIMES",
        pattern: "*",
        isRegex: false,
        lineNumber: 96,
        alias: null),
    TokenDefinition(
        name: "SLASH",
        pattern: "/",
        isRegex: false,
        lineNumber: 97,
        alias: null),
    TokenDefinition(
        name: "CARET",
        pattern: "^",
        isRegex: false,
        lineNumber: 104,
        alias: null),
    TokenDefinition(
        name: "EQ", pattern: "=", isRegex: false, lineNumber: 110, alias: null),
    TokenDefinition(
        name: "LESS",
        pattern: "<",
        isRegex: false,
        lineNumber: 111,
        alias: null),
    TokenDefinition(
        name: "GREATER",
        pattern: ">",
        isRegex: false,
        lineNumber: 112,
        alias: null),
    TokenDefinition(
        name: "LPAREN",
        pattern: "(",
        isRegex: false,
        lineNumber: 114,
        alias: null),
    TokenDefinition(
        name: "RPAREN",
        pattern: ")",
        isRegex: false,
        lineNumber: 115,
        alias: null),
    TokenDefinition(
        name: "LBRACKET",
        pattern: "[",
        isRegex: false,
        lineNumber: 125,
        alias: null),
    TokenDefinition(
        name: "RBRACKET",
        pattern: "]",
        isRegex: false,
        lineNumber: 126,
        alias: null),
    TokenDefinition(
        name: "LBRACE",
        pattern: "{",
        isRegex: false,
        lineNumber: 137,
        alias: null),
    TokenDefinition(
        name: "RBRACE",
        pattern: "}",
        isRegex: false,
        lineNumber: 138,
        alias: null),
    TokenDefinition(
        name: "COMMA",
        pattern: ",",
        isRegex: false,
        lineNumber: 144,
        alias: null),
    TokenDefinition(
        name: "SEMI",
        pattern: ";",
        isRegex: false,
        lineNumber: 154,
        alias: null),
    TokenDefinition(
        name: "COLON",
        pattern: ":",
        isRegex: false,
        lineNumber: 155,
        alias: null),
    TokenDefinition(
        name: "NUMBER",
        pattern: "[0-9]+\\.?[0-9]*([eE][+-]?[0-9]+)?",
        isRegex: true,
        lineNumber: 170,
        alias: null),
    TokenDefinition(
        name: "NAME",
        pattern: "[a-zA-Z][a-zA-Z0-9]*",
        isRegex: true,
        lineNumber: 171,
        alias: null),
  ],
  keywords: [
    "and",
    "or",
    "not",
    "if",
    "then",
    "elif",
    "else",
    "end",
    "fi",
    "true",
    "false"
  ],
  mode: null,
  skipDefinitions: [
    TokenDefinition(
        name: "WHITESPACE",
        pattern: "[ \\t\\r\\n]+",
        isRegex: true,
        lineNumber: 185,
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
