// AUTO-GENERATED FILE - DO NOT EDIT
import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const LatticeGrammar: ParserGrammar = {
  version: 1,
  rules: [
    {
      name: "stylesheet",
      lineNumber: 37,
      body: { type: "repetition", element: { type: "rule_reference", name: "rule" } }
    },
    {
      name: "rule",
      lineNumber: 39,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "lattice_rule" }, { type: "rule_reference", name: "at_rule" }, { type: "rule_reference", name: "qualified_rule" }] }
    },
    {
      name: "lattice_rule",
      lineNumber: 51,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "variable_declaration" }, { type: "rule_reference", name: "mixin_definition" }, { type: "rule_reference", name: "function_definition" }, { type: "rule_reference", name: "use_directive" }, { type: "rule_reference", name: "lattice_control" }] }
    },
    {
      name: "variable_declaration",
      lineNumber: 69,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "VARIABLE" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "value_list" }, { type: "optional", element: { type: "alternation", choices: [{ type: "token_reference", name: "BANG_DEFAULT" }, { type: "token_reference", name: "BANG_GLOBAL" }] } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "mixin_definition",
      lineNumber: 102,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "literal", value: "@mixin" }, { type: "token_reference", name: "FUNCTION" }, { type: "optional", element: { type: "rule_reference", name: "mixin_params" } }, { type: "token_reference", name: "RPAREN" }, { type: "rule_reference", name: "block" }] }, { type: "sequence", elements: [{ type: "literal", value: "@mixin" }, { type: "token_reference", name: "IDENT" }, { type: "rule_reference", name: "block" }] }] }
    },
    {
      name: "mixin_params",
      lineNumber: 105,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "mixin_param" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "mixin_param" }] } }] }
    },
    {
      name: "mixin_param",
      lineNumber: 112,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "VARIABLE" }, { type: "optional", element: { type: "sequence", elements: [{ type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "mixin_value_list" }] } }] }
    },
    {
      name: "mixin_value_list",
      lineNumber: 117,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "mixin_value" }, { type: "repetition", element: { type: "rule_reference", name: "mixin_value" } }] }
    },
    {
      name: "mixin_value",
      lineNumber: 119,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "DIMENSION" }, { type: "token_reference", name: "PERCENTAGE" }, { type: "token_reference", name: "NUMBER" }, { type: "token_reference", name: "STRING" }, { type: "token_reference", name: "IDENT" }, { type: "token_reference", name: "HASH" }, { type: "token_reference", name: "CUSTOM_PROPERTY" }, { type: "token_reference", name: "UNICODE_RANGE" }, { type: "rule_reference", name: "function_call" }, { type: "token_reference", name: "VARIABLE" }, { type: "token_reference", name: "SLASH" }, { type: "token_reference", name: "PLUS" }, { type: "token_reference", name: "MINUS" }] }
    },
    {
      name: "include_directive",
      lineNumber: 130,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "literal", value: "@include" }, { type: "token_reference", name: "FUNCTION" }, { type: "optional", element: { type: "rule_reference", name: "include_args" } }, { type: "token_reference", name: "RPAREN" }, { type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "SEMICOLON" }, { type: "rule_reference", name: "block" }] } }] }, { type: "sequence", elements: [{ type: "literal", value: "@include" }, { type: "token_reference", name: "IDENT" }, { type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "SEMICOLON" }, { type: "rule_reference", name: "block" }] } }] }] }
    },
    {
      name: "include_args",
      lineNumber: 133,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "include_arg" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "include_arg" }] } }] }
    },
    {
      name: "include_arg",
      lineNumber: 137,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "token_reference", name: "VARIABLE" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "value_list" }] }, { type: "rule_reference", name: "value_list" }] }
    },
    {
      name: "lattice_control",
      lineNumber: 160,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "if_directive" }, { type: "rule_reference", name: "for_directive" }, { type: "rule_reference", name: "each_directive" }, { type: "rule_reference", name: "while_directive" }] }
    },
    {
      name: "if_directive",
      lineNumber: 164,
      body: { type: "sequence", elements: [{ type: "literal", value: "@if" }, { type: "rule_reference", name: "lattice_expression" }, { type: "rule_reference", name: "block" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: "@else" }, { type: "literal", value: "if" }, { type: "rule_reference", name: "lattice_expression" }, { type: "rule_reference", name: "block" }] } }, { type: "optional", element: { type: "sequence", elements: [{ type: "literal", value: "@else" }, { type: "rule_reference", name: "block" }] } }] }
    },
    {
      name: "for_directive",
      lineNumber: 171,
      body: { type: "sequence", elements: [{ type: "literal", value: "@for" }, { type: "token_reference", name: "VARIABLE" }, { type: "literal", value: "from" }, { type: "rule_reference", name: "lattice_expression" }, { type: "group", element: { type: "alternation", choices: [{ type: "literal", value: "through" }, { type: "literal", value: "to" }] } }, { type: "rule_reference", name: "lattice_expression" }, { type: "rule_reference", name: "block" }] }
    },
    {
      name: "each_directive",
      lineNumber: 176,
      body: { type: "sequence", elements: [{ type: "literal", value: "@each" }, { type: "token_reference", name: "VARIABLE" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "token_reference", name: "VARIABLE" }] } }, { type: "literal", value: "in" }, { type: "rule_reference", name: "each_list" }, { type: "rule_reference", name: "block" }] }
    },
    {
      name: "each_list",
      lineNumber: 179,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "value" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "value" }] } }] }
    },
    {
      name: "while_directive",
      lineNumber: 184,
      body: { type: "sequence", elements: [{ type: "literal", value: "@while" }, { type: "rule_reference", name: "lattice_expression" }, { type: "rule_reference", name: "block" }] }
    },
    {
      name: "lattice_expression",
      lineNumber: 203,
      body: { type: "rule_reference", name: "lattice_or_expr" }
    },
    {
      name: "lattice_or_expr",
      lineNumber: 205,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "lattice_and_expr" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: "or" }, { type: "rule_reference", name: "lattice_and_expr" }] } }] }
    },
    {
      name: "lattice_and_expr",
      lineNumber: 207,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "lattice_comparison" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "literal", value: "and" }, { type: "rule_reference", name: "lattice_comparison" }] } }] }
    },
    {
      name: "lattice_comparison",
      lineNumber: 209,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "lattice_additive" }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "comparison_op" }, { type: "rule_reference", name: "lattice_additive" }] } }] }
    },
    {
      name: "comparison_op",
      lineNumber: 211,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "EQUALS_EQUALS" }, { type: "token_reference", name: "NOT_EQUALS" }, { type: "token_reference", name: "GREATER" }, { type: "token_reference", name: "GREATER_EQUALS" }, { type: "token_reference", name: "LESS" }, { type: "token_reference", name: "LESS_EQUALS" }] }
    },
    {
      name: "lattice_additive",
      lineNumber: 214,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "lattice_multiplicative" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "PLUS" }, { type: "token_reference", name: "MINUS" }] } }, { type: "rule_reference", name: "lattice_multiplicative" }] } }] }
    },
    {
      name: "lattice_multiplicative",
      lineNumber: 219,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "lattice_unary" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "STAR" }, { type: "token_reference", name: "SLASH" }] } }, { type: "rule_reference", name: "lattice_unary" }] } }] }
    },
    {
      name: "lattice_unary",
      lineNumber: 221,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "token_reference", name: "MINUS" }, { type: "rule_reference", name: "lattice_unary" }] }, { type: "rule_reference", name: "lattice_primary" }] }
    },
    {
      name: "lattice_primary",
      lineNumber: 224,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "VARIABLE" }, { type: "token_reference", name: "NUMBER" }, { type: "token_reference", name: "DIMENSION" }, { type: "token_reference", name: "PERCENTAGE" }, { type: "token_reference", name: "STRING" }, { type: "token_reference", name: "IDENT" }, { type: "token_reference", name: "HASH" }, { type: "literal", value: "true" }, { type: "literal", value: "false" }, { type: "literal", value: "null" }, { type: "rule_reference", name: "function_call" }, { type: "rule_reference", name: "map_literal" }, { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "lattice_expression" }, { type: "token_reference", name: "RPAREN" }] }] }
    },
    {
      name: "map_literal",
      lineNumber: 235,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "map_entry" }, { type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "map_entry" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "map_entry" }] } }, { type: "token_reference", name: "RPAREN" }] }
    },
    {
      name: "map_entry",
      lineNumber: 237,
      body: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "IDENT" }, { type: "token_reference", name: "STRING" }] } }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "lattice_expression" }] }
    },
    {
      name: "function_definition",
      lineNumber: 261,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "literal", value: "@function" }, { type: "token_reference", name: "FUNCTION" }, { type: "optional", element: { type: "rule_reference", name: "mixin_params" } }, { type: "token_reference", name: "RPAREN" }, { type: "rule_reference", name: "function_body" }] }, { type: "sequence", elements: [{ type: "literal", value: "@function" }, { type: "token_reference", name: "IDENT" }, { type: "rule_reference", name: "function_body" }] }] }
    },
    {
      name: "function_body",
      lineNumber: 264,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LBRACE" }, { type: "repetition", element: { type: "rule_reference", name: "function_body_item" } }, { type: "token_reference", name: "RBRACE" }] }
    },
    {
      name: "function_body_item",
      lineNumber: 266,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "variable_declaration" }, { type: "rule_reference", name: "return_directive" }, { type: "rule_reference", name: "lattice_control" }] }
    },
    {
      name: "return_directive",
      lineNumber: 268,
      body: { type: "sequence", elements: [{ type: "literal", value: "@return" }, { type: "rule_reference", name: "lattice_expression" }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "use_directive",
      lineNumber: 281,
      body: { type: "sequence", elements: [{ type: "literal", value: "@use" }, { type: "token_reference", name: "STRING" }, { type: "optional", element: { type: "sequence", elements: [{ type: "literal", value: "as" }, { type: "token_reference", name: "IDENT" }] } }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "at_rule",
      lineNumber: 294,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "AT_KEYWORD" }, { type: "rule_reference", name: "at_prelude" }, { type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "SEMICOLON" }, { type: "rule_reference", name: "block" }] } }] }
    },
    {
      name: "at_prelude",
      lineNumber: 296,
      body: { type: "repetition", element: { type: "rule_reference", name: "at_prelude_token" } }
    },
    {
      name: "at_prelude_token",
      lineNumber: 298,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "IDENT" }, { type: "token_reference", name: "STRING" }, { type: "token_reference", name: "NUMBER" }, { type: "token_reference", name: "DIMENSION" }, { type: "token_reference", name: "PERCENTAGE" }, { type: "token_reference", name: "HASH" }, { type: "token_reference", name: "CUSTOM_PROPERTY" }, { type: "token_reference", name: "UNICODE_RANGE" }, { type: "token_reference", name: "VARIABLE" }, { type: "rule_reference", name: "function_in_prelude" }, { type: "rule_reference", name: "paren_block" }, { type: "token_reference", name: "COLON" }, { type: "token_reference", name: "COMMA" }, { type: "token_reference", name: "SLASH" }, { type: "token_reference", name: "DOT" }, { type: "token_reference", name: "STAR" }, { type: "token_reference", name: "PLUS" }, { type: "token_reference", name: "MINUS" }, { type: "token_reference", name: "GREATER" }, { type: "token_reference", name: "TILDE" }, { type: "token_reference", name: "PIPE" }, { type: "token_reference", name: "EQUALS" }, { type: "token_reference", name: "AMPERSAND" }, { type: "token_reference", name: "CDO" }, { type: "token_reference", name: "CDC" }] }
    },
    {
      name: "function_in_prelude",
      lineNumber: 306,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "FUNCTION" }, { type: "rule_reference", name: "at_prelude_tokens" }, { type: "token_reference", name: "RPAREN" }] }
    },
    {
      name: "paren_block",
      lineNumber: 307,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "at_prelude_tokens" }, { type: "token_reference", name: "RPAREN" }] }
    },
    {
      name: "at_prelude_tokens",
      lineNumber: 308,
      body: { type: "repetition", element: { type: "rule_reference", name: "at_prelude_token" } }
    },
    {
      name: "qualified_rule",
      lineNumber: 314,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "selector_list" }, { type: "rule_reference", name: "block" }] }
    },
    {
      name: "selector_list",
      lineNumber: 320,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "complex_selector" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "complex_selector" }] } }] }
    },
    {
      name: "complex_selector",
      lineNumber: 322,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "compound_selector" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "optional", element: { type: "rule_reference", name: "combinator" } }, { type: "rule_reference", name: "compound_selector" }] } }] }
    },
    {
      name: "combinator",
      lineNumber: 324,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "GREATER" }, { type: "token_reference", name: "PLUS" }, { type: "token_reference", name: "TILDE" }] }
    },
    {
      name: "compound_selector",
      lineNumber: 326,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "rule_reference", name: "simple_selector" }, { type: "repetition", element: { type: "rule_reference", name: "subclass_selector" } }] }, { type: "sequence", elements: [{ type: "rule_reference", name: "subclass_selector" }, { type: "repetition", element: { type: "rule_reference", name: "subclass_selector" } }] }] }
    },
    {
      name: "simple_selector",
      lineNumber: 330,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "IDENT" }, { type: "token_reference", name: "STAR" }, { type: "token_reference", name: "AMPERSAND" }, { type: "token_reference", name: "VARIABLE" }] }
    },
    {
      name: "subclass_selector",
      lineNumber: 333,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "class_selector" }, { type: "rule_reference", name: "id_selector" }, { type: "rule_reference", name: "placeholder_selector" }, { type: "rule_reference", name: "attribute_selector" }, { type: "rule_reference", name: "pseudo_class" }, { type: "rule_reference", name: "pseudo_element" }] }
    },
    {
      name: "placeholder_selector",
      lineNumber: 337,
      body: { type: "token_reference", name: "PLACEHOLDER" }
    },
    {
      name: "class_selector",
      lineNumber: 339,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "DOT" }, { type: "token_reference", name: "IDENT" }] }
    },
    {
      name: "id_selector",
      lineNumber: 341,
      body: { type: "token_reference", name: "HASH" }
    },
    {
      name: "attribute_selector",
      lineNumber: 343,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LBRACKET" }, { type: "token_reference", name: "IDENT" }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "attr_matcher" }, { type: "rule_reference", name: "attr_value" }, { type: "optional", element: { type: "token_reference", name: "IDENT" } }] } }, { type: "token_reference", name: "RBRACKET" }] }
    },
    {
      name: "attr_matcher",
      lineNumber: 345,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "EQUALS" }, { type: "token_reference", name: "TILDE_EQUALS" }, { type: "token_reference", name: "PIPE_EQUALS" }, { type: "token_reference", name: "CARET_EQUALS" }, { type: "token_reference", name: "DOLLAR_EQUALS" }, { type: "token_reference", name: "STAR_EQUALS" }] }
    },
    {
      name: "attr_value",
      lineNumber: 348,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "IDENT" }, { type: "token_reference", name: "STRING" }] }
    },
    {
      name: "pseudo_class",
      lineNumber: 350,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "token_reference", name: "COLON" }, { type: "token_reference", name: "FUNCTION" }, { type: "rule_reference", name: "pseudo_class_args" }, { type: "token_reference", name: "RPAREN" }] }, { type: "sequence", elements: [{ type: "token_reference", name: "COLON" }, { type: "token_reference", name: "IDENT" }] }] }
    },
    {
      name: "pseudo_class_args",
      lineNumber: 353,
      body: { type: "repetition", element: { type: "rule_reference", name: "pseudo_class_arg" } }
    },
    {
      name: "pseudo_class_arg",
      lineNumber: 355,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "IDENT" }, { type: "token_reference", name: "NUMBER" }, { type: "token_reference", name: "DIMENSION" }, { type: "token_reference", name: "STRING" }, { type: "token_reference", name: "HASH" }, { type: "token_reference", name: "PLUS" }, { type: "token_reference", name: "COMMA" }, { type: "token_reference", name: "DOT" }, { type: "token_reference", name: "STAR" }, { type: "token_reference", name: "COLON" }, { type: "token_reference", name: "AMPERSAND" }, { type: "sequence", elements: [{ type: "token_reference", name: "FUNCTION" }, { type: "rule_reference", name: "pseudo_class_args" }, { type: "token_reference", name: "RPAREN" }] }, { type: "sequence", elements: [{ type: "token_reference", name: "LBRACKET" }, { type: "rule_reference", name: "pseudo_class_args" }, { type: "token_reference", name: "RBRACKET" }] }] }
    },
    {
      name: "pseudo_element",
      lineNumber: 360,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "COLON_COLON" }, { type: "token_reference", name: "IDENT" }] }
    },
    {
      name: "block",
      lineNumber: 370,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LBRACE" }, { type: "rule_reference", name: "block_contents" }, { type: "token_reference", name: "RBRACE" }] }
    },
    {
      name: "block_contents",
      lineNumber: 372,
      body: { type: "repetition", element: { type: "rule_reference", name: "block_item" } }
    },
    {
      name: "block_item",
      lineNumber: 374,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "lattice_block_item" }, { type: "rule_reference", name: "at_rule" }, { type: "rule_reference", name: "declaration_or_nested" }] }
    },
    {
      name: "lattice_block_item",
      lineNumber: 380,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "variable_declaration" }, { type: "rule_reference", name: "include_directive" }, { type: "rule_reference", name: "lattice_control" }, { type: "rule_reference", name: "content_directive" }, { type: "rule_reference", name: "extend_directive" }, { type: "rule_reference", name: "at_root_directive" }] }
    },
    {
      name: "content_directive",
      lineNumber: 390,
      body: { type: "sequence", elements: [{ type: "literal", value: "@content" }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "extend_directive",
      lineNumber: 398,
      body: { type: "sequence", elements: [{ type: "literal", value: "@extend" }, { type: "rule_reference", name: "selector_list" }, { type: "token_reference", name: "SEMICOLON" }] }
    },
    {
      name: "at_root_directive",
      lineNumber: 403,
      body: { type: "sequence", elements: [{ type: "literal", value: "@at-root" }, { type: "group", element: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "rule_reference", name: "selector_list" }, { type: "rule_reference", name: "block" }] }, { type: "rule_reference", name: "block" }] } }] }
    },
    {
      name: "declaration_or_nested",
      lineNumber: 405,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "declaration" }, { type: "rule_reference", name: "qualified_rule" }] }
    },
    {
      name: "declaration",
      lineNumber: 414,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "rule_reference", name: "property" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "value_list" }, { type: "optional", element: { type: "rule_reference", name: "priority" } }, { type: "token_reference", name: "SEMICOLON" }] }, { type: "sequence", elements: [{ type: "rule_reference", name: "property" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "block" }] }] }
    },
    {
      name: "property",
      lineNumber: 417,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "IDENT" }, { type: "token_reference", name: "CUSTOM_PROPERTY" }] }
    },
    {
      name: "priority",
      lineNumber: 419,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "BANG" }, { type: "literal", value: "important" }] }
    },
    {
      name: "value_list",
      lineNumber: 430,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "value" }, { type: "repetition", element: { type: "rule_reference", name: "value" } }] }
    },
    {
      name: "value",
      lineNumber: 432,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "DIMENSION" }, { type: "token_reference", name: "PERCENTAGE" }, { type: "token_reference", name: "NUMBER" }, { type: "token_reference", name: "STRING" }, { type: "token_reference", name: "IDENT" }, { type: "token_reference", name: "HASH" }, { type: "token_reference", name: "CUSTOM_PROPERTY" }, { type: "token_reference", name: "UNICODE_RANGE" }, { type: "rule_reference", name: "function_call" }, { type: "token_reference", name: "VARIABLE" }, { type: "token_reference", name: "SLASH" }, { type: "token_reference", name: "COMMA" }, { type: "token_reference", name: "PLUS" }, { type: "token_reference", name: "MINUS" }, { type: "rule_reference", name: "map_literal" }] }
    },
    {
      name: "function_call",
      lineNumber: 438,
      body: { type: "alternation", choices: [{ type: "sequence", elements: [{ type: "token_reference", name: "FUNCTION" }, { type: "rule_reference", name: "function_args" }, { type: "token_reference", name: "RPAREN" }] }, { type: "token_reference", name: "URL_TOKEN" }] }
    },
    {
      name: "function_args",
      lineNumber: 441,
      body: { type: "repetition", element: { type: "rule_reference", name: "function_arg" } }
    },
    {
      name: "function_arg",
      lineNumber: 443,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "DIMENSION" }, { type: "token_reference", name: "PERCENTAGE" }, { type: "token_reference", name: "NUMBER" }, { type: "token_reference", name: "STRING" }, { type: "token_reference", name: "IDENT" }, { type: "token_reference", name: "HASH" }, { type: "token_reference", name: "CUSTOM_PROPERTY" }, { type: "token_reference", name: "COMMA" }, { type: "token_reference", name: "SLASH" }, { type: "token_reference", name: "PLUS" }, { type: "token_reference", name: "MINUS" }, { type: "token_reference", name: "STAR" }, { type: "token_reference", name: "VARIABLE" }, { type: "sequence", elements: [{ type: "token_reference", name: "FUNCTION" }, { type: "rule_reference", name: "function_args" }, { type: "token_reference", name: "RPAREN" }] }] }
    },
  ]
};
