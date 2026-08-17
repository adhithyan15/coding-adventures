# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: nib.grammar
# Regenerate with: grammar-tools compile-grammar nib.grammar
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
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "top_decl", is_token: false)),
      line_number: 42,
    ),
    GT::GrammarRule.new(
      name: "top_decl",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "const_decl", is_token: false),
        GT::RuleReference.new(name: "static_decl", is_token: false),
        GT::RuleReference.new(name: "fn_decl", is_token: false),
      ]),
      line_number: 47,
    ),
    GT::GrammarRule.new(
      name: "const_decl",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "const"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "EQ", is_token: true),
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 60,
    ),
    GT::GrammarRule.new(
      name: "static_decl",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "static"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "EQ", is_token: true),
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 66,
    ),
    GT::GrammarRule.new(
      name: "fn_decl",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "fn"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "param_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "ARROW", is_token: true),
            GT::RuleReference.new(name: "type", is_token: false),
          ])),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 77,
    ),
    GT::GrammarRule.new(
      name: "param_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "param", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "param", is_token: false),
          ])),
      ]),
      line_number: 80,
    ),
    GT::GrammarRule.new(
      name: "param",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
      ]),
      line_number: 87,
    ),
    GT::GrammarRule.new(
      name: "block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "stmt", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 98,
    ),
    GT::GrammarRule.new(
      name: "stmt",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "let_stmt", is_token: false),
        GT::RuleReference.new(name: "assign_stmt", is_token: false),
        GT::RuleReference.new(name: "return_stmt", is_token: false),
        GT::RuleReference.new(name: "for_stmt", is_token: false),
        GT::RuleReference.new(name: "while_stmt", is_token: false),
        GT::RuleReference.new(name: "if_stmt", is_token: false),
        GT::RuleReference.new(name: "expr_stmt", is_token: false),
      ]),
      line_number: 113,
    ),
    GT::GrammarRule.new(
      name: "let_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "let"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "EQ", is_token: true),
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 126,
    ),
    GT::GrammarRule.new(
      name: "assign_stmt",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "EQ", is_token: true),
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 131,
    ),
    GT::GrammarRule.new(
      name: "return_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "return"),
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 136,
    ),
    GT::GrammarRule.new(
      name: "for_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "for"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::Literal.new(value: "in"),
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::RuleReference.new(name: "RANGE", is_token: true),
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 159,
    ),
    GT::GrammarRule.new(
      name: "while_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "while"),
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 170,
    ),
    GT::GrammarRule.new(
      name: "if_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "if"),
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::RuleReference.new(name: "block", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "else"),
            GT::RuleReference.new(name: "block", is_token: false),
          ])),
      ]),
      line_number: 176,
    ),
    GT::GrammarRule.new(
      name: "expr_stmt",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 183,
    ),
    GT::GrammarRule.new(
      name: "type",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "u4"),
        GT::Literal.new(value: "u8"),
        GT::Literal.new(value: "bcd"),
        GT::Literal.new(value: "bool"),
      ]),
      line_number: 218,
    ),
    GT::GrammarRule.new(
      name: "expr",
      body: GT::RuleReference.new(name: "or_expr", is_token: false),
      line_number: 259,
    ),
    GT::GrammarRule.new(
      name: "or_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "and_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LOR", is_token: true),
            GT::RuleReference.new(name: "and_expr", is_token: false),
          ])),
      ]),
      line_number: 265,
    ),
    GT::GrammarRule.new(
      name: "and_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "eq_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LAND", is_token: true),
            GT::RuleReference.new(name: "eq_expr", is_token: false),
          ])),
      ]),
      line_number: 269,
    ),
    GT::GrammarRule.new(
      name: "eq_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "cmp_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "EQ_EQ", is_token: true),
                GT::RuleReference.new(name: "NEQ", is_token: true),
              ])),
            GT::RuleReference.new(name: "cmp_expr", is_token: false),
          ])),
      ]),
      line_number: 274,
    ),
    GT::GrammarRule.new(
      name: "cmp_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "add_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "LT", is_token: true),
                GT::RuleReference.new(name: "GT", is_token: true),
                GT::RuleReference.new(name: "LEQ", is_token: true),
                GT::RuleReference.new(name: "GEQ", is_token: true),
              ])),
            GT::RuleReference.new(name: "add_expr", is_token: false),
          ])),
      ]),
      line_number: 280,
    ),
    GT::GrammarRule.new(
      name: "add_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "shift_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "PLUS", is_token: true),
                GT::RuleReference.new(name: "MINUS", is_token: true),
                GT::RuleReference.new(name: "WRAP_ADD", is_token: true),
                GT::RuleReference.new(name: "SAT_ADD", is_token: true),
              ])),
            GT::RuleReference.new(name: "shift_expr", is_token: false),
          ])),
      ]),
      line_number: 293,
    ),
    GT::GrammarRule.new(
      name: "shift_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "mul_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "SHL", is_token: true),
                GT::RuleReference.new(name: "SHR", is_token: true),
              ])),
            GT::RuleReference.new(name: "mul_expr", is_token: false),
          ])),
      ]),
      line_number: 298,
    ),
    GT::GrammarRule.new(
      name: "mul_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "bitwise_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "STAR", is_token: true),
                GT::RuleReference.new(name: "SLASH", is_token: true),
                GT::RuleReference.new(name: "PERCENT", is_token: true),
              ])),
            GT::RuleReference.new(name: "bitwise_expr", is_token: false),
          ])),
      ]),
      line_number: 308,
    ),
    GT::GrammarRule.new(
      name: "bitwise_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "unary_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "AMP", is_token: true),
                GT::RuleReference.new(name: "PIPE", is_token: true),
                GT::RuleReference.new(name: "CARET", is_token: true),
              ])),
            GT::RuleReference.new(name: "unary_expr", is_token: false),
          ])),
      ]),
      line_number: 314,
    ),
    GT::GrammarRule.new(
      name: "unary_expr",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::RuleReference.new(name: "BANG", is_token: true),
              GT::RuleReference.new(name: "TILDE", is_token: true),
            ])),
          GT::RuleReference.new(name: "unary_expr", is_token: false),
        ]),
        GT::RuleReference.new(name: "primary", is_token: false),
      ]),
      line_number: 322,
    ),
    GT::GrammarRule.new(
      name: "primary",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "INT_LIT", is_token: true),
        GT::RuleReference.new(name: "HEX_LIT", is_token: true),
        GT::Literal.new(value: "true"),
        GT::Literal.new(value: "false"),
        GT::RuleReference.new(name: "call_expr", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "expr", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 330,
    ),
    GT::GrammarRule.new(
      name: "call_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "arg_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 353,
    ),
    GT::GrammarRule.new(
      name: "arg_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "expr", is_token: false),
          ])),
      ]),
      line_number: 356,
    ),
  ],
)
