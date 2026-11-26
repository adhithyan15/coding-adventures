// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/q/q.tokens
// Regenerate with: grammar-tools compile-tokens <source.tokens>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final tokenGrammar = TokenGrammar(
  version: 1,
  caseInsensitive: false,
  definitions: [
    TokenDefinition(
        name: "LE",
        pattern: "<=",
        isRegex: false,
        lineNumber: 100,
        alias: null),
    TokenDefinition(
        name: "GE",
        pattern: ">=",
        isRegex: false,
        lineNumber: 101,
        alias: null),
    TokenDefinition(
        name: "NE",
        pattern: "<>",
        isRegex: false,
        lineNumber: 105,
        alias: null),
    TokenDefinition(
        name: "PLUS",
        pattern: "+",
        isRegex: false,
        lineNumber: 135,
        alias: null),
    TokenDefinition(
        name: "MINUS",
        pattern: "-",
        isRegex: false,
        lineNumber: 136,
        alias: null),
    TokenDefinition(
        name: "STAR",
        pattern: "*",
        isRegex: false,
        lineNumber: 137,
        alias: null),
    TokenDefinition(
        name: "PERCENT",
        pattern: "%",
        isRegex: false,
        lineNumber: 138,
        alias: null),
    TokenDefinition(
        name: "BANG",
        pattern: "!",
        isRegex: false,
        lineNumber: 139,
        alias: null),
    TokenDefinition(
        name: "COMMA",
        pattern: ",",
        isRegex: false,
        lineNumber: 140,
        alias: null),
    TokenDefinition(
        name: "HASH",
        pattern: "#",
        isRegex: false,
        lineNumber: 141,
        alias: null),
    TokenDefinition(
        name: "UNDERSCORE",
        pattern: "_",
        isRegex: false,
        lineNumber: 142,
        alias: null),
    TokenDefinition(
        name: "AMP",
        pattern: "&",
        isRegex: false,
        lineNumber: 143,
        alias: null),
    TokenDefinition(
        name: "PIPE",
        pattern: "|",
        isRegex: false,
        lineNumber: 144,
        alias: null),
    TokenDefinition(
        name: "TILDE",
        pattern: "~",
        isRegex: false,
        lineNumber: 145,
        alias: null),
    TokenDefinition(
        name: "EQ", pattern: "=", isRegex: false, lineNumber: 147, alias: null),
    TokenDefinition(
        name: "LT", pattern: "<", isRegex: false, lineNumber: 148, alias: null),
    TokenDefinition(
        name: "GT", pattern: ">", isRegex: false, lineNumber: 149, alias: null),
    TokenDefinition(
        name: "EACH",
        pattern: "'",
        isRegex: false,
        lineNumber: 170,
        alias: null),
    TokenDefinition(
        name: "REDUCE",
        pattern: "/",
        isRegex: false,
        lineNumber: 171,
        alias: null),
    TokenDefinition(
        name: "SCAN",
        pattern: "\\",
        isRegex: false,
        lineNumber: 172,
        alias: null),
    TokenDefinition(
        name: "COLON",
        pattern: ":",
        isRegex: false,
        lineNumber: 174,
        alias: null),
    TokenDefinition(
        name: "SEMICOLON",
        pattern: ";",
        isRegex: false,
        lineNumber: 175,
        alias: null),
    TokenDefinition(
        name: "LPAREN",
        pattern: "(",
        isRegex: false,
        lineNumber: 177,
        alias: null),
    TokenDefinition(
        name: "RPAREN",
        pattern: ")",
        isRegex: false,
        lineNumber: 178,
        alias: null),
    TokenDefinition(
        name: "LBRACE",
        pattern: "{",
        isRegex: false,
        lineNumber: 196,
        alias: null),
    TokenDefinition(
        name: "RBRACE",
        pattern: "}",
        isRegex: false,
        lineNumber: 197,
        alias: null),
    TokenDefinition(
        name: "LBRACKET",
        pattern: "[",
        isRegex: false,
        lineNumber: 198,
        alias: null),
    TokenDefinition(
        name: "RBRACKET",
        pattern: "]",
        isRegex: false,
        lineNumber: 199,
        alias: null),
    TokenDefinition(
        name: "NUMBER",
        pattern: "[0-9]+(\\.[0-9]+)?([eE][+-]?[0-9]+)?",
        isRegex: true,
        lineNumber: 237,
        alias: null),
    TokenDefinition(
        name: "NAME",
        pattern: "[a-zA-Z][a-zA-Z0-9]*",
        isRegex: true,
        lineNumber: 238,
        alias: null),
    TokenDefinition(
        name: "NEWLINE",
        pattern: "\\r?\\n",
        isRegex: true,
        lineNumber: 253,
        alias: null),
  ],
  keywords: const [],
  mode: null,
  skipDefinitions: [
    TokenDefinition(
        name: "WHITESPACE",
        pattern: "[ \\t]+",
        isRegex: true,
        lineNumber: 256,
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
