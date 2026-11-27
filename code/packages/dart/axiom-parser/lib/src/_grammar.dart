// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/axiom/axiom.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "program",
        body: RuleReference("expr", isToken: false),
        lineNumber: 108),
    GrammarRule(
        name: "expr",
        body: Alternation(choices: [
          RuleReference("if_expr", isToken: false),
          RuleReference("define", isToken: false),
          RuleReference("assignment", isToken: false),
          RuleReference("declaration", isToken: false),
          RuleReference("has_query", isToken: false),
          RuleReference("comparison", isToken: false)
        ]),
        lineNumber: 125),
    GrammarRule(
        name: "if_expr",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expr", isToken: false),
          Literal("then"),
          RuleReference("expr", isToken: false),
          Literal("else"),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 138),
    GrammarRule(
        name: "define",
        body: Alternation(choices: [
          RuleReference("declared_define", isToken: false),
          RuleReference("undeclared_define", isToken: false)
        ]),
        lineNumber: 178),
    GrammarRule(
        name: "declared_define",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("typed_param_list", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expr", isToken: false),
          RuleReference("DEFINE", isToken: true),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 181),
    GrammarRule(
        name: "typed_param_list",
        body: Sequence(elements: [
          RuleReference("typed_param", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("typed_param", isToken: false)
          ]))
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "typed_param",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expr", isToken: false)
        ]),
        lineNumber: 184),
    GrammarRule(
        name: "undeclared_define",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("DEFINE", isToken: true),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 186),
    GrammarRule(
        name: "assignment",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("ASSIGN", isToken: true),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 200),
    GrammarRule(
        name: "declaration",
        body: Sequence(elements: [
          RuleReference("decl_target", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expr", isToken: false)
        ]),
        lineNumber: 208),
    GrammarRule(
        name: "decl_target",
        body: Alternation(choices: [
          RuleReference("NAME", isToken: true),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("name_list", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 210),
    GrammarRule(
        name: "name_list",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 213),
    GrammarRule(
        name: "has_query",
        body: Sequence(elements: [
          RuleReference("type_expr", isToken: false),
          Literal("has"),
          RuleReference("type_expr", isToken: false)
        ]),
        lineNumber: 227),
    GrammarRule(
        name: "comparison",
        body: Sequence(elements: [
          RuleReference("coercion", isToken: false),
          Optional(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("EQ", isToken: true),
              RuleReference("NE", isToken: true),
              RuleReference("LE", isToken: true),
              RuleReference("LESS", isToken: true),
              RuleReference("GREATER", isToken: true),
              RuleReference("GE", isToken: true)
            ])),
            RuleReference("coercion", isToken: false)
          ]))
        ]),
        lineNumber: 272),
    GrammarRule(
        name: "coercion",
        body: Sequence(elements: [
          RuleReference("additive", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COERCE", isToken: true),
            RuleReference("type_expr", isToken: false)
          ]))
        ]),
        lineNumber: 274),
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
        lineNumber: 276),
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
        lineNumber: 277),
    GrammarRule(
        name: "unary",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("MINUS", isToken: true),
            RuleReference("unary", isToken: false)
          ]),
          RuleReference("power", isToken: false)
        ]),
        lineNumber: 283),
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
        lineNumber: 293),
    GrammarRule(
        name: "postfix",
        body: Sequence(elements: [
          RuleReference("atom", isToken: false),
          Optional(element: RuleReference("call_args", isToken: false))
        ]),
        lineNumber: 334),
    GrammarRule(
        name: "call_args",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("arglist", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ]),
          RuleReference("atom", isToken: false)
        ]),
        lineNumber: 336),
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
        lineNumber: 339),
    GrammarRule(
        name: "atom",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("list_literal", isToken: false),
          RuleReference("group", isToken: false)
        ]),
        lineNumber: 341),
    GrammarRule(
        name: "list_literal",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(element: RuleReference("elem_list", isToken: false)),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 353),
    GrammarRule(
        name: "elem_list",
        body: Sequence(elements: [
          RuleReference("expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expr", isToken: false)
          ]))
        ]),
        lineNumber: 354),
    GrammarRule(
        name: "group",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMI", isToken: true),
            RuleReference("expr", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 371),
    GrammarRule(
        name: "type_expr",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(element: RuleReference("type_ctor_args", isToken: false))
        ]),
        lineNumber: 416),
    GrammarRule(
        name: "type_ctor_args",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("type_expr_list", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ]),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 418),
    GrammarRule(
        name: "type_expr_list",
        body: Sequence(elements: [
          RuleReference("type_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expr", isToken: false)
          ]))
        ]),
        lineNumber: 421),
  ],
);
