# frozen_string_literal: true
# AUTO-GENERATED FILE - DO NOT EDIT
require "coding_adventures_grammar_tools"

module CodingAdventures
  module ExcelGrammar
    def self.grammar
      @grammar ||= CodingAdventures::GrammarTools::ParserGrammar.new(
        version: 1,
        rules: [
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "formula",
            line_number: 15,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false)])), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "ws",
            line_number: 17,
            body: CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "SPACE", is_token: true))
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "req_space",
            line_number: 18,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "SPACE", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "SPACE", is_token: true))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "expression",
            line_number: 20,
            body: CodingAdventures::GrammarTools::RuleReference.new(name: "comparison_expr", is_token: false)
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "comparison_expr",
            line_number: 22,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "concat_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "comparison_op", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "concat_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "comparison_op",
            line_number: 23,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NOT_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LESS_THAN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LESS_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "GREATER_THAN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "GREATER_EQUALS", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "concat_expr",
            line_number: 26,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "additive_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "AMP", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "additive_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "additive_expr",
            line_number: 27,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "multiplicative_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "PLUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "MINUS", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "multiplicative_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "multiplicative_expr",
            line_number: 28,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "power_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "STAR", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "SLASH", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "power_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "power_expr",
            line_number: 29,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "unary_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "CARET", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "unary_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "unary_expr",
            line_number: 30,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "prefix_op", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false)])), CodingAdventures::GrammarTools::RuleReference.new(name: "postfix_expr", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "prefix_op",
            line_number: 31,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "PLUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "MINUS", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "postfix_expr",
            line_number: 32,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "primary", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "PERCENT", is_token: true)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "primary",
            line_number: 34,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "parenthesized_expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "constant", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "function_call", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "structure_reference", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "reference_expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "bang_reference", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "bang_name", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "name_reference", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "parenthesized_expression",
            line_number: 43,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "constant",
            line_number: 45,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "NUMBER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "KEYWORD", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ERROR_CONSTANT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "array_constant", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "array_constant",
            line_number: 47,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "array_row", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "array_row", is_token: false)])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACE", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "array_row",
            line_number: 48,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "array_item", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "array_item", is_token: false)])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "array_item",
            line_number: 49,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "NUMBER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "KEYWORD", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ERROR_CONSTANT", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "function_call",
            line_number: 51,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "function_name", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "function_argument_list", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "function_name",
            line_number: 52,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "FUNCTION_NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "function_argument_list",
            line_number: 53,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "function_argument", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "function_argument", is_token: false)])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "function_argument",
            line_number: 54,
            body: CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false))
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "reference_expression",
            line_number: 56,
            body: CodingAdventures::GrammarTools::RuleReference.new(name: "union_reference", is_token: false)
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "union_reference",
            line_number: 57,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "intersection_reference", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "intersection_reference", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "intersection_reference",
            line_number: 58,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "range_reference", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "req_space", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "range_reference", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "range_reference",
            line_number: 59,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "reference_primary", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "reference_primary", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "reference_primary",
            line_number: 61,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "parenthesized_reference", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "prefixed_reference", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "external_reference", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "structure_reference", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "a1_reference", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "bang_reference", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "bang_name", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "name_reference", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "parenthesized_reference",
            line_number: 70,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "reference_expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "prefixed_reference",
            line_number: 71,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "REF_PREFIX", is_token: true), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "a1_reference", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "name_reference", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "structure_reference", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "external_reference",
            line_number: 72,
            body: CodingAdventures::GrammarTools::RuleReference.new(name: "REF_PREFIX", is_token: true)
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "bang_reference",
            line_number: 73,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "BANG", is_token: true), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "CELL", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "COLUMN_REF", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ROW_REF", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NUMBER", is_token: true)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "bang_name",
            line_number: 74,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "BANG", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "name_reference", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "name_reference",
            line_number: 75,
            body: CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "column_reference",
            line_number: 77,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "DOLLAR", is_token: true)), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "COLUMN_REF", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "row_reference",
            line_number: 78,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "DOLLAR", is_token: true)), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "ROW_REF", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NUMBER", is_token: true)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "a1_reference",
            line_number: 80,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "CELL", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "column_reference", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "row_reference", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COLUMN_REF", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ROW_REF", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NUMBER", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "structure_reference",
            line_number: 82,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "table_name", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "intra_table_reference", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "table_name",
            line_number: 83,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "TABLE_NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "intra_table_reference",
            line_number: 84,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "STRUCTURED_KEYWORD", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "structured_column_range", is_token: false), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACKET", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "inner_structure_reference", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACKET", is_token: true)])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "inner_structure_reference",
            line_number: 87,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "structured_keyword_list", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "structured_column_range", is_token: false)]))]), CodingAdventures::GrammarTools::RuleReference.new(name: "structured_column_range", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "structured_keyword_list",
            line_number: 89,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "STRUCTURED_KEYWORD", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "STRUCTURED_KEYWORD", is_token: true)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "structured_column_range",
            line_number: 90,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "structured_column", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ws", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "structured_column", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "structured_column",
            line_number: 91,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "STRUCTURED_COLUMN", is_token: true), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "AT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STRUCTURED_COLUMN", is_token: true)])])
          ),
        ]
      )
    end
  end
end
