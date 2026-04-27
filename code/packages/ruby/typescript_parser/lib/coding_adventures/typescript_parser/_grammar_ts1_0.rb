# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: ts1.0.grammar
# Regenerate with: grammar-tools compile-grammar ts1.0.grammar
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
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "source_element", is_token: false)),
      line_number: 56,
    ),
    GT::GrammarRule.new(
      name: "source_element",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "interface_declaration", is_token: false),
        GT::RuleReference.new(name: "type_alias_declaration", is_token: false),
        GT::RuleReference.new(name: "enum_declaration", is_token: false),
        GT::RuleReference.new(name: "namespace_declaration", is_token: false),
        GT::RuleReference.new(name: "ambient_declaration", is_token: false),
        GT::RuleReference.new(name: "function_declaration", is_token: false),
        GT::RuleReference.new(name: "ts_class_declaration", is_token: false),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 65,
    ),
    GT::GrammarRule.new(
      name: "function_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "function"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "function_body", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 91,
    ),
    GT::GrammarRule.new(
      name: "function_body",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "source_element", is_token: false)),
      line_number: 95,
    ),
    GT::GrammarRule.new(
      name: "typed_parameter_list",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "typed_parameter", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "typed_parameter", is_token: false),
            ])),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "rest_typed_parameter", is_token: false),
            ])),
        ]),
        GT::RuleReference.new(name: "rest_typed_parameter", is_token: false),
      ]),
      line_number: 113,
    ),
    GT::GrammarRule.new(
      name: "typed_parameter",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "accessibility_modifier", is_token: false)),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "binding_pattern", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "QUESTION", is_token: true)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "assignment_expression", is_token: false),
          ])),
      ]),
      line_number: 116,
    ),
    GT::GrammarRule.new(
      name: "rest_typed_parameter",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "ELLIPSIS", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
      ]),
      line_number: 118,
    ),
    GT::GrammarRule.new(
      name: "variable_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "var"),
        GT::RuleReference.new(name: "variable_declaration_list", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 128,
    ),
    GT::GrammarRule.new(
      name: "variable_declaration_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "variable_declaration", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "variable_declaration", is_token: false),
          ])),
      ]),
      line_number: 130,
    ),
    GT::GrammarRule.new(
      name: "variable_declaration",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "assignment_expression", is_token: false),
          ])),
      ]),
      line_number: 132,
    ),
    GT::GrammarRule.new(
      name: "statement",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "block", is_token: false),
        GT::RuleReference.new(name: "variable_statement", is_token: false),
        GT::RuleReference.new(name: "empty_statement", is_token: false),
        GT::RuleReference.new(name: "if_statement", is_token: false),
        GT::RuleReference.new(name: "while_statement", is_token: false),
        GT::RuleReference.new(name: "do_while_statement", is_token: false),
        GT::RuleReference.new(name: "for_statement", is_token: false),
        GT::RuleReference.new(name: "for_in_statement", is_token: false),
        GT::RuleReference.new(name: "continue_statement", is_token: false),
        GT::RuleReference.new(name: "break_statement", is_token: false),
        GT::RuleReference.new(name: "return_statement", is_token: false),
        GT::RuleReference.new(name: "with_statement", is_token: false),
        GT::RuleReference.new(name: "switch_statement", is_token: false),
        GT::RuleReference.new(name: "labelled_statement", is_token: false),
        GT::RuleReference.new(name: "try_statement", is_token: false),
        GT::RuleReference.new(name: "throw_statement", is_token: false),
        GT::RuleReference.new(name: "debugger_statement", is_token: false),
        GT::RuleReference.new(name: "expression_statement", is_token: false),
      ]),
      line_number: 141,
    ),
    GT::GrammarRule.new(
      name: "block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 160,
    ),
    GT::GrammarRule.new(
      name: "empty_statement",
      body: GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      line_number: 162,
    ),
    GT::GrammarRule.new(
      name: "expression_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 164,
    ),
    GT::GrammarRule.new(
      name: "if_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "if"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "else"),
            GT::RuleReference.new(name: "statement", is_token: false),
          ])),
      ]),
      line_number: 166,
    ),
    GT::GrammarRule.new(
      name: "while_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "while"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 168,
    ),
    GT::GrammarRule.new(
      name: "do_while_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "do"),
        GT::RuleReference.new(name: "statement", is_token: false),
        GT::Literal.new(value: "while"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 170,
    ),
    GT::GrammarRule.new(
      name: "for_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "for"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "var"),
              GT::RuleReference.new(name: "variable_declaration_list", is_token: false),
            ]),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 172,
    ),
    GT::GrammarRule.new(
      name: "for_in_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "for"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "var"),
              GT::RuleReference.new(name: "variable_declaration", is_token: false),
            ]),
            GT::RuleReference.new(name: "left_hand_side_expression", is_token: false),
          ])),
        GT::Literal.new(value: "in"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 178,
    ),
    GT::GrammarRule.new(
      name: "continue_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "continue"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 182,
    ),
    GT::GrammarRule.new(
      name: "break_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "break"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 184,
    ),
    GT::GrammarRule.new(
      name: "return_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "return"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 186,
    ),
    GT::GrammarRule.new(
      name: "with_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "with"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 188,
    ),
    GT::GrammarRule.new(
      name: "switch_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "switch"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "case_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "default_clause", is_token: false),
            GT::Repetition.new(element: GT::RuleReference.new(name: "case_clause", is_token: false)),
          ])),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 190,
    ),
    GT::GrammarRule.new(
      name: "case_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "case"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
      ]),
      line_number: 193,
    ),
    GT::GrammarRule.new(
      name: "default_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "default"),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
      ]),
      line_number: 195,
    ),
    GT::GrammarRule.new(
      name: "labelled_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 197,
    ),
    GT::GrammarRule.new(
      name: "try_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "try"),
        GT::RuleReference.new(name: "block", is_token: false),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "catch_clause", is_token: false),
              GT::OptionalElement.new(element: GT::RuleReference.new(name: "finally_clause", is_token: false)),
            ]),
            GT::RuleReference.new(name: "finally_clause", is_token: false),
          ])),
      ]),
      line_number: 199,
    ),
    GT::GrammarRule.new(
      name: "catch_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "catch"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 201,
    ),
    GT::GrammarRule.new(
      name: "finally_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "finally"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 203,
    ),
    GT::GrammarRule.new(
      name: "throw_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "throw"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 205,
    ),
    GT::GrammarRule.new(
      name: "debugger_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "debugger"),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 207,
    ),
    GT::GrammarRule.new(
      name: "expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "assignment_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "assignment_expression", is_token: false),
          ])),
      ]),
      line_number: 229,
    ),
    GT::GrammarRule.new(
      name: "assignment_expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "ts_as_expression", is_token: false),
        GT::RuleReference.new(name: "ts_angle_bracket_assertion", is_token: false),
        GT::RuleReference.new(name: "conditional_expression", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "left_hand_side_expression", is_token: false),
          GT::RuleReference.new(name: "assignment_operator", is_token: false),
          GT::RuleReference.new(name: "assignment_expression", is_token: false),
        ]),
      ]),
      line_number: 241,
    ),
    GT::GrammarRule.new(
      name: "assignment_operator",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "PLUS_EQUALS", is_token: true),
        GT::RuleReference.new(name: "MINUS_EQUALS", is_token: true),
        GT::RuleReference.new(name: "STAR_EQUALS", is_token: true),
        GT::RuleReference.new(name: "SLASH_EQUALS", is_token: true),
        GT::RuleReference.new(name: "PERCENT_EQUALS", is_token: true),
        GT::RuleReference.new(name: "AMPERSAND_EQUALS", is_token: true),
        GT::RuleReference.new(name: "PIPE_EQUALS", is_token: true),
        GT::RuleReference.new(name: "CARET_EQUALS", is_token: true),
        GT::RuleReference.new(name: "LEFT_SHIFT_EQUALS", is_token: true),
        GT::RuleReference.new(name: "RIGHT_SHIFT_EQUALS", is_token: true),
        GT::RuleReference.new(name: "UNSIGNED_RIGHT_SHIFT_EQUALS", is_token: true),
      ]),
      line_number: 246,
    ),
    GT::GrammarRule.new(
      name: "ts_as_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "ts_non_null_expression", is_token: false),
        GT::Literal.new(value: "as"),
        GT::RuleReference.new(name: "type_expression", is_token: false),
      ]),
      line_number: 255,
    ),
    GT::GrammarRule.new(
      name: "ts_non_null_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "left_hand_side_expression", is_token: false),
        GT::RuleReference.new(name: "BANG", is_token: true),
      ]),
      line_number: 260,
    ),
    GT::GrammarRule.new(
      name: "ts_angle_bracket_assertion",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LESS_THAN", is_token: true),
        GT::RuleReference.new(name: "type_expression", is_token: false),
        GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
        GT::RuleReference.new(name: "assignment_expression", is_token: false),
      ]),
      line_number: 264,
    ),
    GT::GrammarRule.new(
      name: "conditional_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "logical_or_expression", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "QUESTION", is_token: true),
            GT::RuleReference.new(name: "assignment_expression", is_token: false),
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "assignment_expression", is_token: false),
          ])),
      ]),
      line_number: 266,
    ),
    GT::GrammarRule.new(
      name: "logical_or_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "logical_and_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "OR_OR", is_token: true),
            GT::RuleReference.new(name: "logical_and_expression", is_token: false),
          ])),
      ]),
      line_number: 269,
    ),
    GT::GrammarRule.new(
      name: "logical_and_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "bitwise_or_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "AND_AND", is_token: true),
            GT::RuleReference.new(name: "bitwise_or_expression", is_token: false),
          ])),
      ]),
      line_number: 271,
    ),
    GT::GrammarRule.new(
      name: "bitwise_or_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "bitwise_xor_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "PIPE", is_token: true),
            GT::RuleReference.new(name: "bitwise_xor_expression", is_token: false),
          ])),
      ]),
      line_number: 273,
    ),
    GT::GrammarRule.new(
      name: "bitwise_xor_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "bitwise_and_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "CARET", is_token: true),
            GT::RuleReference.new(name: "bitwise_and_expression", is_token: false),
          ])),
      ]),
      line_number: 275,
    ),
    GT::GrammarRule.new(
      name: "bitwise_and_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "equality_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "AMPERSAND", is_token: true),
            GT::RuleReference.new(name: "equality_expression", is_token: false),
          ])),
      ]),
      line_number: 277,
    ),
    GT::GrammarRule.new(
      name: "equality_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "relational_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "STRICT_EQUALS", is_token: true),
                GT::RuleReference.new(name: "STRICT_NOT_EQUALS", is_token: true),
                GT::RuleReference.new(name: "EQUALS_EQUALS", is_token: true),
                GT::RuleReference.new(name: "NOT_EQUALS", is_token: true),
              ])),
            GT::RuleReference.new(name: "relational_expression", is_token: false),
          ])),
      ]),
      line_number: 279,
    ),
    GT::GrammarRule.new(
      name: "relational_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "shift_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "LESS_THAN", is_token: true),
                GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
                GT::RuleReference.new(name: "LESS_EQUALS", is_token: true),
                GT::RuleReference.new(name: "GREATER_EQUALS", is_token: true),
                GT::Literal.new(value: "instanceof"),
                GT::Literal.new(value: "in"),
              ])),
            GT::RuleReference.new(name: "shift_expression", is_token: false),
          ])),
      ]),
      line_number: 283,
    ),
    GT::GrammarRule.new(
      name: "shift_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "additive_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "LEFT_SHIFT", is_token: true),
                GT::RuleReference.new(name: "RIGHT_SHIFT", is_token: true),
                GT::RuleReference.new(name: "UNSIGNED_RIGHT_SHIFT", is_token: true),
              ])),
            GT::RuleReference.new(name: "additive_expression", is_token: false),
          ])),
      ]),
      line_number: 287,
    ),
    GT::GrammarRule.new(
      name: "additive_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "multiplicative_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "PLUS", is_token: true),
                GT::RuleReference.new(name: "MINUS", is_token: true),
              ])),
            GT::RuleReference.new(name: "multiplicative_expression", is_token: false),
          ])),
      ]),
      line_number: 290,
    ),
    GT::GrammarRule.new(
      name: "multiplicative_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "unary_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "STAR", is_token: true),
                GT::RuleReference.new(name: "SLASH", is_token: true),
                GT::RuleReference.new(name: "PERCENT", is_token: true),
              ])),
            GT::RuleReference.new(name: "unary_expression", is_token: false),
          ])),
      ]),
      line_number: 293,
    ),
    GT::GrammarRule.new(
      name: "unary_expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "postfix_expression", is_token: false),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "delete"),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "void"),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "typeof"),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "PLUS_PLUS", is_token: true),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "MINUS_MINUS", is_token: true),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "PLUS", is_token: true),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "MINUS", is_token: true),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "TILDE", is_token: true),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "BANG", is_token: true),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
      ]),
      line_number: 296,
    ),
    GT::GrammarRule.new(
      name: "postfix_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "left_hand_side_expression", is_token: false),
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "PLUS_PLUS", is_token: true),
            GT::RuleReference.new(name: "MINUS_MINUS", is_token: true),
          ])),
      ]),
      line_number: 307,
    ),
    GT::GrammarRule.new(
      name: "left_hand_side_expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "call_expression", is_token: false),
        GT::RuleReference.new(name: "new_expression", is_token: false),
      ]),
      line_number: 309,
    ),
    GT::GrammarRule.new(
      name: "call_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "member_expression", is_token: false),
        GT::RuleReference.new(name: "arguments", is_token: false),
        GT::Repetition.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "arguments", is_token: false),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "DOT", is_token: true),
              GT::RuleReference.new(name: "NAME", is_token: true),
            ]),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LBRACKET", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::RuleReference.new(name: "RBRACKET", is_token: true),
            ]),
          ])),
      ]),
      line_number: 311,
    ),
    GT::GrammarRule.new(
      name: "new_expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "member_expression", is_token: false),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "new"),
          GT::RuleReference.new(name: "new_expression", is_token: false),
        ]),
      ]),
      line_number: 314,
    ),
    GT::GrammarRule.new(
      name: "member_expression",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "primary_expression", is_token: false),
          GT::Repetition.new(element: GT::Alternation.new(choices: [
              GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "DOT", is_token: true),
                GT::RuleReference.new(name: "NAME", is_token: true),
              ]),
              GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "LBRACKET", is_token: true),
                GT::RuleReference.new(name: "expression", is_token: false),
                GT::RuleReference.new(name: "RBRACKET", is_token: true),
              ]),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "new"),
          GT::RuleReference.new(name: "member_expression", is_token: false),
          GT::RuleReference.new(name: "arguments", is_token: false),
        ]),
      ]),
      line_number: 317,
    ),
    GT::GrammarRule.new(
      name: "arguments",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "argument_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 320,
    ),
    GT::GrammarRule.new(
      name: "argument_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "assignment_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "assignment_expression", is_token: false),
          ])),
      ]),
      line_number: 322,
    ),
    GT::GrammarRule.new(
      name: "primary_expression",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "this"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "REGEX", is_token: true),
        GT::Literal.new(value: "true"),
        GT::Literal.new(value: "false"),
        GT::Literal.new(value: "null"),
        GT::RuleReference.new(name: "array_literal", is_token: false),
        GT::RuleReference.new(name: "object_literal", is_token: false),
        GT::RuleReference.new(name: "function_expression", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 324,
    ),
    GT::GrammarRule.new(
      name: "array_literal",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "element_list", is_token: false)),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 337,
    ),
    GT::GrammarRule.new(
      name: "element_list",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "assignment_expression", is_token: false)),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "assignment_expression", is_token: false)),
          ])),
      ]),
      line_number: 339,
    ),
    GT::GrammarRule.new(
      name: "object_literal",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "property_assignment", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "property_assignment", is_token: false),
              ])),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
          ])),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 344,
    ),
    GT::GrammarRule.new(
      name: "property_assignment",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "getter_property", is_token: false),
        GT::RuleReference.new(name: "setter_property", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::RuleReference.new(name: "assignment_expression", is_token: false),
        ]),
      ]),
      line_number: 346,
    ),
    GT::GrammarRule.new(
      name: "getter_property",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "function_body", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 351,
    ),
    GT::GrammarRule.new(
      name: "setter_property",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "function_body", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 354,
    ),
    GT::GrammarRule.new(
      name: "property_name",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
      ]),
      line_number: 356,
    ),
    GT::GrammarRule.new(
      name: "function_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "function"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "function_body", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 361,
    ),
    GT::GrammarRule.new(
      name: "binding_pattern",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "object_binding_pattern", is_token: false),
        GT::RuleReference.new(name: "array_binding_pattern", is_token: false),
      ]),
      line_number: 372,
    ),
    GT::GrammarRule.new(
      name: "object_binding_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "binding_property", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "binding_property", is_token: false),
              ])),
          ])),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 374,
    ),
    GT::GrammarRule.new(
      name: "binding_property",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 376,
    ),
    GT::GrammarRule.new(
      name: "array_binding_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "binding_element", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "binding_element", is_token: false),
              ])),
          ])),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 379,
    ),
    GT::GrammarRule.new(
      name: "binding_element",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "binding_pattern", is_token: false),
      ]),
      line_number: 381,
    ),
    GT::GrammarRule.new(
      name: "type_annotation",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "type_expression", is_token: false),
      ]),
      line_number: 410,
    ),
    GT::GrammarRule.new(
      name: "type_expression",
      body: GT::RuleReference.new(name: "union_type", is_token: false),
      line_number: 428,
    ),
    GT::GrammarRule.new(
      name: "union_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "intersection_type", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "PIPE", is_token: true),
            GT::RuleReference.new(name: "intersection_type", is_token: false),
          ])),
      ]),
      line_number: 436,
    ),
    GT::GrammarRule.new(
      name: "intersection_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "array_type", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "AMPERSAND", is_token: true),
            GT::RuleReference.new(name: "array_type", is_token: false),
          ])),
      ]),
      line_number: 444,
    ),
    GT::GrammarRule.new(
      name: "array_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "primary_type", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LBRACKET", is_token: true),
            GT::RuleReference.new(name: "RBRACKET", is_token: true),
          ])),
      ]),
      line_number: 451,
    ),
    GT::GrammarRule.new(
      name: "primary_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "predefined_type", is_token: false),
        GT::RuleReference.new(name: "type_reference", is_token: false),
        GT::RuleReference.new(name: "literal_type", is_token: false),
        GT::RuleReference.new(name: "object_type", is_token: false),
        GT::RuleReference.new(name: "tuple_type", is_token: false),
        GT::RuleReference.new(name: "function_type", is_token: false),
        GT::RuleReference.new(name: "constructor_type", is_token: false),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "typeof"),
          GT::RuleReference.new(name: "left_hand_side_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "keyof"),
          GT::RuleReference.new(name: "type_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "unique"),
          GT::Literal.new(value: "symbol"),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "type_expression", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 456,
    ),
    GT::GrammarRule.new(
      name: "predefined_type",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "any"),
        GT::Literal.new(value: "string"),
        GT::Literal.new(value: "number"),
        GT::Literal.new(value: "boolean"),
        GT::Literal.new(value: "void"),
        GT::Literal.new(value: "object"),
        GT::Literal.new(value: "symbol"),
        GT::Literal.new(value: "undefined"),
        GT::Literal.new(value: "null"),
      ]),
      line_number: 484,
    ),
    GT::GrammarRule.new(
      name: "literal_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::Literal.new(value: "true"),
        GT::Literal.new(value: "false"),
      ]),
      line_number: 498,
    ),
    GT::GrammarRule.new(
      name: "type_reference",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "DOT", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_arguments", is_token: false)),
      ]),
      line_number: 510,
    ),
    GT::GrammarRule.new(
      name: "type_arguments",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LESS_THAN", is_token: true),
        GT::RuleReference.new(name: "type_argument_list", is_token: false),
        GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
      ]),
      line_number: 520,
    ),
    GT::GrammarRule.new(
      name: "type_argument_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
      ]),
      line_number: 521,
    ),
    GT::GrammarRule.new(
      name: "type_parameters",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LESS_THAN", is_token: true),
        GT::RuleReference.new(name: "type_parameter_list", is_token: false),
        GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
      ]),
      line_number: 538,
    ),
    GT::GrammarRule.new(
      name: "type_parameter_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type_parameter", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "type_parameter", is_token: false),
          ])),
      ]),
      line_number: 539,
    ),
    GT::GrammarRule.new(
      name: "type_parameter",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "extends"),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
      ]),
      line_number: 540,
    ),
    GT::GrammarRule.new(
      name: "object_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "type_member_semicolon", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 557,
    ),
    GT::GrammarRule.new(
      name: "type_member_semicolon",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type_member", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 558,
    ),
    GT::GrammarRule.new(
      name: "type_member",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "construct_signature", is_token: false),
        GT::RuleReference.new(name: "call_signature", is_token: false),
        GT::RuleReference.new(name: "index_signature", is_token: false),
        GT::RuleReference.new(name: "method_signature", is_token: false),
        GT::RuleReference.new(name: "property_signature", is_token: false),
      ]),
      line_number: 559,
    ),
    GT::GrammarRule.new(
      name: "property_signature",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Literal.new(value: "readonly")),
        GT::RuleReference.new(name: "property_name", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "QUESTION", is_token: true)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
      ]),
      line_number: 567,
    ),
    GT::GrammarRule.new(
      name: "index_signature",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "type_expression", is_token: false),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "type_expression", is_token: false),
      ]),
      line_number: 572,
    ),
    GT::GrammarRule.new(
      name: "method_signature",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "property_name", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "QUESTION", is_token: true)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
      ]),
      line_number: 576,
    ),
    GT::GrammarRule.new(
      name: "call_signature",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
      ]),
      line_number: 580,
    ),
    GT::GrammarRule.new(
      name: "construct_signature",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "new"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
      ]),
      line_number: 584,
    ),
    GT::GrammarRule.new(
      name: "tuple_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "tuple_element_list", is_token: false)),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 598,
    ),
    GT::GrammarRule.new(
      name: "tuple_element_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "tuple_element", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "tuple_element", is_token: false),
          ])),
      ]),
      line_number: 599,
    ),
    GT::GrammarRule.new(
      name: "tuple_element",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "ELLIPSIS", is_token: true)),
        GT::RuleReference.new(name: "type_expression", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "QUESTION", is_token: true)),
      ]),
      line_number: 600,
    ),
    GT::GrammarRule.new(
      name: "function_type",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "ARROW", is_token: true),
        GT::RuleReference.new(name: "type_expression", is_token: false),
      ]),
      line_number: 618,
    ),
    GT::GrammarRule.new(
      name: "constructor_type",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "new"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "ARROW", is_token: true),
        GT::RuleReference.new(name: "type_expression", is_token: false),
      ]),
      line_number: 622,
    ),
    GT::GrammarRule.new(
      name: "interface_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "interface"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "interface_heritage", is_token: false)),
        GT::RuleReference.new(name: "object_type", is_token: false),
      ]),
      line_number: 654,
    ),
    GT::GrammarRule.new(
      name: "interface_heritage",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "extends"),
        GT::RuleReference.new(name: "type_reference", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "type_reference", is_token: false),
          ])),
      ]),
      line_number: 655,
    ),
    GT::GrammarRule.new(
      name: "type_alias_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "type"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "type_expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 671,
    ),
    GT::GrammarRule.new(
      name: "enum_declaration",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Literal.new(value: "const")),
        GT::Literal.new(value: "enum"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "enum_body", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 695,
    ),
    GT::GrammarRule.new(
      name: "enum_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "enum_member", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "enum_member", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
      ]),
      line_number: 696,
    ),
    GT::GrammarRule.new(
      name: "enum_member",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "property_name", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "assignment_expression", is_token: false),
          ])),
      ]),
      line_number: 697,
    ),
    GT::GrammarRule.new(
      name: "namespace_declaration",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "namespace"),
            GT::Literal.new(value: "module"),
          ])),
        GT::RuleReference.new(name: "qualified_name", is_token: false),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "namespace_element", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 719,
    ),
    GT::GrammarRule.new(
      name: "qualified_name",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "DOT", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 720,
    ),
    GT::GrammarRule.new(
      name: "namespace_element",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "namespace_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_declaration", is_token: false),
        GT::RuleReference.new(name: "type_alias_declaration", is_token: false),
        GT::RuleReference.new(name: "ts_class_declaration", is_token: false),
        GT::RuleReference.new(name: "function_declaration", is_token: false),
        GT::RuleReference.new(name: "enum_declaration", is_token: false),
        GT::RuleReference.new(name: "variable_statement", is_token: false),
        GT::RuleReference.new(name: "export_assignment", is_token: false),
        GT::RuleReference.new(name: "export_namespace_element", is_token: false),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 724,
    ),
    GT::GrammarRule.new(
      name: "export_assignment",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "export"),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 737,
    ),
    GT::GrammarRule.new(
      name: "export_namespace_element",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "export"),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "namespace_declaration", is_token: false),
            GT::RuleReference.new(name: "interface_declaration", is_token: false),
            GT::RuleReference.new(name: "type_alias_declaration", is_token: false),
            GT::RuleReference.new(name: "ts_class_declaration", is_token: false),
            GT::RuleReference.new(name: "function_declaration", is_token: false),
            GT::RuleReference.new(name: "enum_declaration", is_token: false),
            GT::RuleReference.new(name: "variable_statement", is_token: false),
          ])),
      ]),
      line_number: 740,
    ),
    GT::GrammarRule.new(
      name: "ambient_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "declare"),
        GT::RuleReference.new(name: "ambient_declaration_body", is_token: false),
      ]),
      line_number: 769,
    ),
    GT::GrammarRule.new(
      name: "ambient_declaration_body",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "variable_statement", is_token: false),
        GT::RuleReference.new(name: "ambient_function_declaration", is_token: false),
        GT::RuleReference.new(name: "ts_class_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_declaration", is_token: false),
        GT::RuleReference.new(name: "type_alias_declaration", is_token: false),
        GT::RuleReference.new(name: "enum_declaration", is_token: false),
        GT::RuleReference.new(name: "namespace_declaration", is_token: false),
        GT::RuleReference.new(name: "ambient_module_declaration", is_token: false),
      ]),
      line_number: 770,
    ),
    GT::GrammarRule.new(
      name: "ambient_function_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "function"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 783,
    ),
    GT::GrammarRule.new(
      name: "ambient_module_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "module"),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "namespace_element", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 787,
    ),
    GT::GrammarRule.new(
      name: "ts_class_declaration",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Literal.new(value: "abstract")),
        GT::Literal.new(value: "class"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "ts_class_heritage", is_token: false)),
        GT::RuleReference.new(name: "ts_class_body", is_token: false),
      ]),
      line_number: 829,
    ),
    GT::GrammarRule.new(
      name: "ts_class_heritage",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "extends"),
            GT::RuleReference.new(name: "type_reference", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "implements"),
            GT::RuleReference.new(name: "type_reference", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "type_reference", is_token: false),
              ])),
          ])),
      ]),
      line_number: 834,
    ),
    GT::GrammarRule.new(
      name: "ts_class_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "ts_class_element", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 836,
    ),
    GT::GrammarRule.new(
      name: "ts_class_element",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "ts_method_definition", is_token: false),
        GT::RuleReference.new(name: "ts_property_declaration", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "index_signature", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 838,
    ),
    GT::GrammarRule.new(
      name: "ts_method_definition",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "accessibility_modifier", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "abstract")),
        GT::OptionalElement.new(element: GT::Literal.new(value: "static")),
        GT::OptionalElement.new(element: GT::Literal.new(value: "readonly")),
        GT::RuleReference.new(name: "ts_method_definition_body", is_token: false),
      ]),
      line_number: 844,
    ),
    GT::GrammarRule.new(
      name: "accessibility_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "protected"),
      ]),
      line_number: 850,
    ),
    GT::GrammarRule.new(
      name: "ts_method_definition_body",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "QUESTION", is_token: true)),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::RuleReference.new(name: "type_expression", is_token: false),
            ])),
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "get"),
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::RuleReference.new(name: "type_expression", is_token: false),
            ])),
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "set"),
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "typed_parameter", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "STAR", is_token: true),
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "QUESTION", is_token: true)),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::RuleReference.new(name: "type_expression", is_token: false),
            ])),
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
      ]),
      line_number: 853,
    ),
    GT::GrammarRule.new(
      name: "ts_property_declaration",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "accessibility_modifier", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "static")),
        GT::OptionalElement.new(element: GT::Literal.new(value: "abstract")),
        GT::OptionalElement.new(element: GT::Literal.new(value: "readonly")),
        GT::RuleReference.new(name: "property_name", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "QUESTION", is_token: true)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "assignment_expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 860,
    ),
    GT::GrammarRule.new(
      name: "type_predicate",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "is"),
        GT::RuleReference.new(name: "type_expression", is_token: false),
      ]),
      line_number: 883,
    ),
  ],
)
