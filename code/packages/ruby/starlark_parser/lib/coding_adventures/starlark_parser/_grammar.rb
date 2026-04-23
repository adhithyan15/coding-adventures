# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: starlark.grammar
# Regenerate with: grammar-tools compile-grammar starlark.grammar
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
      name: "file",
      body: GT::Repetition.new(element: GT::Alternation.new(choices: [
          GT::RuleReference.new(name: "NEWLINE", is_token: true),
          GT::RuleReference.new(name: "statement", is_token: false),
        ])),
      line_number: 48,
    ),
    GT::GrammarRule.new(
      name: "statement",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "compound_stmt", is_token: false),
        GT::RuleReference.new(name: "simple_stmt", is_token: false),
      ]),
      line_number: 62,
    ),
    GT::GrammarRule.new(
      name: "simple_stmt",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "small_stmt", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            GT::RuleReference.new(name: "small_stmt", is_token: false),
          ])),
        GT::RuleReference.new(name: "NEWLINE", is_token: true),
      ]),
      line_number: 66,
    ),
    GT::GrammarRule.new(
      name: "small_stmt",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "return_stmt", is_token: false),
        GT::RuleReference.new(name: "break_stmt", is_token: false),
        GT::RuleReference.new(name: "continue_stmt", is_token: false),
        GT::RuleReference.new(name: "pass_stmt", is_token: false),
        GT::RuleReference.new(name: "load_stmt", is_token: false),
        GT::RuleReference.new(name: "assign_stmt", is_token: false),
      ]),
      line_number: 68,
    ),
    GT::GrammarRule.new(
      name: "return_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "return"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
      ]),
      line_number: 82,
    ),
    GT::GrammarRule.new(
      name: "break_stmt",
      body: GT::Literal.new(value: "break"),
      line_number: 85,
    ),
    GT::GrammarRule.new(
      name: "continue_stmt",
      body: GT::Literal.new(value: "continue"),
      line_number: 88,
    ),
    GT::GrammarRule.new(
      name: "pass_stmt",
      body: GT::Literal.new(value: "pass"),
      line_number: 93,
    ),
    GT::GrammarRule.new(
      name: "load_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "load"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "load_arg", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 102,
    ),
    GT::GrammarRule.new(
      name: "load_arg",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "EQUALS", is_token: true),
          GT::RuleReference.new(name: "STRING", is_token: true),
        ]),
        GT::RuleReference.new(name: "STRING", is_token: true),
      ]),
      line_number: 103,
    ),
    GT::GrammarRule.new(
      name: "assign_stmt",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression_list", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "assign_op", is_token: false),
                GT::RuleReference.new(name: "augmented_assign_op", is_token: false),
              ])),
            GT::RuleReference.new(name: "expression_list", is_token: false),
          ])),
      ]),
      line_number: 124,
    ),
    GT::GrammarRule.new(
      name: "assign_op",
      body: GT::RuleReference.new(name: "EQUALS", is_token: true),
      line_number: 127,
    ),
    GT::GrammarRule.new(
      name: "augmented_assign_op",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "PLUS_EQUALS", is_token: true),
        GT::RuleReference.new(name: "MINUS_EQUALS", is_token: true),
        GT::RuleReference.new(name: "STAR_EQUALS", is_token: true),
        GT::RuleReference.new(name: "SLASH_EQUALS", is_token: true),
        GT::RuleReference.new(name: "FLOOR_DIV_EQUALS", is_token: true),
        GT::RuleReference.new(name: "PERCENT_EQUALS", is_token: true),
        GT::RuleReference.new(name: "AMP_EQUALS", is_token: true),
        GT::RuleReference.new(name: "PIPE_EQUALS", is_token: true),
        GT::RuleReference.new(name: "CARET_EQUALS", is_token: true),
        GT::RuleReference.new(name: "LEFT_SHIFT_EQUALS", is_token: true),
        GT::RuleReference.new(name: "RIGHT_SHIFT_EQUALS", is_token: true),
        GT::RuleReference.new(name: "DOUBLE_STAR_EQUALS", is_token: true),
      ]),
      line_number: 129,
    ),
    GT::GrammarRule.new(
      name: "compound_stmt",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "if_stmt", is_token: false),
        GT::RuleReference.new(name: "for_stmt", is_token: false),
        GT::RuleReference.new(name: "def_stmt", is_token: false),
      ]),
      line_number: 138,
    ),
    GT::GrammarRule.new(
      name: "if_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "if"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "suite", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "elif"),
            GT::RuleReference.new(name: "expression", is_token: false),
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "suite", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "else"),
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "suite", is_token: false),
          ])),
      ]),
      line_number: 150,
    ),
    GT::GrammarRule.new(
      name: "for_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "for"),
        GT::RuleReference.new(name: "loop_vars", is_token: false),
        GT::Literal.new(value: "in"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "suite", is_token: false),
      ]),
      line_number: 164,
    ),
    GT::GrammarRule.new(
      name: "loop_vars",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 170,
    ),
    GT::GrammarRule.new(
      name: "def_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "def"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "parameters", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "suite", is_token: false),
      ]),
      line_number: 180,
    ),
    GT::GrammarRule.new(
      name: "suite",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "simple_stmt", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NEWLINE", is_token: true),
          GT::RuleReference.new(name: "INDENT", is_token: true),
          GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
          GT::RuleReference.new(name: "DEDENT", is_token: true),
        ]),
      ]),
      line_number: 191,
    ),
    GT::GrammarRule.new(
      name: "parameters",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "parameter", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "parameter", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
      ]),
      line_number: 212,
    ),
    GT::GrammarRule.new(
      name: "parameter",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOUBLE_STAR", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "STAR", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "EQUALS", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 214,
    ),
    GT::GrammarRule.new(
      name: "expression_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
      ]),
      line_number: 248,
    ),
    GT::GrammarRule.new(
      name: "expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "lambda_expr", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "or_expr", is_token: false),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::Literal.new(value: "if"),
              GT::RuleReference.new(name: "or_expr", is_token: false),
              GT::Literal.new(value: "else"),
              GT::RuleReference.new(name: "expression", is_token: false),
            ])),
        ]),
      ]),
      line_number: 253,
    ),
    GT::GrammarRule.new(
      name: "lambda_expr",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "lambda"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "lambda_params", is_token: false)),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 258,
    ),
    GT::GrammarRule.new(
      name: "lambda_params",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "lambda_param", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "lambda_param", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
      ]),
      line_number: 259,
    ),
    GT::GrammarRule.new(
      name: "lambda_param",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "EQUALS", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "STAR", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOUBLE_STAR", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
      ]),
      line_number: 260,
    ),
    GT::GrammarRule.new(
      name: "or_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "and_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "or"),
            GT::RuleReference.new(name: "and_expr", is_token: false),
          ])),
      ]),
      line_number: 264,
    ),
    GT::GrammarRule.new(
      name: "and_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "not_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "and"),
            GT::RuleReference.new(name: "not_expr", is_token: false),
          ])),
      ]),
      line_number: 268,
    ),
    GT::GrammarRule.new(
      name: "not_expr",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "not"),
          GT::RuleReference.new(name: "not_expr", is_token: false),
        ]),
        GT::RuleReference.new(name: "comparison", is_token: false),
      ]),
      line_number: 272,
    ),
    GT::GrammarRule.new(
      name: "comparison",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "bitwise_or", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "comp_op", is_token: false),
            GT::RuleReference.new(name: "bitwise_or", is_token: false),
          ])),
      ]),
      line_number: 281,
    ),
    GT::GrammarRule.new(
      name: "comp_op",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "EQUALS_EQUALS", is_token: true),
        GT::RuleReference.new(name: "NOT_EQUALS", is_token: true),
        GT::RuleReference.new(name: "LESS_THAN", is_token: true),
        GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
        GT::RuleReference.new(name: "LESS_EQUALS", is_token: true),
        GT::RuleReference.new(name: "GREATER_EQUALS", is_token: true),
        GT::Literal.new(value: "in"),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "not"),
          GT::Literal.new(value: "in"),
        ]),
      ]),
      line_number: 283,
    ),
    GT::GrammarRule.new(
      name: "bitwise_or",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "bitwise_xor", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "PIPE", is_token: true),
            GT::RuleReference.new(name: "bitwise_xor", is_token: false),
          ])),
      ]),
      line_number: 289,
    ),
    GT::GrammarRule.new(
      name: "bitwise_xor",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "bitwise_and", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "CARET", is_token: true),
            GT::RuleReference.new(name: "bitwise_and", is_token: false),
          ])),
      ]),
      line_number: 290,
    ),
    GT::GrammarRule.new(
      name: "bitwise_and",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "shift", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "AMP", is_token: true),
            GT::RuleReference.new(name: "shift", is_token: false),
          ])),
      ]),
      line_number: 291,
    ),
    GT::GrammarRule.new(
      name: "shift",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "arith", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "LEFT_SHIFT", is_token: true),
                GT::RuleReference.new(name: "RIGHT_SHIFT", is_token: true),
              ])),
            GT::RuleReference.new(name: "arith", is_token: false),
          ])),
      ]),
      line_number: 294,
    ),
    GT::GrammarRule.new(
      name: "arith",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "term", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "PLUS", is_token: true),
                GT::RuleReference.new(name: "MINUS", is_token: true),
              ])),
            GT::RuleReference.new(name: "term", is_token: false),
          ])),
      ]),
      line_number: 298,
    ),
    GT::GrammarRule.new(
      name: "term",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "factor", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "STAR", is_token: true),
                GT::RuleReference.new(name: "SLASH", is_token: true),
                GT::RuleReference.new(name: "FLOOR_DIV", is_token: true),
                GT::RuleReference.new(name: "PERCENT", is_token: true),
              ])),
            GT::RuleReference.new(name: "factor", is_token: false),
          ])),
      ]),
      line_number: 303,
    ),
    GT::GrammarRule.new(
      name: "factor",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::RuleReference.new(name: "PLUS", is_token: true),
              GT::RuleReference.new(name: "MINUS", is_token: true),
              GT::RuleReference.new(name: "TILDE", is_token: true),
            ])),
          GT::RuleReference.new(name: "factor", is_token: false),
        ]),
        GT::RuleReference.new(name: "power", is_token: false),
      ]),
      line_number: 309,
    ),
    GT::GrammarRule.new(
      name: "power",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "primary", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "DOUBLE_STAR", is_token: true),
            GT::RuleReference.new(name: "factor", is_token: false),
          ])),
      ]),
      line_number: 317,
    ),
    GT::GrammarRule.new(
      name: "primary",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "atom", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "suffix", is_token: false)),
      ]),
      line_number: 334,
    ),
    GT::GrammarRule.new(
      name: "suffix",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOT", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::RuleReference.new(name: "subscript", is_token: false),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "arguments", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 336,
    ),
    GT::GrammarRule.new(
      name: "subscript",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Sequence.new(elements: [
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
            ])),
        ]),
      ]),
      line_number: 348,
    ),
    GT::GrammarRule.new(
      name: "atom",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "INT", is_token: true),
        GT::RuleReference.new(name: "FLOAT", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "STRING", is_token: true),
          GT::Repetition.new(element: GT::RuleReference.new(name: "STRING", is_token: true)),
        ]),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "True"),
        GT::Literal.new(value: "False"),
        GT::Literal.new(value: "None"),
        GT::RuleReference.new(name: "list_expr", is_token: false),
        GT::RuleReference.new(name: "dict_expr", is_token: false),
        GT::RuleReference.new(name: "paren_expr", is_token: false),
      ]),
      line_number: 357,
    ),
    GT::GrammarRule.new(
      name: "list_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "list_body", is_token: false)),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 373,
    ),
    GT::GrammarRule.new(
      name: "list_body",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "comp_clause", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
            ])),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        ]),
      ]),
      line_number: 375,
    ),
    GT::GrammarRule.new(
      name: "dict_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "dict_body", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 381,
    ),
    GT::GrammarRule.new(
      name: "dict_body",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "dict_entry", is_token: false),
          GT::RuleReference.new(name: "comp_clause", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "dict_entry", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "dict_entry", is_token: false),
            ])),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        ]),
      ]),
      line_number: 383,
    ),
    GT::GrammarRule.new(
      name: "dict_entry",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 386,
    ),
    GT::GrammarRule.new(
      name: "paren_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "paren_body", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 393,
    ),
    GT::GrammarRule.new(
      name: "paren_body",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "comp_clause", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "COMMA", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::Repetition.new(element: GT::Sequence.new(elements: [
                  GT::RuleReference.new(name: "COMMA", is_token: true),
                  GT::RuleReference.new(name: "expression", is_token: false),
                ])),
              GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
            ])),
        ]),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 395,
    ),
    GT::GrammarRule.new(
      name: "comp_clause",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "comp_for", is_token: false),
        GT::Repetition.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "comp_for", is_token: false),
            GT::RuleReference.new(name: "comp_if", is_token: false),
          ])),
      ]),
      line_number: 411,
    ),
    GT::GrammarRule.new(
      name: "comp_for",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "for"),
        GT::RuleReference.new(name: "loop_vars", is_token: false),
        GT::Literal.new(value: "in"),
        GT::RuleReference.new(name: "or_expr", is_token: false),
      ]),
      line_number: 413,
    ),
    GT::GrammarRule.new(
      name: "comp_if",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "if"),
        GT::RuleReference.new(name: "or_expr", is_token: false),
      ]),
      line_number: 415,
    ),
    GT::GrammarRule.new(
      name: "arguments",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "argument", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "argument", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
      ]),
      line_number: 434,
    ),
    GT::GrammarRule.new(
      name: "argument",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOUBLE_STAR", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "STAR", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "EQUALS", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 436,
    ),
  ],
)
