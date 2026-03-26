# frozen_string_literal: true
# AUTO-GENERATED FILE - DO NOT EDIT
require "coding_adventures_grammar_tools"

module CodingAdventures
  module LatticeGrammar
    def self.grammar
      @grammar ||= CodingAdventures::GrammarTools::ParserGrammar.new(
        version: 1,
        rules: [
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "stylesheet",
            line_number: 37,
            body: CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "rule", is_token: false))
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "rule",
            line_number: 39,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_rule", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "at_rule", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "qualified_rule", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "lattice_rule",
            line_number: 51,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "variable_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "mixin_definition", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "function_definition", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "use_directive", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_control", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "variable_declaration",
            line_number: 69,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "VARIABLE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "value_list", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "BANG_DEFAULT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "BANG_GLOBAL", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "mixin_definition",
            line_number: 102,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "@mixin"), CodingAdventures::GrammarTools::RuleReference.new(name: "FUNCTION", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "mixin_params", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "block", is_token: false)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "@mixin"), CodingAdventures::GrammarTools::RuleReference.new(name: "IDENT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "block", is_token: false)])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "mixin_params",
            line_number: 105,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "mixin_param", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "mixin_param", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "mixin_param",
            line_number: 112,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "VARIABLE", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "mixin_value_list", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "mixin_value_list",
            line_number: 117,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "mixin_value", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "mixin_value", is_token: false))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "mixin_value",
            line_number: 119,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "DIMENSION", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "PERCENTAGE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NUMBER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "IDENT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "HASH", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "CUSTOM_PROPERTY", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "UNICODE_RANGE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "function_call", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "VARIABLE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "SLASH", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "PLUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "MINUS", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "include_directive",
            line_number: 130,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "@include"), CodingAdventures::GrammarTools::RuleReference.new(name: "FUNCTION", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "include_args", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "block", is_token: false)]))]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "@include"), CodingAdventures::GrammarTools::RuleReference.new(name: "IDENT", is_token: true), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "block", is_token: false)]))])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "include_args",
            line_number: 133,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "include_arg", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "include_arg", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "include_arg",
            line_number: 137,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "VARIABLE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "value_list", is_token: false)]), CodingAdventures::GrammarTools::RuleReference.new(name: "value_list", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "lattice_control",
            line_number: 160,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "if_directive", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "for_directive", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "each_directive", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "while_directive", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "if_directive",
            line_number: 164,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "@if"), CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "block", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "@else"), CodingAdventures::GrammarTools::Literal.new(value: "if"), CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "block", is_token: false)])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "@else"), CodingAdventures::GrammarTools::RuleReference.new(name: "block", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "for_directive",
            line_number: 171,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "@for"), CodingAdventures::GrammarTools::RuleReference.new(name: "VARIABLE", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "from"), CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_expression", is_token: false), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Literal.new(value: "through"), CodingAdventures::GrammarTools::Literal.new(value: "to")])), CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "block", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "each_directive",
            line_number: 176,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "@each"), CodingAdventures::GrammarTools::RuleReference.new(name: "VARIABLE", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "VARIABLE", is_token: true)])), CodingAdventures::GrammarTools::Literal.new(value: "in"), CodingAdventures::GrammarTools::RuleReference.new(name: "each_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "block", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "each_list",
            line_number: 179,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "value", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "value", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "while_directive",
            line_number: 184,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "@while"), CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "block", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "lattice_expression",
            line_number: 203,
            body: CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_or_expr", is_token: false)
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "lattice_or_expr",
            line_number: 205,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_and_expr", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "or"), CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_and_expr", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "lattice_and_expr",
            line_number: 207,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_comparison", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "and"), CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_comparison", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "lattice_comparison",
            line_number: 209,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_additive", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "comparison_op", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_additive", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "comparison_op",
            line_number: 211,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NOT_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "GREATER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "GREATER_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LESS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LESS_EQUALS", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "lattice_additive",
            line_number: 214,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_multiplicative", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "PLUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "MINUS", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_multiplicative", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "lattice_multiplicative",
            line_number: 219,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_unary", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "STAR", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "SLASH", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_unary", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "lattice_unary",
            line_number: 221,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "MINUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_unary", is_token: false)]), CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_primary", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "lattice_primary",
            line_number: 224,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "VARIABLE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NUMBER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "DIMENSION", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "PERCENTAGE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "IDENT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "HASH", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "true"), CodingAdventures::GrammarTools::Literal.new(value: "false"), CodingAdventures::GrammarTools::Literal.new(value: "null"), CodingAdventures::GrammarTools::RuleReference.new(name: "function_call", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "map_literal", is_token: false), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "map_literal",
            line_number: 235,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "map_entry", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "map_entry", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "map_entry", is_token: false)])), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "map_entry",
            line_number: 237,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "IDENT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_expression", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "function_definition",
            line_number: 261,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "@function"), CodingAdventures::GrammarTools::RuleReference.new(name: "FUNCTION", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "mixin_params", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "function_body", is_token: false)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "@function"), CodingAdventures::GrammarTools::RuleReference.new(name: "IDENT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "function_body", is_token: false)])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "function_body",
            line_number: 264,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACE", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "function_body_item", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACE", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "function_body_item",
            line_number: 266,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "variable_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "return_directive", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_control", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "return_directive",
            line_number: 268,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "@return"), CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "use_directive",
            line_number: 281,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "@use"), CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "as"), CodingAdventures::GrammarTools::RuleReference.new(name: "IDENT", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "at_rule",
            line_number: 294,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "AT_KEYWORD", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "at_prelude", is_token: false), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "block", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "at_prelude",
            line_number: 296,
            body: CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "at_prelude_token", is_token: false))
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "at_prelude_token",
            line_number: 298,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "IDENT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NUMBER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "DIMENSION", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "PERCENTAGE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "HASH", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "CUSTOM_PROPERTY", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "UNICODE_RANGE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "VARIABLE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "function_in_prelude", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "paren_block", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "SLASH", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "DOT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STAR", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "PLUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "MINUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "GREATER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "TILDE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "PIPE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "AMPERSAND", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "CDO", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "CDC", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "function_in_prelude",
            line_number: 306,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "FUNCTION", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "at_prelude_tokens", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "paren_block",
            line_number: 307,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "at_prelude_tokens", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "at_prelude_tokens",
            line_number: 308,
            body: CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "at_prelude_token", is_token: false))
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "qualified_rule",
            line_number: 314,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "selector_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "block", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "selector_list",
            line_number: 320,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "complex_selector", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "complex_selector", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "complex_selector",
            line_number: 322,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "compound_selector", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "combinator", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "compound_selector", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "combinator",
            line_number: 324,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "GREATER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "PLUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "TILDE", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "compound_selector",
            line_number: 326,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "simple_selector", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "subclass_selector", is_token: false))]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "subclass_selector", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "subclass_selector", is_token: false))])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "simple_selector",
            line_number: 330,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "IDENT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STAR", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "AMPERSAND", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "VARIABLE", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "subclass_selector",
            line_number: 333,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "class_selector", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "id_selector", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "placeholder_selector", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "attribute_selector", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "pseudo_class", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "pseudo_element", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "placeholder_selector",
            line_number: 337,
            body: CodingAdventures::GrammarTools::RuleReference.new(name: "PLACEHOLDER", is_token: true)
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "class_selector",
            line_number: 339,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "DOT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "IDENT", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "id_selector",
            line_number: 341,
            body: CodingAdventures::GrammarTools::RuleReference.new(name: "HASH", is_token: true)
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "attribute_selector",
            line_number: 343,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACKET", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "IDENT", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "attr_matcher", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "attr_value", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "IDENT", is_token: true))])), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACKET", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "attr_matcher",
            line_number: 345,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "TILDE_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "PIPE_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "CARET_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "DOLLAR_EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STAR_EQUALS", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "attr_value",
            line_number: 348,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "IDENT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "pseudo_class",
            line_number: 350,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "FUNCTION", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "pseudo_class_args", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "IDENT", is_token: true)])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "pseudo_class_args",
            line_number: 353,
            body: CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "pseudo_class_arg", is_token: false))
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "pseudo_class_arg",
            line_number: 355,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "IDENT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NUMBER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "DIMENSION", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "HASH", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "PLUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "DOT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STAR", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "AMPERSAND", is_token: true), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "FUNCTION", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "pseudo_class_args", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACKET", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "pseudo_class_args", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACKET", is_token: true)])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "pseudo_element",
            line_number: 360,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COLON_COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "IDENT", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "block",
            line_number: 370,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "block_contents", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACE", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "block_contents",
            line_number: 372,
            body: CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "block_item", is_token: false))
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "block_item",
            line_number: 374,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_block_item", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "at_rule", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "declaration_or_nested", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "lattice_block_item",
            line_number: 380,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "variable_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "include_directive", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "lattice_control", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "content_directive", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "extend_directive", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "at_root_directive", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "content_directive",
            line_number: 390,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "@content"), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "extend_directive",
            line_number: 398,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "@extend"), CodingAdventures::GrammarTools::RuleReference.new(name: "selector_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "at_root_directive",
            line_number: 403,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Literal.new(value: "@at-root"), CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "selector_list", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "block", is_token: false)]), CodingAdventures::GrammarTools::RuleReference.new(name: "block", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "declaration_or_nested",
            line_number: 405,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "qualified_rule", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "declaration",
            line_number: 414,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "property", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "value_list", is_token: false), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "priority", is_token: false)), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)]), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "property", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "block", is_token: false)])])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "property",
            line_number: 417,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "IDENT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "CUSTOM_PROPERTY", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "priority",
            line_number: 419,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "BANG", is_token: true), CodingAdventures::GrammarTools::Literal.new(value: "important")])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "value_list",
            line_number: 430,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "value", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "value", is_token: false))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "value",
            line_number: 432,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "DIMENSION", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "PERCENTAGE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NUMBER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "IDENT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "HASH", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "CUSTOM_PROPERTY", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "UNICODE_RANGE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "function_call", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "VARIABLE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "SLASH", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "PLUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "MINUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "map_literal", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "function_call",
            line_number: 438,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "FUNCTION", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "function_args", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)]), CodingAdventures::GrammarTools::RuleReference.new(name: "URL_TOKEN", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "function_args",
            line_number: 441,
            body: CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "function_arg", is_token: false))
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "function_arg",
            line_number: 443,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "DIMENSION", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "PERCENTAGE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NUMBER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "IDENT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "HASH", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "CUSTOM_PROPERTY", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "SLASH", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "PLUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "MINUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STAR", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "VARIABLE", is_token: true), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "FUNCTION", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "function_args", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])])
          ),
        ]
      )
    end
  end
end
