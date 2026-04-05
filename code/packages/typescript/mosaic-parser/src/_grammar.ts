// AUTO-GENERATED FILE — DO NOT EDIT
// Source: mosaic.grammar
// Regenerate with: grammar-tools compile-grammar mosaic.grammar
//
// This file embeds a ParserGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .grammar file at runtime.

import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const PARSER_GRAMMAR: ParserGrammar = {
  version: 1,
  rules: [
  // file = { import_decl } component_decl ;
  {
    name: "file",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "import_decl" } },
      { type: "rule_reference", name: "component_decl" },
    ] },
    lineNumber: 20,
  },
  // import_decl = KEYWORD NAME [ KEYWORD NAME ] KEYWORD STRING SEMICOLON ;
  // Covers: import Button from "...";
  //         import Card as InfoCard from "...";
  {
    name: "import_decl",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "KEYWORD" },   // import
      { type: "token_reference", name: "NAME" },      // component name
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "KEYWORD" }, // as
          { type: "token_reference", name: "NAME" },    // alias
        ] } },
      { type: "token_reference", name: "KEYWORD" },   // from
      { type: "token_reference", name: "STRING" },    // path
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 30,
  },
  // component_decl = KEYWORD NAME LBRACE { slot_decl } node_tree RBRACE ;
  {
    name: "component_decl",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "KEYWORD" },   // component
      { type: "token_reference", name: "NAME" },      // component name
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "slot_decl" } },
      { type: "rule_reference", name: "node_tree" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 48,
  },
  // slot_decl = KEYWORD NAME COLON slot_type [ EQUALS default_value ] SEMICOLON ;
  {
    name: "slot_decl",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "KEYWORD" },   // slot
      { type: "token_reference", name: "NAME" },      // slot name
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "slot_type" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "default_value" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 67,
  },
  // slot_type = KEYWORD | NAME | list_type ;
  // list_type starts with KEYWORD("list") LANGLE, so try it first.
  {
    name: "slot_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "list_type" },
      { type: "token_reference", name: "KEYWORD" },   // primitive types: text, number, bool, image, color, node
      { type: "token_reference", name: "NAME" },      // component type: Button, Card, etc.
    ] },
    lineNumber: 69,
  },
  // list_type = KEYWORD LANGLE slot_type RANGLE ;
  {
    name: "list_type",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "KEYWORD" },   // list
      { type: "token_reference", name: "LANGLE" },
      { type: "rule_reference", name: "slot_type" },
      { type: "token_reference", name: "RANGLE" },
    ] },
    lineNumber: 73,
  },
  // default_value = STRING | NUMBER | DIMENSION | COLOR_HEX | KEYWORD ;
  {
    name: "default_value",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "DIMENSION" },
      { type: "token_reference", name: "COLOR_HEX" },
      { type: "token_reference", name: "KEYWORD" },   // true, false
    ] },
    lineNumber: 75,
  },
  // node_tree = node_element ;
  {
    name: "node_tree",
    body: { type: "rule_reference", name: "node_element" },
    lineNumber: 86,
  },
  // node_element = NAME LBRACE { node_content } RBRACE ;
  {
    name: "node_element",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },      // element type (Column, Text, Row, etc.)
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "node_content" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 88,
  },
  // node_content = property_assignment | child_node | slot_reference | when_block | each_block ;
  // Order matters: property_assignment starts with (NAME|KEYWORD) COLON.
  // child_node starts with NAME LBRACE. when_block/each_block start with KEYWORD LBRACE/AT.
  // slot_reference starts with AT. The parser backtracks on failure.
  {
    name: "node_content",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "property_assignment" },
      { type: "rule_reference", name: "child_node" },
      { type: "rule_reference", name: "slot_reference" },
      { type: "rule_reference", name: "when_block" },
      { type: "rule_reference", name: "each_block" },
    ] },
    lineNumber: 90,
  },
  // property_assignment = (NAME | KEYWORD) COLON property_value SEMICOLON ;
  // Allows keywords (e.g., "color", "text", "node") as property names so that
  // Mosaic slot-type keywords can double as layout/style property identifiers.
  {
    name: "property_assignment",
    body: { type: "sequence", elements: [
      { type: "alternation", choices: [
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "KEYWORD" },
      ] },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "property_value" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 107,
  },
  // property_value = slot_ref | STRING | NUMBER | DIMENSION | COLOR_HEX | KEYWORD | enum_value | NAME ;
  // enum_value (NAME.NAME) comes before NAME so the longer match is tried first.
  {
    name: "property_value",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "slot_ref" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "DIMENSION" },  // before NUMBER (same digit prefix)
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "COLOR_HEX" },
      { type: "token_reference", name: "KEYWORD" },
      { type: "rule_reference", name: "enum_value" }, // NAME.NAME — try before bare NAME
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 109,
  },
  // slot_ref = AT NAME ;
  {
    name: "slot_ref",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "AT" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 118,
  },
  // enum_value = NAME DOT NAME ;
  {
    name: "enum_value",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "DOT" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 120,
  },
  // child_node = node_element ;
  {
    name: "child_node",
    body: { type: "rule_reference", name: "node_element" },
    lineNumber: 127,
  },
  // slot_reference = AT NAME SEMICOLON ;
  {
    name: "slot_reference",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "AT" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 140,
  },
  // when_block = KEYWORD slot_ref LBRACE { node_content } RBRACE ;
  {
    name: "when_block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "KEYWORD" },   // when
      { type: "rule_reference", name: "slot_ref" },
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "node_content" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 152,
  },
  // each_block = KEYWORD slot_ref KEYWORD NAME LBRACE { node_content } RBRACE ;
  {
    name: "each_block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "KEYWORD" },   // each
      { type: "rule_reference", name: "slot_ref" },
      { type: "token_reference", name: "KEYWORD" },   // as
      { type: "token_reference", name: "NAME" },      // loop variable
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "node_content" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 166,
  },
],
};
