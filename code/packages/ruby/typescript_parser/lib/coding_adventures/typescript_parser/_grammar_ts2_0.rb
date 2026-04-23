# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: ts2.0.grammar
# Regenerate with: grammar-tools compile-grammar ts2.0.grammar
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
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "source_element", is_token: false)),
      line_number: 61,
    ),
    GT::GrammarRule.new(
      name: "source_element",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "import_declaration", is_token: false),
        GT::RuleReference.new(name: "export_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_declaration", is_token: false),
        GT::RuleReference.new(name: "type_alias_declaration", is_token: false),
        GT::RuleReference.new(name: "enum_declaration", is_token: false),
        GT::RuleReference.new(name: "namespace_declaration", is_token: false),
        GT::RuleReference.new(name: "ambient_declaration", is_token: false),
        GT::RuleReference.new(name: "function_declaration", is_token: false),
        GT::RuleReference.new(name: "generator_declaration", is_token: false),
        GT::RuleReference.new(name: "ts_class_declaration", is_token: false),
        GT::RuleReference.new(name: "lexical_declaration", is_token: false),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 66,
    ),
    GT::GrammarRule.new(
      name: "function_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "function"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "function_body", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 93,
    ),
    GT::GrammarRule.new(
      name: "function_body",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "source_element", is_token: false)),
      line_number: 97,
    ),
    GT::GrammarRule.new(
      name: "typed_parameter_list",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "typed_parameter", is_token: false),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "typed_parameter", is_token: false),
            ])),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "rest_typed_parameter", is_token: false),
            ])),
        ]),
        GT::RuleReference.new(name: "rest_typed_parameter", is_token: false),
      ]),
      line_number: 115,
    ),
    GT::GrammarRule.new(
      name: "typed_parameter",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "accessibility_modifier", is_token: false)),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "binding_pattern", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "QUESTION", is_token: true)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "assignment_expression", is_token: false),
          ])),
      ]),
      line_number: 118,
    ),
    GT::GrammarRule.new(
      name: "rest_typed_parameter",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "ELLIPSIS", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
      ]),
      line_number: 120,
    ),
    GT::GrammarRule.new(
      name: "lexical_declaration",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "let"),
            GT::Literal.new(value: "const"),
          ])),
        GT::RuleReference.new(name: "typed_binding_list", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 136,
    ),
    GT::GrammarRule.new(
      name: "typed_binding_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "typed_lexical_binding", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "typed_lexical_binding", is_token: false),
          ])),
      ]),
      line_number: 138,
    ),
    GT::GrammarRule.new(
      name: "typed_lexical_binding",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "binding_pattern", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "assignment_expression", is_token: false),
          ])),
      ]),
      line_number: 140,
    ),
    GT::GrammarRule.new(
      name: "lexical_declaration_no_semi",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "let"),
            GT::Literal.new(value: "const"),
          ])),
        GT::RuleReference.new(name: "typed_binding_list", is_token: false),
      ]),
      line_number: 143,
    ),
    GT::GrammarRule.new(
      name: "variable_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "var"),
        GT::RuleReference.new(name: "variable_declaration_list", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 150,
    ),
    GT::GrammarRule.new(
      name: "variable_declaration_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "variable_declaration", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "variable_declaration", is_token: false),
          ])),
      ]),
      line_number: 152,
    ),
    GT::GrammarRule.new(
      name: "variable_declaration",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "binding_pattern", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "assignment_expression", is_token: false),
          ])),
      ]),
      line_number: 154,
    ),
    GT::GrammarRule.new(
      name: "generator_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "function"),
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "function_body", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 165,
    ),
    GT::GrammarRule.new(
      name: "ts_class_declaration",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Literal.new(value: "abstract")),
        GT::Literal.new(value: "class"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "ts_class_heritage", is_token: false)),
        GT::RuleReference.new(name: "ts_class_body", is_token: false),
      ]),
      line_number: 190,
    ),
    GT::GrammarRule.new(
      name: "ts_class_heritage",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "extends"),
            GT::RuleReference.new(name: "type_reference", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "implements"),
            GT::RuleReference.new(name: "type_reference", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "type_reference", is_token: false),
              ])),
          ])),
      ]),
      line_number: 192,
    ),
    GT::GrammarRule.new(
      name: "ts_class_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "ts_class_element", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 194,
    ),
    GT::GrammarRule.new(
      name: "ts_class_element",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "ts_method_definition", is_token: false),
        GT::RuleReference.new(name: "ts_property_declaration", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "index_signature", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 196,
    ),
    GT::GrammarRule.new(
      name: "ts_method_definition",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "accessibility_modifier", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "abstract")),
        GT::OptionalElement.new(element: GT::Literal.new(value: "static")),
        GT::OptionalElement.new(element: GT::Literal.new(value: "readonly")),
        GT::RuleReference.new(name: "ts_method_definition_body", is_token: false),
      ]),
      line_number: 201,
    ),
    GT::GrammarRule.new(
      name: "accessibility_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "public"),
        GT::Literal.new(value: "private"),
        GT::Literal.new(value: "protected"),
      ]),
      line_number: 203,
    ),
    GT::GrammarRule.new(
      name: "ts_method_definition_body",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "QUESTION", is_token: true)),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::RuleReference.new(name: "type_expression", is_token: false),
            ])),
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "get"),
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::RuleReference.new(name: "type_expression", is_token: false),
            ])),
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "set"),
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "typed_parameter", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "STAR", is_token: true),
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "QUESTION", is_token: true)),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::RuleReference.new(name: "type_expression", is_token: false),
            ])),
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
      ]),
      line_number: 205,
    ),
    GT::GrammarRule.new(
      name: "ts_property_declaration",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "accessibility_modifier", is_token: false)),
        GT::OptionalElement.new(element: GT::Literal.new(value: "static")),
        GT::OptionalElement.new(element: GT::Literal.new(value: "abstract")),
        GT::OptionalElement.new(element: GT::Literal.new(value: "readonly")),
        GT::RuleReference.new(name: "property_name", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "QUESTION", is_token: true)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "assignment_expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 210,
    ),
    GT::GrammarRule.new(
      name: "import_declaration",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "import"),
          GT::RuleReference.new(name: "import_clause", is_token: false),
          GT::RuleReference.new(name: "from_clause", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "import"),
          GT::RuleReference.new(name: "STRING", is_token: true),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "import"),
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "EQUALS", is_token: true),
          GT::Literal.new(value: "require"),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "STRING", is_token: true),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
      ]),
      line_number: 232,
    ),
    GT::GrammarRule.new(
      name: "import_clause",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "default_import", is_token: false),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::Group.new(element: GT::Alternation.new(choices: [
                  GT::RuleReference.new(name: "named_imports", is_token: false),
                  GT::RuleReference.new(name: "namespace_import", is_token: false),
                ])),
            ])),
        ]),
        GT::RuleReference.new(name: "named_imports", is_token: false),
        GT::RuleReference.new(name: "namespace_import", is_token: false),
      ]),
      line_number: 239,
    ),
    GT::GrammarRule.new(
      name: "default_import",
      body: GT::RuleReference.new(name: "NAME", is_token: true),
      line_number: 243,
    ),
    GT::GrammarRule.new(
      name: "named_imports",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "import_specifier", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "import_specifier", is_token: false),
              ])),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
          ])),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 245,
    ),
    GT::GrammarRule.new(
      name: "import_specifier",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "as"),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 247,
    ),
    GT::GrammarRule.new(
      name: "namespace_import",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::Literal.new(value: "as"),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 249,
    ),
    GT::GrammarRule.new(
      name: "from_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "from"),
        GT::RuleReference.new(name: "STRING", is_token: true),
      ]),
      line_number: 251,
    ),
    GT::GrammarRule.new(
      name: "export_declaration",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "export"),
          GT::Literal.new(value: "default"),
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::RuleReference.new(name: "function_declaration", is_token: false),
              GT::RuleReference.new(name: "generator_declaration", is_token: false),
              GT::RuleReference.new(name: "ts_class_declaration", is_token: false),
              GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "assignment_expression", is_token: false),
                GT::RuleReference.new(name: "SEMICOLON", is_token: true),
              ]),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "export"),
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::RuleReference.new(name: "function_declaration", is_token: false),
              GT::RuleReference.new(name: "generator_declaration", is_token: false),
              GT::RuleReference.new(name: "ts_class_declaration", is_token: false),
              GT::RuleReference.new(name: "lexical_declaration", is_token: false),
              GT::RuleReference.new(name: "variable_statement", is_token: false),
              GT::RuleReference.new(name: "interface_declaration", is_token: false),
              GT::RuleReference.new(name: "type_alias_declaration", is_token: false),
              GT::RuleReference.new(name: "enum_declaration", is_token: false),
              GT::RuleReference.new(name: "namespace_declaration", is_token: false),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "export"),
          GT::RuleReference.new(name: "named_exports", is_token: false),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "from_clause", is_token: false)),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "export"),
          GT::RuleReference.new(name: "EQUALS", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
      ]),
      line_number: 253,
    ),
    GT::GrammarRule.new(
      name: "named_exports",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "export_specifier", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "export_specifier", is_token: false),
              ])),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
          ])),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 269,
    ),
    GT::GrammarRule.new(
      name: "export_specifier",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "as"),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
      ]),
      line_number: 271,
    ),
    GT::GrammarRule.new(
      name: "statement",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "block", is_token: false),
        GT::RuleReference.new(name: "variable_statement", is_token: false),
        GT::RuleReference.new(name: "empty_statement", is_token: false),
        GT::RuleReference.new(name: "if_statement", is_token: false),
        GT::RuleReference.new(name: "while_statement", is_token: false),
        GT::RuleReference.new(name: "do_while_statement", is_token: false),
        GT::RuleReference.new(name: "for_statement", is_token: false),
        GT::RuleReference.new(name: "for_in_statement", is_token: false),
        GT::RuleReference.new(name: "for_of_statement", is_token: false),
        GT::RuleReference.new(name: "continue_statement", is_token: false),
        GT::RuleReference.new(name: "break_statement", is_token: false),
        GT::RuleReference.new(name: "return_statement", is_token: false),
        GT::RuleReference.new(name: "with_statement", is_token: false),
        GT::RuleReference.new(name: "switch_statement", is_token: false),
        GT::RuleReference.new(name: "labelled_statement", is_token: false),
        GT::RuleReference.new(name: "try_statement", is_token: false),
        GT::RuleReference.new(name: "throw_statement", is_token: false),
        GT::RuleReference.new(name: "debugger_statement", is_token: false),
        GT::RuleReference.new(name: "expression_statement", is_token: false),
      ]),
      line_number: 280,
    ),
    GT::GrammarRule.new(
      name: "block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 300,
    ),
    GT::GrammarRule.new(
      name: "empty_statement",
      body: GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      line_number: 302,
    ),
    GT::GrammarRule.new(
      name: "expression_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 304,
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
      line_number: 306,
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
      line_number: 308,
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
      line_number: 310,
    ),
    GT::GrammarRule.new(
      name: "for_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "for"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "var"),
              GT::RuleReference.new(name: "variable_declaration_list", is_token: false),
            ]),
            GT::RuleReference.new(name: "lexical_declaration_no_semi", is_token: false),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 312,
    ),
    GT::GrammarRule.new(
      name: "for_in_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "for"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "var"),
              GT::RuleReference.new(name: "variable_declaration", is_token: false),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "let"),
              GT::Group.new(element: GT::Alternation.new(choices: [
                  GT::RuleReference.new(name: "NAME", is_token: true),
                  GT::RuleReference.new(name: "binding_pattern", is_token: false),
                ])),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "const"),
              GT::Group.new(element: GT::Alternation.new(choices: [
                  GT::RuleReference.new(name: "NAME", is_token: true),
                  GT::RuleReference.new(name: "binding_pattern", is_token: false),
                ])),
            ]),
            GT::RuleReference.new(name: "left_hand_side_expression", is_token: false),
          ])),
        GT::Literal.new(value: "in"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 320,
    ),
    GT::GrammarRule.new(
      name: "for_of_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "for"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "var"),
              GT::RuleReference.new(name: "variable_declaration", is_token: false),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "let"),
              GT::Group.new(element: GT::Alternation.new(choices: [
                  GT::RuleReference.new(name: "NAME", is_token: true),
                  GT::RuleReference.new(name: "binding_pattern", is_token: false),
                ])),
              GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                  GT::RuleReference.new(name: "COLON", is_token: true),
                  GT::RuleReference.new(name: "type_expression", is_token: false),
                ])),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "const"),
              GT::Group.new(element: GT::Alternation.new(choices: [
                  GT::RuleReference.new(name: "NAME", is_token: true),
                  GT::RuleReference.new(name: "binding_pattern", is_token: false),
                ])),
              GT::OptionalElement.new(element: GT::Sequence.new(elements: [
                  GT::RuleReference.new(name: "COLON", is_token: true),
                  GT::RuleReference.new(name: "type_expression", is_token: false),
                ])),
            ]),
            GT::RuleReference.new(name: "left_hand_side_expression", is_token: false),
          ])),
        GT::Literal.new(value: "of"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 329,
    ),
    GT::GrammarRule.new(
      name: "continue_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "continue"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 336,
    ),
    GT::GrammarRule.new(
      name: "break_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "break"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 338,
    ),
    GT::GrammarRule.new(
      name: "return_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "return"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 340,
    ),
    GT::GrammarRule.new(
      name: "with_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "with"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 342,
    ),
    GT::GrammarRule.new(
      name: "switch_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "switch"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "case_clause", is_token: false)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "default_clause", is_token: false),
            GT::Repetition.new(element: GT::RuleReference.new(name: "case_clause", is_token: false)),
          ])),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 344,
    ),
    GT::GrammarRule.new(
      name: "case_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "case"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
      ]),
      line_number: 347,
    ),
    GT::GrammarRule.new(
      name: "default_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "default"),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
      ]),
      line_number: 349,
    ),
    GT::GrammarRule.new(
      name: "labelled_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 351,
    ),
    GT::GrammarRule.new(
      name: "try_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "try"),
        GT::RuleReference.new(name: "block", is_token: false),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "catch_clause", is_token: false),
              GT::OptionalElement.new(element: GT::RuleReference.new(name: "finally_clause", is_token: false)),
            ]),
            GT::RuleReference.new(name: "finally_clause", is_token: false),
          ])),
      ]),
      line_number: 353,
    ),
    GT::GrammarRule.new(
      name: "catch_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "catch"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 355,
    ),
    GT::GrammarRule.new(
      name: "finally_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "finally"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 357,
    ),
    GT::GrammarRule.new(
      name: "throw_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "throw"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 359,
    ),
    GT::GrammarRule.new(
      name: "debugger_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "debugger"),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 361,
    ),
    GT::GrammarRule.new(
      name: "binding_pattern",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "object_binding_pattern", is_token: false),
        GT::RuleReference.new(name: "array_binding_pattern", is_token: false),
      ]),
      line_number: 375,
    ),
    GT::GrammarRule.new(
      name: "object_binding_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "binding_property", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "binding_property", is_token: false),
              ])),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
          ])),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 377,
    ),
    GT::GrammarRule.new(
      name: "binding_property",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::RuleReference.new(name: "binding_element", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "EQUALS", is_token: true),
              GT::RuleReference.new(name: "assignment_expression", is_token: false),
            ])),
        ]),
      ]),
      line_number: 379,
    ),
    GT::GrammarRule.new(
      name: "binding_element",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "binding_pattern", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "assignment_expression", is_token: false),
          ])),
      ]),
      line_number: 382,
    ),
    GT::GrammarRule.new(
      name: "array_binding_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "array_binding_element", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "array_binding_element", is_token: false),
              ])),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
          ])),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 384,
    ),
    GT::GrammarRule.new(
      name: "array_binding_element",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "binding_element", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "ELLIPSIS", is_token: true),
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::RuleReference.new(name: "NAME", is_token: true),
              GT::RuleReference.new(name: "binding_pattern", is_token: false),
            ])),
        ]),
      ]),
      line_number: 386,
    ),
    GT::GrammarRule.new(
      name: "expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "assignment_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "assignment_expression", is_token: false),
          ])),
      ]),
      line_number: 396,
    ),
    GT::GrammarRule.new(
      name: "assignment_expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "arrow_function", is_token: false),
        GT::RuleReference.new(name: "ts_as_expression", is_token: false),
        GT::RuleReference.new(name: "ts_angle_bracket_assertion", is_token: false),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "yield"),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "STAR", is_token: true)),
          GT::RuleReference.new(name: "assignment_expression", is_token: false),
        ]),
        GT::RuleReference.new(name: "conditional_expression", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "left_hand_side_expression", is_token: false),
          GT::RuleReference.new(name: "assignment_operator", is_token: false),
          GT::RuleReference.new(name: "assignment_expression", is_token: false),
        ]),
      ]),
      line_number: 403,
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
      line_number: 410,
    ),
    GT::GrammarRule.new(
      name: "arrow_function",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "arrow_parameters", is_token: false),
        GT::RuleReference.new(name: "ARROW", is_token: true),
        GT::RuleReference.new(name: "concise_body", is_token: false),
      ]),
      line_number: 426,
    ),
    GT::GrammarRule.new(
      name: "arrow_parameters",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::RuleReference.new(name: "type_expression", is_token: false),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LESS_THAN", is_token: true),
          GT::RuleReference.new(name: "type_parameter_list", is_token: false),
          GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::RuleReference.new(name: "type_expression", is_token: false),
            ])),
        ]),
      ]),
      line_number: 428,
    ),
    GT::GrammarRule.new(
      name: "concise_body",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
        GT::RuleReference.new(name: "assignment_expression", is_token: false),
      ]),
      line_number: 432,
    ),
    GT::GrammarRule.new(
      name: "ts_as_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "ts_non_null_expression", is_token: false),
        GT::Literal.new(value: "as"),
        GT::RuleReference.new(name: "type_expression", is_token: false),
      ]),
      line_number: 438,
    ),
    GT::GrammarRule.new(
      name: "ts_non_null_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "left_hand_side_expression", is_token: false),
        GT::RuleReference.new(name: "BANG", is_token: true),
      ]),
      line_number: 442,
    ),
    GT::GrammarRule.new(
      name: "ts_angle_bracket_assertion",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LESS_THAN", is_token: true),
        GT::RuleReference.new(name: "type_expression", is_token: false),
        GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
        GT::RuleReference.new(name: "assignment_expression", is_token: false),
      ]),
      line_number: 445,
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
      line_number: 449,
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
      line_number: 454,
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
      line_number: 456,
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
      line_number: 460,
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
      line_number: 462,
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
      line_number: 464,
    ),
    GT::GrammarRule.new(
      name: "equality_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "relational_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "STRICT_EQUALS", is_token: true),
                GT::RuleReference.new(name: "STRICT_NOT_EQUALS", is_token: true),
                GT::RuleReference.new(name: "EQUALS_EQUALS", is_token: true),
                GT::RuleReference.new(name: "NOT_EQUALS", is_token: true),
              ])),
            GT::RuleReference.new(name: "relational_expression", is_token: false),
          ])),
      ]),
      line_number: 468,
    ),
    GT::GrammarRule.new(
      name: "relational_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "shift_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "LESS_THAN", is_token: true),
                GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
                GT::RuleReference.new(name: "LESS_EQUALS", is_token: true),
                GT::RuleReference.new(name: "GREATER_EQUALS", is_token: true),
                GT::Literal.new(value: "instanceof"),
                GT::Literal.new(value: "in"),
              ])),
            GT::RuleReference.new(name: "shift_expression", is_token: false),
          ])),
      ]),
      line_number: 474,
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
      line_number: 480,
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
      line_number: 485,
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
      line_number: 488,
    ),
    GT::GrammarRule.new(
      name: "unary_expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "postfix_expression", is_token: false),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "delete"),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "void"),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "typeof"),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
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
          GT::RuleReference.new(name: "TILDE", is_token: true),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "BANG", is_token: true),
          GT::RuleReference.new(name: "unary_expression", is_token: false),
        ]),
      ]),
      line_number: 493,
    ),
    GT::GrammarRule.new(
      name: "postfix_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "left_hand_side_expression", is_token: false),
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "PLUS_PLUS", is_token: true),
            GT::RuleReference.new(name: "MINUS_MINUS", is_token: true),
          ])),
      ]),
      line_number: 506,
    ),
    GT::GrammarRule.new(
      name: "left_hand_side_expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "call_expression", is_token: false),
        GT::RuleReference.new(name: "new_expression", is_token: false),
      ]),
      line_number: 510,
    ),
    GT::GrammarRule.new(
      name: "call_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "member_expression", is_token: false),
        GT::RuleReference.new(name: "arguments", is_token: false),
        GT::Repetition.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "arguments", is_token: false),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "DOT", is_token: true),
              GT::RuleReference.new(name: "NAME", is_token: true),
            ]),
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "LBRACKET", is_token: true),
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::RuleReference.new(name: "RBRACKET", is_token: true),
            ]),
            GT::RuleReference.new(name: "template_literal", is_token: false),
          ])),
      ]),
      line_number: 514,
    ),
    GT::GrammarRule.new(
      name: "new_expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "member_expression", is_token: false),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "new"),
          GT::RuleReference.new(name: "new_expression", is_token: false),
        ]),
      ]),
      line_number: 518,
    ),
    GT::GrammarRule.new(
      name: "member_expression",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "primary_expression", is_token: false),
          GT::Repetition.new(element: GT::Alternation.new(choices: [
              GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "DOT", is_token: true),
                GT::RuleReference.new(name: "NAME", is_token: true),
              ]),
              GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "LBRACKET", is_token: true),
                GT::RuleReference.new(name: "expression", is_token: false),
                GT::RuleReference.new(name: "RBRACKET", is_token: true),
              ]),
              GT::RuleReference.new(name: "template_literal", is_token: false),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "super"),
          GT::RuleReference.new(name: "DOT", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "super"),
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "new"),
          GT::RuleReference.new(name: "member_expression", is_token: false),
          GT::RuleReference.new(name: "arguments", is_token: false),
        ]),
      ]),
      line_number: 521,
    ),
    GT::GrammarRule.new(
      name: "arguments",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "argument_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 527,
    ),
    GT::GrammarRule.new(
      name: "argument_list",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "ELLIPSIS", is_token: true),
              GT::RuleReference.new(name: "assignment_expression", is_token: false),
            ]),
            GT::RuleReference.new(name: "assignment_expression", is_token: false),
          ])),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::Sequence.new(elements: [
                  GT::RuleReference.new(name: "ELLIPSIS", is_token: true),
                  GT::RuleReference.new(name: "assignment_expression", is_token: false),
                ]),
                GT::RuleReference.new(name: "assignment_expression", is_token: false),
              ])),
          ])),
      ]),
      line_number: 529,
    ),
    GT::GrammarRule.new(
      name: "primary_expression",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "this"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "REGEX", is_token: true),
        GT::Literal.new(value: "true"),
        GT::Literal.new(value: "false"),
        GT::Literal.new(value: "null"),
        GT::RuleReference.new(name: "template_literal", is_token: false),
        GT::RuleReference.new(name: "array_literal", is_token: false),
        GT::RuleReference.new(name: "object_literal", is_token: false),
        GT::RuleReference.new(name: "function_expression", is_token: false),
        GT::RuleReference.new(name: "generator_expression", is_token: false),
        GT::RuleReference.new(name: "ts_class_expression", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 534,
    ),
    GT::GrammarRule.new(
      name: "template_literal",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "TEMPLATE_NO_SUB", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "TEMPLATE_HEAD", is_token: true),
          GT::Repetition.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "expression", is_token: false),
              GT::RuleReference.new(name: "TEMPLATE_MIDDLE", is_token: true),
            ])),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "TEMPLATE_TAIL", is_token: true),
        ]),
      ]),
      line_number: 556,
    ),
    GT::GrammarRule.new(
      name: "array_literal",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "array_element_list", is_token: false)),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 561,
    ),
    GT::GrammarRule.new(
      name: "array_element_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "array_element", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "array_element", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
      ]),
      line_number: 563,
    ),
    GT::GrammarRule.new(
      name: "array_element",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "ELLIPSIS", is_token: true),
          GT::RuleReference.new(name: "assignment_expression", is_token: false),
        ]),
        GT::RuleReference.new(name: "assignment_expression", is_token: false),
      ]),
      line_number: 565,
    ),
    GT::GrammarRule.new(
      name: "object_literal",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "property_definition", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "property_definition", is_token: false),
              ])),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
          ])),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 570,
    ),
    GT::GrammarRule.new(
      name: "property_definition",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::RuleReference.new(name: "assignment_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::RuleReference.new(name: "type_expression", is_token: false),
            ])),
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "get"),
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::RuleReference.new(name: "type_expression", is_token: false),
            ])),
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "set"),
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "typed_parameter", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "STAR", is_token: true),
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COLON", is_token: true),
              GT::RuleReference.new(name: "type_expression", is_token: false),
            ])),
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "ELLIPSIS", is_token: true),
          GT::RuleReference.new(name: "assignment_expression", is_token: false),
        ]),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 572,
    ),
    GT::GrammarRule.new(
      name: "property_name",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::RuleReference.new(name: "assignment_expression", is_token: false),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
      ]),
      line_number: 581,
    ),
    GT::GrammarRule.new(
      name: "function_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "function"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "function_body", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 586,
    ),
    GT::GrammarRule.new(
      name: "generator_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "function"),
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "function_body", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 590,
    ),
    GT::GrammarRule.new(
      name: "ts_class_expression",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Literal.new(value: "abstract")),
        GT::Literal.new(value: "class"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "ts_class_heritage", is_token: false)),
        GT::RuleReference.new(name: "ts_class_body", is_token: false),
      ]),
      line_number: 596,
    ),
    GT::GrammarRule.new(
      name: "type_expression",
      body: GT::RuleReference.new(name: "conditional_type", is_token: false),
      line_number: 622,
    ),
    GT::GrammarRule.new(
      name: "conditional_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "union_type", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "extends"),
            GT::RuleReference.new(name: "type_expression", is_token: false),
            GT::RuleReference.new(name: "QUESTION", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
      ]),
      line_number: 650,
    ),
    GT::GrammarRule.new(
      name: "union_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "intersection_type", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "PIPE", is_token: true),
            GT::RuleReference.new(name: "intersection_type", is_token: false),
          ])),
      ]),
      line_number: 655,
    ),
    GT::GrammarRule.new(
      name: "intersection_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "array_type", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "AMPERSAND", is_token: true),
            GT::RuleReference.new(name: "array_type", is_token: false),
          ])),
      ]),
      line_number: 660,
    ),
    GT::GrammarRule.new(
      name: "array_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "primary_type", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LBRACKET", is_token: true),
            GT::RuleReference.new(name: "RBRACKET", is_token: true),
          ])),
      ]),
      line_number: 665,
    ),
    GT::GrammarRule.new(
      name: "primary_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "predefined_type", is_token: false),
        GT::RuleReference.new(name: "type_reference", is_token: false),
        GT::RuleReference.new(name: "literal_type", is_token: false),
        GT::RuleReference.new(name: "object_type", is_token: false),
        GT::RuleReference.new(name: "tuple_type", is_token: false),
        GT::RuleReference.new(name: "mapped_type", is_token: false),
        GT::RuleReference.new(name: "function_type", is_token: false),
        GT::RuleReference.new(name: "constructor_type", is_token: false),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "typeof"),
          GT::RuleReference.new(name: "left_hand_side_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "keyof"),
          GT::RuleReference.new(name: "type_expression", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "unique"),
          GT::Literal.new(value: "symbol"),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "infer"),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "type_expression", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 670,
    ),
    GT::GrammarRule.new(
      name: "predefined_type",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "any"),
        GT::Literal.new(value: "string"),
        GT::Literal.new(value: "number"),
        GT::Literal.new(value: "boolean"),
        GT::Literal.new(value: "void"),
        GT::Literal.new(value: "object"),
        GT::Literal.new(value: "symbol"),
        GT::Literal.new(value: "undefined"),
        GT::Literal.new(value: "null"),
        GT::Literal.new(value: "never"),
      ]),
      line_number: 714,
    ),
    GT::GrammarRule.new(
      name: "literal_type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::Literal.new(value: "true"),
        GT::Literal.new(value: "false"),
      ]),
      line_number: 719,
    ),
    GT::GrammarRule.new(
      name: "type_reference",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "DOT", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_arguments", is_token: false)),
      ]),
      line_number: 723,
    ),
    GT::GrammarRule.new(
      name: "type_arguments",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LESS_THAN", is_token: true),
        GT::RuleReference.new(name: "type_argument_list", is_token: false),
        GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
      ]),
      line_number: 725,
    ),
    GT::GrammarRule.new(
      name: "type_argument_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
      ]),
      line_number: 726,
    ),
    GT::GrammarRule.new(
      name: "type_parameters",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LESS_THAN", is_token: true),
        GT::RuleReference.new(name: "type_parameter_list", is_token: false),
        GT::RuleReference.new(name: "GREATER_THAN", is_token: true),
      ]),
      line_number: 730,
    ),
    GT::GrammarRule.new(
      name: "type_parameter_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type_parameter", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "type_parameter", is_token: false),
          ])),
      ]),
      line_number: 731,
    ),
    GT::GrammarRule.new(
      name: "type_parameter",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "extends"),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
      ]),
      line_number: 732,
    ),
    GT::GrammarRule.new(
      name: "object_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "type_member_semicolon", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 736,
    ),
    GT::GrammarRule.new(
      name: "type_member_semicolon",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type_member", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 737,
    ),
    GT::GrammarRule.new(
      name: "type_member",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "construct_signature", is_token: false),
        GT::RuleReference.new(name: "call_signature", is_token: false),
        GT::RuleReference.new(name: "index_signature", is_token: false),
        GT::RuleReference.new(name: "method_signature", is_token: false),
        GT::RuleReference.new(name: "property_signature", is_token: false),
      ]),
      line_number: 738,
    ),
    GT::GrammarRule.new(
      name: "property_signature",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Literal.new(value: "readonly")),
        GT::RuleReference.new(name: "property_name", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "QUESTION", is_token: true)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
      ]),
      line_number: 744,
    ),
    GT::GrammarRule.new(
      name: "index_signature",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "type_expression", is_token: false),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "type_expression", is_token: false),
      ]),
      line_number: 745,
    ),
    GT::GrammarRule.new(
      name: "method_signature",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "property_name", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "QUESTION", is_token: true)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
      ]),
      line_number: 746,
    ),
    GT::GrammarRule.new(
      name: "call_signature",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
      ]),
      line_number: 747,
    ),
    GT::GrammarRule.new(
      name: "construct_signature",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "new"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
      ]),
      line_number: 748,
    ),
    GT::GrammarRule.new(
      name: "mapped_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "readonly_modifier", is_token: false)),
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "in"),
        GT::RuleReference.new(name: "type_expression", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "as"),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "question_modifier", is_token: false)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 790,
    ),
    GT::GrammarRule.new(
      name: "readonly_modifier",
      body: GT::Alternation.new(choices: [
        GT::Literal.new(value: "readonly"),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "PLUS", is_token: true),
          GT::Literal.new(value: "readonly"),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "MINUS", is_token: true),
          GT::Literal.new(value: "readonly"),
        ]),
      ]),
      line_number: 793,
    ),
    GT::GrammarRule.new(
      name: "question_modifier",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "QUESTION", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "PLUS", is_token: true),
          GT::RuleReference.new(name: "QUESTION", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "MINUS", is_token: true),
          GT::RuleReference.new(name: "QUESTION", is_token: true),
        ]),
      ]),
      line_number: 798,
    ),
    GT::GrammarRule.new(
      name: "tuple_type",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "tuple_element_list", is_token: false)),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 804,
    ),
    GT::GrammarRule.new(
      name: "tuple_element_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "tuple_element", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "tuple_element", is_token: false),
          ])),
      ]),
      line_number: 805,
    ),
    GT::GrammarRule.new(
      name: "tuple_element",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "ELLIPSIS", is_token: true)),
        GT::RuleReference.new(name: "type_expression", is_token: false),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "QUESTION", is_token: true)),
      ]),
      line_number: 806,
    ),
    GT::GrammarRule.new(
      name: "function_type",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "ARROW", is_token: true),
        GT::RuleReference.new(name: "type_expression", is_token: false),
      ]),
      line_number: 810,
    ),
    GT::GrammarRule.new(
      name: "constructor_type",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "new"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "ARROW", is_token: true),
        GT::RuleReference.new(name: "type_expression", is_token: false),
      ]),
      line_number: 814,
    ),
    GT::GrammarRule.new(
      name: "interface_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "interface"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "interface_heritage", is_token: false)),
        GT::RuleReference.new(name: "object_type", is_token: false),
      ]),
      line_number: 834,
    ),
    GT::GrammarRule.new(
      name: "interface_heritage",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "extends"),
        GT::RuleReference.new(name: "type_reference", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "type_reference", is_token: false),
          ])),
      ]),
      line_number: 835,
    ),
    GT::GrammarRule.new(
      name: "type_alias_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "type"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "type_expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 850,
    ),
    GT::GrammarRule.new(
      name: "enum_declaration",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Literal.new(value: "const")),
        GT::Literal.new(value: "enum"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "enum_body", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 860,
    ),
    GT::GrammarRule.new(
      name: "enum_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "enum_member", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "enum_member", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
      ]),
      line_number: 861,
    ),
    GT::GrammarRule.new(
      name: "enum_member",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "property_name", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "EQUALS", is_token: true),
            GT::RuleReference.new(name: "assignment_expression", is_token: false),
          ])),
      ]),
      line_number: 862,
    ),
    GT::GrammarRule.new(
      name: "namespace_declaration",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "namespace"),
            GT::Literal.new(value: "module"),
          ])),
        GT::RuleReference.new(name: "qualified_name", is_token: false),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "namespace_element", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 874,
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
      line_number: 875,
    ),
    GT::GrammarRule.new(
      name: "namespace_element",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "namespace_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_declaration", is_token: false),
        GT::RuleReference.new(name: "type_alias_declaration", is_token: false),
        GT::RuleReference.new(name: "ts_class_declaration", is_token: false),
        GT::RuleReference.new(name: "function_declaration", is_token: false),
        GT::RuleReference.new(name: "generator_declaration", is_token: false),
        GT::RuleReference.new(name: "enum_declaration", is_token: false),
        GT::RuleReference.new(name: "lexical_declaration", is_token: false),
        GT::RuleReference.new(name: "variable_statement", is_token: false),
        GT::RuleReference.new(name: "export_assignment", is_token: false),
        GT::RuleReference.new(name: "export_namespace_element", is_token: false),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 877,
    ),
    GT::GrammarRule.new(
      name: "export_assignment",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "export"),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 890,
    ),
    GT::GrammarRule.new(
      name: "export_namespace_element",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "export"),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "namespace_declaration", is_token: false),
            GT::RuleReference.new(name: "interface_declaration", is_token: false),
            GT::RuleReference.new(name: "type_alias_declaration", is_token: false),
            GT::RuleReference.new(name: "ts_class_declaration", is_token: false),
            GT::RuleReference.new(name: "function_declaration", is_token: false),
            GT::RuleReference.new(name: "generator_declaration", is_token: false),
            GT::RuleReference.new(name: "enum_declaration", is_token: false),
            GT::RuleReference.new(name: "lexical_declaration", is_token: false),
            GT::RuleReference.new(name: "variable_statement", is_token: false),
          ])),
      ]),
      line_number: 892,
    ),
    GT::GrammarRule.new(
      name: "ambient_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "declare"),
        GT::RuleReference.new(name: "ambient_declaration_body", is_token: false),
      ]),
      line_number: 912,
    ),
    GT::GrammarRule.new(
      name: "ambient_declaration_body",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "variable_statement", is_token: false),
        GT::RuleReference.new(name: "ambient_function_declaration", is_token: false),
        GT::RuleReference.new(name: "generator_declaration", is_token: false),
        GT::RuleReference.new(name: "ts_class_declaration", is_token: false),
        GT::RuleReference.new(name: "interface_declaration", is_token: false),
        GT::RuleReference.new(name: "type_alias_declaration", is_token: false),
        GT::RuleReference.new(name: "enum_declaration", is_token: false),
        GT::RuleReference.new(name: "namespace_declaration", is_token: false),
        GT::RuleReference.new(name: "ambient_module_declaration", is_token: false),
        GT::RuleReference.new(name: "ambient_global_augmentation", is_token: false),
      ]),
      line_number: 913,
    ),
    GT::GrammarRule.new(
      name: "ambient_module_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "module"),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "namespace_element", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 924,
    ),
    GT::GrammarRule.new(
      name: "ambient_function_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "function"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type_parameters", is_token: false)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "typed_parameter_list", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "type_expression", is_token: false),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 928,
    ),
    GT::GrammarRule.new(
      name: "ambient_global_augmentation",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "global"),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "namespace_element", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 932,
    ),
    GT::GrammarRule.new(
      name: "type_predicate",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Literal.new(value: "is"),
        GT::RuleReference.new(name: "type_expression", is_token: false),
      ]),
      line_number: 955,
    ),
    GT::GrammarRule.new(
      name: "type_annotation",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "type_expression", is_token: false),
      ]),
      line_number: 961,
    ),
  ],
)
