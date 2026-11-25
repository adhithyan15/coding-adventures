// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/r/r.tokens
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
        lineNumber: 30,
        alias: null),
    TokenDefinition(
        name: "ASSIGN",
        pattern: "<-",
        isRegex: false,
        lineNumber: 31,
        alias: null),
    TokenDefinition(
        name: "LE", pattern: "<=", isRegex: false, lineNumber: 32, alias: null),
    TokenDefinition(
        name: "GE", pattern: ">=", isRegex: false, lineNumber: 33, alias: null),
    TokenDefinition(
        name: "EQEQ",
        pattern: "==",
        isRegex: false,
        lineNumber: 34,
        alias: null),
    TokenDefinition(
        name: "NE", pattern: "!=", isRegex: false, lineNumber: 35, alias: null),
    TokenDefinition(
        name: "RIGHT_SUPER_ASSIGN",
        pattern: "->>",
        isRegex: false,
        lineNumber: 36,
        alias: null),
    TokenDefinition(
        name: "RIGHT_ASSIGN",
        pattern: "->",
        isRegex: false,
        lineNumber: 37,
        alias: null),
    TokenDefinition(
        name: "PIPE_OP",
        pattern: "|>",
        isRegex: false,
        lineNumber: 41,
        alias: null),
    TokenDefinition(
        name: "PERCENT_OP",
        pattern: "%[^%]*%",
        isRegex: true,
        lineNumber: 45,
        alias: null),
    TokenDefinition(
        name: "HEX_LIT",
        pattern: "0[xX][0-9a-fA-F]+L?",
        isRegex: true,
        lineNumber: 63,
        alias: null),
    TokenDefinition(
        name: "COMPLEX_LIT",
        pattern: "[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?i",
        isRegex: true,
        lineNumber: 64,
        alias: null),
    TokenDefinition(
        name: "INT_LIT",
        pattern: "[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?L",
        isRegex: true,
        lineNumber: 65,
        alias: null),
    TokenDefinition(
        name: "NUMBER",
        pattern: "[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?",
        isRegex: true,
        lineNumber: 66,
        alias: null),
    TokenDefinition(
        name: "DQ_STRING",
        pattern: "\"[^\"]*\"",
        isRegex: true,
        lineNumber: 72,
        alias: "STRING"),
    TokenDefinition(
        name: "SQ_STRING",
        pattern: "'[^']*'",
        isRegex: true,
        lineNumber: 73,
        alias: "STRING"),
    TokenDefinition(
        name: "NAME",
        pattern: "[A-Za-z.][A-Za-z0-9._]*",
        isRegex: true,
        lineNumber: 85,
        alias: null),
    TokenDefinition(
        name: "LT", pattern: "<", isRegex: false, lineNumber: 119, alias: null),
    TokenDefinition(
        name: "GT", pattern: ">", isRegex: false, lineNumber: 120, alias: null),
    TokenDefinition(
        name: "EQ", pattern: "=", isRegex: false, lineNumber: 121, alias: null),
    TokenDefinition(
        name: "PLUS",
        pattern: "+",
        isRegex: false,
        lineNumber: 122,
        alias: null),
    TokenDefinition(
        name: "MINUS",
        pattern: "-",
        isRegex: false,
        lineNumber: 123,
        alias: null),
    TokenDefinition(
        name: "STAR",
        pattern: "*",
        isRegex: false,
        lineNumber: 124,
        alias: null),
    TokenDefinition(
        name: "SLASH",
        pattern: "/",
        isRegex: false,
        lineNumber: 125,
        alias: null),
    TokenDefinition(
        name: "CARET",
        pattern: "^",
        isRegex: false,
        lineNumber: 126,
        alias: null),
    TokenDefinition(
        name: "COLON",
        pattern: ":",
        isRegex: false,
        lineNumber: 127,
        alias: null),
    TokenDefinition(
        name: "BACKSLASH",
        pattern: "\\\\",
        isRegex: true,
        lineNumber: 129,
        alias: null),
    TokenDefinition(
        name: "LPAREN",
        pattern: "(",
        isRegex: false,
        lineNumber: 130,
        alias: null),
    TokenDefinition(
        name: "RPAREN",
        pattern: ")",
        isRegex: false,
        lineNumber: 131,
        alias: null),
    TokenDefinition(
        name: "LBRACE",
        pattern: "{",
        isRegex: false,
        lineNumber: 132,
        alias: null),
    TokenDefinition(
        name: "RBRACE",
        pattern: "}",
        isRegex: false,
        lineNumber: 133,
        alias: null),
    TokenDefinition(
        name: "LBRACKET",
        pattern: "[",
        isRegex: false,
        lineNumber: 134,
        alias: null),
    TokenDefinition(
        name: "RBRACKET",
        pattern: "]",
        isRegex: false,
        lineNumber: 135,
        alias: null),
    TokenDefinition(
        name: "COMMA",
        pattern: ",",
        isRegex: false,
        lineNumber: 136,
        alias: null),
    TokenDefinition(
        name: "SEMICOLON",
        pattern: ";",
        isRegex: false,
        lineNumber: 137,
        alias: null),
    TokenDefinition(
        name: "DOLLAR",
        pattern: "\$",
        isRegex: false,
        lineNumber: 138,
        alias: null),
    TokenDefinition(
        name: "NEWLINE",
        pattern: "\\r?\\n",
        isRegex: true,
        lineNumber: 148,
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
    "NA_integer_",
    "NA_real_",
    "NA_character_",
    "Inf",
    "NaN"
  ],
  mode: null,
  skipDefinitions: [
    TokenDefinition(
        name: "WHITESPACE",
        pattern: "[ \\t]+",
        isRegex: true,
        lineNumber: 155,
        alias: null),
    TokenDefinition(
        name: "COMMENT",
        pattern: "#[^\\n]*",
        isRegex: true,
        lineNumber: 156,
        alias: null),
  ],
  reservedKeywords: const [],
  escapeMode: null,
  errorDefinitions: [
    TokenDefinition(
        name: "UNKNOWN",
        pattern: ".",
        isRegex: true,
        lineNumber: 163,
        alias: null),
  ],
  groups: const {},
  caseSensitive: true,
  layoutKeywords: const [],
  contextKeywords: const [],
  softKeywords: const [],
);
