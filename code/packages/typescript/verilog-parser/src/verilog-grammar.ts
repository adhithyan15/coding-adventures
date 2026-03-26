// AUTO-GENERATED FILE - DO NOT EDIT
import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const VerilogGrammar: ParserGrammar = {
  version: 0,
  rules: [
    {
      name: "source_text",
      lineNumber: 42,
      body: { type: "repetition", element: { type: "rule_reference", name: "description" } }
    },
    {
      name: "description",
      lineNumber: 44,
      body: { type: "rule_reference", name: "module_declaration" }
    },
    {
      name: "module_declaration",
      lineNumber: 73,
      body: { type: "sequence", elements: [{ type: "literal", value: "module" }, { type: "token_reference", name: "NAME" }, { type: "optional", element: { type: "rule_reference", name: "parameter_port_list" } }, { type: "optional", element: { type: "rule_reference", name: "port_list" } }, { type: "token_reference", name: "SEMICOLON" }, { type: "repetition", element: { type: "rule_reference", name: "module_item" } }, { type: "literal", value: "endmodule" }] }
    },
    {
      name: "parameter_port_list",
      lineNumber: 91,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "HASH" }, { type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "parameter_declaration" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "parameter_declaration" }] } }, { type: "token_reference", name: "RPAREN" }] }
    },
    {
      name: "parameter_declaration",
      lineNumber: 94,
      body: { type: "sequence", elements: [{ type: "literal", value: "parameter" }, { type: "optional", element: { type: "rule_reference", name: "range" } }, { type: "token_reference", name: "NAME" }, { type: "token_reference", name: "EQUALS" }, { type: "rule_reference", name: "expression" }] }
    },
    {
      name: "localparam_declaration",
      lineNumber: 95,
      body: { type: "sequence", elements: [{ type: "literal", value: "localparam" }, { type: "optional", element: { type: "rule_reference", name: "range" } }, { type: "token_reference", name: "NAME" }, { type: "token_reference", name: "EQUALS" }, { type: "rule_reference", name: "expression" }] }
    },
    {
      name: "port_list",
      lineNumber: 115,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "port" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "port" }] } }, { type: "token_reference", name: "RPAREN" }] }
    },
    {
      name: "port",
      lineNumber: 117,
      body: { type: "sequence", elements: [{ type: "optional", element: { type: "rule_reference", name: "port_direction" } }, { type: "optional", element: { type: "rule_reference", name: "net_type" } }, { type: "optional", element: { type: "literal", value: "signed" } }, { type: "optional", element: { type: "rule_reference", name: "range" } }, { type: "token_reference", name: "NAME" }] }
    },
    {
      name: "port_direction",
      lineNumber: 119,
      body: { type: "alternation", choices: [{ type: "literal", value: "input" }, { type: "literal", value: "output" }, { type: "literal", value: "inout" }] }
    },
    {
      name: "net_type",
      lineNumber: 120,
      body: { type: "alternation", choices: [{ type: "literal", value: "wire" }, { type: "literal", value: "reg" }, { type: "literal", value: "tri" }, { type: "literal", value: "supply0" }, { type: "literal", value: "supply1" }] }
    },
    {
      name: "range",
      lineNumber: 122,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LBRACKET" }, { type: "rule_reference", name: "expression" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "expression" }, { type: "token_reference", name: "RBRACKET" }] }
    },
    {
      name: "module_item",
      lineNumber: 139,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "rule_reference", name: "port_declaration" }, { type: "token_reference", name: "SEMICOLON" }] }, { type: "sequence", elements: [{ type: "rule_reference", name: "net_declaration" }, { type: "token_reference", name: "SEMICOLON" }] }, { type: "sequence", elements: [{ type: "rule_reference", name: "reg_declaration" }, { type: "token_reference", name: "SEMICOLON" }] }, { type: "sequence", elements: [{ type: "rule_reference", name: "integer_declaration" }, { type: "token_reference", name: "SEMICOLON" }] }, { type: "sequence", elements: [{ type: "rule_reference", name: "parameter_declaration" }, { type: "token_reference", name: "SEMICOLON" }] }, { type: "sequence", elements: [{ type: "rule_reference", name: "localparam_declaration" }, { type: "token_reference", name: "SEMICOLON" }] }, { type: "rule_reference", name: "continuous_assign" }, { type: "rule_reference", name: "always_construct" }, { type: "rule_reference", name: "initial_construct" }, { type: "rule_reference", name: "module_instantiation" }, { type: "rule_reference", name: "generate_region" }, { type: "rule_reference", name: "function_declaration" }, { type: "rule_reference", name: "task_declaration" }] }
    },
    {
      name: "port_declaration",
      lineNumber: 174,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "port_direction" }, { type: "optional", element: { type: "rule_reference", name: "net_type" } }, { type: "optional", element: { type: "literal", value: "signed" } }, { type: "optional", element: { type: "rule_reference", name: "range" } }, { type: "rule_reference", name: "name_list" }] }
    },
    {
      name: "net_declaration",
      lineNumber: 176,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "net_type" }, { type: "optional", element: { type: "literal", value: "signed" } }, { type: "optional", element: { type: "rule_reference", name: "range" } }, { type: "rule_reference", name: "name_list" }] }
    },
    {
      name: "reg_declaration",
      lineNumber: 177,
      body: { type: "sequence", elements: [{ type: "literal", value: "reg" }, { type: "optional", element: { type: "literal", value: "signed" } }, { type: "optional", element: { type: "rule_reference", name: "range" } }, { type: "rule_reference", name: "name_list" }] }
    },
    {
      name: "integer_declaration",
      lineNumber: 178,
      body: { type: "sequence", elements: [{ type: "literal", value: "integer" }, { type: "rule_reference", name: "name_list" }] }
    },
    {
      name: "name_list",
      lineNumber: 179,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "token_reference", name: "NAME" }] } }] }
    },
    {
      name: "continuous_assign",
      lineNumber: 198,
      body: { type: "sequence", elements: [{ type: "literal", value: "assign" }, { type: "rule_reference", name: "assignment" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "assignment" }] } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "assignment",
      lineNumber: 199,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "lvalue" }, { type: "token_reference", name: "EQUALS" }, { type: "rule_reference", name: "expression" }] }
    },
    {
      name: "lvalue",
      lineNumber: 203,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "optional", element: { type: "rule_reference", name: "range_select" } }] }, { type: "rule_reference", name: "concatenation" }] }
    },
    {
      name: "range_select",
      lineNumber: 206,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LBRACKET" }, { type: "rule_reference", name: "expression" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "expression" }] } }, { type: "token_reference", name: "RBRACKET" }] }
    },
    {
      name: "always_construct",
      lineNumber: 243,
      body: { type: "sequence", elements: [{ type: "literal", value: "always" }, { type: "token_reference", name: "AT" }, { type: "rule_reference", name: "sensitivity_list" }, { type: "rule_reference", name: "statement" }] }
    },
    {
      name: "initial_construct",
      lineNumber: 244,
      body: { type: "sequence", elements: [{ type: "literal", value: "initial" }, { type: "rule_reference", name: "statement" }] }
    },
    {
      name: "sensitivity_list",
      lineNumber: 246,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "sensitivity_item" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "literal", value: "or" }, { type: "token_reference", name: "COMMA" }] } }, { type: "rule_reference", name: "sensitivity_item" }] } }, { type: "token_reference", name: "RPAREN" }] }, { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "token_reference", name: "STAR" }, { type: "token_reference", name: "RPAREN" }] }] }
    },
    {
      name: "sensitivity_item",
      lineNumber: 250,
      body: { type: "sequence", elements: [{ type: "optional", element: { type: "alternation", choices: [{ type: "literal", value: "posedge" }, { type: "literal", value: "negedge" }] } }, { type: "rule_reference", name: "expression" }] }
    },
    {
      name: "statement",
      lineNumber: 259,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "block_statement" }, { type: "rule_reference", name: "if_statement" }, { type: "rule_reference", name: "case_statement" }, { type: "rule_reference", name: "for_statement" }, { type: "sequence", elements: [{ type: "rule_reference", name: "blocking_assignment" }, { type: "token_reference", name: "SEMICOLON" }] }, { type: "sequence", elements: [{ type: "rule_reference", name: "nonblocking_assignment" }, { type: "token_reference", name: "SEMICOLON" }] }, { type: "sequence", elements: [{ type: "rule_reference", name: "task_call" }, { type: "token_reference", name: "SEMICOLON" }] }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "block_statement",
      lineNumber: 275,
      body: { type: "sequence", elements: [{ type: "literal", value: "begin" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "COLON" }, { type: "token_reference", name: "NAME" }] } }, { type: "repetition", element: { type: "rule_reference", name: "statement" } }, { type: "literal", value: "end" }] }
    },
    {
      name: "if_statement",
      lineNumber: 286,
      body: { type: "sequence", elements: [{ type: "literal", value: "if" }, { type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "expression" }, { type: "token_reference", name: "RPAREN" }, { type: "rule_reference", name: "statement" }, { type: "optional", element: { type: "sequence", elements: [{ type: "literal", value: "else" }, { type: "rule_reference", name: "statement" }] } }] }
    },
    {
      name: "case_statement",
      lineNumber: 301,
      body: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "literal", value: "case" }, { type: "literal", value: "casex" }, { type: "literal", value: "casez" }] } }, { type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "expression" }, { type: "token_reference", name: "RPAREN" }, { type: "repetition", element: { type: "rule_reference", name: "case_item" } }, { type: "literal", value: "endcase" }] }
    },
    {
      name: "case_item",
      lineNumber: 306,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "rule_reference", name: "expression_list" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "statement" }] }, { type: "sequence", elements: [{ type: "literal", value: "default" }, { type: "optional", element: { type: "token_reference", name: "COLON" } }, { type: "rule_reference", name: "statement" }] }] }
    },
    {
      name: "expression_list",
      lineNumber: 309,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "expression" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "expression" }] } }] }
    },
    {
      name: "for_statement",
      lineNumber: 313,
      body: { type: "sequence", elements: [{ type: "literal", value: "for" }, { type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "blocking_assignment" }, { type: "token_reference", name: "SEMICOLON" }, { type: "rule_reference", name: "expression" }, { type: "token_reference", name: "SEMICOLON" }, { type: "rule_reference", name: "blocking_assignment" }, { type: "token_reference", name: "RPAREN" }, { type: "rule_reference", name: "statement" }] }
    },
    {
      name: "blocking_assignment",
      lineNumber: 317,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "lvalue" }, { type: "token_reference", name: "EQUALS" }, { type: "rule_reference", name: "expression" }] }
    },
    {
      name: "nonblocking_assignment",
      lineNumber: 318,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "lvalue" }, { type: "token_reference", name: "LESS_EQUALS" }, { type: "rule_reference", name: "expression" }] }
    },
    {
      name: "task_call",
      lineNumber: 321,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "LPAREN" }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "expression" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "expression" }] } }] } }, { type: "token_reference", name: "RPAREN" }] }
    },
    {
      name: "module_instantiation",
      lineNumber: 340,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "optional", element: { type: "rule_reference", name: "parameter_value_assignment" } }, { type: "rule_reference", name: "instance" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "instance" }] } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "parameter_value_assignment",
      lineNumber: 343,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "HASH" }, { type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "expression" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "expression" }] } }, { type: "token_reference", name: "RPAREN" }] }
    },
    {
      name: "instance",
      lineNumber: 345,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "port_connections" }, { type: "token_reference", name: "RPAREN" }] }
    },
    {
      name: "port_connections",
      lineNumber: 347,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "rule_reference", name: "named_port_connection" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "named_port_connection" }] } }] }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "expression" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "expression" }] } }] } }] }
    },
    {
      name: "named_port_connection",
      lineNumber: 350,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "DOT" }, { type: "token_reference", name: "NAME" }, { type: "token_reference", name: "LPAREN" }, { type: "optional", element: { type: "rule_reference", name: "expression" } }, { type: "token_reference", name: "RPAREN" }] }
    },
    {
      name: "generate_region",
      lineNumber: 377,
      body: { type: "sequence", elements: [{ type: "literal", value: "generate" }, { type: "repetition", element: { type: "rule_reference", name: "generate_item" } }, { type: "literal", value: "endgenerate" }] }
    },
    {
      name: "generate_item",
      lineNumber: 379,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "genvar_declaration" }, { type: "rule_reference", name: "generate_for" }, { type: "rule_reference", name: "generate_if" }, { type: "rule_reference", name: "module_item" }] }
    },
    {
      name: "genvar_declaration",
      lineNumber: 384,
      body: { type: "sequence", elements: [{ type: "literal", value: "genvar" }, { type: "token_reference", name: "NAME" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "token_reference", name: "NAME" }] } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "generate_for",
      lineNumber: 386,
      body: { type: "sequence", elements: [{ type: "literal", value: "for" }, { type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "genvar_assignment" }, { type: "token_reference", name: "SEMICOLON" }, { type: "rule_reference", name: "expression" }, { type: "token_reference", name: "SEMICOLON" }, { type: "rule_reference", name: "genvar_assignment" }, { type: "token_reference", name: "RPAREN" }, { type: "rule_reference", name: "generate_block" }] }
    },
    {
      name: "generate_if",
      lineNumber: 390,
      body: { type: "sequence", elements: [{ type: "literal", value: "if" }, { type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "expression" }, { type: "token_reference", name: "RPAREN" }, { type: "rule_reference", name: "generate_block" }, { type: "optional", element: { type: "sequence", elements: [{ type: "literal", value: "else" }, { type: "rule_reference", name: "generate_block" }] } }] }
    },
    {
      name: "generate_block",
      lineNumber: 393,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "literal", value: "begin" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "COLON" }, { type: "token_reference", name: "NAME" }] } }, { type: "repetition", element: { type: "rule_reference", name: "generate_item" } }, { type: "literal", value: "end" }] }, { type: "rule_reference", name: "generate_item" }] }
    },
    {
      name: "genvar_assignment",
      lineNumber: 396,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "EQUALS" }, { type: "rule_reference", name: "expression" }] }
    },
    {
      name: "function_declaration",
      lineNumber: 415,
      body: { type: "sequence", elements: [{ type: "literal", value: "function" }, { type: "optional", element: { type: "rule_reference", name: "range" } }, { type: "token_reference", name: "NAME" }, { type: "token_reference", name: "SEMICOLON" }, { type: "repetition", element: { type: "rule_reference", name: "function_item" } }, { type: "rule_reference", name: "statement" }, { type: "literal", value: "endfunction" }] }
    },
    {
      name: "function_item",
      lineNumber: 420,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "rule_reference", name: "port_declaration" }, { type: "token_reference", name: "SEMICOLON" }] }, { type: "sequence", elements: [{ type: "rule_reference", name: "reg_declaration" }, { type: "token_reference", name: "SEMICOLON" }] }, { type: "sequence", elements: [{ type: "rule_reference", name: "integer_declaration" }, { type: "token_reference", name: "SEMICOLON" }] }, { type: "sequence", elements: [{ type: "rule_reference", name: "parameter_declaration" }, { type: "token_reference", name: "SEMICOLON" }] }] }
    },
    {
      name: "task_declaration",
      lineNumber: 425,
      body: { type: "sequence", elements: [{ type: "literal", value: "task" }, { type: "token_reference", name: "NAME" }, { type: "token_reference", name: "SEMICOLON" }, { type: "repetition", element: { type: "rule_reference", name: "task_item" } }, { type: "rule_reference", name: "statement" }, { type: "literal", value: "endtask" }] }
    },
    {
      name: "task_item",
      lineNumber: 430,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "rule_reference", name: "port_declaration" }, { type: "token_reference", name: "SEMICOLON" }] }, { type: "sequence", elements: [{ type: "rule_reference", name: "reg_declaration" }, { type: "token_reference", name: "SEMICOLON" }] }, { type: "sequence", elements: [{ type: "rule_reference", name: "integer_declaration" }, { type: "token_reference", name: "SEMICOLON" }] }] }
    },
    {
      name: "expression",
      lineNumber: 458,
      body: { type: "rule_reference", name: "ternary_expr" }
    },
    {
      name: "ternary_expr",
      lineNumber: 464,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "or_expr" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "QUESTION" }, { type: "rule_reference", name: "expression" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "ternary_expr" }] } }] }
    },
    {
      name: "or_expr",
      lineNumber: 467,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "and_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "LOGIC_OR" }, { type: "rule_reference", name: "and_expr" }] } }] }
    },
    {
      name: "and_expr",
      lineNumber: 468,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "bit_or_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "LOGIC_AND" }, { type: "rule_reference", name: "bit_or_expr" }] } }] }
    },
    {
      name: "bit_or_expr",
      lineNumber: 471,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "bit_xor_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "PIPE" }, { type: "rule_reference", name: "bit_xor_expr" }] } }] }
    },
    {
      name: "bit_xor_expr",
      lineNumber: 472,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "bit_and_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "CARET" }, { type: "rule_reference", name: "bit_and_expr" }] } }] }
    },
    {
      name: "bit_and_expr",
      lineNumber: 473,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "equality_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "AMP" }, { type: "rule_reference", name: "equality_expr" }] } }] }
    },
    {
      name: "equality_expr",
      lineNumber: 477,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "relational_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "EQUALS_EQUALS" }, { type: "token_reference", name: "NOT_EQUALS" }, { type: "token_reference", name: "CASE_EQ" }, { type: "token_reference", name: "CASE_NEQ" }] } }, { type: "rule_reference", name: "relational_expr" }] } }] }
    },
    {
      name: "relational_expr",
      lineNumber: 484,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "shift_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "LESS_THAN" }, { type: "token_reference", name: "LESS_EQUALS" }, { type: "token_reference", name: "GREATER_THAN" }, { type: "token_reference", name: "GREATER_EQUALS" }] } }, { type: "rule_reference", name: "shift_expr" }] } }] }
    },
    {
      name: "shift_expr",
      lineNumber: 489,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "additive_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "LEFT_SHIFT" }, { type: "token_reference", name: "RIGHT_SHIFT" }, { type: "token_reference", name: "ARITH_LEFT_SHIFT" }, { type: "token_reference", name: "ARITH_RIGHT_SHIFT" }] } }, { type: "rule_reference", name: "additive_expr" }] } }] }
    },
    {
      name: "additive_expr",
      lineNumber: 494,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "multiplicative_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "PLUS" }, { type: "token_reference", name: "MINUS" }] } }, { type: "rule_reference", name: "multiplicative_expr" }] } }] }
    },
    {
      name: "multiplicative_expr",
      lineNumber: 495,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "power_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "STAR" }, { type: "token_reference", name: "SLASH" }, { type: "token_reference", name: "PERCENT" }] } }, { type: "rule_reference", name: "power_expr" }] } }] }
    },
    {
      name: "power_expr",
      lineNumber: 496,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "unary_expr" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "POWER" }, { type: "rule_reference", name: "unary_expr" }] } }] }
    },
    {
      name: "unary_expr",
      lineNumber: 508,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "PLUS" }, { type: "token_reference", name: "MINUS" }, { type: "token_reference", name: "BANG" }, { type: "token_reference", name: "TILDE" }, { type: "token_reference", name: "AMP" }, { type: "token_reference", name: "PIPE" }, { type: "token_reference", name: "CARET" }, { type: "sequence", elements: [{ type: "token_reference", name: "TILDE" }, { type: "token_reference", name: "AMP" }] }, { type: "sequence", elements: [{ type: "token_reference", name: "TILDE" }, { type: "token_reference", name: "PIPE" }] }, { type: "sequence", elements: [{ type: "token_reference", name: "TILDE" }, { type: "token_reference", name: "CARET" }] }] } }, { type: "rule_reference", name: "unary_expr" }] }, { type: "rule_reference", name: "primary" }] }
    },
    {
      name: "primary",
      lineNumber: 518,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "NUMBER" }, { type: "token_reference", name: "SIZED_NUMBER" }, { type: "token_reference", name: "REAL_NUMBER" }, { type: "token_reference", name: "STRING" }, { type: "token_reference", name: "NAME" }, { type: "token_reference", name: "SYSTEM_ID" }, { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "expression" }, { type: "token_reference", name: "RPAREN" }] }, { type: "rule_reference", name: "concatenation" }, { type: "rule_reference", name: "replication" }, { type: "sequence", elements: [{ type: "rule_reference", name: "primary" }, { type: "token_reference", name: "LBRACKET" }, { type: "rule_reference", name: "expression" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "expression" }] } }, { type: "token_reference", name: "RBRACKET" }] }, { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "LPAREN" }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "expression" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "expression" }] } }] } }, { type: "token_reference", name: "RPAREN" }] }] }
    },
    {
      name: "concatenation",
      lineNumber: 534,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LBRACE" }, { type: "rule_reference", name: "expression" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "expression" }] } }, { type: "token_reference", name: "RBRACE" }] }
    },
    {
      name: "replication",
      lineNumber: 540,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LBRACE" }, { type: "rule_reference", name: "expression" }, { type: "rule_reference", name: "concatenation" }, { type: "token_reference", name: "RBRACE" }] }
    },
  ]
};
