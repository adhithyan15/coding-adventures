// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/j/j.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "program",
        body: Repetition(element: RuleReference("line", isToken: false)),
        lineNumber: 88),
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
        lineNumber: 90),
    GrammarRule(
        name: "statement",
        body: RuleReference("assignment", isToken: false),
        lineNumber: 94),
    GrammarRule(
        name: "assignment",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("ASSIGN_LOCAL", isToken: true),
            RuleReference("assignment", isToken: false)
          ]),
          Sequence(elements: [
            RuleReference("NAME", isToken: true),
            RuleReference("ASSIGN_GLOBAL", isToken: true),
            RuleReference("assignment", isToken: false)
          ]),
          RuleReference("noun_expr", isToken: false)
        ]),
        lineNumber: 100),
    GrammarRule(
        name: "noun_expr",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("term", isToken: false),
            Optional(
                element: Sequence(elements: [
              RuleReference("verb_expr", isToken: false),
              RuleReference("noun_expr", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            RuleReference("verb_expr", isToken: false),
            RuleReference("noun_expr", isToken: false)
          ])
        ]),
        lineNumber: 111),
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
            RuleReference("noun_expr", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 117),
    GrammarRule(
        name: "verb_expr",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("simple_verb", isToken: false),
            Optional(
                element: Sequence(elements: [
              RuleReference("AT", isToken: true),
              RuleReference("verb_expr", isToken: false)
            ]))
          ]),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("verb_train", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 129),
    GrammarRule(
        name: "simple_verb",
        body: Sequence(elements: [
          RuleReference("verb_primitive", isToken: false),
          Optional(
              element: Alternation(choices: [
            RuleReference("REDUCE", isToken: true),
            RuleReference("SCAN", isToken: true)
          ]))
        ]),
        lineNumber: 138),
    GrammarRule(
        name: "verb_primitive",
        body: Alternation(choices: [
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("PERCENT", isToken: true),
          RuleReference("CARET", isToken: true),
          RuleReference("FLOOR", isToken: true),
          RuleReference("CEILING", isToken: true),
          RuleReference("DOLLAR", isToken: true),
          RuleReference("IDOT", isToken: true),
          RuleReference("RAVEL", isToken: true),
          RuleReference("HASH", isToken: true),
          RuleReference("EQ", isToken: true),
          RuleReference("NE", isToken: true),
          RuleReference("LT", isToken: true),
          RuleReference("GT", isToken: true),
          RuleReference("LE", isToken: true),
          RuleReference("GE", isToken: true)
        ]),
        lineNumber: 140),
    GrammarRule(
        name: "verb_train",
        body: Sequence(elements: [
          RuleReference("train_tooth", isToken: false),
          RuleReference("train_tooth", isToken: false),
          Repetition(element: RuleReference("train_tooth", isToken: false))
        ]),
        lineNumber: 148),
    GrammarRule(
        name: "train_tooth",
        body: Alternation(choices: [
          RuleReference("verb_expr", isToken: false),
          RuleReference("term", isToken: false)
        ]),
        lineNumber: 155),
  ],
);
