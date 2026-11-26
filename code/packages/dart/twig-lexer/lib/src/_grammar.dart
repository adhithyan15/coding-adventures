// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/twig/twig.tokens
// Regenerate with: grammar-tools compile-tokens <source.tokens>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final tokenGrammar = TokenGrammar(
  version: 1,
  caseInsensitive: false,
  definitions: [
    TokenDefinition(
        name: "LPAREN",
        pattern: "(",
        isRegex: false,
        lineNumber: 26,
        alias: null),
    TokenDefinition(
        name: "RPAREN",
        pattern: ")",
        isRegex: false,
        lineNumber: 27,
        alias: null),
    TokenDefinition(
        name: "QUOTE",
        pattern: "'",
        isRegex: false,
        lineNumber: 28,
        alias: null),
    TokenDefinition(
        name: "COLON",
        pattern: ":",
        isRegex: false,
        lineNumber: 36,
        alias: null),
    TokenDefinition(
        name: "ARROW",
        pattern: "->",
        isRegex: false,
        lineNumber: 55,
        alias: null),
    TokenDefinition(
        name: "BOOL_TRUE",
        pattern: "#t",
        isRegex: false,
        lineNumber: 63,
        alias: null),
    TokenDefinition(
        name: "BOOL_FALSE",
        pattern: "#f",
        isRegex: false,
        lineNumber: 64,
        alias: null),
    TokenDefinition(
        name: "STRING",
        pattern: "\"([^\"\\\\]|\\\\.)*\"",
        isRegex: true,
        lineNumber: 78,
        alias: null),
    TokenDefinition(
        name: "INTEGER",
        pattern: "-?[0-9]+",
        isRegex: true,
        lineNumber: 89,
        alias: null),
    TokenDefinition(
        name: "NAME",
        pattern: "[A-Za-z+\\-*/=<>!?_\$][A-Za-z+\\-*/=<>!?_\$0-9]*",
        isRegex: true,
        lineNumber: 102,
        alias: null),
  ],
  keywords: [
    "define",
    "lambda",
    "let",
    "if",
    "begin",
    "quote",
    "nil",
    "module",
    "export",
    "import",
    "typed",
    "type",
    "record",
    "union",
    "match",
    "let*"
  ],
  mode: null,
  skipDefinitions: [
    TokenDefinition(
        name: "WHITESPACE",
        pattern: "[ \\t\\r\\n]+",
        isRegex: true,
        lineNumber: 159,
        alias: null),
    TokenDefinition(
        name: "COMMENT",
        pattern: ";[^\\n]*",
        isRegex: true,
        lineNumber: 160,
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
