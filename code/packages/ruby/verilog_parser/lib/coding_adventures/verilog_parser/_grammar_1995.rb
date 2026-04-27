# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: verilog.grammar
# Regenerate with: grammar-tools compile-grammar verilog.grammar
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
      name: "source_text",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "description", is_token: false)),
      line_number: 42,
    ),
    GT::GrammarRule.new(
      name: "description",
      body: GT::RuleReference.new(name: "module_declaration", is_token: false),
      line_number: 44,
    ),
    GT::GrammarRule.new(
      name: "module_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "module"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "parameter_port_list", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "port_list", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "module_item", is_token: false)),
        GT::Literal.new(value: "endmodule"),
      ]),
      line_number: 73,
    ),
    GT::GrammarRule.new(
      name: "parameter_port_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "HASH", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "parameter_declaration", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "parameter_declaration", is_token: false),
          ])),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 91,
    ),
    GT::GrammarRule.new(
      name: "parameter_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "parameter"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "range", is_token: false)),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 94,
    ),
    GT::GrammarRule.new(
      name: "localparam_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "localparam"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "range", is_token: false)),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 95,
    ),
    GT::GrammarRule.new(
      name: "port_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "port", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "port", is_token: false),
          ])),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 115,
    ),
    GT::GrammarRule.new(
      name: "port",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "port_direction", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "net_type", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "signed")),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "range", is_token: false)),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 117,
    ),
    GT::GrammarRule.new(
      name: "port_direction",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "input"),
        GT::Literal.new(value: "output"),
        GT::Literal.new(value: "inout"),
      ]),
      line_number: 119,
    ),
    GT::GrammarRule.new(
      name: "net_type",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "wire"),
        GT::Literal.new(value: "reg"),
        GT::Literal.new(value: "tri"),
        GT::Literal.new(value: "supply0"),
        GT::Literal.new(value: "supply1"),
      ]),
      line_number: 120,
    ),
    GT::GrammarRule.new(
      name: "range",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 122,
    ),
    GT::GrammarRule.new(
      name: "module_item",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "port_declaration", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "net_declaration", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "reg_declaration", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "integer_declaration", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "parameter_declaration", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "localparam_declaration", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::RuleReference.new(name: "continuous_assign", is_token: false),
        GT::RuleReference.new(name: "always_construct", is_token: false),
        GT::RuleReference.new(name: "initial_construct", is_token: false),
        GT::RuleReference.new(name: "module_instantiation", is_token: false),
        GT::RuleReference.new(name: "generate_region", is_token: false),
        GT::RuleReference.new(name: "function_declaration", is_token: false),
        GT::RuleReference.new(name: "task_declaration", is_token: false),
      ]),
      line_number: 139,
    ),
    GT::GrammarRule.new(
      name: "port_declaration",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "port_direction", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "net_type", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "signed")),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "range", is_token: false)),
        GT::RuleReference.new(name: "name_list", is_token: false),
      ]),
      line_number: 174,
    ),
    GT::GrammarRule.new(
      name: "net_declaration",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "net_type", is_token: false),
        GT::OptionalElement.new(element: GT::Literal.new(value: "signed")),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "range", is_token: false)),
        GT::RuleReference.new(name: "name_list", is_token: false),
      ]),
      line_number: 176,
    ),
    GT::GrammarRule.new(
      name: "reg_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "reg"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "signed")),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "range", is_token: false)),
        GT::RuleReference.new(name: "name_list", is_token: false),
      ]),
      line_number: 177,
    ),
    GT::GrammarRule.new(
      name: "integer_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "integer"),
        GT::RuleReference.new(name: "name_list", is_token: false),
      ]),
      line_number: 178,
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
      line_number: 179,
    ),
    GT::GrammarRule.new(
      name: "continuous_assign",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "assign"),
        GT::RuleReference.new(name: "assignment", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "assignment", is_token: false),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 198,
    ),
    GT::GrammarRule.new(
      name: "assignment",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "lvalue", is_token: false),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 199,
    ),
    GT::GrammarRule.new(
      name: "lvalue",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "range_select", is_token: false)),
        ]),
        GT::RuleReference.new(name: "concatenation", is_token: false),
      ]),
      line_number: 203,
    ),
    GT::GrammarRule.new(
      name: "range_select",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 206,
    ),
    GT::GrammarRule.new(
      name: "always_construct",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "always"),
        GT::RuleReference.new(name: "AT", is_token: true),
        GT::RuleReference.new(name: "sensitivity_list", is_token: false),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 243,
    ),
    GT::GrammarRule.new(
      name: "initial_construct",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "initial"),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 244,
    ),
    GT::GrammarRule.new(
      name: "sensitivity_list",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "sensitivity_item", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::Group.new(element: GT::Alternation.new(choices: [
                  GT::Literal.new(value: "or"),
                  GT::RuleReference.new(name: "COMMA", is_token: true),
                ])),
              GT::RuleReference.new(name: "sensitivity_item", is_token: false),
            ])),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "STAR", is_token: true),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 246,
    ),
    GT::GrammarRule.new(
      name: "sensitivity_item",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "posedge"),
            GT::Literal.new(value: "negedge"),
          ])),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 250,
    ),
    GT::GrammarRule.new(
      name: "statement",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "block_statement", is_token: false),
        GT::RuleReference.new(name: "if_statement", is_token: false),
        GT::RuleReference.new(name: "case_statement", is_token: false),
        GT::RuleReference.new(name: "for_statement", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "blocking_assignment", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "nonblocking_assignment", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "task_call", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 259,
    ),
    GT::GrammarRule.new(
      name: "block_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "begin"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
        GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
        GT::Literal.new(value: "end"),
      ]),
      line_number: 275,
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
      line_number: 286,
    ),
    GT::GrammarRule.new(
      name: "case_statement",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "case"),
            GT::Literal.new(value: "casex"),
            GT::Literal.new(value: "casez"),
          ])),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "case_item", is_token: false)),
        GT::Literal.new(value: "endcase"),
      ]),
      line_number: 301,
    ),
    GT::GrammarRule.new(
      name: "case_item",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "expression_list", is_token: false),
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::RuleReference.new(name: "statement", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "default"),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "COLON", is_token: true)),
          GT::RuleReference.new(name: "statement", is_token: false),
        ]),
      ]),
      line_number: 306,
    ),
    GT::GrammarRule.new(
      name: "expression_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
      ]),
      line_number: 309,
    ),
    GT::GrammarRule.new(
      name: "for_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "for"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "blocking_assignment", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::RuleReference.new(name: "blocking_assignment", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 313,
    ),
    GT::GrammarRule.new(
      name: "blocking_assignment",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "lvalue", is_token: false),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 317,
    ),
    GT::GrammarRule.new(
      name: "nonblocking_assignment",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "lvalue", is_token: false),
        GT::RuleReference.new(name: "LESS_EQUALS", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 318,
    ),
    GT::GrammarRule.new(
      name: "task_call",
      body: GT::Sequence.new(elements: [
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
      line_number: 321,
    ),
    GT::GrammarRule.new(
      name: "module_instantiation",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "parameter_value_assignment", is_token: false)),
        GT::RuleReference.new(name: "instance", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "instance", is_token: false),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 340,
    ),
    GT::GrammarRule.new(
      name: "parameter_value_assignment",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "HASH", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 343,
    ),
    GT::GrammarRule.new(
      name: "instance",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "port_connections", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 345,
    ),
    GT::GrammarRule.new(
      name: "port_connections",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "named_port_connection", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "named_port_connection", is_token: false),
            ])),
        ]),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "expression", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "expression", is_token: false),
              ])),
          ])),
      ]),
      line_number: 347,
    ),
    GT::GrammarRule.new(
      name: "named_port_connection",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "DOT", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 350,
    ),
    GT::GrammarRule.new(
      name: "generate_region",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "generate"),
        GT::Repetition.new(element: GT::RuleReference.new(name: "generate_item", is_token: false)),
        GT::Literal.new(value: "endgenerate"),
      ]),
      line_number: 377,
    ),
    GT::GrammarRule.new(
      name: "generate_item",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "genvar_declaration", is_token: false),
        GT::RuleReference.new(name: "generate_for", is_token: false),
        GT::RuleReference.new(name: "generate_if", is_token: false),
        GT::RuleReference.new(name: "module_item", is_token: false),
      ]),
      line_number: 379,
    ),
    GT::GrammarRule.new(
      name: "genvar_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "genvar"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 384,
    ),
    GT::GrammarRule.new(
      name: "generate_for",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "for"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "genvar_assignment", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::RuleReference.new(name: "genvar_assignment", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "generate_block", is_token: false),
      ]),
      line_number: 386,
    ),
    GT::GrammarRule.new(
      name: "generate_if",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "if"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "generate_block", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "else"),
            GT::RuleReference.new(name: "generate_block", is_token: false),
          ])),
      ]),
      line_number: 390,
    ),
    GT::GrammarRule.new(
      name: "generate_block",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "begin"),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::RuleReference.new(name: "NAME", is_token: true),
            ])),
          GT::Repetition.new(element: GT::RuleReference.new(name: "generate_item", is_token: false)),
          GT::Literal.new(value: "end"),
        ]),
        GT::RuleReference.new(name: "generate_item", is_token: false),
      ]),
      line_number: 393,
    ),
    GT::GrammarRule.new(
      name: "genvar_assignment",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 396,
    ),
    GT::GrammarRule.new(
      name: "function_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "function"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "range", is_token: false)),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "function_item", is_token: false)),
        GT::RuleReference.new(name: "statement", is_token: false),
        GT::Literal.new(value: "endfunction"),
      ]),
      line_number: 415,
    ),
    GT::GrammarRule.new(
      name: "function_item",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "port_declaration", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "reg_declaration", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "integer_declaration", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "parameter_declaration", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
      ]),
      line_number: 420,
    ),
    GT::GrammarRule.new(
      name: "task_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "task"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "task_item", is_token: false)),
        GT::RuleReference.new(name: "statement", is_token: false),
        GT::Literal.new(value: "endtask"),
      ]),
      line_number: 425,
    ),
    GT::GrammarRule.new(
      name: "task_item",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "port_declaration", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "reg_declaration", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "integer_declaration", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
      ]),
      line_number: 430,
    ),
    GT::GrammarRule.new(
      name: "expression",
      body: GT::RuleReference.new(name: "ternary_expr", is_token: false),
      line_number: 458,
    ),
    GT::GrammarRule.new(
      name: "ternary_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "or_expr", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "QUESTION", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "ternary_expr", is_token: false),
          ])),
      ]),
      line_number: 464,
    ),
    GT::GrammarRule.new(
      name: "or_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "and_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LOGIC_OR", is_token: true),
            GT::RuleReference.new(name: "and_expr", is_token: false),
          ])),
      ]),
      line_number: 467,
    ),
    GT::GrammarRule.new(
      name: "and_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "bit_or_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LOGIC_AND", is_token: true),
            GT::RuleReference.new(name: "bit_or_expr", is_token: false),
          ])),
      ]),
      line_number: 468,
    ),
    GT::GrammarRule.new(
      name: "bit_or_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "bit_xor_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "PIPE", is_token: true),
            GT::RuleReference.new(name: "bit_xor_expr", is_token: false),
          ])),
      ]),
      line_number: 471,
    ),
    GT::GrammarRule.new(
      name: "bit_xor_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "bit_and_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "CARET", is_token: true),
            GT::RuleReference.new(name: "bit_and_expr", is_token: false),
          ])),
      ]),
      line_number: 472,
    ),
    GT::GrammarRule.new(
      name: "bit_and_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "equality_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "AMP", is_token: true),
            GT::RuleReference.new(name: "equality_expr", is_token: false),
          ])),
      ]),
      line_number: 473,
    ),
    GT::GrammarRule.new(
      name: "equality_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "relational_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "EQUALS_EQUALS", is_token: true),
                GT::RuleReference.new(name: "NOT_EQUALS", is_token: true),
                GT::RuleReference.new(name: "CASE_EQ", is_token: true),
                GT::RuleReference.new(name: "CASE_NEQ", is_token: true),
              ])),
            GT::RuleReference.new(name: "relational_expr", is_token: false),
          ])),
      ]),
      line_number: 477,
    ),
    GT::GrammarRule.new(
      name: "relational_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "shift_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "LESS_THAN", is_token: true),
                GT::RuleReference.new(name: "LESS_EQUALS", is_token: true),
                GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
                GT::RuleReference.new(name: "GREATER_EQUALS", is_token: true),
              ])),
            GT::RuleReference.new(name: "shift_expr", is_token: false),
          ])),
      ]),
      line_number: 484,
    ),
    GT::GrammarRule.new(
      name: "shift_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "additive_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "LEFT_SHIFT", is_token: true),
                GT::RuleReference.new(name: "RIGHT_SHIFT", is_token: true),
                GT::RuleReference.new(name: "ARITH_LEFT_SHIFT", is_token: true),
                GT::RuleReference.new(name: "ARITH_RIGHT_SHIFT", is_token: true),
              ])),
            GT::RuleReference.new(name: "additive_expr", is_token: false),
          ])),
      ]),
      line_number: 489,
    ),
    GT::GrammarRule.new(
      name: "additive_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "multiplicative_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "PLUS", is_token: true),
                GT::RuleReference.new(name: "MINUS", is_token: true),
              ])),
            GT::RuleReference.new(name: "multiplicative_expr", is_token: false),
          ])),
      ]),
      line_number: 494,
    ),
    GT::GrammarRule.new(
      name: "multiplicative_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "power_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "STAR", is_token: true),
                GT::RuleReference.new(name: "SLASH", is_token: true),
                GT::RuleReference.new(name: "PERCENT", is_token: true),
              ])),
            GT::RuleReference.new(name: "power_expr", is_token: false),
          ])),
      ]),
      line_number: 495,
    ),
    GT::GrammarRule.new(
      name: "power_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "unary_expr", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "POWER", is_token: true),
            GT::RuleReference.new(name: "unary_expr", is_token: false),
          ])),
      ]),
      line_number: 496,
    ),
    GT::GrammarRule.new(
      name: "unary_expr",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::RuleReference.new(name: "PLUS", is_token: true),
              GT::RuleReference.new(name: "MINUS", is_token: true),
              GT::RuleReference.new(name: "BANG", is_token: true),
              GT::RuleReference.new(name: "TILDE", is_token: true),
              GT::RuleReference.new(name: "AMP", is_token: true),
              GT::RuleReference.new(name: "PIPE", is_token: true),
              GT::RuleReference.new(name: "CARET", is_token: true),
              GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "TILDE", is_token: true),
                GT::RuleReference.new(name: "AMP", is_token: true),
              ]),
              GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "TILDE", is_token: true),
                GT::RuleReference.new(name: "PIPE", is_token: true),
              ]),
              GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "TILDE", is_token: true),
                GT::RuleReference.new(name: "CARET", is_token: true),
              ]),
            ])),
          GT::RuleReference.new(name: "unary_expr", is_token: false),
        ]),
        GT::RuleReference.new(name: "primary", is_token: false),
      ]),
      line_number: 508,
    ),
    GT::GrammarRule.new(
      name: "primary",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "SIZED_NUMBER", is_token: true),
        GT::RuleReference.new(name: "REAL_NUMBER", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "SYSTEM_ID", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::RuleReference.new(name: "concatenation", is_token: false),
        GT::RuleReference.new(name: "replication", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "primary", is_token: false),
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
            ])),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
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
      ]),
      line_number: 518,
    ),
    GT::GrammarRule.new(
      name: "concatenation",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 534,
    ),
    GT::GrammarRule.new(
      name: "replication",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "concatenation", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 540,
    ),
  ],
)
