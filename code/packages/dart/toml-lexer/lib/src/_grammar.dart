// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/toml/toml.tokens
// Regenerate with: grammar-tools compile-tokens <source.tokens>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final tokenGrammar = TokenGrammar(
  version: 1,
  caseInsensitive: false,
  definitions: [
    TokenDefinition(
        name: "ML_BASIC_STRING",
        pattern: "\"\"\"([^\\\\]|\\\\(.|\\n)|\\n)*?\"\"\"",
        isRegex: true,
        lineNumber: 60,
        alias: null),
    TokenDefinition(
        name: "ML_LITERAL_STRING",
        pattern: "'''[\\s\\S]*?'''",
        isRegex: true,
        lineNumber: 61,
        alias: null),
    TokenDefinition(
        name: "BASIC_STRING",
        pattern: "\"([^\"\\\\\\n]|\\\\.)*\"",
        isRegex: true,
        lineNumber: 70,
        alias: null),
    TokenDefinition(
        name: "LITERAL_STRING",
        pattern: "'[^'\\n]*'",
        isRegex: true,
        lineNumber: 71,
        alias: null),
    TokenDefinition(
        name: "OFFSET_DATETIME_FRAC_TZ",
        pattern:
            "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}\\.\\d+[+-]\\d{2}:\\d{2}",
        isRegex: true,
        lineNumber: 91,
        alias: "OFFSET_DATETIME"),
    TokenDefinition(
        name: "OFFSET_DATETIME_FRAC_Z",
        pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}\\.\\d+Z",
        isRegex: true,
        lineNumber: 92,
        alias: "OFFSET_DATETIME"),
    TokenDefinition(
        name: "OFFSET_DATETIME_TZ",
        pattern:
            "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}[+-]\\d{2}:\\d{2}",
        isRegex: true,
        lineNumber: 93,
        alias: "OFFSET_DATETIME"),
    TokenDefinition(
        name: "OFFSET_DATETIME_Z",
        pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}Z",
        isRegex: true,
        lineNumber: 94,
        alias: "OFFSET_DATETIME"),
    TokenDefinition(
        name: "LOCAL_DATETIME_FRAC",
        pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}\\.\\d+",
        isRegex: true,
        lineNumber: 95,
        alias: "LOCAL_DATETIME"),
    TokenDefinition(
        name: "LOCAL_DATETIME",
        pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}",
        isRegex: true,
        lineNumber: 96,
        alias: null),
    TokenDefinition(
        name: "LOCAL_DATE",
        pattern: "\\d{4}-\\d{2}-\\d{2}",
        isRegex: true,
        lineNumber: 97,
        alias: null),
    TokenDefinition(
        name: "LOCAL_TIME_FRAC",
        pattern: "\\d{2}:\\d{2}:\\d{2}\\.\\d+",
        isRegex: true,
        lineNumber: 98,
        alias: "LOCAL_TIME"),
    TokenDefinition(
        name: "LOCAL_TIME",
        pattern: "\\d{2}:\\d{2}:\\d{2}",
        isRegex: true,
        lineNumber: 99,
        alias: null),
    TokenDefinition(
        name: "FLOAT_INF",
        pattern: "[+-]?inf",
        isRegex: true,
        lineNumber: 114,
        alias: "FLOAT"),
    TokenDefinition(
        name: "FLOAT_NAN",
        pattern: "[+-]?nan",
        isRegex: true,
        lineNumber: 115,
        alias: "FLOAT"),
    TokenDefinition(
        name: "FLOAT_EXP",
        pattern: "[+-]?[0-9][0-9_]*\\.?[0-9_]*[eE][+-]?[0-9][0-9_]*",
        isRegex: true,
        lineNumber: 116,
        alias: "FLOAT"),
    TokenDefinition(
        name: "FLOAT_DEC",
        pattern: "[+-]?[0-9][0-9_]*\\.[0-9][0-9_]*",
        isRegex: true,
        lineNumber: 117,
        alias: "FLOAT"),
    TokenDefinition(
        name: "HEX_INTEGER",
        pattern: "0x[0-9a-fA-F][0-9a-fA-F_]*",
        isRegex: true,
        lineNumber: 129,
        alias: "INTEGER"),
    TokenDefinition(
        name: "OCT_INTEGER",
        pattern: "0o[0-7][0-7_]*",
        isRegex: true,
        lineNumber: 130,
        alias: "INTEGER"),
    TokenDefinition(
        name: "BIN_INTEGER",
        pattern: "0b[01][01_]*",
        isRegex: true,
        lineNumber: 131,
        alias: "INTEGER"),
    TokenDefinition(
        name: "INTEGER",
        pattern: "[+-]?[0-9][0-9_]*",
        isRegex: true,
        lineNumber: 132,
        alias: null),
    TokenDefinition(
        name: "TRUE",
        pattern: "true",
        isRegex: false,
        lineNumber: 143,
        alias: null),
    TokenDefinition(
        name: "FALSE",
        pattern: "false",
        isRegex: false,
        lineNumber: 144,
        alias: null),
    TokenDefinition(
        name: "BARE_KEY",
        pattern: "[A-Za-z0-9_-]+",
        isRegex: true,
        lineNumber: 158,
        alias: null),
    TokenDefinition(
        name: "EQUALS",
        pattern: "=",
        isRegex: false,
        lineNumber: 168,
        alias: null),
    TokenDefinition(
        name: "DOT",
        pattern: ".",
        isRegex: false,
        lineNumber: 169,
        alias: null),
    TokenDefinition(
        name: "COMMA",
        pattern: ",",
        isRegex: false,
        lineNumber: 170,
        alias: null),
    TokenDefinition(
        name: "LBRACKET",
        pattern: "[",
        isRegex: false,
        lineNumber: 171,
        alias: null),
    TokenDefinition(
        name: "RBRACKET",
        pattern: "]",
        isRegex: false,
        lineNumber: 172,
        alias: null),
    TokenDefinition(
        name: "LBRACE",
        pattern: "{",
        isRegex: false,
        lineNumber: 173,
        alias: null),
    TokenDefinition(
        name: "RBRACE",
        pattern: "}",
        isRegex: false,
        lineNumber: 174,
        alias: null),
    TokenDefinition(
        name: "NEWLINE",
        pattern: "\\r?\\n",
        isRegex: true,
        lineNumber: 175,
        alias: null),
  ],
  keywords: const [],
  mode: null,
  skipDefinitions: [
    TokenDefinition(
        name: "COMMENT",
        pattern: "#[^\\n]*",
        isRegex: true,
        lineNumber: 28,
        alias: null),
    TokenDefinition(
        name: "WHITESPACE",
        pattern: "[ \\t]+",
        isRegex: true,
        lineNumber: 29,
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
