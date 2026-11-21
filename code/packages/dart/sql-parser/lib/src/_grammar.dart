// AUTO-GENERATED FILE - DO NOT EDIT
// Source: sql.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 2,
  rules: [
    GrammarRule(
      name: "program",
      body: Sequence(
        elements: [
          RuleReference("statement", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal(";"),
                RuleReference("statement", isToken: false),
              ],
            ),
          ),
          Optional(element: Literal(";")),
        ],
      ),
      lineNumber: 10,
    ),
    GrammarRule(
      name: "statement",
      body: Alternation(
        choices: [
          RuleReference("query_stmt", isToken: false),
          RuleReference("insert_stmt", isToken: false),
          RuleReference("replace_stmt", isToken: false),
          RuleReference("update_stmt", isToken: false),
          RuleReference("delete_stmt", isToken: false),
          RuleReference("create_table_stmt", isToken: false),
          RuleReference("drop_table_stmt", isToken: false),
          RuleReference("alter_table_stmt", isToken: false),
          RuleReference("create_index_stmt", isToken: false),
          RuleReference("drop_index_stmt", isToken: false),
          RuleReference("create_view_stmt", isToken: false),
          RuleReference("drop_view_stmt", isToken: false),
          RuleReference("create_trigger_stmt", isToken: false),
          RuleReference("drop_trigger_stmt", isToken: false),
          RuleReference("begin_stmt", isToken: false),
          RuleReference("commit_stmt", isToken: false),
          RuleReference("rollback_to_stmt", isToken: false),
          RuleReference("rollback_stmt", isToken: false),
          RuleReference("savepoint_stmt", isToken: false),
          RuleReference("release_stmt", isToken: false),
          RuleReference("attach_stmt", isToken: false),
          RuleReference("detach_stmt", isToken: false),
        ],
      ),
      lineNumber: 12,
    ),
    GrammarRule(
      name: "attach_stmt",
      body: Sequence(
        elements: [
          Literal("ATTACH"),
          Optional(element: Literal("DATABASE")),
          RuleReference("expr", isToken: false),
          Literal("AS"),
          RuleReference("NAME", isToken: true),
        ],
      ),
      lineNumber: 25,
    ),
    GrammarRule(
      name: "detach_stmt",
      body: Sequence(
        elements: [
          Literal("DETACH"),
          Optional(element: Literal("DATABASE")),
          RuleReference("NAME", isToken: true),
        ],
      ),
      lineNumber: 26,
    ),
    GrammarRule(
      name: "query_stmt",
      body: Sequence(
        elements: [
          Optional(element: RuleReference("with_clause", isToken: false)),
          Group(
            element: Alternation(
              choices: [
                RuleReference("values_stmt", isToken: false),
                RuleReference("select_stmt", isToken: false),
              ],
            ),
          ),
          Repetition(element: RuleReference("set_op_clause", isToken: false)),
        ],
      ),
      lineNumber: 48,
    ),
    GrammarRule(
      name: "values_stmt",
      body: Sequence(
        elements: [
          Literal("VALUES"),
          RuleReference("row_value", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal(","),
                RuleReference("row_value", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 55,
    ),
    GrammarRule(
      name: "with_clause",
      body: Sequence(
        elements: [
          Literal("WITH"),
          Optional(element: Literal("RECURSIVE")),
          RuleReference("cte_def", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal(","),
                RuleReference("cte_def", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 56,
    ),
    GrammarRule(
      name: "cte_def",
      body: Sequence(
        elements: [
          RuleReference("NAME", isToken: true),
          Optional(
            element: Sequence(
              elements: [
                Literal("("),
                RuleReference("NAME", isToken: true),
                Repetition(
                  element: Sequence(
                    elements: [
                      Literal(","),
                      RuleReference("NAME", isToken: true),
                    ],
                  ),
                ),
                Literal(")"),
              ],
            ),
          ),
          Literal("AS"),
          Optional(
            element: Sequence(
              elements: [
                Optional(element: Literal("NOT")),
                Literal("MATERIALIZED"),
              ],
            ),
          ),
          Literal("("),
          RuleReference("query_stmt", isToken: false),
          Literal(")"),
        ],
      ),
      lineNumber: 61,
    ),
    GrammarRule(
      name: "set_op_clause",
      body: Sequence(
        elements: [
          Group(
            element: Alternation(
              choices: [
                Literal("UNION"),
                Literal("INTERSECT"),
                Literal("EXCEPT"),
              ],
            ),
          ),
          Optional(element: Literal("ALL")),
          Group(
            element: Alternation(
              choices: [
                RuleReference("values_stmt", isToken: false),
                RuleReference("select_stmt", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 68,
    ),
    GrammarRule(
      name: "select_stmt",
      body: Sequence(
        elements: [
          Literal("SELECT"),
          Optional(
            element: Alternation(
              choices: [Literal("DISTINCT"), Literal("ALL")],
            ),
          ),
          RuleReference("select_list", isToken: false),
          Optional(
            element: Sequence(
              elements: [
                Literal("FROM"),
                RuleReference("table_ref", isToken: false),
                Repetition(
                  element: RuleReference("join_clause", isToken: false),
                ),
              ],
            ),
          ),
          Optional(element: RuleReference("where_clause", isToken: false)),
          Optional(element: RuleReference("group_clause", isToken: false)),
          Optional(element: RuleReference("having_clause", isToken: false)),
          Optional(element: RuleReference("window_clause", isToken: false)),
          Optional(element: RuleReference("order_clause", isToken: false)),
          Optional(element: RuleReference("limit_clause", isToken: false)),
        ],
      ),
      lineNumber: 73,
    ),
    GrammarRule(
      name: "select_list",
      body: Alternation(
        choices: [
          RuleReference("STAR", isToken: true),
          Sequence(
            elements: [
              RuleReference("select_item", isToken: false),
              Repetition(
                element: Sequence(
                  elements: [
                    Literal(","),
                    RuleReference("select_item", isToken: false),
                  ],
                ),
              ),
            ],
          ),
        ],
      ),
      lineNumber: 78,
    ),
    GrammarRule(
      name: "select_item",
      body: Sequence(
        elements: [
          RuleReference("expr", isToken: false),
          Optional(
            element: Sequence(
              elements: [
                Optional(element: Literal("AS")),
                RuleReference("NAME", isToken: true),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 79,
    ),
    GrammarRule(
      name: "table_ref",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              Literal("("),
              RuleReference("query_stmt", isToken: false),
              Literal(")"),
              Optional(
                element: Sequence(
                  elements: [
                    Optional(element: Literal("AS")),
                    RuleReference("NAME", isToken: true),
                  ],
                ),
              ),
            ],
          ),
          Sequence(
            elements: [
              RuleReference("table_name", isToken: false),
              Optional(
                element: Alternation(
                  choices: [
                    Sequence(
                      elements: [
                        Literal("AS"),
                        RuleReference("NAME", isToken: true),
                      ],
                    ),
                    RuleReference("NAME", isToken: true),
                  ],
                ),
              ),
              Optional(element: RuleReference("index_hint", isToken: false)),
            ],
          ),
        ],
      ),
      lineNumber: 100,
    ),
    GrammarRule(
      name: "table_name",
      body: Sequence(
        elements: [
          RuleReference("NAME", isToken: true),
          Optional(
            element: Sequence(
              elements: [Literal("."), RuleReference("NAME", isToken: true)],
            ),
          ),
        ],
      ),
      lineNumber: 102,
    ),
    GrammarRule(
      name: "index_hint",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              Literal("INDEXED"),
              Literal("BY"),
              RuleReference("NAME", isToken: true),
            ],
          ),
          Sequence(elements: [Literal("NOT"), Literal("INDEXED")]),
        ],
      ),
      lineNumber: 103,
    ),
    GrammarRule(
      name: "join_clause",
      body: Alternation(
        choices: [
          Group(
            element: Sequence(
              elements: [
                Optional(element: RuleReference("join_type", isToken: false)),
                Literal("JOIN"),
                RuleReference("table_ref", isToken: false),
                Optional(
                  element: Alternation(
                    choices: [
                      Sequence(
                        elements: [
                          Literal("ON"),
                          RuleReference("expr", isToken: false),
                        ],
                      ),
                      Sequence(
                        elements: [
                          Literal("USING"),
                          Literal("("),
                          RuleReference("NAME", isToken: true),
                          Repetition(
                            element: Sequence(
                              elements: [
                                Literal(","),
                                RuleReference("NAME", isToken: true),
                              ],
                            ),
                          ),
                          Literal(")"),
                        ],
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
          Group(
            element: Sequence(
              elements: [
                Literal(","),
                RuleReference("table_ref", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 111,
    ),
    GrammarRule(
      name: "join_type",
      body: Alternation(
        choices: [
          Literal("CROSS"),
          Literal("INNER"),
          Literal("NATURAL"),
          Group(
            element: Sequence(
              elements: [
                Literal("LEFT"),
                Optional(element: Literal("OUTER")),
              ],
            ),
          ),
          Group(
            element: Sequence(
              elements: [
                Literal("RIGHT"),
                Optional(element: Literal("OUTER")),
              ],
            ),
          ),
          Group(
            element: Sequence(
              elements: [
                Literal("FULL"),
                Optional(element: Literal("OUTER")),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 113,
    ),
    GrammarRule(
      name: "where_clause",
      body: Sequence(
        elements: [Literal("WHERE"), RuleReference("expr", isToken: false)],
      ),
      lineNumber: 117,
    ),
    GrammarRule(
      name: "group_clause",
      body: Sequence(
        elements: [
          Literal("GROUP"),
          Literal("BY"),
          RuleReference("column_ref", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal(","),
                RuleReference("column_ref", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 118,
    ),
    GrammarRule(
      name: "having_clause",
      body: Sequence(
        elements: [Literal("HAVING"), RuleReference("expr", isToken: false)],
      ),
      lineNumber: 119,
    ),
    GrammarRule(
      name: "order_clause",
      body: Sequence(
        elements: [
          Literal("ORDER"),
          Literal("BY"),
          RuleReference("order_item", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal(","),
                RuleReference("order_item", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 120,
    ),
    GrammarRule(
      name: "order_item",
      body: Sequence(
        elements: [
          RuleReference("expr", isToken: false),
          Optional(
            element: Sequence(
              elements: [
                Literal("COLLATE"),
                RuleReference("NAME", isToken: true),
              ],
            ),
          ),
          Optional(
            element: Alternation(choices: [Literal("ASC"), Literal("DESC")]),
          ),
          Optional(
            element: Sequence(
              elements: [
                Literal("NULLS"),
                RuleReference("NAME", isToken: true),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 141,
    ),
    GrammarRule(
      name: "limit_clause",
      body: Sequence(
        elements: [
          Literal("LIMIT"),
          RuleReference("signed_number", isToken: false),
          Optional(
            element: Alternation(
              choices: [
                Sequence(
                  elements: [
                    Literal("OFFSET"),
                    RuleReference("signed_number", isToken: false),
                  ],
                ),
                Sequence(
                  elements: [
                    Literal(","),
                    RuleReference("signed_number", isToken: false),
                  ],
                ),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 143,
    ),
    GrammarRule(
      name: "signed_number",
      body: Sequence(
        elements: [
          Optional(element: Literal("-")),
          RuleReference("NUMBER", isToken: true),
        ],
      ),
      lineNumber: 158,
    ),
    GrammarRule(
      name: "conflict_clause",
      body: Sequence(
        elements: [
          Literal("OR"),
          Group(
            element: Alternation(
              choices: [
                Literal("REPLACE"),
                Literal("IGNORE"),
                Literal("ABORT"),
                Literal("FAIL"),
                Literal("ROLLBACK"),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 180,
    ),
    GrammarRule(
      name: "insert_stmt",
      body: Sequence(
        elements: [
          Literal("INSERT"),
          Optional(element: RuleReference("conflict_clause", isToken: false)),
          Literal("INTO"),
          RuleReference("NAME", isToken: true),
          Optional(
            element: Sequence(
              elements: [
                Literal("("),
                RuleReference("NAME", isToken: true),
                Repetition(
                  element: Sequence(
                    elements: [
                      Literal(","),
                      RuleReference("NAME", isToken: true),
                    ],
                  ),
                ),
                Literal(")"),
              ],
            ),
          ),
          RuleReference("insert_body", isToken: false),
          Optional(element: RuleReference("upsert_clause", isToken: false)),
          Optional(element: RuleReference("returning_clause", isToken: false)),
        ],
      ),
      lineNumber: 182,
    ),
    GrammarRule(
      name: "upsert_clause",
      body: Sequence(
        elements: [
          Literal("ON"),
          Literal("CONFLICT"),
          Optional(
            element: Sequence(
              elements: [
                Literal("("),
                RuleReference("NAME", isToken: true),
                Repetition(
                  element: Sequence(
                    elements: [
                      Literal(","),
                      RuleReference("NAME", isToken: true),
                    ],
                  ),
                ),
                Literal(")"),
              ],
            ),
          ),
          Group(
            element: Alternation(
              choices: [
                Sequence(elements: [Literal("DO"), Literal("NOTHING")]),
                Sequence(
                  elements: [
                    Literal("DO"),
                    Literal("UPDATE"),
                    Literal("SET"),
                    RuleReference("upsert_assignment", isToken: false),
                    Repetition(
                      element: Sequence(
                        elements: [
                          Literal(","),
                          RuleReference("upsert_assignment", isToken: false),
                        ],
                      ),
                    ),
                    Optional(
                      element: RuleReference("where_clause", isToken: false),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 199,
    ),
    GrammarRule(
      name: "upsert_assignment",
      body: Sequence(
        elements: [
          RuleReference("NAME", isToken: true),
          Literal("="),
          RuleReference("expr", isToken: false),
        ],
      ),
      lineNumber: 205,
    ),
    GrammarRule(
      name: "replace_stmt",
      body: Sequence(
        elements: [
          Literal("REPLACE"),
          Literal("INTO"),
          RuleReference("NAME", isToken: true),
          Optional(
            element: Sequence(
              elements: [
                Literal("("),
                RuleReference("NAME", isToken: true),
                Repetition(
                  element: Sequence(
                    elements: [
                      Literal(","),
                      RuleReference("NAME", isToken: true),
                    ],
                  ),
                ),
                Literal(")"),
              ],
            ),
          ),
          RuleReference("insert_body", isToken: false),
          Optional(element: RuleReference("returning_clause", isToken: false)),
        ],
      ),
      lineNumber: 206,
    ),
    GrammarRule(
      name: "insert_body",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              Literal("VALUES"),
              RuleReference("row_value", isToken: false),
              Repetition(
                element: Sequence(
                  elements: [
                    Literal(","),
                    RuleReference("row_value", isToken: false),
                  ],
                ),
              ),
            ],
          ),
          Sequence(elements: [Literal("DEFAULT"), Literal("VALUES")]),
          RuleReference("query_stmt", isToken: false),
        ],
      ),
      lineNumber: 210,
    ),
    GrammarRule(
      name: "row_value",
      body: Sequence(
        elements: [
          Literal("("),
          RuleReference("expr", isToken: false),
          Repetition(
            element: Sequence(
              elements: [Literal(","), RuleReference("expr", isToken: false)],
            ),
          ),
          Literal(")"),
        ],
      ),
      lineNumber: 217,
    ),
    GrammarRule(
      name: "row_value_list",
      body: Sequence(
        elements: [
          RuleReference("row_value", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal(","),
                RuleReference("row_value", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 219,
    ),
    GrammarRule(
      name: "update_stmt",
      body: Sequence(
        elements: [
          Literal("UPDATE"),
          Optional(element: RuleReference("conflict_clause", isToken: false)),
          RuleReference("NAME", isToken: true),
          Literal("SET"),
          RuleReference("assignment", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal(","),
                RuleReference("assignment", isToken: false),
              ],
            ),
          ),
          Optional(element: RuleReference("where_clause", isToken: false)),
          Optional(element: RuleReference("returning_clause", isToken: false)),
        ],
      ),
      lineNumber: 228,
    ),
    GrammarRule(
      name: "assignment",
      body: Sequence(
        elements: [
          RuleReference("NAME", isToken: true),
          Literal("="),
          RuleReference("expr", isToken: false),
        ],
      ),
      lineNumber: 230,
    ),
    GrammarRule(
      name: "delete_stmt",
      body: Sequence(
        elements: [
          Literal("DELETE"),
          Literal("FROM"),
          RuleReference("NAME", isToken: true),
          Optional(element: RuleReference("where_clause", isToken: false)),
          Optional(element: RuleReference("returning_clause", isToken: false)),
        ],
      ),
      lineNumber: 232,
    ),
    GrammarRule(
      name: "returning_clause",
      body: Sequence(
        elements: [
          Literal("RETURNING"),
          RuleReference("returning_item", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal(","),
                RuleReference("returning_item", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 234,
    ),
    GrammarRule(
      name: "returning_item",
      body: Alternation(
        choices: [Literal("*"), RuleReference("expr", isToken: false)],
      ),
      lineNumber: 238,
    ),
    GrammarRule(
      name: "create_table_stmt",
      body: Sequence(
        elements: [
          Literal("CREATE"),
          Literal("TABLE"),
          Optional(
            element: Sequence(
              elements: [Literal("IF"), Literal("NOT"), Literal("EXISTS")],
            ),
          ),
          RuleReference("NAME", isToken: true),
          Literal("("),
          RuleReference("col_def", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal(","),
                RuleReference("col_def", isToken: false),
              ],
            ),
          ),
          Repetition(
            element: Sequence(
              elements: [
                Literal(","),
                RuleReference("table_constraint", isToken: false),
              ],
            ),
          ),
          Literal(")"),
          Optional(element: RuleReference("table_options", isToken: false)),
        ],
      ),
      lineNumber: 242,
    ),
    GrammarRule(
      name: "table_constraint",
      body: Alternation(
        choices: [
          Group(
            element: Sequence(
              elements: [
                Literal("PRIMARY"),
                Literal("KEY"),
                Literal("("),
                RuleReference("NAME", isToken: true),
                Repetition(
                  element: Sequence(
                    elements: [
                      Literal(","),
                      RuleReference("NAME", isToken: true),
                    ],
                  ),
                ),
                Literal(")"),
              ],
            ),
          ),
          Group(
            element: Sequence(
              elements: [
                Literal("UNIQUE"),
                Literal("("),
                RuleReference("NAME", isToken: true),
                Repetition(
                  element: Sequence(
                    elements: [
                      Literal(","),
                      RuleReference("NAME", isToken: true),
                    ],
                  ),
                ),
                Literal(")"),
              ],
            ),
          ),
          Group(
            element: Sequence(
              elements: [
                Literal("CHECK"),
                Literal("("),
                RuleReference("expr", isToken: false),
                Literal(")"),
              ],
            ),
          ),
          Group(
            element: Sequence(
              elements: [
                Literal("FOREIGN"),
                Literal("KEY"),
                Literal("("),
                RuleReference("NAME", isToken: true),
                Repetition(
                  element: Sequence(
                    elements: [
                      Literal(","),
                      RuleReference("NAME", isToken: true),
                    ],
                  ),
                ),
                Literal(")"),
                Literal("REFERENCES"),
                RuleReference("NAME", isToken: true),
                Optional(
                  element: Sequence(
                    elements: [
                      Literal("("),
                      RuleReference("NAME", isToken: true),
                      Repetition(
                        element: Sequence(
                          elements: [
                            Literal(","),
                            RuleReference("NAME", isToken: true),
                          ],
                        ),
                      ),
                      Literal(")"),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 250,
    ),
    GrammarRule(
      name: "table_options",
      body: Sequence(
        elements: [
          RuleReference("table_option", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal(","),
                RuleReference("table_option", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 261,
    ),
    GrammarRule(
      name: "table_option",
      body: Alternation(
        choices: [
          Literal("STRICT"),
          Sequence(
            elements: [
              Literal("WITHOUT"),
              RuleReference("NAME", isToken: true),
            ],
          ),
        ],
      ),
      lineNumber: 262,
    ),
    GrammarRule(
      name: "col_def",
      body: Sequence(
        elements: [
          NegativeLookahead(
            element: Group(
              element: Sequence(elements: [Literal("FOREIGN"), Literal("KEY")]),
            ),
          ),
          RuleReference("NAME", isToken: true),
          Optional(element: RuleReference("col_type", isToken: false)),
          Repetition(element: RuleReference("col_constraint", isToken: false)),
        ],
      ),
      lineNumber: 267,
    ),
    GrammarRule(
      name: "col_type",
      body: Sequence(
        elements: [
          RuleReference("NAME", isToken: true),
          Optional(
            element: Sequence(
              elements: [
                Literal("("),
                RuleReference("NUMBER", isToken: true),
                Repetition(
                  element: Sequence(
                    elements: [
                      Literal(","),
                      RuleReference("NUMBER", isToken: true),
                    ],
                  ),
                ),
                Literal(")"),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 279,
    ),
    GrammarRule(
      name: "col_constraint",
      body: Alternation(
        choices: [
          Group(
            element: Sequence(
              elements: [
                Literal("NOT"),
                Literal("NULL"),
                Optional(
                  element: RuleReference("col_conflict_clause", isToken: false),
                ),
              ],
            ),
          ),
          Literal("NULL"),
          Group(
            element: Sequence(
              elements: [
                Literal("PRIMARY"),
                Literal("KEY"),
                Optional(element: Literal("AUTOINCREMENT")),
                Optional(
                  element: RuleReference("col_conflict_clause", isToken: false),
                ),
              ],
            ),
          ),
          Group(
            element: Sequence(
              elements: [
                Literal("UNIQUE"),
                Optional(
                  element: RuleReference("col_conflict_clause", isToken: false),
                ),
              ],
            ),
          ),
          Group(
            element: Sequence(
              elements: [
                Literal("DEFAULT"),
                RuleReference("primary", isToken: false),
              ],
            ),
          ),
          Group(
            element: Sequence(
              elements: [
                Literal("CHECK"),
                Literal("("),
                RuleReference("expr", isToken: false),
                Literal(")"),
              ],
            ),
          ),
          Group(
            element: Sequence(
              elements: [
                Literal("COLLATE"),
                RuleReference("NAME", isToken: true),
              ],
            ),
          ),
          Group(
            element: Sequence(
              elements: [
                Literal("REFERENCES"),
                RuleReference("NAME", isToken: true),
                Optional(
                  element: Sequence(
                    elements: [
                      Literal("("),
                      RuleReference("NAME", isToken: true),
                      Literal(")"),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 280,
    ),
    GrammarRule(
      name: "col_conflict_clause",
      body: Sequence(
        elements: [
          Literal("ON"),
          Literal("CONFLICT"),
          Group(
            element: Alternation(
              choices: [
                Literal("ROLLBACK"),
                Literal("ABORT"),
                Literal("FAIL"),
                Literal("IGNORE"),
                Literal("REPLACE"),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 296,
    ),
    GrammarRule(
      name: "drop_table_stmt",
      body: Sequence(
        elements: [
          Literal("DROP"),
          Literal("TABLE"),
          Optional(
            element: Sequence(elements: [Literal("IF"), Literal("EXISTS")]),
          ),
          RuleReference("NAME", isToken: true),
        ],
      ),
      lineNumber: 304,
    ),
    GrammarRule(
      name: "alter_table_stmt",
      body: Sequence(
        elements: [
          Literal("ALTER"),
          Literal("TABLE"),
          RuleReference("NAME", isToken: true),
          Group(
            element: Alternation(
              choices: [
                Sequence(
                  elements: [
                    Literal("ADD"),
                    Optional(element: Literal("COLUMN")),
                    RuleReference("col_def", isToken: false),
                  ],
                ),
                Sequence(
                  elements: [
                    Literal("RENAME"),
                    Literal("TO"),
                    RuleReference("NAME", isToken: true),
                  ],
                ),
                Sequence(
                  elements: [
                    Literal("RENAME"),
                    Optional(element: Literal("COLUMN")),
                    RuleReference("NAME", isToken: true),
                    Literal("TO"),
                    RuleReference("NAME", isToken: true),
                  ],
                ),
                Sequence(
                  elements: [
                    Literal("DROP"),
                    Optional(element: Literal("COLUMN")),
                    RuleReference("NAME", isToken: true),
                  ],
                ),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 313,
    ),
    GrammarRule(
      name: "create_index_stmt",
      body: Sequence(
        elements: [
          Literal("CREATE"),
          Optional(element: Literal("UNIQUE")),
          Literal("INDEX"),
          Optional(
            element: Sequence(
              elements: [Literal("IF"), Literal("NOT"), Literal("EXISTS")],
            ),
          ),
          RuleReference("NAME", isToken: true),
          Literal("ON"),
          RuleReference("NAME", isToken: true),
          Literal("("),
          RuleReference("index_col", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal(","),
                RuleReference("index_col", isToken: false),
              ],
            ),
          ),
          Literal(")"),
          Optional(element: RuleReference("where_clause", isToken: false)),
        ],
      ),
      lineNumber: 327,
    ),
    GrammarRule(
      name: "index_col",
      body: Sequence(
        elements: [
          RuleReference("expr", isToken: false),
          Optional(
            element: Sequence(
              elements: [
                Literal("COLLATE"),
                RuleReference("NAME", isToken: true),
              ],
            ),
          ),
          Optional(
            element: Alternation(choices: [Literal("ASC"), Literal("DESC")]),
          ),
        ],
      ),
      lineNumber: 344,
    ),
    GrammarRule(
      name: "drop_index_stmt",
      body: Sequence(
        elements: [
          Literal("DROP"),
          Literal("INDEX"),
          Optional(
            element: Sequence(elements: [Literal("IF"), Literal("EXISTS")]),
          ),
          RuleReference("NAME", isToken: true),
        ],
      ),
      lineNumber: 346,
    ),
    GrammarRule(
      name: "create_view_stmt",
      body: Sequence(
        elements: [
          Literal("CREATE"),
          Literal("VIEW"),
          Optional(
            element: Sequence(
              elements: [Literal("IF"), Literal("NOT"), Literal("EXISTS")],
            ),
          ),
          RuleReference("NAME", isToken: true),
          Literal("AS"),
          RuleReference("query_stmt", isToken: false),
        ],
      ),
      lineNumber: 354,
    ),
    GrammarRule(
      name: "drop_view_stmt",
      body: Sequence(
        elements: [
          Literal("DROP"),
          Literal("VIEW"),
          Optional(
            element: Sequence(elements: [Literal("IF"), Literal("EXISTS")]),
          ),
          RuleReference("NAME", isToken: true),
        ],
      ),
      lineNumber: 356,
    ),
    GrammarRule(
      name: "begin_stmt",
      body: Sequence(
        elements: [
          Literal("BEGIN"),
          Optional(element: Literal("TRANSACTION")),
        ],
      ),
      lineNumber: 362,
    ),
    GrammarRule(
      name: "commit_stmt",
      body: Sequence(
        elements: [
          Literal("COMMIT"),
          Optional(element: Literal("TRANSACTION")),
        ],
      ),
      lineNumber: 363,
    ),
    GrammarRule(
      name: "rollback_stmt",
      body: Sequence(
        elements: [
          Literal("ROLLBACK"),
          Optional(element: Literal("TRANSACTION")),
        ],
      ),
      lineNumber: 364,
    ),
    GrammarRule(
      name: "savepoint_stmt",
      body: Sequence(
        elements: [Literal("SAVEPOINT"), RuleReference("NAME", isToken: true)],
      ),
      lineNumber: 380,
    ),
    GrammarRule(
      name: "release_stmt",
      body: Sequence(
        elements: [
          Literal("RELEASE"),
          Optional(element: Literal("SAVEPOINT")),
          RuleReference("NAME", isToken: true),
        ],
      ),
      lineNumber: 381,
    ),
    GrammarRule(
      name: "rollback_to_stmt",
      body: Sequence(
        elements: [
          Literal("ROLLBACK"),
          Literal("TO"),
          Optional(element: Literal("SAVEPOINT")),
          RuleReference("NAME", isToken: true),
        ],
      ),
      lineNumber: 382,
    ),
    GrammarRule(
      name: "expr",
      body: RuleReference("or_expr", isToken: false),
      lineNumber: 386,
    ),
    GrammarRule(
      name: "or_expr",
      body: Sequence(
        elements: [
          RuleReference("and_expr", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal("OR"),
                RuleReference("and_expr", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 387,
    ),
    GrammarRule(
      name: "and_expr",
      body: Sequence(
        elements: [
          RuleReference("not_expr", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal("AND"),
                RuleReference("not_expr", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 388,
    ),
    GrammarRule(
      name: "not_expr",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              Literal("NOT"),
              RuleReference("not_expr", isToken: false),
            ],
          ),
          RuleReference("comparison", isToken: false),
        ],
      ),
      lineNumber: 389,
    ),
    GrammarRule(
      name: "collated",
      body: Sequence(
        elements: [
          RuleReference("bitwise", isToken: false),
          Optional(
            element: Sequence(
              elements: [
                Literal("COLLATE"),
                RuleReference("NAME", isToken: true),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 402,
    ),
    GrammarRule(
      name: "comparison",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              RuleReference("row_value", isToken: false),
              RuleReference("cmp_op", isToken: false),
              RuleReference("row_value", isToken: false),
            ],
          ),
          Sequence(
            elements: [
              RuleReference("row_value", isToken: false),
              Literal("NOT"),
              Literal("IN"),
              Literal("("),
              RuleReference("row_value_list", isToken: false),
              Literal(")"),
            ],
          ),
          Sequence(
            elements: [
              RuleReference("row_value", isToken: false),
              Literal("IN"),
              Literal("("),
              RuleReference("row_value_list", isToken: false),
              Literal(")"),
            ],
          ),
          Sequence(
            elements: [
              RuleReference("collated", isToken: false),
              Optional(
                element: Alternation(
                  choices: [
                    Sequence(
                      elements: [
                        RuleReference("cmp_op", isToken: false),
                        RuleReference("collated", isToken: false),
                      ],
                    ),
                    Sequence(
                      elements: [
                        Literal("BETWEEN"),
                        RuleReference("collated", isToken: false),
                        Literal("AND"),
                        RuleReference("collated", isToken: false),
                      ],
                    ),
                    Sequence(
                      elements: [
                        Literal("NOT"),
                        Literal("BETWEEN"),
                        RuleReference("collated", isToken: false),
                        Literal("AND"),
                        RuleReference("collated", isToken: false),
                      ],
                    ),
                    Sequence(
                      elements: [
                        Literal("IN"),
                        Literal("("),
                        Optional(
                          element: RuleReference("in_expr", isToken: false),
                        ),
                        Literal(")"),
                      ],
                    ),
                    Sequence(
                      elements: [
                        Literal("NOT"),
                        Literal("IN"),
                        Literal("("),
                        Optional(
                          element: RuleReference("in_expr", isToken: false),
                        ),
                        Literal(")"),
                      ],
                    ),
                    Sequence(
                      elements: [
                        Literal("LIKE"),
                        RuleReference("collated", isToken: false),
                        Optional(
                          element: Sequence(
                            elements: [
                              Literal("ESCAPE"),
                              RuleReference("collated", isToken: false),
                            ],
                          ),
                        ),
                      ],
                    ),
                    Sequence(
                      elements: [
                        Literal("NOT"),
                        Literal("LIKE"),
                        RuleReference("collated", isToken: false),
                        Optional(
                          element: Sequence(
                            elements: [
                              Literal("ESCAPE"),
                              RuleReference("collated", isToken: false),
                            ],
                          ),
                        ),
                      ],
                    ),
                    Sequence(
                      elements: [
                        Literal("GLOB"),
                        RuleReference("collated", isToken: false),
                      ],
                    ),
                    Sequence(
                      elements: [
                        Literal("NOT"),
                        Literal("GLOB"),
                        RuleReference("collated", isToken: false),
                      ],
                    ),
                    Sequence(elements: [Literal("IS"), Literal("NULL")]),
                    Sequence(
                      elements: [
                        Literal("IS"),
                        Literal("NOT"),
                        Literal("NULL"),
                      ],
                    ),
                    Sequence(
                      elements: [
                        Literal("IS"),
                        Literal("DISTINCT"),
                        Literal("FROM"),
                        RuleReference("collated", isToken: false),
                      ],
                    ),
                    Sequence(
                      elements: [
                        Literal("IS"),
                        Literal("NOT"),
                        Literal("DISTINCT"),
                        Literal("FROM"),
                        RuleReference("collated", isToken: false),
                      ],
                    ),
                    Sequence(
                      elements: [
                        Literal("IS"),
                        Literal("NOT"),
                        RuleReference("collated", isToken: false),
                      ],
                    ),
                    Sequence(
                      elements: [
                        Literal("IS"),
                        RuleReference("collated", isToken: false),
                      ],
                    ),
                  ],
                ),
              ),
            ],
          ),
        ],
      ),
      lineNumber: 407,
    ),
    GrammarRule(
      name: "in_expr",
      body: Alternation(
        choices: [
          RuleReference("query_stmt", isToken: false),
          RuleReference("value_list", isToken: false),
        ],
      ),
      lineNumber: 435,
    ),
    GrammarRule(
      name: "cmp_op",
      body: Alternation(
        choices: [
          Literal("="),
          RuleReference("NOT_EQUALS", isToken: true),
          Literal("<"),
          Literal(">"),
          Literal("<="),
          Literal(">="),
        ],
      ),
      lineNumber: 437,
    ),
    GrammarRule(
      name: "bitwise",
      body: Sequence(
        elements: [
          RuleReference("additive", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Group(
                  element: Alternation(
                    choices: [
                      Literal("&"),
                      Literal("|"),
                      Literal("<<"),
                      Literal(">>"),
                    ],
                  ),
                ),
                RuleReference("additive", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 451,
    ),
    GrammarRule(
      name: "additive",
      body: Sequence(
        elements: [
          RuleReference("multiplicative", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Group(
                  element: Alternation(
                    choices: [
                      Literal("+"),
                      Literal("-"),
                      Literal("||"),
                      RuleReference("JSON_ARROW", isToken: true),
                      RuleReference("JSON_ARROW_TEXT", isToken: true),
                    ],
                  ),
                ),
                RuleReference("multiplicative", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 452,
    ),
    GrammarRule(
      name: "multiplicative",
      body: Sequence(
        elements: [
          RuleReference("unary", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Group(
                  element: Alternation(
                    choices: [
                      RuleReference("STAR", isToken: true),
                      Literal("/"),
                      Literal("%"),
                    ],
                  ),
                ),
                RuleReference("unary", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 453,
    ),
    GrammarRule(
      name: "unary",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              Group(
                element: Alternation(
                  choices: [Literal("-"), Literal("~"), Literal("+")],
                ),
              ),
              RuleReference("unary", isToken: false),
            ],
          ),
          RuleReference("primary", isToken: false),
        ],
      ),
      lineNumber: 459,
    ),
    GrammarRule(
      name: "primary",
      body: Alternation(
        choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("BLOB", isToken: true),
          Literal("NULL"),
          Literal("TRUE"),
          Literal("FALSE"),
          RuleReference("case_expr", isToken: false),
          RuleReference("cast_expr", isToken: false),
          RuleReference("window_func_call", isToken: false),
          RuleReference("function_call", isToken: false),
          Sequence(
            elements: [
              Literal("EXISTS"),
              Literal("("),
              RuleReference("query_stmt", isToken: false),
              Literal(")"),
            ],
          ),
          Sequence(
            elements: [
              Literal("("),
              RuleReference("query_stmt", isToken: false),
              Literal(")"),
            ],
          ),
          RuleReference("column_ref", isToken: false),
          Sequence(
            elements: [
              Literal("("),
              RuleReference("expr", isToken: false),
              Literal(")"),
            ],
          ),
        ],
      ),
      lineNumber: 479,
    ),
    GrammarRule(
      name: "column_ref",
      body: Sequence(
        elements: [
          RuleReference("NAME", isToken: true),
          Optional(
            element: Sequence(
              elements: [Literal("."), RuleReference("NAME", isToken: true)],
            ),
          ),
        ],
      ),
      lineNumber: 489,
    ),
    GrammarRule(
      name: "function_call",
      body: Sequence(
        elements: [
          Group(
            element: Alternation(
              choices: [
                RuleReference("NAME", isToken: true),
                Literal("REPLACE"),
              ],
            ),
          ),
          Literal("("),
          Group(
            element: Alternation(
              choices: [
                RuleReference("STAR", isToken: true),
                Sequence(
                  elements: [
                    Literal("DISTINCT"),
                    RuleReference("value_list", isToken: false),
                  ],
                ),
                Optional(element: RuleReference("value_list", isToken: false)),
              ],
            ),
          ),
          Literal(")"),
          Optional(element: RuleReference("filter_clause", isToken: false)),
        ],
      ),
      lineNumber: 503,
    ),
    GrammarRule(
      name: "filter_clause",
      body: Sequence(
        elements: [
          Literal("FILTER"),
          Literal("("),
          Literal("WHERE"),
          RuleReference("expr", isToken: false),
          Literal(")"),
        ],
      ),
      lineNumber: 507,
    ),
    GrammarRule(
      name: "cast_expr",
      body: Sequence(
        elements: [
          Literal("CAST"),
          Literal("("),
          RuleReference("expr", isToken: false),
          Literal("AS"),
          RuleReference("NAME", isToken: true),
          Literal(")"),
        ],
      ),
      lineNumber: 513,
    ),
    GrammarRule(
      name: "window_func_call",
      body: Sequence(
        elements: [
          RuleReference("NAME", isToken: true),
          Literal("("),
          Group(
            element: Alternation(
              choices: [
                RuleReference("STAR", isToken: true),
                Optional(element: RuleReference("value_list", isToken: false)),
              ],
            ),
          ),
          Literal(")"),
          Literal("OVER"),
          Group(
            element: Alternation(
              choices: [
                Sequence(
                  elements: [
                    Literal("("),
                    RuleReference("window_spec", isToken: false),
                    Literal(")"),
                  ],
                ),
                RuleReference("window_name_ref", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 542,
    ),
    GrammarRule(
      name: "window_name_ref",
      body: RuleReference("NAME", isToken: true),
      lineNumber: 543,
    ),
    GrammarRule(
      name: "window_spec",
      body: Sequence(
        elements: [
          Optional(element: RuleReference("partition_clause", isToken: false)),
          Optional(element: RuleReference("order_clause", isToken: false)),
          Optional(element: RuleReference("frame_clause", isToken: false)),
        ],
      ),
      lineNumber: 544,
    ),
    GrammarRule(
      name: "window_clause",
      body: Sequence(
        elements: [
          Literal("WINDOW"),
          RuleReference("NAME", isToken: true),
          Literal("AS"),
          Literal("("),
          RuleReference("window_spec", isToken: false),
          Literal(")"),
          Repetition(
            element: Sequence(
              elements: [
                Literal(","),
                RuleReference("NAME", isToken: true),
                Literal("AS"),
                Literal("("),
                RuleReference("window_spec", isToken: false),
                Literal(")"),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 545,
    ),
    GrammarRule(
      name: "partition_clause",
      body: Sequence(
        elements: [
          Literal("PARTITION"),
          Literal("BY"),
          RuleReference("expr", isToken: false),
          Repetition(
            element: Sequence(
              elements: [Literal(","), RuleReference("expr", isToken: false)],
            ),
          ),
        ],
      ),
      lineNumber: 546,
    ),
    GrammarRule(
      name: "value_list",
      body: Sequence(
        elements: [
          RuleReference("expr", isToken: false),
          Repetition(
            element: Sequence(
              elements: [Literal(","), RuleReference("expr", isToken: false)],
            ),
          ),
        ],
      ),
      lineNumber: 547,
    ),
    GrammarRule(
      name: "frame_clause",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              RuleReference("frame_unit", isToken: false),
              Literal("BETWEEN"),
              RuleReference("frame_bound", isToken: false),
              Literal("AND"),
              RuleReference("frame_bound", isToken: false),
            ],
          ),
          Sequence(
            elements: [
              RuleReference("frame_unit", isToken: false),
              RuleReference("frame_bound", isToken: false),
            ],
          ),
        ],
      ),
      lineNumber: 569,
    ),
    GrammarRule(
      name: "frame_unit",
      body: Alternation(
        choices: [Literal("ROWS"), Literal("RANGE"), Literal("GROUPS")],
      ),
      lineNumber: 571,
    ),
    GrammarRule(
      name: "frame_bound",
      body: Alternation(
        choices: [
          Sequence(elements: [Literal("UNBOUNDED"), Literal("PRECEDING")]),
          Sequence(elements: [Literal("UNBOUNDED"), Literal("FOLLOWING")]),
          Sequence(elements: [Literal("CURRENT"), Literal("ROW")]),
          Sequence(
            elements: [
              RuleReference("expr", isToken: false),
              Literal("PRECEDING"),
            ],
          ),
          Sequence(
            elements: [
              RuleReference("expr", isToken: false),
              Literal("FOLLOWING"),
            ],
          ),
        ],
      ),
      lineNumber: 572,
    ),
    GrammarRule(
      name: "create_trigger_stmt",
      body: Sequence(
        elements: [
          Literal("CREATE"),
          Literal("TRIGGER"),
          Optional(
            element: Sequence(
              elements: [Literal("IF"), Literal("NOT"), Literal("EXISTS")],
            ),
          ),
          RuleReference("NAME", isToken: true),
          Group(
            element: Alternation(
              choices: [Literal("BEFORE"), Literal("AFTER")],
            ),
          ),
          Group(
            element: Alternation(
              choices: [
                Literal("INSERT"),
                Literal("UPDATE"),
                Literal("DELETE"),
              ],
            ),
          ),
          Literal("ON"),
          RuleReference("NAME", isToken: true),
          Optional(
            element: Sequence(
              elements: [Literal("FOR"), Literal("EACH"), Literal("ROW")],
            ),
          ),
          Literal("BEGIN"),
          RuleReference("trigger_body_stmt", isToken: false),
          Literal(";"),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("trigger_body_stmt", isToken: false),
                Literal(";"),
              ],
            ),
          ),
          Literal("END"),
        ],
      ),
      lineNumber: 598,
    ),
    GrammarRule(
      name: "trigger_body_stmt",
      body: Alternation(
        choices: [
          RuleReference("insert_stmt", isToken: false),
          RuleReference("replace_stmt", isToken: false),
          RuleReference("update_stmt", isToken: false),
          RuleReference("delete_stmt", isToken: false),
          RuleReference("query_stmt", isToken: false),
        ],
      ),
      lineNumber: 603,
    ),
    GrammarRule(
      name: "drop_trigger_stmt",
      body: Sequence(
        elements: [
          Literal("DROP"),
          Literal("TRIGGER"),
          Optional(
            element: Sequence(elements: [Literal("IF"), Literal("EXISTS")]),
          ),
          RuleReference("NAME", isToken: true),
        ],
      ),
      lineNumber: 605,
    ),
    GrammarRule(
      name: "case_expr",
      body: Sequence(
        elements: [
          Literal("CASE"),
          Optional(element: RuleReference("case_operand", isToken: false)),
          RuleReference("case_when", isToken: false),
          Repetition(element: RuleReference("case_when", isToken: false)),
          Optional(
            element: Sequence(
              elements: [
                Literal("ELSE"),
                RuleReference("expr", isToken: false),
              ],
            ),
          ),
          Literal("END"),
        ],
      ),
      lineNumber: 620,
    ),
    GrammarRule(
      name: "case_operand",
      body: RuleReference("expr", isToken: false),
      lineNumber: 621,
    ),
    GrammarRule(
      name: "case_when",
      body: Sequence(
        elements: [
          Literal("WHEN"),
          RuleReference("expr", isToken: false),
          Literal("THEN"),
          RuleReference("expr", isToken: false),
        ],
      ),
      lineNumber: 622,
    ),
  ],
);
