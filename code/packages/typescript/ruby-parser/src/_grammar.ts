// AUTO-GENERATED FILE - DO NOT EDIT
// Source: ruby.grammar
// Regenerate with: grammar-tools compile-grammar ruby.grammar
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
    body: { type: "repetition", element: { type: "rule_reference", name: "statement" } },
    lineNumber: 27,
  },
  {
    name: "statement",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "endless_def_statement" },
      { type: "rule_reference", name: "def_statement" },
      { type: "rule_reference", name: "class_statement" },
      { type: "rule_reference", name: "module_statement" },
      { type: "rule_reference", name: "if_statement" },
      { type: "rule_reference", name: "unless_statement" },
      { type: "rule_reference", name: "while_statement" },
      { type: "rule_reference", name: "until_statement" },
      { type: "rule_reference", name: "for_statement" },
      { type: "rule_reference", name: "case_statement" },
      { type: "rule_reference", name: "begin_statement" },
      { type: "rule_reference", name: "return_statement" },
      { type: "rule_reference", name: "break_statement" },
      { type: "rule_reference", name: "next_statement" },
      { type: "rule_reference", name: "redo_statement" },
      { type: "rule_reference", name: "retry_statement" },
      { type: "rule_reference", name: "yield_statement" },
      { type: "rule_reference", name: "alias_statement" },
      { type: "rule_reference", name: "undef_statement" },
      { type: "rule_reference", name: "multi_assignment" },
      { type: "rule_reference", name: "modifier_statement" },
      { type: "rule_reference", name: "rightward_assignment" },
      { type: "rule_reference", name: "index_assignment" },
      { type: "rule_reference", name: "assignment" },
      { type: "rule_reference", name: "defined_expression" },
      { type: "rule_reference", name: "method_with_block" },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "method_call" },
        { type: "negative_lookahead", element: { type: "token_reference", name: "PLUS" } },
        { type: "negative_lookahead", element: { type: "token_reference", name: "MINUS" } },
        { type: "negative_lookahead", element: { type: "token_reference", name: "STAR" } },
        { type: "negative_lookahead", element: { type: "token_reference", name: "SLASH" } },
        { type: "negative_lookahead", element: { type: "literal", value: "<<" } },
        { type: "negative_lookahead", element: { type: "literal", value: "==" } },
        { type: "negative_lookahead", element: { type: "literal", value: "!=" } },
        { type: "negative_lookahead", element: { type: "literal", value: "<=" } },
        { type: "negative_lookahead", element: { type: "literal", value: ">=" } },
        { type: "negative_lookahead", element: { type: "literal", value: "<" } },
        { type: "negative_lookahead", element: { type: "literal", value: ">" } },
        { type: "negative_lookahead", element: { type: "literal", value: "&&" } },
        { type: "negative_lookahead", element: { type: "literal", value: "||" } },
        { type: "negative_lookahead", element: { type: "literal", value: "and" } },
        { type: "negative_lookahead", element: { type: "literal", value: "or" } },
        { type: "negative_lookahead", element: { type: "literal", value: "..." } },
        { type: "negative_lookahead", element: { type: "literal", value: ".." } },
        { type: "negative_lookahead", element: { type: "literal", value: "?" } },
      ] },
      { type: "rule_reference", name: "method_call_no_paren" },
      { type: "rule_reference", name: "expression_stmt" },
    ] },
    lineNumber: 28,
  },
  {
    name: "multi_assignment",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "mlhs_target" },
      { type: "token_reference", name: "COMMA" },
      { type: "rule_reference", name: "mlhs_target" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "mlhs_target" },
        ] } },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 83,
  },
  {
    name: "mlhs_target",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "literal", value: "*" } },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 84,
  },
  {
    name: "modifier_statement",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "assignment" },
          { type: "rule_reference", name: "method_call_no_paren" },
          { type: "sequence", elements: [
            { type: "rule_reference", name: "method_call" },
            { type: "negative_lookahead", element: { type: "token_reference", name: "PLUS" } },
            { type: "negative_lookahead", element: { type: "token_reference", name: "MINUS" } },
            { type: "negative_lookahead", element: { type: "token_reference", name: "STAR" } },
            { type: "negative_lookahead", element: { type: "token_reference", name: "SLASH" } },
            { type: "negative_lookahead", element: { type: "literal", value: "<<" } },
            { type: "negative_lookahead", element: { type: "literal", value: "==" } },
            { type: "negative_lookahead", element: { type: "literal", value: "!=" } },
            { type: "negative_lookahead", element: { type: "literal", value: "<=" } },
            { type: "negative_lookahead", element: { type: "literal", value: ">=" } },
            { type: "negative_lookahead", element: { type: "literal", value: "<" } },
            { type: "negative_lookahead", element: { type: "literal", value: ">" } },
            { type: "negative_lookahead", element: { type: "literal", value: "&&" } },
            { type: "negative_lookahead", element: { type: "literal", value: "||" } },
            { type: "negative_lookahead", element: { type: "literal", value: "and" } },
            { type: "negative_lookahead", element: { type: "literal", value: "or" } },
            { type: "negative_lookahead", element: { type: "literal", value: "..." } },
            { type: "negative_lookahead", element: { type: "literal", value: ".." } },
            { type: "negative_lookahead", element: { type: "literal", value: "?" } },
          ] },
          { type: "rule_reference", name: "expression_stmt" },
        ] } },
      { type: "group", element: { type: "alternation", choices: [
          { type: "literal", value: "if_modifier" },
          { type: "literal", value: "unless_modifier" },
          { type: "literal", value: "while_modifier" },
          { type: "literal", value: "until_modifier" },
        ] } },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 120,
  },
  {
    name: "def_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "def" },
      { type: "optional", element: { type: "rule_reference", name: "def_receiver" } },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "optional", element: { type: "rule_reference", name: "params" } },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "rescue" } },
          { type: "negative_lookahead", element: { type: "literal", value: "ensure" } },
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
      { type: "repetition", element: { type: "rule_reference", name: "rescue_clause" } },
      { type: "optional", element: { type: "rule_reference", name: "ensure_clause" } },
      { type: "literal", value: "end" },
    ] },
    lineNumber: 144,
  },
  {
    name: "def_receiver",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "singleton_receiver" },
      { type: "literal", value: "." },
    ] },
    lineNumber: 150,
  },
  {
    name: "endless_def_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "def" },
      { type: "optional", element: { type: "rule_reference", name: "def_receiver" } },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "optional", element: { type: "rule_reference", name: "params" } },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 159,
  },
  {
    name: "class_statement",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "class" },
        { type: "literal", value: "<<" },
        { type: "rule_reference", name: "singleton_receiver" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "negative_lookahead", element: { type: "literal", value: "end" } },
            { type: "rule_reference", name: "statement" },
          ] } },
        { type: "literal", value: "end" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "class" },
        { type: "token_reference", name: "NAME" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "literal", value: "<" },
            { type: "token_reference", name: "NAME" },
          ] } },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "negative_lookahead", element: { type: "literal", value: "end" } },
            { type: "rule_reference", name: "statement" },
          ] } },
        { type: "literal", value: "end" },
      ] },
    ] },
    lineNumber: 180,
  },
  {
    name: "singleton_receiver",
    body: { type: "alternation", choices: [
      { type: "literal", value: "self" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 182,
  },
  {
    name: "module_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "module" },
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
      { type: "literal", value: "end" },
    ] },
    lineNumber: 183,
  },
  {
    name: "method_with_block",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "KEYWORD" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "optional", element: { type: "sequence", elements: [
              { type: "rule_reference", name: "expression" },
              { type: "repetition", element: { type: "sequence", elements: [
                  { type: "token_reference", name: "COMMA" },
                  { type: "rule_reference", name: "expression" },
                ] } },
            ] } },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 185,
  },
  {
    name: "block",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "do_block" },
      { type: "rule_reference", name: "brace_block" },
    ] },
    lineNumber: 186,
  },
  {
    name: "do_block",
    body: { type: "sequence", elements: [
      { type: "literal", value: "do" },
      { type: "optional", element: { type: "rule_reference", name: "block_params" } },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
      { type: "literal", value: "end" },
    ] },
    lineNumber: 187,
  },
  {
    name: "brace_block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "rule_reference", name: "block_params" } },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 188,
  },
  {
    name: "block_params",
    body: { type: "sequence", elements: [
      { type: "literal", value: "|" },
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "token_reference", name: "NAME" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: ";" },
          { type: "token_reference", name: "NAME" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "token_reference", name: "NAME" },
            ] } },
        ] } },
      { type: "literal", value: "|" },
    ] },
    lineNumber: 198,
  },
  {
    name: "return_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "return" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
    ] },
    lineNumber: 200,
  },
  {
    name: "break_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "break" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
    ] },
    lineNumber: 201,
  },
  {
    name: "next_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "next" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
    ] },
    lineNumber: 202,
  },
  {
    name: "redo_statement",
    body: { type: "literal", value: "redo" },
    lineNumber: 206,
  },
  {
    name: "retry_statement",
    body: { type: "literal", value: "retry" },
    lineNumber: 210,
  },
  {
    name: "alias_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "alias" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 221,
  },
  {
    name: "undef_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "undef" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 233,
  },
  {
    name: "yield_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "yield" },
      { type: "optional", element: { type: "rule_reference", name: "yield_args" } },
    ] },
    lineNumber: 255,
  },
  {
    name: "yield_args",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "call_arg" },
            { type: "repetition", element: { type: "sequence", elements: [
                { type: "token_reference", name: "COMMA" },
                { type: "rule_reference", name: "call_arg" },
              ] } },
          ] } },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "call_arg" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "call_arg" },
          ] } },
      ] },
    ] },
    lineNumber: 256,
  },
  {
    name: "super_args",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "call_arg" },
            { type: "repetition", element: { type: "sequence", elements: [
                { type: "token_reference", name: "COMMA" },
                { type: "rule_reference", name: "call_arg" },
              ] } },
          ] } },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "call_arg" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "call_arg" },
          ] } },
      ] },
    ] },
    lineNumber: 283,
  },
  {
    name: "params",
    body: { type: "alternation", choices: [
      { type: "literal", value: "..." },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "param" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "param" },
          ] } },
      ] },
    ] },
    lineNumber: 312,
  },
  {
    name: "param",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "alternation", choices: [
          { type: "literal", value: "*" },
          { type: "literal", value: "**" },
        ] } },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "token_reference", name: "COLON" },
            { type: "optional", element: { type: "rule_reference", name: "expression" } },
          ] },
          { type: "sequence", elements: [
            { type: "token_reference", name: "EQUALS" },
            { type: "rule_reference", name: "expression" },
          ] },
        ] } },
    ] },
    lineNumber: 357,
  },
  {
    name: "if_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "if" },
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "else" } },
          { type: "negative_lookahead", element: { type: "literal", value: "elsif" } },
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
      { type: "repetition", element: { type: "rule_reference", name: "elsif_clause" } },
      { type: "optional", element: { type: "rule_reference", name: "else_clause" } },
      { type: "literal", value: "end" },
    ] },
    lineNumber: 358,
  },
  {
    name: "elsif_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "elsif" },
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "else" } },
          { type: "negative_lookahead", element: { type: "literal", value: "elsif" } },
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
    ] },
    lineNumber: 359,
  },
  {
    name: "else_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "else" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
    ] },
    lineNumber: 360,
  },
  {
    name: "unless_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "unless" },
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "else" } },
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
      { type: "optional", element: { type: "rule_reference", name: "else_clause" } },
      { type: "literal", value: "end" },
    ] },
    lineNumber: 361,
  },
  {
    name: "while_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "while" },
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
      { type: "literal", value: "end" },
    ] },
    lineNumber: 362,
  },
  {
    name: "until_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "until" },
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
      { type: "literal", value: "end" },
    ] },
    lineNumber: 363,
  },
  {
    name: "for_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "for" },
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "expression" },
      { type: "optional", element: { type: "literal", value: "do" } },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
      { type: "literal", value: "end" },
    ] },
    lineNumber: 371,
  },
  {
    name: "case_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "case" },
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "when_clause" },
          { type: "rule_reference", name: "in_clause" },
        ] } },
      { type: "optional", element: { type: "rule_reference", name: "else_clause" } },
      { type: "literal", value: "end" },
    ] },
    lineNumber: 394,
  },
  {
    name: "when_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "when" },
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "expression" },
        ] } },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "when" } },
          { type: "negative_lookahead", element: { type: "literal", value: "in" } },
          { type: "negative_lookahead", element: { type: "literal", value: "else" } },
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
    ] },
    lineNumber: 395,
  },
  {
    name: "in_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "pattern" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "when" } },
          { type: "negative_lookahead", element: { type: "literal", value: "in" } },
          { type: "negative_lookahead", element: { type: "literal", value: "else" } },
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
    ] },
    lineNumber: 417,
  },
  {
    name: "pattern",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "array_pattern" },
      { type: "rule_reference", name: "hash_pattern" },
      { type: "rule_reference", name: "class_pattern" },
      { type: "rule_reference", name: "pin_pattern" },
      { type: "rule_reference", name: "literal_pattern" },
      { type: "rule_reference", name: "binding_pattern" },
    ] },
    lineNumber: 418,
  },
  {
    name: "literal_pattern",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "STRING" },
      { type: "rule_reference", name: "symbol_literal" },
      { type: "token_reference", name: "KEYWORD" },
    ] },
    lineNumber: 419,
  },
  {
    name: "binding_pattern",
    body: { type: "token_reference", name: "NAME" },
    lineNumber: 420,
  },
  {
    name: "array_pattern",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "rule_reference", name: "splat_pattern" },
              { type: "rule_reference", name: "pattern" },
            ] } },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "group", element: { type: "alternation", choices: [
                  { type: "rule_reference", name: "splat_pattern" },
                  { type: "rule_reference", name: "pattern" },
                ] } },
            ] } },
        ] } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 421,
  },
  {
    name: "hash_pattern",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "hash_pattern_pair" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "hash_pattern_pair" },
            ] } },
        ] } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 422,
  },
  {
    name: "hash_pattern_pair",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "optional", element: { type: "rule_reference", name: "pattern" } },
    ] },
    lineNumber: 423,
  },
  {
    name: "splat_pattern",
    body: { type: "sequence", elements: [
      { type: "literal", value: "*" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
    ] },
    lineNumber: 430,
  },
  {
    name: "pin_pattern",
    body: { type: "sequence", elements: [
      { type: "literal", value: "^" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 435,
  },
  {
    name: "class_pattern",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "pattern" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "pattern" },
            ] } },
        ] } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 441,
  },
  {
    name: "begin_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "begin" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "rescue" } },
          { type: "negative_lookahead", element: { type: "literal", value: "ensure" } },
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
      { type: "repetition", element: { type: "rule_reference", name: "rescue_clause" } },
      { type: "optional", element: { type: "rule_reference", name: "ensure_clause" } },
      { type: "literal", value: "end" },
    ] },
    lineNumber: 462,
  },
  {
    name: "rescue_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "rescue" },
      { type: "optional", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "literal", value: "=>" },
            { type: "token_reference", name: "NAME" },
          ] },
          { type: "sequence", elements: [
            { type: "rule_reference", name: "exception_list" },
            { type: "literal", value: "=>" },
            { type: "token_reference", name: "NAME" },
          ] },
        ] } },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "rescue" } },
          { type: "negative_lookahead", element: { type: "literal", value: "ensure" } },
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
    ] },
    lineNumber: 476,
  },
  {
    name: "exception_list",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 477,
  },
  {
    name: "ensure_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "ensure" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
    ] },
    lineNumber: 478,
  },
  {
    name: "index_write_receiver_postfix",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "dot_call" },
      { type: "rule_reference", name: "scope_resolution" },
      { type: "rule_reference", name: "index_suffix" },
    ] },
    lineNumber: 531,
  },
  {
    name: "index_assignment",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "index_write_receiver_postfix" },
          { type: "positive_lookahead", element: { type: "rule_reference", name: "index_write_receiver_postfix" } },
        ] } },
      { type: "rule_reference", name: "index_suffix" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 532,
  },
  {
    name: "assignment",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "EQUALS" },
          { type: "literal", value: "+=" },
          { type: "literal", value: "-=" },
          { type: "literal", value: "*=" },
          { type: "literal", value: "/=" },
          { type: "literal", value: "%=" },
          { type: "literal", value: "**=" },
          { type: "literal", value: "<<=" },
          { type: "literal", value: ">>=" },
          { type: "literal", value: "&=" },
          { type: "literal", value: "|=" },
          { type: "literal", value: "^=" },
          { type: "literal", value: "||=" },
          { type: "literal", value: "&&=" },
        ] } },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 533,
  },
  {
    name: "rightward_assignment",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "literal", value: "=>" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 552,
  },
  {
    name: "method_call",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "NAME" },
          { type: "sequence", elements: [
            { type: "negative_lookahead", element: { type: "literal", value: "super" } },
            { type: "token_reference", name: "KEYWORD" },
          ] },
        ] } },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "call_arg" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "call_arg" },
            ] } },
        ] } },
      { type: "token_reference", name: "RPAREN" },
      { type: "repetition", element: { type: "rule_reference", name: "dot_call" } },
    ] },
    lineNumber: 569,
  },
  {
    name: "dot_call",
    body: { type: "sequence", elements: [
      { type: "literal", value: "." },
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "KEYWORD" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "optional", element: { type: "sequence", elements: [
              { type: "rule_reference", name: "call_arg" },
              { type: "repetition", element: { type: "sequence", elements: [
                  { type: "token_reference", name: "COMMA" },
                  { type: "rule_reference", name: "call_arg" },
                ] } },
            ] } },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "optional", element: { type: "rule_reference", name: "block" } },
    ] },
    lineNumber: 570,
  },
  {
    name: "scope_resolution",
    body: { type: "sequence", elements: [
      { type: "literal", value: "::" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "KEYWORD" },
        ] } },
    ] },
    lineNumber: 578,
  },
  {
    name: "call_arg",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "COLON" },
        { type: "rule_reference", name: "expression" },
      ] },
      { type: "sequence", elements: [
        { type: "optional", element: { type: "alternation", choices: [
            { type: "literal", value: "*" },
            { type: "literal", value: "**" },
            { type: "literal", value: "&" },
          ] } },
        { type: "rule_reference", name: "expression" },
      ] },
    ] },
    lineNumber: 633,
  },
  {
    name: "method_call_no_paren",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "NAME" },
          { type: "sequence", elements: [
            { type: "negative_lookahead", element: { type: "literal", value: "super" } },
            { type: "token_reference", name: "KEYWORD" },
          ] },
        ] } },
      { type: "negative_lookahead", element: { type: "literal", value: "<" } },
      { type: "negative_lookahead", element: { type: "literal", value: ">" } },
      { type: "negative_lookahead", element: { type: "literal", value: "<=" } },
      { type: "negative_lookahead", element: { type: "literal", value: ">=" } },
      { type: "negative_lookahead", element: { type: "literal", value: "!=" } },
      { type: "negative_lookahead", element: { type: "literal", value: "&&" } },
      { type: "negative_lookahead", element: { type: "literal", value: "||" } },
      { type: "negative_lookahead", element: { type: "literal", value: "<<" } },
      { type: "negative_lookahead", element: { type: "literal", value: "[ line start" } },
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 681,
  },
  {
    name: "expression_stmt",
    body: { type: "rule_reference", name: "expression" },
    lineNumber: 684,
  },
  {
    name: "expression",
    body: { type: "rule_reference", name: "ternary" },
    lineNumber: 791,
  },
  {
    name: "ternary",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "range" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "?" },
          { type: "rule_reference", name: "expression" },
          { type: "literal", value: ":" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 792,
  },
  {
    name: "range",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "group", element: { type: "alternation", choices: [
            { type: "literal", value: "..." },
            { type: "literal", value: ".." },
          ] } },
        { type: "rule_reference", name: "logical_or" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "logical_or" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "group", element: { type: "alternation", choices: [
                { type: "literal", value: "..." },
                { type: "literal", value: ".." },
              ] } },
            { type: "optional", element: { type: "rule_reference", name: "logical_or" } },
          ] } },
      ] },
    ] },
    lineNumber: 793,
  },
  {
    name: "logical_or",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "logical_and" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "literal", value: "||" },
              { type: "literal", value: "or" },
            ] } },
          { type: "rule_reference", name: "logical_and" },
        ] } },
    ] },
    lineNumber: 794,
  },
  {
    name: "logical_and",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "logical_not" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "literal", value: "&&" },
              { type: "literal", value: "and" },
            ] } },
          { type: "rule_reference", name: "logical_not" },
        ] } },
    ] },
    lineNumber: 795,
  },
  {
    name: "logical_not",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "group", element: { type: "alternation", choices: [
            { type: "literal", value: "!" },
            { type: "literal", value: "not" },
          ] } } },
      { type: "rule_reference", name: "comparison" },
    ] },
    lineNumber: 802,
  },
  {
    name: "comparison",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "shift" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "literal", value: "==" },
              { type: "literal", value: "!=" },
              { type: "literal", value: "<=" },
              { type: "literal", value: ">=" },
              { type: "literal", value: "<" },
              { type: "literal", value: ">" },
            ] } },
          { type: "rule_reference", name: "shift" },
        ] } },
    ] },
    lineNumber: 818,
  },
  {
    name: "shift",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "sum" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "<<" },
          { type: "rule_reference", name: "sum" },
        ] } },
    ] },
    lineNumber: 819,
  },
  {
    name: "sum",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "term" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "PLUS" },
              { type: "token_reference", name: "MINUS" },
            ] } },
          { type: "rule_reference", name: "term" },
        ] } },
    ] },
    lineNumber: 820,
  },
  {
    name: "term",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "factor" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "STAR" },
              { type: "token_reference", name: "SLASH" },
            ] } },
          { type: "rule_reference", name: "factor" },
        ] } },
    ] },
    lineNumber: 821,
  },
  {
    name: "super_expr",
    body: { type: "sequence", elements: [
      { type: "literal", value: "super" },
      { type: "optional", element: { type: "rule_reference", name: "super_args" } },
    ] },
    lineNumber: 890,
  },
  {
    name: "index_suffix",
    body: { type: "sequence", elements: [
      { type: "negative_lookahead", element: { type: "literal", value: "[ line start" } },
      { type: "token_reference", name: "LBRACKET" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 918,
  },
  {
    name: "factor",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "defined_expression" },
          { type: "rule_reference", name: "lambda_literal" },
          { type: "rule_reference", name: "super_expr" },
          { type: "rule_reference", name: "method_call" },
          { type: "token_reference", name: "NUMBER" },
          { type: "token_reference", name: "STRING" },
          { type: "token_reference", name: "NAME" },
          { type: "group", element: { type: "sequence", elements: [
              { type: "negative_lookahead", element: { type: "literal", value: "end" } },
              { type: "negative_lookahead", element: { type: "literal", value: "rescue" } },
              { type: "negative_lookahead", element: { type: "literal", value: "ensure" } },
              { type: "negative_lookahead", element: { type: "literal", value: "else" } },
              { type: "negative_lookahead", element: { type: "literal", value: "elsif" } },
              { type: "negative_lookahead", element: { type: "literal", value: "when" } },
              { type: "negative_lookahead", element: { type: "literal", value: "then" } },
              { type: "negative_lookahead", element: { type: "literal", value: "in" } },
              { type: "negative_lookahead", element: { type: "literal", value: "do" } },
              { type: "token_reference", name: "KEYWORD" },
            ] } },
          { type: "rule_reference", name: "symbol_literal" },
          { type: "rule_reference", name: "array_literal" },
          { type: "rule_reference", name: "hash_literal" },
          { type: "sequence", elements: [
            { type: "token_reference", name: "LPAREN" },
            { type: "rule_reference", name: "expression" },
            { type: "token_reference", name: "RPAREN" },
          ] },
          { type: "rule_reference", name: "unary_minus" },
        ] } },
      { type: "repetition", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "dot_call" },
          { type: "rule_reference", name: "scope_resolution" },
          { type: "rule_reference", name: "index_suffix" },
        ] } },
    ] },
    lineNumber: 919,
  },
  {
    name: "lambda_literal",
    body: { type: "sequence", elements: [
      { type: "literal", value: "->" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "optional", element: { type: "rule_reference", name: "params" } },
          { type: "token_reference", name: "RPAREN" },
        ] } },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 938,
  },
  {
    name: "unary_minus",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "MINUS" },
      { type: "rule_reference", name: "factor" },
    ] },
    lineNumber: 939,
  },
  {
    name: "defined_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "defined?" },
      { type: "rule_reference", name: "factor" },
    ] },
    lineNumber: 950,
  },
  {
    name: "symbol_literal",
    body: { type: "sequence", elements: [
      { type: "literal", value: ":" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "KEYWORD" },
          { type: "token_reference", name: "STRING" },
        ] } },
    ] },
    lineNumber: 957,
  },
  {
    name: "array_literal",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "expression" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "expression" },
            ] } },
        ] } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 958,
  },
  {
    name: "hash_literal",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "hash_entry" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "hash_entry" },
            ] } },
        ] } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 959,
  },
  {
    name: "hash_entry",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "COLON" },
        { type: "rule_reference", name: "expression" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "COLON" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "expression" },
        { type: "literal", value: "=>" },
        { type: "rule_reference", name: "expression" },
      ] },
    ] },
    lineNumber: 960,
  },
],
};
