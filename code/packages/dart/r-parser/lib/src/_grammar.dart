// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/r/r.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "program",
        body: Repetition(
            element: RuleReference("statement_line", isToken: false)),
        lineNumber: 46),
    GrammarRule(
        name: "statement_line",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("statement", isToken: false),
            Group(
                element: Alternation(choices: [
              RuleReference("NEWLINE", isToken: true),
              RuleReference("SEMICOLON", isToken: true)
            ]))
          ]),
          RuleReference("statement", isToken: false),
          RuleReference("NEWLINE", isToken: true),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "statement",
        body: RuleReference("expr", isToken: false),
        lineNumber: 53),
    GrammarRule(
        name: "expr",
        body: RuleReference("assignment", isToken: false),
        lineNumber: 55),
    GrammarRule(
        name: "assignment",
        body: Sequence(elements: [
          RuleReference("comparison", isToken: false),
          Optional(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("ASSIGN", isToken: true),
              RuleReference("SUPER_ASSIGN", isToken: true),
              RuleReference("RIGHT_ASSIGN", isToken: true),
              RuleReference("RIGHT_SUPER_ASSIGN", isToken: true),
              RuleReference("EQ", isToken: true)
            ])),
            RuleReference("assignment", isToken: false)
          ]))
        ]),
        lineNumber: 64),
    GrammarRule(
        name: "comparison",
        body: Sequence(elements: [
          RuleReference("additive", isToken: false),
          Optional(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("EQEQ", isToken: true),
              RuleReference("NE", isToken: true),
              RuleReference("LE", isToken: true),
              RuleReference("GE", isToken: true),
              RuleReference("LT", isToken: true),
              RuleReference("GT", isToken: true)
            ])),
            RuleReference("additive", isToken: false)
          ]))
        ]),
        lineNumber: 66),
    GrammarRule(
        name: "additive",
        body: Sequence(elements: [
          RuleReference("multiplicative", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("multiplicative", isToken: false)
          ]))
        ]),
        lineNumber: 68),
    GrammarRule(
        name: "multiplicative",
        body: Sequence(elements: [
          RuleReference("special", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true)
            ])),
            RuleReference("special", isToken: false)
          ]))
        ]),
        lineNumber: 70),
    GrammarRule(
        name: "special",
        body: Sequence(elements: [
          RuleReference("pipe", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("PERCENT_OP", isToken: true),
            RuleReference("pipe", isToken: false)
          ]))
        ]),
        lineNumber: 73),
    GrammarRule(
        name: "pipe",
        body: Sequence(elements: [
          RuleReference("range", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("PIPE_OP", isToken: true),
            RuleReference("range", isToken: false)
          ]))
        ]),
        lineNumber: 78),
    GrammarRule(
        name: "range",
        body: Sequence(elements: [
          RuleReference("unary", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("unary", isToken: false)
          ]))
        ]),
        lineNumber: 81),
    GrammarRule(
        name: "unary",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("MINUS", isToken: true),
            RuleReference("unary", isToken: false)
          ]),
          RuleReference("power", isToken: false)
        ]),
        lineNumber: 83),
    GrammarRule(
        name: "power",
        body: Sequence(elements: [
          RuleReference("postfix", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("CARET", isToken: true),
            RuleReference("unary", isToken: false)
          ]))
        ]),
        lineNumber: 87),
    GrammarRule(
        name: "postfix",
        body: Sequence(elements: [
          RuleReference("primary", isToken: false),
          Repetition(
              element: Alternation(choices: [
            RuleReference("call_suffix", isToken: false),
            RuleReference("dindex_suffix", isToken: false),
            RuleReference("index_suffix", isToken: false),
            RuleReference("dollar_suffix", isToken: false)
          ]))
        ]),
        lineNumber: 94),
    GrammarRule(
        name: "call_suffix",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("arg_list", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "dindex_suffix",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("LBRACKET", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("RBRACKET", isToken: true),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 97),
    GrammarRule(
        name: "index_suffix",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(element: RuleReference("subscript", isToken: false)),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            Optional(element: RuleReference("subscript", isToken: false))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 102),
    GrammarRule(
        name: "subscript",
        body: RuleReference("expr", isToken: false),
        lineNumber: 103),
    GrammarRule(
        name: "dollar_suffix",
        body: Sequence(elements: [
          RuleReference("DOLLAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 104),
    GrammarRule(
        name: "arg_list",
        body: Sequence(elements: [
          RuleReference("arg", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("arg", isToken: false)
          ]))
        ]),
        lineNumber: 106),
    GrammarRule(
        name: "arg",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQ", isToken: true),
            Optional(element: RuleReference("expr", isToken: false))
          ]),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 119),
    GrammarRule(
        name: "primary",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("HEX_LIT", isToken: true),
          RuleReference("INT_LIT", isToken: true),
          RuleReference("COMPLEX_LIT", isToken: true),
          RuleReference("STRING", isToken: true),
          Literal("TRUE"),
          Literal("FALSE"),
          Literal("T"),
          Literal("F"),
          Literal("NULL"),
          Literal("NA"),
          Literal("NA_integer_"),
          Literal("NA_real_"),
          Literal("NA_character_"),
          Literal("Inf"),
          Literal("NaN"),
          RuleReference("func_def", isToken: false),
          RuleReference("if_expr", isToken: false),
          RuleReference("for_expr", isToken: false),
          RuleReference("while_expr", isToken: false),
          RuleReference("repeat_expr", isToken: false),
          Literal("break"),
          Literal("next"),
          RuleReference("block", isToken: false),
          RuleReference("group", isToken: false),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 127),
    GrammarRule(
        name: "group",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 154),
    GrammarRule(
        name: "block",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("statement_line", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 156),
    GrammarRule(
        name: "func_def",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("function"),
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("param_list", isToken: false)),
            RuleReference("RPAREN", isToken: true),
            RuleReference("expr", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("BACKSLASH", isToken: true),
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("param_list", isToken: false)),
            RuleReference("RPAREN", isToken: true),
            RuleReference("expr", isToken: false)
          ])
        ]),
        lineNumber: 161),
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
        lineNumber: 164),
    GrammarRule(
        name: "param",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQ", isToken: true),
            RuleReference("expr", isToken: false)
          ]),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 166),
    GrammarRule(
        name: "if_expr",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("expr", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("expr", isToken: false)
          ]))
        ]),
        lineNumber: 169),
    GrammarRule(
        name: "for_expr",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("NAME", isToken: true),
          Literal("in"),
          RuleReference("expr", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 171),
    GrammarRule(
        name: "while_expr",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 173),
    GrammarRule(
        name: "repeat_expr",
        body: Sequence(elements: [
          Literal("repeat"),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 175),
  ],
);
