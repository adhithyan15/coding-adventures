# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: haskell1.1.grammar
# Regenerate with: grammar-tools compile-grammar haskell1.1.grammar
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
      name: "layout_open",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "VIRTUAL_LBRACE", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Literal.new(value: "{"),
      ]),
      line_number: 12,
    ),
    GT::GrammarRule.new(
      name: "layout_close",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "VIRTUAL_RBRACE", is_token: true),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
        GT::Literal.new(value: "}"),
      ]),
      line_number: 13,
    ),
    GT::GrammarRule.new(
      name: "layout_sep",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "VIRTUAL_SEMICOLON", is_token: true),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::RuleReference.new(name: "NEWLINE", is_token: true),
      ]),
      line_number: 14,
    ),
    GT::GrammarRule.new(
      name: "file",
      body: GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "declaration", is_token: false),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "layout_sep", is_token: false)),
        ])),
      line_number: 19,
    ),
    GT::GrammarRule.new(
      name: "declaration",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "module_decl", is_token: false),
        GT::RuleReference.new(name: "let_decl", is_token: false),
        GT::RuleReference.new(name: "do_decl", is_token: false),
        GT::RuleReference.new(name: "expr_decl", is_token: false),
      ]),
      line_number: 20,
    ),
    GT::GrammarRule.new(
      name: "module_decl",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "module"),
        GT::RuleReference.new(name: "module_name", is_token: false),
        GT::Literal.new(value: "where"),
        GT::RuleReference.new(name: "layout_open", is_token: false),
        GT::RuleReference.new(name: "module_body", is_token: false),
        GT::RuleReference.new(name: "layout_close", is_token: false),
      ]),
      line_number: 22,
    ),
    GT::GrammarRule.new(
      name: "module_name",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "DOT", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 23,
    ),
    GT::GrammarRule.new(
      name: "module_body",
      body: GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "declaration", is_token: false),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "layout_sep", is_token: false)),
        ])),
      line_number: 24,
    ),
    GT::GrammarRule.new(
      name: "let_decl",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "let"),
        GT::RuleReference.new(name: "layout_open", is_token: false),
        GT::RuleReference.new(name: "let_bindings", is_token: false),
        GT::RuleReference.new(name: "layout_close", is_token: false),
        GT::Literal.new(value: "in"),
        GT::RuleReference.new(name: "expr_decl", is_token: false),
      ]),
      line_number: 26,
    ),
    GT::GrammarRule.new(
      name: "let_bindings",
      body: GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "binding", is_token: false),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "layout_sep", is_token: false)),
        ])),
      line_number: 27,
    ),
    GT::GrammarRule.new(
      name: "binding",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "expr_decl", is_token: false),
      ]),
      line_number: 28,
    ),
    GT::GrammarRule.new(
      name: "do_decl",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "do"),
        GT::RuleReference.new(name: "layout_open", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "expr_decl", is_token: false),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "layout_sep", is_token: false)),
          ])),
        GT::RuleReference.new(name: "layout_close", is_token: false),
      ]),
      line_number: 30,
    ),
    GT::GrammarRule.new(
      name: "expr_decl",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "lambda_expr", is_token: false),
        GT::RuleReference.new(name: "app_expr", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "INTEGER", is_token: true),
        GT::RuleReference.new(name: "FLOAT", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "CHARACTER", is_token: true),
      ]),
      line_number: 32,
    ),
    GT::GrammarRule.new(
      name: "lambda_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LAMBDA", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "RARROW", is_token: true),
        GT::RuleReference.new(name: "expr_decl", is_token: false),
      ]),
      line_number: 34,
    ),
    GT::GrammarRule.new(
      name: "app_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "atom_expr", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "atom_expr", is_token: false)),
      ]),
      line_number: 35,
    ),
    GT::GrammarRule.new(
      name: "atom_expr",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "INTEGER", is_token: true),
        GT::RuleReference.new(name: "FLOAT", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "CHARACTER", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "expr_decl", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "expr_list", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "expr_list", is_token: false)),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
      ]),
      line_number: 36,
    ),
    GT::GrammarRule.new(
      name: "expr_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expr_decl", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "expr_decl", is_token: false),
          ])),
      ]),
      line_number: 45,
    ),
  ],
)
