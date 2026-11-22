// AUTO-GENERATED FILE - DO NOT EDIT
// Source: verilog1995.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 0,
  rules: [
    GrammarRule(
        name: "source_text",
        body: Repetition(element: RuleReference("description", isToken: false)),
        lineNumber: 45),
    GrammarRule(
        name: "description",
        body: RuleReference("module_declaration", isToken: false),
        lineNumber: 47),
    GrammarRule(
        name: "module_declaration",
        body: Sequence(elements: [
          Literal("module"),
          RuleReference("NAME", isToken: true),
          Optional(
              element: RuleReference("parameter_port_list", isToken: false)),
          Optional(element: RuleReference("port_list", isToken: false)),
          RuleReference("SEMICOLON", isToken: true),
          Repetition(element: RuleReference("module_item", isToken: false)),
          Literal("endmodule")
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "parameter_port_list",
        body: Sequence(elements: [
          RuleReference("HASH", isToken: true),
          RuleReference("LPAREN", isToken: true),
          RuleReference("parameter_declaration", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("parameter_declaration", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 94),
    GrammarRule(
        name: "parameter_declaration",
        body: Sequence(elements: [
          Literal("parameter"),
          Optional(element: RuleReference("range", isToken: false)),
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 97),
    GrammarRule(
        name: "localparam_declaration",
        body: Sequence(elements: [
          Literal("localparam"),
          Optional(element: RuleReference("range", isToken: false)),
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 98),
    GrammarRule(
        name: "port_list",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("port", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("port", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "port",
        body: Sequence(elements: [
          Optional(element: RuleReference("port_direction", isToken: false)),
          Optional(element: RuleReference("net_type", isToken: false)),
          Optional(element: Literal("signed")),
          Optional(element: RuleReference("range", isToken: false)),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 120),
    GrammarRule(
        name: "port_direction",
        body: Alternation(
            choices: [Literal("input"), Literal("output"), Literal("inout")]),
        lineNumber: 122),
    GrammarRule(
        name: "net_type",
        body: Alternation(choices: [
          Literal("wire"),
          Literal("reg"),
          Literal("tri"),
          Literal("supply0"),
          Literal("supply1")
        ]),
        lineNumber: 123),
    GrammarRule(
        name: "range",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 125),
    GrammarRule(
        name: "module_item",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("port_declaration", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("net_declaration", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("reg_declaration", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("integer_declaration", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("parameter_declaration", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("localparam_declaration", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          RuleReference("continuous_assign", isToken: false),
          RuleReference("always_construct", isToken: false),
          RuleReference("initial_construct", isToken: false),
          RuleReference("module_instantiation", isToken: false),
          RuleReference("generate_region", isToken: false),
          RuleReference("function_declaration", isToken: false),
          RuleReference("task_declaration", isToken: false)
        ]),
        lineNumber: 142),
    GrammarRule(
        name: "port_declaration",
        body: Sequence(elements: [
          RuleReference("port_direction", isToken: false),
          Optional(element: RuleReference("net_type", isToken: false)),
          Optional(element: Literal("signed")),
          Optional(element: RuleReference("range", isToken: false)),
          RuleReference("name_list", isToken: false)
        ]),
        lineNumber: 177),
    GrammarRule(
        name: "net_declaration",
        body: Sequence(elements: [
          RuleReference("net_type", isToken: false),
          Optional(element: Literal("signed")),
          Optional(element: RuleReference("range", isToken: false)),
          RuleReference("name_list", isToken: false)
        ]),
        lineNumber: 179),
    GrammarRule(
        name: "reg_declaration",
        body: Sequence(elements: [
          Literal("reg"),
          Optional(element: Literal("signed")),
          Optional(element: RuleReference("range", isToken: false)),
          RuleReference("name_list", isToken: false)
        ]),
        lineNumber: 180),
    GrammarRule(
        name: "integer_declaration",
        body: Sequence(elements: [
          Literal("integer"),
          RuleReference("name_list", isToken: false)
        ]),
        lineNumber: 181),
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
        lineNumber: 182),
    GrammarRule(
        name: "continuous_assign",
        body: Sequence(elements: [
          Literal("assign"),
          RuleReference("assignment", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("assignment", isToken: false)
          ])),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 201),
    GrammarRule(
        name: "assignment",
        body: Sequence(elements: [
          RuleReference("lvalue", isToken: false),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 202),
    GrammarRule(
        name: "lvalue",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            Optional(element: RuleReference("range_select", isToken: false))
          ]),
          RuleReference("concatenation", isToken: false)
        ]),
        lineNumber: 206),
    GrammarRule(
        name: "range_select",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 209),
    GrammarRule(
        name: "always_construct",
        body: Sequence(elements: [
          Literal("always"),
          RuleReference("AT", isToken: true),
          RuleReference("sensitivity_list", isToken: false),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 246),
    GrammarRule(
        name: "initial_construct",
        body: Sequence(elements: [
          Literal("initial"),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 247),
    GrammarRule(
        name: "sensitivity_list",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("sensitivity_item", isToken: false),
            Repetition(
                element: Sequence(elements: [
              Group(
                  element: Alternation(choices: [
                Literal("or"),
                RuleReference("COMMA", isToken: true)
              ])),
              RuleReference("sensitivity_item", isToken: false)
            ])),
            RuleReference("RPAREN", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("STAR", isToken: true),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 249),
    GrammarRule(
        name: "sensitivity_item",
        body: Sequence(elements: [
          Optional(
              element: Alternation(
                  choices: [Literal("posedge"), Literal("negedge")])),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 253),
    GrammarRule(
        name: "statement",
        body: Alternation(choices: [
          RuleReference("block_statement", isToken: false),
          RuleReference("if_statement", isToken: false),
          RuleReference("case_statement", isToken: false),
          RuleReference("for_statement", isToken: false),
          Sequence(elements: [
            RuleReference("blocking_assignment", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("nonblocking_assignment", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("task_call", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 262),
    GrammarRule(
        name: "block_statement",
        body: Sequence(elements: [
          Literal("begin"),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("NAME", isToken: true)
          ])),
          Repetition(element: RuleReference("statement", isToken: false)),
          Literal("end")
        ]),
        lineNumber: 278),
    GrammarRule(
        name: "if_statement",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("statement", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("statement", isToken: false)
          ]))
        ]),
        lineNumber: 289),
    GrammarRule(
        name: "case_statement",
        body: Sequence(elements: [
          Group(
              element: Alternation(choices: [
            Literal("case"),
            Literal("casex"),
            Literal("casez")
          ])),
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true),
          Repetition(element: RuleReference("case_item", isToken: false)),
          Literal("endcase")
        ]),
        lineNumber: 304),
    GrammarRule(
        name: "case_item",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("expression_list", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("statement", isToken: false)
          ]),
          Sequence(elements: [
            Literal("default"),
            Optional(element: RuleReference("COLON", isToken: true)),
            RuleReference("statement", isToken: false)
          ])
        ]),
        lineNumber: 309),
    GrammarRule(
        name: "expression_list",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 312),
    GrammarRule(
        name: "for_statement",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("blocking_assignment", isToken: false),
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("blocking_assignment", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 316),
    GrammarRule(
        name: "blocking_assignment",
        body: Sequence(elements: [
          RuleReference("lvalue", isToken: false),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 320),
    GrammarRule(
        name: "nonblocking_assignment",
        body: Sequence(elements: [
          RuleReference("lvalue", isToken: false),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 321),
    GrammarRule(
        name: "task_call",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("LPAREN", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 324),
    GrammarRule(
        name: "module_instantiation",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element:
                  RuleReference("parameter_value_assignment", isToken: false)),
          RuleReference("instance", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("instance", isToken: false)
          ])),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 343),
    GrammarRule(
        name: "parameter_value_assignment",
        body: Sequence(elements: [
          RuleReference("HASH", isToken: true),
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 346),
    GrammarRule(
        name: "instance",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("LPAREN", isToken: true),
          RuleReference("port_connections", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 348),
    GrammarRule(
        name: "port_connections",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("named_port_connection", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("named_port_connection", isToken: false)
            ]))
          ]),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 350),
    GrammarRule(
        name: "named_port_connection",
        body: Sequence(elements: [
          RuleReference("DOT", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("expression", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 353),
    GrammarRule(
        name: "generate_region",
        body: Sequence(elements: [
          Literal("generate"),
          Repetition(element: RuleReference("generate_item", isToken: false)),
          Literal("endgenerate")
        ]),
        lineNumber: 380),
    GrammarRule(
        name: "generate_item",
        body: Alternation(choices: [
          RuleReference("genvar_declaration", isToken: false),
          RuleReference("generate_for", isToken: false),
          RuleReference("generate_if", isToken: false),
          RuleReference("module_item", isToken: false)
        ]),
        lineNumber: 382),
    GrammarRule(
        name: "genvar_declaration",
        body: Sequence(elements: [
          Literal("genvar"),
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("NAME", isToken: true)
          ])),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 387),
    GrammarRule(
        name: "generate_for",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("genvar_assignment", isToken: false),
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("genvar_assignment", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("generate_block", isToken: false)
        ]),
        lineNumber: 389),
    GrammarRule(
        name: "generate_if",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("generate_block", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            RuleReference("generate_block", isToken: false)
          ]))
        ]),
        lineNumber: 393),
    GrammarRule(
        name: "generate_block",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("begin"),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              RuleReference("NAME", isToken: true)
            ])),
            Repetition(element: RuleReference("generate_item", isToken: false)),
            Literal("end")
          ]),
          RuleReference("generate_item", isToken: false)
        ]),
        lineNumber: 396),
    GrammarRule(
        name: "genvar_assignment",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 399),
    GrammarRule(
        name: "function_declaration",
        body: Sequence(elements: [
          Literal("function"),
          Optional(element: RuleReference("range", isToken: false)),
          RuleReference("NAME", isToken: true),
          RuleReference("SEMICOLON", isToken: true),
          Repetition(element: RuleReference("function_item", isToken: false)),
          RuleReference("statement", isToken: false),
          Literal("endfunction")
        ]),
        lineNumber: 418),
    GrammarRule(
        name: "function_item",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("port_declaration", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("reg_declaration", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("integer_declaration", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("parameter_declaration", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ])
        ]),
        lineNumber: 423),
    GrammarRule(
        name: "task_declaration",
        body: Sequence(elements: [
          Literal("task"),
          RuleReference("NAME", isToken: true),
          RuleReference("SEMICOLON", isToken: true),
          Repetition(element: RuleReference("task_item", isToken: false)),
          RuleReference("statement", isToken: false),
          Literal("endtask")
        ]),
        lineNumber: 428),
    GrammarRule(
        name: "task_item",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("port_declaration", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("reg_declaration", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("integer_declaration", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ])
        ]),
        lineNumber: 433),
    GrammarRule(
        name: "expression",
        body: RuleReference("ternary_expr", isToken: false),
        lineNumber: 461),
    GrammarRule(
        name: "ternary_expr",
        body: Sequence(elements: [
          RuleReference("or_expr", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("QUESTION", isToken: true),
            RuleReference("expression", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("ternary_expr", isToken: false)
          ]))
        ]),
        lineNumber: 467),
    GrammarRule(
        name: "or_expr",
        body: Sequence(elements: [
          RuleReference("and_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("LOGIC_OR", isToken: true),
            RuleReference("and_expr", isToken: false)
          ]))
        ]),
        lineNumber: 470),
    GrammarRule(
        name: "and_expr",
        body: Sequence(elements: [
          RuleReference("bit_or_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("LOGIC_AND", isToken: true),
            RuleReference("bit_or_expr", isToken: false)
          ]))
        ]),
        lineNumber: 471),
    GrammarRule(
        name: "bit_or_expr",
        body: Sequence(elements: [
          RuleReference("bit_xor_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("PIPE", isToken: true),
            RuleReference("bit_xor_expr", isToken: false)
          ]))
        ]),
        lineNumber: 474),
    GrammarRule(
        name: "bit_xor_expr",
        body: Sequence(elements: [
          RuleReference("bit_and_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("CARET", isToken: true),
            RuleReference("bit_and_expr", isToken: false)
          ]))
        ]),
        lineNumber: 475),
    GrammarRule(
        name: "bit_and_expr",
        body: Sequence(elements: [
          RuleReference("equality_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("AMP", isToken: true),
            RuleReference("equality_expr", isToken: false)
          ]))
        ]),
        lineNumber: 476),
    GrammarRule(
        name: "equality_expr",
        body: Sequence(elements: [
          RuleReference("relational_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("EQUALS_EQUALS", isToken: true),
              RuleReference("NOT_EQUALS", isToken: true),
              RuleReference("CASE_EQ", isToken: true),
              RuleReference("CASE_NEQ", isToken: true)
            ])),
            RuleReference("relational_expr", isToken: false)
          ]))
        ]),
        lineNumber: 480),
    GrammarRule(
        name: "relational_expr",
        body: Sequence(elements: [
          RuleReference("shift_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("LESS_THAN", isToken: true),
              RuleReference("LESS_EQUALS", isToken: true),
              RuleReference("GREATER_THAN", isToken: true),
              RuleReference("GREATER_EQUALS", isToken: true)
            ])),
            RuleReference("shift_expr", isToken: false)
          ]))
        ]),
        lineNumber: 487),
    GrammarRule(
        name: "shift_expr",
        body: Sequence(elements: [
          RuleReference("additive_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("LEFT_SHIFT", isToken: true),
              RuleReference("RIGHT_SHIFT", isToken: true),
              RuleReference("ARITH_LEFT_SHIFT", isToken: true),
              RuleReference("ARITH_RIGHT_SHIFT", isToken: true)
            ])),
            RuleReference("additive_expr", isToken: false)
          ]))
        ]),
        lineNumber: 492),
    GrammarRule(
        name: "additive_expr",
        body: Sequence(elements: [
          RuleReference("multiplicative_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("multiplicative_expr", isToken: false)
          ]))
        ]),
        lineNumber: 497),
    GrammarRule(
        name: "multiplicative_expr",
        body: Sequence(elements: [
          RuleReference("power_expr", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true),
              RuleReference("PERCENT", isToken: true)
            ])),
            RuleReference("power_expr", isToken: false)
          ]))
        ]),
        lineNumber: 498),
    GrammarRule(
        name: "power_expr",
        body: Sequence(elements: [
          RuleReference("unary_expr", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("POWER", isToken: true),
            RuleReference("unary_expr", isToken: false)
          ]))
        ]),
        lineNumber: 499),
    GrammarRule(
        name: "unary_expr",
        body: Alternation(choices: [
          Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true),
              RuleReference("BANG", isToken: true),
              RuleReference("TILDE", isToken: true),
              RuleReference("AMP", isToken: true),
              RuleReference("PIPE", isToken: true),
              RuleReference("CARET", isToken: true),
              Sequence(elements: [
                RuleReference("TILDE", isToken: true),
                RuleReference("AMP", isToken: true)
              ]),
              Sequence(elements: [
                RuleReference("TILDE", isToken: true),
                RuleReference("PIPE", isToken: true)
              ]),
              Sequence(elements: [
                RuleReference("TILDE", isToken: true),
                RuleReference("CARET", isToken: true)
              ])
            ])),
            RuleReference("unary_expr", isToken: false)
          ]),
          RuleReference("primary", isToken: false)
        ]),
        lineNumber: 511),
    GrammarRule(
        name: "primary",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("SIZED_NUMBER", isToken: true),
          RuleReference("REAL_NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("SYSTEM_ID", isToken: true),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("expression", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ]),
          RuleReference("concatenation", isToken: false),
          RuleReference("replication", isToken: false),
          Sequence(elements: [
            RuleReference("primary", isToken: false),
            RuleReference("LBRACKET", isToken: true),
            RuleReference("expression", isToken: false),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              RuleReference("expression", isToken: false)
            ])),
            RuleReference("RBRACKET", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("expression", isToken: false),
              Repetition(
                  element: Sequence(elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("expression", isToken: false)
              ]))
            ])),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 521),
    GrammarRule(
        name: "concatenation",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 537),
    GrammarRule(
        name: "replication",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("concatenation", isToken: false),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 543),
  ],
);
