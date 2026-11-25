// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/matlab/matlab.tokens
// Regenerate with: grammar-tools compile-tokens <source.tokens>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final tokenGrammar = TokenGrammar(
  version: 0,
  caseInsensitive: false,
  definitions: [
    TokenDefinition(
        name: "ELEM_POW",
        pattern: ".^",
        isRegex: false,
        lineNumber: 32,
        alias: null),
    TokenDefinition(
        name: "ELEM_MUL",
        pattern: ".*",
        isRegex: false,
        lineNumber: 33,
        alias: null),
    TokenDefinition(
        name: "ELEM_RDIV",
        pattern: "./",
        isRegex: false,
        lineNumber: 34,
        alias: null),
    TokenDefinition(
        name: "ELEM_LDIV",
        pattern: "\\.\\\\",
        isRegex: true,
        lineNumber: 35,
        alias: null),
    TokenDefinition(
        name: "ELEM_TRANSPOSE",
        pattern: ".'",
        isRegex: false,
        lineNumber: 36,
        alias: null),
    TokenDefinition(
        name: "EQ_EQ",
        pattern: "==",
        isRegex: false,
        lineNumber: 38,
        alias: null),
    TokenDefinition(
        name: "NE", pattern: "~=", isRegex: false, lineNumber: 39, alias: null),
    TokenDefinition(
        name: "LE", pattern: "<=", isRegex: false, lineNumber: 40, alias: null),
    TokenDefinition(
        name: "GE", pattern: ">=", isRegex: false, lineNumber: 41, alias: null),
    TokenDefinition(
        name: "AND_AND",
        pattern: "&&",
        isRegex: false,
        lineNumber: 42,
        alias: null),
    TokenDefinition(
        name: "OR_OR",
        pattern: "||",
        isRegex: false,
        lineNumber: 43,
        alias: null),
    TokenDefinition(
        name: "CHARARRAY",
        pattern: "`[0-9]+`",
        isRegex: true,
        lineNumber: 57,
        alias: null),
    TokenDefinition(
        name: "DQ_STRING",
        pattern: "\"([^\"]|\"\")*\"",
        isRegex: true,
        lineNumber: 58,
        alias: "STRING"),
    TokenDefinition(
        name: "NUMBER",
        pattern: "([0-9]+(\\.[0-9]+)?|\\.[0-9]+)([eE][+-]?[0-9]+)?",
        isRegex: true,
        lineNumber: 68,
        alias: null),
    TokenDefinition(
        name: "NAME",
        pattern: "[A-Za-z_][A-Za-z0-9_]*",
        isRegex: true,
        lineNumber: 79,
        alias: null),
    TokenDefinition(
        name: "TRANSPOSE",
        pattern: "'",
        isRegex: false,
        lineNumber: 111,
        alias: null),
    TokenDefinition(
        name: "PLUS",
        pattern: "+",
        isRegex: false,
        lineNumber: 112,
        alias: null),
    TokenDefinition(
        name: "MINUS",
        pattern: "-",
        isRegex: false,
        lineNumber: 113,
        alias: null),
    TokenDefinition(
        name: "STAR",
        pattern: "*",
        isRegex: false,
        lineNumber: 114,
        alias: null),
    TokenDefinition(
        name: "SLASH",
        pattern: "/",
        isRegex: false,
        lineNumber: 115,
        alias: null),
    TokenDefinition(
        name: "BACKSLASH",
        pattern: "\\\\",
        isRegex: true,
        lineNumber: 116,
        alias: null),
    TokenDefinition(
        name: "CARET",
        pattern: "^",
        isRegex: false,
        lineNumber: 117,
        alias: null),
    TokenDefinition(
        name: "LT", pattern: "<", isRegex: false, lineNumber: 118, alias: null),
    TokenDefinition(
        name: "GT", pattern: ">", isRegex: false, lineNumber: 119, alias: null),
    TokenDefinition(
        name: "EQ", pattern: "=", isRegex: false, lineNumber: 120, alias: null),
    TokenDefinition(
        name: "TILDE",
        pattern: "~",
        isRegex: false,
        lineNumber: 121,
        alias: null),
    TokenDefinition(
        name: "AMP",
        pattern: "&",
        isRegex: false,
        lineNumber: 122,
        alias: null),
    TokenDefinition(
        name: "PIPE",
        pattern: "|",
        isRegex: false,
        lineNumber: 123,
        alias: null),
    TokenDefinition(
        name: "COLON",
        pattern: ":",
        isRegex: false,
        lineNumber: 124,
        alias: null),
    TokenDefinition(
        name: "DOT",
        pattern: ".",
        isRegex: false,
        lineNumber: 125,
        alias: null),
    TokenDefinition(
        name: "AT", pattern: "@", isRegex: false, lineNumber: 126, alias: null),
    TokenDefinition(
        name: "LPAREN",
        pattern: "(",
        isRegex: false,
        lineNumber: 127,
        alias: null),
    TokenDefinition(
        name: "RPAREN",
        pattern: ")",
        isRegex: false,
        lineNumber: 128,
        alias: null),
    TokenDefinition(
        name: "LBRACKET",
        pattern: "[",
        isRegex: false,
        lineNumber: 129,
        alias: null),
    TokenDefinition(
        name: "RBRACKET",
        pattern: "]",
        isRegex: false,
        lineNumber: 130,
        alias: null),
    TokenDefinition(
        name: "LBRACE",
        pattern: "{",
        isRegex: false,
        lineNumber: 131,
        alias: null),
    TokenDefinition(
        name: "RBRACE",
        pattern: "}",
        isRegex: false,
        lineNumber: 132,
        alias: null),
    TokenDefinition(
        name: "COMMA",
        pattern: ",",
        isRegex: false,
        lineNumber: 133,
        alias: null),
    TokenDefinition(
        name: "SEMICOLON",
        pattern: ";",
        isRegex: false,
        lineNumber: 134,
        alias: null),
    TokenDefinition(
        name: "NEWLINE",
        pattern: "\\r?\\n",
        isRegex: true,
        lineNumber: 145,
        alias: null),
  ],
  keywords: [
    "if",
    "elseif",
    "else",
    "end",
    "for",
    "while",
    "break",
    "continue",
    "switch",
    "case",
    "otherwise",
    "function",
    "return",
    "global",
    "persistent",
    "try",
    "catch"
  ],
  mode: null,
  skipDefinitions: [
    TokenDefinition(
        name: "WHITESPACE",
        pattern: "[ \\t]+",
        isRegex: true,
        lineNumber: 153,
        alias: null),
    TokenDefinition(
        name: "COMMENT",
        pattern: "%[^\\n]*",
        isRegex: true,
        lineNumber: 154,
        alias: null),
  ],
  reservedKeywords: const [],
  escapeMode: null,
  errorDefinitions: [
    TokenDefinition(
        name: "UNKNOWN",
        pattern: ".",
        isRegex: true,
        lineNumber: 161,
        alias: null),
  ],
  groups: const {},
  caseSensitive: true,
  layoutKeywords: const [],
  contextKeywords: const [],
  softKeywords: const [],
);
