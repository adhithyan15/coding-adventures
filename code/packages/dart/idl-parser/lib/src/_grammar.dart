// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/idl/idl.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 0,
  rules: [
    GrammarRule(
        name: "program",
        body: Repetition(
            element: RuleReference("top_level_item", isToken: false)),
        lineNumber: 279),
    GrammarRule(
        name: "top_level_item",
        body: Alternation(choices: [
          RuleReference("pro_def", isToken: false),
          RuleReference("func_def", isToken: false),
          RuleReference("statement_line", isToken: false)
        ]),
        lineNumber: 281),
    GrammarRule(
        name: "statement_line",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("statement", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("STMT_SEP", isToken: true),
              RuleReference("statement", isToken: false)
            ])),
            Optional(element: RuleReference("NEWLINE", isToken: true))
          ]),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 292),
    GrammarRule(
        name: "block_body",
        body: Repetition(
            element: RuleReference("statement_line", isToken: false)),
        lineNumber: 302),
    GrammarRule(
        name: "statement",
        body: Alternation(choices: [
          RuleReference("if_stmt", isToken: false),
          RuleReference("for_stmt", isToken: false),
          RuleReference("while_stmt", isToken: false),
          RuleReference("repeat_stmt", isToken: false),
          RuleReference("begin_block", isToken: false),
          RuleReference("break_stmt", isToken: false),
          RuleReference("continue_stmt", isToken: false),
          RuleReference("return_stmt", isToken: false),
          RuleReference("procedure_call_stmt", isToken: false),
          RuleReference("assignment_stmt", isToken: false),
          RuleReference("expr_stmt", isToken: false)
        ]),
        lineNumber: 304),
    GrammarRule(
        name: "if_stmt",
        body: Sequence(elements: [
          Literal("IF"),
          RuleReference("expr", isToken: false),
          Literal("THEN"),
          RuleReference("then_branch", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("ELSE"),
            RuleReference("else_branch", isToken: false)
          ]))
        ]),
        lineNumber: 324),
    GrammarRule(
        name: "then_branch",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("BEGIN"),
            RuleReference("block_body", isToken: false),
            Group(
                element:
                    Alternation(choices: [Literal("ENDIF"), Literal("END")]))
          ]),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 326),
    GrammarRule(
        name: "else_branch",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("BEGIN"),
            RuleReference("block_body", isToken: false),
            Group(
                element:
                    Alternation(choices: [Literal("ENDELSE"), Literal("END")]))
          ]),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 329),
    GrammarRule(
        name: "for_stmt",
        body: Sequence(elements: [
          Literal("FOR"),
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("expr", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expr", isToken: false)
          ])),
          Literal("DO"),
          RuleReference("for_body", isToken: false)
        ]),
        lineNumber: 335),
    GrammarRule(
        name: "for_body",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("BEGIN"),
            RuleReference("block_body", isToken: false),
            Group(
                element:
                    Alternation(choices: [Literal("ENDFOR"), Literal("END")]))
          ]),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 337),
    GrammarRule(
        name: "while_stmt",
        body: Sequence(elements: [
          Literal("WHILE"),
          RuleReference("expr", isToken: false),
          Literal("DO"),
          RuleReference("while_body", isToken: false)
        ]),
        lineNumber: 342),
    GrammarRule(
        name: "while_body",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("BEGIN"),
            RuleReference("block_body", isToken: false),
            Group(
                element:
                    Alternation(choices: [Literal("ENDWHILE"), Literal("END")]))
          ]),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 344),
    GrammarRule(
        name: "repeat_stmt",
        body: Sequence(elements: [
          Literal("REPEAT"),
          RuleReference("repeat_body", isToken: false),
          Literal("UNTIL"),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 351),
    GrammarRule(
        name: "repeat_body",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("BEGIN"),
            RuleReference("block_body", isToken: false),
            Group(
                element:
                    Alternation(choices: [Literal("ENDREP"), Literal("END")]))
          ]),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 353),
    GrammarRule(
        name: "begin_block",
        body: Sequence(elements: [
          Literal("BEGIN"),
          RuleReference("block_body", isToken: false),
          Literal("END")
        ]),
        lineNumber: 361),
    GrammarRule(name: "break_stmt", body: Literal("BREAK"), lineNumber: 363),
    GrammarRule(
        name: "continue_stmt", body: Literal("CONTINUE"), lineNumber: 364),
    GrammarRule(
        name: "return_stmt",
        body: Sequence(elements: [
          Literal("RETURN"),
          Optional(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expr", isToken: false)
          ]))
        ]),
        lineNumber: 375),
    GrammarRule(
        name: "procedure_call_stmt",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COMMA", isToken: true),
          RuleReference("arg_list", isToken: false)
        ]),
        lineNumber: 393),
    GrammarRule(
        name: "assignment_stmt",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(element: RuleReference("index_suffix", isToken: false)),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 402),
    GrammarRule(
        name: "expr_stmt",
        body: RuleReference("expr", isToken: false),
        lineNumber: 408),
    GrammarRule(
        name: "pro_def",
        body: Sequence(elements: [
          Literal("PRO"),
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("params", isToken: false)
          ])),
          RuleReference("block_body", isToken: false),
          Literal("END")
        ]),
        lineNumber: 429),
    GrammarRule(
        name: "func_def",
        body: Sequence(elements: [
          Literal("FUNCTION"),
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("params", isToken: false)
          ])),
          RuleReference("block_body", isToken: false),
          Literal("END")
        ]),
        lineNumber: 430),
    GrammarRule(
        name: "params",
        body: Sequence(elements: [
          RuleReference("param", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("param", isToken: false)
          ]))
        ]),
        lineNumber: 432),
    GrammarRule(
        name: "param",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 433),
    GrammarRule(
        name: "expr",
        body: RuleReference("logical", isToken: false),
        lineNumber: 444),
    GrammarRule(
        name: "logical",
        body: Sequence(elements: [
          RuleReference("comparison", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(
                    choices: [Literal("AND"), Literal("OR"), Literal("XOR")])),
            RuleReference("comparison", isToken: false)
          ]))
        ]),
        lineNumber: 446),
    GrammarRule(
        name: "comparison",
        body: Sequence(elements: [
          RuleReference("additive", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              Literal("EQ"),
              Literal("NE"),
              Literal("LE"),
              Literal("LT"),
              Literal("GE"),
              Literal("GT")
            ])),
            RuleReference("additive", isToken: false)
          ]))
        ]),
        lineNumber: 448),
    GrammarRule(
        name: "additive",
        body: Sequence(elements: [
          RuleReference("unary", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("unary", isToken: false)
          ]))
        ]),
        lineNumber: 454),
    GrammarRule(
        name: "unary",
        body: Alternation(choices: [
          Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true),
              Literal("NOT")
            ])),
            RuleReference("unary", isToken: false)
          ]),
          RuleReference("multiplicative", isToken: false)
        ]),
        lineNumber: 461),
    GrammarRule(
        name: "multiplicative",
        body: Sequence(elements: [
          RuleReference("power", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true),
              RuleReference("HASH_HASH", isToken: true),
              RuleReference("HASH", isToken: true)
            ])),
            RuleReference("power", isToken: false)
          ]))
        ]),
        lineNumber: 472),
    GrammarRule(
        name: "power",
        body: Sequence(elements: [
          RuleReference("postfix", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("CARET", isToken: true),
            RuleReference("postfix", isToken: false)
          ]))
        ]),
        lineNumber: 479),
    GrammarRule(
        name: "postfix",
        body: Sequence(elements: [
          RuleReference("primary", isToken: false),
          Repetition(
              element: Alternation(choices: [
            RuleReference("index_suffix", isToken: false),
            RuleReference("call_suffix", isToken: false)
          ]))
        ]),
        lineNumber: 487),
    GrammarRule(
        name: "index_suffix",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("subscript_list", isToken: false),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 489),
    GrammarRule(
        name: "call_suffix",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("arg_list", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 490),
    GrammarRule(
        name: "subscript_list",
        body: Sequence(elements: [
          RuleReference("subscript", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("subscript", isToken: false)
          ]))
        ]),
        lineNumber: 499),
    GrammarRule(
        name: "subscript",
        body: Alternation(choices: [
          RuleReference("STAR", isToken: true),
          RuleReference("range_subscript", isToken: false),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 507),
    GrammarRule(
        name: "range_subscript",
        body: Sequence(elements: [
          RuleReference("expr", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("range_end", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("expr", isToken: false)
          ]))
        ]),
        lineNumber: 515),
    GrammarRule(
        name: "range_end",
        body: Alternation(choices: [
          RuleReference("STAR", isToken: true),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 516),
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
        lineNumber: 527),
    GrammarRule(
        name: "arg",
        body: Alternation(choices: [
          RuleReference("keyword_arg", isToken: false),
          RuleReference("bool_keyword_arg", isToken: false),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 535),
    GrammarRule(
        name: "keyword_arg",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expr", isToken: false)
        ]),
        lineNumber: 542),
    GrammarRule(
        name: "bool_keyword_arg",
        body: Sequence(elements: [
          RuleReference("SLASH", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 549),
    GrammarRule(
        name: "primary",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("array_literal", isToken: false),
          RuleReference("group", isToken: false),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 555),
    GrammarRule(
        name: "group",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expr", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 561),
    GrammarRule(
        name: "array_literal",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(element: RuleReference("array_elements", isToken: false)),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 571),
    GrammarRule(
        name: "array_elements",
        body: Sequence(elements: [
          RuleReference("expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expr", isToken: false)
          ]))
        ]),
        lineNumber: 572),
  ],
);
