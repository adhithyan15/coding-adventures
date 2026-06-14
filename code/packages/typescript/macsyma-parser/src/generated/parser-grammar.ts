// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/macsyma/macsyma.grammar
// Regenerate with: grammar-tools compile-grammar code/grammars/macsyma/macsyma.grammar
//
// This file embeds a ParserGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .grammar file at
// runtime.

import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const PARSER_GRAMMAR: ParserGrammar = {
  version: 2,
  rules: [
  {
    name: "program",
    body: { type: "repetition", element: { type: "rule_reference", name: "statement" } },
    lineNumber: 31,
  },
  {
    name: "statement",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "SEMI" },
          { type: "token_reference", name: "DOLLAR" },
        ] } },
    ] },
    lineNumber: 33,
  },
  {
    name: "expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "if_expr" },
      { type: "rule_reference", name: "for_expr" },
      { type: "rule_reference", name: "while_expr" },
      { type: "rule_reference", name: "block_expr" },
      { type: "rule_reference", name: "return_expr" },
      { type: "rule_reference", name: "assign" },
    ] },
    lineNumber: 44,
  },
  {
    name: "if_expr",
    body: { type: "sequence", elements: [
      { type: "literal", value: "if" },
      { type: "rule_reference", name: "expression" },
      { type: "literal", value: "then" },
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "elseif" },
          { type: "rule_reference", name: "expression" },
          { type: "literal", value: "then" },
          { type: "rule_reference", name: "expression" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "else" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 54,
  },
  {
    name: "for_expr",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "for_each_expr" },
      { type: "rule_reference", name: "for_range_expr" },
    ] },
    lineNumber: 67,
  },
  {
    name: "for_each_expr",
    body: { type: "sequence", elements: [
      { type: "literal", value: "for" },
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "expression" },
      { type: "literal", value: "do" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 69,
  },
  {
    name: "for_range_expr",
    body: { type: "sequence", elements: [
      { type: "literal", value: "for" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: ":" },
          { type: "rule_reference", name: "expression" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "step" },
          { type: "rule_reference", name: "expression" },
        ] } },
      { type: "group", element: { type: "alternation", choices: [
          { type: "literal", value: "thru" },
          { type: "literal", value: "while" },
          { type: "literal", value: "unless" },
        ] } },
      { type: "rule_reference", name: "expression" },
      { type: "literal", value: "do" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 71,
  },
  {
    name: "while_expr",
    body: { type: "sequence", elements: [
      { type: "literal", value: "while" },
      { type: "rule_reference", name: "expression" },
      { type: "literal", value: "do" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 76,
  },
  {
    name: "block_expr",
    body: { type: "sequence", elements: [
      { type: "literal", value: "block" },
      { type: "literal", value: "(" },
      { type: "optional", element: { type: "rule_reference", name: "arglist" } },
      { type: "literal", value: ")" },
    ] },
    lineNumber: 82,
  },
  {
    name: "return_expr",
    body: { type: "sequence", elements: [
      { type: "literal", value: "return" },
      { type: "literal", value: "(" },
      { type: "rule_reference", name: "expression" },
      { type: "literal", value: ")" },
    ] },
    lineNumber: 87,
  },
  {
    name: "assign",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "logical_or" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "COLON" },
              { type: "token_reference", name: "COLONEQ" },
            ] } },
          { type: "rule_reference", name: "assign" },
        ] } },
    ] },
    lineNumber: 92,
  },
  {
    name: "logical_or",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "logical_and" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "or" },
          { type: "rule_reference", name: "logical_and" },
        ] } },
    ] },
    lineNumber: 97,
  },
  {
    name: "logical_and",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "logical_not" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "and" },
          { type: "rule_reference", name: "logical_not" },
        ] } },
    ] },
    lineNumber: 98,
  },
  {
    name: "logical_not",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "not" },
        { type: "rule_reference", name: "logical_not" },
      ] },
      { type: "rule_reference", name: "comparison" },
    ] },
    lineNumber: 99,
  },
  {
    name: "comparison",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "additive" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "EQ" },
              { type: "token_reference", name: "HASH" },
              { type: "token_reference", name: "LT" },
              { type: "token_reference", name: "GT" },
              { type: "token_reference", name: "LEQ" },
              { type: "token_reference", name: "GEQ" },
            ] } },
          { type: "rule_reference", name: "additive" },
        ] } },
    ] },
    lineNumber: 103,
  },
  {
    name: "additive",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "multiplicative" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "PLUS" },
              { type: "token_reference", name: "MINUS" },
            ] } },
          { type: "rule_reference", name: "multiplicative" },
        ] } },
    ] },
    lineNumber: 105,
  },
  {
    name: "multiplicative",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "unary" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "STAR" },
              { type: "token_reference", name: "SLASH" },
            ] } },
          { type: "rule_reference", name: "unary" },
        ] } },
    ] },
    lineNumber: 106,
  },
  {
    name: "unary",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "group", element: { type: "alternation", choices: [
            { type: "token_reference", name: "MINUS" },
            { type: "token_reference", name: "PLUS" },
          ] } },
        { type: "rule_reference", name: "unary" },
      ] },
      { type: "rule_reference", name: "power" },
    ] },
    lineNumber: 110,
  },
  {
    name: "power",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "postfix" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "CARET" },
              { type: "token_reference", name: "STAREQ" },
            ] } },
          { type: "rule_reference", name: "unary" },
        ] } },
    ] },
    lineNumber: 114,
  },
  {
    name: "postfix",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "atom" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "optional", element: { type: "rule_reference", name: "arglist" } },
          { type: "token_reference", name: "RPAREN" },
        ] } },
    ] },
    lineNumber: 118,
  },
  {
    name: "arglist",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 119,
  },
  {
    name: "atom",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "true" },
      { type: "literal", value: "false" },
      { type: "rule_reference", name: "group" },
      { type: "rule_reference", name: "list" },
    ] },
    lineNumber: 121,
  },
  {
    name: "group",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 129,
  },
  {
    name: "list",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "rule_reference", name: "arglist" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 130,
  },
],
};
