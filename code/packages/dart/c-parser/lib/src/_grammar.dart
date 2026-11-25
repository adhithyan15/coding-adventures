// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/c/c.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "translation_unit",
        body:
            Repetition(element: RuleReference("function_def", isToken: false)),
        lineNumber: 28),
    GrammarRule(
        name: "function_def",
        body: Sequence(elements: [
          RuleReference("type_spec", isToken: false),
          RuleReference("NAME", isToken: true),
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("param_list", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          RuleReference("compound_stmt", isToken: false)
        ]),
        lineNumber: 30),
    GrammarRule(
        name: "param_list",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("param", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("param", isToken: false)
            ]))
          ]),
          Literal("void")
        ]),
        lineNumber: 32),
    GrammarRule(
        name: "param",
        body: Sequence(elements: [
          RuleReference("type_spec", isToken: false),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 33),
    GrammarRule(
        name: "type_spec",
        body: Sequence(elements: [
          RuleReference("type_kw", isToken: false),
          Repetition(element: RuleReference("type_kw", isToken: false))
        ]),
        lineNumber: 41),
    GrammarRule(
        name: "type_kw",
        body: Alternation(choices: [
          Literal("void"),
          Literal("char"),
          Literal("short"),
          Literal("int"),
          Literal("long"),
          Literal("signed"),
          Literal("unsigned"),
          Literal("float"),
          Literal("double"),
          Literal("int8_t"),
          Literal("int16_t"),
          Literal("int32_t"),
          Literal("int64_t"),
          Literal("uint8_t"),
          Literal("uint16_t"),
          Literal("uint32_t"),
          Literal("uint64_t"),
          Literal("size_t")
        ]),
        lineNumber: 43),
    GrammarRule(
        name: "compound_stmt",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("block_item", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 52),
    GrammarRule(
        name: "block_item",
        body: Alternation(choices: [
          RuleReference("declaration", isToken: false),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 54),
    GrammarRule(
        name: "declaration",
        body: Sequence(elements: [
          RuleReference("init_declarator", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 58),
    GrammarRule(
        name: "init_declarator",
        body: Sequence(elements: [
          RuleReference("type_spec", isToken: false),
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("LBRACKET", isToken: true),
            Optional(element: RuleReference("INT_LIT", isToken: true)),
            RuleReference("RBRACKET", isToken: true)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQ", isToken: true),
            RuleReference("initializer", isToken: false)
          ]))
        ]),
        lineNumber: 62),
    GrammarRule(
        name: "initializer",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("LBRACE", isToken: true),
            Optional(element: RuleReference("init_list", isToken: false)),
            RuleReference("RBRACE", isToken: true)
          ]),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 63),
    GrammarRule(
        name: "init_list",
        body: Sequence(elements: [
          RuleReference("expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expr", isToken: false)
          ]))
        ]),
        lineNumber: 64),
    GrammarRule(
        name: "statement",
        body: Alternation(choices: [
          RuleReference("compound_stmt", isToken: false),
          RuleReference("if_stmt", isToken: false),
          RuleReference("while_stmt", isToken: false),
          RuleReference("for_stmt", isToken: false),
          RuleReference("return_stmt", isToken: false),
          RuleReference("expr_stmt", isToken: false)
        ]),
        lineNumber: 66),
    GrammarRule(
        name: "if_stmt",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("statement", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("statement", isToken: false)
          ]))
        ]),
        lineNumber: 73),
    GrammarRule(
        name: "while_stmt",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 74),
    GrammarRule(
        name: "for_stmt",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("for_clause", isToken: false)),
          RuleReference("SEMICOLON", isToken: true),
          Optional(element: RuleReference("expr", isToken: false)),
          RuleReference("SEMICOLON", isToken: true),
          Optional(element: RuleReference("expr", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 75),
    GrammarRule(
        name: "for_clause",
        body: Alternation(choices: [
          RuleReference("init_declarator", isToken: false),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "return_stmt",
        body: Sequence(elements: [
          Literal("return"),
          Optional(element: RuleReference("expr", isToken: false)),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 77),
    GrammarRule(
        name: "expr_stmt",
        body: Sequence(elements: [
          Optional(element: RuleReference("expr", isToken: false)),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 78),
    GrammarRule(
        name: "expr",
        body: RuleReference("assignment", isToken: false),
        lineNumber: 84),
    GrammarRule(
        name: "assignment",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("unary", isToken: false),
            RuleReference("EQ", isToken: true),
            RuleReference("assignment", isToken: false)
          ]),
          RuleReference("logical_or", isToken: false)
        ]),
        lineNumber: 87),
    GrammarRule(
        name: "logical_or",
        body: Sequence(elements: [
          RuleReference("logical_and", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("OR_OR", isToken: true),
            RuleReference("logical_and", isToken: false)
          ]))
        ]),
        lineNumber: 89),
    GrammarRule(
        name: "logical_and",
        body: Sequence(elements: [
          RuleReference("bit_or", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("AND_AND", isToken: true),
            RuleReference("bit_or", isToken: false)
          ]))
        ]),
        lineNumber: 90),
    GrammarRule(
        name: "bit_or",
        body: Sequence(elements: [
          RuleReference("bit_xor", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("PIPE", isToken: true),
            RuleReference("bit_xor", isToken: false)
          ]))
        ]),
        lineNumber: 91),
    GrammarRule(
        name: "bit_xor",
        body: Sequence(elements: [
          RuleReference("bit_and", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("CARET", isToken: true),
            RuleReference("bit_and", isToken: false)
          ]))
        ]),
        lineNumber: 92),
    GrammarRule(
        name: "bit_and",
        body: Sequence(elements: [
          RuleReference("equality", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("AMP", isToken: true),
            RuleReference("equality", isToken: false)
          ]))
        ]),
        lineNumber: 93),
    GrammarRule(
        name: "equality",
        body: Sequence(elements: [
          RuleReference("relational", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("EQ_EQ", isToken: true),
              RuleReference("NE", isToken: true)
            ])),
            RuleReference("relational", isToken: false)
          ]))
        ]),
        lineNumber: 94),
    GrammarRule(
        name: "relational",
        body: Sequence(elements: [
          RuleReference("shift", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("LE", isToken: true),
              RuleReference("GE", isToken: true),
              RuleReference("LT", isToken: true),
              RuleReference("GT", isToken: true)
            ])),
            RuleReference("shift", isToken: false)
          ]))
        ]),
        lineNumber: 95),
    GrammarRule(
        name: "shift",
        body: Sequence(elements: [
          RuleReference("additive", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("SHL", isToken: true),
              RuleReference("SHR", isToken: true)
            ])),
            RuleReference("additive", isToken: false)
          ]))
        ]),
        lineNumber: 96),
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
        lineNumber: 97),
    GrammarRule(
        name: "multiplicative",
        body: Sequence(elements: [
          RuleReference("cast", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true),
              RuleReference("PERCENT", isToken: true)
            ])),
            RuleReference("cast", isToken: false)
          ]))
        ]),
        lineNumber: 98),
    GrammarRule(
        name: "cast",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("type_spec", isToken: false),
            RuleReference("RPAREN", isToken: true),
            RuleReference("cast", isToken: false)
          ]),
          RuleReference("unary", isToken: false)
        ]),
        lineNumber: 103),
    GrammarRule(
        name: "unary",
        body: Alternation(choices: [
          Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true),
              RuleReference("TILDE", isToken: true),
              RuleReference("BANG", isToken: true)
            ])),
            RuleReference("unary", isToken: false)
          ]),
          RuleReference("postfix", isToken: false)
        ]),
        lineNumber: 105),
    GrammarRule(
        name: "postfix",
        body: Sequence(elements: [
          RuleReference("primary", isToken: false),
          Repetition(
              element: Alternation(choices: [
            RuleReference("call_suffix", isToken: false),
            RuleReference("index_suffix", isToken: false)
          ]))
        ]),
        lineNumber: 107),
    GrammarRule(
        name: "call_suffix",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("arg_list", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 108),
    GrammarRule(
        name: "index_suffix",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 109),
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
        lineNumber: 110),
    GrammarRule(
        name: "primary",
        body: Alternation(choices: [
          RuleReference("FLOAT_LIT", isToken: true),
          RuleReference("INT_LIT", isToken: true),
          RuleReference("CHAR_LIT", isToken: true),
          RuleReference("STR_LIT", isToken: true),
          RuleReference("NAME", isToken: true),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("expr", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 112),
  ],
);
