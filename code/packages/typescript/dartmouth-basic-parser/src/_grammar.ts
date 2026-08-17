// AUTO-GENERATED FILE - DO NOT EDIT
// Source: dartmouth_basic.grammar
// Regenerate with: grammar-tools compile-grammar dartmouth_basic.grammar
//
// This file embeds a ParserGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .grammar file at
// runtime.

import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const PARSER_GRAMMAR: ParserGrammar = {
  version: 1,
  rules: [
  {
    name: "program",
    body: { type: "repetition", element: { type: "rule_reference", name: "line" } },
    lineNumber: 70,
  },
  {
    name: "line",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LINE_NUM" },
      { type: "optional", element: { type: "rule_reference", name: "statement" } },
      { type: "token_reference", name: "NEWLINE" },
    ] },
    lineNumber: 81,
  },
  {
    name: "statement",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "let_stmt" },
      { type: "rule_reference", name: "print_stmt" },
      { type: "rule_reference", name: "input_stmt" },
      { type: "rule_reference", name: "if_stmt" },
      { type: "rule_reference", name: "goto_stmt" },
      { type: "rule_reference", name: "gosub_stmt" },
      { type: "rule_reference", name: "return_stmt" },
      { type: "rule_reference", name: "for_stmt" },
      { type: "rule_reference", name: "next_stmt" },
      { type: "rule_reference", name: "end_stmt" },
      { type: "rule_reference", name: "stop_stmt" },
      { type: "rule_reference", name: "rem_stmt" },
      { type: "rule_reference", name: "read_stmt" },
      { type: "rule_reference", name: "data_stmt" },
      { type: "rule_reference", name: "restore_stmt" },
      { type: "rule_reference", name: "dim_stmt" },
      { type: "rule_reference", name: "def_stmt" },
    ] },
    lineNumber: 91,
  },
  {
    name: "let_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "LET" },
      { type: "rule_reference", name: "variable" },
      { type: "token_reference", name: "EQ" },
      { type: "rule_reference", name: "expr" },
    ] },
    lineNumber: 121,
  },
  {
    name: "print_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "PRINT" },
      { type: "optional", element: { type: "rule_reference", name: "print_list" } },
    ] },
    lineNumber: 137,
  },
  {
    name: "print_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "print_item" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "print_sep" },
          { type: "rule_reference", name: "print_item" },
        ] } },
      { type: "optional", element: { type: "rule_reference", name: "print_sep" } },
    ] },
    lineNumber: 139,
  },
  {
    name: "print_item",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "STRING" },
      { type: "rule_reference", name: "expr" },
    ] },
    lineNumber: 141,
  },
  {
    name: "print_sep",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "COMMA" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 143,
  },
  {
    name: "input_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "INPUT" },
      { type: "rule_reference", name: "variable" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "variable" },
        ] } },
    ] },
    lineNumber: 155,
  },
  {
    name: "if_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "IF" },
      { type: "rule_reference", name: "expr" },
      { type: "rule_reference", name: "relop" },
      { type: "rule_reference", name: "expr" },
      { type: "literal", value: "THEN" },
      { type: "token_reference", name: "NUMBER" },
    ] },
    lineNumber: 170,
  },
  {
    name: "relop",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "EQ" },
      { type: "token_reference", name: "LT" },
      { type: "token_reference", name: "GT" },
      { type: "token_reference", name: "LE" },
      { type: "token_reference", name: "GE" },
      { type: "token_reference", name: "NE" },
    ] },
    lineNumber: 172,
  },
  {
    name: "goto_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "GOTO" },
      { type: "token_reference", name: "NUMBER" },
    ] },
    lineNumber: 183,
  },
  {
    name: "gosub_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "GOSUB" },
      { type: "token_reference", name: "NUMBER" },
    ] },
    lineNumber: 198,
  },
  {
    name: "return_stmt",
    body: { type: "literal", value: "RETURN" },
    lineNumber: 200,
  },
  {
    name: "for_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "FOR" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "EQ" },
      { type: "rule_reference", name: "expr" },
      { type: "literal", value: "TO" },
      { type: "rule_reference", name: "expr" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "STEP" },
          { type: "rule_reference", name: "expr" },
        ] } },
    ] },
    lineNumber: 222,
  },
  {
    name: "next_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "NEXT" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 224,
  },
  {
    name: "end_stmt",
    body: { type: "literal", value: "END" },
    lineNumber: 233,
  },
  {
    name: "stop_stmt",
    body: { type: "literal", value: "STOP" },
    lineNumber: 234,
  },
  {
    name: "rem_stmt",
    body: { type: "literal", value: "REM" },
    lineNumber: 247,
  },
  {
    name: "read_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "READ" },
      { type: "rule_reference", name: "variable" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "variable" },
        ] } },
    ] },
    lineNumber: 263,
  },
  {
    name: "data_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "DATA" },
      { type: "token_reference", name: "NUMBER" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "token_reference", name: "NUMBER" },
        ] } },
    ] },
    lineNumber: 265,
  },
  {
    name: "restore_stmt",
    body: { type: "literal", value: "RESTORE" },
    lineNumber: 267,
  },
  {
    name: "dim_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "DIM" },
      { type: "rule_reference", name: "dim_decl" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "dim_decl" },
        ] } },
    ] },
    lineNumber: 280,
  },
  {
    name: "dim_decl",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "token_reference", name: "NUMBER" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "token_reference", name: "NUMBER" },
        ] } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 282,
  },
  {
    name: "def_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "DEF" },
      { type: "token_reference", name: "USER_FN" },
      { type: "token_reference", name: "LPAREN" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "EQ" },
      { type: "rule_reference", name: "expr" },
    ] },
    lineNumber: 295,
  },
  {
    name: "variable",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expr" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "expr" },
          ] } },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 312,
  },
  {
    name: "expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "term" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "PLUS" },
              { type: "token_reference", name: "MINUS" },
            ] } },
          { type: "rule_reference", name: "term" },
        ] } },
    ] },
    lineNumber: 335,
  },
  {
    name: "term",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "power" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "STAR" },
              { type: "token_reference", name: "SLASH" },
            ] } },
          { type: "rule_reference", name: "power" },
        ] } },
    ] },
    lineNumber: 337,
  },
  {
    name: "power",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "unary" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "CARET" },
          { type: "rule_reference", name: "power" },
        ] } },
    ] },
    lineNumber: 343,
  },
  {
    name: "unary",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "MINUS" },
        { type: "rule_reference", name: "primary" },
      ] },
      { type: "rule_reference", name: "primary" },
    ] },
    lineNumber: 348,
  },
  {
    name: "primary",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "STRING" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "BUILTIN_FN" },
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expr" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "USER_FN" },
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expr" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "rule_reference", name: "variable" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expr" },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 366,
  },
],
};
