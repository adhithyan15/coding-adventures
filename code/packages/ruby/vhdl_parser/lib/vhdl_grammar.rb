# frozen_string_literal: true
# AUTO-GENERATED FILE - DO NOT EDIT
require "coding_adventures_grammar_tools"

module CodingAdventures
  module VhdlGrammar
    def self.grammar
      @grammar ||= CodingAdventures::GrammarTools::ParserGrammar.new(
        version: 0,
        rules: [
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "design_file",
            line_number: 64,
            body: CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "design_unit", is_token: false))
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "design_unit",
            line_number: 66,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "context_item", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "library_unit", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "context_item",
            line_number: 68,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "library_clause", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "use_clause", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "library_clause",
            line_number: 71,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "library"), CodingAdventures::GrammarTools::RuleReference.new(name: "name_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "use_clause",
            line_number: 74,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "use"), CodingAdventures::GrammarTools::RuleReference.new(name: "selected_name", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "selected_name",
            line_number: 77,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "DOT", is_token: true), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "all")]))]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "name_list",
            line_number: 79,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "library_unit",
            line_number: 81,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "entity_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "architecture_body", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "package_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "package_body", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "entity_declaration",
            line_number: 112,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "entity"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "is"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "generic_clause", is_token: false)), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "port_clause", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "end"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Literal.new(value: "entity")), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "generic_clause",
            line_number: 117,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "generic"), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "interface_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "port_clause",
            line_number: 118,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "port"), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "interface_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "interface_list",
            line_number: 123,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "interface_element", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "interface_element", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "interface_element",
            line_number: 124,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "name_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "mode", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "subtype_indication", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "VAR_ASSIGN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "mode",
            line_number: 132,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Literal.new(value: "in"), CodingAdventures::GrammarTools::Literal.new(value: "out"), CodingAdventures::GrammarTools::Literal.new(value: "inout"), CodingAdventures::GrammarTools::Literal.new(value: "buffer")])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "architecture_body",
            line_number: 154,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "architecture"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "of"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "is"), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "block_declarative_item", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "begin"), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "concurrent_statement", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "end"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Literal.new(value: "architecture")), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "block_declarative_item",
            line_number: 160,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "signal_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "constant_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "type_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "subtype_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "component_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "function_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "function_body", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "procedure_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "procedure_body", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "signal_declaration",
            line_number: 189,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "signal"), CodingAdventures::GrammarTools::RuleReference.new(name: "name_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "subtype_indication", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "VAR_ASSIGN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "constant_declaration",
            line_number: 191,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "constant"), CodingAdventures::GrammarTools::RuleReference.new(name: "name_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "subtype_indication", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "VAR_ASSIGN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "variable_declaration",
            line_number: 193,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "variable"), CodingAdventures::GrammarTools::RuleReference.new(name: "name_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "subtype_indication", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "VAR_ASSIGN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "type_declaration",
            line_number: 218,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "type"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "is"), CodingAdventures::GrammarTools::RuleReference.new(name: "type_definition", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "subtype_declaration",
            line_number: 219,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "subtype"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "is"), CodingAdventures::GrammarTools::RuleReference.new(name: "subtype_indication", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "type_definition",
            line_number: 221,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "enumeration_type", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "array_type", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "record_type", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "enumeration_type",
            line_number: 227,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "CHAR_LITERAL", is_token: true)])), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "CHAR_LITERAL", is_token: true)]))])), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "array_type",
            line_number: 232,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "array"), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "index_constraint", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "of"), CodingAdventures::GrammarTools::RuleReference.new(name: "subtype_indication", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "index_constraint",
            line_number: 234,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "discrete_range", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "discrete_range", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "discrete_range",
            line_number: 235,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "subtype_indication", is_token: false), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Literal.new(value: "to"), CodingAdventures::GrammarTools::Literal.new(value: "downto")])), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "record_type",
            line_number: 239,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "record"), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "subtype_indication", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])), CodingAdventures::GrammarTools::Literal.new(value: "end"), CodingAdventures::GrammarTools::Literal.new(value: "record"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "subtype_indication",
            line_number: 247,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "selected_name", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "constraint", is_token: false))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "constraint",
            line_number: 249,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Literal.new(value: "to"), CodingAdventures::GrammarTools::Literal.new(value: "downto")])), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "range"), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Literal.new(value: "to"), CodingAdventures::GrammarTools::Literal.new(value: "downto")])), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "concurrent_statement",
            line_number: 264,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "process_statement", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "signal_assignment_concurrent", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "component_instantiation", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "generate_statement", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "signal_assignment_concurrent",
            line_number: 272,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LESS_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "waveform", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "waveform",
            line_number: 274,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "waveform_element", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "waveform_element", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "waveform_element",
            line_number: 275,
            body: CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "process_statement",
            line_number: 307,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true)])), CodingAdventures::GrammarTools::Literal.new(value: "process"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "sensitivity_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Literal.new(value: "is")), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "process_declarative_item", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "begin"), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "sequential_statement", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "end"), CodingAdventures::GrammarTools::Literal.new(value: "process"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "sensitivity_list",
            line_number: 315,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "process_declarative_item",
            line_number: 317,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "variable_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "constant_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "type_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "subtype_declaration", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "sequential_statement",
            line_number: 329,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "signal_assignment_seq", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "variable_assignment", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "if_statement", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "case_statement", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "loop_statement", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "return_statement", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "null_statement", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "signal_assignment_seq",
            line_number: 342,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LESS_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "waveform", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "variable_assignment",
            line_number: 346,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "VAR_ASSIGN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "if_statement",
            line_number: 356,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "if"), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: "then"), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "sequential_statement", is_token: false)), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "elsif"), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: "then"), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "sequential_statement", is_token: false))])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "else"), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "sequential_statement", is_token: false))])), CodingAdventures::GrammarTools::Literal.new(value: "end"), CodingAdventures::GrammarTools::Literal.new(value: "if"), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "case_statement",
            line_number: 372,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "case"), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: "is"), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "when"), CodingAdventures::GrammarTools::RuleReference.new(name: "choices", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "ARROW", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "sequential_statement", is_token: false))])), CodingAdventures::GrammarTools::Literal.new(value: "end"), CodingAdventures::GrammarTools::Literal.new(value: "case"), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "choices",
            line_number: 376,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "choice", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "PIPE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "choice", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "choice",
            line_number: 377,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "discrete_range", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: "others")])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "loop_statement",
            line_number: 391,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true)])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "for"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "in"), CodingAdventures::GrammarTools::RuleReference.new(name: "discrete_range", is_token: false)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "while"), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])])), CodingAdventures::GrammarTools::Literal.new(value: "loop"), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "sequential_statement", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "end"), CodingAdventures::GrammarTools::Literal.new(value: "loop"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "return_statement",
            line_number: 398,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "return"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "null_statement",
            line_number: 399,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "null"), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "component_declaration",
            line_number: 425,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "component"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Literal.new(value: "is")), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "generic_clause", is_token: false)), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "port_clause", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "end"), CodingAdventures::GrammarTools::Literal.new(value: "component"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "component_instantiation",
            line_number: 430,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "entity"), CodingAdventures::GrammarTools::RuleReference.new(name: "selected_name", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)]))])])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "generic"), CodingAdventures::GrammarTools::Literal.new(value: "map"), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "association_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "port"), CodingAdventures::GrammarTools::Literal.new(value: "map"), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "association_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "association_list",
            line_number: 437,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "association_element", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "association_element", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "association_element",
            line_number: 438,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ARROW", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ARROW", is_token: true)])), CodingAdventures::GrammarTools::Literal.new(value: "open")])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "generate_statement",
            line_number: 461,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "for_generate", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "if_generate", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "for_generate",
            line_number: 463,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "for"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "in"), CodingAdventures::GrammarTools::RuleReference.new(name: "discrete_range", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: "generate"), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "concurrent_statement", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "end"), CodingAdventures::GrammarTools::Literal.new(value: "generate"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "if_generate",
            line_number: 467,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "if"), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: "generate"), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "concurrent_statement", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "end"), CodingAdventures::GrammarTools::Literal.new(value: "generate"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "package_declaration",
            line_number: 488,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "package"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "is"), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "package_declarative_item", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "end"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Literal.new(value: "package")), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "package_body",
            line_number: 492,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "package"), CodingAdventures::GrammarTools::Literal.new(value: "body"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "is"), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "package_body_declarative_item", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "end"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "package"), CodingAdventures::GrammarTools::Literal.new(value: "body")])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "package_declarative_item",
            line_number: 496,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "type_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "subtype_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "constant_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "signal_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "component_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "function_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "procedure_declaration", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "package_body_declarative_item",
            line_number: 504,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "type_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "subtype_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "constant_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "function_body", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "procedure_body", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "function_declaration",
            line_number: 520,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Literal.new(value: "pure"), CodingAdventures::GrammarTools::Literal.new(value: "impure")])), CodingAdventures::GrammarTools::Literal.new(value: "function"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "interface_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])), CodingAdventures::GrammarTools::Literal.new(value: "return"), CodingAdventures::GrammarTools::RuleReference.new(name: "subtype_indication", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "function_body",
            line_number: 525,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Literal.new(value: "pure"), CodingAdventures::GrammarTools::Literal.new(value: "impure")])), CodingAdventures::GrammarTools::Literal.new(value: "function"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "interface_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])), CodingAdventures::GrammarTools::Literal.new(value: "return"), CodingAdventures::GrammarTools::RuleReference.new(name: "subtype_indication", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: "is"), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "process_declarative_item", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "begin"), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "sequential_statement", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "end"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Literal.new(value: "function")), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "procedure_declaration",
            line_number: 534,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "procedure"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "interface_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "procedure_body",
            line_number: 537,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "procedure"), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "interface_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])), CodingAdventures::GrammarTools::Literal.new(value: "is"), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "process_declarative_item", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "begin"), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "sequential_statement", is_token: false)), CodingAdventures::GrammarTools::Literal.new(value: "end"), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Literal.new(value: "procedure")), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "expression",
            line_number: 574,
            body: CodingAdventures::GrammarTools::RuleReference.new(name: "logical_expr", is_token: false)
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "logical_expr",
            line_number: 581,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "relation", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "logical_op", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "relation", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "logical_op",
            line_number: 582,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Literal.new(value: "and"), CodingAdventures::GrammarTools::Literal.new(value: "or"), CodingAdventures::GrammarTools::Literal.new(value: "xor"), CodingAdventures::GrammarTools::Literal.new(value: "nand"), CodingAdventures::GrammarTools::Literal.new(value: "nor"), CodingAdventures::GrammarTools::Literal.new(value: "xnor")])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "relation",
            line_number: 586,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "shift_expr", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "relational_op", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "shift_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "relational_op",
            line_number: 587,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NOT_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LESS_THAN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LESS_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "GREATER_THAN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "GREATER_EQUALS", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "shift_expr",
            line_number: 592,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "adding_expr", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "shift_op", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "adding_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "shift_op",
            line_number: 593,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Literal.new(value: "sll"), CodingAdventures::GrammarTools::Literal.new(value: "srl"), CodingAdventures::GrammarTools::Literal.new(value: "sla"), CodingAdventures::GrammarTools::Literal.new(value: "sra"), CodingAdventures::GrammarTools::Literal.new(value: "rol"), CodingAdventures::GrammarTools::Literal.new(value: "ror")])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "adding_expr",
            line_number: 597,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "multiplying_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "adding_op", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "multiplying_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "adding_op",
            line_number: 598,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "PLUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "MINUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "AMPERSAND", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "multiplying_expr",
            line_number: 601,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "unary_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "multiplying_op", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "unary_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "multiplying_op",
            line_number: 602,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "STAR", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "SLASH", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "mod"), CodingAdventures::GrammarTools::Literal.new(value: "rem")])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "unary_expr",
            line_number: 605,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "abs"), CodingAdventures::GrammarTools::RuleReference.new(name: "unary_expr", is_token: false)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "not"), CodingAdventures::GrammarTools::RuleReference.new(name: "unary_expr", is_token: false)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "PLUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "MINUS", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "unary_expr", is_token: false)]), CodingAdventures::GrammarTools::RuleReference.new(name: "power_expr", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "power_expr",
            line_number: 611,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "primary", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "POWER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "primary", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "primary",
            line_number: 619,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "NUMBER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "REAL_NUMBER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "BASED_LITERAL", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "CHAR_LITERAL", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "BIT_STRING", is_token: true), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "TICK", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true)]))]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)]))])), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)]), CodingAdventures::GrammarTools::RuleReference.new(name: "aggregate", is_token: false), CodingAdventures::GrammarTools::Literal.new(value: "null")])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "aggregate",
            line_number: 635,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "element_association", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "element_association", is_token: false)])), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "element_association",
            line_number: 636,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "choices", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "ARROW", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)])
          ),
        ]
      )
    end
  end
end
