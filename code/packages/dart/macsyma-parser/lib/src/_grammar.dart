// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/macsyma/macsyma.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 2,
  rules: [
    GrammarRule(
        name: "program",
        body: Repetition(element: RuleReference("statement", isToken: false)),
        lineNumber: 31),
    GrammarRule(
        name: "statement",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          Group(
              element: Alternation(choices: [
            RuleReference("SEMI", isToken: true),
            RuleReference("DOLLAR", isToken: true)
          ]))
        ]),
        lineNumber: 33),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("if_expr", isToken: false),
          RuleReference("for_expr", isToken: false),
          RuleReference("while_expr", isToken: false),
          RuleReference("block_expr", isToken: false),
          RuleReference("return_expr", isToken: false),
          RuleReference("assign", isToken: false)
        ]),
        lineNumber: 44),
    GrammarRule(
        name: "if_expr",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          Literal("then"),
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("elseif"),
            RuleReference("expression", isToken: false),
            Literal("then"),
            RuleReference("expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 54),
    GrammarRule(
        name: "for_expr",
        body: Alternation(choices: [
          RuleReference("for_each_expr", isToken: false),
          RuleReference("for_range_expr", isToken: false)
        ]),
        lineNumber: 67),
    GrammarRule(
        name: "for_each_expr",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("NAME", isToken: true),
          Literal("in"),
          RuleReference("expression", isToken: false),
          Literal("do"),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 69),
    GrammarRule(
        name: "for_range_expr",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal(":"),
            RuleReference("expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("step"),
            RuleReference("expression", isToken: false)
          ])),
          Group(
              element: Alternation(choices: [
            Literal("thru"),
            Literal("while"),
            Literal("unless")
          ])),
          RuleReference("expression", isToken: false),
          Literal("do"),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 71),
    GrammarRule(
        name: "while_expr",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          Literal("do"),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "block_expr",
        body: Sequence(elements: [
          Literal("block"),
          Literal("("),
          Optional(element: RuleReference("arglist", isToken: false)),
          Literal(")")
        ]),
        lineNumber: 82),
    GrammarRule(
        name: "return_expr",
        body: Sequence(elements: [
          Literal("return"),
          Literal("("),
          RuleReference("expression", isToken: false),
          Literal(")")
        ]),
        lineNumber: 87),
    GrammarRule(
        name: "assign",
        body: Sequence(elements: [
          RuleReference("logical_or", isToken: false),
          Optional(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("COLON", isToken: true),
              RuleReference("COLONEQ", isToken: true)
            ])),
            RuleReference("assign", isToken: false)
          ]))
        ]),
        lineNumber: 92),
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
        lineNumber: 97),
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
        lineNumber: 98),
    GrammarRule(
        name: "logical_not",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("not"),
            RuleReference("logical_not", isToken: false)
          ]),
          RuleReference("comparison", isToken: false)
        ]),
        lineNumber: 99),
    GrammarRule(
        name: "comparison",
        body: Sequence(elements: [
          RuleReference("additive", isToken: false),
          Optional(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("EQ", isToken: true),
              RuleReference("HASH", isToken: true),
              RuleReference("LT", isToken: true),
              RuleReference("GT", isToken: true),
              RuleReference("LEQ", isToken: true),
              RuleReference("GEQ", isToken: true)
            ])),
            RuleReference("additive", isToken: false)
          ]))
        ]),
        lineNumber: 103),
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
        lineNumber: 105),
    GrammarRule(
        name: "multiplicative",
        body: Sequence(elements: [
          RuleReference("unary", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true)
            ])),
            RuleReference("unary", isToken: false)
          ]))
        ]),
        lineNumber: 106),
    GrammarRule(
        name: "unary",
        body: Alternation(choices: [
          Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("MINUS", isToken: true),
              RuleReference("PLUS", isToken: true)
            ])),
            RuleReference("unary", isToken: false)
          ]),
          RuleReference("power", isToken: false)
        ]),
        lineNumber: 110),
    GrammarRule(
        name: "power",
        body: Sequence(elements: [
          RuleReference("postfix", isToken: false),
          Optional(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("CARET", isToken: true),
              RuleReference("STAREQ", isToken: true)
            ])),
            RuleReference("unary", isToken: false)
          ]))
        ]),
        lineNumber: 114),
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
        lineNumber: 118),
    GrammarRule(
        name: "arglist",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 119),
    GrammarRule(
        name: "atom",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("NAME", isToken: true),
          Literal("true"),
          Literal("false"),
          RuleReference("group", isToken: false),
          RuleReference("list", isToken: false)
        ]),
        lineNumber: 121),
    GrammarRule(
        name: "group",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 129),
    GrammarRule(
        name: "list",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(element: RuleReference("arglist", isToken: false)),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 130),
  ],
);
