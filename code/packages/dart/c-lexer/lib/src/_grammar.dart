// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/c/c.tokens
// Regenerate with: grammar-tools compile-tokens <source.tokens>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final tokenGrammar = TokenGrammar(
  version: 1,
  caseInsensitive: false,
  definitions: [
    TokenDefinition(
        name: "EQ_EQ",
        pattern: "==",
        isRegex: false,
        lineNumber: 25,
        alias: null),
    TokenDefinition(
        name: "HASH_HASH",
        pattern: "##",
        isRegex: false,
        lineNumber: 26,
        alias: null),
    TokenDefinition(
        name: "NE", pattern: "!=", isRegex: false, lineNumber: 27, alias: null),
    TokenDefinition(
        name: "LE", pattern: "<=", isRegex: false, lineNumber: 28, alias: null),
    TokenDefinition(
        name: "GE", pattern: ">=", isRegex: false, lineNumber: 29, alias: null),
    TokenDefinition(
        name: "SHL",
        pattern: "<<",
        isRegex: false,
        lineNumber: 30,
        alias: null),
    TokenDefinition(
        name: "SHR",
        pattern: ">>",
        isRegex: false,
        lineNumber: 31,
        alias: null),
    TokenDefinition(
        name: "AND_AND",
        pattern: "&&",
        isRegex: false,
        lineNumber: 32,
        alias: null),
    TokenDefinition(
        name: "OR_OR",
        pattern: "||",
        isRegex: false,
        lineNumber: 33,
        alias: null),
    TokenDefinition(
        name: "FLOAT_LIT",
        pattern:
            "(([0-9]+\\.[0-9]*|\\.[0-9]+)([eE][+-]?[0-9]+)?|[0-9]+[eE][+-]?[0-9]+)[fFlL]?",
        isRegex: true,
        lineNumber: 43,
        alias: null),
    TokenDefinition(
        name: "INT_LIT",
        pattern: "(0[xX][0-9a-fA-F]+|[0-9]+)([uUlL]*)",
        isRegex: true,
        lineNumber: 48,
        alias: null),
    TokenDefinition(
        name: "CHAR_LIT",
        pattern: "'([^'\\\\]|\\\\.)'",
        isRegex: true,
        lineNumber: 52,
        alias: null),
    TokenDefinition(
        name: "STR_LIT",
        pattern: "\"([^\"\\\\]|\\\\.)*\"",
        isRegex: true,
        lineNumber: 55,
        alias: null),
    TokenDefinition(
        name: "NAME",
        pattern: "[A-Za-z_][A-Za-z0-9_]*",
        isRegex: true,
        lineNumber: 90,
        alias: null),
    TokenDefinition(
        name: "PLUS",
        pattern: "+",
        isRegex: false,
        lineNumber: 95,
        alias: null),
    TokenDefinition(
        name: "MINUS",
        pattern: "-",
        isRegex: false,
        lineNumber: 96,
        alias: null),
    TokenDefinition(
        name: "STAR",
        pattern: "*",
        isRegex: false,
        lineNumber: 97,
        alias: null),
    TokenDefinition(
        name: "SLASH",
        pattern: "/",
        isRegex: false,
        lineNumber: 98,
        alias: null),
    TokenDefinition(
        name: "PERCENT",
        pattern: "%",
        isRegex: false,
        lineNumber: 99,
        alias: null),
    TokenDefinition(
        name: "AMP",
        pattern: "&",
        isRegex: false,
        lineNumber: 100,
        alias: null),
    TokenDefinition(
        name: "PIPE",
        pattern: "|",
        isRegex: false,
        lineNumber: 101,
        alias: null),
    TokenDefinition(
        name: "CARET",
        pattern: "^",
        isRegex: false,
        lineNumber: 102,
        alias: null),
    TokenDefinition(
        name: "TILDE",
        pattern: "~",
        isRegex: false,
        lineNumber: 103,
        alias: null),
    TokenDefinition(
        name: "BANG",
        pattern: "!",
        isRegex: false,
        lineNumber: 104,
        alias: null),
    TokenDefinition(
        name: "LT", pattern: "<", isRegex: false, lineNumber: 105, alias: null),
    TokenDefinition(
        name: "GT", pattern: ">", isRegex: false, lineNumber: 106, alias: null),
    TokenDefinition(
        name: "EQ", pattern: "=", isRegex: false, lineNumber: 107, alias: null),
    TokenDefinition(
        name: "LPAREN",
        pattern: "(",
        isRegex: false,
        lineNumber: 108,
        alias: null),
    TokenDefinition(
        name: "RPAREN",
        pattern: ")",
        isRegex: false,
        lineNumber: 109,
        alias: null),
    TokenDefinition(
        name: "LBRACE",
        pattern: "{",
        isRegex: false,
        lineNumber: 110,
        alias: null),
    TokenDefinition(
        name: "RBRACE",
        pattern: "}",
        isRegex: false,
        lineNumber: 111,
        alias: null),
    TokenDefinition(
        name: "LBRACKET",
        pattern: "[",
        isRegex: false,
        lineNumber: 112,
        alias: null),
    TokenDefinition(
        name: "RBRACKET",
        pattern: "]",
        isRegex: false,
        lineNumber: 113,
        alias: null),
    TokenDefinition(
        name: "SEMICOLON",
        pattern: ";",
        isRegex: false,
        lineNumber: 114,
        alias: null),
    TokenDefinition(
        name: "COMMA",
        pattern: ",",
        isRegex: false,
        lineNumber: 115,
        alias: null),
    TokenDefinition(
        name: "HASH",
        pattern: "#",
        isRegex: false,
        lineNumber: 116,
        alias: null),
    TokenDefinition(
        name: "DOT",
        pattern: ".",
        isRegex: false,
        lineNumber: 117,
        alias: null),
  ],
  keywords: [
    "void",
    "char",
    "short",
    "int",
    "long",
    "signed",
    "unsigned",
    "float",
    "double",
    "int8_t",
    "int16_t",
    "int32_t",
    "int64_t",
    "uint8_t",
    "uint16_t",
    "uint32_t",
    "uint64_t",
    "size_t",
    "if",
    "else",
    "while",
    "for",
    "return"
  ],
  mode: null,
  skipDefinitions: [
    TokenDefinition(
        name: "WHITESPACE",
        pattern: "[ \\t\\r\\n]+",
        isRegex: true,
        lineNumber: 123,
        alias: null),
    TokenDefinition(
        name: "LINE_COMMENT",
        pattern: "\\/\\/[^\\n]*",
        isRegex: true,
        lineNumber: 124,
        alias: null),
    TokenDefinition(
        name: "BLOCK_COMMENT",
        pattern: "\\/\\*([^*]|\\*+[^*\\/])*\\*+\\/",
        isRegex: true,
        lineNumber: 125,
        alias: null),
  ],
  reservedKeywords: const [],
  escapeMode: null,
  errorDefinitions: [
    TokenDefinition(
        name: "UNKNOWN",
        pattern: ".",
        isRegex: true,
        lineNumber: 128,
        alias: null),
  ],
  groups: const {},
  caseSensitive: true,
  layoutKeywords: const [],
  contextKeywords: const [],
  softKeywords: const [],
);
