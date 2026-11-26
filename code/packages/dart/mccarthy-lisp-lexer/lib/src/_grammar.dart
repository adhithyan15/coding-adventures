// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/mccarthy_lisp/mccarthy_lisp.tokens
// Regenerate with: grammar-tools compile-tokens <source.tokens>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final tokenGrammar = TokenGrammar(
  version: 1,
  caseInsensitive: false,
  definitions: [
    TokenDefinition(
        name: "INTEGER",
        pattern: "-?[0-9]+",
        isRegex: true,
        lineNumber: 59,
        alias: null),
    TokenDefinition(
        name: "SYMBOL",
        pattern: "[A-Z][A-Z0-9-]*",
        isRegex: true,
        lineNumber: 69,
        alias: null),
    TokenDefinition(
        name: "LPAREN",
        pattern: "(",
        isRegex: false,
        lineNumber: 78,
        alias: null),
    TokenDefinition(
        name: "RPAREN",
        pattern: ")",
        isRegex: false,
        lineNumber: 79,
        alias: null),
    TokenDefinition(
        name: "QUOTE",
        pattern: "'",
        isRegex: false,
        lineNumber: 80,
        alias: null),
    TokenDefinition(
        name: "DOT", pattern: ".", isRegex: false, lineNumber: 81, alias: null),
  ],
  keywords: const [],
  mode: null,
  skipDefinitions: [
    TokenDefinition(
        name: "WHITESPACE",
        pattern: "[ \\t\\r\\n]+",
        isRegex: true,
        lineNumber: 42,
        alias: null),
    TokenDefinition(
        name: "COMMENT",
        pattern: ";[^\\n]*",
        isRegex: true,
        lineNumber: 43,
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
  startMode: null,
  transitions: const [],
);
