// AUTO-GENERATED FILE - DO NOT EDIT
// Source: algol60.grammar
// Regenerate with: grammar-tools compile-grammar algol60.grammar
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
    body: { type: "rule_reference", name: "block" },
    lineNumber: 47,
  },
  {
    name: "block",
    body: { type: "sequence", elements: [
      { type: "literal", value: "begin" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "declaration" },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
      { type: "repetition", element: { type: "token_reference", name: "SEMICOLON" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "statement" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "SEMICOLON" },
              { type: "optional", element: { type: "rule_reference", name: "statement" } },
            ] } },
        ] } },
      { type: "literal", value: "end" },
    ] },
    lineNumber: 53,
  },
  {
    name: "declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "type_decl" },
      { type: "rule_reference", name: "own_decl" },
      { type: "rule_reference", name: "own_array_decl" },
      { type: "rule_reference", name: "array_decl" },
      { type: "rule_reference", name: "switch_decl" },
      { type: "rule_reference", name: "procedure_decl" },
    ] },
    lineNumber: 60,
  },
  {
    name: "type_decl",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "ident_list" },
    ] },
    lineNumber: 71,
  },
  {
    name: "own_decl",
    body: { type: "sequence", elements: [
      { type: "literal", value: "own" },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "ident_list" },
    ] },
    lineNumber: 76,
  },
  {
    name: "own_array_decl",
    body: { type: "sequence", elements: [
      { type: "literal", value: "own" },
      { type: "optional", element: { type: "rule_reference", name: "type" } },
      { type: "literal", value: "array" },
      { type: "rule_reference", name: "array_segment" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "array_segment" },
        ] } },
    ] },
    lineNumber: 81,
  },
  {
    name: "type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "integer" },
      { type: "literal", value: "real" },
      { type: "literal", value: "boolean" },
      { type: "literal", value: "string" },
    ] },
    lineNumber: 83,
  },
  {
    name: "ident_list",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 85,
  },
  {
    name: "array_decl",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "rule_reference", name: "type" } },
      { type: "literal", value: "array" },
      { type: "rule_reference", name: "array_segment" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "array_segment" },
        ] } },
    ] },
    lineNumber: 93,
  },
  {
    name: "array_segment",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "ident_list" },
      { type: "token_reference", name: "LBRACKET" },
      { type: "rule_reference", name: "bound_pair" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "bound_pair" },
        ] } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 95,
  },
  {
    name: "bound_pair",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "arith_expr" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "arith_expr" },
    ] },
    lineNumber: 99,
  },
  {
    name: "switch_decl",
    body: { type: "sequence", elements: [
      { type: "literal", value: "switch" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "ASSIGN" },
      { type: "rule_reference", name: "switch_list" },
    ] },
    lineNumber: 104,
  },
  {
    name: "switch_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "desig_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "desig_expr" },
        ] } },
    ] },
    lineNumber: 106,
  },
  {
    name: "procedure_decl",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "rule_reference", name: "type" } },
      { type: "literal", value: "procedure" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "formal_params" } },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "optional", element: { type: "rule_reference", name: "value_part" } },
      { type: "repetition", element: { type: "rule_reference", name: "spec_part" } },
      { type: "rule_reference", name: "proc_body" },
    ] },
    lineNumber: 113,
  },
  {
    name: "formal_params",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "ident_list" } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 118,
  },
  {
    name: "value_part",
    body: { type: "sequence", elements: [
      { type: "literal", value: "value" },
      { type: "rule_reference", name: "ident_list" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 123,
  },
  {
    name: "spec_part",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "specifier" },
      { type: "rule_reference", name: "ident_list" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 130,
  },
  {
    name: "specifier",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "type" },
        { type: "literal", value: "array" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "type" },
        { type: "literal", value: "procedure" },
      ] },
      { type: "literal", value: "array" },
      { type: "literal", value: "label" },
      { type: "literal", value: "switch" },
      { type: "literal", value: "procedure" },
      { type: "rule_reference", name: "type" },
    ] },
    lineNumber: 132,
  },
  {
    name: "proc_body",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "block" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 140,
  },
  {
    name: "statement",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "label" },
            { type: "token_reference", name: "COLON" },
          ] } },
        { type: "rule_reference", name: "unlabeled_stmt" },
      ] },
      { type: "sequence", elements: [
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "label" },
            { type: "token_reference", name: "COLON" },
          ] } },
        { type: "rule_reference", name: "cond_stmt" },
      ] },
    ] },
    lineNumber: 152,
  },
  {
    name: "label",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "INTEGER_LIT" },
    ] },
    lineNumber: 155,
  },
  {
    name: "unlabeled_stmt",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "assign_stmt" },
      { type: "rule_reference", name: "dummy_stmt" },
      { type: "rule_reference", name: "goto_stmt" },
      { type: "rule_reference", name: "proc_stmt" },
      { type: "rule_reference", name: "compound_stmt" },
      { type: "rule_reference", name: "block" },
      { type: "rule_reference", name: "for_stmt" },
    ] },
    lineNumber: 165,
  },
  {
    name: "dummy_stmt",
    body: { type: "alternation", choices: [
      { type: "positive_lookahead", element: { type: "token_reference", name: "SEMICOLON" } },
      { type: "positive_lookahead", element: { type: "literal", value: "end" } },
      { type: "positive_lookahead", element: { type: "literal", value: "else" } },
    ] },
    lineNumber: 175,
  },
  {
    name: "cond_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "if" },
      { type: "rule_reference", name: "bool_expr" },
      { type: "literal", value: "then" },
      { type: "rule_reference", name: "unlabeled_stmt" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "else" },
          { type: "rule_reference", name: "statement" },
        ] } },
    ] },
    lineNumber: 181,
  },
  {
    name: "compound_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "begin" },
      { type: "repetition", element: { type: "token_reference", name: "SEMICOLON" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "statement" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "SEMICOLON" },
              { type: "optional", element: { type: "rule_reference", name: "statement" } },
            ] } },
        ] } },
      { type: "literal", value: "end" },
    ] },
    lineNumber: 185,
  },
  {
    name: "assign_stmt",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "left_part" },
      { type: "repetition", element: { type: "rule_reference", name: "left_part" } },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 191,
  },
  {
    name: "left_part",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "variable" },
      { type: "token_reference", name: "ASSIGN" },
    ] },
    lineNumber: 193,
  },
  {
    name: "goto_stmt",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "goto" },
        { type: "rule_reference", name: "desig_expr" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "go" },
        { type: "literal", value: "to" },
        { type: "rule_reference", name: "desig_expr" },
      ] },
    ] },
    lineNumber: 197,
  },
  {
    name: "proc_stmt",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "optional", element: { type: "rule_reference", name: "actual_params" } },
          { type: "token_reference", name: "RPAREN" },
        ] } },
    ] },
    lineNumber: 202,
  },
  {
    name: "actual_params",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 204,
  },
  {
    name: "for_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "for" },
      { type: "rule_reference", name: "variable" },
      { type: "token_reference", name: "ASSIGN" },
      { type: "rule_reference", name: "for_list" },
      { type: "literal", value: "do" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 212,
  },
  {
    name: "for_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "for_elem" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "for_elem" },
        ] } },
    ] },
    lineNumber: 214,
  },
  {
    name: "for_elem",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "arith_expr" },
        { type: "literal", value: "step" },
        { type: "rule_reference", name: "arith_expr" },
        { type: "literal", value: "until" },
        { type: "rule_reference", name: "arith_expr" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "arith_expr" },
        { type: "literal", value: "while" },
        { type: "rule_reference", name: "bool_expr" },
      ] },
      { type: "rule_reference", name: "arith_expr" },
    ] },
    lineNumber: 218,
  },
  {
    name: "expression",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "if" },
        { type: "rule_reference", name: "bool_expr" },
        { type: "literal", value: "then" },
        { type: "rule_reference", name: "expression" },
        { type: "literal", value: "else" },
        { type: "rule_reference", name: "expression" },
      ] },
      { type: "rule_reference", name: "expr_eqv" },
    ] },
    lineNumber: 250,
  },
  {
    name: "expr_eqv",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expr_impl" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "eqv" },
          { type: "rule_reference", name: "expr_impl" },
        ] } },
    ] },
    lineNumber: 253,
  },
  {
    name: "expr_impl",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expr_or" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "impl" },
          { type: "rule_reference", name: "expr_or" },
        ] } },
    ] },
    lineNumber: 254,
  },
  {
    name: "expr_or",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expr_and" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "or" },
          { type: "rule_reference", name: "expr_and" },
        ] } },
    ] },
    lineNumber: 255,
  },
  {
    name: "expr_and",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expr_not" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "and" },
          { type: "rule_reference", name: "expr_not" },
        ] } },
    ] },
    lineNumber: 256,
  },
  {
    name: "expr_not",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "not" },
        { type: "rule_reference", name: "expr_not" },
      ] },
      { type: "rule_reference", name: "expr_cmp" },
    ] },
    lineNumber: 257,
  },
  {
    name: "expr_cmp",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expr_add" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "EQ" },
              { type: "token_reference", name: "NEQ" },
              { type: "token_reference", name: "LT" },
              { type: "token_reference", name: "LEQ" },
              { type: "token_reference", name: "GT" },
              { type: "token_reference", name: "GEQ" },
            ] } },
          { type: "rule_reference", name: "expr_add" },
        ] } },
    ] },
    lineNumber: 258,
  },
  {
    name: "expr_add",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "alternation", choices: [
          { type: "token_reference", name: "PLUS" },
          { type: "token_reference", name: "MINUS" },
        ] } },
      { type: "rule_reference", name: "expr_mul" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "PLUS" },
              { type: "token_reference", name: "MINUS" },
            ] } },
          { type: "rule_reference", name: "expr_mul" },
        ] } },
    ] },
    lineNumber: 259,
  },
  {
    name: "expr_mul",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expr_pow" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "STAR" },
              { type: "token_reference", name: "SLASH" },
              { type: "literal", value: "div" },
              { type: "literal", value: "mod" },
            ] } },
          { type: "rule_reference", name: "expr_pow" },
        ] } },
    ] },
    lineNumber: 260,
  },
  {
    name: "expr_pow",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expr_atom" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "CARET" },
              { type: "token_reference", name: "POWER" },
            ] } },
          { type: "rule_reference", name: "expr_atom" },
        ] } },
    ] },
    lineNumber: 261,
  },
  {
    name: "expr_atom",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "INTEGER_LIT" },
      { type: "token_reference", name: "REAL_LIT" },
      { type: "token_reference", name: "STRING_LIT" },
      { type: "literal", value: "true" },
      { type: "literal", value: "false" },
      { type: "rule_reference", name: "proc_call" },
      { type: "rule_reference", name: "variable" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 262,
  },
  {
    name: "arith_expr",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "if" },
        { type: "rule_reference", name: "bool_expr" },
        { type: "literal", value: "then" },
        { type: "rule_reference", name: "arith_expr" },
        { type: "literal", value: "else" },
        { type: "rule_reference", name: "arith_expr" },
      ] },
      { type: "rule_reference", name: "simple_arith" },
    ] },
    lineNumber: 274,
  },
  {
    name: "simple_arith",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "alternation", choices: [
          { type: "token_reference", name: "PLUS" },
          { type: "token_reference", name: "MINUS" },
        ] } },
      { type: "rule_reference", name: "term" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "PLUS" },
              { type: "token_reference", name: "MINUS" },
            ] } },
          { type: "rule_reference", name: "term" },
        ] } },
    ] },
    lineNumber: 278,
  },
  {
    name: "term",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "factor" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "STAR" },
              { type: "token_reference", name: "SLASH" },
              { type: "literal", value: "div" },
              { type: "literal", value: "mod" },
            ] } },
          { type: "rule_reference", name: "factor" },
        ] } },
    ] },
    lineNumber: 283,
  },
  {
    name: "factor",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "primary" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "CARET" },
              { type: "token_reference", name: "POWER" },
            ] } },
          { type: "rule_reference", name: "primary" },
        ] } },
    ] },
    lineNumber: 289,
  },
  {
    name: "primary",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "INTEGER_LIT" },
      { type: "token_reference", name: "REAL_LIT" },
      { type: "token_reference", name: "STRING_LIT" },
      { type: "literal", value: "true" },
      { type: "literal", value: "false" },
      { type: "rule_reference", name: "proc_call" },
      { type: "rule_reference", name: "variable" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "arith_expr" },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 291,
  },
  {
    name: "bool_expr",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "if" },
        { type: "rule_reference", name: "bool_expr" },
        { type: "literal", value: "then" },
        { type: "rule_reference", name: "bool_expr" },
        { type: "literal", value: "else" },
        { type: "rule_reference", name: "bool_expr" },
      ] },
      { type: "rule_reference", name: "simple_bool" },
    ] },
    lineNumber: 309,
  },
  {
    name: "simple_bool",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "implication" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "eqv" },
          { type: "rule_reference", name: "implication" },
        ] } },
    ] },
    lineNumber: 312,
  },
  {
    name: "implication",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "bool_term" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "impl" },
          { type: "rule_reference", name: "bool_term" },
        ] } },
    ] },
    lineNumber: 314,
  },
  {
    name: "bool_term",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "bool_factor" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "or" },
          { type: "rule_reference", name: "bool_factor" },
        ] } },
    ] },
    lineNumber: 316,
  },
  {
    name: "bool_factor",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "bool_secondary" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "and" },
          { type: "rule_reference", name: "bool_secondary" },
        ] } },
    ] },
    lineNumber: 318,
  },
  {
    name: "bool_secondary",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "not" },
        { type: "rule_reference", name: "bool_secondary" },
      ] },
      { type: "rule_reference", name: "bool_primary" },
    ] },
    lineNumber: 320,
  },
  {
    name: "bool_primary",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "relation" },
      { type: "literal", value: "true" },
      { type: "literal", value: "false" },
      { type: "rule_reference", name: "proc_call" },
      { type: "rule_reference", name: "variable" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "bool_expr" },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 322,
  },
  {
    name: "relation",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "simple_arith" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "EQ" },
          { type: "token_reference", name: "NEQ" },
          { type: "token_reference", name: "LT" },
          { type: "token_reference", name: "LEQ" },
          { type: "token_reference", name: "GT" },
          { type: "token_reference", name: "GEQ" },
        ] } },
      { type: "rule_reference", name: "simple_arith" },
    ] },
    lineNumber: 332,
  },
  {
    name: "desig_expr",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "if" },
        { type: "rule_reference", name: "bool_expr" },
        { type: "literal", value: "then" },
        { type: "rule_reference", name: "desig_expr" },
        { type: "literal", value: "else" },
        { type: "rule_reference", name: "desig_expr" },
      ] },
      { type: "rule_reference", name: "simple_desig" },
    ] },
    lineNumber: 337,
  },
  {
    name: "simple_desig",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "LBRACKET" },
        { type: "rule_reference", name: "arith_expr" },
        { type: "token_reference", name: "RBRACKET" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "desig_expr" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "rule_reference", name: "label" },
    ] },
    lineNumber: 340,
  },
  {
    name: "variable",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LBRACKET" },
          { type: "rule_reference", name: "subscripts" },
          { type: "token_reference", name: "RBRACKET" },
        ] } },
    ] },
    lineNumber: 352,
  },
  {
    name: "subscripts",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "arith_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "arith_expr" },
        ] } },
    ] },
    lineNumber: 354,
  },
  {
    name: "proc_call",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "actual_params" } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 359,
  },
],
};
