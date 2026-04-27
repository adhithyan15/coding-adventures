// AUTO-GENERATED FILE — DO NOT EDIT
// Source: verilog.grammar
// Regenerate with: grammar-tools compile-grammar verilog.grammar
//
// This file embeds a ParserGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .grammar file at
// runtime.

import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const PARSER_GRAMMAR: ParserGrammar = {
  version: 0,
  rules: [
  {
    name: "source_text",
    body: { type: "repetition", element: { type: "rule_reference", name: "description" } },
    lineNumber: 45,
  },
  {
    name: "description",
    body: { type: "rule_reference", name: "module_declaration" },
    lineNumber: 47,
  },
  {
    name: "module_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "module" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "parameter_port_list" } },
      { type: "optional", element: { type: "rule_reference", name: "port_list" } },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "repetition", element: { type: "rule_reference", name: "module_item" } },
      { type: "literal", value: "endmodule" },
    ] },
    lineNumber: 76,
  },
  {
    name: "parameter_port_list",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "HASH" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "parameter_declaration" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "parameter_declaration" },
        ] } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 94,
  },
  {
    name: "parameter_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "parameter" },
      { type: "optional", element: { type: "rule_reference", name: "range" } },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 97,
  },
  {
    name: "localparam_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "localparam" },
      { type: "optional", element: { type: "rule_reference", name: "range" } },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 98,
  },
  {
    name: "port_list",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "port" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "port" },
        ] } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 118,
  },
  {
    name: "port",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "rule_reference", name: "port_direction" } },
      { type: "optional", element: { type: "rule_reference", name: "net_type" } },
      { type: "optional", element: { type: "literal", value: "signed" } },
      { type: "optional", element: { type: "rule_reference", name: "range" } },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 120,
  },
  {
    name: "port_direction",
    body: { type: "alternation", choices: [
      { type: "literal", value: "input" },
      { type: "literal", value: "output" },
      { type: "literal", value: "inout" },
    ] },
    lineNumber: 122,
  },
  {
    name: "net_type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "wire" },
      { type: "literal", value: "reg" },
      { type: "literal", value: "tri" },
      { type: "literal", value: "supply0" },
      { type: "literal", value: "supply1" },
    ] },
    lineNumber: 123,
  },
  {
    name: "range",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 125,
  },
  {
    name: "module_item",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "port_declaration" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "net_declaration" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "reg_declaration" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "integer_declaration" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "parameter_declaration" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "localparam_declaration" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "rule_reference", name: "continuous_assign" },
      { type: "rule_reference", name: "always_construct" },
      { type: "rule_reference", name: "initial_construct" },
      { type: "rule_reference", name: "module_instantiation" },
      { type: "rule_reference", name: "generate_region" },
      { type: "rule_reference", name: "function_declaration" },
      { type: "rule_reference", name: "task_declaration" },
    ] },
    lineNumber: 142,
  },
  {
    name: "port_declaration",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "port_direction" },
      { type: "optional", element: { type: "rule_reference", name: "net_type" } },
      { type: "optional", element: { type: "literal", value: "signed" } },
      { type: "optional", element: { type: "rule_reference", name: "range" } },
      { type: "rule_reference", name: "name_list" },
    ] },
    lineNumber: 177,
  },
  {
    name: "net_declaration",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "net_type" },
      { type: "optional", element: { type: "literal", value: "signed" } },
      { type: "optional", element: { type: "rule_reference", name: "range" } },
      { type: "rule_reference", name: "name_list" },
    ] },
    lineNumber: 179,
  },
  {
    name: "reg_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "reg" },
      { type: "optional", element: { type: "literal", value: "signed" } },
      { type: "optional", element: { type: "rule_reference", name: "range" } },
      { type: "rule_reference", name: "name_list" },
    ] },
    lineNumber: 180,
  },
  {
    name: "integer_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "integer" },
      { type: "rule_reference", name: "name_list" },
    ] },
    lineNumber: 181,
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
    lineNumber: 182,
  },
  {
    name: "continuous_assign",
    body: { type: "sequence", elements: [
      { type: "literal", value: "assign" },
      { type: "rule_reference", name: "assignment" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "assignment" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 201,
  },
  {
    name: "assignment",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "lvalue" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 202,
  },
  {
    name: "lvalue",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "optional", element: { type: "rule_reference", name: "range_select" } },
      ] },
      { type: "rule_reference", name: "concatenation" },
    ] },
    lineNumber: 206,
  },
  {
    name: "range_select",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "rule_reference", name: "expression" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "expression" },
        ] } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 209,
  },
  {
    name: "always_construct",
    body: { type: "sequence", elements: [
      { type: "literal", value: "always" },
      { type: "token_reference", name: "AT" },
      { type: "rule_reference", name: "sensitivity_list" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 246,
  },
  {
    name: "initial_construct",
    body: { type: "sequence", elements: [
      { type: "literal", value: "initial" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 247,
  },
  {
    name: "sensitivity_list",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "sensitivity_item" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "group", element: { type: "alternation", choices: [
                { type: "literal", value: "or" },
                { type: "token_reference", name: "COMMA" },
              ] } },
            { type: "rule_reference", name: "sensitivity_item" },
          ] } },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "token_reference", name: "STAR" },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 249,
  },
  {
    name: "sensitivity_item",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "alternation", choices: [
          { type: "literal", value: "posedge" },
          { type: "literal", value: "negedge" },
        ] } },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 253,
  },
  {
    name: "statement",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "block_statement" },
      { type: "rule_reference", name: "if_statement" },
      { type: "rule_reference", name: "case_statement" },
      { type: "rule_reference", name: "for_statement" },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "blocking_assignment" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "nonblocking_assignment" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "task_call" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 262,
  },
  {
    name: "block_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "begin" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "token_reference", name: "NAME" },
        ] } },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
      { type: "literal", value: "end" },
    ] },
    lineNumber: 278,
  },
  {
    name: "if_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "if" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "else" },
          { type: "rule_reference", name: "statement" },
        ] } },
    ] },
    lineNumber: 289,
  },
  {
    name: "case_statement",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "literal", value: "case" },
          { type: "literal", value: "casex" },
          { type: "literal", value: "casez" },
        ] } },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "repetition", element: { type: "rule_reference", name: "case_item" } },
      { type: "literal", value: "endcase" },
    ] },
    lineNumber: 304,
  },
  {
    name: "case_item",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "expression_list" },
        { type: "token_reference", name: "COLON" },
        { type: "rule_reference", name: "statement" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "default" },
        { type: "optional", element: { type: "token_reference", name: "COLON" } },
        { type: "rule_reference", name: "statement" },
      ] },
    ] },
    lineNumber: 309,
  },
  {
    name: "expression_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 312,
  },
  {
    name: "for_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "for" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "blocking_assignment" },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "rule_reference", name: "blocking_assignment" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 316,
  },
  {
    name: "blocking_assignment",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "lvalue" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 320,
  },
  {
    name: "nonblocking_assignment",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "lvalue" },
      { type: "token_reference", name: "LESS_EQUALS" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 321,
  },
  {
    name: "task_call",
    body: { type: "sequence", elements: [
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
    lineNumber: 324,
  },
  {
    name: "module_instantiation",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "parameter_value_assignment" } },
      { type: "rule_reference", name: "instance" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "instance" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 343,
  },
  {
    name: "parameter_value_assignment",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "HASH" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "expression" },
        ] } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 346,
  },
  {
    name: "instance",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "port_connections" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 348,
  },
  {
    name: "port_connections",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "named_port_connection" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "named_port_connection" },
          ] } },
      ] },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "expression" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "expression" },
            ] } },
        ] } },
    ] },
    lineNumber: 350,
  },
  {
    name: "named_port_connection",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "DOT" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 353,
  },
  {
    name: "generate_region",
    body: { type: "sequence", elements: [
      { type: "literal", value: "generate" },
      { type: "repetition", element: { type: "rule_reference", name: "generate_item" } },
      { type: "literal", value: "endgenerate" },
    ] },
    lineNumber: 380,
  },
  {
    name: "generate_item",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "genvar_declaration" },
      { type: "rule_reference", name: "generate_for" },
      { type: "rule_reference", name: "generate_if" },
      { type: "rule_reference", name: "module_item" },
    ] },
    lineNumber: 382,
  },
  {
    name: "genvar_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "genvar" },
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "token_reference", name: "NAME" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 387,
  },
  {
    name: "generate_for",
    body: { type: "sequence", elements: [
      { type: "literal", value: "for" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "genvar_assignment" },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "rule_reference", name: "genvar_assignment" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "generate_block" },
    ] },
    lineNumber: 389,
  },
  {
    name: "generate_if",
    body: { type: "sequence", elements: [
      { type: "literal", value: "if" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "generate_block" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "else" },
          { type: "rule_reference", name: "generate_block" },
        ] } },
    ] },
    lineNumber: 393,
  },
  {
    name: "generate_block",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "begin" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COLON" },
            { type: "token_reference", name: "NAME" },
          ] } },
        { type: "repetition", element: { type: "rule_reference", name: "generate_item" } },
        { type: "literal", value: "end" },
      ] },
      { type: "rule_reference", name: "generate_item" },
    ] },
    lineNumber: 396,
  },
  {
    name: "genvar_assignment",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 399,
  },
  {
    name: "function_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "function" },
      { type: "optional", element: { type: "rule_reference", name: "range" } },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "repetition", element: { type: "rule_reference", name: "function_item" } },
      { type: "rule_reference", name: "statement" },
      { type: "literal", value: "endfunction" },
    ] },
    lineNumber: 418,
  },
  {
    name: "function_item",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "port_declaration" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "reg_declaration" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "integer_declaration" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "parameter_declaration" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 423,
  },
  {
    name: "task_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "task" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "repetition", element: { type: "rule_reference", name: "task_item" } },
      { type: "rule_reference", name: "statement" },
      { type: "literal", value: "endtask" },
    ] },
    lineNumber: 428,
  },
  {
    name: "task_item",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "port_declaration" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "reg_declaration" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "integer_declaration" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 433,
  },
  {
    name: "expression",
    body: { type: "rule_reference", name: "ternary_expr" },
    lineNumber: 461,
  },
  {
    name: "ternary_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "or_expr" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "QUESTION" },
          { type: "rule_reference", name: "expression" },
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "ternary_expr" },
        ] } },
    ] },
    lineNumber: 467,
  },
  {
    name: "or_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "and_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LOGIC_OR" },
          { type: "rule_reference", name: "and_expr" },
        ] } },
    ] },
    lineNumber: 470,
  },
  {
    name: "and_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "bit_or_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LOGIC_AND" },
          { type: "rule_reference", name: "bit_or_expr" },
        ] } },
    ] },
    lineNumber: 471,
  },
  {
    name: "bit_or_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "bit_xor_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "PIPE" },
          { type: "rule_reference", name: "bit_xor_expr" },
        ] } },
    ] },
    lineNumber: 474,
  },
  {
    name: "bit_xor_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "bit_and_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "CARET" },
          { type: "rule_reference", name: "bit_and_expr" },
        ] } },
    ] },
    lineNumber: 475,
  },
  {
    name: "bit_and_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "equality_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "AMP" },
          { type: "rule_reference", name: "equality_expr" },
        ] } },
    ] },
    lineNumber: 476,
  },
  {
    name: "equality_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "relational_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "EQUALS_EQUALS" },
              { type: "token_reference", name: "NOT_EQUALS" },
              { type: "token_reference", name: "CASE_EQ" },
              { type: "token_reference", name: "CASE_NEQ" },
            ] } },
          { type: "rule_reference", name: "relational_expr" },
        ] } },
    ] },
    lineNumber: 480,
  },
  {
    name: "relational_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "shift_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "LESS_THAN" },
              { type: "token_reference", name: "LESS_EQUALS" },
              { type: "token_reference", name: "GREATER_THAN" },
              { type: "token_reference", name: "GREATER_EQUALS" },
            ] } },
          { type: "rule_reference", name: "shift_expr" },
        ] } },
    ] },
    lineNumber: 487,
  },
  {
    name: "shift_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "additive_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "LEFT_SHIFT" },
              { type: "token_reference", name: "RIGHT_SHIFT" },
              { type: "token_reference", name: "ARITH_LEFT_SHIFT" },
              { type: "token_reference", name: "ARITH_RIGHT_SHIFT" },
            ] } },
          { type: "rule_reference", name: "additive_expr" },
        ] } },
    ] },
    lineNumber: 492,
  },
  {
    name: "additive_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "multiplicative_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "PLUS" },
              { type: "token_reference", name: "MINUS" },
            ] } },
          { type: "rule_reference", name: "multiplicative_expr" },
        ] } },
    ] },
    lineNumber: 497,
  },
  {
    name: "multiplicative_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "power_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "STAR" },
              { type: "token_reference", name: "SLASH" },
              { type: "token_reference", name: "PERCENT" },
            ] } },
          { type: "rule_reference", name: "power_expr" },
        ] } },
    ] },
    lineNumber: 498,
  },
  {
    name: "power_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "unary_expr" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "POWER" },
          { type: "rule_reference", name: "unary_expr" },
        ] } },
    ] },
    lineNumber: 499,
  },
  {
    name: "unary_expr",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "group", element: { type: "alternation", choices: [
            { type: "token_reference", name: "PLUS" },
            { type: "token_reference", name: "MINUS" },
            { type: "token_reference", name: "BANG" },
            { type: "token_reference", name: "TILDE" },
            { type: "token_reference", name: "AMP" },
            { type: "token_reference", name: "PIPE" },
            { type: "token_reference", name: "CARET" },
            { type: "sequence", elements: [
              { type: "token_reference", name: "TILDE" },
              { type: "token_reference", name: "AMP" },
            ] },
            { type: "sequence", elements: [
              { type: "token_reference", name: "TILDE" },
              { type: "token_reference", name: "PIPE" },
            ] },
            { type: "sequence", elements: [
              { type: "token_reference", name: "TILDE" },
              { type: "token_reference", name: "CARET" },
            ] },
          ] } },
        { type: "rule_reference", name: "unary_expr" },
      ] },
      { type: "rule_reference", name: "primary" },
    ] },
    lineNumber: 511,
  },
  {
    name: "primary",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "SIZED_NUMBER" },
      { type: "token_reference", name: "REAL_NUMBER" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "SYSTEM_ID" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "rule_reference", name: "concatenation" },
      { type: "rule_reference", name: "replication" },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "primary" },
        { type: "token_reference", name: "LBRACKET" },
        { type: "rule_reference", name: "expression" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COLON" },
            { type: "rule_reference", name: "expression" },
          ] } },
        { type: "token_reference", name: "RBRACKET" },
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
    ] },
    lineNumber: 521,
  },
  {
    name: "concatenation",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "expression" },
        ] } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 537,
  },
  {
    name: "replication",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "expression" },
      { type: "rule_reference", name: "concatenation" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 543,
  },
],
};
