# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: java17.grammar
# Regenerate with: grammar-tools compile-grammar java17.grammar
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
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "program_item", is_token: false)),
      line_number: 231,
    ),
    GT::GrammarRule.new(
      name: "program_item",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "package_declaration", is_token: false),
        GT::RuleReference.new(name: "import_declaration", is_token: false),
        GT::RuleReference.new(name: "type_declaration", is_token: false),
        GT::RuleReference.new(name: "method_declaration", is_token: false),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 232,
    ),
    GT::GrammarRule.new(
      name: "compilation_unit",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "package_declaration", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "import_declaration", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "type_declaration", is_token: false)),
      ]),
      line_number: 233,
    ),
    GT::GrammarRule.new(
      name: "package_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "package"),
        GT::RuleReference.new(name: "qualified_name", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 249,
    ),
    GT::GrammarRule.new(
      name: "import_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "import"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "static")),
        GT::RuleReference.new(name: "qualified_name", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "DOT", is_token: true),
            GT::RuleReference.new(name: "STAR", is_token: true),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 264,
    ),
    GT::GrammarRule.new(
      name: "type_declaration",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "class_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_declaration", is_token: false),
        GT::RuleReference.new(name: "enum_declaration", is_token: false),
        GT::RuleReference.new(name: "annotation_type_declaration", is_token: false),
        GT::RuleReference.new(name: "record_declaration", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 280,
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
      line_number: 299,
    ),
    GT::GrammarRule.new(
      name: "annotation",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "AT", is_token: true),
        GT::RuleReference.new(name: "qualified_name", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::OptionalElement.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "element_value_pairs", is_token: false),
                GT::RuleReference.new(name: "element_value", is_token: false),
              ])),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
      ]),
      line_number: 322,
    ),
    GT::GrammarRule.new(
      name: "annotations",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
      line_number: 324,
    ),
    GT::GrammarRule.new(
      name: "element_value_pairs",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "element_value_pair", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "element_value_pair", is_token: false),
          ])),
      ]),
      line_number: 326,
    ),
    GT::GrammarRule.new(
      name: "element_value_pair",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "element_value", is_token: false),
      ]),
      line_number: 328,
    ),
    GT::GrammarRule.new(
      name: "element_value",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "annotation", is_token: false),
        GT::RuleReference.new(name: "element_value_array", is_token: false),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 336,
    ),
    GT::GrammarRule.new(
      name: "element_value_array",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "element_value", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "element_value", is_token: false),
              ])),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 340,
    ),
    GT::GrammarRule.new(
      name: "annotation_type_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "class_modifier", is_token: false)),
        GT::RuleReference.new(name: "AT", is_token: true),
        GT::Literal.new(value: "interface"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "annotation_type_body", is_token: false),
      ]),
      line_number: 356,
    ),
    GT::GrammarRule.new(
      name: "annotation_type_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "annotation_type_element_declaration", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 359,
    ),
    GT::GrammarRule.new(
      name: "annotation_type_element_declaration",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "annotation_element_declaration", is_token: false),
        GT::RuleReference.new(name: "field_declaration", is_token: false),
        GT::RuleReference.new(name: "class_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_declaration", is_token: false),
        GT::RuleReference.new(name: "enum_declaration", is_token: false),
        GT::RuleReference.new(name: "annotation_type_declaration", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 361,
    ),
    GT::GrammarRule.new(
      name: "annotation_element_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "method_modifier", is_token: false)),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "default"),
            GT::RuleReference.new(name: "element_value", is_token: false),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 369,
    ),
    GT::GrammarRule.new(
      name: "class_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "class_modifier", is_token: false)),
        GT::Literal.new(value: "class"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "extends"),
            GT::RuleReference.new(name: "class_type", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "implements"),
            GT::RuleReference.new(name: "interface_type_list", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "permits_clause", is_token: false)),
        GT::RuleReference.new(name: "class_body", is_token: false),
      ]),
      line_number: 406,
    ),
    GT::GrammarRule.new(
      name: "class_modifier",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "annotation", is_token: false),
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "abstract"),
        GT::Literal.new(value: "final"),
        GT::Literal.new(value: "static"),
        GT::Literal.new(value: "strictfp"),
        GT::RuleReference.new(name: "non_sealed_modifier", is_token: false),
        GT::Literal.new(value: "sealed"),
      ]),
      line_number: 431,
    ),
    GT::GrammarRule.new(
      name: "non_sealed_modifier",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "non"),
        GT::RuleReference.new(name: "MINUS", is_token: true),
        GT::Literal.new(value: "sealed"),
      ]),
      line_number: 458,
    ),
    GT::GrammarRule.new(
      name: "permits_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "permits"),
        GT::RuleReference.new(name: "class_type", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "class_type", is_token: false),
          ])),
      ]),
      line_number: 471,
    ),
    GT::GrammarRule.new(
      name: "class_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "class_body_declaration", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 473,
    ),
    GT::GrammarRule.new(
      name: "class_body_declaration",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "static_initializer", is_token: false),
        GT::RuleReference.new(name: "instance_initializer", is_token: false),
        GT::RuleReference.new(name: "constructor_declaration", is_token: false),
        GT::RuleReference.new(name: "method_declaration", is_token: false),
        GT::RuleReference.new(name: "field_declaration", is_token: false),
        GT::RuleReference.new(name: "class_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_declaration", is_token: false),
        GT::RuleReference.new(name: "enum_declaration", is_token: false),
        GT::RuleReference.new(name: "annotation_type_declaration", is_token: false),
        GT::RuleReference.new(name: "record_declaration", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 475,
    ),
    GT::GrammarRule.new(
      name: "interface_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "interface_modifier", is_token: false)),
        GT::Literal.new(value: "interface"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "extends"),
            GT::RuleReference.new(name: "interface_type_list", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "permits_clause", is_token: false)),
        GT::RuleReference.new(name: "interface_body", is_token: false),
      ]),
      line_number: 523,
    ),
    GT::GrammarRule.new(
      name: "interface_modifier",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "annotation", is_token: false),
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "abstract"),
        GT::Literal.new(value: "static"),
        GT::Literal.new(value: "strictfp"),
        GT::RuleReference.new(name: "non_sealed_modifier", is_token: false),
        GT::Literal.new(value: "sealed"),
      ]),
      line_number: 528,
    ),
    GT::GrammarRule.new(
      name: "interface_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "interface_body_declaration", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 538,
    ),
    GT::GrammarRule.new(
      name: "interface_body_declaration",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "interface_method_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_field_declaration", is_token: false),
        GT::RuleReference.new(name: "class_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_declaration", is_token: false),
        GT::RuleReference.new(name: "enum_declaration", is_token: false),
        GT::RuleReference.new(name: "annotation_type_declaration", is_token: false),
        GT::RuleReference.new(name: "record_declaration", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 544,
    ),
    GT::GrammarRule.new(
      name: "interface_field_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "field_modifier", is_token: false)),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "variable_declarators", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 553,
    ),
    GT::GrammarRule.new(
      name: "interface_method_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "interface_method_modifier", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "result_type", is_token: false),
        GT::RuleReference.new(name: "method_declarator", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "throws_clause", is_token: false)),
        GT::RuleReference.new(name: "method_body", is_token: false),
      ]),
      line_number: 576,
    ),
    GT::GrammarRule.new(
      name: "interface_method_modifier",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "annotation", is_token: false),
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "abstract"),
        GT::Literal.new(value: "default"),
        GT::Literal.new(value: "static"),
        GT::Literal.new(value: "strictfp"),
      ]),
      line_number: 579,
    ),
    GT::GrammarRule.new(
      name: "interface_type_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "class_type", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "class_type", is_token: false),
          ])),
      ]),
      line_number: 587,
    ),
    GT::GrammarRule.new(
      name: "enum_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "class_modifier", is_token: false)),
        GT::Literal.new(value: "enum"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "implements"),
            GT::RuleReference.new(name: "interface_type_list", is_token: false),
          ])),
        GT::RuleReference.new(name: "enum_body", is_token: false),
      ]),
      line_number: 607,
    ),
    GT::GrammarRule.new(
      name: "enum_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "enum_constant_list", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            GT::Repetition.new(element: GT::RuleReference.new(name: "class_body_declaration", is_token: false)),
          ])),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 611,
    ),
    GT::GrammarRule.new(
      name: "enum_constant_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "enum_constant", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "enum_constant", is_token: false),
          ])),
      ]),
      line_number: 613,
    ),
    GT::GrammarRule.new(
      name: "enum_constant",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "annotations", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "argument_list", is_token: false)),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "class_body", is_token: false)),
      ]),
      line_number: 615,
    ),
    GT::GrammarRule.new(
      name: "record_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "class_modifier", is_token: false)),
        GT::Literal.new(value: "record"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "record_components", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "implements"),
            GT::RuleReference.new(name: "interface_type_list", is_token: false),
          ])),
        GT::RuleReference.new(name: "record_body", is_token: false),
      ]),
      line_number: 674,
    ),
    GT::GrammarRule.new(
      name: "record_components",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "record_component_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 689,
    ),
    GT::GrammarRule.new(
      name: "record_component_list",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "record_component", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "record_component", is_token: false),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "record_component", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "record_component", is_token: false),
            ])),
          GT::RuleReference.new(name: "COMMA", is_token: true),
          GT::RuleReference.new(name: "varargs_record_component", is_token: false),
        ]),
        GT::RuleReference.new(name: "varargs_record_component", is_token: false),
      ]),
      line_number: 691,
    ),
    GT::GrammarRule.new(
      name: "record_component",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 695,
    ),
    GT::GrammarRule.new(
      name: "varargs_record_component",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "ELLIPSIS", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 697,
    ),
    GT::GrammarRule.new(
      name: "record_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "record_body_declaration", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 718,
    ),
    GT::GrammarRule.new(
      name: "record_body_declaration",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "compact_constructor_declaration", is_token: false),
        GT::RuleReference.new(name: "constructor_declaration", is_token: false),
        GT::RuleReference.new(name: "method_declaration", is_token: false),
        GT::RuleReference.new(name: "field_declaration", is_token: false),
        GT::RuleReference.new(name: "class_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_declaration", is_token: false),
        GT::RuleReference.new(name: "enum_declaration", is_token: false),
        GT::RuleReference.new(name: "annotation_type_declaration", is_token: false),
        GT::RuleReference.new(name: "record_declaration", is_token: false),
        GT::RuleReference.new(name: "static_initializer", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 720,
    ),
    GT::GrammarRule.new(
      name: "compact_constructor_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "constructor_modifier", is_token: false)),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 740,
    ),
    GT::GrammarRule.new(
      name: "type_parameters",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LESS_THAN", is_token: true),
        GT::RuleReference.new(name: "type_parameter", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "type_parameter", is_token: false),
          ])),
        GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
      ]),
      line_number: 770,
    ),
    GT::GrammarRule.new(
      name: "type_parameter",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "extends"),
            GT::RuleReference.new(name: "bound", is_token: false),
          ])),
      ]),
      line_number: 772,
    ),
    GT::GrammarRule.new(
      name: "bound",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "class_type", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "AMPERSAND", is_token: true),
            GT::RuleReference.new(name: "class_type", is_token: false),
          ])),
      ]),
      line_number: 774,
    ),
    GT::GrammarRule.new(
      name: "type_arguments",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LESS_THAN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "type_argument", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "type_argument", is_token: false),
              ])),
          ])),
        GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
      ]),
      line_number: 788,
    ),
    GT::GrammarRule.new(
      name: "type_argument",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
          GT::RuleReference.new(name: "type", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
          GT::RuleReference.new(name: "QUESTION", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::Group.new(element: GT::Alternation.new(choices: [
                  GT::Literal.new(value: "extends"),
                  GT::Literal.new(value: "super"),
                ])),
              GT::RuleReference.new(name: "type", is_token: false),
            ])),
        ]),
      ]),
      line_number: 790,
    ),
    GT::GrammarRule.new(
      name: "field_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "field_modifier", is_token: false)),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "variable_declarators", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 808,
    ),
    GT::GrammarRule.new(
      name: "field_modifier",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "annotation", is_token: false),
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "static"),
        GT::Literal.new(value: "final"),
        GT::Literal.new(value: "transient"),
        GT::Literal.new(value: "volatile"),
      ]),
      line_number: 810,
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
      line_number: 819,
    ),
    GT::GrammarRule.new(
      name: "variable_declarator",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LBRACKET", is_token: true),
            GT::RuleReference.new(name: "RBRACKET", is_token: true),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "variable_initializer", is_token: false),
          ])),
      ]),
      line_number: 821,
    ),
    GT::GrammarRule.new(
      name: "variable_initializer",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "array_initializer", is_token: false),
      ]),
      line_number: 823,
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
      line_number: 826,
    ),
    GT::GrammarRule.new(
      name: "method_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "method_modifier", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "result_type", is_token: false),
        GT::RuleReference.new(name: "method_declarator", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "throws_clause", is_token: false)),
        GT::RuleReference.new(name: "method_body", is_token: false),
      ]),
      line_number: 843,
    ),
    GT::GrammarRule.new(
      name: "method_modifier",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "annotation", is_token: false),
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "static"),
        GT::Literal.new(value: "abstract"),
        GT::Literal.new(value: "final"),
        GT::Literal.new(value: "synchronized"),
        GT::Literal.new(value: "native"),
        GT::Literal.new(value: "strictfp"),
      ]),
      line_number: 846,
    ),
    GT::GrammarRule.new(
      name: "result_type",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "void"),
        GT::RuleReference.new(name: "type", is_token: false),
      ]),
      line_number: 857,
    ),
    GT::GrammarRule.new(
      name: "method_declarator",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LBRACKET", is_token: true),
            GT::RuleReference.new(name: "RBRACKET", is_token: true),
          ])),
      ]),
      line_number: 860,
    ),
    GT::GrammarRule.new(
      name: "formal_parameter_list",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "formal_parameter", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "formal_parameter", is_token: false),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "formal_parameter", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "formal_parameter", is_token: false),
            ])),
          GT::RuleReference.new(name: "COMMA", is_token: true),
          GT::RuleReference.new(name: "varargs_parameter", is_token: false),
        ]),
        GT::RuleReference.new(name: "varargs_parameter", is_token: false),
      ]),
      line_number: 877,
    ),
    GT::GrammarRule.new(
      name: "formal_parameter",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "final")),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LBRACKET", is_token: true),
            GT::RuleReference.new(name: "RBRACKET", is_token: true),
          ])),
      ]),
      line_number: 881,
    ),
    GT::GrammarRule.new(
      name: "varargs_parameter",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "final")),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "ELLIPSIS", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 883,
    ),
    GT::GrammarRule.new(
      name: "throws_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "throws"),
        GT::RuleReference.new(name: "class_type", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "class_type", is_token: false),
          ])),
      ]),
      line_number: 885,
    ),
    GT::GrammarRule.new(
      name: "method_body",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "block", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 887,
    ),
    GT::GrammarRule.new(
      name: "constructor_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "constructor_modifier", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "throws_clause", is_token: false)),
        GT::RuleReference.new(name: "constructor_body", is_token: false),
      ]),
      line_number: 904,
    ),
    GT::GrammarRule.new(
      name: "constructor_modifier",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "annotation", is_token: false),
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "private"),
      ]),
      line_number: 908,
    ),
    GT::GrammarRule.new(
      name: "constructor_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "explicit_constructor_invocation", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "block_statement", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 913,
    ),
    GT::GrammarRule.new(
      name: "explicit_constructor_invocation",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_arguments", is_token: false)),
          GT::Literal.new(value: "this"),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "argument_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_arguments", is_token: false)),
          GT::Literal.new(value: "super"),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "argument_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
      ]),
      line_number: 921,
    ),
    GT::GrammarRule.new(
      name: "static_initializer",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "static"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 933,
    ),
    GT::GrammarRule.new(
      name: "instance_initializer",
      body: GT::RuleReference.new(name: "block", is_token: false),
      line_number: 935,
    ),
    GT::GrammarRule.new(
      name: "type",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
          GT::RuleReference.new(name: "primitive_type", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LBRACKET", is_token: true),
              GT::RuleReference.new(name: "RBRACKET", is_token: true),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
          GT::RuleReference.new(name: "class_type", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LBRACKET", is_token: true),
              GT::RuleReference.new(name: "RBRACKET", is_token: true),
            ])),
        ]),
      ]),
      line_number: 965,
    ),
    GT::GrammarRule.new(
      name: "primitive_type",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "boolean"),
        GT::Literal.new(value: "byte"),
        GT::Literal.new(value: "short"),
        GT::Literal.new(value: "int"),
        GT::Literal.new(value: "long"),
        GT::Literal.new(value: "char"),
        GT::Literal.new(value: "float"),
        GT::Literal.new(value: "double"),
      ]),
      line_number: 968,
    ),
    GT::GrammarRule.new(
      name: "class_type",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
        GT::RuleReference.new(name: "qualified_name", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_arguments", is_token: false)),
      ]),
      line_number: 982,
    ),
    GT::GrammarRule.new(
      name: "local_var_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::Literal.new(value: "var"),
      ]),
      line_number: 1003,
    ),
    GT::GrammarRule.new(
      name: "statement",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "block", is_token: false),
        GT::RuleReference.new(name: "var_declaration", is_token: false),
        GT::RuleReference.new(name: "empty_statement", is_token: false),
        GT::RuleReference.new(name: "expression_statement", is_token: false),
        GT::RuleReference.new(name: "if_statement", is_token: false),
        GT::RuleReference.new(name: "while_statement", is_token: false),
        GT::RuleReference.new(name: "do_while_statement", is_token: false),
        GT::RuleReference.new(name: "for_statement", is_token: false),
        GT::RuleReference.new(name: "enhanced_for_statement", is_token: false),
        GT::RuleReference.new(name: "switch_statement", is_token: false),
        GT::RuleReference.new(name: "try_statement", is_token: false),
        GT::RuleReference.new(name: "throw_statement", is_token: false),
        GT::RuleReference.new(name: "return_statement", is_token: false),
        GT::RuleReference.new(name: "break_statement", is_token: false),
        GT::RuleReference.new(name: "continue_statement", is_token: false),
        GT::RuleReference.new(name: "yield_statement", is_token: false),
        GT::RuleReference.new(name: "synchronized_statement", is_token: false),
        GT::RuleReference.new(name: "assert_statement", is_token: false),
        GT::RuleReference.new(name: "labelled_statement", is_token: false),
      ]),
      line_number: 1041,
    ),
    GT::GrammarRule.new(
      name: "block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "block_statement", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 1063,
    ),
    GT::GrammarRule.new(
      name: "block_statement",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "var_declaration", is_token: false),
        GT::RuleReference.new(name: "class_declaration", is_token: false),
        GT::RuleReference.new(name: "record_declaration", is_token: false),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 1065,
    ),
    GT::GrammarRule.new(
      name: "var_declaration",
      body: GT::RuleReference.new(name: "local_variable_declaration_statement", is_token: false),
      line_number: 1083,
    ),
    GT::GrammarRule.new(
      name: "local_variable_declaration_statement",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "final")),
        GT::RuleReference.new(name: "local_var_type", is_token: false),
        GT::RuleReference.new(name: "variable_declarators", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1085,
    ),
    GT::GrammarRule.new(
      name: "empty_statement",
      body: GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      line_number: 1090,
    ),
    GT::GrammarRule.new(
      name: "expression_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1094,
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
      line_number: 1101,
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
      line_number: 1105,
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
      line_number: 1109,
    ),
    GT::GrammarRule.new(
      name: "for_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "for"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "for_init", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "for_update", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 1113,
    ),
    GT::GrammarRule.new(
      name: "for_init",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
          GT::OptionalElement.new(element: GT::Literal.new(value: "final")),
          GT::RuleReference.new(name: "local_var_type", is_token: false),
          GT::RuleReference.new(name: "variable_declarators", is_token: false),
        ]),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression_list", is_token: false)),
      ]),
      line_number: 1116,
    ),
    GT::GrammarRule.new(
      name: "for_update",
      body: GT::RuleReference.new(name: "expression_list", is_token: false),
      line_number: 1119,
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
      line_number: 1121,
    ),
    GT::GrammarRule.new(
      name: "enhanced_for_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "for"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "final")),
        GT::RuleReference.new(name: "local_var_type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 1129,
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
      line_number: 1182,
    ),
    GT::GrammarRule.new(
      name: "switch_block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "switch_rule", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 1188,
    ),
    GT::GrammarRule.new(
      name: "switch_rule",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "switch_label", is_token: false),
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::Repetition.new(element: GT::RuleReference.new(name: "block_statement", is_token: false)),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "switch_label", is_token: false),
          GT::RuleReference.new(name: "ARROW", is_token: true),
          GT::RuleReference.new(name: "switch_rule_body", is_token: false),
        ]),
      ]),
      line_number: 1199,
    ),
    GT::GrammarRule.new(
      name: "switch_rule_body",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "block", is_token: false),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "throw"),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
      ]),
      line_number: 1202,
    ),
    GT::GrammarRule.new(
      name: "switch_label",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "case"),
          GT::RuleReference.new(name: "case_constant", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "case_constant", is_token: false),
            ])),
        ]),
        GT::Literal.new(value: "default"),
      ]),
      line_number: 1215,
    ),
    GT::GrammarRule.new(
      name: "case_constant",
      body: GT::RuleReference.new(name: "expression", is_token: false),
      line_number: 1218,
    ),
    GT::GrammarRule.new(
      name: "yield_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "yield"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1240,
    ),
    GT::GrammarRule.new(
      name: "try_statement",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "try"),
          GT::RuleReference.new(name: "resource_specification", is_token: false),
          GT::RuleReference.new(name: "block", is_token: false),
          GT::Repetition.new(element: GT::RuleReference.new(name: "catch_clause", is_token: false)),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "finally_clause", is_token: false)),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "try"),
          GT::RuleReference.new(name: "block", is_token: false),
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "catch_clause", is_token: false),
                GT::Repetition.new(element: GT::RuleReference.new(name: "catch_clause", is_token: false)),
                GT::OptionalElement.new(element: GT::RuleReference.new(name: "finally_clause", is_token: false)),
              ]),
              GT::RuleReference.new(name: "finally_clause", is_token: false),
            ])),
        ]),
      ]),
      line_number: 1256,
    ),
    GT::GrammarRule.new(
      name: "resource_specification",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "resource", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            GT::RuleReference.new(name: "resource", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "SEMICOLON", is_token: true)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1260,
    ),
    GT::GrammarRule.new(
      name: "resource",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
          GT::OptionalElement.new(element: GT::Literal.new(value: "final")),
          GT::RuleReference.new(name: "local_var_type", is_token: false),
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "EQUALS", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
        GT::RuleReference.new(name: "qualified_name", is_token: false),
      ]),
      line_number: 1262,
    ),
    GT::GrammarRule.new(
      name: "catch_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "catch"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "catch_formal_parameter", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 1267,
    ),
    GT::GrammarRule.new(
      name: "catch_formal_parameter",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "final")),
        GT::RuleReference.new(name: "catch_type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 1269,
    ),
    GT::GrammarRule.new(
      name: "catch_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "class_type", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "PIPE", is_token: true),
            GT::RuleReference.new(name: "class_type", is_token: false),
          ])),
      ]),
      line_number: 1271,
    ),
    GT::GrammarRule.new(
      name: "finally_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "finally"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 1273,
    ),
    GT::GrammarRule.new(
      name: "throw_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "throw"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1277,
    ),
    GT::GrammarRule.new(
      name: "return_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "return"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1281,
    ),
    GT::GrammarRule.new(
      name: "break_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "break"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1285,
    ),
    GT::GrammarRule.new(
      name: "continue_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "continue"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1287,
    ),
    GT::GrammarRule.new(
      name: "synchronized_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "synchronized"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 1291,
    ),
    GT::GrammarRule.new(
      name: "assert_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "assert"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1295,
    ),
    GT::GrammarRule.new(
      name: "labelled_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 1299,
    ),
    GT::GrammarRule.new(
      name: "expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "lambda_expression", is_token: false),
        GT::RuleReference.new(name: "assignment_expression", is_token: false),
      ]),
      line_number: 1346,
    ),
    GT::GrammarRule.new(
      name: "lambda_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "lambda_parameters", is_token: false),
        GT::RuleReference.new(name: "ARROW", is_token: true),
        GT::RuleReference.new(name: "lambda_body", is_token: false),
      ]),
      line_number: 1349,
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
      line_number: 1351,
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
      line_number: 1361,
    ),
    GT::GrammarRule.new(
      name: "lambda_parameter",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
          GT::OptionalElement.new(element: GT::Literal.new(value: "final")),
          GT::RuleReference.new(name: "type", is_token: false),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
          GT::OptionalElement.new(element: GT::Literal.new(value: "final")),
          GT::Literal.new(value: "var"),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
          GT::OptionalElement.new(element: GT::Literal.new(value: "final")),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
      ]),
      line_number: 1363,
    ),
    GT::GrammarRule.new(
      name: "lambda_body",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 1367,
    ),
    GT::GrammarRule.new(
      name: "assignment_expression",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "unary_expression", is_token: false),
          GT::RuleReference.new(name: "assignment_operator", is_token: false),
          GT::RuleReference.new(name: "assignment_expression", is_token: false),
        ]),
        GT::RuleReference.new(name: "conditional_expression", is_token: false),
      ]),
      line_number: 1374,
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
      line_number: 1377,
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
      line_number: 1392,
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
      line_number: 1397,
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
      line_number: 1401,
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
      line_number: 1405,
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
      line_number: 1409,
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
      line_number: 1413,
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
      line_number: 1417,
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
              GT::Literal.new(value: "instanceof"),
              GT::RuleReference.new(name: "pattern", is_token: false),
            ]),
          ])),
      ]),
      line_number: 1461,
    ),
    GT::GrammarRule.new(
      name: "pattern",
      body: GT::RuleReference.new(name: "type_pattern", is_token: false),
      line_number: 1492,
    ),
    GT::GrammarRule.new(
      name: "type_pattern",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::OptionalElement.new(element: GT::Literal.new(value: "final")),
          GT::Repetition.new(element: GT::RuleReference.new(name: "annotation", is_token: false)),
          GT::RuleReference.new(name: "type", is_token: false),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::RuleReference.new(name: "type", is_token: false),
      ]),
      line_number: 1494,
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
      line_number: 1499,
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
      line_number: 1504,
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
      line_number: 1509,
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
        GT::RuleReference.new(name: "unary_expression_not_plus_minus", is_token: false),
      ]),
      line_number: 1514,
    ),
    GT::GrammarRule.new(
      name: "unary_expression_not_plus_minus",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "TILDE", is_token: true),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "BANG", is_token: true),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
        GT::RuleReference.new(name: "cast_expression", is_token: false),
        GT::RuleReference.new(name: "postfix_expression", is_token: false),
      ]),
      line_number: 1520,
    ),
    GT::GrammarRule.new(
      name: "cast_expression",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "primitive_type", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LBRACKET", is_token: true),
              GT::RuleReference.new(name: "RBRACKET", is_token: true),
            ])),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "class_type", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "AMPERSAND", is_token: true),
              GT::RuleReference.new(name: "class_type", is_token: false),
            ])),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LBRACKET", is_token: true),
              GT::RuleReference.new(name: "RBRACKET", is_token: true),
            ])),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::RuleReference.new(name: "unary_expression_not_plus_minus", is_token: false),
        ]),
      ]),
      line_number: 1533,
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
      line_number: 1539,
    ),
    GT::GrammarRule.new(
      name: "primary_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "primary", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "primary_suffix", is_token: false)),
      ]),
      line_number: 1560,
    ),
    GT::GrammarRule.new(
      name: "primary_suffix",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOUBLE_COLON", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_arguments", is_token: false)),
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::RuleReference.new(name: "NAME", is_token: true),
              GT::Literal.new(value: "new"),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOT", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_arguments", is_token: false)),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOT", is_token: true),
          GT::Literal.new(value: "class"),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOT", is_token: true),
          GT::Literal.new(value: "this"),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOT", is_token: true),
          GT::Literal.new(value: "super"),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOT", is_token: true),
          GT::Literal.new(value: "new"),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_arguments", is_token: false)),
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_arguments", is_token: false)),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "argument_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "class_body", is_token: false)),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "argument_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
      ]),
      line_number: 1566,
    ),
    GT::GrammarRule.new(
      name: "primary",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "literal", is_token: false),
        GT::Literal.new(value: "this"),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "super"),
          GT::RuleReference.new(name: "DOUBLE_COLON", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_arguments", is_token: false)),
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::RuleReference.new(name: "NAME", is_token: true),
              GT::Literal.new(value: "new"),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "super"),
          GT::RuleReference.new(name: "DOT", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_arguments", is_token: false)),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "super"),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "argument_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "new"),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_arguments", is_token: false)),
          GT::RuleReference.new(name: "class_type", is_token: false),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "argument_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "class_body", is_token: false)),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "new"),
          GT::RuleReference.new(name: "array_creation_type", is_token: false),
          GT::RuleReference.new(name: "array_dimension_exprs", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LBRACKET", is_token: true),
              GT::RuleReference.new(name: "RBRACKET", is_token: true),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "new"),
          GT::RuleReference.new(name: "array_creation_type", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LBRACKET", is_token: true),
              GT::RuleReference.new(name: "RBRACKET", is_token: true),
            ])),
          GT::RuleReference.new(name: "array_initializer", is_token: false),
        ]),
        GT::RuleReference.new(name: "switch_expression", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 1596,
    ),
    GT::GrammarRule.new(
      name: "argument_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
      ]),
      line_number: 1608,
    ),
    GT::GrammarRule.new(
      name: "switch_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "switch"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "switch_block", is_token: false),
      ]),
      line_number: 1620,
    ),
    GT::GrammarRule.new(
      name: "array_creation_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "primitive_type", is_token: false),
        GT::RuleReference.new(name: "class_type", is_token: false),
      ]),
      line_number: 1624,
    ),
    GT::GrammarRule.new(
      name: "array_dimension_exprs",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LBRACKET", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
            GT::RuleReference.new(name: "RBRACKET", is_token: true),
          ])),
      ]),
      line_number: 1627,
    ),
    GT::GrammarRule.new(
      name: "literal",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "CHAR", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::Literal.new(value: "true"),
        GT::Literal.new(value: "false"),
        GT::Literal.new(value: "null"),
      ]),
      line_number: 1653,
    ),
  ],
)
