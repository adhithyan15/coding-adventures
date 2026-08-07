// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/algol/algol60.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
      name: "program",
      body: RuleReference("block", isToken: false),
      lineNumber: 47,
    ),
    GrammarRule(
      name: "block",
      body: Sequence(
        elements: [
          Literal("begin"),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("declaration", isToken: false),
                RuleReference("SEMICOLON", isToken: true),
              ],
            ),
          ),
          Repetition(element: RuleReference("SEMICOLON", isToken: true)),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("statement", isToken: false),
                Repetition(
                  element: Sequence(
                    elements: [
                      RuleReference("SEMICOLON", isToken: true),
                      Optional(
                        element: RuleReference("statement", isToken: false),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
          Literal("end"),
        ],
      ),
      lineNumber: 53,
    ),
    GrammarRule(
      name: "declaration",
      body: Alternation(
        choices: [
          RuleReference("type_decl", isToken: false),
          RuleReference("own_decl", isToken: false),
          RuleReference("own_array_decl", isToken: false),
          RuleReference("array_decl", isToken: false),
          RuleReference("switch_decl", isToken: false),
          RuleReference("procedure_decl", isToken: false),
        ],
      ),
      lineNumber: 60,
    ),
    GrammarRule(
      name: "type_decl",
      body: Sequence(
        elements: [
          RuleReference("type", isToken: false),
          RuleReference("ident_list", isToken: false),
        ],
      ),
      lineNumber: 71,
    ),
    GrammarRule(
      name: "own_decl",
      body: Sequence(
        elements: [
          Literal("own"),
          RuleReference("type", isToken: false),
          RuleReference("ident_list", isToken: false),
        ],
      ),
      lineNumber: 76,
    ),
    GrammarRule(
      name: "own_array_decl",
      body: Sequence(
        elements: [
          Literal("own"),
          Optional(element: RuleReference("type", isToken: false)),
          Literal("array"),
          RuleReference("array_segment", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("array_segment", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 81,
    ),
    GrammarRule(
      name: "type",
      body: Alternation(
        choices: [
          Literal("integer"),
          Literal("real"),
          Literal("boolean"),
          Literal("string"),
        ],
      ),
      lineNumber: 83,
    ),
    GrammarRule(
      name: "ident_list",
      body: Sequence(
        elements: [
          RuleReference("NAME", isToken: true),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("NAME", isToken: true),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 85,
    ),
    GrammarRule(
      name: "array_decl",
      body: Sequence(
        elements: [
          Optional(element: RuleReference("type", isToken: false)),
          Literal("array"),
          RuleReference("array_segment", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("array_segment", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 93,
    ),
    GrammarRule(
      name: "array_segment",
      body: Sequence(
        elements: [
          RuleReference("ident_list", isToken: false),
          RuleReference("LBRACKET", isToken: true),
          RuleReference("bound_pair", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("bound_pair", isToken: false),
              ],
            ),
          ),
          RuleReference("RBRACKET", isToken: true),
        ],
      ),
      lineNumber: 95,
    ),
    GrammarRule(
      name: "bound_pair",
      body: Sequence(
        elements: [
          RuleReference("arith_expr", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("arith_expr", isToken: false),
        ],
      ),
      lineNumber: 99,
    ),
    GrammarRule(
      name: "switch_decl",
      body: Sequence(
        elements: [
          Literal("switch"),
          RuleReference("NAME", isToken: true),
          RuleReference("ASSIGN", isToken: true),
          RuleReference("switch_list", isToken: false),
        ],
      ),
      lineNumber: 104,
    ),
    GrammarRule(
      name: "switch_list",
      body: Sequence(
        elements: [
          RuleReference("desig_expr", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("desig_expr", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 106,
    ),
    GrammarRule(
      name: "procedure_decl",
      body: Sequence(
        elements: [
          Optional(element: RuleReference("type", isToken: false)),
          Literal("procedure"),
          RuleReference("NAME", isToken: true),
          Optional(element: RuleReference("formal_params", isToken: false)),
          RuleReference("SEMICOLON", isToken: true),
          Optional(element: RuleReference("value_part", isToken: false)),
          Repetition(element: RuleReference("spec_part", isToken: false)),
          RuleReference("proc_body", isToken: false),
        ],
      ),
      lineNumber: 113,
    ),
    GrammarRule(
      name: "formal_params",
      body: Sequence(
        elements: [
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("ident_list", isToken: false)),
          RuleReference("RPAREN", isToken: true),
        ],
      ),
      lineNumber: 118,
    ),
    GrammarRule(
      name: "value_part",
      body: Sequence(
        elements: [
          Literal("value"),
          RuleReference("ident_list", isToken: false),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 123,
    ),
    GrammarRule(
      name: "spec_part",
      body: Sequence(
        elements: [
          RuleReference("specifier", isToken: false),
          RuleReference("ident_list", isToken: false),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 130,
    ),
    GrammarRule(
      name: "specifier",
      body: Alternation(
        choices: [
          Sequence(
            elements: [RuleReference("type", isToken: false), Literal("array")],
          ),
          Sequence(
            elements: [
              RuleReference("type", isToken: false),
              Literal("procedure"),
            ],
          ),
          Literal("array"),
          Literal("label"),
          Literal("switch"),
          Literal("procedure"),
          RuleReference("type", isToken: false),
        ],
      ),
      lineNumber: 132,
    ),
    GrammarRule(
      name: "proc_body",
      body: Alternation(
        choices: [
          RuleReference("block", isToken: false),
          RuleReference("statement", isToken: false),
        ],
      ),
      lineNumber: 140,
    ),
    GrammarRule(
      name: "statement",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              Repetition(
                element: Sequence(
                  elements: [
                    RuleReference("label", isToken: false),
                    RuleReference("COLON", isToken: true),
                  ],
                ),
              ),
              RuleReference("unlabeled_stmt", isToken: false),
            ],
          ),
          Sequence(
            elements: [
              Repetition(
                element: Sequence(
                  elements: [
                    RuleReference("label", isToken: false),
                    RuleReference("COLON", isToken: true),
                  ],
                ),
              ),
              RuleReference("cond_stmt", isToken: false),
            ],
          ),
        ],
      ),
      lineNumber: 152,
    ),
    GrammarRule(
      name: "label",
      body: Alternation(
        choices: [
          RuleReference("NAME", isToken: true),
          RuleReference("INTEGER_LIT", isToken: true),
        ],
      ),
      lineNumber: 155,
    ),
    GrammarRule(
      name: "unlabeled_stmt",
      body: Alternation(
        choices: [
          RuleReference("assign_stmt", isToken: false),
          RuleReference("dummy_stmt", isToken: false),
          RuleReference("goto_stmt", isToken: false),
          RuleReference("proc_stmt", isToken: false),
          RuleReference("compound_stmt", isToken: false),
          RuleReference("block", isToken: false),
          RuleReference("for_stmt", isToken: false),
        ],
      ),
      lineNumber: 165,
    ),
    GrammarRule(
      name: "dummy_stmt",
      body: Alternation(
        choices: [
          PositiveLookahead(element: RuleReference("SEMICOLON", isToken: true)),
          PositiveLookahead(element: Literal("end")),
          PositiveLookahead(element: Literal("else")),
        ],
      ),
      lineNumber: 175,
    ),
    GrammarRule(
      name: "cond_stmt",
      body: Sequence(
        elements: [
          Literal("if"),
          RuleReference("bool_expr", isToken: false),
          Literal("then"),
          RuleReference("unlabeled_stmt", isToken: false),
          Optional(
            element: Sequence(
              elements: [
                Literal("else"),
                RuleReference("statement", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 181,
    ),
    GrammarRule(
      name: "compound_stmt",
      body: Sequence(
        elements: [
          Literal("begin"),
          Repetition(element: RuleReference("SEMICOLON", isToken: true)),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("statement", isToken: false),
                Repetition(
                  element: Sequence(
                    elements: [
                      RuleReference("SEMICOLON", isToken: true),
                      Optional(
                        element: RuleReference("statement", isToken: false),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
          Literal("end"),
        ],
      ),
      lineNumber: 185,
    ),
    GrammarRule(
      name: "assign_stmt",
      body: Sequence(
        elements: [
          RuleReference("left_part", isToken: false),
          Repetition(element: RuleReference("left_part", isToken: false)),
          RuleReference("expression", isToken: false),
        ],
      ),
      lineNumber: 191,
    ),
    GrammarRule(
      name: "left_part",
      body: Sequence(
        elements: [
          RuleReference("variable", isToken: false),
          RuleReference("ASSIGN", isToken: true),
        ],
      ),
      lineNumber: 193,
    ),
    GrammarRule(
      name: "goto_stmt",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              Literal("goto"),
              RuleReference("desig_expr", isToken: false),
            ],
          ),
          Sequence(
            elements: [
              Literal("go"),
              Literal("to"),
              RuleReference("desig_expr", isToken: false),
            ],
          ),
        ],
      ),
      lineNumber: 197,
    ),
    GrammarRule(
      name: "proc_stmt",
      body: Sequence(
        elements: [
          RuleReference("NAME", isToken: true),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("LPAREN", isToken: true),
                Optional(
                  element: RuleReference("actual_params", isToken: false),
                ),
                RuleReference("RPAREN", isToken: true),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 202,
    ),
    GrammarRule(
      name: "actual_params",
      body: Sequence(
        elements: [
          RuleReference("expression", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("expression", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 204,
    ),
    GrammarRule(
      name: "for_stmt",
      body: Sequence(
        elements: [
          Literal("for"),
          RuleReference("variable", isToken: false),
          RuleReference("ASSIGN", isToken: true),
          RuleReference("for_list", isToken: false),
          Literal("do"),
          RuleReference("statement", isToken: false),
        ],
      ),
      lineNumber: 212,
    ),
    GrammarRule(
      name: "for_list",
      body: Sequence(
        elements: [
          RuleReference("for_elem", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("for_elem", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 214,
    ),
    GrammarRule(
      name: "for_elem",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              RuleReference("arith_expr", isToken: false),
              Literal("step"),
              RuleReference("arith_expr", isToken: false),
              Literal("until"),
              RuleReference("arith_expr", isToken: false),
            ],
          ),
          Sequence(
            elements: [
              RuleReference("arith_expr", isToken: false),
              Literal("while"),
              RuleReference("bool_expr", isToken: false),
            ],
          ),
          RuleReference("arith_expr", isToken: false),
        ],
      ),
      lineNumber: 218,
    ),
    GrammarRule(
      name: "expression",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              Literal("if"),
              RuleReference("bool_expr", isToken: false),
              Literal("then"),
              RuleReference("expression", isToken: false),
              Literal("else"),
              RuleReference("expression", isToken: false),
            ],
          ),
          RuleReference("expr_eqv", isToken: false),
        ],
      ),
      lineNumber: 250,
    ),
    GrammarRule(
      name: "expr_eqv",
      body: Sequence(
        elements: [
          RuleReference("expr_impl", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal("eqv"),
                RuleReference("expr_impl", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 253,
    ),
    GrammarRule(
      name: "expr_impl",
      body: Sequence(
        elements: [
          RuleReference("expr_or", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal("impl"),
                RuleReference("expr_or", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 254,
    ),
    GrammarRule(
      name: "expr_or",
      body: Sequence(
        elements: [
          RuleReference("expr_and", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal("or"),
                RuleReference("expr_and", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 255,
    ),
    GrammarRule(
      name: "expr_and",
      body: Sequence(
        elements: [
          RuleReference("expr_not", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal("and"),
                RuleReference("expr_not", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 256,
    ),
    GrammarRule(
      name: "expr_not",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              Literal("not"),
              RuleReference("expr_not", isToken: false),
            ],
          ),
          RuleReference("expr_cmp", isToken: false),
        ],
      ),
      lineNumber: 257,
    ),
    GrammarRule(
      name: "expr_cmp",
      body: Sequence(
        elements: [
          RuleReference("expr_add", isToken: false),
          Optional(
            element: Sequence(
              elements: [
                Group(
                  element: Alternation(
                    choices: [
                      RuleReference("EQ", isToken: true),
                      RuleReference("NEQ", isToken: true),
                      RuleReference("LT", isToken: true),
                      RuleReference("LEQ", isToken: true),
                      RuleReference("GT", isToken: true),
                      RuleReference("GEQ", isToken: true),
                    ],
                  ),
                ),
                RuleReference("expr_add", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 258,
    ),
    GrammarRule(
      name: "expr_add",
      body: Sequence(
        elements: [
          Optional(
            element: Alternation(
              choices: [
                RuleReference("PLUS", isToken: true),
                RuleReference("MINUS", isToken: true),
              ],
            ),
          ),
          RuleReference("expr_mul", isToken: false),
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
                RuleReference("expr_mul", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 259,
    ),
    GrammarRule(
      name: "expr_mul",
      body: Sequence(
        elements: [
          RuleReference("expr_pow", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Group(
                  element: Alternation(
                    choices: [
                      RuleReference("STAR", isToken: true),
                      RuleReference("SLASH", isToken: true),
                      Literal("div"),
                      Literal("mod"),
                    ],
                  ),
                ),
                RuleReference("expr_pow", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 260,
    ),
    GrammarRule(
      name: "expr_pow",
      body: Sequence(
        elements: [
          RuleReference("expr_atom", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Group(
                  element: Alternation(
                    choices: [
                      RuleReference("CARET", isToken: true),
                      RuleReference("POWER", isToken: true),
                    ],
                  ),
                ),
                RuleReference("expr_atom", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 261,
    ),
    GrammarRule(
      name: "expr_atom",
      body: Alternation(
        choices: [
          RuleReference("INTEGER_LIT", isToken: true),
          RuleReference("REAL_LIT", isToken: true),
          RuleReference("STRING_LIT", isToken: true),
          Literal("true"),
          Literal("false"),
          RuleReference("proc_call", isToken: false),
          RuleReference("variable", isToken: false),
          Sequence(
            elements: [
              RuleReference("LPAREN", isToken: true),
              RuleReference("expression", isToken: false),
              RuleReference("RPAREN", isToken: true),
            ],
          ),
        ],
      ),
      lineNumber: 262,
    ),
    GrammarRule(
      name: "arith_expr",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              Literal("if"),
              RuleReference("bool_expr", isToken: false),
              Literal("then"),
              RuleReference("arith_expr", isToken: false),
              Literal("else"),
              RuleReference("arith_expr", isToken: false),
            ],
          ),
          RuleReference("simple_arith", isToken: false),
        ],
      ),
      lineNumber: 274,
    ),
    GrammarRule(
      name: "simple_arith",
      body: Sequence(
        elements: [
          Optional(
            element: Alternation(
              choices: [
                RuleReference("PLUS", isToken: true),
                RuleReference("MINUS", isToken: true),
              ],
            ),
          ),
          RuleReference("term", isToken: false),
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
                RuleReference("term", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 278,
    ),
    GrammarRule(
      name: "term",
      body: Sequence(
        elements: [
          RuleReference("factor", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Group(
                  element: Alternation(
                    choices: [
                      RuleReference("STAR", isToken: true),
                      RuleReference("SLASH", isToken: true),
                      Literal("div"),
                      Literal("mod"),
                    ],
                  ),
                ),
                RuleReference("factor", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 283,
    ),
    GrammarRule(
      name: "factor",
      body: Sequence(
        elements: [
          RuleReference("primary", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Group(
                  element: Alternation(
                    choices: [
                      RuleReference("CARET", isToken: true),
                      RuleReference("POWER", isToken: true),
                    ],
                  ),
                ),
                RuleReference("primary", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 289,
    ),
    GrammarRule(
      name: "primary",
      body: Alternation(
        choices: [
          RuleReference("INTEGER_LIT", isToken: true),
          RuleReference("REAL_LIT", isToken: true),
          RuleReference("STRING_LIT", isToken: true),
          Literal("true"),
          Literal("false"),
          RuleReference("proc_call", isToken: false),
          RuleReference("variable", isToken: false),
          Sequence(
            elements: [
              RuleReference("LPAREN", isToken: true),
              RuleReference("arith_expr", isToken: false),
              RuleReference("RPAREN", isToken: true),
            ],
          ),
        ],
      ),
      lineNumber: 291,
    ),
    GrammarRule(
      name: "bool_expr",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              Literal("if"),
              RuleReference("bool_expr", isToken: false),
              Literal("then"),
              RuleReference("bool_expr", isToken: false),
              Literal("else"),
              RuleReference("bool_expr", isToken: false),
            ],
          ),
          RuleReference("simple_bool", isToken: false),
        ],
      ),
      lineNumber: 309,
    ),
    GrammarRule(
      name: "simple_bool",
      body: Sequence(
        elements: [
          RuleReference("implication", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal("eqv"),
                RuleReference("implication", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 312,
    ),
    GrammarRule(
      name: "implication",
      body: Sequence(
        elements: [
          RuleReference("bool_term", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal("impl"),
                RuleReference("bool_term", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 314,
    ),
    GrammarRule(
      name: "bool_term",
      body: Sequence(
        elements: [
          RuleReference("bool_factor", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal("or"),
                RuleReference("bool_factor", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 316,
    ),
    GrammarRule(
      name: "bool_factor",
      body: Sequence(
        elements: [
          RuleReference("bool_secondary", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                Literal("and"),
                RuleReference("bool_secondary", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 318,
    ),
    GrammarRule(
      name: "bool_secondary",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              Literal("not"),
              RuleReference("bool_secondary", isToken: false),
            ],
          ),
          RuleReference("bool_primary", isToken: false),
        ],
      ),
      lineNumber: 320,
    ),
    GrammarRule(
      name: "bool_primary",
      body: Alternation(
        choices: [
          RuleReference("relation", isToken: false),
          Literal("true"),
          Literal("false"),
          RuleReference("proc_call", isToken: false),
          RuleReference("variable", isToken: false),
          Sequence(
            elements: [
              RuleReference("LPAREN", isToken: true),
              RuleReference("bool_expr", isToken: false),
              RuleReference("RPAREN", isToken: true),
            ],
          ),
        ],
      ),
      lineNumber: 322,
    ),
    GrammarRule(
      name: "relation",
      body: Sequence(
        elements: [
          RuleReference("simple_arith", isToken: false),
          Group(
            element: Alternation(
              choices: [
                RuleReference("EQ", isToken: true),
                RuleReference("NEQ", isToken: true),
                RuleReference("LT", isToken: true),
                RuleReference("LEQ", isToken: true),
                RuleReference("GT", isToken: true),
                RuleReference("GEQ", isToken: true),
              ],
            ),
          ),
          RuleReference("simple_arith", isToken: false),
        ],
      ),
      lineNumber: 332,
    ),
    GrammarRule(
      name: "desig_expr",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              Literal("if"),
              RuleReference("bool_expr", isToken: false),
              Literal("then"),
              RuleReference("desig_expr", isToken: false),
              Literal("else"),
              RuleReference("desig_expr", isToken: false),
            ],
          ),
          RuleReference("simple_desig", isToken: false),
        ],
      ),
      lineNumber: 337,
    ),
    GrammarRule(
      name: "simple_desig",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              RuleReference("NAME", isToken: true),
              RuleReference("LBRACKET", isToken: true),
              RuleReference("arith_expr", isToken: false),
              RuleReference("RBRACKET", isToken: true),
            ],
          ),
          Sequence(
            elements: [
              RuleReference("LPAREN", isToken: true),
              RuleReference("desig_expr", isToken: false),
              RuleReference("RPAREN", isToken: true),
            ],
          ),
          RuleReference("label", isToken: false),
        ],
      ),
      lineNumber: 340,
    ),
    GrammarRule(
      name: "variable",
      body: Sequence(
        elements: [
          RuleReference("NAME", isToken: true),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("LBRACKET", isToken: true),
                RuleReference("subscripts", isToken: false),
                RuleReference("RBRACKET", isToken: true),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 352,
    ),
    GrammarRule(
      name: "subscripts",
      body: Sequence(
        elements: [
          RuleReference("arith_expr", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("arith_expr", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 354,
    ),
    GrammarRule(
      name: "proc_call",
      body: Sequence(
        elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("actual_params", isToken: false)),
          RuleReference("RPAREN", isToken: true),
        ],
      ),
      lineNumber: 359,
    ),
  ],
);
