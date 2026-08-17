# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: css.grammar
# Regenerate with: grammar-tools compile-grammar css.grammar
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
      name: "stylesheet",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "rule", is_token: false)),
      line_number: 33,
    ),
    GT::GrammarRule.new(
      name: "rule",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "at_rule", is_token: false),
        GT::RuleReference.new(name: "qualified_rule", is_token: false),
      ]),
      line_number: 35,
    ),
    GT::GrammarRule.new(
      name: "at_rule",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "AT_KEYWORD", is_token: true),
        GT::RuleReference.new(name: "at_prelude", is_token: false),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            GT::RuleReference.new(name: "block", is_token: false),
          ])),
      ]),
      line_number: 55,
    ),
    GT::GrammarRule.new(
      name: "at_prelude",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "at_prelude_token", is_token: false)),
      line_number: 61,
    ),
    GT::GrammarRule.new(
      name: "at_prelude_token",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "IDENT", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "DIMENSION", is_token: true),
        GT::RuleReference.new(name: "PERCENTAGE", is_token: true),
        GT::RuleReference.new(name: "HASH", is_token: true),
        GT::RuleReference.new(name: "CUSTOM_PROPERTY", is_token: true),
        GT::RuleReference.new(name: "UNICODE_RANGE", is_token: true),
        GT::RuleReference.new(name: "function_in_prelude", is_token: false),
        GT::RuleReference.new(name: "paren_block", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "COMMA", is_token: true),
        GT::RuleReference.new(name: "SLASH", is_token: true),
        GT::RuleReference.new(name: "DOT", is_token: true),
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::RuleReference.new(name: "PLUS", is_token: true),
        GT::RuleReference.new(name: "MINUS", is_token: true),
        GT::RuleReference.new(name: "GREATER", is_token: true),
        GT::RuleReference.new(name: "TILDE", is_token: true),
        GT::RuleReference.new(name: "PIPE", is_token: true),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "AMPERSAND", is_token: true),
        GT::RuleReference.new(name: "CDO", is_token: true),
        GT::RuleReference.new(name: "CDC", is_token: true),
      ]),
      line_number: 63,
    ),
    GT::GrammarRule.new(
      name: "function_in_prelude",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "FUNCTION", is_token: true),
        GT::RuleReference.new(name: "at_prelude_tokens", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 71,
    ),
    GT::GrammarRule.new(
      name: "paren_block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "at_prelude_tokens", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 72,
    ),
    GT::GrammarRule.new(
      name: "at_prelude_tokens",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "at_prelude_token", is_token: false)),
      line_number: 73,
    ),
    GT::GrammarRule.new(
      name: "qualified_rule",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "selector_list", is_token: false),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 85,
    ),
    GT::GrammarRule.new(
      name: "selector_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "complex_selector", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "complex_selector", is_token: false),
          ])),
      ]),
      line_number: 96,
    ),
    GT::GrammarRule.new(
      name: "complex_selector",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "compound_selector", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "combinator", is_token: false)),
            GT::RuleReference.new(name: "compound_selector", is_token: false),
          ])),
      ]),
      line_number: 105,
    ),
    GT::GrammarRule.new(
      name: "combinator",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "GREATER", is_token: true),
        GT::RuleReference.new(name: "PLUS", is_token: true),
        GT::RuleReference.new(name: "TILDE", is_token: true),
      ]),
      line_number: 112,
    ),
    GT::GrammarRule.new(
      name: "compound_selector",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "simple_selector", is_token: false),
          GT::Repetition.new(element: GT::RuleReference.new(name: "subclass_selector", is_token: false)),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "subclass_selector", is_token: false),
          GT::Repetition.new(element: GT::RuleReference.new(name: "subclass_selector", is_token: false)),
        ]),
      ]),
      line_number: 124,
    ),
    GT::GrammarRule.new(
      name: "simple_selector",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "IDENT", is_token: true),
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::RuleReference.new(name: "AMPERSAND", is_token: true),
      ]),
      line_number: 131,
    ),
    GT::GrammarRule.new(
      name: "subclass_selector",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "class_selector", is_token: false),
        GT::RuleReference.new(name: "id_selector", is_token: false),
        GT::RuleReference.new(name: "attribute_selector", is_token: false),
        GT::RuleReference.new(name: "pseudo_class", is_token: false),
        GT::RuleReference.new(name: "pseudo_element", is_token: false),
      ]),
      line_number: 139,
    ),
    GT::GrammarRule.new(
      name: "class_selector",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "DOT", is_token: true),
        GT::RuleReference.new(name: "IDENT", is_token: true),
      ]),
      line_number: 145,
    ),
    GT::GrammarRule.new(
      name: "id_selector",
      body: GT::RuleReference.new(name: "HASH", is_token: true),
      line_number: 150,
    ),
    GT::GrammarRule.new(
      name: "attribute_selector",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "IDENT", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "attr_matcher", is_token: false),
            GT::RuleReference.new(name: "attr_value", is_token: false),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "IDENT", is_token: true)),
          ])),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 161,
    ),
    GT::GrammarRule.new(
      name: "attr_matcher",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "TILDE_EQUALS", is_token: true),
        GT::RuleReference.new(name: "PIPE_EQUALS", is_token: true),
        GT::RuleReference.new(name: "CARET_EQUALS", is_token: true),
        GT::RuleReference.new(name: "DOLLAR_EQUALS", is_token: true),
        GT::RuleReference.new(name: "STAR_EQUALS", is_token: true),
      ]),
      line_number: 163,
    ),
    GT::GrammarRule.new(
      name: "attr_value",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "IDENT", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
      ]),
      line_number: 166,
    ),
    GT::GrammarRule.new(
      name: "pseudo_class",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::RuleReference.new(name: "FUNCTION", is_token: true),
          GT::RuleReference.new(name: "pseudo_class_args", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::RuleReference.new(name: "IDENT", is_token: true),
        ]),
      ]),
      line_number: 173,
    ),
    GT::GrammarRule.new(
      name: "pseudo_class_args",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "pseudo_class_arg", is_token: false)),
      line_number: 181,
    ),
    GT::GrammarRule.new(
      name: "pseudo_class_arg",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "IDENT", is_token: true),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "DIMENSION", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "HASH", is_token: true),
        GT::RuleReference.new(name: "PLUS", is_token: true),
        GT::RuleReference.new(name: "COMMA", is_token: true),
        GT::RuleReference.new(name: "DOT", is_token: true),
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "AMPERSAND", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "FUNCTION", is_token: true),
          GT::RuleReference.new(name: "pseudo_class_args", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::RuleReference.new(name: "pseudo_class_args", is_token: false),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
      ]),
      line_number: 183,
    ),
    GT::GrammarRule.new(
      name: "pseudo_element",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "COLON_COLON", is_token: true),
        GT::RuleReference.new(name: "IDENT", is_token: true),
      ]),
      line_number: 190,
    ),
    GT::GrammarRule.new(
      name: "block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "block_contents", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 200,
    ),
    GT::GrammarRule.new(
      name: "block_contents",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "block_item", is_token: false)),
      line_number: 202,
    ),
    GT::GrammarRule.new(
      name: "block_item",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "at_rule", is_token: false),
        GT::RuleReference.new(name: "declaration_or_nested", is_token: false),
      ]),
      line_number: 211,
    ),
    GT::GrammarRule.new(
      name: "declaration_or_nested",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "declaration", is_token: false),
        GT::RuleReference.new(name: "qualified_rule", is_token: false),
      ]),
      line_number: 217,
    ),
    GT::GrammarRule.new(
      name: "declaration",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "property", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "value_list", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "priority", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 231,
    ),
    GT::GrammarRule.new(
      name: "property",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "IDENT", is_token: true),
        GT::RuleReference.new(name: "CUSTOM_PROPERTY", is_token: true),
      ]),
      line_number: 233,
    ),
    GT::GrammarRule.new(
      name: "priority",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "BANG", is_token: true),
        GT::Literal.new(value: "important"),
      ]),
      line_number: 238,
    ),
    GT::GrammarRule.new(
      name: "value_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "value", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "value", is_token: false)),
      ]),
      line_number: 251,
    ),
    GT::GrammarRule.new(
      name: "value",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "DIMENSION", is_token: true),
        GT::RuleReference.new(name: "PERCENTAGE", is_token: true),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "IDENT", is_token: true),
        GT::RuleReference.new(name: "HASH", is_token: true),
        GT::RuleReference.new(name: "CUSTOM_PROPERTY", is_token: true),
        GT::RuleReference.new(name: "UNICODE_RANGE", is_token: true),
        GT::RuleReference.new(name: "function_call", is_token: false),
        GT::RuleReference.new(name: "SLASH", is_token: true),
        GT::RuleReference.new(name: "COMMA", is_token: true),
        GT::RuleReference.new(name: "PLUS", is_token: true),
        GT::RuleReference.new(name: "MINUS", is_token: true),
      ]),
      line_number: 253,
    ),
    GT::GrammarRule.new(
      name: "function_call",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "FUNCTION", is_token: true),
          GT::RuleReference.new(name: "function_args", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::RuleReference.new(name: "URL_TOKEN", is_token: true),
      ]),
      line_number: 267,
    ),
    GT::GrammarRule.new(
      name: "function_args",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "function_arg", is_token: false)),
      line_number: 272,
    ),
    GT::GrammarRule.new(
      name: "function_arg",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "DIMENSION", is_token: true),
        GT::RuleReference.new(name: "PERCENTAGE", is_token: true),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "IDENT", is_token: true),
        GT::RuleReference.new(name: "HASH", is_token: true),
        GT::RuleReference.new(name: "CUSTOM_PROPERTY", is_token: true),
        GT::RuleReference.new(name: "COMMA", is_token: true),
        GT::RuleReference.new(name: "SLASH", is_token: true),
        GT::RuleReference.new(name: "PLUS", is_token: true),
        GT::RuleReference.new(name: "MINUS", is_token: true),
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "FUNCTION", is_token: true),
          GT::RuleReference.new(name: "function_args", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 274,
    ),
  ],
)
