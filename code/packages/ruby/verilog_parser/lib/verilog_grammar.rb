# frozen_string_literal: true
# AUTO-GENERATED FILE - DO NOT EDIT
require "coding_adventures_grammar_tools"

module CodingAdventures
  module VerilogGrammar
    def self.grammar
      @grammar ||= CodingAdventures::GrammarTools::ParserGrammar.new(
        version: 0,
        rules: [
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "source_text",
            line_number: 42,
            body: CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "description", is_token: false))
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "description",
            line_number: 44,
            body: CodingAdventures::GrammarTools::RuleReference.new(name: "module_declaration", is_token: false)
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "module_declaration",
            line_number: 73,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "module"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "parameter_port_list", is_token: false)), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "port_list", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "module_item", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "endmodule")])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "parameter_port_list",
            line_number: 91,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "HASH", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "parameter_declaration", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "parameter_declaration", is_token: false)])), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "parameter_declaration",
            line_number: 94,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "parameter"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "range", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "localparam_declaration",
            line_number: 95,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "localparam"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "range", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "port_list",
            line_number: 115,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "port", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "port", is_token: false)])), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "port",
            line_number: 117,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "port_direction", is_token: false)), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "net_type", is_token: false)), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Literal.new(value: "signed")), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "range", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "port_direction",
            line_number: 119,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Literal.new(value: "input"), CodingAdventures::GrammarTools::Literal.new(value: "output"), CodingAdventures::GrammarTools::Literal.new(value: "inout")])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "net_type",
            line_number: 120,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Literal.new(value: "wire"), CodingAdventures::GrammarTools::Literal.new(value: "reg"), CodingAdventures::GrammarTools::Literal.new(value: "tri"), CodingAdventures::GrammarTools::Literal.new(value: "supply0"), CodingAdventures::GrammarTools::Literal.new(value: "supply1")])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "range",
            line_number: 122,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACKET", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACKET", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "module_item",
            line_number: 139,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "port_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "net_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "reg_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "integer_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "parameter_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "localparam_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)]), CodingAdventures::GrammarTools::RuleReference.new(name: "continuous_assign", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "always_construct", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "initial_construct", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "module_instantiation", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "generate_region", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "function_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "task_declaration", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "port_declaration",
            line_number: 174,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "port_direction", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "net_type", is_token: false)), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Literal.new(value: "signed")), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "range", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "name_list", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "net_declaration",
            line_number: 176,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "net_type", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Literal.new(value: "signed")), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "range", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "name_list", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "reg_declaration",
            line_number: 177,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "reg"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Literal.new(value: "signed")), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "range", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "name_list", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "integer_declaration",
            line_number: 178,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "integer"), CodingAdventures::GrammarTools::RuleReference.new(name: "name_list", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "name_list",
            line_number: 179,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "continuous_assign",
            line_number: 198,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "assign"), CodingAdventures::GrammarTools::RuleReference.new(name: "assignment", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "assignment", is_token: false)])), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "assignment",
            line_number: 199,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "lvalue", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "lvalue",
            line_number: 203,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "range_select", is_token: false))]), CodingAdventures::GrammarTools::RuleReference.new(name: "concatenation", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "range_select",
            line_number: 206,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACKET", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACKET", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "always_construct",
            line_number: 243,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "always"), CodingAdventures::GrammarTools::RuleReference.new(name: "AT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "sensitivity_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "statement", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "initial_construct",
            line_number: 244,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "initial"), CodingAdventures::GrammarTools::RuleReference.new(name: "statement", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "sensitivity_list",
            line_number: 246,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "sensitivity_item", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Literal.new(value: "or"), CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "sensitivity_item", is_token: false)])), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STAR", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "sensitivity_item",
            line_number: 250,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Literal.new(value: "posedge"), CodingAdventures::GrammarTools::Literal.new(value: "negedge")])), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "statement",
            line_number: 259,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "block_statement", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "if_statement", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "case_statement", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "for_statement", is_token: false), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "blocking_assignment", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "nonblocking_assignment", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "task_call", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)]), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "block_statement",
            line_number: 275,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "begin"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)])), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "statement", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "end")])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "if_statement",
            line_number: 286,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "if"), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "statement", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "else"), CodingAdventures::GrammarTools::RuleReference.new(name: "statement", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "case_statement",
            line_number: 301,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Literal.new(value: "case"), CodingAdventures::GrammarTools::Literal.new(value: "casex"), CodingAdventures::GrammarTools::Literal.new(value: "casez")])), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "case_item", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "endcase")])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "case_item",
            line_number: 306,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "expression_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "statement", is_token: false)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "default"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true)), CodingAdventures::GrammarTools::RuleReference.new(name: "statement", is_token: false)])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "expression_list",
            line_number: 309,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "for_statement",
            line_number: 313,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "for"), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "blocking_assignment", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "blocking_assignment", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "statement", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "blocking_assignment",
            line_number: 317,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "lvalue", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "nonblocking_assignment",
            line_number: 318,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "lvalue", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "LESS_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "task_call",
            line_number: 321,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)]))])), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "module_instantiation",
            line_number: 340,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "parameter_value_assignment", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "instance", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "instance", is_token: false)])), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "parameter_value_assignment",
            line_number: 343,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "HASH", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "instance",
            line_number: 345,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "port_connections", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "port_connections",
            line_number: 347,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "named_port_connection", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "named_port_connection", is_token: false)]))]), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)]))]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "named_port_connection",
            line_number: 350,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "DOT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "generate_region",
            line_number: 377,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "generate"), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "generate_item", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "endgenerate")])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "generate_item",
            line_number: 379,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "genvar_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "generate_for", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "generate_if", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "module_item", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "genvar_declaration",
            line_number: 384,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "genvar"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "generate_for",
            line_number: 386,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "for"), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "genvar_assignment", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "genvar_assignment", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "generate_block", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "generate_if",
            line_number: 390,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "if"), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "generate_block", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "else"), CodingAdventures::GrammarTools::RuleReference.new(name: "generate_block", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "generate_block",
            line_number: 393,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "begin"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)])), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "generate_item", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "end")]), CodingAdventures::GrammarTools::RuleReference.new(name: "generate_item", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "genvar_assignment",
            line_number: 396,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "function_declaration",
            line_number: 415,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "function"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "range", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "function_item", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "statement", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: "endfunction")])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "function_item",
            line_number: 420,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "port_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "reg_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "integer_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "parameter_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "task_declaration",
            line_number: 425,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "task"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "task_item", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "statement", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: "endtask")])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "task_item",
            line_number: 430,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "port_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "reg_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "integer_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "expression",
            line_number: 458,
            body: CodingAdventures::GrammarTools::RuleReference.new(name: "ternary_expr", is_token: false)
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "ternary_expr",
            line_number: 464,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "or_expr", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "QUESTION", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ternary_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "or_expr",
            line_number: 467,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "and_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LOGIC_OR", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "and_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "and_expr",
            line_number: 468,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "bit_or_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LOGIC_AND", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "bit_or_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "bit_or_expr",
            line_number: 471,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "bit_xor_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "PIPE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "bit_xor_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "bit_xor_expr",
            line_number: 472,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "bit_and_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "CARET", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "bit_and_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "bit_and_expr",
            line_number: 473,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "equality_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "AMP", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "equality_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "equality_expr",
            line_number: 477,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "relational_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NOT_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "CASE_EQ", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "CASE_NEQ", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "relational_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "relational_expr",
            line_number: 484,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "shift_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "LESS_THAN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LESS_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "GREATER_THAN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "GREATER_EQUALS", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "shift_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "shift_expr",
            line_number: 489,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "additive_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "LEFT_SHIFT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "RIGHT_SHIFT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ARITH_LEFT_SHIFT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ARITH_RIGHT_SHIFT", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "additive_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "additive_expr",
            line_number: 494,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "multiplicative_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "PLUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "MINUS", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "multiplicative_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "multiplicative_expr",
            line_number: 495,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "power_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "STAR", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "SLASH", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "PERCENT", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "power_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "power_expr",
            line_number: 496,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "unary_expr", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "POWER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "unary_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "unary_expr",
            line_number: 508,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "PLUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "MINUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "BANG", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "TILDE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "AMP", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "PIPE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "CARET", is_token: true), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "TILDE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "AMP", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "TILDE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "PIPE", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "TILDE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "CARET", is_token: true)])])), CodingAdventures::GrammarTools::RuleReference.new(name: "unary_expr", is_token: false)]), CodingAdventures::GrammarTools::RuleReference.new(name: "primary", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "primary",
            line_number: 518,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "NUMBER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "SIZED_NUMBER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "REAL_NUMBER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "SYSTEM_ID", is_token: true), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)]), CodingAdventures::GrammarTools::RuleReference.new(name: "concatenation", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "replication", is_token: false), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "primary", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACKET", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACKET", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)]))])), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "concatenation",
            line_number: 534,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACE", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "replication",
            line_number: 540,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "concatenation", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACE", is_token: true)])
          ),
        ]
      )
    end
  end
end
