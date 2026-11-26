// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/nib/nib.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "program",
        body: Repetition(element: RuleReference("top_decl", isToken: false)),
        lineNumber: 42),
    GrammarRule(
        name: "top_decl",
        body: Alternation(choices: [
          RuleReference("const_decl", isToken: false),
          RuleReference("static_decl", isToken: false),
          RuleReference("fn_decl", isToken: false)
        ]),
        lineNumber: 47),
    GrammarRule(
        name: "const_decl",
        body: Sequence(elements: [
          Literal("const"),
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type", isToken: false),
          RuleReference("EQ", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 60),
    GrammarRule(
        name: "static_decl",
        body: Sequence(elements: [
          Literal("static"),
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type", isToken: false),
          RuleReference("EQ", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 66),
    GrammarRule(
        name: "fn_decl",
        body: Sequence(elements: [
          Literal("fn"),
          RuleReference("NAME", isToken: true),
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("param_list", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("type", isToken: false)
          ])),
          RuleReference("block", isToken: false)
        ]),
        lineNumber: 77),
    GrammarRule(
        name: "param_list",
        body: Sequence(elements: [
          RuleReference("param", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("param", isToken: false)
          ]))
        ]),
        lineNumber: 80),
    GrammarRule(
        name: "param",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type", isToken: false)
        ]),
        lineNumber: 87),
    GrammarRule(
        name: "block",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("stmt", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 98),
    GrammarRule(
        name: "stmt",
        body: Alternation(choices: [
          RuleReference("let_stmt", isToken: false),
          RuleReference("assign_stmt", isToken: false),
          RuleReference("return_stmt", isToken: false),
          RuleReference("for_stmt", isToken: false),
          RuleReference("while_stmt", isToken: false),
          RuleReference("if_stmt", isToken: false),
          RuleReference("expr_stmt", isToken: false)
        ]),
        lineNumber: 113),
    GrammarRule(
        name: "let_stmt",
        body: Sequence(elements: [
          Literal("let"),
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type", isToken: false),
          RuleReference("EQ", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 126),
    GrammarRule(
        name: "assign_stmt",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQ", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 131),
    GrammarRule(
        name: "return_stmt",
        body: Sequence(elements: [
          Literal("return"),
          RuleReference("expr", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 136),
    GrammarRule(
        name: "for_stmt",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type", isToken: false),
          Literal("in"),
          RuleReference("expr", isToken: false),
          RuleReference("RANGE", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("block", isToken: false)
        ]),
        lineNumber: 159),
    GrammarRule(
        name: "while_stmt",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expr", isToken: false),
          RuleReference("block", isToken: false)
        ]),
        lineNumber: 170),
    GrammarRule(
        name: "if_stmt",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expr", isToken: false),
          RuleReference("block", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("block", isToken: false)
          ]))
        ]),
        lineNumber: 176),
    GrammarRule(
        name: "expr_stmt",
        body: Sequence(elements: [
          RuleReference("expr", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "type",
        body: Alternation(choices: [
          Literal("u4"),
          Literal("u8"),
          Literal("bcd"),
          Literal("bool")
        ]),
        lineNumber: 218),
    GrammarRule(
        name: "expr",
        body: RuleReference("or_expr", isToken: false),
        lineNumber: 259),
    GrammarRule(
        name: "or_expr",
        body: Sequence(elements: [
          RuleReference("and_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("LOR", isToken: true),
            RuleReference("and_expr", isToken: false)
          ]))
        ]),
        lineNumber: 265),
    GrammarRule(
        name: "and_expr",
        body: Sequence(elements: [
          RuleReference("eq_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("LAND", isToken: true),
            RuleReference("eq_expr", isToken: false)
          ]))
        ]),
        lineNumber: 269),
    GrammarRule(
        name: "eq_expr",
        body: Sequence(elements: [
          RuleReference("cmp_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("EQ_EQ", isToken: true),
              RuleReference("NEQ", isToken: true)
            ])),
            RuleReference("cmp_expr", isToken: false)
          ]))
        ]),
        lineNumber: 274),
    GrammarRule(
        name: "cmp_expr",
        body: Sequence(elements: [
          RuleReference("add_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("LT", isToken: true),
              RuleReference("GT", isToken: true),
              RuleReference("LEQ", isToken: true),
              RuleReference("GEQ", isToken: true)
            ])),
            RuleReference("add_expr", isToken: false)
          ]))
        ]),
        lineNumber: 280),
    GrammarRule(
        name: "add_expr",
        body: Sequence(elements: [
          RuleReference("shift_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true),
              RuleReference("WRAP_ADD", isToken: true),
              RuleReference("SAT_ADD", isToken: true)
            ])),
            RuleReference("shift_expr", isToken: false)
          ]))
        ]),
        lineNumber: 293),
    GrammarRule(
        name: "shift_expr",
        body: Sequence(elements: [
          RuleReference("mul_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("SHL", isToken: true),
              RuleReference("SHR", isToken: true)
            ])),
            RuleReference("mul_expr", isToken: false)
          ]))
        ]),
        lineNumber: 298),
    GrammarRule(
        name: "mul_expr",
        body: Sequence(elements: [
          RuleReference("bitwise_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true),
              RuleReference("PERCENT", isToken: true)
            ])),
            RuleReference("bitwise_expr", isToken: false)
          ]))
        ]),
        lineNumber: 308),
    GrammarRule(
        name: "bitwise_expr",
        body: Sequence(elements: [
          RuleReference("unary_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("AMP", isToken: true),
              RuleReference("PIPE", isToken: true),
              RuleReference("CARET", isToken: true)
            ])),
            RuleReference("unary_expr", isToken: false)
          ]))
        ]),
        lineNumber: 314),
    GrammarRule(
        name: "unary_expr",
        body: Alternation(choices: [
          Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("BANG", isToken: true),
              RuleReference("TILDE", isToken: true)
            ])),
            RuleReference("unary_expr", isToken: false)
          ]),
          RuleReference("primary", isToken: false)
        ]),
        lineNumber: 322),
    GrammarRule(
        name: "primary",
        body: Alternation(choices: [
          RuleReference("INT_LIT", isToken: true),
          RuleReference("HEX_LIT", isToken: true),
          Literal("true"),
          Literal("false"),
          RuleReference("call_expr", isToken: false),
          RuleReference("NAME", isToken: true),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("expr", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 330),
    GrammarRule(
        name: "call_expr",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("arg_list", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 353),
    GrammarRule(
        name: "arg_list",
        body: Sequence(elements: [
          RuleReference("expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expr", isToken: false)
          ]))
        ]),
        lineNumber: 356),
  ],
);
