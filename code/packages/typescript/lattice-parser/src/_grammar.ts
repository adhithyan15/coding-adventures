// AUTO-GENERATED FILE — DO NOT EDIT
// Source: lattice.grammar
// Regenerate with: grammar-tools compile-grammar lattice.grammar
//
// This file embeds a ParserGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .grammar file at
// runtime.

import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const PARSER_GRAMMAR: ParserGrammar = {
  version: 1,
  rules: [
  {
    name: "stylesheet",
    body: { type: "repetition", element: { type: "rule_reference", name: "rule" } },
    lineNumber: 37,
  },
  {
    name: "rule",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "lattice_rule" },
      { type: "rule_reference", name: "at_rule" },
      { type: "rule_reference", name: "qualified_rule" },
    ] },
    lineNumber: 39,
  },
  {
    name: "lattice_rule",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "variable_declaration" },
      { type: "rule_reference", name: "mixin_definition" },
      { type: "rule_reference", name: "function_definition" },
      { type: "rule_reference", name: "use_directive" },
      { type: "rule_reference", name: "lattice_control" },
    ] },
    lineNumber: 51,
  },
  {
    name: "variable_declaration",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "VARIABLE" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "value_list" },
      { type: "optional", element: { type: "alternation", choices: [
          { type: "token_reference", name: "BANG_DEFAULT" },
          { type: "token_reference", name: "BANG_GLOBAL" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 69,
  },
  {
    name: "mixin_definition",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "@mixin" },
        { type: "token_reference", name: "FUNCTION" },
        { type: "optional", element: { type: "rule_reference", name: "mixin_params" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "rule_reference", name: "block" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "@mixin" },
        { type: "token_reference", name: "IDENT" },
        { type: "rule_reference", name: "block" },
      ] },
    ] },
    lineNumber: 102,
  },
  {
    name: "mixin_params",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "mixin_param" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "mixin_param" },
        ] } },
    ] },
    lineNumber: 105,
  },
  {
    name: "mixin_param",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "VARIABLE" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "mixin_value_list" },
        ] } },
    ] },
    lineNumber: 112,
  },
  {
    name: "mixin_value_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "mixin_value" },
      { type: "repetition", element: { type: "rule_reference", name: "mixin_value" } },
    ] },
    lineNumber: 117,
  },
  {
    name: "mixin_value",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "DIMENSION" },
      { type: "token_reference", name: "PERCENTAGE" },
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "IDENT" },
      { type: "token_reference", name: "HASH" },
      { type: "token_reference", name: "CUSTOM_PROPERTY" },
      { type: "token_reference", name: "UNICODE_RANGE" },
      { type: "rule_reference", name: "function_call" },
      { type: "token_reference", name: "VARIABLE" },
      { type: "token_reference", name: "SLASH" },
      { type: "token_reference", name: "PLUS" },
      { type: "token_reference", name: "MINUS" },
    ] },
    lineNumber: 119,
  },
  {
    name: "include_directive",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "@include" },
        { type: "token_reference", name: "FUNCTION" },
        { type: "optional", element: { type: "rule_reference", name: "include_args" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "group", element: { type: "alternation", choices: [
            { type: "token_reference", name: "SEMICOLON" },
            { type: "rule_reference", name: "block" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "@include" },
        { type: "token_reference", name: "IDENT" },
        { type: "group", element: { type: "alternation", choices: [
            { type: "token_reference", name: "SEMICOLON" },
            { type: "rule_reference", name: "block" },
          ] } },
      ] },
    ] },
    lineNumber: 130,
  },
  {
    name: "include_args",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "include_arg" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "include_arg" },
        ] } },
    ] },
    lineNumber: 133,
  },
  {
    name: "include_arg",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "VARIABLE" },
        { type: "token_reference", name: "COLON" },
        { type: "rule_reference", name: "value_list" },
      ] },
      { type: "rule_reference", name: "value_list" },
    ] },
    lineNumber: 137,
  },
  {
    name: "lattice_control",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "if_directive" },
      { type: "rule_reference", name: "for_directive" },
      { type: "rule_reference", name: "each_directive" },
      { type: "rule_reference", name: "while_directive" },
    ] },
    lineNumber: 160,
  },
  {
    name: "if_directive",
    body: { type: "sequence", elements: [
      { type: "literal", value: "@if" },
      { type: "rule_reference", name: "lattice_expression" },
      { type: "rule_reference", name: "block" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "@else" },
          { type: "literal", value: "if" },
          { type: "rule_reference", name: "lattice_expression" },
          { type: "rule_reference", name: "block" },
        ] } },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "@else" },
          { type: "rule_reference", name: "block" },
        ] } },
    ] },
    lineNumber: 164,
  },
  {
    name: "for_directive",
    body: { type: "sequence", elements: [
      { type: "literal", value: "@for" },
      { type: "token_reference", name: "VARIABLE" },
      { type: "literal", value: "from" },
      { type: "rule_reference", name: "lattice_expression" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "literal", value: "through" },
          { type: "literal", value: "to" },
        ] } },
      { type: "rule_reference", name: "lattice_expression" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 171,
  },
  {
    name: "each_directive",
    body: { type: "sequence", elements: [
      { type: "literal", value: "@each" },
      { type: "token_reference", name: "VARIABLE" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "token_reference", name: "VARIABLE" },
        ] } },
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "each_list" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 176,
  },
  {
    name: "each_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "value" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "value" },
        ] } },
    ] },
    lineNumber: 179,
  },
  {
    name: "while_directive",
    body: { type: "sequence", elements: [
      { type: "literal", value: "@while" },
      { type: "rule_reference", name: "lattice_expression" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 184,
  },
  {
    name: "lattice_expression",
    body: { type: "rule_reference", name: "lattice_or_expr" },
    lineNumber: 203,
  },
  {
    name: "lattice_or_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "lattice_and_expr" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "or" },
          { type: "rule_reference", name: "lattice_and_expr" },
        ] } },
    ] },
    lineNumber: 205,
  },
  {
    name: "lattice_and_expr",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "lattice_comparison" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "literal", value: "and" },
          { type: "rule_reference", name: "lattice_comparison" },
        ] } },
    ] },
    lineNumber: 207,
  },
  {
    name: "lattice_comparison",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "lattice_additive" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "comparison_op" },
          { type: "rule_reference", name: "lattice_additive" },
        ] } },
    ] },
    lineNumber: 209,
  },
  {
    name: "comparison_op",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "EQUALS_EQUALS" },
      { type: "token_reference", name: "NOT_EQUALS" },
      { type: "token_reference", name: "GREATER" },
      { type: "token_reference", name: "GREATER_EQUALS" },
      { type: "token_reference", name: "LESS" },
      { type: "token_reference", name: "LESS_EQUALS" },
    ] },
    lineNumber: 211,
  },
  {
    name: "lattice_additive",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "lattice_multiplicative" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "PLUS" },
              { type: "token_reference", name: "MINUS" },
            ] } },
          { type: "rule_reference", name: "lattice_multiplicative" },
        ] } },
    ] },
    lineNumber: 214,
  },
  {
    name: "lattice_multiplicative",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "lattice_unary" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "STAR" },
              { type: "token_reference", name: "SLASH" },
            ] } },
          { type: "rule_reference", name: "lattice_unary" },
        ] } },
    ] },
    lineNumber: 219,
  },
  {
    name: "lattice_unary",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "MINUS" },
        { type: "rule_reference", name: "lattice_unary" },
      ] },
      { type: "rule_reference", name: "lattice_primary" },
    ] },
    lineNumber: 221,
  },
  {
    name: "lattice_primary",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "VARIABLE" },
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "DIMENSION" },
      { type: "token_reference", name: "PERCENTAGE" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "IDENT" },
      { type: "token_reference", name: "HASH" },
      { type: "literal", value: "true" },
      { type: "literal", value: "false" },
      { type: "literal", value: "null" },
      { type: "rule_reference", name: "function_call" },
      { type: "rule_reference", name: "map_literal" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "lattice_expression" },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 224,
  },
  {
    name: "map_literal",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "map_entry" },
      { type: "token_reference", name: "COMMA" },
      { type: "rule_reference", name: "map_entry" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "map_entry" },
        ] } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 235,
  },
  {
    name: "map_entry",
    body: { type: "sequence", elements: [
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "IDENT" },
          { type: "token_reference", name: "STRING" },
        ] } },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "lattice_expression" },
    ] },
    lineNumber: 237,
  },
  {
    name: "function_definition",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "@function" },
        { type: "token_reference", name: "FUNCTION" },
        { type: "optional", element: { type: "rule_reference", name: "mixin_params" } },
        { type: "token_reference", name: "RPAREN" },
        { type: "rule_reference", name: "function_body" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "@function" },
        { type: "token_reference", name: "IDENT" },
        { type: "rule_reference", name: "function_body" },
      ] },
    ] },
    lineNumber: 261,
  },
  {
    name: "function_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "function_body_item" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 264,
  },
  {
    name: "function_body_item",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "variable_declaration" },
      { type: "rule_reference", name: "return_directive" },
      { type: "rule_reference", name: "lattice_control" },
    ] },
    lineNumber: 266,
  },
  {
    name: "return_directive",
    body: { type: "sequence", elements: [
      { type: "literal", value: "@return" },
      { type: "rule_reference", name: "lattice_expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 268,
  },
  {
    name: "use_directive",
    body: { type: "sequence", elements: [
      { type: "literal", value: "@use" },
      { type: "token_reference", name: "STRING" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "as" },
          { type: "token_reference", name: "IDENT" },
        ] } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 281,
  },
  {
    name: "at_rule",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "AT_KEYWORD" },
      { type: "rule_reference", name: "at_prelude" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "token_reference", name: "SEMICOLON" },
          { type: "rule_reference", name: "block" },
        ] } },
    ] },
    lineNumber: 294,
  },
  {
    name: "at_prelude",
    body: { type: "repetition", element: { type: "rule_reference", name: "at_prelude_token" } },
    lineNumber: 296,
  },
  {
    name: "at_prelude_token",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "IDENT" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "DIMENSION" },
      { type: "token_reference", name: "PERCENTAGE" },
      { type: "token_reference", name: "HASH" },
      { type: "token_reference", name: "CUSTOM_PROPERTY" },
      { type: "token_reference", name: "UNICODE_RANGE" },
      { type: "token_reference", name: "VARIABLE" },
      { type: "rule_reference", name: "function_in_prelude" },
      { type: "rule_reference", name: "paren_block" },
      { type: "token_reference", name: "COLON" },
      { type: "token_reference", name: "COMMA" },
      { type: "token_reference", name: "SLASH" },
      { type: "token_reference", name: "DOT" },
      { type: "token_reference", name: "STAR" },
      { type: "token_reference", name: "PLUS" },
      { type: "token_reference", name: "MINUS" },
      { type: "token_reference", name: "GREATER" },
      { type: "token_reference", name: "TILDE" },
      { type: "token_reference", name: "PIPE" },
      { type: "token_reference", name: "EQUALS" },
      { type: "token_reference", name: "AMPERSAND" },
      { type: "token_reference", name: "CDO" },
      { type: "token_reference", name: "CDC" },
    ] },
    lineNumber: 298,
  },
  {
    name: "function_in_prelude",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "FUNCTION" },
      { type: "rule_reference", name: "at_prelude_tokens" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 306,
  },
  {
    name: "paren_block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "at_prelude_tokens" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 307,
  },
  {
    name: "at_prelude_tokens",
    body: { type: "repetition", element: { type: "rule_reference", name: "at_prelude_token" } },
    lineNumber: 308,
  },
  {
    name: "qualified_rule",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "selector_list" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 314,
  },
  {
    name: "selector_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "complex_selector" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "complex_selector" },
        ] } },
    ] },
    lineNumber: 320,
  },
  {
    name: "complex_selector",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "compound_selector" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "optional", element: { type: "rule_reference", name: "combinator" } },
          { type: "rule_reference", name: "compound_selector" },
        ] } },
    ] },
    lineNumber: 322,
  },
  {
    name: "combinator",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "GREATER" },
      { type: "token_reference", name: "PLUS" },
      { type: "token_reference", name: "TILDE" },
    ] },
    lineNumber: 324,
  },
  {
    name: "compound_selector",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "simple_selector" },
        { type: "repetition", element: { type: "rule_reference", name: "subclass_selector" } },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "subclass_selector" },
        { type: "repetition", element: { type: "rule_reference", name: "subclass_selector" } },
      ] },
    ] },
    lineNumber: 326,
  },
  {
    name: "simple_selector",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "IDENT" },
      { type: "token_reference", name: "STAR" },
      { type: "token_reference", name: "AMPERSAND" },
      { type: "token_reference", name: "VARIABLE" },
      { type: "token_reference", name: "PERCENTAGE" },
    ] },
    lineNumber: 331,
  },
  {
    name: "subclass_selector",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "class_selector" },
      { type: "rule_reference", name: "id_selector" },
      { type: "rule_reference", name: "placeholder_selector" },
      { type: "rule_reference", name: "attribute_selector" },
      { type: "rule_reference", name: "pseudo_class" },
      { type: "rule_reference", name: "pseudo_element" },
    ] },
    lineNumber: 334,
  },
  {
    name: "placeholder_selector",
    body: { type: "token_reference", name: "PLACEHOLDER" },
    lineNumber: 338,
  },
  {
    name: "class_selector",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "DOT" },
      { type: "token_reference", name: "IDENT" },
    ] },
    lineNumber: 340,
  },
  {
    name: "id_selector",
    body: { type: "token_reference", name: "HASH" },
    lineNumber: 342,
  },
  {
    name: "attribute_selector",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "token_reference", name: "IDENT" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "attr_matcher" },
          { type: "rule_reference", name: "attr_value" },
          { type: "optional", element: { type: "token_reference", name: "IDENT" } },
        ] } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 344,
  },
  {
    name: "attr_matcher",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "EQUALS" },
      { type: "token_reference", name: "TILDE_EQUALS" },
      { type: "token_reference", name: "PIPE_EQUALS" },
      { type: "token_reference", name: "CARET_EQUALS" },
      { type: "token_reference", name: "DOLLAR_EQUALS" },
      { type: "token_reference", name: "STAR_EQUALS" },
    ] },
    lineNumber: 346,
  },
  {
    name: "attr_value",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "IDENT" },
      { type: "token_reference", name: "STRING" },
    ] },
    lineNumber: 349,
  },
  {
    name: "pseudo_class",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "COLON" },
        { type: "token_reference", name: "FUNCTION" },
        { type: "rule_reference", name: "pseudo_class_args" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "COLON" },
        { type: "token_reference", name: "IDENT" },
      ] },
    ] },
    lineNumber: 351,
  },
  {
    name: "pseudo_class_args",
    body: { type: "repetition", element: { type: "rule_reference", name: "pseudo_class_arg" } },
    lineNumber: 354,
  },
  {
    name: "pseudo_class_arg",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "IDENT" },
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "DIMENSION" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "HASH" },
      { type: "token_reference", name: "PLUS" },
      { type: "token_reference", name: "COMMA" },
      { type: "token_reference", name: "DOT" },
      { type: "token_reference", name: "STAR" },
      { type: "token_reference", name: "COLON" },
      { type: "token_reference", name: "AMPERSAND" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "FUNCTION" },
        { type: "rule_reference", name: "pseudo_class_args" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LBRACKET" },
        { type: "rule_reference", name: "pseudo_class_args" },
        { type: "token_reference", name: "RBRACKET" },
      ] },
    ] },
    lineNumber: 356,
  },
  {
    name: "pseudo_element",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "COLON_COLON" },
      { type: "token_reference", name: "IDENT" },
    ] },
    lineNumber: 361,
  },
  {
    name: "block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "block_contents" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 371,
  },
  {
    name: "block_contents",
    body: { type: "repetition", element: { type: "rule_reference", name: "block_item" } },
    lineNumber: 373,
  },
  {
    name: "block_item",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "lattice_block_item" },
      { type: "rule_reference", name: "at_rule" },
      { type: "rule_reference", name: "declaration_or_nested" },
    ] },
    lineNumber: 375,
  },
  {
    name: "lattice_block_item",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "variable_declaration" },
      { type: "rule_reference", name: "include_directive" },
      { type: "rule_reference", name: "lattice_control" },
      { type: "rule_reference", name: "content_directive" },
      { type: "rule_reference", name: "extend_directive" },
      { type: "rule_reference", name: "at_root_directive" },
    ] },
    lineNumber: 381,
  },
  {
    name: "content_directive",
    body: { type: "sequence", elements: [
      { type: "literal", value: "@content" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 391,
  },
  {
    name: "extend_directive",
    body: { type: "sequence", elements: [
      { type: "literal", value: "@extend" },
      { type: "rule_reference", name: "selector_list" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 399,
  },
  {
    name: "at_root_directive",
    body: { type: "sequence", elements: [
      { type: "literal", value: "@at-root" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "rule_reference", name: "selector_list" },
            { type: "rule_reference", name: "block" },
          ] },
          { type: "rule_reference", name: "block" },
        ] } },
    ] },
    lineNumber: 404,
  },
  {
    name: "declaration_or_nested",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "declaration" },
      { type: "rule_reference", name: "qualified_rule" },
    ] },
    lineNumber: 406,
  },
  {
    name: "declaration",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "property" },
        { type: "token_reference", name: "COLON" },
        { type: "rule_reference", name: "value_list" },
        { type: "optional", element: { type: "rule_reference", name: "priority" } },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "property" },
        { type: "token_reference", name: "COLON" },
        { type: "rule_reference", name: "block" },
      ] },
    ] },
    lineNumber: 415,
  },
  {
    name: "property",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "IDENT" },
      { type: "token_reference", name: "CUSTOM_PROPERTY" },
    ] },
    lineNumber: 418,
  },
  {
    name: "priority",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "BANG" },
      { type: "literal", value: "important" },
    ] },
    lineNumber: 420,
  },
  {
    name: "value_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "value" },
      { type: "repetition", element: { type: "rule_reference", name: "value" } },
    ] },
    lineNumber: 431,
  },
  {
    name: "value",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "DIMENSION" },
      { type: "token_reference", name: "PERCENTAGE" },
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "IDENT" },
      { type: "token_reference", name: "HASH" },
      { type: "token_reference", name: "CUSTOM_PROPERTY" },
      { type: "token_reference", name: "UNICODE_RANGE" },
      { type: "rule_reference", name: "function_call" },
      { type: "token_reference", name: "VARIABLE" },
      { type: "token_reference", name: "SLASH" },
      { type: "token_reference", name: "COMMA" },
      { type: "token_reference", name: "PLUS" },
      { type: "token_reference", name: "MINUS" },
      { type: "rule_reference", name: "map_literal" },
    ] },
    lineNumber: 433,
  },
  {
    name: "function_call",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "FUNCTION" },
        { type: "rule_reference", name: "function_args" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "token_reference", name: "URL_TOKEN" },
    ] },
    lineNumber: 439,
  },
  {
    name: "function_args",
    body: { type: "repetition", element: { type: "rule_reference", name: "function_arg" } },
    lineNumber: 442,
  },
  {
    name: "function_arg",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "DIMENSION" },
      { type: "token_reference", name: "PERCENTAGE" },
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "IDENT" },
      { type: "token_reference", name: "HASH" },
      { type: "token_reference", name: "CUSTOM_PROPERTY" },
      { type: "token_reference", name: "COMMA" },
      { type: "token_reference", name: "SLASH" },
      { type: "token_reference", name: "PLUS" },
      { type: "token_reference", name: "MINUS" },
      { type: "token_reference", name: "STAR" },
      { type: "token_reference", name: "VARIABLE" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "FUNCTION" },
        { type: "rule_reference", name: "function_args" },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 444,
  },
],
};
