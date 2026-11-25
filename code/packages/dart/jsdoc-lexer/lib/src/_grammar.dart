// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/jsdoc/jsdoc.tokens
// Regenerate with: grammar-tools compile-tokens <source.tokens>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final tokenGrammar = TokenGrammar(
  version: 1,
  caseInsensitive: false,
  definitions: [
    TokenDefinition(
        name: "AT_TAG",
        pattern: "@[a-zA-Z_\$][a-zA-Z0-9_\$-]*",
        isRegex: true,
        lineNumber: 45,
        alias: null),
    TokenDefinition(
        name: "LBRACE",
        pattern: "{",
        isRegex: false,
        lineNumber: 48,
        alias: null),
    TokenDefinition(
        name: "RBRACE",
        pattern: "}",
        isRegex: false,
        lineNumber: 49,
        alias: null),
    TokenDefinition(
        name: "LBRACKET",
        pattern: "[",
        isRegex: false,
        lineNumber: 50,
        alias: null),
    TokenDefinition(
        name: "RBRACKET",
        pattern: "]",
        isRegex: false,
        lineNumber: 51,
        alias: null),
    TokenDefinition(
        name: "LPAREN",
        pattern: "(",
        isRegex: false,
        lineNumber: 52,
        alias: null),
    TokenDefinition(
        name: "RPAREN",
        pattern: ")",
        isRegex: false,
        lineNumber: 53,
        alias: null),
    TokenDefinition(
        name: "ANGLE_OPEN",
        pattern: "<",
        isRegex: false,
        lineNumber: 54,
        alias: null),
    TokenDefinition(
        name: "ANGLE_CLOSE",
        pattern: ">",
        isRegex: false,
        lineNumber: 55,
        alias: null),
    TokenDefinition(
        name: "PIPE",
        pattern: "|",
        isRegex: false,
        lineNumber: 58,
        alias: null),
    TokenDefinition(
        name: "AMP", pattern: "&", isRegex: false, lineNumber: 59, alias: null),
    TokenDefinition(
        name: "COMMA",
        pattern: ",",
        isRegex: false,
        lineNumber: 60,
        alias: null),
    TokenDefinition(
        name: "COLON",
        pattern: ":",
        isRegex: false,
        lineNumber: 61,
        alias: null),
    TokenDefinition(
        name: "EQUALS",
        pattern: "=",
        isRegex: false,
        lineNumber: 62,
        alias: null),
    TokenDefinition(
        name: "ELLIPSIS",
        pattern: "...",
        isRegex: false,
        lineNumber: 63,
        alias: null),
    TokenDefinition(
        name: "QUESTION",
        pattern: "?",
        isRegex: false,
        lineNumber: 64,
        alias: null),
    TokenDefinition(
        name: "BANG",
        pattern: "!",
        isRegex: false,
        lineNumber: 65,
        alias: null),
    TokenDefinition(
        name: "STAR",
        pattern: "*",
        isRegex: false,
        lineNumber: 66,
        alias: null),
    TokenDefinition(
        name: "ARROW",
        pattern: "=>",
        isRegex: false,
        lineNumber: 67,
        alias: null),
    TokenDefinition(
        name: "DOT", pattern: ".", isRegex: false, lineNumber: 68, alias: null),
    TokenDefinition(
        name: "NEWLINE",
        pattern: "\\n",
        isRegex: true,
        lineNumber: 72,
        alias: null),
    TokenDefinition(
        name: "NAME",
        pattern: "[a-zA-Z_\$][a-zA-Z0-9_\$]*",
        isRegex: true,
        lineNumber: 79,
        alias: null),
    TokenDefinition(
        name: "NUMBER",
        pattern: "-?[0-9]+(\\.[0-9]+)?([eE][+-]?[0-9]+)?",
        isRegex: true,
        lineNumber: 82,
        alias: null),
    TokenDefinition(
        name: "STRING_DQ",
        pattern: "\"([^\"\\\\]|\\\\.)*\"",
        isRegex: true,
        lineNumber: 85,
        alias: "STRING"),
    TokenDefinition(
        name: "STRING_SQ",
        pattern: "'([^'\\\\]|\\\\.)*'",
        isRegex: true,
        lineNumber: 86,
        alias: "STRING"),
    TokenDefinition(
        name: "DESCRIPTION_TEXT",
        pattern: "[a-zA-Z_\$0-9 \\t][^\\n@{}\\[\\]()<>|&,:=?!*]*",
        isRegex: true,
        lineNumber: 100,
        alias: null),
  ],
  keywords: const [],
  mode: null,
  skipDefinitions: [
    TokenDefinition(
        name: "HSPACE",
        pattern: "[ \\t]+",
        isRegex: true,
        lineNumber: 109,
        alias: null),
    TokenDefinition(
        name: "LEADING_STAR",
        pattern: "\\n[ \\t]*\\*[ \\t]?",
        isRegex: true,
        lineNumber: 117,
        alias: null),
  ],
  reservedKeywords: const [],
  escapeMode: null,
  errorDefinitions: [
    TokenDefinition(
        name: "BAD_STRING_DQ",
        pattern: "\"[^\"\\n]*\$",
        isRegex: true,
        lineNumber: 124,
        alias: null),
    TokenDefinition(
        name: "BAD_STRING_SQ",
        pattern: "'[^'\\n]*\$",
        isRegex: true,
        lineNumber: 125,
        alias: null),
    TokenDefinition(
        name: "UNTERMINATED_TYPE",
        pattern: "\\{[^}]*\$",
        isRegex: true,
        lineNumber: 126,
        alias: null),
  ],
  groups: const {},
  caseSensitive: true,
  layoutKeywords: const [],
  contextKeywords: const [],
  softKeywords: const [],
);
