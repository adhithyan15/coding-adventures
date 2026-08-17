# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: algol60.grammar
# Regenerate with: grammar-tools compile-grammar algol60.grammar
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
      body: GT::RuleReference.new(name: "block", is_token: false),
      line_number: 47,
    ),
    GT::GrammarRule.new(
      name: "block",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "begin"),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "declaration", is_token: false),
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          ])),
        GT::Repetition.new(element: GT::RuleReference.new(name: "SEMICOLON", is_token: true)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "statement", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "SEMICOLON", is_token: true),
                GT::OptionalElement.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
              ])),
          ])),
        GT::Literal.new(value: "end"),
      ]),
      line_number: 53,
    ),
    GT::GrammarRule.new(
      name: "declaration",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "type_decl", is_token: false),
        GT::RuleReference.new(name: "own_decl", is_token: false),
        GT::RuleReference.new(name: "own_array_decl", is_token: false),
        GT::RuleReference.new(name: "array_decl", is_token: false),
        GT::RuleReference.new(name: "switch_decl", is_token: false),
        GT::RuleReference.new(name: "procedure_decl", is_token: false),
      ]),
      line_number: 60,
    ),
    GT::GrammarRule.new(
      name: "type_decl",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "ident_list", is_token: false),
      ]),
      line_number: 71,
    ),
    GT::GrammarRule.new(
      name: "own_decl",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "own"),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "ident_list", is_token: false),
      ]),
      line_number: 76,
    ),
    GT::GrammarRule.new(
      name: "own_array_decl",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "own"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type", is_token: false)),
        GT::Literal.new(value: "array"),
        GT::RuleReference.new(name: "array_segment", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "array_segment", is_token: false),
          ])),
      ]),
      line_number: 81,
    ),
    GT::GrammarRule.new(
      name: "type",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "integer"),
        GT::Literal.new(value: "real"),
        GT::Literal.new(value: "boolean"),
        GT::Literal.new(value: "string"),
      ]),
      line_number: 83,
    ),
    GT::GrammarRule.new(
      name: "ident_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 85,
    ),
    GT::GrammarRule.new(
      name: "array_decl",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type", is_token: false)),
        GT::Literal.new(value: "array"),
        GT::RuleReference.new(name: "array_segment", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "array_segment", is_token: false),
          ])),
      ]),
      line_number: 93,
    ),
    GT::GrammarRule.new(
      name: "array_segment",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "ident_list", is_token: false),
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "bound_pair", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "bound_pair", is_token: false),
          ])),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 95,
    ),
    GT::GrammarRule.new(
      name: "bound_pair",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "arith_expr", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "arith_expr", is_token: false),
      ]),
      line_number: 99,
    ),
    GT::GrammarRule.new(
      name: "switch_decl",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "switch"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "ASSIGN", is_token: true),
        GT::RuleReference.new(name: "switch_list", is_token: false),
      ]),
      line_number: 104,
    ),
    GT::GrammarRule.new(
      name: "switch_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "desig_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "desig_expr", is_token: false),
          ])),
      ]),
      line_number: 106,
    ),
    GT::GrammarRule.new(
      name: "procedure_decl",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type", is_token: false)),
        GT::Literal.new(value: "procedure"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_params", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "value_part", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "spec_part", is_token: false)),
        GT::RuleReference.new(name: "proc_body", is_token: false),
      ]),
      line_number: 113,
    ),
    GT::GrammarRule.new(
      name: "formal_params",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "ident_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 118,
    ),
    GT::GrammarRule.new(
      name: "value_part",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "value"),
        GT::RuleReference.new(name: "ident_list", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 123,
    ),
    GT::GrammarRule.new(
      name: "spec_part",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "specifier", is_token: false),
        GT::RuleReference.new(name: "ident_list", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 130,
    ),
    GT::GrammarRule.new(
      name: "specifier",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "type", is_token: false),
          GT::Literal.new(value: "array"),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "type", is_token: false),
          GT::Literal.new(value: "procedure"),
        ]),
        GT::Literal.new(value: "array"),
        GT::Literal.new(value: "label"),
        GT::Literal.new(value: "switch"),
        GT::Literal.new(value: "procedure"),
        GT::RuleReference.new(name: "type", is_token: false),
      ]),
      line_number: 132,
    ),
    GT::GrammarRule.new(
      name: "proc_body",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "block", is_token: false),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 140,
    ),
    GT::GrammarRule.new(
      name: "statement",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "label", is_token: false),
              GT::RuleReference.new(name: "COLON", is_token: true),
            ])),
          GT::RuleReference.new(name: "unlabeled_stmt", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "label", is_token: false),
              GT::RuleReference.new(name: "COLON", is_token: true),
            ])),
          GT::RuleReference.new(name: "cond_stmt", is_token: false),
        ]),
      ]),
      line_number: 152,
    ),
    GT::GrammarRule.new(
      name: "label",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "INTEGER_LIT", is_token: true),
      ]),
      line_number: 155,
    ),
    GT::GrammarRule.new(
      name: "unlabeled_stmt",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "assign_stmt", is_token: false),
        GT::RuleReference.new(name: "dummy_stmt", is_token: false),
        GT::RuleReference.new(name: "goto_stmt", is_token: false),
        GT::RuleReference.new(name: "proc_stmt", is_token: false),
        GT::RuleReference.new(name: "compound_stmt", is_token: false),
        GT::RuleReference.new(name: "block", is_token: false),
        GT::RuleReference.new(name: "for_stmt", is_token: false),
      ]),
      line_number: 165,
    ),
    GT::GrammarRule.new(
      name: "dummy_stmt",
      body: GT::Alternation.new(choices: [
        GT::PositiveLookahead.new(element: GT::RuleReference.new(name: "SEMICOLON", is_token: true)),
        GT::PositiveLookahead.new(element: GT::Literal.new(value: "end")),
        GT::PositiveLookahead.new(element: GT::Literal.new(value: "else")),
      ]),
      line_number: 175,
    ),
    GT::GrammarRule.new(
      name: "cond_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "if"),
        GT::RuleReference.new(name: "bool_expr", is_token: false),
        GT::Literal.new(value: "then"),
        GT::RuleReference.new(name: "unlabeled_stmt", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "else"),
            GT::RuleReference.new(name: "statement", is_token: false),
          ])),
      ]),
      line_number: 181,
    ),
    GT::GrammarRule.new(
      name: "compound_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "begin"),
        GT::Repetition.new(element: GT::RuleReference.new(name: "SEMICOLON", is_token: true)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "statement", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "SEMICOLON", is_token: true),
                GT::OptionalElement.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
              ])),
          ])),
        GT::Literal.new(value: "end"),
      ]),
      line_number: 185,
    ),
    GT::GrammarRule.new(
      name: "assign_stmt",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "left_part", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "left_part", is_token: false)),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 191,
    ),
    GT::GrammarRule.new(
      name: "left_part",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "variable", is_token: false),
        GT::RuleReference.new(name: "ASSIGN", is_token: true),
      ]),
      line_number: 193,
    ),
    GT::GrammarRule.new(
      name: "goto_stmt",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "goto"),
          GT::RuleReference.new(name: "desig_expr", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "go"),
          GT::Literal.new(value: "to"),
          GT::RuleReference.new(name: "desig_expr", is_token: false),
        ]),
      ]),
      line_number: 197,
    ),
    GT::GrammarRule.new(
      name: "proc_stmt",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "actual_params", is_token: false)),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
      ]),
      line_number: 202,
    ),
    GT::GrammarRule.new(
      name: "actual_params",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
      ]),
      line_number: 204,
    ),
    GT::GrammarRule.new(
      name: "for_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "for"),
        GT::RuleReference.new(name: "variable", is_token: false),
        GT::RuleReference.new(name: "ASSIGN", is_token: true),
        GT::RuleReference.new(name: "for_list", is_token: false),
        GT::Literal.new(value: "do"),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 212,
    ),
    GT::GrammarRule.new(
      name: "for_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "for_elem", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "for_elem", is_token: false),
          ])),
      ]),
      line_number: 214,
    ),
    GT::GrammarRule.new(
      name: "for_elem",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "arith_expr", is_token: false),
          GT::Literal.new(value: "step"),
          GT::RuleReference.new(name: "arith_expr", is_token: false),
          GT::Literal.new(value: "until"),
          GT::RuleReference.new(name: "arith_expr", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "arith_expr", is_token: false),
          GT::Literal.new(value: "while"),
          GT::RuleReference.new(name: "bool_expr", is_token: false),
        ]),
        GT::RuleReference.new(name: "arith_expr", is_token: false),
      ]),
      line_number: 218,
    ),
    GT::GrammarRule.new(
      name: "expression",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "if"),
          GT::RuleReference.new(name: "bool_expr", is_token: false),
          GT::Literal.new(value: "then"),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::Literal.new(value: "else"),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
        GT::RuleReference.new(name: "expr_eqv", is_token: false),
      ]),
      line_number: 250,
    ),
    GT::GrammarRule.new(
      name: "expr_eqv",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expr_impl", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "eqv"),
            GT::RuleReference.new(name: "expr_impl", is_token: false),
          ])),
      ]),
      line_number: 253,
    ),
    GT::GrammarRule.new(
      name: "expr_impl",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expr_or", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "impl"),
            GT::RuleReference.new(name: "expr_or", is_token: false),
          ])),
      ]),
      line_number: 254,
    ),
    GT::GrammarRule.new(
      name: "expr_or",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expr_and", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "or"),
            GT::RuleReference.new(name: "expr_and", is_token: false),
          ])),
      ]),
      line_number: 255,
    ),
    GT::GrammarRule.new(
      name: "expr_and",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expr_not", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "and"),
            GT::RuleReference.new(name: "expr_not", is_token: false),
          ])),
      ]),
      line_number: 256,
    ),
    GT::GrammarRule.new(
      name: "expr_not",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "not"),
          GT::RuleReference.new(name: "expr_not", is_token: false),
        ]),
        GT::RuleReference.new(name: "expr_cmp", is_token: false),
      ]),
      line_number: 257,
    ),
    GT::GrammarRule.new(
      name: "expr_cmp",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expr_add", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "EQ", is_token: true),
                GT::RuleReference.new(name: "NEQ", is_token: true),
                GT::RuleReference.new(name: "LT", is_token: true),
                GT::RuleReference.new(name: "LEQ", is_token: true),
                GT::RuleReference.new(name: "GT", is_token: true),
                GT::RuleReference.new(name: "GEQ", is_token: true),
              ])),
            GT::RuleReference.new(name: "expr_add", is_token: false),
          ])),
      ]),
      line_number: 258,
    ),
    GT::GrammarRule.new(
      name: "expr_add",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "PLUS", is_token: true),
            GT::RuleReference.new(name: "MINUS", is_token: true),
          ])),
        GT::RuleReference.new(name: "expr_mul", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "PLUS", is_token: true),
                GT::RuleReference.new(name: "MINUS", is_token: true),
              ])),
            GT::RuleReference.new(name: "expr_mul", is_token: false),
          ])),
      ]),
      line_number: 259,
    ),
    GT::GrammarRule.new(
      name: "expr_mul",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expr_pow", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "STAR", is_token: true),
                GT::RuleReference.new(name: "SLASH", is_token: true),
                GT::Literal.new(value: "div"),
                GT::Literal.new(value: "mod"),
              ])),
            GT::RuleReference.new(name: "expr_pow", is_token: false),
          ])),
      ]),
      line_number: 260,
    ),
    GT::GrammarRule.new(
      name: "expr_pow",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expr_atom", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "CARET", is_token: true),
                GT::RuleReference.new(name: "POWER", is_token: true),
              ])),
            GT::RuleReference.new(name: "expr_atom", is_token: false),
          ])),
      ]),
      line_number: 261,
    ),
    GT::GrammarRule.new(
      name: "expr_atom",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "INTEGER_LIT", is_token: true),
        GT::RuleReference.new(name: "REAL_LIT", is_token: true),
        GT::RuleReference.new(name: "STRING_LIT", is_token: true),
        GT::Literal.new(value: "true"),
        GT::Literal.new(value: "false"),
        GT::RuleReference.new(name: "proc_call", is_token: false),
        GT::RuleReference.new(name: "variable", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 262,
    ),
    GT::GrammarRule.new(
      name: "arith_expr",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "if"),
          GT::RuleReference.new(name: "bool_expr", is_token: false),
          GT::Literal.new(value: "then"),
          GT::RuleReference.new(name: "arith_expr", is_token: false),
          GT::Literal.new(value: "else"),
          GT::RuleReference.new(name: "arith_expr", is_token: false),
        ]),
        GT::RuleReference.new(name: "simple_arith", is_token: false),
      ]),
      line_number: 274,
    ),
    GT::GrammarRule.new(
      name: "simple_arith",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "PLUS", is_token: true),
            GT::RuleReference.new(name: "MINUS", is_token: true),
          ])),
        GT::RuleReference.new(name: "term", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "PLUS", is_token: true),
                GT::RuleReference.new(name: "MINUS", is_token: true),
              ])),
            GT::RuleReference.new(name: "term", is_token: false),
          ])),
      ]),
      line_number: 278,
    ),
    GT::GrammarRule.new(
      name: "term",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "factor", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "STAR", is_token: true),
                GT::RuleReference.new(name: "SLASH", is_token: true),
                GT::Literal.new(value: "div"),
                GT::Literal.new(value: "mod"),
              ])),
            GT::RuleReference.new(name: "factor", is_token: false),
          ])),
      ]),
      line_number: 283,
    ),
    GT::GrammarRule.new(
      name: "factor",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "primary", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "CARET", is_token: true),
                GT::RuleReference.new(name: "POWER", is_token: true),
              ])),
            GT::RuleReference.new(name: "primary", is_token: false),
          ])),
      ]),
      line_number: 289,
    ),
    GT::GrammarRule.new(
      name: "primary",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "INTEGER_LIT", is_token: true),
        GT::RuleReference.new(name: "REAL_LIT", is_token: true),
        GT::RuleReference.new(name: "STRING_LIT", is_token: true),
        GT::Literal.new(value: "true"),
        GT::Literal.new(value: "false"),
        GT::RuleReference.new(name: "proc_call", is_token: false),
        GT::RuleReference.new(name: "variable", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "arith_expr", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 291,
    ),
    GT::GrammarRule.new(
      name: "bool_expr",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "if"),
          GT::RuleReference.new(name: "bool_expr", is_token: false),
          GT::Literal.new(value: "then"),
          GT::RuleReference.new(name: "bool_expr", is_token: false),
          GT::Literal.new(value: "else"),
          GT::RuleReference.new(name: "bool_expr", is_token: false),
        ]),
        GT::RuleReference.new(name: "simple_bool", is_token: false),
      ]),
      line_number: 309,
    ),
    GT::GrammarRule.new(
      name: "simple_bool",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "implication", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "eqv"),
            GT::RuleReference.new(name: "implication", is_token: false),
          ])),
      ]),
      line_number: 312,
    ),
    GT::GrammarRule.new(
      name: "implication",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "bool_term", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "impl"),
            GT::RuleReference.new(name: "bool_term", is_token: false),
          ])),
      ]),
      line_number: 314,
    ),
    GT::GrammarRule.new(
      name: "bool_term",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "bool_factor", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "or"),
            GT::RuleReference.new(name: "bool_factor", is_token: false),
          ])),
      ]),
      line_number: 316,
    ),
    GT::GrammarRule.new(
      name: "bool_factor",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "bool_secondary", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "and"),
            GT::RuleReference.new(name: "bool_secondary", is_token: false),
          ])),
      ]),
      line_number: 318,
    ),
    GT::GrammarRule.new(
      name: "bool_secondary",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "not"),
          GT::RuleReference.new(name: "bool_secondary", is_token: false),
        ]),
        GT::RuleReference.new(name: "bool_primary", is_token: false),
      ]),
      line_number: 320,
    ),
    GT::GrammarRule.new(
      name: "bool_primary",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "relation", is_token: false),
        GT::Literal.new(value: "true"),
        GT::Literal.new(value: "false"),
        GT::RuleReference.new(name: "proc_call", is_token: false),
        GT::RuleReference.new(name: "variable", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "bool_expr", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 322,
    ),
    GT::GrammarRule.new(
      name: "relation",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "simple_arith", is_token: false),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "EQ", is_token: true),
            GT::RuleReference.new(name: "NEQ", is_token: true),
            GT::RuleReference.new(name: "LT", is_token: true),
            GT::RuleReference.new(name: "LEQ", is_token: true),
            GT::RuleReference.new(name: "GT", is_token: true),
            GT::RuleReference.new(name: "GEQ", is_token: true),
          ])),
        GT::RuleReference.new(name: "simple_arith", is_token: false),
      ]),
      line_number: 332,
    ),
    GT::GrammarRule.new(
      name: "desig_expr",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "if"),
          GT::RuleReference.new(name: "bool_expr", is_token: false),
          GT::Literal.new(value: "then"),
          GT::RuleReference.new(name: "desig_expr", is_token: false),
          GT::Literal.new(value: "else"),
          GT::RuleReference.new(name: "desig_expr", is_token: false),
        ]),
        GT::RuleReference.new(name: "simple_desig", is_token: false),
      ]),
      line_number: 337,
    ),
    GT::GrammarRule.new(
      name: "simple_desig",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::RuleReference.new(name: "arith_expr", is_token: false),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "desig_expr", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::RuleReference.new(name: "label", is_token: false),
      ]),
      line_number: 340,
    ),
    GT::GrammarRule.new(
      name: "variable",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LBRACKET", is_token: true),
            GT::RuleReference.new(name: "subscripts", is_token: false),
            GT::RuleReference.new(name: "RBRACKET", is_token: true),
          ])),
      ]),
      line_number: 352,
    ),
    GT::GrammarRule.new(
      name: "subscripts",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "arith_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "arith_expr", is_token: false),
          ])),
      ]),
      line_number: 354,
    ),
    GT::GrammarRule.new(
      name: "proc_call",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "actual_params", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 359,
    ),
  ],
)
