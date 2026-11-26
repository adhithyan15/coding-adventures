// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/s/s.tokens
// Regenerate with: grammar-tools compile-tokens <source.tokens>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final tokenGrammar = TokenGrammar(
  version: 0,
  caseInsensitive: false,
  definitions: [
    TokenDefinition(
        name: "SUPER_ASSIGN",
        pattern: "<<-",
        isRegex: false,
        lineNumber: 39,
        alias: null),
    TokenDefinition(
        name: "ASSIGN",
        pattern: "<-",
        isRegex: false,
        lineNumber: 40,
        alias: null),
    TokenDefinition(
        name: "LE", pattern: "<=", isRegex: false, lineNumber: 41, alias: null),
    TokenDefinition(
        name: "GE", pattern: ">=", isRegex: false, lineNumber: 42, alias: null),
    TokenDefinition(
        name: "EQEQ",
        pattern: "==",
        isRegex: false,
        lineNumber: 43,
        alias: null),
    TokenDefinition(
        name: "NE", pattern: "!=", isRegex: false, lineNumber: 44, alias: null),
    TokenDefinition(
        name: "RIGHT_ASSIGN",
        pattern: "->",
        isRegex: false,
        lineNumber: 45,
        alias: null),
    TokenDefinition(
        name: "PERCENT_OP",
        pattern: "%[^%]*%",
        isRegex: true,
        lineNumber: 50,
        alias: null),
    TokenDefinition(
        name: "NUMBER",
        pattern: "[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?",
        isRegex: true,
        lineNumber: 65,
        alias: null),
    TokenDefinition(
        name: "DQ_STRING",
        pattern: "\"[^\"]*\"",
        isRegex: true,
        lineNumber: 76,
        alias: "STRING"),
    TokenDefinition(
        name: "SQ_STRING",
        pattern: "'[^']*'",
        isRegex: true,
        lineNumber: 77,
        alias: "STRING"),
    TokenDefinition(
        name: "NAME",
        pattern: "[A-Za-z.][A-Za-z0-9.]*",
        isRegex: true,
        lineNumber: 93,
        alias: null),
    TokenDefinition(
        name: "LT", pattern: "<", isRegex: false, lineNumber: 123, alias: null),
    TokenDefinition(
        name: "GT", pattern: ">", isRegex: false, lineNumber: 124, alias: null),
    TokenDefinition(
        name: "EQ", pattern: "=", isRegex: false, lineNumber: 125, alias: null),
    TokenDefinition(
        name: "PLUS",
        pattern: "+",
        isRegex: false,
        lineNumber: 126,
        alias: null),
    TokenDefinition(
        name: "MINUS",
        pattern: "-",
        isRegex: false,
        lineNumber: 127,
        alias: null),
    TokenDefinition(
        name: "STAR",
        pattern: "*",
        isRegex: false,
        lineNumber: 128,
        alias: null),
    TokenDefinition(
        name: "SLASH",
        pattern: "/",
        isRegex: false,
        lineNumber: 129,
        alias: null),
    TokenDefinition(
        name: "CARET",
        pattern: "^",
        isRegex: false,
        lineNumber: 130,
        alias: null),
    TokenDefinition(
        name: "COLON",
        pattern: ":",
        isRegex: false,
        lineNumber: 131,
        alias: null),
    TokenDefinition(
        name: "UNDERSCORE",
        pattern: "_",
        isRegex: false,
        lineNumber: 132,
        alias: null),
    TokenDefinition(
        name: "LPAREN",
        pattern: "(",
        isRegex: false,
        lineNumber: 133,
        alias: null),
    TokenDefinition(
        name: "RPAREN",
        pattern: ")",
        isRegex: false,
        lineNumber: 134,
        alias: null),
    TokenDefinition(
        name: "LBRACE",
        pattern: "{",
        isRegex: false,
        lineNumber: 135,
        alias: null),
    TokenDefinition(
        name: "RBRACE",
        pattern: "}",
        isRegex: false,
        lineNumber: 136,
        alias: null),
    TokenDefinition(
        name: "LBRACKET",
        pattern: "[",
        isRegex: false,
        lineNumber: 137,
        alias: null),
    TokenDefinition(
        name: "RBRACKET",
        pattern: "]",
        isRegex: false,
        lineNumber: 138,
        alias: null),
    TokenDefinition(
        name: "COMMA",
        pattern: ",",
        isRegex: false,
        lineNumber: 139,
        alias: null),
    TokenDefinition(
        name: "SEMICOLON",
        pattern: ";",
        isRegex: false,
        lineNumber: 140,
        alias: null),
    TokenDefinition(
        name: "DOLLAR",
        pattern: "\$",
        isRegex: false,
        lineNumber: 141,
        alias: null),
    TokenDefinition(
        name: "NEWLINE",
        pattern: "\\r?\\n",
        isRegex: true,
        lineNumber: 151,
        alias: null),
  ],
  keywords: [
    "if",
    "else",
    "for",
    "while",
    "repeat",
    "function",
    "break",
    "next",
    "in",
    "TRUE",
    "FALSE",
    "T",
    "F",
    "NULL",
    "NA",
    "Inf",
    "NaN"
  ],
  mode: null,
  skipDefinitions: [
    TokenDefinition(
        name: "WHITESPACE",
        pattern: "[ \\t]+",
        isRegex: true,
        lineNumber: 161,
        alias: null),
    TokenDefinition(
        name: "COMMENT",
        pattern: "#[^\\n]*",
        isRegex: true,
        lineNumber: 162,
        alias: null),
  ],
  reservedKeywords: const [],
  escapeMode: null,
  errorDefinitions: [
    TokenDefinition(
        name: "UNKNOWN",
        pattern: ".",
        isRegex: true,
        lineNumber: 172,
        alias: null),
  ],
  groups: const {},
  caseSensitive: true,
  layoutKeywords: const [],
  contextKeywords: const [],
  softKeywords: const [],
);
