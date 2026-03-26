// AUTO-GENERATED FILE - DO NOT EDIT
import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const ExcelGrammar: ParserGrammar = {
  version: 1,
  rules: [
    {
      name: "formula",
      lineNumber: 15,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "ws" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "EQUALS" }, { type: "rule_reference", name: "ws" }] } }, { type: "rule_reference", name: "expression" }, { type: "rule_reference", name: "ws" }] }
    },
    {
      name: "ws",
      lineNumber: 17,
      body: { type: "repetition", element: { type: "token_reference", name: "SPACE" } }
    },
    {
      name: "req_space",
      lineNumber: 18,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "SPACE" }, { type: "repetition", element: { type: "token_reference", name: "SPACE" } }] }
    },
    {
      name: "expression",
      lineNumber: 20,
      body: { type: "rule_reference", name: "comparison_expr" }
    },
    {
      name: "comparison_expr",
      lineNumber: 22,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "concat_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "rule_reference", name: "ws" }, { type: "rule_reference", name: "comparison_op" }, { type: "rule_reference", name: "ws" }, { type: "rule_reference", name: "concat_expr" }] } }] }
    },
    {
      name: "comparison_op",
      lineNumber: 23,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "EQUALS" }, { type: "token_reference", name: "NOT_EQUALS" }, { type: "token_reference", name: "LESS_THAN" }, { type: "token_reference", name: "LESS_EQUALS" }, { type: "token_reference", name: "GREATER_THAN" }, { type: "token_reference", name: "GREATER_EQUALS" }] }
    },
    {
      name: "concat_expr",
      lineNumber: 26,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "additive_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "rule_reference", name: "ws" }, { type: "token_reference", name: "AMP" }, { type: "rule_reference", name: "ws" }, { type: "rule_reference", name: "additive_expr" }] } }] }
    },
    {
      name: "additive_expr",
      lineNumber: 27,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "multiplicative_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "rule_reference", name: "ws" }, { type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "PLUS" }, { type: "token_reference", name: "MINUS" }] } }, { type: "rule_reference", name: "ws" }, { type: "rule_reference", name: "multiplicative_expr" }] } }] }
    },
    {
      name: "multiplicative_expr",
      lineNumber: 28,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "power_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "rule_reference", name: "ws" }, { type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "STAR" }, { type: "token_reference", name: "SLASH" }] } }, { type: "rule_reference", name: "ws" }, { type: "rule_reference", name: "power_expr" }] } }] }
    },
    {
      name: "power_expr",
      lineNumber: 29,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "unary_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "rule_reference", name: "ws" }, { type: "token_reference", name: "CARET" }, { type: "rule_reference", name: "ws" }, { type: "rule_reference", name: "unary_expr" }] } }] }
    },
    {
      name: "unary_expr",
      lineNumber: 30,
      body: { type: "sequence", elements: [{ type: "repetition", element: { type: "sequence", elements: [{ type: "rule_reference", name: "prefix_op" }, { type: "rule_reference", name: "ws" }] } }, { type: "rule_reference", name: "postfix_expr" }] }
    },
    {
      name: "prefix_op",
      lineNumber: 31,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "PLUS" }, { type: "token_reference", name: "MINUS" }] }
    },
    {
      name: "postfix_expr",
      lineNumber: 32,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "primary" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "rule_reference", name: "ws" }, { type: "token_reference", name: "PERCENT" }] } }] }
    },
    {
      name: "primary",
      lineNumber: 34,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "parenthesized_expression" }, { type: "rule_reference", name: "constant" }, { type: "rule_reference", name: "function_call" }, { type: "rule_reference", name: "structure_reference" }, { type: "rule_reference", name: "reference_expression" }, { type: "rule_reference", name: "bang_reference" }, { type: "rule_reference", name: "bang_name" }, { type: "rule_reference", name: "name_reference" }] }
    },
    {
      name: "parenthesized_expression",
      lineNumber: 43,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "ws" }, { type: "rule_reference", name: "expression" }, { type: "rule_reference", name: "ws" }, { type: "token_reference", name: "RPAREN" }] }
    },
    {
      name: "constant",
      lineNumber: 45,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "NUMBER" }, { type: "token_reference", name: "STRING" }, { type: "token_reference", name: "KEYWORD" }, { type: "token_reference", name: "ERROR_CONSTANT" }, { type: "rule_reference", name: "array_constant" }] }
    },
    {
      name: "array_constant",
      lineNumber: 47,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LBRACE" }, { type: "rule_reference", name: "ws" }, { type: "rule_reference", name: "array_row" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "rule_reference", name: "ws" }, { type: "token_reference", name: "SEMICOLON" }, { type: "rule_reference", name: "ws" }, { type: "rule_reference", name: "array_row" }] } }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "ws" }, { type: "token_reference", name: "SEMICOLON" }] } }, { type: "rule_reference", name: "ws" }, { type: "token_reference", name: "RBRACE" }] }
    },
    {
      name: "array_row",
      lineNumber: 48,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "array_item" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "rule_reference", name: "ws" }, { type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "ws" }, { type: "rule_reference", name: "array_item" }] } }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "ws" }, { type: "token_reference", name: "COMMA" }] } }] }
    },
    {
      name: "array_item",
      lineNumber: 49,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "NUMBER" }, { type: "token_reference", name: "STRING" }, { type: "token_reference", name: "KEYWORD" }, { type: "token_reference", name: "ERROR_CONSTANT" }] }
    },
    {
      name: "function_call",
      lineNumber: 51,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "function_name" }, { type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "ws" }, { type: "optional", element: { type: "rule_reference", name: "function_argument_list" } }, { type: "rule_reference", name: "ws" }, { type: "token_reference", name: "RPAREN" }] }
    },
    {
      name: "function_name",
      lineNumber: 52,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "FUNCTION_NAME" }, { type: "token_reference", name: "NAME" }] }
    },
    {
      name: "function_argument_list",
      lineNumber: 53,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "function_argument" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "rule_reference", name: "ws" }, { type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "ws" }, { type: "rule_reference", name: "function_argument" }] } }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "ws" }, { type: "token_reference", name: "COMMA" }] } }] }
    },
    {
      name: "function_argument",
      lineNumber: 54,
      body: { type: "optional", element: { type: "rule_reference", name: "expression" } }
    },
    {
      name: "reference_expression",
      lineNumber: 56,
      body: { type: "rule_reference", name: "union_reference" }
    },
    {
      name: "union_reference",
      lineNumber: 57,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "intersection_reference" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "rule_reference", name: "ws" }, { type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "ws" }, { type: "rule_reference", name: "intersection_reference" }] } }] }
    },
    {
      name: "intersection_reference",
      lineNumber: 58,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "range_reference" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "rule_reference", name: "req_space" }, { type: "rule_reference", name: "range_reference" }] } }] }
    },
    {
      name: "range_reference",
      lineNumber: 59,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "reference_primary" }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "ws" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "ws" }, { type: "rule_reference", name: "reference_primary" }] } }] }
    },
    {
      name: "reference_primary",
      lineNumber: 61,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "parenthesized_reference" }, { type: "rule_reference", name: "prefixed_reference" }, { type: "rule_reference", name: "external_reference" }, { type: "rule_reference", name: "structure_reference" }, { type: "rule_reference", name: "a1_reference" }, { type: "rule_reference", name: "bang_reference" }, { type: "rule_reference", name: "bang_name" }, { type: "rule_reference", name: "name_reference" }] }
    },
    {
      name: "parenthesized_reference",
      lineNumber: 70,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "ws" }, { type: "rule_reference", name: "reference_expression" }, { type: "rule_reference", name: "ws" }, { type: "token_reference", name: "RPAREN" }] }
    },
    {
      name: "prefixed_reference",
      lineNumber: 71,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "REF_PREFIX" }, { type: "group", element: { type: "alternation", choices: [{ type: "rule_reference", name: "a1_reference" }, { type: "rule_reference", name: "name_reference" }, { type: "rule_reference", name: "structure_reference" }] } }] }
    },
    {
      name: "external_reference",
      lineNumber: 72,
      body: { type: "token_reference", name: "REF_PREFIX" }
    },
    {
      name: "bang_reference",
      lineNumber: 73,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "BANG" }, { type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "CELL" }, { type: "token_reference", name: "COLUMN_REF" }, { type: "token_reference", name: "ROW_REF" }, { type: "token_reference", name: "NUMBER" }] } }] }
    },
    {
      name: "bang_name",
      lineNumber: 74,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "BANG" }, { type: "rule_reference", name: "name_reference" }] }
    },
    {
      name: "name_reference",
      lineNumber: 75,
      body: { type: "token_reference", name: "NAME" }
    },
    {
      name: "column_reference",
      lineNumber: 77,
      body: { type: "sequence", elements: [{ type: "optional", element: { type: "token_reference", name: "DOLLAR" } }, { type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "COLUMN_REF" }, { type: "token_reference", name: "NAME" }] } }] }
    },
    {
      name: "row_reference",
      lineNumber: 78,
      body: { type: "sequence", elements: [{ type: "optional", element: { type: "token_reference", name: "DOLLAR" } }, { type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "ROW_REF" }, { type: "token_reference", name: "NUMBER" }] } }] }
    },
    {
      name: "a1_reference",
      lineNumber: 80,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "CELL" }, { type: "rule_reference", name: "column_reference" }, { type: "rule_reference", name: "row_reference" }, { type: "token_reference", name: "COLUMN_REF" }, { type: "token_reference", name: "ROW_REF" }, { type: "token_reference", name: "NAME" }, { type: "token_reference", name: "NUMBER" }] }
    },
    {
      name: "structure_reference",
      lineNumber: 82,
      body: { type: "sequence", elements: [{ type: "optional", element: { type: "rule_reference", name: "table_name" } }, { type: "rule_reference", name: "intra_table_reference" }] }
    },
    {
      name: "table_name",
      lineNumber: 83,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "TABLE_NAME" }, { type: "token_reference", name: "NAME" }] }
    },
    {
      name: "intra_table_reference",
      lineNumber: 84,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "STRUCTURED_KEYWORD" }, { type: "rule_reference", name: "structured_column_range" }, { type: "sequence", elements: [{ type: "token_reference", name: "LBRACKET" }, { type: "rule_reference", name: "ws" }, { type: "optional", element: { type: "rule_reference", name: "inner_structure_reference" } }, { type: "rule_reference", name: "ws" }, { type: "token_reference", name: "RBRACKET" }] }] }
    },
    {
      name: "inner_structure_reference",
      lineNumber: 87,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "rule_reference", name: "structured_keyword_list" }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "ws" }, { type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "ws" }, { type: "rule_reference", name: "structured_column_range" }] } }] }, { type: "rule_reference", name: "structured_column_range" }] }
    },
    {
      name: "structured_keyword_list",
      lineNumber: 89,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "STRUCTURED_KEYWORD" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "rule_reference", name: "ws" }, { type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "ws" }, { type: "token_reference", name: "STRUCTURED_KEYWORD" }] } }] }
    },
    {
      name: "structured_column_range",
      lineNumber: 90,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "structured_column" }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "ws" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "ws" }, { type: "rule_reference", name: "structured_column" }] } }] }
    },
    {
      name: "structured_column",
      lineNumber: 91,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "STRUCTURED_COLUMN" }, { type: "sequence", elements: [{ type: "token_reference", name: "AT" }, { type: "token_reference", name: "STRUCTURED_COLUMN" }] }] }
    },
  ]
};
