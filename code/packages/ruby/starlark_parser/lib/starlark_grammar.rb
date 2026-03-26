# frozen_string_literal: true
# AUTO-GENERATED FILE - DO NOT EDIT
require "coding_adventures_grammar_tools"

module CodingAdventures
  module StarlarkGrammar
    def self.grammar
      @grammar ||= CodingAdventures::GrammarTools::ParserGrammar.new(
        version: 1,
        rules: [
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "file",
            line_number: 34,
            body: CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "NEWLINE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "statement", is_token: false)]))
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "statement",
            line_number: 48,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "compound_stmt", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "simple_stmt", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "simple_stmt",
            line_number: 52,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "small_stmt", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "small_stmt", is_token: false)])), CodingAdventures::GrammarTools::RuleReference.new(name: "NEWLINE", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "small_stmt",
            line_number: 54,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "return_stmt", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "break_stmt", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "continue_stmt", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "pass_stmt", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "load_stmt", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "assign_stmt", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "return_stmt",
            line_number: 68,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "return"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "break_stmt",
            line_number: 71,
            body: CodingAdventures::GrammarTools::Literal.new(value: "break")
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "continue_stmt",
            line_number: 74,
            body: CodingAdventures::GrammarTools::Literal.new(value: "continue")
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "pass_stmt",
            line_number: 79,
            body: CodingAdventures::GrammarTools::Literal.new(value: "pass")
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "load_stmt",
            line_number: 88,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "load"), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "load_arg", is_token: false)])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true)), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "load_arg",
            line_number: 89,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true)]), CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "assign_stmt",
            line_number: 110,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "expression_list", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "assign_op", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "augmented_assign_op", is_token: false)])), CodingAdventures::GrammarTools::RuleReference.new(name: "expression_list", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "assign_op",
            line_number: 113,
            body: CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS", is_token: true)
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "augmented_assign_op",
            line_number: 115,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "PLUS_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "MINUS_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STAR_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "SLASH_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "FLOOR_DIV_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "PERCENT_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "AMP_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "PIPE_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "CARET_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LEFT_SHIFT_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "RIGHT_SHIFT_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "DOUBLE_STAR_EQUALS", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "compound_stmt",
            line_number: 124,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "if_stmt", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "for_stmt", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "def_stmt", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "if_stmt",
            line_number: 136,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "if"), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "suite", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "elif"), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "suite", is_token: false)])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "else"), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "suite", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "for_stmt",
            line_number: 150,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "for"), CodingAdventures::GrammarTools::RuleReference.new(name: "loop_vars", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: "in"), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "suite", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "loop_vars",
            line_number: 156,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "def_stmt",
            line_number: 166,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "def"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "parameters", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "suite", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "suite",
            line_number: 177,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "simple_stmt", is_token: false), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NEWLINE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "INDENT", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "statement", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "DEDENT", is_token: true)])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "parameters",
            line_number: 198,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "parameter", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "parameter", is_token: false)])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "parameter",
            line_number: 200,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "DOUBLE_STAR", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "STAR", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)]), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "expression_list",
            line_number: 234,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "expression",
            line_number: 239,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "lambda_expr", is_token: false), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "or_expr", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "if"), CodingAdventures::GrammarTools::RuleReference.new(name: "or_expr", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: "else"), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)]))])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "lambda_expr",
            line_number: 244,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "lambda"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "lambda_params", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "lambda_params",
            line_number: 245,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "lambda_param", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "lambda_param", is_token: false)])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "lambda_param",
            line_number: 246,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)]))]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "STAR", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "DOUBLE_STAR", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "or_expr",
            line_number: 250,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "and_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "or"), CodingAdventures::GrammarTools::RuleReference.new(name: "and_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "and_expr",
            line_number: 254,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "not_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "and"), CodingAdventures::GrammarTools::RuleReference.new(name: "not_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "not_expr",
            line_number: 258,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "not"), CodingAdventures::GrammarTools::RuleReference.new(name: "not_expr", is_token: false)]), CodingAdventures::GrammarTools::RuleReference.new(name: "comparison", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "comparison",
            line_number: 267,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "bitwise_or", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "comp_op", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "bitwise_or", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "comp_op",
            line_number: 269,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NOT_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LESS_THAN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "GREATER_THAN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LESS_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "GREATER_EQUALS", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "in"), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "not"), CodingAdventures::GrammarTools::Literal.new(value: "in")])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "bitwise_or",
            line_number: 275,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "bitwise_xor", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "PIPE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "bitwise_xor", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "bitwise_xor",
            line_number: 276,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "bitwise_and", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "CARET", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "bitwise_and", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "bitwise_and",
            line_number: 277,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "shift", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "AMP", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "shift", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "shift",
            line_number: 280,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "arith", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "LEFT_SHIFT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "RIGHT_SHIFT", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "arith", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "arith",
            line_number: 284,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "term", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "PLUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "MINUS", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "term", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "term",
            line_number: 289,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "factor", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "STAR", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "SLASH", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "FLOOR_DIV", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "PERCENT", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "factor", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "factor",
            line_number: 295,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "PLUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "MINUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "TILDE", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "factor", is_token: false)]), CodingAdventures::GrammarTools::RuleReference.new(name: "power", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "power",
            line_number: 303,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "primary", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "DOUBLE_STAR", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "factor", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "primary",
            line_number: 320,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "atom", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "suffix", is_token: false))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "suffix",
            line_number: 322,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "DOT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACKET", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "subscript", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACKET", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "arguments", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "subscript",
            line_number: 334,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false))]))])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "atom",
            line_number: 343,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "INT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "FLOAT", is_token: true), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true))]), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "True"), CodingAdventures::GrammarTools::Literal.new(value: "False"), CodingAdventures::GrammarTools::Literal.new(value: "None"), CodingAdventures::GrammarTools::RuleReference.new(name: "list_expr", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "dict_expr", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "paren_expr", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "list_expr",
            line_number: 359,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACKET", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "list_body", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACKET", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "list_body",
            line_number: 361,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "comp_clause", is_token: false)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true))])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "dict_expr",
            line_number: 367,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACE", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "dict_body", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACE", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "dict_body",
            line_number: 369,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "dict_entry", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "comp_clause", is_token: false)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "dict_entry", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "dict_entry", is_token: false)])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true))])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "dict_entry",
            line_number: 372,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "paren_expr",
            line_number: 379,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "paren_body", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "paren_body",
            line_number: 381,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "comp_clause", is_token: false)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true))]))]), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "comp_clause",
            line_number: 397,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "comp_for", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "comp_for", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "comp_if", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "comp_for",
            line_number: 399,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "for"), CodingAdventures::GrammarTools::RuleReference.new(name: "loop_vars", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: "in"), CodingAdventures::GrammarTools::RuleReference.new(name: "or_expr", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "comp_if",
            line_number: 401,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "if"), CodingAdventures::GrammarTools::RuleReference.new(name: "or_expr", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "arguments",
            line_number: 420,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "argument", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "argument", is_token: false)])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "argument",
            line_number: 422,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "DOUBLE_STAR", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "STAR", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)]), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])
          ),
        ]
      )
    end
  end
end
