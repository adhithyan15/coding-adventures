# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: csharp3.0.grammar
# Regenerate with: grammar-tools compile-grammar csharp3.0.grammar
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
      name: "compilation_unit",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "extern_alias_directive", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "using_directive", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "global_attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "namespace_member_declaration", is_token: false)),
      ]),
      line_number: 64,
    ),
    GT::GrammarRule.new(
      name: "extern_alias_directive",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "extern"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 80,
    ),
    GT::GrammarRule.new(
      name: "using_directive",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "using"),
          GT::RuleReference.new(name: "qualified_name", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "using"),
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "EQUALS", is_token: true),
          GT::RuleReference.new(name: "qualified_name", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
      ]),
      line_number: 93,
    ),
    GT::GrammarRule.new(
      name: "qualified_name",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "DOT", is_token: true),
              GT::RuleReference.new(name: "NAME", is_token: true),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "COLON_COLON", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "DOT", is_token: true),
              GT::RuleReference.new(name: "NAME", is_token: true),
            ])),
        ]),
      ]),
      line_number: 107,
    ),
    GT::GrammarRule.new(
      name: "namespace_or_type_name",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "namespace_or_type_part", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "DOT", is_token: true),
            GT::RuleReference.new(name: "namespace_or_type_part", is_token: false),
          ])),
      ]),
      line_number: 112,
    ),
    GT::GrammarRule.new(
      name: "namespace_or_type_part",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "COLON_COLON", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_argument_list", is_token: false)),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_argument_list", is_token: false)),
        ]),
      ]),
      line_number: 114,
    ),
    GT::GrammarRule.new(
      name: "type_parameter_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LESS_THAN", is_token: true),
        GT::RuleReference.new(name: "type_parameter", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "type_parameter", is_token: false),
          ])),
        GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
      ]),
      line_number: 128,
    ),
    GT::GrammarRule.new(
      name: "type_parameter",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 130,
    ),
    GT::GrammarRule.new(
      name: "type_argument_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LESS_THAN", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "type", is_token: false),
          ])),
        GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
      ]),
      line_number: 132,
    ),
    GT::GrammarRule.new(
      name: "type_parameter_constraints_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "where"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "type_parameter_constraints", is_token: false),
      ]),
      line_number: 144,
    ),
    GT::GrammarRule.new(
      name: "type_parameter_constraints",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "primary_constraint", is_token: false),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "secondary_constraints", is_token: false),
            ])),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "constructor_constraint", is_token: false),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "secondary_constraints", is_token: false),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "constructor_constraint", is_token: false),
            ])),
        ]),
        GT::RuleReference.new(name: "constructor_constraint", is_token: false),
      ]),
      line_number: 146,
    ),
    GT::GrammarRule.new(
      name: "primary_constraint",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "class"),
        GT::Literal.new(value: "struct"),
        GT::RuleReference.new(name: "namespace_or_type_name", is_token: false),
      ]),
      line_number: 151,
    ),
    GT::GrammarRule.new(
      name: "secondary_constraints",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "namespace_or_type_name", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "namespace_or_type_name", is_token: false),
          ])),
      ]),
      line_number: 155,
    ),
    GT::GrammarRule.new(
      name: "constructor_constraint",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "new"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 157,
    ),
    GT::GrammarRule.new(
      name: "global_attribute_section",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "global_attribute_target", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "attribute_list", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 165,
    ),
    GT::GrammarRule.new(
      name: "global_attribute_target",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "assembly"),
        GT::Literal.new(value: "module"),
      ]),
      line_number: 167,
    ),
    GT::GrammarRule.new(
      name: "namespace_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "namespace"),
        GT::RuleReference.new(name: "qualified_name", is_token: false),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "extern_alias_directive", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "using_directive", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "namespace_member_declaration", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "SEMICOLON", is_token: true)),
      ]),
      line_number: 176,
    ),
    GT::GrammarRule.new(
      name: "namespace_member_declaration",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "namespace_declaration", is_token: false),
        GT::RuleReference.new(name: "type_declaration", is_token: false),
      ]),
      line_number: 186,
    ),
    GT::GrammarRule.new(
      name: "type_declaration",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "class_declaration", is_token: false),
        GT::RuleReference.new(name: "struct_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_declaration", is_token: false),
        GT::RuleReference.new(name: "enum_declaration", is_token: false),
        GT::RuleReference.new(name: "delegate_declaration", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 189,
    ),
    GT::GrammarRule.new(
      name: "attribute_section",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "attribute_target", is_token: false),
            GT::RuleReference.new(name: "COLON", is_token: true),
          ])),
        GT::RuleReference.new(name: "attribute_list", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 203,
    ),
    GT::GrammarRule.new(
      name: "attribute_target",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "field"),
        GT::Literal.new(value: "event"),
        GT::Literal.new(value: "method"),
        GT::Literal.new(value: "param"),
        GT::Literal.new(value: "property"),
        GT::Literal.new(value: "return"),
        GT::Literal.new(value: "type"),
      ]),
      line_number: 205,
    ),
    GT::GrammarRule.new(
      name: "attribute_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "attribute", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "attribute", is_token: false),
          ])),
      ]),
      line_number: 213,
    ),
    GT::GrammarRule.new(
      name: "attribute",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "namespace_or_type_name", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "attribute_arguments", is_token: false)),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
      ]),
      line_number: 215,
    ),
    GT::GrammarRule.new(
      name: "attribute_arguments",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "attribute_argument", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "attribute_argument", is_token: false),
          ])),
      ]),
      line_number: 217,
    ),
    GT::GrammarRule.new(
      name: "attribute_argument",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "EQUALS", is_token: true),
          ])),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 219,
    ),
    GT::GrammarRule.new(
      name: "class_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "class_modifier", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "partial")),
        GT::Literal.new(value: "class"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameter_list", is_token: false)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "class_base_list", is_token: false),
          ])),
        GT::Repetition.new(element: GT::RuleReference.new(name: "type_parameter_constraints_clause", is_token: false)),
        GT::RuleReference.new(name: "class_body", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "SEMICOLON", is_token: true)),
      ]),
      line_number: 249,
    ),
    GT::GrammarRule.new(
      name: "class_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "internal"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "new"),
        GT::Literal.new(value: "abstract"),
        GT::Literal.new(value: "sealed"),
        GT::Literal.new(value: "static"),
      ]),
      line_number: 255,
    ),
    GT::GrammarRule.new(
      name: "class_base_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type_name", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "type_name", is_token: false),
          ])),
      ]),
      line_number: 264,
    ),
    GT::GrammarRule.new(
      name: "class_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "class_member_declaration", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 266,
    ),
    GT::GrammarRule.new(
      name: "type_name",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "DOT", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_argument_list", is_token: false)),
      ]),
      line_number: 277,
    ),
    GT::GrammarRule.new(
      name: "type_argument_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LESS_THAN", is_token: true),
        GT::RuleReference.new(name: "type_argument", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "type_argument", is_token: false),
          ])),
        GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
      ]),
      line_number: 279,
    ),
    GT::GrammarRule.new(
      name: "type_argument",
      body: GT::RuleReference.new(name: "type", is_token: false),
      line_number: 281,
    ),
    GT::GrammarRule.new(
      name: "class_member_declaration",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "constant_declaration", is_token: false),
        GT::RuleReference.new(name: "field_declaration", is_token: false),
        GT::RuleReference.new(name: "method_declaration", is_token: false),
        GT::RuleReference.new(name: "property_declaration", is_token: false),
        GT::RuleReference.new(name: "event_declaration", is_token: false),
        GT::RuleReference.new(name: "indexer_declaration", is_token: false),
        GT::RuleReference.new(name: "operator_declaration", is_token: false),
        GT::RuleReference.new(name: "conversion_operator_declaration", is_token: false),
        GT::RuleReference.new(name: "constructor_declaration", is_token: false),
        GT::RuleReference.new(name: "destructor_declaration", is_token: false),
        GT::RuleReference.new(name: "static_constructor_declaration", is_token: false),
        GT::RuleReference.new(name: "type_declaration", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 315,
    ),
    GT::GrammarRule.new(
      name: "constant_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "constant_modifier", is_token: false)),
        GT::Literal.new(value: "const"),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "constant_declarators", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 333,
    ),
    GT::GrammarRule.new(
      name: "constant_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "internal"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "new"),
      ]),
      line_number: 336,
    ),
    GT::GrammarRule.new(
      name: "constant_declarators",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "constant_declarator", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "constant_declarator", is_token: false),
          ])),
      ]),
      line_number: 342,
    ),
    GT::GrammarRule.new(
      name: "constant_declarator",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 344,
    ),
    GT::GrammarRule.new(
      name: "field_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "field_modifier", is_token: false)),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "variable_declarators", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 353,
    ),
    GT::GrammarRule.new(
      name: "field_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "internal"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "new"),
        GT::Literal.new(value: "static"),
        GT::Literal.new(value: "readonly"),
        GT::Literal.new(value: "volatile"),
      ]),
      line_number: 356,
    ),
    GT::GrammarRule.new(
      name: "variable_declarators",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "variable_declarator", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "variable_declarator", is_token: false),
          ])),
      ]),
      line_number: 365,
    ),
    GT::GrammarRule.new(
      name: "variable_declarator",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "variable_initializer", is_token: false),
          ])),
      ]),
      line_number: 367,
    ),
    GT::GrammarRule.new(
      name: "variable_initializer",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "array_initializer", is_token: false),
      ]),
      line_number: 369,
    ),
    GT::GrammarRule.new(
      name: "array_initializer",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "variable_initializer", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "variable_initializer", is_token: false),
              ])),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 372,
    ),
    GT::GrammarRule.new(
      name: "method_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "method_modifier", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "partial")),
        GT::RuleReference.new(name: "return_type", is_token: false),
        GT::RuleReference.new(name: "qualified_name", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "type_parameter_constraints_clause", is_token: false)),
        GT::RuleReference.new(name: "method_body", is_token: false),
      ]),
      line_number: 403,
    ),
    GT::GrammarRule.new(
      name: "method_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "internal"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "new"),
        GT::Literal.new(value: "static"),
        GT::Literal.new(value: "virtual"),
        GT::Literal.new(value: "sealed"),
        GT::Literal.new(value: "override"),
        GT::Literal.new(value: "abstract"),
        GT::Literal.new(value: "extern"),
      ]),
      line_number: 409,
    ),
    GT::GrammarRule.new(
      name: "return_type",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "void"),
        GT::RuleReference.new(name: "type", is_token: false),
      ]),
      line_number: 421,
    ),
    GT::GrammarRule.new(
      name: "method_body",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "block", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 424,
    ),
    GT::GrammarRule.new(
      name: "formal_parameter_list",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "fixed_parameters", is_token: false),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "parameter_array", is_token: false),
            ])),
        ]),
        GT::RuleReference.new(name: "parameter_array", is_token: false),
      ]),
      line_number: 443,
    ),
    GT::GrammarRule.new(
      name: "fixed_parameters",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "fixed_parameter", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "fixed_parameter", is_token: false),
          ])),
      ]),
      line_number: 446,
    ),
    GT::GrammarRule.new(
      name: "fixed_parameter",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "parameter_modifier", is_token: false)),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 448,
    ),
    GT::GrammarRule.new(
      name: "parameter_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "ref"),
        GT::Literal.new(value: "out"),
        GT::Literal.new(value: "this"),
      ]),
      line_number: 450,
    ),
    GT::GrammarRule.new(
      name: "parameter_array",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Literal.new(value: "params"),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 457,
    ),
    GT::GrammarRule.new(
      name: "property_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "property_modifier", is_token: false)),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "qualified_name", is_token: false),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "accessor_declarations", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 481,
    ),
    GT::GrammarRule.new(
      name: "property_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "internal"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "new"),
        GT::Literal.new(value: "static"),
        GT::Literal.new(value: "virtual"),
        GT::Literal.new(value: "sealed"),
        GT::Literal.new(value: "override"),
        GT::Literal.new(value: "abstract"),
        GT::Literal.new(value: "extern"),
      ]),
      line_number: 484,
    ),
    GT::GrammarRule.new(
      name: "accessor_declarations",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "get_accessor_declaration", is_token: false),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "set_accessor_declaration", is_token: false)),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "set_accessor_declaration", is_token: false),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "get_accessor_declaration", is_token: false)),
        ]),
      ]),
      line_number: 496,
    ),
    GT::GrammarRule.new(
      name: "get_accessor_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "accessor_modifier", is_token: false)),
        GT::Literal.new(value: "get"),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "block", is_token: false),
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          ])),
      ]),
      line_number: 499,
    ),
    GT::GrammarRule.new(
      name: "set_accessor_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "accessor_modifier", is_token: false)),
        GT::Literal.new(value: "set"),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "block", is_token: false),
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          ])),
      ]),
      line_number: 502,
    ),
    GT::GrammarRule.new(
      name: "accessor_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "internal"),
        GT::Literal.new(value: "private"),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "protected"),
          GT::Literal.new(value: "internal"),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "internal"),
          GT::Literal.new(value: "protected"),
        ]),
      ]),
      line_number: 505,
    ),
    GT::GrammarRule.new(
      name: "event_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "event_modifier", is_token: false)),
        GT::Literal.new(value: "event"),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "variable_declarators", is_token: false),
              GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            ]),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "qualified_name", is_token: false),
              GT::RuleReference.new(name: "LBRACE", is_token: true),
              GT::RuleReference.new(name: "event_accessor_declarations", is_token: false),
              GT::RuleReference.new(name: "RBRACE", is_token: true),
            ]),
          ])),
      ]),
      line_number: 517,
    ),
    GT::GrammarRule.new(
      name: "event_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "internal"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "new"),
        GT::Literal.new(value: "static"),
        GT::Literal.new(value: "virtual"),
        GT::Literal.new(value: "sealed"),
        GT::Literal.new(value: "override"),
        GT::Literal.new(value: "abstract"),
        GT::Literal.new(value: "extern"),
      ]),
      line_number: 521,
    ),
    GT::GrammarRule.new(
      name: "event_accessor_declarations",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "add_accessor_declaration", is_token: false),
          GT::RuleReference.new(name: "remove_accessor_declaration", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "remove_accessor_declaration", is_token: false),
          GT::RuleReference.new(name: "add_accessor_declaration", is_token: false),
        ]),
      ]),
      line_number: 533,
    ),
    GT::GrammarRule.new(
      name: "add_accessor_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Literal.new(value: "add"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 536,
    ),
    GT::GrammarRule.new(
      name: "remove_accessor_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Literal.new(value: "remove"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 538,
    ),
    GT::GrammarRule.new(
      name: "indexer_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "indexer_modifier", is_token: false)),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::Literal.new(value: "this"),
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "formal_parameter_list", is_token: false),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "accessor_declarations", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 546,
    ),
    GT::GrammarRule.new(
      name: "indexer_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "internal"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "new"),
        GT::Literal.new(value: "virtual"),
        GT::Literal.new(value: "sealed"),
        GT::Literal.new(value: "override"),
        GT::Literal.new(value: "abstract"),
        GT::Literal.new(value: "extern"),
      ]),
      line_number: 550,
    ),
    GT::GrammarRule.new(
      name: "operator_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::RuleReference.new(name: "operator_modifiers", is_token: false),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::Literal.new(value: "operator"),
        GT::RuleReference.new(name: "overloadable_operator", is_token: false),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "type", is_token: false),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "method_body", is_token: false),
      ]),
      line_number: 567,
    ),
    GT::GrammarRule.new(
      name: "operator_modifiers",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "static"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "extern")),
      ]),
      line_number: 572,
    ),
    GT::GrammarRule.new(
      name: "overloadable_operator",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "PLUS", is_token: true),
        GT::RuleReference.new(name: "MINUS", is_token: true),
        GT::RuleReference.new(name: "BANG", is_token: true),
        GT::RuleReference.new(name: "TILDE", is_token: true),
        GT::RuleReference.new(name: "PLUS_PLUS", is_token: true),
        GT::RuleReference.new(name: "MINUS_MINUS", is_token: true),
        GT::Literal.new(value: "true"),
        GT::Literal.new(value: "false"),
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::RuleReference.new(name: "SLASH", is_token: true),
        GT::RuleReference.new(name: "PERCENT", is_token: true),
        GT::RuleReference.new(name: "AMPERSAND", is_token: true),
        GT::RuleReference.new(name: "PIPE", is_token: true),
        GT::RuleReference.new(name: "CARET", is_token: true),
        GT::RuleReference.new(name: "LEFT_SHIFT", is_token: true),
        GT::RuleReference.new(name: "RIGHT_SHIFT", is_token: true),
        GT::RuleReference.new(name: "EQUALS_EQUALS", is_token: true),
        GT::RuleReference.new(name: "NOT_EQUALS", is_token: true),
        GT::RuleReference.new(name: "LESS_THAN", is_token: true),
        GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
        GT::RuleReference.new(name: "LESS_EQUALS", is_token: true),
        GT::RuleReference.new(name: "GREATER_EQUALS", is_token: true),
      ]),
      line_number: 574,
    ),
    GT::GrammarRule.new(
      name: "conversion_operator_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::RuleReference.new(name: "operator_modifiers", is_token: false),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "implicit"),
            GT::Literal.new(value: "explicit"),
          ])),
        GT::Literal.new(value: "operator"),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "method_body", is_token: false),
      ]),
      line_number: 601,
    ),
    GT::GrammarRule.new(
      name: "constructor_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "constructor_modifier", is_token: false)),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "constructor_initializer", is_token: false)),
        GT::RuleReference.new(name: "method_body", is_token: false),
      ]),
      line_number: 611,
    ),
    GT::GrammarRule.new(
      name: "constructor_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "internal"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "extern"),
      ]),
      line_number: 615,
    ),
    GT::GrammarRule.new(
      name: "constructor_initializer",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::Literal.new(value: "base"),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "argument_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::Literal.new(value: "this"),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "argument_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 621,
    ),
    GT::GrammarRule.new(
      name: "destructor_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "extern")),
        GT::RuleReference.new(name: "TILDE", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "method_body", is_token: false),
      ]),
      line_number: 628,
    ),
    GT::GrammarRule.new(
      name: "static_constructor_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::RuleReference.new(name: "static_constructor_modifiers", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "method_body", is_token: false),
      ]),
      line_number: 635,
    ),
    GT::GrammarRule.new(
      name: "static_constructor_modifiers",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "static"),
          GT::OptionalElement.new(element: GT::Literal.new(value: "extern")),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "extern"),
          GT::Literal.new(value: "static"),
        ]),
      ]),
      line_number: 638,
    ),
    GT::GrammarRule.new(
      name: "struct_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "struct_modifier", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "partial")),
        GT::Literal.new(value: "struct"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameter_list", is_token: false)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "interface_type_list", is_token: false),
          ])),
        GT::Repetition.new(element: GT::RuleReference.new(name: "type_parameter_constraints_clause", is_token: false)),
        GT::RuleReference.new(name: "struct_body", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "SEMICOLON", is_token: true)),
      ]),
      line_number: 648,
    ),
    GT::GrammarRule.new(
      name: "struct_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "internal"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "new"),
      ]),
      line_number: 654,
    ),
    GT::GrammarRule.new(
      name: "interface_type_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type_name", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "type_name", is_token: false),
          ])),
      ]),
      line_number: 660,
    ),
    GT::GrammarRule.new(
      name: "struct_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "struct_member_declaration", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 662,
    ),
    GT::GrammarRule.new(
      name: "struct_member_declaration",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "constant_declaration", is_token: false),
        GT::RuleReference.new(name: "field_declaration", is_token: false),
        GT::RuleReference.new(name: "method_declaration", is_token: false),
        GT::RuleReference.new(name: "property_declaration", is_token: false),
        GT::RuleReference.new(name: "event_declaration", is_token: false),
        GT::RuleReference.new(name: "indexer_declaration", is_token: false),
        GT::RuleReference.new(name: "operator_declaration", is_token: false),
        GT::RuleReference.new(name: "conversion_operator_declaration", is_token: false),
        GT::RuleReference.new(name: "constructor_declaration", is_token: false),
        GT::RuleReference.new(name: "static_constructor_declaration", is_token: false),
        GT::RuleReference.new(name: "type_declaration", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 664,
    ),
    GT::GrammarRule.new(
      name: "interface_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "interface_modifier", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "partial")),
        GT::Literal.new(value: "interface"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameter_list", is_token: false)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "interface_type_list", is_token: false),
          ])),
        GT::Repetition.new(element: GT::RuleReference.new(name: "type_parameter_constraints_clause", is_token: false)),
        GT::RuleReference.new(name: "interface_body", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "SEMICOLON", is_token: true)),
      ]),
      line_number: 684,
    ),
    GT::GrammarRule.new(
      name: "interface_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "internal"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "new"),
      ]),
      line_number: 690,
    ),
    GT::GrammarRule.new(
      name: "interface_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "interface_member_declaration", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 696,
    ),
    GT::GrammarRule.new(
      name: "interface_member_declaration",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "interface_method_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_property_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_event_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_indexer_declaration", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 698,
    ),
    GT::GrammarRule.new(
      name: "interface_method_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "new")),
        GT::RuleReference.new(name: "return_type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "type_parameter_constraints_clause", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 704,
    ),
    GT::GrammarRule.new(
      name: "interface_property_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "new")),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "interface_accessors", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 710,
    ),
    GT::GrammarRule.new(
      name: "interface_accessors",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "get"),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::Literal.new(value: "set"),
              GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "set"),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::Literal.new(value: "get"),
              GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            ])),
        ]),
      ]),
      line_number: 713,
    ),
    GT::GrammarRule.new(
      name: "interface_event_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "new")),
        GT::Literal.new(value: "event"),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 716,
    ),
    GT::GrammarRule.new(
      name: "interface_indexer_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "new")),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::Literal.new(value: "this"),
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "formal_parameter_list", is_token: false),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "interface_accessors", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 718,
    ),
    GT::GrammarRule.new(
      name: "enum_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "enum_modifier", is_token: false)),
        GT::Literal.new(value: "enum"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "integral_type", is_token: false),
          ])),
        GT::RuleReference.new(name: "enum_body", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "SEMICOLON", is_token: true)),
      ]),
      line_number: 728,
    ),
    GT::GrammarRule.new(
      name: "enum_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "internal"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "new"),
      ]),
      line_number: 732,
    ),
    GT::GrammarRule.new(
      name: "integral_type",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "byte"),
        GT::Literal.new(value: "sbyte"),
        GT::Literal.new(value: "short"),
        GT::Literal.new(value: "ushort"),
        GT::Literal.new(value: "int"),
        GT::Literal.new(value: "uint"),
        GT::Literal.new(value: "long"),
        GT::Literal.new(value: "ulong"),
      ]),
      line_number: 738,
    ),
    GT::GrammarRule.new(
      name: "enum_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "enum_member_declarations", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 747,
    ),
    GT::GrammarRule.new(
      name: "enum_member_declarations",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "enum_member_declaration", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "enum_member_declaration", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
      ]),
      line_number: 749,
    ),
    GT::GrammarRule.new(
      name: "enum_member_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
      ]),
      line_number: 752,
    ),
    GT::GrammarRule.new(
      name: "delegate_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "delegate_modifier", is_token: false)),
        GT::Literal.new(value: "delegate"),
        GT::RuleReference.new(name: "return_type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "type_parameter_constraints_clause", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 761,
    ),
    GT::GrammarRule.new(
      name: "delegate_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "internal"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "new"),
      ]),
      line_number: 767,
    ),
    GT::GrammarRule.new(
      name: "type",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "value_type", is_token: false),
          GT::Repetition.new(element: GT::RuleReference.new(name: "rank_specifier", is_token: false)),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "QUESTION", is_token: true)),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "reference_type", is_token: false),
          GT::Repetition.new(element: GT::RuleReference.new(name: "rank_specifier", is_token: false)),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "void"),
          GT::RuleReference.new(name: "STAR", is_token: true),
        ]),
      ]),
      line_number: 794,
    ),
    GT::GrammarRule.new(
      name: "value_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "primitive_type", is_token: false),
        GT::RuleReference.new(name: "type_name", is_token: false),
      ]),
      line_number: 798,
    ),
    GT::GrammarRule.new(
      name: "reference_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "type_name", is_token: false),
        GT::Literal.new(value: "object"),
        GT::Literal.new(value: "string"),
      ]),
      line_number: 801,
    ),
    GT::GrammarRule.new(
      name: "primitive_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "numeric_type", is_token: false),
        GT::Literal.new(value: "bool"),
      ]),
      line_number: 805,
    ),
    GT::GrammarRule.new(
      name: "numeric_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "integral_type", is_token: false),
        GT::RuleReference.new(name: "floating_point_type", is_token: false),
        GT::Literal.new(value: "decimal"),
      ]),
      line_number: 808,
    ),
    GT::GrammarRule.new(
      name: "floating_point_type",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "float"),
        GT::Literal.new(value: "double"),
      ]),
      line_number: 812,
    ),
    GT::GrammarRule.new(
      name: "rank_specifier",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 815,
    ),
    GT::GrammarRule.new(
      name: "pointer_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "STAR", is_token: true),
      ]),
      line_number: 817,
    ),
    GT::GrammarRule.new(
      name: "block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 840,
    ),
    GT::GrammarRule.new(
      name: "statement",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "block", is_token: false),
        GT::RuleReference.new(name: "local_variable_declaration_statement", is_token: false),
        GT::RuleReference.new(name: "local_constant_declaration_statement", is_token: false),
        GT::RuleReference.new(name: "empty_statement", is_token: false),
        GT::RuleReference.new(name: "expression_statement", is_token: false),
        GT::RuleReference.new(name: "if_statement", is_token: false),
        GT::RuleReference.new(name: "while_statement", is_token: false),
        GT::RuleReference.new(name: "do_while_statement", is_token: false),
        GT::RuleReference.new(name: "for_statement", is_token: false),
        GT::RuleReference.new(name: "foreach_statement", is_token: false),
        GT::RuleReference.new(name: "switch_statement", is_token: false),
        GT::RuleReference.new(name: "try_statement", is_token: false),
        GT::RuleReference.new(name: "throw_statement", is_token: false),
        GT::RuleReference.new(name: "return_statement", is_token: false),
        GT::RuleReference.new(name: "break_statement", is_token: false),
        GT::RuleReference.new(name: "continue_statement", is_token: false),
        GT::RuleReference.new(name: "goto_statement", is_token: false),
        GT::RuleReference.new(name: "lock_statement", is_token: false),
        GT::RuleReference.new(name: "using_statement", is_token: false),
        GT::RuleReference.new(name: "checked_statement", is_token: false),
        GT::RuleReference.new(name: "unchecked_statement", is_token: false),
        GT::RuleReference.new(name: "labelled_statement", is_token: false),
        GT::RuleReference.new(name: "unsafe_statement", is_token: false),
        GT::RuleReference.new(name: "fixed_statement", is_token: false),
        GT::RuleReference.new(name: "yield_statement", is_token: false),
      ]),
      line_number: 842,
    ),
    GT::GrammarRule.new(
      name: "local_variable_declaration_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "variable_declarators", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 882,
    ),
    GT::GrammarRule.new(
      name: "local_constant_declaration_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "const"),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "constant_declarators", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 884,
    ),
    GT::GrammarRule.new(
      name: "empty_statement",
      body: GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      line_number: 886,
    ),
    GT::GrammarRule.new(
      name: "expression_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 888,
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
      line_number: 890,
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
      line_number: 892,
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
      line_number: 894,
    ),
    GT::GrammarRule.new(
      name: "for_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "for"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "for_initializer", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "for_iterator", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 896,
    ),
    GT::GrammarRule.new(
      name: "for_initializer",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "local_variable_declaration", is_token: false),
        GT::RuleReference.new(name: "expression_list", is_token: false),
      ]),
      line_number: 899,
    ),
    GT::GrammarRule.new(
      name: "for_iterator",
      body: GT::RuleReference.new(name: "expression_list", is_token: false),
      line_number: 902,
    ),
    GT::GrammarRule.new(
      name: "local_variable_declaration",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "variable_declarators", is_token: false),
      ]),
      line_number: 904,
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
      line_number: 906,
    ),
    GT::GrammarRule.new(
      name: "foreach_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "foreach"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "in"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 908,
    ),
    GT::GrammarRule.new(
      name: "switch_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "switch"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "switch_block", is_token: false),
      ]),
      line_number: 910,
    ),
    GT::GrammarRule.new(
      name: "switch_block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "switch_section", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 912,
    ),
    GT::GrammarRule.new(
      name: "switch_section",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "switch_label", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
      ]),
      line_number: 914,
    ),
    GT::GrammarRule.new(
      name: "switch_label",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "case"),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "COLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "default"),
          GT::RuleReference.new(name: "COLON", is_token: true),
        ]),
      ]),
      line_number: 916,
    ),
    GT::GrammarRule.new(
      name: "try_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "try"),
        GT::RuleReference.new(name: "block", is_token: false),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "catch_clauses", is_token: false),
              GT::OptionalElement.new(element: GT::RuleReference.new(name: "finally_clause", is_token: false)),
            ]),
            GT::RuleReference.new(name: "finally_clause", is_token: false),
          ])),
      ]),
      line_number: 921,
    ),
    GT::GrammarRule.new(
      name: "catch_clauses",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "specific_catch_clause", is_token: false),
          GT::Repetition.new(element: GT::RuleReference.new(name: "specific_catch_clause", is_token: false)),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "general_catch_clause", is_token: false)),
        ]),
        GT::RuleReference.new(name: "general_catch_clause", is_token: false),
      ]),
      line_number: 924,
    ),
    GT::GrammarRule.new(
      name: "specific_catch_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "catch"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type_name", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 927,
    ),
    GT::GrammarRule.new(
      name: "general_catch_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "catch"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 929,
    ),
    GT::GrammarRule.new(
      name: "finally_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "finally"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 931,
    ),
    GT::GrammarRule.new(
      name: "throw_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "throw"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 933,
    ),
    GT::GrammarRule.new(
      name: "return_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "return"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 935,
    ),
    GT::GrammarRule.new(
      name: "break_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "break"),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 937,
    ),
    GT::GrammarRule.new(
      name: "continue_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "continue"),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 939,
    ),
    GT::GrammarRule.new(
      name: "goto_statement",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "goto"),
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "goto"),
          GT::Literal.new(value: "case"),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "goto"),
          GT::Literal.new(value: "default"),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
      ]),
      line_number: 941,
    ),
    GT::GrammarRule.new(
      name: "lock_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "lock"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 945,
    ),
    GT::GrammarRule.new(
      name: "using_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "using"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "resource_acquisition", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 947,
    ),
    GT::GrammarRule.new(
      name: "resource_acquisition",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "type", is_token: false),
          GT::RuleReference.new(name: "variable_declarators", is_token: false),
        ]),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 949,
    ),
    GT::GrammarRule.new(
      name: "checked_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "checked"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 952,
    ),
    GT::GrammarRule.new(
      name: "unchecked_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "unchecked"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 954,
    ),
    GT::GrammarRule.new(
      name: "labelled_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 956,
    ),
    GT::GrammarRule.new(
      name: "unsafe_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "unsafe"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 958,
    ),
    GT::GrammarRule.new(
      name: "fixed_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "fixed"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "variable_declarators", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 960,
    ),
    GT::GrammarRule.new(
      name: "yield_statement",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "yield"),
          GT::Literal.new(value: "return"),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "yield"),
          GT::Literal.new(value: "break"),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
      ]),
      line_number: 967,
    ),
    GT::GrammarRule.new(
      name: "expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "assignment_expression", is_token: false),
        GT::RuleReference.new(name: "lambda_expression", is_token: false),
        GT::RuleReference.new(name: "query_expression", is_token: false),
      ]),
      line_number: 1010,
    ),
    GT::GrammarRule.new(
      name: "lambda_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "lambda_parameters", is_token: false),
        GT::RuleReference.new(name: "LAMBDA", is_token: true),
        GT::RuleReference.new(name: "lambda_body", is_token: false),
      ]),
      line_number: 1054,
    ),
    GT::GrammarRule.new(
      name: "lambda_parameters",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "lambda_parameter_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 1056,
    ),
    GT::GrammarRule.new(
      name: "lambda_parameter_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "lambda_parameter", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "lambda_parameter", is_token: false),
          ])),
      ]),
      line_number: 1059,
    ),
    GT::GrammarRule.new(
      name: "lambda_parameter",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type", is_token: false)),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 1061,
    ),
    GT::GrammarRule.new(
      name: "lambda_body",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 1063,
    ),
    GT::GrammarRule.new(
      name: "query_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "from_clause", is_token: false),
        GT::RuleReference.new(name: "query_body", is_token: false),
      ]),
      line_number: 1113,
    ),
    GT::GrammarRule.new(
      name: "from_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "from"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type", is_token: false)),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "in"),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 1115,
    ),
    GT::GrammarRule.new(
      name: "query_body",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "query_body_clause", is_token: false)),
        GT::RuleReference.new(name: "select_or_group_clause", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "query_continuation", is_token: false)),
      ]),
      line_number: 1117,
    ),
    GT::GrammarRule.new(
      name: "query_body_clause",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "from_clause", is_token: false),
        GT::RuleReference.new(name: "let_clause", is_token: false),
        GT::RuleReference.new(name: "where_clause", is_token: false),
        GT::RuleReference.new(name: "join_clause", is_token: false),
        GT::RuleReference.new(name: "orderby_clause", is_token: false),
      ]),
      line_number: 1119,
    ),
    GT::GrammarRule.new(
      name: "let_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "let"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 1125,
    ),
    GT::GrammarRule.new(
      name: "where_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "where"),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 1127,
    ),
    GT::GrammarRule.new(
      name: "join_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "join"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type", is_token: false)),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "in"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Literal.new(value: "on"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Literal.new(value: "equals"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "into"),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 1129,
    ),
    GT::GrammarRule.new(
      name: "orderby_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "orderby"),
        GT::RuleReference.new(name: "ordering", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "ordering", is_token: false),
          ])),
      ]),
      line_number: 1133,
    ),
    GT::GrammarRule.new(
      name: "ordering",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "ascending"),
            GT::Literal.new(value: "descending"),
          ])),
      ]),
      line_number: 1135,
    ),
    GT::GrammarRule.new(
      name: "select_or_group_clause",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "select"),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "group"),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::Literal.new(value: "by"),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
      ]),
      line_number: 1137,
    ),
    GT::GrammarRule.new(
      name: "query_continuation",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "into"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "query_body", is_token: false),
      ]),
      line_number: 1140,
    ),
    GT::GrammarRule.new(
      name: "assignment_expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "conditional_expression", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "unary_expression", is_token: false),
          GT::RuleReference.new(name: "assignment_operator", is_token: false),
          GT::RuleReference.new(name: "assignment_expression", is_token: false),
        ]),
      ]),
      line_number: 1146,
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
      ]),
      line_number: 1149,
    ),
    GT::GrammarRule.new(
      name: "conditional_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "null_coalescing_expression", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "QUESTION", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
      ]),
      line_number: 1161,
    ),
    GT::GrammarRule.new(
      name: "null_coalescing_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "logical_or_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "QUESTION_QUESTION", is_token: true),
            GT::RuleReference.new(name: "logical_or_expression", is_token: false),
          ])),
      ]),
      line_number: 1166,
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
      line_number: 1168,
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
      line_number: 1170,
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
      line_number: 1172,
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
      line_number: 1174,
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
      line_number: 1176,
    ),
    GT::GrammarRule.new(
      name: "equality_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "relational_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "EQUALS_EQUALS", is_token: true),
                GT::RuleReference.new(name: "NOT_EQUALS", is_token: true),
              ])),
            GT::RuleReference.new(name: "relational_expression", is_token: false),
          ])),
      ]),
      line_number: 1178,
    ),
    GT::GrammarRule.new(
      name: "relational_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "shift_expression", is_token: false),
        GT::Repetition.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::Group.new(element: GT::Alternation.new(choices: [
                  GT::RuleReference.new(name: "LESS_THAN", is_token: true),
                  GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
                  GT::RuleReference.new(name: "LESS_EQUALS", is_token: true),
                  GT::RuleReference.new(name: "GREATER_EQUALS", is_token: true),
                ])),
              GT::RuleReference.new(name: "shift_expression", is_token: false),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "is"),
              GT::RuleReference.new(name: "type", is_token: false),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "as"),
              GT::RuleReference.new(name: "type", is_token: false),
            ]),
          ])),
      ]),
      line_number: 1181,
    ),
    GT::GrammarRule.new(
      name: "shift_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "additive_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "LEFT_SHIFT", is_token: true),
                GT::RuleReference.new(name: "RIGHT_SHIFT", is_token: true),
              ])),
            GT::RuleReference.new(name: "additive_expression", is_token: false),
          ])),
      ]),
      line_number: 1187,
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
      line_number: 1190,
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
      line_number: 1193,
    ),
    GT::GrammarRule.new(
      name: "unary_expression",
      body: GT::Alternation.new(choices: [
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
          GT::RuleReference.new(name: "BANG", is_token: true),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "TILDE", is_token: true),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
        GT::RuleReference.new(name: "cast_expression", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "AMPERSAND", is_token: true),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "STAR", is_token: true),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
        GT::RuleReference.new(name: "postfix_expression", is_token: false),
      ]),
      line_number: 1202,
    ),
    GT::GrammarRule.new(
      name: "cast_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "unary_expression", is_token: false),
      ]),
      line_number: 1213,
    ),
    GT::GrammarRule.new(
      name: "postfix_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "primary_expression", is_token: false),
        GT::Repetition.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "PLUS_PLUS", is_token: true),
            GT::RuleReference.new(name: "MINUS_MINUS", is_token: true),
          ])),
      ]),
      line_number: 1215,
    ),
    GT::GrammarRule.new(
      name: "primary_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "primary", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "primary_suffix", is_token: false)),
      ]),
      line_number: 1224,
    ),
    GT::GrammarRule.new(
      name: "primary_suffix",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOT", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_argument_list", is_token: false)),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "argument_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::RuleReference.new(name: "expression_list", is_token: false),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "ARROW", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
      ]),
      line_number: 1226,
    ),
    GT::GrammarRule.new(
      name: "primary",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "literal", is_token: false),
        GT::Literal.new(value: "this"),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "base"),
          GT::RuleReference.new(name: "DOT", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "base"),
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::RuleReference.new(name: "expression_list", is_token: false),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
        GT::RuleReference.new(name: "typeof_expression", is_token: false),
        GT::RuleReference.new(name: "sizeof_expression", is_token: false),
        GT::RuleReference.new(name: "checked_expression", is_token: false),
        GT::RuleReference.new(name: "unchecked_expression", is_token: false),
        GT::RuleReference.new(name: "default_value_expression", is_token: false),
        GT::RuleReference.new(name: "new_expression", is_token: false),
        GT::RuleReference.new(name: "anonymous_method_expression", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_argument_list", is_token: false)),
        ]),
      ]),
      line_number: 1231,
    ),
    GT::GrammarRule.new(
      name: "typeof_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "typeof"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type_or_void", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1247,
    ),
    GT::GrammarRule.new(
      name: "type_or_void",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::Literal.new(value: "void"),
      ]),
      line_number: 1249,
    ),
    GT::GrammarRule.new(
      name: "sizeof_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "sizeof"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1251,
    ),
    GT::GrammarRule.new(
      name: "checked_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "checked"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1253,
    ),
    GT::GrammarRule.new(
      name: "unchecked_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "unchecked"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1255,
    ),
    GT::GrammarRule.new(
      name: "default_value_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "default"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1257,
    ),
    GT::GrammarRule.new(
      name: "anonymous_method_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "delegate"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameter_list", is_token: false)),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 1266,
    ),
    GT::GrammarRule.new(
      name: "new_expression",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "new"),
          GT::RuleReference.new(name: "new_anonymous_type", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "new"),
          GT::RuleReference.new(name: "new_implicitly_typed_array", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "new"),
          GT::RuleReference.new(name: "new_object_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "new"),
          GT::RuleReference.new(name: "new_array_expression", is_token: false),
        ]),
      ]),
      line_number: 1301,
    ),
    GT::GrammarRule.new(
      name: "new_anonymous_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "anonymous_type_member", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "anonymous_type_member", is_token: false),
              ])),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 1309,
    ),
    GT::GrammarRule.new(
      name: "anonymous_type_member",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "EQUALS", is_token: true),
          ])),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 1311,
    ),
    GT::GrammarRule.new(
      name: "new_implicitly_typed_array",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
        GT::RuleReference.new(name: "array_initializer", is_token: false),
      ]),
      line_number: 1315,
    ),
    GT::GrammarRule.new(
      name: "new_object_expression",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "type_name", is_token: false),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "argument_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "object_or_collection_initializer", is_token: false)),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "type_name", is_token: false),
          GT::RuleReference.new(name: "object_or_collection_initializer", is_token: false),
        ]),
      ]),
      line_number: 1322,
    ),
    GT::GrammarRule.new(
      name: "object_or_collection_initializer",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "initializer_list", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 1325,
    ),
    GT::GrammarRule.new(
      name: "initializer_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "initializer_item", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "initializer_item", is_token: false),
          ])),
      ]),
      line_number: 1327,
    ),
    GT::GrammarRule.new(
      name: "initializer_item",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "EQUALS", is_token: true),
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::RuleReference.new(name: "object_or_collection_initializer", is_token: false),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "expression_list", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 1333,
    ),
    GT::GrammarRule.new(
      name: "new_array_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "array_type", is_token: false),
        GT::RuleReference.new(name: "array_creation_suffix", is_token: false),
      ]),
      line_number: 1337,
    ),
    GT::GrammarRule.new(
      name: "array_type",
      body: GT::Group.new(element: GT::Alternation.new(choices: [
          GT::RuleReference.new(name: "primitive_type", is_token: false),
          GT::RuleReference.new(name: "type_name", is_token: false),
        ])),
      line_number: 1339,
    ),
    GT::GrammarRule.new(
      name: "array_creation_suffix",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "rank_specifier", is_token: false),
          GT::Repetition.new(element: GT::RuleReference.new(name: "rank_specifier", is_token: false)),
          GT::RuleReference.new(name: "array_initializer", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::RuleReference.new(name: "expression_list", is_token: false),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
          GT::Repetition.new(element: GT::RuleReference.new(name: "rank_specifier", is_token: false)),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "array_initializer", is_token: false)),
        ]),
      ]),
      line_number: 1341,
    ),
    GT::GrammarRule.new(
      name: "argument_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "argument", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "argument", is_token: false),
          ])),
      ]),
      line_number: 1352,
    ),
    GT::GrammarRule.new(
      name: "argument",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "ref"),
            GT::Literal.new(value: "out"),
          ])),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 1354,
    ),
    GT::GrammarRule.new(
      name: "literal",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "CHAR", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "VERBATIM_STRING", is_token: true),
        GT::Literal.new(value: "true"),
        GT::Literal.new(value: "false"),
        GT::Literal.new(value: "null"),
      ]),
      line_number: 1362,
    ),
  ],
)
