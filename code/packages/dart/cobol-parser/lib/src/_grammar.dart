// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/cobol/cobol.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "program",
        body: Sequence(elements: [
          RuleReference("identification_division", isToken: false),
          Optional(
              element: RuleReference("environment_division", isToken: false)),
          Optional(element: RuleReference("data_division", isToken: false)),
          RuleReference("procedure_division", isToken: false)
        ]),
        lineNumber: 33),
    GrammarRule(
        name: "identification_division",
        body: Sequence(elements: [
          Literal("IDENTIFICATION"),
          Literal("DIVISION"),
          RuleReference("DOT", isToken: true),
          RuleReference("program_id", isToken: false),
          Repetition(element: RuleReference("id_paragraph", isToken: false))
        ]),
        lineNumber: 44),
    GrammarRule(
        name: "program_id",
        body: Sequence(elements: [
          Literal("PROGRAM-ID"),
          RuleReference("DOT", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("DOT", isToken: true)
        ]),
        lineNumber: 49),
    GrammarRule(
        name: "id_paragraph",
        body: Sequence(elements: [
          RuleReference("id_keyword", isToken: false),
          RuleReference("DOT", isToken: true),
          Repetition(element: RuleReference("comment_word", isToken: false)),
          RuleReference("DOT", isToken: true)
        ]),
        lineNumber: 51),
    GrammarRule(
        name: "id_keyword",
        body: Alternation(choices: [
          Literal("AUTHOR"),
          Literal("INSTALLATION"),
          Literal("DATE-WRITTEN"),
          Literal("DATE-COMPILED"),
          Literal("SECURITY"),
          Literal("REMARKS")
        ]),
        lineNumber: 52),
    GrammarRule(
        name: "comment_word",
        body: Alternation(choices: [
          RuleReference("NAME", isToken: true),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true)
        ]),
        lineNumber: 55),
    GrammarRule(
        name: "environment_division",
        body: Sequence(elements: [
          Literal("ENVIRONMENT"),
          Literal("DIVISION"),
          RuleReference("DOT", isToken: true),
          Repetition(element: RuleReference("env_section", isToken: false))
        ]),
        lineNumber: 64),
    GrammarRule(
        name: "env_section",
        body: Alternation(choices: [
          RuleReference("configuration_section", isToken: false),
          RuleReference("input_output_section", isToken: false)
        ]),
        lineNumber: 65),
    GrammarRule(
        name: "configuration_section",
        body: Sequence(elements: [
          Literal("CONFIGURATION"),
          Literal("SECTION"),
          RuleReference("DOT", isToken: true),
          Repetition(element: RuleReference("config_paragraph", isToken: false))
        ]),
        lineNumber: 67),
    GrammarRule(
        name: "config_paragraph",
        body: Sequence(elements: [
          Group(
              element: Alternation(choices: [
            Literal("SOURCE-COMPUTER"),
            Literal("OBJECT-COMPUTER")
          ])),
          RuleReference("DOT", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("DOT", isToken: true)
        ]),
        lineNumber: 69),
    GrammarRule(
        name: "input_output_section",
        body: Sequence(elements: [
          Literal("INPUT-OUTPUT"),
          Literal("SECTION"),
          RuleReference("DOT", isToken: true),
          Literal("FILE-CONTROL"),
          RuleReference("DOT", isToken: true),
          Repetition(element: RuleReference("select_sentence", isToken: false))
        ]),
        lineNumber: 71),
    GrammarRule(
        name: "select_sentence",
        body: Sequence(elements: [
          Literal("SELECT"),
          RuleReference("NAME", isToken: true),
          Literal("ASSIGN"),
          Literal("TO"),
          RuleReference("NAME", isToken: true),
          RuleReference("DOT", isToken: true)
        ]),
        lineNumber: 73),
    GrammarRule(
        name: "data_division",
        body: Sequence(elements: [
          Literal("DATA"),
          Literal("DIVISION"),
          RuleReference("DOT", isToken: true),
          Repetition(element: RuleReference("data_section", isToken: false))
        ]),
        lineNumber: 81),
    GrammarRule(
        name: "data_section",
        body: Alternation(choices: [
          RuleReference("working_storage_section", isToken: false),
          RuleReference("file_section", isToken: false)
        ]),
        lineNumber: 82),
    GrammarRule(
        name: "working_storage_section",
        body: Sequence(elements: [
          Literal("WORKING-STORAGE"),
          Literal("SECTION"),
          RuleReference("DOT", isToken: true),
          Repetition(element: RuleReference("data_entry", isToken: false))
        ]),
        lineNumber: 84),
    GrammarRule(
        name: "file_section",
        body: Sequence(elements: [
          Literal("FILE"),
          Literal("SECTION"),
          RuleReference("DOT", isToken: true),
          Repetition(
              element: RuleReference("file_or_data_entry", isToken: false))
        ]),
        lineNumber: 85),
    GrammarRule(
        name: "file_or_data_entry",
        body: Alternation(choices: [
          RuleReference("fd_entry", isToken: false),
          RuleReference("data_entry", isToken: false)
        ]),
        lineNumber: 86),
    GrammarRule(
        name: "fd_entry",
        body: Sequence(elements: [
          Literal("FD"),
          RuleReference("NAME", isToken: true),
          Repetition(element: RuleReference("comment_word", isToken: false)),
          RuleReference("DOT", isToken: true)
        ]),
        lineNumber: 87),
    GrammarRule(
        name: "data_entry",
        body: Sequence(elements: [
          RuleReference("NUMBER", isToken: true),
          Group(
              element: Alternation(choices: [
            RuleReference("NAME", isToken: true),
            Literal("FILLER")
          ])),
          Repetition(element: RuleReference("data_clause", isToken: false)),
          RuleReference("DOT", isToken: true)
        ]),
        lineNumber: 89),
    GrammarRule(
        name: "data_clause",
        body: Alternation(choices: [
          RuleReference("picture_clause", isToken: false),
          RuleReference("value_clause", isToken: false)
        ]),
        lineNumber: 90),
    GrammarRule(
        name: "picture_clause",
        body: Sequence(elements: [
          Group(
              element:
                  Alternation(choices: [Literal("PICTURE"), Literal("PIC")])),
          RuleReference("PIC_STRING", isToken: true)
        ]),
        lineNumber: 92),
    GrammarRule(
        name: "value_clause",
        body: Sequence(elements: [
          Literal("VALUE"),
          Optional(element: Literal("IS")),
          RuleReference("value_item", isToken: false),
          Repetition(element: RuleReference("value_item", isToken: false))
        ]),
        lineNumber: 98),
    GrammarRule(
        name: "value_item",
        body: Sequence(elements: [
          RuleReference("literal", isToken: false),
          Optional(
              element: Sequence(elements: [
            Group(
                element: Alternation(
                    choices: [Literal("THRU"), Literal("THROUGH")])),
            RuleReference("literal", isToken: false)
          ]))
        ]),
        lineNumber: 99),
    GrammarRule(
        name: "procedure_division",
        body: Sequence(elements: [
          Literal("PROCEDURE"),
          Literal("DIVISION"),
          RuleReference("DOT", isToken: true),
          Repetition(element: RuleReference("paragraph", isToken: false))
        ]),
        lineNumber: 108),
    GrammarRule(
        name: "paragraph",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("DOT", isToken: true),
          Repetition(element: RuleReference("sentence", isToken: false))
        ]),
        lineNumber: 109),
    GrammarRule(
        name: "sentence",
        body: Sequence(elements: [
          RuleReference("statement", isToken: false),
          Repetition(element: RuleReference("statement", isToken: false)),
          RuleReference("DOT", isToken: true)
        ]),
        lineNumber: 110),
    GrammarRule(
        name: "statement",
        body: Alternation(choices: [
          RuleReference("move_stmt", isToken: false),
          RuleReference("display_stmt", isToken: false),
          RuleReference("accept_stmt", isToken: false),
          RuleReference("add_stmt", isToken: false),
          RuleReference("subtract_stmt", isToken: false),
          RuleReference("multiply_stmt", isToken: false),
          RuleReference("divide_stmt", isToken: false),
          RuleReference("compute_stmt", isToken: false),
          RuleReference("perform_stmt", isToken: false),
          RuleReference("goto_stmt", isToken: false),
          RuleReference("if_stmt", isToken: false),
          RuleReference("set_stmt", isToken: false),
          RuleReference("evaluate_stmt", isToken: false),
          RuleReference("string_stmt", isToken: false),
          RuleReference("unstring_stmt", isToken: false),
          RuleReference("inspect_stmt", isToken: false),
          RuleReference("stop_stmt", isToken: false)
        ]),
        lineNumber: 112),
    GrammarRule(
        name: "move_stmt",
        body: Sequence(elements: [
          Literal("MOVE"),
          RuleReference("operand", isToken: false),
          Literal("TO"),
          RuleReference("NAME", isToken: true),
          Repetition(element: RuleReference("NAME", isToken: true))
        ]),
        lineNumber: 117),
    GrammarRule(
        name: "set_stmt",
        body: Sequence(elements: [
          Literal("SET"),
          RuleReference("NAME", isToken: true),
          Literal("TO"),
          Literal("TRUE")
        ]),
        lineNumber: 120),
    GrammarRule(
        name: "evaluate_stmt",
        body: Sequence(elements: [
          Literal("EVALUATE"),
          RuleReference("operand", isToken: false),
          Repetition(element: RuleReference("when_branch", isToken: false)),
          Literal("END-EVALUATE")
        ]),
        lineNumber: 127),
    GrammarRule(
        name: "when_branch",
        body: Sequence(elements: [
          Literal("WHEN"),
          Group(
              element: Alternation(choices: [
            Literal("OTHER"),
            Sequence(elements: [
              RuleReference("when_value", isToken: false),
              Repetition(element: RuleReference("when_value", isToken: false))
            ])
          ])),
          Repetition(element: RuleReference("statement", isToken: false))
        ]),
        lineNumber: 128),
    GrammarRule(
        name: "when_value",
        body: Sequence(elements: [
          RuleReference("operand", isToken: false),
          Optional(
              element: Sequence(elements: [
            Group(
                element: Alternation(
                    choices: [Literal("THRU"), Literal("THROUGH")])),
            RuleReference("operand", isToken: false)
          ]))
        ]),
        lineNumber: 129),
    GrammarRule(
        name: "string_stmt",
        body: Sequence(elements: [
          Literal("STRING"),
          RuleReference("operand", isToken: false),
          Repetition(element: RuleReference("operand", isToken: false)),
          Literal("DELIMITED"),
          Literal("BY"),
          RuleReference("string_delim", isToken: false),
          Literal("INTO"),
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("WITH"),
            Literal("POINTER"),
            RuleReference("NAME", isToken: true)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("ON"),
            Literal("OVERFLOW"),
            Repetition(element: RuleReference("statement", isToken: false))
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("NOT"),
            Literal("ON"),
            Literal("OVERFLOW"),
            Repetition(element: RuleReference("statement", isToken: false))
          ])),
          Optional(element: Literal("END-STRING"))
        ]),
        lineNumber: 143),
    GrammarRule(
        name: "string_delim",
        body: Alternation(choices: [
          Literal("SIZE"),
          RuleReference("operand", isToken: false)
        ]),
        lineNumber: 148),
    GrammarRule(
        name: "unstring_stmt",
        body: Sequence(elements: [
          Literal("UNSTRING"),
          RuleReference("operand", isToken: false),
          Literal("DELIMITED"),
          Literal("BY"),
          RuleReference("operand", isToken: false),
          Literal("INTO"),
          RuleReference("NAME", isToken: true),
          Repetition(element: RuleReference("NAME", isToken: true)),
          Optional(
              element: Sequence(elements: [
            Literal("WITH"),
            Literal("POINTER"),
            RuleReference("NAME", isToken: true)
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("ON"),
            Literal("OVERFLOW"),
            Repetition(element: RuleReference("statement", isToken: false))
          ])),
          Optional(
              element: Sequence(elements: [
            Literal("NOT"),
            Literal("ON"),
            Literal("OVERFLOW"),
            Repetition(element: RuleReference("statement", isToken: false))
          ])),
          Optional(element: Literal("END-UNSTRING"))
        ]),
        lineNumber: 160),
    GrammarRule(
        name: "inspect_stmt",
        body: Sequence(elements: [
          Literal("INSPECT"),
          RuleReference("operand", isToken: false),
          Group(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("inspect_tallying", isToken: false),
              Optional(
                  element: RuleReference("inspect_replacing", isToken: false))
            ]),
            RuleReference("inspect_replacing", isToken: false),
            RuleReference("inspect_converting", isToken: false)
          ])),
          Optional(element: Literal("END-INSPECT"))
        ]),
        lineNumber: 177),
    GrammarRule(
        name: "inspect_tallying",
        body: Sequence(elements: [
          Literal("TALLYING"),
          RuleReference("tally_for", isToken: false),
          Repetition(element: RuleReference("tally_for", isToken: false))
        ]),
        lineNumber: 182),
    GrammarRule(
        name: "tally_for",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          Literal("FOR"),
          RuleReference("tally_item", isToken: false),
          Repetition(element: RuleReference("tally_item", isToken: false))
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "tally_item",
        body: Alternation(choices: [
          Sequence(elements: [
            Group(
                element:
                    Alternation(choices: [Literal("ALL"), Literal("LEADING")])),
            RuleReference("operand", isToken: false),
            Repetition(element: RuleReference("inspect_region", isToken: false))
          ]),
          Sequence(elements: [
            Literal("CHARACTERS"),
            Repetition(element: RuleReference("inspect_region", isToken: false))
          ])
        ]),
        lineNumber: 184),
    GrammarRule(
        name: "inspect_replacing",
        body: Sequence(elements: [
          Literal("REPLACING"),
          RuleReference("replace_item", isToken: false),
          Repetition(element: RuleReference("replace_item", isToken: false))
        ]),
        lineNumber: 186),
    GrammarRule(
        name: "replace_item",
        body: Alternation(choices: [
          Sequence(elements: [
            Literal("CHARACTERS"),
            Literal("BY"),
            RuleReference("operand", isToken: false),
            Repetition(element: RuleReference("inspect_region", isToken: false))
          ]),
          Sequence(elements: [
            Group(
                element:
                    Alternation(choices: [Literal("ALL"), Literal("LEADING")])),
            RuleReference("operand", isToken: false),
            Literal("BY"),
            RuleReference("operand", isToken: false),
            Repetition(element: RuleReference("inspect_region", isToken: false))
          ])
        ]),
        lineNumber: 187),
    GrammarRule(
        name: "inspect_converting",
        body: Sequence(elements: [
          Literal("CONVERTING"),
          RuleReference("operand", isToken: false),
          Literal("TO"),
          RuleReference("operand", isToken: false),
          Repetition(element: RuleReference("inspect_region", isToken: false))
        ]),
        lineNumber: 196),
    GrammarRule(
        name: "inspect_region",
        body: Sequence(elements: [
          Group(
              element:
                  Alternation(choices: [Literal("BEFORE"), Literal("AFTER")])),
          RuleReference("operand", isToken: false)
        ]),
        lineNumber: 200),
    GrammarRule(
        name: "display_stmt",
        body: Sequence(elements: [
          Literal("DISPLAY"),
          RuleReference("operand", isToken: false),
          Repetition(element: RuleReference("operand", isToken: false))
        ]),
        lineNumber: 201),
    GrammarRule(
        name: "accept_stmt",
        body: Sequence(elements: [
          Literal("ACCEPT"),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 202),
    GrammarRule(
        name: "add_stmt",
        body: Sequence(elements: [
          Literal("ADD"),
          RuleReference("operand", isToken: false),
          Repetition(element: RuleReference("operand", isToken: false)),
          Literal("TO"),
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("GIVING"),
            RuleReference("NAME", isToken: true)
          ])),
          Optional(element: Literal("ROUNDED")),
          Optional(element: RuleReference("size_error", isToken: false))
        ]),
        lineNumber: 205),
    GrammarRule(
        name: "subtract_stmt",
        body: Sequence(elements: [
          Literal("SUBTRACT"),
          RuleReference("operand", isToken: false),
          Repetition(element: RuleReference("operand", isToken: false)),
          Literal("FROM"),
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            Literal("GIVING"),
            RuleReference("NAME", isToken: true)
          ])),
          Optional(element: Literal("ROUNDED")),
          Optional(element: RuleReference("size_error", isToken: false))
        ]),
        lineNumber: 207),
    GrammarRule(
        name: "multiply_stmt",
        body: Sequence(elements: [
          Literal("MULTIPLY"),
          RuleReference("operand", isToken: false),
          Literal("BY"),
          RuleReference("operand", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("GIVING"),
            RuleReference("NAME", isToken: true)
          ])),
          Optional(element: Literal("ROUNDED")),
          Optional(element: RuleReference("size_error", isToken: false))
        ]),
        lineNumber: 209),
    GrammarRule(
        name: "divide_stmt",
        body: Sequence(elements: [
          Literal("DIVIDE"),
          RuleReference("operand", isToken: false),
          Literal("INTO"),
          RuleReference("operand", isToken: false),
          Optional(
              element: Sequence(elements: [
            Literal("GIVING"),
            RuleReference("NAME", isToken: true)
          ])),
          Optional(element: Literal("ROUNDED")),
          Optional(element: RuleReference("size_error", isToken: false))
        ]),
        lineNumber: 211),
    GrammarRule(
        name: "compute_stmt",
        body: Sequence(elements: [
          Literal("COMPUTE"),
          RuleReference("NAME", isToken: true),
          Optional(element: Literal("ROUNDED")),
          RuleReference("EQ", isToken: true),
          RuleReference("arith_expr", isToken: false),
          Optional(element: RuleReference("size_error", isToken: false))
        ]),
        lineNumber: 219),
    GrammarRule(
        name: "size_error",
        body: Sequence(elements: [
          Literal("ON"),
          Literal("SIZE"),
          Literal("ERROR"),
          Repetition(element: RuleReference("statement", isToken: false))
        ]),
        lineNumber: 220),
    GrammarRule(
        name: "arith_expr",
        body: Sequence(elements: [
          RuleReference("arith_term", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("arith_term", isToken: false)
          ]))
        ]),
        lineNumber: 236),
    GrammarRule(
        name: "arith_term",
        body: Sequence(elements: [
          RuleReference("arith_factor", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true)
            ])),
            RuleReference("arith_factor", isToken: false)
          ]))
        ]),
        lineNumber: 237),
    GrammarRule(
        name: "arith_factor",
        body: Sequence(elements: [
          RuleReference("arith_unary", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("POW", isToken: true),
            RuleReference("arith_unary", isToken: false)
          ]))
        ]),
        lineNumber: 238),
    GrammarRule(
        name: "arith_unary",
        body: Sequence(elements: [
          Optional(
              element: Alternation(choices: [
            RuleReference("PLUS", isToken: true),
            RuleReference("MINUS", isToken: true)
          ])),
          RuleReference("arith_primary", isToken: false)
        ]),
        lineNumber: 239),
    GrammarRule(
        name: "arith_primary",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("NAME", isToken: true),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("arith_expr", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 240),
    GrammarRule(
        name: "perform_stmt",
        body: Sequence(elements: [
          Literal("PERFORM"),
          RuleReference("NAME", isToken: true),
          Optional(
              element: Alternation(choices: [
            Sequence(elements: [
              Literal("THROUGH"),
              RuleReference("NAME", isToken: true)
            ]),
            Sequence(elements: [
              Literal("THRU"),
              RuleReference("NAME", isToken: true)
            ])
          ])),
          Optional(
              element: Alternation(choices: [
            Sequence(elements: [
              RuleReference("operand", isToken: false),
              Literal("TIMES")
            ]),
            Sequence(elements: [
              Literal("UNTIL"),
              RuleReference("condition", isToken: false)
            ]),
            RuleReference("perform_varying", isToken: false)
          ]))
        ]),
        lineNumber: 241),
    GrammarRule(
        name: "perform_varying",
        body: Sequence(elements: [
          Literal("VARYING"),
          RuleReference("NAME", isToken: true),
          Literal("FROM"),
          RuleReference("operand", isToken: false),
          Literal("BY"),
          RuleReference("operand", isToken: false),
          Literal("UNTIL"),
          RuleReference("condition", isToken: false)
        ]),
        lineNumber: 245),
    GrammarRule(
        name: "goto_stmt",
        body: Sequence(elements: [
          Literal("GO"),
          Optional(element: Literal("TO")),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 246),
    GrammarRule(
        name: "stop_stmt",
        body: Sequence(elements: [
          Literal("STOP"),
          Group(
              element: Alternation(choices: [
            Literal("RUN"),
            RuleReference("NUMBER", isToken: true),
            RuleReference("STRING", isToken: true)
          ]))
        ]),
        lineNumber: 247),
    GrammarRule(
        name: "if_stmt",
        body: Sequence(elements: [
          Literal("IF"),
          RuleReference("condition", isToken: false),
          Repetition(element: RuleReference("statement", isToken: false)),
          Optional(
              element: Sequence(elements: [
            Literal("ELSE"),
            Repetition(element: RuleReference("statement", isToken: false))
          ]))
        ]),
        lineNumber: 263),
    GrammarRule(
        name: "condition",
        body: RuleReference("disjunction", isToken: false),
        lineNumber: 264),
    GrammarRule(
        name: "disjunction",
        body: Sequence(elements: [
          RuleReference("conjunction", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("OR"),
            RuleReference("conjunction", isToken: false)
          ]))
        ]),
        lineNumber: 265),
    GrammarRule(
        name: "conjunction",
        body: Sequence(elements: [
          RuleReference("negation", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Literal("AND"),
            RuleReference("negation", isToken: false)
          ]))
        ]),
        lineNumber: 266),
    GrammarRule(
        name: "negation",
        body: Sequence(elements: [
          Optional(element: Literal("NOT")),
          RuleReference("simple_condition", isToken: false)
        ]),
        lineNumber: 272),
    GrammarRule(
        name: "simple_condition",
        body: Alternation(choices: [
          RuleReference("relation", isToken: false),
          RuleReference("condition_name", isToken: false),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("condition", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 273),
    GrammarRule(
        name: "relation",
        body: Sequence(elements: [
          RuleReference("operand", isToken: false),
          RuleReference("relop", isToken: false),
          RuleReference("operand", isToken: false)
        ]),
        lineNumber: 274),
    GrammarRule(
        name: "condition_name",
        body: RuleReference("NAME", isToken: true),
        lineNumber: 275),
    GrammarRule(
        name: "relop",
        body: Sequence(elements: [
          Optional(element: Literal("IS")),
          Optional(element: Literal("NOT")),
          Group(
              element: Alternation(choices: [
            Sequence(elements: [
              Literal("GREATER"),
              Optional(element: Literal("THAN"))
            ]),
            Sequence(elements: [
              Literal("LESS"),
              Optional(element: Literal("THAN"))
            ]),
            Sequence(
                elements: [Literal("EQUAL"), Optional(element: Literal("TO"))]),
            RuleReference("GE", isToken: true),
            RuleReference("LE", isToken: true),
            RuleReference("NE", isToken: true),
            RuleReference("GT", isToken: true),
            RuleReference("LT", isToken: true),
            RuleReference("EQ", isToken: true)
          ]))
        ]),
        lineNumber: 279),
    GrammarRule(
        name: "operand",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            Optional(
                element: Sequence(elements: [
              RuleReference("LPAREN", isToken: true),
              RuleReference("operand", isToken: false),
              RuleReference("COLON", isToken: true),
              Optional(element: RuleReference("operand", isToken: false)),
              RuleReference("RPAREN", isToken: true)
            ]))
          ]),
          RuleReference("literal", isToken: false)
        ]),
        lineNumber: 293),
    GrammarRule(
        name: "literal",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("figurative", isToken: false)
        ]),
        lineNumber: 294),
    GrammarRule(
        name: "figurative",
        body: Alternation(choices: [
          Literal("ZERO"),
          Literal("ZEROS"),
          Literal("ZEROES"),
          Literal("SPACE"),
          Literal("SPACES"),
          Literal("HIGH-VALUE"),
          Literal("HIGH-VALUES"),
          Literal("LOW-VALUE"),
          Literal("LOW-VALUES"),
          Literal("QUOTE"),
          Literal("QUOTES")
        ]),
        lineNumber: 295),
  ],
);
