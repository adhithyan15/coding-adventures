// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/typescript/typescript.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "program",
        body: Repetition(element: RuleReference("statement", isToken: false)),
        lineNumber: 34),
    GrammarRule(
        name: "statement",
        body: Alternation(choices: [
          RuleReference("var_declaration", isToken: false),
          RuleReference("assignment", isToken: false),
          RuleReference("expression_stmt", isToken: false)
        ]),
        lineNumber: 35),
    GrammarRule(
        name: "var_declaration",
        body: Sequence(elements: [
          RuleReference("KEYWORD", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 36),
    GrammarRule(
        name: "assignment",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("expression", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 37),
    GrammarRule(
        name: "expression_stmt",
        body: Sequence(elements: [
          RuleReference("expression", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 38),
    GrammarRule(
        name: "expression",
        body: Sequence(elements: [
          RuleReference("term", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("PLUS", isToken: true),
              RuleReference("MINUS", isToken: true)
            ])),
            RuleReference("term", isToken: false)
          ]))
        ]),
        lineNumber: 39),
    GrammarRule(
        name: "term",
        body: Sequence(elements: [
          RuleReference("factor", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Group(
                element: Alternation(choices: [
              RuleReference("STAR", isToken: true),
              RuleReference("SLASH", isToken: true)
            ])),
            RuleReference("factor", isToken: false)
          ]))
        ]),
        lineNumber: 40),
    GrammarRule(
        name: "factor",
        body: Alternation(choices: [
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("KEYWORD", isToken: true),
          Sequence(elements: [
            RuleReference("LPAREN", isToken: true),
            RuleReference("expression", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 41),
  ],
);
