// AUTO-GENERATED FILE - DO NOT EDIT
// Source: dot.grammar
// Regenerate with: grammar-tools compile-grammar dot.grammar
//
// This file embeds a ParserGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .grammar file at
// runtime.

import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const PARSER_GRAMMAR: ParserGrammar = {
  version: 1,
  rules: [
  {
    name: "graph",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "literal", value: "STRICT" } },
      { type: "literal", value: "DIGRAPH" },
      { type: "optional", element: { type: "rule_reference", name: "id" } },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "stmt_list" },
      { type: "token_reference", name: "RBRACE" },
      { type: "token_reference", name: "EOF" },
    ] },
    lineNumber: 10,
  },
  {
    name: "stmt_list",
    body: { type: "repetition", element: { type: "alternation", choices: [
        { type: "sequence", elements: [
          { type: "rule_reference", name: "stmt" },
          { type: "optional", element: { type: "token_reference", name: "SEMICOLON" } },
        ] },
        { type: "token_reference", name: "SEMICOLON" },
      ] } },
    lineNumber: 11,
  },
  {
    name: "stmt",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "attr_stmt" },
      { type: "rule_reference", name: "edge_stmt" },
      { type: "rule_reference", name: "assignment" },
      { type: "rule_reference", name: "node_stmt" },
    ] },
    lineNumber: 13,
  },
  {
    name: "attr_stmt",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "attr_target" },
      { type: "rule_reference", name: "attr_list" },
    ] },
    lineNumber: 14,
  },
  {
    name: "attr_target",
    body: { type: "alternation", choices: [
      { type: "literal", value: "GRAPH" },
      { type: "literal", value: "NODE" },
      { type: "literal", value: "EDGE" },
    ] },
    lineNumber: 15,
  },
  {
    name: "edge_stmt",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "node_id" },
      { type: "rule_reference", name: "edge_rhs" },
      { type: "optional", element: { type: "rule_reference", name: "attr_list" } },
    ] },
    lineNumber: 17,
  },
  {
    name: "edge_rhs",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "EDGEOP" },
      { type: "rule_reference", name: "node_id" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EDGEOP" },
          { type: "rule_reference", name: "node_id" },
        ] } },
    ] },
    lineNumber: 18,
  },
  {
    name: "node_stmt",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "node_id" },
      { type: "optional", element: { type: "rule_reference", name: "attr_list" } },
    ] },
    lineNumber: 20,
  },
  {
    name: "assignment",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "id" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "id" },
    ] },
    lineNumber: 21,
  },
  {
    name: "node_id",
    body: { type: "rule_reference", name: "id" },
    lineNumber: 22,
  },
  {
    name: "id",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "STRING" },
    ] },
    lineNumber: 23,
  },
  {
    name: "attr_list",
    body: { type: "one_or_more", element: { type: "rule_reference", name: "bracket_attr_list" } },
    lineNumber: 25,
  },
  {
    name: "bracket_attr_list",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "rule_reference", name: "a_list" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 26,
  },
  {
    name: "a_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "a_pair" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "optional", element: { type: "rule_reference", name: "attr_separator" } },
          { type: "rule_reference", name: "a_pair" },
        ] } },
      { type: "optional", element: { type: "rule_reference", name: "attr_separator" } },
    ] },
    lineNumber: 27,
  },
  {
    name: "a_pair",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "id" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "id" },
        ] } },
    ] },
    lineNumber: 28,
  },
  {
    name: "attr_separator",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "COMMA" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 29,
  },
],
};
