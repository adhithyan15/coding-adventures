# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: python3.8.grammar
# Regenerate with: grammar-tools compile-grammar python3.8.grammar
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
      name: "file_input",
      body: GT::Repetition.new(element: GT::Alternation.new(choices: [
          GT::RuleReference.new(name: "NEWLINE", is_token: true),
          GT::RuleReference.new(name: "statement", is_token: false),
        ])),
      line_number: 71,
    ),
    GT::GrammarRule.new(
      name: "single_input",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NEWLINE", is_token: true),
        GT::RuleReference.new(name: "simple_stmt", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "compound_stmt", is_token: false),
          GT::RuleReference.new(name: "NEWLINE", is_token: true),
        ]),
      ]),
      line_number: 72,
    ),
    GT::GrammarRule.new(
      name: "eval_input",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression_list", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "NEWLINE", is_token: true)),
      ]),
      line_number: 73,
    ),
    GT::GrammarRule.new(
      name: "statement",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "compound_stmt", is_token: false),
        GT::RuleReference.new(name: "simple_stmt", is_token: false),
      ]),
      line_number: 94,
    ),
    GT::GrammarRule.new(
      name: "simple_stmt",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "small_stmt", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            GT::RuleReference.new(name: "small_stmt", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "SEMICOLON", is_token: true)),
        GT::RuleReference.new(name: "NEWLINE", is_token: true),
      ]),
      line_number: 102,
    ),
    GT::GrammarRule.new(
      name: "small_stmt",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "return_stmt", is_token: false),
        GT::RuleReference.new(name: "raise_stmt", is_token: false),
        GT::RuleReference.new(name: "import_stmt", is_token: false),
        GT::RuleReference.new(name: "from_import_stmt", is_token: false),
        GT::RuleReference.new(name: "global_stmt", is_token: false),
        GT::RuleReference.new(name: "nonlocal_stmt", is_token: false),
        GT::RuleReference.new(name: "assert_stmt", is_token: false),
        GT::RuleReference.new(name: "del_stmt", is_token: false),
        GT::RuleReference.new(name: "break_stmt", is_token: false),
        GT::RuleReference.new(name: "continue_stmt", is_token: false),
        GT::RuleReference.new(name: "pass_stmt", is_token: false),
        GT::RuleReference.new(name: "yield_stmt", is_token: false),
        GT::RuleReference.new(name: "assign_stmt", is_token: false),
      ]),
      line_number: 104,
    ),
    GT::GrammarRule.new(
      name: "return_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "return"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression_list", is_token: false)),
      ]),
      line_number: 130,
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
      line_number: 140,
    ),
    GT::GrammarRule.new(
      name: "import_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "import"),
        GT::RuleReference.new(name: "dotted_as_names", is_token: false),
      ]),
      line_number: 149,
    ),
    GT::GrammarRule.new(
      name: "dotted_as_names",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "dotted_as_name", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "dotted_as_name", is_token: false),
          ])),
      ]),
      line_number: 151,
    ),
    GT::GrammarRule.new(
      name: "dotted_as_name",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "dotted_name", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "as"),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 152,
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
      line_number: 153,
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
            GT::Repetition.new(element: GT::RuleReference.new(name: "DOT", is_token: true)),
          ])),
        GT::Literal.new(value: "import"),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "STAR", is_token: true),
            GT::RuleReference.new(name: "import_as_names", is_token: false),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LPAREN", is_token: true),
              GT::RuleReference.new(name: "import_as_names", is_token: false),
              GT::RuleReference.new(name: "RPAREN", is_token: true),
            ]),
          ])),
      ]),
      line_number: 168,
    ),
    GT::GrammarRule.new(
      name: "import_as_names",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "import_as_name", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "import_as_name", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
      ]),
      line_number: 172,
    ),
    GT::GrammarRule.new(
      name: "import_as_name",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "as"),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 173,
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
      line_number: 185,
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
      line_number: 200,
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
      line_number: 210,
    ),
    GT::GrammarRule.new(
      name: "del_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "del"),
        GT::RuleReference.new(name: "target_list", is_token: false),
      ]),
      line_number: 219,
    ),
    GT::GrammarRule.new(
      name: "break_stmt",
      body: GT::Literal.new(value: "break"),
      line_number: 229,
    ),
    GT::GrammarRule.new(
      name: "continue_stmt",
      body: GT::Literal.new(value: "continue"),
      line_number: 230,
    ),
    GT::GrammarRule.new(
      name: "pass_stmt",
      body: GT::Literal.new(value: "pass"),
      line_number: 239,
    ),
    GT::GrammarRule.new(
      name: "yield_stmt",
      body: GT::RuleReference.new(name: "yield_expr", is_token: false),
      line_number: 250,
    ),
    GT::GrammarRule.new(
      name: "assign_stmt",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "expression_list", is_token: false),
          GT::RuleReference.new(name: "assign_tail", is_token: false),
        ]),
        GT::RuleReference.new(name: "expression_list", is_token: false),
      ]),
      line_number: 278,
    ),
    GT::GrammarRule.new(
      name: "assign_tail",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "EQUALS", is_token: true),
              GT::RuleReference.new(name: "expression_list", is_token: false),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "EQUALS", is_token: true),
              GT::RuleReference.new(name: "expression_list", is_token: false),
            ])),
          GT::RuleReference.new(name: "EQUALS", is_token: true),
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::RuleReference.new(name: "yield_expr", is_token: false),
              GT::RuleReference.new(name: "expression_list", is_token: false),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "augmented_assign_op", is_token: false),
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::RuleReference.new(name: "yield_expr", is_token: false),
              GT::RuleReference.new(name: "expression_list", is_token: false),
            ])),
        ]),
      ]),
      line_number: 281,
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
      line_number: 292,
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
      line_number: 314,
    ),
    GT::GrammarRule.new(
      name: "target",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "STAR", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 316,
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
        GT::RuleReference.new(name: "async_stmt", is_token: false),
        GT::RuleReference.new(name: "decorated", is_token: false),
      ]),
      line_number: 326,
    ),
    GT::GrammarRule.new(
      name: "if_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "if"),
        GT::RuleReference.new(name: "named_expression", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "suite", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "elif"),
            GT::RuleReference.new(name: "named_expression", is_token: false),
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "suite", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "else"),
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "suite", is_token: false),
          ])),
      ]),
      line_number: 354,
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
      line_number: 386,
    ),
    GT::GrammarRule.new(
      name: "while_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "while"),
        GT::RuleReference.new(name: "named_expression", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "suite", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "else"),
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "suite", is_token: false),
          ])),
      ]),
      line_number: 406,
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
      line_number: 448,
    ),
    GT::GrammarRule.new(
      name: "except_clauses",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "except_clause", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "except_clause", is_token: false)),
      ]),
      line_number: 452,
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
      line_number: 459,
    ),
    GT::GrammarRule.new(
      name: "with_stmt",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "with"),
        GT::RuleReference.new(name: "with_item", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "with_item", is_token: false),
          ])),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "suite", is_token: false),
      ]),
      line_number: 480,
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
      line_number: 482,
    ),
    GT::GrammarRule.new(
      name: "def_stmt",
      body: GT::Sequence.new(elements: [
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
      line_number: 512,
    ),
    GT::GrammarRule.new(
      name: "parameters",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "parameter_list", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
      ]),
      line_number: 552,
    ),
    GT::GrammarRule.new(
      name: "parameter_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "param_or_sep", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "param_or_sep", is_token: false),
          ])),
      ]),
      line_number: 563,
    ),
    GT::GrammarRule.new(
      name: "param_or_sep",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOUBLE_STAR", is_token: true),
          GT::RuleReference.new(name: "typed_param", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "STAR", is_token: true),
          GT::RuleReference.new(name: "typed_param", is_token: false),
        ]),
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::RuleReference.new(name: "SLASH", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "typed_param", is_token: false),
          GT::RuleReference.new(name: "EQUALS", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
        GT::RuleReference.new(name: "typed_param", is_token: false),
      ]),
      line_number: 565,
    ),
    GT::GrammarRule.new(
      name: "typed_param",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
      ]),
      line_number: 575,
    ),
    GT::GrammarRule.new(
      name: "class_stmt",
      body: GT::Sequence.new(elements: [
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
      line_number: 602,
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
      line_number: 621,
    ),
    GT::GrammarRule.new(
      name: "decorated",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "decorator", is_token: false)),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "def_stmt", is_token: false),
            GT::RuleReference.new(name: "class_stmt", is_token: false),
            GT::RuleReference.new(name: "async_stmt", is_token: false),
          ])),
      ]),
      line_number: 647,
    ),
    GT::GrammarRule.new(
      name: "decorator",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "AT", is_token: true),
        GT::RuleReference.new(name: "dotted_name", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "arguments", is_token: false)),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::RuleReference.new(name: "NEWLINE", is_token: true),
      ]),
      line_number: 649,
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
      line_number: 670,
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
      line_number: 709,
    ),
    GT::GrammarRule.new(
      name: "named_expression",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "COLONEQUALS", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 756,
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
      line_number: 763,
    ),
    GT::GrammarRule.new(
      name: "lambda_expr",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "lambda"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "lambda_params", is_token: false)),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 784,
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
      line_number: 786,
    ),
    GT::GrammarRule.new(
      name: "lambda_param",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOUBLE_STAR", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "STAR", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::RuleReference.new(name: "SLASH", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "EQUALS", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 788,
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
      line_number: 804,
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
      line_number: 811,
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
      line_number: 817,
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
      line_number: 838,
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
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "is"),
          GT::OptionalElement.new(element: GT::Literal.new(value: "not")),
        ]),
      ]),
      line_number: 840,
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
      line_number: 860,
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
      line_number: 861,
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
      line_number: 862,
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
      line_number: 867,
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
      line_number: 875,
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
      line_number: 890,
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
      line_number: 899,
    ),
    GT::GrammarRule.new(
      name: "await_expr",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Literal.new(value: "await")),
        GT::RuleReference.new(name: "primary", is_token: false),
      ]),
      line_number: 912,
    ),
    GT::GrammarRule.new(
      name: "primary",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "atom", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "suffix", is_token: false)),
      ]),
      line_number: 930,
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
          GT::RuleReference.new(name: "subscript_list", is_token: false),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "arguments", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 932,
    ),
    GT::GrammarRule.new(
      name: "subscript_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "subscript", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "subscript", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
      ]),
      line_number: 939,
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
      line_number: 946,
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
        GT::Literal.new(value: "True"),
        GT::Literal.new(value: "False"),
        GT::Literal.new(value: "None"),
        GT::RuleReference.new(name: "ELLIPSIS", is_token: true),
        GT::RuleReference.new(name: "list_expr", is_token: false),
        GT::RuleReference.new(name: "dict_expr", is_token: false),
        GT::RuleReference.new(name: "set_expr", is_token: false),
        GT::RuleReference.new(name: "paren_expr", is_token: false),
      ]),
      line_number: 956,
    ),
    GT::GrammarRule.new(
      name: "list_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "list_body", is_token: false)),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 995,
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
      line_number: 997,
    ),
    GT::GrammarRule.new(
      name: "dict_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "dict_body", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 1014,
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
      line_number: 1016,
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
      line_number: 1019,
    ),
    GT::GrammarRule.new(
      name: "set_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "set_body", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 1036,
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
      line_number: 1038,
    ),
    GT::GrammarRule.new(
      name: "paren_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "paren_body", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1055,
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
      line_number: 1057,
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
      line_number: 1087,
    ),
    GT::GrammarRule.new(
      name: "comp_for",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "for"),
        GT::RuleReference.new(name: "target_list", is_token: false),
        GT::Literal.new(value: "in"),
        GT::RuleReference.new(name: "or_expr", is_token: false),
      ]),
      line_number: 1089,
    ),
    GT::GrammarRule.new(
      name: "comp_if",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "if"),
        GT::RuleReference.new(name: "or_expr", is_token: false),
      ]),
      line_number: 1091,
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
      line_number: 1116,
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
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "comp_clause", is_token: false),
        ]),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 1118,
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
      line_number: 1147,
    ),
  ],
)
