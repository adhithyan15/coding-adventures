// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/typescript/ts5.8.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "program",
        body: Sequence(elements: [
          Optional(element: RuleReference("HASHBANG", isToken: true)),
          Repetition(element: RuleReference("source_element", isToken: false))
        ]),
        lineNumber: 71),
    GrammarRule(
        name: "source_element",
        body: Alternation(choices: [
          RuleReference("import_declaration", isToken: false),
          RuleReference("export_declaration", isToken: false),
          RuleReference("function_declaration", isToken: false),
          RuleReference("generator_declaration", isToken: false),
          RuleReference("async_function_declaration", isToken: false),
          RuleReference("async_generator_declaration", isToken: false),
          RuleReference("ts_class_declaration", isToken: false),
          RuleReference("interface_declaration", isToken: false),
          RuleReference("type_alias_declaration", isToken: false),
          RuleReference("enum_declaration", isToken: false),
          RuleReference("namespace_declaration", isToken: false),
          RuleReference("ambient_declaration", isToken: false),
          Sequence(elements: [
            RuleReference("decorator", isToken: false),
            RuleReference("ts_class_declaration", isToken: false)
          ]),
          RuleReference("lexical_declaration", isToken: false),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 73),
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
        lineNumber: 99),
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
        lineNumber: 103),
    GrammarRule(
        name: "yield_expression",
        body: Sequence(elements: [
          Literal("yield"),
          Optional(element: RuleReference("STAR", isToken: true)),
          RuleReference("assignment_expression", isToken: false)
        ]),
        lineNumber: 107),
    GrammarRule(
        name: "async_function_declaration",
        body: Sequence(elements: [
          Literal("async"),
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
        lineNumber: 109),
    GrammarRule(
        name: "async_function_expression",
        body: Sequence(elements: [
          Literal("async"),
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
        lineNumber: 113),
    GrammarRule(
        name: "async_arrow_function",
        body: Sequence(elements: [
          Literal("async"),
          RuleReference("arrow_parameters", isToken: false),
          RuleReference("ARROW", isToken: true),
          RuleReference("concise_body", isToken: false)
        ]),
        lineNumber: 117),
    GrammarRule(
        name: "async_method",
        body: Sequence(elements: [
          Literal("async"),
          Optional(element: RuleReference("STAR", isToken: true)),
          RuleReference("property_name", isToken: false),
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
        lineNumber: 119),
    GrammarRule(
        name: "await_expression",
        body: Sequence(elements: [
          Literal("await"),
          RuleReference("unary_expression", isToken: false)
        ]),
        lineNumber: 123),
    GrammarRule(
        name: "async_generator_declaration",
        body: Sequence(elements: [
          Literal("async"),
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
        lineNumber: 125),
    GrammarRule(
        name: "async_generator_expression",
        body: Sequence(elements: [
          Literal("async"),
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
        lineNumber: 130),
    GrammarRule(
        name: "lexical_declaration",
        body: Sequence(elements: [
          Group(
              element:
                  Alternation(choices: [Literal("let"), Literal("const")])),
          RuleReference("binding_list", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 135),
    GrammarRule(
        name: "binding_list",
        body: Sequence(elements: [
          RuleReference("lexical_binding", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("lexical_binding", isToken: false)
          ]))
        ]),
        lineNumber: 137),
    GrammarRule(
        name: "lexical_binding",
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
        lineNumber: 139),
    GrammarRule(
        name: "using_declaration",
        body: Sequence(elements: [
          Literal("using"),
          RuleReference("binding_list", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 169),
    GrammarRule(
        name: "await_using_declaration",
        body: Sequence(elements: [
          Literal("await"),
          Literal("using"),
          RuleReference("binding_list", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 171),
    GrammarRule(
        name: "decorator",
        body: Sequence(elements: [
          RuleReference("AT", isToken: true),
          RuleReference("decorator_expression", isToken: false)
        ]),
        lineNumber: 192),
    GrammarRule(
        name: "decorator_expression",
        body: Alternation(choices: [
          RuleReference("decorator_member_expression", isToken: false),
          Sequence(elements: [
            RuleReference("decorator_member_expression", isToken: false),
            RuleReference("arguments", isToken: false)
          ])
        ]),
        lineNumber: 197),
    GrammarRule(
        name: "decorator_member_expression",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Repetition(
              element: Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("NAME", isToken: true)
          ]))
        ]),
        lineNumber: 200),
    GrammarRule(
        name: "decorated_class_declaration",
        body: Sequence(elements: [
          Repetition(element: RuleReference("decorator", isToken: false)),
          RuleReference("ts_class_declaration", isToken: false)
        ]),
        lineNumber: 209),
    GrammarRule(
        name: "import_declaration",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("import"),
            RuleReference("import_clause", isToken: false),
            RuleReference("from_clause", isToken: false),
            Optional(
                element: RuleReference("import_attributes", isToken: false)),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            Literal("import"),
            RuleReference("module_specifier", isToken: false),
            Optional(
                element: RuleReference("import_attributes", isToken: false)),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            Literal("import"),
            Literal("type"),
            RuleReference("import_clause", isToken: false),
            RuleReference("from_clause", isToken: false),
            Optional(
                element: RuleReference("import_attributes", isToken: false)),
            RuleReference("SEMICOLON", isToken: true)
          ])
        ]),
        lineNumber: 225),
    GrammarRule(
        name: "import_clause",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("default_import", isToken: false),
            Optional(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("named_imports", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            RuleReference("default_import", isToken: false),
            Optional(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("namespace_import", isToken: false)
            ]))
          ]),
          RuleReference("named_imports", isToken: false),
          RuleReference("namespace_import", isToken: false)
        ]),
        lineNumber: 229),
    GrammarRule(
        name: "default_import",
        body: RuleReference("NAME", isToken: true),
        lineNumber: 234),
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
        lineNumber: 236),
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
        lineNumber: 238),
    GrammarRule(
        name: "namespace_import",
        body: Sequence(elements: [
          RuleReference("STAR", isToken: true),
          Literal("as"),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 240),
    GrammarRule(
        name: "from_clause",
        body: Sequence(elements: [
          Literal("from"),
          RuleReference("STRING", isToken: true)
        ]),
        lineNumber: 242),
    GrammarRule(
        name: "module_specifier",
        body: RuleReference("STRING", isToken: true),
        lineNumber: 244),
    GrammarRule(
        name: "import_attributes",
        body: Sequence(elements: [
          Literal("with"),
          RuleReference("LBRACE", isToken: true),
          RuleReference("attribute_list", isToken: false),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 247),
    GrammarRule(
        name: "attribute_list",
        body: Sequence(elements: [
          RuleReference("import_attribute", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("import_attribute", isToken: false)
          ])),
          Optional(element: RuleReference("COMMA", isToken: true))
        ]),
        lineNumber: 249),
    GrammarRule(
        name: "import_attribute",
        body: Sequence(elements: [
          Group(
              element: Alternation(choices: [
            RuleReference("NAME", isToken: true),
            RuleReference("STRING", isToken: true)
          ])),
          RuleReference("COLON", isToken: true),
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
              RuleReference("async_function_declaration", isToken: false),
              RuleReference("async_generator_declaration", isToken: false),
              RuleReference("decorated_class_declaration", isToken: false),
              RuleReference("ts_class_declaration", isToken: false),
              RuleReference("interface_declaration", isToken: false),
              RuleReference("type_alias_declaration", isToken: false),
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
              RuleReference("async_function_declaration", isToken: false),
              RuleReference("async_generator_declaration", isToken: false),
              RuleReference("decorated_class_declaration", isToken: false),
              RuleReference("ts_class_declaration", isToken: false),
              RuleReference("interface_declaration", isToken: false),
              RuleReference("type_alias_declaration", isToken: false),
              RuleReference("enum_declaration", isToken: false),
              RuleReference("namespace_declaration", isToken: false),
              RuleReference("lexical_declaration", isToken: false),
              RuleReference("variable_statement", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            Literal("export"),
            RuleReference("named_exports", isToken: false),
            Optional(
                element: Sequence(elements: [
              RuleReference("from_clause", isToken: false),
              Optional(
                  element: RuleReference("import_attributes", isToken: false))
            ])),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            Literal("export"),
            RuleReference("STAR", isToken: true),
            RuleReference("from_clause", isToken: false),
            Optional(
                element: RuleReference("import_attributes", isToken: false)),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            Literal("export"),
            Literal("type"),
            RuleReference("named_exports", isToken: false),
            Optional(
                element: Sequence(elements: [
              RuleReference("from_clause", isToken: false),
              Optional(
                  element: RuleReference("import_attributes", isToken: false))
            ])),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            Literal("export"),
            Literal("type"),
            RuleReference("STAR", isToken: true),
            RuleReference("from_clause", isToken: false),
            Optional(
                element: RuleReference("import_attributes", isToken: false)),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            Literal("export"),
            Literal("type"),
            RuleReference("STAR", isToken: true),
            Literal("as"),
            RuleReference("NAME", isToken: true),
            RuleReference("from_clause", isToken: false),
            Optional(
                element: RuleReference("import_attributes", isToken: false)),
            RuleReference("SEMICOLON", isToken: true)
          ])
        ]),
        lineNumber: 265),
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
        lineNumber: 287),
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
        lineNumber: 289),
    GrammarRule(
        name: "binding_pattern",
        body: Alternation(choices: [
          RuleReference("object_binding_pattern", isToken: false),
          RuleReference("array_binding_pattern", isToken: false)
        ]),
        lineNumber: 295),
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
          Optional(
              element: RuleReference("object_rest_property", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 298),
    GrammarRule(
        name: "object_rest_property",
        body: Sequence(elements: [
          RuleReference("ELLIPSIS", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 301),
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
            ])),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 303),
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
        lineNumber: 305),
    GrammarRule(
        name: "binding_element",
        body: Alternation(choices: [
          Sequence(elements: [
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
          Sequence(elements: [
            RuleReference("ELLIPSIS", isToken: true),
            RuleReference("NAME", isToken: true)
          ])
        ]),
        lineNumber: 308),
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
          RuleReference("for_await_of_statement", isToken: false),
          RuleReference("continue_statement", isToken: false),
          RuleReference("break_statement", isToken: false),
          RuleReference("return_statement", isToken: false),
          RuleReference("with_statement", isToken: false),
          RuleReference("switch_statement", isToken: false),
          RuleReference("labelled_statement", isToken: false),
          RuleReference("try_statement", isToken: false),
          RuleReference("throw_statement", isToken: false),
          RuleReference("debugger_statement", isToken: false),
          RuleReference("lexical_declaration", isToken: false),
          RuleReference("using_declaration", isToken: false),
          RuleReference("await_using_declaration", isToken: false),
          RuleReference("expression_statement", isToken: false)
        ]),
        lineNumber: 318),
    GrammarRule(
        name: "block",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("statement", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 342),
    GrammarRule(
        name: "variable_statement",
        body: Sequence(elements: [
          Literal("var"),
          RuleReference("variable_declaration_list", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 344),
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
        lineNumber: 346),
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
        lineNumber: 348),
    GrammarRule(
        name: "empty_statement",
        body: RuleReference("SEMICOLON", isToken: true),
        lineNumber: 350),
    GrammarRule(
        name: "expression_statement",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 352),
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
        lineNumber: 354),
    GrammarRule(
        name: "while_statement",
        body: Sequence(elements: [
          Literal("while"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 356),
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
        lineNumber: 358),
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
            Sequence(elements: [
              Literal("let"),
              RuleReference("binding_list", isToken: false)
            ]),
            Sequence(elements: [
              Literal("const"),
              RuleReference("binding_list", isToken: false)
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
        lineNumber: 360),
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
              RuleReference("binding_element", isToken: false)
            ]),
            Sequence(elements: [
              Literal("const"),
              RuleReference("binding_element", isToken: false)
            ]),
            RuleReference("left_hand_side_expression", isToken: false)
          ])),
          Literal("in"),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 369),
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
              RuleReference("binding_element", isToken: false)
            ]),
            Sequence(elements: [
              Literal("const"),
              RuleReference("binding_element", isToken: false)
            ]),
            Sequence(elements: [
              Literal("using"),
              RuleReference("binding_element", isToken: false)
            ]),
            RuleReference("left_hand_side_expression", isToken: false)
          ])),
          Literal("of"),
          RuleReference("assignment_expression", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 378),
    GrammarRule(
        name: "for_await_of_statement",
        body: Sequence(elements: [
          Literal("for"),
          Literal("await"),
          RuleReference("LPAREN", isToken: true),
          Group(
              element: Alternation(choices: [
            Sequence(elements: [
              Literal("var"),
              RuleReference("variable_declaration", isToken: false)
            ]),
            Sequence(elements: [
              Literal("let"),
              RuleReference("binding_element", isToken: false)
            ]),
            Sequence(elements: [
              Literal("const"),
              RuleReference("binding_element", isToken: false)
            ]),
            Sequence(elements: [
              Literal("using"),
              RuleReference("binding_element", isToken: false)
            ]),
            Sequence(elements: [
              Literal("await"),
              Literal("using"),
              RuleReference("binding_element", isToken: false)
            ]),
            RuleReference("left_hand_side_expression", isToken: false)
          ])),
          Literal("of"),
          RuleReference("assignment_expression", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 387),
    GrammarRule(
        name: "continue_statement",
        body: Sequence(elements: [
          Literal("continue"),
          Optional(element: RuleReference("NAME", isToken: true)),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 396),
    GrammarRule(
        name: "break_statement",
        body: Sequence(elements: [
          Literal("break"),
          Optional(element: RuleReference("NAME", isToken: true)),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 398),
    GrammarRule(
        name: "return_statement",
        body: Sequence(elements: [
          Literal("return"),
          Optional(element: RuleReference("expression", isToken: false)),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 400),
    GrammarRule(
        name: "with_statement",
        body: Sequence(elements: [
          Literal("with"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 402),
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
        lineNumber: 404),
    GrammarRule(
        name: "case_clause",
        body: Sequence(elements: [
          Literal("case"),
          RuleReference("expression", isToken: false),
          RuleReference("COLON", isToken: true),
          Repetition(element: RuleReference("statement", isToken: false))
        ]),
        lineNumber: 407),
    GrammarRule(
        name: "default_clause",
        body: Sequence(elements: [
          Literal("default"),
          RuleReference("COLON", isToken: true),
          Repetition(element: RuleReference("statement", isToken: false))
        ]),
        lineNumber: 409),
    GrammarRule(
        name: "labelled_statement",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("statement", isToken: false)
        ]),
        lineNumber: 411),
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
        lineNumber: 413),
    GrammarRule(
        name: "catch_clause",
        body: Sequence(elements: [
          Literal("catch"),
          Optional(
              element: Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("NAME", isToken: true),
            RuleReference("RPAREN", isToken: true)
          ])),
          RuleReference("block", isToken: false)
        ]),
        lineNumber: 415),
    GrammarRule(
        name: "finally_clause",
        body: Sequence(elements: [
          Literal("finally"),
          RuleReference("block", isToken: false)
        ]),
        lineNumber: 417),
    GrammarRule(
        name: "throw_statement",
        body: Sequence(elements: [
          Literal("throw"),
          RuleReference("expression", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 419),
    GrammarRule(
        name: "debugger_statement",
        body: Sequence(elements: [
          Literal("debugger"),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 421),
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
        lineNumber: 427),
    GrammarRule(
        name: "assignment_expression",
        body: Alternation(choices: [
          RuleReference("conditional_expression", isToken: false),
          RuleReference("arrow_function", isToken: false),
          RuleReference("async_arrow_function", isToken: false),
          RuleReference("yield_expression", isToken: false),
          RuleReference("ts_as_expression", isToken: false),
          RuleReference("ts_satisfies_expression", isToken: false),
          Sequence(elements: [
            RuleReference("left_hand_side_expression", isToken: false),
            RuleReference("assignment_operator", isToken: false),
            RuleReference("assignment_expression", isToken: false)
          ])
        ]),
        lineNumber: 429),
    GrammarRule(
        name: "assignment_operator",
        body: Alternation(choices: [
          RuleReference("EQUALS", isToken: true),
          RuleReference("PLUS_EQUALS", isToken: true),
          RuleReference("MINUS_EQUALS", isToken: true),
          RuleReference("STAR_STAR_EQUALS", isToken: true),
          RuleReference("STAR_EQUALS", isToken: true),
          RuleReference("SLASH_EQUALS", isToken: true),
          RuleReference("PERCENT_EQUALS", isToken: true),
          RuleReference("AMPERSAND_EQUALS", isToken: true),
          RuleReference("PIPE_EQUALS", isToken: true),
          RuleReference("CARET_EQUALS", isToken: true),
          RuleReference("LEFT_SHIFT_EQUALS", isToken: true),
          RuleReference("RIGHT_SHIFT_EQUALS", isToken: true),
          RuleReference("UNSIGNED_RIGHT_SHIFT_EQUALS", isToken: true),
          RuleReference("OR_OR_EQUALS", isToken: true),
          RuleReference("AND_AND_EQUALS", isToken: true),
          RuleReference("NULLISH_COALESCE_EQUALS", isToken: true)
        ]),
        lineNumber: 437),
    GrammarRule(
        name: "conditional_expression",
        body: Sequence(elements: [
          RuleReference("nullish_coalescing_expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("QUESTION", isToken: true),
            RuleReference("assignment_expression", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("assignment_expression", isToken: false)
          ]))
        ]),
        lineNumber: 444),
    GrammarRule(
        name: "nullish_coalescing_expression",
        body: Sequence(elements: [
          RuleReference("logical_or_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("NULLISH_COALESCE", isToken: true),
            RuleReference("logical_or_expression", isToken: false)
          ]))
        ]),
        lineNumber: 447),
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
        lineNumber: 450),
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
        lineNumber: 452),
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
        lineNumber: 454),
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
        lineNumber: 456),
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
        lineNumber: 458),
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
        lineNumber: 460),
    GrammarRule(
        name: "relational_expression",
        body: Alternation(choices: [
          Sequence(elements: [
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
          Sequence(elements: [
            RuleReference("PRIVATE_NAME", isToken: true),
            Literal("in"),
            RuleReference("shift_expression", isToken: false)
          ])
        ]),
        lineNumber: 465),
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
        lineNumber: 470),
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
        lineNumber: 473),
    GrammarRule(
        name: "multiplicative_expression",
        body: Sequence(elements: [
          RuleReference("exponentiation_expression", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true),
              RuleReference("PERCENT", isToken: true)
            ])),
            RuleReference("exponentiation_expression", isToken: false)
          ]))
        ]),
        lineNumber: 476),
    GrammarRule(
        name: "exponentiation_expression",
        body: Sequence(elements: [
          RuleReference("unary_expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("STAR_STAR", isToken: true),
            RuleReference("exponentiation_expression", isToken: false)
          ]))
        ]),
        lineNumber: 479),
    GrammarRule(
        name: "unary_expression",
        body: Alternation(choices: [
          RuleReference("postfix_expression", isToken: false),
          RuleReference("await_expression", isToken: false),
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
        lineNumber: 481),
    GrammarRule(
        name: "postfix_expression",
        body: Sequence(elements: [
          RuleReference("left_hand_side_expression", isToken: false),
          Optional(
              element: Alternation(choices: [
            RuleReference("PLUS_PLUS", isToken: true),
            RuleReference("MINUS_MINUS", isToken: true),
            RuleReference("BANG", isToken: true)
          ]))
        ]),
        lineNumber: 494),
    GrammarRule(
        name: "left_hand_side_expression",
        body: Alternation(choices: [
          RuleReference("call_expression", isToken: false),
          RuleReference("optional_chain_expression", isToken: false),
          RuleReference("new_expression", isToken: false)
        ]),
        lineNumber: 496),
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
              RuleReference("DOT", isToken: true),
              RuleReference("PRIVATE_NAME", isToken: true)
            ]),
            Sequence(elements: [
              RuleReference("LBRACKET", isToken: true),
              RuleReference("expression", isToken: false),
              RuleReference("RBRACKET", isToken: true)
            ]),
            RuleReference("template_literal", isToken: false)
          ]))
        ]),
        lineNumber: 500),
    GrammarRule(
        name: "optional_chain_expression",
        body: Sequence(elements: [
          RuleReference("member_expression", isToken: false),
          Repetition(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("OPTIONAL_CHAIN", isToken: true),
              RuleReference("NAME", isToken: true)
            ]),
            Sequence(elements: [
              RuleReference("OPTIONAL_CHAIN", isToken: true),
              RuleReference("PRIVATE_NAME", isToken: true)
            ]),
            Sequence(elements: [
              RuleReference("OPTIONAL_CHAIN", isToken: true),
              RuleReference("LBRACKET", isToken: true),
              RuleReference("expression", isToken: false),
              RuleReference("RBRACKET", isToken: true)
            ]),
            Sequence(elements: [
              RuleReference("OPTIONAL_CHAIN", isToken: true),
              RuleReference("arguments", isToken: false)
            ]),
            Sequence(elements: [
              RuleReference("DOT", isToken: true),
              RuleReference("NAME", isToken: true)
            ]),
            Sequence(elements: [
              RuleReference("DOT", isToken: true),
              RuleReference("PRIVATE_NAME", isToken: true)
            ]),
            Sequence(elements: [
              RuleReference("LBRACKET", isToken: true),
              RuleReference("expression", isToken: false),
              RuleReference("RBRACKET", isToken: true)
            ]),
            RuleReference("arguments", isToken: false),
            RuleReference("template_literal", isToken: false)
          ]))
        ]),
        lineNumber: 504),
    GrammarRule(
        name: "new_expression",
        body: Alternation(choices: [
          RuleReference("member_expression", isToken: false),
          Sequence(elements: [
            Literal("new"),
            RuleReference("new_expression", isToken: false)
          ])
        ]),
        lineNumber: 515),
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
                RuleReference("DOT", isToken: true),
                RuleReference("PRIVATE_NAME", isToken: true)
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
            Literal("new"),
            RuleReference("member_expression", isToken: false),
            RuleReference("arguments", isToken: false)
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
            RuleReference("DOT", isToken: true),
            Literal("target")
          ])
        ]),
        lineNumber: 518),
    GrammarRule(
        name: "arguments",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("argument_list", isToken: false),
            Optional(element: RuleReference("COMMA", isToken: true))
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 526),
    GrammarRule(
        name: "argument_list",
        body: Sequence(elements: [
          Group(
              element: Alternation(choices: [
            RuleReference("spread_element", isToken: false),
            RuleReference("assignment_expression", isToken: false)
          ])),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            Group(
                element: Alternation(choices: [
              RuleReference("spread_element", isToken: false),
              RuleReference("assignment_expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 528),
    GrammarRule(
        name: "spread_element",
        body: Sequence(elements: [
          RuleReference("ELLIPSIS", isToken: true),
          RuleReference("assignment_expression", isToken: false)
        ]),
        lineNumber: 531),
    GrammarRule(
        name: "arrow_function",
        body: Sequence(elements: [
          RuleReference("arrow_parameters", isToken: false),
          RuleReference("ARROW", isToken: true),
          RuleReference("concise_body", isToken: false)
        ]),
        lineNumber: 533),
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
        lineNumber: 535),
    GrammarRule(
        name: "concise_body",
        body: Alternation(choices: [
          RuleReference("assignment_expression", isToken: false),
          Sequence(elements: [
            RuleReference("LBRACE", isToken: true),
            RuleReference("function_body", isToken: false),
            RuleReference("RBRACE", isToken: true)
          ])
        ]),
        lineNumber: 539),
    GrammarRule(
        name: "primary_expression",
        body: Alternation(choices: [
          Literal("this"),
          RuleReference("NAME", isToken: true),
          RuleReference("NUMBER", isToken: true),
          RuleReference("BIGINT", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("REGEX", isToken: true),
          Literal("true"),
          Literal("false"),
          Literal("null"),
          RuleReference("array_literal", isToken: false),
          RuleReference("object_literal", isToken: false),
          RuleReference("function_expression", isToken: false),
          RuleReference("generator_expression", isToken: false),
          RuleReference("async_function_expression", isToken: false),
          RuleReference("ts_class_expression", isToken: false),
          RuleReference("template_literal", isToken: false),
          RuleReference("dynamic_import", isToken: false),
          RuleReference("import_meta", isToken: false),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("expression", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 542),
    GrammarRule(
        name: "dynamic_import",
        body: Sequence(elements: [
          Literal("import"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("assignment_expression", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("LBRACE", isToken: true),
            Literal("with"),
            RuleReference("COLON", isToken: true),
            RuleReference("import_attributes", isToken: false),
            RuleReference("RBRACE", isToken: true)
          ])),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 563),
    GrammarRule(
        name: "import_meta",
        body: Sequence(elements: [
          Literal("import"),
          RuleReference("DOT", isToken: true),
          Literal("meta")
        ]),
        lineNumber: 565),
    GrammarRule(
        name: "array_literal",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(element: RuleReference("element_list", isToken: false)),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 567),
    GrammarRule(
        name: "element_list",
        body: Sequence(elements: [
          Optional(
              element: Alternation(choices: [
            RuleReference("spread_element", isToken: false),
            RuleReference("assignment_expression", isToken: false)
          ])),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            Optional(
                element: Alternation(choices: [
              RuleReference("spread_element", isToken: false),
              RuleReference("assignment_expression", isToken: false)
            ]))
          ]))
        ]),
        lineNumber: 569),
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
        lineNumber: 572),
    GrammarRule(
        name: "property_definition",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("property_name", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("assignment_expression", isToken: false)
          ]),
          RuleReference("NAME", isToken: true),
          RuleReference("method_definition", isToken: false),
          RuleReference("async_method", isToken: false),
          RuleReference("object_spread_property", isToken: false)
        ]),
        lineNumber: 574),
    GrammarRule(
        name: "object_spread_property",
        body: Sequence(elements: [
          RuleReference("ELLIPSIS", isToken: true),
          RuleReference("assignment_expression", isToken: false)
        ]),
        lineNumber: 580),
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
        lineNumber: 582),
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
        lineNumber: 587),
    GrammarRule(
        name: "method_definition",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("property_name", isToken: false),
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
        lineNumber: 591),
    GrammarRule(
        name: "template_literal",
        body: Alternation(choices: [
          RuleReference("TEMPLATE_NO_SUB", isToken: true),
          Sequence(elements: [
            RuleReference("TEMPLATE_HEAD", isToken: true),
            Repetition(element: RuleReference("template_span", isToken: false)),
            RuleReference("TEMPLATE_TAIL", isToken: true)
          ])
        ]),
        lineNumber: 600),
    GrammarRule(
        name: "template_span",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          RuleReference("TEMPLATE_MIDDLE", isToken: true)
        ]),
        lineNumber: 603),
    GrammarRule(
        name: "type_annotation",
        body: Sequence(elements: [
          RuleReference("COLON", isToken: true),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 609),
    GrammarRule(
        name: "type_expression",
        body: RuleReference("conditional_type", isToken: false),
        lineNumber: 611),
    GrammarRule(
        name: "conditional_type",
        body: Alternation(choices: [
          RuleReference("union_type", isToken: false),
          Sequence(elements: [
            RuleReference("union_type", isToken: false),
            Literal("extends"),
            RuleReference("type_expression", isToken: false),
            RuleReference("QUESTION", isToken: true),
            RuleReference("type_expression", isToken: false),
            RuleReference("COLON", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])
        ]),
        lineNumber: 620),
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
        lineNumber: 623),
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
        lineNumber: 625),
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
        lineNumber: 627),
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
          RuleReference("mapped_type", isToken: false),
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
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              Literal("extends"),
              RuleReference("type_expression", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            Literal("readonly"),
            RuleReference("array_type", isToken: false)
          ]),
          RuleReference("template_literal_type", isToken: false),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("type_expression", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 633),
    GrammarRule(
        name: "predefined_type",
        body: Alternation(choices: [
          Literal("any"),
          Literal("string"),
          Literal("number"),
          Literal("boolean"),
          Literal("void"),
          Literal("never"),
          Literal("object"),
          Literal("symbol"),
          Literal("bigint"),
          Literal("undefined"),
          Literal("null"),
          Literal("unknown")
        ]),
        lineNumber: 649),
    GrammarRule(
        name: "literal_type",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("BIGINT", isToken: true),
          RuleReference("STRING", isToken: true),
          Literal("true"),
          Literal("false")
        ]),
        lineNumber: 652),
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
        lineNumber: 654),
    GrammarRule(
        name: "type_arguments",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_argument_list", isToken: false),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 655),
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
        lineNumber: 656),
    GrammarRule(
        name: "type_parameters",
        body: Sequence(elements: [
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("type_parameter_list", isToken: false),
          RuleReference("GREATER_THAN", isToken: true)
        ]),
        lineNumber: 658),
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
        lineNumber: 659),
    GrammarRule(
        name: "type_parameter",
        body: Sequence(elements: [
          Optional(
              element: Alternation(choices: [
            Literal("in"),
            Literal("out"),
            Sequence(elements: [Literal("in"), Literal("out")]),
            Literal("const")
          ])),
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
        lineNumber: 660),
    GrammarRule(
        name: "object_type",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("type_member", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 667),
    GrammarRule(
        name: "type_member",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("construct_signature", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("call_signature", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("index_signature", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("method_signature", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("property_signature", isToken: false),
            RuleReference("SEMICOLON", isToken: true)
          ])
        ]),
        lineNumber: 669),
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
        lineNumber: 675),
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
        lineNumber: 677),
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
        lineNumber: 679),
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
        lineNumber: 682),
    GrammarRule(
        name: "construct_signature",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("abstract"),
            Literal("new"),
            Optional(element: RuleReference("type_parameters", isToken: false)),
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("typed_parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true),
            RuleReference("ARROW", isToken: true),
            RuleReference("type_expression", isToken: false)
          ]),
          Sequence(elements: [
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
          ])
        ]),
        lineNumber: 684),
    GrammarRule(
        name: "tuple_type",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          Optional(
              element: RuleReference("tuple_element_list", isToken: false)),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 698),
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
        lineNumber: 699),
    GrammarRule(
        name: "tuple_element",
        body: Sequence(elements: [
          Optional(
              element: Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("COLON", isToken: true)
          ])),
          Optional(element: Literal("readonly")),
          Optional(element: RuleReference("ELLIPSIS", isToken: true)),
          RuleReference("type_expression", isToken: false),
          Optional(element: RuleReference("QUESTION", isToken: true))
        ]),
        lineNumber: 700),
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
        lineNumber: 706),
    GrammarRule(
        name: "constructor_type",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("abstract"),
            Literal("new"),
            Optional(element: RuleReference("type_parameters", isToken: false)),
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("typed_parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true),
            RuleReference("ARROW", isToken: true),
            RuleReference("type_expression", isToken: false)
          ]),
          Sequence(elements: [
            Literal("new"),
            Optional(element: RuleReference("type_parameters", isToken: false)),
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("typed_parameter_list", isToken: false)),
            RuleReference("RPAREN", isToken: true),
            RuleReference("ARROW", isToken: true),
            RuleReference("type_expression", isToken: false)
          ])
        ]),
        lineNumber: 708),
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
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 715),
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
        lineNumber: 719),
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
        lineNumber: 720),
    GrammarRule(
        name: "template_literal_type",
        body: Alternation(choices: [
          RuleReference("TEMPLATE_NO_SUB", isToken: true),
          Sequence(elements: [
            RuleReference("TEMPLATE_HEAD", isToken: true),
            Repetition(
                element: Sequence(elements: [
              RuleReference("type_expression", isToken: false),
              RuleReference("TEMPLATE_MIDDLE", isToken: true)
            ])),
            RuleReference("type_expression", isToken: false),
            RuleReference("TEMPLATE_TAIL", isToken: true)
          ])
        ]),
        lineNumber: 726),
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
        lineNumber: 733),
    GrammarRule(
        name: "typed_parameter",
        body: Sequence(elements: [
          Optional(
              element: Alternation(choices: [
            Literal("public"),
            Literal("private"),
            Literal("protected")
          ])),
          Optional(element: Literal("override")),
          Optional(element: Literal("readonly")),
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
        lineNumber: 736),
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
        lineNumber: 740),
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
        lineNumber: 746),
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
        lineNumber: 748),
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
        lineNumber: 750),
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
        lineNumber: 752),
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
        lineNumber: 754),
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
        lineNumber: 756),
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
        lineNumber: 758),
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
        lineNumber: 760),
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
        lineNumber: 762),
    GrammarRule(
        name: "export_assignment",
        body: Sequence(elements: [
          Literal("export"),
          RuleReference("EQUALS", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 767),
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
            RuleReference("lexical_declaration", isToken: false),
            RuleReference("variable_statement", isToken: false)
          ]))
        ]),
        lineNumber: 769),
    GrammarRule(
        name: "ambient_declaration",
        body: Sequence(elements: [
          Literal("declare"),
          RuleReference("ambient_declaration_body", isToken: false)
        ]),
        lineNumber: 773),
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
          RuleReference("ambient_module_declaration", isToken: false),
          Sequence(elements: [
            Literal("global"),
            RuleReference("LBRACE", isToken: true),
            Repetition(
                element: RuleReference("namespace_element", isToken: false)),
            RuleReference("RBRACE", isToken: true)
          ])
        ]),
        lineNumber: 775),
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
        lineNumber: 780),
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
        lineNumber: 784),
    GrammarRule(
        name: "ts_class_declaration",
        body: Sequence(elements: [
          Repetition(element: RuleReference("decorator", isToken: false)),
          Optional(
              element: RuleReference("ts_class_modifiers", isToken: false)),
          Literal("class"),
          RuleReference("NAME", isToken: true),
          Optional(element: RuleReference("type_parameters", isToken: false)),
          Optional(element: RuleReference("ts_class_heritage", isToken: false)),
          RuleReference("ts_class_body", isToken: false)
        ]),
        lineNumber: 800),
    GrammarRule(
        name: "ts_class_expression",
        body: Sequence(elements: [
          Repetition(element: RuleReference("decorator", isToken: false)),
          Optional(
              element: RuleReference("ts_class_modifiers", isToken: false)),
          Literal("class"),
          Optional(element: RuleReference("NAME", isToken: true)),
          Optional(element: RuleReference("type_parameters", isToken: false)),
          Optional(element: RuleReference("ts_class_heritage", isToken: false)),
          RuleReference("ts_class_body", isToken: false)
        ]),
        lineNumber: 803),
    GrammarRule(
        name: "ts_class_modifiers",
        body: Alternation(choices: [Literal("abstract"), Literal("declare")]),
        lineNumber: 806),
    GrammarRule(
        name: "ts_class_heritage",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("extends"),
            RuleReference("type_reference", isToken: false),
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
          Sequence(elements: [
            Literal("implements"),
            RuleReference("type_reference", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("type_reference", isToken: false)
            ]))
          ])
        ]),
        lineNumber: 808),
    GrammarRule(
        name: "ts_class_body",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Repetition(
              element: RuleReference("ts_class_element", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 811),
    GrammarRule(
        name: "ts_class_element",
        body: Alternation(choices: [
          RuleReference("ts_class_member", isToken: false),
          RuleReference("static_block", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 813),
    GrammarRule(
        name: "ts_class_member",
        body: Alternation(choices: [
          RuleReference("ts_constructor_declaration", isToken: false),
          RuleReference("ts_method_declaration", isToken: false),
          RuleReference("ts_property_declaration", isToken: false),
          RuleReference("ts_private_field_declaration", isToken: false),
          RuleReference("ts_private_method_declaration", isToken: false),
          RuleReference("ts_accessor_declaration", isToken: false),
          RuleReference("index_signature", isToken: false)
        ]),
        lineNumber: 817),
    GrammarRule(
        name: "static_block",
        body: Sequence(elements: [
          Literal("static"),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("statement", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 825),
    GrammarRule(
        name: "ts_private_field_declaration",
        body: Sequence(elements: [
          Optional(element: Literal("static")),
          RuleReference("PRIVATE_NAME", isToken: true),
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
        lineNumber: 827),
    GrammarRule(
        name: "ts_private_method_declaration",
        body: Alternation(choices: [
          Sequence(elements: [
            Optional(element: Literal("static")),
            RuleReference("PRIVATE_NAME", isToken: true),
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("formal_parameters", isToken: false)),
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
            Optional(element: Literal("static")),
            Literal("get"),
            RuleReference("PRIVATE_NAME", isToken: true),
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
            Optional(element: Literal("static")),
            Literal("set"),
            RuleReference("PRIVATE_NAME", isToken: true),
            RuleReference("LPAREN", isToken: true),
            RuleReference("formal_parameter", isToken: false),
            RuleReference("RPAREN", isToken: true),
            RuleReference("LBRACE", isToken: true),
            RuleReference("function_body", isToken: false),
            RuleReference("RBRACE", isToken: true)
          ]),
          Sequence(elements: [
            Optional(element: Literal("static")),
            Literal("async"),
            Optional(element: RuleReference("STAR", isToken: true)),
            RuleReference("PRIVATE_NAME", isToken: true),
            RuleReference("LPAREN", isToken: true),
            Optional(
                element: RuleReference("formal_parameters", isToken: false)),
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
        lineNumber: 831),
    GrammarRule(
        name: "ts_constructor_declaration",
        body: Sequence(elements: [
          Optional(
              element: RuleReference("accessibility_modifier", isToken: false)),
          Literal("constructor"),
          RuleReference("LPAREN", isToken: true),
          Optional(
              element: RuleReference("ts_constructor_params", isToken: false)),
          RuleReference("RPAREN", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("COLON", isToken: true),
            Literal("void")
          ])),
          RuleReference("LBRACE", isToken: true),
          RuleReference("function_body", isToken: false),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 846),
    GrammarRule(
        name: "ts_constructor_params",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("ts_constructor_param", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("ts_constructor_param", isToken: false)
            ])),
            Optional(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("rest_typed_parameter", isToken: false)
            ]))
          ]),
          RuleReference("rest_typed_parameter", isToken: false)
        ]),
        lineNumber: 850),
    GrammarRule(
        name: "ts_constructor_param",
        body: Sequence(elements: [
          Optional(
              element: RuleReference("accessibility_modifier", isToken: false)),
          Optional(element: Literal("override")),
          Optional(element: Literal("readonly")),
          RuleReference("typed_parameter", isToken: false)
        ]),
        lineNumber: 853),
    GrammarRule(
        name: "accessibility_modifier",
        body: Alternation(choices: [
          Literal("public"),
          Literal("private"),
          Literal("protected")
        ]),
        lineNumber: 855),
    GrammarRule(
        name: "ts_method_declaration",
        body: Sequence(elements: [
          Repetition(element: RuleReference("decorator", isToken: false)),
          Repetition(
              element: RuleReference("ts_member_modifier", isToken: false)),
          RuleReference("ts_method_body", isToken: false)
        ]),
        lineNumber: 857),
    GrammarRule(
        name: "ts_member_modifier",
        body: Alternation(choices: [
          Literal("public"),
          Literal("private"),
          Literal("protected"),
          Literal("static"),
          Literal("abstract"),
          Literal("readonly"),
          Literal("override"),
          Literal("declare")
        ]),
        lineNumber: 859),
    GrammarRule(
        name: "ts_method_body",
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
          ]),
          Sequence(elements: [
            Literal("async"),
            Optional(element: RuleReference("STAR", isToken: true)),
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
        lineNumber: 862),
    GrammarRule(
        name: "ts_property_declaration",
        body: Sequence(elements: [
          Repetition(element: RuleReference("decorator", isToken: false)),
          Repetition(
              element: RuleReference("ts_member_modifier", isToken: false)),
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
        lineNumber: 876),
    GrammarRule(
        name: "ts_accessor_declaration",
        body: Sequence(elements: [
          Repetition(element: RuleReference("decorator", isToken: false)),
          Repetition(
              element: RuleReference("ts_member_modifier", isToken: false)),
          Literal("accessor"),
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
        lineNumber: 879),
    GrammarRule(
        name: "ts_as_expression",
        body: Sequence(elements: [
          RuleReference("conditional_expression", isToken: false),
          Literal("as"),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 886),
    GrammarRule(
        name: "ts_satisfies_expression",
        body: Sequence(elements: [
          RuleReference("conditional_expression", isToken: false),
          Literal("satisfies"),
          RuleReference("type_expression", isToken: false)
        ]),
        lineNumber: 888),
    GrammarRule(
        name: "type_predicate",
        body: Alternation(choices: [
          Sequence(elements: [
            Optional(element: Literal("asserts")),
            RuleReference("NAME", isToken: true),
            Literal("is"),
            RuleReference("type_expression", isToken: false)
          ]),
          Sequence(elements: [
            Literal("asserts"),
            RuleReference("NAME", isToken: true)
          ])
        ]),
        lineNumber: 893),
  ],
);
