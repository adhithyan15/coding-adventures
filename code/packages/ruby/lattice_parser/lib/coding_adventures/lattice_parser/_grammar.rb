# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: lattice.grammar
# Regenerate with: grammar-tools compile-grammar lattice.grammar
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
      name: "stylesheet",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "rule", is_token: false)),
      line_number: 37,
    ),
    GT::GrammarRule.new(
      name: "rule",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "lattice_rule", is_token: false),
        GT::RuleReference.new(name: "at_rule", is_token: false),
        GT::RuleReference.new(name: "qualified_rule", is_token: false),
      ]),
      line_number: 39,
    ),
    GT::GrammarRule.new(
      name: "lattice_rule",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "variable_declaration", is_token: false),
        GT::RuleReference.new(name: "mixin_definition", is_token: false),
        GT::RuleReference.new(name: "function_definition", is_token: false),
        GT::RuleReference.new(name: "use_directive", is_token: false),
        GT::RuleReference.new(name: "lattice_control", is_token: false),
      ]),
      line_number: 51,
    ),
    GT::GrammarRule.new(
      name: "variable_declaration",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "VARIABLE", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "value_list", is_token: false),
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "BANG_DEFAULT", is_token: true),
            GT::RuleReference.new(name: "BANG_GLOBAL", is_token: true),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 69,
    ),
    GT::GrammarRule.new(
      name: "mixin_definition",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "@mixin"),
          GT::RuleReference.new(name: "FUNCTION", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "mixin_params", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::RuleReference.new(name: "block", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "@mixin"),
          GT::RuleReference.new(name: "IDENT", is_token: true),
          GT::RuleReference.new(name: "block", is_token: false),
        ]),
      ]),
      line_number: 102,
    ),
    GT::GrammarRule.new(
      name: "mixin_params",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "mixin_param", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "mixin_param", is_token: false),
          ])),
      ]),
      line_number: 105,
    ),
    GT::GrammarRule.new(
      name: "mixin_param",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "VARIABLE", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COLON", is_token: true),
            GT::RuleReference.new(name: "mixin_value_list", is_token: false),
          ])),
      ]),
      line_number: 112,
    ),
    GT::GrammarRule.new(
      name: "mixin_value_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "mixin_value", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "mixin_value", is_token: false)),
      ]),
      line_number: 117,
    ),
    GT::GrammarRule.new(
      name: "mixin_value",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "DIMENSION", is_token: true),
        GT::RuleReference.new(name: "PERCENTAGE", is_token: true),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "IDENT", is_token: true),
        GT::RuleReference.new(name: "HASH", is_token: true),
        GT::RuleReference.new(name: "CUSTOM_PROPERTY", is_token: true),
        GT::RuleReference.new(name: "UNICODE_RANGE", is_token: true),
        GT::RuleReference.new(name: "function_call", is_token: false),
        GT::RuleReference.new(name: "VARIABLE", is_token: true),
        GT::RuleReference.new(name: "SLASH", is_token: true),
        GT::RuleReference.new(name: "PLUS", is_token: true),
        GT::RuleReference.new(name: "MINUS", is_token: true),
      ]),
      line_number: 119,
    ),
    GT::GrammarRule.new(
      name: "include_directive",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "@include"),
          GT::RuleReference.new(name: "FUNCTION", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "include_args", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::RuleReference.new(name: "SEMICOLON", is_token: true),
              GT::RuleReference.new(name: "block", is_token: false),
            ])),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "@include"),
          GT::RuleReference.new(name: "IDENT", is_token: true),
          GT::Group.new(element: GT::Alternation.new(choices: [
              GT::RuleReference.new(name: "SEMICOLON", is_token: true),
              GT::RuleReference.new(name: "block", is_token: false),
            ])),
        ]),
      ]),
      line_number: 130,
    ),
    GT::GrammarRule.new(
      name: "include_args",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "include_arg", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "include_arg", is_token: false),
          ])),
      ]),
      line_number: 133,
    ),
    GT::GrammarRule.new(
      name: "include_arg",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "VARIABLE", is_token: true),
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::RuleReference.new(name: "value_list", is_token: false),
        ]),
        GT::RuleReference.new(name: "value_list", is_token: false),
      ]),
      line_number: 137,
    ),
    GT::GrammarRule.new(
      name: "lattice_control",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "if_directive", is_token: false),
        GT::RuleReference.new(name: "for_directive", is_token: false),
        GT::RuleReference.new(name: "each_directive", is_token: false),
        GT::RuleReference.new(name: "while_directive", is_token: false),
      ]),
      line_number: 160,
    ),
    GT::GrammarRule.new(
      name: "if_directive",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "@if"),
        GT::RuleReference.new(name: "lattice_expression", is_token: false),
        GT::RuleReference.new(name: "block", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "@else"),
            GT::Literal.new(value: "if"),
            GT::RuleReference.new(name: "lattice_expression", is_token: false),
            GT::RuleReference.new(name: "block", is_token: false),
          ])),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "@else"),
            GT::RuleReference.new(name: "block", is_token: false),
          ])),
      ]),
      line_number: 164,
    ),
    GT::GrammarRule.new(
      name: "for_directive",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "@for"),
        GT::RuleReference.new(name: "VARIABLE", is_token: true),
        GT::Literal.new(value: "from"),
        GT::RuleReference.new(name: "lattice_expression", is_token: false),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Literal.new(value: "through"),
            GT::Literal.new(value: "to"),
          ])),
        GT::RuleReference.new(name: "lattice_expression", is_token: false),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 171,
    ),
    GT::GrammarRule.new(
      name: "each_directive",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "@each"),
        GT::RuleReference.new(name: "VARIABLE", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "VARIABLE", is_token: true),
          ])),
        GT::Literal.new(value: "in"),
        GT::RuleReference.new(name: "each_list", is_token: false),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 176,
    ),
    GT::GrammarRule.new(
      name: "each_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "value", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "value", is_token: false),
          ])),
      ]),
      line_number: 179,
    ),
    GT::GrammarRule.new(
      name: "while_directive",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "@while"),
        GT::RuleReference.new(name: "lattice_expression", is_token: false),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 184,
    ),
    GT::GrammarRule.new(
      name: "lattice_expression",
      body: GT::RuleReference.new(name: "lattice_or_expr", is_token: false),
      line_number: 203,
    ),
    GT::GrammarRule.new(
      name: "lattice_or_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "lattice_and_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "or"),
            GT::RuleReference.new(name: "lattice_and_expr", is_token: false),
          ])),
      ]),
      line_number: 205,
    ),
    GT::GrammarRule.new(
      name: "lattice_and_expr",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "lattice_comparison", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "and"),
            GT::RuleReference.new(name: "lattice_comparison", is_token: false),
          ])),
      ]),
      line_number: 207,
    ),
    GT::GrammarRule.new(
      name: "lattice_comparison",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "lattice_additive", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "comparison_op", is_token: false),
            GT::RuleReference.new(name: "lattice_additive", is_token: false),
          ])),
      ]),
      line_number: 209,
    ),
    GT::GrammarRule.new(
      name: "comparison_op",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "EQUALS_EQUALS", is_token: true),
        GT::RuleReference.new(name: "NOT_EQUALS", is_token: true),
        GT::RuleReference.new(name: "GREATER", is_token: true),
        GT::RuleReference.new(name: "GREATER_EQUALS", is_token: true),
        GT::RuleReference.new(name: "LESS", is_token: true),
        GT::RuleReference.new(name: "LESS_EQUALS", is_token: true),
      ]),
      line_number: 211,
    ),
    GT::GrammarRule.new(
      name: "lattice_additive",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "lattice_multiplicative", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "PLUS", is_token: true),
                GT::RuleReference.new(name: "MINUS", is_token: true),
              ])),
            GT::RuleReference.new(name: "lattice_multiplicative", is_token: false),
          ])),
      ]),
      line_number: 214,
    ),
    GT::GrammarRule.new(
      name: "lattice_multiplicative",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "lattice_unary", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::Group.new(element: GT::Alternation.new(choices: [
                GT::RuleReference.new(name: "STAR", is_token: true),
                GT::RuleReference.new(name: "SLASH", is_token: true),
              ])),
            GT::RuleReference.new(name: "lattice_unary", is_token: false),
          ])),
      ]),
      line_number: 219,
    ),
    GT::GrammarRule.new(
      name: "lattice_unary",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "MINUS", is_token: true),
          GT::RuleReference.new(name: "lattice_unary", is_token: false),
        ]),
        GT::RuleReference.new(name: "lattice_primary", is_token: false),
      ]),
      line_number: 221,
    ),
    GT::GrammarRule.new(
      name: "lattice_primary",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "VARIABLE", is_token: true),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "DIMENSION", is_token: true),
        GT::RuleReference.new(name: "PERCENTAGE", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "IDENT", is_token: true),
        GT::RuleReference.new(name: "HASH", is_token: true),
        GT::Literal.new(value: "true"),
        GT::Literal.new(value: "false"),
        GT::Literal.new(value: "null"),
        GT::RuleReference.new(name: "function_call", is_token: false),
        GT::RuleReference.new(name: "map_literal", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "lattice_expression", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 224,
    ),
    GT::GrammarRule.new(
      name: "map_literal",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "map_entry", is_token: false),
        GT::RuleReference.new(name: "COMMA", is_token: true),
        GT::RuleReference.new(name: "map_entry", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "map_entry", is_token: false),
          ])),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 235,
    ),
    GT::GrammarRule.new(
      name: "map_entry",
      body: GT::Sequence.new(elements: [
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "IDENT", is_token: true),
            GT::RuleReference.new(name: "STRING", is_token: true),
          ])),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "lattice_expression", is_token: false),
      ]),
      line_number: 237,
    ),
    GT::GrammarRule.new(
      name: "function_definition",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "@function"),
          GT::RuleReference.new(name: "FUNCTION", is_token: true),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "mixin_params", is_token: false)),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::Literal.new(value: "@function"),
          GT::RuleReference.new(name: "IDENT", is_token: true),
          GT::RuleReference.new(name: "function_body", is_token: false),
        ]),
      ]),
      line_number: 261,
    ),
    GT::GrammarRule.new(
      name: "function_body",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "function_body_item", is_token: false)),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 264,
    ),
    GT::GrammarRule.new(
      name: "function_body_item",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "variable_declaration", is_token: false),
        GT::RuleReference.new(name: "return_directive", is_token: false),
        GT::RuleReference.new(name: "lattice_control", is_token: false),
      ]),
      line_number: 266,
    ),
    GT::GrammarRule.new(
      name: "return_directive",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "@return"),
        GT::RuleReference.new(name: "lattice_expression", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 268,
    ),
    GT::GrammarRule.new(
      name: "use_directive",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "@use"),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::Literal.new(value: "as"),
            GT::RuleReference.new(name: "IDENT", is_token: true),
          ])),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 281,
    ),
    GT::GrammarRule.new(
      name: "at_rule",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "AT_KEYWORD", is_token: true),
        GT::RuleReference.new(name: "at_prelude", is_token: false),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "SEMICOLON", is_token: true),
            GT::RuleReference.new(name: "block", is_token: false),
          ])),
      ]),
      line_number: 294,
    ),
    GT::GrammarRule.new(
      name: "at_prelude",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "at_prelude_token", is_token: false)),
      line_number: 296,
    ),
    GT::GrammarRule.new(
      name: "at_prelude_token",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "IDENT", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "DIMENSION", is_token: true),
        GT::RuleReference.new(name: "PERCENTAGE", is_token: true),
        GT::RuleReference.new(name: "HASH", is_token: true),
        GT::RuleReference.new(name: "CUSTOM_PROPERTY", is_token: true),
        GT::RuleReference.new(name: "UNICODE_RANGE", is_token: true),
        GT::RuleReference.new(name: "VARIABLE", is_token: true),
        GT::RuleReference.new(name: "function_in_prelude", is_token: false),
        GT::RuleReference.new(name: "paren_block", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "COMMA", is_token: true),
        GT::RuleReference.new(name: "SLASH", is_token: true),
        GT::RuleReference.new(name: "DOT", is_token: true),
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::RuleReference.new(name: "PLUS", is_token: true),
        GT::RuleReference.new(name: "MINUS", is_token: true),
        GT::RuleReference.new(name: "GREATER", is_token: true),
        GT::RuleReference.new(name: "TILDE", is_token: true),
        GT::RuleReference.new(name: "PIPE", is_token: true),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "AMPERSAND", is_token: true),
        GT::RuleReference.new(name: "CDO", is_token: true),
        GT::RuleReference.new(name: "CDC", is_token: true),
      ]),
      line_number: 298,
    ),
    GT::GrammarRule.new(
      name: "function_in_prelude",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "FUNCTION", is_token: true),
        GT::RuleReference.new(name: "at_prelude_tokens", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 306,
    ),
    GT::GrammarRule.new(
      name: "paren_block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "at_prelude_tokens", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 307,
    ),
    GT::GrammarRule.new(
      name: "at_prelude_tokens",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "at_prelude_token", is_token: false)),
      line_number: 308,
    ),
    GT::GrammarRule.new(
      name: "qualified_rule",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "selector_list", is_token: false),
        GT::RuleReference.new(name: "block", is_token: false),
      ]),
      line_number: 314,
    ),
    GT::GrammarRule.new(
      name: "selector_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "complex_selector", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "COMMA", is_token: true),
            GT::RuleReference.new(name: "complex_selector", is_token: false),
          ])),
      ]),
      line_number: 320,
    ),
    GT::GrammarRule.new(
      name: "complex_selector",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "compound_selector", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "combinator", is_token: false)),
            GT::RuleReference.new(name: "compound_selector", is_token: false),
          ])),
      ]),
      line_number: 322,
    ),
    GT::GrammarRule.new(
      name: "combinator",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "GREATER", is_token: true),
        GT::RuleReference.new(name: "PLUS", is_token: true),
        GT::RuleReference.new(name: "TILDE", is_token: true),
      ]),
      line_number: 324,
    ),
    GT::GrammarRule.new(
      name: "compound_selector",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "simple_selector", is_token: false),
          GT::Repetition.new(element: GT::RuleReference.new(name: "subclass_selector", is_token: false)),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "subclass_selector", is_token: false),
          GT::Repetition.new(element: GT::RuleReference.new(name: "subclass_selector", is_token: false)),
        ]),
      ]),
      line_number: 326,
    ),
    GT::GrammarRule.new(
      name: "simple_selector",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "IDENT", is_token: true),
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::RuleReference.new(name: "AMPERSAND", is_token: true),
        GT::RuleReference.new(name: "VARIABLE", is_token: true),
        GT::RuleReference.new(name: "PERCENTAGE", is_token: true),
      ]),
      line_number: 331,
    ),
    GT::GrammarRule.new(
      name: "subclass_selector",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "class_selector", is_token: false),
        GT::RuleReference.new(name: "id_selector", is_token: false),
        GT::RuleReference.new(name: "placeholder_selector", is_token: false),
        GT::RuleReference.new(name: "attribute_selector", is_token: false),
        GT::RuleReference.new(name: "pseudo_class", is_token: false),
        GT::RuleReference.new(name: "pseudo_element", is_token: false),
      ]),
      line_number: 334,
    ),
    GT::GrammarRule.new(
      name: "placeholder_selector",
      body: GT::RuleReference.new(name: "PLACEHOLDER", is_token: true),
      line_number: 338,
    ),
    GT::GrammarRule.new(
      name: "class_selector",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "DOT", is_token: true),
        GT::RuleReference.new(name: "IDENT", is_token: true),
      ]),
      line_number: 340,
    ),
    GT::GrammarRule.new(
      name: "id_selector",
      body: GT::RuleReference.new(name: "HASH", is_token: true),
      line_number: 342,
    ),
    GT::GrammarRule.new(
      name: "attribute_selector",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "IDENT", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "attr_matcher", is_token: false),
            GT::RuleReference.new(name: "attr_value", is_token: false),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "IDENT", is_token: true)),
          ])),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 344,
    ),
    GT::GrammarRule.new(
      name: "attr_matcher",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "TILDE_EQUALS", is_token: true),
        GT::RuleReference.new(name: "PIPE_EQUALS", is_token: true),
        GT::RuleReference.new(name: "CARET_EQUALS", is_token: true),
        GT::RuleReference.new(name: "DOLLAR_EQUALS", is_token: true),
        GT::RuleReference.new(name: "STAR_EQUALS", is_token: true),
      ]),
      line_number: 346,
    ),
    GT::GrammarRule.new(
      name: "attr_value",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "IDENT", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
      ]),
      line_number: 349,
    ),
    GT::GrammarRule.new(
      name: "pseudo_class",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::RuleReference.new(name: "FUNCTION", is_token: true),
          GT::RuleReference.new(name: "pseudo_class_args", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::RuleReference.new(name: "IDENT", is_token: true),
        ]),
      ]),
      line_number: 351,
    ),
    GT::GrammarRule.new(
      name: "pseudo_class_args",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "pseudo_class_arg", is_token: false)),
      line_number: 354,
    ),
    GT::GrammarRule.new(
      name: "pseudo_class_arg",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "IDENT", is_token: true),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "DIMENSION", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "HASH", is_token: true),
        GT::RuleReference.new(name: "PLUS", is_token: true),
        GT::RuleReference.new(name: "COMMA", is_token: true),
        GT::RuleReference.new(name: "DOT", is_token: true),
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "AMPERSAND", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "FUNCTION", is_token: true),
          GT::RuleReference.new(name: "pseudo_class_args", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::RuleReference.new(name: "pseudo_class_args", is_token: false),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
      ]),
      line_number: 356,
    ),
    GT::GrammarRule.new(
      name: "pseudo_element",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "COLON_COLON", is_token: true),
        GT::RuleReference.new(name: "IDENT", is_token: true),
      ]),
      line_number: 361,
    ),
    GT::GrammarRule.new(
      name: "block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::RuleReference.new(name: "block_contents", is_token: false),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 371,
    ),
    GT::GrammarRule.new(
      name: "block_contents",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "block_item", is_token: false)),
      line_number: 373,
    ),
    GT::GrammarRule.new(
      name: "block_item",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "lattice_block_item", is_token: false),
        GT::RuleReference.new(name: "at_rule", is_token: false),
        GT::RuleReference.new(name: "declaration_or_nested", is_token: false),
      ]),
      line_number: 375,
    ),
    GT::GrammarRule.new(
      name: "lattice_block_item",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "variable_declaration", is_token: false),
        GT::RuleReference.new(name: "include_directive", is_token: false),
        GT::RuleReference.new(name: "lattice_control", is_token: false),
        GT::RuleReference.new(name: "content_directive", is_token: false),
        GT::RuleReference.new(name: "extend_directive", is_token: false),
        GT::RuleReference.new(name: "at_root_directive", is_token: false),
      ]),
      line_number: 381,
    ),
    GT::GrammarRule.new(
      name: "content_directive",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "@content"),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 391,
    ),
    GT::GrammarRule.new(
      name: "extend_directive",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "@extend"),
        GT::RuleReference.new(name: "selector_list", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 399,
    ),
    GT::GrammarRule.new(
      name: "at_root_directive",
      body: GT::Sequence.new(elements: [
        GT::Literal.new(value: "@at-root"),
        GT::Group.new(element: GT::Alternation.new(choices: [
            GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "selector_list", is_token: false),
              GT::RuleReference.new(name: "block", is_token: false),
            ]),
            GT::RuleReference.new(name: "block", is_token: false),
          ])),
      ]),
      line_number: 404,
    ),
    GT::GrammarRule.new(
      name: "declaration_or_nested",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "declaration", is_token: false),
        GT::RuleReference.new(name: "qualified_rule", is_token: false),
      ]),
      line_number: 406,
    ),
    GT::GrammarRule.new(
      name: "declaration",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "property", is_token: false),
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::RuleReference.new(name: "value_list", is_token: false),
          GT::OptionalElement.new(element: GT::RuleReference.new(name: "priority", is_token: false)),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "property", is_token: false),
          GT::RuleReference.new(name: "COLON", is_token: true),
          GT::RuleReference.new(name: "block", is_token: false),
        ]),
      ]),
      line_number: 415,
    ),
    GT::GrammarRule.new(
      name: "property",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "IDENT", is_token: true),
        GT::RuleReference.new(name: "CUSTOM_PROPERTY", is_token: true),
      ]),
      line_number: 418,
    ),
    GT::GrammarRule.new(
      name: "priority",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "BANG", is_token: true),
        GT::Literal.new(value: "important"),
      ]),
      line_number: 420,
    ),
    GT::GrammarRule.new(
      name: "value_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "value", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "value", is_token: false)),
      ]),
      line_number: 431,
    ),
    GT::GrammarRule.new(
      name: "value",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "DIMENSION", is_token: true),
        GT::RuleReference.new(name: "PERCENTAGE", is_token: true),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "IDENT", is_token: true),
        GT::RuleReference.new(name: "HASH", is_token: true),
        GT::RuleReference.new(name: "CUSTOM_PROPERTY", is_token: true),
        GT::RuleReference.new(name: "UNICODE_RANGE", is_token: true),
        GT::RuleReference.new(name: "function_call", is_token: false),
        GT::RuleReference.new(name: "VARIABLE", is_token: true),
        GT::RuleReference.new(name: "SLASH", is_token: true),
        GT::RuleReference.new(name: "COMMA", is_token: true),
        GT::RuleReference.new(name: "PLUS", is_token: true),
        GT::RuleReference.new(name: "MINUS", is_token: true),
        GT::RuleReference.new(name: "map_literal", is_token: false),
      ]),
      line_number: 433,
    ),
    GT::GrammarRule.new(
      name: "function_call",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "FUNCTION", is_token: true),
          GT::RuleReference.new(name: "function_args", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::RuleReference.new(name: "URL_TOKEN", is_token: true),
      ]),
      line_number: 439,
    ),
    GT::GrammarRule.new(
      name: "function_args",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "function_arg", is_token: false)),
      line_number: 442,
    ),
    GT::GrammarRule.new(
      name: "function_arg",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "DIMENSION", is_token: true),
        GT::RuleReference.new(name: "PERCENTAGE", is_token: true),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "IDENT", is_token: true),
        GT::RuleReference.new(name: "HASH", is_token: true),
        GT::RuleReference.new(name: "CUSTOM_PROPERTY", is_token: true),
        GT::RuleReference.new(name: "COMMA", is_token: true),
        GT::RuleReference.new(name: "SLASH", is_token: true),
        GT::RuleReference.new(name: "PLUS", is_token: true),
        GT::RuleReference.new(name: "MINUS", is_token: true),
        GT::RuleReference.new(name: "STAR", is_token: true),
        GT::RuleReference.new(name: "VARIABLE", is_token: true),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "FUNCTION", is_token: true),
          GT::RuleReference.new(name: "function_args", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 444,
    ),
  ],
)
