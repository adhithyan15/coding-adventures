// AUTO-GENERATED FILE - DO NOT EDIT
// Sources: code/grammars/python/python*.grammar (legacy minor lines + LANG84 release tables); release list and inventory from code/packages/rust/python-parser/src/_grammar.rs
// Regenerate with: grammar-tools compile-grammar <source.grammar> (per version)
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammarLegacyV2_7 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "file",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("statement", isToken: false)
        ])),
        lineNumber: 47),
    GrammarRule(
        name: "statement",
        body: Alternation(choices: [
          RuleReference("compound_stmt", isToken: false),
          RuleReference("simple_stmt", isToken: false)
        ]),
        lineNumber: 64),
    GrammarRule(
        name: "simple_stmt",
        body: Sequence(elements: [
          RuleReference("small_stmt", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("small_stmt", isToken: false)
          ])),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 74),
    GrammarRule(
        name: "small_stmt",
        body: Alternation(choices: [
          RuleReference("print_stmt", isToken: false),
          RuleReference("del_stmt", isToken: false),
          RuleReference("pass_stmt", isToken: false),
          RuleReference("break_stmt", isToken: false),
          RuleReference("continue_stmt", isToken: false),
          RuleReference("return_stmt", isToken: false),
          RuleReference("yield_stmt", isToken: false),
          RuleReference("raise_stmt", isToken: false),
          RuleReference("import_stmt", isToken: false),
          RuleReference("global_stmt", isToken: false),
          RuleReference("exec_stmt", isToken: false),
          RuleReference("assert_stmt", isToken: false),
          RuleReference("assign_stmt", isToken: false)
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "print_stmt",
        body: Sequence(elements: [
          Literal("print"),
          Optional(
              element: Alternation(choices: [
            RuleReference("print_chevron", isToken: false),
            RuleReference("print_args", isToken: false)
          ]))
        ]),
        lineNumber: 116),
    GrammarRule(
        name: "print_chevron",
        body: Sequence(elements: [
          RuleReference("RIGHT_SHIFT", isToken: true),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("print_args", isToken: false)
          ]))
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "print_args",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 120),
    GrammarRule(
        name: "del_stmt",
        body: Sequence(elements: [
          Literal("del"),
          RuleReference("target_list", isToken: false)
        ]),
        lineNumber: 133),
    GrammarRule(name: "pass_stmt", body: Literal("pass"), lineNumber: 145),
    GrammarRule(name: "break_stmt", body: Literal("break"), lineNumber: 156),
    GrammarRule(
        name: "continue_stmt", body: Literal("continue"), lineNumber: 158),
    GrammarRule(
        name: "return_stmt",
        body: Sequence(elements: [
          Literal("return"),
          Optional(element: RuleReference("expression_list", isToken: false))
        ]),
        lineNumber: 170),
    GrammarRule(
        name: "yield_stmt",
        body: Sequence(elements: [
          Literal("yield"),
          Optional(element: RuleReference("expression_list", isToken: false))
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "raise_stmt",
        body: Sequence(elements: [
          Literal("raise"),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Optional(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("expression", isToken: false),
              Optional(
                  element: Sequence(elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("expression", isToken: false)
              ]))
            ]))
          ]))
        ]),
        lineNumber: 205),
    GrammarRule(
        name: "import_stmt",
        body: Alternation(choices: [
          RuleReference("import_name", isToken: false),
          RuleReference("import_from", isToken: false)
        ]),
        lineNumber: 230),
    GrammarRule(
        name: "import_name",
        body: Sequence(elements: [
          Literal("import"),
          RuleReference("dotted_as_names", isToken: false)
        ]),
        lineNumber: 232),
    GrammarRule(
        name: "import_from",
        body: Sequence(elements: [
          Literal("from"),
          RuleReference("import_from_module", isToken: false),
          Literal("import"),
          Group(
              element: Alternation(choices: [
            RuleReference("STAR", isToken: true),
            Sequence(elements: [
              RuleReference("LPAREN", isToken: true),
              RuleReference("import_as_names", isToken: false),
              RuleReference("RPAREN", isToken: true)
            ]),
            RuleReference("import_as_names", isToken: false)
          ]))
        ]),
        lineNumber: 234),
    GrammarRule(
        name: "import_from_module",
        body: Alternation(choices: [
          Sequence(elements: [
            Repetition(element: RuleReference("DOT", isToken: true)),
            RuleReference("dotted_name", isToken: false)
          ]),
          Repetition(element: RuleReference("DOT", isToken: true))
        ]),
        lineNumber: 241),
    GrammarRule(
        name: "dotted_name",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 244),
    GrammarRule(
        name: "dotted_as_names",
        body: Sequence(elements: [
          RuleReference("dotted_as_name", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("dotted_as_name", isToken: false)
          ]))
        ]),
        lineNumber: 247),
    GrammarRule(
        name: "dotted_as_name",
        body: Sequence(elements: [
          RuleReference("dotted_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 248),
    GrammarRule(
        name: "import_as_names",
        body: Sequence(elements: [
          RuleReference("import_as_name", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("import_as_name", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 250),
    GrammarRule(
        name: "import_as_name",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 251),
    GrammarRule(
        name: "global_stmt",
        body: Sequence(elements: [
          Literal("global"),
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 263),
    GrammarRule(
        name: "exec_stmt",
        body: Sequence(elements: [
          Literal("exec"),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("in"),
            RuleReference("expression", isToken: false),
            Optional(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 285),
    GrammarRule(
        name: "assert_stmt",
        body: Sequence(elements: [
          Literal("assert"),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 298),
    GrammarRule(
        name: "assign_stmt",
        body: Sequence(elements: [
          RuleReference("expression_list", isToken: false),
          Optional(element: RuleReference("assign_tail", isToken: false))
        ]),
        lineNumber: 326),
    GrammarRule(
        name: "assign_tail",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("augmented_assign_op", isToken: false),
            RuleReference("expression_list", isToken: false)
          ]),
          Sequence(elements: [
            Group(
                element: Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("expression_list", isToken: false)
            ])),
            Repetition(
                element: Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("expression_list", isToken: false)
            ]))
          ])
        ]),
        lineNumber: 328),
    GrammarRule(
        name: "augmented_assign_op",
        body: Alternation(choices: [
          RuleReference("PLUS_EQUALS", isToken: true),
          RuleReference("MINUS_EQUALS", isToken: true),
          RuleReference("STAR_EQUALS", isToken: true),
          RuleReference("SLASH_EQUALS", isToken: true),
          RuleReference("FLOOR_DIV_EQUALS", isToken: true),
          RuleReference("PERCENT_EQUALS", isToken: true),
          RuleReference("DOUBLE_STAR_EQUALS", isToken: true),
          RuleReference("AMP_EQUALS", isToken: true),
          RuleReference("PIPE_EQUALS", isToken: true),
          RuleReference("CARET_EQUALS", isToken: true),
          RuleReference("LEFT_SHIFT_EQUALS", isToken: true),
          RuleReference("RIGHT_SHIFT_EQUALS", isToken: true)
        ]),
        lineNumber: 331),
    GrammarRule(
        name: "target_list",
        body: Sequence(elements: [
          RuleReference("target", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("target", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 347),
    GrammarRule(
        name: "target",
        body: Alternation(choices: [
          RuleReference("NAME", isToken: true),
          Sequence(elements: [
            RuleReference("primary", isToken: false),
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("primary", isToken: false),
            RuleReference("LBRACKET", isToken: true),
            RuleReference("subscript", isToken: false),
            RuleReference("RBRACKET", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("target_list", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LBRACKET", isToken: true),
            RuleReference("target_list", isToken: false),
            RuleReference("RBRACKET", isToken: true)
          ])
        ]),
        lineNumber: 349),
    GrammarRule(
        name: "compound_stmt",
        body: Alternation(choices: [
          RuleReference("if_stmt", isToken: false),
          RuleReference("while_stmt", isToken: false),
          RuleReference("for_stmt", isToken: false),
          RuleReference("try_stmt", isToken: false),
          RuleReference("with_stmt", isToken: false),
          RuleReference("def_stmt", isToken: false),
          RuleReference("class_stmt", isToken: false)
        ]),
        lineNumber: 359),
    GrammarRule(
        name: "if_stmt",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("elif"),
            RuleReference("expression", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ]))
        ]),
        lineNumber: 382),
    GrammarRule(
        name: "while_stmt",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ]))
        ]),
        lineNumber: 399),
    GrammarRule(
        name: "for_stmt",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("target_list", isToken: false),
          Literal("in"),
          RuleReference("expression_list", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ]))
        ]),
        lineNumber: 421),
    GrammarRule(
        name: "try_stmt",
        body: Sequence(elements: [
          Literal("try"),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Group(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("except_clauses", isToken: false),
              Optional(
                  element: Sequence(elements: [
                Literal("else"),
                RuleReference("COLON", isToken: true),
                RuleReference("suite", isToken: false)
              ])),
              Optional(
                  element: Sequence(elements: [
                Literal("finally"),
                RuleReference("COLON", isToken: true),
                RuleReference("suite", isToken: false)
              ]))
            ]),
            Sequence(elements: [
              Literal("finally"),
              RuleReference("COLON", isToken: true),
              RuleReference("suite", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 455),
    GrammarRule(
        name: "except_clauses",
        body: Sequence(elements: [
          RuleReference("except_clause", isToken: false),
          Repetition(element: RuleReference("except_clause", isToken: false))
        ]),
        lineNumber: 459),
    GrammarRule(
        name: "except_clause",
        body: Sequence(elements: [
          Literal("except"),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Optional(
                element: Sequence(elements: [
              Group(
                  element: Alternation(choices: [
                RuleReference("COMMA", isToken: true),
                Literal("as")
              ])),
              RuleReference("NAME", isToken: true)
            ]))
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 466),
    GrammarRule(
        name: "with_stmt",
        body: Sequence(elements: [
          Literal("with"),
          RuleReference("with_item", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("with_item", isToken: false)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 485),
    GrammarRule(
        name: "with_item",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("target", isToken: false)
          ]))
        ]),
        lineNumber: 487),
    GrammarRule(
        name: "def_stmt",
        body: Sequence(elements: [
          Repetition(element: RuleReference("decorator", isToken: false)),
          Literal("def"),
          RuleReference("NAME", isToken: true),
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("parameters", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 512),
    GrammarRule(
        name: "class_stmt",
        body: Sequence(elements: [
          Repetition(element: RuleReference("decorator", isToken: false)),
          Literal("class"),
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("expression_list", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 540),
    GrammarRule(
        name: "decorator",
        body: Sequence(elements: [
          RuleReference("AT", isToken: true),
          RuleReference("dotted_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 555),
    GrammarRule(
        name: "suite",
        body: Alternation(choices: [
          RuleReference("simple_stmt", isToken: false),
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("INDENT", isToken: true),
            Repetition(element: RuleReference("statement", isToken: false)),
            RuleReference("DEDENT", isToken: true)
          ])
        ]),
        lineNumber: 571),
    GrammarRule(
        name: "parameters",
        body: Sequence(elements: [
          RuleReference("parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("parameter", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 597),
    GrammarRule(
        name: "parameter",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("fpdef", isToken: false),
            Optional(
                element: Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ])
        ]),
        lineNumber: 599),
    GrammarRule(
        name: "fpdef",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("fpdef_list", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 609),
    GrammarRule(
        name: "fpdef_list",
        body: Sequence(elements: [
          RuleReference("fpdef_item", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("fpdef_item", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 611),
    GrammarRule(
        name: "fpdef_item",
        body: Alternation(choices: [
          RuleReference("NAME", isToken: true),
          RuleReference("fpdef", isToken: false)
        ]),
        lineNumber: 613),
    GrammarRule(
        name: "expression_list",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 646),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("lambda_expr", isToken: false),
          Sequence(elements: [
            RuleReference("or_expr", isToken: false),
            Optional(
                element: Sequence(elements: [
              Literal("if"),
              RuleReference("or_expr", isToken: false),
              Literal("else"),
              RuleReference("expression", isToken: false)
            ]))
          ])
        ]),
        lineNumber: 658),
    GrammarRule(
        name: "lambda_expr",
        body: Sequence(elements: [
          Literal("lambda"),
          Optional(element: RuleReference("lambda_params", isToken: false)),
          RuleReference("COLON", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 671),
    GrammarRule(
        name: "lambda_params",
        body: Sequence(elements: [
          RuleReference("lambda_param", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("lambda_param", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 673),
    GrammarRule(
        name: "lambda_param",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ])
        ]),
        lineNumber: 675),
    GrammarRule(
        name: "or_expr",
        body: Sequence(elements: [
          RuleReference("and_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("or"),
            RuleReference("and_expr", isToken: false)
          ]))
        ]),
        lineNumber: 688),
    GrammarRule(
        name: "and_expr",
        body: Sequence(elements: [
          RuleReference("not_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            RuleReference("not_expr", isToken: false)
          ]))
        ]),
        lineNumber: 698),
    GrammarRule(
        name: "not_expr",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("not"),
            RuleReference("not_expr", isToken: false)
          ]),
          RuleReference("comparison", isToken: false)
        ]),
        lineNumber: 708),
    GrammarRule(
        name: "comparison",
        body: Sequence(elements: [
          RuleReference("bitwise_or", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("comp_op", isToken: false),
            RuleReference("bitwise_or", isToken: false)
          ]))
        ]),
        lineNumber: 734),
    GrammarRule(
        name: "comp_op",
        body: Alternation(choices: [
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("DIAMOND", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          Literal("in"),
          Sequence(elements: [Literal("not"), Literal("in")]),
          Literal("is"),
          Sequence(elements: [Literal("is"), Literal("not")])
        ]),
        lineNumber: 736),
    GrammarRule(
        name: "bitwise_or",
        body: Sequence(elements: [
          RuleReference("bitwise_xor", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("PIPE", isToken: true),
            RuleReference("bitwise_xor", isToken: false)
          ]))
        ]),
        lineNumber: 752),
    GrammarRule(
        name: "bitwise_xor",
        body: Sequence(elements: [
          RuleReference("bitwise_and", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("CARET", isToken: true),
            RuleReference("bitwise_and", isToken: false)
          ]))
        ]),
        lineNumber: 753),
    GrammarRule(
        name: "bitwise_and",
        body: Sequence(elements: [
          RuleReference("shift", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("AMP", isToken: true),
            RuleReference("shift", isToken: false)
          ]))
        ]),
        lineNumber: 754),
    GrammarRule(
        name: "shift",
        body: Sequence(elements: [
          RuleReference("arith", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("LEFT_SHIFT", isToken: true),
              RuleReference("RIGHT_SHIFT", isToken: true)
            ])),
            RuleReference("arith", isToken: false)
          ]))
        ]),
        lineNumber: 765),
    GrammarRule(
        name: "arith",
        body: Sequence(elements: [
          RuleReference("term", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("term", isToken: false)
          ]))
        ]),
        lineNumber: 774),
    GrammarRule(
        name: "term",
        body: Sequence(elements: [
          RuleReference("factor", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true),
              RuleReference("FLOOR_DIV", isToken: true),
              RuleReference("PERCENT", isToken: true)
            ])),
            RuleReference("factor", isToken: false)
          ]))
        ]),
        lineNumber: 791),
    GrammarRule(
        name: "factor",
        body: Alternation(choices: [
          Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true),
              RuleReference("TILDE", isToken: true)
            ])),
            RuleReference("factor", isToken: false)
          ]),
          RuleReference("power", isToken: false)
        ]),
        lineNumber: 803),
    GrammarRule(
        name: "power",
        body: Sequence(elements: [
          RuleReference("primary", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("factor", isToken: false)
          ]))
        ]),
        lineNumber: 817),
    GrammarRule(
        name: "primary",
        body: Sequence(elements: [
          RuleReference("atom", isToken: false),
          Repetition(element: RuleReference("suffix", isToken: false))
        ]),
        lineNumber: 835),
    GrammarRule(
        name: "suffix",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LBRACKET", isToken: true),
            RuleReference("subscript", isToken: false),
            RuleReference("RBRACKET", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 837),
    GrammarRule(
        name: "subscript",
        body: Alternation(choices: [
          RuleReference("expression", isToken: false),
          Sequence(elements: [
            Optional(element: RuleReference("expression", isToken: false)),
            RuleReference("COLON", isToken: true),
            Optional(element: RuleReference("expression", isToken: false)),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              Optional(element: RuleReference("expression", isToken: false))
            ]))
          ])
        ]),
        lineNumber: 854),
    GrammarRule(
        name: "atom",
        body: Alternation(choices: [
          RuleReference("INT", isToken: true),
          RuleReference("FLOAT", isToken: true),
          Sequence(elements: [
            RuleReference("STRING", isToken: true),
            Repetition(element: RuleReference("STRING", isToken: true))
          ]),
          RuleReference("NAME", isToken: true),
          RuleReference("list_expr", isToken: false),
          RuleReference("dict_or_set_expr", isToken: false),
          RuleReference("paren_expr", isToken: false),
          RuleReference("backtick_expr", isToken: false)
        ]),
        lineNumber: 869),
    GrammarRule(
        name: "backtick_expr",
        body: Sequence(elements: [
          RuleReference("BACKTICK", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("BACKTICK", isToken: true)
        ]),
        lineNumber: 898),
    GrammarRule(
        name: "list_expr",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(element: RuleReference("list_body", isToken: false)),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 913),
    GrammarRule(
        name: "list_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("expression", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])
        ]),
        lineNumber: 915),
    GrammarRule(
        name: "dict_or_set_expr",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(element: RuleReference("dict_or_set_body", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 941),
    GrammarRule(
        name: "dict_or_set_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("dict_entry", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("dict_entry", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("dict_entry", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("expression", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])
        ]),
        lineNumber: 943),
    GrammarRule(
        name: "dict_entry",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 948),
    GrammarRule(
        name: "paren_expr",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("paren_body", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 972),
    GrammarRule(
        name: "paren_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("COMMA", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("expression", isToken: false),
              Repetition(
                  element: Sequence(elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("expression", isToken: false)
              ])),
              Optional(element: RuleReference("COMMA", isToken: true))
            ]))
          ]),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 974),
    GrammarRule(
        name: "comp_clause",
        body: Sequence(elements: [
          RuleReference("comp_for", isToken: false),
          Repetition(
              element: Alternation(choices: [
            RuleReference("comp_for", isToken: false),
            RuleReference("comp_if", isToken: false)
          ]))
        ]),
        lineNumber: 1003),
    GrammarRule(
        name: "comp_for",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("target_list", isToken: false),
          Literal("in"),
          RuleReference("or_expr", isToken: false)
        ]),
        lineNumber: 1005),
    GrammarRule(
        name: "comp_if",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("or_expr", isToken: false)
        ]),
        lineNumber: 1007),
    GrammarRule(
        name: "arguments",
        body: Sequence(elements: [
          RuleReference("argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("argument", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 1031),
    GrammarRule(
        name: "argument",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 1033),
  ],
);

final parserGrammarLegacyV3_0 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "file",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("statement", isToken: false)
        ])),
        lineNumber: 103),
    GrammarRule(
        name: "statement",
        body: Alternation(choices: [
          RuleReference("compound_stmt", isToken: false),
          RuleReference("simple_stmt", isToken: false)
        ]),
        lineNumber: 117),
    GrammarRule(
        name: "simple_stmt",
        body: Sequence(elements: [
          RuleReference("small_stmt", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("small_stmt", isToken: false)
          ])),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 121),
    GrammarRule(
        name: "small_stmt",
        body: Alternation(choices: [
          RuleReference("return_stmt", isToken: false),
          RuleReference("yield_stmt", isToken: false),
          RuleReference("raise_stmt", isToken: false),
          RuleReference("break_stmt", isToken: false),
          RuleReference("continue_stmt", isToken: false),
          RuleReference("pass_stmt", isToken: false),
          RuleReference("import_stmt", isToken: false),
          RuleReference("from_import_stmt", isToken: false),
          RuleReference("global_stmt", isToken: false),
          RuleReference("nonlocal_stmt", isToken: false),
          RuleReference("del_stmt", isToken: false),
          RuleReference("assert_stmt", isToken: false),
          RuleReference("assign_stmt", isToken: false)
        ]),
        lineNumber: 123),
    GrammarRule(
        name: "return_stmt",
        body: Sequence(elements: [
          Literal("return"),
          Optional(element: RuleReference("expression_list", isToken: false))
        ]),
        lineNumber: 173),
    GrammarRule(
        name: "yield_stmt",
        body: RuleReference("yield_expr", isToken: false),
        lineNumber: 182),
    GrammarRule(
        name: "raise_stmt",
        body: Sequence(elements: [
          Literal("raise"),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Optional(
                element: Sequence(elements: [
              Literal("from"),
              RuleReference("expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 211),
    GrammarRule(name: "break_stmt", body: Literal("break"), lineNumber: 214),
    GrammarRule(
        name: "continue_stmt", body: Literal("continue"), lineNumber: 217),
    GrammarRule(name: "pass_stmt", body: Literal("pass"), lineNumber: 227),
    GrammarRule(
        name: "import_stmt",
        body: Sequence(elements: [
          Literal("import"),
          RuleReference("dotted_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ])),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("dotted_name", isToken: false),
            Optional(
                element: Sequence(elements: [
              Literal("as"),
              RuleReference("NAME", isToken: true)
            ]))
          ]))
        ]),
        lineNumber: 234),
    GrammarRule(
        name: "from_import_stmt",
        body: Sequence(elements: [
          Literal("from"),
          Group(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("dots", isToken: false),
              RuleReference("dotted_name", isToken: false)
            ]),
            RuleReference("dots", isToken: false),
            RuleReference("dotted_name", isToken: false)
          ])),
          Literal("import"),
          Group(
              element: Alternation(choices: [
            RuleReference("STAR", isToken: true),
            Sequence(elements: [
              RuleReference("LPAREN", isToken: true),
              RuleReference("import_names", isToken: false),
              RuleReference("RPAREN", isToken: true)
            ]),
            RuleReference("import_names", isToken: false)
          ]))
        ]),
        lineNumber: 248),
    GrammarRule(
        name: "dots",
        body: Sequence(elements: [
          RuleReference("DOT", isToken: true),
          Repetition(element: RuleReference("DOT", isToken: true))
        ]),
        lineNumber: 253),
    GrammarRule(
        name: "import_names",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ])),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              Literal("as"),
              RuleReference("NAME", isToken: true)
            ]))
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 255),
    GrammarRule(
        name: "dotted_name",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 257),
    GrammarRule(
        name: "global_stmt",
        body: Sequence(elements: [
          Literal("global"),
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 263),
    GrammarRule(
        name: "nonlocal_stmt",
        body: Sequence(elements: [
          Literal("nonlocal"),
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 295),
    GrammarRule(
        name: "del_stmt",
        body: Sequence(elements: [
          Literal("del"),
          RuleReference("target_list", isToken: false)
        ]),
        lineNumber: 302),
    GrammarRule(
        name: "assert_stmt",
        body: Sequence(elements: [
          Literal("assert"),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 307),
    GrammarRule(
        name: "assign_stmt",
        body: Sequence(elements: [
          RuleReference("expression_list", isToken: false),
          Optional(
              element: Alternation(choices: [
            Repetition(
                element: Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("expression_list", isToken: false)
            ])),
            Sequence(elements: [
              RuleReference("augmented_assign_op", isToken: false),
              RuleReference("expression_list", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 337),
    GrammarRule(
        name: "augmented_assign_op",
        body: Alternation(choices: [
          RuleReference("PLUS_EQUALS", isToken: true),
          RuleReference("MINUS_EQUALS", isToken: true),
          RuleReference("STAR_EQUALS", isToken: true),
          RuleReference("SLASH_EQUALS", isToken: true),
          RuleReference("FLOOR_DIV_EQUALS", isToken: true),
          RuleReference("PERCENT_EQUALS", isToken: true),
          RuleReference("AMP_EQUALS", isToken: true),
          RuleReference("PIPE_EQUALS", isToken: true),
          RuleReference("CARET_EQUALS", isToken: true),
          RuleReference("LEFT_SHIFT_EQUALS", isToken: true),
          RuleReference("RIGHT_SHIFT_EQUALS", isToken: true),
          RuleReference("DOUBLE_STAR_EQUALS", isToken: true)
        ]),
        lineNumber: 341),
    GrammarRule(
        name: "compound_stmt",
        body: Alternation(choices: [
          RuleReference("if_stmt", isToken: false),
          RuleReference("while_stmt", isToken: false),
          RuleReference("for_stmt", isToken: false),
          RuleReference("try_stmt", isToken: false),
          RuleReference("with_stmt", isToken: false),
          RuleReference("def_stmt", isToken: false),
          RuleReference("class_stmt", isToken: false),
          RuleReference("decorated", isToken: false)
        ]),
        lineNumber: 350),
    GrammarRule(
        name: "if_stmt",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("elif"),
            RuleReference("expression", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ]))
        ]),
        lineNumber: 369),
    GrammarRule(
        name: "while_stmt",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ]))
        ]),
        lineNumber: 381),
    GrammarRule(
        name: "for_stmt",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("target_list", isToken: false),
          Literal("in"),
          RuleReference("expression_list", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ]))
        ]),
        lineNumber: 398),
    GrammarRule(
        name: "try_stmt",
        body: Sequence(elements: [
          Literal("try"),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Group(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("except_clause", isToken: false),
              Repetition(
                  element: RuleReference("except_clause", isToken: false)),
              Optional(
                  element: Sequence(elements: [
                Literal("else"),
                RuleReference("COLON", isToken: true),
                RuleReference("suite", isToken: false)
              ])),
              Optional(
                  element: Sequence(elements: [
                Literal("finally"),
                RuleReference("COLON", isToken: true),
                RuleReference("suite", isToken: false)
              ]))
            ]),
            Sequence(elements: [
              Literal("finally"),
              RuleReference("COLON", isToken: true),
              RuleReference("suite", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 432),
    GrammarRule(
        name: "except_clause",
        body: Sequence(elements: [
          Literal("except"),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Optional(
                element: Sequence(elements: [
              Literal("as"),
              RuleReference("NAME", isToken: true)
            ]))
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 438),
    GrammarRule(
        name: "with_stmt",
        body: Sequence(elements: [
          Literal("with"),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("target", isToken: false)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 456),
    GrammarRule(
        name: "def_stmt",
        body: Sequence(elements: [
          Literal("def"),
          RuleReference("NAME", isToken: true),
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("parameters", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 494),
    GrammarRule(
        name: "parameters",
        body: Sequence(elements: [
          RuleReference("parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("parameter", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 529),
    GrammarRule(
        name: "parameter",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ]),
          RuleReference("STAR", isToken: true),
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("COLON", isToken: true),
            RuleReference("expression", isToken: false),
            Optional(
                element: Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ])
        ]),
        lineNumber: 531),
    GrammarRule(
        name: "class_stmt",
        body: Sequence(elements: [
          Literal("class"),
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("class_args", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 566),
    GrammarRule(
        name: "class_args",
        body: Sequence(elements: [
          RuleReference("class_arg", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("class_arg", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 568),
    GrammarRule(
        name: "class_arg",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 570),
    GrammarRule(
        name: "decorated",
        body: Sequence(elements: [
          RuleReference("decorator", isToken: false),
          Repetition(element: RuleReference("decorator", isToken: false)),
          Group(
              element: Alternation(choices: [
            RuleReference("def_stmt", isToken: false),
            RuleReference("class_stmt", isToken: false)
          ]))
        ]),
        lineNumber: 596),
    GrammarRule(
        name: "decorator",
        body: Sequence(elements: [
          RuleReference("AT", isToken: true),
          RuleReference("dotted_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 598),
    GrammarRule(
        name: "suite",
        body: Alternation(choices: [
          RuleReference("simple_stmt", isToken: false),
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("INDENT", isToken: true),
            Repetition(element: RuleReference("statement", isToken: false)),
            RuleReference("DEDENT", isToken: true)
          ])
        ]),
        lineNumber: 614),
    GrammarRule(
        name: "expression_list",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 643),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("lambda_expr", isToken: false),
          Sequence(elements: [
            RuleReference("or_expr", isToken: false),
            Optional(
                element: Sequence(elements: [
              Literal("if"),
              RuleReference("or_expr", isToken: false),
              Literal("else"),
              RuleReference("expression", isToken: false)
            ]))
          ])
        ]),
        lineNumber: 648),
    GrammarRule(
        name: "lambda_expr",
        body: Sequence(elements: [
          Literal("lambda"),
          Optional(element: RuleReference("lambda_params", isToken: false)),
          RuleReference("COLON", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 656),
    GrammarRule(
        name: "lambda_params",
        body: Sequence(elements: [
          RuleReference("lambda_param", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("lambda_param", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 657),
    GrammarRule(
        name: "lambda_param",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          RuleReference("STAR", isToken: true)
        ]),
        lineNumber: 658),
    GrammarRule(
        name: "or_expr",
        body: Sequence(elements: [
          RuleReference("and_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("or"),
            RuleReference("and_expr", isToken: false)
          ]))
        ]),
        lineNumber: 662),
    GrammarRule(
        name: "and_expr",
        body: Sequence(elements: [
          RuleReference("not_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            RuleReference("not_expr", isToken: false)
          ]))
        ]),
        lineNumber: 666),
    GrammarRule(
        name: "not_expr",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("not"),
            RuleReference("not_expr", isToken: false)
          ]),
          RuleReference("comparison", isToken: false)
        ]),
        lineNumber: 670),
    GrammarRule(
        name: "comparison",
        body: Sequence(elements: [
          RuleReference("bitwise_or", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("comp_op", isToken: false),
            RuleReference("bitwise_or", isToken: false)
          ]))
        ]),
        lineNumber: 687),
    GrammarRule(
        name: "comp_op",
        body: Alternation(choices: [
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          Literal("in"),
          Sequence(elements: [Literal("not"), Literal("in")]),
          Literal("is"),
          Sequence(elements: [Literal("is"), Literal("not")])
        ]),
        lineNumber: 689),
    GrammarRule(
        name: "bitwise_or",
        body: Sequence(elements: [
          RuleReference("bitwise_xor", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("PIPE", isToken: true),
            RuleReference("bitwise_xor", isToken: false)
          ]))
        ]),
        lineNumber: 700),
    GrammarRule(
        name: "bitwise_xor",
        body: Sequence(elements: [
          RuleReference("bitwise_and", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("CARET", isToken: true),
            RuleReference("bitwise_and", isToken: false)
          ]))
        ]),
        lineNumber: 701),
    GrammarRule(
        name: "bitwise_and",
        body: Sequence(elements: [
          RuleReference("shift", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("AMP", isToken: true),
            RuleReference("shift", isToken: false)
          ]))
        ]),
        lineNumber: 702),
    GrammarRule(
        name: "shift",
        body: Sequence(elements: [
          RuleReference("arith", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("LEFT_SHIFT", isToken: true),
              RuleReference("RIGHT_SHIFT", isToken: true)
            ])),
            RuleReference("arith", isToken: false)
          ]))
        ]),
        lineNumber: 705),
    GrammarRule(
        name: "arith",
        body: Sequence(elements: [
          RuleReference("term", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("term", isToken: false)
          ]))
        ]),
        lineNumber: 709),
    GrammarRule(
        name: "term",
        body: Sequence(elements: [
          RuleReference("factor", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true),
              RuleReference("FLOOR_DIV", isToken: true),
              RuleReference("PERCENT", isToken: true)
            ])),
            RuleReference("factor", isToken: false)
          ]))
        ]),
        lineNumber: 726),
    GrammarRule(
        name: "factor",
        body: Alternation(choices: [
          Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true),
              RuleReference("TILDE", isToken: true)
            ])),
            RuleReference("factor", isToken: false)
          ]),
          RuleReference("power", isToken: false)
        ]),
        lineNumber: 732),
    GrammarRule(
        name: "power",
        body: Sequence(elements: [
          RuleReference("primary", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("factor", isToken: false)
          ]))
        ]),
        lineNumber: 738),
    GrammarRule(
        name: "primary",
        body: Sequence(elements: [
          RuleReference("atom", isToken: false),
          Repetition(element: RuleReference("suffix", isToken: false))
        ]),
        lineNumber: 753),
    GrammarRule(
        name: "suffix",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LBRACKET", isToken: true),
            RuleReference("subscript", isToken: false),
            RuleReference("RBRACKET", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 755),
    GrammarRule(
        name: "subscript",
        body: Alternation(choices: [
          RuleReference("expression", isToken: false),
          Sequence(elements: [
            Optional(element: RuleReference("expression", isToken: false)),
            RuleReference("COLON", isToken: true),
            Optional(element: RuleReference("expression", isToken: false)),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              Optional(element: RuleReference("expression", isToken: false))
            ]))
          ])
        ]),
        lineNumber: 768),
    GrammarRule(
        name: "atom",
        body: Alternation(choices: [
          RuleReference("INT", isToken: true),
          RuleReference("FLOAT", isToken: true),
          RuleReference("IMAG", isToken: true),
          Sequence(elements: [
            RuleReference("STRING", isToken: true),
            Repetition(element: RuleReference("STRING", isToken: true))
          ]),
          RuleReference("NAME", isToken: true),
          Literal("True"),
          Literal("False"),
          Literal("None"),
          RuleReference("ELLIPSIS", isToken: true),
          RuleReference("list_expr", isToken: false),
          RuleReference("dict_or_set_expr", isToken: false),
          RuleReference("paren_expr", isToken: false),
          RuleReference("generator_expr", isToken: false)
        ]),
        lineNumber: 777),
    GrammarRule(
        name: "yield_expr",
        body: Sequence(elements: [
          Literal("yield"),
          Optional(element: RuleReference("expression_list", isToken: false))
        ]),
        lineNumber: 821),
    GrammarRule(
        name: "list_expr",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(element: RuleReference("list_body", isToken: false)),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 830),
    GrammarRule(
        name: "list_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("expression", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])
        ]),
        lineNumber: 832),
    GrammarRule(
        name: "dict_or_set_expr",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(element: RuleReference("dict_or_set_body", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 849),
    GrammarRule(
        name: "dict_or_set_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("expression", isToken: false),
              RuleReference("COLON", isToken: true),
              RuleReference("expression", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("expression", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])
        ]),
        lineNumber: 851),
    GrammarRule(
        name: "paren_expr",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("paren_body", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 861),
    GrammarRule(
        name: "paren_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("COMMA", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("expression", isToken: false),
              Repetition(
                  element: Sequence(elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("expression", isToken: false)
              ])),
              Optional(element: RuleReference("COMMA", isToken: true))
            ]))
          ]),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 863),
    GrammarRule(
        name: "generator_expr",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("comp_clause", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 873),
    GrammarRule(
        name: "comp_clause",
        body: Sequence(elements: [
          RuleReference("comp_for", isToken: false),
          Repetition(
              element: Alternation(choices: [
            RuleReference("comp_for", isToken: false),
            RuleReference("comp_if", isToken: false)
          ]))
        ]),
        lineNumber: 887),
    GrammarRule(
        name: "comp_for",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("target_list", isToken: false),
          Literal("in"),
          RuleReference("or_expr", isToken: false)
        ]),
        lineNumber: 889),
    GrammarRule(
        name: "comp_if",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("or_expr", isToken: false)
        ]),
        lineNumber: 891),
    GrammarRule(
        name: "target_list",
        body: Sequence(elements: [
          RuleReference("target", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("target", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 919),
    GrammarRule(
        name: "target",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          RuleReference("NAME", isToken: true),
          RuleReference("primary", isToken: false)
        ]),
        lineNumber: 921),
    GrammarRule(
        name: "arguments",
        body: Sequence(elements: [
          RuleReference("argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("argument", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 945),
    GrammarRule(
        name: "argument",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 947),
  ],
);

final parserGrammarLegacyV3_6 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "file",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("statement", isToken: false)
        ])),
        lineNumber: 39),
    GrammarRule(
        name: "statement",
        body: Alternation(choices: [
          RuleReference("compound_stmt", isToken: false),
          RuleReference("simple_stmt", isToken: false)
        ]),
        lineNumber: 53),
    GrammarRule(
        name: "simple_stmt",
        body: Sequence(elements: [
          RuleReference("small_stmt", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("small_stmt", isToken: false)
          ])),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 58),
    GrammarRule(
        name: "small_stmt",
        body: Alternation(choices: [
          RuleReference("return_stmt", isToken: false),
          RuleReference("break_stmt", isToken: false),
          RuleReference("continue_stmt", isToken: false),
          RuleReference("pass_stmt", isToken: false),
          RuleReference("import_stmt", isToken: false),
          RuleReference("from_import_stmt", isToken: false),
          RuleReference("raise_stmt", isToken: false),
          RuleReference("yield_stmt", isToken: false),
          RuleReference("del_stmt", isToken: false),
          RuleReference("assert_stmt", isToken: false),
          RuleReference("global_stmt", isToken: false),
          RuleReference("nonlocal_stmt", isToken: false),
          RuleReference("annotated_assign_stmt", isToken: false),
          RuleReference("assign_stmt", isToken: false)
        ]),
        lineNumber: 60),
    GrammarRule(
        name: "return_stmt",
        body: Sequence(elements: [
          Literal("return"),
          Optional(element: RuleReference("expression_list", isToken: false))
        ]),
        lineNumber: 83),
    GrammarRule(name: "break_stmt", body: Literal("break"), lineNumber: 86),
    GrammarRule(
        name: "continue_stmt", body: Literal("continue"), lineNumber: 89),
    GrammarRule(name: "pass_stmt", body: Literal("pass"), lineNumber: 97),
    GrammarRule(
        name: "import_stmt",
        body: Sequence(elements: [
          Literal("import"),
          RuleReference("dotted_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ])),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("dotted_name", isToken: false),
            Optional(
                element: Sequence(elements: [
              Literal("as"),
              RuleReference("NAME", isToken: true)
            ]))
          ]))
        ]),
        lineNumber: 104),
    GrammarRule(
        name: "from_import_stmt",
        body: Sequence(elements: [
          Literal("from"),
          Group(
              element: Alternation(choices: [
            RuleReference("dotted_name", isToken: false),
            RuleReference("relative_import", isToken: false)
          ])),
          Literal("import"),
          Group(
              element: Alternation(choices: [
            RuleReference("STAR", isToken: true),
            RuleReference("import_names", isToken: false)
          ]))
        ]),
        lineNumber: 113),
    GrammarRule(
        name: "relative_import",
        body: Sequence(elements: [
          Repetition(element: RuleReference("DOT", isToken: true)),
          Optional(element: RuleReference("dotted_name", isToken: false))
        ]),
        lineNumber: 116),
    GrammarRule(
        name: "import_names",
        body: Sequence(elements: [
          RuleReference("import_name", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("import_name", isToken: false)
          ]))
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "import_name",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 119),
    GrammarRule(
        name: "dotted_name",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 121),
    GrammarRule(
        name: "raise_stmt",
        body: Sequence(elements: [
          Literal("raise"),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Optional(
                element: Sequence(elements: [
              Literal("from"),
              RuleReference("expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 127),
    GrammarRule(
        name: "yield_stmt",
        body: RuleReference("yield_expr", isToken: false),
        lineNumber: 133),
    GrammarRule(
        name: "yield_expr",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("yield"),
            Literal("from"),
            RuleReference("expression", isToken: false)
          ]),
          Sequence(elements: [
            Literal("yield"),
            Optional(element: RuleReference("expression_list", isToken: false))
          ])
        ]),
        lineNumber: 135),
    GrammarRule(
        name: "del_stmt",
        body: Sequence(elements: [
          Literal("del"),
          RuleReference("target_list", isToken: false)
        ]),
        lineNumber: 143),
    GrammarRule(
        name: "assert_stmt",
        body: Sequence(elements: [
          Literal("assert"),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 148),
    GrammarRule(
        name: "global_stmt",
        body: Sequence(elements: [
          Literal("global"),
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 153),
    GrammarRule(
        name: "nonlocal_stmt",
        body: Sequence(elements: [
          Literal("nonlocal"),
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 158),
    GrammarRule(
        name: "annotated_assign_stmt",
        body: Sequence(elements: [
          RuleReference("target", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 198),
    GrammarRule(
        name: "assign_stmt",
        body: Sequence(elements: [
          RuleReference("expression_list", isToken: false),
          Optional(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("expression_list", isToken: false),
              Repetition(
                  element: Sequence(elements: [
                RuleReference("EQUALS", isToken: true),
                RuleReference("expression_list", isToken: false)
              ]))
            ]),
            Sequence(elements: [
              RuleReference("augmented_assign_op", isToken: false),
              RuleReference("expression_list", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 212),
    GrammarRule(
        name: "augmented_assign_op",
        body: Alternation(choices: [
          RuleReference("PLUS_EQUALS", isToken: true),
          RuleReference("MINUS_EQUALS", isToken: true),
          RuleReference("STAR_EQUALS", isToken: true),
          RuleReference("SLASH_EQUALS", isToken: true),
          RuleReference("FLOOR_DIV_EQUALS", isToken: true),
          RuleReference("PERCENT_EQUALS", isToken: true),
          RuleReference("AMP_EQUALS", isToken: true),
          RuleReference("PIPE_EQUALS", isToken: true),
          RuleReference("CARET_EQUALS", isToken: true),
          RuleReference("LEFT_SHIFT_EQUALS", isToken: true),
          RuleReference("RIGHT_SHIFT_EQUALS", isToken: true),
          RuleReference("DOUBLE_STAR_EQUALS", isToken: true),
          RuleReference("AT_EQUALS", isToken: true)
        ]),
        lineNumber: 216),
    GrammarRule(
        name: "compound_stmt",
        body: Alternation(choices: [
          RuleReference("if_stmt", isToken: false),
          RuleReference("for_stmt", isToken: false),
          RuleReference("while_stmt", isToken: false),
          RuleReference("try_stmt", isToken: false),
          RuleReference("with_stmt", isToken: false),
          RuleReference("def_stmt", isToken: false),
          RuleReference("class_stmt", isToken: false),
          RuleReference("decorated_stmt", isToken: false),
          RuleReference("async_stmt", isToken: false)
        ]),
        lineNumber: 228),
    GrammarRule(
        name: "if_stmt",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("elif"),
            RuleReference("expression", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ]))
        ]),
        lineNumber: 252),
    GrammarRule(
        name: "for_stmt",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("target_list", isToken: false),
          Literal("in"),
          RuleReference("expression_list", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ]))
        ]),
        lineNumber: 276),
    GrammarRule(
        name: "while_stmt",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ]))
        ]),
        lineNumber: 289),
    GrammarRule(
        name: "try_stmt",
        body: Sequence(elements: [
          Literal("try"),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Group(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("except_clauses", isToken: false),
              Optional(
                  element: Sequence(elements: [
                Literal("else"),
                RuleReference("COLON", isToken: true),
                RuleReference("suite", isToken: false)
              ])),
              Optional(
                  element: Sequence(elements: [
                Literal("finally"),
                RuleReference("COLON", isToken: true),
                RuleReference("suite", isToken: false)
              ]))
            ]),
            Sequence(elements: [
              Literal("finally"),
              RuleReference("COLON", isToken: true),
              RuleReference("suite", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 327),
    GrammarRule(
        name: "except_clauses",
        body: Sequence(elements: [
          RuleReference("except_clause", isToken: false),
          Repetition(element: RuleReference("except_clause", isToken: false))
        ]),
        lineNumber: 331),
    GrammarRule(
        name: "except_clause",
        body: Sequence(elements: [
          Literal("except"),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Optional(
                element: Sequence(elements: [
              Literal("as"),
              RuleReference("NAME", isToken: true)
            ]))
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 333),
    GrammarRule(
        name: "with_stmt",
        body: Sequence(elements: [
          Literal("with"),
          RuleReference("with_item", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("with_item", isToken: false)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 349),
    GrammarRule(
        name: "with_item",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("target", isToken: false)
          ]))
        ]),
        lineNumber: 351),
    GrammarRule(
        name: "def_stmt",
        body: Sequence(elements: [
          Literal("def"),
          RuleReference("NAME", isToken: true),
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("parameters", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 370),
    GrammarRule(
        name: "class_stmt",
        body: Sequence(elements: [
          Literal("class"),
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 391),
    GrammarRule(
        name: "decorated_stmt",
        body: Sequence(elements: [
          RuleReference("decorator", isToken: false),
          Repetition(element: RuleReference("decorator", isToken: false)),
          Group(
              element: Alternation(choices: [
            RuleReference("def_stmt", isToken: false),
            RuleReference("class_stmt", isToken: false),
            RuleReference("async_stmt", isToken: false)
          ]))
        ]),
        lineNumber: 408),
    GrammarRule(
        name: "decorator",
        body: Sequence(elements: [
          RuleReference("AT", isToken: true),
          RuleReference("dotted_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 410),
    GrammarRule(
        name: "async_stmt",
        body: Sequence(elements: [
          Literal("async"),
          Group(
              element: Alternation(choices: [
            RuleReference("async_def_stmt", isToken: false),
            RuleReference("async_for_stmt", isToken: false),
            RuleReference("async_with_stmt", isToken: false)
          ]))
        ]),
        lineNumber: 461),
    GrammarRule(
        name: "async_def_stmt",
        body: Sequence(elements: [
          Literal("def"),
          RuleReference("NAME", isToken: true),
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("parameters", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 470),
    GrammarRule(
        name: "async_for_stmt",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("target_list", isToken: false),
          Literal("in"),
          RuleReference("expression_list", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ]))
        ]),
        lineNumber: 479),
    GrammarRule(
        name: "async_with_stmt",
        body: Sequence(elements: [
          Literal("with"),
          RuleReference("with_item", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("with_item", isToken: false)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 488),
    GrammarRule(
        name: "suite",
        body: Alternation(choices: [
          RuleReference("simple_stmt", isToken: false),
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("INDENT", isToken: true),
            Repetition(element: RuleReference("statement", isToken: false)),
            RuleReference("DEDENT", isToken: true)
          ])
        ]),
        lineNumber: 505),
    GrammarRule(
        name: "parameters",
        body: Sequence(elements: [
          RuleReference("parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("parameter", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 534),
    GrammarRule(
        name: "parameter",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ]),
          RuleReference("STAR", isToken: true),
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("COLON", isToken: true),
            RuleReference("expression", isToken: false),
            Optional(
                element: Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ])
        ]),
        lineNumber: 536),
    GrammarRule(
        name: "target",
        body: Alternation(choices: [
          RuleReference("NAME", isToken: true),
          Sequence(elements: [
            RuleReference("primary", isToken: false),
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("primary", isToken: false),
            RuleReference("LBRACKET", isToken: true),
            RuleReference("subscript", isToken: false),
            RuleReference("RBRACKET", isToken: true)
          ])
        ]),
        lineNumber: 557),
    GrammarRule(
        name: "target_list",
        body: Sequence(elements: [
          RuleReference("target_item", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("target_item", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 559),
    GrammarRule(
        name: "target_item",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("target", isToken: false)
          ]),
          RuleReference("target", isToken: false)
        ]),
        lineNumber: 561),
    GrammarRule(
        name: "expression_list",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 595),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("lambda_expr", isToken: false),
          Sequence(elements: [
            RuleReference("or_expr", isToken: false),
            Optional(
                element: Sequence(elements: [
              Literal("if"),
              RuleReference("or_expr", isToken: false),
              Literal("else"),
              RuleReference("expression", isToken: false)
            ]))
          ])
        ]),
        lineNumber: 598),
    GrammarRule(
        name: "lambda_expr",
        body: Sequence(elements: [
          Literal("lambda"),
          Optional(element: RuleReference("lambda_params", isToken: false)),
          RuleReference("COLON", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 604),
    GrammarRule(
        name: "lambda_params",
        body: Sequence(elements: [
          RuleReference("lambda_param", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("lambda_param", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 605),
    GrammarRule(
        name: "lambda_param",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          RuleReference("STAR", isToken: true),
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ])
        ]),
        lineNumber: 606),
    GrammarRule(
        name: "or_expr",
        body: Sequence(elements: [
          RuleReference("and_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("or"),
            RuleReference("and_expr", isToken: false)
          ]))
        ]),
        lineNumber: 613),
    GrammarRule(
        name: "and_expr",
        body: Sequence(elements: [
          RuleReference("not_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            RuleReference("not_expr", isToken: false)
          ]))
        ]),
        lineNumber: 617),
    GrammarRule(
        name: "not_expr",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("not"),
            RuleReference("not_expr", isToken: false)
          ]),
          RuleReference("comparison", isToken: false)
        ]),
        lineNumber: 621),
    GrammarRule(
        name: "comparison",
        body: Sequence(elements: [
          RuleReference("bitwise_or", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("comp_op", isToken: false),
            RuleReference("bitwise_or", isToken: false)
          ]))
        ]),
        lineNumber: 633),
    GrammarRule(
        name: "comp_op",
        body: Alternation(choices: [
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          Literal("in"),
          Sequence(elements: [Literal("not"), Literal("in")]),
          Literal("is"),
          Sequence(elements: [Literal("is"), Literal("not")])
        ]),
        lineNumber: 635),
    GrammarRule(
        name: "bitwise_or",
        body: Sequence(elements: [
          RuleReference("bitwise_xor", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("PIPE", isToken: true),
            RuleReference("bitwise_xor", isToken: false)
          ]))
        ]),
        lineNumber: 645),
    GrammarRule(
        name: "bitwise_xor",
        body: Sequence(elements: [
          RuleReference("bitwise_and", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("CARET", isToken: true),
            RuleReference("bitwise_and", isToken: false)
          ]))
        ]),
        lineNumber: 646),
    GrammarRule(
        name: "bitwise_and",
        body: Sequence(elements: [
          RuleReference("shift", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("AMP", isToken: true),
            RuleReference("shift", isToken: false)
          ]))
        ]),
        lineNumber: 647),
    GrammarRule(
        name: "shift",
        body: Sequence(elements: [
          RuleReference("arith", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("LEFT_SHIFT", isToken: true),
              RuleReference("RIGHT_SHIFT", isToken: true)
            ])),
            RuleReference("arith", isToken: false)
          ]))
        ]),
        lineNumber: 652),
    GrammarRule(
        name: "arith",
        body: Sequence(elements: [
          RuleReference("term", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("term", isToken: false)
          ]))
        ]),
        lineNumber: 657),
    GrammarRule(
        name: "term",
        body: Sequence(elements: [
          RuleReference("factor", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true),
              RuleReference("FLOOR_DIV", isToken: true),
              RuleReference("PERCENT", isToken: true),
              RuleReference("AT", isToken: true)
            ])),
            RuleReference("factor", isToken: false)
          ]))
        ]),
        lineNumber: 665),
    GrammarRule(
        name: "factor",
        body: Alternation(choices: [
          Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true),
              RuleReference("TILDE", isToken: true)
            ])),
            RuleReference("factor", isToken: false)
          ]),
          RuleReference("power", isToken: false)
        ]),
        lineNumber: 671),
    GrammarRule(
        name: "power",
        body: Sequence(elements: [
          RuleReference("await_expr", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("factor", isToken: false)
          ]))
        ]),
        lineNumber: 679),
    GrammarRule(
        name: "await_expr",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("await"),
            RuleReference("primary", isToken: false)
          ]),
          RuleReference("primary", isToken: false)
        ]),
        lineNumber: 710),
    GrammarRule(
        name: "primary",
        body: Sequence(elements: [
          RuleReference("atom", isToken: false),
          Repetition(element: RuleReference("suffix", isToken: false))
        ]),
        lineNumber: 728),
    GrammarRule(
        name: "suffix",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LBRACKET", isToken: true),
            RuleReference("subscript", isToken: false),
            RuleReference("RBRACKET", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 730),
    GrammarRule(
        name: "subscript",
        body: Alternation(choices: [
          RuleReference("expression", isToken: false),
          Sequence(elements: [
            Optional(element: RuleReference("expression", isToken: false)),
            RuleReference("COLON", isToken: true),
            Optional(element: RuleReference("expression", isToken: false)),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              Optional(element: RuleReference("expression", isToken: false))
            ]))
          ])
        ]),
        lineNumber: 742),
    GrammarRule(
        name: "atom",
        body: Alternation(choices: [
          RuleReference("INT", isToken: true),
          RuleReference("FLOAT", isToken: true),
          RuleReference("IMAGINARY", isToken: true),
          RuleReference("string_atom", isToken: false),
          RuleReference("fstring", isToken: false),
          RuleReference("NAME", isToken: true),
          Literal("True"),
          Literal("False"),
          Literal("None"),
          RuleReference("ELLIPSIS", isToken: true),
          RuleReference("list_expr", isToken: false),
          RuleReference("dict_set_expr", isToken: false),
          RuleReference("paren_expr", isToken: false),
          RuleReference("yield_expr", isToken: false)
        ]),
        lineNumber: 751),
    GrammarRule(
        name: "string_atom",
        body: Sequence(elements: [
          RuleReference("STRING", isToken: true),
          Repetition(
              element: Alternation(choices: [
            RuleReference("STRING", isToken: true),
            RuleReference("fstring", isToken: false)
          ]))
        ]),
        lineNumber: 769),
    GrammarRule(
        name: "fstring",
        body: Sequence(elements: [
          RuleReference("FSTRING_START", isToken: true),
          Repetition(element: RuleReference("fstring_part", isToken: false)),
          RuleReference("FSTRING_END", isToken: true)
        ]),
        lineNumber: 833),
    GrammarRule(
        name: "fstring_part",
        body: Alternation(choices: [
          RuleReference("FSTRING_MIDDLE", isToken: true),
          RuleReference("fstring_expr", isToken: false)
        ]),
        lineNumber: 835),
    GrammarRule(
        name: "fstring_expr",
        body: Sequence(elements: [
          RuleReference("LBRACE_EXPR", isToken: true),
          RuleReference("expression", isToken: false),
          Optional(
              element: RuleReference("fstring_conversion", isToken: false)),
          Optional(
              element: RuleReference("fstring_format_spec", isToken: false)),
          RuleReference("RBRACE_EXPR", isToken: true)
        ]),
        lineNumber: 849),
    GrammarRule(
        name: "fstring_conversion",
        body: Sequence(elements: [
          RuleReference("EXCL", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 854),
    GrammarRule(
        name: "fstring_format_spec",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          Repetition(
              element: Alternation(choices: [
            RuleReference("FSTRING_MIDDLE", isToken: true),
            RuleReference("fstring_expr", isToken: false)
          ]))
        ]),
        lineNumber: 862),
    GrammarRule(
        name: "list_expr",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(element: RuleReference("list_body", isToken: false)),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 878),
    GrammarRule(
        name: "list_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("expression", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])
        ]),
        lineNumber: 880),
    GrammarRule(
        name: "dict_set_expr",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(element: RuleReference("dict_set_body", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 902),
    GrammarRule(
        name: "dict_set_body",
        body: Alternation(choices: [
          RuleReference("dict_body", isToken: false),
          RuleReference("set_body", isToken: false)
        ]),
        lineNumber: 904),
    GrammarRule(
        name: "dict_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("dict_entry", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("dict_entry", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("dict_entry", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])
        ]),
        lineNumber: 906),
    GrammarRule(
        name: "dict_entry",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("expression", isToken: false)
          ])
        ]),
        lineNumber: 909),
    GrammarRule(
        name: "set_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("expression", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])
        ]),
        lineNumber: 912),
    GrammarRule(
        name: "paren_expr",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("paren_body", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 925),
    GrammarRule(
        name: "paren_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("COMMA", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("expression", isToken: false),
              Repetition(
                  element: Sequence(elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("expression", isToken: false)
              ])),
              Optional(element: RuleReference("COMMA", isToken: true))
            ]))
          ]),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 927),
    GrammarRule(
        name: "comp_clause",
        body: Sequence(elements: [
          RuleReference("comp_for", isToken: false),
          Repetition(
              element: Alternation(choices: [
            RuleReference("comp_for", isToken: false),
            RuleReference("comp_if", isToken: false)
          ]))
        ]),
        lineNumber: 955),
    GrammarRule(
        name: "comp_for",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("for"),
            RuleReference("target_list", isToken: false),
            Literal("in"),
            RuleReference("or_expr", isToken: false)
          ]),
          Sequence(elements: [
            Literal("async"),
            Literal("for"),
            RuleReference("target_list", isToken: false),
            Literal("in"),
            RuleReference("or_expr", isToken: false)
          ])
        ]),
        lineNumber: 959),
    GrammarRule(
        name: "comp_if",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("or_expr", isToken: false)
        ]),
        lineNumber: 964),
    GrammarRule(
        name: "arguments",
        body: Sequence(elements: [
          RuleReference("argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("argument", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 987),
    GrammarRule(
        name: "argument",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 989),
  ],
);

final parserGrammarLegacyV3_8 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "file_input",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("statement", isToken: false)
        ])),
        lineNumber: 71),
    GrammarRule(
        name: "single_input",
        body: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("simple_stmt", isToken: false),
          Sequence(elements: [
            RuleReference("compound_stmt", isToken: false),
            RuleReference("NEWLINE", isToken: true)
          ])
        ]),
        lineNumber: 72),
    GrammarRule(
        name: "eval_input",
        body: Sequence(elements: [
          RuleReference("expression_list", isToken: false),
          Repetition(element: RuleReference("NEWLINE", isToken: true))
        ]),
        lineNumber: 73),
    GrammarRule(
        name: "statement",
        body: Alternation(choices: [
          RuleReference("compound_stmt", isToken: false),
          RuleReference("simple_stmt", isToken: false)
        ]),
        lineNumber: 94),
    GrammarRule(
        name: "simple_stmt",
        body: Sequence(elements: [
          RuleReference("small_stmt", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("small_stmt", isToken: false)
          ])),
          Optional(element: RuleReference("SEMICOLON", isToken: true)),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 102),
    GrammarRule(
        name: "small_stmt",
        body: Alternation(choices: [
          RuleReference("return_stmt", isToken: false),
          RuleReference("raise_stmt", isToken: false),
          RuleReference("import_stmt", isToken: false),
          RuleReference("from_import_stmt", isToken: false),
          RuleReference("global_stmt", isToken: false),
          RuleReference("nonlocal_stmt", isToken: false),
          RuleReference("assert_stmt", isToken: false),
          RuleReference("del_stmt", isToken: false),
          RuleReference("break_stmt", isToken: false),
          RuleReference("continue_stmt", isToken: false),
          RuleReference("pass_stmt", isToken: false),
          RuleReference("yield_stmt", isToken: false),
          RuleReference("assign_stmt", isToken: false)
        ]),
        lineNumber: 104),
    GrammarRule(
        name: "return_stmt",
        body: Sequence(elements: [
          Literal("return"),
          Optional(element: RuleReference("expression_list", isToken: false))
        ]),
        lineNumber: 130),
    GrammarRule(
        name: "raise_stmt",
        body: Sequence(elements: [
          Literal("raise"),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Optional(
                element: Sequence(elements: [
              Literal("from"),
              RuleReference("expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 140),
    GrammarRule(
        name: "import_stmt",
        body: Sequence(elements: [
          Literal("import"),
          RuleReference("dotted_as_names", isToken: false)
        ]),
        lineNumber: 149),
    GrammarRule(
        name: "dotted_as_names",
        body: Sequence(elements: [
          RuleReference("dotted_as_name", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("dotted_as_name", isToken: false)
          ]))
        ]),
        lineNumber: 151),
    GrammarRule(
        name: "dotted_as_name",
        body: Sequence(elements: [
          RuleReference("dotted_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 152),
    GrammarRule(
        name: "dotted_name",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 153),
    GrammarRule(
        name: "from_import_stmt",
        body: Sequence(elements: [
          Literal("from"),
          Group(
              element: Alternation(choices: [
            Sequence(elements: [
              Repetition(element: RuleReference("DOT", isToken: true)),
              RuleReference("dotted_name", isToken: false)
            ]),
            Repetition(element: RuleReference("DOT", isToken: true))
          ])),
          Literal("import"),
          Group(
              element: Alternation(choices: [
            RuleReference("STAR", isToken: true),
            RuleReference("import_as_names", isToken: false),
            Sequence(elements: [
              RuleReference("LPAREN", isToken: true),
              RuleReference("import_as_names", isToken: false),
              RuleReference("RPAREN", isToken: true)
            ])
          ]))
        ]),
        lineNumber: 168),
    GrammarRule(
        name: "import_as_names",
        body: Sequence(elements: [
          RuleReference("import_as_name", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("import_as_name", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 172),
    GrammarRule(
        name: "import_as_name",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 173),
    GrammarRule(
        name: "global_stmt",
        body: Sequence(elements: [
          Literal("global"),
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 185),
    GrammarRule(
        name: "nonlocal_stmt",
        body: Sequence(elements: [
          Literal("nonlocal"),
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 200),
    GrammarRule(
        name: "assert_stmt",
        body: Sequence(elements: [
          Literal("assert"),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 210),
    GrammarRule(
        name: "del_stmt",
        body: Sequence(elements: [
          Literal("del"),
          RuleReference("target_list", isToken: false)
        ]),
        lineNumber: 219),
    GrammarRule(name: "break_stmt", body: Literal("break"), lineNumber: 229),
    GrammarRule(
        name: "continue_stmt", body: Literal("continue"), lineNumber: 230),
    GrammarRule(name: "pass_stmt", body: Literal("pass"), lineNumber: 239),
    GrammarRule(
        name: "yield_stmt",
        body: RuleReference("yield_expr", isToken: false),
        lineNumber: 250),
    GrammarRule(
        name: "assign_stmt",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression_list", isToken: false),
            RuleReference("assign_tail", isToken: false)
          ]),
          RuleReference("expression_list", isToken: false)
        ]),
        lineNumber: 278),
    GrammarRule(
        name: "assign_tail",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("expression", isToken: false),
            Optional(
                element: Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("expression_list", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            Repetition(
                element: Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("expression_list", isToken: false)
            ])),
            RuleReference("EQUALS", isToken: true),
            Group(
                element: Alternation(choices: [
              RuleReference("yield_expr", isToken: false),
              RuleReference("expression_list", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            RuleReference("augmented_assign_op", isToken: false),
            Group(
                element: Alternation(choices: [
              RuleReference("yield_expr", isToken: false),
              RuleReference("expression_list", isToken: false)
            ]))
          ])
        ]),
        lineNumber: 281),
    GrammarRule(
        name: "augmented_assign_op",
        body: Alternation(choices: [
          RuleReference("PLUS_EQUALS", isToken: true),
          RuleReference("MINUS_EQUALS", isToken: true),
          RuleReference("STAR_EQUALS", isToken: true),
          RuleReference("SLASH_EQUALS", isToken: true),
          RuleReference("FLOOR_DIV_EQUALS", isToken: true),
          RuleReference("PERCENT_EQUALS", isToken: true),
          RuleReference("AMP_EQUALS", isToken: true),
          RuleReference("PIPE_EQUALS", isToken: true),
          RuleReference("CARET_EQUALS", isToken: true),
          RuleReference("LEFT_SHIFT_EQUALS", isToken: true),
          RuleReference("RIGHT_SHIFT_EQUALS", isToken: true),
          RuleReference("DOUBLE_STAR_EQUALS", isToken: true),
          RuleReference("AT_EQUALS", isToken: true)
        ]),
        lineNumber: 292),
    GrammarRule(
        name: "target_list",
        body: Sequence(elements: [
          RuleReference("target", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("target", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 314),
    GrammarRule(
        name: "target",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 316),
    GrammarRule(
        name: "compound_stmt",
        body: Alternation(choices: [
          RuleReference("if_stmt", isToken: false),
          RuleReference("for_stmt", isToken: false),
          RuleReference("while_stmt", isToken: false),
          RuleReference("try_stmt", isToken: false),
          RuleReference("with_stmt", isToken: false),
          RuleReference("def_stmt", isToken: false),
          RuleReference("class_stmt", isToken: false),
          RuleReference("async_stmt", isToken: false),
          RuleReference("decorated", isToken: false)
        ]),
        lineNumber: 326),
    GrammarRule(
        name: "if_stmt",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("named_expression", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("elif"),
            RuleReference("named_expression", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ]))
        ]),
        lineNumber: 354),
    GrammarRule(
        name: "for_stmt",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("target_list", isToken: false),
          Literal("in"),
          RuleReference("expression_list", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ]))
        ]),
        lineNumber: 386),
    GrammarRule(
        name: "while_stmt",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("named_expression", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ]))
        ]),
        lineNumber: 406),
    GrammarRule(
        name: "try_stmt",
        body: Sequence(elements: [
          Literal("try"),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Group(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("except_clauses", isToken: false),
              Optional(
                  element: Sequence(elements: [
                Literal("else"),
                RuleReference("COLON", isToken: true),
                RuleReference("suite", isToken: false)
              ])),
              Optional(
                  element: Sequence(elements: [
                Literal("finally"),
                RuleReference("COLON", isToken: true),
                RuleReference("suite", isToken: false)
              ]))
            ]),
            Sequence(elements: [
              Literal("finally"),
              RuleReference("COLON", isToken: true),
              RuleReference("suite", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 448),
    GrammarRule(
        name: "except_clauses",
        body: Sequence(elements: [
          RuleReference("except_clause", isToken: false),
          Repetition(element: RuleReference("except_clause", isToken: false))
        ]),
        lineNumber: 452),
    GrammarRule(
        name: "except_clause",
        body: Sequence(elements: [
          Literal("except"),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Optional(
                element: Sequence(elements: [
              Literal("as"),
              RuleReference("NAME", isToken: true)
            ]))
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 459),
    GrammarRule(
        name: "with_stmt",
        body: Sequence(elements: [
          Literal("with"),
          RuleReference("with_item", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("with_item", isToken: false)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 480),
    GrammarRule(
        name: "with_item",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("target", isToken: false)
          ]))
        ]),
        lineNumber: 482),
    GrammarRule(
        name: "def_stmt",
        body: Sequence(elements: [
          Literal("def"),
          RuleReference("NAME", isToken: true),
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("parameters", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 512),
    GrammarRule(
        name: "parameters",
        body: Sequence(elements: [
          RuleReference("parameter_list", isToken: false),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 552),
    GrammarRule(
        name: "parameter_list",
        body: Sequence(elements: [
          RuleReference("param_or_sep", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("param_or_sep", isToken: false)
          ]))
        ]),
        lineNumber: 563),
    GrammarRule(
        name: "param_or_sep",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("typed_param", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("typed_param", isToken: false)
          ]),
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true),
          Sequence(elements: [
            RuleReference("typed_param", isToken: false),
            RuleReference("EQUALS", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          RuleReference("typed_param", isToken: false)
        ]),
        lineNumber: 565),
    GrammarRule(
        name: "typed_param",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 575),
    GrammarRule(
        name: "class_stmt",
        body: Sequence(elements: [
          Literal("class"),
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 602),
    GrammarRule(
        name: "async_stmt",
        body: Sequence(elements: [
          Literal("async"),
          Group(
              element: Alternation(choices: [
            RuleReference("def_stmt", isToken: false),
            RuleReference("for_stmt", isToken: false),
            RuleReference("with_stmt", isToken: false)
          ]))
        ]),
        lineNumber: 621),
    GrammarRule(
        name: "decorated",
        body: Sequence(elements: [
          Repetition(element: RuleReference("decorator", isToken: false)),
          Group(
              element: Alternation(choices: [
            RuleReference("def_stmt", isToken: false),
            RuleReference("class_stmt", isToken: false),
            RuleReference("async_stmt", isToken: false)
          ]))
        ]),
        lineNumber: 647),
    GrammarRule(
        name: "decorator",
        body: Sequence(elements: [
          RuleReference("AT", isToken: true),
          RuleReference("dotted_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 649),
    GrammarRule(
        name: "suite",
        body: Alternation(choices: [
          RuleReference("simple_stmt", isToken: false),
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("INDENT", isToken: true),
            Repetition(element: RuleReference("statement", isToken: false)),
            RuleReference("DEDENT", isToken: true)
          ])
        ]),
        lineNumber: 670),
    GrammarRule(
        name: "expression_list",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 709),
    GrammarRule(
        name: "named_expression",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("COLONEQUALS", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 756),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("lambda_expr", isToken: false),
          Sequence(elements: [
            RuleReference("or_expr", isToken: false),
            Optional(
                element: Sequence(elements: [
              Literal("if"),
              RuleReference("or_expr", isToken: false),
              Literal("else"),
              RuleReference("expression", isToken: false)
            ]))
          ])
        ]),
        lineNumber: 763),
    GrammarRule(
        name: "lambda_expr",
        body: Sequence(elements: [
          Literal("lambda"),
          Optional(element: RuleReference("lambda_params", isToken: false)),
          RuleReference("COLON", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 784),
    GrammarRule(
        name: "lambda_params",
        body: Sequence(elements: [
          RuleReference("lambda_param", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("lambda_param", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 786),
    GrammarRule(
        name: "lambda_param",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true),
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 788),
    GrammarRule(
        name: "or_expr",
        body: Sequence(elements: [
          RuleReference("and_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("or"),
            RuleReference("and_expr", isToken: false)
          ]))
        ]),
        lineNumber: 804),
    GrammarRule(
        name: "and_expr",
        body: Sequence(elements: [
          RuleReference("not_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            RuleReference("not_expr", isToken: false)
          ]))
        ]),
        lineNumber: 811),
    GrammarRule(
        name: "not_expr",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("not"),
            RuleReference("not_expr", isToken: false)
          ]),
          RuleReference("comparison", isToken: false)
        ]),
        lineNumber: 817),
    GrammarRule(
        name: "comparison",
        body: Sequence(elements: [
          RuleReference("bitwise_or", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("comp_op", isToken: false),
            RuleReference("bitwise_or", isToken: false)
          ]))
        ]),
        lineNumber: 838),
    GrammarRule(
        name: "comp_op",
        body: Alternation(choices: [
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          Literal("in"),
          Sequence(elements: [Literal("not"), Literal("in")]),
          Sequence(elements: [Literal("is"), Optional(element: Literal("not"))])
        ]),
        lineNumber: 840),
    GrammarRule(
        name: "bitwise_or",
        body: Sequence(elements: [
          RuleReference("bitwise_xor", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("PIPE", isToken: true),
            RuleReference("bitwise_xor", isToken: false)
          ]))
        ]),
        lineNumber: 860),
    GrammarRule(
        name: "bitwise_xor",
        body: Sequence(elements: [
          RuleReference("bitwise_and", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("CARET", isToken: true),
            RuleReference("bitwise_and", isToken: false)
          ]))
        ]),
        lineNumber: 861),
    GrammarRule(
        name: "bitwise_and",
        body: Sequence(elements: [
          RuleReference("shift", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("AMP", isToken: true),
            RuleReference("shift", isToken: false)
          ]))
        ]),
        lineNumber: 862),
    GrammarRule(
        name: "shift",
        body: Sequence(elements: [
          RuleReference("arith", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("LEFT_SHIFT", isToken: true),
              RuleReference("RIGHT_SHIFT", isToken: true)
            ])),
            RuleReference("arith", isToken: false)
          ]))
        ]),
        lineNumber: 867),
    GrammarRule(
        name: "arith",
        body: Sequence(elements: [
          RuleReference("term", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("term", isToken: false)
          ]))
        ]),
        lineNumber: 875),
    GrammarRule(
        name: "term",
        body: Sequence(elements: [
          RuleReference("factor", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true),
              RuleReference("FLOOR_DIV", isToken: true),
              RuleReference("PERCENT", isToken: true),
              RuleReference("AT", isToken: true)
            ])),
            RuleReference("factor", isToken: false)
          ]))
        ]),
        lineNumber: 884),
    GrammarRule(
        name: "factor",
        body: Alternation(choices: [
          Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true),
              RuleReference("TILDE", isToken: true)
            ])),
            RuleReference("factor", isToken: false)
          ]),
          RuleReference("power", isToken: false)
        ]),
        lineNumber: 890),
    GrammarRule(
        name: "power",
        body: Sequence(elements: [
          RuleReference("await_expr", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("factor", isToken: false)
          ]))
        ]),
        lineNumber: 899),
    GrammarRule(
        name: "await_expr",
        body: Sequence(elements: [
          Optional(element: Literal("await")),
          RuleReference("primary", isToken: false)
        ]),
        lineNumber: 912),
    GrammarRule(
        name: "primary",
        body: Sequence(elements: [
          RuleReference("atom", isToken: false),
          Repetition(element: RuleReference("suffix", isToken: false))
        ]),
        lineNumber: 930),
    GrammarRule(
        name: "suffix",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LBRACKET", isToken: true),
            RuleReference("subscript_list", isToken: false),
            RuleReference("RBRACKET", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 932),
    GrammarRule(
        name: "subscript_list",
        body: Sequence(elements: [
          RuleReference("subscript", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("subscript", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 939),
    GrammarRule(
        name: "subscript",
        body: Alternation(choices: [
          RuleReference("expression", isToken: false),
          Sequence(elements: [
            Optional(element: RuleReference("expression", isToken: false)),
            RuleReference("COLON", isToken: true),
            Optional(element: RuleReference("expression", isToken: false)),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              Optional(element: RuleReference("expression", isToken: false))
            ]))
          ])
        ]),
        lineNumber: 946),
    GrammarRule(
        name: "atom",
        body: Alternation(choices: [
          RuleReference("INT", isToken: true),
          RuleReference("FLOAT", isToken: true),
          RuleReference("IMAG", isToken: true),
          Sequence(elements: [
            RuleReference("STRING", isToken: true),
            Repetition(element: RuleReference("STRING", isToken: true))
          ]),
          RuleReference("NAME", isToken: true),
          Literal("True"),
          Literal("False"),
          Literal("None"),
          RuleReference("ELLIPSIS", isToken: true),
          RuleReference("list_expr", isToken: false),
          RuleReference("dict_expr", isToken: false),
          RuleReference("set_expr", isToken: false),
          RuleReference("paren_expr", isToken: false)
        ]),
        lineNumber: 956),
    GrammarRule(
        name: "list_expr",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(element: RuleReference("list_body", isToken: false)),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 995),
    GrammarRule(
        name: "list_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("expression", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])
        ]),
        lineNumber: 997),
    GrammarRule(
        name: "dict_expr",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(element: RuleReference("dict_body", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 1014),
    GrammarRule(
        name: "dict_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("dict_entry", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("dict_entry", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("dict_entry", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])
        ]),
        lineNumber: 1016),
    GrammarRule(
        name: "dict_entry",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("expression", isToken: false)
          ])
        ]),
        lineNumber: 1019),
    GrammarRule(
        name: "set_expr",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          RuleReference("set_body", isToken: false),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 1036),
    GrammarRule(
        name: "set_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("COMMA", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("expression", isToken: false),
              Repetition(
                  element: Sequence(elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("expression", isToken: false)
              ])),
              Optional(element: RuleReference("COMMA", isToken: true))
            ]))
          ])
        ]),
        lineNumber: 1038),
    GrammarRule(
        name: "paren_expr",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("paren_body", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 1055),
    GrammarRule(
        name: "paren_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("COMMA", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("expression", isToken: false),
              Repetition(
                  element: Sequence(elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("expression", isToken: false)
              ])),
              Optional(element: RuleReference("COMMA", isToken: true))
            ]))
          ]),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 1057),
    GrammarRule(
        name: "comp_clause",
        body: Sequence(elements: [
          RuleReference("comp_for", isToken: false),
          Repetition(
              element: Alternation(choices: [
            RuleReference("comp_for", isToken: false),
            RuleReference("comp_if", isToken: false)
          ]))
        ]),
        lineNumber: 1087),
    GrammarRule(
        name: "comp_for",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("target_list", isToken: false),
          Literal("in"),
          RuleReference("or_expr", isToken: false)
        ]),
        lineNumber: 1089),
    GrammarRule(
        name: "comp_if",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("or_expr", isToken: false)
        ]),
        lineNumber: 1091),
    GrammarRule(
        name: "arguments",
        body: Sequence(elements: [
          RuleReference("argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("argument", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 1116),
    GrammarRule(
        name: "argument",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 1118),
    GrammarRule(
        name: "yield_expr",
        body: Sequence(elements: [
          Literal("yield"),
          Optional(
              element: Alternation(choices: [
            Sequence(elements: [
              Literal("from"),
              RuleReference("expression", isToken: false)
            ]),
            RuleReference("expression_list", isToken: false)
          ]))
        ]),
        lineNumber: 1147),
  ],
);

final parserGrammarLegacyV3_10 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "file",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("statement", isToken: false)
        ])),
        lineNumber: 56),
    GrammarRule(
        name: "statement",
        body: Alternation(choices: [
          RuleReference("compound_stmt", isToken: false),
          RuleReference("simple_stmt", isToken: false)
        ]),
        lineNumber: 70),
    GrammarRule(
        name: "simple_stmt",
        body: Sequence(elements: [
          RuleReference("small_stmt", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("small_stmt", isToken: false)
          ])),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 74),
    GrammarRule(
        name: "small_stmt",
        body: Alternation(choices: [
          RuleReference("return_stmt", isToken: false),
          RuleReference("raise_stmt", isToken: false),
          RuleReference("assert_stmt", isToken: false),
          RuleReference("del_stmt", isToken: false),
          RuleReference("pass_stmt", isToken: false),
          RuleReference("break_stmt", isToken: false),
          RuleReference("continue_stmt", isToken: false),
          RuleReference("import_stmt", isToken: false),
          RuleReference("global_stmt", isToken: false),
          RuleReference("nonlocal_stmt", isToken: false),
          RuleReference("assign_stmt", isToken: false)
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "return_stmt",
        body: Sequence(elements: [
          Literal("return"),
          Optional(element: RuleReference("expression_list", isToken: false))
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "raise_stmt",
        body: Sequence(elements: [
          Literal("raise"),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Optional(
                element: Sequence(elements: [
              Literal("from"),
              RuleReference("expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 102),
    GrammarRule(
        name: "assert_stmt",
        body: Sequence(elements: [
          Literal("assert"),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 107),
    GrammarRule(
        name: "del_stmt",
        body: Sequence(elements: [
          Literal("del"),
          RuleReference("target_list", isToken: false)
        ]),
        lineNumber: 114),
    GrammarRule(name: "pass_stmt", body: Literal("pass"), lineNumber: 119),
    GrammarRule(name: "break_stmt", body: Literal("break"), lineNumber: 122),
    GrammarRule(
        name: "continue_stmt", body: Literal("continue"), lineNumber: 125),
    GrammarRule(
        name: "import_stmt",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("import"),
            RuleReference("dotted_name_list", isToken: false)
          ]),
          Sequence(elements: [
            Literal("from"),
            RuleReference("import_from", isToken: false),
            Literal("import"),
            RuleReference("import_targets", isToken: false)
          ])
        ]),
        lineNumber: 134),
    GrammarRule(
        name: "dotted_name_list",
        body: Sequence(elements: [
          RuleReference("dotted_name", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("dotted_name", isToken: false)
          ]))
        ]),
        lineNumber: 137),
    GrammarRule(
        name: "dotted_name",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 138),
    GrammarRule(
        name: "import_from",
        body: Alternation(choices: [
          Sequence(elements: [
            Repetition(element: RuleReference("DOT", isToken: true)),
            RuleReference("dotted_name", isToken: false)
          ]),
          Sequence(elements: [
            Repetition(element: RuleReference("DOT", isToken: true)),
            RuleReference("DOT", isToken: true)
          ])
        ]),
        lineNumber: 140),
    GrammarRule(
        name: "import_targets",
        body: Alternation(choices: [
          RuleReference("STAR", isToken: true),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("import_name_list", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ]),
          RuleReference("import_name_list", isToken: false)
        ]),
        lineNumber: 141),
    GrammarRule(
        name: "import_name_list",
        body: Sequence(elements: [
          RuleReference("import_name", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("import_name", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 144),
    GrammarRule(
        name: "import_name",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 145),
    GrammarRule(
        name: "global_stmt",
        body: Sequence(elements: [
          Literal("global"),
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 150),
    GrammarRule(
        name: "nonlocal_stmt",
        body: Sequence(elements: [
          Literal("nonlocal"),
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 159),
    GrammarRule(
        name: "assign_stmt",
        body: Sequence(elements: [
          RuleReference("expression_list", isToken: false),
          Optional(
              element: Alternation(choices: [
            RuleReference("assign_suffix", isToken: false),
            RuleReference("augmented_assign", isToken: false),
            RuleReference("annotation", isToken: false)
          ]))
        ]),
        lineNumber: 175),
    GrammarRule(
        name: "assign_suffix",
        body: Sequence(elements: [
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression_list", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            RuleReference("expression_list", isToken: false)
          ]))
        ]),
        lineNumber: 178),
    GrammarRule(
        name: "augmented_assign",
        body: Sequence(elements: [
          RuleReference("augmented_assign_op", isToken: false),
          RuleReference("expression_list", isToken: false)
        ]),
        lineNumber: 180),
    GrammarRule(
        name: "augmented_assign_op",
        body: Alternation(choices: [
          RuleReference("PLUS_EQUALS", isToken: true),
          RuleReference("MINUS_EQUALS", isToken: true),
          RuleReference("STAR_EQUALS", isToken: true),
          RuleReference("SLASH_EQUALS", isToken: true),
          RuleReference("FLOOR_DIV_EQUALS", isToken: true),
          RuleReference("PERCENT_EQUALS", isToken: true),
          RuleReference("AMP_EQUALS", isToken: true),
          RuleReference("PIPE_EQUALS", isToken: true),
          RuleReference("CARET_EQUALS", isToken: true),
          RuleReference("LEFT_SHIFT_EQUALS", isToken: true),
          RuleReference("RIGHT_SHIFT_EQUALS", isToken: true),
          RuleReference("DOUBLE_STAR_EQUALS", isToken: true),
          RuleReference("AT_EQUALS", isToken: true)
        ]),
        lineNumber: 182),
    GrammarRule(
        name: "annotation",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            RuleReference("expression_list", isToken: false)
          ]))
        ]),
        lineNumber: 191),
    GrammarRule(
        name: "compound_stmt",
        body: Alternation(choices: [
          RuleReference("if_stmt", isToken: false),
          RuleReference("for_stmt", isToken: false),
          RuleReference("while_stmt", isToken: false),
          RuleReference("with_stmt", isToken: false),
          RuleReference("try_stmt", isToken: false),
          RuleReference("def_stmt", isToken: false),
          RuleReference("class_stmt", isToken: false),
          RuleReference("async_stmt", isToken: false),
          RuleReference("match_stmt", isToken: false)
        ]),
        lineNumber: 199),
    GrammarRule(
        name: "if_stmt",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("elif"),
            RuleReference("expression", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ]))
        ]),
        lineNumber: 217),
    GrammarRule(
        name: "for_stmt",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("target_list", isToken: false),
          Literal("in"),
          RuleReference("expression_list", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ]))
        ]),
        lineNumber: 230),
    GrammarRule(
        name: "while_stmt",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ]))
        ]),
        lineNumber: 239),
    GrammarRule(
        name: "with_stmt",
        body: Sequence(elements: [
          Literal("with"),
          RuleReference("with_items", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 265),
    GrammarRule(
        name: "with_items",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("with_item", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("with_item", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true)),
            RuleReference("RPAREN", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("with_item", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("with_item", isToken: false)
            ]))
          ])
        ]),
        lineNumber: 267),
    GrammarRule(
        name: "with_item",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("target", isToken: false)
          ]))
        ]),
        lineNumber: 270),
    GrammarRule(
        name: "try_stmt",
        body: Sequence(elements: [
          Literal("try"),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Group(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("except_clauses", isToken: false),
              Optional(
                  element: Sequence(elements: [
                Literal("else"),
                RuleReference("COLON", isToken: true),
                RuleReference("suite", isToken: false)
              ])),
              Optional(
                  element: Sequence(elements: [
                Literal("finally"),
                RuleReference("COLON", isToken: true),
                RuleReference("suite", isToken: false)
              ]))
            ]),
            Sequence(elements: [
              Literal("finally"),
              RuleReference("COLON", isToken: true),
              RuleReference("suite", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 286),
    GrammarRule(
        name: "except_clauses",
        body: Sequence(elements: [
          RuleReference("except_clause", isToken: false),
          Repetition(element: RuleReference("except_clause", isToken: false))
        ]),
        lineNumber: 290),
    GrammarRule(
        name: "except_clause",
        body: Sequence(elements: [
          Literal("except"),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Optional(
                element: Sequence(elements: [
              Literal("as"),
              RuleReference("NAME", isToken: true)
            ]))
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 291),
    GrammarRule(
        name: "def_stmt",
        body: Sequence(elements: [
          Repetition(element: RuleReference("decorator", isToken: false)),
          Literal("def"),
          RuleReference("NAME", isToken: true),
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("parameters", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 302),
    GrammarRule(
        name: "class_stmt",
        body: Sequence(elements: [
          Repetition(element: RuleReference("decorator", isToken: false)),
          Literal("class"),
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 315),
    GrammarRule(
        name: "decorator",
        body: Sequence(elements: [
          RuleReference("AT", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 323),
    GrammarRule(
        name: "async_stmt",
        body: Sequence(elements: [
          Literal("async"),
          Group(
              element: Alternation(choices: [
            RuleReference("def_stmt", isToken: false),
            RuleReference("for_stmt", isToken: false),
            RuleReference("with_stmt", isToken: false)
          ]))
        ]),
        lineNumber: 331),
    GrammarRule(
        name: "suite",
        body: Alternation(choices: [
          RuleReference("simple_stmt", isToken: false),
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("INDENT", isToken: true),
            Repetition(element: RuleReference("statement", isToken: false)),
            RuleReference("DEDENT", isToken: true)
          ])
        ]),
        lineNumber: 340),
    GrammarRule(
        name: "match_stmt",
        body: Sequence(elements: [
          Literal("match"),
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("NEWLINE", isToken: true),
          RuleReference("INDENT", isToken: true),
          Repetition(element: RuleReference("case_clause", isToken: false)),
          RuleReference("DEDENT", isToken: true)
        ]),
        lineNumber: 401),
    GrammarRule(
        name: "case_clause",
        body: Sequence(elements: [
          Literal("case"),
          RuleReference("pattern", isToken: false),
          Optional(element: RuleReference("guard", isToken: false)),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 418),
    GrammarRule(
        name: "guard",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 427),
    GrammarRule(
        name: "pattern",
        body: Sequence(elements: [
          RuleReference("or_pattern", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 462),
    GrammarRule(
        name: "or_pattern",
        body: Sequence(elements: [
          RuleReference("closed_pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("PIPE", isToken: true),
            RuleReference("closed_pattern", isToken: false)
          ]))
        ]),
        lineNumber: 477),
    GrammarRule(
        name: "closed_pattern",
        body: Alternation(choices: [
          RuleReference("literal_pattern", isToken: false),
          RuleReference("capture_pattern", isToken: false),
          RuleReference("wildcard_pattern", isToken: false),
          RuleReference("value_pattern", isToken: false),
          RuleReference("group_pattern", isToken: false),
          RuleReference("sequence_pattern", isToken: false),
          RuleReference("mapping_pattern", isToken: false),
          RuleReference("class_pattern", isToken: false),
          RuleReference("star_pattern", isToken: false)
        ]),
        lineNumber: 486),
    GrammarRule(
        name: "literal_pattern",
        body: Alternation(choices: [
          RuleReference("signed_number", isToken: false),
          RuleReference("complex_number", isToken: false),
          Sequence(elements: [
            RuleReference("STRING", isToken: true),
            Repetition(element: RuleReference("STRING", isToken: true))
          ]),
          Literal("True"),
          Literal("False"),
          Literal("None")
        ]),
        lineNumber: 523),
    GrammarRule(
        name: "signed_number",
        body: Sequence(elements: [
          Optional(element: RuleReference("MINUS", isToken: true)),
          Group(
              element: Alternation(choices: [
            RuleReference("INT", isToken: true),
            RuleReference("FLOAT", isToken: true)
          ]))
        ]),
        lineNumber: 533),
    GrammarRule(
        name: "complex_number",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("signed_number", isToken: false),
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("IMAG", isToken: true)
          ]),
          Sequence(elements: [
            Optional(element: RuleReference("MINUS", isToken: true)),
            RuleReference("IMAG", isToken: true)
          ])
        ]),
        lineNumber: 542),
    GrammarRule(
        name: "capture_pattern",
        body: RuleReference("NAME", isToken: true),
        lineNumber: 572),
    GrammarRule(name: "wildcard_pattern", body: Literal("_"), lineNumber: 592),
    GrammarRule(
        name: "value_pattern",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("DOT", isToken: true),
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 619),
    GrammarRule(
        name: "group_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 635),
    GrammarRule(
        name: "sequence_pattern",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("LBRACKET", isToken: true),
            Optional(
                element:
                    RuleReference("sequence_pattern_content", isToken: false)),
            RuleReference("RBRACKET", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("sequence_pattern_tuple_content", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 663),
    GrammarRule(
        name: "sequence_pattern_content",
        body: Sequence(elements: [
          RuleReference("maybe_star_pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("maybe_star_pattern", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 667),
    GrammarRule(
        name: "sequence_pattern_tuple_content",
        body: Sequence(elements: [
          RuleReference("maybe_star_pattern", isToken: false),
          RuleReference("COMMA", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("maybe_star_pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("maybe_star_pattern", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ]))
        ]),
        lineNumber: 671),
    GrammarRule(
        name: "maybe_star_pattern",
        body: Alternation(choices: [
          RuleReference("star_pattern", isToken: false),
          RuleReference("pattern", isToken: false)
        ]),
        lineNumber: 674),
    GrammarRule(
        name: "star_pattern",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(
              elements: [RuleReference("STAR", isToken: true), Literal("_")])
        ]),
        lineNumber: 689),
    GrammarRule(
        name: "mapping_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element:
                  RuleReference("mapping_pattern_content", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 724),
    GrammarRule(
        name: "mapping_pattern_content",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("mapping_entry", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("mapping_entry", isToken: false)
            ])),
            Optional(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              Optional(
                  element: RuleReference("double_star_capture", isToken: false))
            ]))
          ]),
          RuleReference("double_star_capture", isToken: false)
        ]),
        lineNumber: 726),
    GrammarRule(
        name: "mapping_entry",
        body: Sequence(elements: [
          Group(
              element: Alternation(choices: [
            RuleReference("literal_pattern", isToken: false),
            RuleReference("value_pattern", isToken: false)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("pattern", isToken: false)
        ]),
        lineNumber: 729),
    GrammarRule(
        name: "double_star_capture",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            Literal("_")
          ])
        ]),
        lineNumber: 732),
    GrammarRule(
        name: "class_pattern",
        body: Sequence(elements: [
          RuleReference("name_or_dotted", isToken: false),
          RuleReference("LPAREN", isToken: true),
          Optional(
              element: RuleReference("class_pattern_content", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 785),
    GrammarRule(
        name: "name_or_dotted",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 787),
    GrammarRule(
        name: "class_pattern_content",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("class_positional_patterns", isToken: false),
            Optional(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("class_keyword_patterns", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ]),
          Sequence(elements: [
            RuleReference("class_keyword_patterns", isToken: false),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])
        ]),
        lineNumber: 789),
    GrammarRule(
        name: "class_positional_patterns",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("pattern", isToken: false)
          ]))
        ]),
        lineNumber: 792),
    GrammarRule(
        name: "class_keyword_patterns",
        body: Sequence(elements: [
          RuleReference("keyword_pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("keyword_pattern", isToken: false)
          ]))
        ]),
        lineNumber: 794),
    GrammarRule(
        name: "keyword_pattern",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("pattern", isToken: false)
        ]),
        lineNumber: 796),
    GrammarRule(
        name: "expression_list",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 828),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("lambda_expr", isToken: false),
          RuleReference("walrus_expr", isToken: false)
        ]),
        lineNumber: 834),
    GrammarRule(
        name: "walrus_expr",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("or_expr", isToken: false),
            Optional(
                element: Sequence(elements: [
              Literal("if"),
              RuleReference("or_expr", isToken: false),
              Literal("else"),
              RuleReference("expression", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            RuleReference("or_expr", isToken: false),
            RuleReference("WALRUS", isToken: true),
            RuleReference("expression", isToken: false)
          ])
        ]),
        lineNumber: 837),
    GrammarRule(
        name: "lambda_expr",
        body: Sequence(elements: [
          Literal("lambda"),
          Optional(element: RuleReference("lambda_params", isToken: false)),
          RuleReference("COLON", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 844),
    GrammarRule(
        name: "lambda_params",
        body: Sequence(elements: [
          RuleReference("lambda_param", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("lambda_param", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 845),
    GrammarRule(
        name: "lambda_param",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          RuleReference("STAR", isToken: true)
        ]),
        lineNumber: 846),
    GrammarRule(
        name: "or_expr",
        body: Sequence(elements: [
          RuleReference("and_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("or"),
            RuleReference("and_expr", isToken: false)
          ]))
        ]),
        lineNumber: 849),
    GrammarRule(
        name: "and_expr",
        body: Sequence(elements: [
          RuleReference("not_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            RuleReference("not_expr", isToken: false)
          ]))
        ]),
        lineNumber: 852),
    GrammarRule(
        name: "not_expr",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("not"),
            RuleReference("not_expr", isToken: false)
          ]),
          RuleReference("comparison", isToken: false)
        ]),
        lineNumber: 855),
    GrammarRule(
        name: "comparison",
        body: Sequence(elements: [
          RuleReference("bitwise_or", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("comp_op", isToken: false),
            RuleReference("bitwise_or", isToken: false)
          ]))
        ]),
        lineNumber: 862),
    GrammarRule(
        name: "comp_op",
        body: Alternation(choices: [
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          Literal("in"),
          Sequence(elements: [Literal("not"), Literal("in")]),
          Literal("is"),
          Sequence(elements: [Literal("is"), Literal("not")])
        ]),
        lineNumber: 864),
    GrammarRule(
        name: "bitwise_or",
        body: Sequence(elements: [
          RuleReference("bitwise_xor", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("PIPE", isToken: true),
            RuleReference("bitwise_xor", isToken: false)
          ]))
        ]),
        lineNumber: 871),
    GrammarRule(
        name: "bitwise_xor",
        body: Sequence(elements: [
          RuleReference("bitwise_and", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("CARET", isToken: true),
            RuleReference("bitwise_and", isToken: false)
          ]))
        ]),
        lineNumber: 872),
    GrammarRule(
        name: "bitwise_and",
        body: Sequence(elements: [
          RuleReference("shift", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("AMP", isToken: true),
            RuleReference("shift", isToken: false)
          ]))
        ]),
        lineNumber: 873),
    GrammarRule(
        name: "shift",
        body: Sequence(elements: [
          RuleReference("arith", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("LEFT_SHIFT", isToken: true),
              RuleReference("RIGHT_SHIFT", isToken: true)
            ])),
            RuleReference("arith", isToken: false)
          ]))
        ]),
        lineNumber: 876),
    GrammarRule(
        name: "arith",
        body: Sequence(elements: [
          RuleReference("term", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("term", isToken: false)
          ]))
        ]),
        lineNumber: 879),
    GrammarRule(
        name: "term",
        body: Sequence(elements: [
          RuleReference("factor", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true),
              RuleReference("FLOOR_DIV", isToken: true),
              RuleReference("PERCENT", isToken: true),
              RuleReference("AT", isToken: true)
            ])),
            RuleReference("factor", isToken: false)
          ]))
        ]),
        lineNumber: 884),
    GrammarRule(
        name: "factor",
        body: Alternation(choices: [
          Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true),
              RuleReference("TILDE", isToken: true)
            ])),
            RuleReference("factor", isToken: false)
          ]),
          RuleReference("power", isToken: false)
        ]),
        lineNumber: 887),
    GrammarRule(
        name: "power",
        body: Sequence(elements: [
          RuleReference("await_expr", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("factor", isToken: false)
          ]))
        ]),
        lineNumber: 892),
    GrammarRule(
        name: "await_expr",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("await"),
            RuleReference("primary", isToken: false)
          ]),
          RuleReference("primary", isToken: false)
        ]),
        lineNumber: 897),
    GrammarRule(
        name: "primary",
        body: Sequence(elements: [
          RuleReference("atom", isToken: false),
          Repetition(element: RuleReference("suffix", isToken: false))
        ]),
        lineNumber: 906),
    GrammarRule(
        name: "suffix",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LBRACKET", isToken: true),
            RuleReference("subscript", isToken: false),
            RuleReference("RBRACKET", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 908),
    GrammarRule(
        name: "subscript",
        body: Sequence(elements: [
          RuleReference("subscript_item", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("subscript_item", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 916),
    GrammarRule(
        name: "subscript_item",
        body: Alternation(choices: [
          RuleReference("expression", isToken: false),
          Sequence(elements: [
            Optional(element: RuleReference("expression", isToken: false)),
            RuleReference("COLON", isToken: true),
            Optional(element: RuleReference("expression", isToken: false)),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              Optional(element: RuleReference("expression", isToken: false))
            ]))
          ])
        ]),
        lineNumber: 918),
    GrammarRule(
        name: "atom",
        body: Alternation(choices: [
          RuleReference("INT", isToken: true),
          RuleReference("FLOAT", isToken: true),
          RuleReference("IMAG", isToken: true),
          Sequence(elements: [
            RuleReference("STRING", isToken: true),
            Repetition(element: RuleReference("STRING", isToken: true))
          ]),
          RuleReference("NAME", isToken: true),
          RuleReference("ELLIPSIS", isToken: true),
          Literal("True"),
          Literal("False"),
          Literal("None"),
          RuleReference("list_expr", isToken: false),
          RuleReference("dict_or_set_expr", isToken: false),
          RuleReference("paren_expr", isToken: false)
        ]),
        lineNumber: 927),
    GrammarRule(
        name: "list_expr",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(element: RuleReference("list_body", isToken: false)),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 944),
    GrammarRule(
        name: "list_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("expression", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])
        ]),
        lineNumber: 946),
    GrammarRule(
        name: "dict_or_set_expr",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(element: RuleReference("dict_or_set_body", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 958),
    GrammarRule(
        name: "dict_or_set_body",
        body: Alternation(choices: [
          RuleReference("dict_body", isToken: false),
          RuleReference("set_body", isToken: false)
        ]),
        lineNumber: 960),
    GrammarRule(
        name: "dict_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("dict_entry", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("dict_entry", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("dict_entry", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ]),
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              Group(
                  element: Alternation(choices: [
                RuleReference("dict_entry", isToken: false),
                Sequence(elements: [
                  RuleReference("DOUBLE_STAR", isToken: true),
                  RuleReference("expression", isToken: false)
                ])
              ]))
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])
        ]),
        lineNumber: 962),
    GrammarRule(
        name: "dict_entry",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 966),
    GrammarRule(
        name: "set_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("expression", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])
        ]),
        lineNumber: 968),
    GrammarRule(
        name: "paren_expr",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("paren_body", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 977),
    GrammarRule(
        name: "paren_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("COMMA", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("expression", isToken: false),
              Repetition(
                  element: Sequence(elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("expression", isToken: false)
              ])),
              Optional(element: RuleReference("COMMA", isToken: true))
            ]))
          ]),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 979),
    GrammarRule(
        name: "comp_clause",
        body: Sequence(elements: [
          RuleReference("comp_for", isToken: false),
          Repetition(
              element: Alternation(choices: [
            RuleReference("comp_for", isToken: false),
            RuleReference("comp_if", isToken: false)
          ]))
        ]),
        lineNumber: 999),
    GrammarRule(
        name: "comp_for",
        body: Sequence(elements: [
          Optional(element: Literal("async")),
          Literal("for"),
          RuleReference("target_list", isToken: false),
          Literal("in"),
          RuleReference("or_expr", isToken: false)
        ]),
        lineNumber: 1001),
    GrammarRule(
        name: "comp_if",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("or_expr", isToken: false)
        ]),
        lineNumber: 1003),
    GrammarRule(
        name: "target",
        body: RuleReference("primary", isToken: false),
        lineNumber: 1020),
    GrammarRule(
        name: "target_list",
        body: Sequence(elements: [
          RuleReference("target", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("target", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 1021),
    GrammarRule(
        name: "arguments",
        body: Sequence(elements: [
          RuleReference("argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("argument", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 1039),
    GrammarRule(
        name: "argument",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 1041),
    GrammarRule(
        name: "parameters",
        body: RuleReference("parameter_list", isToken: false),
        lineNumber: 1068),
    GrammarRule(
        name: "parameter_list",
        body: Sequence(elements: [
          RuleReference("param_with_default", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("param_with_default", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            Optional(
                element: Alternation(choices: [
              RuleReference("slash_params", isToken: false),
              RuleReference("star_params", isToken: false),
              RuleReference("double_star_param", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 1070),
    GrammarRule(
        name: "slash_params",
        body: Sequence(elements: [
          RuleReference("SLASH", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("param_with_default", isToken: false),
              Repetition(
                  element: Sequence(elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("param_with_default", isToken: false)
              ])),
              Optional(
                  element: Sequence(elements: [
                RuleReference("COMMA", isToken: true),
                Optional(
                    element: Alternation(choices: [
                  RuleReference("star_params", isToken: false),
                  RuleReference("double_star_param", isToken: false)
                ]))
              ]))
            ]))
          ]))
        ]),
        lineNumber: 1073),
    GrammarRule(
        name: "star_params",
        body: Sequence(elements: [
          RuleReference("STAR", isToken: true),
          Optional(element: RuleReference("NAME", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("param_with_default", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("param_with_default", isToken: false)
            ]))
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            Optional(
                element: RuleReference("double_star_param", isToken: false))
          ]))
        ]),
        lineNumber: 1076),
    GrammarRule(
        name: "double_star_param",
        body: Sequence(elements: [
          RuleReference("DOUBLE_STAR", isToken: true),
          RuleReference("NAME", isToken: true),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 1079),
    GrammarRule(
        name: "param_with_default",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 1081),
  ],
);

final parserGrammarLegacyV3_12 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "file",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("statement", isToken: false)
        ])),
        lineNumber: 34),
    GrammarRule(
        name: "statement",
        body: Alternation(choices: [
          RuleReference("compound_stmt", isToken: false),
          RuleReference("simple_stmt", isToken: false)
        ]),
        lineNumber: 43),
    GrammarRule(
        name: "simple_stmt",
        body: Sequence(elements: [
          RuleReference("small_stmt", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("small_stmt", isToken: false)
          ])),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 45),
    GrammarRule(
        name: "small_stmt",
        body: Alternation(choices: [
          RuleReference("return_stmt", isToken: false),
          RuleReference("import_stmt", isToken: false),
          RuleReference("from_import_stmt", isToken: false),
          RuleReference("raise_stmt", isToken: false),
          RuleReference("pass_stmt", isToken: false),
          RuleReference("del_stmt", isToken: false),
          RuleReference("yield_stmt", isToken: false),
          RuleReference("assert_stmt", isToken: false),
          RuleReference("break_stmt", isToken: false),
          RuleReference("continue_stmt", isToken: false),
          RuleReference("global_stmt", isToken: false),
          RuleReference("nonlocal_stmt", isToken: false),
          RuleReference("type_alias_stmt", isToken: false),
          RuleReference("assign_stmt", isToken: false)
        ]),
        lineNumber: 47),
    GrammarRule(
        name: "return_stmt",
        body: Sequence(elements: [
          Literal("return"),
          Optional(element: RuleReference("expression_list", isToken: false))
        ]),
        lineNumber: 66),
    GrammarRule(name: "pass_stmt", body: Literal("pass"), lineNumber: 67),
    GrammarRule(name: "break_stmt", body: Literal("break"), lineNumber: 68),
    GrammarRule(
        name: "continue_stmt", body: Literal("continue"), lineNumber: 69),
    GrammarRule(
        name: "del_stmt",
        body: Sequence(elements: [
          Literal("del"),
          RuleReference("target_list", isToken: false)
        ]),
        lineNumber: 70),
    GrammarRule(
        name: "yield_stmt",
        body: RuleReference("yield_expr", isToken: false),
        lineNumber: 71),
    GrammarRule(
        name: "assert_stmt",
        body: Sequence(elements: [
          Literal("assert"),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 75),
    GrammarRule(
        name: "global_stmt",
        body: Sequence(elements: [
          Literal("global"),
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 80),
    GrammarRule(
        name: "nonlocal_stmt",
        body: Sequence(elements: [
          Literal("nonlocal"),
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 81),
    GrammarRule(
        name: "import_stmt",
        body: Sequence(elements: [
          Literal("import"),
          RuleReference("dotted_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ])),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("dotted_name", isToken: false),
            Optional(
                element: Sequence(elements: [
              Literal("as"),
              RuleReference("NAME", isToken: true)
            ]))
          ]))
        ]),
        lineNumber: 86),
    GrammarRule(
        name: "from_import_stmt",
        body: Sequence(elements: [
          Literal("from"),
          Group(
              element: Alternation(choices: [
            Sequence(elements: [
              Repetition(element: RuleReference("DOT", isToken: true)),
              RuleReference("dotted_name", isToken: false)
            ]),
            Sequence(elements: [
              Repetition(element: RuleReference("DOT", isToken: true)),
              RuleReference("DOT", isToken: true)
            ])
          ])),
          Literal("import"),
          Group(
              element: Alternation(choices: [
            RuleReference("STAR", isToken: true),
            RuleReference("import_names", isToken: false)
          ]))
        ]),
        lineNumber: 93),
    GrammarRule(
        name: "import_names",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ])),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              Literal("as"),
              RuleReference("NAME", isToken: true)
            ]))
          ]))
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "dotted_name",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 97),
    GrammarRule(
        name: "raise_stmt",
        body: Sequence(elements: [
          Literal("raise"),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Optional(
                element: Sequence(elements: [
              Literal("from"),
              RuleReference("expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 103),
    GrammarRule(
        name: "type_alias_stmt",
        body: Sequence(elements: [
          Literal("type"),
          RuleReference("NAME", isToken: true),
          Optional(element: RuleReference("type_params", isToken: false)),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 119),
    GrammarRule(
        name: "type_params",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("type_param", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_param", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true)),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 136),
    GrammarRule(
        name: "type_param",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ])
        ]),
        lineNumber: 138),
    GrammarRule(
        name: "assign_stmt",
        body: Sequence(elements: [
          RuleReference("expression_list", isToken: false),
          Optional(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("COLON", isToken: true),
              RuleReference("expression", isToken: false),
              Optional(
                  element: Sequence(elements: [
                RuleReference("EQUALS", isToken: true),
                RuleReference("expression_list", isToken: false)
              ]))
            ]),
            Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("expression_list", isToken: false),
              Repetition(
                  element: Sequence(elements: [
                RuleReference("EQUALS", isToken: true),
                RuleReference("expression_list", isToken: false)
              ]))
            ]),
            Sequence(elements: [
              RuleReference("augmented_assign_op", isToken: false),
              RuleReference("expression_list", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 154),
    GrammarRule(
        name: "augmented_assign_op",
        body: Alternation(choices: [
          RuleReference("PLUS_EQUALS", isToken: true),
          RuleReference("MINUS_EQUALS", isToken: true),
          RuleReference("STAR_EQUALS", isToken: true),
          RuleReference("SLASH_EQUALS", isToken: true),
          RuleReference("FLOOR_DIV_EQUALS", isToken: true),
          RuleReference("PERCENT_EQUALS", isToken: true),
          RuleReference("AMP_EQUALS", isToken: true),
          RuleReference("PIPE_EQUALS", isToken: true),
          RuleReference("CARET_EQUALS", isToken: true),
          RuleReference("LEFT_SHIFT_EQUALS", isToken: true),
          RuleReference("RIGHT_SHIFT_EQUALS", isToken: true),
          RuleReference("DOUBLE_STAR_EQUALS", isToken: true),
          RuleReference("AT_EQUALS", isToken: true)
        ]),
        lineNumber: 159),
    GrammarRule(
        name: "compound_stmt",
        body: Alternation(choices: [
          RuleReference("if_stmt", isToken: false),
          RuleReference("for_stmt", isToken: false),
          RuleReference("while_stmt", isToken: false),
          RuleReference("try_stmt", isToken: false),
          RuleReference("with_stmt", isToken: false),
          RuleReference("def_stmt", isToken: false),
          RuleReference("class_stmt", isToken: false),
          RuleReference("decorated", isToken: false),
          RuleReference("async_stmt", isToken: false),
          RuleReference("match_stmt", isToken: false)
        ]),
        lineNumber: 168),
    GrammarRule(
        name: "if_stmt",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("elif"),
            RuleReference("expression", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ]))
        ]),
        lineNumber: 180),
    GrammarRule(
        name: "for_stmt",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("target_list", isToken: false),
          Literal("in"),
          RuleReference("expression_list", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ]))
        ]),
        lineNumber: 186),
    GrammarRule(
        name: "while_stmt",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("COLON", isToken: true),
            RuleReference("suite", isToken: false)
          ]))
        ]),
        lineNumber: 190),
    GrammarRule(
        name: "try_stmt",
        body: Sequence(elements: [
          Literal("try"),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false),
          Group(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("except_clauses", isToken: false),
              Optional(
                  element: Sequence(elements: [
                Literal("else"),
                RuleReference("COLON", isToken: true),
                RuleReference("suite", isToken: false)
              ])),
              Optional(
                  element: Sequence(elements: [
                Literal("finally"),
                RuleReference("COLON", isToken: true),
                RuleReference("suite", isToken: false)
              ]))
            ]),
            Sequence(elements: [
              RuleReference("except_star_clauses", isToken: false),
              Optional(
                  element: Sequence(elements: [
                Literal("else"),
                RuleReference("COLON", isToken: true),
                RuleReference("suite", isToken: false)
              ])),
              Optional(
                  element: Sequence(elements: [
                Literal("finally"),
                RuleReference("COLON", isToken: true),
                RuleReference("suite", isToken: false)
              ]))
            ]),
            Sequence(elements: [
              Literal("finally"),
              RuleReference("COLON", isToken: true),
              RuleReference("suite", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 202),
    GrammarRule(
        name: "except_clauses",
        body: Sequence(elements: [
          RuleReference("except_clause", isToken: false),
          Repetition(element: RuleReference("except_clause", isToken: false))
        ]),
        lineNumber: 207),
    GrammarRule(
        name: "except_clause",
        body: Sequence(elements: [
          Literal("except"),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Optional(
                element: Sequence(elements: [
              Literal("as"),
              RuleReference("NAME", isToken: true)
            ]))
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 214),
    GrammarRule(
        name: "except_star_clauses",
        body: Sequence(elements: [
          RuleReference("except_star_clause", isToken: false),
          Repetition(
              element: RuleReference("except_star_clause", isToken: false))
        ]),
        lineNumber: 234),
    GrammarRule(
        name: "except_star_clause",
        body: Sequence(elements: [
          Literal("except"),
          RuleReference("STAR", isToken: true),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 236),
    GrammarRule(
        name: "with_stmt",
        body: Sequence(elements: [
          Literal("with"),
          RuleReference("with_items", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 241),
    GrammarRule(
        name: "with_items",
        body: Sequence(elements: [
          RuleReference("with_item", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("with_item", isToken: false)
          ]))
        ]),
        lineNumber: 242),
    GrammarRule(
        name: "with_item",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("target", isToken: false)
          ]))
        ]),
        lineNumber: 243),
    GrammarRule(
        name: "def_stmt",
        body: Sequence(elements: [
          Literal("def"),
          RuleReference("NAME", isToken: true),
          Optional(element: RuleReference("type_params", isToken: false)),
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("parameters", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 258),
    GrammarRule(
        name: "parameters",
        body: Sequence(elements: [
          RuleReference("parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("parameter", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 280),
    GrammarRule(
        name: "parameter",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ]),
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true),
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              RuleReference("expression", isToken: false)
            ])),
            Optional(
                element: Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ])
        ]),
        lineNumber: 282),
    GrammarRule(
        name: "class_stmt",
        body: Sequence(elements: [
          Literal("class"),
          RuleReference("NAME", isToken: true),
          Optional(element: RuleReference("type_params", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 300),
    GrammarRule(
        name: "decorated",
        body: Sequence(elements: [
          RuleReference("decorator", isToken: false),
          Repetition(element: RuleReference("decorator", isToken: false)),
          Group(
              element: Alternation(choices: [
            RuleReference("def_stmt", isToken: false),
            RuleReference("class_stmt", isToken: false),
            RuleReference("async_stmt", isToken: false)
          ]))
        ]),
        lineNumber: 307),
    GrammarRule(
        name: "decorator",
        body: Sequence(elements: [
          RuleReference("AT", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 309),
    GrammarRule(
        name: "async_stmt",
        body: Sequence(elements: [
          Literal("async"),
          Group(
              element: Alternation(choices: [
            RuleReference("def_stmt", isToken: false),
            RuleReference("for_stmt", isToken: false),
            RuleReference("with_stmt", isToken: false)
          ]))
        ]),
        lineNumber: 313),
    GrammarRule(
        name: "match_stmt",
        body: Sequence(elements: [
          Literal("match"),
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("NEWLINE", isToken: true),
          RuleReference("INDENT", isToken: true),
          Repetition(element: RuleReference("case_clause", isToken: false)),
          RuleReference("DEDENT", isToken: true)
        ]),
        lineNumber: 329),
    GrammarRule(
        name: "case_clause",
        body: Sequence(elements: [
          Literal("case"),
          RuleReference("pattern", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("if"),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 333),
    GrammarRule(
        name: "pattern",
        body: Sequence(elements: [
          RuleReference("or_pattern", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 344),
    GrammarRule(
        name: "or_pattern",
        body: Sequence(elements: [
          RuleReference("closed_pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("PIPE", isToken: true),
            RuleReference("closed_pattern", isToken: false)
          ]))
        ]),
        lineNumber: 346),
    GrammarRule(
        name: "closed_pattern",
        body: Alternation(choices: [
          RuleReference("literal_pattern", isToken: false),
          RuleReference("capture_pattern", isToken: false),
          RuleReference("wildcard_pattern", isToken: false),
          RuleReference("sequence_pattern", isToken: false),
          RuleReference("mapping_pattern", isToken: false),
          RuleReference("class_pattern", isToken: false),
          RuleReference("group_pattern", isToken: false),
          RuleReference("value_pattern", isToken: false)
        ]),
        lineNumber: 348),
    GrammarRule(
        name: "literal_pattern",
        body: Alternation(choices: [
          RuleReference("INT", isToken: true),
          RuleReference("FLOAT", isToken: true),
          Sequence(elements: [
            RuleReference("STRING", isToken: true),
            Repetition(element: RuleReference("STRING", isToken: true))
          ]),
          RuleReference("IMAG", isToken: true),
          Literal("None"),
          Literal("True"),
          Literal("False"),
          Sequence(elements: [
            RuleReference("MINUS", isToken: true),
            Group(
                element: Alternation(choices: [
              RuleReference("INT", isToken: true),
              RuleReference("FLOAT", isToken: true),
              RuleReference("IMAG", isToken: true)
            ]))
          ])
        ]),
        lineNumber: 358),
    GrammarRule(
        name: "capture_pattern",
        body: RuleReference("NAME", isToken: true),
        lineNumber: 362),
    GrammarRule(name: "wildcard_pattern", body: Literal("_"), lineNumber: 363),
    GrammarRule(
        name: "sequence_pattern",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("LBRACKET", isToken: true),
            Optional(element: RuleReference("pattern_list", isToken: false)),
            RuleReference("RBRACKET", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("pattern_list", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 366),
    GrammarRule(
        name: "pattern_list",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("pattern", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 369),
    GrammarRule(
        name: "mapping_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(element: RuleReference("mapping_items", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 372),
    GrammarRule(
        name: "mapping_items",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("mapping_item", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("mapping_item", isToken: false)
            ])),
            Optional(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              Optional(
                  element: Sequence(elements: [
                RuleReference("DOUBLE_STAR", isToken: true),
                RuleReference("NAME", isToken: true)
              ]))
            ]))
          ]),
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ])
        ]),
        lineNumber: 374),
    GrammarRule(
        name: "mapping_item",
        body: Sequence(elements: [
          Group(
              element: Alternation(choices: [
            RuleReference("literal_pattern", isToken: false),
            RuleReference("value_pattern", isToken: false)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("pattern", isToken: false)
        ]),
        lineNumber: 377),
    GrammarRule(
        name: "class_pattern",
        body: Sequence(elements: [
          RuleReference("dotted_name", isToken: false),
          RuleReference("LPAREN", isToken: true),
          Optional(
              element: RuleReference("class_pattern_args", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 380),
    GrammarRule(
        name: "class_pattern_args",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("pattern", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ]),
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true),
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("NAME", isToken: true),
              RuleReference("EQUALS", isToken: true),
              RuleReference("pattern", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ]),
          Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("pattern", isToken: false)
            ])),
            RuleReference("COMMA", isToken: true),
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true),
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("NAME", isToken: true),
              RuleReference("EQUALS", isToken: true),
              RuleReference("pattern", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])
        ]),
        lineNumber: 382),
    GrammarRule(
        name: "group_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 387),
    GrammarRule(
        name: "value_pattern",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("DOT", isToken: true),
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 390),
    GrammarRule(
        name: "suite",
        body: Alternation(choices: [
          RuleReference("simple_stmt", isToken: false),
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("INDENT", isToken: true),
            Repetition(element: RuleReference("statement", isToken: false)),
            RuleReference("DEDENT", isToken: true)
          ])
        ]),
        lineNumber: 399),
    GrammarRule(
        name: "target_list",
        body: Sequence(elements: [
          RuleReference("target", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("target", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 405),
    GrammarRule(
        name: "target",
        body: Alternation(choices: [
          RuleReference("NAME", isToken: true),
          Sequence(elements: [
            RuleReference("target", isToken: false),
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("target", isToken: false),
            RuleReference("LBRACKET", isToken: true),
            RuleReference("subscript", isToken: false),
            RuleReference("RBRACKET", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("target", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("target_list", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LBRACKET", isToken: true),
            RuleReference("target_list", isToken: false),
            RuleReference("RBRACKET", isToken: true)
          ])
        ]),
        lineNumber: 407),
    GrammarRule(
        name: "expression_list",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 438),
    GrammarRule(
        name: "named_expression",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("WALRUS", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 442),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("lambda_expr", isToken: false),
          Sequence(elements: [
            RuleReference("or_expr", isToken: false),
            Optional(
                element: Sequence(elements: [
              Literal("if"),
              RuleReference("or_expr", isToken: false),
              Literal("else"),
              RuleReference("expression", isToken: false)
            ]))
          ]),
          RuleReference("named_expression", isToken: false)
        ]),
        lineNumber: 444),
    GrammarRule(
        name: "lambda_expr",
        body: Sequence(elements: [
          Literal("lambda"),
          Optional(element: RuleReference("lambda_params", isToken: false)),
          RuleReference("COLON", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 448),
    GrammarRule(
        name: "lambda_params",
        body: Sequence(elements: [
          RuleReference("lambda_param", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("lambda_param", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 449),
    GrammarRule(
        name: "lambda_param",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true)
        ]),
        lineNumber: 450),
    GrammarRule(
        name: "or_expr",
        body: Sequence(elements: [
          RuleReference("and_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("or"),
            RuleReference("and_expr", isToken: false)
          ]))
        ]),
        lineNumber: 456),
    GrammarRule(
        name: "and_expr",
        body: Sequence(elements: [
          RuleReference("not_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            RuleReference("not_expr", isToken: false)
          ]))
        ]),
        lineNumber: 457),
    GrammarRule(
        name: "not_expr",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("not"),
            RuleReference("not_expr", isToken: false)
          ]),
          RuleReference("comparison", isToken: false)
        ]),
        lineNumber: 458),
    GrammarRule(
        name: "comparison",
        body: Sequence(elements: [
          RuleReference("bitwise_or", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("comp_op", isToken: false),
            RuleReference("bitwise_or", isToken: false)
          ]))
        ]),
        lineNumber: 461),
    GrammarRule(
        name: "comp_op",
        body: Alternation(choices: [
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          Literal("in"),
          Sequence(elements: [Literal("not"), Literal("in")]),
          Literal("is"),
          Sequence(elements: [Literal("is"), Literal("not")])
        ]),
        lineNumber: 463),
    GrammarRule(
        name: "bitwise_or",
        body: Sequence(elements: [
          RuleReference("bitwise_xor", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("PIPE", isToken: true),
            RuleReference("bitwise_xor", isToken: false)
          ]))
        ]),
        lineNumber: 469),
    GrammarRule(
        name: "bitwise_xor",
        body: Sequence(elements: [
          RuleReference("bitwise_and", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("CARET", isToken: true),
            RuleReference("bitwise_and", isToken: false)
          ]))
        ]),
        lineNumber: 470),
    GrammarRule(
        name: "bitwise_and",
        body: Sequence(elements: [
          RuleReference("shift", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("AMP", isToken: true),
            RuleReference("shift", isToken: false)
          ]))
        ]),
        lineNumber: 471),
    GrammarRule(
        name: "shift",
        body: Sequence(elements: [
          RuleReference("arith", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("LEFT_SHIFT", isToken: true),
              RuleReference("RIGHT_SHIFT", isToken: true)
            ])),
            RuleReference("arith", isToken: false)
          ]))
        ]),
        lineNumber: 472),
    GrammarRule(
        name: "arith",
        body: Sequence(elements: [
          RuleReference("term", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("term", isToken: false)
          ]))
        ]),
        lineNumber: 473),
    GrammarRule(
        name: "term",
        body: Sequence(elements: [
          RuleReference("factor", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true),
              RuleReference("FLOOR_DIV", isToken: true),
              RuleReference("PERCENT", isToken: true),
              RuleReference("AT", isToken: true)
            ])),
            RuleReference("factor", isToken: false)
          ]))
        ]),
        lineNumber: 474),
    GrammarRule(
        name: "factor",
        body: Alternation(choices: [
          Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true),
              RuleReference("TILDE", isToken: true)
            ])),
            RuleReference("factor", isToken: false)
          ]),
          RuleReference("power", isToken: false)
        ]),
        lineNumber: 475),
    GrammarRule(
        name: "power",
        body: Sequence(elements: [
          RuleReference("await_expr", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("factor", isToken: false)
          ]))
        ]),
        lineNumber: 479),
    GrammarRule(
        name: "await_expr",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("await"),
            RuleReference("primary", isToken: false)
          ]),
          RuleReference("primary", isToken: false)
        ]),
        lineNumber: 482),
    GrammarRule(
        name: "primary",
        body: Sequence(elements: [
          RuleReference("atom", isToken: false),
          Repetition(element: RuleReference("suffix", isToken: false))
        ]),
        lineNumber: 490),
    GrammarRule(
        name: "suffix",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LBRACKET", isToken: true),
            RuleReference("subscript", isToken: false),
            RuleReference("RBRACKET", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 492),
    GrammarRule(
        name: "subscript",
        body: Alternation(choices: [
          RuleReference("expression", isToken: false),
          Sequence(elements: [
            Optional(element: RuleReference("expression", isToken: false)),
            RuleReference("COLON", isToken: true),
            Optional(element: RuleReference("expression", isToken: false)),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              Optional(element: RuleReference("expression", isToken: false))
            ]))
          ])
        ]),
        lineNumber: 498),
    GrammarRule(
        name: "atom",
        body: Alternation(choices: [
          RuleReference("INT", isToken: true),
          RuleReference("FLOAT", isToken: true),
          RuleReference("IMAG", isToken: true),
          Sequence(elements: [
            RuleReference("STRING", isToken: true),
            Repetition(element: RuleReference("STRING", isToken: true))
          ]),
          RuleReference("fstring", isToken: false),
          RuleReference("NAME", isToken: true),
          Literal("True"),
          Literal("False"),
          Literal("None"),
          RuleReference("ELLIPSIS", isToken: true),
          RuleReference("list_expr", isToken: false),
          RuleReference("dict_expr", isToken: false),
          RuleReference("set_expr", isToken: false),
          RuleReference("paren_expr", isToken: false)
        ]),
        lineNumber: 505),
    GrammarRule(
        name: "fstring",
        body: Sequence(elements: [
          RuleReference("FSTRING_START", isToken: true),
          Repetition(
              element: Alternation(choices: [
            RuleReference("FSTRING_MIDDLE", isToken: true),
            RuleReference("fstring_replacement", isToken: false)
          ])),
          RuleReference("FSTRING_END", isToken: true)
        ]),
        lineNumber: 531),
    GrammarRule(
        name: "fstring_replacement",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          RuleReference("expression", isToken: false),
          Optional(element: RuleReference("EQUALS", isToken: true)),
          Optional(
              element: RuleReference("fstring_conversion", isToken: false)),
          Optional(
              element: RuleReference("fstring_format_spec", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 533),
    GrammarRule(
        name: "fstring_conversion",
        body: Sequence(
            elements: [Literal("!"), RuleReference("NAME", isToken: true)]),
        lineNumber: 537),
    GrammarRule(
        name: "fstring_format_spec",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          Repetition(
              element: Alternation(choices: [
            RuleReference("FSTRING_MIDDLE", isToken: true),
            RuleReference("fstring_replacement", isToken: false)
          ]))
        ]),
        lineNumber: 541),
    GrammarRule(
        name: "list_expr",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(element: RuleReference("list_body", isToken: false)),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 548),
    GrammarRule(
        name: "list_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("expression", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])
        ]),
        lineNumber: 550),
    GrammarRule(
        name: "dict_expr",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(element: RuleReference("dict_body", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 555),
    GrammarRule(
        name: "dict_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("dict_entry", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("dict_entry", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("dict_entry", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])
        ]),
        lineNumber: 557),
    GrammarRule(
        name: "dict_entry",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("expression", isToken: false)
          ])
        ]),
        lineNumber: 560),
    GrammarRule(
        name: "set_expr",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          RuleReference("set_body", isToken: false),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 564),
    GrammarRule(
        name: "set_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("COMMA", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("expression", isToken: false),
              Repetition(
                  element: Sequence(elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("expression", isToken: false)
              ])),
              Optional(element: RuleReference("COMMA", isToken: true))
            ]))
          ])
        ]),
        lineNumber: 566),
    GrammarRule(
        name: "paren_expr",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("paren_body", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 573),
    GrammarRule(
        name: "paren_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("comp_clause", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("expression", isToken: false),
            RuleReference("COMMA", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("expression", isToken: false),
              Repetition(
                  element: Sequence(elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("expression", isToken: false)
              ])),
              Optional(element: RuleReference("COMMA", isToken: true))
            ]))
          ]),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 575),
    GrammarRule(
        name: "comp_clause",
        body: Sequence(elements: [
          RuleReference("comp_for", isToken: false),
          Repetition(
              element: Alternation(choices: [
            RuleReference("comp_for", isToken: false),
            RuleReference("comp_if", isToken: false)
          ]))
        ]),
        lineNumber: 581),
    GrammarRule(
        name: "comp_for",
        body: Sequence(elements: [
          Optional(element: Literal("async")),
          Literal("for"),
          RuleReference("target_list", isToken: false),
          Literal("in"),
          RuleReference("or_expr", isToken: false)
        ]),
        lineNumber: 583),
    GrammarRule(
        name: "comp_if",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("or_expr", isToken: false)
        ]),
        lineNumber: 585),
    GrammarRule(
        name: "yield_expr",
        body: Sequence(elements: [
          Literal("yield"),
          Optional(
              element: Alternation(choices: [
            Sequence(elements: [
              Literal("from"),
              RuleReference("expression", isToken: false)
            ]),
            RuleReference("expression_list", isToken: false)
          ]))
        ]),
        lineNumber: 591),
    GrammarRule(
        name: "arguments",
        body: Sequence(elements: [
          RuleReference("argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("argument", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 600),
    GrammarRule(
        name: "argument",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("DOUBLE_STAR", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 602),
  ],
);

final parserGrammarReleaseg_4e25ec3b5e94 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "file",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("statement", isToken: false)
        ])),
        lineNumber: 41),
    GrammarRule(
        name: "statement",
        body: RuleReference("simple_stmt", isToken: false),
        lineNumber: 42),
    GrammarRule(
        name: "simple_stmt",
        body: Sequence(elements: [
          RuleReference("small_stmt", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("small_stmt", isToken: false)
          ])),
          Optional(element: RuleReference("SEMICOLON", isToken: true)),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 43),
    GrammarRule(
        name: "small_stmt",
        body: Alternation(choices: [
          RuleReference("print_stmt", isToken: false),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 44),
    GrammarRule(
        name: "print_stmt",
        body: Sequence(elements: [
          Literal("print"),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("expression", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ]))
        ]),
        lineNumber: 45),
    GrammarRule(
        name: "expression",
        body: RuleReference("arith", isToken: false),
        lineNumber: 47),
    GrammarRule(
        name: "arith",
        body: Sequence(elements: [
          RuleReference("term", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("term", isToken: false)
          ]))
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "term",
        body: Sequence(elements: [
          RuleReference("factor", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true),
              RuleReference("FLOOR_DIV", isToken: true),
              RuleReference("PERCENT", isToken: true)
            ])),
            RuleReference("factor", isToken: false)
          ]))
        ]),
        lineNumber: 49),
    GrammarRule(
        name: "factor",
        body: Alternation(choices: [
          Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("factor", isToken: false)
          ]),
          RuleReference("primary", isToken: false)
        ]),
        lineNumber: 50),
    GrammarRule(
        name: "primary",
        body: Sequence(elements: [
          RuleReference("atom", isToken: false),
          Repetition(element: RuleReference("suffix", isToken: false))
        ]),
        lineNumber: 51),
    GrammarRule(
        name: "suffix",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("arguments", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 52),
    GrammarRule(
        name: "arguments",
        body: Sequence(elements: [
          RuleReference("argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("argument", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 53),
    GrammarRule(
        name: "argument",
        body: RuleReference("expression", isToken: false),
        lineNumber: 54),
    GrammarRule(
        name: "atom",
        body: Alternation(choices: [
          RuleReference("INT", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("paren_expr", isToken: false)
        ]),
        lineNumber: 55),
    GrammarRule(
        name: "paren_expr",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("paren_body", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 56),
    GrammarRule(
        name: "paren_body",
        body: RuleReference("expression", isToken: false),
        lineNumber: 57),
  ],
);

final parserGrammarReleaseg_5bd2c0aa595b = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "file",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("statement", isToken: false)
        ])),
        lineNumber: 40),
    GrammarRule(
        name: "statement",
        body: RuleReference("simple_stmt", isToken: false),
        lineNumber: 41),
    GrammarRule(
        name: "simple_stmt",
        body: Sequence(elements: [
          RuleReference("small_stmt", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("small_stmt", isToken: false)
          ])),
          Optional(element: RuleReference("SEMICOLON", isToken: true)),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 42),
    GrammarRule(
        name: "small_stmt",
        body: RuleReference("expression", isToken: false),
        lineNumber: 43),
    GrammarRule(
        name: "expression",
        body: RuleReference("arith", isToken: false),
        lineNumber: 45),
    GrammarRule(
        name: "arith",
        body: Sequence(elements: [
          RuleReference("term", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("term", isToken: false)
          ]))
        ]),
        lineNumber: 46),
    GrammarRule(
        name: "term",
        body: Sequence(elements: [
          RuleReference("factor", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true),
              RuleReference("FLOOR_DIV", isToken: true),
              RuleReference("PERCENT", isToken: true)
            ])),
            RuleReference("factor", isToken: false)
          ]))
        ]),
        lineNumber: 47),
    GrammarRule(
        name: "factor",
        body: Alternation(choices: [
          Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("factor", isToken: false)
          ]),
          RuleReference("primary", isToken: false)
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "primary",
        body: Sequence(elements: [
          RuleReference("atom", isToken: false),
          Repetition(element: RuleReference("suffix", isToken: false))
        ]),
        lineNumber: 49),
    GrammarRule(
        name: "suffix",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("arguments", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 50),
    GrammarRule(
        name: "arguments",
        body: Sequence(elements: [
          RuleReference("argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("argument", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 51),
    GrammarRule(
        name: "argument",
        body: RuleReference("expression", isToken: false),
        lineNumber: 52),
    GrammarRule(
        name: "atom",
        body: Alternation(choices: [
          RuleReference("INT", isToken: true),
          RuleReference("FLOAT", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("paren_expr", isToken: false)
        ]),
        lineNumber: 53),
    GrammarRule(
        name: "paren_expr",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("paren_body", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 54),
    GrammarRule(
        name: "paren_body",
        body: RuleReference("expression", isToken: false),
        lineNumber: 55),
  ],
);

final parserGrammarReleaseg_a8aa787740a9 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "file",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("statement", isToken: false)
        ])),
        lineNumber: 38),
    GrammarRule(
        name: "statement",
        body: RuleReference("simple_stmt", isToken: false),
        lineNumber: 39),
    GrammarRule(
        name: "simple_stmt",
        body: Sequence(elements: [
          RuleReference("small_stmt", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("small_stmt", isToken: false)
          ])),
          Optional(element: RuleReference("SEMICOLON", isToken: true)),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 40),
    GrammarRule(
        name: "small_stmt",
        body: RuleReference("expression", isToken: false),
        lineNumber: 41),
    GrammarRule(
        name: "expression",
        body: RuleReference("arith", isToken: false),
        lineNumber: 43),
    GrammarRule(
        name: "arith",
        body: Sequence(elements: [
          RuleReference("term", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("term", isToken: false)
          ]))
        ]),
        lineNumber: 44),
    GrammarRule(
        name: "term",
        body: Sequence(elements: [
          RuleReference("factor", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true),
              RuleReference("FLOOR_DIV", isToken: true),
              RuleReference("PERCENT", isToken: true)
            ])),
            RuleReference("factor", isToken: false)
          ]))
        ]),
        lineNumber: 45),
    GrammarRule(
        name: "factor",
        body: Alternation(choices: [
          Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("factor", isToken: false)
          ]),
          RuleReference("primary", isToken: false)
        ]),
        lineNumber: 46),
    GrammarRule(
        name: "primary",
        body: Sequence(elements: [
          RuleReference("atom", isToken: false),
          Repetition(element: RuleReference("suffix", isToken: false))
        ]),
        lineNumber: 47),
    GrammarRule(
        name: "suffix",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("arguments", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "arguments",
        body: Sequence(elements: [
          RuleReference("argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("argument", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 49),
    GrammarRule(
        name: "argument",
        body: RuleReference("expression", isToken: false),
        lineNumber: 50),
    GrammarRule(
        name: "atom",
        body: Alternation(choices: [
          RuleReference("INT", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("paren_expr", isToken: false)
        ]),
        lineNumber: 51),
    GrammarRule(
        name: "paren_expr",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("paren_body", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 52),
    GrammarRule(
        name: "paren_body",
        body: RuleReference("expression", isToken: false),
        lineNumber: 53),
  ],
);

final _legacyByVersion = <String, ParserGrammar>{
  '2.7': parserGrammarLegacyV2_7,
  '3.0': parserGrammarLegacyV3_0,
  '3.6': parserGrammarLegacyV3_6,
  '3.8': parserGrammarLegacyV3_8,
  '3.10': parserGrammarLegacyV3_10,
  '3.12': parserGrammarLegacyV3_12,
};

final _releaseById = <String, ParserGrammar>{
  '2.2': parserGrammarReleaseg_4e25ec3b5e94,
  '2.4': parserGrammarReleaseg_4e25ec3b5e94,
  '3.0a1': parserGrammarReleaseg_a8aa787740a9,
  '2.6': parserGrammarReleaseg_4e25ec3b5e94,
  '3.6.0': parserGrammarReleaseg_5bd2c0aa595b,
  '2.7.18': parserGrammarReleaseg_4e25ec3b5e94,
  '3.5.10': parserGrammarReleaseg_5bd2c0aa595b,
  '3.12.3': parserGrammarReleaseg_5bd2c0aa595b,
  '3.13.16': parserGrammarReleaseg_5bd2c0aa595b,
  '3.11.17': parserGrammarReleaseg_5bd2c0aa595b,
  '3.16.0a1': parserGrammarReleaseg_5bd2c0aa595b,
};

const _classByRelease = <String, String>{
  '2.2': 'g_4e25ec3b5e94',
  '2.4': 'g_4e25ec3b5e94',
  '3.0a1': 'g_a8aa787740a9',
  '2.6': 'g_4e25ec3b5e94',
  '3.6.0': 'g_5bd2c0aa595b',
  '2.7.18': 'g_4e25ec3b5e94',
  '3.5.10': 'g_5bd2c0aa595b',
  '3.12.3': 'g_5bd2c0aa595b',
  '3.13.16': 'g_5bd2c0aa595b',
  '3.11.17': 'g_5bd2c0aa595b',
  '3.16.0a1': 'g_5bd2c0aa595b',
};

/// Releases with a pair, in inventory (date) order.
const supportedPythonParserReleases = <String>[
  '2.2',
  '2.4',
  '3.0a1',
  '2.6',
  '3.6.0',
  '2.7.18',
  '3.5.10',
  '3.12.3',
  '3.13.16',
  '3.11.17',
  '3.16.0a1',
];

/// The full inventory: every row ID to its grammar stage.
const _stageByRelease = <String, String>{
  '0.9.8': 'none',
  '0.9.9': 'none',
  '1.0.1': 'none',
  '1.0.2': 'none',
  '1.1': 'none',
  '1.1.1': 'none',
  '1.2b1': 'none',
  '1.2b2': 'none',
  '1.2b3': 'none',
  '1.2b4': 'none',
  '1.2': 'none',
  '1.3b1': 'none',
  '1.3': 'none',
  '1.4b1': 'none',
  '1.4b2': 'none',
  '1.4b3': 'none',
  '1.4': 'none',
  '1.5a1': 'none',
  '1.5a2': 'none',
  '1.5a3': 'none',
  '1.5a4': 'none',
  '1.5b1': 'none',
  '1.5b2': 'none',
  '1.5': 'none',
  '1.5.1': 'none',
  '1.5.2a1': 'none',
  '1.5.2a2': 'none',
  '1.5.2b1': 'none',
  '1.5.2b2': 'none',
  '1.5.2c1': 'none',
  '1.5.2': 'none',
  '1.6a1': 'none',
  '1.6a2': 'none',
  '2.0b1': 'none',
  '2.0b2': 'none',
  '2.0c1': 'none',
  '2.0': 'none',
  '2.1a1': 'none',
  '2.1a2': 'none',
  '2.1b1': 'none',
  '2.1b2': 'none',
  '2.1c1': 'none',
  '2.1c2': 'none',
  '2.1': 'none',
  '2.0.1c1': 'none',
  '2.0.1': 'none',
  '2.1.1c1': 'none',
  '2.1.1': 'none',
  '2.2a3': 'none',
  '2.2': 'P',
  '2.1.2c1': 'none',
  '2.1.2': 'none',
  '2.2.1c1': 'none',
  '2.2.1c2': 'none',
  '2.1.3': 'none',
  '2.2.1': 'none',
  '2.2.2b1': 'none',
  '2.2.2': 'none',
  '2.2.3c1': 'none',
  '2.2.3': 'none',
  '2.3c1': 'none',
  '2.3c2': 'none',
  '2.3.1': 'none',
  '2.3.2c1': 'none',
  '2.3.2': 'none',
  '2.3.3c1': 'none',
  '2.3.3': 'none',
  '2.3.4c1': 'none',
  '2.3.4': 'none',
  '2.4a1': 'none',
  '2.4a2': 'none',
  '2.4a3': 'none',
  '2.4b1': 'none',
  '2.4b2': 'none',
  '2.4c1': 'none',
  '2.4': 'P',
  '2.3.5c1': 'none',
  '2.3.5': 'none',
  '2.4.1c1': 'none',
  '2.4.1c2': 'none',
  '2.4.1': 'none',
  '2.4.2c1': 'none',
  '2.4.2': 'none',
  '2.4.3c1': 'none',
  '2.4.3': 'none',
  '2.5a0': 'none',
  '2.5a1': 'none',
  '2.5a2': 'none',
  '2.5b1': 'none',
  '2.5b2': 'none',
  '2.5b3': 'none',
  '2.5c1': 'none',
  '2.5c2': 'none',
  '2.5': 'none',
  '2.4.4c1': 'none',
  '2.4.4': 'none',
  '2.3.6c1': 'none',
  '2.3.6': 'none',
  '2.5.1c1': 'none',
  '2.5.1': 'none',
  '3.0a1': 'P',
  '3.0a2': 'none',
  '2.5.2c1': 'none',
  '2.5.2': 'none',
  '2.6a1': 'none',
  '3.0a3': 'none',
  '2.4.5c1': 'none',
  '2.3.7c1': 'none',
  '2.4.5': 'none',
  '2.3.7': 'none',
  '3.0a4': 'none',
  '2.6a2': 'none',
  '3.0a5': 'none',
  '2.6a3': 'none',
  '3.0b1': 'none',
  '2.6b1': 'none',
  '3.0b2': 'none',
  '2.6b2': 'none',
  '2.6b3': 'none',
  '3.0b3': 'none',
  '2.6rc1': 'none',
  '3.0rc1': 'none',
  '2.6rc2': 'none',
  '2.6': 'P',
  '3.0rc2': 'none',
  '3.0rc3': 'none',
  '3.0': 'none',
  '2.6.1': 'none',
  '2.4.6c1': 'none',
  '2.5.3c1': 'none',
  '2.4.6': 'none',
  '2.5.3': 'none',
  '2.5.4': 'none',
  '3.0.1': 'none',
  '3.1a1': 'none',
  '3.1a2': 'none',
  '2.6.2c1': 'none',
  '2.6.2': 'none',
  '3.1b1': 'none',
  '3.1rc1': 'none',
  '3.1rc2': 'none',
  '3.1': 'none',
  '3.1.1rc1': 'none',
  '3.1.1': 'none',
  '2.6.3rc1': 'none',
  '2.6.3': 'none',
  '2.6.4rc1': 'none',
  '2.6.4rc2': 'none',
  '2.6.4': 'none',
  '2.7a1': 'none',
  '2.7a2': 'none',
  '2.5.5c1': 'none',
  '2.5.5c2': 'none',
  '2.5.5': 'none',
  '2.7a3': 'none',
  '2.6.5rc1': 'none',
  '2.7a4': 'none',
  '3.1.2rc1': 'none',
  '2.6.5rc2': 'none',
  '2.6.5': 'none',
  '3.1.2': 'none',
  '2.7b1': 'none',
  '2.7b2': 'none',
  '2.7rc1': 'none',
  '2.7rc2': 'none',
  '2.7': 'none',
  '3.2a1': 'none',
  '2.6.6rc1': 'none',
  '2.6.6rc2': 'none',
  '2.6.6': 'none',
  '3.2a2': 'none',
  '3.2a3': 'none',
  '3.2a4': 'none',
  '3.1.3rc1': 'none',
  '2.7.1rc1': 'none',
  '2.7.1': 'none',
  '3.1.3': 'none',
  '3.2b1': 'none',
  '3.2b2': 'none',
  '3.2rc1': 'none',
  '3.2rc2': 'none',
  '3.2rc3': 'none',
  '3.2': 'none',
  '2.5.6c1': 'none',
  '3.2.1b1': 'none',
  '3.2.1rc1': 'none',
  '2.5.6': 'none',
  '3.1.4rc1': 'none',
  '2.7.2rc1': 'none',
  '2.6.7': 'none',
  '2.7.2': 'none',
  '3.1.4': 'none',
  '3.2.1rc2': 'none',
  '3.2.1': 'none',
  '3.2.2rc1': 'none',
  '3.2.2': 'none',
  '3.1.5rc1': 'none',
  '2.7.3rc1': 'none',
  '2.6.8rc1': 'none',
  '3.2.3rc1': 'none',
  '3.3.0a1': 'none',
  '3.1.5rc2': 'none',
  '2.7.3rc2': 'none',
  '2.6.8rc2': 'none',
  '3.2.3rc2': 'none',
  '3.3.0a2': 'none',
  '3.1.5': 'none',
  '2.7.3': 'none',
  '2.6.8': 'none',
  '3.2.3': 'none',
  '3.3.0a3': 'none',
  '3.3.0a4': 'none',
  '3.3.0b1': 'none',
  '3.3.0b2': 'none',
  '3.3.0rc1': 'none',
  '3.3.0rc2': 'none',
  '3.3.0rc3': 'none',
  '3.3.0': 'none',
  '2.7.4rc1': 'none',
  '3.2.4rc1': 'none',
  '3.3.1rc1': 'none',
  '2.7.4': 'none',
  '3.2.4': 'none',
  '3.3.1': 'none',
  '2.7.5': 'none',
  '3.2.5': 'none',
  '3.3.2': 'none',
  '3.4.0a1': 'none',
  '3.4.0a2': 'none',
  '3.4.0a3': 'none',
  '2.6.9rc1': 'none',
  '3.4.0a4': 'none',
  '2.7.6rc1': 'none',
  '3.3.3rc1': 'none',
  '2.6.9': 'none',
  '2.7.6': 'none',
  '3.3.3rc2': 'none',
  '3.3.3': 'none',
  '3.4.0b1': 'none',
  '3.4.0b2': 'none',
  '3.4.0b3': 'none',
  '3.3.4rc1': 'none',
  '3.3.4': 'none',
  '3.4.0rc1': 'none',
  '3.3.5rc1': 'none',
  '3.4.0rc2': 'none',
  '3.3.5rc2': 'none',
  '3.3.5': 'none',
  '3.4.0rc3': 'none',
  '3.4.0': 'none',
  '3.4.1rc1': 'none',
  '2.7.7rc1': 'none',
  '3.4.1': 'none',
  '2.7.7': 'none',
  '2.7.8': 'none',
  '3.4.2rc1': 'none',
  '3.3.6rc1': 'none',
  '3.2.6rc1': 'none',
  '3.4.2': 'none',
  '3.2.6': 'none',
  '3.3.6': 'none',
  '2.7.9rc1': 'none',
  '2.7.9': 'none',
  '3.4.3rc1': 'none',
  '3.5.0a1': 'none',
  '3.4.3': 'none',
  '3.5.0a2': 'none',
  '3.5.0a3': 'none',
  '3.5.0a4': 'none',
  '2.7.10rc1': 'none',
  '2.7.10': 'none',
  '3.5.0b1': 'none',
  '3.5.0b2': 'none',
  '3.5.0b3': 'none',
  '3.5.0b4': 'none',
  '3.5.0rc1': 'none',
  '3.5.0rc2': 'none',
  '3.5.0rc3': 'none',
  '3.5.0rc4': 'none',
  '3.5.0': 'none',
  '2.7.11rc1': 'none',
  '3.5.1rc1': 'none',
  '2.7.11': 'none',
  '3.5.1': 'none',
  '3.4.4rc1': 'none',
  '3.4.4': 'none',
  '3.6.0a1': 'none',
  '3.4.5rc1': 'none',
  '2.7.12rc1': 'none',
  '3.5.2rc1': 'none',
  '3.6.0a2': 'none',
  '2.7.12': 'none',
  '3.5.2': 'none',
  '3.4.5': 'none',
  '3.6.0a3': 'none',
  '3.6.0a4': 'none',
  '3.6.0b1': 'none',
  '3.6.0b2': 'none',
  '3.6.0b3': 'none',
  '3.6.0b4': 'none',
  '2.7.13rc1': 'none',
  '3.6.0rc1': 'none',
  '3.6.0rc2': 'none',
  '2.7.13': 'none',
  '3.6.0': 'P',
  '3.4.6rc1': 'none',
  '3.5.3rc1': 'none',
  '3.5.3': 'none',
  '3.4.6': 'none',
  '3.6.1rc1': 'none',
  '3.6.1': 'none',
  '3.6.2rc1': 'none',
  '3.6.2rc2': 'none',
  '3.6.2': 'none',
  '3.4.7rc1': 'none',
  '3.5.4rc1': 'none',
  '3.5.4': 'none',
  '3.4.7': 'none',
  '2.7.14rc1': 'none',
  '3.3.7rc1': 'none',
  '2.7.14': 'none',
  '3.6.3rc1': 'none',
  '3.7.0a1': 'none',
  '3.3.7': 'none',
  '3.6.3': 'none',
  '3.7.0a2': 'none',
  '3.6.4rc1': 'none',
  '3.7.0a3': 'none',
  '3.6.4': 'none',
  '3.7.0a4': 'none',
  '3.4.8rc1': 'none',
  '3.5.5rc1': 'none',
  '3.7.0b1': 'none',
  '3.4.8': 'none',
  '3.5.5': 'none',
  '3.7.0b2': 'none',
  '3.6.5rc1': 'none',
  '3.6.5': 'none',
  '3.7.0b3': 'none',
  '2.7.15rc1': 'none',
  '2.7.15': 'none',
  '3.7.0b4': 'none',
  '3.7.0b5': 'none',
  '3.6.6rc1': 'none',
  '3.7.0rc1': 'none',
  '3.6.6': 'none',
  '3.7.0': 'none',
  '3.4.9rc1': 'none',
  '3.5.6rc1': 'none',
  '3.4.9': 'none',
  '3.5.6': 'none',
  '3.7.1rc1': 'none',
  '3.6.7rc1': 'none',
  '3.7.1rc2': 'none',
  '3.6.7rc2': 'none',
  '3.6.7': 'none',
  '3.7.1': 'none',
  '3.7.2rc1': 'none',
  '3.6.8rc1': 'none',
  '3.6.8': 'none',
  '3.7.2': 'none',
  '3.8.0a1': 'none',
  '2.7.16rc1': 'none',
  '3.8.0a2': 'none',
  '2.7.16': 'none',
  '3.4.10rc1': 'none',
  '3.5.7rc1': 'none',
  '3.7.3rc1': 'none',
  '3.5.7': 'none',
  '3.4.10': 'none',
  '3.8.0a3': 'none',
  '3.7.3': 'none',
  '3.8.0a4': 'none',
  '3.8.0b1': 'none',
  '3.7.4rc1': 'none',
  '3.6.9rc1': 'none',
  '3.7.4rc2': 'none',
  '3.6.9': 'none',
  '3.8.0b2': 'none',
  '3.7.4': 'none',
  '3.8.0b3': 'none',
  '3.8.0b4': 'none',
  '3.5.8rc1': 'none',
  '3.8.0rc1': 'none',
  '3.7.5rc1': 'none',
  '2.7.17rc1': 'none',
  '3.5.8rc2': 'none',
  '3.8.0': 'none',
  '3.7.5': 'none',
  '2.7.17': 'none',
  '3.5.8': 'none',
  '3.5.9': 'none',
  '3.9.0a1': 'none',
  '3.8.1rc1': 'none',
  '3.7.6rc1': 'none',
  '3.6.10rc1': 'none',
  '3.8.1': 'none',
  '3.7.6': 'none',
  '3.6.10': 'none',
  '3.9.0a2': 'none',
  '3.9.0a3': 'none',
  '3.8.2rc1': 'none',
  '3.8.2rc2': 'none',
  '3.8.2': 'none',
  '3.9.0a4': 'none',
  '3.7.7rc1': 'none',
  '3.7.7': 'none',
  '3.9.0a5': 'none',
  '2.7.18rc1': 'none',
  '2.7.18': 'P',
  '3.9.0a6': 'none',
  '3.8.3rc1': 'none',
  '3.8.3': 'none',
  '3.9.0b1': 'none',
  '3.9.0b2': 'none',
  '3.9.0b3': 'none',
  '3.7.8rc1': 'none',
  '3.6.11rc1': 'none',
  '3.7.8': 'none',
  '3.6.11': 'none',
  '3.8.4rc1': 'none',
  '3.9.0b4': 'none',
  '3.8.4': 'none',
  '3.8.5': 'none',
  '3.9.0b5': 'none',
  '3.9.0rc1': 'none',
  '3.7.9': 'none',
  '3.6.12': 'none',
  '3.5.10rc1': 'none',
  '3.5.10': 'P',
  '3.8.6rc1': 'none',
  '3.9.0rc2': 'none',
  '3.8.6': 'none',
  '3.9.0': 'none',
  '3.10.0a1': 'none',
  '3.10.0a2': 'none',
  '3.9.1rc1': 'none',
  '3.9.1': 'none',
  '3.8.7rc1': 'none',
  '3.10.0a3': 'none',
  '3.8.7': 'none',
  '3.10.0a4': 'none',
  '3.10.0a5': 'none',
  '3.7.10': 'none',
  '3.6.13': 'none',
  '3.8.8rc1': 'none',
  '3.9.2rc1': 'none',
  '3.8.8': 'none',
  '3.9.2': 'none',
  '3.10.0a6': 'none',
  '3.9.3': 'none',
  '3.8.9': 'none',
  '3.9.4': 'none',
  '3.10.0a7': 'none',
  '3.8.10': 'none',
  '3.9.5': 'none',
  '3.10.0b1': 'none',
  '3.10.0b2': 'none',
  '3.10.0b3': 'none',
  '3.9.6': 'none',
  '3.8.11': 'none',
  '3.7.11': 'none',
  '3.6.14': 'none',
  '3.10.0b4': 'none',
  '3.10.0rc1': 'none',
  '3.8.12': 'none',
  '3.9.7': 'none',
  '3.7.12': 'none',
  '3.6.15': 'none',
  '3.10.0rc2': 'none',
  '3.10.0': 'none',
  '3.11.0a1': 'none',
  '3.11.0a2': 'none',
  '3.9.8': 'none',
  '3.9.9': 'none',
  '3.10.1': 'none',
  '3.11.0a3': 'none',
  '3.10.2': 'none',
  '3.11.0a4': 'none',
  '3.9.10': 'none',
  '3.11.0a5': 'none',
  '3.11.0a6': 'none',
  '3.8.13': 'none',
  '3.10.3': 'none',
  '3.9.11': 'none',
  '3.7.13': 'none',
  '3.10.4': 'none',
  '3.9.12': 'none',
  '3.11.0a7': 'none',
  '3.11.0b1': 'none',
  '3.9.13': 'none',
  '3.11.0b2': 'none',
  '3.11.0b3': 'none',
  '3.10.5': 'none',
  '3.11.0b4': 'none',
  '3.11.0b5': 'none',
  '3.10.6': 'none',
  '3.11.0rc1': 'none',
  '3.10.7': 'none',
  '3.7.14': 'none',
  '3.9.14': 'none',
  '3.8.14': 'none',
  '3.11.0rc2': 'none',
  '3.7.15': 'none',
  '3.10.8': 'none',
  '3.9.15': 'none',
  '3.8.15': 'none',
  '3.11.0': 'none',
  '3.12.0a1': 'none',
  '3.12.0a2': 'none',
  '3.9.16': 'none',
  '3.10.9': 'none',
  '3.12.0a3': 'none',
  '3.7.16': 'none',
  '3.8.16': 'none',
  '3.11.1': 'none',
  '3.12.0a4': 'none',
  '3.10.10': 'none',
  '3.12.0a5': 'none',
  '3.11.2': 'none',
  '3.12.0a6': 'none',
  '3.12.0a7': 'none',
  '3.10.11': 'none',
  '3.11.3': 'none',
  '3.12.0b1': 'none',
  '3.7.17': 'none',
  '3.9.17': 'none',
  '3.8.17': 'none',
  '3.12.0b2': 'none',
  '3.11.4': 'none',
  '3.10.12': 'none',
  '3.12.0b3': 'none',
  '3.12.0b4': 'none',
  '3.12.0rc1': 'none',
  '3.11.5': 'none',
  '3.10.13': 'none',
  '3.9.18': 'none',
  '3.8.18': 'none',
  '3.12.0rc2': 'none',
  '3.12.0rc3': 'none',
  '3.12.0': 'none',
  '3.11.6': 'none',
  '3.13.0a1': 'none',
  '3.13.0a2': 'none',
  '3.11.7': 'none',
  '3.12.1': 'none',
  '3.13.0a3': 'none',
  '3.12.2': 'none',
  '3.11.8': 'none',
  '3.13.0a4': 'none',
  '3.13.0a5': 'none',
  '3.8.19': 'none',
  '3.9.19': 'none',
  '3.10.14': 'none',
  '3.11.9': 'none',
  '3.12.3': 'P',
  '3.13.0a6': 'none',
  '3.13.0b1': 'none',
  '3.13.0b2': 'none',
  '3.12.4': 'none',
  '3.13.0b3': 'none',
  '3.13.0b4': 'none',
  '3.13.0rc1': 'none',
  '3.12.5': 'none',
  '3.12.6': 'none',
  '3.9.20': 'none',
  '3.8.20': 'none',
  '3.13.0rc2': 'none',
  '3.10.15': 'none',
  '3.11.10': 'none',
  '3.13.0rc3': 'none',
  '3.12.7': 'none',
  '3.13.0': 'none',
  '3.14.0a1': 'none',
  '3.14.0a2': 'none',
  '3.11.11': 'none',
  '3.10.16': 'none',
  '3.9.21': 'none',
  '3.13.1': 'none',
  '3.12.8': 'none',
  '3.14.0a3': 'none',
  '3.14.0a4': 'none',
  '3.12.9': 'none',
  '3.13.2': 'none',
  '3.14.0a5': 'none',
  '3.14.0a6': 'none',
  '3.14.0a7': 'none',
  '3.12.10': 'none',
  '3.10.17': 'none',
  '3.13.3': 'none',
  '3.11.12': 'none',
  '3.9.22': 'none',
  '3.14.0b1': 'none',
  '3.14.0b2': 'none',
  '3.13.4': 'none',
  '3.12.11': 'none',
  '3.10.18': 'none',
  '3.11.13': 'none',
  '3.9.23': 'none',
  '3.13.5': 'none',
  '3.14.0b3': 'none',
  '3.14.0b4': 'none',
  '3.14.0rc1': 'none',
  '3.13.6': 'none',
  '3.13.7': 'none',
  '3.14.0rc2': 'none',
  '3.14.0rc3': 'none',
  '3.14.0': 'none',
  '3.13.8': 'none',
  '3.12.12': 'none',
  '3.9.24': 'none',
  '3.10.19': 'none',
  '3.11.14': 'none',
  '3.15.0a1': 'none',
  '3.13.9': 'none',
  '3.9.25': 'none',
  '3.15.0a2': 'none',
  '3.13.10': 'none',
  '3.14.1': 'none',
  '3.13.11': 'none',
  '3.14.2': 'none',
  '3.15.0a3': 'none',
  '3.15.0a4': 'none',
  '3.15.0a5': 'none',
  '3.14.3': 'none',
  '3.13.12': 'none',
  '3.15.0a6': 'none',
  '3.10.20': 'none',
  '3.11.15': 'none',
  '3.12.13': 'none',
  '3.15.0a7': 'none',
  '3.15.0a8': 'none',
  '3.14.4': 'none',
  '3.13.13': 'none',
  '3.14.5rc1': 'none',
  '3.15.0b1': 'none',
  '3.14.5': 'none',
  '3.15.0b2': 'none',
  '3.14.6': 'none',
  '3.13.14': 'none',
  '3.15.0b3': 'none',
  '3.15.0b4': 'none',
  '3.15.0rc1': 'none',
  '3.14.7': 'none',
  '3.13.15': 'none',
  '3.12.14': 'none',
  '3.10.21': 'none',
  '3.11.16': 'none',
  '3.15.0rc2': 'none',
  '3.14.8': 'none',
  '3.13.16': 'P',
  '3.12.15': 'none',
  '3.10.22': 'none',
  '3.11.17': 'P',
  '3.15.0rc3': 'none',
  '3.15.0': 'none',
  '3.16.0a1': 'P',
};

/// Every inventory ID in file order (for suggestions).
const pythonInventoryIds = <String>[
  '0.9.8',
  '0.9.9',
  '1.0.1',
  '1.0.2',
  '1.1',
  '1.1.1',
  '1.2b1',
  '1.2b2',
  '1.2b3',
  '1.2b4',
  '1.2',
  '1.3b1',
  '1.3',
  '1.4b1',
  '1.4b2',
  '1.4b3',
  '1.4',
  '1.5a1',
  '1.5a2',
  '1.5a3',
  '1.5a4',
  '1.5b1',
  '1.5b2',
  '1.5',
  '1.5.1',
  '1.5.2a1',
  '1.5.2a2',
  '1.5.2b1',
  '1.5.2b2',
  '1.5.2c1',
  '1.5.2',
  '1.6a1',
  '1.6a2',
  '2.0b1',
  '2.0b2',
  '2.0c1',
  '2.0',
  '2.1a1',
  '2.1a2',
  '2.1b1',
  '2.1b2',
  '2.1c1',
  '2.1c2',
  '2.1',
  '2.0.1c1',
  '2.0.1',
  '2.1.1c1',
  '2.1.1',
  '2.2a3',
  '2.2',
  '2.1.2c1',
  '2.1.2',
  '2.2.1c1',
  '2.2.1c2',
  '2.1.3',
  '2.2.1',
  '2.2.2b1',
  '2.2.2',
  '2.2.3c1',
  '2.2.3',
  '2.3c1',
  '2.3c2',
  '2.3.1',
  '2.3.2c1',
  '2.3.2',
  '2.3.3c1',
  '2.3.3',
  '2.3.4c1',
  '2.3.4',
  '2.4a1',
  '2.4a2',
  '2.4a3',
  '2.4b1',
  '2.4b2',
  '2.4c1',
  '2.4',
  '2.3.5c1',
  '2.3.5',
  '2.4.1c1',
  '2.4.1c2',
  '2.4.1',
  '2.4.2c1',
  '2.4.2',
  '2.4.3c1',
  '2.4.3',
  '2.5a0',
  '2.5a1',
  '2.5a2',
  '2.5b1',
  '2.5b2',
  '2.5b3',
  '2.5c1',
  '2.5c2',
  '2.5',
  '2.4.4c1',
  '2.4.4',
  '2.3.6c1',
  '2.3.6',
  '2.5.1c1',
  '2.5.1',
  '3.0a1',
  '3.0a2',
  '2.5.2c1',
  '2.5.2',
  '2.6a1',
  '3.0a3',
  '2.4.5c1',
  '2.3.7c1',
  '2.4.5',
  '2.3.7',
  '3.0a4',
  '2.6a2',
  '3.0a5',
  '2.6a3',
  '3.0b1',
  '2.6b1',
  '3.0b2',
  '2.6b2',
  '2.6b3',
  '3.0b3',
  '2.6rc1',
  '3.0rc1',
  '2.6rc2',
  '2.6',
  '3.0rc2',
  '3.0rc3',
  '3.0',
  '2.6.1',
  '2.4.6c1',
  '2.5.3c1',
  '2.4.6',
  '2.5.3',
  '2.5.4',
  '3.0.1',
  '3.1a1',
  '3.1a2',
  '2.6.2c1',
  '2.6.2',
  '3.1b1',
  '3.1rc1',
  '3.1rc2',
  '3.1',
  '3.1.1rc1',
  '3.1.1',
  '2.6.3rc1',
  '2.6.3',
  '2.6.4rc1',
  '2.6.4rc2',
  '2.6.4',
  '2.7a1',
  '2.7a2',
  '2.5.5c1',
  '2.5.5c2',
  '2.5.5',
  '2.7a3',
  '2.6.5rc1',
  '2.7a4',
  '3.1.2rc1',
  '2.6.5rc2',
  '2.6.5',
  '3.1.2',
  '2.7b1',
  '2.7b2',
  '2.7rc1',
  '2.7rc2',
  '2.7',
  '3.2a1',
  '2.6.6rc1',
  '2.6.6rc2',
  '2.6.6',
  '3.2a2',
  '3.2a3',
  '3.2a4',
  '3.1.3rc1',
  '2.7.1rc1',
  '2.7.1',
  '3.1.3',
  '3.2b1',
  '3.2b2',
  '3.2rc1',
  '3.2rc2',
  '3.2rc3',
  '3.2',
  '2.5.6c1',
  '3.2.1b1',
  '3.2.1rc1',
  '2.5.6',
  '3.1.4rc1',
  '2.7.2rc1',
  '2.6.7',
  '2.7.2',
  '3.1.4',
  '3.2.1rc2',
  '3.2.1',
  '3.2.2rc1',
  '3.2.2',
  '3.1.5rc1',
  '2.7.3rc1',
  '2.6.8rc1',
  '3.2.3rc1',
  '3.3.0a1',
  '3.1.5rc2',
  '2.7.3rc2',
  '2.6.8rc2',
  '3.2.3rc2',
  '3.3.0a2',
  '3.1.5',
  '2.7.3',
  '2.6.8',
  '3.2.3',
  '3.3.0a3',
  '3.3.0a4',
  '3.3.0b1',
  '3.3.0b2',
  '3.3.0rc1',
  '3.3.0rc2',
  '3.3.0rc3',
  '3.3.0',
  '2.7.4rc1',
  '3.2.4rc1',
  '3.3.1rc1',
  '2.7.4',
  '3.2.4',
  '3.3.1',
  '2.7.5',
  '3.2.5',
  '3.3.2',
  '3.4.0a1',
  '3.4.0a2',
  '3.4.0a3',
  '2.6.9rc1',
  '3.4.0a4',
  '2.7.6rc1',
  '3.3.3rc1',
  '2.6.9',
  '2.7.6',
  '3.3.3rc2',
  '3.3.3',
  '3.4.0b1',
  '3.4.0b2',
  '3.4.0b3',
  '3.3.4rc1',
  '3.3.4',
  '3.4.0rc1',
  '3.3.5rc1',
  '3.4.0rc2',
  '3.3.5rc2',
  '3.3.5',
  '3.4.0rc3',
  '3.4.0',
  '3.4.1rc1',
  '2.7.7rc1',
  '3.4.1',
  '2.7.7',
  '2.7.8',
  '3.4.2rc1',
  '3.3.6rc1',
  '3.2.6rc1',
  '3.4.2',
  '3.2.6',
  '3.3.6',
  '2.7.9rc1',
  '2.7.9',
  '3.4.3rc1',
  '3.5.0a1',
  '3.4.3',
  '3.5.0a2',
  '3.5.0a3',
  '3.5.0a4',
  '2.7.10rc1',
  '2.7.10',
  '3.5.0b1',
  '3.5.0b2',
  '3.5.0b3',
  '3.5.0b4',
  '3.5.0rc1',
  '3.5.0rc2',
  '3.5.0rc3',
  '3.5.0rc4',
  '3.5.0',
  '2.7.11rc1',
  '3.5.1rc1',
  '2.7.11',
  '3.5.1',
  '3.4.4rc1',
  '3.4.4',
  '3.6.0a1',
  '3.4.5rc1',
  '2.7.12rc1',
  '3.5.2rc1',
  '3.6.0a2',
  '2.7.12',
  '3.5.2',
  '3.4.5',
  '3.6.0a3',
  '3.6.0a4',
  '3.6.0b1',
  '3.6.0b2',
  '3.6.0b3',
  '3.6.0b4',
  '2.7.13rc1',
  '3.6.0rc1',
  '3.6.0rc2',
  '2.7.13',
  '3.6.0',
  '3.4.6rc1',
  '3.5.3rc1',
  '3.5.3',
  '3.4.6',
  '3.6.1rc1',
  '3.6.1',
  '3.6.2rc1',
  '3.6.2rc2',
  '3.6.2',
  '3.4.7rc1',
  '3.5.4rc1',
  '3.5.4',
  '3.4.7',
  '2.7.14rc1',
  '3.3.7rc1',
  '2.7.14',
  '3.6.3rc1',
  '3.7.0a1',
  '3.3.7',
  '3.6.3',
  '3.7.0a2',
  '3.6.4rc1',
  '3.7.0a3',
  '3.6.4',
  '3.7.0a4',
  '3.4.8rc1',
  '3.5.5rc1',
  '3.7.0b1',
  '3.4.8',
  '3.5.5',
  '3.7.0b2',
  '3.6.5rc1',
  '3.6.5',
  '3.7.0b3',
  '2.7.15rc1',
  '2.7.15',
  '3.7.0b4',
  '3.7.0b5',
  '3.6.6rc1',
  '3.7.0rc1',
  '3.6.6',
  '3.7.0',
  '3.4.9rc1',
  '3.5.6rc1',
  '3.4.9',
  '3.5.6',
  '3.7.1rc1',
  '3.6.7rc1',
  '3.7.1rc2',
  '3.6.7rc2',
  '3.6.7',
  '3.7.1',
  '3.7.2rc1',
  '3.6.8rc1',
  '3.6.8',
  '3.7.2',
  '3.8.0a1',
  '2.7.16rc1',
  '3.8.0a2',
  '2.7.16',
  '3.4.10rc1',
  '3.5.7rc1',
  '3.7.3rc1',
  '3.5.7',
  '3.4.10',
  '3.8.0a3',
  '3.7.3',
  '3.8.0a4',
  '3.8.0b1',
  '3.7.4rc1',
  '3.6.9rc1',
  '3.7.4rc2',
  '3.6.9',
  '3.8.0b2',
  '3.7.4',
  '3.8.0b3',
  '3.8.0b4',
  '3.5.8rc1',
  '3.8.0rc1',
  '3.7.5rc1',
  '2.7.17rc1',
  '3.5.8rc2',
  '3.8.0',
  '3.7.5',
  '2.7.17',
  '3.5.8',
  '3.5.9',
  '3.9.0a1',
  '3.8.1rc1',
  '3.7.6rc1',
  '3.6.10rc1',
  '3.8.1',
  '3.7.6',
  '3.6.10',
  '3.9.0a2',
  '3.9.0a3',
  '3.8.2rc1',
  '3.8.2rc2',
  '3.8.2',
  '3.9.0a4',
  '3.7.7rc1',
  '3.7.7',
  '3.9.0a5',
  '2.7.18rc1',
  '2.7.18',
  '3.9.0a6',
  '3.8.3rc1',
  '3.8.3',
  '3.9.0b1',
  '3.9.0b2',
  '3.9.0b3',
  '3.7.8rc1',
  '3.6.11rc1',
  '3.7.8',
  '3.6.11',
  '3.8.4rc1',
  '3.9.0b4',
  '3.8.4',
  '3.8.5',
  '3.9.0b5',
  '3.9.0rc1',
  '3.7.9',
  '3.6.12',
  '3.5.10rc1',
  '3.5.10',
  '3.8.6rc1',
  '3.9.0rc2',
  '3.8.6',
  '3.9.0',
  '3.10.0a1',
  '3.10.0a2',
  '3.9.1rc1',
  '3.9.1',
  '3.8.7rc1',
  '3.10.0a3',
  '3.8.7',
  '3.10.0a4',
  '3.10.0a5',
  '3.7.10',
  '3.6.13',
  '3.8.8rc1',
  '3.9.2rc1',
  '3.8.8',
  '3.9.2',
  '3.10.0a6',
  '3.9.3',
  '3.8.9',
  '3.9.4',
  '3.10.0a7',
  '3.8.10',
  '3.9.5',
  '3.10.0b1',
  '3.10.0b2',
  '3.10.0b3',
  '3.9.6',
  '3.8.11',
  '3.7.11',
  '3.6.14',
  '3.10.0b4',
  '3.10.0rc1',
  '3.8.12',
  '3.9.7',
  '3.7.12',
  '3.6.15',
  '3.10.0rc2',
  '3.10.0',
  '3.11.0a1',
  '3.11.0a2',
  '3.9.8',
  '3.9.9',
  '3.10.1',
  '3.11.0a3',
  '3.10.2',
  '3.11.0a4',
  '3.9.10',
  '3.11.0a5',
  '3.11.0a6',
  '3.8.13',
  '3.10.3',
  '3.9.11',
  '3.7.13',
  '3.10.4',
  '3.9.12',
  '3.11.0a7',
  '3.11.0b1',
  '3.9.13',
  '3.11.0b2',
  '3.11.0b3',
  '3.10.5',
  '3.11.0b4',
  '3.11.0b5',
  '3.10.6',
  '3.11.0rc1',
  '3.10.7',
  '3.7.14',
  '3.9.14',
  '3.8.14',
  '3.11.0rc2',
  '3.7.15',
  '3.10.8',
  '3.9.15',
  '3.8.15',
  '3.11.0',
  '3.12.0a1',
  '3.12.0a2',
  '3.9.16',
  '3.10.9',
  '3.12.0a3',
  '3.7.16',
  '3.8.16',
  '3.11.1',
  '3.12.0a4',
  '3.10.10',
  '3.12.0a5',
  '3.11.2',
  '3.12.0a6',
  '3.12.0a7',
  '3.10.11',
  '3.11.3',
  '3.12.0b1',
  '3.7.17',
  '3.9.17',
  '3.8.17',
  '3.12.0b2',
  '3.11.4',
  '3.10.12',
  '3.12.0b3',
  '3.12.0b4',
  '3.12.0rc1',
  '3.11.5',
  '3.10.13',
  '3.9.18',
  '3.8.18',
  '3.12.0rc2',
  '3.12.0rc3',
  '3.12.0',
  '3.11.6',
  '3.13.0a1',
  '3.13.0a2',
  '3.11.7',
  '3.12.1',
  '3.13.0a3',
  '3.12.2',
  '3.11.8',
  '3.13.0a4',
  '3.13.0a5',
  '3.8.19',
  '3.9.19',
  '3.10.14',
  '3.11.9',
  '3.12.3',
  '3.13.0a6',
  '3.13.0b1',
  '3.13.0b2',
  '3.12.4',
  '3.13.0b3',
  '3.13.0b4',
  '3.13.0rc1',
  '3.12.5',
  '3.12.6',
  '3.9.20',
  '3.8.20',
  '3.13.0rc2',
  '3.10.15',
  '3.11.10',
  '3.13.0rc3',
  '3.12.7',
  '3.13.0',
  '3.14.0a1',
  '3.14.0a2',
  '3.11.11',
  '3.10.16',
  '3.9.21',
  '3.13.1',
  '3.12.8',
  '3.14.0a3',
  '3.14.0a4',
  '3.12.9',
  '3.13.2',
  '3.14.0a5',
  '3.14.0a6',
  '3.14.0a7',
  '3.12.10',
  '3.10.17',
  '3.13.3',
  '3.11.12',
  '3.9.22',
  '3.14.0b1',
  '3.14.0b2',
  '3.13.4',
  '3.12.11',
  '3.10.18',
  '3.11.13',
  '3.9.23',
  '3.13.5',
  '3.14.0b3',
  '3.14.0b4',
  '3.14.0rc1',
  '3.13.6',
  '3.13.7',
  '3.14.0rc2',
  '3.14.0rc3',
  '3.14.0',
  '3.13.8',
  '3.12.12',
  '3.9.24',
  '3.10.19',
  '3.11.14',
  '3.15.0a1',
  '3.13.9',
  '3.9.25',
  '3.15.0a2',
  '3.13.10',
  '3.14.1',
  '3.13.11',
  '3.14.2',
  '3.15.0a3',
  '3.15.0a4',
  '3.15.0a5',
  '3.14.3',
  '3.13.12',
  '3.15.0a6',
  '3.10.20',
  '3.11.15',
  '3.12.13',
  '3.15.0a7',
  '3.15.0a8',
  '3.14.4',
  '3.13.13',
  '3.14.5rc1',
  '3.15.0b1',
  '3.14.5',
  '3.15.0b2',
  '3.14.6',
  '3.13.14',
  '3.15.0b3',
  '3.15.0b4',
  '3.15.0rc1',
  '3.14.7',
  '3.13.15',
  '3.12.14',
  '3.10.21',
  '3.11.16',
  '3.15.0rc2',
  '3.14.8',
  '3.13.16',
  '3.12.15',
  '3.10.22',
  '3.11.17',
  '3.15.0rc3',
  '3.15.0',
  '3.16.0a1',
];

/// The grammar stage of an inventory row, or `null` when the
/// ID names no row.
String? pythonReleaseStage(String release) => _stageByRelease[release];

/// Returns the compiled parser grammar for a resolved legacy
/// version. Callers must resolve through
/// [resolvePythonParserVersion] first.
ParserGrammar legacyParserGrammar(String version) => _legacyByVersion[version]!;

/// The compiled parser grammar of exactly one paired release,
/// or `null` when the release has no pair.
ParserGrammar? releaseParserGrammar(String release) => _releaseById[release];

/// The deduplicated parser-table class of exactly [release],
/// or `null` for a release without one.
String? pythonParserGrammarClass(String release) => _classByRelease[release];
