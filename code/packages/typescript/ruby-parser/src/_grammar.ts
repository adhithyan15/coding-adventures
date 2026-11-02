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
      { type: "sequence", elements: [
        { type: "rule_reference", name: "return_statement" },
        { type: "negative_lookahead", element: { type: "literal", value: "if_modifier" } },
        { type: "negative_lookahead", element: { type: "literal", value: "unless_modifier" } },
        { type: "negative_lookahead", element: { type: "literal", value: "while_modifier" } },
        { type: "negative_lookahead", element: { type: "literal", value: "until_modifier" } },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "break_statement" },
        { type: "negative_lookahead", element: { type: "literal", value: "if_modifier" } },
        { type: "negative_lookahead", element: { type: "literal", value: "unless_modifier" } },
        { type: "negative_lookahead", element: { type: "literal", value: "while_modifier" } },
        { type: "negative_lookahead", element: { type: "literal", value: "until_modifier" } },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "next_statement" },
        { type: "negative_lookahead", element: { type: "literal", value: "if_modifier" } },
        { type: "negative_lookahead", element: { type: "literal", value: "unless_modifier" } },
        { type: "negative_lookahead", element: { type: "literal", value: "while_modifier" } },
        { type: "negative_lookahead", element: { type: "literal", value: "until_modifier" } },
      ] },
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
        { type: "negative_lookahead", element: { type: "literal", value: "+" } },
        { type: "negative_lookahead", element: { type: "literal", value: "-" } },
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
    lineNumber: 87,
  },
  {
    name: "mlhs_target",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "literal", value: "*" } },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 88,
  },
  {
    name: "modifier_statement",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "return_statement" },
          { type: "rule_reference", name: "break_statement" },
          { type: "rule_reference", name: "next_statement" },
          { type: "rule_reference", name: "assignment" },
          { type: "rule_reference", name: "method_call_no_paren" },
          { type: "sequence", elements: [
            { type: "rule_reference", name: "method_call" },
            { type: "negative_lookahead", element: { type: "literal", value: "+" } },
            { type: "negative_lookahead", element: { type: "literal", value: "-" } },
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
    lineNumber: 124,
  },
  {
    name: "def_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "def" },
      { type: "optional", element: { type: "rule_reference", name: "def_receiver" } },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "( line start" } },
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
    lineNumber: 148,
  },
  {
    name: "def_receiver",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "singleton_receiver" },
      { type: "literal", value: "." },
    ] },
    lineNumber: 154,
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
    lineNumber: 163,
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
    lineNumber: 184,
  },
  {
    name: "singleton_receiver",
    body: { type: "alternation", choices: [
      { type: "literal", value: "self" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 186,
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
    lineNumber: 187,
  },
  {
    name: "method_with_block",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "KEYWORD" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "( line start" } },
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
    lineNumber: 189,
  },
  {
    name: "block",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "do_block" },
      { type: "rule_reference", name: "brace_block" },
    ] },
    lineNumber: 190,
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
    lineNumber: 191,
  },
  {
    name: "brace_block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "rule_reference", name: "block_params" } },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 192,
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
    lineNumber: 202,
  },
  {
    name: "return_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "return" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "literal", value: ";" },
          { type: "optional", element: { type: "rule_reference", name: "expression" } },
        ] } },
    ] },
    lineNumber: 209,
  },
  {
    name: "break_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "break" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "literal", value: ";" },
          { type: "optional", element: { type: "rule_reference", name: "expression" } },
        ] } },
    ] },
    lineNumber: 210,
  },
  {
    name: "next_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "next" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "literal", value: ";" },
          { type: "optional", element: { type: "rule_reference", name: "expression" } },
        ] } },
    ] },
    lineNumber: 211,
  },
  {
    name: "redo_statement",
    body: { type: "literal", value: "redo" },
    lineNumber: 215,
  },
  {
    name: "retry_statement",
    body: { type: "literal", value: "retry" },
    lineNumber: 219,
  },
  {
    name: "alias_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "alias" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 230,
  },
  {
    name: "undef_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "undef" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 242,
  },
  {
    name: "yield_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "yield" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "literal", value: ";" },
          { type: "optional", element: { type: "rule_reference", name: "yield_args" } },
        ] } },
    ] },
    lineNumber: 264,
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
    lineNumber: 265,
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
    lineNumber: 292,
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
    lineNumber: 321,
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
    lineNumber: 366,
  },
  {
    name: "if_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "if" },
      { type: "rule_reference", name: "expression" },
      { type: "optional", element: { type: "literal", value: "then" } },
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
    lineNumber: 370,
  },
  {
    name: "elsif_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "elsif" },
      { type: "rule_reference", name: "expression" },
      { type: "optional", element: { type: "literal", value: "then" } },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "else" } },
          { type: "negative_lookahead", element: { type: "literal", value: "elsif" } },
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
    ] },
    lineNumber: 371,
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
    lineNumber: 372,
  },
  {
    name: "unless_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "unless" },
      { type: "rule_reference", name: "expression" },
      { type: "optional", element: { type: "literal", value: "then" } },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "else" } },
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
      { type: "optional", element: { type: "rule_reference", name: "else_clause" } },
      { type: "literal", value: "end" },
    ] },
    lineNumber: 373,
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
    lineNumber: 374,
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
    lineNumber: 375,
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
    lineNumber: 383,
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
    lineNumber: 406,
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
      { type: "optional", element: { type: "literal", value: "then" } },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "when" } },
          { type: "negative_lookahead", element: { type: "literal", value: "in" } },
          { type: "negative_lookahead", element: { type: "literal", value: "else" } },
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
    ] },
    lineNumber: 407,
  },
  {
    name: "in_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "pattern" },
      { type: "optional", element: { type: "literal", value: "then" } },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "when" } },
          { type: "negative_lookahead", element: { type: "literal", value: "in" } },
          { type: "negative_lookahead", element: { type: "literal", value: "else" } },
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
    ] },
    lineNumber: 429,
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
    lineNumber: 430,
  },
  {
    name: "literal_pattern",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "STRING" },
      { type: "rule_reference", name: "symbol_literal" },
      { type: "token_reference", name: "KEYWORD" },
    ] },
    lineNumber: 431,
  },
  {
    name: "binding_pattern",
    body: { type: "token_reference", name: "NAME" },
    lineNumber: 432,
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
    lineNumber: 433,
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
    lineNumber: 434,
  },
  {
    name: "hash_pattern_pair",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "optional", element: { type: "rule_reference", name: "pattern" } },
    ] },
    lineNumber: 435,
  },
  {
    name: "splat_pattern",
    body: { type: "sequence", elements: [
      { type: "literal", value: "*" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
    ] },
    lineNumber: 442,
  },
  {
    name: "pin_pattern",
    body: { type: "sequence", elements: [
      { type: "literal", value: "^" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 447,
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
    lineNumber: 453,
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
    lineNumber: 474,
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
            { type: "group", element: { type: "alternation", choices: [
                { type: "sequence", elements: [
                  { type: "literal", value: "=>" },
                  { type: "token_reference", name: "NAME" },
                ] },
                { type: "literal", value: ";" },
              ] } },
          ] },
          { type: "literal", value: ";" },
        ] } },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "rescue" } },
          { type: "negative_lookahead", element: { type: "literal", value: "ensure" } },
          { type: "negative_lookahead", element: { type: "literal", value: "end" } },
          { type: "rule_reference", name: "statement" },
        ] } },
    ] },
    lineNumber: 494,
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
    lineNumber: 495,
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
    lineNumber: 496,
  },
  {
    name: "index_write_receiver_postfix",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "dot_call" },
      { type: "rule_reference", name: "scope_resolution" },
      { type: "rule_reference", name: "index_suffix" },
    ] },
    lineNumber: 549,
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
    lineNumber: 550,
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
    lineNumber: 551,
  },
  {
    name: "rightward_assignment",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "literal", value: "=>" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 570,
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
      { type: "negative_lookahead", element: { type: "literal", value: "( line start" } },
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
    lineNumber: 591,
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
          { type: "negative_lookahead", element: { type: "literal", value: "( line start" } },
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
    lineNumber: 592,
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
    lineNumber: 600,
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
    lineNumber: 655,
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
      { type: "negative_lookahead", element: { type: "literal", value: "( line start" } },
      { type: "negative_lookahead", element: { type: "literal", value: "- line start" } },
      { type: "negative_lookahead", element: { type: "literal", value: "+ line start" } },
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 709,
  },
  {
    name: "expression_stmt",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "optional", element: { type: "literal", value: ";" } },
    ] },
    lineNumber: 713,
  },
  {
    name: "expression",
    body: { type: "rule_reference", name: "ternary" },
    lineNumber: 820,
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
    lineNumber: 821,
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
    lineNumber: 822,
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
    lineNumber: 823,
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
    lineNumber: 824,
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
    lineNumber: 831,
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
    lineNumber: 847,
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
    lineNumber: 848,
  },
  {
    name: "sum",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "term" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "negative_lookahead", element: { type: "literal", value: "- line start" } },
          { type: "negative_lookahead", element: { type: "literal", value: "+ line start" } },
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "PLUS" },
              { type: "token_reference", name: "MINUS" },
            ] } },
          { type: "rule_reference", name: "term" },
        ] } },
    ] },
    lineNumber: 853,
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
    lineNumber: 854,
  },
  {
    name: "super_expr",
    body: { type: "sequence", elements: [
      { type: "literal", value: "super" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "literal", value: ";" },
          { type: "optional", element: { type: "rule_reference", name: "super_args" } },
        ] } },
    ] },
    lineNumber: 926,
  },
  {
    name: "index_suffix",
    body: { type: "sequence", elements: [
      { type: "negative_lookahead", element: { type: "literal", value: "[ line start" } },
      { type: "token_reference", name: "LBRACKET" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 954,
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
              { type: "negative_lookahead", element: { type: "literal", value: "if_modifier" } },
              { type: "negative_lookahead", element: { type: "literal", value: "unless_modifier" } },
              { type: "negative_lookahead", element: { type: "literal", value: "while_modifier" } },
              { type: "negative_lookahead", element: { type: "literal", value: "until_modifier" } },
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
          { type: "rule_reference", name: "unary_plus" },
        ] } },
      { type: "repetition", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "dot_call" },
          { type: "rule_reference", name: "scope_resolution" },
          { type: "rule_reference", name: "index_suffix" },
        ] } },
    ] },
    lineNumber: 958,
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
    lineNumber: 977,
  },
  {
    name: "unary_minus",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "MINUS" },
      { type: "rule_reference", name: "factor" },
    ] },
    lineNumber: 978,
  },
  {
    name: "unary_plus",
    body: { type: "sequence", elements: [
      { type: "literal", value: "+ line start" },
      { type: "rule_reference", name: "factor" },
    ] },
    lineNumber: 985,
  },
  {
    name: "defined_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "defined?" },
      { type: "rule_reference", name: "factor" },
    ] },
    lineNumber: 996,
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
    lineNumber: 1003,
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
    lineNumber: 1004,
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
    lineNumber: 1005,
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
    lineNumber: 1006,
  },
],
};
