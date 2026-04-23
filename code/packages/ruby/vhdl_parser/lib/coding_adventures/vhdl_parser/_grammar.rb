# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: vhdl.grammar
# Regenerate with: grammar-tools compile-grammar vhdl.grammar
#
# This file embeds a ParserGrammar as native Ruby data structures.
# Downstream packages require this file directly instead of reading
# and parsing the .grammar file at runtime.

require "coding_adventures_grammar_tools"

GT = CodingAdventures::GrammarTools unless defined?(GT)

PARSER_GRAMMAR = GT::ParserGrammar.new(
  version: 0,
  rules: [
    GT::GrammarRule.new(
      name: "design_file",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "design_unit", is_token: false)),
      line_number: 67,
    ),
    GT::GrammarRule.new(
      name: "design_unit",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "context_item", is_token: false)),
        GT::RuleReference.new(name: "library_unit", is_token: false),
      ]),
      line_number: 69,
    ),
    GT::GrammarRule.new(
      name: "context_item",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "library_clause", is_token: false),
        GT::RuleReference.new(name: "use_clause", is_token: false),
      ]),
      line_number: 71,
    ),
    GT::GrammarRule.new(
      name: "library_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "library"),
        GT::RuleReference.new(name: "name_list", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 74,
    ),
    GT::GrammarRule.new(
      name: "use_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "use"),
        GT::RuleReference.new(name: "selected_name", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 77,
    ),
    GT::GrammarRule.new(
      name: "selected_name",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "DOT", is_token: true),
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "NAME", is_token: true),
                GT::Literal.new(value: "all"),
              ])),
          ])),
      ]),
      line_number: 80,
    ),
    GT::GrammarRule.new(
      name: "name_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 82,
    ),
    GT::GrammarRule.new(
      name: "library_unit",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "entity_declaration", is_token: false),
        GT::RuleReference.new(name: "architecture_body", is_token: false),
        GT::RuleReference.new(name: "package_declaration", is_token: false),
        GT::RuleReference.new(name: "package_body", is_token: false),
      ]),
      line_number: 84,
    ),
    GT::GrammarRule.new(
      name: "entity_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "entity"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "is"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "generic_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "port_clause", is_token: false)),
        GT::Literal.new(value: "end"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "entity")),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 115,
    ),
    GT::GrammarRule.new(
      name: "generic_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "generic"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "interface_list", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 120,
    ),
    GT::GrammarRule.new(
      name: "port_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "port"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "interface_list", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 121,
    ),
    GT::GrammarRule.new(
      name: "interface_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "interface_element", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            GT::RuleReference.new(name: "interface_element", is_token: false),
          ])),
      ]),
      line_number: 126,
    ),
    GT::GrammarRule.new(
      name: "interface_element",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "name_list", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "mode", is_token: false)),
        GT::RuleReference.new(name: "subtype_indication", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "VAR_ASSIGN", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
      ]),
      line_number: 127,
    ),
    GT::GrammarRule.new(
      name: "mode",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "in"),
        GT::Literal.new(value: "out"),
        GT::Literal.new(value: "inout"),
        GT::Literal.new(value: "buffer"),
      ]),
      line_number: 135,
    ),
    GT::GrammarRule.new(
      name: "architecture_body",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "architecture"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "of"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "is"),
        GT::Repetition.new(element: GT::RuleReference.new(name: "block_declarative_item", is_token: false)),
        GT::Literal.new(value: "begin"),
        GT::Repetition.new(element: GT::RuleReference.new(name: "concurrent_statement", is_token: false)),
        GT::Literal.new(value: "end"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "architecture")),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 157,
    ),
    GT::GrammarRule.new(
      name: "block_declarative_item",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "signal_declaration", is_token: false),
        GT::RuleReference.new(name: "constant_declaration", is_token: false),
        GT::RuleReference.new(name: "type_declaration", is_token: false),
        GT::RuleReference.new(name: "subtype_declaration", is_token: false),
        GT::RuleReference.new(name: "component_declaration", is_token: false),
        GT::RuleReference.new(name: "function_declaration", is_token: false),
        GT::RuleReference.new(name: "function_body", is_token: false),
        GT::RuleReference.new(name: "procedure_declaration", is_token: false),
        GT::RuleReference.new(name: "procedure_body", is_token: false),
      ]),
      line_number: 163,
    ),
    GT::GrammarRule.new(
      name: "signal_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "signal"),
        GT::RuleReference.new(name: "name_list", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "subtype_indication", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "VAR_ASSIGN", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 192,
    ),
    GT::GrammarRule.new(
      name: "constant_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "constant"),
        GT::RuleReference.new(name: "name_list", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "subtype_indication", is_token: false),
        GT::RuleReference.new(name: "VAR_ASSIGN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 194,
    ),
    GT::GrammarRule.new(
      name: "variable_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "variable"),
        GT::RuleReference.new(name: "name_list", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "subtype_indication", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "VAR_ASSIGN", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 196,
    ),
    GT::GrammarRule.new(
      name: "type_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "type"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "is"),
        GT::RuleReference.new(name: "type_definition", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 221,
    ),
    GT::GrammarRule.new(
      name: "subtype_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "subtype"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "is"),
        GT::RuleReference.new(name: "subtype_indication", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 222,
    ),
    GT::GrammarRule.new(
      name: "type_definition",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "enumeration_type", is_token: false),
        GT::RuleReference.new(name: "array_type", is_token: false),
        GT::RuleReference.new(name: "record_type", is_token: false),
      ]),
      line_number: 224,
    ),
    GT::GrammarRule.new(
      name: "enumeration_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "CHAR_LITERAL", is_token: true),
          ])),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "NAME", is_token: true),
                GT::RuleReference.new(name: "CHAR_LITERAL", is_token: true),
              ])),
          ])),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 230,
    ),
    GT::GrammarRule.new(
      name: "array_type",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "array"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "index_constraint", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::Literal.new(value: "of"),
        GT::RuleReference.new(name: "subtype_indication", is_token: false),
      ]),
      line_number: 235,
    ),
    GT::GrammarRule.new(
      name: "index_constraint",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "discrete_range", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "discrete_range", is_token: false),
          ])),
      ]),
      line_number: 237,
    ),
    GT::GrammarRule.new(
      name: "discrete_range",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "subtype_indication", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::Literal.new(value: "to"),
              GT::Literal.new(value: "downto"),
            ])),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
      ]),
      line_number: 238,
    ),
    GT::GrammarRule.new(
      name: "record_type",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "record"),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "subtype_indication", is_token: false),
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          ])),
        GT::Literal.new(value: "end"),
        GT::Literal.new(value: "record"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
      ]),
      line_number: 242,
    ),
    GT::GrammarRule.new(
      name: "subtype_indication",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "selected_name", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "constraint", is_token: false)),
      ]),
      line_number: 250,
    ),
    GT::GrammarRule.new(
      name: "constraint",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::Literal.new(value: "to"),
              GT::Literal.new(value: "downto"),
            ])),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "range"),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::Literal.new(value: "to"),
              GT::Literal.new(value: "downto"),
            ])),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
      ]),
      line_number: 252,
    ),
    GT::GrammarRule.new(
      name: "concurrent_statement",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "process_statement", is_token: false),
        GT::RuleReference.new(name: "signal_assignment_concurrent", is_token: false),
        GT::RuleReference.new(name: "component_instantiation", is_token: false),
        GT::RuleReference.new(name: "generate_statement", is_token: false),
      ]),
      line_number: 267,
    ),
    GT::GrammarRule.new(
      name: "signal_assignment_concurrent",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LESS_EQUALS", is_token: true),
        GT::RuleReference.new(name: "waveform", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 275,
    ),
    GT::GrammarRule.new(
      name: "waveform",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "waveform_element", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "waveform_element", is_token: false),
          ])),
      ]),
      line_number: 277,
    ),
    GT::GrammarRule.new(
      name: "waveform_element",
      body: GT::RuleReference.new(name: "expression", is_token: false),
      line_number: 278,
    ),
    GT::GrammarRule.new(
      name: "process_statement",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "COLON", is_token: true),
          ])),
        GT::Literal.new(value: "process"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::RuleReference.new(name: "sensitivity_list", is_token: false),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::OptionalElement.new(element: GT::Literal.new(value: "is")),
        GT::Repetition.new(element: GT::RuleReference.new(name: "process_declarative_item", is_token: false)),
        GT::Literal.new(value: "begin"),
        GT::Repetition.new(element: GT::RuleReference.new(name: "sequential_statement", is_token: false)),
        GT::Literal.new(value: "end"),
        GT::Literal.new(value: "process"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 310,
    ),
    GT::GrammarRule.new(
      name: "sensitivity_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 318,
    ),
    GT::GrammarRule.new(
      name: "process_declarative_item",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "variable_declaration", is_token: false),
        GT::RuleReference.new(name: "constant_declaration", is_token: false),
        GT::RuleReference.new(name: "type_declaration", is_token: false),
        GT::RuleReference.new(name: "subtype_declaration", is_token: false),
      ]),
      line_number: 320,
    ),
    GT::GrammarRule.new(
      name: "sequential_statement",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "signal_assignment_seq", is_token: false),
        GT::RuleReference.new(name: "variable_assignment", is_token: false),
        GT::RuleReference.new(name: "if_statement", is_token: false),
        GT::RuleReference.new(name: "case_statement", is_token: false),
        GT::RuleReference.new(name: "loop_statement", is_token: false),
        GT::RuleReference.new(name: "return_statement", is_token: false),
        GT::RuleReference.new(name: "null_statement", is_token: false),
      ]),
      line_number: 332,
    ),
    GT::GrammarRule.new(
      name: "signal_assignment_seq",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LESS_EQUALS", is_token: true),
        GT::RuleReference.new(name: "waveform", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 345,
    ),
    GT::GrammarRule.new(
      name: "variable_assignment",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "VAR_ASSIGN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 349,
    ),
    GT::GrammarRule.new(
      name: "if_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "if"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Literal.new(value: "then"),
        GT::Repetition.new(element: GT::RuleReference.new(name: "sequential_statement", is_token: false)),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "elsif"),
            GT::RuleReference.new(name: "expression", is_token: false),
            GT::Literal.new(value: "then"),
            GT::Repetition.new(element: GT::RuleReference.new(name: "sequential_statement", is_token: false)),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "else"),
            GT::Repetition.new(element: GT::RuleReference.new(name: "sequential_statement", is_token: false)),
          ])),
        GT::Literal.new(value: "end"),
        GT::Literal.new(value: "if"),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 359,
    ),
    GT::GrammarRule.new(
      name: "case_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "case"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Literal.new(value: "is"),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "when"),
            GT::RuleReference.new(name: "choices", is_token: false),
            GT::RuleReference.new(name: "ARROW", is_token: true),
            GT::Repetition.new(element: GT::RuleReference.new(name: "sequential_statement", is_token: false)),
          ])),
        GT::Literal.new(value: "end"),
        GT::Literal.new(value: "case"),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 375,
    ),
    GT::GrammarRule.new(
      name: "choices",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "choice", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "PIPE", is_token: true),
            GT::RuleReference.new(name: "choice", is_token: false),
          ])),
      ]),
      line_number: 379,
    ),
    GT::GrammarRule.new(
      name: "choice",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "discrete_range", is_token: false),
        GT::Literal.new(value: "others"),
      ]),
      line_number: 380,
    ),
    GT::GrammarRule.new(
      name: "loop_statement",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "COLON", is_token: true),
          ])),
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "for"),
              GT::RuleReference.new(name: "NAME", is_token: true),
              GT::Literal.new(value: "in"),
              GT::RuleReference.new(name: "discrete_range", is_token: false),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "while"),
              GT::RuleReference.new(name: "expression", is_token: false),
            ]),
          ])),
        GT::Literal.new(value: "loop"),
        GT::Repetition.new(element: GT::RuleReference.new(name: "sequential_statement", is_token: false)),
        GT::Literal.new(value: "end"),
        GT::Literal.new(value: "loop"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 394,
    ),
    GT::GrammarRule.new(
      name: "return_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "return"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 401,
    ),
    GT::GrammarRule.new(
      name: "null_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "null"),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 402,
    ),
    GT::GrammarRule.new(
      name: "component_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "component"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Literal.new(value: "is")),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "generic_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "port_clause", is_token: false)),
        GT::Literal.new(value: "end"),
        GT::Literal.new(value: "component"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 428,
    ),
    GT::GrammarRule.new(
      name: "component_instantiation",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "entity"),
              GT::RuleReference.new(name: "selected_name", is_token: false),
              GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                  GT::RuleReference.new(name: "LPAREN", is_token: true),
                  GT::RuleReference.new(name: "NAME", is_token: true),
                  GT::RuleReference.new(name: "RPAREN", is_token: true),
                ])),
            ]),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "generic"),
            GT::Literal.new(value: "map"),
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::RuleReference.new(name: "association_list", is_token: false),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "port"),
            GT::Literal.new(value: "map"),
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::RuleReference.new(name: "association_list", is_token: false),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 433,
    ),
    GT::GrammarRule.new(
      name: "association_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "association_element", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "association_element", is_token: false),
          ])),
      ]),
      line_number: 440,
    ),
    GT::GrammarRule.new(
      name: "association_element",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "NAME", is_token: true),
              GT::RuleReference.new(name: "ARROW", is_token: true),
            ])),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "NAME", is_token: true),
              GT::RuleReference.new(name: "ARROW", is_token: true),
            ])),
          GT::Literal.new(value: "open"),
        ]),
      ]),
      line_number: 441,
    ),
    GT::GrammarRule.new(
      name: "generate_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "for_generate", is_token: false),
            GT::RuleReference.new(name: "if_generate", is_token: false),
          ])),
      ]),
      line_number: 464,
    ),
    GT::GrammarRule.new(
      name: "for_generate",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "for"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "in"),
        GT::RuleReference.new(name: "discrete_range", is_token: false),
        GT::Literal.new(value: "generate"),
        GT::Repetition.new(element: GT::RuleReference.new(name: "concurrent_statement", is_token: false)),
        GT::Literal.new(value: "end"),
        GT::Literal.new(value: "generate"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 466,
    ),
    GT::GrammarRule.new(
      name: "if_generate",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "if"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Literal.new(value: "generate"),
        GT::Repetition.new(element: GT::RuleReference.new(name: "concurrent_statement", is_token: false)),
        GT::Literal.new(value: "end"),
        GT::Literal.new(value: "generate"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 470,
    ),
    GT::GrammarRule.new(
      name: "package_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "package"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "is"),
        GT::Repetition.new(element: GT::RuleReference.new(name: "package_declarative_item", is_token: false)),
        GT::Literal.new(value: "end"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "package")),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 491,
    ),
    GT::GrammarRule.new(
      name: "package_body",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "package"),
        GT::Literal.new(value: "body"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "is"),
        GT::Repetition.new(element: GT::RuleReference.new(name: "package_body_declarative_item", is_token: false)),
        GT::Literal.new(value: "end"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "package"),
            GT::Literal.new(value: "body"),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 495,
    ),
    GT::GrammarRule.new(
      name: "package_declarative_item",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "type_declaration", is_token: false),
        GT::RuleReference.new(name: "subtype_declaration", is_token: false),
        GT::RuleReference.new(name: "constant_declaration", is_token: false),
        GT::RuleReference.new(name: "signal_declaration", is_token: false),
        GT::RuleReference.new(name: "component_declaration", is_token: false),
        GT::RuleReference.new(name: "function_declaration", is_token: false),
        GT::RuleReference.new(name: "procedure_declaration", is_token: false),
      ]),
      line_number: 499,
    ),
    GT::GrammarRule.new(
      name: "package_body_declarative_item",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "type_declaration", is_token: false),
        GT::RuleReference.new(name: "subtype_declaration", is_token: false),
        GT::RuleReference.new(name: "constant_declaration", is_token: false),
        GT::RuleReference.new(name: "function_body", is_token: false),
        GT::RuleReference.new(name: "procedure_body", is_token: false),
      ]),
      line_number: 507,
    ),
    GT::GrammarRule.new(
      name: "function_declaration",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "pure"),
            GT::Literal.new(value: "impure"),
          ])),
        GT::Literal.new(value: "function"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::RuleReference.new(name: "interface_list", is_token: false),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::Literal.new(value: "return"),
        GT::RuleReference.new(name: "subtype_indication", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 523,
    ),
    GT::GrammarRule.new(
      name: "function_body",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "pure"),
            GT::Literal.new(value: "impure"),
          ])),
        GT::Literal.new(value: "function"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::RuleReference.new(name: "interface_list", is_token: false),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::Literal.new(value: "return"),
        GT::RuleReference.new(name: "subtype_indication", is_token: false),
        GT::Literal.new(value: "is"),
        GT::Repetition.new(element: GT::RuleReference.new(name: "process_declarative_item", is_token: false)),
        GT::Literal.new(value: "begin"),
        GT::Repetition.new(element: GT::RuleReference.new(name: "sequential_statement", is_token: false)),
        GT::Literal.new(value: "end"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "function")),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 528,
    ),
    GT::GrammarRule.new(
      name: "procedure_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "procedure"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::RuleReference.new(name: "interface_list", is_token: false),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 537,
    ),
    GT::GrammarRule.new(
      name: "procedure_body",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "procedure"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::RuleReference.new(name: "interface_list", is_token: false),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::Literal.new(value: "is"),
        GT::Repetition.new(element: GT::RuleReference.new(name: "process_declarative_item", is_token: false)),
        GT::Literal.new(value: "begin"),
        GT::Repetition.new(element: GT::RuleReference.new(name: "sequential_statement", is_token: false)),
        GT::Literal.new(value: "end"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "procedure")),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 540,
    ),
    GT::GrammarRule.new(
      name: "expression",
      body: GT::RuleReference.new(name: "logical_expr", is_token: false),
      line_number: 577,
    ),
    GT::GrammarRule.new(
      name: "logical_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "relation", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "logical_op", is_token: false),
            GT::RuleReference.new(name: "relation", is_token: false),
          ])),
      ]),
      line_number: 584,
    ),
    GT::GrammarRule.new(
      name: "logical_op",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "and"),
        GT::Literal.new(value: "or"),
        GT::Literal.new(value: "xor"),
        GT::Literal.new(value: "nand"),
        GT::Literal.new(value: "nor"),
        GT::Literal.new(value: "xnor"),
      ]),
      line_number: 585,
    ),
    GT::GrammarRule.new(
      name: "relation",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "shift_expr", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "relational_op", is_token: false),
            GT::RuleReference.new(name: "shift_expr", is_token: false),
          ])),
      ]),
      line_number: 589,
    ),
    GT::GrammarRule.new(
      name: "relational_op",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "NOT_EQUALS", is_token: true),
        GT::RuleReference.new(name: "LESS_THAN", is_token: true),
        GT::RuleReference.new(name: "LESS_EQUALS", is_token: true),
        GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
        GT::RuleReference.new(name: "GREATER_EQUALS", is_token: true),
      ]),
      line_number: 590,
    ),
    GT::GrammarRule.new(
      name: "shift_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "adding_expr", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "shift_op", is_token: false),
            GT::RuleReference.new(name: "adding_expr", is_token: false),
          ])),
      ]),
      line_number: 595,
    ),
    GT::GrammarRule.new(
      name: "shift_op",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "sll"),
        GT::Literal.new(value: "srl"),
        GT::Literal.new(value: "sla"),
        GT::Literal.new(value: "sra"),
        GT::Literal.new(value: "rol"),
        GT::Literal.new(value: "ror"),
      ]),
      line_number: 596,
    ),
    GT::GrammarRule.new(
      name: "adding_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "multiplying_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "adding_op", is_token: false),
            GT::RuleReference.new(name: "multiplying_expr", is_token: false),
          ])),
      ]),
      line_number: 600,
    ),
    GT::GrammarRule.new(
      name: "adding_op",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "PLUS", is_token: true),
        GT::RuleReference.new(name: "MINUS", is_token: true),
        GT::RuleReference.new(name: "AMPERSAND", is_token: true),
      ]),
      line_number: 601,
    ),
    GT::GrammarRule.new(
      name: "multiplying_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "unary_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "multiplying_op", is_token: false),
            GT::RuleReference.new(name: "unary_expr", is_token: false),
          ])),
      ]),
      line_number: 604,
    ),
    GT::GrammarRule.new(
      name: "multiplying_op",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::RuleReference.new(name: "SLASH", is_token: true),
        GT::Literal.new(value: "mod"),
        GT::Literal.new(value: "rem"),
      ]),
      line_number: 605,
    ),
    GT::GrammarRule.new(
      name: "unary_expr",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "abs"),
          GT::RuleReference.new(name: "unary_expr", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "not"),
          GT::RuleReference.new(name: "unary_expr", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::RuleReference.new(name: "PLUS", is_token: true),
              GT::RuleReference.new(name: "MINUS", is_token: true),
            ])),
          GT::RuleReference.new(name: "unary_expr", is_token: false),
        ]),
        GT::RuleReference.new(name: "power_expr", is_token: false),
      ]),
      line_number: 608,
    ),
    GT::GrammarRule.new(
      name: "power_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "primary", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "POWER", is_token: true),
            GT::RuleReference.new(name: "primary", is_token: false),
          ])),
      ]),
      line_number: 614,
    ),
    GT::GrammarRule.new(
      name: "primary",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "REAL_NUMBER", is_token: true),
        GT::RuleReference.new(name: "BASED_LITERAL", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "CHAR_LITERAL", is_token: true),
        GT::RuleReference.new(name: "BIT_STRING", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "TICK", is_token: true),
              GT::RuleReference.new(name: "NAME", is_token: true),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::Repetition.new(element: GT::Sequence.new(elements: [
                  GT::RuleReference.new(name: "COMMA", is_token: true),
                  GT::RuleReference.new(name: "expression", is_token: false),
                ])),
            ])),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::RuleReference.new(name: "aggregate", is_token: false),
        GT::Literal.new(value: "null"),
      ]),
      line_number: 622,
    ),
    GT::GrammarRule.new(
      name: "aggregate",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "element_association", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "element_association", is_token: false),
          ])),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 638,
    ),
    GT::GrammarRule.new(
      name: "element_association",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "choices", is_token: false),
            GT::RuleReference.new(name: "ARROW", is_token: true),
          ])),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 639,
    ),
  ],
)
