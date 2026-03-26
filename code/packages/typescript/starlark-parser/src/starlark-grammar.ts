// AUTO-GENERATED FILE - DO NOT EDIT
import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const StarlarkGrammar: ParserGrammar = {
  version: 1,
  rules: [
    {
      name: "file",
      lineNumber: 34,
      body: { type: "repetition", element: { type: "alternation", choices: [{ type: "token_reference", name: "NEWLINE" }, { type: "rule_reference", name: "statement" }] } }
    },
    {
      name: "statement",
      lineNumber: 48,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "compound_stmt" }, { type: "rule_reference", name: "simple_stmt" }] }
    },
    {
      name: "simple_stmt",
      lineNumber: 52,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "small_stmt" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "SEMICOLON" }, { type: "rule_reference", name: "small_stmt" }] } }, { type: "token_reference", name: "NEWLINE" }] }
    },
    {
      name: "small_stmt",
      lineNumber: 54,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "return_stmt" }, { type: "rule_reference", name: "break_stmt" }, { type: "rule_reference", name: "continue_stmt" }, { type: "rule_reference", name: "pass_stmt" }, { type: "rule_reference", name: "load_stmt" }, { type: "rule_reference", name: "assign_stmt" }] }
    },
    {
      name: "return_stmt",
      lineNumber: 68,
      body: { type: "sequence", elements: [{ type: "literal", value: "return" }, { type: "optional", element: { type: "rule_reference", name: "expression" } }] }
    },
    {
      name: "break_stmt",
      lineNumber: 71,
      body: { type: "literal", value: "break" }
    },
    {
      name: "continue_stmt",
      lineNumber: 74,
      body: { type: "literal", value: "continue" }
    },
    {
      name: "pass_stmt",
      lineNumber: 79,
      body: { type: "literal", value: "pass" }
    },
    {
      name: "load_stmt",
      lineNumber: 88,
      body: { type: "sequence", elements: [{ type: "literal", value: "load" }, { type: "token_reference", name: "LPAREN" }, { type: "token_reference", name: "STRING" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "load_arg" }] } }, { type: "optional", element: { type: "token_reference", name: "COMMA" } }, { type: "token_reference", name: "RPAREN" }] }
    },
    {
      name: "load_arg",
      lineNumber: 89,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "EQUALS" }, { type: "token_reference", name: "STRING" }] }, { type: "token_reference", name: "STRING" }] }
    },
    {
      name: "assign_stmt",
      lineNumber: 110,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "expression_list" }, { type: "optional", element: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "rule_reference", name: "assign_op" }, { type: "rule_reference", name: "augmented_assign_op" }] } }, { type: "rule_reference", name: "expression_list" }] } }] }
    },
    {
      name: "assign_op",
      lineNumber: 113,
      body: { type: "token_reference", name: "EQUALS" }
    },
    {
      name: "augmented_assign_op",
      lineNumber: 115,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "PLUS_EQUALS" }, { type: "token_reference", name: "MINUS_EQUALS" }, { type: "token_reference", name: "STAR_EQUALS" }, { type: "token_reference", name: "SLASH_EQUALS" }, { type: "token_reference", name: "FLOOR_DIV_EQUALS" }, { type: "token_reference", name: "PERCENT_EQUALS" }, { type: "token_reference", name: "AMP_EQUALS" }, { type: "token_reference", name: "PIPE_EQUALS" }, { type: "token_reference", name: "CARET_EQUALS" }, { type: "token_reference", name: "LEFT_SHIFT_EQUALS" }, { type: "token_reference", name: "RIGHT_SHIFT_EQUALS" }, { type: "token_reference", name: "DOUBLE_STAR_EQUALS" }] }
    },
    {
      name: "compound_stmt",
      lineNumber: 124,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "if_stmt" }, { type: "rule_reference", name: "for_stmt" }, { type: "rule_reference", name: "def_stmt" }] }
    },
    {
      name: "if_stmt",
      lineNumber: 136,
      body: { type: "sequence", elements: [{ type: "literal", value: "if" }, { type: "rule_reference", name: "expression" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "suite" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: "elif" }, { type: "rule_reference", name: "expression" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "suite" }] } }, { type: "optional", element: { type: "sequence", elements: [{ type: "literal", value: "else" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "suite" }] } }] }
    },
    {
      name: "for_stmt",
      lineNumber: 150,
      body: { type: "sequence", elements: [{ type: "literal", value: "for" }, { type: "rule_reference", name: "loop_vars" }, { type: "literal", value: "in" }, { type: "rule_reference", name: "expression" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "suite" }] }
    },
    {
      name: "loop_vars",
      lineNumber: 156,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "token_reference", name: "NAME" }] } }] }
    },
    {
      name: "def_stmt",
      lineNumber: 166,
      body: { type: "sequence", elements: [{ type: "literal", value: "def" }, { type: "token_reference", name: "NAME" }, { type: "token_reference", name: "LPAREN" }, { type: "optional", element: { type: "rule_reference", name: "parameters" } }, { type: "token_reference", name: "RPAREN" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "suite" }] }
    },
    {
      name: "suite",
      lineNumber: 177,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "simple_stmt" }, { type: "sequence", elements: [{ type: "token_reference", name: "NEWLINE" }, { type: "token_reference", name: "INDENT" }, { type: "repetition", element: { type: "rule_reference", name: "statement" } }, { type: "token_reference", name: "DEDENT" }] }] }
    },
    {
      name: "parameters",
      lineNumber: 198,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "parameter" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "parameter" }] } }, { type: "optional", element: { type: "token_reference", name: "COMMA" } }] }
    },
    {
      name: "parameter",
      lineNumber: 200,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "token_reference", name: "DOUBLE_STAR" }, { type: "token_reference", name: "NAME" }] }, { type: "sequence", elements: [{ type: "token_reference", name: "STAR" }, { type: "token_reference", name: "NAME" }] }, { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "EQUALS" }, { type: "rule_reference", name: "expression" }] }, { type: "token_reference", name: "NAME" }] }
    },
    {
      name: "expression_list",
      lineNumber: 234,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "expression" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "expression" }] } }, { type: "optional", element: { type: "token_reference", name: "COMMA" } }] }
    },
    {
      name: "expression",
      lineNumber: 239,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "lambda_expr" }, { type: "sequence", elements: [{ type: "rule_reference", name: "or_expr" }, { type: "optional", element: { type: "sequence", elements: [{ type: "literal", value: "if" }, { type: "rule_reference", name: "or_expr" }, { type: "literal", value: "else" }, { type: "rule_reference", name: "expression" }] } }] }] }
    },
    {
      name: "lambda_expr",
      lineNumber: 244,
      body: { type: "sequence", elements: [{ type: "literal", value: "lambda" }, { type: "optional", element: { type: "rule_reference", name: "lambda_params" } }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "expression" }] }
    },
    {
      name: "lambda_params",
      lineNumber: 245,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "lambda_param" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "lambda_param" }] } }, { type: "optional", element: { type: "token_reference", name: "COMMA" } }] }
    },
    {
      name: "lambda_param",
      lineNumber: 246,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "EQUALS" }, { type: "rule_reference", name: "expression" }] } }] }, { type: "sequence", elements: [{ type: "token_reference", name: "STAR" }, { type: "token_reference", name: "NAME" }] }, { type: "sequence", elements: [{ type: "token_reference", name: "DOUBLE_STAR" }, { type: "token_reference", name: "NAME" }] }] }
    },
    {
      name: "or_expr",
      lineNumber: 250,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "and_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: "or" }, { type: "rule_reference", name: "and_expr" }] } }] }
    },
    {
      name: "and_expr",
      lineNumber: 254,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "not_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: "and" }, { type: "rule_reference", name: "not_expr" }] } }] }
    },
    {
      name: "not_expr",
      lineNumber: 258,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "literal", value: "not" }, { type: "rule_reference", name: "not_expr" }] }, { type: "rule_reference", name: "comparison" }] }
    },
    {
      name: "comparison",
      lineNumber: 267,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "bitwise_or" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "rule_reference", name: "comp_op" }, { type: "rule_reference", name: "bitwise_or" }] } }] }
    },
    {
      name: "comp_op",
      lineNumber: 269,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "EQUALS_EQUALS" }, { type: "token_reference", name: "NOT_EQUALS" }, { type: "token_reference", name: "LESS_THAN" }, { type: "token_reference", name: "GREATER_THAN" }, { type: "token_reference", name: "LESS_EQUALS" }, { type: "token_reference", name: "GREATER_EQUALS" }, { type: "literal", value: "in" }, { type: "sequence", elements: [{ type: "literal", value: "not" }, { type: "literal", value: "in" }] }] }
    },
    {
      name: "bitwise_or",
      lineNumber: 275,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "bitwise_xor" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "PIPE" }, { type: "rule_reference", name: "bitwise_xor" }] } }] }
    },
    {
      name: "bitwise_xor",
      lineNumber: 276,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "bitwise_and" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "CARET" }, { type: "rule_reference", name: "bitwise_and" }] } }] }
    },
    {
      name: "bitwise_and",
      lineNumber: 277,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "shift" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "AMP" }, { type: "rule_reference", name: "shift" }] } }] }
    },
    {
      name: "shift",
      lineNumber: 280,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "arith" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "LEFT_SHIFT" }, { type: "token_reference", name: "RIGHT_SHIFT" }] } }, { type: "rule_reference", name: "arith" }] } }] }
    },
    {
      name: "arith",
      lineNumber: 284,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "term" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "PLUS" }, { type: "token_reference", name: "MINUS" }] } }, { type: "rule_reference", name: "term" }] } }] }
    },
    {
      name: "term",
      lineNumber: 289,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "factor" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "STAR" }, { type: "token_reference", name: "SLASH" }, { type: "token_reference", name: "FLOOR_DIV" }, { type: "token_reference", name: "PERCENT" }] } }, { type: "rule_reference", name: "factor" }] } }] }
    },
    {
      name: "factor",
      lineNumber: 295,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "PLUS" }, { type: "token_reference", name: "MINUS" }, { type: "token_reference", name: "TILDE" }] } }, { type: "rule_reference", name: "factor" }] }, { type: "rule_reference", name: "power" }] }
    },
    {
      name: "power",
      lineNumber: 303,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "primary" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "DOUBLE_STAR" }, { type: "rule_reference", name: "factor" }] } }] }
    },
    {
      name: "primary",
      lineNumber: 320,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "atom" }, { type: "repetition", element: { type: "rule_reference", name: "suffix" } }] }
    },
    {
      name: "suffix",
      lineNumber: 322,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "token_reference", name: "DOT" }, { type: "token_reference", name: "NAME" }] }, { type: "sequence", elements: [{ type: "token_reference", name: "LBRACKET" }, { type: "rule_reference", name: "subscript" }, { type: "token_reference", name: "RBRACKET" }] }, { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "optional", element: { type: "rule_reference", name: "arguments" } }, { type: "token_reference", name: "RPAREN" }] }] }
    },
    {
      name: "subscript",
      lineNumber: 334,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "expression" }, { type: "sequence", elements: [{ type: "optional", element: { type: "rule_reference", name: "expression" } }, { type: "token_reference", name: "COLON" }, { type: "optional", element: { type: "rule_reference", name: "expression" } }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "COLON" }, { type: "optional", element: { type: "rule_reference", name: "expression" } }] } }] }] }
    },
    {
      name: "atom",
      lineNumber: 343,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "INT" }, { type: "token_reference", name: "FLOAT" }, { type: "sequence", elements: [{ type: "token_reference", name: "STRING" }, { type: "repetition", element: { type: "token_reference", name: "STRING" } }] }, { type: "token_reference", name: "NAME" }, { type: "literal", value: "True" }, { type: "literal", value: "False" }, { type: "literal", value: "None" }, { type: "rule_reference", name: "list_expr" }, { type: "rule_reference", name: "dict_expr" }, { type: "rule_reference", name: "paren_expr" }] }
    },
    {
      name: "list_expr",
      lineNumber: 359,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LBRACKET" }, { type: "optional", element: { type: "rule_reference", name: "list_body" } }, { type: "token_reference", name: "RBRACKET" }] }
    },
    {
      name: "list_body",
      lineNumber: 361,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "rule_reference", name: "expression" }, { type: "rule_reference", name: "comp_clause" }] }, { type: "sequence", elements: [{ type: "rule_reference", name: "expression" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "expression" }] } }, { type: "optional", element: { type: "token_reference", name: "COMMA" } }] }] }
    },
    {
      name: "dict_expr",
      lineNumber: 367,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LBRACE" }, { type: "optional", element: { type: "rule_reference", name: "dict_body" } }, { type: "token_reference", name: "RBRACE" }] }
    },
    {
      name: "dict_body",
      lineNumber: 369,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "rule_reference", name: "dict_entry" }, { type: "rule_reference", name: "comp_clause" }] }, { type: "sequence", elements: [{ type: "rule_reference", name: "dict_entry" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "dict_entry" }] } }, { type: "optional", element: { type: "token_reference", name: "COMMA" } }] }] }
    },
    {
      name: "dict_entry",
      lineNumber: 372,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "expression" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "expression" }] }
    },
    {
      name: "paren_expr",
      lineNumber: 379,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "optional", element: { type: "rule_reference", name: "paren_body" } }, { type: "token_reference", name: "RPAREN" }] }
    },
    {
      name: "paren_body",
      lineNumber: 381,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "rule_reference", name: "expression" }, { type: "rule_reference", name: "comp_clause" }] }, { type: "sequence", elements: [{ type: "rule_reference", name: "expression" }, { type: "token_reference", name: "COMMA" }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "expression" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "expression" }] } }, { type: "optional", element: { type: "token_reference", name: "COMMA" } }] } }] }, { type: "rule_reference", name: "expression" }] }
    },
    {
      name: "comp_clause",
      lineNumber: 397,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "comp_for" }, { type: "repetition", element: { type: "alternation", choices: [{ type: "rule_reference", name: "comp_for" }, { type: "rule_reference", name: "comp_if" }] } }] }
    },
    {
      name: "comp_for",
      lineNumber: 399,
      body: { type: "sequence", elements: [{ type: "literal", value: "for" }, { type: "rule_reference", name: "loop_vars" }, { type: "literal", value: "in" }, { type: "rule_reference", name: "or_expr" }] }
    },
    {
      name: "comp_if",
      lineNumber: 401,
      body: { type: "sequence", elements: [{ type: "literal", value: "if" }, { type: "rule_reference", name: "or_expr" }] }
    },
    {
      name: "arguments",
      lineNumber: 420,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "argument" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "argument" }] } }, { type: "optional", element: { type: "token_reference", name: "COMMA" } }] }
    },
    {
      name: "argument",
      lineNumber: 422,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "token_reference", name: "DOUBLE_STAR" }, { type: "rule_reference", name: "expression" }] }, { type: "sequence", elements: [{ type: "token_reference", name: "STAR" }, { type: "rule_reference", name: "expression" }] }, { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "EQUALS" }, { type: "rule_reference", name: "expression" }] }, { type: "rule_reference", name: "expression" }] }
    },
  ]
};
