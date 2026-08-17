# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: dartmouth_basic.grammar
# Regenerate with: grammar-tools compile-grammar dartmouth_basic.grammar
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
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "line", is_token: false)),
      line_number: 70,
    ),
    GT::GrammarRule.new(
      name: "line",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LINE_NUM", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
        GT::RuleReference.new(name: "NEWLINE", is_token: true),
      ]),
      line_number: 81,
    ),
    GT::GrammarRule.new(
      name: "statement",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "let_stmt", is_token: false),
        GT::RuleReference.new(name: "print_stmt", is_token: false),
        GT::RuleReference.new(name: "input_stmt", is_token: false),
        GT::RuleReference.new(name: "if_stmt", is_token: false),
        GT::RuleReference.new(name: "goto_stmt", is_token: false),
        GT::RuleReference.new(name: "gosub_stmt", is_token: false),
        GT::RuleReference.new(name: "return_stmt", is_token: false),
        GT::RuleReference.new(name: "for_stmt", is_token: false),
        GT::RuleReference.new(name: "next_stmt", is_token: false),
        GT::RuleReference.new(name: "end_stmt", is_token: false),
        GT::RuleReference.new(name: "stop_stmt", is_token: false),
        GT::RuleReference.new(name: "rem_stmt", is_token: false),
        GT::RuleReference.new(name: "read_stmt", is_token: false),
        GT::RuleReference.new(name: "data_stmt", is_token: false),
        GT::RuleReference.new(name: "restore_stmt", is_token: false),
        GT::RuleReference.new(name: "dim_stmt", is_token: false),
        GT::RuleReference.new(name: "def_stmt", is_token: false),
      ]),
      line_number: 91,
    ),
    GT::GrammarRule.new(
      name: "let_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "LET"),
        GT::RuleReference.new(name: "variable", is_token: false),
        GT::RuleReference.new(name: "EQ", is_token: true),
        GT::RuleReference.new(name: "expr", is_token: false),
      ]),
      line_number: 121,
    ),
    GT::GrammarRule.new(
      name: "print_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "PRINT"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "print_list", is_token: false)),
      ]),
      line_number: 137,
    ),
    GT::GrammarRule.new(
      name: "print_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "print_item", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "print_sep", is_token: false),
            GT::RuleReference.new(name: "print_item", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "print_sep", is_token: false)),
      ]),
      line_number: 139,
    ),
    GT::GrammarRule.new(
      name: "print_item",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "expr", is_token: false),
      ]),
      line_number: 141,
    ),
    GT::GrammarRule.new(
      name: "print_sep",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "COMMA", is_token: true),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 143,
    ),
    GT::GrammarRule.new(
      name: "input_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "INPUT"),
        GT::RuleReference.new(name: "variable", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "variable", is_token: false),
          ])),
      ]),
      line_number: 155,
    ),
    GT::GrammarRule.new(
      name: "if_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "IF"),
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::RuleReference.new(name: "relop", is_token: false),
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::Literal.new(value: "THEN"),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
      ]),
      line_number: 170,
    ),
    GT::GrammarRule.new(
      name: "relop",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "EQ", is_token: true),
        GT::RuleReference.new(name: "LT", is_token: true),
        GT::RuleReference.new(name: "GT", is_token: true),
        GT::RuleReference.new(name: "LE", is_token: true),
        GT::RuleReference.new(name: "GE", is_token: true),
        GT::RuleReference.new(name: "NE", is_token: true),
      ]),
      line_number: 172,
    ),
    GT::GrammarRule.new(
      name: "goto_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "GOTO"),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
      ]),
      line_number: 183,
    ),
    GT::GrammarRule.new(
      name: "gosub_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "GOSUB"),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
      ]),
      line_number: 198,
    ),
    GT::GrammarRule.new(
      name: "return_stmt",
      body: GT::Literal.new(value: "RETURN"),
      line_number: 200,
    ),
    GT::GrammarRule.new(
      name: "for_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "FOR"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "EQ", is_token: true),
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::Literal.new(value: "TO"),
        GT::RuleReference.new(name: "expr", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "STEP"),
            GT::RuleReference.new(name: "expr", is_token: false),
          ])),
      ]),
      line_number: 222,
    ),
    GT::GrammarRule.new(
      name: "next_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "NEXT"),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 224,
    ),
    GT::GrammarRule.new(
      name: "end_stmt",
      body: GT::Literal.new(value: "END"),
      line_number: 233,
    ),
    GT::GrammarRule.new(
      name: "stop_stmt",
      body: GT::Literal.new(value: "STOP"),
      line_number: 234,
    ),
    GT::GrammarRule.new(
      name: "rem_stmt",
      body: GT::Literal.new(value: "REM"),
      line_number: 247,
    ),
    GT::GrammarRule.new(
      name: "read_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "READ"),
        GT::RuleReference.new(name: "variable", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "variable", is_token: false),
          ])),
      ]),
      line_number: 263,
    ),
    GT::GrammarRule.new(
      name: "data_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "DATA"),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "NUMBER", is_token: true),
          ])),
      ]),
      line_number: 265,
    ),
    GT::GrammarRule.new(
      name: "restore_stmt",
      body: GT::Literal.new(value: "RESTORE"),
      line_number: 267,
    ),
    GT::GrammarRule.new(
      name: "dim_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "DIM"),
        GT::RuleReference.new(name: "dim_decl", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "dim_decl", is_token: false),
          ])),
      ]),
      line_number: 280,
    ),
    GT::GrammarRule.new(
      name: "dim_decl",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "NUMBER", is_token: true),
          ])),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 282,
    ),
    GT::GrammarRule.new(
      name: "def_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "DEF"),
        GT::RuleReference.new(name: "USER_FN", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "EQ", is_token: true),
        GT::RuleReference.new(name: "expr", is_token: false),
      ]),
      line_number: 295,
    ),
    GT::GrammarRule.new(
      name: "variable",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "expr", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "expr", is_token: false),
            ])),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 312,
    ),
    GT::GrammarRule.new(
      name: "expr",
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
      line_number: 335,
    ),
    GT::GrammarRule.new(
      name: "term",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "power", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "STAR", is_token: true),
                GT::RuleReference.new(name: "SLASH", is_token: true),
              ])),
            GT::RuleReference.new(name: "power", is_token: false),
          ])),
      ]),
      line_number: 337,
    ),
    GT::GrammarRule.new(
      name: "power",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "unary", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "CARET", is_token: true),
            GT::RuleReference.new(name: "power", is_token: false),
          ])),
      ]),
      line_number: 343,
    ),
    GT::GrammarRule.new(
      name: "unary",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "MINUS", is_token: true),
          GT::RuleReference.new(name: "primary", is_token: false),
        ]),
        GT::RuleReference.new(name: "primary", is_token: false),
      ]),
      line_number: 348,
    ),
    GT::GrammarRule.new(
      name: "primary",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "BUILTIN_FN", is_token: true),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "expr", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "USER_FN", is_token: true),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "expr", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::RuleReference.new(name: "variable", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "expr", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 366,
    ),
  ],
)
