// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/apl/apl.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "program",
        body: Repetition(element: RuleReference("line", isToken: false)),
        lineNumber: 71),
    GrammarRule(
        name: "line",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("statement", isToken: false),
            RuleReference("NEWLINE", isToken: true)
          ]),
          RuleReference("statement", isToken: false),
          RuleReference("NEWLINE", isToken: true)
        ]),
        lineNumber: 73),
    GrammarRule(
        name: "statement",
        body: RuleReference("assignment", isToken: false),
        lineNumber: 77),
    GrammarRule(
        name: "assignment",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("ARROW", isToken: true),
            RuleReference("assignment", isToken: false)
          ]),
          RuleReference("value_expr", isToken: false)
        ]),
        lineNumber: 84),
    GrammarRule(
        name: "value_expr",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("term", isToken: false),
            Optional(
                element: Sequence(elements: [
              RuleReference("function_expr", isToken: false),
              RuleReference("value_expr", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            RuleReference("function_expr", isToken: false),
            RuleReference("value_expr", isToken: false)
          ])
        ]),
        lineNumber: 95),
    GrammarRule(
        name: "term",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NUMBER", isToken: true),
            Repetition(element: RuleReference("NUMBER", isToken: true))
          ]),
          RuleReference("NAME", isToken: true),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("value_expr", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 104),
    GrammarRule(
        name: "function_atom",
        body: Alternation(choices: [
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("TIMES", isToken: true),
          RuleReference("DIVIDE", isToken: true),
          RuleReference("CEILING", isToken: true),
          RuleReference("FLOOR", isToken: true),
          RuleReference("RHO", isToken: true),
          RuleReference("IOTA", isToken: true),
          RuleReference("RAVEL", isToken: true),
          RuleReference("EQ", isToken: true),
          RuleReference("NE", isToken: true),
          RuleReference("LT", isToken: true),
          RuleReference("LE", isToken: true),
          RuleReference("GE", isToken: true),
          RuleReference("GT", isToken: true)
        ]),
        lineNumber: 117),
    GrammarRule(
        name: "function_expr",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("function_atom", isToken: false),
            Optional(
                element: Alternation(choices: [
              RuleReference("REDUCE", isToken: true),
              RuleReference("SCAN", isToken: true)
            ]))
          ]),
          Sequence(elements: [
            RuleReference("OUTER", isToken: true),
            RuleReference("function_atom", isToken: false)
          ])
        ]),
        lineNumber: 129),
  ],
);
