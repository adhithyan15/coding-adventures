// AUTO-GENERATED FILE - DO NOT EDIT
// Source: excel.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
      name: "formula",
      body: Sequence(
        elements: [
          RuleReference("ws", isToken: false),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("EQUALS", isToken: true),
                RuleReference("ws", isToken: false),
              ],
            ),
          ),
          RuleReference("expression", isToken: false),
          RuleReference("ws", isToken: false),
        ],
      ),
      lineNumber: 15,
    ),
    GrammarRule(
      name: "ws",
      body: Repetition(element: RuleReference("SPACE", isToken: true)),
      lineNumber: 17,
    ),
    GrammarRule(
      name: "req_space",
      body: Sequence(
        elements: [
          RuleReference("SPACE", isToken: true),
          Repetition(element: RuleReference("SPACE", isToken: true)),
        ],
      ),
      lineNumber: 18,
    ),
    GrammarRule(
      name: "expression",
      body: RuleReference("comparison_expr", isToken: false),
      lineNumber: 20,
    ),
    GrammarRule(
      name: "comparison_expr",
      body: Sequence(
        elements: [
          RuleReference("concat_expr", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("ws", isToken: false),
                RuleReference("comparison_op", isToken: false),
                RuleReference("ws", isToken: false),
                RuleReference("concat_expr", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 22,
    ),
    GrammarRule(
      name: "comparison_op",
      body: Alternation(
        choices: [
          RuleReference("EQUALS", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
        ],
      ),
      lineNumber: 23,
    ),
    GrammarRule(
      name: "concat_expr",
      body: Sequence(
        elements: [
          RuleReference("additive_expr", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("ws", isToken: false),
                RuleReference("AMP", isToken: true),
                RuleReference("ws", isToken: false),
                RuleReference("additive_expr", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 26,
    ),
    GrammarRule(
      name: "additive_expr",
      body: Sequence(
        elements: [
          RuleReference("multiplicative_expr", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("ws", isToken: false),
                Group(
                  element: Alternation(
                    choices: [
                      RuleReference("PLUS", isToken: true),
                      RuleReference("MINUS", isToken: true),
                    ],
                  ),
                ),
                RuleReference("ws", isToken: false),
                RuleReference("multiplicative_expr", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 27,
    ),
    GrammarRule(
      name: "multiplicative_expr",
      body: Sequence(
        elements: [
          RuleReference("power_expr", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("ws", isToken: false),
                Group(
                  element: Alternation(
                    choices: [
                      RuleReference("STAR", isToken: true),
                      RuleReference("SLASH", isToken: true),
                    ],
                  ),
                ),
                RuleReference("ws", isToken: false),
                RuleReference("power_expr", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 28,
    ),
    GrammarRule(
      name: "power_expr",
      body: Sequence(
        elements: [
          RuleReference("unary_expr", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("ws", isToken: false),
                RuleReference("CARET", isToken: true),
                RuleReference("ws", isToken: false),
                RuleReference("unary_expr", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 29,
    ),
    GrammarRule(
      name: "unary_expr",
      body: Sequence(
        elements: [
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("prefix_op", isToken: false),
                RuleReference("ws", isToken: false),
              ],
            ),
          ),
          RuleReference("postfix_expr", isToken: false),
        ],
      ),
      lineNumber: 30,
    ),
    GrammarRule(
      name: "prefix_op",
      body: Alternation(
        choices: [
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
        ],
      ),
      lineNumber: 31,
    ),
    GrammarRule(
      name: "postfix_expr",
      body: Sequence(
        elements: [
          RuleReference("primary", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("ws", isToken: false),
                RuleReference("PERCENT", isToken: true),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 32,
    ),
    GrammarRule(
      name: "primary",
      body: Alternation(
        choices: [
          RuleReference("parenthesized_expression", isToken: false),
          RuleReference("constant", isToken: false),
          RuleReference("function_call", isToken: false),
          RuleReference("structure_reference", isToken: false),
          RuleReference("reference_expression", isToken: false),
          RuleReference("bang_reference", isToken: false),
          RuleReference("bang_name", isToken: false),
          RuleReference("name_reference", isToken: false),
        ],
      ),
      lineNumber: 34,
    ),
    GrammarRule(
      name: "parenthesized_expression",
      body: Sequence(
        elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("ws", isToken: false),
          RuleReference("expression", isToken: false),
          RuleReference("ws", isToken: false),
          RuleReference("RPAREN", isToken: true),
        ],
      ),
      lineNumber: 43,
    ),
    GrammarRule(
      name: "constant",
      body: Alternation(
        choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("KEYWORD", isToken: true),
          RuleReference("ERROR_CONSTANT", isToken: true),
          RuleReference("array_constant", isToken: false),
        ],
      ),
      lineNumber: 45,
    ),
    GrammarRule(
      name: "array_constant",
      body: Sequence(
        elements: [
          RuleReference("LBRACE", isToken: true),
          RuleReference("ws", isToken: false),
          RuleReference("array_row", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("ws", isToken: false),
                RuleReference("SEMICOLON", isToken: true),
                RuleReference("ws", isToken: false),
                RuleReference("array_row", isToken: false),
              ],
            ),
          ),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("ws", isToken: false),
                RuleReference("SEMICOLON", isToken: true),
              ],
            ),
          ),
          RuleReference("ws", isToken: false),
          RuleReference("RBRACE", isToken: true),
        ],
      ),
      lineNumber: 47,
    ),
    GrammarRule(
      name: "array_row",
      body: Sequence(
        elements: [
          RuleReference("array_item", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("ws", isToken: false),
                RuleReference("COMMA", isToken: true),
                RuleReference("ws", isToken: false),
                RuleReference("array_item", isToken: false),
              ],
            ),
          ),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("ws", isToken: false),
                RuleReference("COMMA", isToken: true),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 48,
    ),
    GrammarRule(
      name: "array_item",
      body: Alternation(
        choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("KEYWORD", isToken: true),
          RuleReference("ERROR_CONSTANT", isToken: true),
        ],
      ),
      lineNumber: 49,
    ),
    GrammarRule(
      name: "function_call",
      body: Sequence(
        elements: [
          RuleReference("function_name", isToken: false),
          RuleReference("LPAREN", isToken: true),
          RuleReference("ws", isToken: false),
          Optional(
            element: RuleReference("function_argument_list", isToken: false),
          ),
          RuleReference("ws", isToken: false),
          RuleReference("RPAREN", isToken: true),
        ],
      ),
      lineNumber: 51,
    ),
    GrammarRule(
      name: "function_name",
      body: Alternation(
        choices: [
          RuleReference("FUNCTION_NAME", isToken: true),
          RuleReference("NAME", isToken: true),
        ],
      ),
      lineNumber: 52,
    ),
    GrammarRule(
      name: "function_argument_list",
      body: Sequence(
        elements: [
          RuleReference("function_argument", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("ws", isToken: false),
                RuleReference("COMMA", isToken: true),
                RuleReference("ws", isToken: false),
                RuleReference("function_argument", isToken: false),
              ],
            ),
          ),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("ws", isToken: false),
                RuleReference("COMMA", isToken: true),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 53,
    ),
    GrammarRule(
      name: "function_argument",
      body: Optional(element: RuleReference("expression", isToken: false)),
      lineNumber: 54,
    ),
    GrammarRule(
      name: "reference_expression",
      body: RuleReference("union_reference", isToken: false),
      lineNumber: 56,
    ),
    GrammarRule(
      name: "union_reference",
      body: Sequence(
        elements: [
          RuleReference("intersection_reference", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("ws", isToken: false),
                RuleReference("COMMA", isToken: true),
                RuleReference("ws", isToken: false),
                RuleReference("intersection_reference", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 57,
    ),
    GrammarRule(
      name: "intersection_reference",
      body: Sequence(
        elements: [
          RuleReference("range_reference", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("req_space", isToken: false),
                RuleReference("range_reference", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 58,
    ),
    GrammarRule(
      name: "range_reference",
      body: Sequence(
        elements: [
          RuleReference("reference_primary", isToken: false),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("ws", isToken: false),
                RuleReference("COLON", isToken: true),
                RuleReference("ws", isToken: false),
                RuleReference("reference_primary", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 59,
    ),
    GrammarRule(
      name: "reference_primary",
      body: Alternation(
        choices: [
          RuleReference("parenthesized_reference", isToken: false),
          RuleReference("prefixed_reference", isToken: false),
          RuleReference("external_reference", isToken: false),
          RuleReference("structure_reference", isToken: false),
          RuleReference("a1_reference", isToken: false),
          RuleReference("bang_reference", isToken: false),
          RuleReference("bang_name", isToken: false),
          RuleReference("name_reference", isToken: false),
        ],
      ),
      lineNumber: 61,
    ),
    GrammarRule(
      name: "parenthesized_reference",
      body: Sequence(
        elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("ws", isToken: false),
          RuleReference("reference_expression", isToken: false),
          RuleReference("ws", isToken: false),
          RuleReference("RPAREN", isToken: true),
        ],
      ),
      lineNumber: 70,
    ),
    GrammarRule(
      name: "prefixed_reference",
      body: Sequence(
        elements: [
          RuleReference("REF_PREFIX", isToken: true),
          Group(
            element: Alternation(
              choices: [
                RuleReference("a1_reference", isToken: false),
                RuleReference("name_reference", isToken: false),
                RuleReference("structure_reference", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 71,
    ),
    GrammarRule(
      name: "external_reference",
      body: RuleReference("REF_PREFIX", isToken: true),
      lineNumber: 72,
    ),
    GrammarRule(
      name: "bang_reference",
      body: Sequence(
        elements: [
          RuleReference("BANG", isToken: true),
          Group(
            element: Alternation(
              choices: [
                RuleReference("CELL", isToken: true),
                RuleReference("COLUMN_REF", isToken: true),
                RuleReference("ROW_REF", isToken: true),
                RuleReference("NUMBER", isToken: true),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 73,
    ),
    GrammarRule(
      name: "bang_name",
      body: Sequence(
        elements: [
          RuleReference("BANG", isToken: true),
          RuleReference("name_reference", isToken: false),
        ],
      ),
      lineNumber: 74,
    ),
    GrammarRule(
      name: "name_reference",
      body: RuleReference("NAME", isToken: true),
      lineNumber: 75,
    ),
    GrammarRule(
      name: "column_reference",
      body: Sequence(
        elements: [
          Optional(element: RuleReference("DOLLAR", isToken: true)),
          Group(
            element: Alternation(
              choices: [
                RuleReference("COLUMN_REF", isToken: true),
                RuleReference("NAME", isToken: true),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 77,
    ),
    GrammarRule(
      name: "row_reference",
      body: Sequence(
        elements: [
          Optional(element: RuleReference("DOLLAR", isToken: true)),
          Group(
            element: Alternation(
              choices: [
                RuleReference("ROW_REF", isToken: true),
                RuleReference("NUMBER", isToken: true),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 78,
    ),
    GrammarRule(
      name: "a1_reference",
      body: Alternation(
        choices: [
          RuleReference("CELL", isToken: true),
          RuleReference("column_reference", isToken: false),
          RuleReference("row_reference", isToken: false),
          RuleReference("COLUMN_REF", isToken: true),
          RuleReference("ROW_REF", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("NUMBER", isToken: true),
        ],
      ),
      lineNumber: 80,
    ),
    GrammarRule(
      name: "structure_reference",
      body: Sequence(
        elements: [
          Optional(element: RuleReference("table_name", isToken: false)),
          RuleReference("intra_table_reference", isToken: false),
        ],
      ),
      lineNumber: 82,
    ),
    GrammarRule(
      name: "table_name",
      body: Alternation(
        choices: [
          RuleReference("TABLE_NAME", isToken: true),
          RuleReference("NAME", isToken: true),
        ],
      ),
      lineNumber: 83,
    ),
    GrammarRule(
      name: "intra_table_reference",
      body: Alternation(
        choices: [
          RuleReference("STRUCTURED_KEYWORD", isToken: true),
          RuleReference("structured_column_range", isToken: false),
          Sequence(
            elements: [
              RuleReference("LBRACKET", isToken: true),
              RuleReference("ws", isToken: false),
              Optional(
                element: RuleReference(
                  "inner_structure_reference",
                  isToken: false,
                ),
              ),
              RuleReference("ws", isToken: false),
              RuleReference("RBRACKET", isToken: true),
            ],
          ),
        ],
      ),
      lineNumber: 84,
    ),
    GrammarRule(
      name: "inner_structure_reference",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              RuleReference("structured_keyword_list", isToken: false),
              Optional(
                element: Sequence(
                  elements: [
                    RuleReference("ws", isToken: false),
                    RuleReference("COMMA", isToken: true),
                    RuleReference("ws", isToken: false),
                    RuleReference("structured_column_range", isToken: false),
                  ],
                ),
              ),
            ],
          ),
          RuleReference("structured_column_range", isToken: false),
        ],
      ),
      lineNumber: 87,
    ),
    GrammarRule(
      name: "structured_keyword_list",
      body: Sequence(
        elements: [
          RuleReference("STRUCTURED_KEYWORD", isToken: true),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("ws", isToken: false),
                RuleReference("COMMA", isToken: true),
                RuleReference("ws", isToken: false),
                RuleReference("STRUCTURED_KEYWORD", isToken: true),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 89,
    ),
    GrammarRule(
      name: "structured_column_range",
      body: Sequence(
        elements: [
          RuleReference("structured_column", isToken: false),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("ws", isToken: false),
                RuleReference("COLON", isToken: true),
                RuleReference("ws", isToken: false),
                RuleReference("structured_column", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 90,
    ),
    GrammarRule(
      name: "structured_column",
      body: Alternation(
        choices: [
          RuleReference("STRUCTURED_COLUMN", isToken: true),
          Sequence(
            elements: [
              RuleReference("AT", isToken: true),
              RuleReference("STRUCTURED_COLUMN", isToken: true),
            ],
          ),
        ],
      ),
      lineNumber: 91,
    ),
  ],
);
