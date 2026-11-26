// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/css/css.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "stylesheet",
        body: Repetition(element: RuleReference("rule", isToken: false)),
        lineNumber: 33),
    GrammarRule(
        name: "rule",
        body: Alternation(choices: [
          RuleReference("at_rule", isToken: false),
          RuleReference("qualified_rule", isToken: false)
        ]),
        lineNumber: 35),
    GrammarRule(
        name: "at_rule",
        body: Sequence(elements: [
          RuleReference("AT_KEYWORD", isToken: true),
          RuleReference("at_prelude", isToken: false),
          Group(
              element: Alternation(choices: [
            RuleReference("SEMICOLON", isToken: true),
            RuleReference("block", isToken: false)
          ]))
        ]),
        lineNumber: 55),
    GrammarRule(
        name: "at_prelude",
        body: Repetition(
            element: RuleReference("at_prelude_token", isToken: false)),
        lineNumber: 61),
    GrammarRule(
        name: "at_prelude_token",
        body: Alternation(choices: [
          RuleReference("IDENT", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("NUMBER", isToken: true),
          RuleReference("DIMENSION", isToken: true),
          RuleReference("PERCENTAGE", isToken: true),
          RuleReference("HASH", isToken: true),
          RuleReference("CUSTOM_PROPERTY", isToken: true),
          RuleReference("UNICODE_RANGE", isToken: true),
          RuleReference("function_in_prelude", isToken: false),
          RuleReference("paren_block", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("COMMA", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("DOT", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("GREATER", isToken: true),
          RuleReference("TILDE", isToken: true),
          RuleReference("PIPE", isToken: true),
          RuleReference("EQUALS", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          RuleReference("CDO", isToken: true),
          RuleReference("CDC", isToken: true)
        ]),
        lineNumber: 63),
    GrammarRule(
        name: "function_in_prelude",
        body: Sequence(elements: [
          RuleReference("FUNCTION", isToken: true),
          RuleReference("at_prelude_tokens", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 71),
    GrammarRule(
        name: "paren_block",
        body: Sequence(elements: [
          RuleReference("LPAREN", isToken: true),
          RuleReference("at_prelude_tokens", isToken: false),
          RuleReference("RPAREN", isToken: true)
        ]),
        lineNumber: 72),
    GrammarRule(
        name: "at_prelude_tokens",
        body: Repetition(
            element: RuleReference("at_prelude_token", isToken: false)),
        lineNumber: 73),
    GrammarRule(
        name: "qualified_rule",
        body: Sequence(elements: [
          RuleReference("selector_list", isToken: false),
          RuleReference("block", isToken: false)
        ]),
        lineNumber: 85),
    GrammarRule(
        name: "selector_list",
        body: Sequence(elements: [
          RuleReference("complex_selector", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("COMMA", isToken: true),
            RuleReference("complex_selector", isToken: false)
          ]))
        ]),
        lineNumber: 96),
    GrammarRule(
        name: "complex_selector",
        body: Sequence(elements: [
          RuleReference("compound_selector", isToken: false),
          Repetition(
              element: Sequence(elements: [
            Optional(element: RuleReference("combinator", isToken: false)),
            RuleReference("compound_selector", isToken: false)
          ]))
        ]),
        lineNumber: 105),
    GrammarRule(
        name: "combinator",
        body: Alternation(choices: [
          RuleReference("GREATER", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("TILDE", isToken: true)
        ]),
        lineNumber: 112),
    GrammarRule(
        name: "compound_selector",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("simple_selector", isToken: false),
            Repetition(
                element: RuleReference("subclass_selector", isToken: false))
          ]),
          Sequence(elements: [
            RuleReference("subclass_selector", isToken: false),
            Repetition(
                element: RuleReference("subclass_selector", isToken: false))
          ])
        ]),
        lineNumber: 124),
    GrammarRule(
        name: "simple_selector",
        body: Alternation(choices: [
          RuleReference("IDENT", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("AMPERSAND", isToken: true)
        ]),
        lineNumber: 131),
    GrammarRule(
        name: "subclass_selector",
        body: Alternation(choices: [
          RuleReference("class_selector", isToken: false),
          RuleReference("id_selector", isToken: false),
          RuleReference("attribute_selector", isToken: false),
          RuleReference("pseudo_class", isToken: false),
          RuleReference("pseudo_element", isToken: false)
        ]),
        lineNumber: 139),
    GrammarRule(
        name: "class_selector",
        body: Sequence(elements: [
          RuleReference("DOT", isToken: true),
          RuleReference("IDENT", isToken: true)
        ]),
        lineNumber: 145),
    GrammarRule(
        name: "id_selector",
        body: RuleReference("HASH", isToken: true),
        lineNumber: 150),
    GrammarRule(
        name: "attribute_selector",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("IDENT", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("attr_matcher", isToken: false),
            RuleReference("attr_value", isToken: false),
            Optional(element: RuleReference("IDENT", isToken: true))
          ])),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 161),
    GrammarRule(
        name: "attr_matcher",
        body: Alternation(choices: [
          RuleReference("EQUALS", isToken: true),
          RuleReference("TILDE_EQUALS", isToken: true),
          RuleReference("PIPE_EQUALS", isToken: true),
          RuleReference("CARET_EQUALS", isToken: true),
          RuleReference("DOLLAR_EQUALS", isToken: true),
          RuleReference("STAR_EQUALS", isToken: true)
        ]),
        lineNumber: 163),
    GrammarRule(
        name: "attr_value",
        body: Alternation(choices: [
          RuleReference("IDENT", isToken: true),
          RuleReference("STRING", isToken: true)
        ]),
        lineNumber: 166),
    GrammarRule(
        name: "pseudo_class",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("FUNCTION", isToken: true),
            RuleReference("pseudo_class_args", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("COLON", isToken: true),
            RuleReference("IDENT", isToken: true)
          ])
        ]),
        lineNumber: 173),
    GrammarRule(
        name: "pseudo_class_args",
        body: Repetition(
            element: RuleReference("pseudo_class_arg", isToken: false)),
        lineNumber: 181),
    GrammarRule(
        name: "pseudo_class_arg",
        body: Alternation(choices: [
          RuleReference("IDENT", isToken: true),
          RuleReference("NUMBER", isToken: true),
          RuleReference("DIMENSION", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("HASH", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("COMMA", isToken: true),
          RuleReference("DOT", isToken: true),
          RuleReference("STAR", isToken: true),
          RuleReference("COLON", isToken: true),
          RuleReference("AMPERSAND", isToken: true),
          Sequence(elements: [
            RuleReference("FUNCTION", isToken: true),
            RuleReference("pseudo_class_args", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ]),
          Sequence(elements: [
            RuleReference("LBRACKET", isToken: true),
            RuleReference("pseudo_class_args", isToken: false),
            RuleReference("RBRACKET", isToken: true)
          ])
        ]),
        lineNumber: 183),
    GrammarRule(
        name: "pseudo_element",
        body: Sequence(elements: [
          RuleReference("COLON_COLON", isToken: true),
          RuleReference("IDENT", isToken: true)
        ]),
        lineNumber: 190),
    GrammarRule(
        name: "block",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          RuleReference("block_contents", isToken: false),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 200),
    GrammarRule(
        name: "block_contents",
        body: Repetition(element: RuleReference("block_item", isToken: false)),
        lineNumber: 202),
    GrammarRule(
        name: "block_item",
        body: Alternation(choices: [
          RuleReference("at_rule", isToken: false),
          RuleReference("declaration_or_nested", isToken: false)
        ]),
        lineNumber: 211),
    GrammarRule(
        name: "declaration_or_nested",
        body: Alternation(choices: [
          RuleReference("declaration", isToken: false),
          RuleReference("qualified_rule", isToken: false)
        ]),
        lineNumber: 217),
    GrammarRule(
        name: "declaration",
        body: Sequence(elements: [
          RuleReference("property", isToken: false),
          RuleReference("COLON", isToken: true),
          RuleReference("value_list", isToken: false),
          Optional(element: RuleReference("priority", isToken: false)),
          RuleReference("SEMICOLON", isToken: true)
        ]),
        lineNumber: 231),
    GrammarRule(
        name: "property",
        body: Alternation(choices: [
          RuleReference("IDENT", isToken: true),
          RuleReference("CUSTOM_PROPERTY", isToken: true)
        ]),
        lineNumber: 233),
    GrammarRule(
        name: "priority",
        body: Sequence(elements: [
          RuleReference("BANG", isToken: true),
          Literal("important")
        ]),
        lineNumber: 238),
    GrammarRule(
        name: "value_list",
        body: Sequence(elements: [
          RuleReference("value", isToken: false),
          Repetition(element: RuleReference("value", isToken: false))
        ]),
        lineNumber: 251),
    GrammarRule(
        name: "value",
        body: Alternation(choices: [
          RuleReference("DIMENSION", isToken: true),
          RuleReference("PERCENTAGE", isToken: true),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("IDENT", isToken: true),
          RuleReference("HASH", isToken: true),
          RuleReference("CUSTOM_PROPERTY", isToken: true),
          RuleReference("UNICODE_RANGE", isToken: true),
          RuleReference("function_call", isToken: false),
          RuleReference("SLASH", isToken: true),
          RuleReference("COMMA", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true)
        ]),
        lineNumber: 253),
    GrammarRule(
        name: "function_call",
        body: Alternation(choices: [
          Sequence(elements: [
            RuleReference("FUNCTION", isToken: true),
            RuleReference("function_args", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ]),
          RuleReference("URL_TOKEN", isToken: true)
        ]),
        lineNumber: 267),
    GrammarRule(
        name: "function_args",
        body:
            Repetition(element: RuleReference("function_arg", isToken: false)),
        lineNumber: 272),
    GrammarRule(
        name: "function_arg",
        body: Alternation(choices: [
          RuleReference("DIMENSION", isToken: true),
          RuleReference("PERCENTAGE", isToken: true),
          RuleReference("NUMBER", isToken: true),
          RuleReference("STRING", isToken: true),
          RuleReference("IDENT", isToken: true),
          RuleReference("HASH", isToken: true),
          RuleReference("CUSTOM_PROPERTY", isToken: true),
          RuleReference("COMMA", isToken: true),
          RuleReference("SLASH", isToken: true),
          RuleReference("PLUS", isToken: true),
          RuleReference("MINUS", isToken: true),
          RuleReference("STAR", isToken: true),
          Sequence(elements: [
            RuleReference("FUNCTION", isToken: true),
            RuleReference("function_args", isToken: false),
            RuleReference("RPAREN", isToken: true)
          ])
        ]),
        lineNumber: 274),
  ],
);
