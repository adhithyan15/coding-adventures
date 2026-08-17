# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: csharp1.0.grammar
# Regenerate with: grammar-tools compile-grammar csharp1.0.grammar
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
      line_number: 96,
    ),
    GT::GrammarRule.new(
      name: "extern_alias_directive",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "extern"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 117,
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
      line_number: 143,
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
      line_number: 160,
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
      line_number: 179,
    ),
    GT::GrammarRule.new(
      name: "global_attribute_target",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "assembly"),
        GT::Literal.new(value: "module"),
      ]),
      line_number: 181,
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
      line_number: 212,
    ),
    GT::GrammarRule.new(
      name: "namespace_member_declaration",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "namespace_declaration", is_token: false),
        GT::RuleReference.new(name: "type_declaration", is_token: false),
      ]),
      line_number: 226,
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
      line_number: 229,
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
      line_number: 259,
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
      line_number: 261,
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
      line_number: 269,
    ),
    GT::GrammarRule.new(
      name: "attribute",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "qualified_name", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "attribute_arguments", is_token: false)),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
      ]),
      line_number: 271,
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
      line_number: 273,
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
      line_number: 277,
    ),
    GT::GrammarRule.new(
      name: "class_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "class_modifier", is_token: false)),
        GT::Literal.new(value: "class"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "class_base_list", is_token: false),
          ])),
        GT::RuleReference.new(name: "class_body", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "SEMICOLON", is_token: true)),
      ]),
      line_number: 302,
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
      ]),
      line_number: 306,
    ),
    GT::GrammarRule.new(
      name: "class_base_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "qualified_name", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "qualified_name", is_token: false),
          ])),
      ]),
      line_number: 324,
    ),
    GT::GrammarRule.new(
      name: "class_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "class_member_declaration", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
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
      line_number: 348,
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
      line_number: 379,
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
      line_number: 382,
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
      line_number: 388,
    ),
    GT::GrammarRule.new(
      name: "constant_declarator",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 390,
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
      line_number: 413,
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
      line_number: 416,
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
      line_number: 432,
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
      line_number: 434,
    ),
    GT::GrammarRule.new(
      name: "variable_initializer",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "array_initializer", is_token: false),
      ]),
      line_number: 436,
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
      line_number: 448,
    ),
    GT::GrammarRule.new(
      name: "method_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "method_modifier", is_token: false)),
        GT::RuleReference.new(name: "return_type", is_token: false),
        GT::RuleReference.new(name: "qualified_name", is_token: false),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "method_body", is_token: false),
      ]),
      line_number: 479,
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
      line_number: 483,
    ),
    GT::GrammarRule.new(
      name: "return_type",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "void"),
        GT::RuleReference.new(name: "type", is_token: false),
      ]),
      line_number: 495,
    ),
    GT::GrammarRule.new(
      name: "method_body",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "block", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 501,
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
      line_number: 538,
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
      line_number: 541,
    ),
    GT::GrammarRule.new(
      name: "fixed_parameter",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "parameter_modifier", is_token: false)),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 543,
    ),
    GT::GrammarRule.new(
      name: "parameter_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "ref"),
        GT::Literal.new(value: "out"),
      ]),
      line_number: 545,
    ),
    GT::GrammarRule.new(
      name: "parameter_array",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Literal.new(value: "params"),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 551,
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
      line_number: 584,
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
      line_number: 587,
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
      line_number: 603,
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
      line_number: 606,
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
      line_number: 609,
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
      line_number: 615,
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
      line_number: 651,
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
      line_number: 655,
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
      line_number: 667,
    ),
    GT::GrammarRule.new(
      name: "add_accessor_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Literal.new(value: "add"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 670,
    ),
    GT::GrammarRule.new(
      name: "remove_accessor_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Literal.new(value: "remove"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 672,
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
      line_number: 703,
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
      line_number: 707,
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
      line_number: 765,
    ),
    GT::GrammarRule.new(
      name: "operator_modifiers",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "static"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "extern")),
      ]),
      line_number: 770,
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
      line_number: 776,
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
      line_number: 826,
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
      line_number: 861,
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
      line_number: 865,
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
      line_number: 874,
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
      line_number: 907,
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
      line_number: 937,
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
      line_number: 940,
    ),
    GT::GrammarRule.new(
      name: "struct_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "struct_modifier", is_token: false)),
        GT::Literal.new(value: "struct"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "interface_type_list", is_token: false),
          ])),
        GT::RuleReference.new(name: "struct_body", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "SEMICOLON", is_token: true)),
      ]),
      line_number: 985,
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
      line_number: 989,
    ),
    GT::GrammarRule.new(
      name: "interface_type_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "qualified_name", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "qualified_name", is_token: false),
          ])),
      ]),
      line_number: 997,
    ),
    GT::GrammarRule.new(
      name: "struct_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "struct_member_declaration", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 999,
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
      line_number: 1004,
    ),
    GT::GrammarRule.new(
      name: "interface_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "interface_modifier", is_token: false)),
        GT::Literal.new(value: "interface"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "interface_type_list", is_token: false),
          ])),
        GT::RuleReference.new(name: "interface_body", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "SEMICOLON", is_token: true)),
      ]),
      line_number: 1055,
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
      line_number: 1059,
    ),
    GT::GrammarRule.new(
      name: "interface_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "interface_member_declaration", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 1065,
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
      line_number: 1067,
    ),
    GT::GrammarRule.new(
      name: "interface_method_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "new")),
        GT::RuleReference.new(name: "return_type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1076,
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
      line_number: 1082,
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
      line_number: 1085,
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
      line_number: 1090,
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
      line_number: 1094,
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
      line_number: 1129,
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
      line_number: 1133,
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
      line_number: 1139,
    ),
    GT::GrammarRule.new(
      name: "enum_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "enum_member_declarations", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 1148,
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
      line_number: 1150,
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
      line_number: 1153,
    ),
    GT::GrammarRule.new(
      name: "delegate_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "delegate_modifier", is_token: false)),
        GT::Literal.new(value: "delegate"),
        GT::RuleReference.new(name: "return_type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1184,
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
      line_number: 1188,
    ),
    GT::GrammarRule.new(
      name: "type",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "value_type", is_token: false),
          GT::Repetition.new(element: GT::RuleReference.new(name: "rank_specifier", is_token: false)),
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
      line_number: 1233,
    ),
    GT::GrammarRule.new(
      name: "value_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "primitive_type", is_token: false),
        GT::RuleReference.new(name: "qualified_name", is_token: false),
      ]),
      line_number: 1237,
    ),
    GT::GrammarRule.new(
      name: "reference_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "qualified_name", is_token: false),
        GT::Literal.new(value: "object"),
        GT::Literal.new(value: "string"),
      ]),
      line_number: 1240,
    ),
    GT::GrammarRule.new(
      name: "primitive_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "numeric_type", is_token: false),
        GT::Literal.new(value: "bool"),
      ]),
      line_number: 1244,
    ),
    GT::GrammarRule.new(
      name: "numeric_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "integral_type", is_token: false),
        GT::RuleReference.new(name: "floating_point_type", is_token: false),
        GT::Literal.new(value: "decimal"),
      ]),
      line_number: 1247,
    ),
    GT::GrammarRule.new(
      name: "floating_point_type",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "float"),
        GT::Literal.new(value: "double"),
      ]),
      line_number: 1251,
    ),
    GT::GrammarRule.new(
      name: "rank_specifier",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 1267,
    ),
    GT::GrammarRule.new(
      name: "pointer_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "STAR", is_token: true),
      ]),
      line_number: 1278,
    ),
    GT::GrammarRule.new(
      name: "block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 1305,
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
      ]),
      line_number: 1307,
    ),
    GT::GrammarRule.new(
      name: "local_variable_declaration_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "variable_declarators", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1343,
    ),
    GT::GrammarRule.new(
      name: "local_constant_declaration_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "const"),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "constant_declarators", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1353,
    ),
    GT::GrammarRule.new(
      name: "empty_statement",
      body: GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      line_number: 1360,
    ),
    GT::GrammarRule.new(
      name: "expression_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1372,
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
      line_number: 1389,
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
      line_number: 1399,
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
      line_number: 1410,
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
      line_number: 1425,
    ),
    GT::GrammarRule.new(
      name: "for_initializer",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "local_variable_declaration", is_token: false),
        GT::RuleReference.new(name: "expression_list", is_token: false),
      ]),
      line_number: 1428,
    ),
    GT::GrammarRule.new(
      name: "for_iterator",
      body: GT::RuleReference.new(name: "expression_list", is_token: false),
      line_number: 1431,
    ),
    GT::GrammarRule.new(
      name: "local_variable_declaration",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "variable_declarators", is_token: false),
      ]),
      line_number: 1433,
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
      line_number: 1435,
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
      line_number: 1463,
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
      line_number: 1494,
    ),
    GT::GrammarRule.new(
      name: "switch_block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "switch_section", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 1496,
    ),
    GT::GrammarRule.new(
      name: "switch_section",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "switch_label", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
      ]),
      line_number: 1498,
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
      line_number: 1500,
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
      line_number: 1523,
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
      line_number: 1526,
    ),
    GT::GrammarRule.new(
      name: "specific_catch_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "catch"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "qualified_name", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 1529,
    ),
    GT::GrammarRule.new(
      name: "general_catch_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "catch"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 1531,
    ),
    GT::GrammarRule.new(
      name: "finally_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "finally"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 1533,
    ),
    GT::GrammarRule.new(
      name: "throw_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "throw"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1550,
    ),
    GT::GrammarRule.new(
      name: "return_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "return"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1554,
    ),
    GT::GrammarRule.new(
      name: "break_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "break"),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1565,
    ),
    GT::GrammarRule.new(
      name: "continue_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "continue"),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1567,
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
      line_number: 1599,
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
      line_number: 1620,
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
      line_number: 1643,
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
      line_number: 1645,
    ),
    GT::GrammarRule.new(
      name: "checked_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "checked"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 1674,
    ),
    GT::GrammarRule.new(
      name: "unchecked_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "unchecked"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 1676,
    ),
    GT::GrammarRule.new(
      name: "labelled_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 1688,
    ),
    GT::GrammarRule.new(
      name: "unsafe_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "unsafe"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 1714,
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
      line_number: 1739,
    ),
    GT::GrammarRule.new(
      name: "expression",
      body: GT::RuleReference.new(name: "assignment_expression", is_token: false),
      line_number: 1786,
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
      line_number: 1788,
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
      line_number: 1791,
    ),
    GT::GrammarRule.new(
      name: "conditional_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "logical_or_expression", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "QUESTION", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
      ]),
      line_number: 1809,
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
      line_number: 1818,
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
      line_number: 1826,
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
      line_number: 1834,
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
      line_number: 1842,
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
      line_number: 1850,
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
      line_number: 1862,
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
      line_number: 1885,
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
      line_number: 1904,
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
      line_number: 1919,
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
      line_number: 1932,
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
      line_number: 1950,
    ),
    GT::GrammarRule.new(
      name: "cast_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "unary_expression", is_token: false),
      ]),
      line_number: 1971,
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
      line_number: 1980,
    ),
    GT::GrammarRule.new(
      name: "primary_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "primary", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "primary_suffix", is_token: false)),
      ]),
      line_number: 2000,
    ),
    GT::GrammarRule.new(
      name: "primary_suffix",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOT", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
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
      line_number: 2005,
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
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 2012,
    ),
    GT::GrammarRule.new(
      name: "typeof_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "typeof"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type_or_void", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 2035,
    ),
    GT::GrammarRule.new(
      name: "type_or_void",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::Literal.new(value: "void"),
      ]),
      line_number: 2037,
    ),
    GT::GrammarRule.new(
      name: "sizeof_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "sizeof"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 2047,
    ),
    GT::GrammarRule.new(
      name: "checked_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "checked"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 2056,
    ),
    GT::GrammarRule.new(
      name: "unchecked_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "unchecked"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 2058,
    ),
    GT::GrammarRule.new(
      name: "default_value_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "default"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 2072,
    ),
    GT::GrammarRule.new(
      name: "new_expression",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "new"),
          GT::RuleReference.new(name: "new_object_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "new"),
          GT::RuleReference.new(name: "new_array_expression", is_token: false),
        ]),
      ]),
      line_number: 2094,
    ),
    GT::GrammarRule.new(
      name: "new_object_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "qualified_name", is_token: false),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "argument_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 2099,
    ),
    GT::GrammarRule.new(
      name: "new_array_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "array_type", is_token: false),
        GT::RuleReference.new(name: "array_creation_suffix", is_token: false),
      ]),
      line_number: 2108,
    ),
    GT::GrammarRule.new(
      name: "array_type",
      body: GT::Group.new(element: GT::Alternation.new(choices: [
          GT::RuleReference.new(name: "primitive_type", is_token: false),
          GT::RuleReference.new(name: "qualified_name", is_token: false),
        ])),
      line_number: 2110,
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
      line_number: 2112,
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
      line_number: 2123,
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
      line_number: 2125,
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
      line_number: 2142,
    ),
  ],
)
