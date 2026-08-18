defmodule CodingAdventures.SqlParser.Grammar do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: sql.grammar
  # Regenerate with: grammar-tools compile-grammar sql.grammar
  #
  # This file embeds a ParserGrammar as native Elixir data structures.
  # Call parser_grammar/0 instead of reading and parsing the .grammar file.

  alias CodingAdventures.GrammarTools.ParserGrammar

  def parser_grammar do
    %ParserGrammar{
      rules: [
        %{
          name: "program",
          body: {:sequence, [
            {:rule_reference, "statement", false},
            {:repetition, {:sequence, [
                {:literal, ";"},
                {:rule_reference, "statement", false},
              ]}},
            {:optional, {:literal, ";"}},
          ]},
          line_number: 10,
        },
        %{
          name: "statement",
          body: {:alternation, [
            {:rule_reference, "query_stmt", false},
            {:rule_reference, "insert_stmt", false},
            {:rule_reference, "replace_stmt", false},
            {:rule_reference, "update_stmt", false},
            {:rule_reference, "delete_stmt", false},
            {:rule_reference, "create_table_stmt", false},
            {:rule_reference, "drop_table_stmt", false},
            {:rule_reference, "alter_table_stmt", false},
            {:rule_reference, "create_index_stmt", false},
            {:rule_reference, "drop_index_stmt", false},
            {:rule_reference, "create_view_stmt", false},
            {:rule_reference, "drop_view_stmt", false},
            {:rule_reference, "create_trigger_stmt", false},
            {:rule_reference, "drop_trigger_stmt", false},
            {:rule_reference, "begin_stmt", false},
            {:rule_reference, "commit_stmt", false},
            {:rule_reference, "rollback_to_stmt", false},
            {:rule_reference, "rollback_stmt", false},
            {:rule_reference, "savepoint_stmt", false},
            {:rule_reference, "release_stmt", false},
            {:rule_reference, "attach_stmt", false},
            {:rule_reference, "detach_stmt", false},
          ]},
          line_number: 12,
        },
        %{
          name: "attach_stmt",
          body: {:sequence, [
            {:literal, "ATTACH"},
            {:optional, {:literal, "DATABASE"}},
            {:rule_reference, "expr", false},
            {:literal, "AS"},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 25,
        },
        %{
          name: "detach_stmt",
          body: {:sequence, [
            {:literal, "DETACH"},
            {:optional, {:literal, "DATABASE"}},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 26,
        },
        %{
          name: "query_stmt",
          body: {:sequence, [
            {:optional, {:rule_reference, "with_clause", false}},
            {:group, {:alternation, [
                {:rule_reference, "values_stmt", false},
                {:rule_reference, "select_stmt", false},
              ]}},
            {:repetition, {:rule_reference, "set_op_clause", false}},
          ]},
          line_number: 48,
        },
        %{
          name: "values_stmt",
          body: {:sequence, [
            {:literal, "VALUES"},
            {:rule_reference, "row_value", false},
            {:repetition, {:sequence, [
                {:literal, ","},
                {:rule_reference, "row_value", false},
              ]}},
          ]},
          line_number: 55,
        },
        %{
          name: "with_clause",
          body: {:sequence, [
            {:literal, "WITH"},
            {:optional, {:literal, "RECURSIVE"}},
            {:rule_reference, "cte_def", false},
            {:repetition, {:sequence, [
                {:literal, ","},
                {:rule_reference, "cte_def", false},
              ]}},
          ]},
          line_number: 56,
        },
        %{
          name: "cte_def",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:literal, "("},
                {:rule_reference, "NAME", true},
                {:repetition, {:sequence, [
                    {:literal, ","},
                    {:rule_reference, "NAME", true},
                  ]}},
                {:literal, ")"},
              ]}},
            {:literal, "AS"},
            {:optional, {:sequence, [
                {:optional, {:literal, "NOT"}},
                {:literal, "MATERIALIZED"},
              ]}},
            {:literal, "("},
            {:rule_reference, "query_stmt", false},
            {:literal, ")"},
          ]},
          line_number: 61,
        },
        %{
          name: "set_op_clause",
          body: {:sequence, [
            {:group, {:alternation, [
                {:literal, "UNION"},
                {:literal, "INTERSECT"},
                {:literal, "EXCEPT"},
              ]}},
            {:optional, {:literal, "ALL"}},
            {:group, {:alternation, [
                {:rule_reference, "values_stmt", false},
                {:rule_reference, "select_stmt", false},
              ]}},
          ]},
          line_number: 68,
        },
        %{
          name: "select_stmt",
          body: {:sequence, [
            {:literal, "SELECT"},
            {:optional, {:alternation, [
                {:literal, "DISTINCT"},
                {:literal, "ALL"},
              ]}},
            {:rule_reference, "select_list", false},
            {:optional, {:sequence, [
                {:literal, "FROM"},
                {:rule_reference, "table_ref", false},
                {:repetition, {:rule_reference, "join_clause", false}},
              ]}},
            {:optional, {:rule_reference, "where_clause", false}},
            {:optional, {:rule_reference, "group_clause", false}},
            {:optional, {:rule_reference, "having_clause", false}},
            {:optional, {:rule_reference, "window_clause", false}},
            {:optional, {:rule_reference, "order_clause", false}},
            {:optional, {:rule_reference, "limit_clause", false}},
          ]},
          line_number: 73,
        },
        %{
          name: "select_list",
          body: {:alternation, [
            {:rule_reference, "STAR", true},
            {:sequence, [
              {:rule_reference, "select_item", false},
              {:repetition, {:sequence, [
                  {:literal, ","},
                  {:rule_reference, "select_item", false},
                ]}},
            ]},
          ]},
          line_number: 78,
        },
        %{
          name: "select_item",
          body: {:sequence, [
            {:rule_reference, "expr", false},
            {:optional, {:sequence, [
                {:optional, {:literal, "AS"}},
                {:rule_reference, "NAME", true},
              ]}},
          ]},
          line_number: 79,
        },
        %{
          name: "table_ref",
          body: {:alternation, [
            {:sequence, [
              {:literal, "("},
              {:rule_reference, "query_stmt", false},
              {:literal, ")"},
              {:optional, {:sequence, [
                  {:optional, {:literal, "AS"}},
                  {:rule_reference, "NAME", true},
                ]}},
            ]},
            {:sequence, [
              {:rule_reference, "table_name", false},
              {:optional, {:alternation, [
                  {:sequence, [
                    {:literal, "AS"},
                    {:rule_reference, "NAME", true},
                  ]},
                  {:rule_reference, "NAME", true},
                ]}},
              {:optional, {:rule_reference, "index_hint", false}},
            ]},
          ]},
          line_number: 100,
        },
        %{
          name: "table_name",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:literal, "."},
                {:rule_reference, "NAME", true},
              ]}},
          ]},
          line_number: 102,
        },
        %{
          name: "index_hint",
          body: {:alternation, [
            {:sequence, [
              {:literal, "INDEXED"},
              {:literal, "BY"},
              {:rule_reference, "NAME", true},
            ]},
            {:sequence, [
              {:literal, "NOT"},
              {:literal, "INDEXED"},
            ]},
          ]},
          line_number: 103,
        },
        %{
          name: "join_clause",
          body: {:alternation, [
            {:group, {:sequence, [
                {:optional, {:rule_reference, "join_type", false}},
                {:literal, "JOIN"},
                {:rule_reference, "table_ref", false},
                {:optional, {:alternation, [
                    {:sequence, [
                      {:literal, "ON"},
                      {:rule_reference, "expr", false},
                    ]},
                    {:sequence, [
                      {:literal, "USING"},
                      {:literal, "("},
                      {:rule_reference, "NAME", true},
                      {:repetition, {:sequence, [
                          {:literal, ","},
                          {:rule_reference, "NAME", true},
                        ]}},
                      {:literal, ")"},
                    ]},
                  ]}},
              ]}},
            {:group, {:sequence, [
                {:literal, ","},
                {:rule_reference, "table_ref", false},
              ]}},
          ]},
          line_number: 111,
        },
        %{
          name: "join_type",
          body: {:alternation, [
            {:literal, "CROSS"},
            {:literal, "INNER"},
            {:literal, "NATURAL"},
            {:group, {:sequence, [
                {:literal, "LEFT"},
                {:optional, {:literal, "OUTER"}},
              ]}},
            {:group, {:sequence, [
                {:literal, "RIGHT"},
                {:optional, {:literal, "OUTER"}},
              ]}},
            {:group, {:sequence, [
                {:literal, "FULL"},
                {:optional, {:literal, "OUTER"}},
              ]}},
          ]},
          line_number: 113,
        },
        %{
          name: "where_clause",
          body: {:sequence, [
            {:literal, "WHERE"},
            {:rule_reference, "expr", false},
          ]},
          line_number: 117,
        },
        %{
          name: "group_clause",
          body: {:sequence, [
            {:literal, "GROUP"},
            {:literal, "BY"},
            {:rule_reference, "column_ref", false},
            {:repetition, {:sequence, [
                {:literal, ","},
                {:rule_reference, "column_ref", false},
              ]}},
          ]},
          line_number: 118,
        },
        %{
          name: "having_clause",
          body: {:sequence, [
            {:literal, "HAVING"},
            {:rule_reference, "expr", false},
          ]},
          line_number: 119,
        },
        %{
          name: "order_clause",
          body: {:sequence, [
            {:literal, "ORDER"},
            {:literal, "BY"},
            {:rule_reference, "order_item", false},
            {:repetition, {:sequence, [
                {:literal, ","},
                {:rule_reference, "order_item", false},
              ]}},
          ]},
          line_number: 120,
        },
        %{
          name: "order_item",
          body: {:sequence, [
            {:rule_reference, "expr", false},
            {:optional, {:sequence, [
                {:literal, "COLLATE"},
                {:rule_reference, "NAME", true},
              ]}},
            {:optional, {:alternation, [
                {:literal, "ASC"},
                {:literal, "DESC"},
              ]}},
            {:optional, {:sequence, [
                {:literal, "NULLS"},
                {:rule_reference, "NAME", true},
              ]}},
          ]},
          line_number: 141,
        },
        %{
          name: "limit_clause",
          body: {:sequence, [
            {:literal, "LIMIT"},
            {:rule_reference, "signed_number", false},
            {:optional, {:alternation, [
                {:sequence, [
                  {:literal, "OFFSET"},
                  {:rule_reference, "signed_number", false},
                ]},
                {:sequence, [
                  {:literal, ","},
                  {:rule_reference, "signed_number", false},
                ]},
              ]}},
          ]},
          line_number: 143,
        },
        %{
          name: "signed_number",
          body: {:sequence, [
            {:optional, {:literal, "-"}},
            {:rule_reference, "NUMBER", true},
          ]},
          line_number: 158,
        },
        %{
          name: "conflict_clause",
          body: {:sequence, [
            {:literal, "OR"},
            {:group, {:alternation, [
                {:literal, "REPLACE"},
                {:literal, "IGNORE"},
                {:literal, "ABORT"},
                {:literal, "FAIL"},
                {:literal, "ROLLBACK"},
              ]}},
          ]},
          line_number: 180,
        },
        %{
          name: "insert_stmt",
          body: {:sequence, [
            {:literal, "INSERT"},
            {:optional, {:rule_reference, "conflict_clause", false}},
            {:literal, "INTO"},
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:literal, "("},
                {:rule_reference, "NAME", true},
                {:repetition, {:sequence, [
                    {:literal, ","},
                    {:rule_reference, "NAME", true},
                  ]}},
                {:literal, ")"},
              ]}},
            {:rule_reference, "insert_body", false},
            {:optional, {:rule_reference, "upsert_clause", false}},
            {:optional, {:rule_reference, "returning_clause", false}},
          ]},
          line_number: 182,
        },
        %{
          name: "upsert_clause",
          body: {:sequence, [
            {:literal, "ON"},
            {:literal, "CONFLICT"},
            {:optional, {:sequence, [
                {:literal, "("},
                {:rule_reference, "NAME", true},
                {:repetition, {:sequence, [
                    {:literal, ","},
                    {:rule_reference, "NAME", true},
                  ]}},
                {:literal, ")"},
              ]}},
            {:group, {:alternation, [
                {:sequence, [
                  {:literal, "DO"},
                  {:literal, "NOTHING"},
                ]},
                {:sequence, [
                  {:literal, "DO"},
                  {:literal, "UPDATE"},
                  {:literal, "SET"},
                  {:rule_reference, "upsert_assignment", false},
                  {:repetition, {:sequence, [
                      {:literal, ","},
                      {:rule_reference, "upsert_assignment", false},
                    ]}},
                  {:optional, {:rule_reference, "where_clause", false}},
                ]},
              ]}},
          ]},
          line_number: 199,
        },
        %{
          name: "upsert_assignment",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:literal, "="},
            {:rule_reference, "expr", false},
          ]},
          line_number: 205,
        },
        %{
          name: "replace_stmt",
          body: {:sequence, [
            {:literal, "REPLACE"},
            {:literal, "INTO"},
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:literal, "("},
                {:rule_reference, "NAME", true},
                {:repetition, {:sequence, [
                    {:literal, ","},
                    {:rule_reference, "NAME", true},
                  ]}},
                {:literal, ")"},
              ]}},
            {:rule_reference, "insert_body", false},
            {:optional, {:rule_reference, "returning_clause", false}},
          ]},
          line_number: 206,
        },
        %{
          name: "insert_body",
          body: {:alternation, [
            {:sequence, [
              {:literal, "VALUES"},
              {:rule_reference, "row_value", false},
              {:repetition, {:sequence, [
                  {:literal, ","},
                  {:rule_reference, "row_value", false},
                ]}},
            ]},
            {:sequence, [
              {:literal, "DEFAULT"},
              {:literal, "VALUES"},
            ]},
            {:rule_reference, "query_stmt", false},
          ]},
          line_number: 210,
        },
        %{
          name: "row_value",
          body: {:sequence, [
            {:literal, "("},
            {:rule_reference, "expr", false},
            {:repetition, {:sequence, [
                {:literal, ","},
                {:rule_reference, "expr", false},
              ]}},
            {:literal, ")"},
          ]},
          line_number: 217,
        },
        %{
          name: "row_value_list",
          body: {:sequence, [
            {:rule_reference, "row_value", false},
            {:repetition, {:sequence, [
                {:literal, ","},
                {:rule_reference, "row_value", false},
              ]}},
          ]},
          line_number: 219,
        },
        %{
          name: "update_stmt",
          body: {:sequence, [
            {:literal, "UPDATE"},
            {:optional, {:rule_reference, "conflict_clause", false}},
            {:rule_reference, "NAME", true},
            {:literal, "SET"},
            {:rule_reference, "assignment", false},
            {:repetition, {:sequence, [
                {:literal, ","},
                {:rule_reference, "assignment", false},
              ]}},
            {:optional, {:rule_reference, "where_clause", false}},
            {:optional, {:rule_reference, "returning_clause", false}},
          ]},
          line_number: 228,
        },
        %{
          name: "assignment",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:literal, "="},
            {:rule_reference, "expr", false},
          ]},
          line_number: 230,
        },
        %{
          name: "delete_stmt",
          body: {:sequence, [
            {:literal, "DELETE"},
            {:literal, "FROM"},
            {:rule_reference, "NAME", true},
            {:optional, {:rule_reference, "where_clause", false}},
            {:optional, {:rule_reference, "returning_clause", false}},
          ]},
          line_number: 232,
        },
        %{
          name: "returning_clause",
          body: {:sequence, [
            {:literal, "RETURNING"},
            {:rule_reference, "returning_item", false},
            {:repetition, {:sequence, [
                {:literal, ","},
                {:rule_reference, "returning_item", false},
              ]}},
          ]},
          line_number: 234,
        },
        %{
          name: "returning_item",
          body: {:alternation, [
            {:literal, "*"},
            {:rule_reference, "expr", false},
          ]},
          line_number: 238,
        },
        %{
          name: "create_table_stmt",
          body: {:sequence, [
            {:literal, "CREATE"},
            {:literal, "TABLE"},
            {:optional, {:sequence, [
                {:literal, "IF"},
                {:literal, "NOT"},
                {:literal, "EXISTS"},
              ]}},
            {:rule_reference, "NAME", true},
            {:literal, "("},
            {:rule_reference, "col_def", false},
            {:repetition, {:sequence, [
                {:literal, ","},
                {:rule_reference, "col_def", false},
              ]}},
            {:repetition, {:sequence, [
                {:literal, ","},
                {:rule_reference, "table_constraint", false},
              ]}},
            {:literal, ")"},
            {:optional, {:rule_reference, "table_options", false}},
          ]},
          line_number: 242,
        },
        %{
          name: "table_constraint",
          body: {:alternation, [
            {:group, {:sequence, [
                {:literal, "PRIMARY"},
                {:literal, "KEY"},
                {:literal, "("},
                {:rule_reference, "NAME", true},
                {:repetition, {:sequence, [
                    {:literal, ","},
                    {:rule_reference, "NAME", true},
                  ]}},
                {:literal, ")"},
              ]}},
            {:group, {:sequence, [
                {:literal, "UNIQUE"},
                {:literal, "("},
                {:rule_reference, "NAME", true},
                {:repetition, {:sequence, [
                    {:literal, ","},
                    {:rule_reference, "NAME", true},
                  ]}},
                {:literal, ")"},
              ]}},
            {:group, {:sequence, [
                {:literal, "CHECK"},
                {:literal, "("},
                {:rule_reference, "expr", false},
                {:literal, ")"},
              ]}},
            {:group, {:sequence, [
                {:literal, "FOREIGN"},
                {:literal, "KEY"},
                {:literal, "("},
                {:rule_reference, "NAME", true},
                {:repetition, {:sequence, [
                    {:literal, ","},
                    {:rule_reference, "NAME", true},
                  ]}},
                {:literal, ")"},
                {:literal, "REFERENCES"},
                {:rule_reference, "NAME", true},
                {:optional, {:sequence, [
                    {:literal, "("},
                    {:rule_reference, "NAME", true},
                    {:repetition, {:sequence, [
                        {:literal, ","},
                        {:rule_reference, "NAME", true},
                      ]}},
                    {:literal, ")"},
                  ]}},
              ]}},
          ]},
          line_number: 250,
        },
        %{
          name: "table_options",
          body: {:sequence, [
            {:rule_reference, "table_option", false},
            {:repetition, {:sequence, [
                {:literal, ","},
                {:rule_reference, "table_option", false},
              ]}},
          ]},
          line_number: 261,
        },
        %{
          name: "table_option",
          body: {:alternation, [
            {:literal, "STRICT"},
            {:sequence, [
              {:literal, "WITHOUT"},
              {:rule_reference, "NAME", true},
            ]},
          ]},
          line_number: 262,
        },
        %{
          name: "col_def",
          body: {:sequence, [
            {:negative_lookahead, {:group, {:sequence, [
                  {:literal, "FOREIGN"},
                  {:literal, "KEY"},
                ]}}},
            {:rule_reference, "NAME", true},
            {:optional, {:rule_reference, "col_type", false}},
            {:repetition, {:rule_reference, "col_constraint", false}},
          ]},
          line_number: 267,
        },
        %{
          name: "col_type",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:literal, "("},
                {:rule_reference, "NUMBER", true},
                {:repetition, {:sequence, [
                    {:literal, ","},
                    {:rule_reference, "NUMBER", true},
                  ]}},
                {:literal, ")"},
              ]}},
          ]},
          line_number: 279,
        },
        %{
          name: "col_constraint",
          body: {:alternation, [
            {:group, {:sequence, [
                {:literal, "NOT"},
                {:literal, "NULL"},
                {:optional, {:rule_reference, "col_conflict_clause", false}},
              ]}},
            {:literal, "NULL"},
            {:group, {:sequence, [
                {:literal, "PRIMARY"},
                {:literal, "KEY"},
                {:optional, {:literal, "AUTOINCREMENT"}},
                {:optional, {:rule_reference, "col_conflict_clause", false}},
              ]}},
            {:group, {:sequence, [
                {:literal, "UNIQUE"},
                {:optional, {:rule_reference, "col_conflict_clause", false}},
              ]}},
            {:group, {:sequence, [
                {:literal, "DEFAULT"},
                {:rule_reference, "primary", false},
              ]}},
            {:group, {:sequence, [
                {:literal, "CHECK"},
                {:literal, "("},
                {:rule_reference, "expr", false},
                {:literal, ")"},
              ]}},
            {:group, {:sequence, [
                {:literal, "COLLATE"},
                {:rule_reference, "NAME", true},
              ]}},
            {:group, {:sequence, [
                {:literal, "REFERENCES"},
                {:rule_reference, "NAME", true},
                {:optional, {:sequence, [
                    {:literal, "("},
                    {:rule_reference, "NAME", true},
                    {:literal, ")"},
                  ]}},
              ]}},
          ]},
          line_number: 280,
        },
        %{
          name: "col_conflict_clause",
          body: {:sequence, [
            {:literal, "ON"},
            {:literal, "CONFLICT"},
            {:group, {:alternation, [
                {:literal, "ROLLBACK"},
                {:literal, "ABORT"},
                {:literal, "FAIL"},
                {:literal, "IGNORE"},
                {:literal, "REPLACE"},
              ]}},
          ]},
          line_number: 296,
        },
        %{
          name: "drop_table_stmt",
          body: {:sequence, [
            {:literal, "DROP"},
            {:literal, "TABLE"},
            {:optional, {:sequence, [
                {:literal, "IF"},
                {:literal, "EXISTS"},
              ]}},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 304,
        },
        %{
          name: "alter_table_stmt",
          body: {:sequence, [
            {:literal, "ALTER"},
            {:literal, "TABLE"},
            {:rule_reference, "NAME", true},
            {:group, {:alternation, [
                {:sequence, [
                  {:literal, "ADD"},
                  {:optional, {:literal, "COLUMN"}},
                  {:rule_reference, "col_def", false},
                ]},
                {:sequence, [
                  {:literal, "RENAME"},
                  {:literal, "TO"},
                  {:rule_reference, "NAME", true},
                ]},
                {:sequence, [
                  {:literal, "RENAME"},
                  {:optional, {:literal, "COLUMN"}},
                  {:rule_reference, "NAME", true},
                  {:literal, "TO"},
                  {:rule_reference, "NAME", true},
                ]},
                {:sequence, [
                  {:literal, "DROP"},
                  {:optional, {:literal, "COLUMN"}},
                  {:rule_reference, "NAME", true},
                ]},
              ]}},
          ]},
          line_number: 313,
        },
        %{
          name: "create_index_stmt",
          body: {:sequence, [
            {:literal, "CREATE"},
            {:optional, {:literal, "UNIQUE"}},
            {:literal, "INDEX"},
            {:optional, {:sequence, [
                {:literal, "IF"},
                {:literal, "NOT"},
                {:literal, "EXISTS"},
              ]}},
            {:rule_reference, "NAME", true},
            {:literal, "ON"},
            {:rule_reference, "NAME", true},
            {:literal, "("},
            {:rule_reference, "index_col", false},
            {:repetition, {:sequence, [
                {:literal, ","},
                {:rule_reference, "index_col", false},
              ]}},
            {:literal, ")"},
            {:optional, {:rule_reference, "where_clause", false}},
          ]},
          line_number: 327,
        },
        %{
          name: "index_col",
          body: {:sequence, [
            {:rule_reference, "expr", false},
            {:optional, {:sequence, [
                {:literal, "COLLATE"},
                {:rule_reference, "NAME", true},
              ]}},
            {:optional, {:alternation, [
                {:literal, "ASC"},
                {:literal, "DESC"},
              ]}},
          ]},
          line_number: 344,
        },
        %{
          name: "drop_index_stmt",
          body: {:sequence, [
            {:literal, "DROP"},
            {:literal, "INDEX"},
            {:optional, {:sequence, [
                {:literal, "IF"},
                {:literal, "EXISTS"},
              ]}},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 346,
        },
        %{
          name: "create_view_stmt",
          body: {:sequence, [
            {:literal, "CREATE"},
            {:literal, "VIEW"},
            {:optional, {:sequence, [
                {:literal, "IF"},
                {:literal, "NOT"},
                {:literal, "EXISTS"},
              ]}},
            {:rule_reference, "NAME", true},
            {:literal, "AS"},
            {:rule_reference, "query_stmt", false},
          ]},
          line_number: 354,
        },
        %{
          name: "drop_view_stmt",
          body: {:sequence, [
            {:literal, "DROP"},
            {:literal, "VIEW"},
            {:optional, {:sequence, [
                {:literal, "IF"},
                {:literal, "EXISTS"},
              ]}},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 356,
        },
        %{
          name: "begin_stmt",
          body: {:sequence, [
            {:literal, "BEGIN"},
            {:optional, {:literal, "TRANSACTION"}},
          ]},
          line_number: 362,
        },
        %{
          name: "commit_stmt",
          body: {:sequence, [
            {:literal, "COMMIT"},
            {:optional, {:literal, "TRANSACTION"}},
          ]},
          line_number: 363,
        },
        %{
          name: "rollback_stmt",
          body: {:sequence, [
            {:literal, "ROLLBACK"},
            {:optional, {:literal, "TRANSACTION"}},
          ]},
          line_number: 364,
        },
        %{
          name: "savepoint_stmt",
          body: {:sequence, [
            {:literal, "SAVEPOINT"},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 380,
        },
        %{
          name: "release_stmt",
          body: {:sequence, [
            {:literal, "RELEASE"},
            {:optional, {:literal, "SAVEPOINT"}},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 381,
        },
        %{
          name: "rollback_to_stmt",
          body: {:sequence, [
            {:literal, "ROLLBACK"},
            {:literal, "TO"},
            {:optional, {:literal, "SAVEPOINT"}},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 382,
        },
        %{
          name: "expr",
          body: {:rule_reference, "or_expr", false},
          line_number: 386,
        },
        %{
          name: "or_expr",
          body: {:sequence, [
            {:rule_reference, "and_expr", false},
            {:repetition, {:sequence, [
                {:literal, "OR"},
                {:rule_reference, "and_expr", false},
              ]}},
          ]},
          line_number: 387,
        },
        %{
          name: "and_expr",
          body: {:sequence, [
            {:rule_reference, "not_expr", false},
            {:repetition, {:sequence, [
                {:literal, "AND"},
                {:rule_reference, "not_expr", false},
              ]}},
          ]},
          line_number: 388,
        },
        %{
          name: "not_expr",
          body: {:alternation, [
            {:sequence, [
              {:literal, "NOT"},
              {:rule_reference, "not_expr", false},
            ]},
            {:rule_reference, "comparison", false},
          ]},
          line_number: 389,
        },
        %{
          name: "collated",
          body: {:sequence, [
            {:rule_reference, "bitwise", false},
            {:optional, {:sequence, [
                {:literal, "COLLATE"},
                {:rule_reference, "NAME", true},
              ]}},
          ]},
          line_number: 402,
        },
        %{
          name: "comparison",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "row_value", false},
              {:rule_reference, "cmp_op", false},
              {:rule_reference, "row_value", false},
            ]},
            {:sequence, [
              {:rule_reference, "row_value", false},
              {:literal, "NOT"},
              {:literal, "IN"},
              {:literal, "("},
              {:rule_reference, "row_value_list", false},
              {:literal, ")"},
            ]},
            {:sequence, [
              {:rule_reference, "row_value", false},
              {:literal, "IN"},
              {:literal, "("},
              {:rule_reference, "row_value_list", false},
              {:literal, ")"},
            ]},
            {:sequence, [
              {:rule_reference, "collated", false},
              {:optional, {:alternation, [
                  {:sequence, [
                    {:rule_reference, "cmp_op", false},
                    {:rule_reference, "collated", false},
                  ]},
                  {:sequence, [
                    {:literal, "BETWEEN"},
                    {:rule_reference, "collated", false},
                    {:literal, "AND"},
                    {:rule_reference, "collated", false},
                  ]},
                  {:sequence, [
                    {:literal, "NOT"},
                    {:literal, "BETWEEN"},
                    {:rule_reference, "collated", false},
                    {:literal, "AND"},
                    {:rule_reference, "collated", false},
                  ]},
                  {:sequence, [
                    {:literal, "IN"},
                    {:literal, "("},
                    {:optional, {:rule_reference, "in_expr", false}},
                    {:literal, ")"},
                  ]},
                  {:sequence, [
                    {:literal, "NOT"},
                    {:literal, "IN"},
                    {:literal, "("},
                    {:optional, {:rule_reference, "in_expr", false}},
                    {:literal, ")"},
                  ]},
                  {:sequence, [
                    {:literal, "LIKE"},
                    {:rule_reference, "collated", false},
                    {:optional, {:sequence, [
                        {:literal, "ESCAPE"},
                        {:rule_reference, "collated", false},
                      ]}},
                  ]},
                  {:sequence, [
                    {:literal, "NOT"},
                    {:literal, "LIKE"},
                    {:rule_reference, "collated", false},
                    {:optional, {:sequence, [
                        {:literal, "ESCAPE"},
                        {:rule_reference, "collated", false},
                      ]}},
                  ]},
                  {:sequence, [
                    {:literal, "GLOB"},
                    {:rule_reference, "collated", false},
                  ]},
                  {:sequence, [
                    {:literal, "NOT"},
                    {:literal, "GLOB"},
                    {:rule_reference, "collated", false},
                  ]},
                  {:sequence, [
                    {:literal, "IS"},
                    {:literal, "NULL"},
                  ]},
                  {:sequence, [
                    {:literal, "IS"},
                    {:literal, "NOT"},
                    {:literal, "NULL"},
                  ]},
                  {:sequence, [
                    {:literal, "IS"},
                    {:literal, "DISTINCT"},
                    {:literal, "FROM"},
                    {:rule_reference, "collated", false},
                  ]},
                  {:sequence, [
                    {:literal, "IS"},
                    {:literal, "NOT"},
                    {:literal, "DISTINCT"},
                    {:literal, "FROM"},
                    {:rule_reference, "collated", false},
                  ]},
                  {:sequence, [
                    {:literal, "IS"},
                    {:literal, "NOT"},
                    {:rule_reference, "collated", false},
                  ]},
                  {:sequence, [
                    {:literal, "IS"},
                    {:rule_reference, "collated", false},
                  ]},
                ]}},
            ]},
          ]},
          line_number: 407,
        },
        %{
          name: "in_expr",
          body: {:alternation, [
            {:rule_reference, "query_stmt", false},
            {:rule_reference, "value_list", false},
          ]},
          line_number: 435,
        },
        %{
          name: "cmp_op",
          body: {:alternation, [
            {:literal, "="},
            {:rule_reference, "NOT_EQUALS", true},
            {:literal, "<"},
            {:literal, ">"},
            {:literal, "<="},
            {:literal, ">="},
          ]},
          line_number: 437,
        },
        %{
          name: "bitwise",
          body: {:sequence, [
            {:rule_reference, "additive", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:literal, "&"},
                    {:literal, "|"},
                    {:literal, "<<"},
                    {:literal, ">>"},
                  ]}},
                {:rule_reference, "additive", false},
              ]}},
          ]},
          line_number: 451,
        },
        %{
          name: "additive",
          body: {:sequence, [
            {:rule_reference, "multiplicative", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:literal, "+"},
                    {:literal, "-"},
                    {:literal, "||"},
                    {:rule_reference, "JSON_ARROW", true},
                    {:rule_reference, "JSON_ARROW_TEXT", true},
                  ]}},
                {:rule_reference, "multiplicative", false},
              ]}},
          ]},
          line_number: 452,
        },
        %{
          name: "multiplicative",
          body: {:sequence, [
            {:rule_reference, "unary", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "STAR", true},
                    {:literal, "/"},
                    {:literal, "%"},
                  ]}},
                {:rule_reference, "unary", false},
              ]}},
          ]},
          line_number: 453,
        },
        %{
          name: "unary",
          body: {:alternation, [
            {:sequence, [
              {:group, {:alternation, [
                  {:literal, "-"},
                  {:literal, "~"},
                  {:literal, "+"},
                ]}},
              {:rule_reference, "unary", false},
            ]},
            {:rule_reference, "primary", false},
          ]},
          line_number: 459,
        },
        %{
          name: "primary",
          body: {:alternation, [
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "STRING", true},
            {:rule_reference, "BLOB", true},
            {:literal, "NULL"},
            {:literal, "TRUE"},
            {:literal, "FALSE"},
            {:rule_reference, "case_expr", false},
            {:rule_reference, "cast_expr", false},
            {:rule_reference, "window_func_call", false},
            {:rule_reference, "function_call", false},
            {:sequence, [
              {:literal, "EXISTS"},
              {:literal, "("},
              {:rule_reference, "query_stmt", false},
              {:literal, ")"},
            ]},
            {:sequence, [
              {:literal, "("},
              {:rule_reference, "query_stmt", false},
              {:literal, ")"},
            ]},
            {:rule_reference, "column_ref", false},
            {:sequence, [
              {:literal, "("},
              {:rule_reference, "expr", false},
              {:literal, ")"},
            ]},
          ]},
          line_number: 479,
        },
        %{
          name: "column_ref",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:literal, "."},
                {:rule_reference, "NAME", true},
              ]}},
          ]},
          line_number: 489,
        },
        %{
          name: "function_call",
          body: {:sequence, [
            {:group, {:alternation, [
                {:rule_reference, "NAME", true},
                {:literal, "REPLACE"},
              ]}},
            {:literal, "("},
            {:group, {:alternation, [
                {:rule_reference, "STAR", true},
                {:sequence, [
                  {:literal, "DISTINCT"},
                  {:rule_reference, "value_list", false},
                ]},
                {:optional, {:rule_reference, "value_list", false}},
              ]}},
            {:literal, ")"},
            {:optional, {:rule_reference, "filter_clause", false}},
          ]},
          line_number: 503,
        },
        %{
          name: "filter_clause",
          body: {:sequence, [
            {:literal, "FILTER"},
            {:literal, "("},
            {:literal, "WHERE"},
            {:rule_reference, "expr", false},
            {:literal, ")"},
          ]},
          line_number: 507,
        },
        %{
          name: "cast_expr",
          body: {:sequence, [
            {:literal, "CAST"},
            {:literal, "("},
            {:rule_reference, "expr", false},
            {:literal, "AS"},
            {:rule_reference, "NAME", true},
            {:literal, ")"},
          ]},
          line_number: 513,
        },
        %{
          name: "window_func_call",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:literal, "("},
            {:group, {:alternation, [
                {:rule_reference, "STAR", true},
                {:optional, {:rule_reference, "value_list", false}},
              ]}},
            {:literal, ")"},
            {:literal, "OVER"},
            {:group, {:alternation, [
                {:sequence, [
                  {:literal, "("},
                  {:rule_reference, "window_spec", false},
                  {:literal, ")"},
                ]},
                {:rule_reference, "window_name_ref", false},
              ]}},
          ]},
          line_number: 542,
        },
        %{
          name: "window_name_ref",
          body: {:rule_reference, "NAME", true},
          line_number: 543,
        },
        %{
          name: "window_spec",
          body: {:sequence, [
            {:optional, {:rule_reference, "partition_clause", false}},
            {:optional, {:rule_reference, "order_clause", false}},
            {:optional, {:rule_reference, "frame_clause", false}},
          ]},
          line_number: 544,
        },
        %{
          name: "window_clause",
          body: {:sequence, [
            {:literal, "WINDOW"},
            {:rule_reference, "NAME", true},
            {:literal, "AS"},
            {:literal, "("},
            {:rule_reference, "window_spec", false},
            {:literal, ")"},
            {:repetition, {:sequence, [
                {:literal, ","},
                {:rule_reference, "NAME", true},
                {:literal, "AS"},
                {:literal, "("},
                {:rule_reference, "window_spec", false},
                {:literal, ")"},
              ]}},
          ]},
          line_number: 545,
        },
        %{
          name: "partition_clause",
          body: {:sequence, [
            {:literal, "PARTITION"},
            {:literal, "BY"},
            {:rule_reference, "expr", false},
            {:repetition, {:sequence, [
                {:literal, ","},
                {:rule_reference, "expr", false},
              ]}},
          ]},
          line_number: 546,
        },
        %{
          name: "value_list",
          body: {:sequence, [
            {:rule_reference, "expr", false},
            {:repetition, {:sequence, [
                {:literal, ","},
                {:rule_reference, "expr", false},
              ]}},
          ]},
          line_number: 547,
        },
        %{
          name: "frame_clause",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "frame_unit", false},
              {:literal, "BETWEEN"},
              {:rule_reference, "frame_bound", false},
              {:literal, "AND"},
              {:rule_reference, "frame_bound", false},
            ]},
            {:sequence, [
              {:rule_reference, "frame_unit", false},
              {:rule_reference, "frame_bound", false},
            ]},
          ]},
          line_number: 569,
        },
        %{
          name: "frame_unit",
          body: {:alternation, [
            {:literal, "ROWS"},
            {:literal, "RANGE"},
            {:literal, "GROUPS"},
          ]},
          line_number: 571,
        },
        %{
          name: "frame_bound",
          body: {:alternation, [
            {:sequence, [
              {:literal, "UNBOUNDED"},
              {:literal, "PRECEDING"},
            ]},
            {:sequence, [
              {:literal, "UNBOUNDED"},
              {:literal, "FOLLOWING"},
            ]},
            {:sequence, [
              {:literal, "CURRENT"},
              {:literal, "ROW"},
            ]},
            {:sequence, [
              {:rule_reference, "expr", false},
              {:literal, "PRECEDING"},
            ]},
            {:sequence, [
              {:rule_reference, "expr", false},
              {:literal, "FOLLOWING"},
            ]},
          ]},
          line_number: 572,
        },
        %{
          name: "create_trigger_stmt",
          body: {:sequence, [
            {:literal, "CREATE"},
            {:literal, "TRIGGER"},
            {:optional, {:sequence, [
                {:literal, "IF"},
                {:literal, "NOT"},
                {:literal, "EXISTS"},
              ]}},
            {:rule_reference, "NAME", true},
            {:group, {:alternation, [
                {:literal, "BEFORE"},
                {:literal, "AFTER"},
              ]}},
            {:group, {:alternation, [
                {:literal, "INSERT"},
                {:literal, "UPDATE"},
                {:literal, "DELETE"},
              ]}},
            {:literal, "ON"},
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:literal, "FOR"},
                {:literal, "EACH"},
                {:literal, "ROW"},
              ]}},
            {:literal, "BEGIN"},
            {:rule_reference, "trigger_body_stmt", false},
            {:literal, ";"},
            {:repetition, {:sequence, [
                {:rule_reference, "trigger_body_stmt", false},
                {:literal, ";"},
              ]}},
            {:literal, "END"},
          ]},
          line_number: 598,
        },
        %{
          name: "trigger_body_stmt",
          body: {:alternation, [
            {:rule_reference, "insert_stmt", false},
            {:rule_reference, "replace_stmt", false},
            {:rule_reference, "update_stmt", false},
            {:rule_reference, "delete_stmt", false},
            {:rule_reference, "query_stmt", false},
          ]},
          line_number: 603,
        },
        %{
          name: "drop_trigger_stmt",
          body: {:sequence, [
            {:literal, "DROP"},
            {:literal, "TRIGGER"},
            {:optional, {:sequence, [
                {:literal, "IF"},
                {:literal, "EXISTS"},
              ]}},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 605,
        },
        %{
          name: "case_expr",
          body: {:sequence, [
            {:literal, "CASE"},
            {:optional, {:rule_reference, "case_operand", false}},
            {:rule_reference, "case_when", false},
            {:repetition, {:rule_reference, "case_when", false}},
            {:optional, {:sequence, [
                {:literal, "ELSE"},
                {:rule_reference, "expr", false},
              ]}},
            {:literal, "END"},
          ]},
          line_number: 620,
        },
        %{
          name: "case_operand",
          body: {:rule_reference, "expr", false},
          line_number: 621,
        },
        %{
          name: "case_when",
          body: {:sequence, [
            {:literal, "WHEN"},
            {:rule_reference, "expr", false},
            {:literal, "THEN"},
            {:rule_reference, "expr", false},
          ]},
          line_number: 622,
        },
      ],
      version: 2,
    }
  end
end
