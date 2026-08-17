# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: ruby.grammar
# Regenerate with: grammar-tools compile-grammar ruby.grammar
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
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
      line_number: 27,
    ),
    GT::GrammarRule.new(
      name: "statement",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "endless_def_statement", is_token: false),
        GT::RuleReference.new(name: "def_statement", is_token: false),
        GT::RuleReference.new(name: "class_statement", is_token: false),
        GT::RuleReference.new(name: "module_statement", is_token: false),
        GT::RuleReference.new(name: "if_statement", is_token: false),
        GT::RuleReference.new(name: "unless_statement", is_token: false),
        GT::RuleReference.new(name: "while_statement", is_token: false),
        GT::RuleReference.new(name: "until_statement", is_token: false),
        GT::RuleReference.new(name: "case_statement", is_token: false),
        GT::RuleReference.new(name: "begin_statement", is_token: false),
        GT::RuleReference.new(name: "return_statement", is_token: false),
        GT::RuleReference.new(name: "break_statement", is_token: false),
        GT::RuleReference.new(name: "next_statement", is_token: false),
        GT::RuleReference.new(name: "redo_statement", is_token: false),
        GT::RuleReference.new(name: "retry_statement", is_token: false),
        GT::RuleReference.new(name: "yield_statement", is_token: false),
        GT::RuleReference.new(name: "alias_statement", is_token: false),
        GT::RuleReference.new(name: "undef_statement", is_token: false),
        GT::RuleReference.new(name: "multi_assignment", is_token: false),
        GT::RuleReference.new(name: "modifier_statement", is_token: false),
        GT::RuleReference.new(name: "rightward_assignment", is_token: false),
        GT::RuleReference.new(name: "index_assignment", is_token: false),
        GT::RuleReference.new(name: "assignment", is_token: false),
        GT::RuleReference.new(name: "defined_expression", is_token: false),
        GT::RuleReference.new(name: "method_with_block", is_token: false),
        GT::RuleReference.new(name: "method_call", is_token: false),
        GT::RuleReference.new(name: "method_call_no_paren", is_token: false),
        GT::RuleReference.new(name: "expression_stmt", is_token: false),
      ]),
      line_number: 28,
    ),
    GT::GrammarRule.new(
      name: "multi_assignment",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "mlhs_target", is_token: false),
        GT::RuleReference.new(name: "COMMA", is_token: true),
        GT::RuleReference.new(name: "mlhs_target", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "mlhs_target", is_token: false),
          ])),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
      ]),
      line_number: 71,
    ),
    GT::GrammarRule.new(
      name: "mlhs_target",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Literal.new(value: "*")),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 72,
    ),
    GT::GrammarRule.new(
      name: "modifier_statement",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "assignment", is_token: false),
            GT::RuleReference.new(name: "method_call_no_paren", is_token: false),
            GT::RuleReference.new(name: "method_call", is_token: false),
            GT::RuleReference.new(name: "expression_stmt", is_token: false),
          ])),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "if_modifier"),
            GT::Literal.new(value: "unless_modifier"),
            GT::Literal.new(value: "while_modifier"),
            GT::Literal.new(value: "until_modifier"),
          ])),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 108,
    ),
    GT::GrammarRule.new(
      name: "def_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "def"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "def_receiver", is_token: false)),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "params", is_token: false)),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "rescue")),
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "ensure")),
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "end")),
            GT::RuleReference.new(name: "statement", is_token: false),
          ])),
        GT::Repetition.new(element: GT::RuleReference.new(name: "rescue_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "ensure_clause", is_token: false)),
        GT::Literal.new(value: "end"),
      ]),
      line_number: 132,
    ),
    GT::GrammarRule.new(
      name: "def_receiver",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "singleton_receiver", is_token: false),
        GT::Literal.new(value: "."),
      ]),
      line_number: 138,
    ),
    GT::GrammarRule.new(
      name: "endless_def_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "def"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "def_receiver", is_token: false)),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "params", is_token: false)),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 147,
    ),
    GT::GrammarRule.new(
      name: "class_statement",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "class"),
          GT::Literal.new(value: "<<"),
          GT::RuleReference.new(name: "singleton_receiver", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::NegativeLookahead.new(element: GT::Literal.new(value: "end")),
              GT::RuleReference.new(name: "statement", is_token: false),
            ])),
          GT::Literal.new(value: "end"),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "class"),
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::Literal.new(value: "<"),
              GT::RuleReference.new(name: "NAME", is_token: true),
            ])),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::NegativeLookahead.new(element: GT::Literal.new(value: "end")),
              GT::RuleReference.new(name: "statement", is_token: false),
            ])),
          GT::Literal.new(value: "end"),
        ]),
      ]),
      line_number: 168,
    ),
    GT::GrammarRule.new(
      name: "singleton_receiver",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "self"),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 170,
    ),
    GT::GrammarRule.new(
      name: "module_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "module"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "end")),
            GT::RuleReference.new(name: "statement", is_token: false),
          ])),
        GT::Literal.new(value: "end"),
      ]),
      line_number: 171,
    ),
    GT::GrammarRule.new(
      name: "method_with_block",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "KEYWORD", is_token: true),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "expression", is_token: false),
                GT::Repetition.new(element: GT::Sequence.new(elements: [
                    GT::RuleReference.new(name: "COMMA", is_token: true),
                    GT::RuleReference.new(name: "expression", is_token: false),
                  ])),
              ])),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 173,
    ),
    GT::GrammarRule.new(
      name: "block",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "do_block", is_token: false),
        GT::RuleReference.new(name: "brace_block", is_token: false),
      ]),
      line_number: 174,
    ),
    GT::GrammarRule.new(
      name: "do_block",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "do"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "block_params", is_token: false)),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "end")),
            GT::RuleReference.new(name: "statement", is_token: false),
          ])),
        GT::Literal.new(value: "end"),
      ]),
      line_number: 175,
    ),
    GT::GrammarRule.new(
      name: "brace_block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "block_params", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 176,
    ),
    GT::GrammarRule.new(
      name: "block_params",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "|"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: ";"),
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "NAME", is_token: true),
              ])),
          ])),
        GT::Literal.new(value: "|"),
      ]),
      line_number: 186,
    ),
    GT::GrammarRule.new(
      name: "return_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "return"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
      ]),
      line_number: 188,
    ),
    GT::GrammarRule.new(
      name: "break_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "break"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
      ]),
      line_number: 189,
    ),
    GT::GrammarRule.new(
      name: "next_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "next"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
      ]),
      line_number: 190,
    ),
    GT::GrammarRule.new(
      name: "redo_statement",
      body: GT::Literal.new(value: "redo"),
      line_number: 194,
    ),
    GT::GrammarRule.new(
      name: "retry_statement",
      body: GT::Literal.new(value: "retry"),
      line_number: 198,
    ),
    GT::GrammarRule.new(
      name: "alias_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "alias"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 209,
    ),
    GT::GrammarRule.new(
      name: "undef_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "undef"),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 221,
    ),
    GT::GrammarRule.new(
      name: "yield_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "yield"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "yield_args", is_token: false)),
      ]),
      line_number: 243,
    ),
    GT::GrammarRule.new(
      name: "yield_args",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "call_arg", is_token: false),
              GT::Repetition.new(element: GT::Sequence.new(elements: [
                  GT::RuleReference.new(name: "COMMA", is_token: true),
                  GT::RuleReference.new(name: "call_arg", is_token: false),
                ])),
            ])),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "call_arg", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "call_arg", is_token: false),
            ])),
        ]),
      ]),
      line_number: 244,
    ),
    GT::GrammarRule.new(
      name: "super_args",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "call_arg", is_token: false),
              GT::Repetition.new(element: GT::Sequence.new(elements: [
                  GT::RuleReference.new(name: "COMMA", is_token: true),
                  GT::RuleReference.new(name: "call_arg", is_token: false),
                ])),
            ])),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "call_arg", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "call_arg", is_token: false),
            ])),
        ]),
      ]),
      line_number: 271,
    ),
    GT::GrammarRule.new(
      name: "params",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "..."),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "param", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "param", is_token: false),
            ])),
        ]),
      ]),
      line_number: 300,
    ),
    GT::GrammarRule.new(
      name: "param",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "*"),
            GT::Literal.new(value: "**"),
          ])),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
            ]),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "EQUALS", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
            ]),
          ])),
      ]),
      line_number: 345,
    ),
    GT::GrammarRule.new(
      name: "if_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "if"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "else")),
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "elsif")),
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "end")),
            GT::RuleReference.new(name: "statement", is_token: false),
          ])),
        GT::Repetition.new(element: GT::RuleReference.new(name: "elsif_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "else_clause", is_token: false)),
        GT::Literal.new(value: "end"),
      ]),
      line_number: 346,
    ),
    GT::GrammarRule.new(
      name: "elsif_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "elsif"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "else")),
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "elsif")),
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "end")),
            GT::RuleReference.new(name: "statement", is_token: false),
          ])),
      ]),
      line_number: 347,
    ),
    GT::GrammarRule.new(
      name: "else_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "else"),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "end")),
            GT::RuleReference.new(name: "statement", is_token: false),
          ])),
      ]),
      line_number: 348,
    ),
    GT::GrammarRule.new(
      name: "unless_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "unless"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "else")),
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "end")),
            GT::RuleReference.new(name: "statement", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "else_clause", is_token: false)),
        GT::Literal.new(value: "end"),
      ]),
      line_number: 349,
    ),
    GT::GrammarRule.new(
      name: "while_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "while"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "end")),
            GT::RuleReference.new(name: "statement", is_token: false),
          ])),
        GT::Literal.new(value: "end"),
      ]),
      line_number: 350,
    ),
    GT::GrammarRule.new(
      name: "until_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "until"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "end")),
            GT::RuleReference.new(name: "statement", is_token: false),
          ])),
        GT::Literal.new(value: "end"),
      ]),
      line_number: 351,
    ),
    GT::GrammarRule.new(
      name: "case_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "case"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Repetition.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "when_clause", is_token: false),
            GT::RuleReference.new(name: "in_clause", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "else_clause", is_token: false)),
        GT::Literal.new(value: "end"),
      ]),
      line_number: 374,
    ),
    GT::GrammarRule.new(
      name: "when_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "when"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "when")),
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "in")),
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "else")),
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "end")),
            GT::RuleReference.new(name: "statement", is_token: false),
          ])),
      ]),
      line_number: 375,
    ),
    GT::GrammarRule.new(
      name: "in_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "in"),
        GT::RuleReference.new(name: "pattern", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "when")),
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "in")),
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "else")),
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "end")),
            GT::RuleReference.new(name: "statement", is_token: false),
          ])),
      ]),
      line_number: 397,
    ),
    GT::GrammarRule.new(
      name: "pattern",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "array_pattern", is_token: false),
        GT::RuleReference.new(name: "hash_pattern", is_token: false),
        GT::RuleReference.new(name: "class_pattern", is_token: false),
        GT::RuleReference.new(name: "pin_pattern", is_token: false),
        GT::RuleReference.new(name: "literal_pattern", is_token: false),
        GT::RuleReference.new(name: "binding_pattern", is_token: false),
      ]),
      line_number: 398,
    ),
    GT::GrammarRule.new(
      name: "literal_pattern",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "symbol_literal", is_token: false),
        GT::RuleReference.new(name: "KEYWORD", is_token: true),
      ]),
      line_number: 399,
    ),
    GT::GrammarRule.new(
      name: "binding_pattern",
      body: GT::RuleReference.new(name: "NAME", is_token: true),
      line_number: 400,
    ),
    GT::GrammarRule.new(
      name: "array_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "splat_pattern", is_token: false),
                GT::RuleReference.new(name: "pattern", is_token: false),
              ])),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::Group.new(element: GT::Alternation.new(choices: [
                    GT::RuleReference.new(name: "splat_pattern", is_token: false),
                    GT::RuleReference.new(name: "pattern", is_token: false),
                  ])),
              ])),
          ])),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 401,
    ),
    GT::GrammarRule.new(
      name: "hash_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "hash_pattern_pair", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "hash_pattern_pair", is_token: false),
              ])),
          ])),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 402,
    ),
    GT::GrammarRule.new(
      name: "hash_pattern_pair",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "pattern", is_token: false)),
      ]),
      line_number: 403,
    ),
    GT::GrammarRule.new(
      name: "splat_pattern",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "*"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
      ]),
      line_number: 410,
    ),
    GT::GrammarRule.new(
      name: "pin_pattern",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "^"),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 415,
    ),
    GT::GrammarRule.new(
      name: "class_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "pattern", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "pattern", is_token: false),
              ])),
          ])),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 421,
    ),
    GT::GrammarRule.new(
      name: "begin_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "begin"),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "rescue")),
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "ensure")),
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "end")),
            GT::RuleReference.new(name: "statement", is_token: false),
          ])),
        GT::Repetition.new(element: GT::RuleReference.new(name: "rescue_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "ensure_clause", is_token: false)),
        GT::Literal.new(value: "end"),
      ]),
      line_number: 442,
    ),
    GT::GrammarRule.new(
      name: "rescue_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "rescue"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "exception_list", is_token: false),
            GT::Literal.new(value: "=>"),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "rescue")),
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "ensure")),
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "end")),
            GT::RuleReference.new(name: "statement", is_token: false),
          ])),
      ]),
      line_number: 451,
    ),
    GT::GrammarRule.new(
      name: "exception_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 452,
    ),
    GT::GrammarRule.new(
      name: "ensure_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "ensure"),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::NegativeLookahead.new(element: GT::Literal.new(value: "end")),
            GT::RuleReference.new(name: "statement", is_token: false),
          ])),
      ]),
      line_number: 453,
    ),
    GT::GrammarRule.new(
      name: "index_write_receiver_postfix",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "dot_call", is_token: false),
        GT::RuleReference.new(name: "scope_resolution", is_token: false),
        GT::RuleReference.new(name: "index_suffix", is_token: false),
      ]),
      line_number: 506,
    ),
    GT::GrammarRule.new(
      name: "index_assignment",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "index_write_receiver_postfix", is_token: false),
            GT::PositiveLookahead.new(element: GT::RuleReference.new(name: "index_write_receiver_postfix", is_token: false)),
          ])),
        GT::RuleReference.new(name: "index_suffix", is_token: false),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 507,
    ),
    GT::GrammarRule.new(
      name: "assignment",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::Literal.new(value: "+="),
            GT::Literal.new(value: "-="),
            GT::Literal.new(value: "*="),
            GT::Literal.new(value: "/="),
            GT::Literal.new(value: "%="),
            GT::Literal.new(value: "**="),
            GT::Literal.new(value: "<<="),
            GT::Literal.new(value: ">>="),
            GT::Literal.new(value: "&="),
            GT::Literal.new(value: "|="),
            GT::Literal.new(value: "^="),
            GT::Literal.new(value: "||="),
            GT::Literal.new(value: "&&="),
          ])),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 508,
    ),
    GT::GrammarRule.new(
      name: "rightward_assignment",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Literal.new(value: "=>"),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 527,
    ),
    GT::GrammarRule.new(
      name: "method_call",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::Sequence.new(elements: [
              GT::NegativeLookahead.new(element: GT::Literal.new(value: "super")),
              GT::RuleReference.new(name: "KEYWORD", is_token: true),
            ]),
          ])),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "call_arg", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "call_arg", is_token: false),
              ])),
          ])),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "dot_call", is_token: false)),
      ]),
      line_number: 544,
    ),
    GT::GrammarRule.new(
      name: "dot_call",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "."),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "KEYWORD", is_token: true),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "call_arg", is_token: false),
                GT::Repetition.new(element: GT::Sequence.new(elements: [
                    GT::RuleReference.new(name: "COMMA", is_token: true),
                    GT::RuleReference.new(name: "call_arg", is_token: false),
                  ])),
              ])),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "block", is_token: false)),
      ]),
      line_number: 545,
    ),
    GT::GrammarRule.new(
      name: "scope_resolution",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "::"),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "KEYWORD", is_token: true),
          ])),
      ]),
      line_number: 553,
    ),
    GT::GrammarRule.new(
      name: "call_arg",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::OptionalElement.new(element: GT::Alternation.new(choices: [
              GT::Literal.new(value: "*"),
              GT::Literal.new(value: "**"),
              GT::Literal.new(value: "&"),
            ])),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
      ]),
      line_number: 608,
    ),
    GT::GrammarRule.new(
      name: "method_call_no_paren",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::Sequence.new(elements: [
              GT::NegativeLookahead.new(element: GT::Literal.new(value: "super")),
              GT::RuleReference.new(name: "KEYWORD", is_token: true),
            ]),
          ])),
        GT::NegativeLookahead.new(element: GT::Literal.new(value: "<")),
        GT::NegativeLookahead.new(element: GT::Literal.new(value: ">")),
        GT::NegativeLookahead.new(element: GT::Literal.new(value: "<=")),
        GT::NegativeLookahead.new(element: GT::Literal.new(value: ">=")),
        GT::NegativeLookahead.new(element: GT::Literal.new(value: "!=")),
        GT::NegativeLookahead.new(element: GT::Literal.new(value: "&&")),
        GT::NegativeLookahead.new(element: GT::Literal.new(value: "||")),
        GT::NegativeLookahead.new(element: GT::Literal.new(value: "<<")),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
      ]),
      line_number: 656,
    ),
    GT::GrammarRule.new(
      name: "expression_stmt",
      body: GT::RuleReference.new(name: "expression", is_token: false),
      line_number: 659,
    ),
    GT::GrammarRule.new(
      name: "expression",
      body: GT::RuleReference.new(name: "ternary", is_token: false),
      line_number: 766,
    ),
    GT::GrammarRule.new(
      name: "ternary",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "range", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "?"),
            GT::RuleReference.new(name: "expression", is_token: false),
            GT::Literal.new(value: ":"),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
      ]),
      line_number: 767,
    ),
    GT::GrammarRule.new(
      name: "range",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::Literal.new(value: "..."),
              GT::Literal.new(value: ".."),
            ])),
          GT::RuleReference.new(name: "logical_or", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "logical_or", is_token: false),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::Group.new(element: GT::Alternation.new(choices: [
                  GT::Literal.new(value: "..."),
                  GT::Literal.new(value: ".."),
                ])),
              GT::OptionalElement.new(element: GT::RuleReference.new(name: "logical_or", is_token: false)),
            ])),
        ]),
      ]),
      line_number: 768,
    ),
    GT::GrammarRule.new(
      name: "logical_or",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "logical_and", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::Literal.new(value: "||"),
                GT::Literal.new(value: "or"),
              ])),
            GT::RuleReference.new(name: "logical_and", is_token: false),
          ])),
      ]),
      line_number: 769,
    ),
    GT::GrammarRule.new(
      name: "logical_and",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "logical_not", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::Literal.new(value: "&&"),
                GT::Literal.new(value: "and"),
              ])),
            GT::RuleReference.new(name: "logical_not", is_token: false),
          ])),
      ]),
      line_number: 770,
    ),
    GT::GrammarRule.new(
      name: "logical_not",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::Group.new(element: GT::Alternation.new(choices: [
              GT::Literal.new(value: "!"),
              GT::Literal.new(value: "not"),
            ]))),
        GT::RuleReference.new(name: "comparison", is_token: false),
      ]),
      line_number: 777,
    ),
    GT::GrammarRule.new(
      name: "comparison",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "shift", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::Literal.new(value: "=="),
                GT::Literal.new(value: "!="),
                GT::Literal.new(value: "<="),
                GT::Literal.new(value: ">="),
                GT::Literal.new(value: "<"),
                GT::Literal.new(value: ">"),
              ])),
            GT::RuleReference.new(name: "shift", is_token: false),
          ])),
      ]),
      line_number: 793,
    ),
    GT::GrammarRule.new(
      name: "shift",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "sum", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "<<"),
            GT::RuleReference.new(name: "sum", is_token: false),
          ])),
      ]),
      line_number: 794,
    ),
    GT::GrammarRule.new(
      name: "sum",
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
      line_number: 795,
    ),
    GT::GrammarRule.new(
      name: "term",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "factor", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "STAR", is_token: true),
                GT::RuleReference.new(name: "SLASH", is_token: true),
              ])),
            GT::RuleReference.new(name: "factor", is_token: false),
          ])),
      ]),
      line_number: 796,
    ),
    GT::GrammarRule.new(
      name: "super_expr",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "super"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "super_args", is_token: false)),
      ]),
      line_number: 865,
    ),
    GT::GrammarRule.new(
      name: "index_suffix",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 877,
    ),
    GT::GrammarRule.new(
      name: "factor",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "defined_expression", is_token: false),
            GT::RuleReference.new(name: "lambda_literal", is_token: false),
            GT::RuleReference.new(name: "super_expr", is_token: false),
            GT::RuleReference.new(name: "method_call", is_token: false),
            GT::RuleReference.new(name: "NUMBER", is_token: true),
            GT::RuleReference.new(name: "STRING", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::Group.new(element: GT::Sequence.new(elements: [
                GT::NegativeLookahead.new(element: GT::Literal.new(value: "end")),
                GT::NegativeLookahead.new(element: GT::Literal.new(value: "rescue")),
                GT::NegativeLookahead.new(element: GT::Literal.new(value: "ensure")),
                GT::NegativeLookahead.new(element: GT::Literal.new(value: "else")),
                GT::NegativeLookahead.new(element: GT::Literal.new(value: "elsif")),
                GT::NegativeLookahead.new(element: GT::Literal.new(value: "when")),
                GT::NegativeLookahead.new(element: GT::Literal.new(value: "then")),
                GT::NegativeLookahead.new(element: GT::Literal.new(value: "in")),
                GT::NegativeLookahead.new(element: GT::Literal.new(value: "do")),
                GT::RuleReference.new(name: "KEYWORD", is_token: true),
              ])),
            GT::RuleReference.new(name: "symbol_literal", is_token: false),
            GT::RuleReference.new(name: "array_literal", is_token: false),
            GT::RuleReference.new(name: "hash_literal", is_token: false),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LPAREN", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::RuleReference.new(name: "RPAREN", is_token: true),
            ]),
            GT::RuleReference.new(name: "unary_minus", is_token: false),
          ])),
        GT::Repetition.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "dot_call", is_token: false),
            GT::RuleReference.new(name: "scope_resolution", is_token: false),
            GT::RuleReference.new(name: "index_suffix", is_token: false),
          ])),
      ]),
      line_number: 878,
    ),
    GT::GrammarRule.new(
      name: "lambda_literal",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "->"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "params", is_token: false)),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 897,
    ),
    GT::GrammarRule.new(
      name: "unary_minus",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "MINUS", is_token: true),
        GT::RuleReference.new(name: "factor", is_token: false),
      ]),
      line_number: 898,
    ),
    GT::GrammarRule.new(
      name: "defined_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "defined?"),
        GT::RuleReference.new(name: "factor", is_token: false),
      ]),
      line_number: 909,
    ),
    GT::GrammarRule.new(
      name: "symbol_literal",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: ":"),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "KEYWORD", is_token: true),
            GT::RuleReference.new(name: "STRING", is_token: true),
          ])),
      ]),
      line_number: 916,
    ),
    GT::GrammarRule.new(
      name: "array_literal",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "expression", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "expression", is_token: false),
              ])),
          ])),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 917,
    ),
    GT::GrammarRule.new(
      name: "hash_literal",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "hash_entry", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "hash_entry", is_token: false),
              ])),
          ])),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 918,
    ),
    GT::GrammarRule.new(
      name: "hash_entry",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "COLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::Literal.new(value: "=>"),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
      ]),
      line_number: 919,
    ),
  ],
)
