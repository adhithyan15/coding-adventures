# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: python3.10.grammar
# Regenerate with: grammar-tools compile-grammar python3.10.grammar
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
      line_number: 56,
    ),
    GT::GrammarRule.new(
      name: "statement",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "compound_stmt", is_token: false),
        GT::RuleReference.new(name: "simple_stmt", is_token: false),
      ]),
      line_number: 70,
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
      line_number: 74,
    ),
    GT::GrammarRule.new(
      name: "small_stmt",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "return_stmt", is_token: false),
        GT::RuleReference.new(name: "raise_stmt", is_token: false),
        GT::RuleReference.new(name: "assert_stmt", is_token: false),
        GT::RuleReference.new(name: "del_stmt", is_token: false),
        GT::RuleReference.new(name: "pass_stmt", is_token: false),
        GT::RuleReference.new(name: "break_stmt", is_token: false),
        GT::RuleReference.new(name: "continue_stmt", is_token: false),
        GT::RuleReference.new(name: "import_stmt", is_token: false),
        GT::RuleReference.new(name: "global_stmt", is_token: false),
        GT::RuleReference.new(name: "nonlocal_stmt", is_token: false),
        GT::RuleReference.new(name: "assign_stmt", is_token: false),
      ]),
      line_number: 76,
    ),
    GT::GrammarRule.new(
      name: "return_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "return"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression_list", is_token: false)),
      ]),
      line_number: 96,
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
      line_number: 102,
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
      line_number: 107,
    ),
    GT::GrammarRule.new(
      name: "del_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "del"),
        GT::RuleReference.new(name: "target_list", is_token: false),
      ]),
      line_number: 114,
    ),
    GT::GrammarRule.new(
      name: "pass_stmt",
      body: GT::Literal.new(value: "pass"),
      line_number: 119,
    ),
    GT::GrammarRule.new(
      name: "break_stmt",
      body: GT::Literal.new(value: "break"),
      line_number: 122,
    ),
    GT::GrammarRule.new(
      name: "continue_stmt",
      body: GT::Literal.new(value: "continue"),
      line_number: 125,
    ),
    GT::GrammarRule.new(
      name: "import_stmt",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "import"),
          GT::RuleReference.new(name: "dotted_name_list", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "from"),
          GT::RuleReference.new(name: "import_from", is_token: false),
          GT::Literal.new(value: "import"),
          GT::RuleReference.new(name: "import_targets", is_token: false),
        ]),
      ]),
      line_number: 134,
    ),
    GT::GrammarRule.new(
      name: "dotted_name_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "dotted_name", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "dotted_name", is_token: false),
          ])),
      ]),
      line_number: 137,
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
      line_number: 138,
    ),
    GT::GrammarRule.new(
      name: "import_from",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Repetition.new(element: GT::RuleReference.new(name: "DOT", is_token: true)),
          GT::RuleReference.new(name: "dotted_name", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Repetition.new(element: GT::RuleReference.new(name: "DOT", is_token: true)),
          GT::RuleReference.new(name: "DOT", is_token: true),
        ]),
      ]),
      line_number: 140,
    ),
    GT::GrammarRule.new(
      name: "import_targets",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "import_name_list", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::RuleReference.new(name: "import_name_list", is_token: false),
      ]),
      line_number: 141,
    ),
    GT::GrammarRule.new(
      name: "import_name_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "import_name", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "import_name", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
      ]),
      line_number: 144,
    ),
    GT::GrammarRule.new(
      name: "import_name",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "as"),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 145,
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
      line_number: 150,
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
      line_number: 159,
    ),
    GT::GrammarRule.new(
      name: "assign_stmt",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression_list", is_token: false),
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "assign_suffix", is_token: false),
            GT::RuleReference.new(name: "augmented_assign", is_token: false),
            GT::RuleReference.new(name: "annotation", is_token: false),
          ])),
      ]),
      line_number: 175,
    ),
    GT::GrammarRule.new(
      name: "assign_suffix",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "expression_list", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "expression_list", is_token: false),
          ])),
      ]),
      line_number: 178,
    ),
    GT::GrammarRule.new(
      name: "augmented_assign",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "augmented_assign_op", is_token: false),
        GT::RuleReference.new(name: "expression_list", is_token: false),
      ]),
      line_number: 180,
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
      line_number: 182,
    ),
    GT::GrammarRule.new(
      name: "annotation",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "expression_list", is_token: false),
          ])),
      ]),
      line_number: 191,
    ),
    GT::GrammarRule.new(
      name: "compound_stmt",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "if_stmt", is_token: false),
        GT::RuleReference.new(name: "for_stmt", is_token: false),
        GT::RuleReference.new(name: "while_stmt", is_token: false),
        GT::RuleReference.new(name: "with_stmt", is_token: false),
        GT::RuleReference.new(name: "try_stmt", is_token: false),
        GT::RuleReference.new(name: "def_stmt", is_token: false),
        GT::RuleReference.new(name: "class_stmt", is_token: false),
        GT::RuleReference.new(name: "async_stmt", is_token: false),
        GT::RuleReference.new(name: "match_stmt", is_token: false),
      ]),
      line_number: 199,
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
      line_number: 217,
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
      line_number: 230,
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
      line_number: 239,
    ),
    GT::GrammarRule.new(
      name: "with_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "with"),
        GT::RuleReference.new(name: "with_items", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "suite", is_token: false),
      ]),
      line_number: 265,
    ),
    GT::GrammarRule.new(
      name: "with_items",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "with_item", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "with_item", is_token: false),
            ])),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "with_item", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "with_item", is_token: false),
            ])),
        ]),
      ]),
      line_number: 267,
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
      line_number: 270,
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
              GT::Literal.new(value: "finally"),
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::RuleReference.new(name: "suite", is_token: false),
            ]),
          ])),
      ]),
      line_number: 286,
    ),
    GT::GrammarRule.new(
      name: "except_clauses",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "except_clause", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "except_clause", is_token: false)),
      ]),
      line_number: 290,
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
      line_number: 291,
    ),
    GT::GrammarRule.new(
      name: "def_stmt",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "decorator", is_token: false)),
        GT::Literal.new(value: "def"),
        GT::RuleReference.new(name: "NAME", is_token: true),
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
      line_number: 302,
    ),
    GT::GrammarRule.new(
      name: "class_stmt",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "decorator", is_token: false)),
        GT::Literal.new(value: "class"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "arguments", is_token: false)),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "suite", is_token: false),
      ]),
      line_number: 315,
    ),
    GT::GrammarRule.new(
      name: "decorator",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "AT", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "NEWLINE", is_token: true),
      ]),
      line_number: 323,
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
      line_number: 331,
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
      line_number: 340,
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
      line_number: 401,
    ),
    GT::GrammarRule.new(
      name: "case_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "case"),
        GT::RuleReference.new(name: "pattern", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "guard", is_token: false)),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "suite", is_token: false),
      ]),
      line_number: 418,
    ),
    GT::GrammarRule.new(
      name: "guard",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "if"),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 427,
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
      line_number: 462,
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
      line_number: 477,
    ),
    GT::GrammarRule.new(
      name: "closed_pattern",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "literal_pattern", is_token: false),
        GT::RuleReference.new(name: "capture_pattern", is_token: false),
        GT::RuleReference.new(name: "wildcard_pattern", is_token: false),
        GT::RuleReference.new(name: "value_pattern", is_token: false),
        GT::RuleReference.new(name: "group_pattern", is_token: false),
        GT::RuleReference.new(name: "sequence_pattern", is_token: false),
        GT::RuleReference.new(name: "mapping_pattern", is_token: false),
        GT::RuleReference.new(name: "class_pattern", is_token: false),
        GT::RuleReference.new(name: "star_pattern", is_token: false),
      ]),
      line_number: 486,
    ),
    GT::GrammarRule.new(
      name: "literal_pattern",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "signed_number", is_token: false),
        GT::RuleReference.new(name: "complex_number", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "STRING", is_token: true),
          GT::Repetition.new(element: GT::RuleReference.new(name: "STRING", is_token: true)),
        ]),
        GT::Literal.new(value: "True"),
        GT::Literal.new(value: "False"),
        GT::Literal.new(value: "None"),
      ]),
      line_number: 523,
    ),
    GT::GrammarRule.new(
      name: "signed_number",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "MINUS", is_token: true)),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "INT", is_token: true),
            GT::RuleReference.new(name: "FLOAT", is_token: true),
          ])),
      ]),
      line_number: 533,
    ),
    GT::GrammarRule.new(
      name: "complex_number",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "signed_number", is_token: false),
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::RuleReference.new(name: "PLUS", is_token: true),
              GT::RuleReference.new(name: "MINUS", is_token: true),
            ])),
          GT::RuleReference.new(name: "IMAG", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "MINUS", is_token: true)),
          GT::RuleReference.new(name: "IMAG", is_token: true),
        ]),
      ]),
      line_number: 542,
    ),
    GT::GrammarRule.new(
      name: "capture_pattern",
      body: GT::RuleReference.new(name: "NAME", is_token: true),
      line_number: 572,
    ),
    GT::GrammarRule.new(
      name: "wildcard_pattern",
      body: GT::Literal.new(value: "_"),
      line_number: 592,
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
      line_number: 619,
    ),
    GT::GrammarRule.new(
      name: "group_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "pattern", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 635,
    ),
    GT::GrammarRule.new(
      name: "sequence_pattern",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "sequence_pattern_content", is_token: false)),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "sequence_pattern_tuple_content", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 663,
    ),
    GT::GrammarRule.new(
      name: "sequence_pattern_content",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "maybe_star_pattern", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "maybe_star_pattern", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
      ]),
      line_number: 667,
    ),
    GT::GrammarRule.new(
      name: "sequence_pattern_tuple_content",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "maybe_star_pattern", is_token: false),
        GT::RuleReference.new(name: "COMMA", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "maybe_star_pattern", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "maybe_star_pattern", is_token: false),
              ])),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
          ])),
      ]),
      line_number: 671,
    ),
    GT::GrammarRule.new(
      name: "maybe_star_pattern",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "star_pattern", is_token: false),
        GT::RuleReference.new(name: "pattern", is_token: false),
      ]),
      line_number: 674,
    ),
    GT::GrammarRule.new(
      name: "star_pattern",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "STAR", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "STAR", is_token: true),
          GT::Literal.new(value: "_"),
        ]),
      ]),
      line_number: 689,
    ),
    GT::GrammarRule.new(
      name: "mapping_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "mapping_pattern_content", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 724,
    ),
    GT::GrammarRule.new(
      name: "mapping_pattern_content",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "mapping_entry", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "mapping_entry", is_token: false),
            ])),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::OptionalElement.new(element: GT::RuleReference.new(name: "double_star_capture", is_token: false)),
            ])),
        ]),
        GT::RuleReference.new(name: "double_star_capture", is_token: false),
      ]),
      line_number: 726,
    ),
    GT::GrammarRule.new(
      name: "mapping_entry",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "literal_pattern", is_token: false),
            GT::RuleReference.new(name: "value_pattern", is_token: false),
          ])),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "pattern", is_token: false),
      ]),
      line_number: 729,
    ),
    GT::GrammarRule.new(
      name: "double_star_capture",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOUBLE_STAR", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOUBLE_STAR", is_token: true),
          GT::Literal.new(value: "_"),
        ]),
      ]),
      line_number: 732,
    ),
    GT::GrammarRule.new(
      name: "class_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "name_or_dotted", is_token: false),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "class_pattern_content", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 785,
    ),
    GT::GrammarRule.new(
      name: "name_or_dotted",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "DOT", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 787,
    ),
    GT::GrammarRule.new(
      name: "class_pattern_content",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "class_positional_patterns", is_token: false),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "class_keyword_patterns", is_token: false),
            ])),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "class_keyword_patterns", is_token: false),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        ]),
      ]),
      line_number: 789,
    ),
    GT::GrammarRule.new(
      name: "class_positional_patterns",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "pattern", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "pattern", is_token: false),
          ])),
      ]),
      line_number: 792,
    ),
    GT::GrammarRule.new(
      name: "class_keyword_patterns",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "keyword_pattern", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "keyword_pattern", is_token: false),
          ])),
      ]),
      line_number: 794,
    ),
    GT::GrammarRule.new(
      name: "keyword_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "pattern", is_token: false),
      ]),
      line_number: 796,
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
      line_number: 828,
    ),
    GT::GrammarRule.new(
      name: "expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "lambda_expr", is_token: false),
        GT::RuleReference.new(name: "walrus_expr", is_token: false),
      ]),
      line_number: 834,
    ),
    GT::GrammarRule.new(
      name: "walrus_expr",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "or_expr", is_token: false),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::Literal.new(value: "if"),
              GT::RuleReference.new(name: "or_expr", is_token: false),
              GT::Literal.new(value: "else"),
              GT::RuleReference.new(name: "expression", is_token: false),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "or_expr", is_token: false),
          GT::RuleReference.new(name: "WALRUS", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
      ]),
      line_number: 837,
    ),
    GT::GrammarRule.new(
      name: "lambda_expr",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "lambda"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "lambda_params", is_token: false)),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 844,
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
      line_number: 845,
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
      ]),
      line_number: 846,
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
      line_number: 849,
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
      line_number: 852,
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
      line_number: 855,
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
      line_number: 862,
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
      line_number: 864,
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
      line_number: 871,
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
      line_number: 872,
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
      line_number: 873,
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
      line_number: 876,
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
      line_number: 879,
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
      line_number: 884,
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
      line_number: 887,
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
      line_number: 892,
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
      line_number: 897,
    ),
    GT::GrammarRule.new(
      name: "primary",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "atom", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "suffix", is_token: false)),
      ]),
      line_number: 906,
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
      line_number: 908,
    ),
    GT::GrammarRule.new(
      name: "subscript",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "subscript_item", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "subscript_item", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
      ]),
      line_number: 916,
    ),
    GT::GrammarRule.new(
      name: "subscript_item",
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
      line_number: 918,
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
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "ELLIPSIS", is_token: true),
        GT::Literal.new(value: "True"),
        GT::Literal.new(value: "False"),
        GT::Literal.new(value: "None"),
        GT::RuleReference.new(name: "list_expr", is_token: false),
        GT::RuleReference.new(name: "dict_or_set_expr", is_token: false),
        GT::RuleReference.new(name: "paren_expr", is_token: false),
      ]),
      line_number: 927,
    ),
    GT::GrammarRule.new(
      name: "list_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "list_body", is_token: false)),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 944,
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
      line_number: 946,
    ),
    GT::GrammarRule.new(
      name: "dict_or_set_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "dict_or_set_body", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 958,
    ),
    GT::GrammarRule.new(
      name: "dict_or_set_body",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "dict_body", is_token: false),
        GT::RuleReference.new(name: "set_body", is_token: false),
      ]),
      line_number: 960,
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
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOUBLE_STAR", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::Group.new(element: GT::Alternation.new(choices: [
                  GT::RuleReference.new(name: "dict_entry", is_token: false),
                  GT::Sequence.new(elements: [
                    GT::RuleReference.new(name: "DOUBLE_STAR", is_token: true),
                    GT::RuleReference.new(name: "expression", is_token: false),
                  ]),
                ])),
            ])),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        ]),
      ]),
      line_number: 962,
    ),
    GT::GrammarRule.new(
      name: "dict_entry",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 966,
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
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
            ])),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        ]),
      ]),
      line_number: 968,
    ),
    GT::GrammarRule.new(
      name: "paren_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "paren_body", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 977,
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
      line_number: 979,
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
      line_number: 999,
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
      line_number: 1001,
    ),
    GT::GrammarRule.new(
      name: "comp_if",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "if"),
        GT::RuleReference.new(name: "or_expr", is_token: false),
      ]),
      line_number: 1003,
    ),
    GT::GrammarRule.new(
      name: "target",
      body: GT::RuleReference.new(name: "primary", is_token: false),
      line_number: 1020,
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
      line_number: 1021,
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
      line_number: 1039,
    ),
    GT::GrammarRule.new(
      name: "argument",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "comp_clause", is_token: false),
        ]),
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
      line_number: 1041,
    ),
    GT::GrammarRule.new(
      name: "parameters",
      body: GT::RuleReference.new(name: "parameter_list", is_token: false),
      line_number: 1068,
    ),
    GT::GrammarRule.new(
      name: "parameter_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "param_with_default", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "param_with_default", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::OptionalElement.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "slash_params", is_token: false),
                GT::RuleReference.new(name: "star_params", is_token: false),
                GT::RuleReference.new(name: "double_star_param", is_token: false),
              ])),
          ])),
      ]),
      line_number: 1070,
    ),
    GT::GrammarRule.new(
      name: "slash_params",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "SLASH", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "param_with_default", is_token: false),
                GT::Repetition.new(element: GT::Sequence.new(elements: [
                    GT::RuleReference.new(name: "COMMA", is_token: true),
                    GT::RuleReference.new(name: "param_with_default", is_token: false),
                  ])),
                GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                    GT::RuleReference.new(name: "COMMA", is_token: true),
                    GT::OptionalElement.new(element: GT::Alternation.new(choices: [
                        GT::RuleReference.new(name: "star_params", is_token: false),
                        GT::RuleReference.new(name: "double_star_param", is_token: false),
                      ])),
                  ])),
              ])),
          ])),
      ]),
      line_number: 1073,
    ),
    GT::GrammarRule.new(
      name: "star_params",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "param_with_default", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "param_with_default", is_token: false),
              ])),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "double_star_param", is_token: false)),
          ])),
      ]),
      line_number: 1076,
    ),
    GT::GrammarRule.new(
      name: "double_star_param",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "DOUBLE_STAR", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
      ]),
      line_number: 1079,
    ),
    GT::GrammarRule.new(
      name: "param_with_default",
      body: GT::Sequence.new(elements: [
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
      line_number: 1081,
    ),
  ],
)
