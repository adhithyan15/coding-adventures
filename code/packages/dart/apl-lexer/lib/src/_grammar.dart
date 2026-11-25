// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/apl/apl.tokens
// Regenerate with: grammar-tools compile-tokens <source.tokens>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final tokenGrammar = TokenGrammar(
  version: 1,
  caseInsensitive: false,
  definitions: [
    TokenDefinition(
        name: "OUTER",
        pattern: "∘.",
        isRegex: false,
        lineNumber: 40,
        alias: null),
    TokenDefinition(
        name: "ARROW",
        pattern: "←",
        isRegex: false,
        lineNumber: 51,
        alias: null),
    TokenDefinition(
        name: "PLUS",
        pattern: "+",
        isRegex: false,
        lineNumber: 56,
        alias: null),
    TokenDefinition(
        name: "MINUS",
        pattern: "-",
        isRegex: false,
        lineNumber: 57,
        alias: null),
    TokenDefinition(
        name: "TIMES",
        pattern: "×",
        isRegex: false,
        lineNumber: 58,
        alias: null),
    TokenDefinition(
        name: "DIVIDE",
        pattern: "÷",
        isRegex: false,
        lineNumber: 59,
        alias: null),
    TokenDefinition(
        name: "CEILING",
        pattern: "⌈",
        isRegex: false,
        lineNumber: 60,
        alias: null),
    TokenDefinition(
        name: "FLOOR",
        pattern: "⌊",
        isRegex: false,
        lineNumber: 61,
        alias: null),
    TokenDefinition(
        name: "RHO", pattern: "⍴", isRegex: false, lineNumber: 62, alias: null),
    TokenDefinition(
        name: "IOTA",
        pattern: "⍳",
        isRegex: false,
        lineNumber: 63,
        alias: null),
    TokenDefinition(
        name: "RAVEL",
        pattern: ",",
        isRegex: false,
        lineNumber: 64,
        alias: null),
    TokenDefinition(
        name: "EQ", pattern: "=", isRegex: false, lineNumber: 66, alias: null),
    TokenDefinition(
        name: "NE", pattern: "≠", isRegex: false, lineNumber: 67, alias: null),
    TokenDefinition(
        name: "LT", pattern: "<", isRegex: false, lineNumber: 68, alias: null),
    TokenDefinition(
        name: "LE", pattern: "≤", isRegex: false, lineNumber: 69, alias: null),
    TokenDefinition(
        name: "GE", pattern: "≥", isRegex: false, lineNumber: 70, alias: null),
    TokenDefinition(
        name: "GT", pattern: ">", isRegex: false, lineNumber: 71, alias: null),
    TokenDefinition(
        name: "REDUCE",
        pattern: "/",
        isRegex: false,
        lineNumber: 81,
        alias: null),
    TokenDefinition(
        name: "SCAN",
        pattern: "\\",
        isRegex: false,
        lineNumber: 82,
        alias: null),
    TokenDefinition(
        name: "LPAREN",
        pattern: "(",
        isRegex: false,
        lineNumber: 84,
        alias: null),
    TokenDefinition(
        name: "RPAREN",
        pattern: ")",
        isRegex: false,
        lineNumber: 85,
        alias: null),
    TokenDefinition(
        name: "NUMBER",
        pattern: "¯?[0-9]+(\\.[0-9]+)?([Ee]¯?[0-9]+)?",
        isRegex: true,
        lineNumber: 101,
        alias: null),
    TokenDefinition(
        name: "NAME",
        pattern: "[A-Za-z][A-Za-z0-9]*",
        isRegex: true,
        lineNumber: 102,
        alias: null),
    TokenDefinition(
        name: "NEWLINE",
        pattern: "\\r?\\n",
        isRegex: true,
        lineNumber: 117,
        alias: null),
  ],
  keywords: const [],
  mode: null,
  skipDefinitions: [
    TokenDefinition(
        name: "WHITESPACE",
        pattern: "[ \\t]+",
        isRegex: true,
        lineNumber: 120,
        alias: null),
    TokenDefinition(
        name: "COMMENT",
        pattern: "⍝[^\\r\\n]*",
        isRegex: true,
        lineNumber: 121,
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
