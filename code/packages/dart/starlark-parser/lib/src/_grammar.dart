// AUTO-GENERATED FILE - DO NOT EDIT
// Source: starlark.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "file",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("statement", isToken: false)
        ])),
        lineNumber: 48),
    GrammarRule(
        name: "statement",
        body: Alternation(choices: [
          RuleReference("compound_stmt", isToken: false),
          RuleReference("simple_stmt", isToken: false)
        ]),
        lineNumber: 62),
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
        lineNumber: 66),
    GrammarRule(
        name: "small_stmt",
        body: Alternation(choices: [
          RuleReference("return_stmt", isToken: false),
          RuleReference("break_stmt", isToken: false),
          RuleReference("continue_stmt", isToken: false),
          RuleReference("pass_stmt", isToken: false),
          RuleReference("load_stmt", isToken: false),
          RuleReference("assign_stmt", isToken: false)
        ]),
        lineNumber: 68),
    GrammarRule(
        name: "return_stmt",
        body: Sequence(elements: [
          Literal("return"),
          Optional(element: RuleReference("expression", isToken: false))
        ]),
        lineNumber: 82),
    GrammarRule(name: "break_stmt", body: Literal("break"), lineNumber: 85),
    GrammarRule(
        name: "continue_stmt", body: Literal("continue"), lineNumber: 88),
    GrammarRule(name: "pass_stmt", body: Literal("pass"), lineNumber: 93),
    GrammarRule(
        name: "load_stmt",
        body: Sequence(elements: [
          Literal("load"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("STRING", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("load_arg", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 102),
    GrammarRule(
        name: "load_arg",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true),
            RuleReference("STRING", isToken: true)
          ]),
          RuleReference("STRING", isToken: true)
        ]),
        lineNumber: 103),
    GrammarRule(
        name: "assign_stmt",
        body: Sequence(elements: [
          RuleReference("expression_list", isToken: false),
          Optional(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("assign_op", isToken: false),
              RuleReference("augmented_assign_op", isToken: false)
            ])),
            RuleReference("expression_list", isToken: false)
          ]))
        ]),
        lineNumber: 124),
    GrammarRule(
        name: "assign_op",
        body: RuleReference("EQUALS", isToken: true),
        lineNumber: 127),
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
        lineNumber: 129),
    GrammarRule(
        name: "compound_stmt",
        body: Alternation(choices: [
          RuleReference("if_stmt", isToken: false),
          RuleReference("for_stmt", isToken: false),
          RuleReference("def_stmt", isToken: false)
        ]),
        lineNumber: 138),
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
        lineNumber: 150),
    GrammarRule(
        name: "for_stmt",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("loop_vars", isToken: false),
          Literal("in"),
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 164),
    GrammarRule(
        name: "loop_vars",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 170),
    GrammarRule(
        name: "def_stmt",
        body: Sequence(elements: [
          Literal("def"),
          RuleReference("NAME", isToken: true),
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("parameters", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("suite", isToken: false)
        ]),
        lineNumber: 180),
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
        lineNumber: 191),
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
        lineNumber: 212),
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
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true),
            RuleReference("expression", isToken: false)
          ]),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 214),
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
        lineNumber: 248),
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
        lineNumber: 253),
    GrammarRule(
        name: "lambda_expr",
        body: Sequence(elements: [
          Literal("lambda"),
          Optional(element: RuleReference("lambda_params", isToken: false)),
          RuleReference("COLON", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 258),
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
        lineNumber: 259),
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
        lineNumber: 260),
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
        lineNumber: 264),
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
        lineNumber: 268),
    GrammarRule(
        name: "not_expr",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("not"),
            RuleReference("not_expr", isToken: false)
          ]),
          RuleReference("comparison", isToken: false)
        ]),
        lineNumber: 272),
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
        lineNumber: 281),
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
          Sequence(elements: [Literal("not"), Literal("in")])
        ]),
        lineNumber: 283),
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
        lineNumber: 289),
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
        lineNumber: 290),
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
        lineNumber: 291),
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
        lineNumber: 294),
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
        lineNumber: 298),
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
        lineNumber: 303),
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
        lineNumber: 309),
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
        lineNumber: 317),
    GrammarRule(
        name: "primary",
        body: Sequence(elements: [
          RuleReference("atom", isToken: false),
          Repetition(element: RuleReference("suffix", isToken: false))
        ]),
        lineNumber: 334),
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
        lineNumber: 336),
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
        lineNumber: 348),
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
          Literal("True"),
          Literal("False"),
          Literal("None"),
          RuleReference("list_expr", isToken: false),
          RuleReference("dict_expr", isToken: false),
          RuleReference("paren_expr", isToken: false)
        ]),
        lineNumber: 357),
    GrammarRule(
        name: "list_expr",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(element: RuleReference("list_body", isToken: false)),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 373),
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
        lineNumber: 375),
    GrammarRule(
        name: "dict_expr",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(element: RuleReference("dict_body", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 381),
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
        lineNumber: 383),
    GrammarRule(
        name: "dict_entry",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 386),
    GrammarRule(
        name: "paren_expr",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("paren_body", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 393),
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
        lineNumber: 395),
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
        lineNumber: 411),
    GrammarRule(
        name: "comp_for",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("loop_vars", isToken: false),
          Literal("in"),
          RuleReference("or_expr", isToken: false)
        ]),
        lineNumber: 413),
    GrammarRule(
        name: "comp_if",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("or_expr", isToken: false)
        ]),
        lineNumber: 415),
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
        lineNumber: 434),
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
        lineNumber: 436),
  ],
);
