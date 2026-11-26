// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/matlab/matlab.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 0,
  rules: [
    GrammarRule(
        name: "program",
        body: Repetition(
            element: RuleReference("statement_line", isToken: false)),
        lineNumber: 36),
    GrammarRule(
        name: "statement_line",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("statement", isToken: false),
            RuleReference("stmt_term", isToken: false)
          ]),
          RuleReference("statement", isToken: false),
          RuleReference("stmt_term", isToken: false)
        ]),
        lineNumber: 41),
    GrammarRule(
        name: "stmt_term",
        body: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("COMMA", isToken: true)
        ]),
        lineNumber: 45),
    GrammarRule(
        name: "statement",
        body: Alternation(choices: [
          RuleReference("func_def", isToken: false),
          RuleReference("if_stmt", isToken: false),
          RuleReference("for_stmt", isToken: false),
          RuleReference("while_stmt", isToken: false),
          RuleReference("switch_stmt", isToken: false),
          RuleReference("try_stmt", isToken: false),
          RuleReference("break_stmt", isToken: false),
          RuleReference("continue_stmt", isToken: false),
          RuleReference("return_stmt", isToken: false),
          RuleReference("global_stmt", isToken: false),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 47),
    GrammarRule(
        name: "block_body",
        body: Repetition(
            element: RuleReference("statement_line", isToken: false)),
        lineNumber: 62),
    GrammarRule(
        name: "if_stmt",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expr", isToken: false),
          RuleReference("block_body", isToken: false),
          Repetition(element: RuleReference("elseif_clause", isToken: false)),
          Optional(element: RuleReference("else_clause", isToken: false)),
          Literal("end")
        ]),
        lineNumber: 68),
    GrammarRule(
        name: "elseif_clause",
        body: Sequence(elements: [
          Literal("elseif"),
          RuleReference("expr", isToken: false),
          RuleReference("block_body", isToken: false)
        ]),
        lineNumber: 69),
    GrammarRule(
        name: "else_clause",
        body: Sequence(elements: [
          Literal("else"),
          RuleReference("block_body", isToken: false)
        ]),
        lineNumber: 70),
    GrammarRule(
        name: "for_stmt",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("NAME", isToken: true),
          RuleReference("EQ", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("block_body", isToken: false),
          Literal("end")
        ]),
        lineNumber: 72),
    GrammarRule(
        name: "while_stmt",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expr", isToken: false),
          RuleReference("block_body", isToken: false),
          Literal("end")
        ]),
        lineNumber: 74),
    GrammarRule(
        name: "switch_stmt",
        body: Sequence(elements: [
          Literal("switch"),
          RuleReference("expr", isToken: false),
          RuleReference("block_body", isToken: false),
          Repetition(element: RuleReference("case_clause", isToken: false)),
          Optional(element: RuleReference("otherwise_clause", isToken: false)),
          Literal("end")
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "case_clause",
        body: Sequence(elements: [
          Literal("case"),
          RuleReference("expr", isToken: false),
          RuleReference("block_body", isToken: false)
        ]),
        lineNumber: 77),
    GrammarRule(
        name: "otherwise_clause",
        body: Sequence(elements: [
          Literal("otherwise"),
          RuleReference("block_body", isToken: false)
        ]),
        lineNumber: 78),
    GrammarRule(
        name: "try_stmt",
        body: Sequence(elements: [
          Literal("try"),
          RuleReference("block_body", isToken: false),
          Optional(element: RuleReference("catch_clause", isToken: false)),
          Literal("end")
        ]),
        lineNumber: 80),
    GrammarRule(
        name: "catch_clause",
        body: Sequence(elements: [
          Literal("catch"),
          Optional(element: RuleReference("NAME", isToken: true)),
          RuleReference("block_body", isToken: false)
        ]),
        lineNumber: 81),
    GrammarRule(name: "break_stmt", body: Literal("break"), lineNumber: 83),
    GrammarRule(
        name: "continue_stmt", body: Literal("continue"), lineNumber: 84),
    GrammarRule(name: "return_stmt", body: Literal("return"), lineNumber: 85),
    GrammarRule(
        name: "global_stmt",
        body: Sequence(elements: [
          Group(
              element: Alternation(
                  choices: [Literal("global"), Literal("persistent")])),
          RuleReference("name_list", isToken: false)
        ]),
        lineNumber: 86),
    GrammarRule(
        name: "func_def",
        body: Sequence(elements: [
          Literal("function"),
          Optional(element: RuleReference("func_returns", isToken: false)),
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("name_list", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          RuleReference("block_body", isToken: false),
          Literal("end")
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "func_returns",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQ", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LBRACKET", isToken: true),
            Optional(element: RuleReference("name_list", isToken: false)),
            RuleReference("RBRACKET", isToken: true),
            RuleReference("EQ", isToken: true)
          ])
        ]),
        lineNumber: 97),
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
        lineNumber: 99),
    GrammarRule(
        name: "expr",
        body: RuleReference("assignment", isToken: false),
        lineNumber: 105),
    GrammarRule(
        name: "assignment",
        body: Sequence(elements: [
          RuleReference("logical_or", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQ", isToken: true),
            RuleReference("assignment", isToken: false)
          ]))
        ]),
        lineNumber: 108),
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
        lineNumber: 110),
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
        lineNumber: 111),
    GrammarRule(
        name: "bit_or",
        body: Sequence(elements: [
          RuleReference("bit_and", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("PIPE", isToken: true),
            RuleReference("bit_and", isToken: false)
          ]))
        ]),
        lineNumber: 112),
    GrammarRule(
        name: "bit_and",
        body: Sequence(elements: [
          RuleReference("comparison", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("AMP", isToken: true),
            RuleReference("comparison", isToken: false)
          ]))
        ]),
        lineNumber: 113),
    GrammarRule(
        name: "comparison",
        body: Sequence(elements: [
          RuleReference("colon_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("EQ_EQ", isToken: true),
              RuleReference("NE", isToken: true),
              RuleReference("LE", isToken: true),
              RuleReference("GE", isToken: true),
              RuleReference("LT", isToken: true),
              RuleReference("GT", isToken: true)
            ])),
            RuleReference("colon_expr", isToken: false)
          ]))
        ]),
        lineNumber: 115),
    GrammarRule(
        name: "colon_expr",
        body: Sequence(elements: [
          RuleReference("additive", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("additive", isToken: false),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              RuleReference("additive", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 119),
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
        lineNumber: 121),
    GrammarRule(
        name: "multiplicative",
        body: Sequence(elements: [
          RuleReference("unary", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true),
              RuleReference("BACKSLASH", isToken: true),
              RuleReference("ELEM_MUL", isToken: true),
              RuleReference("ELEM_RDIV", isToken: true),
              RuleReference("ELEM_LDIV", isToken: true)
            ])),
            RuleReference("unary", isToken: false)
          ]))
        ]),
        lineNumber: 124),
    GrammarRule(
        name: "unary",
        body: Alternation(choices: [
          Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true),
              RuleReference("TILDE", isToken: true)
            ])),
            RuleReference("unary", isToken: false)
          ]),
          RuleReference("power", isToken: false)
        ]),
        lineNumber: 127),
    GrammarRule(
        name: "power",
        body: Sequence(elements: [
          RuleReference("postfix", isToken: false),
          Optional(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("CARET", isToken: true),
              RuleReference("ELEM_POW", isToken: true)
            ])),
            RuleReference("unary", isToken: false)
          ]))
        ]),
        lineNumber: 131),
    GrammarRule(
        name: "postfix",
        body: Sequence(elements: [
          RuleReference("primary", isToken: false),
          Repetition(
              element: Alternation(choices: [
            RuleReference("transpose_suffix", isToken: false),
            RuleReference("call_suffix", isToken: false),
            RuleReference("cell_suffix", isToken: false),
            RuleReference("field_suffix", isToken: false)
          ]))
        ]),
        lineNumber: 134),
    GrammarRule(
        name: "transpose_suffix",
        body: Alternation(choices: [
          RuleReference("TRANSPOSE", isToken: true),
          RuleReference("ELEM_TRANSPOSE", isToken: true)
        ]),
        lineNumber: 136),
    GrammarRule(
        name: "call_suffix",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("arg_list", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 137),
    GrammarRule(
        name: "cell_suffix",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(element: RuleReference("arg_list", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 138),
    GrammarRule(
        name: "field_suffix",
        body: Sequence(elements: [
          RuleReference("DOT", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 139),
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
        lineNumber: 142),
    GrammarRule(
        name: "arg",
        body: Alternation(choices: [
          RuleReference("COLON", isToken: true),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 143),
    GrammarRule(
        name: "primary",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("matrix_literal", isToken: false),
          RuleReference("cell_literal", isToken: false),
          RuleReference("lambda", isToken: false),
          RuleReference("group", isToken: false),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 149),
    GrammarRule(
        name: "group",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 157),
    GrammarRule(
        name: "lambda",
        body: Sequence(elements: [
          RuleReference("AT", isToken: true),
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("name_list", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 159),
    GrammarRule(
        name: "matrix_literal",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(element: RuleReference("matrix_rows", isToken: false)),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 176),
    GrammarRule(
        name: "cell_literal",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(element: RuleReference("matrix_rows", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 177),
    GrammarRule(
        name: "matrix_rows",
        body: Sequence(elements: [
          RuleReference("matrix_row", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("row_sep", isToken: false),
            RuleReference("matrix_row", isToken: false)
          ])),
          Optional(element: RuleReference("row_sep", isToken: false))
        ]),
        lineNumber: 179),
    GrammarRule(
        name: "row_sep",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 180),
    GrammarRule(
        name: "matrix_row",
        body: Sequence(elements: [
          RuleReference("expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Optional(element: RuleReference("COMMA", isToken: true)),
            RuleReference("expr", isToken: false)
          ]))
        ]),
        lineNumber: 181),
  ],
);
