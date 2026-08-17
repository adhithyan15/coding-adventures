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
  version: 2,
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
        GT::RuleReference.new(name: "query_stmt", is_token: false),
        GT::RuleReference.new(name: "insert_stmt", is_token: false),
        GT::RuleReference.new(name: "replace_stmt", is_token: false),
        GT::RuleReference.new(name: "update_stmt", is_token: false),
        GT::RuleReference.new(name: "delete_stmt", is_token: false),
        GT::RuleReference.new(name: "create_table_stmt", is_token: false),
        GT::RuleReference.new(name: "drop_table_stmt", is_token: false),
        GT::RuleReference.new(name: "alter_table_stmt", is_token: false),
        GT::RuleReference.new(name: "create_index_stmt", is_token: false),
        GT::RuleReference.new(name: "drop_index_stmt", is_token: false),
        GT::RuleReference.new(name: "create_view_stmt", is_token: false),
        GT::RuleReference.new(name: "drop_view_stmt", is_token: false),
        GT::RuleReference.new(name: "create_trigger_stmt", is_token: false),
        GT::RuleReference.new(name: "drop_trigger_stmt", is_token: false),
        GT::RuleReference.new(name: "begin_stmt", is_token: false),
        GT::RuleReference.new(name: "commit_stmt", is_token: false),
        GT::RuleReference.new(name: "rollback_to_stmt", is_token: false),
        GT::RuleReference.new(name: "rollback_stmt", is_token: false),
        GT::RuleReference.new(name: "savepoint_stmt", is_token: false),
        GT::RuleReference.new(name: "release_stmt", is_token: false),
        GT::RuleReference.new(name: "attach_stmt", is_token: false),
        GT::RuleReference.new(name: "detach_stmt", is_token: false),
      ]),
      line_number: 12,
    ),
    GT::GrammarRule.new(
      name: "attach_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "ATTACH"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "DATABASE")),
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::Literal.new(value: "AS"),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 25,
    ),
    GT::GrammarRule.new(
      name: "detach_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "DETACH"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "DATABASE")),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 26,
    ),
    GT::GrammarRule.new(
      name: "query_stmt",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "with_clause", is_token: false)),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "values_stmt", is_token: false),
            GT::RuleReference.new(name: "select_stmt", is_token: false),
          ])),
        GT::Repetition.new(element: GT::RuleReference.new(name: "set_op_clause", is_token: false)),
      ]),
      line_number: 48,
    ),
    GT::GrammarRule.new(
      name: "values_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "VALUES"),
        GT::RuleReference.new(name: "row_value", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ","),
            GT::RuleReference.new(name: "row_value", is_token: false),
          ])),
      ]),
      line_number: 55,
    ),
    GT::GrammarRule.new(
      name: "with_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "WITH"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "RECURSIVE")),
        GT::RuleReference.new(name: "cte_def", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ","),
            GT::RuleReference.new(name: "cte_def", is_token: false),
          ])),
      ]),
      line_number: 56,
    ),
    GT::GrammarRule.new(
      name: "cte_def",
      body: GT::Sequence.new(elements: [
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
        GT::Literal.new(value: "AS"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::OptionalElement.new(element: GT::Literal.new(value: "NOT")),
            GT::Literal.new(value: "MATERIALIZED"),
          ])),
        GT::Literal.new(value: "("),
        GT::RuleReference.new(name: "query_stmt", is_token: false),
        GT::Literal.new(value: ")"),
      ]),
      line_number: 61,
    ),
    GT::GrammarRule.new(
      name: "set_op_clause",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "UNION"),
            GT::Literal.new(value: "INTERSECT"),
            GT::Literal.new(value: "EXCEPT"),
          ])),
        GT::OptionalElement.new(element: GT::Literal.new(value: "ALL")),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "values_stmt", is_token: false),
            GT::RuleReference.new(name: "select_stmt", is_token: false),
          ])),
      ]),
      line_number: 68,
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
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "FROM"),
            GT::RuleReference.new(name: "table_ref", is_token: false),
            GT::Repetition.new(element: GT::RuleReference.new(name: "join_clause", is_token: false)),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "where_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "group_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "having_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "window_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "order_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "limit_clause", is_token: false)),
      ]),
      line_number: 73,
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
      line_number: 78,
    ),
    GT::GrammarRule.new(
      name: "select_item",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::OptionalElement.new(element: GT::Literal.new(value: "AS")),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 79,
    ),
    GT::GrammarRule.new(
      name: "table_ref",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "("),
          GT::RuleReference.new(name: "query_stmt", is_token: false),
          GT::Literal.new(value: ")"),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::OptionalElement.new(element: GT::Literal.new(value: "AS")),
              GT::RuleReference.new(name: "NAME", is_token: true),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "table_name", is_token: false),
          GT::OptionalElement.new(element: GT::Alternation.new(choices: [
              GT::Sequence.new(elements: [
                GT::Literal.new(value: "AS"),
                GT::RuleReference.new(name: "NAME", is_token: true),
              ]),
              GT::RuleReference.new(name: "NAME", is_token: true),
            ])),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "index_hint", is_token: false)),
        ]),
      ]),
      line_number: 100,
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
      line_number: 102,
    ),
    GT::GrammarRule.new(
      name: "index_hint",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "INDEXED"),
          GT::Literal.new(value: "BY"),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "NOT"),
          GT::Literal.new(value: "INDEXED"),
        ]),
      ]),
      line_number: 103,
    ),
    GT::GrammarRule.new(
      name: "join_clause",
      body: GT::Alternation.new(choices: [
        GT::Group.new(element: GT::Sequence.new(elements: [
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "join_type", is_token: false)),
            GT::Literal.new(value: "JOIN"),
            GT::RuleReference.new(name: "table_ref", is_token: false),
            GT::OptionalElement.new(element: GT::Alternation.new(choices: [
                GT::Sequence.new(elements: [
                  GT::Literal.new(value: "ON"),
                  GT::RuleReference.new(name: "expr", is_token: false),
                ]),
                GT::Sequence.new(elements: [
                  GT::Literal.new(value: "USING"),
                  GT::Literal.new(value: "("),
                  GT::RuleReference.new(name: "NAME", is_token: true),
                  GT::Repetition.new(element: GT::Sequence.new(elements: [
                      GT::Literal.new(value: ","),
                      GT::RuleReference.new(name: "NAME", is_token: true),
                    ])),
                  GT::Literal.new(value: ")"),
                ]),
              ])),
          ])),
        GT::Group.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ","),
            GT::RuleReference.new(name: "table_ref", is_token: false),
          ])),
      ]),
      line_number: 111,
    ),
    GT::GrammarRule.new(
      name: "join_type",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "CROSS"),
        GT::Literal.new(value: "INNER"),
        GT::Literal.new(value: "NATURAL"),
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
      line_number: 113,
    ),
    GT::GrammarRule.new(
      name: "where_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "WHERE"),
        GT::RuleReference.new(name: "expr", is_token: false),
      ]),
      line_number: 117,
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
      line_number: 118,
    ),
    GT::GrammarRule.new(
      name: "having_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "HAVING"),
        GT::RuleReference.new(name: "expr", is_token: false),
      ]),
      line_number: 119,
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
      line_number: 120,
    ),
    GT::GrammarRule.new(
      name: "order_item",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "COLLATE"),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "ASC"),
            GT::Literal.new(value: "DESC"),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "NULLS"),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 141,
    ),
    GT::GrammarRule.new(
      name: "limit_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "LIMIT"),
        GT::RuleReference.new(name: "signed_number", is_token: false),
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "OFFSET"),
              GT::RuleReference.new(name: "signed_number", is_token: false),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: ","),
              GT::RuleReference.new(name: "signed_number", is_token: false),
            ]),
          ])),
      ]),
      line_number: 143,
    ),
    GT::GrammarRule.new(
      name: "signed_number",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Literal.new(value: "-")),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
      ]),
      line_number: 158,
    ),
    GT::GrammarRule.new(
      name: "conflict_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "OR"),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "REPLACE"),
            GT::Literal.new(value: "IGNORE"),
            GT::Literal.new(value: "ABORT"),
            GT::Literal.new(value: "FAIL"),
            GT::Literal.new(value: "ROLLBACK"),
          ])),
      ]),
      line_number: 180,
    ),
    GT::GrammarRule.new(
      name: "insert_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "INSERT"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "conflict_clause", is_token: false)),
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
        GT::RuleReference.new(name: "insert_body", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "upsert_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "returning_clause", is_token: false)),
      ]),
      line_number: 182,
    ),
    GT::GrammarRule.new(
      name: "upsert_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "ON"),
        GT::Literal.new(value: "CONFLICT"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "("),
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::Literal.new(value: ","),
                GT::RuleReference.new(name: "NAME", is_token: true),
              ])),
            GT::Literal.new(value: ")"),
          ])),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "DO"),
              GT::Literal.new(value: "NOTHING"),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "DO"),
              GT::Literal.new(value: "UPDATE"),
              GT::Literal.new(value: "SET"),
              GT::RuleReference.new(name: "upsert_assignment", is_token: false),
              GT::Repetition.new(element: GT::Sequence.new(elements: [
                  GT::Literal.new(value: ","),
                  GT::RuleReference.new(name: "upsert_assignment", is_token: false),
                ])),
              GT::OptionalElement.new(element: GT::RuleReference.new(name: "where_clause", is_token: false)),
            ]),
          ])),
      ]),
      line_number: 199,
    ),
    GT::GrammarRule.new(
      name: "upsert_assignment",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "="),
        GT::RuleReference.new(name: "expr", is_token: false),
      ]),
      line_number: 205,
    ),
    GT::GrammarRule.new(
      name: "replace_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "REPLACE"),
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
        GT::RuleReference.new(name: "insert_body", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "returning_clause", is_token: false)),
      ]),
      line_number: 206,
    ),
    GT::GrammarRule.new(
      name: "insert_body",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "VALUES"),
          GT::RuleReference.new(name: "row_value", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::Literal.new(value: ","),
              GT::RuleReference.new(name: "row_value", is_token: false),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "DEFAULT"),
          GT::Literal.new(value: "VALUES"),
        ]),
        GT::RuleReference.new(name: "query_stmt", is_token: false),
      ]),
      line_number: 210,
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
      line_number: 217,
    ),
    GT::GrammarRule.new(
      name: "row_value_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "row_value", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ","),
            GT::RuleReference.new(name: "row_value", is_token: false),
          ])),
      ]),
      line_number: 219,
    ),
    GT::GrammarRule.new(
      name: "update_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "UPDATE"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "conflict_clause", is_token: false)),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "SET"),
        GT::RuleReference.new(name: "assignment", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ","),
            GT::RuleReference.new(name: "assignment", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "where_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "returning_clause", is_token: false)),
      ]),
      line_number: 228,
    ),
    GT::GrammarRule.new(
      name: "assignment",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "="),
        GT::RuleReference.new(name: "expr", is_token: false),
      ]),
      line_number: 230,
    ),
    GT::GrammarRule.new(
      name: "delete_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "DELETE"),
        GT::Literal.new(value: "FROM"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "where_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "returning_clause", is_token: false)),
      ]),
      line_number: 232,
    ),
    GT::GrammarRule.new(
      name: "returning_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "RETURNING"),
        GT::RuleReference.new(name: "returning_item", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ","),
            GT::RuleReference.new(name: "returning_item", is_token: false),
          ])),
      ]),
      line_number: 234,
    ),
    GT::GrammarRule.new(
      name: "returning_item",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "*"),
        GT::RuleReference.new(name: "expr", is_token: false),
      ]),
      line_number: 238,
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
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ","),
            GT::RuleReference.new(name: "table_constraint", is_token: false),
          ])),
        GT::Literal.new(value: ")"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "table_options", is_token: false)),
      ]),
      line_number: 242,
    ),
    GT::GrammarRule.new(
      name: "table_constraint",
      body: GT::Alternation.new(choices: [
        GT::Group.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "PRIMARY"),
            GT::Literal.new(value: "KEY"),
            GT::Literal.new(value: "("),
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::Literal.new(value: ","),
                GT::RuleReference.new(name: "NAME", is_token: true),
              ])),
            GT::Literal.new(value: ")"),
          ])),
        GT::Group.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "UNIQUE"),
            GT::Literal.new(value: "("),
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::Literal.new(value: ","),
                GT::RuleReference.new(name: "NAME", is_token: true),
              ])),
            GT::Literal.new(value: ")"),
          ])),
        GT::Group.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "CHECK"),
            GT::Literal.new(value: "("),
            GT::RuleReference.new(name: "expr", is_token: false),
            GT::Literal.new(value: ")"),
          ])),
        GT::Group.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "FOREIGN"),
            GT::Literal.new(value: "KEY"),
            GT::Literal.new(value: "("),
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::Literal.new(value: ","),
                GT::RuleReference.new(name: "NAME", is_token: true),
              ])),
            GT::Literal.new(value: ")"),
            GT::Literal.new(value: "REFERENCES"),
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
          ])),
      ]),
      line_number: 250,
    ),
    GT::GrammarRule.new(
      name: "table_options",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "table_option", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ","),
            GT::RuleReference.new(name: "table_option", is_token: false),
          ])),
      ]),
      line_number: 261,
    ),
    GT::GrammarRule.new(
      name: "table_option",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "STRICT"),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "WITHOUT"),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
      ]),
      line_number: 262,
    ),
    GT::GrammarRule.new(
      name: "col_def",
      body: GT::Sequence.new(elements: [
        GT::NegativeLookahead.new(element: GT::Group.new(element: GT::Sequence.new(elements: [
              GT::Literal.new(value: "FOREIGN"),
              GT::Literal.new(value: "KEY"),
            ]))),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "col_type", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "col_constraint", is_token: false)),
      ]),
      line_number: 267,
    ),
    GT::GrammarRule.new(
      name: "col_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "("),
            GT::RuleReference.new(name: "NUMBER", is_token: true),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::Literal.new(value: ","),
                GT::RuleReference.new(name: "NUMBER", is_token: true),
              ])),
            GT::Literal.new(value: ")"),
          ])),
      ]),
      line_number: 279,
    ),
    GT::GrammarRule.new(
      name: "col_constraint",
      body: GT::Alternation.new(choices: [
        GT::Group.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "NOT"),
            GT::Literal.new(value: "NULL"),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "col_conflict_clause", is_token: false)),
          ])),
        GT::Literal.new(value: "NULL"),
        GT::Group.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "PRIMARY"),
            GT::Literal.new(value: "KEY"),
            GT::OptionalElement.new(element: GT::Literal.new(value: "AUTOINCREMENT")),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "col_conflict_clause", is_token: false)),
          ])),
        GT::Group.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "UNIQUE"),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "col_conflict_clause", is_token: false)),
          ])),
        GT::Group.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "DEFAULT"),
            GT::RuleReference.new(name: "primary", is_token: false),
          ])),
        GT::Group.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "CHECK"),
            GT::Literal.new(value: "("),
            GT::RuleReference.new(name: "expr", is_token: false),
            GT::Literal.new(value: ")"),
          ])),
        GT::Group.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "COLLATE"),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
        GT::Group.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "REFERENCES"),
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                GT::Literal.new(value: "("),
                GT::RuleReference.new(name: "NAME", is_token: true),
                GT::Literal.new(value: ")"),
              ])),
          ])),
      ]),
      line_number: 280,
    ),
    GT::GrammarRule.new(
      name: "col_conflict_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "ON"),
        GT::Literal.new(value: "CONFLICT"),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "ROLLBACK"),
            GT::Literal.new(value: "ABORT"),
            GT::Literal.new(value: "FAIL"),
            GT::Literal.new(value: "IGNORE"),
            GT::Literal.new(value: "REPLACE"),
          ])),
      ]),
      line_number: 296,
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
      line_number: 304,
    ),
    GT::GrammarRule.new(
      name: "alter_table_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "ALTER"),
        GT::Literal.new(value: "TABLE"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "ADD"),
              GT::OptionalElement.new(element: GT::Literal.new(value: "COLUMN")),
              GT::RuleReference.new(name: "col_def", is_token: false),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "RENAME"),
              GT::Literal.new(value: "TO"),
              GT::RuleReference.new(name: "NAME", is_token: true),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "RENAME"),
              GT::OptionalElement.new(element: GT::Literal.new(value: "COLUMN")),
              GT::RuleReference.new(name: "NAME", is_token: true),
              GT::Literal.new(value: "TO"),
              GT::RuleReference.new(name: "NAME", is_token: true),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "DROP"),
              GT::OptionalElement.new(element: GT::Literal.new(value: "COLUMN")),
              GT::RuleReference.new(name: "NAME", is_token: true),
            ]),
          ])),
      ]),
      line_number: 313,
    ),
    GT::GrammarRule.new(
      name: "create_index_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "CREATE"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "UNIQUE")),
        GT::Literal.new(value: "INDEX"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "IF"),
            GT::Literal.new(value: "NOT"),
            GT::Literal.new(value: "EXISTS"),
          ])),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "ON"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "("),
        GT::RuleReference.new(name: "index_col", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ","),
            GT::RuleReference.new(name: "index_col", is_token: false),
          ])),
        GT::Literal.new(value: ")"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "where_clause", is_token: false)),
      ]),
      line_number: 327,
    ),
    GT::GrammarRule.new(
      name: "index_col",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "COLLATE"),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "ASC"),
            GT::Literal.new(value: "DESC"),
          ])),
      ]),
      line_number: 344,
    ),
    GT::GrammarRule.new(
      name: "drop_index_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "DROP"),
        GT::Literal.new(value: "INDEX"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "IF"),
            GT::Literal.new(value: "EXISTS"),
          ])),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 346,
    ),
    GT::GrammarRule.new(
      name: "create_view_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "CREATE"),
        GT::Literal.new(value: "VIEW"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "IF"),
            GT::Literal.new(value: "NOT"),
            GT::Literal.new(value: "EXISTS"),
          ])),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "AS"),
        GT::RuleReference.new(name: "query_stmt", is_token: false),
      ]),
      line_number: 354,
    ),
    GT::GrammarRule.new(
      name: "drop_view_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "DROP"),
        GT::Literal.new(value: "VIEW"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "IF"),
            GT::Literal.new(value: "EXISTS"),
          ])),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 356,
    ),
    GT::GrammarRule.new(
      name: "begin_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "BEGIN"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "TRANSACTION")),
      ]),
      line_number: 362,
    ),
    GT::GrammarRule.new(
      name: "commit_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "COMMIT"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "TRANSACTION")),
      ]),
      line_number: 363,
    ),
    GT::GrammarRule.new(
      name: "rollback_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "ROLLBACK"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "TRANSACTION")),
      ]),
      line_number: 364,
    ),
    GT::GrammarRule.new(
      name: "savepoint_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "SAVEPOINT"),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 380,
    ),
    GT::GrammarRule.new(
      name: "release_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "RELEASE"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "SAVEPOINT")),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 381,
    ),
    GT::GrammarRule.new(
      name: "rollback_to_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "ROLLBACK"),
        GT::Literal.new(value: "TO"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "SAVEPOINT")),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 382,
    ),
    GT::GrammarRule.new(
      name: "expr",
      body: GT::RuleReference.new(name: "or_expr", is_token: false),
      line_number: 386,
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
      line_number: 387,
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
      line_number: 388,
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
      line_number: 389,
    ),
    GT::GrammarRule.new(
      name: "collated",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "bitwise", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "COLLATE"),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 402,
    ),
    GT::GrammarRule.new(
      name: "comparison",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "row_value", is_token: false),
          GT::RuleReference.new(name: "cmp_op", is_token: false),
          GT::RuleReference.new(name: "row_value", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "row_value", is_token: false),
          GT::Literal.new(value: "NOT"),
          GT::Literal.new(value: "IN"),
          GT::Literal.new(value: "("),
          GT::RuleReference.new(name: "row_value_list", is_token: false),
          GT::Literal.new(value: ")"),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "row_value", is_token: false),
          GT::Literal.new(value: "IN"),
          GT::Literal.new(value: "("),
          GT::RuleReference.new(name: "row_value_list", is_token: false),
          GT::Literal.new(value: ")"),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "collated", is_token: false),
          GT::OptionalElement.new(element: GT::Alternation.new(choices: [
              GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "cmp_op", is_token: false),
                GT::RuleReference.new(name: "collated", is_token: false),
              ]),
              GT::Sequence.new(elements: [
                GT::Literal.new(value: "BETWEEN"),
                GT::RuleReference.new(name: "collated", is_token: false),
                GT::Literal.new(value: "AND"),
                GT::RuleReference.new(name: "collated", is_token: false),
              ]),
              GT::Sequence.new(elements: [
                GT::Literal.new(value: "NOT"),
                GT::Literal.new(value: "BETWEEN"),
                GT::RuleReference.new(name: "collated", is_token: false),
                GT::Literal.new(value: "AND"),
                GT::RuleReference.new(name: "collated", is_token: false),
              ]),
              GT::Sequence.new(elements: [
                GT::Literal.new(value: "IN"),
                GT::Literal.new(value: "("),
                GT::OptionalElement.new(element: GT::RuleReference.new(name: "in_expr", is_token: false)),
                GT::Literal.new(value: ")"),
              ]),
              GT::Sequence.new(elements: [
                GT::Literal.new(value: "NOT"),
                GT::Literal.new(value: "IN"),
                GT::Literal.new(value: "("),
                GT::OptionalElement.new(element: GT::RuleReference.new(name: "in_expr", is_token: false)),
                GT::Literal.new(value: ")"),
              ]),
              GT::Sequence.new(elements: [
                GT::Literal.new(value: "LIKE"),
                GT::RuleReference.new(name: "collated", is_token: false),
                GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                    GT::Literal.new(value: "ESCAPE"),
                    GT::RuleReference.new(name: "collated", is_token: false),
                  ])),
              ]),
              GT::Sequence.new(elements: [
                GT::Literal.new(value: "NOT"),
                GT::Literal.new(value: "LIKE"),
                GT::RuleReference.new(name: "collated", is_token: false),
                GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                    GT::Literal.new(value: "ESCAPE"),
                    GT::RuleReference.new(name: "collated", is_token: false),
                  ])),
              ]),
              GT::Sequence.new(elements: [
                GT::Literal.new(value: "GLOB"),
                GT::RuleReference.new(name: "collated", is_token: false),
              ]),
              GT::Sequence.new(elements: [
                GT::Literal.new(value: "NOT"),
                GT::Literal.new(value: "GLOB"),
                GT::RuleReference.new(name: "collated", is_token: false),
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
              GT::Sequence.new(elements: [
                GT::Literal.new(value: "IS"),
                GT::Literal.new(value: "DISTINCT"),
                GT::Literal.new(value: "FROM"),
                GT::RuleReference.new(name: "collated", is_token: false),
              ]),
              GT::Sequence.new(elements: [
                GT::Literal.new(value: "IS"),
                GT::Literal.new(value: "NOT"),
                GT::Literal.new(value: "DISTINCT"),
                GT::Literal.new(value: "FROM"),
                GT::RuleReference.new(name: "collated", is_token: false),
              ]),
              GT::Sequence.new(elements: [
                GT::Literal.new(value: "IS"),
                GT::Literal.new(value: "NOT"),
                GT::RuleReference.new(name: "collated", is_token: false),
              ]),
              GT::Sequence.new(elements: [
                GT::Literal.new(value: "IS"),
                GT::RuleReference.new(name: "collated", is_token: false),
              ]),
            ])),
        ]),
      ]),
      line_number: 407,
    ),
    GT::GrammarRule.new(
      name: "in_expr",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "query_stmt", is_token: false),
        GT::RuleReference.new(name: "value_list", is_token: false),
      ]),
      line_number: 435,
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
      line_number: 437,
    ),
    GT::GrammarRule.new(
      name: "bitwise",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "additive", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::Literal.new(value: "&"),
                GT::Literal.new(value: "|"),
                GT::Literal.new(value: "<<"),
                GT::Literal.new(value: ">>"),
              ])),
            GT::RuleReference.new(name: "additive", is_token: false),
          ])),
      ]),
      line_number: 451,
    ),
    GT::GrammarRule.new(
      name: "additive",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "multiplicative", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::Literal.new(value: "+"),
                GT::Literal.new(value: "-"),
                GT::Literal.new(value: "||"),
                GT::RuleReference.new(name: "JSON_ARROW", is_token: true),
                GT::RuleReference.new(name: "JSON_ARROW_TEXT", is_token: true),
              ])),
            GT::RuleReference.new(name: "multiplicative", is_token: false),
          ])),
      ]),
      line_number: 452,
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
      line_number: 453,
    ),
    GT::GrammarRule.new(
      name: "unary",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::Literal.new(value: "-"),
              GT::Literal.new(value: "~"),
              GT::Literal.new(value: "+"),
            ])),
          GT::RuleReference.new(name: "unary", is_token: false),
        ]),
        GT::RuleReference.new(name: "primary", is_token: false),
      ]),
      line_number: 459,
    ),
    GT::GrammarRule.new(
      name: "primary",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "BLOB", is_token: true),
        GT::Literal.new(value: "NULL"),
        GT::Literal.new(value: "TRUE"),
        GT::Literal.new(value: "FALSE"),
        GT::RuleReference.new(name: "case_expr", is_token: false),
        GT::RuleReference.new(name: "cast_expr", is_token: false),
        GT::RuleReference.new(name: "window_func_call", is_token: false),
        GT::RuleReference.new(name: "function_call", is_token: false),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "EXISTS"),
          GT::Literal.new(value: "("),
          GT::RuleReference.new(name: "query_stmt", is_token: false),
          GT::Literal.new(value: ")"),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "("),
          GT::RuleReference.new(name: "query_stmt", is_token: false),
          GT::Literal.new(value: ")"),
        ]),
        GT::RuleReference.new(name: "column_ref", is_token: false),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "("),
          GT::RuleReference.new(name: "expr", is_token: false),
          GT::Literal.new(value: ")"),
        ]),
      ]),
      line_number: 479,
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
      line_number: 489,
    ),
    GT::GrammarRule.new(
      name: "function_call",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::Literal.new(value: "REPLACE"),
          ])),
        GT::Literal.new(value: "("),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "STAR", is_token: true),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "DISTINCT"),
              GT::RuleReference.new(name: "value_list", is_token: false),
            ]),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "value_list", is_token: false)),
          ])),
        GT::Literal.new(value: ")"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "filter_clause", is_token: false)),
      ]),
      line_number: 503,
    ),
    GT::GrammarRule.new(
      name: "filter_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "FILTER"),
        GT::Literal.new(value: "("),
        GT::Literal.new(value: "WHERE"),
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::Literal.new(value: ")"),
      ]),
      line_number: 507,
    ),
    GT::GrammarRule.new(
      name: "cast_expr",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "CAST"),
        GT::Literal.new(value: "("),
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::Literal.new(value: "AS"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: ")"),
      ]),
      line_number: 513,
    ),
    GT::GrammarRule.new(
      name: "window_func_call",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "("),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "STAR", is_token: true),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "value_list", is_token: false)),
          ])),
        GT::Literal.new(value: ")"),
        GT::Literal.new(value: "OVER"),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "("),
              GT::RuleReference.new(name: "window_spec", is_token: false),
              GT::Literal.new(value: ")"),
            ]),
            GT::RuleReference.new(name: "window_name_ref", is_token: false),
          ])),
      ]),
      line_number: 542,
    ),
    GT::GrammarRule.new(
      name: "window_name_ref",
      body: GT::RuleReference.new(name: "NAME", is_token: true),
      line_number: 543,
    ),
    GT::GrammarRule.new(
      name: "window_spec",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "partition_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "order_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "frame_clause", is_token: false)),
      ]),
      line_number: 544,
    ),
    GT::GrammarRule.new(
      name: "window_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "WINDOW"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "AS"),
        GT::Literal.new(value: "("),
        GT::RuleReference.new(name: "window_spec", is_token: false),
        GT::Literal.new(value: ")"),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ","),
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::Literal.new(value: "AS"),
            GT::Literal.new(value: "("),
            GT::RuleReference.new(name: "window_spec", is_token: false),
            GT::Literal.new(value: ")"),
          ])),
      ]),
      line_number: 545,
    ),
    GT::GrammarRule.new(
      name: "partition_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "PARTITION"),
        GT::Literal.new(value: "BY"),
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ","),
            GT::RuleReference.new(name: "expr", is_token: false),
          ])),
      ]),
      line_number: 546,
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
      line_number: 547,
    ),
    GT::GrammarRule.new(
      name: "frame_clause",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "frame_unit", is_token: false),
          GT::Literal.new(value: "BETWEEN"),
          GT::RuleReference.new(name: "frame_bound", is_token: false),
          GT::Literal.new(value: "AND"),
          GT::RuleReference.new(name: "frame_bound", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "frame_unit", is_token: false),
          GT::RuleReference.new(name: "frame_bound", is_token: false),
        ]),
      ]),
      line_number: 569,
    ),
    GT::GrammarRule.new(
      name: "frame_unit",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "ROWS"),
        GT::Literal.new(value: "RANGE"),
        GT::Literal.new(value: "GROUPS"),
      ]),
      line_number: 571,
    ),
    GT::GrammarRule.new(
      name: "frame_bound",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "UNBOUNDED"),
          GT::Literal.new(value: "PRECEDING"),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "UNBOUNDED"),
          GT::Literal.new(value: "FOLLOWING"),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "CURRENT"),
          GT::Literal.new(value: "ROW"),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "expr", is_token: false),
          GT::Literal.new(value: "PRECEDING"),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "expr", is_token: false),
          GT::Literal.new(value: "FOLLOWING"),
        ]),
      ]),
      line_number: 572,
    ),
    GT::GrammarRule.new(
      name: "create_trigger_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "CREATE"),
        GT::Literal.new(value: "TRIGGER"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "IF"),
            GT::Literal.new(value: "NOT"),
            GT::Literal.new(value: "EXISTS"),
          ])),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "BEFORE"),
            GT::Literal.new(value: "AFTER"),
          ])),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "INSERT"),
            GT::Literal.new(value: "UPDATE"),
            GT::Literal.new(value: "DELETE"),
          ])),
        GT::Literal.new(value: "ON"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "FOR"),
            GT::Literal.new(value: "EACH"),
            GT::Literal.new(value: "ROW"),
          ])),
        GT::Literal.new(value: "BEGIN"),
        GT::RuleReference.new(name: "trigger_body_stmt", is_token: false),
        GT::Literal.new(value: ";"),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "trigger_body_stmt", is_token: false),
            GT::Literal.new(value: ";"),
          ])),
        GT::Literal.new(value: "END"),
      ]),
      line_number: 598,
    ),
    GT::GrammarRule.new(
      name: "trigger_body_stmt",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "insert_stmt", is_token: false),
        GT::RuleReference.new(name: "replace_stmt", is_token: false),
        GT::RuleReference.new(name: "update_stmt", is_token: false),
        GT::RuleReference.new(name: "delete_stmt", is_token: false),
        GT::RuleReference.new(name: "query_stmt", is_token: false),
      ]),
      line_number: 603,
    ),
    GT::GrammarRule.new(
      name: "drop_trigger_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "DROP"),
        GT::Literal.new(value: "TRIGGER"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "IF"),
            GT::Literal.new(value: "EXISTS"),
          ])),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 605,
    ),
    GT::GrammarRule.new(
      name: "case_expr",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "CASE"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "case_operand", is_token: false)),
        GT::RuleReference.new(name: "case_when", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "case_when", is_token: false)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "ELSE"),
            GT::RuleReference.new(name: "expr", is_token: false),
          ])),
        GT::Literal.new(value: "END"),
      ]),
      line_number: 620,
    ),
    GT::GrammarRule.new(
      name: "case_operand",
      body: GT::RuleReference.new(name: "expr", is_token: false),
      line_number: 621,
    ),
    GT::GrammarRule.new(
      name: "case_when",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "WHEN"),
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::Literal.new(value: "THEN"),
        GT::RuleReference.new(name: "expr", is_token: false),
      ]),
      line_number: 622,
    ),
  ],
)
