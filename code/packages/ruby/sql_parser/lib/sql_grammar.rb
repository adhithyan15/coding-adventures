# frozen_string_literal: true
# AUTO-GENERATED FILE - DO NOT EDIT
require "coding_adventures_grammar_tools"

module CodingAdventures
  module SqlGrammar
    def self.grammar
      @grammar ||= CodingAdventures::GrammarTools::ParserGrammar.new(
        version: 1,
        rules: [
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "program",
            line_number: 10,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "statement", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: ";"), CodingAdventures::GrammarTools::RuleReference.new(name: "statement", is_token: false)])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Literal.new(value: ";"))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "statement",
            line_number: 12,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "select_stmt", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "insert_stmt", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "update_stmt", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "delete_stmt", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "create_table_stmt", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "drop_table_stmt", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "select_stmt",
            line_number: 17,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "SELECT"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Literal.new(value: "DISTINCT"), CodingAdventures::GrammarTools::Literal.new(value: "ALL")])), CodingAdventures::GrammarTools::RuleReference.new(name: "select_list", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: "FROM"), CodingAdventures::GrammarTools::RuleReference.new(name: "table_ref", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "join_clause", is_token: false)), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "where_clause", is_token: false)), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "group_clause", is_token: false)), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "having_clause", is_token: false)), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "order_clause", is_token: false)), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "limit_clause", is_token: false))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "select_list",
            line_number: 22,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "STAR", is_token: true), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "select_item", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: ","), CodingAdventures::GrammarTools::RuleReference.new(name: "select_item", is_token: false)]))])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "select_item",
            line_number: 23,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "expr", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "AS"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "table_ref",
            line_number: 25,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "table_name", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "AS"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "table_name",
            line_number: 26,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "."), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "join_clause",
            line_number: 28,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "join_type", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: "JOIN"), CodingAdventures::GrammarTools::RuleReference.new(name: "table_ref", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: "ON"), CodingAdventures::GrammarTools::RuleReference.new(name: "expr", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "join_type",
            line_number: 29,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Literal.new(value: "CROSS"), CodingAdventures::GrammarTools::Literal.new(value: "INNER"), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "LEFT"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Literal.new(value: "OUTER"))])), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "RIGHT"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Literal.new(value: "OUTER"))])), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "FULL"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Literal.new(value: "OUTER"))]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "where_clause",
            line_number: 32,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "WHERE"), CodingAdventures::GrammarTools::RuleReference.new(name: "expr", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "group_clause",
            line_number: 33,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "GROUP"), CodingAdventures::GrammarTools::Literal.new(value: "BY"), CodingAdventures::GrammarTools::RuleReference.new(name: "column_ref", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: ","), CodingAdventures::GrammarTools::RuleReference.new(name: "column_ref", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "having_clause",
            line_number: 34,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "HAVING"), CodingAdventures::GrammarTools::RuleReference.new(name: "expr", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "order_clause",
            line_number: 35,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "ORDER"), CodingAdventures::GrammarTools::Literal.new(value: "BY"), CodingAdventures::GrammarTools::RuleReference.new(name: "order_item", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: ","), CodingAdventures::GrammarTools::RuleReference.new(name: "order_item", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "order_item",
            line_number: 36,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "expr", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Literal.new(value: "ASC"), CodingAdventures::GrammarTools::Literal.new(value: "DESC")]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "limit_clause",
            line_number: 37,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "LIMIT"), CodingAdventures::GrammarTools::RuleReference.new(name: "NUMBER", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "OFFSET"), CodingAdventures::GrammarTools::RuleReference.new(name: "NUMBER", is_token: true)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "insert_stmt",
            line_number: 41,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "INSERT"), CodingAdventures::GrammarTools::Literal.new(value: "INTO"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "("), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: ","), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)])), CodingAdventures::GrammarTools::Literal.new(value: ")")])), CodingAdventures::GrammarTools::Literal.new(value: "VALUES"), CodingAdventures::GrammarTools::RuleReference.new(name: "row_value", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: ","), CodingAdventures::GrammarTools::RuleReference.new(name: "row_value", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "row_value",
            line_number: 44,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "("), CodingAdventures::GrammarTools::RuleReference.new(name: "expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: ","), CodingAdventures::GrammarTools::RuleReference.new(name: "expr", is_token: false)])), CodingAdventures::GrammarTools::Literal.new(value: ")")])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "update_stmt",
            line_number: 46,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "UPDATE"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "SET"), CodingAdventures::GrammarTools::RuleReference.new(name: "assignment", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: ","), CodingAdventures::GrammarTools::RuleReference.new(name: "assignment", is_token: false)])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "where_clause", is_token: false))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "assignment",
            line_number: 48,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "="), CodingAdventures::GrammarTools::RuleReference.new(name: "expr", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "delete_stmt",
            line_number: 50,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "DELETE"), CodingAdventures::GrammarTools::Literal.new(value: "FROM"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "where_clause", is_token: false))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "create_table_stmt",
            line_number: 54,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "CREATE"), CodingAdventures::GrammarTools::Literal.new(value: "TABLE"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "IF"), CodingAdventures::GrammarTools::Literal.new(value: "NOT"), CodingAdventures::GrammarTools::Literal.new(value: "EXISTS")])), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "("), CodingAdventures::GrammarTools::RuleReference.new(name: "col_def", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: ","), CodingAdventures::GrammarTools::RuleReference.new(name: "col_def", is_token: false)])), CodingAdventures::GrammarTools::Literal.new(value: ")")])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "col_def",
            line_number: 56,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "col_constraint", is_token: false))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "col_constraint",
            line_number: 57,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "NOT"), CodingAdventures::GrammarTools::Literal.new(value: "NULL")])), CodingAdventures::GrammarTools::Literal.new(value: "NULL"), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "PRIMARY"), CodingAdventures::GrammarTools::Literal.new(value: "KEY")])), CodingAdventures::GrammarTools::Literal.new(value: "UNIQUE"), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "DEFAULT"), CodingAdventures::GrammarTools::RuleReference.new(name: "primary", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "drop_table_stmt",
            line_number: 60,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "DROP"), CodingAdventures::GrammarTools::Literal.new(value: "TABLE"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "IF"), CodingAdventures::GrammarTools::Literal.new(value: "EXISTS")])), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "expr",
            line_number: 64,
            body: CodingAdventures::GrammarTools::RuleReference.new(name: "or_expr", is_token: false)
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "or_expr",
            line_number: 65,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "and_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "OR"), CodingAdventures::GrammarTools::RuleReference.new(name: "and_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "and_expr",
            line_number: 66,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "not_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "AND"), CodingAdventures::GrammarTools::RuleReference.new(name: "not_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "not_expr",
            line_number: 67,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "NOT"), CodingAdventures::GrammarTools::RuleReference.new(name: "not_expr", is_token: false)]), CodingAdventures::GrammarTools::RuleReference.new(name: "comparison", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "comparison",
            line_number: 68,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "additive", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "cmp_op", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "additive", is_token: false)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "BETWEEN"), CodingAdventures::GrammarTools::RuleReference.new(name: "additive", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: "AND"), CodingAdventures::GrammarTools::RuleReference.new(name: "additive", is_token: false)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "NOT"), CodingAdventures::GrammarTools::Literal.new(value: "BETWEEN"), CodingAdventures::GrammarTools::RuleReference.new(name: "additive", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: "AND"), CodingAdventures::GrammarTools::RuleReference.new(name: "additive", is_token: false)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "IN"), CodingAdventures::GrammarTools::Literal.new(value: "("), CodingAdventures::GrammarTools::RuleReference.new(name: "value_list", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: ")")]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "NOT"), CodingAdventures::GrammarTools::Literal.new(value: "IN"), CodingAdventures::GrammarTools::Literal.new(value: "("), CodingAdventures::GrammarTools::RuleReference.new(name: "value_list", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: ")")]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "LIKE"), CodingAdventures::GrammarTools::RuleReference.new(name: "additive", is_token: false)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "NOT"), CodingAdventures::GrammarTools::Literal.new(value: "LIKE"), CodingAdventures::GrammarTools::RuleReference.new(name: "additive", is_token: false)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "IS"), CodingAdventures::GrammarTools::Literal.new(value: "NULL")]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "IS"), CodingAdventures::GrammarTools::Literal.new(value: "NOT"), CodingAdventures::GrammarTools::Literal.new(value: "NULL")])]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "cmp_op",
            line_number: 78,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Literal.new(value: "="), CodingAdventures::GrammarTools::RuleReference.new(name: "NOT_EQUALS", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "<"), CodingAdventures::GrammarTools::Literal.new(value: ">"), CodingAdventures::GrammarTools::Literal.new(value: "<="), CodingAdventures::GrammarTools::Literal.new(value: ">=")])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "additive",
            line_number: 79,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "multiplicative", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Literal.new(value: "+"), CodingAdventures::GrammarTools::Literal.new(value: "-")])), CodingAdventures::GrammarTools::RuleReference.new(name: "multiplicative", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "multiplicative",
            line_number: 80,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "unary", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "STAR", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "/"), CodingAdventures::GrammarTools::Literal.new(value: "%")])), CodingAdventures::GrammarTools::RuleReference.new(name: "unary", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "unary",
            line_number: 81,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "-"), CodingAdventures::GrammarTools::RuleReference.new(name: "unary", is_token: false)]), CodingAdventures::GrammarTools::RuleReference.new(name: "primary", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "primary",
            line_number: 82,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "NUMBER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "NULL"), CodingAdventures::GrammarTools::Literal.new(value: "TRUE"), CodingAdventures::GrammarTools::Literal.new(value: "FALSE"), CodingAdventures::GrammarTools::RuleReference.new(name: "function_call", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "column_ref", is_token: false), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "("), CodingAdventures::GrammarTools::RuleReference.new(name: "expr", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: ")")])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "column_ref",
            line_number: 85,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "."), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "function_call",
            line_number: 86,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "("), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "STAR", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "value_list", is_token: false))])), CodingAdventures::GrammarTools::Literal.new(value: ")")])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "value_list",
            line_number: 87,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: ","), CodingAdventures::GrammarTools::RuleReference.new(name: "expr", is_token: false)]))])
          ),
        ]
      )
    end
  end
end
