// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/mosaic/mosaic.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "file",
        body: Sequence(elements: [
          Repetition(element: RuleReference("import_decl", isToken: false)),
          RuleReference("component_decl", isToken: false)
        ]),
        lineNumber: 20),
    GrammarRule(
        name: "import_decl",
        body: Sequence(elements: [
          RuleReference("KEYWORD", isToken: true),
          RuleReference("NAME", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("KEYWORD", isToken: true),
            RuleReference("NAME", isToken: true)
          ])),
          RuleReference("KEYWORD", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 30),
    GrammarRule(
        name: "component_decl",
        body: Sequence(elements: [
          RuleReference("KEYWORD", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("slot_decl", isToken: false)),
          RuleReference("node_tree", isToken: false),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 48),
    GrammarRule(
        name: "slot_decl",
        body: Sequence(elements: [
          RuleReference("KEYWORD", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("slot_type", isToken: false),
          Optional(
              element: Sequence(elements: [
            RuleReference("EQUALS", isToken: true),
            RuleReference("default_value", isToken: false)
          ])),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 67),
    GrammarRule(
        name: "slot_type",
        body: Alternation(choices: [
          RuleReference("list_type", isToken: false),
          RuleReference("KEYWORD", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 69),
    GrammarRule(
        name: "list_type",
        body: Sequence(elements: [
          RuleReference("KEYWORD", isToken: true),
          RuleReference("LANGLE", isToken: true),
          RuleReference("slot_type", isToken: false),
          RuleReference("RANGLE", isToken: true)
        ]),
        lineNumber: 73),
    GrammarRule(
        name: "default_value",
        body: Alternation(choices: [
          RuleReference("STRING", isToken: true),
          RuleReference("NUMBER", isToken: true),
          RuleReference("DIMENSION", isToken: true),
          RuleReference("COLOR_HEX", isToken: true),
          RuleReference("KEYWORD", isToken: true)
        ]),
        lineNumber: 75),
    GrammarRule(
        name: "node_tree",
        body: RuleReference("node_element", isToken: false),
        lineNumber: 86),
    GrammarRule(
        name: "node_element",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("node_content", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 88),
    GrammarRule(
        name: "node_content",
        body: Alternation(choices: [
          RuleReference("property_assignment", isToken: false),
          RuleReference("child_node", isToken: false),
          RuleReference("slot_reference", isToken: false),
          RuleReference("when_block", isToken: false),
          RuleReference("each_block", isToken: false)
        ]),
        lineNumber: 90),
    GrammarRule(
        name: "property_assignment",
        body: Sequence(elements: [
          Group(
              element: Alternation(choices: [
            RuleReference("NAME", isToken: true),
            RuleReference("KEYWORD", isToken: true)
          ])),
          RuleReference("COLON", isToken: true),
          RuleReference("property_value", isToken: false),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 107),
    GrammarRule(
        name: "property_value",
        body: Alternation(choices: [
          RuleReference("slot_ref", isToken: false),
          RuleReference("enum_value", isToken: false),
          RuleReference("STRING", isToken: true),
          RuleReference("NUMBER", isToken: true),
          RuleReference("DIMENSION", isToken: true),
          RuleReference("COLOR_HEX", isToken: true),
          RuleReference("KEYWORD", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 111),
    GrammarRule(
        name: "slot_ref",
        body: Sequence(elements: [
          RuleReference("AT", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 122),
    GrammarRule(
        name: "enum_value",
        body: Sequence(elements: [
          RuleReference("NAME", isToken: true),
          RuleReference("DOT", isToken: true),
          RuleReference("NAME", isToken: true)
        ]),
        lineNumber: 124),
    GrammarRule(
        name: "child_node",
        body: RuleReference("node_element", isToken: false),
        lineNumber: 131),
    GrammarRule(
        name: "slot_reference",
        body: Sequence(elements: [
          RuleReference("AT", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 144),
    GrammarRule(
        name: "when_block",
        body: Sequence(elements: [
          RuleReference("KEYWORD", isToken: true),
          RuleReference("slot_ref", isToken: false),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("node_content", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 156),
    GrammarRule(
        name: "each_block",
        body: Sequence(elements: [
          RuleReference("KEYWORD", isToken: true),
          RuleReference("slot_ref", isToken: false),
          RuleReference("KEYWORD", isToken: true),
          RuleReference("NAME", isToken: true),
          RuleReference("LBRACE", isToken: true),
          Repetition(element: RuleReference("node_content", isToken: false)),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 170),
  ],
);
