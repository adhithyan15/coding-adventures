// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/typescript/ts1.0.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "program",
        body: Repetition(
            element: RuleReference("source_element", isToken: false)),
        lineNumber: 56),
    GrammarRule(
        name: "source_element",
        body: Alternation(choices: [
          RuleReference("interface_declaration", isToken: false),
          RuleReference("type_alias_declaration", isToken: false),
          RuleReference("enum_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("ambient_declaration", isToken: false),
          RuleReference("function_declaration", isToken: false),
          RuleReference("ts_class_declaration", isToken: false),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 65),
    GrammarRule(
        name: "function_declaration",
        body: Sequence(elements: [
          Literal("function"),
          RuleReference("NAME", isToken: true),
          Optional(element: RuleReference("type_parameters", isToken: false)),
          RuleReference("LPAREN", isToken: true),
          Optional(
              element: RuleReference("typed_parameter_list", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("LBRACE", isToken: true),
          RuleReference("function_body", isToken: false),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 91),
    GrammarRule(
        name: "function_body",
        body: Repetition(
            element: RuleReference("source_element", isToken: false)),
        lineNumber: 95),
    GrammarRule(
        name: "typed_parameter_list",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("typed_parameter", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("typed_parameter", isToken: false)
            ])),
            Optional(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("rest_typed_parameter", isToken: false)
            ]))
          ]),
          RuleReference("rest_typed_parameter", isToken: false)
        ]),
        lineNumber: 113),
    GrammarRule(
        name: "typed_parameter",
        body: Sequence(elements: [
          Optional(
              element: RuleReference("accessibility_modifier", isToken: false)),
          Group(
              element: Alternation(choices: [
            RuleReference("NAME", isToken: true),
            RuleReference("binding_pattern", isToken: false)
          ])),
          Optional(element: RuleReference("QUESTION", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            RuleReference("assignment_expression", isToken: false)
          ]))
        ]),
        lineNumber: 116),
    GrammarRule(
        name: "rest_typed_parameter",
        body: Sequence(elements: [
          RuleReference("ELLIPSIS", isToken: true),
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ]))
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "variable_statement",
        body: Sequence(elements: [
          Literal("var"),
          RuleReference("variable_declaration_list", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 128),
    GrammarRule(
        name: "variable_declaration_list",
        body: Sequence(elements: [
          RuleReference("variable_declaration", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("variable_declaration", isToken: false)
          ]))
        ]),
        lineNumber: 130),
    GrammarRule(
        name: "variable_declaration",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            RuleReference("assignment_expression", isToken: false)
          ]))
        ]),
        lineNumber: 132),
    GrammarRule(
        name: "statement",
        body: Alternation(choices: [
          RuleReference("block", isToken: false),
          RuleReference("variable_statement", isToken: false),
          RuleReference("empty_statement", isToken: false),
          RuleReference("if_statement", isToken: false),
          RuleReference("while_statement", isToken: false),
          RuleReference("do_while_statement", isToken: false),
          RuleReference("for_statement", isToken: false),
          RuleReference("for_in_statement", isToken: false),
          RuleReference("continue_statement", isToken: false),
          RuleReference("break_statement", isToken: false),
          RuleReference("return_statement", isToken: false),
          RuleReference("with_statement", isToken: false),
          RuleReference("switch_statement", isToken: false),
          RuleReference("labelled_statement", isToken: false),
          RuleReference("try_statement", isToken: false),
          RuleReference("throw_statement", isToken: false),
          RuleReference("debugger_statement", isToken: false),
          RuleReference("expression_statement", isToken: false)
        ]),
        lineNumber: 141),
    GrammarRule(
        name: "block",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("statement", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 160),
    GrammarRule(
        name: "empty_statement",
        body: RuleReference("SEMICOLON", isToken: true),
        lineNumber: 162),
    GrammarRule(
        name: "expression_statement",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 164),
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
        lineNumber: 166),
    GrammarRule(
        name: "while_statement",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 168),
    GrammarRule(
        name: "do_while_statement",
        body: Sequence(elements: [
          Literal("do"),
          RuleReference("statement", isToken: false),
          Literal("while"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 170),
    GrammarRule(
        name: "for_statement",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("LPAREN", isToken: true),
          Group(
              element: Alternation(choices: [
            Sequence(elements: [
              Literal("var"),
              RuleReference("variable_declaration_list", isToken: false)
            ]),
            Optional(element: RuleReference("expression", isToken: false))
          ])),
          RuleReference("SEMICOLON", isToken: true),
          Optional(element: RuleReference("expression", isToken: false)),
          RuleReference("SEMICOLON", isToken: true),
          Optional(element: RuleReference("expression", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 172),
    GrammarRule(
        name: "for_in_statement",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("LPAREN", isToken: true),
          Group(
              element: Alternation(choices: [
            Sequence(elements: [
              Literal("var"),
              RuleReference("variable_declaration", isToken: false)
            ]),
            RuleReference("left_hand_side_expression", isToken: false)
          ])),
          Literal("in"),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 178),
    GrammarRule(
        name: "continue_statement",
        body: Sequence(elements: [
          Literal("continue"),
          Optional(element: RuleReference("NAME", isToken: true)),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 182),
    GrammarRule(
        name: "break_statement",
        body: Sequence(elements: [
          Literal("break"),
          Optional(element: RuleReference("NAME", isToken: true)),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 184),
    GrammarRule(
        name: "return_statement",
        body: Sequence(elements: [
          Literal("return"),
          Optional(element: RuleReference("expression", isToken: false)),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 186),
    GrammarRule(
        name: "with_statement",
        body: Sequence(elements: [
          Literal("with"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 188),
    GrammarRule(
        name: "switch_statement",
        body: Sequence(elements: [
          Literal("switch"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("case_clause", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("default_clause", isToken: false),
            Repetition(element: RuleReference("case_clause", isToken: false))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 190),
    GrammarRule(
        name: "case_clause",
        body: Sequence(elements: [
          Literal("case"),
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          Repetition(element: RuleReference("statement", isToken: false))
        ]),
        lineNumber: 193),
    GrammarRule(
        name: "default_clause",
        body: Sequence(elements: [
          Literal("default"),
          RuleReference("COLON", isToken: true),
          Repetition(element: RuleReference("statement", isToken: false))
        ]),
        lineNumber: 195),
    GrammarRule(
        name: "labelled_statement",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 197),
    GrammarRule(
        name: "try_statement",
        body: Sequence(elements: [
          Literal("try"),
          RuleReference("block", isToken: false),
          Group(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("catch_clause", isToken: false),
              Optional(element: RuleReference("finally_clause", isToken: false))
            ]),
            RuleReference("finally_clause", isToken: false)
          ]))
        ]),
        lineNumber: 199),
    GrammarRule(
        name: "catch_clause",
        body: Sequence(elements: [
          Literal("catch"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("RPAREN", isToken: true),
          RuleReference("block", isToken: false)
        ]),
        lineNumber: 201),
    GrammarRule(
        name: "finally_clause",
        body: Sequence(elements: [
          Literal("finally"),
          RuleReference("block", isToken: false)
        ]),
        lineNumber: 203),
    GrammarRule(
        name: "throw_statement",
        body: Sequence(elements: [
          Literal("throw"),
          RuleReference("expression", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 205),
    GrammarRule(
        name: "debugger_statement",
        body: Sequence(elements: [
          Literal("debugger"),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 207),
    GrammarRule(
        name: "expression",
        body: Sequence(elements: [
          RuleReference("assignment_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("assignment_expression", isToken: false)
          ]))
        ]),
        lineNumber: 229),
    GrammarRule(
        name: "assignment_expression",
        body: Alternation(choices: [
          RuleReference("ts_as_expression", isToken: false),
          RuleReference("ts_angle_bracket_assertion", isToken: false),
          RuleReference("conditional_expression", isToken: false),
          Sequence(elements: [
            RuleReference("left_hand_side_expression", isToken: false),
            RuleReference("assignment_operator", isToken: false),
            RuleReference("assignment_expression", isToken: false)
          ])
        ]),
        lineNumber: 241),
    GrammarRule(
        name: "assignment_operator",
        body: Alternation(choices: [
          RuleReference("EQUALS", isToken: true),
          RuleReference("PLUS_EQUALS", isToken: true),
          RuleReference("MINUS_EQUALS", isToken: true),
          RuleReference("STAR_EQUALS", isToken: true),
          RuleReference("SLASH_EQUALS", isToken: true),
          RuleReference("PERCENT_EQUALS", isToken: true),
          RuleReference("AMPERSAND_EQUALS", isToken: true),
          RuleReference("PIPE_EQUALS", isToken: true),
          RuleReference("CARET_EQUALS", isToken: true),
          RuleReference("LEFT_SHIFT_EQUALS", isToken: true),
          RuleReference("RIGHT_SHIFT_EQUALS", isToken: true),
          RuleReference("UNSIGNED_RIGHT_SHIFT_EQUALS", isToken: true)
        ]),
        lineNumber: 246),
    GrammarRule(
        name: "ts_as_expression",
        body: Sequence(elements: [
          RuleReference("ts_non_null_expression", isToken: false),
          Literal("as"),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 255),
    GrammarRule(
        name: "ts_non_null_expression",
        body: Sequence(elements: [
          RuleReference("left_hand_side_expression", isToken: false),
          RuleReference("BANG", isToken: true)
        ]),
        lineNumber: 260),
    GrammarRule(
        name: "ts_angle_bracket_assertion",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("assignment_expression", isToken: false)
        ]),
        lineNumber: 264),
    GrammarRule(
        name: "conditional_expression",
        body: Sequence(elements: [
          RuleReference("logical_or_expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("QUESTION", isToken: true),
            RuleReference("assignment_expression", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("assignment_expression", isToken: false)
          ]))
        ]),
        lineNumber: 266),
    GrammarRule(
        name: "logical_or_expression",
        body: Sequence(elements: [
          RuleReference("logical_and_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("OR_OR", isToken: true),
            RuleReference("logical_and_expression", isToken: false)
          ]))
        ]),
        lineNumber: 269),
    GrammarRule(
        name: "logical_and_expression",
        body: Sequence(elements: [
          RuleReference("bitwise_or_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("AND_AND", isToken: true),
            RuleReference("bitwise_or_expression", isToken: false)
          ]))
        ]),
        lineNumber: 271),
    GrammarRule(
        name: "bitwise_or_expression",
        body: Sequence(elements: [
          RuleReference("bitwise_xor_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("PIPE", isToken: true),
            RuleReference("bitwise_xor_expression", isToken: false)
          ]))
        ]),
        lineNumber: 273),
    GrammarRule(
        name: "bitwise_xor_expression",
        body: Sequence(elements: [
          RuleReference("bitwise_and_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("CARET", isToken: true),
            RuleReference("bitwise_and_expression", isToken: false)
          ]))
        ]),
        lineNumber: 275),
    GrammarRule(
        name: "bitwise_and_expression",
        body: Sequence(elements: [
          RuleReference("equality_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("AMPERSAND", isToken: true),
            RuleReference("equality_expression", isToken: false)
          ]))
        ]),
        lineNumber: 277),
    GrammarRule(
        name: "equality_expression",
        body: Sequence(elements: [
          RuleReference("relational_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STRICT_EQUALS", isToken: true),
              RuleReference("STRICT_NOT_EQUALS", isToken: true),
              RuleReference("EQUALS_EQUALS", isToken: true),
              RuleReference("NOT_EQUALS", isToken: true)
            ])),
            RuleReference("relational_expression", isToken: false)
          ]))
        ]),
        lineNumber: 279),
    GrammarRule(
        name: "relational_expression",
        body: Sequence(elements: [
          RuleReference("shift_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("LESS_THAN", isToken: true),
              RuleReference("GREATER_THAN", isToken: true),
              RuleReference("LESS_EQUALS", isToken: true),
              RuleReference("GREATER_EQUALS", isToken: true),
              Literal("instanceof"),
              Literal("in")
            ])),
            RuleReference("shift_expression", isToken: false)
          ]))
        ]),
        lineNumber: 283),
    GrammarRule(
        name: "shift_expression",
        body: Sequence(elements: [
          RuleReference("additive_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("LEFT_SHIFT", isToken: true),
              RuleReference("RIGHT_SHIFT", isToken: true),
              RuleReference("UNSIGNED_RIGHT_SHIFT", isToken: true)
            ])),
            RuleReference("additive_expression", isToken: false)
          ]))
        ]),
        lineNumber: 287),
    GrammarRule(
        name: "additive_expression",
        body: Sequence(elements: [
          RuleReference("multiplicative_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("multiplicative_expression", isToken: false)
          ]))
        ]),
        lineNumber: 290),
    GrammarRule(
        name: "multiplicative_expression",
        body: Sequence(elements: [
          RuleReference("unary_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true),
              RuleReference("PERCENT", isToken: true)
            ])),
            RuleReference("unary_expression", isToken: false)
          ]))
        ]),
        lineNumber: 293),
    GrammarRule(
        name: "unary_expression",
        body: Alternation(choices: [
          RuleReference("postfix_expression", isToken: false),
          Sequence(elements: [
            Literal("delete"),
            RuleReference("unary_expression", isToken: false)
          ]),
          Sequence(elements: [
            Literal("void"),
            RuleReference("unary_expression", isToken: false)
          ]),
          Sequence(elements: [
            Literal("typeof"),
            RuleReference("unary_expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("PLUS_PLUS", isToken: true),
            RuleReference("unary_expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("MINUS_MINUS", isToken: true),
            RuleReference("unary_expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("PLUS", isToken: true),
            RuleReference("unary_expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("MINUS", isToken: true),
            RuleReference("unary_expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("TILDE", isToken: true),
            RuleReference("unary_expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("BANG", isToken: true),
            RuleReference("unary_expression", isToken: false)
          ])
        ]),
        lineNumber: 296),
    GrammarRule(
        name: "postfix_expression",
        body: Sequence(elements: [
          RuleReference("left_hand_side_expression", isToken: false),
          Optional(
              element: Alternation(choices: [
            RuleReference("PLUS_PLUS", isToken: true),
            RuleReference("MINUS_MINUS", isToken: true)
          ]))
        ]),
        lineNumber: 307),
    GrammarRule(
        name: "left_hand_side_expression",
        body: Alternation(choices: [
          RuleReference("call_expression", isToken: false),
          RuleReference("new_expression", isToken: false)
        ]),
        lineNumber: 309),
    GrammarRule(
        name: "call_expression",
        body: Sequence(elements: [
          RuleReference("member_expression", isToken: false),
          RuleReference("arguments", isToken: false),
          Repetition(
              element: Alternation(choices: [
            RuleReference("arguments", isToken: false),
            Sequence(elements: [
              RuleReference("DOT", isToken: true),
              RuleReference("NAME", isToken: true)
            ]),
            Sequence(elements: [
              RuleReference("LBRACKET", isToken: true),
              RuleReference("expression", isToken: false),
              RuleReference("RBRACKET", isToken: true)
            ])
          ]))
        ]),
        lineNumber: 311),
    GrammarRule(
        name: "new_expression",
        body: Alternation(choices: [
          RuleReference("member_expression", isToken: false),
          Sequence(elements: [
            Literal("new"),
            RuleReference("new_expression", isToken: false)
          ])
        ]),
        lineNumber: 314),
    GrammarRule(
        name: "member_expression",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("primary_expression", isToken: false),
            Repetition(
                element: Alternation(choices: [
              Sequence(elements: [
                RuleReference("DOT", isToken: true),
                RuleReference("NAME", isToken: true)
              ]),
              Sequence(elements: [
                RuleReference("LBRACKET", isToken: true),
                RuleReference("expression", isToken: false),
                RuleReference("RBRACKET", isToken: true)
              ])
            ]))
          ]),
          Sequence(elements: [
            Literal("new"),
            RuleReference("member_expression", isToken: false),
            RuleReference("arguments", isToken: false)
          ])
        ]),
        lineNumber: 317),
    GrammarRule(
        name: "arguments",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("argument_list", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 320),
    GrammarRule(
        name: "argument_list",
        body: Sequence(elements: [
          RuleReference("assignment_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("assignment_expression", isToken: false)
          ]))
        ]),
        lineNumber: 322),
    GrammarRule(
        name: "primary_expression",
        body: Alternation(choices: [
          Literal("this"),
          RuleReference("NAME", isToken: true),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("REGEX", isToken: true),
          Literal("true"),
          Literal("false"),
          Literal("null"),
          RuleReference("array_literal", isToken: false),
          RuleReference("object_literal", isToken: false),
          RuleReference("function_expression", isToken: false),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("expression", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 324),
    GrammarRule(
        name: "array_literal",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(element: RuleReference("element_list", isToken: false)),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 337),
    GrammarRule(
        name: "element_list",
        body: Sequence(elements: [
          Optional(
              element: RuleReference("assignment_expression", isToken: false)),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            Optional(
                element: RuleReference("assignment_expression", isToken: false))
          ]))
        ]),
        lineNumber: 339),
    GrammarRule(
        name: "object_literal",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("property_assignment", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("property_assignment", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 344),
    GrammarRule(
        name: "property_assignment",
        body: Alternation(choices: [
          RuleReference("getter_property", isToken: false),
          RuleReference("setter_property", isToken: false),
          Sequence(elements: [
            RuleReference("property_name", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("assignment_expression", isToken: false)
          ])
        ]),
        lineNumber: 346),
    GrammarRule(
        name: "getter_property",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true),
          RuleReference("LBRACE", isToken: true),
          RuleReference("function_body", isToken: false),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 351),
    GrammarRule(
        name: "setter_property",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("LPAREN", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("RPAREN", isToken: true),
          RuleReference("LBRACE", isToken: true),
          RuleReference("function_body", isToken: false),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 354),
    GrammarRule(
        name: "property_name",
        body: Alternation(choices: [
          RuleReference("NAME", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("NUMBER", isToken: true)
        ]),
        lineNumber: 356),
    GrammarRule(
        name: "function_expression",
        body: Sequence(elements: [
          Literal("function"),
          Optional(element: RuleReference("NAME", isToken: true)),
          Optional(element: RuleReference("type_parameters", isToken: false)),
          RuleReference("LPAREN", isToken: true),
          Optional(
              element: RuleReference("typed_parameter_list", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("LBRACE", isToken: true),
          RuleReference("function_body", isToken: false),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 361),
    GrammarRule(
        name: "binding_pattern",
        body: Alternation(choices: [
          RuleReference("object_binding_pattern", isToken: false),
          RuleReference("array_binding_pattern", isToken: false)
        ]),
        lineNumber: 372),
    GrammarRule(
        name: "object_binding_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("binding_property", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("binding_property", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 374),
    GrammarRule(
        name: "binding_property",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("property_name", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 376),
    GrammarRule(
        name: "array_binding_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("binding_element", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("binding_element", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 379),
    GrammarRule(
        name: "binding_element",
        body: Alternation(choices: [
          RuleReference("NAME", isToken: true),
          RuleReference("binding_pattern", isToken: false)
        ]),
        lineNumber: 381),
    GrammarRule(
        name: "type_annotation",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 410),
    GrammarRule(
        name: "type_expression",
        body: RuleReference("union_type", isToken: false),
        lineNumber: 428),
    GrammarRule(
        name: "union_type",
        body: Sequence(elements: [
          RuleReference("intersection_type", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("PIPE", isToken: true),
            RuleReference("intersection_type", isToken: false)
          ]))
        ]),
        lineNumber: 436),
    GrammarRule(
        name: "intersection_type",
        body: Sequence(elements: [
          RuleReference("array_type", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("AMPERSAND", isToken: true),
            RuleReference("array_type", isToken: false)
          ]))
        ]),
        lineNumber: 444),
    GrammarRule(
        name: "array_type",
        body: Sequence(elements: [
          RuleReference("primary_type", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("LBRACKET", isToken: true),
            RuleReference("RBRACKET", isToken: true)
          ]))
        ]),
        lineNumber: 451),
    GrammarRule(
        name: "primary_type",
        body: Alternation(choices: [
          RuleReference("predefined_type", isToken: false),
          RuleReference("type_reference", isToken: false),
          RuleReference("literal_type", isToken: false),
          RuleReference("object_type", isToken: false),
          RuleReference("tuple_type", isToken: false),
          RuleReference("function_type", isToken: false),
          RuleReference("constructor_type", isToken: false),
          Sequence(elements: [
            Literal("typeof"),
            RuleReference("left_hand_side_expression", isToken: false)
          ]),
          Sequence(elements: [
            Literal("keyof"),
            RuleReference("type_expression", isToken: false)
          ]),
          Sequence(elements: [Literal("unique"), Literal("symbol")]),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("type_expression", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 456),
    GrammarRule(
        name: "predefined_type",
        body: Alternation(choices: [
          Literal("any"),
          Literal("string"),
          Literal("number"),
          Literal("boolean"),
          Literal("void"),
          Literal("object"),
          Literal("symbol"),
          Literal("undefined"),
          Literal("null")
        ]),
        lineNumber: 484),
    GrammarRule(
        name: "literal_type",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          Literal("true"),
          Literal("false")
        ]),
        lineNumber: 498),
    GrammarRule(
        name: "type_reference",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ])),
          Optional(element: RuleReference("type_arguments", isToken: false))
        ]),
        lineNumber: 510),
    GrammarRule(
        name: "type_arguments",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_argument_list", isToken: false),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 520),
    GrammarRule(
        name: "type_argument_list",
        body: Sequence(elements: [
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ]))
        ]),
        lineNumber: 521),
    GrammarRule(
        name: "type_parameters",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_parameter_list", isToken: false),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 538),
    GrammarRule(
        name: "type_parameter_list",
        body: Sequence(elements: [
          RuleReference("type_parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_parameter", isToken: false)
          ]))
        ]),
        lineNumber: 539),
    GrammarRule(
        name: "type_parameter",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("extends"),
            RuleReference("type_expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            RuleReference("type_expression", isToken: false)
          ]))
        ]),
        lineNumber: 540),
    GrammarRule(
        name: "object_type",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Repetition(
              element: RuleReference("type_member_semicolon", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 557),
    GrammarRule(
        name: "type_member_semicolon",
        body: Sequence(elements: [
          RuleReference("type_member", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 558),
    GrammarRule(
        name: "type_member",
        body: Alternation(choices: [
          RuleReference("construct_signature", isToken: false),
          RuleReference("call_signature", isToken: false),
          RuleReference("index_signature", isToken: false),
          RuleReference("method_signature", isToken: false),
          RuleReference("property_signature", isToken: false)
        ]),
        lineNumber: 559),
    GrammarRule(
        name: "property_signature",
        body: Sequence(elements: [
          Optional(element: Literal("readonly")),
          RuleReference("property_name", isToken: false),
          Optional(element: RuleReference("QUESTION", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ]))
        ]),
        lineNumber: 567),
    GrammarRule(
        name: "index_signature",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("RBRACKET", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 572),
    GrammarRule(
        name: "method_signature",
        body: Sequence(elements: [
          RuleReference("property_name", isToken: false),
          Optional(element: RuleReference("QUESTION", isToken: true)),
          Optional(element: RuleReference("type_parameters", isToken: false)),
          RuleReference("LPAREN", isToken: true),
          Optional(
              element: RuleReference("typed_parameter_list", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ]))
        ]),
        lineNumber: 576),
    GrammarRule(
        name: "call_signature",
        body: Sequence(elements: [
          Optional(element: RuleReference("type_parameters", isToken: false)),
          RuleReference("LPAREN", isToken: true),
          Optional(
              element: RuleReference("typed_parameter_list", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ]))
        ]),
        lineNumber: 580),
    GrammarRule(
        name: "construct_signature",
        body: Sequence(elements: [
          Literal("new"),
          Optional(element: RuleReference("type_parameters", isToken: false)),
          RuleReference("LPAREN", isToken: true),
          Optional(
              element: RuleReference("typed_parameter_list", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ]))
        ]),
        lineNumber: 584),
    GrammarRule(
        name: "tuple_type",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: RuleReference("tuple_element_list", isToken: false)),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 598),
    GrammarRule(
        name: "tuple_element_list",
        body: Sequence(elements: [
          RuleReference("tuple_element", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("tuple_element", isToken: false)
          ]))
        ]),
        lineNumber: 599),
    GrammarRule(
        name: "tuple_element",
        body: Sequence(elements: [
          Optional(element: RuleReference("ELLIPSIS", isToken: true)),
          RuleReference("type_expression", isToken: false),
          Optional(element: RuleReference("QUESTION", isToken: true))
        ]),
        lineNumber: 600),
    GrammarRule(
        name: "function_type",
        body: Sequence(elements: [
          Optional(element: RuleReference("type_parameters", isToken: false)),
          RuleReference("LPAREN", isToken: true),
          Optional(
              element: RuleReference("typed_parameter_list", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          RuleReference("ARROW", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 618),
    GrammarRule(
        name: "constructor_type",
        body: Sequence(elements: [
          Literal("new"),
          Optional(element: RuleReference("type_parameters", isToken: false)),
          RuleReference("LPAREN", isToken: true),
          Optional(
              element: RuleReference("typed_parameter_list", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          RuleReference("ARROW", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 622),
    GrammarRule(
        name: "interface_declaration",
        body: Sequence(elements: [
          Literal("interface"),
          RuleReference("NAME", isToken: true),
          Optional(element: RuleReference("type_parameters", isToken: false)),
          Optional(
              element: RuleReference("interface_heritage", isToken: false)),
          RuleReference("object_type", isToken: false)
        ]),
        lineNumber: 654),
    GrammarRule(
        name: "interface_heritage",
        body: Sequence(elements: [
          Literal("extends"),
          RuleReference("type_reference", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_reference", isToken: false)
          ]))
        ]),
        lineNumber: 655),
    GrammarRule(
        name: "type_alias_declaration",
        body: Sequence(elements: [
          Literal("type"),
          RuleReference("NAME", isToken: true),
          Optional(element: RuleReference("type_parameters", isToken: false)),
          RuleReference("EQUALS", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 671),
    GrammarRule(
        name: "enum_declaration",
        body: Sequence(elements: [
          Optional(element: Literal("const")),
          Literal("enum"),
          RuleReference("NAME", isToken: true),
          RuleReference("LBRACE", isToken: true),
          Optional(element: RuleReference("enum_body", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 695),
    GrammarRule(
        name: "enum_body",
        body: Sequence(elements: [
          RuleReference("enum_member", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("enum_member", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 696),
    GrammarRule(
        name: "enum_member",
        body: Sequence(elements: [
          RuleReference("property_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            RuleReference("assignment_expression", isToken: false)
          ]))
        ]),
        lineNumber: 697),
    GrammarRule(
        name: "namespace_declaration",
        body: Sequence(elements: [
          Group(
              element: Alternation(
                  choices: [Literal("namespace"), Literal("module")])),
          RuleReference("qualified_name", isToken: false),
          RuleReference("LBRACE", isToken: true),
          Repetition(
              element: RuleReference("namespace_element", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 719),
    GrammarRule(
        name: "qualified_name",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 720),
    GrammarRule(
        name: "namespace_element",
        body: Alternation(choices: [
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("interface_declaration", isToken: false),
          RuleReference("type_alias_declaration", isToken: false),
          RuleReference("ts_class_declaration", isToken: false),
          RuleReference("function_declaration", isToken: false),
          RuleReference("enum_declaration", isToken: false),
          RuleReference("variable_statement", isToken: false),
          RuleReference("export_assignment", isToken: false),
          RuleReference("export_namespace_element", isToken: false),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 724),
    GrammarRule(
        name: "export_assignment",
        body: Sequence(elements: [
          Literal("export"),
          RuleReference("EQUALS", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 737),
    GrammarRule(
        name: "export_namespace_element",
        body: Sequence(elements: [
          Literal("export"),
          Group(
              element: Alternation(choices: [
            RuleReference("namespace_declaration", isToken: false),
            RuleReference("interface_declaration", isToken: false),
            RuleReference("type_alias_declaration", isToken: false),
            RuleReference("ts_class_declaration", isToken: false),
            RuleReference("function_declaration", isToken: false),
            RuleReference("enum_declaration", isToken: false),
            RuleReference("variable_statement", isToken: false)
          ]))
        ]),
        lineNumber: 740),
    GrammarRule(
        name: "ambient_declaration",
        body: Sequence(elements: [
          Literal("declare"),
          RuleReference("ambient_declaration_body", isToken: false)
        ]),
        lineNumber: 769),
    GrammarRule(
        name: "ambient_declaration_body",
        body: Alternation(choices: [
          RuleReference("variable_statement", isToken: false),
          RuleReference("ambient_function_declaration", isToken: false),
          RuleReference("ts_class_declaration", isToken: false),
          RuleReference("interface_declaration", isToken: false),
          RuleReference("type_alias_declaration", isToken: false),
          RuleReference("enum_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("ambient_module_declaration", isToken: false)
        ]),
        lineNumber: 770),
    GrammarRule(
        name: "ambient_function_declaration",
        body: Sequence(elements: [
          Literal("function"),
          RuleReference("NAME", isToken: true),
          Optional(element: RuleReference("type_parameters", isToken: false)),
          RuleReference("LPAREN", isToken: true),
          Optional(
              element: RuleReference("typed_parameter_list", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 783),
    GrammarRule(
        name: "ambient_module_declaration",
        body: Sequence(elements: [
          Literal("module"),
          RuleReference("STRING", isToken: true),
          RuleReference("LBRACE", isToken: true),
          Repetition(
              element: RuleReference("namespace_element", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 787),
    GrammarRule(
        name: "ts_class_declaration",
        body: Sequence(elements: [
          Optional(element: Literal("abstract")),
          Literal("class"),
          RuleReference("NAME", isToken: true),
          Optional(element: RuleReference("type_parameters", isToken: false)),
          Optional(element: RuleReference("ts_class_heritage", isToken: false)),
          RuleReference("ts_class_body", isToken: false)
        ]),
        lineNumber: 829),
    GrammarRule(
        name: "ts_class_heritage",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            Literal("extends"),
            RuleReference("type_reference", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("implements"),
            RuleReference("type_reference", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("type_reference", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 834),
    GrammarRule(
        name: "ts_class_body",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Repetition(
              element: RuleReference("ts_class_element", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 836),
    GrammarRule(
        name: "ts_class_element",
        body: Alternation(choices: [
          RuleReference("ts_method_definition", isToken: false),
          RuleReference("ts_property_declaration", isToken: false),
          Sequence(elements: [
            RuleReference("index_signature", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 838),
    GrammarRule(
        name: "ts_method_definition",
        body: Sequence(elements: [
          Optional(
              element: RuleReference("accessibility_modifier", isToken: false)),
          Optional(element: Literal("abstract")),
          Optional(element: Literal("static")),
          Optional(element: Literal("readonly")),
          RuleReference("ts_method_definition_body", isToken: false)
        ]),
        lineNumber: 844),
    GrammarRule(
        name: "accessibility_modifier",
        body: Alternation(choices: [
          Literal("public"),
          Literal("private"),
          Literal("protected")
        ]),
        lineNumber: 850),
    GrammarRule(
        name: "ts_method_definition_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("property_name", isToken: false),
            Optional(element: RuleReference("QUESTION", isToken: true)),
            Optional(element: RuleReference("type_parameters", isToken: false)),
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("typed_parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              RuleReference("type_expression", isToken: false)
            ])),
            RuleReference("LBRACE", isToken: true),
            RuleReference("function_body", isToken: false),
            RuleReference("RBRACE", isToken: true)
          ]),
          Sequence(elements: [
            Literal("get"),
            RuleReference("property_name", isToken: false),
            RuleReference("LPAREN", isToken: true),
            RuleReference("RPAREN", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              RuleReference("type_expression", isToken: false)
            ])),
            RuleReference("LBRACE", isToken: true),
            RuleReference("function_body", isToken: false),
            RuleReference("RBRACE", isToken: true)
          ]),
          Sequence(elements: [
            Literal("set"),
            RuleReference("property_name", isToken: false),
            RuleReference("LPAREN", isToken: true),
            RuleReference("typed_parameter", isToken: false),
            RuleReference("RPAREN", isToken: true),
            RuleReference("LBRACE", isToken: true),
            RuleReference("function_body", isToken: false),
            RuleReference("RBRACE", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("property_name", isToken: false),
            Optional(element: RuleReference("QUESTION", isToken: true)),
            Optional(element: RuleReference("type_parameters", isToken: false)),
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("typed_parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              RuleReference("type_expression", isToken: false)
            ])),
            RuleReference("LBRACE", isToken: true),
            RuleReference("function_body", isToken: false),
            RuleReference("RBRACE", isToken: true)
          ])
        ]),
        lineNumber: 853),
    GrammarRule(
        name: "ts_property_declaration",
        body: Sequence(elements: [
          Optional(
              element: RuleReference("accessibility_modifier", isToken: false)),
          Optional(element: Literal("static")),
          Optional(element: Literal("abstract")),
          Optional(element: Literal("readonly")),
          RuleReference("property_name", isToken: false),
          Optional(element: RuleReference("QUESTION", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            RuleReference("assignment_expression", isToken: false)
          ])),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 860),
    GrammarRule(
        name: "type_predicate",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Literal("is"),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 883),
  ],
);
