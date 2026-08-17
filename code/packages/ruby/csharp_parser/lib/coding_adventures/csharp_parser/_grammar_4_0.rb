# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: csharp4.0.grammar
# Regenerate with: grammar-tools compile-grammar csharp4.0.grammar
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
      line_number: 130,
    ),
    GT::GrammarRule.new(
      name: "extern_alias_directive",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "extern"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 139,
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
      line_number: 147,
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
      line_number: 154,
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
      line_number: 157,
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
      line_number: 159,
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
      line_number: 197,
    ),
    GT::GrammarRule.new(
      name: "type_parameter",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "variance_annotation", is_token: false)),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 201,
    ),
    GT::GrammarRule.new(
      name: "variance_annotation",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "in"),
        GT::Literal.new(value: "out"),
      ]),
      line_number: 203,
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
      line_number: 206,
    ),
    GT::GrammarRule.new(
      name: "type_parameter_constraints_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "where"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "type_parameter_constraints", is_token: false),
      ]),
      line_number: 212,
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
      line_number: 214,
    ),
    GT::GrammarRule.new(
      name: "primary_constraint",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "class"),
        GT::Literal.new(value: "struct"),
        GT::RuleReference.new(name: "namespace_or_type_name", is_token: false),
      ]),
      line_number: 219,
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
      line_number: 223,
    ),
    GT::GrammarRule.new(
      name: "constructor_constraint",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "new"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 225,
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
      line_number: 231,
    ),
    GT::GrammarRule.new(
      name: "global_attribute_target",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "assembly"),
        GT::Literal.new(value: "module"),
      ]),
      line_number: 233,
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
      line_number: 240,
    ),
    GT::GrammarRule.new(
      name: "namespace_member_declaration",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "namespace_declaration", is_token: false),
        GT::RuleReference.new(name: "type_declaration", is_token: false),
      ]),
      line_number: 250,
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
      line_number: 253,
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
      line_number: 264,
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
      line_number: 266,
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
      line_number: 274,
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
      line_number: 276,
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
      line_number: 278,
    ),
    GT::GrammarRule.new(
      name: "attribute_argument",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "COLON", is_token: true),
          ])),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 280,
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
      line_number: 290,
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
      line_number: 296,
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
      line_number: 305,
    ),
    GT::GrammarRule.new(
      name: "class_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "class_member_declaration", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 307,
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
      line_number: 322,
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
      line_number: 324,
    ),
    GT::GrammarRule.new(
      name: "type_argument",
      body: GT::RuleReference.new(name: "type", is_token: false),
      line_number: 326,
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
      line_number: 332,
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
      line_number: 350,
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
      line_number: 353,
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
      line_number: 359,
    ),
    GT::GrammarRule.new(
      name: "constant_declarator",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 361,
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
      line_number: 367,
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
      line_number: 370,
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
      line_number: 379,
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
      line_number: 381,
    ),
    GT::GrammarRule.new(
      name: "variable_initializer",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "array_initializer", is_token: false),
      ]),
      line_number: 383,
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
      line_number: 386,
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
      line_number: 395,
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
      line_number: 401,
    ),
    GT::GrammarRule.new(
      name: "return_type",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "void"),
        GT::RuleReference.new(name: "type", is_token: false),
      ]),
      line_number: 413,
    ),
    GT::GrammarRule.new(
      name: "method_body",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "block", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 416,
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
      line_number: 451,
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
      line_number: 454,
    ),
    GT::GrammarRule.new(
      name: "fixed_parameter",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "parameter_modifier", is_token: false)),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
      ]),
      line_number: 457,
    ),
    GT::GrammarRule.new(
      name: "parameter_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "ref"),
        GT::Literal.new(value: "out"),
        GT::Literal.new(value: "this"),
      ]),
      line_number: 460,
    ),
    GT::GrammarRule.new(
      name: "parameter_array",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Literal.new(value: "params"),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 464,
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
      line_number: 472,
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
      line_number: 475,
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
      line_number: 487,
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
      line_number: 490,
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
      line_number: 493,
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
      line_number: 496,
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
      line_number: 506,
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
      line_number: 510,
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
      line_number: 522,
    ),
    GT::GrammarRule.new(
      name: "add_accessor_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Literal.new(value: "add"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 525,
    ),
    GT::GrammarRule.new(
      name: "remove_accessor_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Literal.new(value: "remove"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 527,
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
      line_number: 533,
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
      line_number: 537,
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
      line_number: 552,
    ),
    GT::GrammarRule.new(
      name: "operator_modifiers",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "static"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "extern")),
      ]),
      line_number: 557,
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
      line_number: 559,
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
      line_number: 586,
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
      line_number: 594,
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
      line_number: 598,
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
      line_number: 604,
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
      line_number: 611,
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
      line_number: 618,
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
      line_number: 621,
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
      line_number: 628,
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
      line_number: 634,
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
      line_number: 640,
    ),
    GT::GrammarRule.new(
      name: "struct_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "struct_member_declaration", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 642,
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
      line_number: 644,
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
      line_number: 678,
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
      line_number: 684,
    ),
    GT::GrammarRule.new(
      name: "interface_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "interface_member_declaration", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 690,
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
      line_number: 692,
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
      line_number: 698,
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
      line_number: 704,
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
      line_number: 707,
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
      line_number: 710,
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
      line_number: 712,
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
      line_number: 720,
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
      line_number: 724,
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
      line_number: 730,
    ),
    GT::GrammarRule.new(
      name: "enum_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "enum_member_declarations", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 739,
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
      line_number: 741,
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
      line_number: 744,
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
      line_number: 759,
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
      line_number: 765,
    ),
    GT::GrammarRule.new(
      name: "type",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "dynamic"),
          GT::Repetition.new(element: GT::RuleReference.new(name: "rank_specifier", is_token: false)),
        ]),
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
      line_number: 797,
    ),
    GT::GrammarRule.new(
      name: "value_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "primitive_type", is_token: false),
        GT::RuleReference.new(name: "type_name", is_token: false),
      ]),
      line_number: 802,
    ),
    GT::GrammarRule.new(
      name: "reference_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "type_name", is_token: false),
        GT::Literal.new(value: "object"),
        GT::Literal.new(value: "string"),
      ]),
      line_number: 805,
    ),
    GT::GrammarRule.new(
      name: "primitive_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "numeric_type", is_token: false),
        GT::Literal.new(value: "bool"),
      ]),
      line_number: 809,
    ),
    GT::GrammarRule.new(
      name: "numeric_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "integral_type", is_token: false),
        GT::RuleReference.new(name: "floating_point_type", is_token: false),
        GT::Literal.new(value: "decimal"),
      ]),
      line_number: 812,
    ),
    GT::GrammarRule.new(
      name: "floating_point_type",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "float"),
        GT::Literal.new(value: "double"),
      ]),
      line_number: 816,
    ),
    GT::GrammarRule.new(
      name: "rank_specifier",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 819,
    ),
    GT::GrammarRule.new(
      name: "pointer_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "STAR", is_token: true),
      ]),
      line_number: 821,
    ),
    GT::GrammarRule.new(
      name: "block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 829,
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
      line_number: 831,
    ),
    GT::GrammarRule.new(
      name: "local_variable_declaration_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "variable_declarators", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 857,
    ),
    GT::GrammarRule.new(
      name: "local_constant_declaration_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "const"),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "constant_declarators", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 859,
    ),
    GT::GrammarRule.new(
      name: "empty_statement",
      body: GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      line_number: 861,
    ),
    GT::GrammarRule.new(
      name: "expression_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 863,
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
      line_number: 865,
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
      line_number: 867,
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
      line_number: 869,
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
      line_number: 871,
    ),
    GT::GrammarRule.new(
      name: "for_initializer",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "local_variable_declaration", is_token: false),
        GT::RuleReference.new(name: "expression_list", is_token: false),
      ]),
      line_number: 874,
    ),
    GT::GrammarRule.new(
      name: "for_iterator",
      body: GT::RuleReference.new(name: "expression_list", is_token: false),
      line_number: 877,
    ),
    GT::GrammarRule.new(
      name: "local_variable_declaration",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "variable_declarators", is_token: false),
      ]),
      line_number: 879,
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
      line_number: 881,
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
      line_number: 883,
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
      line_number: 885,
    ),
    GT::GrammarRule.new(
      name: "switch_block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "switch_section", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 887,
    ),
    GT::GrammarRule.new(
      name: "switch_section",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "switch_label", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
      ]),
      line_number: 889,
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
      line_number: 891,
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
      line_number: 894,
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
      line_number: 897,
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
      line_number: 900,
    ),
    GT::GrammarRule.new(
      name: "general_catch_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "catch"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 902,
    ),
    GT::GrammarRule.new(
      name: "finally_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "finally"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 904,
    ),
    GT::GrammarRule.new(
      name: "throw_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "throw"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 906,
    ),
    GT::GrammarRule.new(
      name: "return_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "return"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 908,
    ),
    GT::GrammarRule.new(
      name: "break_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "break"),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 910,
    ),
    GT::GrammarRule.new(
      name: "continue_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "continue"),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 912,
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
      line_number: 914,
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
      line_number: 918,
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
      line_number: 920,
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
      line_number: 922,
    ),
    GT::GrammarRule.new(
      name: "checked_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "checked"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 925,
    ),
    GT::GrammarRule.new(
      name: "unchecked_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "unchecked"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 927,
    ),
    GT::GrammarRule.new(
      name: "labelled_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 929,
    ),
    GT::GrammarRule.new(
      name: "unsafe_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "unsafe"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 931,
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
      line_number: 933,
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
      line_number: 935,
    ),
    GT::GrammarRule.new(
      name: "expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "assignment_expression", is_token: false),
        GT::RuleReference.new(name: "lambda_expression", is_token: false),
        GT::RuleReference.new(name: "query_expression", is_token: false),
      ]),
      line_number: 954,
    ),
    GT::GrammarRule.new(
      name: "lambda_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "lambda_parameters", is_token: false),
        GT::RuleReference.new(name: "LAMBDA", is_token: true),
        GT::RuleReference.new(name: "lambda_body", is_token: false),
      ]),
      line_number: 962,
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
      line_number: 964,
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
      line_number: 967,
    ),
    GT::GrammarRule.new(
      name: "lambda_parameter",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type", is_token: false)),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 969,
    ),
    GT::GrammarRule.new(
      name: "lambda_body",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 971,
    ),
    GT::GrammarRule.new(
      name: "query_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "from_clause", is_token: false),
        GT::RuleReference.new(name: "query_body", is_token: false),
      ]),
      line_number: 978,
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
      line_number: 980,
    ),
    GT::GrammarRule.new(
      name: "query_body",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "query_body_clause", is_token: false)),
        GT::RuleReference.new(name: "select_or_group_clause", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "query_continuation", is_token: false)),
      ]),
      line_number: 982,
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
      line_number: 984,
    ),
    GT::GrammarRule.new(
      name: "let_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "let"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 990,
    ),
    GT::GrammarRule.new(
      name: "where_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "where"),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 992,
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
      line_number: 994,
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
      line_number: 998,
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
      line_number: 1000,
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
      line_number: 1002,
    ),
    GT::GrammarRule.new(
      name: "query_continuation",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "into"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "query_body", is_token: false),
      ]),
      line_number: 1005,
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
      line_number: 1011,
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
      line_number: 1014,
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
      line_number: 1026,
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
      line_number: 1029,
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
      line_number: 1031,
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
      line_number: 1033,
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
      line_number: 1035,
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
      line_number: 1037,
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
      line_number: 1039,
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
      line_number: 1041,
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
      line_number: 1044,
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
      line_number: 1050,
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
      line_number: 1053,
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
      line_number: 1056,
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
      line_number: 1065,
    ),
    GT::GrammarRule.new(
      name: "cast_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "unary_expression", is_token: false),
      ]),
      line_number: 1076,
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
      line_number: 1078,
    ),
    GT::GrammarRule.new(
      name: "primary_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "primary", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "primary_suffix", is_token: false)),
      ]),
      line_number: 1088,
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
      line_number: 1090,
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
      line_number: 1095,
    ),
    GT::GrammarRule.new(
      name: "typeof_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "typeof"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type_or_void", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1109,
    ),
    GT::GrammarRule.new(
      name: "type_or_void",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::Literal.new(value: "void"),
      ]),
      line_number: 1111,
    ),
    GT::GrammarRule.new(
      name: "sizeof_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "sizeof"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1113,
    ),
    GT::GrammarRule.new(
      name: "checked_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "checked"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1115,
    ),
    GT::GrammarRule.new(
      name: "unchecked_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "unchecked"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1117,
    ),
    GT::GrammarRule.new(
      name: "default_value_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "default"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1119,
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
      line_number: 1121,
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
      line_number: 1127,
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
      line_number: 1132,
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
      line_number: 1134,
    ),
    GT::GrammarRule.new(
      name: "new_implicitly_typed_array",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
        GT::RuleReference.new(name: "array_initializer", is_token: false),
      ]),
      line_number: 1136,
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
      line_number: 1138,
    ),
    GT::GrammarRule.new(
      name: "object_or_collection_initializer",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "initializer_list", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 1141,
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
      line_number: 1143,
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
      line_number: 1145,
    ),
    GT::GrammarRule.new(
      name: "new_array_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "array_type", is_token: false),
        GT::RuleReference.new(name: "array_creation_suffix", is_token: false),
      ]),
      line_number: 1149,
    ),
    GT::GrammarRule.new(
      name: "array_type",
      body: GT::Group.new(element: GT::Alternation.new(choices: [
          GT::RuleReference.new(name: "primitive_type", is_token: false),
          GT::RuleReference.new(name: "type_name", is_token: false),
        ])),
      line_number: 1151,
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
      line_number: 1153,
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
      line_number: 1191,
    ),
    GT::GrammarRule.new(
      name: "argument",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "COLON", is_token: true),
          ])),
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "ref"),
            GT::Literal.new(value: "out"),
          ])),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 1193,
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
      line_number: 1199,
    ),
  ],
)
