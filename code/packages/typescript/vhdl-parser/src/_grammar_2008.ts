// AUTO-GENERATED FILE — DO NOT EDIT
// Source: vhdl.grammar
// Regenerate with: grammar-tools compile-grammar vhdl.grammar
//
// This file embeds a ParserGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .grammar file at
// runtime.

import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const PARSER_GRAMMAR: ParserGrammar = {
  version: 0,
  rules: [
  {
    name: "design_file",
    body: { type: "repetition", element: { type: "rule_reference", name: "design_unit" } },
    lineNumber: 64,
  },
  {
    name: "design_unit",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "context_item" } },
      { type: "rule_reference", name: "library_unit" },
    ] },
    lineNumber: 66,
  },
  {
    name: "context_item",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "library_clause" },
      { type: "rule_reference", name: "use_clause" },
    ] },
    lineNumber: 68,
  },
  {
    name: "library_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "library" },
      { type: "rule_reference", name: "name_list" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 71,
  },
  {
    name: "use_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "use" },
      { type: "rule_reference", name: "selected_name" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 74,
  },
  {
    name: "selected_name",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "DOT" },
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "NAME" },
              { type: "literal", value: "all" },
            ] } },
        ] } },
    ] },
    lineNumber: 77,
  },
  {
    name: "name_list",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 79,
  },
  {
    name: "library_unit",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "entity_declaration" },
      { type: "rule_reference", name: "architecture_body" },
      { type: "rule_reference", name: "package_declaration" },
      { type: "rule_reference", name: "package_body" },
    ] },
    lineNumber: 81,
  },
  {
    name: "entity_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "entity" },
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "is" },
      { type: "optional", element: { type: "rule_reference", name: "generic_clause" } },
      { type: "optional", element: { type: "rule_reference", name: "port_clause" } },
      { type: "literal", value: "end" },
      { type: "optional", element: { type: "literal", value: "entity" } },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 112,
  },
  {
    name: "generic_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "generic" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "interface_list" },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 117,
  },
  {
    name: "port_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "port" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "interface_list" },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 118,
  },
  {
    name: "interface_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "interface_element" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "SEMICOLON" },
          { type: "rule_reference", name: "interface_element" },
        ] } },
    ] },
    lineNumber: 123,
  },
  {
    name: "interface_element",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "name_list" },
      { type: "token_reference", name: "COLON" },
      { type: "optional", element: { type: "rule_reference", name: "mode" } },
      { type: "rule_reference", name: "subtype_indication" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "VAR_ASSIGN" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 124,
  },
  {
    name: "mode",
    body: { type: "alternation", choices: [
      { type: "literal", value: "in" },
      { type: "literal", value: "out" },
      { type: "literal", value: "inout" },
      { type: "literal", value: "buffer" },
    ] },
    lineNumber: 132,
  },
  {
    name: "architecture_body",
    body: { type: "sequence", elements: [
      { type: "literal", value: "architecture" },
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "of" },
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "is" },
      { type: "repetition", element: { type: "rule_reference", name: "block_declarative_item" } },
      { type: "literal", value: "begin" },
      { type: "repetition", element: { type: "rule_reference", name: "concurrent_statement" } },
      { type: "literal", value: "end" },
      { type: "optional", element: { type: "literal", value: "architecture" } },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 154,
  },
  {
    name: "block_declarative_item",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "signal_declaration" },
      { type: "rule_reference", name: "constant_declaration" },
      { type: "rule_reference", name: "type_declaration" },
      { type: "rule_reference", name: "subtype_declaration" },
      { type: "rule_reference", name: "component_declaration" },
      { type: "rule_reference", name: "function_declaration" },
      { type: "rule_reference", name: "function_body" },
      { type: "rule_reference", name: "procedure_declaration" },
      { type: "rule_reference", name: "procedure_body" },
    ] },
    lineNumber: 160,
  },
  {
    name: "signal_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "signal" },
      { type: "rule_reference", name: "name_list" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "subtype_indication" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "VAR_ASSIGN" },
          { type: "rule_reference", name: "expression" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 189,
  },
  {
    name: "constant_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "constant" },
      { type: "rule_reference", name: "name_list" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "subtype_indication" },
      { type: "token_reference", name: "VAR_ASSIGN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 191,
  },
  {
    name: "variable_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "variable" },
      { type: "rule_reference", name: "name_list" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "subtype_indication" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "VAR_ASSIGN" },
          { type: "rule_reference", name: "expression" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 193,
  },
  {
    name: "type_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "is" },
      { type: "rule_reference", name: "type_definition" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 218,
  },
  {
    name: "subtype_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "subtype" },
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "is" },
      { type: "rule_reference", name: "subtype_indication" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 219,
  },
  {
    name: "type_definition",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "enumeration_type" },
      { type: "rule_reference", name: "array_type" },
      { type: "rule_reference", name: "record_type" },
    ] },
    lineNumber: 221,
  },
  {
    name: "enumeration_type",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "CHAR_LITERAL" },
        ] } },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "NAME" },
              { type: "token_reference", name: "CHAR_LITERAL" },
            ] } },
        ] } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 227,
  },
  {
    name: "array_type",
    body: { type: "sequence", elements: [
      { type: "literal", value: "array" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "index_constraint" },
      { type: "token_reference", name: "RPAREN" },
      { type: "literal", value: "of" },
      { type: "rule_reference", name: "subtype_indication" },
    ] },
    lineNumber: 232,
  },
  {
    name: "index_constraint",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "discrete_range" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "discrete_range" },
        ] } },
    ] },
    lineNumber: 234,
  },
  {
    name: "discrete_range",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "subtype_indication" },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "expression" },
        { type: "group", element: { type: "alternation", choices: [
            { type: "literal", value: "to" },
            { type: "literal", value: "downto" },
          ] } },
        { type: "rule_reference", name: "expression" },
      ] },
    ] },
    lineNumber: 235,
  },
  {
    name: "record_type",
    body: { type: "sequence", elements: [
      { type: "literal", value: "record" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "subtype_indication" },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
      { type: "literal", value: "end" },
      { type: "literal", value: "record" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
    ] },
    lineNumber: 239,
  },
  {
    name: "subtype_indication",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "selected_name" },
      { type: "optional", element: { type: "rule_reference", name: "constraint" } },
    ] },
    lineNumber: 247,
  },
  {
    name: "constraint",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expression" },
        { type: "group", element: { type: "alternation", choices: [
            { type: "literal", value: "to" },
            { type: "literal", value: "downto" },
          ] } },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "range" },
        { type: "rule_reference", name: "expression" },
        { type: "group", element: { type: "alternation", choices: [
            { type: "literal", value: "to" },
            { type: "literal", value: "downto" },
          ] } },
        { type: "rule_reference", name: "expression" },
      ] },
    ] },
    lineNumber: 249,
  },
  {
    name: "concurrent_statement",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "process_statement" },
      { type: "rule_reference", name: "signal_assignment_concurrent" },
      { type: "rule_reference", name: "component_instantiation" },
      { type: "rule_reference", name: "generate_statement" },
    ] },
    lineNumber: 264,
  },
  {
    name: "signal_assignment_concurrent",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LESS_EQUALS" },
      { type: "rule_reference", name: "waveform" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 272,
  },
  {
    name: "waveform",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "waveform_element" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "waveform_element" },
        ] } },
    ] },
    lineNumber: 274,
  },
  {
    name: "waveform_element",
    body: { type: "rule_reference", name: "expression" },
    lineNumber: 275,
  },
  {
    name: "process_statement",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "COLON" },
        ] } },
      { type: "literal", value: "process" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "rule_reference", name: "sensitivity_list" },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "optional", element: { type: "literal", value: "is" } },
      { type: "repetition", element: { type: "rule_reference", name: "process_declarative_item" } },
      { type: "literal", value: "begin" },
      { type: "repetition", element: { type: "rule_reference", name: "sequential_statement" } },
      { type: "literal", value: "end" },
      { type: "literal", value: "process" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 307,
  },
  {
    name: "sensitivity_list",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 315,
  },
  {
    name: "process_declarative_item",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "variable_declaration" },
      { type: "rule_reference", name: "constant_declaration" },
      { type: "rule_reference", name: "type_declaration" },
      { type: "rule_reference", name: "subtype_declaration" },
    ] },
    lineNumber: 317,
  },
  {
    name: "sequential_statement",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "signal_assignment_seq" },
      { type: "rule_reference", name: "variable_assignment" },
      { type: "rule_reference", name: "if_statement" },
      { type: "rule_reference", name: "case_statement" },
      { type: "rule_reference", name: "loop_statement" },
      { type: "rule_reference", name: "return_statement" },
      { type: "rule_reference", name: "null_statement" },
    ] },
    lineNumber: 329,
  },
  {
    name: "signal_assignment_seq",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LESS_EQUALS" },
      { type: "rule_reference", name: "waveform" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 342,
  },
  {
    name: "variable_assignment",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "VAR_ASSIGN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 346,
  },
  {
    name: "if_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "if" },
      { type: "rule_reference", name: "expression" },
      { type: "literal", value: "then" },
      { type: "repetition", element: { type: "rule_reference", name: "sequential_statement" } },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "elsif" },
          { type: "rule_reference", name: "expression" },
          { type: "literal", value: "then" },
          { type: "repetition", element: { type: "rule_reference", name: "sequential_statement" } },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "else" },
          { type: "repetition", element: { type: "rule_reference", name: "sequential_statement" } },
        ] } },
      { type: "literal", value: "end" },
      { type: "literal", value: "if" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 356,
  },
  {
    name: "case_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "case" },
      { type: "rule_reference", name: "expression" },
      { type: "literal", value: "is" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "when" },
          { type: "rule_reference", name: "choices" },
          { type: "token_reference", name: "ARROW" },
          { type: "repetition", element: { type: "rule_reference", name: "sequential_statement" } },
        ] } },
      { type: "literal", value: "end" },
      { type: "literal", value: "case" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 372,
  },
  {
    name: "choices",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "choice" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "PIPE" },
          { type: "rule_reference", name: "choice" },
        ] } },
    ] },
    lineNumber: 376,
  },
  {
    name: "choice",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "expression" },
      { type: "rule_reference", name: "discrete_range" },
      { type: "literal", value: "others" },
    ] },
    lineNumber: 377,
  },
  {
    name: "loop_statement",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "COLON" },
        ] } },
      { type: "optional", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "literal", value: "for" },
            { type: "token_reference", name: "NAME" },
            { type: "literal", value: "in" },
            { type: "rule_reference", name: "discrete_range" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "while" },
            { type: "rule_reference", name: "expression" },
          ] },
        ] } },
      { type: "literal", value: "loop" },
      { type: "repetition", element: { type: "rule_reference", name: "sequential_statement" } },
      { type: "literal", value: "end" },
      { type: "literal", value: "loop" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 391,
  },
  {
    name: "return_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "return" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 398,
  },
  {
    name: "null_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "null" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 399,
  },
  {
    name: "component_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "component" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "literal", value: "is" } },
      { type: "optional", element: { type: "rule_reference", name: "generic_clause" } },
      { type: "optional", element: { type: "rule_reference", name: "port_clause" } },
      { type: "literal", value: "end" },
      { type: "literal", value: "component" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 425,
  },
  {
    name: "component_instantiation",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "NAME" },
          { type: "sequence", elements: [
            { type: "literal", value: "entity" },
            { type: "rule_reference", name: "selected_name" },
            { type: "optional", element: { type: "sequence", elements: [
                { type: "token_reference", name: "LPAREN" },
                { type: "token_reference", name: "NAME" },
                { type: "token_reference", name: "RPAREN" },
              ] } },
          ] },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "generic" },
          { type: "literal", value: "map" },
          { type: "token_reference", name: "LPAREN" },
          { type: "rule_reference", name: "association_list" },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "port" },
          { type: "literal", value: "map" },
          { type: "token_reference", name: "LPAREN" },
          { type: "rule_reference", name: "association_list" },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 430,
  },
  {
    name: "association_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "association_element" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "association_element" },
        ] } },
    ] },
    lineNumber: 437,
  },
  {
    name: "association_element",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "NAME" },
            { type: "token_reference", name: "ARROW" },
          ] } },
        { type: "rule_reference", name: "expression" },
      ] },
      { type: "sequence", elements: [
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "NAME" },
            { type: "token_reference", name: "ARROW" },
          ] } },
        { type: "literal", value: "open" },
      ] },
    ] },
    lineNumber: 438,
  },
  {
    name: "generate_statement",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "for_generate" },
          { type: "rule_reference", name: "if_generate" },
        ] } },
    ] },
    lineNumber: 461,
  },
  {
    name: "for_generate",
    body: { type: "sequence", elements: [
      { type: "literal", value: "for" },
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "discrete_range" },
      { type: "literal", value: "generate" },
      { type: "repetition", element: { type: "rule_reference", name: "concurrent_statement" } },
      { type: "literal", value: "end" },
      { type: "literal", value: "generate" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 463,
  },
  {
    name: "if_generate",
    body: { type: "sequence", elements: [
      { type: "literal", value: "if" },
      { type: "rule_reference", name: "expression" },
      { type: "literal", value: "generate" },
      { type: "repetition", element: { type: "rule_reference", name: "concurrent_statement" } },
      { type: "literal", value: "end" },
      { type: "literal", value: "generate" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 467,
  },
  {
    name: "package_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "package" },
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "is" },
      { type: "repetition", element: { type: "rule_reference", name: "package_declarative_item" } },
      { type: "literal", value: "end" },
      { type: "optional", element: { type: "literal", value: "package" } },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 488,
  },
  {
    name: "package_body",
    body: { type: "sequence", elements: [
      { type: "literal", value: "package" },
      { type: "literal", value: "body" },
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "is" },
      { type: "repetition", element: { type: "rule_reference", name: "package_body_declarative_item" } },
      { type: "literal", value: "end" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "package" },
          { type: "literal", value: "body" },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 492,
  },
  {
    name: "package_declarative_item",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "type_declaration" },
      { type: "rule_reference", name: "subtype_declaration" },
      { type: "rule_reference", name: "constant_declaration" },
      { type: "rule_reference", name: "signal_declaration" },
      { type: "rule_reference", name: "component_declaration" },
      { type: "rule_reference", name: "function_declaration" },
      { type: "rule_reference", name: "procedure_declaration" },
    ] },
    lineNumber: 496,
  },
  {
    name: "package_body_declarative_item",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "type_declaration" },
      { type: "rule_reference", name: "subtype_declaration" },
      { type: "rule_reference", name: "constant_declaration" },
      { type: "rule_reference", name: "function_body" },
      { type: "rule_reference", name: "procedure_body" },
    ] },
    lineNumber: 504,
  },
  {
    name: "function_declaration",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "alternation", choices: [
          { type: "literal", value: "pure" },
          { type: "literal", value: "impure" },
        ] } },
      { type: "literal", value: "function" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "rule_reference", name: "interface_list" },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "literal", value: "return" },
      { type: "rule_reference", name: "subtype_indication" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 520,
  },
  {
    name: "function_body",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "alternation", choices: [
          { type: "literal", value: "pure" },
          { type: "literal", value: "impure" },
        ] } },
      { type: "literal", value: "function" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "rule_reference", name: "interface_list" },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "literal", value: "return" },
      { type: "rule_reference", name: "subtype_indication" },
      { type: "literal", value: "is" },
      { type: "repetition", element: { type: "rule_reference", name: "process_declarative_item" } },
      { type: "literal", value: "begin" },
      { type: "repetition", element: { type: "rule_reference", name: "sequential_statement" } },
      { type: "literal", value: "end" },
      { type: "optional", element: { type: "literal", value: "function" } },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 525,
  },
  {
    name: "procedure_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "procedure" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "rule_reference", name: "interface_list" },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 534,
  },
  {
    name: "procedure_body",
    body: { type: "sequence", elements: [
      { type: "literal", value: "procedure" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "rule_reference", name: "interface_list" },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "literal", value: "is" },
      { type: "repetition", element: { type: "rule_reference", name: "process_declarative_item" } },
      { type: "literal", value: "begin" },
      { type: "repetition", element: { type: "rule_reference", name: "sequential_statement" } },
      { type: "literal", value: "end" },
      { type: "optional", element: { type: "literal", value: "procedure" } },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 537,
  },
  {
    name: "expression",
    body: { type: "rule_reference", name: "logical_expr" },
    lineNumber: 574,
  },
  {
    name: "logical_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "relation" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "logical_op" },
          { type: "rule_reference", name: "relation" },
        ] } },
    ] },
    lineNumber: 581,
  },
  {
    name: "logical_op",
    body: { type: "alternation", choices: [
      { type: "literal", value: "and" },
      { type: "literal", value: "or" },
      { type: "literal", value: "xor" },
      { type: "literal", value: "nand" },
      { type: "literal", value: "nor" },
      { type: "literal", value: "xnor" },
    ] },
    lineNumber: 582,
  },
  {
    name: "relation",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "shift_expr" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "relational_op" },
          { type: "rule_reference", name: "shift_expr" },
        ] } },
    ] },
    lineNumber: 586,
  },
  {
    name: "relational_op",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "EQUALS" },
      { type: "token_reference", name: "NOT_EQUALS" },
      { type: "token_reference", name: "LESS_THAN" },
      { type: "token_reference", name: "LESS_EQUALS" },
      { type: "token_reference", name: "GREATER_THAN" },
      { type: "token_reference", name: "GREATER_EQUALS" },
    ] },
    lineNumber: 587,
  },
  {
    name: "shift_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "adding_expr" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "shift_op" },
          { type: "rule_reference", name: "adding_expr" },
        ] } },
    ] },
    lineNumber: 592,
  },
  {
    name: "shift_op",
    body: { type: "alternation", choices: [
      { type: "literal", value: "sll" },
      { type: "literal", value: "srl" },
      { type: "literal", value: "sla" },
      { type: "literal", value: "sra" },
      { type: "literal", value: "rol" },
      { type: "literal", value: "ror" },
    ] },
    lineNumber: 593,
  },
  {
    name: "adding_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "multiplying_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "adding_op" },
          { type: "rule_reference", name: "multiplying_expr" },
        ] } },
    ] },
    lineNumber: 597,
  },
  {
    name: "adding_op",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "PLUS" },
      { type: "token_reference", name: "MINUS" },
      { type: "token_reference", name: "AMPERSAND" },
    ] },
    lineNumber: 598,
  },
  {
    name: "multiplying_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "unary_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "multiplying_op" },
          { type: "rule_reference", name: "unary_expr" },
        ] } },
    ] },
    lineNumber: 601,
  },
  {
    name: "multiplying_op",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "STAR" },
      { type: "token_reference", name: "SLASH" },
      { type: "literal", value: "mod" },
      { type: "literal", value: "rem" },
    ] },
    lineNumber: 602,
  },
  {
    name: "unary_expr",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "abs" },
        { type: "rule_reference", name: "unary_expr" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "not" },
        { type: "rule_reference", name: "unary_expr" },
      ] },
      { type: "sequence", elements: [
        { type: "group", element: { type: "alternation", choices: [
            { type: "token_reference", name: "PLUS" },
            { type: "token_reference", name: "MINUS" },
          ] } },
        { type: "rule_reference", name: "unary_expr" },
      ] },
      { type: "rule_reference", name: "power_expr" },
    ] },
    lineNumber: 605,
  },
  {
    name: "power_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "primary" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "POWER" },
          { type: "rule_reference", name: "primary" },
        ] } },
    ] },
    lineNumber: 611,
  },
  {
    name: "primary",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "REAL_NUMBER" },
      { type: "token_reference", name: "BASED_LITERAL" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "CHAR_LITERAL" },
      { type: "token_reference", name: "BIT_STRING" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "TICK" },
            { type: "token_reference", name: "NAME" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "expression" },
            { type: "repetition", element: { type: "sequence", elements: [
                { type: "token_reference", name: "COMMA" },
                { type: "rule_reference", name: "expression" },
              ] } },
          ] } },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "rule_reference", name: "aggregate" },
      { type: "literal", value: "null" },
    ] },
    lineNumber: 619,
  },
  {
    name: "aggregate",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "element_association" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "element_association" },
        ] } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 635,
  },
  {
    name: "element_association",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "choices" },
          { type: "token_reference", name: "ARROW" },
        ] } },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 636,
  },
],
};
