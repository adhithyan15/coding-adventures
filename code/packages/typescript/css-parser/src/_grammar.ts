// AUTO-GENERATED FILE - DO NOT EDIT
// Source: css.grammar
// Regenerate with: grammar-tools compile-grammar css.grammar
//
// This file embeds a ParserGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .grammar file at
// runtime.

import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const PARSER_GRAMMAR: ParserGrammar = {
  version: 1,
  rules: [
  {
    name: "stylesheet",
    body: { type: "repetition", element: { type: "rule_reference", name: "rule" } },
    lineNumber: 33,
  },
  {
    name: "rule",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "at_rule" },
      { type: "rule_reference", name: "qualified_rule" },
    ] },
    lineNumber: 35,
  },
  {
    name: "at_rule",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "AT_KEYWORD" },
      { type: "rule_reference", name: "at_prelude" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "SEMICOLON" },
          { type: "rule_reference", name: "block" },
        ] } },
    ] },
    lineNumber: 55,
  },
  {
    name: "at_prelude",
    body: { type: "repetition", element: { type: "rule_reference", name: "at_prelude_token" } },
    lineNumber: 61,
  },
  {
    name: "at_prelude_token",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "IDENT" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "DIMENSION" },
      { type: "token_reference", name: "PERCENTAGE" },
      { type: "token_reference", name: "HASH" },
      { type: "token_reference", name: "CUSTOM_PROPERTY" },
      { type: "token_reference", name: "UNICODE_RANGE" },
      { type: "rule_reference", name: "function_in_prelude" },
      { type: "rule_reference", name: "paren_block" },
      { type: "token_reference", name: "COLON" },
      { type: "token_reference", name: "COMMA" },
      { type: "token_reference", name: "SLASH" },
      { type: "token_reference", name: "DOT" },
      { type: "token_reference", name: "STAR" },
      { type: "token_reference", name: "PLUS" },
      { type: "token_reference", name: "MINUS" },
      { type: "token_reference", name: "GREATER" },
      { type: "token_reference", name: "TILDE" },
      { type: "token_reference", name: "PIPE" },
      { type: "token_reference", name: "EQUALS" },
      { type: "token_reference", name: "AMPERSAND" },
      { type: "token_reference", name: "CDO" },
      { type: "token_reference", name: "CDC" },
    ] },
    lineNumber: 63,
  },
  {
    name: "function_in_prelude",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "FUNCTION" },
      { type: "rule_reference", name: "at_prelude_tokens" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 71,
  },
  {
    name: "paren_block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "at_prelude_tokens" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 72,
  },
  {
    name: "at_prelude_tokens",
    body: { type: "repetition", element: { type: "rule_reference", name: "at_prelude_token" } },
    lineNumber: 73,
  },
  {
    name: "qualified_rule",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "selector_list" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 85,
  },
  {
    name: "selector_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "complex_selector" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "complex_selector" },
        ] } },
    ] },
    lineNumber: 96,
  },
  {
    name: "complex_selector",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "compound_selector" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "optional", element: { type: "rule_reference", name: "combinator" } },
          { type: "rule_reference", name: "compound_selector" },
        ] } },
    ] },
    lineNumber: 105,
  },
  {
    name: "combinator",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "GREATER" },
      { type: "token_reference", name: "PLUS" },
      { type: "token_reference", name: "TILDE" },
    ] },
    lineNumber: 112,
  },
  {
    name: "compound_selector",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "simple_selector" },
        { type: "repetition", element: { type: "rule_reference", name: "subclass_selector" } },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "subclass_selector" },
        { type: "repetition", element: { type: "rule_reference", name: "subclass_selector" } },
      ] },
    ] },
    lineNumber: 124,
  },
  {
    name: "simple_selector",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "IDENT" },
      { type: "token_reference", name: "STAR" },
      { type: "token_reference", name: "AMPERSAND" },
    ] },
    lineNumber: 131,
  },
  {
    name: "subclass_selector",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "class_selector" },
      { type: "rule_reference", name: "id_selector" },
      { type: "rule_reference", name: "attribute_selector" },
      { type: "rule_reference", name: "pseudo_class" },
      { type: "rule_reference", name: "pseudo_element" },
    ] },
    lineNumber: 139,
  },
  {
    name: "class_selector",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "DOT" },
      { type: "token_reference", name: "IDENT" },
    ] },
    lineNumber: 145,
  },
  {
    name: "id_selector",
    body: { type: "token_reference", name: "HASH" },
    lineNumber: 150,
  },
  {
    name: "attribute_selector",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "token_reference", name: "IDENT" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "attr_matcher" },
          { type: "rule_reference", name: "attr_value" },
          { type: "optional", element: { type: "token_reference", name: "IDENT" } },
        ] } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 161,
  },
  {
    name: "attr_matcher",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "EQUALS" },
      { type: "token_reference", name: "TILDE_EQUALS" },
      { type: "token_reference", name: "PIPE_EQUALS" },
      { type: "token_reference", name: "CARET_EQUALS" },
      { type: "token_reference", name: "DOLLAR_EQUALS" },
      { type: "token_reference", name: "STAR_EQUALS" },
    ] },
    lineNumber: 163,
  },
  {
    name: "attr_value",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "IDENT" },
      { type: "token_reference", name: "STRING" },
    ] },
    lineNumber: 166,
  },
  {
    name: "pseudo_class",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "COLON" },
        { type: "token_reference", name: "FUNCTION" },
        { type: "rule_reference", name: "pseudo_class_args" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "COLON" },
        { type: "token_reference", name: "IDENT" },
      ] },
    ] },
    lineNumber: 173,
  },
  {
    name: "pseudo_class_args",
    body: { type: "repetition", element: { type: "rule_reference", name: "pseudo_class_arg" } },
    lineNumber: 181,
  },
  {
    name: "pseudo_class_arg",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "IDENT" },
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "DIMENSION" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "HASH" },
      { type: "token_reference", name: "PLUS" },
      { type: "token_reference", name: "COMMA" },
      { type: "token_reference", name: "DOT" },
      { type: "token_reference", name: "STAR" },
      { type: "token_reference", name: "COLON" },
      { type: "token_reference", name: "AMPERSAND" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "FUNCTION" },
        { type: "rule_reference", name: "pseudo_class_args" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LBRACKET" },
        { type: "rule_reference", name: "pseudo_class_args" },
        { type: "token_reference", name: "RBRACKET" },
      ] },
    ] },
    lineNumber: 183,
  },
  {
    name: "pseudo_element",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "COLON_COLON" },
      { type: "token_reference", name: "IDENT" },
    ] },
    lineNumber: 190,
  },
  {
    name: "block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "block_contents" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 200,
  },
  {
    name: "block_contents",
    body: { type: "repetition", element: { type: "rule_reference", name: "block_item" } },
    lineNumber: 202,
  },
  {
    name: "block_item",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "at_rule" },
      { type: "rule_reference", name: "declaration_or_nested" },
    ] },
    lineNumber: 211,
  },
  {
    name: "declaration_or_nested",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "declaration" },
      { type: "rule_reference", name: "qualified_rule" },
    ] },
    lineNumber: 217,
  },
  {
    name: "declaration",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "property" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "value_list" },
      { type: "optional", element: { type: "rule_reference", name: "priority" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 231,
  },
  {
    name: "property",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "IDENT" },
      { type: "token_reference", name: "CUSTOM_PROPERTY" },
    ] },
    lineNumber: 233,
  },
  {
    name: "priority",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "BANG" },
      { type: "literal", value: "important" },
    ] },
    lineNumber: 238,
  },
  {
    name: "value_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "value" },
      { type: "repetition", element: { type: "rule_reference", name: "value" } },
    ] },
    lineNumber: 251,
  },
  {
    name: "value",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "DIMENSION" },
      { type: "token_reference", name: "PERCENTAGE" },
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "IDENT" },
      { type: "token_reference", name: "HASH" },
      { type: "token_reference", name: "CUSTOM_PROPERTY" },
      { type: "token_reference", name: "UNICODE_RANGE" },
      { type: "rule_reference", name: "function_call" },
      { type: "token_reference", name: "SLASH" },
      { type: "token_reference", name: "COMMA" },
      { type: "token_reference", name: "PLUS" },
      { type: "token_reference", name: "MINUS" },
    ] },
    lineNumber: 253,
  },
  {
    name: "function_call",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "FUNCTION" },
        { type: "rule_reference", name: "function_args" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "token_reference", name: "URL_TOKEN" },
    ] },
    lineNumber: 267,
  },
  {
    name: "function_args",
    body: { type: "repetition", element: { type: "rule_reference", name: "function_arg" } },
    lineNumber: 272,
  },
  {
    name: "function_arg",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "DIMENSION" },
      { type: "token_reference", name: "PERCENTAGE" },
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "IDENT" },
      { type: "token_reference", name: "HASH" },
      { type: "token_reference", name: "CUSTOM_PROPERTY" },
      { type: "token_reference", name: "COMMA" },
      { type: "token_reference", name: "SLASH" },
      { type: "token_reference", name: "PLUS" },
      { type: "token_reference", name: "MINUS" },
      { type: "token_reference", name: "STAR" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "FUNCTION" },
        { type: "rule_reference", name: "function_args" },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 274,
  },
],
};
