// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/prolog/iso.tokens
// Regenerate with: grammar-tools compile-tokens <source.tokens>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final tokenGrammar = TokenGrammar(
  version: 1,
  caseInsensitive: false,
  definitions: [
    TokenDefinition(
        name: "DCG",
        pattern: "-->",
        isRegex: false,
        lineNumber: 32,
        alias: null),
    TokenDefinition(
        name: "QUERY",
        pattern: "?-",
        isRegex: false,
        lineNumber: 33,
        alias: null),
    TokenDefinition(
        name: "RULE",
        pattern: ":-",
        isRegex: false,
        lineNumber: 34,
        alias: null),
    TokenDefinition(
        name: "NAF",
        pattern: "\\+",
        isRegex: false,
        lineNumber: 35,
        alias: null),
    TokenDefinition(
        name: "PROB_SEP",
        pattern: "::",
        isRegex: false,
        lineNumber: 36,
        alias: null),
    TokenDefinition(
        name: "LPAREN",
        pattern: "(",
        isRegex: false,
        lineNumber: 39,
        alias: null),
    TokenDefinition(
        name: "RPAREN",
        pattern: ")",
        isRegex: false,
        lineNumber: 40,
        alias: null),
    TokenDefinition(
        name: "LBRACKET",
        pattern: "[",
        isRegex: false,
        lineNumber: 41,
        alias: null),
    TokenDefinition(
        name: "RBRACKET",
        pattern: "]",
        isRegex: false,
        lineNumber: 42,
        alias: null),
    TokenDefinition(
        name: "LCURLY",
        pattern: "{",
        isRegex: false,
        lineNumber: 43,
        alias: null),
    TokenDefinition(
        name: "RCURLY",
        pattern: "}",
        isRegex: false,
        lineNumber: 44,
        alias: null),
    TokenDefinition(
        name: "BAR", pattern: "|", isRegex: false, lineNumber: 45, alias: null),
    TokenDefinition(
        name: "COMMA",
        pattern: ",",
        isRegex: false,
        lineNumber: 46,
        alias: null),
    TokenDefinition(
        name: "SEMICOLON",
        pattern: ";",
        isRegex: false,
        lineNumber: 47,
        alias: null),
    TokenDefinition(
        name: "CUT", pattern: "!", isRegex: false, lineNumber: 48, alias: null),
    TokenDefinition(
        name: "DOT", pattern: ".", isRegex: false, lineNumber: 49, alias: null),
    TokenDefinition(
        name: "FLOAT",
        pattern: "[0-9]+\\.[0-9]+([eE][-+]?[0-9]+)?",
        isRegex: true,
        lineNumber: 52,
        alias: null),
    TokenDefinition(
        name: "INTEGER",
        pattern: "[0-9]+",
        isRegex: true,
        lineNumber: 53,
        alias: null),
    TokenDefinition(
        name: "STRING",
        pattern: "\"([^\"\\\\]|\\\\.)*\"",
        isRegex: true,
        lineNumber: 56,
        alias: null),
    TokenDefinition(
        name: "QUOTED_ATOM",
        pattern: "'([^'\\\\]|\\\\.)*'",
        isRegex: true,
        lineNumber: 57,
        alias: "ATOM"),
    TokenDefinition(
        name: "ANON_VAR",
        pattern: "_(?![A-Za-z0-9_])",
        isRegex: true,
        lineNumber: 64,
        alias: null),
    TokenDefinition(
        name: "VARIABLE",
        pattern: "(?:[A-Z][A-Za-z0-9_]*|_[A-Za-z0-9_]+)",
        isRegex: true,
        lineNumber: 65,
        alias: null),
    TokenDefinition(
        name: "ATOM",
        pattern: "[a-z][A-Za-z0-9_]*",
        isRegex: true,
        lineNumber: 66,
        alias: null),
    TokenDefinition(
        name: "ATOM_SYMBOLIC",
        pattern: "[#\$&*+\\-\\/:<=>?@\\\\^~]+",
        isRegex: true,
        lineNumber: 71,
        alias: "ATOM"),
  ],
  keywords: const [],
  mode: null,
  skipDefinitions: [
    TokenDefinition(
        name: "WHITESPACE",
        pattern: "[ \\t\\r\\n]+",
        isRegex: true,
        lineNumber: 22,
        alias: null),
    TokenDefinition(
        name: "LINE_COMMENT",
        pattern: "%[^\\n]*",
        isRegex: true,
        lineNumber: 23,
        alias: null),
    TokenDefinition(
        name: "BLOCK_COMMENT",
        pattern: "\\/\\*[\\s\\S]*?\\*\\/",
        isRegex: true,
        lineNumber: 26,
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
