# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: es2019.grammar
# Regenerate with: grammar-tools compile-grammar es2019.grammar
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
      line_number: 31,
    ),
    GT::GrammarRule.new(
      name: "source_element",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "import_declaration", is_token: false),
        GT::RuleReference.new(name: "export_declaration", is_token: false),
        GT::RuleReference.new(name: "function_declaration", is_token: false),
        GT::RuleReference.new(name: "generator_declaration", is_token: false),
        GT::RuleReference.new(name: "async_function_declaration", is_token: false),
        GT::RuleReference.new(name: "async_generator_declaration", is_token: false),
        GT::RuleReference.new(name: "class_declaration", is_token: false),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 33,
    ),
    GT::GrammarRule.new(
      name: "function_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "function"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameters", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "function_body", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 48,
    ),
    GT::GrammarRule.new(
      name: "formal_parameters",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "formal_parameter", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "formal_parameter", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
      ]),
      line_number: 51,
    ),
    GT::GrammarRule.new(
      name: "formal_parameter",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::RuleReference.new(name: "NAME", is_token: true),
              GT::RuleReference.new(name: "binding_pattern", is_token: false),
            ])),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "EQUALS", is_token: true),
              GT::RuleReference.new(name: "assignment_expression", is_token: false),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "ELLIPSIS", is_token: true),
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::RuleReference.new(name: "NAME", is_token: true),
              GT::RuleReference.new(name: "binding_pattern", is_token: false),
            ])),
        ]),
      ]),
      line_number: 53,
    ),
    GT::GrammarRule.new(
      name: "function_body",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "source_element", is_token: false)),
      line_number: 56,
    ),
    GT::GrammarRule.new(
      name: "generator_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "function"),
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameters", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "function_body", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 60,
    ),
    GT::GrammarRule.new(
      name: "generator_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "function"),
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameters", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "function_body", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 63,
    ),
    GT::GrammarRule.new(
      name: "yield_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "yield"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "STAR", is_token: true)),
        GT::RuleReference.new(name: "assignment_expression", is_token: false),
      ]),
      line_number: 66,
    ),
    GT::GrammarRule.new(
      name: "async_function_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "async"),
        GT::Literal.new(value: "function"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameters", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "function_body", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 70,
    ),
    GT::GrammarRule.new(
      name: "async_function_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "async"),
        GT::Literal.new(value: "function"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameters", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "function_body", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 73,
    ),
    GT::GrammarRule.new(
      name: "async_arrow_function",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "async"),
        GT::RuleReference.new(name: "arrow_parameters", is_token: false),
        GT::RuleReference.new(name: "ARROW", is_token: true),
        GT::RuleReference.new(name: "concise_body", is_token: false),
      ]),
      line_number: 76,
    ),
    GT::GrammarRule.new(
      name: "async_method",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "async"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "STAR", is_token: true)),
        GT::RuleReference.new(name: "property_name", is_token: false),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameters", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "function_body", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 78,
    ),
    GT::GrammarRule.new(
      name: "await_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "await"),
        GT::RuleReference.new(name: "unary_expression", is_token: false),
      ]),
      line_number: 81,
    ),
    GT::GrammarRule.new(
      name: "async_generator_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "async"),
        GT::Literal.new(value: "function"),
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameters", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "function_body", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 85,
    ),
    GT::GrammarRule.new(
      name: "async_generator_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "async"),
        GT::Literal.new(value: "function"),
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameters", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "function_body", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 89,
    ),
    GT::GrammarRule.new(
      name: "lexical_declaration",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "let"),
            GT::Literal.new(value: "const"),
          ])),
        GT::RuleReference.new(name: "binding_list", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 95,
    ),
    GT::GrammarRule.new(
      name: "binding_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "lexical_binding", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "lexical_binding", is_token: false),
          ])),
      ]),
      line_number: 97,
    ),
    GT::GrammarRule.new(
      name: "lexical_binding",
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
      line_number: 99,
    ),
    GT::GrammarRule.new(
      name: "class_declaration",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "class"),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "class_heritage", is_token: false)),
        GT::RuleReference.new(name: "class_body", is_token: false),
      ]),
      line_number: 103,
    ),
    GT::GrammarRule.new(
      name: "class_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "class"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "class_heritage", is_token: false)),
        GT::RuleReference.new(name: "class_body", is_token: false),
      ]),
      line_number: 105,
    ),
    GT::GrammarRule.new(
      name: "class_heritage",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "extends"),
        GT::RuleReference.new(name: "left_hand_side_expression", is_token: false),
      ]),
      line_number: 107,
    ),
    GT::GrammarRule.new(
      name: "class_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "class_element", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 109,
    ),
    GT::GrammarRule.new(
      name: "class_element",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::OptionalElement.new(element: GT::Literal.new(value: "static")),
          GT::RuleReference.new(name: "method_definition", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::OptionalElement.new(element: GT::Literal.new(value: "static")),
          GT::RuleReference.new(name: "async_method", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 111,
    ),
    GT::GrammarRule.new(
      name: "method_definition",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameters", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "get"),
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "set"),
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "formal_parameter", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "STAR", is_token: true),
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameters", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
      ]),
      line_number: 115,
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
          GT::RuleReference.new(name: "module_specifier", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
      ]),
      line_number: 126,
    ),
    GT::GrammarRule.new(
      name: "import_clause",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "default_import", is_token: false),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "named_imports", is_token: false),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "default_import", is_token: false),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "COMMA", is_token: true),
              GT::RuleReference.new(name: "namespace_import", is_token: false),
            ])),
        ]),
        GT::RuleReference.new(name: "named_imports", is_token: false),
        GT::RuleReference.new(name: "namespace_import", is_token: false),
      ]),
      line_number: 129,
    ),
    GT::GrammarRule.new(
      name: "default_import",
      body: GT::RuleReference.new(name: "NAME", is_token: true),
      line_number: 134,
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
      line_number: 136,
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
      line_number: 138,
    ),
    GT::GrammarRule.new(
      name: "namespace_import",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::Literal.new(value: "as"),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 140,
    ),
    GT::GrammarRule.new(
      name: "from_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "from"),
        GT::RuleReference.new(name: "STRING", is_token: true),
      ]),
      line_number: 142,
    ),
    GT::GrammarRule.new(
      name: "module_specifier",
      body: GT::RuleReference.new(name: "STRING", is_token: true),
      line_number: 144,
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
              GT::RuleReference.new(name: "async_function_declaration", is_token: false),
              GT::RuleReference.new(name: "async_generator_declaration", is_token: false),
              GT::RuleReference.new(name: "class_declaration", is_token: false),
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
              GT::RuleReference.new(name: "async_function_declaration", is_token: false),
              GT::RuleReference.new(name: "async_generator_declaration", is_token: false),
              GT::RuleReference.new(name: "class_declaration", is_token: false),
              GT::RuleReference.new(name: "lexical_declaration", is_token: false),
              GT::RuleReference.new(name: "variable_statement", is_token: false),
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
          GT::RuleReference.new(name: "STAR", is_token: true),
          GT::RuleReference.new(name: "from_clause", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
      ]),
      line_number: 146,
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
      line_number: 159,
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
      line_number: 161,
    ),
    GT::GrammarRule.new(
      name: "binding_pattern",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "object_binding_pattern", is_token: false),
        GT::RuleReference.new(name: "array_binding_pattern", is_token: false),
      ]),
      line_number: 167,
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
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "object_rest_property", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 170,
    ),
    GT::GrammarRule.new(
      name: "object_rest_property",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "ELLIPSIS", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
      ]),
      line_number: 173,
    ),
    GT::GrammarRule.new(
      name: "array_binding_pattern",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "binding_element", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "binding_element", is_token: false),
              ])),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
          ])),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 175,
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
      line_number: 177,
    ),
    GT::GrammarRule.new(
      name: "binding_element",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::RuleReference.new(name: "NAME", is_token: true),
              GT::RuleReference.new(name: "binding_pattern", is_token: false),
            ])),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "EQUALS", is_token: true),
              GT::RuleReference.new(name: "assignment_expression", is_token: false),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "ELLIPSIS", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ]),
      ]),
      line_number: 180,
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
        GT::RuleReference.new(name: "for_await_of_statement", is_token: false),
        GT::RuleReference.new(name: "continue_statement", is_token: false),
        GT::RuleReference.new(name: "break_statement", is_token: false),
        GT::RuleReference.new(name: "return_statement", is_token: false),
        GT::RuleReference.new(name: "with_statement", is_token: false),
        GT::RuleReference.new(name: "switch_statement", is_token: false),
        GT::RuleReference.new(name: "labelled_statement", is_token: false),
        GT::RuleReference.new(name: "try_statement", is_token: false),
        GT::RuleReference.new(name: "throw_statement", is_token: false),
        GT::RuleReference.new(name: "debugger_statement", is_token: false),
        GT::RuleReference.new(name: "lexical_declaration", is_token: false),
        GT::RuleReference.new(name: "expression_statement", is_token: false),
      ]),
      line_number: 187,
    ),
    GT::GrammarRule.new(
      name: "block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 209,
    ),
    GT::GrammarRule.new(
      name: "variable_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "var"),
        GT::RuleReference.new(name: "variable_declaration_list", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 211,
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
      line_number: 213,
    ),
    GT::GrammarRule.new(
      name: "variable_declaration",
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
      line_number: 215,
    ),
    GT::GrammarRule.new(
      name: "empty_statement",
      body: GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      line_number: 217,
    ),
    GT::GrammarRule.new(
      name: "expression_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 219,
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
      line_number: 221,
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
      line_number: 223,
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
      line_number: 225,
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
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "let"),
              GT::RuleReference.new(name: "binding_list", is_token: false),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "const"),
              GT::RuleReference.new(name: "binding_list", is_token: false),
            ]),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 227,
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
              GT::RuleReference.new(name: "binding_element", is_token: false),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "const"),
              GT::RuleReference.new(name: "binding_element", is_token: false),
            ]),
            GT::RuleReference.new(name: "left_hand_side_expression", is_token: false),
          ])),
        GT::Literal.new(value: "in"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 236,
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
              GT::RuleReference.new(name: "binding_element", is_token: false),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "const"),
              GT::RuleReference.new(name: "binding_element", is_token: false),
            ]),
            GT::RuleReference.new(name: "left_hand_side_expression", is_token: false),
          ])),
        GT::Literal.new(value: "of"),
        GT::RuleReference.new(name: "assignment_expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 243,
    ),
    GT::GrammarRule.new(
      name: "for_await_of_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "for"),
        GT::Literal.new(value: "await"),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "var"),
              GT::RuleReference.new(name: "variable_declaration", is_token: false),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "let"),
              GT::RuleReference.new(name: "binding_element", is_token: false),
            ]),
            GT::Sequence.new(elements: [
              GT::Literal.new(value: "const"),
              GT::RuleReference.new(name: "binding_element", is_token: false),
            ]),
            GT::RuleReference.new(name: "left_hand_side_expression", is_token: false),
          ])),
        GT::Literal.new(value: "of"),
        GT::RuleReference.new(name: "assignment_expression", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 250,
    ),
    GT::GrammarRule.new(
      name: "continue_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "continue"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 257,
    ),
    GT::GrammarRule.new(
      name: "break_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "break"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 259,
    ),
    GT::GrammarRule.new(
      name: "return_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "return"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "expression", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 261,
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
      line_number: 263,
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
      line_number: 265,
    ),
    GT::GrammarRule.new(
      name: "case_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "case"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
      ]),
      line_number: 268,
    ),
    GT::GrammarRule.new(
      name: "default_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "default"),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "statement", is_token: false)),
      ]),
      line_number: 270,
    ),
    GT::GrammarRule.new(
      name: "labelled_statement",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 272,
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
      line_number: 274,
    ),
    GT::GrammarRule.new(
      name: "catch_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "catch"),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "LPAREN", is_token: true),
            GT::RuleReference.new(name: "NAME", is_token: true),
            GT::RuleReference.new(name: "RPAREN", is_token: true),
          ])),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 284,
    ),
    GT::GrammarRule.new(
      name: "finally_clause",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "finally"),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 286,
    ),
    GT::GrammarRule.new(
      name: "throw_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "throw"),
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 288,
    ),
    GT::GrammarRule.new(
      name: "debugger_statement",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "debugger"),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 290,
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
      line_number: 296,
    ),
    GT::GrammarRule.new(
      name: "assignment_expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "conditional_expression", is_token: false),
        GT::RuleReference.new(name: "arrow_function", is_token: false),
        GT::RuleReference.new(name: "async_arrow_function", is_token: false),
        GT::RuleReference.new(name: "yield_expression", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "left_hand_side_expression", is_token: false),
          GT::RuleReference.new(name: "assignment_operator", is_token: false),
          GT::RuleReference.new(name: "assignment_expression", is_token: false),
        ]),
      ]),
      line_number: 298,
    ),
    GT::GrammarRule.new(
      name: "assignment_operator",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "PLUS_EQUALS", is_token: true),
        GT::RuleReference.new(name: "MINUS_EQUALS", is_token: true),
        GT::RuleReference.new(name: "STAR_STAR_EQUALS", is_token: true),
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
      line_number: 304,
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
      line_number: 310,
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
      line_number: 313,
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
      line_number: 315,
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
      line_number: 317,
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
      line_number: 319,
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
      line_number: 321,
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
      line_number: 323,
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
      line_number: 327,
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
      line_number: 331,
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
      line_number: 334,
    ),
    GT::GrammarRule.new(
      name: "multiplicative_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "exponentiation_expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "STAR", is_token: true),
                GT::RuleReference.new(name: "SLASH", is_token: true),
                GT::RuleReference.new(name: "PERCENT", is_token: true),
              ])),
            GT::RuleReference.new(name: "exponentiation_expression", is_token: false),
          ])),
      ]),
      line_number: 337,
    ),
    GT::GrammarRule.new(
      name: "exponentiation_expression",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "unary_expression", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "STAR_STAR", is_token: true),
            GT::RuleReference.new(name: "exponentiation_expression", is_token: false),
          ])),
      ]),
      line_number: 340,
    ),
    GT::GrammarRule.new(
      name: "unary_expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "postfix_expression", is_token: false),
        GT::RuleReference.new(name: "await_expression", is_token: false),
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
      line_number: 342,
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
      line_number: 354,
    ),
    GT::GrammarRule.new(
      name: "left_hand_side_expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "call_expression", is_token: false),
        GT::RuleReference.new(name: "new_expression", is_token: false),
      ]),
      line_number: 356,
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
      line_number: 358,
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
      line_number: 362,
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
          GT::Literal.new(value: "new"),
          GT::RuleReference.new(name: "member_expression", is_token: false),
          GT::RuleReference.new(name: "arguments", is_token: false),
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
          GT::RuleReference.new(name: "DOT", is_token: true),
          GT::Literal.new(value: "target"),
        ]),
      ]),
      line_number: 365,
    ),
    GT::GrammarRule.new(
      name: "arguments",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "argument_list", is_token: false),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
          ])),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 372,
    ),
    GT::GrammarRule.new(
      name: "argument_list",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "spread_element", is_token: false),
            GT::RuleReference.new(name: "assignment_expression", is_token: false),
          ])),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "spread_element", is_token: false),
                GT::RuleReference.new(name: "assignment_expression", is_token: false),
              ])),
          ])),
      ]),
      line_number: 374,
    ),
    GT::GrammarRule.new(
      name: "spread_element",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "ELLIPSIS", is_token: true),
        GT::RuleReference.new(name: "assignment_expression", is_token: false),
      ]),
      line_number: 377,
    ),
    GT::GrammarRule.new(
      name: "arrow_function",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "arrow_parameters", is_token: false),
        GT::RuleReference.new(name: "ARROW", is_token: true),
        GT::RuleReference.new(name: "concise_body", is_token: false),
      ]),
      line_number: 379,
    ),
    GT::GrammarRule.new(
      name: "arrow_parameters",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameters", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 381,
    ),
    GT::GrammarRule.new(
      name: "concise_body",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "assignment_expression", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LBRACE", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
          GT::RuleReference.new(name: "RBRACE", is_token: true),
        ]),
      ]),
      line_number: 384,
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
        GT::RuleReference.new(name: "array_literal", is_token: false),
        GT::RuleReference.new(name: "object_literal", is_token: false),
        GT::RuleReference.new(name: "function_expression", is_token: false),
        GT::RuleReference.new(name: "generator_expression", is_token: false),
        GT::RuleReference.new(name: "async_function_expression", is_token: false),
        GT::RuleReference.new(name: "class_expression", is_token: false),
        GT::RuleReference.new(name: "template_literal", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 387,
    ),
    GT::GrammarRule.new(
      name: "array_literal",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "element_list", is_token: false)),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 404,
    ),
    GT::GrammarRule.new(
      name: "element_list",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "spread_element", is_token: false),
            GT::RuleReference.new(name: "assignment_expression", is_token: false),
          ])),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::OptionalElement.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "spread_element", is_token: false),
                GT::RuleReference.new(name: "assignment_expression", is_token: false),
              ])),
          ])),
      ]),
      line_number: 406,
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
      line_number: 409,
    ),
    GT::GrammarRule.new(
      name: "property_definition",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "property_name", is_token: false),
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::RuleReference.new(name: "assignment_expression", is_token: false),
        ]),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "method_definition", is_token: false),
        GT::RuleReference.new(name: "async_method", is_token: false),
        GT::RuleReference.new(name: "object_spread_property", is_token: false),
      ]),
      line_number: 411,
    ),
    GT::GrammarRule.new(
      name: "object_spread_property",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "ELLIPSIS", is_token: true),
        GT::RuleReference.new(name: "assignment_expression", is_token: false),
      ]),
      line_number: 417,
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
      line_number: 419,
    ),
    GT::GrammarRule.new(
      name: "function_expression",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "function"),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "NAME", is_token: true)),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_parameters", is_token: false)),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "function_body", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 424,
    ),
    GT::GrammarRule.new(
      name: "template_literal",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "TEMPLATE_NO_SUB", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "TEMPLATE_HEAD", is_token: true),
          GT::Repetition.new(element: GT::RuleReference.new(name: "template_span", is_token: false)),
          GT::RuleReference.new(name: "TEMPLATE_TAIL", is_token: true),
        ]),
      ]),
      line_number: 427,
    ),
    GT::GrammarRule.new(
      name: "template_span",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::RuleReference.new(name: "TEMPLATE_MIDDLE", is_token: true),
      ]),
      line_number: 430,
    ),
  ],
)
