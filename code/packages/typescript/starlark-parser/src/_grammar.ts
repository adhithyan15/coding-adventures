// AUTO-GENERATED FILE — DO NOT EDIT
// Source: starlark.grammar
// Regenerate with: grammar-tools compile-grammar starlark.grammar
//
// This file embeds a ParserGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .grammar file at
// runtime.

import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const PARSER_GRAMMAR: ParserGrammar = {
  version: 1,
  rules: [
  {
    name: "file",
    body: { type: "repetition", element: { type: "alternation", choices: [
        { type: "token_reference", name: "NEWLINE" },
        { type: "rule_reference", name: "statement" },
      ] } },
    lineNumber: 34,
  },
  {
    name: "statement",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "compound_stmt" },
      { type: "rule_reference", name: "simple_stmt" },
    ] },
    lineNumber: 48,
  },
  {
    name: "simple_stmt",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "small_stmt" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "SEMICOLON" },
          { type: "rule_reference", name: "small_stmt" },
        ] } },
      { type: "token_reference", name: "NEWLINE" },
    ] },
    lineNumber: 52,
  },
  {
    name: "small_stmt",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "return_stmt" },
      { type: "rule_reference", name: "break_stmt" },
      { type: "rule_reference", name: "continue_stmt" },
      { type: "rule_reference", name: "pass_stmt" },
      { type: "rule_reference", name: "load_stmt" },
      { type: "rule_reference", name: "assign_stmt" },
    ] },
    lineNumber: 54,
  },
  {
    name: "return_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "return" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
    ] },
    lineNumber: 68,
  },
  {
    name: "break_stmt",
    body: { type: "literal", value: "break" },
    lineNumber: 71,
  },
  {
    name: "continue_stmt",
    body: { type: "literal", value: "continue" },
    lineNumber: 74,
  },
  {
    name: "pass_stmt",
    body: { type: "literal", value: "pass" },
    lineNumber: 79,
  },
  {
    name: "load_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "load" },
      { type: "token_reference", name: "LPAREN" },
      { type: "token_reference", name: "STRING" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "load_arg" },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 88,
  },
  {
    name: "load_arg",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "EQUALS" },
        { type: "token_reference", name: "STRING" },
      ] },
      { type: "token_reference", name: "STRING" },
    ] },
    lineNumber: 89,
  },
  {
    name: "assign_stmt",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression_list" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "rule_reference", name: "assign_op" },
              { type: "rule_reference", name: "augmented_assign_op" },
            ] } },
          { type: "rule_reference", name: "expression_list" },
        ] } },
    ] },
    lineNumber: 110,
  },
  {
    name: "assign_op",
    body: { type: "token_reference", name: "EQUALS" },
    lineNumber: 113,
  },
  {
    name: "augmented_assign_op",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "PLUS_EQUALS" },
      { type: "token_reference", name: "MINUS_EQUALS" },
      { type: "token_reference", name: "STAR_EQUALS" },
      { type: "token_reference", name: "SLASH_EQUALS" },
      { type: "token_reference", name: "FLOOR_DIV_EQUALS" },
      { type: "token_reference", name: "PERCENT_EQUALS" },
      { type: "token_reference", name: "AMP_EQUALS" },
      { type: "token_reference", name: "PIPE_EQUALS" },
      { type: "token_reference", name: "CARET_EQUALS" },
      { type: "token_reference", name: "LEFT_SHIFT_EQUALS" },
      { type: "token_reference", name: "RIGHT_SHIFT_EQUALS" },
      { type: "token_reference", name: "DOUBLE_STAR_EQUALS" },
    ] },
    lineNumber: 115,
  },
  {
    name: "compound_stmt",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "if_stmt" },
      { type: "rule_reference", name: "for_stmt" },
      { type: "rule_reference", name: "def_stmt" },
    ] },
    lineNumber: 124,
  },
  {
    name: "if_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "if" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "suite" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "elif" },
          { type: "rule_reference", name: "expression" },
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "suite" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "else" },
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "suite" },
        ] } },
    ] },
    lineNumber: 136,
  },
  {
    name: "for_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "for" },
      { type: "rule_reference", name: "loop_vars" },
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "suite" },
    ] },
    lineNumber: 150,
  },
  {
    name: "loop_vars",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 156,
  },
  {
    name: "def_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "def" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "parameters" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "suite" },
    ] },
    lineNumber: 166,
  },
  {
    name: "suite",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "simple_stmt" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "NEWLINE" },
        { type: "token_reference", name: "INDENT" },
        { type: "repetition", element: { type: "rule_reference", name: "statement" } },
        { type: "token_reference", name: "DEDENT" },
      ] },
    ] },
    lineNumber: 177,
  },
  {
    name: "parameters",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "parameter" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "parameter" },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
    ] },
    lineNumber: 198,
  },
  {
    name: "parameter",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "DOUBLE_STAR" },
        { type: "token_reference", name: "NAME" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "STAR" },
        { type: "token_reference", name: "NAME" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "EQUALS" },
        { type: "rule_reference", name: "expression" },
      ] },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 200,
  },
  {
    name: "expression_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "expression" },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
    ] },
    lineNumber: 234,
  },
  {
    name: "expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "lambda_expr" },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "or_expr" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "literal", value: "if" },
            { type: "rule_reference", name: "or_expr" },
            { type: "literal", value: "else" },
            { type: "rule_reference", name: "expression" },
          ] } },
      ] },
    ] },
    lineNumber: 239,
  },
  {
    name: "lambda_expr",
    body: { type: "sequence", elements: [
      { type: "literal", value: "lambda" },
      { type: "optional", element: { type: "rule_reference", name: "lambda_params" } },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 244,
  },
  {
    name: "lambda_params",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "lambda_param" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "lambda_param" },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
    ] },
    lineNumber: 245,
  },
  {
    name: "lambda_param",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "EQUALS" },
            { type: "rule_reference", name: "expression" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "STAR" },
        { type: "token_reference", name: "NAME" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "DOUBLE_STAR" },
        { type: "token_reference", name: "NAME" },
      ] },
    ] },
    lineNumber: 246,
  },
  {
    name: "or_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "and_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "or" },
          { type: "rule_reference", name: "and_expr" },
        ] } },
    ] },
    lineNumber: 250,
  },
  {
    name: "and_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "not_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "and" },
          { type: "rule_reference", name: "not_expr" },
        ] } },
    ] },
    lineNumber: 254,
  },
  {
    name: "not_expr",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "not" },
        { type: "rule_reference", name: "not_expr" },
      ] },
      { type: "rule_reference", name: "comparison" },
    ] },
    lineNumber: 258,
  },
  {
    name: "comparison",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "bitwise_or" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "comp_op" },
          { type: "rule_reference", name: "bitwise_or" },
        ] } },
    ] },
    lineNumber: 267,
  },
  {
    name: "comp_op",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "EQUALS_EQUALS" },
      { type: "token_reference", name: "NOT_EQUALS" },
      { type: "token_reference", name: "LESS_THAN" },
      { type: "token_reference", name: "GREATER_THAN" },
      { type: "token_reference", name: "LESS_EQUALS" },
      { type: "token_reference", name: "GREATER_EQUALS" },
      { type: "literal", value: "in" },
      { type: "sequence", elements: [
        { type: "literal", value: "not" },
        { type: "literal", value: "in" },
      ] },
    ] },
    lineNumber: 269,
  },
  {
    name: "bitwise_or",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "bitwise_xor" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "PIPE" },
          { type: "rule_reference", name: "bitwise_xor" },
        ] } },
    ] },
    lineNumber: 275,
  },
  {
    name: "bitwise_xor",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "bitwise_and" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "CARET" },
          { type: "rule_reference", name: "bitwise_and" },
        ] } },
    ] },
    lineNumber: 276,
  },
  {
    name: "bitwise_and",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "shift" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "AMP" },
          { type: "rule_reference", name: "shift" },
        ] } },
    ] },
    lineNumber: 277,
  },
  {
    name: "shift",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "arith" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "LEFT_SHIFT" },
              { type: "token_reference", name: "RIGHT_SHIFT" },
            ] } },
          { type: "rule_reference", name: "arith" },
        ] } },
    ] },
    lineNumber: 280,
  },
  {
    name: "arith",
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
    lineNumber: 284,
  },
  {
    name: "term",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "factor" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "STAR" },
              { type: "token_reference", name: "SLASH" },
              { type: "token_reference", name: "FLOOR_DIV" },
              { type: "token_reference", name: "PERCENT" },
            ] } },
          { type: "rule_reference", name: "factor" },
        ] } },
    ] },
    lineNumber: 289,
  },
  {
    name: "factor",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "group", element: { type: "alternation", choices: [
            { type: "token_reference", name: "PLUS" },
            { type: "token_reference", name: "MINUS" },
            { type: "token_reference", name: "TILDE" },
          ] } },
        { type: "rule_reference", name: "factor" },
      ] },
      { type: "rule_reference", name: "power" },
    ] },
    lineNumber: 295,
  },
  {
    name: "power",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "primary" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "DOUBLE_STAR" },
          { type: "rule_reference", name: "factor" },
        ] } },
    ] },
    lineNumber: 303,
  },
  {
    name: "primary",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "atom" },
      { type: "repetition", element: { type: "rule_reference", name: "suffix" } },
    ] },
    lineNumber: 320,
  },
  {
    name: "suffix",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "DOT" },
        { type: "token_reference", name: "NAME" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LBRACKET" },
        { type: "rule_reference", name: "subscript" },
        { type: "token_reference", name: "RBRACKET" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "arguments" } },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 322,
  },
  {
    name: "subscript",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "expression" },
      { type: "sequence", elements: [
        { type: "optional", element: { type: "rule_reference", name: "expression" } },
        { type: "token_reference", name: "COLON" },
        { type: "optional", element: { type: "rule_reference", name: "expression" } },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COLON" },
            { type: "optional", element: { type: "rule_reference", name: "expression" } },
          ] } },
      ] },
    ] },
    lineNumber: 334,
  },
  {
    name: "atom",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "INT" },
      { type: "token_reference", name: "FLOAT" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "STRING" },
        { type: "repetition", element: { type: "token_reference", name: "STRING" } },
      ] },
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "True" },
      { type: "literal", value: "False" },
      { type: "literal", value: "None" },
      { type: "rule_reference", name: "list_expr" },
      { type: "rule_reference", name: "dict_expr" },
      { type: "rule_reference", name: "paren_expr" },
    ] },
    lineNumber: 343,
  },
  {
    name: "list_expr",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "rule_reference", name: "list_body" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 359,
  },
  {
    name: "list_body",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "expression" },
        { type: "rule_reference", name: "comp_clause" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "expression" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "expression" },
          ] } },
        { type: "optional", element: { type: "token_reference", name: "COMMA" } },
      ] },
    ] },
    lineNumber: 361,
  },
  {
    name: "dict_expr",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "rule_reference", name: "dict_body" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 367,
  },
  {
    name: "dict_body",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "dict_entry" },
        { type: "rule_reference", name: "comp_clause" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "dict_entry" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "dict_entry" },
          ] } },
        { type: "optional", element: { type: "token_reference", name: "COMMA" } },
      ] },
    ] },
    lineNumber: 369,
  },
  {
    name: "dict_entry",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 372,
  },
  {
    name: "paren_expr",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "paren_body" } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 379,
  },
  {
    name: "paren_body",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "expression" },
        { type: "rule_reference", name: "comp_clause" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "COMMA" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "expression" },
            { type: "repetition", element: { type: "sequence", elements: [
                { type: "token_reference", name: "COMMA" },
                { type: "rule_reference", name: "expression" },
              ] } },
            { type: "optional", element: { type: "token_reference", name: "COMMA" } },
          ] } },
      ] },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 381,
  },
  {
    name: "comp_clause",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "comp_for" },
      { type: "repetition", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "comp_for" },
          { type: "rule_reference", name: "comp_if" },
        ] } },
    ] },
    lineNumber: 397,
  },
  {
    name: "comp_for",
    body: { type: "sequence", elements: [
      { type: "literal", value: "for" },
      { type: "rule_reference", name: "loop_vars" },
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "or_expr" },
    ] },
    lineNumber: 399,
  },
  {
    name: "comp_if",
    body: { type: "sequence", elements: [
      { type: "literal", value: "if" },
      { type: "rule_reference", name: "or_expr" },
    ] },
    lineNumber: 401,
  },
  {
    name: "arguments",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "argument" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "argument" },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
    ] },
    lineNumber: 420,
  },
  {
    name: "argument",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "DOUBLE_STAR" },
        { type: "rule_reference", name: "expression" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "STAR" },
        { type: "rule_reference", name: "expression" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "EQUALS" },
        { type: "rule_reference", name: "expression" },
      ] },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 422,
  },
],
};
