# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: mosaic.grammar
# Regenerate with: grammar-tools compile-grammar mosaic.grammar
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
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "import_decl", is_token: false)),
        GT::RuleReference.new(name: "component_decl", is_token: false),
      ]),
      line_number: 20,
    ),
    GT::GrammarRule.new(
      name: "import_decl",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "KEYWORD", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "KEYWORD", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
        GT::RuleReference.new(name: "KEYWORD", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 30,
    ),
    GT::GrammarRule.new(
      name: "component_decl",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "KEYWORD", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "slot_decl", is_token: false)),
        GT::RuleReference.new(name: "node_tree", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 48,
    ),
    GT::GrammarRule.new(
      name: "slot_decl",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "KEYWORD", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "slot_type", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "default_value", is_token: false),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 67,
    ),
    GT::GrammarRule.new(
      name: "slot_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "list_type", is_token: false),
        GT::RuleReference.new(name: "KEYWORD", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 69,
    ),
    GT::GrammarRule.new(
      name: "list_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "KEYWORD", is_token: true),
        GT::RuleReference.new(name: "LANGLE", is_token: true),
        GT::RuleReference.new(name: "slot_type", is_token: false),
        GT::RuleReference.new(name: "RANGLE", is_token: true),
      ]),
      line_number: 73,
    ),
    GT::GrammarRule.new(
      name: "default_value",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "DIMENSION", is_token: true),
        GT::RuleReference.new(name: "COLOR_HEX", is_token: true),
        GT::RuleReference.new(name: "KEYWORD", is_token: true),
      ]),
      line_number: 75,
    ),
    GT::GrammarRule.new(
      name: "node_tree",
      body: GT::RuleReference.new(name: "node_element", is_token: false),
      line_number: 86,
    ),
    GT::GrammarRule.new(
      name: "node_element",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "node_content", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 88,
    ),
    GT::GrammarRule.new(
      name: "node_content",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "property_assignment", is_token: false),
        GT::RuleReference.new(name: "child_node", is_token: false),
        GT::RuleReference.new(name: "slot_reference", is_token: false),
        GT::RuleReference.new(name: "when_block", is_token: false),
        GT::RuleReference.new(name: "each_block", is_token: false),
      ]),
      line_number: 90,
    ),
    GT::GrammarRule.new(
      name: "property_assignment",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "KEYWORD", is_token: true),
          ])),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "property_value", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 107,
    ),
    GT::GrammarRule.new(
      name: "property_value",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "slot_ref", is_token: false),
        GT::RuleReference.new(name: "enum_value", is_token: false),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "DIMENSION", is_token: true),
        GT::RuleReference.new(name: "COLOR_HEX", is_token: true),
        GT::RuleReference.new(name: "KEYWORD", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 111,
    ),
    GT::GrammarRule.new(
      name: "slot_ref",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "AT", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 122,
    ),
    GT::GrammarRule.new(
      name: "enum_value",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "DOT", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 124,
    ),
    GT::GrammarRule.new(
      name: "child_node",
      body: GT::RuleReference.new(name: "node_element", is_token: false),
      line_number: 131,
    ),
    GT::GrammarRule.new(
      name: "slot_reference",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "AT", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 144,
    ),
    GT::GrammarRule.new(
      name: "when_block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "KEYWORD", is_token: true),
        GT::RuleReference.new(name: "slot_ref", is_token: false),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "node_content", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 156,
    ),
    GT::GrammarRule.new(
      name: "each_block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "KEYWORD", is_token: true),
        GT::RuleReference.new(name: "slot_ref", is_token: false),
        GT::RuleReference.new(name: "KEYWORD", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "node_content", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 170,
    ),
  ],
)
