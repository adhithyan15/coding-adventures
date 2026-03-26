// AUTO-GENERATED FILE - DO NOT EDIT
import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const SqlGrammar: ParserGrammar = {
  version: 1,
  rules: [
    {
      name: "program",
      lineNumber: 10,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "statement" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: ";" }, { type: "rule_reference", name: "statement" }] } }, { type: "optional", element: { type: "literal", value: ";" } }] }
    },
    {
      name: "statement",
      lineNumber: 12,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "select_stmt" }, { type: "rule_reference", name: "insert_stmt" }, { type: "rule_reference", name: "update_stmt" }, { type: "rule_reference", name: "delete_stmt" }, { type: "rule_reference", name: "create_table_stmt" }, { type: "rule_reference", name: "drop_table_stmt" }] }
    },
    {
      name: "select_stmt",
      lineNumber: 17,
      body: { type: "sequence", elements: [{ type: "literal", value: "SELECT" }, { type: "optional", element: { type: "alternation", choices: [{ type: "literal", value: "DISTINCT" }, { type: "literal", value: "ALL" }] } }, { type: "rule_reference", name: "select_list" }, { type: "literal", value: "FROM" }, { type: "rule_reference", name: "table_ref" }, { type: "repetition", element: { type: "rule_reference", name: "join_clause" } }, { type: "optional", element: { type: "rule_reference", name: "where_clause" } }, { type: "optional", element: { type: "rule_reference", name: "group_clause" } }, { type: "optional", element: { type: "rule_reference", name: "having_clause" } }, { type: "optional", element: { type: "rule_reference", name: "order_clause" } }, { type: "optional", element: { type: "rule_reference", name: "limit_clause" } }] }
    },
    {
      name: "select_list",
      lineNumber: 22,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "STAR" }, { type: "sequence", elements: [{ type: "rule_reference", name: "select_item" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: "," }, { type: "rule_reference", name: "select_item" }] } }] }] }
    },
    {
      name: "select_item",
      lineNumber: 23,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "expr" }, { type: "optional", element: { type: "sequence", elements: [{ type: "literal", value: "AS" }, { type: "token_reference", name: "NAME" }] } }] }
    },
    {
      name: "table_ref",
      lineNumber: 25,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "table_name" }, { type: "optional", element: { type: "sequence", elements: [{ type: "literal", value: "AS" }, { type: "token_reference", name: "NAME" }] } }] }
    },
    {
      name: "table_name",
      lineNumber: 26,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "optional", element: { type: "sequence", elements: [{ type: "literal", value: "." }, { type: "token_reference", name: "NAME" }] } }] }
    },
    {
      name: "join_clause",
      lineNumber: 28,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "join_type" }, { type: "literal", value: "JOIN" }, { type: "rule_reference", name: "table_ref" }, { type: "literal", value: "ON" }, { type: "rule_reference", name: "expr" }] }
    },
    {
      name: "join_type",
      lineNumber: 29,
      body: { type: "alternation", choices: [{ type: "literal", value: "CROSS" }, { type: "literal", value: "INNER" }, { type: "group", element: { type: "sequence", elements: [{ type: "literal", value: "LEFT" }, { type: "optional", element: { type: "literal", value: "OUTER" } }] } }, { type: "group", element: { type: "sequence", elements: [{ type: "literal", value: "RIGHT" }, { type: "optional", element: { type: "literal", value: "OUTER" } }] } }, { type: "group", element: { type: "sequence", elements: [{ type: "literal", value: "FULL" }, { type: "optional", element: { type: "literal", value: "OUTER" } }] } }] }
    },
    {
      name: "where_clause",
      lineNumber: 32,
      body: { type: "sequence", elements: [{ type: "literal", value: "WHERE" }, { type: "rule_reference", name: "expr" }] }
    },
    {
      name: "group_clause",
      lineNumber: 33,
      body: { type: "sequence", elements: [{ type: "literal", value: "GROUP" }, { type: "literal", value: "BY" }, { type: "rule_reference", name: "column_ref" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: "," }, { type: "rule_reference", name: "column_ref" }] } }] }
    },
    {
      name: "having_clause",
      lineNumber: 34,
      body: { type: "sequence", elements: [{ type: "literal", value: "HAVING" }, { type: "rule_reference", name: "expr" }] }
    },
    {
      name: "order_clause",
      lineNumber: 35,
      body: { type: "sequence", elements: [{ type: "literal", value: "ORDER" }, { type: "literal", value: "BY" }, { type: "rule_reference", name: "order_item" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: "," }, { type: "rule_reference", name: "order_item" }] } }] }
    },
    {
      name: "order_item",
      lineNumber: 36,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "expr" }, { type: "optional", element: { type: "alternation", choices: [{ type: "literal", value: "ASC" }, { type: "literal", value: "DESC" }] } }] }
    },
    {
      name: "limit_clause",
      lineNumber: 37,
      body: { type: "sequence", elements: [{ type: "literal", value: "LIMIT" }, { type: "token_reference", name: "NUMBER" }, { type: "optional", element: { type: "sequence", elements: [{ type: "literal", value: "OFFSET" }, { type: "token_reference", name: "NUMBER" }] } }] }
    },
    {
      name: "insert_stmt",
      lineNumber: 41,
      body: { type: "sequence", elements: [{ type: "literal", value: "INSERT" }, { type: "literal", value: "INTO" }, { type: "token_reference", name: "NAME" }, { type: "optional", element: { type: "sequence", elements: [{ type: "literal", value: "(" }, { type: "token_reference", name: "NAME" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: "," }, { type: "token_reference", name: "NAME" }] } }, { type: "literal", value: ")" }] } }, { type: "literal", value: "VALUES" }, { type: "rule_reference", name: "row_value" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: "," }, { type: "rule_reference", name: "row_value" }] } }] }
    },
    {
      name: "row_value",
      lineNumber: 44,
      body: { type: "sequence", elements: [{ type: "literal", value: "(" }, { type: "rule_reference", name: "expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: "," }, { type: "rule_reference", name: "expr" }] } }, { type: "literal", value: ")" }] }
    },
    {
      name: "update_stmt",
      lineNumber: 46,
      body: { type: "sequence", elements: [{ type: "literal", value: "UPDATE" }, { type: "token_reference", name: "NAME" }, { type: "literal", value: "SET" }, { type: "rule_reference", name: "assignment" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: "," }, { type: "rule_reference", name: "assignment" }] } }, { type: "optional", element: { type: "rule_reference", name: "where_clause" } }] }
    },
    {
      name: "assignment",
      lineNumber: 48,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "literal", value: "=" }, { type: "rule_reference", name: "expr" }] }
    },
    {
      name: "delete_stmt",
      lineNumber: 50,
      body: { type: "sequence", elements: [{ type: "literal", value: "DELETE" }, { type: "literal", value: "FROM" }, { type: "token_reference", name: "NAME" }, { type: "optional", element: { type: "rule_reference", name: "where_clause" } }] }
    },
    {
      name: "create_table_stmt",
      lineNumber: 54,
      body: { type: "sequence", elements: [{ type: "literal", value: "CREATE" }, { type: "literal", value: "TABLE" }, { type: "optional", element: { type: "sequence", elements: [{ type: "literal", value: "IF" }, { type: "literal", value: "NOT" }, { type: "literal", value: "EXISTS" }] } }, { type: "token_reference", name: "NAME" }, { type: "literal", value: "(" }, { type: "rule_reference", name: "col_def" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: "," }, { type: "rule_reference", name: "col_def" }] } }, { type: "literal", value: ")" }] }
    },
    {
      name: "col_def",
      lineNumber: 56,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "NAME" }, { type: "repetition", element: { type: "rule_reference", name: "col_constraint" } }] }
    },
    {
      name: "col_constraint",
      lineNumber: 57,
      body: { type: "alternation", choices: [{ type: "group", element: { type: "sequence", elements: [{ type: "literal", value: "NOT" }, { type: "literal", value: "NULL" }] } }, { type: "literal", value: "NULL" }, { type: "group", element: { type: "sequence", elements: [{ type: "literal", value: "PRIMARY" }, { type: "literal", value: "KEY" }] } }, { type: "literal", value: "UNIQUE" }, { type: "group", element: { type: "sequence", elements: [{ type: "literal", value: "DEFAULT" }, { type: "rule_reference", name: "primary" }] } }] }
    },
    {
      name: "drop_table_stmt",
      lineNumber: 60,
      body: { type: "sequence", elements: [{ type: "literal", value: "DROP" }, { type: "literal", value: "TABLE" }, { type: "optional", element: { type: "sequence", elements: [{ type: "literal", value: "IF" }, { type: "literal", value: "EXISTS" }] } }, { type: "token_reference", name: "NAME" }] }
    },
    {
      name: "expr",
      lineNumber: 64,
      body: { type: "rule_reference", name: "or_expr" }
    },
    {
      name: "or_expr",
      lineNumber: 65,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "and_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: "OR" }, { type: "rule_reference", name: "and_expr" }] } }] }
    },
    {
      name: "and_expr",
      lineNumber: 66,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "not_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: "AND" }, { type: "rule_reference", name: "not_expr" }] } }] }
    },
    {
      name: "not_expr",
      lineNumber: 67,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "literal", value: "NOT" }, { type: "rule_reference", name: "not_expr" }] }, { type: "rule_reference", name: "comparison" }] }
    },
    {
      name: "comparison",
      lineNumber: 68,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "additive" }, { type: "optional", element: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "rule_reference", name: "cmp_op" }, { type: "rule_reference", name: "additive" }] }, { type: "sequence", elements: [{ type: "literal", value: "BETWEEN" }, { type: "rule_reference", name: "additive" }, { type: "literal", value: "AND" }, { type: "rule_reference", name: "additive" }] }, { type: "sequence", elements: [{ type: "literal", value: "NOT" }, { type: "literal", value: "BETWEEN" }, { type: "rule_reference", name: "additive" }, { type: "literal", value: "AND" }, { type: "rule_reference", name: "additive" }] }, { type: "sequence", elements: [{ type: "literal", value: "IN" }, { type: "literal", value: "(" }, { type: "rule_reference", name: "value_list" }, { type: "literal", value: ")" }] }, { type: "sequence", elements: [{ type: "literal", value: "NOT" }, { type: "literal", value: "IN" }, { type: "literal", value: "(" }, { type: "rule_reference", name: "value_list" }, { type: "literal", value: ")" }] }, { type: "sequence", elements: [{ type: "literal", value: "LIKE" }, { type: "rule_reference", name: "additive" }] }, { type: "sequence", elements: [{ type: "literal", value: "NOT" }, { type: "literal", value: "LIKE" }, { type: "rule_reference", name: "additive" }] }, { type: "sequence", elements: [{ type: "literal", value: "IS" }, { type: "literal", value: "NULL" }] }, { type: "sequence", elements: [{ type: "literal", value: "IS" }, { type: "literal", value: "NOT" }, { type: "literal", value: "NULL" }] }] } }] }
    },
    {
      name: "cmp_op",
      lineNumber: 78,
      body: { type: "alternation", choices: [{ type: "literal", value: "=" }, { type: "token_reference", name: "NOT_EQUALS" }, { type: "literal", value: "<" }, { type: "literal", value: ">" }, { type: "literal", value: "<=" }, { type: "literal", value: ">=" }] }
    },
    {
      name: "additive",
      lineNumber: 79,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "multiplicative" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "literal", value: "+" }, { type: "literal", value: "-" }] } }, { type: "rule_reference", name: "multiplicative" }] } }] }
    },
    {
      name: "multiplicative",
      lineNumber: 80,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "unary" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "STAR" }, { type: "literal", value: "/" }, { type: "literal", value: "%" }] } }, { type: "rule_reference", name: "unary" }] } }] }
    },
    {
      name: "unary",
      lineNumber: 81,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "literal", value: "-" }, { type: "rule_reference", name: "unary" }] }, { type: "rule_reference", name: "primary" }] }
    },
    {
      name: "primary",
      lineNumber: 82,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "NUMBER" }, { type: "token_reference", name: "STRING" }, { type: "literal", value: "NULL" }, { type: "literal", value: "TRUE" }, { type: "literal", value: "FALSE" }, { type: "rule_reference", name: "function_call" }, { type: "rule_reference", name: "column_ref" }, { type: "sequence", elements: [{ type: "literal", value: "(" }, { type: "rule_reference", name: "expr" }, { type: "literal", value: ")" }] }] }
    },
    {
      name: "column_ref",
      lineNumber: 85,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "optional", element: { type: "sequence", elements: [{ type: "literal", value: "." }, { type: "token_reference", name: "NAME" }] } }] }
    },
    {
      name: "function_call",
      lineNumber: 86,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "literal", value: "(" }, { type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "STAR" }, { type: "optional", element: { type: "rule_reference", name: "value_list" } }] } }, { type: "literal", value: ")" }] }
    },
    {
      name: "value_list",
      lineNumber: 87,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: "," }, { type: "rule_reference", name: "expr" }] } }] }
    },
  ]
};
