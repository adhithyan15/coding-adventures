# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: python3.12.grammar
# Regenerate with: grammar-tools compile-grammar python3.12.grammar
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
      line_number: 34,
    ),
    GT::GrammarRule.new(
      name: "statement",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "compound_stmt", is_token: false),
        GT::RuleReference.new(name: "simple_stmt", is_token: false),
      ]),
      line_number: 43,
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
      line_number: 45,
    ),
    GT::GrammarRule.new(
      name: "small_stmt",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "return_stmt", is_token: false),
        GT::RuleReference.new(name: "import_stmt", is_token: false),
        GT::RuleReference.new(name: "from_import_stmt", is_token: false),
        GT::RuleReference.new(name: "raise_stmt", is_token: false),
        GT::RuleReference.new(name: "pass_stmt", is_token: false),
        GT::RuleReference.new(name: "del_stmt", is_token: false),
        GT::RuleReference.new(name: "yield_stmt", is_token: false),
        GT::RuleReference.new(name: "assert_stmt", is_token: false),
        GT::RuleReference.new(name: "break_stmt", is_token: false),
        GT::RuleReference.new(name: "continue_stmt", is_token: false),
        GT::RuleReference.new(name: "global_stmt", is_token: false),
        GT::RuleReference.new(name: "nonlocal_stmt", is_token: false),
        GT::RuleReference.new(name: "type_alias_stmt", is_token: false),
        GT::RuleReference.new(name: "assign_stmt", is_token: false),
      ]),
      line_number: 47,
    ),
    GT::GrammarRule.new(
      name: "return_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "return"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression_list", is_token: false)),
      ]),
      line_number: 66,
    ),
    GT::GrammarRule.new(
      name: "pass_stmt",
      body: GT::Literal.new(value: "pass"),
      line_number: 67,
    ),
    GT::GrammarRule.new(
      name: "break_stmt",
      body: GT::Literal.new(value: "break"),
      line_number: 68,
    ),
    GT::GrammarRule.new(
      name: "continue_stmt",
      body: GT::Literal.new(value: "continue"),
      line_number: 69,
    ),
    GT::GrammarRule.new(
      name: "del_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "del"),
        GT::RuleReference.new(name: "target_list", is_token: false),
      ]),
      line_number: 70,
    ),
    GT::GrammarRule.new(
      name: "yield_stmt",
      body: GT::RuleReference.new(name: "yield_expr", is_token: false),
      line_number: 71,
    ),
    GT::GrammarRule.new(
      name: "assert_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "assert"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
      ]),
      line_number: 75,
    ),
    GT::GrammarRule.new(
      name: "global_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "global"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 80,
    ),
    GT::GrammarRule.new(
      name: "nonlocal_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "nonlocal"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 81,
    ),
    GT::GrammarRule.new(
      name: "import_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "import"),
        GT::RuleReference.new(name: "dotted_name", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "as"),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "dotted_name", is_token: false),
            GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                GT::Literal.new(value: "as"),
                GT::RuleReference.new(name: "NAME", is_token: true),
              ])),
          ])),
      ]),
      line_number: 86,
    ),
    GT::GrammarRule.new(
      name: "from_import_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "from"),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::Repetition.new(element: GT::RuleReference.new(name: "DOT", is_token: true)),
              GT::RuleReference.new(name: "dotted_name", is_token: false),
            ]),
            GT::Sequence.new(elements: [
              GT::Repetition.new(element: GT::RuleReference.new(name: "DOT", is_token: true)),
              GT::RuleReference.new(name: "DOT", is_token: true),
            ]),
          ])),
        GT::Literal.new(value: "import"),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "STAR", is_token: true),
            GT::RuleReference.new(name: "import_names", is_token: false),
          ])),
      ]),
      line_number: 93,
    ),
    GT::GrammarRule.new(
      name: "import_names",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "as"),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                GT::Literal.new(value: "as"),
                GT::RuleReference.new(name: "NAME", is_token: true),
              ])),
          ])),
      ]),
      line_number: 96,
    ),
    GT::GrammarRule.new(
      name: "dotted_name",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "DOT", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 97,
    ),
    GT::GrammarRule.new(
      name: "raise_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "raise"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "expression", is_token: false),
            GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                GT::Literal.new(value: "from"),
                GT::RuleReference.new(name: "expression", is_token: false),
              ])),
          ])),
      ]),
      line_number: 103,
    ),
    GT::GrammarRule.new(
      name: "type_alias_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "type"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_params", is_token: false)),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 119,
    ),
    GT::GrammarRule.new(
      name: "type_params",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "type_param", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "type_param", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 136,
    ),
    GT::GrammarRule.new(
      name: "type_param",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
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
      line_number: 138,
    ),
    GT::GrammarRule.new(
      name: "assign_stmt",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression_list", is_token: false),
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                  GT::RuleReference.new(name: "EQUALS", is_token: true),
                  GT::RuleReference.new(name: "expression_list", is_token: false),
                ])),
            ]),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "EQUALS", is_token: true),
              GT::RuleReference.new(name: "expression_list", is_token: false),
              GT::Repetition.new(element: GT::Sequence.new(elements: [
                  GT::RuleReference.new(name: "EQUALS", is_token: true),
                  GT::RuleReference.new(name: "expression_list", is_token: false),
                ])),
            ]),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "augmented_assign_op", is_token: false),
              GT::RuleReference.new(name: "expression_list", is_token: false),
            ]),
          ])),
      ]),
      line_number: 154,
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
        GT::RuleReference.new(name: "AT_EQUALS", is_token: true),
      ]),
      line_number: 159,
    ),
    GT::GrammarRule.new(
      name: "compound_stmt",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "if_stmt", is_token: false),
        GT::RuleReference.new(name: "for_stmt", is_token: false),
        GT::RuleReference.new(name: "while_stmt", is_token: false),
        GT::RuleReference.new(name: "try_stmt", is_token: false),
        GT::RuleReference.new(name: "with_stmt", is_token: false),
        GT::RuleReference.new(name: "def_stmt", is_token: false),
        GT::RuleReference.new(name: "class_stmt", is_token: false),
        GT::RuleReference.new(name: "decorated", is_token: false),
        GT::RuleReference.new(name: "async_stmt", is_token: false),
        GT::RuleReference.new(name: "match_stmt", is_token: false),
      ]),
      line_number: 168,
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
      line_number: 180,
    ),
    GT::GrammarRule.new(
      name: "for_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "for"),
        GT::RuleReference.new(name: "target_list", is_token: false),
        GT::Literal.new(value: "in"),
        GT::RuleReference.new(name: "expression_list", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "suite", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "else"),
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "suite", is_token: false),
          ])),
      ]),
      line_number: 186,
    ),
    GT::GrammarRule.new(
      name: "while_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "while"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "suite", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "else"),
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "suite", is_token: false),
          ])),
      ]),
      line_number: 190,
    ),
    GT::GrammarRule.new(
      name: "try_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "try"),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "suite", is_token: false),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "except_clauses", is_token: false),
              GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                  GT::Literal.new(value: "else"),
                  GT::RuleReference.new(name: "COLON", is_token: true),
                  GT::RuleReference.new(name: "suite", is_token: false),
                ])),
              GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                  GT::Literal.new(value: "finally"),
                  GT::RuleReference.new(name: "COLON", is_token: true),
                  GT::RuleReference.new(name: "suite", is_token: false),
                ])),
            ]),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "except_star_clauses", is_token: false),
              GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                  GT::Literal.new(value: "else"),
                  GT::RuleReference.new(name: "COLON", is_token: true),
                  GT::RuleReference.new(name: "suite", is_token: false),
                ])),
              GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                  GT::Literal.new(value: "finally"),
                  GT::RuleReference.new(name: "COLON", is_token: true),
                  GT::RuleReference.new(name: "suite", is_token: false),
                ])),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "finally"),
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::RuleReference.new(name: "suite", is_token: false),
            ]),
          ])),
      ]),
      line_number: 202,
    ),
    GT::GrammarRule.new(
      name: "except_clauses",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "except_clause", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "except_clause", is_token: false)),
      ]),
      line_number: 207,
    ),
    GT::GrammarRule.new(
      name: "except_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "except"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "expression", is_token: false),
            GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                GT::Literal.new(value: "as"),
                GT::RuleReference.new(name: "NAME", is_token: true),
              ])),
          ])),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "suite", is_token: false),
      ]),
      line_number: 214,
    ),
    GT::GrammarRule.new(
      name: "except_star_clauses",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "except_star_clause", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "except_star_clause", is_token: false)),
      ]),
      line_number: 234,
    ),
    GT::GrammarRule.new(
      name: "except_star_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "except"),
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "as"),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "suite", is_token: false),
      ]),
      line_number: 236,
    ),
    GT::GrammarRule.new(
      name: "with_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "with"),
        GT::RuleReference.new(name: "with_items", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "suite", is_token: false),
      ]),
      line_number: 241,
    ),
    GT::GrammarRule.new(
      name: "with_items",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "with_item", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "with_item", is_token: false),
          ])),
      ]),
      line_number: 242,
    ),
    GT::GrammarRule.new(
      name: "with_item",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "as"),
            GT::RuleReference.new(name: "target", is_token: false),
          ])),
      ]),
      line_number: 243,
    ),
    GT::GrammarRule.new(
      name: "def_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "def"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_params", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "parameters", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "ARROW", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "suite", is_token: false),
      ]),
      line_number: 258,
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
      line_number: 280,
    ),
    GT::GrammarRule.new(
      name: "parameter",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOUBLE_STAR", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "STAR", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
            ])),
        ]),
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::RuleReference.new(name: "SLASH", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
            ])),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "EQUALS", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
            ])),
        ]),
      ]),
      line_number: 282,
    ),
    GT::GrammarRule.new(
      name: "class_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "class"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_params", is_token: false)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "arguments", is_token: false)),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "suite", is_token: false),
      ]),
      line_number: 300,
    ),
    GT::GrammarRule.new(
      name: "decorated",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "decorator", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "decorator", is_token: false)),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "def_stmt", is_token: false),
            GT::RuleReference.new(name: "class_stmt", is_token: false),
            GT::RuleReference.new(name: "async_stmt", is_token: false),
          ])),
      ]),
      line_number: 307,
    ),
    GT::GrammarRule.new(
      name: "decorator",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "AT", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "NEWLINE", is_token: true),
      ]),
      line_number: 309,
    ),
    GT::GrammarRule.new(
      name: "async_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "async"),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "def_stmt", is_token: false),
            GT::RuleReference.new(name: "for_stmt", is_token: false),
            GT::RuleReference.new(name: "with_stmt", is_token: false),
          ])),
      ]),
      line_number: 313,
    ),
    GT::GrammarRule.new(
      name: "match_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "match"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "NEWLINE", is_token: true),
        GT::RuleReference.new(name: "INDENT", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "case_clause", is_token: false)),
        GT::RuleReference.new(name: "DEDENT", is_token: true),
      ]),
      line_number: 329,
    ),
    GT::GrammarRule.new(
      name: "case_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "case"),
        GT::RuleReference.new(name: "pattern", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "if"),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "suite", is_token: false),
      ]),
      line_number: 333,
    ),
    GT::GrammarRule.new(
      name: "pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "or_pattern", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "as"),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 344,
    ),
    GT::GrammarRule.new(
      name: "or_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "closed_pattern", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "PIPE", is_token: true),
            GT::RuleReference.new(name: "closed_pattern", is_token: false),
          ])),
      ]),
      line_number: 346,
    ),
    GT::GrammarRule.new(
      name: "closed_pattern",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "literal_pattern", is_token: false),
        GT::RuleReference.new(name: "capture_pattern", is_token: false),
        GT::RuleReference.new(name: "wildcard_pattern", is_token: false),
        GT::RuleReference.new(name: "sequence_pattern", is_token: false),
        GT::RuleReference.new(name: "mapping_pattern", is_token: false),
        GT::RuleReference.new(name: "class_pattern", is_token: false),
        GT::RuleReference.new(name: "group_pattern", is_token: false),
        GT::RuleReference.new(name: "value_pattern", is_token: false),
      ]),
      line_number: 348,
    ),
    GT::GrammarRule.new(
      name: "literal_pattern",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "INT", is_token: true),
        GT::RuleReference.new(name: "FLOAT", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "STRING", is_token: true),
          GT::Repetition.new(element: GT::RuleReference.new(name: "STRING", is_token: true)),
        ]),
        GT::RuleReference.new(name: "IMAG", is_token: true),
        GT::Literal.new(value: "None"),
        GT::Literal.new(value: "True"),
        GT::Literal.new(value: "False"),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "MINUS", is_token: true),
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::RuleReference.new(name: "INT", is_token: true),
              GT::RuleReference.new(name: "FLOAT", is_token: true),
              GT::RuleReference.new(name: "IMAG", is_token: true),
            ])),
        ]),
      ]),
      line_number: 358,
    ),
    GT::GrammarRule.new(
      name: "capture_pattern",
      body: GT::RuleReference.new(name: "NAME", is_token: true),
      line_number: 362,
    ),
    GT::GrammarRule.new(
      name: "wildcard_pattern",
      body: GT::Literal.new(value: "_"),
      line_number: 363,
    ),
    GT::GrammarRule.new(
      name: "sequence_pattern",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "pattern_list", is_token: false)),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "pattern_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 366,
    ),
    GT::GrammarRule.new(
      name: "pattern_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "pattern", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "pattern", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
      ]),
      line_number: 369,
    ),
    GT::GrammarRule.new(
      name: "mapping_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "mapping_items", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 372,
    ),
    GT::GrammarRule.new(
      name: "mapping_items",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "mapping_item", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "mapping_item", is_token: false),
            ])),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                  GT::RuleReference.new(name: "DOUBLE_STAR", is_token: true),
                  GT::RuleReference.new(name: "NAME", is_token: true),
                ])),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOUBLE_STAR", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
      ]),
      line_number: 374,
    ),
    GT::GrammarRule.new(
      name: "mapping_item",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "literal_pattern", is_token: false),
            GT::RuleReference.new(name: "value_pattern", is_token: false),
          ])),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "pattern", is_token: false),
      ]),
      line_number: 377,
    ),
    GT::GrammarRule.new(
      name: "class_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "dotted_name", is_token: false),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "class_pattern_args", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 380,
    ),
    GT::GrammarRule.new(
      name: "class_pattern_args",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "pattern", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "pattern", is_token: false),
            ])),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "EQUALS", is_token: true),
          GT::RuleReference.new(name: "pattern", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "NAME", is_token: true),
              GT::RuleReference.new(name: "EQUALS", is_token: true),
              GT::RuleReference.new(name: "pattern", is_token: false),
            ])),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "pattern", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "pattern", is_token: false),
            ])),
          GT::RuleReference.new(name: "COMMA", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "EQUALS", is_token: true),
          GT::RuleReference.new(name: "pattern", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "NAME", is_token: true),
              GT::RuleReference.new(name: "EQUALS", is_token: true),
              GT::RuleReference.new(name: "pattern", is_token: false),
            ])),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        ]),
      ]),
      line_number: 382,
    ),
    GT::GrammarRule.new(
      name: "group_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "pattern", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 387,
    ),
    GT::GrammarRule.new(
      name: "value_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "DOT", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "DOT", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 390,
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
      line_number: 399,
    ),
    GT::GrammarRule.new(
      name: "target_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "target", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "target", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
      ]),
      line_number: 405,
    ),
    GT::GrammarRule.new(
      name: "target",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "target", is_token: false),
          GT::RuleReference.new(name: "DOT", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "target", is_token: false),
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::RuleReference.new(name: "subscript", is_token: false),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "STAR", is_token: true),
          GT::RuleReference.new(name: "target", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "target_list", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::RuleReference.new(name: "target_list", is_token: false),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
      ]),
      line_number: 407,
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
      line_number: 438,
    ),
    GT::GrammarRule.new(
      name: "named_expression",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "WALRUS", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 442,
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
        GT::RuleReference.new(name: "named_expression", is_token: false),
      ]),
      line_number: 444,
    ),
    GT::GrammarRule.new(
      name: "lambda_expr",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "lambda"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "lambda_params", is_token: false)),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 448,
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
      line_number: 449,
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
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::RuleReference.new(name: "SLASH", is_token: true),
      ]),
      line_number: 450,
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
      line_number: 456,
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
      line_number: 457,
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
      line_number: 458,
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
      line_number: 461,
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
        GT::Literal.new(value: "is"),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "is"),
          GT::Literal.new(value: "not"),
        ]),
      ]),
      line_number: 463,
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
      line_number: 469,
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
      line_number: 470,
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
      line_number: 471,
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
      line_number: 472,
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
      line_number: 473,
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
                GT::RuleReference.new(name: "AT", is_token: true),
              ])),
            GT::RuleReference.new(name: "factor", is_token: false),
          ])),
      ]),
      line_number: 474,
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
      line_number: 475,
    ),
    GT::GrammarRule.new(
      name: "power",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "await_expr", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "DOUBLE_STAR", is_token: true),
            GT::RuleReference.new(name: "factor", is_token: false),
          ])),
      ]),
      line_number: 479,
    ),
    GT::GrammarRule.new(
      name: "await_expr",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "await"),
          GT::RuleReference.new(name: "primary", is_token: false),
        ]),
        GT::RuleReference.new(name: "primary", is_token: false),
      ]),
      line_number: 482,
    ),
    GT::GrammarRule.new(
      name: "primary",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "atom", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "suffix", is_token: false)),
      ]),
      line_number: 490,
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
      line_number: 492,
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
      line_number: 498,
    ),
    GT::GrammarRule.new(
      name: "atom",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "INT", is_token: true),
        GT::RuleReference.new(name: "FLOAT", is_token: true),
        GT::RuleReference.new(name: "IMAG", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "STRING", is_token: true),
          GT::Repetition.new(element: GT::RuleReference.new(name: "STRING", is_token: true)),
        ]),
        GT::RuleReference.new(name: "fstring", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "True"),
        GT::Literal.new(value: "False"),
        GT::Literal.new(value: "None"),
        GT::RuleReference.new(name: "ELLIPSIS", is_token: true),
        GT::RuleReference.new(name: "list_expr", is_token: false),
        GT::RuleReference.new(name: "dict_expr", is_token: false),
        GT::RuleReference.new(name: "set_expr", is_token: false),
        GT::RuleReference.new(name: "paren_expr", is_token: false),
      ]),
      line_number: 505,
    ),
    GT::GrammarRule.new(
      name: "fstring",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "FSTRING_START", is_token: true),
        GT::Repetition.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "FSTRING_MIDDLE", is_token: true),
            GT::RuleReference.new(name: "fstring_replacement", is_token: false),
          ])),
        GT::RuleReference.new(name: "FSTRING_END", is_token: true),
      ]),
      line_number: 531,
    ),
    GT::GrammarRule.new(
      name: "fstring_replacement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "EQUALS", is_token: true)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "fstring_conversion", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "fstring_format_spec", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 533,
    ),
    GT::GrammarRule.new(
      name: "fstring_conversion",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "!"),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 537,
    ),
    GT::GrammarRule.new(
      name: "fstring_format_spec",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::Repetition.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "FSTRING_MIDDLE", is_token: true),
            GT::RuleReference.new(name: "fstring_replacement", is_token: false),
          ])),
      ]),
      line_number: 541,
    ),
    GT::GrammarRule.new(
      name: "list_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "list_body", is_token: false)),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 548,
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
      line_number: 550,
    ),
    GT::GrammarRule.new(
      name: "dict_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "dict_body", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 555,
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
      line_number: 557,
    ),
    GT::GrammarRule.new(
      name: "dict_entry",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOUBLE_STAR", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
      ]),
      line_number: 560,
    ),
    GT::GrammarRule.new(
      name: "set_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "set_body", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 564,
    ),
    GT::GrammarRule.new(
      name: "set_body",
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
      ]),
      line_number: 566,
    ),
    GT::GrammarRule.new(
      name: "paren_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "paren_body", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 573,
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
      line_number: 575,
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
      line_number: 581,
    ),
    GT::GrammarRule.new(
      name: "comp_for",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Literal.new(value: "async")),
        GT::Literal.new(value: "for"),
        GT::RuleReference.new(name: "target_list", is_token: false),
        GT::Literal.new(value: "in"),
        GT::RuleReference.new(name: "or_expr", is_token: false),
      ]),
      line_number: 583,
    ),
    GT::GrammarRule.new(
      name: "comp_if",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "if"),
        GT::RuleReference.new(name: "or_expr", is_token: false),
      ]),
      line_number: 585,
    ),
    GT::GrammarRule.new(
      name: "yield_expr",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "yield"),
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "from"),
              GT::RuleReference.new(name: "expression", is_token: false),
            ]),
            GT::RuleReference.new(name: "expression_list", is_token: false),
          ])),
      ]),
      line_number: 591,
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
      line_number: 600,
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
      line_number: 602,
    ),
  ],
)
