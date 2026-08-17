# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: csharp10.0.grammar
# Regenerate with: grammar-tools compile-grammar csharp10.0.grammar
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
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "file_scoped_namespace_declaration", is_token: false),
            GT::Sequence.new(elements: [
              GT::Repetition.new(element: GT::RuleReference.new(name: "top_level_statement", is_token: false)),
              GT::Repetition.new(element: GT::RuleReference.new(name: "namespace_member_declaration", is_token: false)),
            ]),
          ])),
      ]),
      line_number: 173,
    ),
    GT::GrammarRule.new(
      name: "extern_alias_directive",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "extern"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 185,
    ),
    GT::GrammarRule.new(
      name: "using_directive",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::OptionalElement.new(element: GT::Literal.new(value: "global")),
          GT::Literal.new(value: "using"),
          GT::Literal.new(value: "static"),
          GT::RuleReference.new(name: "qualified_name", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::OptionalElement.new(element: GT::Literal.new(value: "global")),
          GT::Literal.new(value: "using"),
          GT::RuleReference.new(name: "qualified_name", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::OptionalElement.new(element: GT::Literal.new(value: "global")),
          GT::Literal.new(value: "using"),
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "EQUALS", is_token: true),
          GT::RuleReference.new(name: "qualified_name", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
      ]),
      line_number: 208,
    ),
    GT::GrammarRule.new(
      name: "qualified_name",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "name_part", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "DOT", is_token: true),
            GT::RuleReference.new(name: "name_part", is_token: false),
          ])),
      ]),
      line_number: 220,
    ),
    GT::GrammarRule.new(
      name: "name_part",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON_COLON", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_argument_list", is_token: false)),
      ]),
      line_number: 222,
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
      line_number: 224,
    ),
    GT::GrammarRule.new(
      name: "type_argument",
      body: GT::RuleReference.new(name: "type", is_token: false),
      line_number: 226,
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
      line_number: 234,
    ),
    GT::GrammarRule.new(
      name: "global_attribute_target",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "assembly"),
        GT::Literal.new(value: "module"),
      ]),
      line_number: 236,
    ),
    GT::GrammarRule.new(
      name: "file_scoped_namespace_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "namespace"),
        GT::RuleReference.new(name: "qualified_name", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "type_declaration", is_token: false)),
      ]),
      line_number: 266,
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
      line_number: 276,
    ),
    GT::GrammarRule.new(
      name: "namespace_member_declaration",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "namespace_declaration", is_token: false),
        GT::RuleReference.new(name: "type_declaration", is_token: false),
      ]),
      line_number: 288,
    ),
    GT::GrammarRule.new(
      name: "type_declaration",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "class_declaration", is_token: false),
        GT::RuleReference.new(name: "struct_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_declaration", is_token: false),
        GT::RuleReference.new(name: "enum_declaration", is_token: false),
        GT::RuleReference.new(name: "delegate_declaration", is_token: false),
        GT::RuleReference.new(name: "record_declaration", is_token: false),
        GT::RuleReference.new(name: "record_struct_declaration", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 291,
    ),
    GT::GrammarRule.new(
      name: "top_level_statement",
      body: GT::RuleReference.new(name: "statement", is_token: false),
      line_number: 312,
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
      line_number: 325,
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
      line_number: 327,
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
      line_number: 335,
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
      line_number: 337,
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
      line_number: 339,
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
      line_number: 341,
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
        GT::Repetition.new(element: GT::RuleReference.new(name: "type_parameter_constraint_clause", is_token: false)),
        GT::RuleReference.new(name: "class_body", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "SEMICOLON", is_token: true)),
      ]),
      line_number: 350,
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
      line_number: 356,
    ),
    GT::GrammarRule.new(
      name: "class_base_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "type", is_token: false),
          ])),
      ]),
      line_number: 365,
    ),
    GT::GrammarRule.new(
      name: "class_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "class_member_declaration", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 367,
    ),
    GT::GrammarRule.new(
      name: "record_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "record_modifier", is_token: false)),
        GT::Literal.new(value: "record"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "class")),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameter_list", is_token: false)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::RuleReference.new(name: "record_parameter_list", is_token: false),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "record_base_list", is_token: false),
          ])),
        GT::Repetition.new(element: GT::RuleReference.new(name: "type_parameter_constraint_clause", is_token: false)),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "record_body", is_token: false),
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          ])),
      ]),
      line_number: 376,
    ),
    GT::GrammarRule.new(
      name: "record_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "internal"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "new"),
        GT::Literal.new(value: "abstract"),
        GT::Literal.new(value: "sealed"),
      ]),
      line_number: 384,
    ),
    GT::GrammarRule.new(
      name: "record_parameter_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "record_parameter", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "record_parameter", is_token: false),
          ])),
      ]),
      line_number: 392,
    ),
    GT::GrammarRule.new(
      name: "record_parameter",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "ref"),
            GT::Literal.new(value: "out"),
            GT::Literal.new(value: "in"),
          ])),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
      ]),
      line_number: 394,
    ),
    GT::GrammarRule.new(
      name: "record_base_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "record_base_type", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "type", is_token: false),
          ])),
      ]),
      line_number: 397,
    ),
    GT::GrammarRule.new(
      name: "record_base_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "argument_list", is_token: false)),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
      ]),
      line_number: 399,
    ),
    GT::GrammarRule.new(
      name: "record_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "class_member_declaration", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 401,
    ),
    GT::GrammarRule.new(
      name: "record_struct_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "record_struct_modifier", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "readonly")),
        GT::Literal.new(value: "record"),
        GT::Literal.new(value: "struct"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameter_list", is_token: false)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::RuleReference.new(name: "record_parameter_list", is_token: false),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "interface_type_list", is_token: false),
          ])),
        GT::Repetition.new(element: GT::RuleReference.new(name: "type_parameter_constraint_clause", is_token: false)),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "record_struct_body", is_token: false),
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          ])),
      ]),
      line_number: 442,
    ),
    GT::GrammarRule.new(
      name: "record_struct_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "internal"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "new"),
      ]),
      line_number: 450,
    ),
    GT::GrammarRule.new(
      name: "record_struct_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "struct_member_declaration", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 456,
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
      line_number: 464,
    ),
    GT::GrammarRule.new(
      name: "type_parameter",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "in"),
            GT::Literal.new(value: "out"),
          ])),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 466,
    ),
    GT::GrammarRule.new(
      name: "type_parameter_constraint_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "where"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "type_parameter_constraints", is_token: false),
      ]),
      line_number: 468,
    ),
    GT::GrammarRule.new(
      name: "type_parameter_constraints",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type_parameter_constraint", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "type_parameter_constraint", is_token: false),
          ])),
      ]),
      line_number: 470,
    ),
    GT::GrammarRule.new(
      name: "type_parameter_constraint",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "class"),
        GT::Literal.new(value: "struct"),
        GT::Literal.new(value: "unmanaged"),
        GT::Literal.new(value: "notnull"),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "new"),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::RuleReference.new(name: "type", is_token: false),
      ]),
      line_number: 473,
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
      line_number: 486,
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
      line_number: 508,
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
      line_number: 511,
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
      line_number: 517,
    ),
    GT::GrammarRule.new(
      name: "constant_declarator",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 519,
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
      line_number: 525,
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
        GT::Literal.new(value: "ref"),
      ]),
      line_number: 528,
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
      line_number: 538,
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
      line_number: 540,
    ),
    GT::GrammarRule.new(
      name: "variable_initializer",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "array_initializer", is_token: false),
      ]),
      line_number: 542,
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
      line_number: 545,
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
        GT::Repetition.new(element: GT::RuleReference.new(name: "type_parameter_constraint_clause", is_token: false)),
        GT::RuleReference.new(name: "method_body", is_token: false),
      ]),
      line_number: 551,
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
        GT::Literal.new(value: "async"),
      ]),
      line_number: 557,
    ),
    GT::GrammarRule.new(
      name: "return_type",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "void"),
        GT::Sequence.new(elements: [
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::Literal.new(value: "ref"),
              GT::OptionalElement.new(element: GT::Literal.new(value: "readonly")),
            ])),
          GT::RuleReference.new(name: "type", is_token: false),
        ]),
      ]),
      line_number: 570,
    ),
    GT::GrammarRule.new(
      name: "method_body",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "block", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LAMBDA_ARROW", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 573,
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
      line_number: 581,
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
      line_number: 584,
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
      line_number: 586,
    ),
    GT::GrammarRule.new(
      name: "parameter_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "ref"),
        GT::Literal.new(value: "out"),
        GT::Literal.new(value: "in"),
        GT::Literal.new(value: "this"),
      ]),
      line_number: 589,
    ),
    GT::GrammarRule.new(
      name: "parameter_array",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Literal.new(value: "params"),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 594,
    ),
    GT::GrammarRule.new(
      name: "property_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "property_modifier", is_token: false)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "ref"),
            GT::OptionalElement.new(element: GT::Literal.new(value: "readonly")),
          ])),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "qualified_name", is_token: false),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LBRACE", is_token: true),
              GT::RuleReference.new(name: "accessor_declarations", is_token: false),
              GT::RuleReference.new(name: "RBRACE", is_token: true),
              GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                  GT::RuleReference.new(name: "EQUALS", is_token: true),
                  GT::RuleReference.new(name: "expression", is_token: false),
                  GT::RuleReference.new(name: "SEMICOLON", is_token: true),
                ])),
            ]),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LAMBDA_ARROW", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            ]),
          ])),
      ]),
      line_number: 602,
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
      line_number: 607,
    ),
    GT::GrammarRule.new(
      name: "accessor_declarations",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "get_accessor_declaration", is_token: false),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "set_or_init_accessor_declaration", is_token: false)),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "set_or_init_accessor_declaration", is_token: false),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "get_accessor_declaration", is_token: false)),
        ]),
      ]),
      line_number: 619,
    ),
    GT::GrammarRule.new(
      name: "get_accessor_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "accessor_modifier", is_token: false)),
        GT::Literal.new(value: "get"),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "block", is_token: false),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LAMBDA_ARROW", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            ]),
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          ])),
      ]),
      line_number: 622,
    ),
    GT::GrammarRule.new(
      name: "set_or_init_accessor_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "accessor_modifier", is_token: false)),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "set"),
            GT::Literal.new(value: "init"),
          ])),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "block", is_token: false),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LAMBDA_ARROW", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            ]),
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          ])),
      ]),
      line_number: 625,
    ),
    GT::GrammarRule.new(
      name: "accessor_modifier",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "protected"),
          GT::Literal.new(value: "internal"),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "internal"),
          GT::Literal.new(value: "protected"),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "private"),
          GT::Literal.new(value: "protected"),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "protected"),
          GT::Literal.new(value: "private"),
        ]),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "internal"),
        GT::Literal.new(value: "private"),
      ]),
      line_number: 629,
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
      line_number: 643,
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
      line_number: 647,
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
      line_number: 659,
    ),
    GT::GrammarRule.new(
      name: "add_accessor_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Literal.new(value: "add"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 662,
    ),
    GT::GrammarRule.new(
      name: "remove_accessor_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Literal.new(value: "remove"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 664,
    ),
    GT::GrammarRule.new(
      name: "indexer_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "indexer_modifier", is_token: false)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "ref"),
            GT::OptionalElement.new(element: GT::Literal.new(value: "readonly")),
          ])),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::Literal.new(value: "this"),
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "formal_parameter_list", is_token: false),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LBRACE", is_token: true),
              GT::RuleReference.new(name: "accessor_declarations", is_token: false),
              GT::RuleReference.new(name: "RBRACE", is_token: true),
            ]),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LAMBDA_ARROW", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            ]),
          ])),
      ]),
      line_number: 672,
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
      line_number: 677,
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
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "block", is_token: false),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LAMBDA_ARROW", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            ]),
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          ])),
      ]),
      line_number: 694,
    ),
    GT::GrammarRule.new(
      name: "operator_modifiers",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "static"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "extern")),
      ]),
      line_number: 699,
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
      line_number: 701,
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
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "block", is_token: false),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LAMBDA_ARROW", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            ]),
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          ])),
      ]),
      line_number: 728,
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
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "block", is_token: false),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LAMBDA_ARROW", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            ]),
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          ])),
      ]),
      line_number: 737,
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
      line_number: 742,
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
      line_number: 748,
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
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "block", is_token: false),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LAMBDA_ARROW", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            ]),
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          ])),
      ]),
      line_number: 755,
    ),
    GT::GrammarRule.new(
      name: "static_constructor_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::RuleReference.new(name: "static_constructor_modifiers", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "block", is_token: false),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LAMBDA_ARROW", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            ]),
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          ])),
      ]),
      line_number: 763,
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
      line_number: 767,
    ),
    GT::GrammarRule.new(
      name: "struct_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "struct_modifier", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "partial")),
        GT::OptionalElement.new(element: GT::Literal.new(value: "ref")),
        GT::Literal.new(value: "struct"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameter_list", is_token: false)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "interface_type_list", is_token: false),
          ])),
        GT::Repetition.new(element: GT::RuleReference.new(name: "type_parameter_constraint_clause", is_token: false)),
        GT::RuleReference.new(name: "struct_body", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "SEMICOLON", is_token: true)),
      ]),
      line_number: 776,
    ),
    GT::GrammarRule.new(
      name: "struct_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "internal"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "new"),
        GT::Literal.new(value: "readonly"),
      ]),
      line_number: 783,
    ),
    GT::GrammarRule.new(
      name: "interface_type_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "type", is_token: false),
          ])),
      ]),
      line_number: 790,
    ),
    GT::GrammarRule.new(
      name: "struct_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "struct_member_declaration", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 792,
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
      line_number: 794,
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
        GT::Repetition.new(element: GT::RuleReference.new(name: "type_parameter_constraint_clause", is_token: false)),
        GT::RuleReference.new(name: "interface_body", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "SEMICOLON", is_token: true)),
      ]),
      line_number: 813,
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
      line_number: 820,
    ),
    GT::GrammarRule.new(
      name: "interface_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "interface_member_declaration", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 826,
    ),
    GT::GrammarRule.new(
      name: "interface_member_declaration",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "interface_method_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_property_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_event_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_indexer_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_constant_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_operator_declaration", is_token: false),
        GT::RuleReference.new(name: "type_declaration", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 828,
    ),
    GT::GrammarRule.new(
      name: "interface_method_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "interface_method_modifier", is_token: false)),
        GT::RuleReference.new(name: "return_type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "type_parameter_constraint_clause", is_token: false)),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "block", is_token: false),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LAMBDA_ARROW", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            ]),
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          ])),
      ]),
      line_number: 837,
    ),
    GT::GrammarRule.new(
      name: "interface_method_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "new"),
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "protected"),
        GT::Literal.new(value: "internal"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "static"),
        GT::Literal.new(value: "virtual"),
        GT::Literal.new(value: "abstract"),
        GT::Literal.new(value: "sealed"),
        GT::Literal.new(value: "override"),
        GT::Literal.new(value: "extern"),
        GT::Literal.new(value: "async"),
      ]),
      line_number: 843,
    ),
    GT::GrammarRule.new(
      name: "interface_property_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "interface_method_modifier", is_token: false)),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "accessor_declarations", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          ])),
      ]),
      line_number: 856,
    ),
    GT::GrammarRule.new(
      name: "interface_event_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "interface_method_modifier", is_token: false)),
        GT::Literal.new(value: "event"),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LBRACE", is_token: true),
              GT::RuleReference.new(name: "event_accessor_declarations", is_token: false),
              GT::RuleReference.new(name: "RBRACE", is_token: true),
            ]),
          ])),
      ]),
      line_number: 861,
    ),
    GT::GrammarRule.new(
      name: "interface_indexer_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "interface_method_modifier", is_token: false)),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::Literal.new(value: "this"),
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "formal_parameter_list", is_token: false),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "accessor_declarations", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 866,
    ),
    GT::GrammarRule.new(
      name: "interface_constant_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "interface_method_modifier", is_token: false)),
        GT::Literal.new(value: "const"),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 870,
    ),
    GT::GrammarRule.new(
      name: "interface_operator_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "static"),
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
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "block", is_token: false),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LAMBDA_ARROW", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            ]),
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          ])),
      ]),
      line_number: 873,
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
      line_number: 884,
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
      line_number: 888,
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
      line_number: 894,
    ),
    GT::GrammarRule.new(
      name: "enum_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "enum_member_declarations", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 903,
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
      line_number: 905,
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
      line_number: 908,
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
        GT::Repetition.new(element: GT::RuleReference.new(name: "type_parameter_constraint_clause", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 914,
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
      line_number: 920,
    ),
    GT::GrammarRule.new(
      name: "type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "nullable_type", is_token: false),
        GT::RuleReference.new(name: "non_nullable_type", is_token: false),
      ]),
      line_number: 932,
    ),
    GT::GrammarRule.new(
      name: "non_nullable_type",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "tuple_type", is_token: false),
          GT::Repetition.new(element: GT::RuleReference.new(name: "rank_specifier", is_token: false)),
        ]),
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
      line_number: 935,
    ),
    GT::GrammarRule.new(
      name: "nullable_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "non_nullable_type", is_token: false),
        GT::RuleReference.new(name: "QUESTION", is_token: true),
      ]),
      line_number: 940,
    ),
    GT::GrammarRule.new(
      name: "value_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "primitive_type", is_token: false),
        GT::RuleReference.new(name: "qualified_name", is_token: false),
      ]),
      line_number: 942,
    ),
    GT::GrammarRule.new(
      name: "reference_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "qualified_name", is_token: false),
        GT::Literal.new(value: "object"),
        GT::Literal.new(value: "string"),
        GT::Literal.new(value: "dynamic"),
      ]),
      line_number: 945,
    ),
    GT::GrammarRule.new(
      name: "primitive_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "numeric_type", is_token: false),
        GT::Literal.new(value: "bool"),
      ]),
      line_number: 950,
    ),
    GT::GrammarRule.new(
      name: "numeric_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "integral_type", is_token: false),
        GT::RuleReference.new(name: "floating_point_type", is_token: false),
        GT::Literal.new(value: "decimal"),
      ]),
      line_number: 953,
    ),
    GT::GrammarRule.new(
      name: "floating_point_type",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "float"),
        GT::Literal.new(value: "double"),
      ]),
      line_number: 957,
    ),
    GT::GrammarRule.new(
      name: "rank_specifier",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 960,
    ),
    GT::GrammarRule.new(
      name: "pointer_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "STAR", is_token: true),
      ]),
      line_number: 962,
    ),
    GT::GrammarRule.new(
      name: "tuple_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "tuple_element", is_token: false),
        GT::RuleReference.new(name: "COMMA", is_token: true),
        GT::RuleReference.new(name: "tuple_element", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "tuple_element", is_token: false),
          ])),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 964,
    ),
    GT::GrammarRule.new(
      name: "tuple_element",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
      ]),
      line_number: 966,
    ),
    GT::GrammarRule.new(
      name: "block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 974,
    ),
    GT::GrammarRule.new(
      name: "statement",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "block", is_token: false),
        GT::RuleReference.new(name: "local_variable_declaration_statement", is_token: false),
        GT::RuleReference.new(name: "local_constant_declaration_statement", is_token: false),
        GT::RuleReference.new(name: "using_declaration_statement", is_token: false),
        GT::RuleReference.new(name: "empty_statement", is_token: false),
        GT::RuleReference.new(name: "expression_statement", is_token: false),
        GT::RuleReference.new(name: "if_statement", is_token: false),
        GT::RuleReference.new(name: "while_statement", is_token: false),
        GT::RuleReference.new(name: "do_while_statement", is_token: false),
        GT::RuleReference.new(name: "for_statement", is_token: false),
        GT::RuleReference.new(name: "foreach_statement", is_token: false),
        GT::RuleReference.new(name: "await_foreach_statement", is_token: false),
        GT::RuleReference.new(name: "switch_statement", is_token: false),
        GT::RuleReference.new(name: "try_statement", is_token: false),
        GT::RuleReference.new(name: "throw_statement", is_token: false),
        GT::RuleReference.new(name: "return_statement", is_token: false),
        GT::RuleReference.new(name: "break_statement", is_token: false),
        GT::RuleReference.new(name: "continue_statement", is_token: false),
        GT::RuleReference.new(name: "goto_statement", is_token: false),
        GT::RuleReference.new(name: "lock_statement", is_token: false),
        GT::RuleReference.new(name: "using_statement", is_token: false),
        GT::RuleReference.new(name: "await_using_statement", is_token: false),
        GT::RuleReference.new(name: "checked_statement", is_token: false),
        GT::RuleReference.new(name: "unchecked_statement", is_token: false),
        GT::RuleReference.new(name: "labelled_statement", is_token: false),
        GT::RuleReference.new(name: "unsafe_statement", is_token: false),
        GT::RuleReference.new(name: "fixed_statement", is_token: false),
        GT::RuleReference.new(name: "yield_statement", is_token: false),
        GT::RuleReference.new(name: "local_function_declaration", is_token: false),
      ]),
      line_number: 976,
    ),
    GT::GrammarRule.new(
      name: "local_variable_declaration_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "local_variable_declaration", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1006,
    ),
    GT::GrammarRule.new(
      name: "local_variable_declaration",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::Literal.new(value: "ref"),
              GT::OptionalElement.new(element: GT::Literal.new(value: "readonly")),
            ])),
          GT::RuleReference.new(name: "type", is_token: false),
          GT::RuleReference.new(name: "variable_declarators", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "var"),
          GT::RuleReference.new(name: "variable_declarators", is_token: false),
        ]),
        GT::RuleReference.new(name: "deconstruction_declaration", is_token: false),
      ]),
      line_number: 1008,
    ),
    GT::GrammarRule.new(
      name: "deconstruction_declaration",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "var"),
          GT::RuleReference.new(name: "deconstruction_tuple", is_token: false),
          GT::RuleReference.new(name: "EQUALS", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "deconstruction_element", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "deconstruction_element", is_token: false),
            ])),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::RuleReference.new(name: "EQUALS", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
        ]),
      ]),
      line_number: 1012,
    ),
    GT::GrammarRule.new(
      name: "deconstruction_tuple",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1016,
    ),
    GT::GrammarRule.new(
      name: "deconstruction_element",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 1018,
    ),
    GT::GrammarRule.new(
      name: "local_constant_declaration_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "const"),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "constant_declarators", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1020,
    ),
    GT::GrammarRule.new(
      name: "using_declaration_statement",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Literal.new(value: "await")),
        GT::Literal.new(value: "using"),
        GT::OptionalElement.new(element: GT::Literal.new(value: "ref")),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "variable_declarators", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1022,
    ),
    GT::GrammarRule.new(
      name: "empty_statement",
      body: GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      line_number: 1024,
    ),
    GT::GrammarRule.new(
      name: "expression_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1026,
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
      line_number: 1028,
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
      line_number: 1030,
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
      line_number: 1032,
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
      line_number: 1034,
    ),
    GT::GrammarRule.new(
      name: "for_initializer",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "local_variable_declaration", is_token: false),
        GT::RuleReference.new(name: "expression_list", is_token: false),
      ]),
      line_number: 1037,
    ),
    GT::GrammarRule.new(
      name: "for_iterator",
      body: GT::RuleReference.new(name: "expression_list", is_token: false),
      line_number: 1040,
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
      line_number: 1042,
    ),
    GT::GrammarRule.new(
      name: "foreach_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "foreach"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "type", is_token: false),
              GT::RuleReference.new(name: "NAME", is_token: true),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "var"),
              GT::RuleReference.new(name: "NAME", is_token: true),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "var"),
              GT::RuleReference.new(name: "deconstruction_tuple", is_token: false),
            ]),
          ])),
        GT::Literal.new(value: "in"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 1044,
    ),
    GT::GrammarRule.new(
      name: "await_foreach_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "await"),
        GT::Literal.new(value: "foreach"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "type", is_token: false),
              GT::RuleReference.new(name: "NAME", is_token: true),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "var"),
              GT::RuleReference.new(name: "NAME", is_token: true),
            ]),
          ])),
        GT::Literal.new(value: "in"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 1047,
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
      line_number: 1050,
    ),
    GT::GrammarRule.new(
      name: "switch_block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "switch_section", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 1052,
    ),
    GT::GrammarRule.new(
      name: "switch_section",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "switch_label", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
      ]),
      line_number: 1054,
    ),
    GT::GrammarRule.new(
      name: "switch_label",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "case"),
          GT::RuleReference.new(name: "pattern", is_token: false),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::Literal.new(value: "when"),
              GT::RuleReference.new(name: "expression", is_token: false),
            ])),
          GT::RuleReference.new(name: "COLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "default"),
          GT::RuleReference.new(name: "COLON", is_token: true),
        ]),
      ]),
      line_number: 1056,
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
      line_number: 1059,
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
      line_number: 1062,
    ),
    GT::GrammarRule.new(
      name: "specific_catch_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "catch"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "when"),
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::RuleReference.new(name: "expression", is_token: false),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 1065,
    ),
    GT::GrammarRule.new(
      name: "general_catch_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "catch"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 1068,
    ),
    GT::GrammarRule.new(
      name: "finally_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "finally"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 1070,
    ),
    GT::GrammarRule.new(
      name: "throw_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "throw"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1072,
    ),
    GT::GrammarRule.new(
      name: "return_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "return"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::OptionalElement.new(element: GT::Literal.new(value: "ref")),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1074,
    ),
    GT::GrammarRule.new(
      name: "break_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "break"),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1076,
    ),
    GT::GrammarRule.new(
      name: "continue_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "continue"),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 1078,
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
      line_number: 1080,
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
      line_number: 1084,
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
      line_number: 1086,
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
      line_number: 1088,
    ),
    GT::GrammarRule.new(
      name: "await_using_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "await"),
        GT::Literal.new(value: "using"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "resource_acquisition", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 1091,
    ),
    GT::GrammarRule.new(
      name: "checked_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "checked"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 1093,
    ),
    GT::GrammarRule.new(
      name: "unchecked_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "unchecked"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 1095,
    ),
    GT::GrammarRule.new(
      name: "labelled_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 1097,
    ),
    GT::GrammarRule.new(
      name: "unsafe_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "unsafe"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 1099,
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
      line_number: 1101,
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
      line_number: 1103,
    ),
    GT::GrammarRule.new(
      name: "local_function_declaration",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "local_function_modifier", is_token: false)),
        GT::RuleReference.new(name: "return_type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "type_parameter_constraint_clause", is_token: false)),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "block", is_token: false),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LAMBDA_ARROW", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            ]),
          ])),
      ]),
      line_number: 1106,
    ),
    GT::GrammarRule.new(
      name: "local_function_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "static"),
        GT::Literal.new(value: "async"),
        GT::Literal.new(value: "unsafe"),
      ]),
      line_number: 1112,
    ),
    GT::GrammarRule.new(
      name: "pattern",
      body: GT::RuleReference.new(name: "or_pattern", is_token: false),
      line_number: 1139,
    ),
    GT::GrammarRule.new(
      name: "or_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "and_pattern", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "or"),
            GT::RuleReference.new(name: "and_pattern", is_token: false),
          ])),
      ]),
      line_number: 1141,
    ),
    GT::GrammarRule.new(
      name: "and_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "not_pattern", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "and"),
            GT::RuleReference.new(name: "not_pattern", is_token: false),
          ])),
      ]),
      line_number: 1143,
    ),
    GT::GrammarRule.new(
      name: "not_pattern",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "not"),
          GT::RuleReference.new(name: "atomic_pattern", is_token: false),
        ]),
        GT::RuleReference.new(name: "atomic_pattern", is_token: false),
      ]),
      line_number: 1145,
    ),
    GT::GrammarRule.new(
      name: "atomic_pattern",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "parenthesized_pattern", is_token: false),
        GT::RuleReference.new(name: "relational_pattern", is_token: false),
        GT::RuleReference.new(name: "discard_pattern", is_token: false),
        GT::RuleReference.new(name: "constant_pattern", is_token: false),
        GT::RuleReference.new(name: "var_pattern", is_token: false),
        GT::RuleReference.new(name: "declaration_pattern", is_token: false),
        GT::RuleReference.new(name: "property_pattern", is_token: false),
        GT::RuleReference.new(name: "tuple_pattern", is_token: false),
        GT::RuleReference.new(name: "positional_pattern", is_token: false),
      ]),
      line_number: 1148,
    ),
    GT::GrammarRule.new(
      name: "parenthesized_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "pattern", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1158,
    ),
    GT::GrammarRule.new(
      name: "relational_pattern",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "LESS_THAN", is_token: true),
            GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
            GT::RuleReference.new(name: "LESS_EQUALS", is_token: true),
            GT::RuleReference.new(name: "GREATER_EQUALS", is_token: true),
          ])),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 1160,
    ),
    GT::GrammarRule.new(
      name: "constant_pattern",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "literal", is_token: false),
        GT::RuleReference.new(name: "qualified_name", is_token: false),
      ]),
      line_number: 1162,
    ),
    GT::GrammarRule.new(
      name: "declaration_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 1165,
    ),
    GT::GrammarRule.new(
      name: "var_pattern",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "var"),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 1167,
    ),
    GT::GrammarRule.new(
      name: "discard_pattern",
      body: GT::RuleReference.new(name: "NAME", is_token: true),
      line_number: 1169,
    ),
    GT::GrammarRule.new(
      name: "property_pattern",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type", is_token: false)),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "property_subpattern_list", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
      ]),
      line_number: 1180,
    ),
    GT::GrammarRule.new(
      name: "property_subpattern_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "property_subpattern", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "property_subpattern", is_token: false),
          ])),
      ]),
      line_number: 1182,
    ),
    GT::GrammarRule.new(
      name: "property_subpattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "dotted_property_name", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "pattern", is_token: false),
      ]),
      line_number: 1187,
    ),
    GT::GrammarRule.new(
      name: "dotted_property_name",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "DOT", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 1197,
    ),
    GT::GrammarRule.new(
      name: "tuple_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "subpattern", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "subpattern", is_token: false),
          ])),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1199,
    ),
    GT::GrammarRule.new(
      name: "positional_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "subpattern", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "subpattern", is_token: false),
              ])),
          ])),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "property_pattern", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
      ]),
      line_number: 1201,
    ),
    GT::GrammarRule.new(
      name: "subpattern",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "COLON", is_token: true),
          ])),
        GT::RuleReference.new(name: "pattern", is_token: false),
      ]),
      line_number: 1204,
    ),
    GT::GrammarRule.new(
      name: "expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "lambda_expression", is_token: false),
        GT::RuleReference.new(name: "assignment_expression", is_token: false),
      ]),
      line_number: 1225,
    ),
    GT::GrammarRule.new(
      name: "lambda_expression",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "async")),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type", is_token: false)),
        GT::RuleReference.new(name: "lambda_parameters", is_token: false),
        GT::RuleReference.new(name: "LAMBDA_ARROW", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "expression", is_token: false),
            GT::RuleReference.new(name: "block", is_token: false),
          ])),
      ]),
      line_number: 1240,
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
      line_number: 1243,
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
      line_number: 1246,
    ),
    GT::GrammarRule.new(
      name: "lambda_parameter",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "attribute_section", is_token: false)),
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "ref"),
            GT::Literal.new(value: "out"),
            GT::Literal.new(value: "in"),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type", is_token: false)),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 1249,
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
        GT::RuleReference.new(name: "throw_expression", is_token: false),
      ]),
      line_number: 1251,
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
        GT::RuleReference.new(name: "QUESTION_QUESTION_EQUALS", is_token: true),
      ]),
      line_number: 1255,
    ),
    GT::GrammarRule.new(
      name: "throw_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "throw"),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 1268,
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
      line_number: 1270,
    ),
    GT::GrammarRule.new(
      name: "null_coalescing_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "logical_or_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "NULL_COALESCING", is_token: true),
            GT::RuleReference.new(name: "logical_or_expression", is_token: false),
          ])),
      ]),
      line_number: 1273,
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
      line_number: 1276,
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
      line_number: 1278,
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
      line_number: 1280,
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
      line_number: 1282,
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
      line_number: 1284,
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
      line_number: 1286,
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
              GT::RuleReference.new(name: "pattern", is_token: false),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "as"),
              GT::RuleReference.new(name: "type", is_token: false),
            ]),
          ])),
      ]),
      line_number: 1289,
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
      line_number: 1295,
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
      line_number: 1298,
    ),
    GT::GrammarRule.new(
      name: "multiplicative_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "range_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "STAR", is_token: true),
                GT::RuleReference.new(name: "SLASH", is_token: true),
                GT::RuleReference.new(name: "PERCENT", is_token: true),
              ])),
            GT::RuleReference.new(name: "range_expression", is_token: false),
          ])),
      ]),
      line_number: 1301,
    ),
    GT::GrammarRule.new(
      name: "range_expression",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "unary_expression", is_token: false),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "DOT_DOT", is_token: true),
              GT::OptionalElement.new(element: GT::RuleReference.new(name: "unary_expression", is_token: false)),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "DOT_DOT", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "unary_expression", is_token: false)),
        ]),
      ]),
      line_number: 1304,
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
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "CARET", is_token: true),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "await"),
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
      line_number: 1307,
    ),
    GT::GrammarRule.new(
      name: "cast_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "unary_expression", is_token: false),
      ]),
      line_number: 1320,
    ),
    GT::GrammarRule.new(
      name: "postfix_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "primary_expression", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "postfix_operator", is_token: false)),
      ]),
      line_number: 1322,
    ),
    GT::GrammarRule.new(
      name: "postfix_operator",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "PLUS_PLUS", is_token: true),
        GT::RuleReference.new(name: "MINUS_MINUS", is_token: true),
        GT::RuleReference.new(name: "BANG", is_token: true),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "with"),
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "with_initializer_list", is_token: false)),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
      ]),
      line_number: 1324,
    ),
    GT::GrammarRule.new(
      name: "with_initializer_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "with_initializer", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "with_initializer", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
      ]),
      line_number: 1329,
    ),
    GT::GrammarRule.new(
      name: "with_initializer",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 1331,
    ),
    GT::GrammarRule.new(
      name: "primary_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "primary", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "primary_suffix", is_token: false)),
      ]),
      line_number: 1337,
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
          GT::RuleReference.new(name: "NULL_CONDITIONAL_DOT", is_token: true),
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
          GT::RuleReference.new(name: "argument_list", is_token: false),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NULL_CONDITIONAL_BRACKET", is_token: true),
          GT::RuleReference.new(name: "argument_list", is_token: false),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
        GT::RuleReference.new(name: "BANG", is_token: true),
      ]),
      line_number: 1339,
    ),
    GT::GrammarRule.new(
      name: "primary",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "literal", is_token: false),
        GT::RuleReference.new(name: "interpolated_string", is_token: false),
        GT::Literal.new(value: "this"),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "base"),
          GT::RuleReference.new(name: "DOT", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "base"),
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::RuleReference.new(name: "argument_list", is_token: false),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
        GT::RuleReference.new(name: "typeof_expression", is_token: false),
        GT::RuleReference.new(name: "sizeof_expression", is_token: false),
        GT::RuleReference.new(name: "checked_expression", is_token: false),
        GT::RuleReference.new(name: "unchecked_expression", is_token: false),
        GT::RuleReference.new(name: "default_value_expression", is_token: false),
        GT::RuleReference.new(name: "nameof_expression", is_token: false),
        GT::RuleReference.new(name: "new_expression", is_token: false),
        GT::RuleReference.new(name: "target_typed_new_expression", is_token: false),
        GT::RuleReference.new(name: "stackalloc_expression", is_token: false),
        GT::RuleReference.new(name: "switch_expression", is_token: false),
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
      line_number: 1346,
    ),
    GT::GrammarRule.new(
      name: "interpolated_string",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "INTERPOLATED_STRING", is_token: true),
        GT::RuleReference.new(name: "INTERPOLATED_VERBATIM", is_token: true),
      ]),
      line_number: 1364,
    ),
    GT::GrammarRule.new(
      name: "typeof_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "typeof"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type_or_void", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1367,
    ),
    GT::GrammarRule.new(
      name: "type_or_void",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::Literal.new(value: "void"),
      ]),
      line_number: 1369,
    ),
    GT::GrammarRule.new(
      name: "sizeof_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "sizeof"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1371,
    ),
    GT::GrammarRule.new(
      name: "checked_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "checked"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1373,
    ),
    GT::GrammarRule.new(
      name: "unchecked_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "unchecked"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1375,
    ),
    GT::GrammarRule.new(
      name: "default_value_expression",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "default"),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "type", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Literal.new(value: "default"),
      ]),
      line_number: 1377,
    ),
    GT::GrammarRule.new(
      name: "nameof_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "nameof"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 1380,
    ),
    GT::GrammarRule.new(
      name: "target_typed_new_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "new"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "argument_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "object_or_collection_initializer", is_token: false)),
      ]),
      line_number: 1382,
    ),
    GT::GrammarRule.new(
      name: "new_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "new"),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "anonymous_object_creation", is_token: false),
            GT::RuleReference.new(name: "new_array_expression", is_token: false),
            GT::RuleReference.new(name: "new_object_expression", is_token: false),
          ])),
      ]),
      line_number: 1385,
    ),
    GT::GrammarRule.new(
      name: "new_object_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "argument_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "object_or_collection_initializer", is_token: false)),
      ]),
      line_number: 1389,
    ),
    GT::GrammarRule.new(
      name: "object_or_collection_initializer",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "object_initializer", is_token: false),
        GT::RuleReference.new(name: "collection_initializer", is_token: false),
      ]),
      line_number: 1391,
    ),
    GT::GrammarRule.new(
      name: "object_initializer",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "member_initializer", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "member_initializer", is_token: false),
              ])),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 1394,
    ),
    GT::GrammarRule.new(
      name: "member_initializer",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "expression", is_token: false),
            GT::RuleReference.new(name: "object_initializer", is_token: false),
          ])),
      ]),
      line_number: 1396,
    ),
    GT::GrammarRule.new(
      name: "collection_initializer",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "element_initializer", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "element_initializer", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 1398,
    ),
    GT::GrammarRule.new(
      name: "element_initializer",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "expression_list", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
      ]),
      line_number: 1400,
    ),
    GT::GrammarRule.new(
      name: "anonymous_object_creation",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "anonymous_member", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "anonymous_member", is_token: false),
              ])),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 1403,
    ),
    GT::GrammarRule.new(
      name: "anonymous_member",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "EQUALS", is_token: true),
          ])),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 1405,
    ),
    GT::GrammarRule.new(
      name: "new_array_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "array_type", is_token: false),
        GT::RuleReference.new(name: "array_creation_suffix", is_token: false),
      ]),
      line_number: 1407,
    ),
    GT::GrammarRule.new(
      name: "array_type",
      body: GT::Group.new(element: GT::Alternation.new(choices: [
          GT::RuleReference.new(name: "primitive_type", is_token: false),
          GT::RuleReference.new(name: "qualified_name", is_token: false),
        ])),
      line_number: 1409,
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
      line_number: 1411,
    ),
    GT::GrammarRule.new(
      name: "stackalloc_expression",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "stackalloc"),
          GT::RuleReference.new(name: "type", is_token: false),
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "array_initializer", is_token: false)),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "stackalloc"),
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
          GT::RuleReference.new(name: "array_initializer", is_token: false),
        ]),
      ]),
      line_number: 1415,
    ),
    GT::GrammarRule.new(
      name: "switch_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "primary_expression", is_token: false),
        GT::Literal.new(value: "switch"),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "switch_expression_arm", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "switch_expression_arm", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 1419,
    ),
    GT::GrammarRule.new(
      name: "switch_expression_arm",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "pattern", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "when"),
            GT::RuleReference.new(name: "expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "LAMBDA_ARROW", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 1423,
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
      line_number: 1425,
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
            GT::Literal.new(value: "in"),
          ])),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 1427,
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
      line_number: 1438,
    ),
  ],
)
