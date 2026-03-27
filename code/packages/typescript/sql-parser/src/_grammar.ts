// AUTO-GENERATED FILE — DO NOT EDIT
// Source: sql.grammar
// Regenerate with: grammar-tools compile-grammar sql.grammar
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
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "statement" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: ";" },
          { type: "rule_reference", name: "statement" },
        ] } },
      { type: "optional", element: { type: "literal", value: ";" } },
    ] },
    lineNumber: 10,
  },
  {
    name: "statement",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "select_stmt" },
      { type: "rule_reference", name: "insert_stmt" },
      { type: "rule_reference", name: "update_stmt" },
      { type: "rule_reference", name: "delete_stmt" },
      { type: "rule_reference", name: "create_table_stmt" },
      { type: "rule_reference", name: "drop_table_stmt" },
    ] },
    lineNumber: 12,
  },
  {
    name: "select_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "SELECT" },
      { type: "optional", element: { type: "alternation", choices: [
          { type: "literal", value: "DISTINCT" },
          { type: "literal", value: "ALL" },
        ] } },
      { type: "rule_reference", name: "select_list" },
      { type: "literal", value: "FROM" },
      { type: "rule_reference", name: "table_ref" },
      { type: "repetition", element: { type: "rule_reference", name: "join_clause" } },
      { type: "optional", element: { type: "rule_reference", name: "where_clause" } },
      { type: "optional", element: { type: "rule_reference", name: "group_clause" } },
      { type: "optional", element: { type: "rule_reference", name: "having_clause" } },
      { type: "optional", element: { type: "rule_reference", name: "order_clause" } },
      { type: "optional", element: { type: "rule_reference", name: "limit_clause" } },
    ] },
    lineNumber: 17,
  },
  {
    name: "select_list",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "STAR" },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "select_item" },
        { type: "repetition", element: { type: "sequence", elements: [
            { type: "literal", value: "," },
            { type: "rule_reference", name: "select_item" },
          ] } },
      ] },
    ] },
    lineNumber: 22,
  },
  {
    name: "select_item",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expr" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "AS" },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 23,
  },
  {
    name: "table_ref",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "table_name" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "AS" },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 25,
  },
  {
    name: "table_name",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "." },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 26,
  },
  {
    name: "join_clause",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "join_type" },
      { type: "literal", value: "JOIN" },
      { type: "rule_reference", name: "table_ref" },
      { type: "literal", value: "ON" },
      { type: "rule_reference", name: "expr" },
    ] },
    lineNumber: 28,
  },
  {
    name: "join_type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "CROSS" },
      { type: "literal", value: "INNER" },
      { type: "group", element: { type: "sequence", elements: [
          { type: "literal", value: "LEFT" },
          { type: "optional", element: { type: "literal", value: "OUTER" } },
        ] } },
      { type: "group", element: { type: "sequence", elements: [
          { type: "literal", value: "RIGHT" },
          { type: "optional", element: { type: "literal", value: "OUTER" } },
        ] } },
      { type: "group", element: { type: "sequence", elements: [
          { type: "literal", value: "FULL" },
          { type: "optional", element: { type: "literal", value: "OUTER" } },
        ] } },
    ] },
    lineNumber: 29,
  },
  {
    name: "where_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "WHERE" },
      { type: "rule_reference", name: "expr" },
    ] },
    lineNumber: 32,
  },
  {
    name: "group_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "GROUP" },
      { type: "literal", value: "BY" },
      { type: "rule_reference", name: "column_ref" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "," },
          { type: "rule_reference", name: "column_ref" },
        ] } },
    ] },
    lineNumber: 33,
  },
  {
    name: "having_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "HAVING" },
      { type: "rule_reference", name: "expr" },
    ] },
    lineNumber: 34,
  },
  {
    name: "order_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "ORDER" },
      { type: "literal", value: "BY" },
      { type: "rule_reference", name: "order_item" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "," },
          { type: "rule_reference", name: "order_item" },
        ] } },
    ] },
    lineNumber: 35,
  },
  {
    name: "order_item",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expr" },
      { type: "optional", element: { type: "alternation", choices: [
          { type: "literal", value: "ASC" },
          { type: "literal", value: "DESC" },
        ] } },
    ] },
    lineNumber: 36,
  },
  {
    name: "limit_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "LIMIT" },
      { type: "token_reference", name: "NUMBER" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "OFFSET" },
          { type: "token_reference", name: "NUMBER" },
        ] } },
    ] },
    lineNumber: 37,
  },
  {
    name: "insert_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "INSERT" },
      { type: "literal", value: "INTO" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "(" },
          { type: "token_reference", name: "NAME" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "literal", value: "," },
              { type: "token_reference", name: "NAME" },
            ] } },
          { type: "literal", value: ")" },
        ] } },
      { type: "literal", value: "VALUES" },
      { type: "rule_reference", name: "row_value" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "," },
          { type: "rule_reference", name: "row_value" },
        ] } },
    ] },
    lineNumber: 41,
  },
  {
    name: "row_value",
    body: { type: "sequence", elements: [
      { type: "literal", value: "(" },
      { type: "rule_reference", name: "expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "," },
          { type: "rule_reference", name: "expr" },
        ] } },
      { type: "literal", value: ")" },
    ] },
    lineNumber: 44,
  },
  {
    name: "update_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "UPDATE" },
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "SET" },
      { type: "rule_reference", name: "assignment" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "," },
          { type: "rule_reference", name: "assignment" },
        ] } },
      { type: "optional", element: { type: "rule_reference", name: "where_clause" } },
    ] },
    lineNumber: 46,
  },
  {
    name: "assignment",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "=" },
      { type: "rule_reference", name: "expr" },
    ] },
    lineNumber: 48,
  },
  {
    name: "delete_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "DELETE" },
      { type: "literal", value: "FROM" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "rule_reference", name: "where_clause" } },
    ] },
    lineNumber: 50,
  },
  {
    name: "create_table_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "CREATE" },
      { type: "literal", value: "TABLE" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "IF" },
          { type: "literal", value: "NOT" },
          { type: "literal", value: "EXISTS" },
        ] } },
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "(" },
      { type: "rule_reference", name: "col_def" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "," },
          { type: "rule_reference", name: "col_def" },
        ] } },
      { type: "literal", value: ")" },
    ] },
    lineNumber: 54,
  },
  {
    name: "col_def",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "rule_reference", name: "col_constraint" } },
    ] },
    lineNumber: 56,
  },
  {
    name: "col_constraint",
    body: { type: "alternation", choices: [
      { type: "group", element: { type: "sequence", elements: [
          { type: "literal", value: "NOT" },
          { type: "literal", value: "NULL" },
        ] } },
      { type: "literal", value: "NULL" },
      { type: "group", element: { type: "sequence", elements: [
          { type: "literal", value: "PRIMARY" },
          { type: "literal", value: "KEY" },
        ] } },
      { type: "literal", value: "UNIQUE" },
      { type: "group", element: { type: "sequence", elements: [
          { type: "literal", value: "DEFAULT" },
          { type: "rule_reference", name: "primary" },
        ] } },
    ] },
    lineNumber: 57,
  },
  {
    name: "drop_table_stmt",
    body: { type: "sequence", elements: [
      { type: "literal", value: "DROP" },
      { type: "literal", value: "TABLE" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "IF" },
          { type: "literal", value: "EXISTS" },
        ] } },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 60,
  },
  {
    name: "expr",
    body: { type: "rule_reference", name: "or_expr" },
    lineNumber: 64,
  },
  {
    name: "or_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "and_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "OR" },
          { type: "rule_reference", name: "and_expr" },
        ] } },
    ] },
    lineNumber: 65,
  },
  {
    name: "and_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "not_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "AND" },
          { type: "rule_reference", name: "not_expr" },
        ] } },
    ] },
    lineNumber: 66,
  },
  {
    name: "not_expr",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "NOT" },
        { type: "rule_reference", name: "not_expr" },
      ] },
      { type: "rule_reference", name: "comparison" },
    ] },
    lineNumber: 67,
  },
  {
    name: "comparison",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "additive" },
      { type: "optional", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "rule_reference", name: "cmp_op" },
            { type: "rule_reference", name: "additive" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "BETWEEN" },
            { type: "rule_reference", name: "additive" },
            { type: "literal", value: "AND" },
            { type: "rule_reference", name: "additive" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "NOT" },
            { type: "literal", value: "BETWEEN" },
            { type: "rule_reference", name: "additive" },
            { type: "literal", value: "AND" },
            { type: "rule_reference", name: "additive" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "IN" },
            { type: "literal", value: "(" },
            { type: "rule_reference", name: "value_list" },
            { type: "literal", value: ")" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "NOT" },
            { type: "literal", value: "IN" },
            { type: "literal", value: "(" },
            { type: "rule_reference", name: "value_list" },
            { type: "literal", value: ")" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "LIKE" },
            { type: "rule_reference", name: "additive" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "NOT" },
            { type: "literal", value: "LIKE" },
            { type: "rule_reference", name: "additive" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "IS" },
            { type: "literal", value: "NULL" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "IS" },
            { type: "literal", value: "NOT" },
            { type: "literal", value: "NULL" },
          ] },
        ] } },
    ] },
    lineNumber: 68,
  },
  {
    name: "cmp_op",
    body: { type: "alternation", choices: [
      { type: "literal", value: "=" },
      { type: "token_reference", name: "NOT_EQUALS" },
      { type: "literal", value: "<" },
      { type: "literal", value: ">" },
      { type: "literal", value: "<=" },
      { type: "literal", value: ">=" },
    ] },
    lineNumber: 78,
  },
  {
    name: "additive",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "multiplicative" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "literal", value: "+" },
              { type: "literal", value: "-" },
            ] } },
          { type: "rule_reference", name: "multiplicative" },
        ] } },
    ] },
    lineNumber: 79,
  },
  {
    name: "multiplicative",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "unary" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "STAR" },
              { type: "literal", value: "/" },
              { type: "literal", value: "%" },
            ] } },
          { type: "rule_reference", name: "unary" },
        ] } },
    ] },
    lineNumber: 80,
  },
  {
    name: "unary",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "-" },
        { type: "rule_reference", name: "unary" },
      ] },
      { type: "rule_reference", name: "primary" },
    ] },
    lineNumber: 81,
  },
  {
    name: "primary",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "STRING" },
      { type: "literal", value: "NULL" },
      { type: "literal", value: "TRUE" },
      { type: "literal", value: "FALSE" },
      { type: "rule_reference", name: "function_call" },
      { type: "rule_reference", name: "column_ref" },
      { type: "sequence", elements: [
        { type: "literal", value: "(" },
        { type: "rule_reference", name: "expr" },
        { type: "literal", value: ")" },
      ] },
    ] },
    lineNumber: 82,
  },
  {
    name: "column_ref",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "." },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 85,
  },
  {
    name: "function_call",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "(" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "STAR" },
          { type: "optional", element: { type: "rule_reference", name: "value_list" } },
        ] } },
      { type: "literal", value: ")" },
    ] },
    lineNumber: 86,
  },
  {
    name: "value_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "," },
          { type: "rule_reference", name: "expr" },
        ] } },
    ] },
    lineNumber: 87,
  },
],
};
