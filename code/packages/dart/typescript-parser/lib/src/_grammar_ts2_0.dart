// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/typescript/ts2.0.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "program",
        body: Repetition(
            element: RuleReference("source_element", isToken: false)),
        lineNumber: 61),
    GrammarRule(
        name: "source_element",
        body: Alternation(choices: [
          RuleReference("import_declaration", isToken: false),
          RuleReference("export_declaration", isToken: false),
          RuleReference("interface_declaration", isToken: false),
          RuleReference("type_alias_declaration", isToken: false),
          RuleReference("enum_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("ambient_declaration", isToken: false),
          RuleReference("function_declaration", isToken: false),
          RuleReference("generator_declaration", isToken: false),
          RuleReference("ts_class_declaration", isToken: false),
          RuleReference("lexical_declaration", isToken: false),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 66),
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
        lineNumber: 93),
    GrammarRule(
        name: "function_body",
        body: Repetition(
            element: RuleReference("source_element", isToken: false)),
        lineNumber: 97),
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
        lineNumber: 115),
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
        lineNumber: 118),
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
        lineNumber: 120),
    GrammarRule(
        name: "lexical_declaration",
        body: Sequence(elements: [
          Group(
              element:
                  Alternation(choices: [Literal("let"), Literal("const")])),
          RuleReference("typed_binding_list", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 136),
    GrammarRule(
        name: "typed_binding_list",
        body: Sequence(elements: [
          RuleReference("typed_lexical_binding", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("typed_lexical_binding", isToken: false)
          ]))
        ]),
        lineNumber: 138),
    GrammarRule(
        name: "typed_lexical_binding",
        body: Sequence(elements: [
          Group(
              element: Alternation(choices: [
            RuleReference("NAME", isToken: true),
            RuleReference("binding_pattern", isToken: false)
          ])),
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
        lineNumber: 140),
    GrammarRule(
        name: "lexical_declaration_no_semi",
        body: Sequence(elements: [
          Group(
              element:
                  Alternation(choices: [Literal("let"), Literal("const")])),
          RuleReference("typed_binding_list", isToken: false)
        ]),
        lineNumber: 143),
    GrammarRule(
        name: "variable_statement",
        body: Sequence(elements: [
          Literal("var"),
          RuleReference("variable_declaration_list", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 150),
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
        lineNumber: 152),
    GrammarRule(
        name: "variable_declaration",
        body: Sequence(elements: [
          Group(
              element: Alternation(choices: [
            RuleReference("NAME", isToken: true),
            RuleReference("binding_pattern", isToken: false)
          ])),
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
        lineNumber: 154),
    GrammarRule(
        name: "generator_declaration",
        body: Sequence(elements: [
          Literal("function"),
          RuleReference("STAR", isToken: true),
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
        lineNumber: 165),
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
        lineNumber: 190),
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
        lineNumber: 192),
    GrammarRule(
        name: "ts_class_body",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Repetition(
              element: RuleReference("ts_class_element", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 194),
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
        lineNumber: 196),
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
        lineNumber: 201),
    GrammarRule(
        name: "accessibility_modifier",
        body: Alternation(choices: [
          Literal("public"),
          Literal("private"),
          Literal("protected")
        ]),
        lineNumber: 203),
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
        lineNumber: 205),
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
        lineNumber: 210),
    GrammarRule(
        name: "import_declaration",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("import"),
            RuleReference("import_clause", isToken: false),
            RuleReference("from_clause", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            Literal("import"),
            RuleReference("STRING", isToken: true),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            Literal("import"),
            RuleReference("NAME", isToken: true),
            RuleReference("EQUALS", isToken: true),
            Literal("require"),
            RuleReference("LPAREN", isToken: true),
            RuleReference("STRING", isToken: true),
            RuleReference("RPAREN", isToken: true),
            RuleReference("SEMICOLON", isToken: true)
          ])
        ]),
        lineNumber: 232),
    GrammarRule(
        name: "import_clause",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("default_import", isToken: false),
            Optional(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              Group(
                  element: Alternation(choices: [
                RuleReference("named_imports", isToken: false),
                RuleReference("namespace_import", isToken: false)
              ]))
            ]))
          ]),
          RuleReference("named_imports", isToken: false),
          RuleReference("namespace_import", isToken: false)
        ]),
        lineNumber: 239),
    GrammarRule(
        name: "default_import",
        body: RuleReference("NAME", isToken: true),
        lineNumber: 243),
    GrammarRule(
        name: "named_imports",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("import_specifier", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("import_specifier", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 245),
    GrammarRule(
        name: "import_specifier",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 247),
    GrammarRule(
        name: "namespace_import",
        body: Sequence(elements: [
          RuleReference("STAR", isToken: true),
          Literal("as"),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 249),
    GrammarRule(
        name: "from_clause",
        body: Sequence(elements: [
          Literal("from"),
          RuleReference("STRING", isToken: true)
        ]),
        lineNumber: 251),
    GrammarRule(
        name: "export_declaration",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("export"),
            Literal("default"),
            Group(
                element: Alternation(choices: [
              RuleReference("function_declaration", isToken: false),
              RuleReference("generator_declaration", isToken: false),
              RuleReference("ts_class_declaration", isToken: false),
              Sequence(elements: [
                RuleReference("assignment_expression", isToken: false),
                RuleReference("SEMICOLON", isToken: true)
              ])
            ]))
          ]),
          Sequence(elements: [
            Literal("export"),
            Group(
                element: Alternation(choices: [
              RuleReference("function_declaration", isToken: false),
              RuleReference("generator_declaration", isToken: false),
              RuleReference("ts_class_declaration", isToken: false),
              RuleReference("lexical_declaration", isToken: false),
              RuleReference("variable_statement", isToken: false),
              RuleReference("interface_declaration", isToken: false),
              RuleReference("type_alias_declaration", isToken: false),
              RuleReference("enum_declaration", isToken: false),
              RuleReference("namespace_declaration", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            Literal("export"),
            RuleReference("named_exports", isToken: false),
            Optional(element: RuleReference("from_clause", isToken: false)),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            Literal("export"),
            RuleReference("EQUALS", isToken: true),
            RuleReference("NAME", isToken: true),
            RuleReference("SEMICOLON", isToken: true)
          ])
        ]),
        lineNumber: 253),
    GrammarRule(
        name: "named_exports",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("export_specifier", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("export_specifier", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 269),
    GrammarRule(
        name: "export_specifier",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 271),
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
          RuleReference("for_of_statement", isToken: false),
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
        lineNumber: 280),
    GrammarRule(
        name: "block",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("statement", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 300),
    GrammarRule(
        name: "empty_statement",
        body: RuleReference("SEMICOLON", isToken: true),
        lineNumber: 302),
    GrammarRule(
        name: "expression_statement",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 304),
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
        lineNumber: 306),
    GrammarRule(
        name: "while_statement",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 308),
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
        lineNumber: 310),
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
            RuleReference("lexical_declaration_no_semi", isToken: false),
            Optional(element: RuleReference("expression", isToken: false))
          ])),
          RuleReference("SEMICOLON", isToken: true),
          Optional(element: RuleReference("expression", isToken: false)),
          RuleReference("SEMICOLON", isToken: true),
          Optional(element: RuleReference("expression", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 312),
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
            Sequence(elements: [
              Literal("let"),
              Group(
                  element: Alternation(choices: [
                RuleReference("NAME", isToken: true),
                RuleReference("binding_pattern", isToken: false)
              ]))
            ]),
            Sequence(elements: [
              Literal("const"),
              Group(
                  element: Alternation(choices: [
                RuleReference("NAME", isToken: true),
                RuleReference("binding_pattern", isToken: false)
              ]))
            ]),
            RuleReference("left_hand_side_expression", isToken: false)
          ])),
          Literal("in"),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 320),
    GrammarRule(
        name: "for_of_statement",
        body: Sequence(elements: [
          Literal("for"),
          RuleReference("LPAREN", isToken: true),
          Group(
              element: Alternation(choices: [
            Sequence(elements: [
              Literal("var"),
              RuleReference("variable_declaration", isToken: false)
            ]),
            Sequence(elements: [
              Literal("let"),
              Group(
                  element: Alternation(choices: [
                RuleReference("NAME", isToken: true),
                RuleReference("binding_pattern", isToken: false)
              ])),
              Optional(
                  element: Sequence(elements: [
                RuleReference("COLON", isToken: true),
                RuleReference("type_expression", isToken: false)
              ]))
            ]),
            Sequence(elements: [
              Literal("const"),
              Group(
                  element: Alternation(choices: [
                RuleReference("NAME", isToken: true),
                RuleReference("binding_pattern", isToken: false)
              ])),
              Optional(
                  element: Sequence(elements: [
                RuleReference("COLON", isToken: true),
                RuleReference("type_expression", isToken: false)
              ]))
            ]),
            RuleReference("left_hand_side_expression", isToken: false)
          ])),
          Literal("of"),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 329),
    GrammarRule(
        name: "continue_statement",
        body: Sequence(elements: [
          Literal("continue"),
          Optional(element: RuleReference("NAME", isToken: true)),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 336),
    GrammarRule(
        name: "break_statement",
        body: Sequence(elements: [
          Literal("break"),
          Optional(element: RuleReference("NAME", isToken: true)),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 338),
    GrammarRule(
        name: "return_statement",
        body: Sequence(elements: [
          Literal("return"),
          Optional(element: RuleReference("expression", isToken: false)),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 340),
    GrammarRule(
        name: "with_statement",
        body: Sequence(elements: [
          Literal("with"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 342),
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
        lineNumber: 344),
    GrammarRule(
        name: "case_clause",
        body: Sequence(elements: [
          Literal("case"),
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          Repetition(element: RuleReference("statement", isToken: false))
        ]),
        lineNumber: 347),
    GrammarRule(
        name: "default_clause",
        body: Sequence(elements: [
          Literal("default"),
          RuleReference("COLON", isToken: true),
          Repetition(element: RuleReference("statement", isToken: false))
        ]),
        lineNumber: 349),
    GrammarRule(
        name: "labelled_statement",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 351),
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
        lineNumber: 353),
    GrammarRule(
        name: "catch_clause",
        body: Sequence(elements: [
          Literal("catch"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("RPAREN", isToken: true),
          RuleReference("block", isToken: false)
        ]),
        lineNumber: 355),
    GrammarRule(
        name: "finally_clause",
        body: Sequence(elements: [
          Literal("finally"),
          RuleReference("block", isToken: false)
        ]),
        lineNumber: 357),
    GrammarRule(
        name: "throw_statement",
        body: Sequence(elements: [
          Literal("throw"),
          RuleReference("expression", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 359),
    GrammarRule(
        name: "debugger_statement",
        body: Sequence(elements: [
          Literal("debugger"),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 361),
    GrammarRule(
        name: "binding_pattern",
        body: Alternation(choices: [
          RuleReference("object_binding_pattern", isToken: false),
          RuleReference("array_binding_pattern", isToken: false)
        ]),
        lineNumber: 375),
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
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 377),
    GrammarRule(
        name: "binding_property",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("property_name", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("binding_element", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("EQUALS", isToken: true),
              RuleReference("assignment_expression", isToken: false)
            ]))
          ])
        ]),
        lineNumber: 379),
    GrammarRule(
        name: "binding_element",
        body: Sequence(elements: [
          Group(
              element: Alternation(choices: [
            RuleReference("NAME", isToken: true),
            RuleReference("binding_pattern", isToken: false)
          ])),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            RuleReference("assignment_expression", isToken: false)
          ]))
        ]),
        lineNumber: 382),
    GrammarRule(
        name: "array_binding_pattern",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("array_binding_element", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("array_binding_element", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 384),
    GrammarRule(
        name: "array_binding_element",
        body: Alternation(choices: [
          RuleReference("binding_element", isToken: false),
          Sequence(elements: [
            RuleReference("ELLIPSIS", isToken: true),
            Group(
                element: Alternation(choices: [
              RuleReference("NAME", isToken: true),
              RuleReference("binding_pattern", isToken: false)
            ]))
          ])
        ]),
        lineNumber: 386),
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
        lineNumber: 396),
    GrammarRule(
        name: "assignment_expression",
        body: Alternation(choices: [
          RuleReference("arrow_function", isToken: false),
          RuleReference("ts_as_expression", isToken: false),
          RuleReference("ts_angle_bracket_assertion", isToken: false),
          Sequence(elements: [
            Literal("yield"),
            Optional(element: RuleReference("STAR", isToken: true)),
            RuleReference("assignment_expression", isToken: false)
          ]),
          RuleReference("conditional_expression", isToken: false),
          Sequence(elements: [
            RuleReference("left_hand_side_expression", isToken: false),
            RuleReference("assignment_operator", isToken: false),
            RuleReference("assignment_expression", isToken: false)
          ])
        ]),
        lineNumber: 403),
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
        lineNumber: 410),
    GrammarRule(
        name: "arrow_function",
        body: Sequence(elements: [
          RuleReference("arrow_parameters", isToken: false),
          RuleReference("ARROW", isToken: true),
          RuleReference("concise_body", isToken: false)
        ]),
        lineNumber: 426),
    GrammarRule(
        name: "arrow_parameters",
        body: Alternation(choices: [
          RuleReference("NAME", isToken: true),
          Sequence(elements: [
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
          Sequence(elements: [
            RuleReference("LESS_THAN", isToken: true),
            RuleReference("type_parameter_list", isToken: false),
            RuleReference("GREATER_THAN", isToken: true),
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("typed_parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("COLON", isToken: true),
              RuleReference("type_expression", isToken: false)
            ]))
          ])
        ]),
        lineNumber: 428),
    GrammarRule(
        name: "concise_body",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("LBRACE", isToken: true),
            RuleReference("function_body", isToken: false),
            RuleReference("RBRACE", isToken: true)
          ]),
          RuleReference("assignment_expression", isToken: false)
        ]),
        lineNumber: 432),
    GrammarRule(
        name: "ts_as_expression",
        body: Sequence(elements: [
          RuleReference("ts_non_null_expression", isToken: false),
          Literal("as"),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 438),
    GrammarRule(
        name: "ts_non_null_expression",
        body: Sequence(elements: [
          RuleReference("left_hand_side_expression", isToken: false),
          RuleReference("BANG", isToken: true)
        ]),
        lineNumber: 442),
    GrammarRule(
        name: "ts_angle_bracket_assertion",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_expression", isToken: false),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("assignment_expression", isToken: false)
        ]),
        lineNumber: 445),
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
        lineNumber: 449),
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
        lineNumber: 454),
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
        lineNumber: 456),
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
        lineNumber: 460),
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
        lineNumber: 462),
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
        lineNumber: 464),
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
        lineNumber: 468),
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
        lineNumber: 474),
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
        lineNumber: 480),
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
        lineNumber: 485),
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
        lineNumber: 488),
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
        lineNumber: 493),
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
        lineNumber: 506),
    GrammarRule(
        name: "left_hand_side_expression",
        body: Alternation(choices: [
          RuleReference("call_expression", isToken: false),
          RuleReference("new_expression", isToken: false)
        ]),
        lineNumber: 510),
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
            ]),
            RuleReference("template_literal", isToken: false)
          ]))
        ]),
        lineNumber: 514),
    GrammarRule(
        name: "new_expression",
        body: Alternation(choices: [
          RuleReference("member_expression", isToken: false),
          Sequence(elements: [
            Literal("new"),
            RuleReference("new_expression", isToken: false)
          ])
        ]),
        lineNumber: 518),
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
              ]),
              RuleReference("template_literal", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            Literal("super"),
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            Literal("super"),
            RuleReference("LBRACKET", isToken: true),
            RuleReference("expression", isToken: false),
            RuleReference("RBRACKET", isToken: true)
          ]),
          Sequence(elements: [
            Literal("new"),
            RuleReference("member_expression", isToken: false),
            RuleReference("arguments", isToken: false)
          ])
        ]),
        lineNumber: 521),
    GrammarRule(
        name: "arguments",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          Optional(element: RuleReference("argument_list", isToken: false)),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 527),
    GrammarRule(
        name: "argument_list",
        body: Sequence(elements: [
          Group(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("ELLIPSIS", isToken: true),
              RuleReference("assignment_expression", isToken: false)
            ]),
            RuleReference("assignment_expression", isToken: false)
          ])),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            Group(
                element: Alternation(choices: [
              Sequence(elements: [
                RuleReference("ELLIPSIS", isToken: true),
                RuleReference("assignment_expression", isToken: false)
              ]),
              RuleReference("assignment_expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 529),
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
          RuleReference("template_literal", isToken: false),
          RuleReference("array_literal", isToken: false),
          RuleReference("object_literal", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("generator_expression", isToken: false),
          RuleReference("ts_class_expression", isToken: false),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("expression", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 534),
    GrammarRule(
        name: "template_literal",
        body: Alternation(choices: [
          RuleReference("TEMPLATE_NO_SUB", isToken: true),
          Sequence(elements: [
            RuleReference("TEMPLATE_HEAD", isToken: true),
            Repetition(
                element: Sequence(elements: [
              RuleReference("expression", isToken: false),
              RuleReference("TEMPLATE_MIDDLE", isToken: true)
            ])),
            RuleReference("expression", isToken: false),
            RuleReference("TEMPLATE_TAIL", isToken: true)
          ])
        ]),
        lineNumber: 556),
    GrammarRule(
        name: "array_literal",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: RuleReference("array_element_list", isToken: false)),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 561),
    GrammarRule(
        name: "array_element_list",
        body: Sequence(elements: [
          RuleReference("array_element", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("array_element", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 563),
    GrammarRule(
        name: "array_element",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("ELLIPSIS", isToken: true),
            RuleReference("assignment_expression", isToken: false)
          ]),
          RuleReference("assignment_expression", isToken: false)
        ]),
        lineNumber: 565),
    GrammarRule(
        name: "object_literal",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("property_definition", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("property_definition", isToken: false)
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 570),
    GrammarRule(
        name: "property_definition",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("property_name", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("assignment_expression", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("property_name", isToken: false),
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
            RuleReference("ELLIPSIS", isToken: true),
            RuleReference("assignment_expression", isToken: false)
          ]),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 572),
    GrammarRule(
        name: "property_name",
        body: Alternation(choices: [
          RuleReference("NAME", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("NUMBER", isToken: true),
          Sequence(elements: [
            RuleReference("LBRACKET", isToken: true),
            RuleReference("assignment_expression", isToken: false),
            RuleReference("RBRACKET", isToken: true)
          ])
        ]),
        lineNumber: 581),
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
        lineNumber: 586),
    GrammarRule(
        name: "generator_expression",
        body: Sequence(elements: [
          Literal("function"),
          RuleReference("STAR", isToken: true),
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
        lineNumber: 590),
    GrammarRule(
        name: "ts_class_expression",
        body: Sequence(elements: [
          Optional(element: Literal("abstract")),
          Literal("class"),
          Optional(element: RuleReference("NAME", isToken: true)),
          Optional(element: RuleReference("type_parameters", isToken: false)),
          Optional(element: RuleReference("ts_class_heritage", isToken: false)),
          RuleReference("ts_class_body", isToken: false)
        ]),
        lineNumber: 596),
    GrammarRule(
        name: "type_expression",
        body: RuleReference("conditional_type", isToken: false),
        lineNumber: 622),
    GrammarRule(
        name: "conditional_type",
        body: Sequence(elements: [
          RuleReference("union_type", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("extends"),
            RuleReference("type_expression", isToken: false),
            RuleReference("QUESTION", isToken: true),
            RuleReference("type_expression", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ]))
        ]),
        lineNumber: 650),
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
        lineNumber: 655),
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
        lineNumber: 660),
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
        lineNumber: 665),
    GrammarRule(
        name: "primary_type",
        body: Alternation(choices: [
          RuleReference("predefined_type", isToken: false),
          RuleReference("type_reference", isToken: false),
          RuleReference("literal_type", isToken: false),
          RuleReference("object_type", isToken: false),
          RuleReference("tuple_type", isToken: false),
          RuleReference("mapped_type", isToken: false),
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
            Literal("infer"),
            RuleReference("NAME", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("type_expression", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 670),
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
          Literal("null"),
          Literal("never")
        ]),
        lineNumber: 714),
    GrammarRule(
        name: "literal_type",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          Literal("true"),
          Literal("false")
        ]),
        lineNumber: 719),
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
        lineNumber: 723),
    GrammarRule(
        name: "type_arguments",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_argument_list", isToken: false),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 725),
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
        lineNumber: 726),
    GrammarRule(
        name: "type_parameters",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_parameter_list", isToken: false),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 730),
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
        lineNumber: 731),
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
        lineNumber: 732),
    GrammarRule(
        name: "object_type",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Repetition(
              element: RuleReference("type_member_semicolon", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 736),
    GrammarRule(
        name: "type_member_semicolon",
        body: Sequence(elements: [
          RuleReference("type_member", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 737),
    GrammarRule(
        name: "type_member",
        body: Alternation(choices: [
          RuleReference("construct_signature", isToken: false),
          RuleReference("call_signature", isToken: false),
          RuleReference("index_signature", isToken: false),
          RuleReference("method_signature", isToken: false),
          RuleReference("property_signature", isToken: false)
        ]),
        lineNumber: 738),
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
        lineNumber: 744),
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
        lineNumber: 745),
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
        lineNumber: 746),
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
        lineNumber: 747),
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
        lineNumber: 748),
    GrammarRule(
        name: "mapped_type",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(element: RuleReference("readonly_modifier", isToken: false)),
          RuleReference("LBRACKET", isToken: true),
          RuleReference("NAME", isToken: true),
          Literal("in"),
          RuleReference("type_expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("as"),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("RBRACKET", isToken: true),
          Optional(element: RuleReference("question_modifier", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])),
          RuleReference("SEMICOLON", isToken: true),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 790),
    GrammarRule(
        name: "readonly_modifier",
        body: Alternation(choices: [
          Literal("readonly"),
          Sequence(elements: [
            RuleReference("PLUS", isToken: true),
            Literal("readonly")
          ]),
          Sequence(elements: [
            RuleReference("MINUS", isToken: true),
            Literal("readonly")
          ])
        ]),
        lineNumber: 793),
    GrammarRule(
        name: "question_modifier",
        body: Alternation(choices: [
          RuleReference("QUESTION", isToken: true),
          Sequence(elements: [
            RuleReference("PLUS", isToken: true),
            RuleReference("QUESTION", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("MINUS", isToken: true),
            RuleReference("QUESTION", isToken: true)
          ])
        ]),
        lineNumber: 798),
    GrammarRule(
        name: "tuple_type",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: RuleReference("tuple_element_list", isToken: false)),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 804),
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
        lineNumber: 805),
    GrammarRule(
        name: "tuple_element",
        body: Sequence(elements: [
          Optional(element: RuleReference("ELLIPSIS", isToken: true)),
          RuleReference("type_expression", isToken: false),
          Optional(element: RuleReference("QUESTION", isToken: true))
        ]),
        lineNumber: 806),
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
        lineNumber: 810),
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
        lineNumber: 814),
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
        lineNumber: 834),
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
        lineNumber: 835),
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
        lineNumber: 850),
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
        lineNumber: 860),
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
        lineNumber: 861),
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
        lineNumber: 862),
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
        lineNumber: 874),
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
        lineNumber: 875),
    GrammarRule(
        name: "namespace_element",
        body: Alternation(choices: [
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("interface_declaration", isToken: false),
          RuleReference("type_alias_declaration", isToken: false),
          RuleReference("ts_class_declaration", isToken: false),
          RuleReference("function_declaration", isToken: false),
          RuleReference("generator_declaration", isToken: false),
          RuleReference("enum_declaration", isToken: false),
          RuleReference("lexical_declaration", isToken: false),
          RuleReference("variable_statement", isToken: false),
          RuleReference("export_assignment", isToken: false),
          RuleReference("export_namespace_element", isToken: false),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 877),
    GrammarRule(
        name: "export_assignment",
        body: Sequence(elements: [
          Literal("export"),
          RuleReference("EQUALS", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 890),
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
            RuleReference("generator_declaration", isToken: false),
            RuleReference("enum_declaration", isToken: false),
            RuleReference("lexical_declaration", isToken: false),
            RuleReference("variable_statement", isToken: false)
          ]))
        ]),
        lineNumber: 892),
    GrammarRule(
        name: "ambient_declaration",
        body: Sequence(elements: [
          Literal("declare"),
          RuleReference("ambient_declaration_body", isToken: false)
        ]),
        lineNumber: 912),
    GrammarRule(
        name: "ambient_declaration_body",
        body: Alternation(choices: [
          RuleReference("variable_statement", isToken: false),
          RuleReference("ambient_function_declaration", isToken: false),
          RuleReference("generator_declaration", isToken: false),
          RuleReference("ts_class_declaration", isToken: false),
          RuleReference("interface_declaration", isToken: false),
          RuleReference("type_alias_declaration", isToken: false),
          RuleReference("enum_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("ambient_module_declaration", isToken: false),
          RuleReference("ambient_global_augmentation", isToken: false)
        ]),
        lineNumber: 913),
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
        lineNumber: 924),
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
        lineNumber: 928),
    GrammarRule(
        name: "ambient_global_augmentation",
        body: Sequence(elements: [
          Literal("global"),
          RuleReference("LBRACE", isToken: true),
          Repetition(
              element: RuleReference("namespace_element", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 932),
    GrammarRule(
        name: "type_predicate",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Literal("is"),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 955),
    GrammarRule(
        name: "type_annotation",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 961),
  ],
);
