# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: sql.grammar
# Regenerate with: grammar-tools compile-grammar sql.grammar
#
# This file embeds a ParserGrammar as native Ruby data structures.
# Downstream packages require this file directly instead of reading
# and parsing the .grammar file at runtime.

require "coding_adventures_grammar_tools"

GT = CodingAdventures::GrammarTools unless defined?(GT)

PARSER_GRAMMAR = GT::ParserGrammar.new(
  version: 1,
  rules: [
    GT::GrammarRule.new(
      name: "program",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "statement", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ";"),
            GT::RuleReference.new(name: "statement", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Literal.new(value: ";")),
      ]),
      line_number: 10,
    ),
    GT::GrammarRule.new(
      name: "statement",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "select_stmt", is_token: false),
        GT::RuleReference.new(name: "insert_stmt", is_token: false),
        GT::RuleReference.new(name: "update_stmt", is_token: false),
        GT::RuleReference.new(name: "delete_stmt", is_token: false),
        GT::RuleReference.new(name: "create_table_stmt", is_token: false),
        GT::RuleReference.new(name: "drop_table_stmt", is_token: false),
      ]),
      line_number: 12,
    ),
    GT::GrammarRule.new(
      name: "select_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "SELECT"),
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "DISTINCT"),
            GT::Literal.new(value: "ALL"),
          ])),
        GT::RuleReference.new(name: "select_list", is_token: false),
        GT::Literal.new(value: "FROM"),
        GT::RuleReference.new(name: "table_ref", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "join_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "where_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "group_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "having_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "order_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "limit_clause", is_token: false)),
      ]),
      line_number: 17,
    ),
    GT::GrammarRule.new(
      name: "select_list",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "select_item", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::Literal.new(value: ","),
              GT::RuleReference.new(name: "select_item", is_token: false),
            ])),
        ]),
      ]),
      line_number: 22,
    ),
    GT::GrammarRule.new(
      name: "select_item",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "AS"),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 23,
    ),
    GT::GrammarRule.new(
      name: "table_ref",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "table_name", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "AS"),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 25,
    ),
    GT::GrammarRule.new(
      name: "table_name",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "."),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 26,
    ),
    GT::GrammarRule.new(
      name: "join_clause",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "join_type", is_token: false),
        GT::Literal.new(value: "JOIN"),
        GT::RuleReference.new(name: "table_ref", is_token: false),
        GT::Literal.new(value: "ON"),
        GT::RuleReference.new(name: "expr", is_token: false),
      ]),
      line_number: 28,
    ),
    GT::GrammarRule.new(
      name: "join_type",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "CROSS"),
        GT::Literal.new(value: "INNER"),
        GT::Group.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "LEFT"),
            GT::OptionalElement.new(element: GT::Literal.new(value: "OUTER")),
          ])),
        GT::Group.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "RIGHT"),
            GT::OptionalElement.new(element: GT::Literal.new(value: "OUTER")),
          ])),
        GT::Group.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "FULL"),
            GT::OptionalElement.new(element: GT::Literal.new(value: "OUTER")),
          ])),
      ]),
      line_number: 29,
    ),
    GT::GrammarRule.new(
      name: "where_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "WHERE"),
        GT::RuleReference.new(name: "expr", is_token: false),
      ]),
      line_number: 32,
    ),
    GT::GrammarRule.new(
      name: "group_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "GROUP"),
        GT::Literal.new(value: "BY"),
        GT::RuleReference.new(name: "column_ref", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ","),
            GT::RuleReference.new(name: "column_ref", is_token: false),
          ])),
      ]),
      line_number: 33,
    ),
    GT::GrammarRule.new(
      name: "having_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "HAVING"),
        GT::RuleReference.new(name: "expr", is_token: false),
      ]),
      line_number: 34,
    ),
    GT::GrammarRule.new(
      name: "order_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "ORDER"),
        GT::Literal.new(value: "BY"),
        GT::RuleReference.new(name: "order_item", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ","),
            GT::RuleReference.new(name: "order_item", is_token: false),
          ])),
      ]),
      line_number: 35,
    ),
    GT::GrammarRule.new(
      name: "order_item",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "ASC"),
            GT::Literal.new(value: "DESC"),
          ])),
      ]),
      line_number: 36,
    ),
    GT::GrammarRule.new(
      name: "limit_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "LIMIT"),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "OFFSET"),
            GT::RuleReference.new(name: "NUMBER", is_token: true),
          ])),
      ]),
      line_number: 37,
    ),
    GT::GrammarRule.new(
      name: "insert_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "INSERT"),
        GT::Literal.new(value: "INTO"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "("),
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::Literal.new(value: ","),
                GT::RuleReference.new(name: "NAME", is_token: true),
              ])),
            GT::Literal.new(value: ")"),
          ])),
        GT::Literal.new(value: "VALUES"),
        GT::RuleReference.new(name: "row_value", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ","),
            GT::RuleReference.new(name: "row_value", is_token: false),
          ])),
      ]),
      line_number: 41,
    ),
    GT::GrammarRule.new(
      name: "row_value",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "("),
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ","),
            GT::RuleReference.new(name: "expr", is_token: false),
          ])),
        GT::Literal.new(value: ")"),
      ]),
      line_number: 44,
    ),
    GT::GrammarRule.new(
      name: "update_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "UPDATE"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "SET"),
        GT::RuleReference.new(name: "assignment", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ","),
            GT::RuleReference.new(name: "assignment", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "where_clause", is_token: false)),
      ]),
      line_number: 46,
    ),
    GT::GrammarRule.new(
      name: "assignment",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "="),
        GT::RuleReference.new(name: "expr", is_token: false),
      ]),
      line_number: 48,
    ),
    GT::GrammarRule.new(
      name: "delete_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "DELETE"),
        GT::Literal.new(value: "FROM"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "where_clause", is_token: false)),
      ]),
      line_number: 50,
    ),
    GT::GrammarRule.new(
      name: "create_table_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "CREATE"),
        GT::Literal.new(value: "TABLE"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "IF"),
            GT::Literal.new(value: "NOT"),
            GT::Literal.new(value: "EXISTS"),
          ])),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "("),
        GT::RuleReference.new(name: "col_def", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ","),
            GT::RuleReference.new(name: "col_def", is_token: false),
          ])),
        GT::Literal.new(value: ")"),
      ]),
      line_number: 54,
    ),
    GT::GrammarRule.new(
      name: "col_def",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "col_constraint", is_token: false)),
      ]),
      line_number: 56,
    ),
    GT::GrammarRule.new(
      name: "col_constraint",
      body: GT::Alternation.new(choices: [
        GT::Group.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "NOT"),
            GT::Literal.new(value: "NULL"),
          ])),
        GT::Literal.new(value: "NULL"),
        GT::Group.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "PRIMARY"),
            GT::Literal.new(value: "KEY"),
          ])),
        GT::Literal.new(value: "UNIQUE"),
        GT::Group.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "DEFAULT"),
            GT::RuleReference.new(name: "primary", is_token: false),
          ])),
      ]),
      line_number: 57,
    ),
    GT::GrammarRule.new(
      name: "drop_table_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "DROP"),
        GT::Literal.new(value: "TABLE"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "IF"),
            GT::Literal.new(value: "EXISTS"),
          ])),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 60,
    ),
    GT::GrammarRule.new(
      name: "expr",
      body: GT::RuleReference.new(name: "or_expr", is_token: false),
      line_number: 64,
    ),
    GT::GrammarRule.new(
      name: "or_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "and_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "OR"),
            GT::RuleReference.new(name: "and_expr", is_token: false),
          ])),
      ]),
      line_number: 65,
    ),
    GT::GrammarRule.new(
      name: "and_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "not_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "AND"),
            GT::RuleReference.new(name: "not_expr", is_token: false),
          ])),
      ]),
      line_number: 66,
    ),
    GT::GrammarRule.new(
      name: "not_expr",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "NOT"),
          GT::RuleReference.new(name: "not_expr", is_token: false),
        ]),
        GT::RuleReference.new(name: "comparison", is_token: false),
      ]),
      line_number: 67,
    ),
    GT::GrammarRule.new(
      name: "comparison",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "additive", is_token: false),
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "cmp_op", is_token: false),
              GT::RuleReference.new(name: "additive", is_token: false),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "BETWEEN"),
              GT::RuleReference.new(name: "additive", is_token: false),
              GT::Literal.new(value: "AND"),
              GT::RuleReference.new(name: "additive", is_token: false),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "NOT"),
              GT::Literal.new(value: "BETWEEN"),
              GT::RuleReference.new(name: "additive", is_token: false),
              GT::Literal.new(value: "AND"),
              GT::RuleReference.new(name: "additive", is_token: false),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "IN"),
              GT::Literal.new(value: "("),
              GT::RuleReference.new(name: "value_list", is_token: false),
              GT::Literal.new(value: ")"),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "NOT"),
              GT::Literal.new(value: "IN"),
              GT::Literal.new(value: "("),
              GT::RuleReference.new(name: "value_list", is_token: false),
              GT::Literal.new(value: ")"),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "LIKE"),
              GT::RuleReference.new(name: "additive", is_token: false),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "NOT"),
              GT::Literal.new(value: "LIKE"),
              GT::RuleReference.new(name: "additive", is_token: false),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "IS"),
              GT::Literal.new(value: "NULL"),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "IS"),
              GT::Literal.new(value: "NOT"),
              GT::Literal.new(value: "NULL"),
            ]),
          ])),
      ]),
      line_number: 68,
    ),
    GT::GrammarRule.new(
      name: "cmp_op",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "="),
        GT::RuleReference.new(name: "NOT_EQUALS", is_token: true),
        GT::Literal.new(value: "<"),
        GT::Literal.new(value: ">"),
        GT::Literal.new(value: "<="),
        GT::Literal.new(value: ">="),
      ]),
      line_number: 78,
    ),
    GT::GrammarRule.new(
      name: "additive",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "multiplicative", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::Literal.new(value: "+"),
                GT::Literal.new(value: "-"),
              ])),
            GT::RuleReference.new(name: "multiplicative", is_token: false),
          ])),
      ]),
      line_number: 79,
    ),
    GT::GrammarRule.new(
      name: "multiplicative",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "unary", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "STAR", is_token: true),
                GT::Literal.new(value: "/"),
                GT::Literal.new(value: "%"),
              ])),
            GT::RuleReference.new(name: "unary", is_token: false),
          ])),
      ]),
      line_number: 80,
    ),
    GT::GrammarRule.new(
      name: "unary",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "-"),
          GT::RuleReference.new(name: "unary", is_token: false),
        ]),
        GT::RuleReference.new(name: "primary", is_token: false),
      ]),
      line_number: 81,
    ),
    GT::GrammarRule.new(
      name: "primary",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::Literal.new(value: "NULL"),
        GT::Literal.new(value: "TRUE"),
        GT::Literal.new(value: "FALSE"),
        GT::RuleReference.new(name: "function_call", is_token: false),
        GT::RuleReference.new(name: "column_ref", is_token: false),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "("),
          GT::RuleReference.new(name: "expr", is_token: false),
          GT::Literal.new(value: ")"),
        ]),
      ]),
      line_number: 82,
    ),
    GT::GrammarRule.new(
      name: "column_ref",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "."),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 85,
    ),
    GT::GrammarRule.new(
      name: "function_call",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "("),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "STAR", is_token: true),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "value_list", is_token: false)),
          ])),
        GT::Literal.new(value: ")"),
      ]),
      line_number: 86,
    ),
    GT::GrammarRule.new(
      name: "value_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ","),
            GT::RuleReference.new(name: "expr", is_token: false),
          ])),
      ]),
      line_number: 87,
    ),
  ],
)
