// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/maple/maple.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "program",
        body: Sequence(elements: [
          Repetition(element: RuleReference("statement_line", isToken: false)),
          Optional(element: RuleReference("statement", isToken: false))
        ]),
        lineNumber: 288),
    GrammarRule(
        name: "statement_line",
        body: Sequence(elements: [
          RuleReference("statement", isToken: false),
          Group(
              element: Alternation(choices: [
            RuleReference("SEMI", isToken: true),
            RuleReference("COLON", isToken: true)
          ]))
        ]),
        lineNumber: 294),
    GrammarRule(
        name: "statement",
        body: Alternation(choices: [
          RuleReference("if_expr", isToken: false),
          RuleReference("assignment", isToken: false)
        ]),
        lineNumber: 298),
    GrammarRule(
        name: "if_expr",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expr", isToken: false),
          Literal("then"),
          RuleReference("statement", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("elif"),
            RuleReference("expr", isToken: false),
            Literal("then"),
            RuleReference("statement", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("statement", isToken: false)
          ])),
          Group(
              element: Alternation(choices: [
            Sequence(elements: [Literal("end"), Literal("if")]),
            Literal("fi")
          ]))
        ]),
        lineNumber: 342),
    GrammarRule(
        name: "assignment",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("ASSIGN", isToken: true),
            Group(
                element: Alternation(choices: [
              RuleReference("arrow_def", isToken: false),
              RuleReference("expr", isToken: false)
            ]))
          ]),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 351),
    GrammarRule(
        name: "arrow_def",
        body: Sequence(elements: [
          RuleReference("arrow_params", isToken: false),
          RuleReference("ARROW", isToken: true),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 354),
    GrammarRule(
        name: "arrow_params",
        body: Alternation(choices: [
          RuleReference("NAME", isToken: true),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("NAME", isToken: true),
              Repetition(
                  element: Sequence(elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("NAME", isToken: true)
              ]))
            ])),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 362),
    GrammarRule(
        name: "expr",
        body: RuleReference("logical_or", isToken: false),
        lineNumber: 371),
    GrammarRule(
        name: "logical_or",
        body: Sequence(elements: [
          RuleReference("logical_and", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("or"),
            RuleReference("logical_and", isToken: false)
          ]))
        ]),
        lineNumber: 373),
    GrammarRule(
        name: "logical_and",
        body: Sequence(elements: [
          RuleReference("logical_not", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            RuleReference("logical_not", isToken: false)
          ]))
        ]),
        lineNumber: 374),
    GrammarRule(
        name: "logical_not",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("not"),
            RuleReference("logical_not", isToken: false)
          ]),
          RuleReference("comparison", isToken: false)
        ]),
        lineNumber: 375),
    GrammarRule(
        name: "comparison",
        body: Sequence(elements: [
          RuleReference("additive", isToken: false),
          Optional(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("EQ", isToken: true),
              RuleReference("NEQ", isToken: true),
              RuleReference("LESS", isToken: true),
              RuleReference("GREATER", isToken: true),
              RuleReference("LE", isToken: true),
              RuleReference("GE", isToken: true)
            ])),
            RuleReference("additive", isToken: false)
          ]))
        ]),
        lineNumber: 378),
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
        lineNumber: 380),
    GrammarRule(
        name: "multiplicative",
        body: Sequence(elements: [
          RuleReference("unary", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("TIMES", isToken: true),
              RuleReference("SLASH", isToken: true)
            ])),
            RuleReference("unary", isToken: false)
          ]))
        ]),
        lineNumber: 384),
    GrammarRule(
        name: "unary",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("MINUS", isToken: true),
            RuleReference("unary", isToken: false)
          ]),
          RuleReference("power", isToken: false)
        ]),
        lineNumber: 389),
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
        lineNumber: 398),
    GrammarRule(
        name: "postfix",
        body: Sequence(elements: [
          RuleReference("atom", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("arglist", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ]))
        ]),
        lineNumber: 401),
    GrammarRule(
        name: "arglist",
        body: Sequence(elements: [
          RuleReference("expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expr", isToken: false)
          ]))
        ]),
        lineNumber: 403),
    GrammarRule(
        name: "atom",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("NAME", isToken: true),
          Literal("true"),
          Literal("false"),
          RuleReference("list_literal", isToken: false),
          RuleReference("set_literal", isToken: false),
          RuleReference("group", isToken: false)
        ]),
        lineNumber: 405),
    GrammarRule(
        name: "list_literal",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(element: RuleReference("arglist", isToken: false)),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 419),
    GrammarRule(
        name: "set_literal",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(element: RuleReference("arglist", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 433),
    GrammarRule(
        name: "group",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 435),
  ],
);
