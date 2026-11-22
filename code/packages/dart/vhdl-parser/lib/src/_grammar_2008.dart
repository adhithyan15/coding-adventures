// AUTO-GENERATED FILE - DO NOT EDIT
// Source: vhdl2008.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 0,
  rules: [
    GrammarRule(
      name: "design_file",
      body: Repetition(element: RuleReference("design_unit", isToken: false)),
      lineNumber: 67,
    ),
    GrammarRule(
      name: "design_unit",
      body: Sequence(
        elements: [
          Repetition(element: RuleReference("context_item", isToken: false)),
          RuleReference("library_unit", isToken: false),
        ],
      ),
      lineNumber: 69,
    ),
    GrammarRule(
      name: "context_item",
      body: Alternation(
        choices: [
          RuleReference("library_clause", isToken: false),
          RuleReference("use_clause", isToken: false),
        ],
      ),
      lineNumber: 71,
    ),
    GrammarRule(
      name: "library_clause",
      body: Sequence(
        elements: [
          Literal("library"),
          RuleReference("name_list", isToken: false),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 74,
    ),
    GrammarRule(
      name: "use_clause",
      body: Sequence(
        elements: [
          Literal("use"),
          RuleReference("selected_name", isToken: false),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 77,
    ),
    GrammarRule(
      name: "selected_name",
      body: Sequence(
        elements: [
          RuleReference("NAME", isToken: true),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("DOT", isToken: true),
                Group(
                  element: Alternation(
                    choices: [
                      RuleReference("NAME", isToken: true),
                      Literal("all"),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 80,
    ),
    GrammarRule(
      name: "name_list",
      body: Sequence(
        elements: [
          RuleReference("NAME", isToken: true),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("NAME", isToken: true),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 82,
    ),
    GrammarRule(
      name: "library_unit",
      body: Alternation(
        choices: [
          RuleReference("entity_declaration", isToken: false),
          RuleReference("architecture_body", isToken: false),
          RuleReference("package_declaration", isToken: false),
          RuleReference("package_body", isToken: false),
        ],
      ),
      lineNumber: 84,
    ),
    GrammarRule(
      name: "entity_declaration",
      body: Sequence(
        elements: [
          Literal("entity"),
          RuleReference("NAME", isToken: true),
          Literal("is"),
          Optional(element: RuleReference("generic_clause", isToken: false)),
          Optional(element: RuleReference("port_clause", isToken: false)),
          Literal("end"),
          Optional(element: Literal("entity")),
          Optional(element: RuleReference("NAME", isToken: true)),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 115,
    ),
    GrammarRule(
      name: "generic_clause",
      body: Sequence(
        elements: [
          Literal("generic"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("interface_list", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 120,
    ),
    GrammarRule(
      name: "port_clause",
      body: Sequence(
        elements: [
          Literal("port"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("interface_list", isToken: false),
          RuleReference("RPAREN", isToken: true),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 121,
    ),
    GrammarRule(
      name: "interface_list",
      body: Sequence(
        elements: [
          RuleReference("interface_element", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("SEMICOLON", isToken: true),
                RuleReference("interface_element", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 126,
    ),
    GrammarRule(
      name: "interface_element",
      body: Sequence(
        elements: [
          RuleReference("name_list", isToken: false),
          RuleReference("COLON", isToken: true),
          Optional(element: RuleReference("mode", isToken: false)),
          RuleReference("subtype_indication", isToken: false),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("VAR_ASSIGN", isToken: true),
                RuleReference("expression", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 127,
    ),
    GrammarRule(
      name: "mode",
      body: Alternation(
        choices: [
          Literal("in"),
          Literal("out"),
          Literal("inout"),
          Literal("buffer"),
        ],
      ),
      lineNumber: 135,
    ),
    GrammarRule(
      name: "architecture_body",
      body: Sequence(
        elements: [
          Literal("architecture"),
          RuleReference("NAME", isToken: true),
          Literal("of"),
          RuleReference("NAME", isToken: true),
          Literal("is"),
          Repetition(
            element: RuleReference("block_declarative_item", isToken: false),
          ),
          Literal("begin"),
          Repetition(
            element: RuleReference("concurrent_statement", isToken: false),
          ),
          Literal("end"),
          Optional(element: Literal("architecture")),
          Optional(element: RuleReference("NAME", isToken: true)),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 157,
    ),
    GrammarRule(
      name: "block_declarative_item",
      body: Alternation(
        choices: [
          RuleReference("signal_declaration", isToken: false),
          RuleReference("constant_declaration", isToken: false),
          RuleReference("type_declaration", isToken: false),
          RuleReference("subtype_declaration", isToken: false),
          RuleReference("component_declaration", isToken: false),
          RuleReference("function_declaration", isToken: false),
          RuleReference("function_body", isToken: false),
          RuleReference("procedure_declaration", isToken: false),
          RuleReference("procedure_body", isToken: false),
        ],
      ),
      lineNumber: 163,
    ),
    GrammarRule(
      name: "signal_declaration",
      body: Sequence(
        elements: [
          Literal("signal"),
          RuleReference("name_list", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("subtype_indication", isToken: false),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("VAR_ASSIGN", isToken: true),
                RuleReference("expression", isToken: false),
              ],
            ),
          ),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 192,
    ),
    GrammarRule(
      name: "constant_declaration",
      body: Sequence(
        elements: [
          Literal("constant"),
          RuleReference("name_list", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("subtype_indication", isToken: false),
          RuleReference("VAR_ASSIGN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 194,
    ),
    GrammarRule(
      name: "variable_declaration",
      body: Sequence(
        elements: [
          Literal("variable"),
          RuleReference("name_list", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("subtype_indication", isToken: false),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("VAR_ASSIGN", isToken: true),
                RuleReference("expression", isToken: false),
              ],
            ),
          ),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 196,
    ),
    GrammarRule(
      name: "type_declaration",
      body: Sequence(
        elements: [
          Literal("type"),
          RuleReference("NAME", isToken: true),
          Literal("is"),
          RuleReference("type_definition", isToken: false),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 221,
    ),
    GrammarRule(
      name: "subtype_declaration",
      body: Sequence(
        elements: [
          Literal("subtype"),
          RuleReference("NAME", isToken: true),
          Literal("is"),
          RuleReference("subtype_indication", isToken: false),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 222,
    ),
    GrammarRule(
      name: "type_definition",
      body: Alternation(
        choices: [
          RuleReference("enumeration_type", isToken: false),
          RuleReference("array_type", isToken: false),
          RuleReference("record_type", isToken: false),
        ],
      ),
      lineNumber: 224,
    ),
    GrammarRule(
      name: "enumeration_type",
      body: Sequence(
        elements: [
          RuleReference("LPAREN", isToken: true),
          Group(
            element: Alternation(
              choices: [
                RuleReference("NAME", isToken: true),
                RuleReference("CHAR_LITERAL", isToken: true),
              ],
            ),
          ),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                Group(
                  element: Alternation(
                    choices: [
                      RuleReference("NAME", isToken: true),
                      RuleReference("CHAR_LITERAL", isToken: true),
                    ],
                  ),
                ),
              ],
            ),
          ),
          RuleReference("RPAREN", isToken: true),
        ],
      ),
      lineNumber: 230,
    ),
    GrammarRule(
      name: "array_type",
      body: Sequence(
        elements: [
          Literal("array"),
          RuleReference("LPAREN", isToken: true),
          RuleReference("index_constraint", isToken: false),
          RuleReference("RPAREN", isToken: true),
          Literal("of"),
          RuleReference("subtype_indication", isToken: false),
        ],
      ),
      lineNumber: 235,
    ),
    GrammarRule(
      name: "index_constraint",
      body: Sequence(
        elements: [
          RuleReference("discrete_range", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("discrete_range", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 237,
    ),
    GrammarRule(
      name: "discrete_range",
      body: Alternation(
        choices: [
          RuleReference("subtype_indication", isToken: false),
          Sequence(
            elements: [
              RuleReference("expression", isToken: false),
              Group(
                element: Alternation(
                  choices: [Literal("to"), Literal("downto")],
                ),
              ),
              RuleReference("expression", isToken: false),
            ],
          ),
        ],
      ),
      lineNumber: 238,
    ),
    GrammarRule(
      name: "record_type",
      body: Sequence(
        elements: [
          Literal("record"),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("NAME", isToken: true),
                RuleReference("COLON", isToken: true),
                RuleReference("subtype_indication", isToken: false),
                RuleReference("SEMICOLON", isToken: true),
              ],
            ),
          ),
          Literal("end"),
          Literal("record"),
          Optional(element: RuleReference("NAME", isToken: true)),
        ],
      ),
      lineNumber: 242,
    ),
    GrammarRule(
      name: "subtype_indication",
      body: Sequence(
        elements: [
          RuleReference("selected_name", isToken: false),
          Optional(element: RuleReference("constraint", isToken: false)),
        ],
      ),
      lineNumber: 250,
    ),
    GrammarRule(
      name: "constraint",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              RuleReference("LPAREN", isToken: true),
              RuleReference("expression", isToken: false),
              Group(
                element: Alternation(
                  choices: [Literal("to"), Literal("downto")],
                ),
              ),
              RuleReference("expression", isToken: false),
              RuleReference("RPAREN", isToken: true),
            ],
          ),
          Sequence(
            elements: [
              Literal("range"),
              RuleReference("expression", isToken: false),
              Group(
                element: Alternation(
                  choices: [Literal("to"), Literal("downto")],
                ),
              ),
              RuleReference("expression", isToken: false),
            ],
          ),
        ],
      ),
      lineNumber: 252,
    ),
    GrammarRule(
      name: "concurrent_statement",
      body: Alternation(
        choices: [
          RuleReference("process_statement", isToken: false),
          RuleReference("signal_assignment_concurrent", isToken: false),
          RuleReference("component_instantiation", isToken: false),
          RuleReference("generate_statement", isToken: false),
        ],
      ),
      lineNumber: 267,
    ),
    GrammarRule(
      name: "signal_assignment_concurrent",
      body: Sequence(
        elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("waveform", isToken: false),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 275,
    ),
    GrammarRule(
      name: "waveform",
      body: Sequence(
        elements: [
          RuleReference("waveform_element", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("waveform_element", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 277,
    ),
    GrammarRule(
      name: "waveform_element",
      body: RuleReference("expression", isToken: false),
      lineNumber: 278,
    ),
    GrammarRule(
      name: "process_statement",
      body: Sequence(
        elements: [
          Optional(
            element: Sequence(
              elements: [
                RuleReference("NAME", isToken: true),
                RuleReference("COLON", isToken: true),
              ],
            ),
          ),
          Literal("process"),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("LPAREN", isToken: true),
                RuleReference("sensitivity_list", isToken: false),
                RuleReference("RPAREN", isToken: true),
              ],
            ),
          ),
          Optional(element: Literal("is")),
          Repetition(
            element: RuleReference("process_declarative_item", isToken: false),
          ),
          Literal("begin"),
          Repetition(
            element: RuleReference("sequential_statement", isToken: false),
          ),
          Literal("end"),
          Literal("process"),
          Optional(element: RuleReference("NAME", isToken: true)),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 310,
    ),
    GrammarRule(
      name: "sensitivity_list",
      body: Sequence(
        elements: [
          RuleReference("NAME", isToken: true),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("NAME", isToken: true),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 318,
    ),
    GrammarRule(
      name: "process_declarative_item",
      body: Alternation(
        choices: [
          RuleReference("variable_declaration", isToken: false),
          RuleReference("constant_declaration", isToken: false),
          RuleReference("type_declaration", isToken: false),
          RuleReference("subtype_declaration", isToken: false),
        ],
      ),
      lineNumber: 320,
    ),
    GrammarRule(
      name: "sequential_statement",
      body: Alternation(
        choices: [
          RuleReference("signal_assignment_seq", isToken: false),
          RuleReference("variable_assignment", isToken: false),
          RuleReference("if_statement", isToken: false),
          RuleReference("case_statement", isToken: false),
          RuleReference("loop_statement", isToken: false),
          RuleReference("return_statement", isToken: false),
          RuleReference("null_statement", isToken: false),
        ],
      ),
      lineNumber: 332,
    ),
    GrammarRule(
      name: "signal_assignment_seq",
      body: Sequence(
        elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("waveform", isToken: false),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 345,
    ),
    GrammarRule(
      name: "variable_assignment",
      body: Sequence(
        elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("VAR_ASSIGN", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 349,
    ),
    GrammarRule(
      name: "if_statement",
      body: Sequence(
        elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          Literal("then"),
          Repetition(
            element: RuleReference("sequential_statement", isToken: false),
          ),
          Repetition(
            element: Sequence(
              elements: [
                Literal("elsif"),
                RuleReference("expression", isToken: false),
                Literal("then"),
                Repetition(
                  element: RuleReference(
                    "sequential_statement",
                    isToken: false,
                  ),
                ),
              ],
            ),
          ),
          Optional(
            element: Sequence(
              elements: [
                Literal("else"),
                Repetition(
                  element: RuleReference(
                    "sequential_statement",
                    isToken: false,
                  ),
                ),
              ],
            ),
          ),
          Literal("end"),
          Literal("if"),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 359,
    ),
    GrammarRule(
      name: "case_statement",
      body: Sequence(
        elements: [
          Literal("case"),
          RuleReference("expression", isToken: false),
          Literal("is"),
          Repetition(
            element: Sequence(
              elements: [
                Literal("when"),
                RuleReference("choices", isToken: false),
                RuleReference("ARROW", isToken: true),
                Repetition(
                  element: RuleReference(
                    "sequential_statement",
                    isToken: false,
                  ),
                ),
              ],
            ),
          ),
          Literal("end"),
          Literal("case"),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 375,
    ),
    GrammarRule(
      name: "choices",
      body: Sequence(
        elements: [
          RuleReference("choice", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("PIPE", isToken: true),
                RuleReference("choice", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 379,
    ),
    GrammarRule(
      name: "choice",
      body: Alternation(
        choices: [
          RuleReference("expression", isToken: false),
          RuleReference("discrete_range", isToken: false),
          Literal("others"),
        ],
      ),
      lineNumber: 380,
    ),
    GrammarRule(
      name: "loop_statement",
      body: Sequence(
        elements: [
          Optional(
            element: Sequence(
              elements: [
                RuleReference("NAME", isToken: true),
                RuleReference("COLON", isToken: true),
              ],
            ),
          ),
          Optional(
            element: Alternation(
              choices: [
                Sequence(
                  elements: [
                    Literal("for"),
                    RuleReference("NAME", isToken: true),
                    Literal("in"),
                    RuleReference("discrete_range", isToken: false),
                  ],
                ),
                Sequence(
                  elements: [
                    Literal("while"),
                    RuleReference("expression", isToken: false),
                  ],
                ),
              ],
            ),
          ),
          Literal("loop"),
          Repetition(
            element: RuleReference("sequential_statement", isToken: false),
          ),
          Literal("end"),
          Literal("loop"),
          Optional(element: RuleReference("NAME", isToken: true)),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 394,
    ),
    GrammarRule(
      name: "return_statement",
      body: Sequence(
        elements: [
          Literal("return"),
          Optional(element: RuleReference("expression", isToken: false)),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 401,
    ),
    GrammarRule(
      name: "null_statement",
      body: Sequence(
        elements: [Literal("null"), RuleReference("SEMICOLON", isToken: true)],
      ),
      lineNumber: 402,
    ),
    GrammarRule(
      name: "component_declaration",
      body: Sequence(
        elements: [
          Literal("component"),
          RuleReference("NAME", isToken: true),
          Optional(element: Literal("is")),
          Optional(element: RuleReference("generic_clause", isToken: false)),
          Optional(element: RuleReference("port_clause", isToken: false)),
          Literal("end"),
          Literal("component"),
          Optional(element: RuleReference("NAME", isToken: true)),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 428,
    ),
    GrammarRule(
      name: "component_instantiation",
      body: Sequence(
        elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          Group(
            element: Alternation(
              choices: [
                RuleReference("NAME", isToken: true),
                Sequence(
                  elements: [
                    Literal("entity"),
                    RuleReference("selected_name", isToken: false),
                    Optional(
                      element: Sequence(
                        elements: [
                          RuleReference("LPAREN", isToken: true),
                          RuleReference("NAME", isToken: true),
                          RuleReference("RPAREN", isToken: true),
                        ],
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
          Optional(
            element: Sequence(
              elements: [
                Literal("generic"),
                Literal("map"),
                RuleReference("LPAREN", isToken: true),
                RuleReference("association_list", isToken: false),
                RuleReference("RPAREN", isToken: true),
              ],
            ),
          ),
          Optional(
            element: Sequence(
              elements: [
                Literal("port"),
                Literal("map"),
                RuleReference("LPAREN", isToken: true),
                RuleReference("association_list", isToken: false),
                RuleReference("RPAREN", isToken: true),
              ],
            ),
          ),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 433,
    ),
    GrammarRule(
      name: "association_list",
      body: Sequence(
        elements: [
          RuleReference("association_element", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("association_element", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 440,
    ),
    GrammarRule(
      name: "association_element",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              Optional(
                element: Sequence(
                  elements: [
                    RuleReference("NAME", isToken: true),
                    RuleReference("ARROW", isToken: true),
                  ],
                ),
              ),
              RuleReference("expression", isToken: false),
            ],
          ),
          Sequence(
            elements: [
              Optional(
                element: Sequence(
                  elements: [
                    RuleReference("NAME", isToken: true),
                    RuleReference("ARROW", isToken: true),
                  ],
                ),
              ),
              Literal("open"),
            ],
          ),
        ],
      ),
      lineNumber: 441,
    ),
    GrammarRule(
      name: "generate_statement",
      body: Sequence(
        elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          Group(
            element: Alternation(
              choices: [
                RuleReference("for_generate", isToken: false),
                RuleReference("if_generate", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 464,
    ),
    GrammarRule(
      name: "for_generate",
      body: Sequence(
        elements: [
          Literal("for"),
          RuleReference("NAME", isToken: true),
          Literal("in"),
          RuleReference("discrete_range", isToken: false),
          Literal("generate"),
          Repetition(
            element: RuleReference("concurrent_statement", isToken: false),
          ),
          Literal("end"),
          Literal("generate"),
          Optional(element: RuleReference("NAME", isToken: true)),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 466,
    ),
    GrammarRule(
      name: "if_generate",
      body: Sequence(
        elements: [
          Literal("if"),
          RuleReference("expression", isToken: false),
          Literal("generate"),
          Repetition(
            element: RuleReference("concurrent_statement", isToken: false),
          ),
          Literal("end"),
          Literal("generate"),
          Optional(element: RuleReference("NAME", isToken: true)),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 470,
    ),
    GrammarRule(
      name: "package_declaration",
      body: Sequence(
        elements: [
          Literal("package"),
          RuleReference("NAME", isToken: true),
          Literal("is"),
          Repetition(
            element: RuleReference("package_declarative_item", isToken: false),
          ),
          Literal("end"),
          Optional(element: Literal("package")),
          Optional(element: RuleReference("NAME", isToken: true)),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 491,
    ),
    GrammarRule(
      name: "package_body",
      body: Sequence(
        elements: [
          Literal("package"),
          Literal("body"),
          RuleReference("NAME", isToken: true),
          Literal("is"),
          Repetition(
            element: RuleReference(
              "package_body_declarative_item",
              isToken: false,
            ),
          ),
          Literal("end"),
          Optional(
            element: Sequence(elements: [Literal("package"), Literal("body")]),
          ),
          Optional(element: RuleReference("NAME", isToken: true)),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 495,
    ),
    GrammarRule(
      name: "package_declarative_item",
      body: Alternation(
        choices: [
          RuleReference("type_declaration", isToken: false),
          RuleReference("subtype_declaration", isToken: false),
          RuleReference("constant_declaration", isToken: false),
          RuleReference("signal_declaration", isToken: false),
          RuleReference("component_declaration", isToken: false),
          RuleReference("function_declaration", isToken: false),
          RuleReference("procedure_declaration", isToken: false),
        ],
      ),
      lineNumber: 499,
    ),
    GrammarRule(
      name: "package_body_declarative_item",
      body: Alternation(
        choices: [
          RuleReference("type_declaration", isToken: false),
          RuleReference("subtype_declaration", isToken: false),
          RuleReference("constant_declaration", isToken: false),
          RuleReference("function_body", isToken: false),
          RuleReference("procedure_body", isToken: false),
        ],
      ),
      lineNumber: 507,
    ),
    GrammarRule(
      name: "function_declaration",
      body: Sequence(
        elements: [
          Optional(
            element: Alternation(choices: [Literal("pure"), Literal("impure")]),
          ),
          Literal("function"),
          RuleReference("NAME", isToken: true),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("LPAREN", isToken: true),
                RuleReference("interface_list", isToken: false),
                RuleReference("RPAREN", isToken: true),
              ],
            ),
          ),
          Literal("return"),
          RuleReference("subtype_indication", isToken: false),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 523,
    ),
    GrammarRule(
      name: "function_body",
      body: Sequence(
        elements: [
          Optional(
            element: Alternation(choices: [Literal("pure"), Literal("impure")]),
          ),
          Literal("function"),
          RuleReference("NAME", isToken: true),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("LPAREN", isToken: true),
                RuleReference("interface_list", isToken: false),
                RuleReference("RPAREN", isToken: true),
              ],
            ),
          ),
          Literal("return"),
          RuleReference("subtype_indication", isToken: false),
          Literal("is"),
          Repetition(
            element: RuleReference("process_declarative_item", isToken: false),
          ),
          Literal("begin"),
          Repetition(
            element: RuleReference("sequential_statement", isToken: false),
          ),
          Literal("end"),
          Optional(element: Literal("function")),
          Optional(element: RuleReference("NAME", isToken: true)),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 528,
    ),
    GrammarRule(
      name: "procedure_declaration",
      body: Sequence(
        elements: [
          Literal("procedure"),
          RuleReference("NAME", isToken: true),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("LPAREN", isToken: true),
                RuleReference("interface_list", isToken: false),
                RuleReference("RPAREN", isToken: true),
              ],
            ),
          ),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 537,
    ),
    GrammarRule(
      name: "procedure_body",
      body: Sequence(
        elements: [
          Literal("procedure"),
          RuleReference("NAME", isToken: true),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("LPAREN", isToken: true),
                RuleReference("interface_list", isToken: false),
                RuleReference("RPAREN", isToken: true),
              ],
            ),
          ),
          Literal("is"),
          Repetition(
            element: RuleReference("process_declarative_item", isToken: false),
          ),
          Literal("begin"),
          Repetition(
            element: RuleReference("sequential_statement", isToken: false),
          ),
          Literal("end"),
          Optional(element: Literal("procedure")),
          Optional(element: RuleReference("NAME", isToken: true)),
          RuleReference("SEMICOLON", isToken: true),
        ],
      ),
      lineNumber: 540,
    ),
    GrammarRule(
      name: "expression",
      body: RuleReference("logical_expr", isToken: false),
      lineNumber: 577,
    ),
    GrammarRule(
      name: "logical_expr",
      body: Sequence(
        elements: [
          RuleReference("relation", isToken: false),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("logical_op", isToken: false),
                RuleReference("relation", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 584,
    ),
    GrammarRule(
      name: "logical_op",
      body: Alternation(
        choices: [
          Literal("and"),
          Literal("or"),
          Literal("xor"),
          Literal("nand"),
          Literal("nor"),
          Literal("xnor"),
        ],
      ),
      lineNumber: 585,
    ),
    GrammarRule(
      name: "relation",
      body: Sequence(
        elements: [
          RuleReference("shift_expr", isToken: false),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("relational_op", isToken: false),
                RuleReference("shift_expr", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 589,
    ),
    GrammarRule(
      name: "relational_op",
      body: Alternation(
        choices: [
          RuleReference("EQUALS", isToken: true),
          RuleReference("NOT_EQUALS", isToken: true),
          RuleReference("LESS_THAN", isToken: true),
          RuleReference("LESS_EQUALS", isToken: true),
          RuleReference("GREATER_THAN", isToken: true),
          RuleReference("GREATER_EQUALS", isToken: true),
        ],
      ),
      lineNumber: 590,
    ),
    GrammarRule(
      name: "shift_expr",
      body: Sequence(
        elements: [
          RuleReference("adding_expr", isToken: false),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("shift_op", isToken: false),
                RuleReference("adding_expr", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 595,
    ),
    GrammarRule(
      name: "shift_op",
      body: Alternation(
        choices: [
          Literal("sll"),
          Literal("srl"),
          Literal("sla"),
          Literal("sra"),
          Literal("rol"),
          Literal("ror"),
        ],
      ),
      lineNumber: 596,
    ),
    GrammarRule(
      name: "adding_expr",
      body: Sequence(
        elements: [
          RuleReference("multiplying_expr", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("adding_op", isToken: false),
                RuleReference("multiplying_expr", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 600,
    ),
    GrammarRule(
      name: "adding_op",
      body: Alternation(
        choices: [
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
        ],
      ),
      lineNumber: 601,
    ),
    GrammarRule(
      name: "multiplying_expr",
      body: Sequence(
        elements: [
          RuleReference("unary_expr", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("multiplying_op", isToken: false),
                RuleReference("unary_expr", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 604,
    ),
    GrammarRule(
      name: "multiplying_op",
      body: Alternation(
        choices: [
          RuleReference("STAR", isToken: true),
          RuleReference("SLASH", isToken: true),
          Literal("mod"),
          Literal("rem"),
        ],
      ),
      lineNumber: 605,
    ),
    GrammarRule(
      name: "unary_expr",
      body: Alternation(
        choices: [
          Sequence(
            elements: [
              Literal("abs"),
              RuleReference("unary_expr", isToken: false),
            ],
          ),
          Sequence(
            elements: [
              Literal("not"),
              RuleReference("unary_expr", isToken: false),
            ],
          ),
          Sequence(
            elements: [
              Group(
                element: Alternation(
                  choices: [
                    RuleReference("PLUS", isToken: true),
                    RuleReference("MINUS", isToken: true),
                  ],
                ),
              ),
              RuleReference("unary_expr", isToken: false),
            ],
          ),
          RuleReference("power_expr", isToken: false),
        ],
      ),
      lineNumber: 608,
    ),
    GrammarRule(
      name: "power_expr",
      body: Sequence(
        elements: [
          RuleReference("primary", isToken: false),
          Optional(
            element: Sequence(
              elements: [
                RuleReference("POWER", isToken: true),
                RuleReference("primary", isToken: false),
              ],
            ),
          ),
        ],
      ),
      lineNumber: 614,
    ),
    GrammarRule(
      name: "primary",
      body: Alternation(
        choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("REAL_NUMBER", isToken: true),
          RuleReference("BASED_LITERAL", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("CHAR_LITERAL", isToken: true),
          RuleReference("BIT_STRING", isToken: true),
          Sequence(
            elements: [
              RuleReference("NAME", isToken: true),
              Optional(
                element: Sequence(
                  elements: [
                    RuleReference("TICK", isToken: true),
                    RuleReference("NAME", isToken: true),
                  ],
                ),
              ),
            ],
          ),
          Sequence(
            elements: [
              RuleReference("NAME", isToken: true),
              RuleReference("LPAREN", isToken: true),
              Optional(
                element: Sequence(
                  elements: [
                    RuleReference("expression", isToken: false),
                    Repetition(
                      element: Sequence(
                        elements: [
                          RuleReference("COMMA", isToken: true),
                          RuleReference("expression", isToken: false),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
              RuleReference("RPAREN", isToken: true),
            ],
          ),
          Sequence(
            elements: [
              RuleReference("LPAREN", isToken: true),
              RuleReference("expression", isToken: false),
              RuleReference("RPAREN", isToken: true),
            ],
          ),
          RuleReference("aggregate", isToken: false),
          Literal("null"),
        ],
      ),
      lineNumber: 622,
    ),
    GrammarRule(
      name: "aggregate",
      body: Sequence(
        elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("element_association", isToken: false),
          Repetition(
            element: Sequence(
              elements: [
                RuleReference("COMMA", isToken: true),
                RuleReference("element_association", isToken: false),
              ],
            ),
          ),
          RuleReference("RPAREN", isToken: true),
        ],
      ),
      lineNumber: 638,
    ),
    GrammarRule(
      name: "element_association",
      body: Sequence(
        elements: [
          Optional(
            element: Sequence(
              elements: [
                RuleReference("choices", isToken: false),
                RuleReference("ARROW", isToken: true),
              ],
            ),
          ),
          RuleReference("expression", isToken: false),
        ],
      ),
      lineNumber: 639,
    ),
  ],
);
