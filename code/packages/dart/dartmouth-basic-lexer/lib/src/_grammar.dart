// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/dartmouth_basic/dartmouth_basic.tokens
// Regenerate with: grammar-tools compile-tokens <source.tokens>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final tokenGrammar = TokenGrammar(
  version: 1,
  caseInsensitive: true,
  definitions: [
    TokenDefinition(
        name: "LE", pattern: "<=", isRegex: false, lineNumber: 50, alias: null),
    TokenDefinition(
        name: "GE", pattern: ">=", isRegex: false, lineNumber: 51, alias: null),
    TokenDefinition(
        name: "NE", pattern: "<>", isRegex: false, lineNumber: 52, alias: null),
    TokenDefinition(
        name: "NUMBER",
        pattern: "[0-9]*\\.?[0-9]+([Ee][+-]?[0-9]+)?",
        isRegex: true,
        lineNumber: 85,
        alias: null),
    TokenDefinition(
        name: "LINE_NUM",
        pattern: "[0-9]+",
        isRegex: true,
        lineNumber: 86,
        alias: null),
    TokenDefinition(
        name: "STRING_BODY",
        pattern: "\"[^\"]*\"",
        isRegex: true,
        lineNumber: 112,
        alias: "STRING"),
    TokenDefinition(
        name: "BUILTIN_FN",
        pattern: "(?:sin|cos|tan|atn|exp|log|abs|sqr|int|rnd|sgn)",
        isRegex: true,
        lineNumber: 168,
        alias: null),
    TokenDefinition(
        name: "USER_FN",
        pattern: "fn[a-z]",
        isRegex: true,
        lineNumber: 169,
        alias: null),
    TokenDefinition(
        name: "NAME",
        pattern: "[a-z][a-z0-9]*\\\$?",
        isRegex: true,
        lineNumber: 204,
        alias: null),
    TokenDefinition(
        name: "PLUS",
        pattern: "+",
        isRegex: false,
        lineNumber: 244,
        alias: null),
    TokenDefinition(
        name: "MINUS",
        pattern: "-",
        isRegex: false,
        lineNumber: 245,
        alias: null),
    TokenDefinition(
        name: "STAR",
        pattern: "*",
        isRegex: false,
        lineNumber: 246,
        alias: null),
    TokenDefinition(
        name: "SLASH",
        pattern: "/",
        isRegex: false,
        lineNumber: 247,
        alias: null),
    TokenDefinition(
        name: "CARET",
        pattern: "^",
        isRegex: false,
        lineNumber: 248,
        alias: null),
    TokenDefinition(
        name: "EQ", pattern: "=", isRegex: false, lineNumber: 249, alias: null),
    TokenDefinition(
        name: "LT", pattern: "<", isRegex: false, lineNumber: 250, alias: null),
    TokenDefinition(
        name: "GT", pattern: ">", isRegex: false, lineNumber: 251, alias: null),
    TokenDefinition(
        name: "LPAREN",
        pattern: "(",
        isRegex: false,
        lineNumber: 252,
        alias: null),
    TokenDefinition(
        name: "RPAREN",
        pattern: ")",
        isRegex: false,
        lineNumber: 253,
        alias: null),
    TokenDefinition(
        name: "COMMA",
        pattern: ",",
        isRegex: false,
        lineNumber: 254,
        alias: null),
    TokenDefinition(
        name: "SEMICOLON",
        pattern: ";",
        isRegex: false,
        lineNumber: 255,
        alias: null),
    TokenDefinition(
        name: "NEWLINE",
        pattern: "\\r?\\n",
        isRegex: true,
        lineNumber: 276,
        alias: null),
  ],
  keywords: [
    "LET",
    "PRINT",
    "INPUT",
    "IF",
    "THEN",
    "GOTO",
    "GOSUB",
    "RETURN",
    "FOR",
    "TO",
    "STEP",
    "NEXT",
    "END",
    "STOP",
    "REM",
    "READ",
    "DATA",
    "RESTORE",
    "DIM",
    "DEF"
  ],
  mode: null,
  skipDefinitions: [
    TokenDefinition(
        name: "WHITESPACE",
        pattern: "[ \\t]+",
        isRegex: true,
        lineNumber: 288,
        alias: null),
  ],
  reservedKeywords: const [],
  escapeMode: null,
  errorDefinitions: [
    TokenDefinition(
        name: "UNKNOWN",
        pattern: ".",
        isRegex: true,
        lineNumber: 304,
        alias: null),
  ],
  groups: const {},
  caseSensitive: false,
  layoutKeywords: const [],
  contextKeywords: const [],
  softKeywords: const [],
);
