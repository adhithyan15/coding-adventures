// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/reduce/reduce.grammar
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
        lineNumber: 120),
    GrammarRule(
        name: "statement_line",
        body: Sequence(elements: [
          RuleReference("statement", isToken: false),
          Group(
              element: Alternation(choices: [
            RuleReference("SEMI", isToken: true),
            RuleReference("DOLLAR", isToken: true)
          ]))
        ]),
        lineNumber: 129),
    GrammarRule(
        name: "statement",
        body: RuleReference("expr", isToken: false),
        lineNumber: 131),
    GrammarRule(
        name: "expr",
        body: Alternation(choices: [
          RuleReference("if_expr", isToken: false),
          RuleReference("group_expr", isToken: false),
          RuleReference("assignment", isToken: false)
        ]),
        lineNumber: 138),
    GrammarRule(
        name: "if_expr",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expr", isToken: false),
          Literal("then"),
          RuleReference("expr", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("expr", isToken: false)
          ]))
        ]),
        lineNumber: 155),
    GrammarRule(
        name: "group_expr",
        body: Sequence(elements: [
          RuleReference("GROUP_OPEN", isToken: true),
          RuleReference("expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("SEMI", isToken: true),
              RuleReference("DOLLAR", isToken: true)
            ])),
            RuleReference("expr", isToken: false)
          ])),
          RuleReference("GROUP_CLOSE", isToken: true)
        ]),
        lineNumber: 164),
    GrammarRule(
        name: "assignment",
        body: Sequence(elements: [
          RuleReference("logical_or", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("ASSIGN", isToken: true),
            RuleReference("expr", isToken: false)
          ]))
        ]),
        lineNumber: 184),
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
        lineNumber: 186),
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
        lineNumber: 187),
    GrammarRule(
        name: "logical_not",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("not"),
            RuleReference("logical_not", isToken: false)
          ]),
          RuleReference("comparison", isToken: false)
        ]),
        lineNumber: 188),
    GrammarRule(
        name: "comparison",
        body: Sequence(elements: [
          RuleReference("cons", isToken: false),
          Optional(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("EQ", isToken: true),
              Literal("neq"),
              RuleReference("LESS", isToken: true),
              RuleReference("GREATER", isToken: true),
              RuleReference("LE", isToken: true),
              RuleReference("GE", isToken: true)
            ])),
            RuleReference("cons", isToken: false)
          ]))
        ]),
        lineNumber: 198),
    GrammarRule(
        name: "cons",
        body: Sequence(elements: [
          RuleReference("additive", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("cons", isToken: false)
          ]))
        ]),
        lineNumber: 201),
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
        lineNumber: 203),
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
        lineNumber: 204),
    GrammarRule(
        name: "unary",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("MINUS", isToken: true),
            RuleReference("unary", isToken: false)
          ]),
          RuleReference("power", isToken: false)
        ]),
        lineNumber: 209),
    GrammarRule(
        name: "power",
        body: Sequence(elements: [
          RuleReference("postfix", isToken: false),
          Optional(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("CARET", isToken: true),
              RuleReference("POW", isToken: true)
            ])),
            RuleReference("unary", isToken: false)
          ]))
        ]),
        lineNumber: 217),
    GrammarRule(
        name: "postfix",
        body: Sequence(elements: [
          RuleReference("atom", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("arglist", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ]))
        ]),
        lineNumber: 228),
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
        lineNumber: 230),
    GrammarRule(
        name: "atom",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("list_literal", isToken: false),
          RuleReference("group", isToken: false)
        ]),
        lineNumber: 232),
    GrammarRule(
        name: "list_literal",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(element: RuleReference("arglist", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 242),
    GrammarRule(
        name: "group",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 244),
  ],
);
