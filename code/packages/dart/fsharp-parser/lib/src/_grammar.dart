// AUTO-GENERATED FILE - DO NOT EDIT
// Sources: code/grammars/fsharp/fsharp*.grammar (one grammar per supported F# version)
// Regenerate with: grammar-tools compile-grammar <source.grammar> (per version)
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammarV1_0 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "compilation_unit",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("decorated_declaration", isToken: false)
        ])),
        lineNumber: 9),
    GrammarRule(
        name: "decorated_declaration",
        body: Sequence(elements: [
          Repetition(
              element: Sequence(elements: [
            RuleReference("attribute_section", isToken: false),
            RuleReference("NEWLINE", isToken: true)
          ])),
          RuleReference("declaration_body", isToken: false)
        ]),
        lineNumber: 11),
    GrammarRule(
        name: "declaration_body",
        body: Alternation(choices: [
          RuleReference("module_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("open_declaration", isToken: false),
          RuleReference("let_binding", isToken: false),
          RuleReference("use_binding", isToken: false),
          RuleReference("type_declaration", isToken: false),
          RuleReference("member_declaration", isToken: false),
          RuleReference("do_binding", isToken: false),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 13),
    GrammarRule(
        name: "module_declaration",
        body: Sequence(elements: [
          Literal("module"),
          Repetition(element: RuleReference("module_modifier", isToken: false)),
          Optional(element: Literal("rec")),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ]))
        ]),
        lineNumber: 23),
    GrammarRule(
        name: "module_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 25),
    GrammarRule(
        name: "namespace_declaration",
        body: Sequence(elements: [
          Literal("namespace"),
          Repetition(
              element: RuleReference("namespace_modifier", isToken: false)),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 28),
    GrammarRule(
        name: "namespace_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 30),
    GrammarRule(
        name: "open_declaration",
        body: Sequence(elements: [
          Literal("open"),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 33),
    GrammarRule(
        name: "let_binding",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 35),
    GrammarRule(
        name: "use_binding",
        body: Sequence(elements: [
          Literal("use"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 37),
    GrammarRule(
        name: "binding_modifier",
        body: Alternation(choices: [Literal("inline"), Literal("mutable")]),
        lineNumber: 39),
    GrammarRule(
        name: "binding_clause",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          RuleReference("EQUALS", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 42),
    GrammarRule(
        name: "do_binding",
        body: Sequence(elements: [
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 44),
    GrammarRule(
        name: "member_declaration",
        body: Sequence(elements: [
          Optional(element: RuleReference("member_modifier", isToken: false)),
          Literal("member"),
          RuleReference("qualified_name", isToken: false),
          Optional(element: RuleReference("parameter_list", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 46),
    GrammarRule(
        name: "member_modifier",
        body: Alternation(choices: [
          Literal("static"),
          Literal("override"),
          Literal("default"),
          Literal("abstract"),
          Literal("new")
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "type_declaration",
        body: Sequence(elements: [
          Literal("type"),
          Repetition(element: RuleReference("type_modifier", isToken: false)),
          RuleReference("NAME", isToken: true),
          Optional(
              element: RuleReference("generic_parameters", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ])),
          RuleReference("type_definition", isToken: false)
        ]),
        lineNumber: 54),
    GrammarRule(
        name: "type_modifier",
        body: Alternation(choices: [
          Literal("private"),
          Literal("public"),
          Literal("internal")
        ]),
        lineNumber: 56),
    GrammarRule(
        name: "generic_parameters",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_parameter", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 60),
    GrammarRule(
        name: "type_parameter",
        body: Alternation(choices: [
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 62),
    GrammarRule(
        name: "type_definition",
        body: Alternation(choices: [
          RuleReference("class_type", isToken: false),
          RuleReference("interface_type", isToken: false),
          RuleReference("struct_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("union_type", isToken: false),
          RuleReference("alias_type", isToken: false)
        ]),
        lineNumber: 65),
    GrammarRule(
        name: "class_type",
        body: Sequence(elements: [
          Literal("class"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 72),
    GrammarRule(
        name: "interface_type",
        body: Sequence(elements: [
          Literal("interface"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 74),
    GrammarRule(
        name: "struct_type",
        body: Sequence(elements: [
          Literal("struct"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "record_type",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_declaration", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_declaration", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 78),
    GrammarRule(
        name: "field_declaration",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 80),
    GrammarRule(
        name: "union_type",
        body: Sequence(elements: [
          RuleReference("union_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("union_case", isToken: false)
          ]))
        ]),
        lineNumber: 82),
    GrammarRule(
        name: "union_case",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("of"),
            RuleReference("type_expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("STAR", isToken: true),
              RuleReference("type_expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 84),
    GrammarRule(
        name: "alias_type",
        body: RuleReference("type_expression", isToken: false),
        lineNumber: 86),
    GrammarRule(
        name: "field_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 88),
    GrammarRule(
        name: "case_separator",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            Optional(element: RuleReference("PIPE", isToken: true))
          ]),
          RuleReference("PIPE", isToken: true)
        ]),
        lineNumber: 91),
    GrammarRule(
        name: "attribute_section",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("attribute", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("attribute", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 94),
    GrammarRule(
        name: "attribute",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("attribute_target", isToken: false),
            RuleReference("COLON", isToken: true)
          ])),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("attribute_arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ]))
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "attribute_target",
        body: Alternation(choices: [
          Literal("assembly"),
          Literal("field"),
          Literal("method"),
          Literal("module"),
          Literal("param"),
          Literal("property"),
          Literal("return"),
          Literal("type"),
          Literal("event")
        ]),
        lineNumber: 98),
    GrammarRule(
        name: "attribute_arguments",
        body: Sequence(elements: [
          RuleReference("attribute_argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("attribute_argument", isToken: false)
          ]))
        ]),
        lineNumber: 108),
    GrammarRule(
        name: "attribute_argument",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true)
          ])),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 110),
    GrammarRule(
        name: "parameter_list",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false))
        ]),
        lineNumber: 112),
    GrammarRule(
        name: "type_annotation",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 114),
    GrammarRule(
        name: "type_expression",
        body: RuleReference("function_type", isToken: false),
        lineNumber: 116),
    GrammarRule(
        name: "function_type",
        body: Sequence(elements: [
          RuleReference("type_product", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("function_type", isToken: false)
          ]))
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "type_product",
        body: Sequence(elements: [
          RuleReference("type_application", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("type_application", isToken: false)
          ]))
        ]),
        lineNumber: 120),
    GrammarRule(
        name: "type_application",
        body: Sequence(elements: [
          RuleReference("type_atom", isToken: false),
          Repetition(element: RuleReference("type_atom", isToken: false))
        ]),
        lineNumber: 122),
    GrammarRule(
        name: "generic_type_arguments",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 124),
    GrammarRule(
        name: "type_atom",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("qualified_name", isToken: false),
            Optional(
                element:
                    RuleReference("generic_type_arguments", isToken: false))
          ]),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("tuple_type", isToken: false),
          RuleReference("unit_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("parenthesized_type", isToken: false)
        ]),
        lineNumber: 126),
    GrammarRule(
        name: "tuple_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 133),
    GrammarRule(
        name: "unit_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 135),
    GrammarRule(
        name: "parenthesized_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 137),
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
        lineNumber: 139),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("sequence_expression", isToken: false)
        ]),
        lineNumber: 141),
    GrammarRule(
        name: "sequence_expression",
        body: Sequence(elements: [
          RuleReference("infix_expression", isToken: false),
          Repetition(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("NEWLINE", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ]),
            Sequence(elements: [
              RuleReference("SEMICOLON", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 150),
    GrammarRule(
        name: "infix_expression",
        body: Sequence(elements: [
          RuleReference("application_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("infix_operator", isToken: false),
            RuleReference("application_expression", isToken: false)
          ]))
        ]),
        lineNumber: 152),
    GrammarRule(
        name: "infix_operator",
        body: Alternation(choices: [
          RuleReference("PIPE_RIGHT", isToken: true),
          RuleReference("PIPE_LEFT", isToken: true),
          RuleReference("COMPOSE_RIGHT", isToken: true),
          RuleReference("COMPOSE_LEFT", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("COLON_EQUALS", isToken: true),
          RuleReference("DOUBLE_COLON", isToken: true),
          RuleReference("LARROW", isToken: true),
          RuleReference("ARROW", isToken: true),
          RuleReference("DOT_DOT", isToken: true),
          RuleReference("AND_AND", isToken: true),
          RuleReference("OR_OR", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("PERCENT", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          RuleReference("PIPE", isToken: true),
          RuleReference("CARET", isToken: true)
        ]),
        lineNumber: 154),
    GrammarRule(
        name: "application_expression",
        body: Sequence(elements: [
          RuleReference("prefix_expression", isToken: false),
          Repetition(
              element: RuleReference("prefix_expression", isToken: false))
        ]),
        lineNumber: 181),
    GrammarRule(
        name: "prefix_expression",
        body: Sequence(elements: [
          Optional(element: RuleReference("unary_operator", isToken: false)),
          RuleReference("atomic_expression", isToken: false)
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "unary_operator",
        body: Alternation(choices: [
          RuleReference("MINUS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("BANG", isToken: true)
        ]),
        lineNumber: 185),
    GrammarRule(
        name: "atomic_expression",
        body: Alternation(choices: [
          RuleReference("computation_expression", isToken: false),
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("unit_expression", isToken: false),
          RuleReference("tuple_expression", isToken: false),
          RuleReference("list_expression", isToken: false),
          RuleReference("array_expression", isToken: false),
          RuleReference("record_expression", isToken: false),
          RuleReference("parenthesized_expression", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 189),
    GrammarRule(
        name: "computation_expression",
        body: Sequence(elements: [
          RuleReference("qualified_name", isToken: false),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("expression", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 211),
    GrammarRule(
        name: "unit_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 213),
    GrammarRule(
        name: "tuple_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 215),
    GrammarRule(
        name: "list_expression",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          NegativeLookahead(element: RuleReference("LESS_THAN", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 217),
    GrammarRule(
        name: "array_expression",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 219),
    GrammarRule(
        name: "record_expression",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_assignment", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_assignment", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 221),
    GrammarRule(
        name: "field_assignment",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 223),
    GrammarRule(
        name: "parenthesized_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 225),
    GrammarRule(
        name: "element_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 227),
    GrammarRule(
        name: "if_expression",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          Literal("then"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 230),
    GrammarRule(
        name: "match_expression",
        body: Sequence(elements: [
          Literal("match"),
          RuleReference("expression", isToken: false),
          Literal("with"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 232),
    GrammarRule(
        name: "match_case",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("when"),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 234),
    GrammarRule(
        name: "fun_expression",
        body: Sequence(elements: [
          Literal("fun"),
          RuleReference("parameter_list", isToken: false),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 236),
    GrammarRule(
        name: "function_expression",
        body: Sequence(elements: [
          Literal("function"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 238),
    GrammarRule(
        name: "let_expression",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ])),
          Literal("in"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 240),
    GrammarRule(
        name: "for_expression",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("pattern", isToken: false),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false),
          Group(
              element:
                  Alternation(choices: [Literal("to"), Literal("downto")])),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 242),
    GrammarRule(
        name: "while_expression",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 244),
    GrammarRule(
        name: "pattern",
        body: Sequence(elements: [
          RuleReference("pattern_atom", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 246),
    GrammarRule(
        name: "pattern_atom",
        body: Alternation(choices: [
          RuleReference("wildcard_pattern", isToken: false),
          RuleReference("literal_pattern", isToken: false),
          RuleReference("tuple_pattern", isToken: false),
          RuleReference("list_pattern", isToken: false),
          RuleReference("array_pattern", isToken: false),
          RuleReference("record_pattern", isToken: false),
          RuleReference("unit_pattern", isToken: false),
          RuleReference("parenthesized_pattern", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 248),
    GrammarRule(
        name: "wildcard_pattern",
        body: RuleReference("UNDERSCORE", isToken: true),
        lineNumber: 260),
    GrammarRule(
        name: "literal_pattern",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 262),
    GrammarRule(
        name: "tuple_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("pattern", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 269),
    GrammarRule(
        name: "list_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 271),
    GrammarRule(
        name: "array_pattern",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 273),
    GrammarRule(
        name: "record_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 275),
    GrammarRule(
        name: "field_pattern",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("pattern", isToken: false)
        ]),
        lineNumber: 277),
    GrammarRule(
        name: "unit_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 279),
    GrammarRule(
        name: "parenthesized_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 281),
  ],
);

final parserGrammarV2_0 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "compilation_unit",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("decorated_declaration", isToken: false)
        ])),
        lineNumber: 9),
    GrammarRule(
        name: "decorated_declaration",
        body: Sequence(elements: [
          Repetition(
              element: Sequence(elements: [
            RuleReference("attribute_section", isToken: false),
            RuleReference("NEWLINE", isToken: true)
          ])),
          RuleReference("declaration_body", isToken: false)
        ]),
        lineNumber: 11),
    GrammarRule(
        name: "declaration_body",
        body: Alternation(choices: [
          RuleReference("module_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("open_declaration", isToken: false),
          RuleReference("let_binding", isToken: false),
          RuleReference("use_binding", isToken: false),
          RuleReference("type_declaration", isToken: false),
          RuleReference("member_declaration", isToken: false),
          RuleReference("do_binding", isToken: false),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 13),
    GrammarRule(
        name: "module_declaration",
        body: Sequence(elements: [
          Literal("module"),
          Repetition(element: RuleReference("module_modifier", isToken: false)),
          Optional(element: Literal("rec")),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ]))
        ]),
        lineNumber: 23),
    GrammarRule(
        name: "module_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 25),
    GrammarRule(
        name: "namespace_declaration",
        body: Sequence(elements: [
          Literal("namespace"),
          Repetition(
              element: RuleReference("namespace_modifier", isToken: false)),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 28),
    GrammarRule(
        name: "namespace_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 30),
    GrammarRule(
        name: "open_declaration",
        body: Sequence(elements: [
          Literal("open"),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 33),
    GrammarRule(
        name: "let_binding",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 35),
    GrammarRule(
        name: "use_binding",
        body: Sequence(elements: [
          Literal("use"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 37),
    GrammarRule(
        name: "binding_modifier",
        body: Alternation(choices: [Literal("inline"), Literal("mutable")]),
        lineNumber: 39),
    GrammarRule(
        name: "binding_clause",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          RuleReference("EQUALS", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 42),
    GrammarRule(
        name: "do_binding",
        body: Sequence(elements: [
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 44),
    GrammarRule(
        name: "member_declaration",
        body: Sequence(elements: [
          Optional(element: RuleReference("member_modifier", isToken: false)),
          Literal("member"),
          RuleReference("qualified_name", isToken: false),
          Optional(element: RuleReference("parameter_list", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 46),
    GrammarRule(
        name: "member_modifier",
        body: Alternation(choices: [
          Literal("static"),
          Literal("override"),
          Literal("default"),
          Literal("abstract"),
          Literal("new")
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "type_declaration",
        body: Sequence(elements: [
          Literal("type"),
          Repetition(element: RuleReference("type_modifier", isToken: false)),
          RuleReference("NAME", isToken: true),
          Optional(
              element: RuleReference("generic_parameters", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ])),
          RuleReference("type_definition", isToken: false)
        ]),
        lineNumber: 54),
    GrammarRule(
        name: "type_modifier",
        body: Alternation(choices: [
          Literal("private"),
          Literal("public"),
          Literal("internal")
        ]),
        lineNumber: 56),
    GrammarRule(
        name: "generic_parameters",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_parameter", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 60),
    GrammarRule(
        name: "type_parameter",
        body: Alternation(choices: [
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 62),
    GrammarRule(
        name: "type_definition",
        body: Alternation(choices: [
          RuleReference("class_type", isToken: false),
          RuleReference("interface_type", isToken: false),
          RuleReference("struct_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("union_type", isToken: false),
          RuleReference("alias_type", isToken: false)
        ]),
        lineNumber: 65),
    GrammarRule(
        name: "class_type",
        body: Sequence(elements: [
          Literal("class"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 72),
    GrammarRule(
        name: "interface_type",
        body: Sequence(elements: [
          Literal("interface"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 74),
    GrammarRule(
        name: "struct_type",
        body: Sequence(elements: [
          Literal("struct"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "record_type",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_declaration", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_declaration", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 78),
    GrammarRule(
        name: "field_declaration",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 80),
    GrammarRule(
        name: "union_type",
        body: Sequence(elements: [
          RuleReference("union_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("union_case", isToken: false)
          ]))
        ]),
        lineNumber: 82),
    GrammarRule(
        name: "union_case",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("of"),
            RuleReference("type_expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("STAR", isToken: true),
              RuleReference("type_expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 84),
    GrammarRule(
        name: "alias_type",
        body: RuleReference("type_expression", isToken: false),
        lineNumber: 86),
    GrammarRule(
        name: "field_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 88),
    GrammarRule(
        name: "case_separator",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            Optional(element: RuleReference("PIPE", isToken: true))
          ]),
          RuleReference("PIPE", isToken: true)
        ]),
        lineNumber: 91),
    GrammarRule(
        name: "attribute_section",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("attribute", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("attribute", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 94),
    GrammarRule(
        name: "attribute",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("attribute_target", isToken: false),
            RuleReference("COLON", isToken: true)
          ])),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("attribute_arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ]))
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "attribute_target",
        body: Alternation(choices: [
          Literal("assembly"),
          Literal("field"),
          Literal("method"),
          Literal("module"),
          Literal("param"),
          Literal("property"),
          Literal("return"),
          Literal("type"),
          Literal("event")
        ]),
        lineNumber: 98),
    GrammarRule(
        name: "attribute_arguments",
        body: Sequence(elements: [
          RuleReference("attribute_argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("attribute_argument", isToken: false)
          ]))
        ]),
        lineNumber: 108),
    GrammarRule(
        name: "attribute_argument",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true)
          ])),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 110),
    GrammarRule(
        name: "parameter_list",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false))
        ]),
        lineNumber: 112),
    GrammarRule(
        name: "type_annotation",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 114),
    GrammarRule(
        name: "type_expression",
        body: RuleReference("function_type", isToken: false),
        lineNumber: 116),
    GrammarRule(
        name: "function_type",
        body: Sequence(elements: [
          RuleReference("type_product", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("function_type", isToken: false)
          ]))
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "type_product",
        body: Sequence(elements: [
          RuleReference("type_application", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("type_application", isToken: false)
          ]))
        ]),
        lineNumber: 120),
    GrammarRule(
        name: "type_application",
        body: Sequence(elements: [
          RuleReference("type_atom", isToken: false),
          Repetition(element: RuleReference("type_atom", isToken: false))
        ]),
        lineNumber: 122),
    GrammarRule(
        name: "generic_type_arguments",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 124),
    GrammarRule(
        name: "type_atom",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("qualified_name", isToken: false),
            Optional(
                element:
                    RuleReference("generic_type_arguments", isToken: false))
          ]),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("tuple_type", isToken: false),
          RuleReference("unit_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("parenthesized_type", isToken: false)
        ]),
        lineNumber: 126),
    GrammarRule(
        name: "tuple_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 133),
    GrammarRule(
        name: "unit_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 135),
    GrammarRule(
        name: "parenthesized_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 137),
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
        lineNumber: 139),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("sequence_expression", isToken: false)
        ]),
        lineNumber: 141),
    GrammarRule(
        name: "sequence_expression",
        body: Sequence(elements: [
          RuleReference("infix_expression", isToken: false),
          Repetition(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("NEWLINE", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ]),
            Sequence(elements: [
              RuleReference("SEMICOLON", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 150),
    GrammarRule(
        name: "infix_expression",
        body: Sequence(elements: [
          RuleReference("application_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("infix_operator", isToken: false),
            RuleReference("application_expression", isToken: false)
          ]))
        ]),
        lineNumber: 152),
    GrammarRule(
        name: "infix_operator",
        body: Alternation(choices: [
          RuleReference("PIPE_RIGHT", isToken: true),
          RuleReference("PIPE_LEFT", isToken: true),
          RuleReference("COMPOSE_RIGHT", isToken: true),
          RuleReference("COMPOSE_LEFT", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("COLON_EQUALS", isToken: true),
          RuleReference("DOUBLE_COLON", isToken: true),
          RuleReference("LARROW", isToken: true),
          RuleReference("ARROW", isToken: true),
          RuleReference("DOT_DOT", isToken: true),
          RuleReference("AND_AND", isToken: true),
          RuleReference("OR_OR", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("PERCENT", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          RuleReference("PIPE", isToken: true),
          RuleReference("CARET", isToken: true)
        ]),
        lineNumber: 154),
    GrammarRule(
        name: "application_expression",
        body: Sequence(elements: [
          RuleReference("prefix_expression", isToken: false),
          Repetition(
              element: RuleReference("prefix_expression", isToken: false))
        ]),
        lineNumber: 181),
    GrammarRule(
        name: "prefix_expression",
        body: Sequence(elements: [
          Optional(element: RuleReference("unary_operator", isToken: false)),
          RuleReference("atomic_expression", isToken: false)
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "unary_operator",
        body: Alternation(choices: [
          RuleReference("MINUS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("BANG", isToken: true)
        ]),
        lineNumber: 185),
    GrammarRule(
        name: "atomic_expression",
        body: Alternation(choices: [
          RuleReference("computation_expression", isToken: false),
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("unit_expression", isToken: false),
          RuleReference("tuple_expression", isToken: false),
          RuleReference("list_expression", isToken: false),
          RuleReference("array_expression", isToken: false),
          RuleReference("record_expression", isToken: false),
          RuleReference("parenthesized_expression", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 189),
    GrammarRule(
        name: "computation_expression",
        body: Sequence(elements: [
          RuleReference("qualified_name", isToken: false),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("expression", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 211),
    GrammarRule(
        name: "unit_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 213),
    GrammarRule(
        name: "tuple_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 215),
    GrammarRule(
        name: "list_expression",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          NegativeLookahead(element: RuleReference("LESS_THAN", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 217),
    GrammarRule(
        name: "array_expression",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 219),
    GrammarRule(
        name: "record_expression",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_assignment", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_assignment", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 221),
    GrammarRule(
        name: "field_assignment",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 223),
    GrammarRule(
        name: "parenthesized_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 225),
    GrammarRule(
        name: "element_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 227),
    GrammarRule(
        name: "if_expression",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          Literal("then"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 230),
    GrammarRule(
        name: "match_expression",
        body: Sequence(elements: [
          Literal("match"),
          RuleReference("expression", isToken: false),
          Literal("with"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 232),
    GrammarRule(
        name: "match_case",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("when"),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 234),
    GrammarRule(
        name: "fun_expression",
        body: Sequence(elements: [
          Literal("fun"),
          RuleReference("parameter_list", isToken: false),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 236),
    GrammarRule(
        name: "function_expression",
        body: Sequence(elements: [
          Literal("function"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 238),
    GrammarRule(
        name: "let_expression",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ])),
          Literal("in"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 240),
    GrammarRule(
        name: "for_expression",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("pattern", isToken: false),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false),
          Group(
              element:
                  Alternation(choices: [Literal("to"), Literal("downto")])),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 242),
    GrammarRule(
        name: "while_expression",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 244),
    GrammarRule(
        name: "pattern",
        body: Sequence(elements: [
          RuleReference("pattern_atom", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 246),
    GrammarRule(
        name: "pattern_atom",
        body: Alternation(choices: [
          RuleReference("wildcard_pattern", isToken: false),
          RuleReference("literal_pattern", isToken: false),
          RuleReference("tuple_pattern", isToken: false),
          RuleReference("list_pattern", isToken: false),
          RuleReference("array_pattern", isToken: false),
          RuleReference("record_pattern", isToken: false),
          RuleReference("unit_pattern", isToken: false),
          RuleReference("parenthesized_pattern", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 248),
    GrammarRule(
        name: "wildcard_pattern",
        body: RuleReference("UNDERSCORE", isToken: true),
        lineNumber: 260),
    GrammarRule(
        name: "literal_pattern",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 262),
    GrammarRule(
        name: "tuple_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("pattern", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 269),
    GrammarRule(
        name: "list_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 271),
    GrammarRule(
        name: "array_pattern",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 273),
    GrammarRule(
        name: "record_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 275),
    GrammarRule(
        name: "field_pattern",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("pattern", isToken: false)
        ]),
        lineNumber: 277),
    GrammarRule(
        name: "unit_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 279),
    GrammarRule(
        name: "parenthesized_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 281),
  ],
);

final parserGrammarV3_0 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "compilation_unit",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("decorated_declaration", isToken: false)
        ])),
        lineNumber: 9),
    GrammarRule(
        name: "decorated_declaration",
        body: Sequence(elements: [
          Repetition(
              element: Sequence(elements: [
            RuleReference("attribute_section", isToken: false),
            RuleReference("NEWLINE", isToken: true)
          ])),
          RuleReference("declaration_body", isToken: false)
        ]),
        lineNumber: 11),
    GrammarRule(
        name: "declaration_body",
        body: Alternation(choices: [
          RuleReference("module_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("open_declaration", isToken: false),
          RuleReference("let_binding", isToken: false),
          RuleReference("use_binding", isToken: false),
          RuleReference("type_declaration", isToken: false),
          RuleReference("member_declaration", isToken: false),
          RuleReference("do_binding", isToken: false),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 13),
    GrammarRule(
        name: "module_declaration",
        body: Sequence(elements: [
          Literal("module"),
          Repetition(element: RuleReference("module_modifier", isToken: false)),
          Optional(element: Literal("rec")),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ]))
        ]),
        lineNumber: 23),
    GrammarRule(
        name: "module_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 25),
    GrammarRule(
        name: "namespace_declaration",
        body: Sequence(elements: [
          Literal("namespace"),
          Repetition(
              element: RuleReference("namespace_modifier", isToken: false)),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 28),
    GrammarRule(
        name: "namespace_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 30),
    GrammarRule(
        name: "open_declaration",
        body: Sequence(elements: [
          Literal("open"),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 33),
    GrammarRule(
        name: "let_binding",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 35),
    GrammarRule(
        name: "use_binding",
        body: Sequence(elements: [
          Literal("use"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 37),
    GrammarRule(
        name: "binding_modifier",
        body: Alternation(choices: [Literal("inline"), Literal("mutable")]),
        lineNumber: 39),
    GrammarRule(
        name: "binding_clause",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          RuleReference("EQUALS", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 42),
    GrammarRule(
        name: "do_binding",
        body: Sequence(elements: [
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 44),
    GrammarRule(
        name: "member_declaration",
        body: Sequence(elements: [
          Optional(element: RuleReference("member_modifier", isToken: false)),
          Literal("member"),
          RuleReference("qualified_name", isToken: false),
          Optional(element: RuleReference("parameter_list", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 46),
    GrammarRule(
        name: "member_modifier",
        body: Alternation(choices: [
          Literal("static"),
          Literal("override"),
          Literal("default"),
          Literal("abstract"),
          Literal("new")
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "type_declaration",
        body: Sequence(elements: [
          Literal("type"),
          Repetition(element: RuleReference("type_modifier", isToken: false)),
          RuleReference("NAME", isToken: true),
          Optional(
              element: RuleReference("generic_parameters", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ])),
          RuleReference("type_definition", isToken: false)
        ]),
        lineNumber: 54),
    GrammarRule(
        name: "type_modifier",
        body: Alternation(choices: [
          Literal("private"),
          Literal("public"),
          Literal("internal")
        ]),
        lineNumber: 56),
    GrammarRule(
        name: "generic_parameters",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_parameter", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 60),
    GrammarRule(
        name: "type_parameter",
        body: Alternation(choices: [
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 62),
    GrammarRule(
        name: "type_definition",
        body: Alternation(choices: [
          RuleReference("class_type", isToken: false),
          RuleReference("interface_type", isToken: false),
          RuleReference("struct_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("union_type", isToken: false),
          RuleReference("alias_type", isToken: false)
        ]),
        lineNumber: 65),
    GrammarRule(
        name: "class_type",
        body: Sequence(elements: [
          Literal("class"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 72),
    GrammarRule(
        name: "interface_type",
        body: Sequence(elements: [
          Literal("interface"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 74),
    GrammarRule(
        name: "struct_type",
        body: Sequence(elements: [
          Literal("struct"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "record_type",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_declaration", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_declaration", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 78),
    GrammarRule(
        name: "field_declaration",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 80),
    GrammarRule(
        name: "union_type",
        body: Sequence(elements: [
          RuleReference("union_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("union_case", isToken: false)
          ]))
        ]),
        lineNumber: 82),
    GrammarRule(
        name: "union_case",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("of"),
            RuleReference("type_expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("STAR", isToken: true),
              RuleReference("type_expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 84),
    GrammarRule(
        name: "alias_type",
        body: RuleReference("type_expression", isToken: false),
        lineNumber: 86),
    GrammarRule(
        name: "field_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 88),
    GrammarRule(
        name: "case_separator",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            Optional(element: RuleReference("PIPE", isToken: true))
          ]),
          RuleReference("PIPE", isToken: true)
        ]),
        lineNumber: 91),
    GrammarRule(
        name: "attribute_section",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("attribute", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("attribute", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 94),
    GrammarRule(
        name: "attribute",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("attribute_target", isToken: false),
            RuleReference("COLON", isToken: true)
          ])),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("attribute_arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ]))
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "attribute_target",
        body: Alternation(choices: [
          Literal("assembly"),
          Literal("field"),
          Literal("method"),
          Literal("module"),
          Literal("param"),
          Literal("property"),
          Literal("return"),
          Literal("type"),
          Literal("event")
        ]),
        lineNumber: 98),
    GrammarRule(
        name: "attribute_arguments",
        body: Sequence(elements: [
          RuleReference("attribute_argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("attribute_argument", isToken: false)
          ]))
        ]),
        lineNumber: 108),
    GrammarRule(
        name: "attribute_argument",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true)
          ])),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 110),
    GrammarRule(
        name: "parameter_list",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false))
        ]),
        lineNumber: 112),
    GrammarRule(
        name: "type_annotation",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 114),
    GrammarRule(
        name: "type_expression",
        body: RuleReference("function_type", isToken: false),
        lineNumber: 116),
    GrammarRule(
        name: "function_type",
        body: Sequence(elements: [
          RuleReference("type_product", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("function_type", isToken: false)
          ]))
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "type_product",
        body: Sequence(elements: [
          RuleReference("type_application", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("type_application", isToken: false)
          ]))
        ]),
        lineNumber: 120),
    GrammarRule(
        name: "type_application",
        body: Sequence(elements: [
          RuleReference("type_atom", isToken: false),
          Repetition(element: RuleReference("type_atom", isToken: false))
        ]),
        lineNumber: 122),
    GrammarRule(
        name: "generic_type_arguments",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 124),
    GrammarRule(
        name: "type_atom",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("qualified_name", isToken: false),
            Optional(
                element:
                    RuleReference("generic_type_arguments", isToken: false))
          ]),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("tuple_type", isToken: false),
          RuleReference("unit_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("parenthesized_type", isToken: false)
        ]),
        lineNumber: 126),
    GrammarRule(
        name: "tuple_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 133),
    GrammarRule(
        name: "unit_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 135),
    GrammarRule(
        name: "parenthesized_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 137),
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
        lineNumber: 139),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("sequence_expression", isToken: false)
        ]),
        lineNumber: 141),
    GrammarRule(
        name: "sequence_expression",
        body: Sequence(elements: [
          RuleReference("infix_expression", isToken: false),
          Repetition(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("NEWLINE", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ]),
            Sequence(elements: [
              RuleReference("SEMICOLON", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 150),
    GrammarRule(
        name: "infix_expression",
        body: Sequence(elements: [
          RuleReference("application_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("infix_operator", isToken: false),
            RuleReference("application_expression", isToken: false)
          ]))
        ]),
        lineNumber: 152),
    GrammarRule(
        name: "infix_operator",
        body: Alternation(choices: [
          RuleReference("PIPE_RIGHT", isToken: true),
          RuleReference("PIPE_LEFT", isToken: true),
          RuleReference("COMPOSE_RIGHT", isToken: true),
          RuleReference("COMPOSE_LEFT", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("COLON_EQUALS", isToken: true),
          RuleReference("DOUBLE_COLON", isToken: true),
          RuleReference("LARROW", isToken: true),
          RuleReference("ARROW", isToken: true),
          RuleReference("DOT_DOT", isToken: true),
          RuleReference("AND_AND", isToken: true),
          RuleReference("OR_OR", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("PERCENT", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          RuleReference("PIPE", isToken: true),
          RuleReference("CARET", isToken: true)
        ]),
        lineNumber: 154),
    GrammarRule(
        name: "application_expression",
        body: Sequence(elements: [
          RuleReference("prefix_expression", isToken: false),
          Repetition(
              element: RuleReference("prefix_expression", isToken: false))
        ]),
        lineNumber: 181),
    GrammarRule(
        name: "prefix_expression",
        body: Sequence(elements: [
          Optional(element: RuleReference("unary_operator", isToken: false)),
          RuleReference("atomic_expression", isToken: false)
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "unary_operator",
        body: Alternation(choices: [
          RuleReference("MINUS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("BANG", isToken: true)
        ]),
        lineNumber: 185),
    GrammarRule(
        name: "atomic_expression",
        body: Alternation(choices: [
          RuleReference("computation_expression", isToken: false),
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("unit_expression", isToken: false),
          RuleReference("tuple_expression", isToken: false),
          RuleReference("list_expression", isToken: false),
          RuleReference("array_expression", isToken: false),
          RuleReference("record_expression", isToken: false),
          RuleReference("parenthesized_expression", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 189),
    GrammarRule(
        name: "computation_expression",
        body: Sequence(elements: [
          RuleReference("qualified_name", isToken: false),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("expression", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 211),
    GrammarRule(
        name: "unit_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 213),
    GrammarRule(
        name: "tuple_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 215),
    GrammarRule(
        name: "list_expression",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          NegativeLookahead(element: RuleReference("LESS_THAN", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 217),
    GrammarRule(
        name: "array_expression",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 219),
    GrammarRule(
        name: "record_expression",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_assignment", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_assignment", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 221),
    GrammarRule(
        name: "field_assignment",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 223),
    GrammarRule(
        name: "parenthesized_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 225),
    GrammarRule(
        name: "element_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 227),
    GrammarRule(
        name: "if_expression",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          Literal("then"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 230),
    GrammarRule(
        name: "match_expression",
        body: Sequence(elements: [
          Literal("match"),
          RuleReference("expression", isToken: false),
          Literal("with"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 232),
    GrammarRule(
        name: "match_case",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("when"),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 234),
    GrammarRule(
        name: "fun_expression",
        body: Sequence(elements: [
          Literal("fun"),
          RuleReference("parameter_list", isToken: false),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 236),
    GrammarRule(
        name: "function_expression",
        body: Sequence(elements: [
          Literal("function"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 238),
    GrammarRule(
        name: "let_expression",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ])),
          Literal("in"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 240),
    GrammarRule(
        name: "for_expression",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("pattern", isToken: false),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false),
          Group(
              element:
                  Alternation(choices: [Literal("to"), Literal("downto")])),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 242),
    GrammarRule(
        name: "while_expression",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 244),
    GrammarRule(
        name: "pattern",
        body: Sequence(elements: [
          RuleReference("pattern_atom", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 246),
    GrammarRule(
        name: "pattern_atom",
        body: Alternation(choices: [
          RuleReference("wildcard_pattern", isToken: false),
          RuleReference("literal_pattern", isToken: false),
          RuleReference("tuple_pattern", isToken: false),
          RuleReference("list_pattern", isToken: false),
          RuleReference("array_pattern", isToken: false),
          RuleReference("record_pattern", isToken: false),
          RuleReference("unit_pattern", isToken: false),
          RuleReference("parenthesized_pattern", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 248),
    GrammarRule(
        name: "wildcard_pattern",
        body: RuleReference("UNDERSCORE", isToken: true),
        lineNumber: 260),
    GrammarRule(
        name: "literal_pattern",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 262),
    GrammarRule(
        name: "tuple_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("pattern", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 269),
    GrammarRule(
        name: "list_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 271),
    GrammarRule(
        name: "array_pattern",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 273),
    GrammarRule(
        name: "record_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 275),
    GrammarRule(
        name: "field_pattern",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("pattern", isToken: false)
        ]),
        lineNumber: 277),
    GrammarRule(
        name: "unit_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 279),
    GrammarRule(
        name: "parenthesized_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 281),
  ],
);

final parserGrammarV3_1 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "compilation_unit",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("decorated_declaration", isToken: false)
        ])),
        lineNumber: 9),
    GrammarRule(
        name: "decorated_declaration",
        body: Sequence(elements: [
          Repetition(
              element: Sequence(elements: [
            RuleReference("attribute_section", isToken: false),
            RuleReference("NEWLINE", isToken: true)
          ])),
          RuleReference("declaration_body", isToken: false)
        ]),
        lineNumber: 11),
    GrammarRule(
        name: "declaration_body",
        body: Alternation(choices: [
          RuleReference("module_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("open_declaration", isToken: false),
          RuleReference("let_binding", isToken: false),
          RuleReference("use_binding", isToken: false),
          RuleReference("type_declaration", isToken: false),
          RuleReference("member_declaration", isToken: false),
          RuleReference("do_binding", isToken: false),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 13),
    GrammarRule(
        name: "module_declaration",
        body: Sequence(elements: [
          Literal("module"),
          Repetition(element: RuleReference("module_modifier", isToken: false)),
          Optional(element: Literal("rec")),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ]))
        ]),
        lineNumber: 23),
    GrammarRule(
        name: "module_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 25),
    GrammarRule(
        name: "namespace_declaration",
        body: Sequence(elements: [
          Literal("namespace"),
          Repetition(
              element: RuleReference("namespace_modifier", isToken: false)),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 28),
    GrammarRule(
        name: "namespace_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 30),
    GrammarRule(
        name: "open_declaration",
        body: Sequence(elements: [
          Literal("open"),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 33),
    GrammarRule(
        name: "let_binding",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 35),
    GrammarRule(
        name: "use_binding",
        body: Sequence(elements: [
          Literal("use"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 37),
    GrammarRule(
        name: "binding_modifier",
        body: Alternation(choices: [Literal("inline"), Literal("mutable")]),
        lineNumber: 39),
    GrammarRule(
        name: "binding_clause",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          RuleReference("EQUALS", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 42),
    GrammarRule(
        name: "do_binding",
        body: Sequence(elements: [
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 44),
    GrammarRule(
        name: "member_declaration",
        body: Sequence(elements: [
          Optional(element: RuleReference("member_modifier", isToken: false)),
          Literal("member"),
          RuleReference("qualified_name", isToken: false),
          Optional(element: RuleReference("parameter_list", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 46),
    GrammarRule(
        name: "member_modifier",
        body: Alternation(choices: [
          Literal("static"),
          Literal("override"),
          Literal("default"),
          Literal("abstract"),
          Literal("new")
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "type_declaration",
        body: Sequence(elements: [
          Literal("type"),
          Repetition(element: RuleReference("type_modifier", isToken: false)),
          RuleReference("NAME", isToken: true),
          Optional(
              element: RuleReference("generic_parameters", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ])),
          RuleReference("type_definition", isToken: false)
        ]),
        lineNumber: 54),
    GrammarRule(
        name: "type_modifier",
        body: Alternation(choices: [
          Literal("private"),
          Literal("public"),
          Literal("internal")
        ]),
        lineNumber: 56),
    GrammarRule(
        name: "generic_parameters",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_parameter", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 60),
    GrammarRule(
        name: "type_parameter",
        body: Alternation(choices: [
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 62),
    GrammarRule(
        name: "type_definition",
        body: Alternation(choices: [
          RuleReference("class_type", isToken: false),
          RuleReference("interface_type", isToken: false),
          RuleReference("struct_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("union_type", isToken: false),
          RuleReference("alias_type", isToken: false)
        ]),
        lineNumber: 65),
    GrammarRule(
        name: "class_type",
        body: Sequence(elements: [
          Literal("class"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 72),
    GrammarRule(
        name: "interface_type",
        body: Sequence(elements: [
          Literal("interface"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 74),
    GrammarRule(
        name: "struct_type",
        body: Sequence(elements: [
          Literal("struct"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "record_type",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_declaration", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_declaration", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 78),
    GrammarRule(
        name: "field_declaration",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 80),
    GrammarRule(
        name: "union_type",
        body: Sequence(elements: [
          RuleReference("union_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("union_case", isToken: false)
          ]))
        ]),
        lineNumber: 82),
    GrammarRule(
        name: "union_case",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("of"),
            RuleReference("type_expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("STAR", isToken: true),
              RuleReference("type_expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 84),
    GrammarRule(
        name: "alias_type",
        body: RuleReference("type_expression", isToken: false),
        lineNumber: 86),
    GrammarRule(
        name: "field_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 88),
    GrammarRule(
        name: "case_separator",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            Optional(element: RuleReference("PIPE", isToken: true))
          ]),
          RuleReference("PIPE", isToken: true)
        ]),
        lineNumber: 91),
    GrammarRule(
        name: "attribute_section",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("attribute", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("attribute", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 94),
    GrammarRule(
        name: "attribute",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("attribute_target", isToken: false),
            RuleReference("COLON", isToken: true)
          ])),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("attribute_arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ]))
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "attribute_target",
        body: Alternation(choices: [
          Literal("assembly"),
          Literal("field"),
          Literal("method"),
          Literal("module"),
          Literal("param"),
          Literal("property"),
          Literal("return"),
          Literal("type"),
          Literal("event")
        ]),
        lineNumber: 98),
    GrammarRule(
        name: "attribute_arguments",
        body: Sequence(elements: [
          RuleReference("attribute_argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("attribute_argument", isToken: false)
          ]))
        ]),
        lineNumber: 108),
    GrammarRule(
        name: "attribute_argument",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true)
          ])),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 110),
    GrammarRule(
        name: "parameter_list",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false))
        ]),
        lineNumber: 112),
    GrammarRule(
        name: "type_annotation",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 114),
    GrammarRule(
        name: "type_expression",
        body: RuleReference("function_type", isToken: false),
        lineNumber: 116),
    GrammarRule(
        name: "function_type",
        body: Sequence(elements: [
          RuleReference("type_product", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("function_type", isToken: false)
          ]))
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "type_product",
        body: Sequence(elements: [
          RuleReference("type_application", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("type_application", isToken: false)
          ]))
        ]),
        lineNumber: 120),
    GrammarRule(
        name: "type_application",
        body: Sequence(elements: [
          RuleReference("type_atom", isToken: false),
          Repetition(element: RuleReference("type_atom", isToken: false))
        ]),
        lineNumber: 122),
    GrammarRule(
        name: "generic_type_arguments",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 124),
    GrammarRule(
        name: "type_atom",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("qualified_name", isToken: false),
            Optional(
                element:
                    RuleReference("generic_type_arguments", isToken: false))
          ]),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("tuple_type", isToken: false),
          RuleReference("unit_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("parenthesized_type", isToken: false)
        ]),
        lineNumber: 126),
    GrammarRule(
        name: "tuple_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 133),
    GrammarRule(
        name: "unit_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 135),
    GrammarRule(
        name: "parenthesized_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 137),
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
        lineNumber: 139),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("sequence_expression", isToken: false)
        ]),
        lineNumber: 141),
    GrammarRule(
        name: "sequence_expression",
        body: Sequence(elements: [
          RuleReference("infix_expression", isToken: false),
          Repetition(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("NEWLINE", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ]),
            Sequence(elements: [
              RuleReference("SEMICOLON", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 150),
    GrammarRule(
        name: "infix_expression",
        body: Sequence(elements: [
          RuleReference("application_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("infix_operator", isToken: false),
            RuleReference("application_expression", isToken: false)
          ]))
        ]),
        lineNumber: 152),
    GrammarRule(
        name: "infix_operator",
        body: Alternation(choices: [
          RuleReference("PIPE_RIGHT", isToken: true),
          RuleReference("PIPE_LEFT", isToken: true),
          RuleReference("COMPOSE_RIGHT", isToken: true),
          RuleReference("COMPOSE_LEFT", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("COLON_EQUALS", isToken: true),
          RuleReference("DOUBLE_COLON", isToken: true),
          RuleReference("LARROW", isToken: true),
          RuleReference("ARROW", isToken: true),
          RuleReference("DOT_DOT", isToken: true),
          RuleReference("AND_AND", isToken: true),
          RuleReference("OR_OR", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("PERCENT", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          RuleReference("PIPE", isToken: true),
          RuleReference("CARET", isToken: true)
        ]),
        lineNumber: 154),
    GrammarRule(
        name: "application_expression",
        body: Sequence(elements: [
          RuleReference("prefix_expression", isToken: false),
          Repetition(
              element: RuleReference("prefix_expression", isToken: false))
        ]),
        lineNumber: 181),
    GrammarRule(
        name: "prefix_expression",
        body: Sequence(elements: [
          Optional(element: RuleReference("unary_operator", isToken: false)),
          RuleReference("atomic_expression", isToken: false)
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "unary_operator",
        body: Alternation(choices: [
          RuleReference("MINUS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("BANG", isToken: true)
        ]),
        lineNumber: 185),
    GrammarRule(
        name: "atomic_expression",
        body: Alternation(choices: [
          RuleReference("computation_expression", isToken: false),
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("unit_expression", isToken: false),
          RuleReference("tuple_expression", isToken: false),
          RuleReference("list_expression", isToken: false),
          RuleReference("array_expression", isToken: false),
          RuleReference("record_expression", isToken: false),
          RuleReference("parenthesized_expression", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 189),
    GrammarRule(
        name: "computation_expression",
        body: Sequence(elements: [
          RuleReference("qualified_name", isToken: false),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("expression", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 211),
    GrammarRule(
        name: "unit_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 213),
    GrammarRule(
        name: "tuple_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 215),
    GrammarRule(
        name: "list_expression",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          NegativeLookahead(element: RuleReference("LESS_THAN", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 217),
    GrammarRule(
        name: "array_expression",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 219),
    GrammarRule(
        name: "record_expression",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_assignment", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_assignment", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 221),
    GrammarRule(
        name: "field_assignment",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 223),
    GrammarRule(
        name: "parenthesized_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 225),
    GrammarRule(
        name: "element_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 227),
    GrammarRule(
        name: "if_expression",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          Literal("then"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 230),
    GrammarRule(
        name: "match_expression",
        body: Sequence(elements: [
          Literal("match"),
          RuleReference("expression", isToken: false),
          Literal("with"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 232),
    GrammarRule(
        name: "match_case",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("when"),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 234),
    GrammarRule(
        name: "fun_expression",
        body: Sequence(elements: [
          Literal("fun"),
          RuleReference("parameter_list", isToken: false),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 236),
    GrammarRule(
        name: "function_expression",
        body: Sequence(elements: [
          Literal("function"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 238),
    GrammarRule(
        name: "let_expression",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ])),
          Literal("in"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 240),
    GrammarRule(
        name: "for_expression",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("pattern", isToken: false),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false),
          Group(
              element:
                  Alternation(choices: [Literal("to"), Literal("downto")])),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 242),
    GrammarRule(
        name: "while_expression",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 244),
    GrammarRule(
        name: "pattern",
        body: Sequence(elements: [
          RuleReference("pattern_atom", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 246),
    GrammarRule(
        name: "pattern_atom",
        body: Alternation(choices: [
          RuleReference("wildcard_pattern", isToken: false),
          RuleReference("literal_pattern", isToken: false),
          RuleReference("tuple_pattern", isToken: false),
          RuleReference("list_pattern", isToken: false),
          RuleReference("array_pattern", isToken: false),
          RuleReference("record_pattern", isToken: false),
          RuleReference("unit_pattern", isToken: false),
          RuleReference("parenthesized_pattern", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 248),
    GrammarRule(
        name: "wildcard_pattern",
        body: RuleReference("UNDERSCORE", isToken: true),
        lineNumber: 260),
    GrammarRule(
        name: "literal_pattern",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 262),
    GrammarRule(
        name: "tuple_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("pattern", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 269),
    GrammarRule(
        name: "list_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 271),
    GrammarRule(
        name: "array_pattern",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 273),
    GrammarRule(
        name: "record_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 275),
    GrammarRule(
        name: "field_pattern",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("pattern", isToken: false)
        ]),
        lineNumber: 277),
    GrammarRule(
        name: "unit_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 279),
    GrammarRule(
        name: "parenthesized_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 281),
  ],
);

final parserGrammarV4_0 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "compilation_unit",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("decorated_declaration", isToken: false)
        ])),
        lineNumber: 9),
    GrammarRule(
        name: "decorated_declaration",
        body: Sequence(elements: [
          Repetition(
              element: Sequence(elements: [
            RuleReference("attribute_section", isToken: false),
            RuleReference("NEWLINE", isToken: true)
          ])),
          RuleReference("declaration_body", isToken: false)
        ]),
        lineNumber: 11),
    GrammarRule(
        name: "declaration_body",
        body: Alternation(choices: [
          RuleReference("module_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("open_declaration", isToken: false),
          RuleReference("let_binding", isToken: false),
          RuleReference("use_binding", isToken: false),
          RuleReference("type_declaration", isToken: false),
          RuleReference("member_declaration", isToken: false),
          RuleReference("do_binding", isToken: false),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 13),
    GrammarRule(
        name: "module_declaration",
        body: Sequence(elements: [
          Literal("module"),
          Repetition(element: RuleReference("module_modifier", isToken: false)),
          Optional(element: Literal("rec")),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ]))
        ]),
        lineNumber: 23),
    GrammarRule(
        name: "module_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 25),
    GrammarRule(
        name: "namespace_declaration",
        body: Sequence(elements: [
          Literal("namespace"),
          Repetition(
              element: RuleReference("namespace_modifier", isToken: false)),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 28),
    GrammarRule(
        name: "namespace_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 30),
    GrammarRule(
        name: "open_declaration",
        body: Sequence(elements: [
          Literal("open"),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 33),
    GrammarRule(
        name: "let_binding",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 35),
    GrammarRule(
        name: "use_binding",
        body: Sequence(elements: [
          Literal("use"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 37),
    GrammarRule(
        name: "binding_modifier",
        body: Alternation(choices: [Literal("inline"), Literal("mutable")]),
        lineNumber: 39),
    GrammarRule(
        name: "binding_clause",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          RuleReference("EQUALS", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 42),
    GrammarRule(
        name: "do_binding",
        body: Sequence(elements: [
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 44),
    GrammarRule(
        name: "member_declaration",
        body: Sequence(elements: [
          Optional(element: RuleReference("member_modifier", isToken: false)),
          Literal("member"),
          RuleReference("qualified_name", isToken: false),
          Optional(element: RuleReference("parameter_list", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 46),
    GrammarRule(
        name: "member_modifier",
        body: Alternation(choices: [
          Literal("static"),
          Literal("override"),
          Literal("default"),
          Literal("abstract"),
          Literal("new")
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "type_declaration",
        body: Sequence(elements: [
          Literal("type"),
          Repetition(element: RuleReference("type_modifier", isToken: false)),
          RuleReference("NAME", isToken: true),
          Optional(
              element: RuleReference("generic_parameters", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ])),
          RuleReference("type_definition", isToken: false)
        ]),
        lineNumber: 54),
    GrammarRule(
        name: "type_modifier",
        body: Alternation(choices: [
          Literal("private"),
          Literal("public"),
          Literal("internal")
        ]),
        lineNumber: 56),
    GrammarRule(
        name: "generic_parameters",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_parameter", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 60),
    GrammarRule(
        name: "type_parameter",
        body: Alternation(choices: [
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 62),
    GrammarRule(
        name: "type_definition",
        body: Alternation(choices: [
          RuleReference("class_type", isToken: false),
          RuleReference("interface_type", isToken: false),
          RuleReference("struct_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("union_type", isToken: false),
          RuleReference("alias_type", isToken: false)
        ]),
        lineNumber: 65),
    GrammarRule(
        name: "class_type",
        body: Sequence(elements: [
          Literal("class"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 72),
    GrammarRule(
        name: "interface_type",
        body: Sequence(elements: [
          Literal("interface"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 74),
    GrammarRule(
        name: "struct_type",
        body: Sequence(elements: [
          Literal("struct"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "record_type",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_declaration", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_declaration", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 78),
    GrammarRule(
        name: "field_declaration",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 80),
    GrammarRule(
        name: "union_type",
        body: Sequence(elements: [
          RuleReference("union_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("union_case", isToken: false)
          ]))
        ]),
        lineNumber: 82),
    GrammarRule(
        name: "union_case",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("of"),
            RuleReference("type_expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("STAR", isToken: true),
              RuleReference("type_expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 84),
    GrammarRule(
        name: "alias_type",
        body: RuleReference("type_expression", isToken: false),
        lineNumber: 86),
    GrammarRule(
        name: "field_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 88),
    GrammarRule(
        name: "case_separator",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            Optional(element: RuleReference("PIPE", isToken: true))
          ]),
          RuleReference("PIPE", isToken: true)
        ]),
        lineNumber: 91),
    GrammarRule(
        name: "attribute_section",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("attribute", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("attribute", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 94),
    GrammarRule(
        name: "attribute",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("attribute_target", isToken: false),
            RuleReference("COLON", isToken: true)
          ])),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("attribute_arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ]))
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "attribute_target",
        body: Alternation(choices: [
          Literal("assembly"),
          Literal("field"),
          Literal("method"),
          Literal("module"),
          Literal("param"),
          Literal("property"),
          Literal("return"),
          Literal("type"),
          Literal("event")
        ]),
        lineNumber: 98),
    GrammarRule(
        name: "attribute_arguments",
        body: Sequence(elements: [
          RuleReference("attribute_argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("attribute_argument", isToken: false)
          ]))
        ]),
        lineNumber: 108),
    GrammarRule(
        name: "attribute_argument",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true)
          ])),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 110),
    GrammarRule(
        name: "parameter_list",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false))
        ]),
        lineNumber: 112),
    GrammarRule(
        name: "type_annotation",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 114),
    GrammarRule(
        name: "type_expression",
        body: RuleReference("function_type", isToken: false),
        lineNumber: 116),
    GrammarRule(
        name: "function_type",
        body: Sequence(elements: [
          RuleReference("type_product", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("function_type", isToken: false)
          ]))
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "type_product",
        body: Sequence(elements: [
          RuleReference("type_application", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("type_application", isToken: false)
          ]))
        ]),
        lineNumber: 120),
    GrammarRule(
        name: "type_application",
        body: Sequence(elements: [
          RuleReference("type_atom", isToken: false),
          Repetition(element: RuleReference("type_atom", isToken: false))
        ]),
        lineNumber: 122),
    GrammarRule(
        name: "generic_type_arguments",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 124),
    GrammarRule(
        name: "type_atom",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("qualified_name", isToken: false),
            Optional(
                element:
                    RuleReference("generic_type_arguments", isToken: false))
          ]),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("tuple_type", isToken: false),
          RuleReference("unit_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("parenthesized_type", isToken: false)
        ]),
        lineNumber: 126),
    GrammarRule(
        name: "tuple_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 133),
    GrammarRule(
        name: "unit_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 135),
    GrammarRule(
        name: "parenthesized_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 137),
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
        lineNumber: 139),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("sequence_expression", isToken: false)
        ]),
        lineNumber: 141),
    GrammarRule(
        name: "sequence_expression",
        body: Sequence(elements: [
          RuleReference("infix_expression", isToken: false),
          Repetition(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("NEWLINE", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ]),
            Sequence(elements: [
              RuleReference("SEMICOLON", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 150),
    GrammarRule(
        name: "infix_expression",
        body: Sequence(elements: [
          RuleReference("application_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("infix_operator", isToken: false),
            RuleReference("application_expression", isToken: false)
          ]))
        ]),
        lineNumber: 152),
    GrammarRule(
        name: "infix_operator",
        body: Alternation(choices: [
          RuleReference("PIPE_RIGHT", isToken: true),
          RuleReference("PIPE_LEFT", isToken: true),
          RuleReference("COMPOSE_RIGHT", isToken: true),
          RuleReference("COMPOSE_LEFT", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("COLON_EQUALS", isToken: true),
          RuleReference("DOUBLE_COLON", isToken: true),
          RuleReference("LARROW", isToken: true),
          RuleReference("ARROW", isToken: true),
          RuleReference("DOT_DOT", isToken: true),
          RuleReference("AND_AND", isToken: true),
          RuleReference("OR_OR", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("PERCENT", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          RuleReference("PIPE", isToken: true),
          RuleReference("CARET", isToken: true)
        ]),
        lineNumber: 154),
    GrammarRule(
        name: "application_expression",
        body: Sequence(elements: [
          RuleReference("prefix_expression", isToken: false),
          Repetition(
              element: RuleReference("prefix_expression", isToken: false))
        ]),
        lineNumber: 181),
    GrammarRule(
        name: "prefix_expression",
        body: Sequence(elements: [
          Optional(element: RuleReference("unary_operator", isToken: false)),
          RuleReference("atomic_expression", isToken: false)
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "unary_operator",
        body: Alternation(choices: [
          RuleReference("MINUS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("BANG", isToken: true)
        ]),
        lineNumber: 185),
    GrammarRule(
        name: "atomic_expression",
        body: Alternation(choices: [
          RuleReference("computation_expression", isToken: false),
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("unit_expression", isToken: false),
          RuleReference("tuple_expression", isToken: false),
          RuleReference("list_expression", isToken: false),
          RuleReference("array_expression", isToken: false),
          RuleReference("record_expression", isToken: false),
          RuleReference("parenthesized_expression", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 189),
    GrammarRule(
        name: "computation_expression",
        body: Sequence(elements: [
          RuleReference("qualified_name", isToken: false),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("expression", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 211),
    GrammarRule(
        name: "unit_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 213),
    GrammarRule(
        name: "tuple_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 215),
    GrammarRule(
        name: "list_expression",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          NegativeLookahead(element: RuleReference("LESS_THAN", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 217),
    GrammarRule(
        name: "array_expression",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 219),
    GrammarRule(
        name: "record_expression",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_assignment", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_assignment", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 221),
    GrammarRule(
        name: "field_assignment",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 223),
    GrammarRule(
        name: "parenthesized_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 225),
    GrammarRule(
        name: "element_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 227),
    GrammarRule(
        name: "if_expression",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          Literal("then"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 230),
    GrammarRule(
        name: "match_expression",
        body: Sequence(elements: [
          Literal("match"),
          RuleReference("expression", isToken: false),
          Literal("with"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 232),
    GrammarRule(
        name: "match_case",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("when"),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 234),
    GrammarRule(
        name: "fun_expression",
        body: Sequence(elements: [
          Literal("fun"),
          RuleReference("parameter_list", isToken: false),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 236),
    GrammarRule(
        name: "function_expression",
        body: Sequence(elements: [
          Literal("function"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 238),
    GrammarRule(
        name: "let_expression",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ])),
          Literal("in"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 240),
    GrammarRule(
        name: "for_expression",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("pattern", isToken: false),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false),
          Group(
              element:
                  Alternation(choices: [Literal("to"), Literal("downto")])),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 242),
    GrammarRule(
        name: "while_expression",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 244),
    GrammarRule(
        name: "pattern",
        body: Sequence(elements: [
          RuleReference("pattern_atom", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 246),
    GrammarRule(
        name: "pattern_atom",
        body: Alternation(choices: [
          RuleReference("wildcard_pattern", isToken: false),
          RuleReference("literal_pattern", isToken: false),
          RuleReference("tuple_pattern", isToken: false),
          RuleReference("list_pattern", isToken: false),
          RuleReference("array_pattern", isToken: false),
          RuleReference("record_pattern", isToken: false),
          RuleReference("unit_pattern", isToken: false),
          RuleReference("parenthesized_pattern", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 248),
    GrammarRule(
        name: "wildcard_pattern",
        body: RuleReference("UNDERSCORE", isToken: true),
        lineNumber: 260),
    GrammarRule(
        name: "literal_pattern",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 262),
    GrammarRule(
        name: "tuple_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("pattern", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 269),
    GrammarRule(
        name: "list_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 271),
    GrammarRule(
        name: "array_pattern",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 273),
    GrammarRule(
        name: "record_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 275),
    GrammarRule(
        name: "field_pattern",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("pattern", isToken: false)
        ]),
        lineNumber: 277),
    GrammarRule(
        name: "unit_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 279),
    GrammarRule(
        name: "parenthesized_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 281),
  ],
);

final parserGrammarV4_1 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "compilation_unit",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("decorated_declaration", isToken: false)
        ])),
        lineNumber: 9),
    GrammarRule(
        name: "decorated_declaration",
        body: Sequence(elements: [
          Repetition(
              element: Sequence(elements: [
            RuleReference("attribute_section", isToken: false),
            RuleReference("NEWLINE", isToken: true)
          ])),
          RuleReference("declaration_body", isToken: false)
        ]),
        lineNumber: 11),
    GrammarRule(
        name: "declaration_body",
        body: Alternation(choices: [
          RuleReference("module_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("open_declaration", isToken: false),
          RuleReference("let_binding", isToken: false),
          RuleReference("use_binding", isToken: false),
          RuleReference("type_declaration", isToken: false),
          RuleReference("member_declaration", isToken: false),
          RuleReference("do_binding", isToken: false),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 13),
    GrammarRule(
        name: "module_declaration",
        body: Sequence(elements: [
          Literal("module"),
          Repetition(element: RuleReference("module_modifier", isToken: false)),
          Optional(element: Literal("rec")),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ]))
        ]),
        lineNumber: 23),
    GrammarRule(
        name: "module_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 25),
    GrammarRule(
        name: "namespace_declaration",
        body: Sequence(elements: [
          Literal("namespace"),
          Repetition(
              element: RuleReference("namespace_modifier", isToken: false)),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 28),
    GrammarRule(
        name: "namespace_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 30),
    GrammarRule(
        name: "open_declaration",
        body: Sequence(elements: [
          Literal("open"),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 33),
    GrammarRule(
        name: "let_binding",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 35),
    GrammarRule(
        name: "use_binding",
        body: Sequence(elements: [
          Literal("use"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 37),
    GrammarRule(
        name: "binding_modifier",
        body: Alternation(choices: [Literal("inline"), Literal("mutable")]),
        lineNumber: 39),
    GrammarRule(
        name: "binding_clause",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          RuleReference("EQUALS", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 42),
    GrammarRule(
        name: "do_binding",
        body: Sequence(elements: [
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 44),
    GrammarRule(
        name: "member_declaration",
        body: Sequence(elements: [
          Optional(element: RuleReference("member_modifier", isToken: false)),
          Literal("member"),
          RuleReference("qualified_name", isToken: false),
          Optional(element: RuleReference("parameter_list", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 46),
    GrammarRule(
        name: "member_modifier",
        body: Alternation(choices: [
          Literal("static"),
          Literal("override"),
          Literal("default"),
          Literal("abstract"),
          Literal("new")
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "type_declaration",
        body: Sequence(elements: [
          Literal("type"),
          Repetition(element: RuleReference("type_modifier", isToken: false)),
          RuleReference("NAME", isToken: true),
          Optional(
              element: RuleReference("generic_parameters", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ])),
          RuleReference("type_definition", isToken: false)
        ]),
        lineNumber: 54),
    GrammarRule(
        name: "type_modifier",
        body: Alternation(choices: [
          Literal("private"),
          Literal("public"),
          Literal("internal")
        ]),
        lineNumber: 56),
    GrammarRule(
        name: "generic_parameters",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_parameter", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 60),
    GrammarRule(
        name: "type_parameter",
        body: Alternation(choices: [
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 62),
    GrammarRule(
        name: "type_definition",
        body: Alternation(choices: [
          RuleReference("class_type", isToken: false),
          RuleReference("interface_type", isToken: false),
          RuleReference("struct_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("union_type", isToken: false),
          RuleReference("alias_type", isToken: false)
        ]),
        lineNumber: 65),
    GrammarRule(
        name: "class_type",
        body: Sequence(elements: [
          Literal("class"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 72),
    GrammarRule(
        name: "interface_type",
        body: Sequence(elements: [
          Literal("interface"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 74),
    GrammarRule(
        name: "struct_type",
        body: Sequence(elements: [
          Literal("struct"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "record_type",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_declaration", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_declaration", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 78),
    GrammarRule(
        name: "field_declaration",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 80),
    GrammarRule(
        name: "union_type",
        body: Sequence(elements: [
          RuleReference("union_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("union_case", isToken: false)
          ]))
        ]),
        lineNumber: 82),
    GrammarRule(
        name: "union_case",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("of"),
            RuleReference("type_expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("STAR", isToken: true),
              RuleReference("type_expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 84),
    GrammarRule(
        name: "alias_type",
        body: RuleReference("type_expression", isToken: false),
        lineNumber: 86),
    GrammarRule(
        name: "field_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 88),
    GrammarRule(
        name: "case_separator",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            Optional(element: RuleReference("PIPE", isToken: true))
          ]),
          RuleReference("PIPE", isToken: true)
        ]),
        lineNumber: 91),
    GrammarRule(
        name: "attribute_section",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("attribute", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("attribute", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 94),
    GrammarRule(
        name: "attribute",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("attribute_target", isToken: false),
            RuleReference("COLON", isToken: true)
          ])),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("attribute_arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ]))
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "attribute_target",
        body: Alternation(choices: [
          Literal("assembly"),
          Literal("field"),
          Literal("method"),
          Literal("module"),
          Literal("param"),
          Literal("property"),
          Literal("return"),
          Literal("type"),
          Literal("event")
        ]),
        lineNumber: 98),
    GrammarRule(
        name: "attribute_arguments",
        body: Sequence(elements: [
          RuleReference("attribute_argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("attribute_argument", isToken: false)
          ]))
        ]),
        lineNumber: 108),
    GrammarRule(
        name: "attribute_argument",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true)
          ])),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 110),
    GrammarRule(
        name: "parameter_list",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false))
        ]),
        lineNumber: 112),
    GrammarRule(
        name: "type_annotation",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 114),
    GrammarRule(
        name: "type_expression",
        body: RuleReference("function_type", isToken: false),
        lineNumber: 116),
    GrammarRule(
        name: "function_type",
        body: Sequence(elements: [
          RuleReference("type_product", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("function_type", isToken: false)
          ]))
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "type_product",
        body: Sequence(elements: [
          RuleReference("type_application", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("type_application", isToken: false)
          ]))
        ]),
        lineNumber: 120),
    GrammarRule(
        name: "type_application",
        body: Sequence(elements: [
          RuleReference("type_atom", isToken: false),
          Repetition(element: RuleReference("type_atom", isToken: false))
        ]),
        lineNumber: 122),
    GrammarRule(
        name: "generic_type_arguments",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 124),
    GrammarRule(
        name: "type_atom",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("qualified_name", isToken: false),
            Optional(
                element:
                    RuleReference("generic_type_arguments", isToken: false))
          ]),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("tuple_type", isToken: false),
          RuleReference("unit_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("parenthesized_type", isToken: false)
        ]),
        lineNumber: 126),
    GrammarRule(
        name: "tuple_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 133),
    GrammarRule(
        name: "unit_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 135),
    GrammarRule(
        name: "parenthesized_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 137),
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
        lineNumber: 139),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("sequence_expression", isToken: false)
        ]),
        lineNumber: 141),
    GrammarRule(
        name: "sequence_expression",
        body: Sequence(elements: [
          RuleReference("infix_expression", isToken: false),
          Repetition(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("NEWLINE", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ]),
            Sequence(elements: [
              RuleReference("SEMICOLON", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 150),
    GrammarRule(
        name: "infix_expression",
        body: Sequence(elements: [
          RuleReference("application_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("infix_operator", isToken: false),
            RuleReference("application_expression", isToken: false)
          ]))
        ]),
        lineNumber: 152),
    GrammarRule(
        name: "infix_operator",
        body: Alternation(choices: [
          RuleReference("PIPE_RIGHT", isToken: true),
          RuleReference("PIPE_LEFT", isToken: true),
          RuleReference("COMPOSE_RIGHT", isToken: true),
          RuleReference("COMPOSE_LEFT", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("COLON_EQUALS", isToken: true),
          RuleReference("DOUBLE_COLON", isToken: true),
          RuleReference("LARROW", isToken: true),
          RuleReference("ARROW", isToken: true),
          RuleReference("DOT_DOT", isToken: true),
          RuleReference("AND_AND", isToken: true),
          RuleReference("OR_OR", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("PERCENT", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          RuleReference("PIPE", isToken: true),
          RuleReference("CARET", isToken: true)
        ]),
        lineNumber: 154),
    GrammarRule(
        name: "application_expression",
        body: Sequence(elements: [
          RuleReference("prefix_expression", isToken: false),
          Repetition(
              element: RuleReference("prefix_expression", isToken: false))
        ]),
        lineNumber: 181),
    GrammarRule(
        name: "prefix_expression",
        body: Sequence(elements: [
          Optional(element: RuleReference("unary_operator", isToken: false)),
          RuleReference("atomic_expression", isToken: false)
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "unary_operator",
        body: Alternation(choices: [
          RuleReference("MINUS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("BANG", isToken: true)
        ]),
        lineNumber: 185),
    GrammarRule(
        name: "atomic_expression",
        body: Alternation(choices: [
          RuleReference("computation_expression", isToken: false),
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("unit_expression", isToken: false),
          RuleReference("tuple_expression", isToken: false),
          RuleReference("list_expression", isToken: false),
          RuleReference("array_expression", isToken: false),
          RuleReference("record_expression", isToken: false),
          RuleReference("parenthesized_expression", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 189),
    GrammarRule(
        name: "computation_expression",
        body: Sequence(elements: [
          RuleReference("qualified_name", isToken: false),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("expression", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 211),
    GrammarRule(
        name: "unit_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 213),
    GrammarRule(
        name: "tuple_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 215),
    GrammarRule(
        name: "list_expression",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          NegativeLookahead(element: RuleReference("LESS_THAN", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 217),
    GrammarRule(
        name: "array_expression",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 219),
    GrammarRule(
        name: "record_expression",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_assignment", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_assignment", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 221),
    GrammarRule(
        name: "field_assignment",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 223),
    GrammarRule(
        name: "parenthesized_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 225),
    GrammarRule(
        name: "element_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 227),
    GrammarRule(
        name: "if_expression",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          Literal("then"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 230),
    GrammarRule(
        name: "match_expression",
        body: Sequence(elements: [
          Literal("match"),
          RuleReference("expression", isToken: false),
          Literal("with"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 232),
    GrammarRule(
        name: "match_case",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("when"),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 234),
    GrammarRule(
        name: "fun_expression",
        body: Sequence(elements: [
          Literal("fun"),
          RuleReference("parameter_list", isToken: false),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 236),
    GrammarRule(
        name: "function_expression",
        body: Sequence(elements: [
          Literal("function"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 238),
    GrammarRule(
        name: "let_expression",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ])),
          Literal("in"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 240),
    GrammarRule(
        name: "for_expression",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("pattern", isToken: false),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false),
          Group(
              element:
                  Alternation(choices: [Literal("to"), Literal("downto")])),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 242),
    GrammarRule(
        name: "while_expression",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 244),
    GrammarRule(
        name: "pattern",
        body: Sequence(elements: [
          RuleReference("pattern_atom", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 246),
    GrammarRule(
        name: "pattern_atom",
        body: Alternation(choices: [
          RuleReference("wildcard_pattern", isToken: false),
          RuleReference("literal_pattern", isToken: false),
          RuleReference("tuple_pattern", isToken: false),
          RuleReference("list_pattern", isToken: false),
          RuleReference("array_pattern", isToken: false),
          RuleReference("record_pattern", isToken: false),
          RuleReference("unit_pattern", isToken: false),
          RuleReference("parenthesized_pattern", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 248),
    GrammarRule(
        name: "wildcard_pattern",
        body: RuleReference("UNDERSCORE", isToken: true),
        lineNumber: 260),
    GrammarRule(
        name: "literal_pattern",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 262),
    GrammarRule(
        name: "tuple_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("pattern", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 269),
    GrammarRule(
        name: "list_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 271),
    GrammarRule(
        name: "array_pattern",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 273),
    GrammarRule(
        name: "record_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 275),
    GrammarRule(
        name: "field_pattern",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("pattern", isToken: false)
        ]),
        lineNumber: 277),
    GrammarRule(
        name: "unit_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 279),
    GrammarRule(
        name: "parenthesized_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 281),
  ],
);

final parserGrammarV4_5 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "compilation_unit",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("decorated_declaration", isToken: false)
        ])),
        lineNumber: 9),
    GrammarRule(
        name: "decorated_declaration",
        body: Sequence(elements: [
          Repetition(
              element: Sequence(elements: [
            RuleReference("attribute_section", isToken: false),
            RuleReference("NEWLINE", isToken: true)
          ])),
          RuleReference("declaration_body", isToken: false)
        ]),
        lineNumber: 11),
    GrammarRule(
        name: "declaration_body",
        body: Alternation(choices: [
          RuleReference("module_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("open_declaration", isToken: false),
          RuleReference("let_binding", isToken: false),
          RuleReference("use_binding", isToken: false),
          RuleReference("type_declaration", isToken: false),
          RuleReference("member_declaration", isToken: false),
          RuleReference("do_binding", isToken: false),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 13),
    GrammarRule(
        name: "module_declaration",
        body: Sequence(elements: [
          Literal("module"),
          Repetition(element: RuleReference("module_modifier", isToken: false)),
          Optional(element: Literal("rec")),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ]))
        ]),
        lineNumber: 23),
    GrammarRule(
        name: "module_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 25),
    GrammarRule(
        name: "namespace_declaration",
        body: Sequence(elements: [
          Literal("namespace"),
          Repetition(
              element: RuleReference("namespace_modifier", isToken: false)),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 28),
    GrammarRule(
        name: "namespace_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 30),
    GrammarRule(
        name: "open_declaration",
        body: Sequence(elements: [
          Literal("open"),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 33),
    GrammarRule(
        name: "let_binding",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 35),
    GrammarRule(
        name: "use_binding",
        body: Sequence(elements: [
          Literal("use"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 37),
    GrammarRule(
        name: "binding_modifier",
        body: Alternation(choices: [Literal("inline"), Literal("mutable")]),
        lineNumber: 39),
    GrammarRule(
        name: "binding_clause",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          RuleReference("EQUALS", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 42),
    GrammarRule(
        name: "do_binding",
        body: Sequence(elements: [
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 44),
    GrammarRule(
        name: "member_declaration",
        body: Sequence(elements: [
          Optional(element: RuleReference("member_modifier", isToken: false)),
          Literal("member"),
          RuleReference("qualified_name", isToken: false),
          Optional(element: RuleReference("parameter_list", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 46),
    GrammarRule(
        name: "member_modifier",
        body: Alternation(choices: [
          Literal("static"),
          Literal("override"),
          Literal("default"),
          Literal("abstract"),
          Literal("new")
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "type_declaration",
        body: Sequence(elements: [
          Literal("type"),
          Repetition(element: RuleReference("type_modifier", isToken: false)),
          RuleReference("NAME", isToken: true),
          Optional(
              element: RuleReference("generic_parameters", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ])),
          RuleReference("type_definition", isToken: false)
        ]),
        lineNumber: 54),
    GrammarRule(
        name: "type_modifier",
        body: Alternation(choices: [
          Literal("private"),
          Literal("public"),
          Literal("internal")
        ]),
        lineNumber: 56),
    GrammarRule(
        name: "generic_parameters",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_parameter", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 60),
    GrammarRule(
        name: "type_parameter",
        body: Alternation(choices: [
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 62),
    GrammarRule(
        name: "type_definition",
        body: Alternation(choices: [
          RuleReference("class_type", isToken: false),
          RuleReference("interface_type", isToken: false),
          RuleReference("struct_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("union_type", isToken: false),
          RuleReference("alias_type", isToken: false)
        ]),
        lineNumber: 65),
    GrammarRule(
        name: "class_type",
        body: Sequence(elements: [
          Literal("class"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 72),
    GrammarRule(
        name: "interface_type",
        body: Sequence(elements: [
          Literal("interface"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 74),
    GrammarRule(
        name: "struct_type",
        body: Sequence(elements: [
          Literal("struct"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "record_type",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_declaration", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_declaration", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 78),
    GrammarRule(
        name: "field_declaration",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 80),
    GrammarRule(
        name: "union_type",
        body: Sequence(elements: [
          RuleReference("union_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("union_case", isToken: false)
          ]))
        ]),
        lineNumber: 82),
    GrammarRule(
        name: "union_case",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("of"),
            RuleReference("type_expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("STAR", isToken: true),
              RuleReference("type_expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 84),
    GrammarRule(
        name: "alias_type",
        body: RuleReference("type_expression", isToken: false),
        lineNumber: 86),
    GrammarRule(
        name: "field_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 88),
    GrammarRule(
        name: "case_separator",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            Optional(element: RuleReference("PIPE", isToken: true))
          ]),
          RuleReference("PIPE", isToken: true)
        ]),
        lineNumber: 91),
    GrammarRule(
        name: "attribute_section",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("attribute", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("attribute", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 94),
    GrammarRule(
        name: "attribute",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("attribute_target", isToken: false),
            RuleReference("COLON", isToken: true)
          ])),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("attribute_arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ]))
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "attribute_target",
        body: Alternation(choices: [
          Literal("assembly"),
          Literal("field"),
          Literal("method"),
          Literal("module"),
          Literal("param"),
          Literal("property"),
          Literal("return"),
          Literal("type"),
          Literal("event")
        ]),
        lineNumber: 98),
    GrammarRule(
        name: "attribute_arguments",
        body: Sequence(elements: [
          RuleReference("attribute_argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("attribute_argument", isToken: false)
          ]))
        ]),
        lineNumber: 108),
    GrammarRule(
        name: "attribute_argument",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true)
          ])),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 110),
    GrammarRule(
        name: "parameter_list",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false))
        ]),
        lineNumber: 112),
    GrammarRule(
        name: "type_annotation",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 114),
    GrammarRule(
        name: "type_expression",
        body: RuleReference("function_type", isToken: false),
        lineNumber: 116),
    GrammarRule(
        name: "function_type",
        body: Sequence(elements: [
          RuleReference("type_product", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("function_type", isToken: false)
          ]))
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "type_product",
        body: Sequence(elements: [
          RuleReference("type_application", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("type_application", isToken: false)
          ]))
        ]),
        lineNumber: 120),
    GrammarRule(
        name: "type_application",
        body: Sequence(elements: [
          RuleReference("type_atom", isToken: false),
          Repetition(element: RuleReference("type_atom", isToken: false))
        ]),
        lineNumber: 122),
    GrammarRule(
        name: "generic_type_arguments",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 124),
    GrammarRule(
        name: "type_atom",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("qualified_name", isToken: false),
            Optional(
                element:
                    RuleReference("generic_type_arguments", isToken: false))
          ]),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("tuple_type", isToken: false),
          RuleReference("unit_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("parenthesized_type", isToken: false)
        ]),
        lineNumber: 126),
    GrammarRule(
        name: "tuple_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 133),
    GrammarRule(
        name: "unit_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 135),
    GrammarRule(
        name: "parenthesized_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 137),
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
        lineNumber: 139),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("sequence_expression", isToken: false)
        ]),
        lineNumber: 141),
    GrammarRule(
        name: "sequence_expression",
        body: Sequence(elements: [
          RuleReference("infix_expression", isToken: false),
          Repetition(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("NEWLINE", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ]),
            Sequence(elements: [
              RuleReference("SEMICOLON", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 150),
    GrammarRule(
        name: "infix_expression",
        body: Sequence(elements: [
          RuleReference("application_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("infix_operator", isToken: false),
            RuleReference("application_expression", isToken: false)
          ]))
        ]),
        lineNumber: 152),
    GrammarRule(
        name: "infix_operator",
        body: Alternation(choices: [
          RuleReference("PIPE_RIGHT", isToken: true),
          RuleReference("PIPE_LEFT", isToken: true),
          RuleReference("COMPOSE_RIGHT", isToken: true),
          RuleReference("COMPOSE_LEFT", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("COLON_EQUALS", isToken: true),
          RuleReference("DOUBLE_COLON", isToken: true),
          RuleReference("LARROW", isToken: true),
          RuleReference("ARROW", isToken: true),
          RuleReference("DOT_DOT", isToken: true),
          RuleReference("AND_AND", isToken: true),
          RuleReference("OR_OR", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("PERCENT", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          RuleReference("PIPE", isToken: true),
          RuleReference("CARET", isToken: true)
        ]),
        lineNumber: 154),
    GrammarRule(
        name: "application_expression",
        body: Sequence(elements: [
          RuleReference("prefix_expression", isToken: false),
          Repetition(
              element: RuleReference("prefix_expression", isToken: false))
        ]),
        lineNumber: 181),
    GrammarRule(
        name: "prefix_expression",
        body: Sequence(elements: [
          Optional(element: RuleReference("unary_operator", isToken: false)),
          RuleReference("atomic_expression", isToken: false)
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "unary_operator",
        body: Alternation(choices: [
          RuleReference("MINUS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("BANG", isToken: true)
        ]),
        lineNumber: 185),
    GrammarRule(
        name: "atomic_expression",
        body: Alternation(choices: [
          RuleReference("computation_expression", isToken: false),
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("unit_expression", isToken: false),
          RuleReference("tuple_expression", isToken: false),
          RuleReference("list_expression", isToken: false),
          RuleReference("array_expression", isToken: false),
          RuleReference("record_expression", isToken: false),
          RuleReference("parenthesized_expression", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 189),
    GrammarRule(
        name: "computation_expression",
        body: Sequence(elements: [
          RuleReference("qualified_name", isToken: false),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("expression", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 211),
    GrammarRule(
        name: "unit_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 213),
    GrammarRule(
        name: "tuple_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 215),
    GrammarRule(
        name: "list_expression",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          NegativeLookahead(element: RuleReference("LESS_THAN", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 217),
    GrammarRule(
        name: "array_expression",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 219),
    GrammarRule(
        name: "record_expression",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_assignment", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_assignment", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 221),
    GrammarRule(
        name: "field_assignment",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 223),
    GrammarRule(
        name: "parenthesized_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 225),
    GrammarRule(
        name: "element_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 227),
    GrammarRule(
        name: "if_expression",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          Literal("then"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 230),
    GrammarRule(
        name: "match_expression",
        body: Sequence(elements: [
          Literal("match"),
          RuleReference("expression", isToken: false),
          Literal("with"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 232),
    GrammarRule(
        name: "match_case",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("when"),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 234),
    GrammarRule(
        name: "fun_expression",
        body: Sequence(elements: [
          Literal("fun"),
          RuleReference("parameter_list", isToken: false),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 236),
    GrammarRule(
        name: "function_expression",
        body: Sequence(elements: [
          Literal("function"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 238),
    GrammarRule(
        name: "let_expression",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ])),
          Literal("in"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 240),
    GrammarRule(
        name: "for_expression",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("pattern", isToken: false),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false),
          Group(
              element:
                  Alternation(choices: [Literal("to"), Literal("downto")])),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 242),
    GrammarRule(
        name: "while_expression",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 244),
    GrammarRule(
        name: "pattern",
        body: Sequence(elements: [
          RuleReference("pattern_atom", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 246),
    GrammarRule(
        name: "pattern_atom",
        body: Alternation(choices: [
          RuleReference("wildcard_pattern", isToken: false),
          RuleReference("literal_pattern", isToken: false),
          RuleReference("tuple_pattern", isToken: false),
          RuleReference("list_pattern", isToken: false),
          RuleReference("array_pattern", isToken: false),
          RuleReference("record_pattern", isToken: false),
          RuleReference("unit_pattern", isToken: false),
          RuleReference("parenthesized_pattern", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 248),
    GrammarRule(
        name: "wildcard_pattern",
        body: RuleReference("UNDERSCORE", isToken: true),
        lineNumber: 260),
    GrammarRule(
        name: "literal_pattern",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 262),
    GrammarRule(
        name: "tuple_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("pattern", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 269),
    GrammarRule(
        name: "list_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 271),
    GrammarRule(
        name: "array_pattern",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 273),
    GrammarRule(
        name: "record_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 275),
    GrammarRule(
        name: "field_pattern",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("pattern", isToken: false)
        ]),
        lineNumber: 277),
    GrammarRule(
        name: "unit_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 279),
    GrammarRule(
        name: "parenthesized_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 281),
  ],
);

final parserGrammarV4_6 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "compilation_unit",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("decorated_declaration", isToken: false)
        ])),
        lineNumber: 9),
    GrammarRule(
        name: "decorated_declaration",
        body: Sequence(elements: [
          Repetition(
              element: Sequence(elements: [
            RuleReference("attribute_section", isToken: false),
            RuleReference("NEWLINE", isToken: true)
          ])),
          RuleReference("declaration_body", isToken: false)
        ]),
        lineNumber: 11),
    GrammarRule(
        name: "declaration_body",
        body: Alternation(choices: [
          RuleReference("module_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("open_declaration", isToken: false),
          RuleReference("let_binding", isToken: false),
          RuleReference("use_binding", isToken: false),
          RuleReference("type_declaration", isToken: false),
          RuleReference("member_declaration", isToken: false),
          RuleReference("do_binding", isToken: false),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 13),
    GrammarRule(
        name: "module_declaration",
        body: Sequence(elements: [
          Literal("module"),
          Repetition(element: RuleReference("module_modifier", isToken: false)),
          Optional(element: Literal("rec")),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ]))
        ]),
        lineNumber: 23),
    GrammarRule(
        name: "module_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 25),
    GrammarRule(
        name: "namespace_declaration",
        body: Sequence(elements: [
          Literal("namespace"),
          Repetition(
              element: RuleReference("namespace_modifier", isToken: false)),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 28),
    GrammarRule(
        name: "namespace_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 30),
    GrammarRule(
        name: "open_declaration",
        body: Sequence(elements: [
          Literal("open"),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 33),
    GrammarRule(
        name: "let_binding",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 35),
    GrammarRule(
        name: "use_binding",
        body: Sequence(elements: [
          Literal("use"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 37),
    GrammarRule(
        name: "binding_modifier",
        body: Alternation(choices: [Literal("inline"), Literal("mutable")]),
        lineNumber: 39),
    GrammarRule(
        name: "binding_clause",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          RuleReference("EQUALS", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 42),
    GrammarRule(
        name: "do_binding",
        body: Sequence(elements: [
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 44),
    GrammarRule(
        name: "member_declaration",
        body: Sequence(elements: [
          Optional(element: RuleReference("member_modifier", isToken: false)),
          Literal("member"),
          RuleReference("qualified_name", isToken: false),
          Optional(element: RuleReference("parameter_list", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 46),
    GrammarRule(
        name: "member_modifier",
        body: Alternation(choices: [
          Literal("static"),
          Literal("override"),
          Literal("default"),
          Literal("abstract"),
          Literal("new")
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "type_declaration",
        body: Sequence(elements: [
          Literal("type"),
          Repetition(element: RuleReference("type_modifier", isToken: false)),
          RuleReference("NAME", isToken: true),
          Optional(
              element: RuleReference("generic_parameters", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ])),
          RuleReference("type_definition", isToken: false)
        ]),
        lineNumber: 54),
    GrammarRule(
        name: "type_modifier",
        body: Alternation(choices: [
          Literal("private"),
          Literal("public"),
          Literal("internal")
        ]),
        lineNumber: 56),
    GrammarRule(
        name: "generic_parameters",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_parameter", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 60),
    GrammarRule(
        name: "type_parameter",
        body: Alternation(choices: [
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 62),
    GrammarRule(
        name: "type_definition",
        body: Alternation(choices: [
          RuleReference("class_type", isToken: false),
          RuleReference("interface_type", isToken: false),
          RuleReference("struct_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("union_type", isToken: false),
          RuleReference("alias_type", isToken: false)
        ]),
        lineNumber: 65),
    GrammarRule(
        name: "class_type",
        body: Sequence(elements: [
          Literal("class"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 72),
    GrammarRule(
        name: "interface_type",
        body: Sequence(elements: [
          Literal("interface"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 74),
    GrammarRule(
        name: "struct_type",
        body: Sequence(elements: [
          Literal("struct"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "record_type",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_declaration", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_declaration", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 78),
    GrammarRule(
        name: "field_declaration",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 80),
    GrammarRule(
        name: "union_type",
        body: Sequence(elements: [
          RuleReference("union_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("union_case", isToken: false)
          ]))
        ]),
        lineNumber: 82),
    GrammarRule(
        name: "union_case",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("of"),
            RuleReference("type_expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("STAR", isToken: true),
              RuleReference("type_expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 84),
    GrammarRule(
        name: "alias_type",
        body: RuleReference("type_expression", isToken: false),
        lineNumber: 86),
    GrammarRule(
        name: "field_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 88),
    GrammarRule(
        name: "case_separator",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            Optional(element: RuleReference("PIPE", isToken: true))
          ]),
          RuleReference("PIPE", isToken: true)
        ]),
        lineNumber: 91),
    GrammarRule(
        name: "attribute_section",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("attribute", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("attribute", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 94),
    GrammarRule(
        name: "attribute",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("attribute_target", isToken: false),
            RuleReference("COLON", isToken: true)
          ])),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("attribute_arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ]))
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "attribute_target",
        body: Alternation(choices: [
          Literal("assembly"),
          Literal("field"),
          Literal("method"),
          Literal("module"),
          Literal("param"),
          Literal("property"),
          Literal("return"),
          Literal("type"),
          Literal("event")
        ]),
        lineNumber: 98),
    GrammarRule(
        name: "attribute_arguments",
        body: Sequence(elements: [
          RuleReference("attribute_argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("attribute_argument", isToken: false)
          ]))
        ]),
        lineNumber: 108),
    GrammarRule(
        name: "attribute_argument",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true)
          ])),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 110),
    GrammarRule(
        name: "parameter_list",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false))
        ]),
        lineNumber: 112),
    GrammarRule(
        name: "type_annotation",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 114),
    GrammarRule(
        name: "type_expression",
        body: RuleReference("function_type", isToken: false),
        lineNumber: 116),
    GrammarRule(
        name: "function_type",
        body: Sequence(elements: [
          RuleReference("type_product", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("function_type", isToken: false)
          ]))
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "type_product",
        body: Sequence(elements: [
          RuleReference("type_application", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("type_application", isToken: false)
          ]))
        ]),
        lineNumber: 120),
    GrammarRule(
        name: "type_application",
        body: Sequence(elements: [
          RuleReference("type_atom", isToken: false),
          Repetition(element: RuleReference("type_atom", isToken: false))
        ]),
        lineNumber: 122),
    GrammarRule(
        name: "generic_type_arguments",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 124),
    GrammarRule(
        name: "type_atom",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("qualified_name", isToken: false),
            Optional(
                element:
                    RuleReference("generic_type_arguments", isToken: false))
          ]),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("tuple_type", isToken: false),
          RuleReference("unit_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("parenthesized_type", isToken: false)
        ]),
        lineNumber: 126),
    GrammarRule(
        name: "tuple_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 133),
    GrammarRule(
        name: "unit_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 135),
    GrammarRule(
        name: "parenthesized_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 137),
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
        lineNumber: 139),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("sequence_expression", isToken: false)
        ]),
        lineNumber: 141),
    GrammarRule(
        name: "sequence_expression",
        body: Sequence(elements: [
          RuleReference("infix_expression", isToken: false),
          Repetition(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("NEWLINE", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ]),
            Sequence(elements: [
              RuleReference("SEMICOLON", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 150),
    GrammarRule(
        name: "infix_expression",
        body: Sequence(elements: [
          RuleReference("application_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("infix_operator", isToken: false),
            RuleReference("application_expression", isToken: false)
          ]))
        ]),
        lineNumber: 152),
    GrammarRule(
        name: "infix_operator",
        body: Alternation(choices: [
          RuleReference("PIPE_RIGHT", isToken: true),
          RuleReference("PIPE_LEFT", isToken: true),
          RuleReference("COMPOSE_RIGHT", isToken: true),
          RuleReference("COMPOSE_LEFT", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("COLON_EQUALS", isToken: true),
          RuleReference("DOUBLE_COLON", isToken: true),
          RuleReference("LARROW", isToken: true),
          RuleReference("ARROW", isToken: true),
          RuleReference("DOT_DOT", isToken: true),
          RuleReference("AND_AND", isToken: true),
          RuleReference("OR_OR", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("PERCENT", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          RuleReference("PIPE", isToken: true),
          RuleReference("CARET", isToken: true)
        ]),
        lineNumber: 154),
    GrammarRule(
        name: "application_expression",
        body: Sequence(elements: [
          RuleReference("prefix_expression", isToken: false),
          Repetition(
              element: RuleReference("prefix_expression", isToken: false))
        ]),
        lineNumber: 181),
    GrammarRule(
        name: "prefix_expression",
        body: Sequence(elements: [
          Optional(element: RuleReference("unary_operator", isToken: false)),
          RuleReference("atomic_expression", isToken: false)
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "unary_operator",
        body: Alternation(choices: [
          RuleReference("MINUS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("BANG", isToken: true)
        ]),
        lineNumber: 185),
    GrammarRule(
        name: "atomic_expression",
        body: Alternation(choices: [
          RuleReference("computation_expression", isToken: false),
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("unit_expression", isToken: false),
          RuleReference("tuple_expression", isToken: false),
          RuleReference("list_expression", isToken: false),
          RuleReference("array_expression", isToken: false),
          RuleReference("record_expression", isToken: false),
          RuleReference("parenthesized_expression", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 189),
    GrammarRule(
        name: "computation_expression",
        body: Sequence(elements: [
          RuleReference("qualified_name", isToken: false),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("expression", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 211),
    GrammarRule(
        name: "unit_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 213),
    GrammarRule(
        name: "tuple_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 215),
    GrammarRule(
        name: "list_expression",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          NegativeLookahead(element: RuleReference("LESS_THAN", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 217),
    GrammarRule(
        name: "array_expression",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 219),
    GrammarRule(
        name: "record_expression",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_assignment", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_assignment", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 221),
    GrammarRule(
        name: "field_assignment",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 223),
    GrammarRule(
        name: "parenthesized_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 225),
    GrammarRule(
        name: "element_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 227),
    GrammarRule(
        name: "if_expression",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          Literal("then"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 230),
    GrammarRule(
        name: "match_expression",
        body: Sequence(elements: [
          Literal("match"),
          RuleReference("expression", isToken: false),
          Literal("with"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 232),
    GrammarRule(
        name: "match_case",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("when"),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 234),
    GrammarRule(
        name: "fun_expression",
        body: Sequence(elements: [
          Literal("fun"),
          RuleReference("parameter_list", isToken: false),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 236),
    GrammarRule(
        name: "function_expression",
        body: Sequence(elements: [
          Literal("function"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 238),
    GrammarRule(
        name: "let_expression",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ])),
          Literal("in"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 240),
    GrammarRule(
        name: "for_expression",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("pattern", isToken: false),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false),
          Group(
              element:
                  Alternation(choices: [Literal("to"), Literal("downto")])),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 242),
    GrammarRule(
        name: "while_expression",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 244),
    GrammarRule(
        name: "pattern",
        body: Sequence(elements: [
          RuleReference("pattern_atom", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 246),
    GrammarRule(
        name: "pattern_atom",
        body: Alternation(choices: [
          RuleReference("wildcard_pattern", isToken: false),
          RuleReference("literal_pattern", isToken: false),
          RuleReference("tuple_pattern", isToken: false),
          RuleReference("list_pattern", isToken: false),
          RuleReference("array_pattern", isToken: false),
          RuleReference("record_pattern", isToken: false),
          RuleReference("unit_pattern", isToken: false),
          RuleReference("parenthesized_pattern", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 248),
    GrammarRule(
        name: "wildcard_pattern",
        body: RuleReference("UNDERSCORE", isToken: true),
        lineNumber: 260),
    GrammarRule(
        name: "literal_pattern",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 262),
    GrammarRule(
        name: "tuple_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("pattern", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 269),
    GrammarRule(
        name: "list_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 271),
    GrammarRule(
        name: "array_pattern",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 273),
    GrammarRule(
        name: "record_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 275),
    GrammarRule(
        name: "field_pattern",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("pattern", isToken: false)
        ]),
        lineNumber: 277),
    GrammarRule(
        name: "unit_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 279),
    GrammarRule(
        name: "parenthesized_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 281),
  ],
);

final parserGrammarV4_7 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "compilation_unit",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("decorated_declaration", isToken: false)
        ])),
        lineNumber: 9),
    GrammarRule(
        name: "decorated_declaration",
        body: Sequence(elements: [
          Repetition(
              element: Sequence(elements: [
            RuleReference("attribute_section", isToken: false),
            RuleReference("NEWLINE", isToken: true)
          ])),
          RuleReference("declaration_body", isToken: false)
        ]),
        lineNumber: 11),
    GrammarRule(
        name: "declaration_body",
        body: Alternation(choices: [
          RuleReference("module_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("open_declaration", isToken: false),
          RuleReference("let_binding", isToken: false),
          RuleReference("use_binding", isToken: false),
          RuleReference("type_declaration", isToken: false),
          RuleReference("member_declaration", isToken: false),
          RuleReference("do_binding", isToken: false),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 13),
    GrammarRule(
        name: "module_declaration",
        body: Sequence(elements: [
          Literal("module"),
          Repetition(element: RuleReference("module_modifier", isToken: false)),
          Optional(element: Literal("rec")),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ]))
        ]),
        lineNumber: 23),
    GrammarRule(
        name: "module_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 25),
    GrammarRule(
        name: "namespace_declaration",
        body: Sequence(elements: [
          Literal("namespace"),
          Repetition(
              element: RuleReference("namespace_modifier", isToken: false)),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 28),
    GrammarRule(
        name: "namespace_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 30),
    GrammarRule(
        name: "open_declaration",
        body: Sequence(elements: [
          Literal("open"),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 33),
    GrammarRule(
        name: "let_binding",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 35),
    GrammarRule(
        name: "use_binding",
        body: Sequence(elements: [
          Literal("use"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 37),
    GrammarRule(
        name: "binding_modifier",
        body: Alternation(choices: [Literal("inline"), Literal("mutable")]),
        lineNumber: 39),
    GrammarRule(
        name: "binding_clause",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          RuleReference("EQUALS", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 42),
    GrammarRule(
        name: "do_binding",
        body: Sequence(elements: [
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 44),
    GrammarRule(
        name: "member_declaration",
        body: Sequence(elements: [
          Optional(element: RuleReference("member_modifier", isToken: false)),
          Literal("member"),
          RuleReference("qualified_name", isToken: false),
          Optional(element: RuleReference("parameter_list", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 46),
    GrammarRule(
        name: "member_modifier",
        body: Alternation(choices: [
          Literal("static"),
          Literal("override"),
          Literal("default"),
          Literal("abstract"),
          Literal("new")
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "type_declaration",
        body: Sequence(elements: [
          Literal("type"),
          Repetition(element: RuleReference("type_modifier", isToken: false)),
          RuleReference("NAME", isToken: true),
          Optional(
              element: RuleReference("generic_parameters", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ])),
          RuleReference("type_definition", isToken: false)
        ]),
        lineNumber: 54),
    GrammarRule(
        name: "type_modifier",
        body: Alternation(choices: [
          Literal("private"),
          Literal("public"),
          Literal("internal")
        ]),
        lineNumber: 56),
    GrammarRule(
        name: "generic_parameters",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_parameter", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 60),
    GrammarRule(
        name: "type_parameter",
        body: Alternation(choices: [
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 62),
    GrammarRule(
        name: "type_definition",
        body: Alternation(choices: [
          RuleReference("class_type", isToken: false),
          RuleReference("interface_type", isToken: false),
          RuleReference("struct_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("union_type", isToken: false),
          RuleReference("alias_type", isToken: false)
        ]),
        lineNumber: 65),
    GrammarRule(
        name: "class_type",
        body: Sequence(elements: [
          Literal("class"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 72),
    GrammarRule(
        name: "interface_type",
        body: Sequence(elements: [
          Literal("interface"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 74),
    GrammarRule(
        name: "struct_type",
        body: Sequence(elements: [
          Literal("struct"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "record_type",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_declaration", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_declaration", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 78),
    GrammarRule(
        name: "field_declaration",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 80),
    GrammarRule(
        name: "union_type",
        body: Sequence(elements: [
          RuleReference("union_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("union_case", isToken: false)
          ]))
        ]),
        lineNumber: 82),
    GrammarRule(
        name: "union_case",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("of"),
            RuleReference("type_expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("STAR", isToken: true),
              RuleReference("type_expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 84),
    GrammarRule(
        name: "alias_type",
        body: RuleReference("type_expression", isToken: false),
        lineNumber: 86),
    GrammarRule(
        name: "field_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 88),
    GrammarRule(
        name: "case_separator",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            Optional(element: RuleReference("PIPE", isToken: true))
          ]),
          RuleReference("PIPE", isToken: true)
        ]),
        lineNumber: 91),
    GrammarRule(
        name: "attribute_section",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("attribute", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("attribute", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 94),
    GrammarRule(
        name: "attribute",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("attribute_target", isToken: false),
            RuleReference("COLON", isToken: true)
          ])),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("attribute_arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ]))
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "attribute_target",
        body: Alternation(choices: [
          Literal("assembly"),
          Literal("field"),
          Literal("method"),
          Literal("module"),
          Literal("param"),
          Literal("property"),
          Literal("return"),
          Literal("type"),
          Literal("event")
        ]),
        lineNumber: 98),
    GrammarRule(
        name: "attribute_arguments",
        body: Sequence(elements: [
          RuleReference("attribute_argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("attribute_argument", isToken: false)
          ]))
        ]),
        lineNumber: 108),
    GrammarRule(
        name: "attribute_argument",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true)
          ])),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 110),
    GrammarRule(
        name: "parameter_list",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false))
        ]),
        lineNumber: 112),
    GrammarRule(
        name: "type_annotation",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 114),
    GrammarRule(
        name: "type_expression",
        body: RuleReference("function_type", isToken: false),
        lineNumber: 116),
    GrammarRule(
        name: "function_type",
        body: Sequence(elements: [
          RuleReference("type_product", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("function_type", isToken: false)
          ]))
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "type_product",
        body: Sequence(elements: [
          RuleReference("type_application", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("type_application", isToken: false)
          ]))
        ]),
        lineNumber: 120),
    GrammarRule(
        name: "type_application",
        body: Sequence(elements: [
          RuleReference("type_atom", isToken: false),
          Repetition(element: RuleReference("type_atom", isToken: false))
        ]),
        lineNumber: 122),
    GrammarRule(
        name: "generic_type_arguments",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 124),
    GrammarRule(
        name: "type_atom",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("qualified_name", isToken: false),
            Optional(
                element:
                    RuleReference("generic_type_arguments", isToken: false))
          ]),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("tuple_type", isToken: false),
          RuleReference("unit_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("parenthesized_type", isToken: false)
        ]),
        lineNumber: 126),
    GrammarRule(
        name: "tuple_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 133),
    GrammarRule(
        name: "unit_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 135),
    GrammarRule(
        name: "parenthesized_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 137),
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
        lineNumber: 139),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("sequence_expression", isToken: false)
        ]),
        lineNumber: 141),
    GrammarRule(
        name: "sequence_expression",
        body: Sequence(elements: [
          RuleReference("infix_expression", isToken: false),
          Repetition(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("NEWLINE", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ]),
            Sequence(elements: [
              RuleReference("SEMICOLON", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 150),
    GrammarRule(
        name: "infix_expression",
        body: Sequence(elements: [
          RuleReference("application_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("infix_operator", isToken: false),
            RuleReference("application_expression", isToken: false)
          ]))
        ]),
        lineNumber: 152),
    GrammarRule(
        name: "infix_operator",
        body: Alternation(choices: [
          RuleReference("PIPE_RIGHT", isToken: true),
          RuleReference("PIPE_LEFT", isToken: true),
          RuleReference("COMPOSE_RIGHT", isToken: true),
          RuleReference("COMPOSE_LEFT", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("COLON_EQUALS", isToken: true),
          RuleReference("DOUBLE_COLON", isToken: true),
          RuleReference("LARROW", isToken: true),
          RuleReference("ARROW", isToken: true),
          RuleReference("DOT_DOT", isToken: true),
          RuleReference("AND_AND", isToken: true),
          RuleReference("OR_OR", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("PERCENT", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          RuleReference("PIPE", isToken: true),
          RuleReference("CARET", isToken: true)
        ]),
        lineNumber: 154),
    GrammarRule(
        name: "application_expression",
        body: Sequence(elements: [
          RuleReference("prefix_expression", isToken: false),
          Repetition(
              element: RuleReference("prefix_expression", isToken: false))
        ]),
        lineNumber: 181),
    GrammarRule(
        name: "prefix_expression",
        body: Sequence(elements: [
          Optional(element: RuleReference("unary_operator", isToken: false)),
          RuleReference("atomic_expression", isToken: false)
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "unary_operator",
        body: Alternation(choices: [
          RuleReference("MINUS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("BANG", isToken: true)
        ]),
        lineNumber: 185),
    GrammarRule(
        name: "atomic_expression",
        body: Alternation(choices: [
          RuleReference("computation_expression", isToken: false),
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("unit_expression", isToken: false),
          RuleReference("tuple_expression", isToken: false),
          RuleReference("list_expression", isToken: false),
          RuleReference("array_expression", isToken: false),
          RuleReference("record_expression", isToken: false),
          RuleReference("parenthesized_expression", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 189),
    GrammarRule(
        name: "computation_expression",
        body: Sequence(elements: [
          RuleReference("qualified_name", isToken: false),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("expression", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 211),
    GrammarRule(
        name: "unit_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 213),
    GrammarRule(
        name: "tuple_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 215),
    GrammarRule(
        name: "list_expression",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          NegativeLookahead(element: RuleReference("LESS_THAN", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 217),
    GrammarRule(
        name: "array_expression",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 219),
    GrammarRule(
        name: "record_expression",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_assignment", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_assignment", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 221),
    GrammarRule(
        name: "field_assignment",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 223),
    GrammarRule(
        name: "parenthesized_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 225),
    GrammarRule(
        name: "element_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 227),
    GrammarRule(
        name: "if_expression",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          Literal("then"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 230),
    GrammarRule(
        name: "match_expression",
        body: Sequence(elements: [
          Literal("match"),
          RuleReference("expression", isToken: false),
          Literal("with"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 232),
    GrammarRule(
        name: "match_case",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("when"),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 234),
    GrammarRule(
        name: "fun_expression",
        body: Sequence(elements: [
          Literal("fun"),
          RuleReference("parameter_list", isToken: false),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 236),
    GrammarRule(
        name: "function_expression",
        body: Sequence(elements: [
          Literal("function"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 238),
    GrammarRule(
        name: "let_expression",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ])),
          Literal("in"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 240),
    GrammarRule(
        name: "for_expression",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("pattern", isToken: false),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false),
          Group(
              element:
                  Alternation(choices: [Literal("to"), Literal("downto")])),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 242),
    GrammarRule(
        name: "while_expression",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 244),
    GrammarRule(
        name: "pattern",
        body: Sequence(elements: [
          RuleReference("pattern_atom", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 246),
    GrammarRule(
        name: "pattern_atom",
        body: Alternation(choices: [
          RuleReference("wildcard_pattern", isToken: false),
          RuleReference("literal_pattern", isToken: false),
          RuleReference("tuple_pattern", isToken: false),
          RuleReference("list_pattern", isToken: false),
          RuleReference("array_pattern", isToken: false),
          RuleReference("record_pattern", isToken: false),
          RuleReference("unit_pattern", isToken: false),
          RuleReference("parenthesized_pattern", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 248),
    GrammarRule(
        name: "wildcard_pattern",
        body: RuleReference("UNDERSCORE", isToken: true),
        lineNumber: 260),
    GrammarRule(
        name: "literal_pattern",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 262),
    GrammarRule(
        name: "tuple_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("pattern", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 269),
    GrammarRule(
        name: "list_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 271),
    GrammarRule(
        name: "array_pattern",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 273),
    GrammarRule(
        name: "record_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 275),
    GrammarRule(
        name: "field_pattern",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("pattern", isToken: false)
        ]),
        lineNumber: 277),
    GrammarRule(
        name: "unit_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 279),
    GrammarRule(
        name: "parenthesized_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 281),
  ],
);

final parserGrammarV5 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "compilation_unit",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("decorated_declaration", isToken: false)
        ])),
        lineNumber: 9),
    GrammarRule(
        name: "decorated_declaration",
        body: Sequence(elements: [
          Repetition(
              element: Sequence(elements: [
            RuleReference("attribute_section", isToken: false),
            RuleReference("NEWLINE", isToken: true)
          ])),
          RuleReference("declaration_body", isToken: false)
        ]),
        lineNumber: 11),
    GrammarRule(
        name: "declaration_body",
        body: Alternation(choices: [
          RuleReference("module_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("open_declaration", isToken: false),
          RuleReference("let_binding", isToken: false),
          RuleReference("use_binding", isToken: false),
          RuleReference("type_declaration", isToken: false),
          RuleReference("member_declaration", isToken: false),
          RuleReference("do_binding", isToken: false),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 13),
    GrammarRule(
        name: "module_declaration",
        body: Sequence(elements: [
          Literal("module"),
          Repetition(element: RuleReference("module_modifier", isToken: false)),
          Optional(element: Literal("rec")),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ]))
        ]),
        lineNumber: 23),
    GrammarRule(
        name: "module_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 25),
    GrammarRule(
        name: "namespace_declaration",
        body: Sequence(elements: [
          Literal("namespace"),
          Repetition(
              element: RuleReference("namespace_modifier", isToken: false)),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 28),
    GrammarRule(
        name: "namespace_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 30),
    GrammarRule(
        name: "open_declaration",
        body: Sequence(elements: [
          Literal("open"),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 33),
    GrammarRule(
        name: "let_binding",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 35),
    GrammarRule(
        name: "use_binding",
        body: Sequence(elements: [
          Literal("use"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 37),
    GrammarRule(
        name: "binding_modifier",
        body: Alternation(choices: [Literal("inline"), Literal("mutable")]),
        lineNumber: 39),
    GrammarRule(
        name: "binding_clause",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          RuleReference("EQUALS", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 42),
    GrammarRule(
        name: "do_binding",
        body: Sequence(elements: [
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 44),
    GrammarRule(
        name: "member_declaration",
        body: Sequence(elements: [
          Optional(element: RuleReference("member_modifier", isToken: false)),
          Literal("member"),
          RuleReference("qualified_name", isToken: false),
          Optional(element: RuleReference("parameter_list", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 46),
    GrammarRule(
        name: "member_modifier",
        body: Alternation(choices: [
          Literal("static"),
          Literal("override"),
          Literal("default"),
          Literal("abstract"),
          Literal("new")
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "type_declaration",
        body: Sequence(elements: [
          Literal("type"),
          Repetition(element: RuleReference("type_modifier", isToken: false)),
          RuleReference("NAME", isToken: true),
          Optional(
              element: RuleReference("generic_parameters", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ])),
          RuleReference("type_definition", isToken: false)
        ]),
        lineNumber: 54),
    GrammarRule(
        name: "type_modifier",
        body: Alternation(choices: [
          Literal("private"),
          Literal("public"),
          Literal("internal")
        ]),
        lineNumber: 56),
    GrammarRule(
        name: "generic_parameters",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_parameter", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 60),
    GrammarRule(
        name: "type_parameter",
        body: Alternation(choices: [
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 62),
    GrammarRule(
        name: "type_definition",
        body: Alternation(choices: [
          RuleReference("class_type", isToken: false),
          RuleReference("interface_type", isToken: false),
          RuleReference("struct_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("union_type", isToken: false),
          RuleReference("alias_type", isToken: false)
        ]),
        lineNumber: 65),
    GrammarRule(
        name: "class_type",
        body: Sequence(elements: [
          Literal("class"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 72),
    GrammarRule(
        name: "interface_type",
        body: Sequence(elements: [
          Literal("interface"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 74),
    GrammarRule(
        name: "struct_type",
        body: Sequence(elements: [
          Literal("struct"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "record_type",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_declaration", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_declaration", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 78),
    GrammarRule(
        name: "field_declaration",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 80),
    GrammarRule(
        name: "union_type",
        body: Sequence(elements: [
          RuleReference("union_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("union_case", isToken: false)
          ]))
        ]),
        lineNumber: 82),
    GrammarRule(
        name: "union_case",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("of"),
            RuleReference("type_expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("STAR", isToken: true),
              RuleReference("type_expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 84),
    GrammarRule(
        name: "alias_type",
        body: RuleReference("type_expression", isToken: false),
        lineNumber: 86),
    GrammarRule(
        name: "field_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 88),
    GrammarRule(
        name: "case_separator",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            Optional(element: RuleReference("PIPE", isToken: true))
          ]),
          RuleReference("PIPE", isToken: true)
        ]),
        lineNumber: 91),
    GrammarRule(
        name: "attribute_section",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("attribute", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("attribute", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 94),
    GrammarRule(
        name: "attribute",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("attribute_target", isToken: false),
            RuleReference("COLON", isToken: true)
          ])),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("attribute_arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ]))
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "attribute_target",
        body: Alternation(choices: [
          Literal("assembly"),
          Literal("field"),
          Literal("method"),
          Literal("module"),
          Literal("param"),
          Literal("property"),
          Literal("return"),
          Literal("type"),
          Literal("event")
        ]),
        lineNumber: 98),
    GrammarRule(
        name: "attribute_arguments",
        body: Sequence(elements: [
          RuleReference("attribute_argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("attribute_argument", isToken: false)
          ]))
        ]),
        lineNumber: 108),
    GrammarRule(
        name: "attribute_argument",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true)
          ])),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 110),
    GrammarRule(
        name: "parameter_list",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false))
        ]),
        lineNumber: 112),
    GrammarRule(
        name: "type_annotation",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 114),
    GrammarRule(
        name: "type_expression",
        body: RuleReference("function_type", isToken: false),
        lineNumber: 116),
    GrammarRule(
        name: "function_type",
        body: Sequence(elements: [
          RuleReference("type_product", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("function_type", isToken: false)
          ]))
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "type_product",
        body: Sequence(elements: [
          RuleReference("type_application", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("type_application", isToken: false)
          ]))
        ]),
        lineNumber: 120),
    GrammarRule(
        name: "type_application",
        body: Sequence(elements: [
          RuleReference("type_atom", isToken: false),
          Repetition(element: RuleReference("type_atom", isToken: false))
        ]),
        lineNumber: 122),
    GrammarRule(
        name: "generic_type_arguments",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 124),
    GrammarRule(
        name: "type_atom",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("qualified_name", isToken: false),
            Optional(
                element:
                    RuleReference("generic_type_arguments", isToken: false))
          ]),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("tuple_type", isToken: false),
          RuleReference("unit_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("parenthesized_type", isToken: false)
        ]),
        lineNumber: 126),
    GrammarRule(
        name: "tuple_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 133),
    GrammarRule(
        name: "unit_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 135),
    GrammarRule(
        name: "parenthesized_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 137),
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
        lineNumber: 139),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("sequence_expression", isToken: false)
        ]),
        lineNumber: 141),
    GrammarRule(
        name: "sequence_expression",
        body: Sequence(elements: [
          RuleReference("infix_expression", isToken: false),
          Repetition(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("NEWLINE", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ]),
            Sequence(elements: [
              RuleReference("SEMICOLON", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 150),
    GrammarRule(
        name: "infix_expression",
        body: Sequence(elements: [
          RuleReference("application_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("infix_operator", isToken: false),
            RuleReference("application_expression", isToken: false)
          ]))
        ]),
        lineNumber: 152),
    GrammarRule(
        name: "infix_operator",
        body: Alternation(choices: [
          RuleReference("PIPE_RIGHT", isToken: true),
          RuleReference("PIPE_LEFT", isToken: true),
          RuleReference("COMPOSE_RIGHT", isToken: true),
          RuleReference("COMPOSE_LEFT", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("COLON_EQUALS", isToken: true),
          RuleReference("DOUBLE_COLON", isToken: true),
          RuleReference("LARROW", isToken: true),
          RuleReference("ARROW", isToken: true),
          RuleReference("DOT_DOT", isToken: true),
          RuleReference("AND_AND", isToken: true),
          RuleReference("OR_OR", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("PERCENT", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          RuleReference("PIPE", isToken: true),
          RuleReference("CARET", isToken: true)
        ]),
        lineNumber: 154),
    GrammarRule(
        name: "application_expression",
        body: Sequence(elements: [
          RuleReference("prefix_expression", isToken: false),
          Repetition(
              element: RuleReference("prefix_expression", isToken: false))
        ]),
        lineNumber: 181),
    GrammarRule(
        name: "prefix_expression",
        body: Sequence(elements: [
          Optional(element: RuleReference("unary_operator", isToken: false)),
          RuleReference("atomic_expression", isToken: false)
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "unary_operator",
        body: Alternation(choices: [
          RuleReference("MINUS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("BANG", isToken: true)
        ]),
        lineNumber: 185),
    GrammarRule(
        name: "atomic_expression",
        body: Alternation(choices: [
          RuleReference("computation_expression", isToken: false),
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("unit_expression", isToken: false),
          RuleReference("tuple_expression", isToken: false),
          RuleReference("list_expression", isToken: false),
          RuleReference("array_expression", isToken: false),
          RuleReference("record_expression", isToken: false),
          RuleReference("parenthesized_expression", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 189),
    GrammarRule(
        name: "computation_expression",
        body: Sequence(elements: [
          RuleReference("qualified_name", isToken: false),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("expression", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 211),
    GrammarRule(
        name: "unit_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 213),
    GrammarRule(
        name: "tuple_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 215),
    GrammarRule(
        name: "list_expression",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          NegativeLookahead(element: RuleReference("LESS_THAN", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 217),
    GrammarRule(
        name: "array_expression",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 219),
    GrammarRule(
        name: "record_expression",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_assignment", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_assignment", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 221),
    GrammarRule(
        name: "field_assignment",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 223),
    GrammarRule(
        name: "parenthesized_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 225),
    GrammarRule(
        name: "element_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 227),
    GrammarRule(
        name: "if_expression",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          Literal("then"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 230),
    GrammarRule(
        name: "match_expression",
        body: Sequence(elements: [
          Literal("match"),
          RuleReference("expression", isToken: false),
          Literal("with"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 232),
    GrammarRule(
        name: "match_case",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("when"),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 234),
    GrammarRule(
        name: "fun_expression",
        body: Sequence(elements: [
          Literal("fun"),
          RuleReference("parameter_list", isToken: false),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 236),
    GrammarRule(
        name: "function_expression",
        body: Sequence(elements: [
          Literal("function"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 238),
    GrammarRule(
        name: "let_expression",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ])),
          Literal("in"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 240),
    GrammarRule(
        name: "for_expression",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("pattern", isToken: false),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false),
          Group(
              element:
                  Alternation(choices: [Literal("to"), Literal("downto")])),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 242),
    GrammarRule(
        name: "while_expression",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 244),
    GrammarRule(
        name: "pattern",
        body: Sequence(elements: [
          RuleReference("pattern_atom", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 246),
    GrammarRule(
        name: "pattern_atom",
        body: Alternation(choices: [
          RuleReference("wildcard_pattern", isToken: false),
          RuleReference("literal_pattern", isToken: false),
          RuleReference("tuple_pattern", isToken: false),
          RuleReference("list_pattern", isToken: false),
          RuleReference("array_pattern", isToken: false),
          RuleReference("record_pattern", isToken: false),
          RuleReference("unit_pattern", isToken: false),
          RuleReference("parenthesized_pattern", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 248),
    GrammarRule(
        name: "wildcard_pattern",
        body: RuleReference("UNDERSCORE", isToken: true),
        lineNumber: 260),
    GrammarRule(
        name: "literal_pattern",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 262),
    GrammarRule(
        name: "tuple_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("pattern", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 269),
    GrammarRule(
        name: "list_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 271),
    GrammarRule(
        name: "array_pattern",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 273),
    GrammarRule(
        name: "record_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 275),
    GrammarRule(
        name: "field_pattern",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("pattern", isToken: false)
        ]),
        lineNumber: 277),
    GrammarRule(
        name: "unit_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 279),
    GrammarRule(
        name: "parenthesized_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 281),
  ],
);

final parserGrammarV6 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "compilation_unit",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("decorated_declaration", isToken: false)
        ])),
        lineNumber: 9),
    GrammarRule(
        name: "decorated_declaration",
        body: Sequence(elements: [
          Repetition(
              element: Sequence(elements: [
            RuleReference("attribute_section", isToken: false),
            RuleReference("NEWLINE", isToken: true)
          ])),
          RuleReference("declaration_body", isToken: false)
        ]),
        lineNumber: 11),
    GrammarRule(
        name: "declaration_body",
        body: Alternation(choices: [
          RuleReference("module_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("open_declaration", isToken: false),
          RuleReference("let_binding", isToken: false),
          RuleReference("use_binding", isToken: false),
          RuleReference("type_declaration", isToken: false),
          RuleReference("member_declaration", isToken: false),
          RuleReference("do_binding", isToken: false),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 13),
    GrammarRule(
        name: "module_declaration",
        body: Sequence(elements: [
          Literal("module"),
          Repetition(element: RuleReference("module_modifier", isToken: false)),
          Optional(element: Literal("rec")),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ]))
        ]),
        lineNumber: 23),
    GrammarRule(
        name: "module_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 25),
    GrammarRule(
        name: "namespace_declaration",
        body: Sequence(elements: [
          Literal("namespace"),
          Repetition(
              element: RuleReference("namespace_modifier", isToken: false)),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 28),
    GrammarRule(
        name: "namespace_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 30),
    GrammarRule(
        name: "open_declaration",
        body: Sequence(elements: [
          Literal("open"),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 33),
    GrammarRule(
        name: "let_binding",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 35),
    GrammarRule(
        name: "use_binding",
        body: Sequence(elements: [
          Literal("use"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 37),
    GrammarRule(
        name: "binding_modifier",
        body: Alternation(choices: [Literal("inline"), Literal("mutable")]),
        lineNumber: 39),
    GrammarRule(
        name: "binding_clause",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          RuleReference("EQUALS", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 42),
    GrammarRule(
        name: "do_binding",
        body: Sequence(elements: [
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 44),
    GrammarRule(
        name: "member_declaration",
        body: Sequence(elements: [
          Optional(element: RuleReference("member_modifier", isToken: false)),
          Literal("member"),
          RuleReference("qualified_name", isToken: false),
          Optional(element: RuleReference("parameter_list", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 46),
    GrammarRule(
        name: "member_modifier",
        body: Alternation(choices: [
          Literal("static"),
          Literal("override"),
          Literal("default"),
          Literal("abstract"),
          Literal("new")
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "type_declaration",
        body: Sequence(elements: [
          Literal("type"),
          Repetition(element: RuleReference("type_modifier", isToken: false)),
          RuleReference("NAME", isToken: true),
          Optional(
              element: RuleReference("generic_parameters", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ])),
          RuleReference("type_definition", isToken: false)
        ]),
        lineNumber: 54),
    GrammarRule(
        name: "type_modifier",
        body: Alternation(choices: [
          Literal("private"),
          Literal("public"),
          Literal("internal")
        ]),
        lineNumber: 56),
    GrammarRule(
        name: "generic_parameters",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_parameter", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 60),
    GrammarRule(
        name: "type_parameter",
        body: Alternation(choices: [
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 62),
    GrammarRule(
        name: "type_definition",
        body: Alternation(choices: [
          RuleReference("class_type", isToken: false),
          RuleReference("interface_type", isToken: false),
          RuleReference("struct_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("union_type", isToken: false),
          RuleReference("alias_type", isToken: false)
        ]),
        lineNumber: 65),
    GrammarRule(
        name: "class_type",
        body: Sequence(elements: [
          Literal("class"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 72),
    GrammarRule(
        name: "interface_type",
        body: Sequence(elements: [
          Literal("interface"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 74),
    GrammarRule(
        name: "struct_type",
        body: Sequence(elements: [
          Literal("struct"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "record_type",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_declaration", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_declaration", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 78),
    GrammarRule(
        name: "field_declaration",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 80),
    GrammarRule(
        name: "union_type",
        body: Sequence(elements: [
          RuleReference("union_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("union_case", isToken: false)
          ]))
        ]),
        lineNumber: 82),
    GrammarRule(
        name: "union_case",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("of"),
            RuleReference("type_expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("STAR", isToken: true),
              RuleReference("type_expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 84),
    GrammarRule(
        name: "alias_type",
        body: RuleReference("type_expression", isToken: false),
        lineNumber: 86),
    GrammarRule(
        name: "field_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 88),
    GrammarRule(
        name: "case_separator",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            Optional(element: RuleReference("PIPE", isToken: true))
          ]),
          RuleReference("PIPE", isToken: true)
        ]),
        lineNumber: 91),
    GrammarRule(
        name: "attribute_section",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("attribute", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("attribute", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 94),
    GrammarRule(
        name: "attribute",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("attribute_target", isToken: false),
            RuleReference("COLON", isToken: true)
          ])),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("attribute_arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ]))
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "attribute_target",
        body: Alternation(choices: [
          Literal("assembly"),
          Literal("field"),
          Literal("method"),
          Literal("module"),
          Literal("param"),
          Literal("property"),
          Literal("return"),
          Literal("type"),
          Literal("event")
        ]),
        lineNumber: 98),
    GrammarRule(
        name: "attribute_arguments",
        body: Sequence(elements: [
          RuleReference("attribute_argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("attribute_argument", isToken: false)
          ]))
        ]),
        lineNumber: 108),
    GrammarRule(
        name: "attribute_argument",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true)
          ])),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 110),
    GrammarRule(
        name: "parameter_list",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false))
        ]),
        lineNumber: 112),
    GrammarRule(
        name: "type_annotation",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 114),
    GrammarRule(
        name: "type_expression",
        body: RuleReference("function_type", isToken: false),
        lineNumber: 116),
    GrammarRule(
        name: "function_type",
        body: Sequence(elements: [
          RuleReference("type_product", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("function_type", isToken: false)
          ]))
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "type_product",
        body: Sequence(elements: [
          RuleReference("type_application", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("type_application", isToken: false)
          ]))
        ]),
        lineNumber: 120),
    GrammarRule(
        name: "type_application",
        body: Sequence(elements: [
          RuleReference("type_atom", isToken: false),
          Repetition(element: RuleReference("type_atom", isToken: false))
        ]),
        lineNumber: 122),
    GrammarRule(
        name: "generic_type_arguments",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 124),
    GrammarRule(
        name: "type_atom",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("qualified_name", isToken: false),
            Optional(
                element:
                    RuleReference("generic_type_arguments", isToken: false))
          ]),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("tuple_type", isToken: false),
          RuleReference("unit_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("parenthesized_type", isToken: false)
        ]),
        lineNumber: 126),
    GrammarRule(
        name: "tuple_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 133),
    GrammarRule(
        name: "unit_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 135),
    GrammarRule(
        name: "parenthesized_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 137),
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
        lineNumber: 139),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("sequence_expression", isToken: false)
        ]),
        lineNumber: 141),
    GrammarRule(
        name: "sequence_expression",
        body: Sequence(elements: [
          RuleReference("infix_expression", isToken: false),
          Repetition(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("NEWLINE", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ]),
            Sequence(elements: [
              RuleReference("SEMICOLON", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 150),
    GrammarRule(
        name: "infix_expression",
        body: Sequence(elements: [
          RuleReference("application_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("infix_operator", isToken: false),
            RuleReference("application_expression", isToken: false)
          ]))
        ]),
        lineNumber: 152),
    GrammarRule(
        name: "infix_operator",
        body: Alternation(choices: [
          RuleReference("PIPE_RIGHT", isToken: true),
          RuleReference("PIPE_LEFT", isToken: true),
          RuleReference("COMPOSE_RIGHT", isToken: true),
          RuleReference("COMPOSE_LEFT", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("COLON_EQUALS", isToken: true),
          RuleReference("DOUBLE_COLON", isToken: true),
          RuleReference("LARROW", isToken: true),
          RuleReference("ARROW", isToken: true),
          RuleReference("DOT_DOT", isToken: true),
          RuleReference("AND_AND", isToken: true),
          RuleReference("OR_OR", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("PERCENT", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          RuleReference("PIPE", isToken: true),
          RuleReference("CARET", isToken: true)
        ]),
        lineNumber: 154),
    GrammarRule(
        name: "application_expression",
        body: Sequence(elements: [
          RuleReference("prefix_expression", isToken: false),
          Repetition(
              element: RuleReference("prefix_expression", isToken: false))
        ]),
        lineNumber: 181),
    GrammarRule(
        name: "prefix_expression",
        body: Sequence(elements: [
          Optional(element: RuleReference("unary_operator", isToken: false)),
          RuleReference("atomic_expression", isToken: false)
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "unary_operator",
        body: Alternation(choices: [
          RuleReference("MINUS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("BANG", isToken: true)
        ]),
        lineNumber: 185),
    GrammarRule(
        name: "atomic_expression",
        body: Alternation(choices: [
          RuleReference("computation_expression", isToken: false),
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("unit_expression", isToken: false),
          RuleReference("tuple_expression", isToken: false),
          RuleReference("list_expression", isToken: false),
          RuleReference("array_expression", isToken: false),
          RuleReference("record_expression", isToken: false),
          RuleReference("parenthesized_expression", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 189),
    GrammarRule(
        name: "computation_expression",
        body: Sequence(elements: [
          RuleReference("qualified_name", isToken: false),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("expression", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 211),
    GrammarRule(
        name: "unit_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 213),
    GrammarRule(
        name: "tuple_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 215),
    GrammarRule(
        name: "list_expression",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          NegativeLookahead(element: RuleReference("LESS_THAN", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 217),
    GrammarRule(
        name: "array_expression",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 219),
    GrammarRule(
        name: "record_expression",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_assignment", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_assignment", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 221),
    GrammarRule(
        name: "field_assignment",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 223),
    GrammarRule(
        name: "parenthesized_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 225),
    GrammarRule(
        name: "element_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 227),
    GrammarRule(
        name: "if_expression",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          Literal("then"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 230),
    GrammarRule(
        name: "match_expression",
        body: Sequence(elements: [
          Literal("match"),
          RuleReference("expression", isToken: false),
          Literal("with"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 232),
    GrammarRule(
        name: "match_case",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("when"),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 234),
    GrammarRule(
        name: "fun_expression",
        body: Sequence(elements: [
          Literal("fun"),
          RuleReference("parameter_list", isToken: false),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 236),
    GrammarRule(
        name: "function_expression",
        body: Sequence(elements: [
          Literal("function"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 238),
    GrammarRule(
        name: "let_expression",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ])),
          Literal("in"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 240),
    GrammarRule(
        name: "for_expression",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("pattern", isToken: false),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false),
          Group(
              element:
                  Alternation(choices: [Literal("to"), Literal("downto")])),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 242),
    GrammarRule(
        name: "while_expression",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 244),
    GrammarRule(
        name: "pattern",
        body: Sequence(elements: [
          RuleReference("pattern_atom", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 246),
    GrammarRule(
        name: "pattern_atom",
        body: Alternation(choices: [
          RuleReference("wildcard_pattern", isToken: false),
          RuleReference("literal_pattern", isToken: false),
          RuleReference("tuple_pattern", isToken: false),
          RuleReference("list_pattern", isToken: false),
          RuleReference("array_pattern", isToken: false),
          RuleReference("record_pattern", isToken: false),
          RuleReference("unit_pattern", isToken: false),
          RuleReference("parenthesized_pattern", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 248),
    GrammarRule(
        name: "wildcard_pattern",
        body: RuleReference("UNDERSCORE", isToken: true),
        lineNumber: 260),
    GrammarRule(
        name: "literal_pattern",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 262),
    GrammarRule(
        name: "tuple_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("pattern", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 269),
    GrammarRule(
        name: "list_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 271),
    GrammarRule(
        name: "array_pattern",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 273),
    GrammarRule(
        name: "record_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 275),
    GrammarRule(
        name: "field_pattern",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("pattern", isToken: false)
        ]),
        lineNumber: 277),
    GrammarRule(
        name: "unit_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 279),
    GrammarRule(
        name: "parenthesized_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 281),
  ],
);

final parserGrammarV7 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "compilation_unit",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("decorated_declaration", isToken: false)
        ])),
        lineNumber: 9),
    GrammarRule(
        name: "decorated_declaration",
        body: Sequence(elements: [
          Repetition(
              element: Sequence(elements: [
            RuleReference("attribute_section", isToken: false),
            RuleReference("NEWLINE", isToken: true)
          ])),
          RuleReference("declaration_body", isToken: false)
        ]),
        lineNumber: 11),
    GrammarRule(
        name: "declaration_body",
        body: Alternation(choices: [
          RuleReference("module_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("open_declaration", isToken: false),
          RuleReference("let_binding", isToken: false),
          RuleReference("use_binding", isToken: false),
          RuleReference("type_declaration", isToken: false),
          RuleReference("member_declaration", isToken: false),
          RuleReference("do_binding", isToken: false),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 13),
    GrammarRule(
        name: "module_declaration",
        body: Sequence(elements: [
          Literal("module"),
          Repetition(element: RuleReference("module_modifier", isToken: false)),
          Optional(element: Literal("rec")),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ]))
        ]),
        lineNumber: 23),
    GrammarRule(
        name: "module_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 25),
    GrammarRule(
        name: "namespace_declaration",
        body: Sequence(elements: [
          Literal("namespace"),
          Repetition(
              element: RuleReference("namespace_modifier", isToken: false)),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 28),
    GrammarRule(
        name: "namespace_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 30),
    GrammarRule(
        name: "open_declaration",
        body: Sequence(elements: [
          Literal("open"),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 33),
    GrammarRule(
        name: "let_binding",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 35),
    GrammarRule(
        name: "use_binding",
        body: Sequence(elements: [
          Literal("use"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 37),
    GrammarRule(
        name: "binding_modifier",
        body: Alternation(choices: [Literal("inline"), Literal("mutable")]),
        lineNumber: 39),
    GrammarRule(
        name: "binding_clause",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          RuleReference("EQUALS", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 42),
    GrammarRule(
        name: "do_binding",
        body: Sequence(elements: [
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 44),
    GrammarRule(
        name: "member_declaration",
        body: Sequence(elements: [
          Optional(element: RuleReference("member_modifier", isToken: false)),
          Literal("member"),
          RuleReference("qualified_name", isToken: false),
          Optional(element: RuleReference("parameter_list", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 46),
    GrammarRule(
        name: "member_modifier",
        body: Alternation(choices: [
          Literal("static"),
          Literal("override"),
          Literal("default"),
          Literal("abstract"),
          Literal("new")
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "type_declaration",
        body: Sequence(elements: [
          Literal("type"),
          Repetition(element: RuleReference("type_modifier", isToken: false)),
          RuleReference("NAME", isToken: true),
          Optional(
              element: RuleReference("generic_parameters", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ])),
          RuleReference("type_definition", isToken: false)
        ]),
        lineNumber: 54),
    GrammarRule(
        name: "type_modifier",
        body: Alternation(choices: [
          Literal("private"),
          Literal("public"),
          Literal("internal")
        ]),
        lineNumber: 56),
    GrammarRule(
        name: "generic_parameters",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_parameter", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 60),
    GrammarRule(
        name: "type_parameter",
        body: Alternation(choices: [
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 62),
    GrammarRule(
        name: "type_definition",
        body: Alternation(choices: [
          RuleReference("class_type", isToken: false),
          RuleReference("interface_type", isToken: false),
          RuleReference("struct_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("union_type", isToken: false),
          RuleReference("alias_type", isToken: false)
        ]),
        lineNumber: 65),
    GrammarRule(
        name: "class_type",
        body: Sequence(elements: [
          Literal("class"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 72),
    GrammarRule(
        name: "interface_type",
        body: Sequence(elements: [
          Literal("interface"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 74),
    GrammarRule(
        name: "struct_type",
        body: Sequence(elements: [
          Literal("struct"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "record_type",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_declaration", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_declaration", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 78),
    GrammarRule(
        name: "field_declaration",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 80),
    GrammarRule(
        name: "union_type",
        body: Sequence(elements: [
          RuleReference("union_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("union_case", isToken: false)
          ]))
        ]),
        lineNumber: 82),
    GrammarRule(
        name: "union_case",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("of"),
            RuleReference("type_expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("STAR", isToken: true),
              RuleReference("type_expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 84),
    GrammarRule(
        name: "alias_type",
        body: RuleReference("type_expression", isToken: false),
        lineNumber: 86),
    GrammarRule(
        name: "field_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 88),
    GrammarRule(
        name: "case_separator",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            Optional(element: RuleReference("PIPE", isToken: true))
          ]),
          RuleReference("PIPE", isToken: true)
        ]),
        lineNumber: 91),
    GrammarRule(
        name: "attribute_section",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("attribute", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("attribute", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 94),
    GrammarRule(
        name: "attribute",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("attribute_target", isToken: false),
            RuleReference("COLON", isToken: true)
          ])),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("attribute_arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ]))
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "attribute_target",
        body: Alternation(choices: [
          Literal("assembly"),
          Literal("field"),
          Literal("method"),
          Literal("module"),
          Literal("param"),
          Literal("property"),
          Literal("return"),
          Literal("type"),
          Literal("event")
        ]),
        lineNumber: 98),
    GrammarRule(
        name: "attribute_arguments",
        body: Sequence(elements: [
          RuleReference("attribute_argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("attribute_argument", isToken: false)
          ]))
        ]),
        lineNumber: 108),
    GrammarRule(
        name: "attribute_argument",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true)
          ])),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 110),
    GrammarRule(
        name: "parameter_list",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false))
        ]),
        lineNumber: 112),
    GrammarRule(
        name: "type_annotation",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 114),
    GrammarRule(
        name: "type_expression",
        body: RuleReference("function_type", isToken: false),
        lineNumber: 116),
    GrammarRule(
        name: "function_type",
        body: Sequence(elements: [
          RuleReference("type_product", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("function_type", isToken: false)
          ]))
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "type_product",
        body: Sequence(elements: [
          RuleReference("type_application", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("type_application", isToken: false)
          ]))
        ]),
        lineNumber: 120),
    GrammarRule(
        name: "type_application",
        body: Sequence(elements: [
          RuleReference("type_atom", isToken: false),
          Repetition(element: RuleReference("type_atom", isToken: false))
        ]),
        lineNumber: 122),
    GrammarRule(
        name: "generic_type_arguments",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 124),
    GrammarRule(
        name: "type_atom",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("qualified_name", isToken: false),
            Optional(
                element:
                    RuleReference("generic_type_arguments", isToken: false))
          ]),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("tuple_type", isToken: false),
          RuleReference("unit_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("parenthesized_type", isToken: false)
        ]),
        lineNumber: 126),
    GrammarRule(
        name: "tuple_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 133),
    GrammarRule(
        name: "unit_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 135),
    GrammarRule(
        name: "parenthesized_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 137),
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
        lineNumber: 139),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("sequence_expression", isToken: false)
        ]),
        lineNumber: 141),
    GrammarRule(
        name: "sequence_expression",
        body: Sequence(elements: [
          RuleReference("infix_expression", isToken: false),
          Repetition(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("NEWLINE", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ]),
            Sequence(elements: [
              RuleReference("SEMICOLON", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 150),
    GrammarRule(
        name: "infix_expression",
        body: Sequence(elements: [
          RuleReference("application_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("infix_operator", isToken: false),
            RuleReference("application_expression", isToken: false)
          ]))
        ]),
        lineNumber: 152),
    GrammarRule(
        name: "infix_operator",
        body: Alternation(choices: [
          RuleReference("PIPE_RIGHT", isToken: true),
          RuleReference("PIPE_LEFT", isToken: true),
          RuleReference("COMPOSE_RIGHT", isToken: true),
          RuleReference("COMPOSE_LEFT", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("COLON_EQUALS", isToken: true),
          RuleReference("DOUBLE_COLON", isToken: true),
          RuleReference("LARROW", isToken: true),
          RuleReference("ARROW", isToken: true),
          RuleReference("DOT_DOT", isToken: true),
          RuleReference("AND_AND", isToken: true),
          RuleReference("OR_OR", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("PERCENT", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          RuleReference("PIPE", isToken: true),
          RuleReference("CARET", isToken: true)
        ]),
        lineNumber: 154),
    GrammarRule(
        name: "application_expression",
        body: Sequence(elements: [
          RuleReference("prefix_expression", isToken: false),
          Repetition(
              element: RuleReference("prefix_expression", isToken: false))
        ]),
        lineNumber: 181),
    GrammarRule(
        name: "prefix_expression",
        body: Sequence(elements: [
          Optional(element: RuleReference("unary_operator", isToken: false)),
          RuleReference("atomic_expression", isToken: false)
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "unary_operator",
        body: Alternation(choices: [
          RuleReference("MINUS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("BANG", isToken: true)
        ]),
        lineNumber: 185),
    GrammarRule(
        name: "atomic_expression",
        body: Alternation(choices: [
          RuleReference("computation_expression", isToken: false),
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("unit_expression", isToken: false),
          RuleReference("tuple_expression", isToken: false),
          RuleReference("list_expression", isToken: false),
          RuleReference("array_expression", isToken: false),
          RuleReference("record_expression", isToken: false),
          RuleReference("parenthesized_expression", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 189),
    GrammarRule(
        name: "computation_expression",
        body: Sequence(elements: [
          RuleReference("qualified_name", isToken: false),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("expression", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 211),
    GrammarRule(
        name: "unit_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 213),
    GrammarRule(
        name: "tuple_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 215),
    GrammarRule(
        name: "list_expression",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          NegativeLookahead(element: RuleReference("LESS_THAN", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 217),
    GrammarRule(
        name: "array_expression",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 219),
    GrammarRule(
        name: "record_expression",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_assignment", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_assignment", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 221),
    GrammarRule(
        name: "field_assignment",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 223),
    GrammarRule(
        name: "parenthesized_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 225),
    GrammarRule(
        name: "element_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 227),
    GrammarRule(
        name: "if_expression",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          Literal("then"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 230),
    GrammarRule(
        name: "match_expression",
        body: Sequence(elements: [
          Literal("match"),
          RuleReference("expression", isToken: false),
          Literal("with"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 232),
    GrammarRule(
        name: "match_case",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("when"),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 234),
    GrammarRule(
        name: "fun_expression",
        body: Sequence(elements: [
          Literal("fun"),
          RuleReference("parameter_list", isToken: false),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 236),
    GrammarRule(
        name: "function_expression",
        body: Sequence(elements: [
          Literal("function"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 238),
    GrammarRule(
        name: "let_expression",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ])),
          Literal("in"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 240),
    GrammarRule(
        name: "for_expression",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("pattern", isToken: false),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false),
          Group(
              element:
                  Alternation(choices: [Literal("to"), Literal("downto")])),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 242),
    GrammarRule(
        name: "while_expression",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 244),
    GrammarRule(
        name: "pattern",
        body: Sequence(elements: [
          RuleReference("pattern_atom", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 246),
    GrammarRule(
        name: "pattern_atom",
        body: Alternation(choices: [
          RuleReference("wildcard_pattern", isToken: false),
          RuleReference("literal_pattern", isToken: false),
          RuleReference("tuple_pattern", isToken: false),
          RuleReference("list_pattern", isToken: false),
          RuleReference("array_pattern", isToken: false),
          RuleReference("record_pattern", isToken: false),
          RuleReference("unit_pattern", isToken: false),
          RuleReference("parenthesized_pattern", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 248),
    GrammarRule(
        name: "wildcard_pattern",
        body: RuleReference("UNDERSCORE", isToken: true),
        lineNumber: 260),
    GrammarRule(
        name: "literal_pattern",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 262),
    GrammarRule(
        name: "tuple_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("pattern", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 269),
    GrammarRule(
        name: "list_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 271),
    GrammarRule(
        name: "array_pattern",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 273),
    GrammarRule(
        name: "record_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 275),
    GrammarRule(
        name: "field_pattern",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("pattern", isToken: false)
        ]),
        lineNumber: 277),
    GrammarRule(
        name: "unit_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 279),
    GrammarRule(
        name: "parenthesized_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 281),
  ],
);

final parserGrammarV8 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "compilation_unit",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("decorated_declaration", isToken: false)
        ])),
        lineNumber: 9),
    GrammarRule(
        name: "decorated_declaration",
        body: Sequence(elements: [
          Repetition(
              element: Sequence(elements: [
            RuleReference("attribute_section", isToken: false),
            RuleReference("NEWLINE", isToken: true)
          ])),
          RuleReference("declaration_body", isToken: false)
        ]),
        lineNumber: 11),
    GrammarRule(
        name: "declaration_body",
        body: Alternation(choices: [
          RuleReference("module_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("open_declaration", isToken: false),
          RuleReference("let_binding", isToken: false),
          RuleReference("use_binding", isToken: false),
          RuleReference("type_declaration", isToken: false),
          RuleReference("member_declaration", isToken: false),
          RuleReference("do_binding", isToken: false),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 13),
    GrammarRule(
        name: "module_declaration",
        body: Sequence(elements: [
          Literal("module"),
          Repetition(element: RuleReference("module_modifier", isToken: false)),
          Optional(element: Literal("rec")),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ]))
        ]),
        lineNumber: 23),
    GrammarRule(
        name: "module_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 25),
    GrammarRule(
        name: "namespace_declaration",
        body: Sequence(elements: [
          Literal("namespace"),
          Repetition(
              element: RuleReference("namespace_modifier", isToken: false)),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 28),
    GrammarRule(
        name: "namespace_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 30),
    GrammarRule(
        name: "open_declaration",
        body: Sequence(elements: [
          Literal("open"),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 33),
    GrammarRule(
        name: "let_binding",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 35),
    GrammarRule(
        name: "use_binding",
        body: Sequence(elements: [
          Literal("use"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 37),
    GrammarRule(
        name: "binding_modifier",
        body: Alternation(choices: [Literal("inline"), Literal("mutable")]),
        lineNumber: 39),
    GrammarRule(
        name: "binding_clause",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          RuleReference("EQUALS", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 42),
    GrammarRule(
        name: "do_binding",
        body: Sequence(elements: [
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 44),
    GrammarRule(
        name: "member_declaration",
        body: Sequence(elements: [
          Optional(element: RuleReference("member_modifier", isToken: false)),
          Literal("member"),
          RuleReference("qualified_name", isToken: false),
          Optional(element: RuleReference("parameter_list", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 46),
    GrammarRule(
        name: "member_modifier",
        body: Alternation(choices: [
          Literal("static"),
          Literal("override"),
          Literal("default"),
          Literal("abstract"),
          Literal("new")
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "type_declaration",
        body: Sequence(elements: [
          Literal("type"),
          Repetition(element: RuleReference("type_modifier", isToken: false)),
          RuleReference("NAME", isToken: true),
          Optional(
              element: RuleReference("generic_parameters", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ])),
          RuleReference("type_definition", isToken: false)
        ]),
        lineNumber: 54),
    GrammarRule(
        name: "type_modifier",
        body: Alternation(choices: [
          Literal("private"),
          Literal("public"),
          Literal("internal")
        ]),
        lineNumber: 56),
    GrammarRule(
        name: "generic_parameters",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_parameter", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 60),
    GrammarRule(
        name: "type_parameter",
        body: Alternation(choices: [
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 62),
    GrammarRule(
        name: "type_definition",
        body: Alternation(choices: [
          RuleReference("class_type", isToken: false),
          RuleReference("interface_type", isToken: false),
          RuleReference("struct_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("union_type", isToken: false),
          RuleReference("alias_type", isToken: false)
        ]),
        lineNumber: 65),
    GrammarRule(
        name: "class_type",
        body: Sequence(elements: [
          Literal("class"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 72),
    GrammarRule(
        name: "interface_type",
        body: Sequence(elements: [
          Literal("interface"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 74),
    GrammarRule(
        name: "struct_type",
        body: Sequence(elements: [
          Literal("struct"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "record_type",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_declaration", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_declaration", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 78),
    GrammarRule(
        name: "field_declaration",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 80),
    GrammarRule(
        name: "union_type",
        body: Sequence(elements: [
          RuleReference("union_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("union_case", isToken: false)
          ]))
        ]),
        lineNumber: 82),
    GrammarRule(
        name: "union_case",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("of"),
            RuleReference("type_expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("STAR", isToken: true),
              RuleReference("type_expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 84),
    GrammarRule(
        name: "alias_type",
        body: RuleReference("type_expression", isToken: false),
        lineNumber: 86),
    GrammarRule(
        name: "field_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 88),
    GrammarRule(
        name: "case_separator",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            Optional(element: RuleReference("PIPE", isToken: true))
          ]),
          RuleReference("PIPE", isToken: true)
        ]),
        lineNumber: 91),
    GrammarRule(
        name: "attribute_section",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("attribute", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("attribute", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 94),
    GrammarRule(
        name: "attribute",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("attribute_target", isToken: false),
            RuleReference("COLON", isToken: true)
          ])),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("attribute_arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ]))
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "attribute_target",
        body: Alternation(choices: [
          Literal("assembly"),
          Literal("field"),
          Literal("method"),
          Literal("module"),
          Literal("param"),
          Literal("property"),
          Literal("return"),
          Literal("type"),
          Literal("event")
        ]),
        lineNumber: 98),
    GrammarRule(
        name: "attribute_arguments",
        body: Sequence(elements: [
          RuleReference("attribute_argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("attribute_argument", isToken: false)
          ]))
        ]),
        lineNumber: 108),
    GrammarRule(
        name: "attribute_argument",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true)
          ])),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 110),
    GrammarRule(
        name: "parameter_list",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false))
        ]),
        lineNumber: 112),
    GrammarRule(
        name: "type_annotation",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 114),
    GrammarRule(
        name: "type_expression",
        body: RuleReference("function_type", isToken: false),
        lineNumber: 116),
    GrammarRule(
        name: "function_type",
        body: Sequence(elements: [
          RuleReference("type_product", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("function_type", isToken: false)
          ]))
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "type_product",
        body: Sequence(elements: [
          RuleReference("type_application", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("type_application", isToken: false)
          ]))
        ]),
        lineNumber: 120),
    GrammarRule(
        name: "type_application",
        body: Sequence(elements: [
          RuleReference("type_atom", isToken: false),
          Repetition(element: RuleReference("type_atom", isToken: false))
        ]),
        lineNumber: 122),
    GrammarRule(
        name: "generic_type_arguments",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 124),
    GrammarRule(
        name: "type_atom",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("qualified_name", isToken: false),
            Optional(
                element:
                    RuleReference("generic_type_arguments", isToken: false))
          ]),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("tuple_type", isToken: false),
          RuleReference("unit_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("parenthesized_type", isToken: false)
        ]),
        lineNumber: 126),
    GrammarRule(
        name: "tuple_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 133),
    GrammarRule(
        name: "unit_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 135),
    GrammarRule(
        name: "parenthesized_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 137),
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
        lineNumber: 139),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("sequence_expression", isToken: false)
        ]),
        lineNumber: 141),
    GrammarRule(
        name: "sequence_expression",
        body: Sequence(elements: [
          RuleReference("infix_expression", isToken: false),
          Repetition(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("NEWLINE", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ]),
            Sequence(elements: [
              RuleReference("SEMICOLON", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 150),
    GrammarRule(
        name: "infix_expression",
        body: Sequence(elements: [
          RuleReference("application_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("infix_operator", isToken: false),
            RuleReference("application_expression", isToken: false)
          ]))
        ]),
        lineNumber: 152),
    GrammarRule(
        name: "infix_operator",
        body: Alternation(choices: [
          RuleReference("PIPE_RIGHT", isToken: true),
          RuleReference("PIPE_LEFT", isToken: true),
          RuleReference("COMPOSE_RIGHT", isToken: true),
          RuleReference("COMPOSE_LEFT", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("COLON_EQUALS", isToken: true),
          RuleReference("DOUBLE_COLON", isToken: true),
          RuleReference("LARROW", isToken: true),
          RuleReference("ARROW", isToken: true),
          RuleReference("DOT_DOT", isToken: true),
          RuleReference("AND_AND", isToken: true),
          RuleReference("OR_OR", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("PERCENT", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          RuleReference("PIPE", isToken: true),
          RuleReference("CARET", isToken: true)
        ]),
        lineNumber: 154),
    GrammarRule(
        name: "application_expression",
        body: Sequence(elements: [
          RuleReference("prefix_expression", isToken: false),
          Repetition(
              element: RuleReference("prefix_expression", isToken: false))
        ]),
        lineNumber: 181),
    GrammarRule(
        name: "prefix_expression",
        body: Sequence(elements: [
          Optional(element: RuleReference("unary_operator", isToken: false)),
          RuleReference("atomic_expression", isToken: false)
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "unary_operator",
        body: Alternation(choices: [
          RuleReference("MINUS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("BANG", isToken: true)
        ]),
        lineNumber: 185),
    GrammarRule(
        name: "atomic_expression",
        body: Alternation(choices: [
          RuleReference("computation_expression", isToken: false),
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("unit_expression", isToken: false),
          RuleReference("tuple_expression", isToken: false),
          RuleReference("list_expression", isToken: false),
          RuleReference("array_expression", isToken: false),
          RuleReference("record_expression", isToken: false),
          RuleReference("parenthesized_expression", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 189),
    GrammarRule(
        name: "computation_expression",
        body: Sequence(elements: [
          RuleReference("qualified_name", isToken: false),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("expression", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 211),
    GrammarRule(
        name: "unit_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 213),
    GrammarRule(
        name: "tuple_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 215),
    GrammarRule(
        name: "list_expression",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          NegativeLookahead(element: RuleReference("LESS_THAN", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 217),
    GrammarRule(
        name: "array_expression",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 219),
    GrammarRule(
        name: "record_expression",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_assignment", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_assignment", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 221),
    GrammarRule(
        name: "field_assignment",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 223),
    GrammarRule(
        name: "parenthesized_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 225),
    GrammarRule(
        name: "element_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 227),
    GrammarRule(
        name: "if_expression",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          Literal("then"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 230),
    GrammarRule(
        name: "match_expression",
        body: Sequence(elements: [
          Literal("match"),
          RuleReference("expression", isToken: false),
          Literal("with"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 232),
    GrammarRule(
        name: "match_case",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("when"),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 234),
    GrammarRule(
        name: "fun_expression",
        body: Sequence(elements: [
          Literal("fun"),
          RuleReference("parameter_list", isToken: false),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 236),
    GrammarRule(
        name: "function_expression",
        body: Sequence(elements: [
          Literal("function"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 238),
    GrammarRule(
        name: "let_expression",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ])),
          Literal("in"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 240),
    GrammarRule(
        name: "for_expression",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("pattern", isToken: false),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false),
          Group(
              element:
                  Alternation(choices: [Literal("to"), Literal("downto")])),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 242),
    GrammarRule(
        name: "while_expression",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 244),
    GrammarRule(
        name: "pattern",
        body: Sequence(elements: [
          RuleReference("pattern_atom", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 246),
    GrammarRule(
        name: "pattern_atom",
        body: Alternation(choices: [
          RuleReference("wildcard_pattern", isToken: false),
          RuleReference("literal_pattern", isToken: false),
          RuleReference("tuple_pattern", isToken: false),
          RuleReference("list_pattern", isToken: false),
          RuleReference("array_pattern", isToken: false),
          RuleReference("record_pattern", isToken: false),
          RuleReference("unit_pattern", isToken: false),
          RuleReference("parenthesized_pattern", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 248),
    GrammarRule(
        name: "wildcard_pattern",
        body: RuleReference("UNDERSCORE", isToken: true),
        lineNumber: 260),
    GrammarRule(
        name: "literal_pattern",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 262),
    GrammarRule(
        name: "tuple_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("pattern", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 269),
    GrammarRule(
        name: "list_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 271),
    GrammarRule(
        name: "array_pattern",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 273),
    GrammarRule(
        name: "record_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 275),
    GrammarRule(
        name: "field_pattern",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("pattern", isToken: false)
        ]),
        lineNumber: 277),
    GrammarRule(
        name: "unit_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 279),
    GrammarRule(
        name: "parenthesized_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 281),
  ],
);

final parserGrammarV9 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "compilation_unit",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("decorated_declaration", isToken: false)
        ])),
        lineNumber: 9),
    GrammarRule(
        name: "decorated_declaration",
        body: Sequence(elements: [
          Repetition(
              element: Sequence(elements: [
            RuleReference("attribute_section", isToken: false),
            RuleReference("NEWLINE", isToken: true)
          ])),
          RuleReference("declaration_body", isToken: false)
        ]),
        lineNumber: 11),
    GrammarRule(
        name: "declaration_body",
        body: Alternation(choices: [
          RuleReference("module_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("open_declaration", isToken: false),
          RuleReference("let_binding", isToken: false),
          RuleReference("use_binding", isToken: false),
          RuleReference("type_declaration", isToken: false),
          RuleReference("member_declaration", isToken: false),
          RuleReference("do_binding", isToken: false),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 13),
    GrammarRule(
        name: "module_declaration",
        body: Sequence(elements: [
          Literal("module"),
          Repetition(element: RuleReference("module_modifier", isToken: false)),
          Optional(element: Literal("rec")),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ]))
        ]),
        lineNumber: 23),
    GrammarRule(
        name: "module_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 25),
    GrammarRule(
        name: "namespace_declaration",
        body: Sequence(elements: [
          Literal("namespace"),
          Repetition(
              element: RuleReference("namespace_modifier", isToken: false)),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 28),
    GrammarRule(
        name: "namespace_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 30),
    GrammarRule(
        name: "open_declaration",
        body: Sequence(elements: [
          Literal("open"),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 33),
    GrammarRule(
        name: "let_binding",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 35),
    GrammarRule(
        name: "use_binding",
        body: Sequence(elements: [
          Literal("use"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 37),
    GrammarRule(
        name: "binding_modifier",
        body: Alternation(choices: [Literal("inline"), Literal("mutable")]),
        lineNumber: 39),
    GrammarRule(
        name: "binding_clause",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          RuleReference("EQUALS", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 42),
    GrammarRule(
        name: "do_binding",
        body: Sequence(elements: [
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 44),
    GrammarRule(
        name: "member_declaration",
        body: Sequence(elements: [
          Optional(element: RuleReference("member_modifier", isToken: false)),
          Literal("member"),
          RuleReference("qualified_name", isToken: false),
          Optional(element: RuleReference("parameter_list", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 46),
    GrammarRule(
        name: "member_modifier",
        body: Alternation(choices: [
          Literal("static"),
          Literal("override"),
          Literal("default"),
          Literal("abstract"),
          Literal("new")
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "type_declaration",
        body: Sequence(elements: [
          Literal("type"),
          Repetition(element: RuleReference("type_modifier", isToken: false)),
          RuleReference("NAME", isToken: true),
          Optional(
              element: RuleReference("generic_parameters", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ])),
          RuleReference("type_definition", isToken: false)
        ]),
        lineNumber: 54),
    GrammarRule(
        name: "type_modifier",
        body: Alternation(choices: [
          Literal("private"),
          Literal("public"),
          Literal("internal")
        ]),
        lineNumber: 56),
    GrammarRule(
        name: "generic_parameters",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_parameter", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 60),
    GrammarRule(
        name: "type_parameter",
        body: Alternation(choices: [
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 62),
    GrammarRule(
        name: "type_definition",
        body: Alternation(choices: [
          RuleReference("class_type", isToken: false),
          RuleReference("interface_type", isToken: false),
          RuleReference("struct_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("union_type", isToken: false),
          RuleReference("alias_type", isToken: false)
        ]),
        lineNumber: 65),
    GrammarRule(
        name: "class_type",
        body: Sequence(elements: [
          Literal("class"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 72),
    GrammarRule(
        name: "interface_type",
        body: Sequence(elements: [
          Literal("interface"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 74),
    GrammarRule(
        name: "struct_type",
        body: Sequence(elements: [
          Literal("struct"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "record_type",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_declaration", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_declaration", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 78),
    GrammarRule(
        name: "field_declaration",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 80),
    GrammarRule(
        name: "union_type",
        body: Sequence(elements: [
          RuleReference("union_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("union_case", isToken: false)
          ]))
        ]),
        lineNumber: 82),
    GrammarRule(
        name: "union_case",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("of"),
            RuleReference("type_expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("STAR", isToken: true),
              RuleReference("type_expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 84),
    GrammarRule(
        name: "alias_type",
        body: RuleReference("type_expression", isToken: false),
        lineNumber: 86),
    GrammarRule(
        name: "field_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 88),
    GrammarRule(
        name: "case_separator",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            Optional(element: RuleReference("PIPE", isToken: true))
          ]),
          RuleReference("PIPE", isToken: true)
        ]),
        lineNumber: 91),
    GrammarRule(
        name: "attribute_section",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("attribute", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("attribute", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 94),
    GrammarRule(
        name: "attribute",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("attribute_target", isToken: false),
            RuleReference("COLON", isToken: true)
          ])),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("attribute_arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ]))
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "attribute_target",
        body: Alternation(choices: [
          Literal("assembly"),
          Literal("field"),
          Literal("method"),
          Literal("module"),
          Literal("param"),
          Literal("property"),
          Literal("return"),
          Literal("type"),
          Literal("event")
        ]),
        lineNumber: 98),
    GrammarRule(
        name: "attribute_arguments",
        body: Sequence(elements: [
          RuleReference("attribute_argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("attribute_argument", isToken: false)
          ]))
        ]),
        lineNumber: 108),
    GrammarRule(
        name: "attribute_argument",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true)
          ])),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 110),
    GrammarRule(
        name: "parameter_list",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false))
        ]),
        lineNumber: 112),
    GrammarRule(
        name: "type_annotation",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 114),
    GrammarRule(
        name: "type_expression",
        body: RuleReference("function_type", isToken: false),
        lineNumber: 116),
    GrammarRule(
        name: "function_type",
        body: Sequence(elements: [
          RuleReference("type_product", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("function_type", isToken: false)
          ]))
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "type_product",
        body: Sequence(elements: [
          RuleReference("type_application", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("type_application", isToken: false)
          ]))
        ]),
        lineNumber: 120),
    GrammarRule(
        name: "type_application",
        body: Sequence(elements: [
          RuleReference("type_atom", isToken: false),
          Repetition(element: RuleReference("type_atom", isToken: false))
        ]),
        lineNumber: 122),
    GrammarRule(
        name: "generic_type_arguments",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 124),
    GrammarRule(
        name: "type_atom",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("qualified_name", isToken: false),
            Optional(
                element:
                    RuleReference("generic_type_arguments", isToken: false))
          ]),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("tuple_type", isToken: false),
          RuleReference("unit_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("parenthesized_type", isToken: false)
        ]),
        lineNumber: 126),
    GrammarRule(
        name: "tuple_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 133),
    GrammarRule(
        name: "unit_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 135),
    GrammarRule(
        name: "parenthesized_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 137),
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
        lineNumber: 139),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("sequence_expression", isToken: false)
        ]),
        lineNumber: 141),
    GrammarRule(
        name: "sequence_expression",
        body: Sequence(elements: [
          RuleReference("infix_expression", isToken: false),
          Repetition(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("NEWLINE", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ]),
            Sequence(elements: [
              RuleReference("SEMICOLON", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 150),
    GrammarRule(
        name: "infix_expression",
        body: Sequence(elements: [
          RuleReference("application_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("infix_operator", isToken: false),
            RuleReference("application_expression", isToken: false)
          ]))
        ]),
        lineNumber: 152),
    GrammarRule(
        name: "infix_operator",
        body: Alternation(choices: [
          RuleReference("PIPE_RIGHT", isToken: true),
          RuleReference("PIPE_LEFT", isToken: true),
          RuleReference("COMPOSE_RIGHT", isToken: true),
          RuleReference("COMPOSE_LEFT", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("COLON_EQUALS", isToken: true),
          RuleReference("DOUBLE_COLON", isToken: true),
          RuleReference("LARROW", isToken: true),
          RuleReference("ARROW", isToken: true),
          RuleReference("DOT_DOT", isToken: true),
          RuleReference("AND_AND", isToken: true),
          RuleReference("OR_OR", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("PERCENT", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          RuleReference("PIPE", isToken: true),
          RuleReference("CARET", isToken: true)
        ]),
        lineNumber: 154),
    GrammarRule(
        name: "application_expression",
        body: Sequence(elements: [
          RuleReference("prefix_expression", isToken: false),
          Repetition(
              element: RuleReference("prefix_expression", isToken: false))
        ]),
        lineNumber: 181),
    GrammarRule(
        name: "prefix_expression",
        body: Sequence(elements: [
          Optional(element: RuleReference("unary_operator", isToken: false)),
          RuleReference("atomic_expression", isToken: false)
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "unary_operator",
        body: Alternation(choices: [
          RuleReference("MINUS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("BANG", isToken: true)
        ]),
        lineNumber: 185),
    GrammarRule(
        name: "atomic_expression",
        body: Alternation(choices: [
          RuleReference("computation_expression", isToken: false),
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("unit_expression", isToken: false),
          RuleReference("tuple_expression", isToken: false),
          RuleReference("list_expression", isToken: false),
          RuleReference("array_expression", isToken: false),
          RuleReference("record_expression", isToken: false),
          RuleReference("parenthesized_expression", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 189),
    GrammarRule(
        name: "computation_expression",
        body: Sequence(elements: [
          RuleReference("qualified_name", isToken: false),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("expression", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 211),
    GrammarRule(
        name: "unit_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 213),
    GrammarRule(
        name: "tuple_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 215),
    GrammarRule(
        name: "list_expression",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          NegativeLookahead(element: RuleReference("LESS_THAN", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 217),
    GrammarRule(
        name: "array_expression",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 219),
    GrammarRule(
        name: "record_expression",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_assignment", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_assignment", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 221),
    GrammarRule(
        name: "field_assignment",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 223),
    GrammarRule(
        name: "parenthesized_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 225),
    GrammarRule(
        name: "element_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 227),
    GrammarRule(
        name: "if_expression",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          Literal("then"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 230),
    GrammarRule(
        name: "match_expression",
        body: Sequence(elements: [
          Literal("match"),
          RuleReference("expression", isToken: false),
          Literal("with"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 232),
    GrammarRule(
        name: "match_case",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("when"),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 234),
    GrammarRule(
        name: "fun_expression",
        body: Sequence(elements: [
          Literal("fun"),
          RuleReference("parameter_list", isToken: false),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 236),
    GrammarRule(
        name: "function_expression",
        body: Sequence(elements: [
          Literal("function"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 238),
    GrammarRule(
        name: "let_expression",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ])),
          Literal("in"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 240),
    GrammarRule(
        name: "for_expression",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("pattern", isToken: false),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false),
          Group(
              element:
                  Alternation(choices: [Literal("to"), Literal("downto")])),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 242),
    GrammarRule(
        name: "while_expression",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 244),
    GrammarRule(
        name: "pattern",
        body: Sequence(elements: [
          RuleReference("pattern_atom", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 246),
    GrammarRule(
        name: "pattern_atom",
        body: Alternation(choices: [
          RuleReference("wildcard_pattern", isToken: false),
          RuleReference("literal_pattern", isToken: false),
          RuleReference("tuple_pattern", isToken: false),
          RuleReference("list_pattern", isToken: false),
          RuleReference("array_pattern", isToken: false),
          RuleReference("record_pattern", isToken: false),
          RuleReference("unit_pattern", isToken: false),
          RuleReference("parenthesized_pattern", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 248),
    GrammarRule(
        name: "wildcard_pattern",
        body: RuleReference("UNDERSCORE", isToken: true),
        lineNumber: 260),
    GrammarRule(
        name: "literal_pattern",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 262),
    GrammarRule(
        name: "tuple_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("pattern", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 269),
    GrammarRule(
        name: "list_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 271),
    GrammarRule(
        name: "array_pattern",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 273),
    GrammarRule(
        name: "record_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 275),
    GrammarRule(
        name: "field_pattern",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("pattern", isToken: false)
        ]),
        lineNumber: 277),
    GrammarRule(
        name: "unit_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 279),
    GrammarRule(
        name: "parenthesized_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 281),
  ],
);

final parserGrammarV10 = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "compilation_unit",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("decorated_declaration", isToken: false)
        ])),
        lineNumber: 9),
    GrammarRule(
        name: "decorated_declaration",
        body: Sequence(elements: [
          Repetition(
              element: Sequence(elements: [
            RuleReference("attribute_section", isToken: false),
            RuleReference("NEWLINE", isToken: true)
          ])),
          RuleReference("declaration_body", isToken: false)
        ]),
        lineNumber: 11),
    GrammarRule(
        name: "declaration_body",
        body: Alternation(choices: [
          RuleReference("module_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("open_declaration", isToken: false),
          RuleReference("let_binding", isToken: false),
          RuleReference("use_binding", isToken: false),
          RuleReference("type_declaration", isToken: false),
          RuleReference("member_declaration", isToken: false),
          RuleReference("do_binding", isToken: false),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 13),
    GrammarRule(
        name: "module_declaration",
        body: Sequence(elements: [
          Literal("module"),
          Repetition(element: RuleReference("module_modifier", isToken: false)),
          Optional(element: Literal("rec")),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ]))
        ]),
        lineNumber: 23),
    GrammarRule(
        name: "module_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 25),
    GrammarRule(
        name: "namespace_declaration",
        body: Sequence(elements: [
          Literal("namespace"),
          Repetition(
              element: RuleReference("namespace_modifier", isToken: false)),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 28),
    GrammarRule(
        name: "namespace_modifier",
        body: Alternation(choices: [Literal("private"), Literal("internal")]),
        lineNumber: 30),
    GrammarRule(
        name: "open_declaration",
        body: Sequence(elements: [
          Literal("open"),
          RuleReference("qualified_name", isToken: false)
        ]),
        lineNumber: 33),
    GrammarRule(
        name: "let_binding",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 35),
    GrammarRule(
        name: "use_binding",
        body: Sequence(elements: [
          Literal("use"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ]))
        ]),
        lineNumber: 37),
    GrammarRule(
        name: "binding_modifier",
        body: Alternation(choices: [Literal("inline"), Literal("mutable")]),
        lineNumber: 39),
    GrammarRule(
        name: "binding_clause",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          RuleReference("EQUALS", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 42),
    GrammarRule(
        name: "do_binding",
        body: Sequence(elements: [
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 44),
    GrammarRule(
        name: "member_declaration",
        body: Sequence(elements: [
          Optional(element: RuleReference("member_modifier", isToken: false)),
          Literal("member"),
          RuleReference("qualified_name", isToken: false),
          Optional(element: RuleReference("parameter_list", isToken: false)),
          Optional(element: RuleReference("type_annotation", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 46),
    GrammarRule(
        name: "member_modifier",
        body: Alternation(choices: [
          Literal("static"),
          Literal("override"),
          Literal("default"),
          Literal("abstract"),
          Literal("new")
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "type_declaration",
        body: Sequence(elements: [
          Literal("type"),
          Repetition(element: RuleReference("type_modifier", isToken: false)),
          RuleReference("NAME", isToken: true),
          Optional(
              element: RuleReference("generic_parameters", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(element: RuleReference("parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ])),
          RuleReference("type_definition", isToken: false)
        ]),
        lineNumber: 54),
    GrammarRule(
        name: "type_modifier",
        body: Alternation(choices: [
          Literal("private"),
          Literal("public"),
          Literal("internal")
        ]),
        lineNumber: 56),
    GrammarRule(
        name: "generic_parameters",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_parameter", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_parameter", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 60),
    GrammarRule(
        name: "type_parameter",
        body: Alternation(choices: [
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 62),
    GrammarRule(
        name: "type_definition",
        body: Alternation(choices: [
          RuleReference("class_type", isToken: false),
          RuleReference("interface_type", isToken: false),
          RuleReference("struct_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("union_type", isToken: false),
          RuleReference("alias_type", isToken: false)
        ]),
        lineNumber: 65),
    GrammarRule(
        name: "class_type",
        body: Sequence(elements: [
          Literal("class"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 72),
    GrammarRule(
        name: "interface_type",
        body: Sequence(elements: [
          Literal("interface"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 74),
    GrammarRule(
        name: "struct_type",
        body: Sequence(elements: [
          Literal("struct"),
          Repetition(
              element: Alternation(choices: [
            RuleReference("NEWLINE", isToken: true),
            RuleReference("declaration_body", isToken: false)
          ])),
          Literal("end")
        ]),
        lineNumber: 76),
    GrammarRule(
        name: "record_type",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_declaration", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_declaration", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 78),
    GrammarRule(
        name: "field_declaration",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 80),
    GrammarRule(
        name: "union_type",
        body: Sequence(elements: [
          RuleReference("union_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("union_case", isToken: false)
          ]))
        ]),
        lineNumber: 82),
    GrammarRule(
        name: "union_case",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("of"),
            RuleReference("type_expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("STAR", isToken: true),
              RuleReference("type_expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 84),
    GrammarRule(
        name: "alias_type",
        body: RuleReference("type_expression", isToken: false),
        lineNumber: 86),
    GrammarRule(
        name: "field_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 88),
    GrammarRule(
        name: "case_separator",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NEWLINE", isToken: true),
            Optional(element: RuleReference("PIPE", isToken: true))
          ]),
          RuleReference("PIPE", isToken: true)
        ]),
        lineNumber: 91),
    GrammarRule(
        name: "attribute_section",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("attribute", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("attribute", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 94),
    GrammarRule(
        name: "attribute",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("attribute_target", isToken: false),
            RuleReference("COLON", isToken: true)
          ])),
          RuleReference("qualified_name", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("attribute_arguments", isToken: false)),
            RuleReference("RPAREN", isToken: true)
          ]))
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "attribute_target",
        body: Alternation(choices: [
          Literal("assembly"),
          Literal("field"),
          Literal("method"),
          Literal("module"),
          Literal("param"),
          Literal("property"),
          Literal("return"),
          Literal("type"),
          Literal("event")
        ]),
        lineNumber: 98),
    GrammarRule(
        name: "attribute_arguments",
        body: Sequence(elements: [
          RuleReference("attribute_argument", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("attribute_argument", isToken: false)
          ]))
        ]),
        lineNumber: 108),
    GrammarRule(
        name: "attribute_argument",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true)
          ])),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 110),
    GrammarRule(
        name: "parameter_list",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Repetition(element: RuleReference("pattern", isToken: false))
        ]),
        lineNumber: 112),
    GrammarRule(
        name: "type_annotation",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 114),
    GrammarRule(
        name: "type_expression",
        body: RuleReference("function_type", isToken: false),
        lineNumber: 116),
    GrammarRule(
        name: "function_type",
        body: Sequence(elements: [
          RuleReference("type_product", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("ARROW", isToken: true),
            RuleReference("function_type", isToken: false)
          ]))
        ]),
        lineNumber: 118),
    GrammarRule(
        name: "type_product",
        body: Sequence(elements: [
          RuleReference("type_application", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("STAR", isToken: true),
            RuleReference("type_application", isToken: false)
          ]))
        ]),
        lineNumber: 120),
    GrammarRule(
        name: "type_application",
        body: Sequence(elements: [
          RuleReference("type_atom", isToken: false),
          Repetition(element: RuleReference("type_atom", isToken: false))
        ]),
        lineNumber: 122),
    GrammarRule(
        name: "generic_type_arguments",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 124),
    GrammarRule(
        name: "type_atom",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("qualified_name", isToken: false),
            Optional(
                element:
                    RuleReference("generic_type_arguments", isToken: false))
          ]),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("tuple_type", isToken: false),
          RuleReference("unit_type", isToken: false),
          RuleReference("record_type", isToken: false),
          RuleReference("parenthesized_type", isToken: false)
        ]),
        lineNumber: 126),
    GrammarRule(
        name: "tuple_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("type_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 133),
    GrammarRule(
        name: "unit_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 135),
    GrammarRule(
        name: "parenthesized_type",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 137),
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
        lineNumber: 139),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("sequence_expression", isToken: false)
        ]),
        lineNumber: 141),
    GrammarRule(
        name: "sequence_expression",
        body: Sequence(elements: [
          RuleReference("infix_expression", isToken: false),
          Repetition(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("NEWLINE", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ]),
            Sequence(elements: [
              RuleReference("SEMICOLON", isToken: true),
              RuleReference("infix_expression", isToken: false)
            ])
          ]))
        ]),
        lineNumber: 150),
    GrammarRule(
        name: "infix_expression",
        body: Sequence(elements: [
          RuleReference("application_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("infix_operator", isToken: false),
            RuleReference("application_expression", isToken: false)
          ]))
        ]),
        lineNumber: 152),
    GrammarRule(
        name: "infix_operator",
        body: Alternation(choices: [
          RuleReference("PIPE_RIGHT", isToken: true),
          RuleReference("PIPE_LEFT", isToken: true),
          RuleReference("COMPOSE_RIGHT", isToken: true),
          RuleReference("COMPOSE_LEFT", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
          RuleReference("EQUALS_EQUALS", isToken: true),
          RuleReference("COLON_EQUALS", isToken: true),
          RuleReference("DOUBLE_COLON", isToken: true),
          RuleReference("LARROW", isToken: true),
          RuleReference("ARROW", isToken: true),
          RuleReference("DOT_DOT", isToken: true),
          RuleReference("AND_AND", isToken: true),
          RuleReference("OR_OR", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("PERCENT", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          RuleReference("PIPE", isToken: true),
          RuleReference("CARET", isToken: true)
        ]),
        lineNumber: 154),
    GrammarRule(
        name: "application_expression",
        body: Sequence(elements: [
          RuleReference("prefix_expression", isToken: false),
          Repetition(
              element: RuleReference("prefix_expression", isToken: false))
        ]),
        lineNumber: 181),
    GrammarRule(
        name: "prefix_expression",
        body: Sequence(elements: [
          Optional(element: RuleReference("unary_operator", isToken: false)),
          RuleReference("atomic_expression", isToken: false)
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "unary_operator",
        body: Alternation(choices: [
          RuleReference("MINUS", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("BANG", isToken: true)
        ]),
        lineNumber: 185),
    GrammarRule(
        name: "atomic_expression",
        body: Alternation(choices: [
          RuleReference("computation_expression", isToken: false),
          RuleReference("if_expression", isToken: false),
          RuleReference("match_expression", isToken: false),
          RuleReference("let_expression", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("fun_expression", isToken: false),
          RuleReference("for_expression", isToken: false),
          RuleReference("while_expression", isToken: false),
          RuleReference("unit_expression", isToken: false),
          RuleReference("tuple_expression", isToken: false),
          RuleReference("list_expression", isToken: false),
          RuleReference("array_expression", isToken: false),
          RuleReference("record_expression", isToken: false),
          RuleReference("parenthesized_expression", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 189),
    GrammarRule(
        name: "computation_expression",
        body: Sequence(elements: [
          RuleReference("qualified_name", isToken: false),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("expression", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 211),
    GrammarRule(
        name: "unit_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 213),
    GrammarRule(
        name: "tuple_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 215),
    GrammarRule(
        name: "list_expression",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          NegativeLookahead(element: RuleReference("LESS_THAN", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 217),
    GrammarRule(
        name: "array_expression",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("expression", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("expression", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 219),
    GrammarRule(
        name: "record_expression",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_assignment", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_assignment", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 221),
    GrammarRule(
        name: "field_assignment",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 223),
    GrammarRule(
        name: "parenthesized_expression",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 225),
    GrammarRule(
        name: "element_separator",
        body: Alternation(choices: [
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 227),
    GrammarRule(
        name: "if_expression",
        body: Sequence(elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          Literal("then"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("else"),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            RuleReference("expression", isToken: false)
          ]))
        ]),
        lineNumber: 230),
    GrammarRule(
        name: "match_expression",
        body: Sequence(elements: [
          Literal("match"),
          RuleReference("expression", isToken: false),
          Literal("with"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 232),
    GrammarRule(
        name: "match_case",
        body: Sequence(elements: [
          RuleReference("pattern", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("when"),
            RuleReference("expression", isToken: false)
          ])),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 234),
    GrammarRule(
        name: "fun_expression",
        body: Sequence(elements: [
          Literal("fun"),
          RuleReference("parameter_list", isToken: false),
          RuleReference("ARROW", isToken: true),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 236),
    GrammarRule(
        name: "function_expression",
        body: Sequence(elements: [
          Literal("function"),
          Optional(element: RuleReference("case_separator", isToken: false)),
          RuleReference("match_case", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("case_separator", isToken: false),
            RuleReference("match_case", isToken: false)
          ]))
        ]),
        lineNumber: 238),
    GrammarRule(
        name: "let_expression",
        body: Sequence(elements: [
          Literal("let"),
          Optional(element: Literal("rec")),
          Repetition(
              element: RuleReference("binding_modifier", isToken: false)),
          RuleReference("binding_clause", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("and"),
            Repetition(
                element: RuleReference("binding_modifier", isToken: false)),
            RuleReference("binding_clause", isToken: false)
          ])),
          Literal("in"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 240),
    GrammarRule(
        name: "for_expression",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("pattern", isToken: false),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false),
          Group(
              element:
                  Alternation(choices: [Literal("to"), Literal("downto")])),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 242),
    GrammarRule(
        name: "while_expression",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("expression", isToken: false),
          Literal("do"),
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          RuleReference("expression", isToken: false)
        ]),
        lineNumber: 244),
    GrammarRule(
        name: "pattern",
        body: Sequence(elements: [
          RuleReference("pattern_atom", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 246),
    GrammarRule(
        name: "pattern_atom",
        body: Alternation(choices: [
          RuleReference("wildcard_pattern", isToken: false),
          RuleReference("literal_pattern", isToken: false),
          RuleReference("tuple_pattern", isToken: false),
          RuleReference("list_pattern", isToken: false),
          RuleReference("array_pattern", isToken: false),
          RuleReference("record_pattern", isToken: false),
          RuleReference("unit_pattern", isToken: false),
          RuleReference("parenthesized_pattern", isToken: false),
          RuleReference("qualified_name", isToken: false),
          RuleReference("TYPEVAR", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 248),
    GrammarRule(
        name: "wildcard_pattern",
        body: RuleReference("UNDERSCORE", isToken: true),
        lineNumber: 260),
    GrammarRule(
        name: "literal_pattern",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("NULL", isToken: true)
        ]),
        lineNumber: 262),
    GrammarRule(
        name: "tuple_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("COMMA", isToken: true),
          RuleReference("pattern", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("pattern", isToken: false)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 269),
    GrammarRule(
        name: "list_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 271),
    GrammarRule(
        name: "array_pattern",
        body: Sequence(elements: [
          RuleReference("ARRAY_LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("element_separator", isToken: false),
              RuleReference("pattern", isToken: false)
            ]))
          ])),
          RuleReference("ARRAY_RBRACKET", isToken: true)
        ]),
        lineNumber: 273),
    GrammarRule(
        name: "record_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("field_pattern", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("field_separator", isToken: false),
              RuleReference("field_pattern", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 275),
    GrammarRule(
        name: "field_pattern",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("pattern", isToken: false)
        ]),
        lineNumber: 277),
    GrammarRule(
        name: "unit_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 279),
    GrammarRule(
        name: "parenthesized_pattern",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("pattern", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 281),
  ],
);

final _grammarsByVersion = <String, ParserGrammar>{
  '1.0': parserGrammarV1_0,
  '2.0': parserGrammarV2_0,
  '3.0': parserGrammarV3_0,
  '3.1': parserGrammarV3_1,
  '4.0': parserGrammarV4_0,
  '4.1': parserGrammarV4_1,
  '4.5': parserGrammarV4_5,
  '4.6': parserGrammarV4_6,
  '4.7': parserGrammarV4_7,
  '5': parserGrammarV5,
  '6': parserGrammarV6,
  '7': parserGrammarV7,
  '8': parserGrammarV8,
  '9': parserGrammarV9,
  '10': parserGrammarV10,
};

/// Returns the compiled parser grammar for a resolved
/// [supportedFsharpParserVersions] entry. Callers must resolve
/// through [resolveFsharpParserVersion] first.
ParserGrammar parserGrammarForVersion(String version) =>
    _grammarsByVersion[version]!;
