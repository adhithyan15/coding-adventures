// AUTO-GENERATED FILE - DO NOT EDIT
import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const VhdlGrammar: ParserGrammar = {
  version: 0,
  rules: [
    {
      name: "design_file",
      lineNumber: 64,
      body: { type: "repetition", element: { type: "rule_reference", name: "design_unit" } }
    },
    {
      name: "design_unit",
      lineNumber: 66,
      body: { type: "sequence", elements: [{ type: "repetition", element: { type: "rule_reference", name: "context_item" } }, { type: "rule_reference", name: "library_unit" }] }
    },
    {
      name: "context_item",
      lineNumber: 68,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "library_clause" }, { type: "rule_reference", name: "use_clause" }] }
    },
    {
      name: "library_clause",
      lineNumber: 71,
      body: { type: "sequence", elements: [{ type: "literal", value: "library" }, { type: "rule_reference", name: "name_list" }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "use_clause",
      lineNumber: 74,
      body: { type: "sequence", elements: [{ type: "literal", value: "use" }, { type: "rule_reference", name: "selected_name" }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "selected_name",
      lineNumber: 77,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "DOT" }, { type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "NAME" }, { type: "literal", value: "all" }] } }] } }] }
    },
    {
      name: "name_list",
      lineNumber: 79,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "token_reference", name: "NAME" }] } }] }
    },
    {
      name: "library_unit",
      lineNumber: 81,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "entity_declaration" }, { type: "rule_reference", name: "architecture_body" }, { type: "rule_reference", name: "package_declaration" }, { type: "rule_reference", name: "package_body" }] }
    },
    {
      name: "entity_declaration",
      lineNumber: 112,
      body: { type: "sequence", elements: [{ type: "literal", value: "entity" }, { type: "token_reference", name: "NAME" }, { type: "literal", value: "is" }, { type: "optional", element: { type: "rule_reference", name: "generic_clause" } }, { type: "optional", element: { type: "rule_reference", name: "port_clause" } }, { type: "literal", value: "end" }, { type: "optional", element: { type: "literal", value: "entity" } }, { type: "optional", element: { type: "token_reference", name: "NAME" } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "generic_clause",
      lineNumber: 117,
      body: { type: "sequence", elements: [{ type: "literal", value: "generic" }, { type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "interface_list" }, { type: "token_reference", name: "RPAREN" }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "port_clause",
      lineNumber: 118,
      body: { type: "sequence", elements: [{ type: "literal", value: "port" }, { type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "interface_list" }, { type: "token_reference", name: "RPAREN" }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "interface_list",
      lineNumber: 123,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "interface_element" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "SEMICOLON" }, { type: "rule_reference", name: "interface_element" }] } }] }
    },
    {
      name: "interface_element",
      lineNumber: 124,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "name_list" }, { type: "token_reference", name: "COLON" }, { type: "optional", element: { type: "rule_reference", name: "mode" } }, { type: "rule_reference", name: "subtype_indication" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "VAR_ASSIGN" }, { type: "rule_reference", name: "expression" }] } }] }
    },
    {
      name: "mode",
      lineNumber: 132,
      body: { type: "alternation", choices: [{ type: "literal", value: "in" }, { type: "literal", value: "out" }, { type: "literal", value: "inout" }, { type: "literal", value: "buffer" }] }
    },
    {
      name: "architecture_body",
      lineNumber: 154,
      body: { type: "sequence", elements: [{ type: "literal", value: "architecture" }, { type: "token_reference", name: "NAME" }, { type: "literal", value: "of" }, { type: "token_reference", name: "NAME" }, { type: "literal", value: "is" }, { type: "repetition", element: { type: "rule_reference", name: "block_declarative_item" } }, { type: "literal", value: "begin" }, { type: "repetition", element: { type: "rule_reference", name: "concurrent_statement" } }, { type: "literal", value: "end" }, { type: "optional", element: { type: "literal", value: "architecture" } }, { type: "optional", element: { type: "token_reference", name: "NAME" } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "block_declarative_item",
      lineNumber: 160,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "signal_declaration" }, { type: "rule_reference", name: "constant_declaration" }, { type: "rule_reference", name: "type_declaration" }, { type: "rule_reference", name: "subtype_declaration" }, { type: "rule_reference", name: "component_declaration" }, { type: "rule_reference", name: "function_declaration" }, { type: "rule_reference", name: "function_body" }, { type: "rule_reference", name: "procedure_declaration" }, { type: "rule_reference", name: "procedure_body" }] }
    },
    {
      name: "signal_declaration",
      lineNumber: 189,
      body: { type: "sequence", elements: [{ type: "literal", value: "signal" }, { type: "rule_reference", name: "name_list" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "subtype_indication" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "VAR_ASSIGN" }, { type: "rule_reference", name: "expression" }] } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "constant_declaration",
      lineNumber: 191,
      body: { type: "sequence", elements: [{ type: "literal", value: "constant" }, { type: "rule_reference", name: "name_list" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "subtype_indication" }, { type: "token_reference", name: "VAR_ASSIGN" }, { type: "rule_reference", name: "expression" }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "variable_declaration",
      lineNumber: 193,
      body: { type: "sequence", elements: [{ type: "literal", value: "variable" }, { type: "rule_reference", name: "name_list" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "subtype_indication" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "VAR_ASSIGN" }, { type: "rule_reference", name: "expression" }] } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "type_declaration",
      lineNumber: 218,
      body: { type: "sequence", elements: [{ type: "literal", value: "type" }, { type: "token_reference", name: "NAME" }, { type: "literal", value: "is" }, { type: "rule_reference", name: "type_definition" }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "subtype_declaration",
      lineNumber: 219,
      body: { type: "sequence", elements: [{ type: "literal", value: "subtype" }, { type: "token_reference", name: "NAME" }, { type: "literal", value: "is" }, { type: "rule_reference", name: "subtype_indication" }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "type_definition",
      lineNumber: 221,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "enumeration_type" }, { type: "rule_reference", name: "array_type" }, { type: "rule_reference", name: "record_type" }] }
    },
    {
      name: "enumeration_type",
      lineNumber: 227,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "CHAR_LITERAL" }] } }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "CHAR_LITERAL" }] } }] } }, { type: "token_reference", name: "RPAREN" }] }
    },
    {
      name: "array_type",
      lineNumber: 232,
      body: { type: "sequence", elements: [{ type: "literal", value: "array" }, { type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "index_constraint" }, { type: "token_reference", name: "RPAREN" }, { type: "literal", value: "of" }, { type: "rule_reference", name: "subtype_indication" }] }
    },
    {
      name: "index_constraint",
      lineNumber: 234,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "discrete_range" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "discrete_range" }] } }] }
    },
    {
      name: "discrete_range",
      lineNumber: 235,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "subtype_indication" }, { type: "sequence", elements: [{ type: "rule_reference", name: "expression" }, { type: "group", element: { type: "alternation", choices: [{ type: "literal", value: "to" }, { type: "literal", value: "downto" }] } }, { type: "rule_reference", name: "expression" }] }] }
    },
    {
      name: "record_type",
      lineNumber: 239,
      body: { type: "sequence", elements: [{ type: "literal", value: "record" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "subtype_indication" }, { type: "token_reference", name: "SEMICOLON" }] } }, { type: "literal", value: "end" }, { type: "literal", value: "record" }, { type: "optional", element: { type: "token_reference", name: "NAME" } }] }
    },
    {
      name: "subtype_indication",
      lineNumber: 247,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "selected_name" }, { type: "optional", element: { type: "rule_reference", name: "constraint" } }] }
    },
    {
      name: "constraint",
      lineNumber: 249,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "expression" }, { type: "group", element: { type: "alternation", choices: [{ type: "literal", value: "to" }, { type: "literal", value: "downto" }] } }, { type: "rule_reference", name: "expression" }, { type: "token_reference", name: "RPAREN" }] }, { type: "sequence", elements: [{ type: "literal", value: "range" }, { type: "rule_reference", name: "expression" }, { type: "group", element: { type: "alternation", choices: [{ type: "literal", value: "to" }, { type: "literal", value: "downto" }] } }, { type: "rule_reference", name: "expression" }] }] }
    },
    {
      name: "concurrent_statement",
      lineNumber: 264,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "process_statement" }, { type: "rule_reference", name: "signal_assignment_concurrent" }, { type: "rule_reference", name: "component_instantiation" }, { type: "rule_reference", name: "generate_statement" }] }
    },
    {
      name: "signal_assignment_concurrent",
      lineNumber: 272,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "LESS_EQUALS" }, { type: "rule_reference", name: "waveform" }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "waveform",
      lineNumber: 274,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "waveform_element" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "waveform_element" }] } }] }
    },
    {
      name: "waveform_element",
      lineNumber: 275,
      body: { type: "rule_reference", name: "expression" }
    },
    {
      name: "process_statement",
      lineNumber: 307,
      body: { type: "sequence", elements: [{ type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "COLON" }] } }, { type: "literal", value: "process" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "sensitivity_list" }, { type: "token_reference", name: "RPAREN" }] } }, { type: "optional", element: { type: "literal", value: "is" } }, { type: "repetition", element: { type: "rule_reference", name: "process_declarative_item" } }, { type: "literal", value: "begin" }, { type: "repetition", element: { type: "rule_reference", name: "sequential_statement" } }, { type: "literal", value: "end" }, { type: "literal", value: "process" }, { type: "optional", element: { type: "token_reference", name: "NAME" } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "sensitivity_list",
      lineNumber: 315,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "token_reference", name: "NAME" }] } }] }
    },
    {
      name: "process_declarative_item",
      lineNumber: 317,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "variable_declaration" }, { type: "rule_reference", name: "constant_declaration" }, { type: "rule_reference", name: "type_declaration" }, { type: "rule_reference", name: "subtype_declaration" }] }
    },
    {
      name: "sequential_statement",
      lineNumber: 329,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "signal_assignment_seq" }, { type: "rule_reference", name: "variable_assignment" }, { type: "rule_reference", name: "if_statement" }, { type: "rule_reference", name: "case_statement" }, { type: "rule_reference", name: "loop_statement" }, { type: "rule_reference", name: "return_statement" }, { type: "rule_reference", name: "null_statement" }] }
    },
    {
      name: "signal_assignment_seq",
      lineNumber: 342,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "LESS_EQUALS" }, { type: "rule_reference", name: "waveform" }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "variable_assignment",
      lineNumber: 346,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "VAR_ASSIGN" }, { type: "rule_reference", name: "expression" }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "if_statement",
      lineNumber: 356,
      body: { type: "sequence", elements: [{ type: "literal", value: "if" }, { type: "rule_reference", name: "expression" }, { type: "literal", value: "then" }, { type: "repetition", element: { type: "rule_reference", name: "sequential_statement" } }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: "elsif" }, { type: "rule_reference", name: "expression" }, { type: "literal", value: "then" }, { type: "repetition", element: { type: "rule_reference", name: "sequential_statement" } }] } }, { type: "optional", element: { type: "sequence", elements: [{ type: "literal", value: "else" }, { type: "repetition", element: { type: "rule_reference", name: "sequential_statement" } }] } }, { type: "literal", value: "end" }, { type: "literal", value: "if" }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "case_statement",
      lineNumber: 372,
      body: { type: "sequence", elements: [{ type: "literal", value: "case" }, { type: "rule_reference", name: "expression" }, { type: "literal", value: "is" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: "when" }, { type: "rule_reference", name: "choices" }, { type: "token_reference", name: "ARROW" }, { type: "repetition", element: { type: "rule_reference", name: "sequential_statement" } }] } }, { type: "literal", value: "end" }, { type: "literal", value: "case" }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "choices",
      lineNumber: 376,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "choice" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "PIPE" }, { type: "rule_reference", name: "choice" }] } }] }
    },
    {
      name: "choice",
      lineNumber: 377,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "expression" }, { type: "rule_reference", name: "discrete_range" }, { type: "literal", value: "others" }] }
    },
    {
      name: "loop_statement",
      lineNumber: 391,
      body: { type: "sequence", elements: [{ type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "COLON" }] } }, { type: "optional", element: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "literal", value: "for" }, { type: "token_reference", name: "NAME" }, { type: "literal", value: "in" }, { type: "rule_reference", name: "discrete_range" }] }, { type: "sequence", elements: [{ type: "literal", value: "while" }, { type: "rule_reference", name: "expression" }] }] } }, { type: "literal", value: "loop" }, { type: "repetition", element: { type: "rule_reference", name: "sequential_statement" } }, { type: "literal", value: "end" }, { type: "literal", value: "loop" }, { type: "optional", element: { type: "token_reference", name: "NAME" } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "return_statement",
      lineNumber: 398,
      body: { type: "sequence", elements: [{ type: "literal", value: "return" }, { type: "optional", element: { type: "rule_reference", name: "expression" } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "null_statement",
      lineNumber: 399,
      body: { type: "sequence", elements: [{ type: "literal", value: "null" }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "component_declaration",
      lineNumber: 425,
      body: { type: "sequence", elements: [{ type: "literal", value: "component" }, { type: "token_reference", name: "NAME" }, { type: "optional", element: { type: "literal", value: "is" } }, { type: "optional", element: { type: "rule_reference", name: "generic_clause" } }, { type: "optional", element: { type: "rule_reference", name: "port_clause" } }, { type: "literal", value: "end" }, { type: "literal", value: "component" }, { type: "optional", element: { type: "token_reference", name: "NAME" } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "component_instantiation",
      lineNumber: 430,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "COLON" }, { type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "NAME" }, { type: "sequence", elements: [{ type: "literal", value: "entity" }, { type: "rule_reference", name: "selected_name" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "token_reference", name: "NAME" }, { type: "token_reference", name: "RPAREN" }] } }] }] } }, { type: "optional", element: { type: "sequence", elements: [{ type: "literal", value: "generic" }, { type: "literal", value: "map" }, { type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "association_list" }, { type: "token_reference", name: "RPAREN" }] } }, { type: "optional", element: { type: "sequence", elements: [{ type: "literal", value: "port" }, { type: "literal", value: "map" }, { type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "association_list" }, { type: "token_reference", name: "RPAREN" }] } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "association_list",
      lineNumber: 437,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "association_element" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "association_element" }] } }] }
    },
    {
      name: "association_element",
      lineNumber: 438,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "ARROW" }] } }, { type: "rule_reference", name: "expression" }] }, { type: "sequence", elements: [{ type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "ARROW" }] } }, { type: "literal", value: "open" }] }] }
    },
    {
      name: "generate_statement",
      lineNumber: 461,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "COLON" }, { type: "group", element: { type: "alternation", choices: [{ type: "rule_reference", name: "for_generate" }, { type: "rule_reference", name: "if_generate" }] } }] }
    },
    {
      name: "for_generate",
      lineNumber: 463,
      body: { type: "sequence", elements: [{ type: "literal", value: "for" }, { type: "token_reference", name: "NAME" }, { type: "literal", value: "in" }, { type: "rule_reference", name: "discrete_range" }, { type: "literal", value: "generate" }, { type: "repetition", element: { type: "rule_reference", name: "concurrent_statement" } }, { type: "literal", value: "end" }, { type: "literal", value: "generate" }, { type: "optional", element: { type: "token_reference", name: "NAME" } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "if_generate",
      lineNumber: 467,
      body: { type: "sequence", elements: [{ type: "literal", value: "if" }, { type: "rule_reference", name: "expression" }, { type: "literal", value: "generate" }, { type: "repetition", element: { type: "rule_reference", name: "concurrent_statement" } }, { type: "literal", value: "end" }, { type: "literal", value: "generate" }, { type: "optional", element: { type: "token_reference", name: "NAME" } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "package_declaration",
      lineNumber: 488,
      body: { type: "sequence", elements: [{ type: "literal", value: "package" }, { type: "token_reference", name: "NAME" }, { type: "literal", value: "is" }, { type: "repetition", element: { type: "rule_reference", name: "package_declarative_item" } }, { type: "literal", value: "end" }, { type: "optional", element: { type: "literal", value: "package" } }, { type: "optional", element: { type: "token_reference", name: "NAME" } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "package_body",
      lineNumber: 492,
      body: { type: "sequence", elements: [{ type: "literal", value: "package" }, { type: "literal", value: "body" }, { type: "token_reference", name: "NAME" }, { type: "literal", value: "is" }, { type: "repetition", element: { type: "rule_reference", name: "package_body_declarative_item" } }, { type: "literal", value: "end" }, { type: "optional", element: { type: "sequence", elements: [{ type: "literal", value: "package" }, { type: "literal", value: "body" }] } }, { type: "optional", element: { type: "token_reference", name: "NAME" } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "package_declarative_item",
      lineNumber: 496,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "type_declaration" }, { type: "rule_reference", name: "subtype_declaration" }, { type: "rule_reference", name: "constant_declaration" }, { type: "rule_reference", name: "signal_declaration" }, { type: "rule_reference", name: "component_declaration" }, { type: "rule_reference", name: "function_declaration" }, { type: "rule_reference", name: "procedure_declaration" }] }
    },
    {
      name: "package_body_declarative_item",
      lineNumber: 504,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "type_declaration" }, { type: "rule_reference", name: "subtype_declaration" }, { type: "rule_reference", name: "constant_declaration" }, { type: "rule_reference", name: "function_body" }, { type: "rule_reference", name: "procedure_body" }] }
    },
    {
      name: "function_declaration",
      lineNumber: 520,
      body: { type: "sequence", elements: [{ type: "optional", element: { type: "alternation", choices: [{ type: "literal", value: "pure" }, { type: "literal", value: "impure" }] } }, { type: "literal", value: "function" }, { type: "token_reference", name: "NAME" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "interface_list" }, { type: "token_reference", name: "RPAREN" }] } }, { type: "literal", value: "return" }, { type: "rule_reference", name: "subtype_indication" }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "function_body",
      lineNumber: 525,
      body: { type: "sequence", elements: [{ type: "optional", element: { type: "alternation", choices: [{ type: "literal", value: "pure" }, { type: "literal", value: "impure" }] } }, { type: "literal", value: "function" }, { type: "token_reference", name: "NAME" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "interface_list" }, { type: "token_reference", name: "RPAREN" }] } }, { type: "literal", value: "return" }, { type: "rule_reference", name: "subtype_indication" }, { type: "literal", value: "is" }, { type: "repetition", element: { type: "rule_reference", name: "process_declarative_item" } }, { type: "literal", value: "begin" }, { type: "repetition", element: { type: "rule_reference", name: "sequential_statement" } }, { type: "literal", value: "end" }, { type: "optional", element: { type: "literal", value: "function" } }, { type: "optional", element: { type: "token_reference", name: "NAME" } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "procedure_declaration",
      lineNumber: 534,
      body: { type: "sequence", elements: [{ type: "literal", value: "procedure" }, { type: "token_reference", name: "NAME" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "interface_list" }, { type: "token_reference", name: "RPAREN" }] } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "procedure_body",
      lineNumber: 537,
      body: { type: "sequence", elements: [{ type: "literal", value: "procedure" }, { type: "token_reference", name: "NAME" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "interface_list" }, { type: "token_reference", name: "RPAREN" }] } }, { type: "literal", value: "is" }, { type: "repetition", element: { type: "rule_reference", name: "process_declarative_item" } }, { type: "literal", value: "begin" }, { type: "repetition", element: { type: "rule_reference", name: "sequential_statement" } }, { type: "literal", value: "end" }, { type: "optional", element: { type: "literal", value: "procedure" } }, { type: "optional", element: { type: "token_reference", name: "NAME" } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "expression",
      lineNumber: 574,
      body: { type: "rule_reference", name: "logical_expr" }
    },
    {
      name: "logical_expr",
      lineNumber: 581,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "relation" }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "logical_op" }, { type: "rule_reference", name: "relation" }] } }] }
    },
    {
      name: "logical_op",
      lineNumber: 582,
      body: { type: "alternation", choices: [{ type: "literal", value: "and" }, { type: "literal", value: "or" }, { type: "literal", value: "xor" }, { type: "literal", value: "nand" }, { type: "literal", value: "nor" }, { type: "literal", value: "xnor" }] }
    },
    {
      name: "relation",
      lineNumber: 586,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "shift_expr" }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "relational_op" }, { type: "rule_reference", name: "shift_expr" }] } }] }
    },
    {
      name: "relational_op",
      lineNumber: 587,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "EQUALS" }, { type: "token_reference", name: "NOT_EQUALS" }, { type: "token_reference", name: "LESS_THAN" }, { type: "token_reference", name: "LESS_EQUALS" }, { type: "token_reference", name: "GREATER_THAN" }, { type: "token_reference", name: "GREATER_EQUALS" }] }
    },
    {
      name: "shift_expr",
      lineNumber: 592,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "adding_expr" }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "shift_op" }, { type: "rule_reference", name: "adding_expr" }] } }] }
    },
    {
      name: "shift_op",
      lineNumber: 593,
      body: { type: "alternation", choices: [{ type: "literal", value: "sll" }, { type: "literal", value: "srl" }, { type: "literal", value: "sla" }, { type: "literal", value: "sra" }, { type: "literal", value: "rol" }, { type: "literal", value: "ror" }] }
    },
    {
      name: "adding_expr",
      lineNumber: 597,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "multiplying_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "rule_reference", name: "adding_op" }, { type: "rule_reference", name: "multiplying_expr" }] } }] }
    },
    {
      name: "adding_op",
      lineNumber: 598,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "PLUS" }, { type: "token_reference", name: "MINUS" }, { type: "token_reference", name: "AMPERSAND" }] }
    },
    {
      name: "multiplying_expr",
      lineNumber: 601,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "unary_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "rule_reference", name: "multiplying_op" }, { type: "rule_reference", name: "unary_expr" }] } }] }
    },
    {
      name: "multiplying_op",
      lineNumber: 602,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "STAR" }, { type: "token_reference", name: "SLASH" }, { type: "literal", value: "mod" }, { type: "literal", value: "rem" }] }
    },
    {
      name: "unary_expr",
      lineNumber: 605,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "literal", value: "abs" }, { type: "rule_reference", name: "unary_expr" }] }, { type: "sequence", elements: [{ type: "literal", value: "not" }, { type: "rule_reference", name: "unary_expr" }] }, { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "PLUS" }, { type: "token_reference", name: "MINUS" }] } }, { type: "rule_reference", name: "unary_expr" }] }, { type: "rule_reference", name: "power_expr" }] }
    },
    {
      name: "power_expr",
      lineNumber: 611,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "primary" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "POWER" }, { type: "rule_reference", name: "primary" }] } }] }
    },
    {
      name: "primary",
      lineNumber: 619,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "NUMBER" }, { type: "token_reference", name: "REAL_NUMBER" }, { type: "token_reference", name: "BASED_LITERAL" }, { type: "token_reference", name: "STRING" }, { type: "token_reference", name: "CHAR_LITERAL" }, { type: "token_reference", name: "BIT_STRING" }, { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "TICK" }, { type: "token_reference", name: "NAME" }] } }] }, { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "LPAREN" }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "expression" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "expression" }] } }] } }, { type: "token_reference", name: "RPAREN" }] }, { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "expression" }, { type: "token_reference", name: "RPAREN" }] }, { type: "rule_reference", name: "aggregate" }, { type: "literal", value: "null" }] }
    },
    {
      name: "aggregate",
      lineNumber: 635,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "element_association" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "element_association" }] } }, { type: "token_reference", name: "RPAREN" }] }
    },
    {
      name: "element_association",
      lineNumber: 636,
      body: { type: "sequence", elements: [{ type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "choices" }, { type: "token_reference", name: "ARROW" }] } }, { type: "rule_reference", name: "expression" }] }
    },
  ]
};
