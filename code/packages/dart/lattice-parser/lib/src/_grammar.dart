// AUTO-GENERATED FILE - DO NOT EDIT
// Source: lattice.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
      name: "stylesheet",
      body: Repetition(element: RuleReference("rule", isToken: false)),
      lineNumber: 37,
    ),
    GrammarRule(
      name: "rule",
      body: Alternation(
        choices: [
          RuleReference("lattice_rule", isToken: false),
          RuleReference("at_rule", isToken: false),
          RuleReference("qualified_rule", isToken: false),
        ],
      ),
      lineNumber: 39,
    ),
    GrammarRule(
      name: "lattice_rule",
      body: Alternation(
        choices: [
          RuleReference("variable_declaration", isToken: false),
          RuleReference("mixin_definition", isToken: false),
          RuleReference("function_definition", isToken: false),
          RuleReference("use_directive", isToken: false),
          RuleReference("lattice_control", isToken: false),
        ],
      ),
      lineNumber: 51,
    ),
    GrammarRule(
      name: "variable_declaration",
      body: Sequence(
        elements: [
          RuleReference("VARIABLE", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("value_list", isToken: false),
          Optional(
            element: Alternation(
              choices: [
                RuleReference("BANG_DEFAULT", isToken: true),
                RuleReference("BANG_GLOBAL", isToken: true),
              ],
            ),
          ),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 69,
    ),
    GrammarRule(
      name: "mixin_definition",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              Literal("@mixin"),
              RuleReference("FUNCTION", isToken: true),
              Optional(element: RuleReference("mixin_params", isToken: false)),
              RuleReference("RPAREN", isToken: true),
              RuleReference("block", isToken: false),
            ],
          ),
          Sequence(
            elements: [
              Literal("@mixin"),
              RuleReference("IDENT", isToken: true),
              RuleReference("block", isToken: false),
            ],
          ),
        ],
      ),
      lineNumber: 102,
    ),
    GrammarRule(
      name: "mixin_params",
      body: Sequence(
        elements: [
          RuleReference("mixin_param", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("mixin_param", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 105,
    ),
    GrammarRule(
      name: "mixin_param",
      body: Sequence(
        elements: [
          RuleReference("VARIABLE", isToken: true),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("COLON", isToken: true),
                RuleReference("mixin_value_list", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 112,
    ),
    GrammarRule(
      name: "mixin_value_list",
      body: Sequence(
        elements: [
          RuleReference("mixin_value", isToken: false),
          Repetition(element: RuleReference("mixin_value", isToken: false)),
        ],
      ),
      lineNumber: 117,
    ),
    GrammarRule(
      name: "mixin_value",
      body: Alternation(
        choices: [
          RuleReference("DIMENSION", isToken: true),
          RuleReference("PERCENTAGE", isToken: true),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("IDENT", isToken: true),
          RuleReference("HASH", isToken: true),
          RuleReference("CUSTOM_PROPERTY", isToken: true),
          RuleReference("UNICODE_RANGE", isToken: true),
          RuleReference("function_call", isToken: false),
          RuleReference("VARIABLE", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
        ],
      ),
      lineNumber: 119,
    ),
    GrammarRule(
      name: "include_directive",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              Literal("@include"),
              RuleReference("FUNCTION", isToken: true),
              Optional(element: RuleReference("include_args", isToken: false)),
              RuleReference("RPAREN", isToken: true),
              Group(
                element: Alternation(
                  choices: [
                    RuleReference("SEMICOLON", isToken: true),
                    RuleReference("block", isToken: false),
                  ],
                ),
              ),
            ],
          ),
          Sequence(
            elements: [
              Literal("@include"),
              RuleReference("IDENT", isToken: true),
              Group(
                element: Alternation(
                  choices: [
                    RuleReference("SEMICOLON", isToken: true),
                    RuleReference("block", isToken: false),
                  ],
                ),
              ),
            ],
          ),
        ],
      ),
      lineNumber: 130,
    ),
    GrammarRule(
      name: "include_args",
      body: Sequence(
        elements: [
          RuleReference("include_arg", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("include_arg", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 133,
    ),
    GrammarRule(
      name: "include_arg",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              RuleReference("VARIABLE", isToken: true),
              RuleReference("COLON", isToken: true),
              RuleReference("value_list", isToken: false),
            ],
          ),
          RuleReference("value_list", isToken: false),
        ],
      ),
      lineNumber: 137,
    ),
    GrammarRule(
      name: "lattice_control",
      body: Alternation(
        choices: [
          RuleReference("if_directive", isToken: false),
          RuleReference("for_directive", isToken: false),
          RuleReference("each_directive", isToken: false),
          RuleReference("while_directive", isToken: false),
        ],
      ),
      lineNumber: 160,
    ),
    GrammarRule(
      name: "if_directive",
      body: Sequence(
        elements: [
          Literal("@if"),
          RuleReference("lattice_expression", isToken: false),
          RuleReference("block", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal("@else"),
                Literal("if"),
                RuleReference("lattice_expression", isToken: false),
                RuleReference("block", isToken: false),
              ],
            ),
          ),
          Optional(
            element: Sequence(
              elements: [
                Literal("@else"),
                RuleReference("block", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 164,
    ),
    GrammarRule(
      name: "for_directive",
      body: Sequence(
        elements: [
          Literal("@for"),
          RuleReference("VARIABLE", isToken: true),
          Literal("from"),
          RuleReference("lattice_expression", isToken: false),
          Group(
            element: Alternation(choices: [Literal("through"), Literal("to")]),
          ),
          RuleReference("lattice_expression", isToken: false),
          RuleReference("block", isToken: false),
        ],
      ),
      lineNumber: 171,
    ),
    GrammarRule(
      name: "each_directive",
      body: Sequence(
        elements: [
          Literal("@each"),
          RuleReference("VARIABLE", isToken: true),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("VARIABLE", isToken: true),
              ],
            ),
          ),
          Literal("in"),
          RuleReference("each_list", isToken: false),
          RuleReference("block", isToken: false),
        ],
      ),
      lineNumber: 176,
    ),
    GrammarRule(
      name: "each_list",
      body: Sequence(
        elements: [
          RuleReference("value", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("value", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 179,
    ),
    GrammarRule(
      name: "while_directive",
      body: Sequence(
        elements: [
          Literal("@while"),
          RuleReference("lattice_expression", isToken: false),
          RuleReference("block", isToken: false),
        ],
      ),
      lineNumber: 184,
    ),
    GrammarRule(
      name: "lattice_expression",
      body: RuleReference("lattice_or_expr", isToken: false),
      lineNumber: 203,
    ),
    GrammarRule(
      name: "lattice_or_expr",
      body: Sequence(
        elements: [
          RuleReference("lattice_and_expr", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal("or"),
                RuleReference("lattice_and_expr", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 205,
    ),
    GrammarRule(
      name: "lattice_and_expr",
      body: Sequence(
        elements: [
          RuleReference("lattice_comparison", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal("and"),
                RuleReference("lattice_comparison", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 207,
    ),
    GrammarRule(
      name: "lattice_comparison",
      body: Sequence(
        elements: [
          RuleReference("lattice_additive", isToken: false),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("comparison_op", isToken: false),
                RuleReference("lattice_additive", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 209,
    ),
    GrammarRule(
      name: "comparison_op",
      body: Alternation(
        choices: [
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("GREATER", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          RuleReference("LESS", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
        ],
      ),
      lineNumber: 211,
    ),
    GrammarRule(
      name: "lattice_additive",
      body: Sequence(
        elements: [
          RuleReference("lattice_multiplicative", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Group(
                  element: Alternation(
                    choices: [
                      RuleReference("PLUS", isToken: true),
                      RuleReference("MINUS", isToken: true),
                    ],
                  ),
                ),
                RuleReference("lattice_multiplicative", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 214,
    ),
    GrammarRule(
      name: "lattice_multiplicative",
      body: Sequence(
        elements: [
          RuleReference("lattice_unary", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Group(
                  element: Alternation(
                    choices: [
                      RuleReference("STAR", isToken: true),
                      RuleReference("SLASH", isToken: true),
                    ],
                  ),
                ),
                RuleReference("lattice_unary", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 219,
    ),
    GrammarRule(
      name: "lattice_unary",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              RuleReference("MINUS", isToken: true),
              RuleReference("lattice_unary", isToken: false),
            ],
          ),
          RuleReference("lattice_primary", isToken: false),
        ],
      ),
      lineNumber: 221,
    ),
    GrammarRule(
      name: "lattice_primary",
      body: Alternation(
        choices: [
          RuleReference("VARIABLE", isToken: true),
          RuleReference("NUMBER", isToken: true),
          RuleReference("DIMENSION", isToken: true),
          RuleReference("PERCENTAGE", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("IDENT", isToken: true),
          RuleReference("HASH", isToken: true),
          Literal("true"),
          Literal("false"),
          Literal("null"),
          RuleReference("function_call", isToken: false),
          RuleReference("map_literal", isToken: false),
          Sequence(
            elements: [
              RuleReference("LPAREN", isToken: true),
              RuleReference("lattice_expression", isToken: false),
              RuleReference("RPAREN", isToken: true),
            ],
          ),
        ],
      ),
      lineNumber: 224,
    ),
    GrammarRule(
      name: "map_literal",
      body: Sequence(
        elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("map_entry", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("map_entry", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("map_entry", isToken: false),
              ],
            ),
          ),
          RuleReference("RPAREN", isToken: true),
        ],
      ),
      lineNumber: 235,
    ),
    GrammarRule(
      name: "map_entry",
      body: Sequence(
        elements: [
          Group(
            element: Alternation(
              choices: [
                RuleReference("IDENT", isToken: true),
                RuleReference("STRING", isToken: true),
              ],
            ),
          ),
          RuleReference("COLON", isToken: true),
          RuleReference("lattice_expression", isToken: false),
        ],
      ),
      lineNumber: 237,
    ),
    GrammarRule(
      name: "function_definition",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              Literal("@function"),
              RuleReference("FUNCTION", isToken: true),
              Optional(element: RuleReference("mixin_params", isToken: false)),
              RuleReference("RPAREN", isToken: true),
              RuleReference("function_body", isToken: false),
            ],
          ),
          Sequence(
            elements: [
              Literal("@function"),
              RuleReference("IDENT", isToken: true),
              RuleReference("function_body", isToken: false),
            ],
          ),
        ],
      ),
      lineNumber: 261,
    ),
    GrammarRule(
      name: "function_body",
      body: Sequence(
        elements: [
          RuleReference("LBRACE", isToken: true),
          Repetition(
            element: RuleReference("function_body_item", isToken: false),
          ),
          RuleReference("RBRACE", isToken: true),
        ],
      ),
      lineNumber: 264,
    ),
    GrammarRule(
      name: "function_body_item",
      body: Alternation(
        choices: [
          RuleReference("variable_declaration", isToken: false),
          RuleReference("return_directive", isToken: false),
          RuleReference("lattice_control", isToken: false),
        ],
      ),
      lineNumber: 266,
    ),
    GrammarRule(
      name: "return_directive",
      body: Sequence(
        elements: [
          Literal("@return"),
          RuleReference("lattice_expression", isToken: false),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 268,
    ),
    GrammarRule(
      name: "use_directive",
      body: Sequence(
        elements: [
          Literal("@use"),
          RuleReference("STRING", isToken: true),
          Optional(
            element: Sequence(
              elements: [Literal("as"), RuleReference("IDENT", isToken: true)],
            ),
          ),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 281,
    ),
    GrammarRule(
      name: "at_rule",
      body: Sequence(
        elements: [
          RuleReference("AT_KEYWORD", isToken: true),
          RuleReference("at_prelude", isToken: false),
          Group(
            element: Alternation(
              choices: [
                RuleReference("SEMICOLON", isToken: true),
                RuleReference("block", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 294,
    ),
    GrammarRule(
      name: "at_prelude",
      body: Repetition(
        element: RuleReference("at_prelude_token", isToken: false),
      ),
      lineNumber: 296,
    ),
    GrammarRule(
      name: "at_prelude_token",
      body: Alternation(
        choices: [
          RuleReference("IDENT", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("NUMBER", isToken: true),
          RuleReference("DIMENSION", isToken: true),
          RuleReference("PERCENTAGE", isToken: true),
          RuleReference("HASH", isToken: true),
          RuleReference("CUSTOM_PROPERTY", isToken: true),
          RuleReference("UNICODE_RANGE", isToken: true),
          RuleReference("VARIABLE", isToken: true),
          RuleReference("function_in_prelude", isToken: false),
          RuleReference("paren_block", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("COMMA", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("DOT", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("GREATER", isToken: true),
          RuleReference("TILDE", isToken: true),
          RuleReference("PIPE", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          RuleReference("CDO", isToken: true),
          RuleReference("CDC", isToken: true),
        ],
      ),
      lineNumber: 298,
    ),
    GrammarRule(
      name: "function_in_prelude",
      body: Sequence(
        elements: [
          RuleReference("FUNCTION", isToken: true),
          RuleReference("at_prelude_tokens", isToken: false),
          RuleReference("RPAREN", isToken: true),
        ],
      ),
      lineNumber: 306,
    ),
    GrammarRule(
      name: "paren_block",
      body: Sequence(
        elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("at_prelude_tokens", isToken: false),
          RuleReference("RPAREN", isToken: true),
        ],
      ),
      lineNumber: 307,
    ),
    GrammarRule(
      name: "at_prelude_tokens",
      body: Repetition(
        element: RuleReference("at_prelude_token", isToken: false),
      ),
      lineNumber: 308,
    ),
    GrammarRule(
      name: "qualified_rule",
      body: Sequence(
        elements: [
          RuleReference("selector_list", isToken: false),
          RuleReference("block", isToken: false),
        ],
      ),
      lineNumber: 314,
    ),
    GrammarRule(
      name: "selector_list",
      body: Sequence(
        elements: [
          RuleReference("complex_selector", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("complex_selector", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 320,
    ),
    GrammarRule(
      name: "complex_selector",
      body: Sequence(
        elements: [
          RuleReference("compound_selector", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Optional(element: RuleReference("combinator", isToken: false)),
                RuleReference("compound_selector", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 322,
    ),
    GrammarRule(
      name: "combinator",
      body: Alternation(
        choices: [
          RuleReference("GREATER", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("TILDE", isToken: true),
        ],
      ),
      lineNumber: 324,
    ),
    GrammarRule(
      name: "compound_selector",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              RuleReference("simple_selector", isToken: false),
              Repetition(
                element: RuleReference("subclass_selector", isToken: false),
              ),
            ],
          ),
          Sequence(
            elements: [
              RuleReference("subclass_selector", isToken: false),
              Repetition(
                element: RuleReference("subclass_selector", isToken: false),
              ),
            ],
          ),
        ],
      ),
      lineNumber: 326,
    ),
    GrammarRule(
      name: "simple_selector",
      body: Alternation(
        choices: [
          RuleReference("IDENT", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          RuleReference("VARIABLE", isToken: true),
          RuleReference("PERCENTAGE", isToken: true),
        ],
      ),
      lineNumber: 331,
    ),
    GrammarRule(
      name: "subclass_selector",
      body: Alternation(
        choices: [
          RuleReference("class_selector", isToken: false),
          RuleReference("id_selector", isToken: false),
          RuleReference("placeholder_selector", isToken: false),
          RuleReference("attribute_selector", isToken: false),
          RuleReference("pseudo_class", isToken: false),
          RuleReference("pseudo_element", isToken: false),
        ],
      ),
      lineNumber: 334,
    ),
    GrammarRule(
      name: "placeholder_selector",
      body: RuleReference("PLACEHOLDER", isToken: true),
      lineNumber: 338,
    ),
    GrammarRule(
      name: "class_selector",
      body: Sequence(
        elements: [
          RuleReference("DOT", isToken: true),
          RuleReference("IDENT", isToken: true),
        ],
      ),
      lineNumber: 340,
    ),
    GrammarRule(
      name: "id_selector",
      body: RuleReference("HASH", isToken: true),
      lineNumber: 342,
    ),
    GrammarRule(
      name: "attribute_selector",
      body: Sequence(
        elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("IDENT", isToken: true),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("attr_matcher", isToken: false),
                RuleReference("attr_value", isToken: false),
                Optional(element: RuleReference("IDENT", isToken: true)),
              ],
            ),
          ),
          RuleReference("RBRACKET", isToken: true),
        ],
      ),
      lineNumber: 344,
    ),
    GrammarRule(
      name: "attr_matcher",
      body: Alternation(
        choices: [
          RuleReference("EQUALS", isToken: true),
          RuleReference("TILDE_EQUALS", isToken: true),
          RuleReference("PIPE_EQUALS", isToken: true),
          RuleReference("CARET_EQUALS", isToken: true),
          RuleReference("DOLLAR_EQUALS", isToken: true),
          RuleReference("STAR_EQUALS", isToken: true),
        ],
      ),
      lineNumber: 346,
    ),
    GrammarRule(
      name: "attr_value",
      body: Alternation(
        choices: [
          RuleReference("IDENT", isToken: true),
          RuleReference("STRING", isToken: true),
        ],
      ),
      lineNumber: 349,
    ),
    GrammarRule(
      name: "pseudo_class",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              RuleReference("COLON", isToken: true),
              RuleReference("FUNCTION", isToken: true),
              RuleReference("pseudo_class_args", isToken: false),
              RuleReference("RPAREN", isToken: true),
            ],
          ),
          Sequence(
            elements: [
              RuleReference("COLON", isToken: true),
              RuleReference("IDENT", isToken: true),
            ],
          ),
        ],
      ),
      lineNumber: 351,
    ),
    GrammarRule(
      name: "pseudo_class_args",
      body: Repetition(
        element: RuleReference("pseudo_class_arg", isToken: false),
      ),
      lineNumber: 354,
    ),
    GrammarRule(
      name: "pseudo_class_arg",
      body: Alternation(
        choices: [
          RuleReference("IDENT", isToken: true),
          RuleReference("NUMBER", isToken: true),
          RuleReference("DIMENSION", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("HASH", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("COMMA", isToken: true),
          RuleReference("DOT", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          Sequence(
            elements: [
              RuleReference("FUNCTION", isToken: true),
              RuleReference("pseudo_class_args", isToken: false),
              RuleReference("RPAREN", isToken: true),
            ],
          ),
          Sequence(
            elements: [
              RuleReference("LBRACKET", isToken: true),
              RuleReference("pseudo_class_args", isToken: false),
              RuleReference("RBRACKET", isToken: true),
            ],
          ),
        ],
      ),
      lineNumber: 356,
    ),
    GrammarRule(
      name: "pseudo_element",
      body: Sequence(
        elements: [
          RuleReference("COLON_COLON", isToken: true),
          RuleReference("IDENT", isToken: true),
        ],
      ),
      lineNumber: 361,
    ),
    GrammarRule(
      name: "block",
      body: Sequence(
        elements: [
          RuleReference("LBRACE", isToken: true),
          RuleReference("block_contents", isToken: false),
          RuleReference("RBRACE", isToken: true),
        ],
      ),
      lineNumber: 371,
    ),
    GrammarRule(
      name: "block_contents",
      body: Repetition(element: RuleReference("block_item", isToken: false)),
      lineNumber: 373,
    ),
    GrammarRule(
      name: "block_item",
      body: Alternation(
        choices: [
          RuleReference("lattice_block_item", isToken: false),
          RuleReference("at_rule", isToken: false),
          RuleReference("declaration_or_nested", isToken: false),
        ],
      ),
      lineNumber: 375,
    ),
    GrammarRule(
      name: "lattice_block_item",
      body: Alternation(
        choices: [
          RuleReference("variable_declaration", isToken: false),
          RuleReference("include_directive", isToken: false),
          RuleReference("lattice_control", isToken: false),
          RuleReference("content_directive", isToken: false),
          RuleReference("extend_directive", isToken: false),
          RuleReference("at_root_directive", isToken: false),
        ],
      ),
      lineNumber: 381,
    ),
    GrammarRule(
      name: "content_directive",
      body: Sequence(
        elements: [
          Literal("@content"),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 391,
    ),
    GrammarRule(
      name: "extend_directive",
      body: Sequence(
        elements: [
          Literal("@extend"),
          RuleReference("selector_list", isToken: false),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 399,
    ),
    GrammarRule(
      name: "at_root_directive",
      body: Sequence(
        elements: [
          Literal("@at-root"),
          Group(
            element: Alternation(
              choices: [
                Sequence(
                  elements: [
                    RuleReference("selector_list", isToken: false),
                    RuleReference("block", isToken: false),
                  ],
                ),
                RuleReference("block", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 404,
    ),
    GrammarRule(
      name: "declaration_or_nested",
      body: Alternation(
        choices: [
          RuleReference("declaration", isToken: false),
          RuleReference("qualified_rule", isToken: false),
        ],
      ),
      lineNumber: 406,
    ),
    GrammarRule(
      name: "declaration",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              RuleReference("property", isToken: false),
              RuleReference("COLON", isToken: true),
              RuleReference("value_list", isToken: false),
              Optional(element: RuleReference("priority", isToken: false)),
              RuleReference("SEMICOLON", isToken: true),
            ],
          ),
          Sequence(
            elements: [
              RuleReference("property", isToken: false),
              RuleReference("COLON", isToken: true),
              RuleReference("block", isToken: false),
            ],
          ),
        ],
      ),
      lineNumber: 415,
    ),
    GrammarRule(
      name: "property",
      body: Alternation(
        choices: [
          RuleReference("IDENT", isToken: true),
          RuleReference("CUSTOM_PROPERTY", isToken: true),
        ],
      ),
      lineNumber: 418,
    ),
    GrammarRule(
      name: "priority",
      body: Sequence(
        elements: [RuleReference("BANG", isToken: true), Literal("important")],
      ),
      lineNumber: 420,
    ),
    GrammarRule(
      name: "value_list",
      body: Sequence(
        elements: [
          RuleReference("value", isToken: false),
          Repetition(element: RuleReference("value", isToken: false)),
        ],
      ),
      lineNumber: 431,
    ),
    GrammarRule(
      name: "value",
      body: Alternation(
        choices: [
          RuleReference("DIMENSION", isToken: true),
          RuleReference("PERCENTAGE", isToken: true),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("IDENT", isToken: true),
          RuleReference("HASH", isToken: true),
          RuleReference("CUSTOM_PROPERTY", isToken: true),
          RuleReference("UNICODE_RANGE", isToken: true),
          RuleReference("function_call", isToken: false),
          RuleReference("VARIABLE", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("COMMA", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("map_literal", isToken: false),
        ],
      ),
      lineNumber: 433,
    ),
    GrammarRule(
      name: "function_call",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              RuleReference("FUNCTION", isToken: true),
              RuleReference("function_args", isToken: false),
              RuleReference("RPAREN", isToken: true),
            ],
          ),
          RuleReference("URL_TOKEN", isToken: true),
        ],
      ),
      lineNumber: 439,
    ),
    GrammarRule(
      name: "function_args",
      body: Repetition(element: RuleReference("function_arg", isToken: false)),
      lineNumber: 442,
    ),
    GrammarRule(
      name: "function_arg",
      body: Alternation(
        choices: [
          RuleReference("DIMENSION", isToken: true),
          RuleReference("PERCENTAGE", isToken: true),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("IDENT", isToken: true),
          RuleReference("HASH", isToken: true),
          RuleReference("CUSTOM_PROPERTY", isToken: true),
          RuleReference("COMMA", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("VARIABLE", isToken: true),
          Sequence(
            elements: [
              RuleReference("FUNCTION", isToken: true),
              RuleReference("function_args", isToken: false),
              RuleReference("RPAREN", isToken: true),
            ],
          ),
        ],
      ),
      lineNumber: 444,
    ),
  ],
);
