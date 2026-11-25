// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/derive/derive.grammar
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
            RuleReference("NEWLINE", isToken: true)
          ]),
          RuleReference("statement", isToken: false),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 51),
    GrammarRule(
        name: "statement",
        body: RuleReference("expr", isToken: false),
        lineNumber: 55),
    GrammarRule(
        name: "expr",
        body: RuleReference("assignment", isToken: false),
        lineNumber: 57),
    GrammarRule(
        name: "assignment",
        body: Sequence(elements: [
          RuleReference("logical_or", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("ASSIGN", isToken: true),
            RuleReference("assignment", isToken: false)
          ]))
        ]),
        lineNumber: 69),
    GrammarRule(
        name: "logical_or",
        body: Sequence(elements: [
          RuleReference("logical_and", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("OR"),
            RuleReference("logical_and", isToken: false)
          ]))
        ]),
        lineNumber: 71),
    GrammarRule(
        name: "logical_and",
        body: Sequence(elements: [
          RuleReference("logical_not", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("AND"),
            RuleReference("logical_not", isToken: false)
          ]))
        ]),
        lineNumber: 72),
    GrammarRule(
        name: "logical_not",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("NOT"),
            RuleReference("logical_not", isToken: false)
          ]),
          RuleReference("comparison", isToken: false)
        ]),
        lineNumber: 73),
    GrammarRule(
        name: "comparison",
        body: Sequence(elements: [
          RuleReference("additive", isToken: false),
          Optional(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("EQ", isToken: true),
              RuleReference("LE", isToken: true),
              RuleReference("LESS", isToken: true),
              RuleReference("GREATER", isToken: true),
              RuleReference("GE", isToken: true)
            ])),
            RuleReference("additive", isToken: false)
          ]))
        ]),
        lineNumber: 79),
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
        lineNumber: 81),
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
        lineNumber: 82),
    GrammarRule(
        name: "unary",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("MINUS", isToken: true),
            RuleReference("unary", isToken: false)
          ]),
          RuleReference("power", isToken: false)
        ]),
        lineNumber: 88),
    GrammarRule(
        name: "power",
        body: Sequence(elements: [
          RuleReference("postfix", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("POWER", isToken: true),
            RuleReference("unary", isToken: false)
          ]))
        ]),
        lineNumber: 94),
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
        lineNumber: 101),
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
        lineNumber: 103),
    GrammarRule(
        name: "atom",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("vector", isToken: false),
          RuleReference("group", isToken: false)
        ]),
        lineNumber: 105),
    GrammarRule(
        name: "vector",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("row", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMI", isToken: true),
            RuleReference("row", isToken: false)
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 117),
    GrammarRule(
        name: "row",
        body: Sequence(elements: [
          RuleReference("expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expr", isToken: false)
          ]))
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "group",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 120),
  ],
);
