// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/oct/oct.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "program",
        body: Repetition(element: RuleReference("top_decl", isToken: false)),
        lineNumber: 48),
    GrammarRule(
        name: "top_decl",
        body: Alternation(choices: [
          RuleReference("static_decl", isToken: false),
          RuleReference("fn_decl", isToken: false)
        ]),
        lineNumber: 52),
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
        lineNumber: 69),
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
        lineNumber: 88),
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
        lineNumber: 92),
    GrammarRule(
        name: "param",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type", isToken: false)
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "block",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("stmt", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 106),
    GrammarRule(
        name: "stmt",
        body: Alternation(choices: [
          RuleReference("let_stmt", isToken: false),
          RuleReference("static_decl", isToken: false),
          RuleReference("assign_stmt", isToken: false),
          RuleReference("return_stmt", isToken: false),
          RuleReference("if_stmt", isToken: false),
          RuleReference("while_stmt", isToken: false),
          RuleReference("loop_stmt", isToken: false),
          RuleReference("break_stmt", isToken: false),
          RuleReference("expr_stmt", isToken: false)
        ]),
        lineNumber: 117),
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
        lineNumber: 139),
    GrammarRule(
        name: "assign_stmt",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQ", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 148),
    GrammarRule(
        name: "return_stmt",
        body: Sequence(elements: [
          Literal("return"),
          Optional(element: RuleReference("expr", isToken: false)),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 157),
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
        lineNumber: 166),
    GrammarRule(
        name: "while_stmt",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expr", isToken: false),
          RuleReference("block", isToken: false)
        ]),
        lineNumber: 174),
    GrammarRule(
        name: "loop_stmt",
        body: Sequence(elements: [
          Literal("loop"),
          RuleReference("block", isToken: false)
        ]),
        lineNumber: 181),
    GrammarRule(
        name: "break_stmt",
        body: Sequence(elements: [
          Literal("break"),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 185),
    GrammarRule(
        name: "expr_stmt",
        body: Sequence(elements: [
          RuleReference("expr", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 196),
    GrammarRule(
        name: "type",
        body: RuleReference("NAME", isToken: true),
        lineNumber: 230),
    GrammarRule(
        name: "expr",
        body: RuleReference("or_expr", isToken: false),
        lineNumber: 269),
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
        lineNumber: 274),
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
        lineNumber: 278),
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
        lineNumber: 285),
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
        lineNumber: 291),
    GrammarRule(
        name: "add_expr",
        body: Sequence(elements: [
          RuleReference("bitwise_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("bitwise_expr", isToken: false)
          ]))
        ]),
        lineNumber: 297),
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
        lineNumber: 302),
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
        lineNumber: 310),
    GrammarRule(
        name: "primary",
        body: Alternation(choices: [
          RuleReference("intrinsic_call", isToken: false),
          RuleReference("call_expr", isToken: false),
          RuleReference("INT_LIT", isToken: true),
          RuleReference("HEX_LIT", isToken: true),
          RuleReference("BIN_LIT", isToken: true),
          Literal("true"),
          Literal("false"),
          RuleReference("NAME", isToken: true),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("expr", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 323),
    GrammarRule(
        name: "call_expr",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("arg_list", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 341),
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
        lineNumber: 344),
    GrammarRule(
        name: "intrinsic_call",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("in"),
            RuleReference("LPAREN", isToken: true),
            RuleReference("expr", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ]),
          Sequence(elements: [
            Literal("out"),
            RuleReference("LPAREN", isToken: true),
            RuleReference("expr", isToken: false),
            RuleReference("COMMA", isToken: true),
            RuleReference("expr", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ]),
          Sequence(elements: [
            Literal("adc"),
            RuleReference("LPAREN", isToken: true),
            RuleReference("expr", isToken: false),
            RuleReference("COMMA", isToken: true),
            RuleReference("expr", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ]),
          Sequence(elements: [
            Literal("sbb"),
            RuleReference("LPAREN", isToken: true),
            RuleReference("expr", isToken: false),
            RuleReference("COMMA", isToken: true),
            RuleReference("expr", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ]),
          Sequence(elements: [
            Literal("rlc"),
            RuleReference("LPAREN", isToken: true),
            RuleReference("expr", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ]),
          Sequence(elements: [
            Literal("rrc"),
            RuleReference("LPAREN", isToken: true),
            RuleReference("expr", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ]),
          Sequence(elements: [
            Literal("ral"),
            RuleReference("LPAREN", isToken: true),
            RuleReference("expr", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ]),
          Sequence(elements: [
            Literal("rar"),
            RuleReference("LPAREN", isToken: true),
            RuleReference("expr", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ]),
          Sequence(elements: [
            Literal("carry"),
            RuleReference("LPAREN", isToken: true),
            RuleReference("RPAREN", isToken: true)
          ]),
          Sequence(elements: [
            Literal("parity"),
            RuleReference("LPAREN", isToken: true),
            RuleReference("expr", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 370),
  ],
);
