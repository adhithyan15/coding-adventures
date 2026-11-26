// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/mccarthy_lisp/mccarthy_lisp.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "program",
        body: Repetition(element: RuleReference("sexpr", isToken: false)),
        lineNumber: 30),
    GrammarRule(
        name: "sexpr",
        body: Alternation(choices: [
          RuleReference("atom", isToken: false),
          RuleReference("list", isToken: false),
          RuleReference("quoted", isToken: false)
        ]),
        lineNumber: 35),
    GrammarRule(
        name: "atom",
        body: Alternation(choices: [
          RuleReference("SYMBOL", isToken: true),
          RuleReference("INTEGER", isToken: true)
        ]),
        lineNumber: 38),
    GrammarRule(
        name: "list",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("list_body", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 41),
    GrammarRule(
        name: "list_body",
        body: Optional(
            element: Sequence(elements: [
          RuleReference("sexpr", isToken: false),
          Repetition(element: RuleReference("sexpr", isToken: false)),
          Optional(
              element: Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("sexpr", isToken: false)
          ]))
        ])),
        lineNumber: 42),
    GrammarRule(
        name: "quoted",
        body: Sequence(elements: [
          RuleReference("QUOTE", isToken: true),
          RuleReference("sexpr", isToken: false)
        ]),
        lineNumber: 45),
  ],
);
