// AUTO-GENERATED FILE — DO NOT EDIT
// Source: toml.grammar
// Regenerate with: grammar-tools compile-grammar toml.grammar
//
// This file embeds a ParserGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .grammar file at
// runtime.

import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const PARSER_GRAMMAR: ParserGrammar = {
  version: 1,
  rules: [
  {
    name: "document",
    body: { type: "repetition", element: { type: "alternation", choices: [
        { type: "token_reference", name: "NEWLINE" },
        { type: "rule_reference", name: "expression" },
      ] } },
    lineNumber: 38,
  },
  {
    name: "expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "array_table_header" },
      { type: "rule_reference", name: "table_header" },
      { type: "rule_reference", name: "keyval" },
    ] },
    lineNumber: 49,
  },
  {
    name: "keyval",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "key" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "value" },
    ] },
    lineNumber: 57,
  },
  {
    name: "key",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "simple_key" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "DOT" },
          { type: "rule_reference", name: "simple_key" },
        ] } },
    ] },
    lineNumber: 65,
  },
  {
    name: "simple_key",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "BARE_KEY" },
      { type: "token_reference", name: "BASIC_STRING" },
      { type: "token_reference", name: "LITERAL_STRING" },
      { type: "token_reference", name: "TRUE" },
      { type: "token_reference", name: "FALSE" },
      { type: "token_reference", name: "INTEGER" },
      { type: "token_reference", name: "FLOAT" },
      { type: "token_reference", name: "OFFSET_DATETIME" },
      { type: "token_reference", name: "LOCAL_DATETIME" },
      { type: "token_reference", name: "LOCAL_DATE" },
      { type: "token_reference", name: "LOCAL_TIME" },
    ] },
    lineNumber: 82,
  },
  {
    name: "table_header",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "rule_reference", name: "key" },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 92,
  },
  {
    name: "array_table_header",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "token_reference", name: "LBRACKET" },
      { type: "rule_reference", name: "key" },
      { type: "token_reference", name: "RBRACKET" },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 104,
  },
  {
    name: "value",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "BASIC_STRING" },
      { type: "token_reference", name: "ML_BASIC_STRING" },
      { type: "token_reference", name: "LITERAL_STRING" },
      { type: "token_reference", name: "ML_LITERAL_STRING" },
      { type: "token_reference", name: "INTEGER" },
      { type: "token_reference", name: "FLOAT" },
      { type: "token_reference", name: "TRUE" },
      { type: "token_reference", name: "FALSE" },
      { type: "token_reference", name: "OFFSET_DATETIME" },
      { type: "token_reference", name: "LOCAL_DATETIME" },
      { type: "token_reference", name: "LOCAL_DATE" },
      { type: "token_reference", name: "LOCAL_TIME" },
      { type: "rule_reference", name: "array" },
      { type: "rule_reference", name: "inline_table" },
    ] },
    lineNumber: 121,
  },
  {
    name: "array",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "rule_reference", name: "array_values" },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 140,
  },
  {
    name: "array_values",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "token_reference", name: "NEWLINE" } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "value" },
          { type: "repetition", element: { type: "token_reference", name: "NEWLINE" } },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "repetition", element: { type: "token_reference", name: "NEWLINE" } },
              { type: "rule_reference", name: "value" },
              { type: "repetition", element: { type: "token_reference", name: "NEWLINE" } },
            ] } },
          { type: "optional", element: { type: "token_reference", name: "COMMA" } },
          { type: "repetition", element: { type: "token_reference", name: "NEWLINE" } },
        ] } },
    ] },
    lineNumber: 142,
  },
  {
    name: "inline_table",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "keyval" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "keyval" },
            ] } },
        ] } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 162,
  },
],
};
