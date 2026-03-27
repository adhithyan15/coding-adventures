# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: excel.grammar
# Regenerate with: grammar-tools compile-grammar excel.grammar
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
      name: "formula",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "ws", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "ws", is_token: false),
          ])),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "ws", is_token: false),
      ]),
      line_number: 15,
    ),
    GT::GrammarRule.new(
      name: "ws",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "SPACE", is_token: true)),
      line_number: 17,
    ),
    GT::GrammarRule.new(
      name: "req_space",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "SPACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "SPACE", is_token: true)),
      ]),
      line_number: 18,
    ),
    GT::GrammarRule.new(
      name: "expression",
      body: GT::RuleReference.new(name: "comparison_expr", is_token: false),
      line_number: 20,
    ),
    GT::GrammarRule.new(
      name: "comparison_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "concat_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "comparison_op", is_token: false),
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "concat_expr", is_token: false),
          ])),
      ]),
      line_number: 22,
    ),
    GT::GrammarRule.new(
      name: "comparison_op",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "NOT_EQUALS", is_token: true),
        GT::RuleReference.new(name: "LESS_THAN", is_token: true),
        GT::RuleReference.new(name: "LESS_EQUALS", is_token: true),
        GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
        GT::RuleReference.new(name: "GREATER_EQUALS", is_token: true),
      ]),
      line_number: 23,
    ),
    GT::GrammarRule.new(
      name: "concat_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "additive_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "AMP", is_token: true),
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "additive_expr", is_token: false),
          ])),
      ]),
      line_number: 26,
    ),
    GT::GrammarRule.new(
      name: "additive_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "multiplicative_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "PLUS", is_token: true),
                GT::RuleReference.new(name: "MINUS", is_token: true),
              ])),
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "multiplicative_expr", is_token: false),
          ])),
      ]),
      line_number: 27,
    ),
    GT::GrammarRule.new(
      name: "multiplicative_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "power_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "STAR", is_token: true),
                GT::RuleReference.new(name: "SLASH", is_token: true),
              ])),
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "power_expr", is_token: false),
          ])),
      ]),
      line_number: 28,
    ),
    GT::GrammarRule.new(
      name: "power_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "unary_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "CARET", is_token: true),
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "unary_expr", is_token: false),
          ])),
      ]),
      line_number: 29,
    ),
    GT::GrammarRule.new(
      name: "unary_expr",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "prefix_op", is_token: false),
            GT::RuleReference.new(name: "ws", is_token: false),
          ])),
        GT::RuleReference.new(name: "postfix_expr", is_token: false),
      ]),
      line_number: 30,
    ),
    GT::GrammarRule.new(
      name: "prefix_op",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "PLUS", is_token: true),
        GT::RuleReference.new(name: "MINUS", is_token: true),
      ]),
      line_number: 31,
    ),
    GT::GrammarRule.new(
      name: "postfix_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "primary", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "PERCENT", is_token: true),
          ])),
      ]),
      line_number: 32,
    ),
    GT::GrammarRule.new(
      name: "primary",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "parenthesized_expression", is_token: false),
        GT::RuleReference.new(name: "constant", is_token: false),
        GT::RuleReference.new(name: "function_call", is_token: false),
        GT::RuleReference.new(name: "structure_reference", is_token: false),
        GT::RuleReference.new(name: "reference_expression", is_token: false),
        GT::RuleReference.new(name: "bang_reference", is_token: false),
        GT::RuleReference.new(name: "bang_name", is_token: false),
        GT::RuleReference.new(name: "name_reference", is_token: false),
      ]),
      line_number: 34,
    ),
    GT::GrammarRule.new(
      name: "parenthesized_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "ws", is_token: false),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "ws", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 43,
    ),
    GT::GrammarRule.new(
      name: "constant",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "KEYWORD", is_token: true),
        GT::RuleReference.new(name: "ERROR_CONSTANT", is_token: true),
        GT::RuleReference.new(name: "array_constant", is_token: false),
      ]),
      line_number: 45,
    ),
    GT::GrammarRule.new(
      name: "array_constant",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "ws", is_token: false),
        GT::RuleReference.new(name: "array_row", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "array_row", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          ])),
        GT::RuleReference.new(name: "ws", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 47,
    ),
    GT::GrammarRule.new(
      name: "array_row",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "array_item", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "array_item", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "COMMA", is_token: true),
          ])),
      ]),
      line_number: 48,
    ),
    GT::GrammarRule.new(
      name: "array_item",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "KEYWORD", is_token: true),
        GT::RuleReference.new(name: "ERROR_CONSTANT", is_token: true),
      ]),
      line_number: 49,
    ),
    GT::GrammarRule.new(
      name: "function_call",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "function_name", is_token: false),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "ws", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "function_argument_list", is_token: false)),
        GT::RuleReference.new(name: "ws", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 51,
    ),
    GT::GrammarRule.new(
      name: "function_name",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "FUNCTION_NAME", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 52,
    ),
    GT::GrammarRule.new(
      name: "function_argument_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "function_argument", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "function_argument", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "COMMA", is_token: true),
          ])),
      ]),
      line_number: 53,
    ),
    GT::GrammarRule.new(
      name: "function_argument",
      body: GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
      line_number: 54,
    ),
    GT::GrammarRule.new(
      name: "reference_expression",
      body: GT::RuleReference.new(name: "union_reference", is_token: false),
      line_number: 56,
    ),
    GT::GrammarRule.new(
      name: "union_reference",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "intersection_reference", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "intersection_reference", is_token: false),
          ])),
      ]),
      line_number: 57,
    ),
    GT::GrammarRule.new(
      name: "intersection_reference",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "range_reference", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "req_space", is_token: false),
            GT::RuleReference.new(name: "range_reference", is_token: false),
          ])),
      ]),
      line_number: 58,
    ),
    GT::GrammarRule.new(
      name: "range_reference",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "reference_primary", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "reference_primary", is_token: false),
          ])),
      ]),
      line_number: 59,
    ),
    GT::GrammarRule.new(
      name: "reference_primary",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "parenthesized_reference", is_token: false),
        GT::RuleReference.new(name: "prefixed_reference", is_token: false),
        GT::RuleReference.new(name: "external_reference", is_token: false),
        GT::RuleReference.new(name: "structure_reference", is_token: false),
        GT::RuleReference.new(name: "a1_reference", is_token: false),
        GT::RuleReference.new(name: "bang_reference", is_token: false),
        GT::RuleReference.new(name: "bang_name", is_token: false),
        GT::RuleReference.new(name: "name_reference", is_token: false),
      ]),
      line_number: 61,
    ),
    GT::GrammarRule.new(
      name: "parenthesized_reference",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "ws", is_token: false),
        GT::RuleReference.new(name: "reference_expression", is_token: false),
        GT::RuleReference.new(name: "ws", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 70,
    ),
    GT::GrammarRule.new(
      name: "prefixed_reference",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "REF_PREFIX", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "a1_reference", is_token: false),
            GT::RuleReference.new(name: "name_reference", is_token: false),
            GT::RuleReference.new(name: "structure_reference", is_token: false),
          ])),
      ]),
      line_number: 71,
    ),
    GT::GrammarRule.new(
      name: "external_reference",
      body: GT::RuleReference.new(name: "REF_PREFIX", is_token: true),
      line_number: 72,
    ),
    GT::GrammarRule.new(
      name: "bang_reference",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "BANG", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "CELL", is_token: true),
            GT::RuleReference.new(name: "COLUMN_REF", is_token: true),
            GT::RuleReference.new(name: "ROW_REF", is_token: true),
            GT::RuleReference.new(name: "NUMBER", is_token: true),
          ])),
      ]),
      line_number: 73,
    ),
    GT::GrammarRule.new(
      name: "bang_name",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "BANG", is_token: true),
        GT::RuleReference.new(name: "name_reference", is_token: false),
      ]),
      line_number: 74,
    ),
    GT::GrammarRule.new(
      name: "name_reference",
      body: GT::RuleReference.new(name: "NAME", is_token: true),
      line_number: 75,
    ),
    GT::GrammarRule.new(
      name: "column_reference",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "DOLLAR", is_token: true)),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "COLUMN_REF", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 77,
    ),
    GT::GrammarRule.new(
      name: "row_reference",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "DOLLAR", is_token: true)),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "ROW_REF", is_token: true),
            GT::RuleReference.new(name: "NUMBER", is_token: true),
          ])),
      ]),
      line_number: 78,
    ),
    GT::GrammarRule.new(
      name: "a1_reference",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "CELL", is_token: true),
        GT::RuleReference.new(name: "column_reference", is_token: false),
        GT::RuleReference.new(name: "row_reference", is_token: false),
        GT::RuleReference.new(name: "COLUMN_REF", is_token: true),
        GT::RuleReference.new(name: "ROW_REF", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
      ]),
      line_number: 80,
    ),
    GT::GrammarRule.new(
      name: "structure_reference",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "table_name", is_token: false)),
        GT::RuleReference.new(name: "intra_table_reference", is_token: false),
      ]),
      line_number: 82,
    ),
    GT::GrammarRule.new(
      name: "table_name",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "TABLE_NAME", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 83,
    ),
    GT::GrammarRule.new(
      name: "intra_table_reference",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "STRUCTURED_KEYWORD", is_token: true),
        GT::RuleReference.new(name: "structured_column_range", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::RuleReference.new(name: "ws", is_token: false),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "inner_structure_reference", is_token: false)),
          GT::RuleReference.new(name: "ws", is_token: false),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
      ]),
      line_number: 84,
    ),
    GT::GrammarRule.new(
      name: "inner_structure_reference",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "structured_keyword_list", is_token: false),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "ws", is_token: false),
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "ws", is_token: false),
              GT::RuleReference.new(name: "structured_column_range", is_token: false),
            ])),
        ]),
        GT::RuleReference.new(name: "structured_column_range", is_token: false),
      ]),
      line_number: 87,
    ),
    GT::GrammarRule.new(
      name: "structured_keyword_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "STRUCTURED_KEYWORD", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "STRUCTURED_KEYWORD", is_token: true),
          ])),
      ]),
      line_number: 89,
    ),
    GT::GrammarRule.new(
      name: "structured_column_range",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "structured_column", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "ws", is_token: false),
            GT::RuleReference.new(name: "structured_column", is_token: false),
          ])),
      ]),
      line_number: 90,
    ),
    GT::GrammarRule.new(
      name: "structured_column",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "STRUCTURED_COLUMN", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "AT", is_token: true),
          GT::RuleReference.new(name: "STRUCTURED_COLUMN", is_token: true),
        ]),
      ]),
      line_number: 91,
    ),
  ],
)
