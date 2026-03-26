# AUTO-GENERATED FILE - DO NOT EDIT
from grammar_tools.parser_grammar import (
    ParserGrammar, GrammarRule, GrammarElement,
    RuleReference, Literal, Sequence, Alternation,
    Repetition, Optional as OptGroup, Group
)

CssGrammar = ParserGrammar(
    version=1,
    rules=[
        GrammarRule(
            name="stylesheet",
            line_number=33,
            body=Repetition(element=RuleReference(name="rule", is_token=False)),
        ),
        GrammarRule(
            name="rule",
            line_number=35,
            body=Alternation(choices=[RuleReference(name="at_rule", is_token=False), RuleReference(name="qualified_rule", is_token=False)]),
        ),
        GrammarRule(
            name="at_rule",
            line_number=55,
            body=Sequence(elements=[RuleReference(name="AT_KEYWORD", is_token=True), RuleReference(name="at_prelude", is_token=False), Group(element=Alternation(choices=[RuleReference(name="SEMICOLON", is_token=True), RuleReference(name="block", is_token=False)]))]),
        ),
        GrammarRule(
            name="at_prelude",
            line_number=61,
            body=Repetition(element=RuleReference(name="at_prelude_token", is_token=False)),
        ),
        GrammarRule(
            name="at_prelude_token",
            line_number=63,
            body=Alternation(choices=[RuleReference(name="IDENT", is_token=True), RuleReference(name="STRING", is_token=True), RuleReference(name="NUMBER", is_token=True), RuleReference(name="DIMENSION", is_token=True), RuleReference(name="PERCENTAGE", is_token=True), RuleReference(name="HASH", is_token=True), RuleReference(name="CUSTOM_PROPERTY", is_token=True), RuleReference(name="UNICODE_RANGE", is_token=True), RuleReference(name="function_in_prelude", is_token=False), RuleReference(name="paren_block", is_token=False), RuleReference(name="COLON", is_token=True), RuleReference(name="COMMA", is_token=True), RuleReference(name="SLASH", is_token=True), RuleReference(name="DOT", is_token=True), RuleReference(name="STAR", is_token=True), RuleReference(name="PLUS", is_token=True), RuleReference(name="MINUS", is_token=True), RuleReference(name="GREATER", is_token=True), RuleReference(name="TILDE", is_token=True), RuleReference(name="PIPE", is_token=True), RuleReference(name="EQUALS", is_token=True), RuleReference(name="AMPERSAND", is_token=True), RuleReference(name="CDO", is_token=True), RuleReference(name="CDC", is_token=True)]),
        ),
        GrammarRule(
            name="function_in_prelude",
            line_number=71,
            body=Sequence(elements=[RuleReference(name="FUNCTION", is_token=True), RuleReference(name="at_prelude_tokens", is_token=False), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="paren_block",
            line_number=72,
            body=Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="at_prelude_tokens", is_token=False), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="at_prelude_tokens",
            line_number=73,
            body=Repetition(element=RuleReference(name="at_prelude_token", is_token=False)),
        ),
        GrammarRule(
            name="qualified_rule",
            line_number=85,
            body=Sequence(elements=[RuleReference(name="selector_list", is_token=False), RuleReference(name="block", is_token=False)]),
        ),
        GrammarRule(
            name="selector_list",
            line_number=96,
            body=Sequence(elements=[RuleReference(name="complex_selector", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="complex_selector", is_token=False)]))]),
        ),
        GrammarRule(
            name="complex_selector",
            line_number=105,
            body=Sequence(elements=[RuleReference(name="compound_selector", is_token=False), Repetition(element=Sequence(elements=[OptGroup(element=RuleReference(name="combinator", is_token=False)), RuleReference(name="compound_selector", is_token=False)]))]),
        ),
        GrammarRule(
            name="combinator",
            line_number=112,
            body=Alternation(choices=[RuleReference(name="GREATER", is_token=True), RuleReference(name="PLUS", is_token=True), RuleReference(name="TILDE", is_token=True)]),
        ),
        GrammarRule(
            name="compound_selector",
            line_number=124,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="simple_selector", is_token=False), Repetition(element=RuleReference(name="subclass_selector", is_token=False))]), Sequence(elements=[RuleReference(name="subclass_selector", is_token=False), Repetition(element=RuleReference(name="subclass_selector", is_token=False))])]),
        ),
        GrammarRule(
            name="simple_selector",
            line_number=131,
            body=Alternation(choices=[RuleReference(name="IDENT", is_token=True), RuleReference(name="STAR", is_token=True), RuleReference(name="AMPERSAND", is_token=True)]),
        ),
        GrammarRule(
            name="subclass_selector",
            line_number=139,
            body=Alternation(choices=[RuleReference(name="class_selector", is_token=False), RuleReference(name="id_selector", is_token=False), RuleReference(name="attribute_selector", is_token=False), RuleReference(name="pseudo_class", is_token=False), RuleReference(name="pseudo_element", is_token=False)]),
        ),
        GrammarRule(
            name="class_selector",
            line_number=145,
            body=Sequence(elements=[RuleReference(name="DOT", is_token=True), RuleReference(name="IDENT", is_token=True)]),
        ),
        GrammarRule(
            name="id_selector",
            line_number=150,
            body=RuleReference(name="HASH", is_token=True),
        ),
        GrammarRule(
            name="attribute_selector",
            line_number=161,
            body=Sequence(elements=[RuleReference(name="LBRACKET", is_token=True), RuleReference(name="IDENT", is_token=True), OptGroup(element=Sequence(elements=[RuleReference(name="attr_matcher", is_token=False), RuleReference(name="attr_value", is_token=False), OptGroup(element=RuleReference(name="IDENT", is_token=True))])), RuleReference(name="RBRACKET", is_token=True)]),
        ),
        GrammarRule(
            name="attr_matcher",
            line_number=163,
            body=Alternation(choices=[RuleReference(name="EQUALS", is_token=True), RuleReference(name="TILDE_EQUALS", is_token=True), RuleReference(name="PIPE_EQUALS", is_token=True), RuleReference(name="CARET_EQUALS", is_token=True), RuleReference(name="DOLLAR_EQUALS", is_token=True), RuleReference(name="STAR_EQUALS", is_token=True)]),
        ),
        GrammarRule(
            name="attr_value",
            line_number=166,
            body=Alternation(choices=[RuleReference(name="IDENT", is_token=True), RuleReference(name="STRING", is_token=True)]),
        ),
        GrammarRule(
            name="pseudo_class",
            line_number=173,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="COLON", is_token=True), RuleReference(name="FUNCTION", is_token=True), RuleReference(name="pseudo_class_args", is_token=False), RuleReference(name="RPAREN", is_token=True)]), Sequence(elements=[RuleReference(name="COLON", is_token=True), RuleReference(name="IDENT", is_token=True)])]),
        ),
        GrammarRule(
            name="pseudo_class_args",
            line_number=181,
            body=Repetition(element=RuleReference(name="pseudo_class_arg", is_token=False)),
        ),
        GrammarRule(
            name="pseudo_class_arg",
            line_number=183,
            body=Alternation(choices=[RuleReference(name="IDENT", is_token=True), RuleReference(name="NUMBER", is_token=True), RuleReference(name="DIMENSION", is_token=True), RuleReference(name="STRING", is_token=True), RuleReference(name="HASH", is_token=True), RuleReference(name="PLUS", is_token=True), RuleReference(name="COMMA", is_token=True), RuleReference(name="DOT", is_token=True), RuleReference(name="STAR", is_token=True), RuleReference(name="COLON", is_token=True), RuleReference(name="AMPERSAND", is_token=True), Sequence(elements=[RuleReference(name="FUNCTION", is_token=True), RuleReference(name="pseudo_class_args", is_token=False), RuleReference(name="RPAREN", is_token=True)]), Sequence(elements=[RuleReference(name="LBRACKET", is_token=True), RuleReference(name="pseudo_class_args", is_token=False), RuleReference(name="RBRACKET", is_token=True)])]),
        ),
        GrammarRule(
            name="pseudo_element",
            line_number=190,
            body=Sequence(elements=[RuleReference(name="COLON_COLON", is_token=True), RuleReference(name="IDENT", is_token=True)]),
        ),
        GrammarRule(
            name="block",
            line_number=200,
            body=Sequence(elements=[RuleReference(name="LBRACE", is_token=True), RuleReference(name="block_contents", is_token=False), RuleReference(name="RBRACE", is_token=True)]),
        ),
        GrammarRule(
            name="block_contents",
            line_number=202,
            body=Repetition(element=RuleReference(name="block_item", is_token=False)),
        ),
        GrammarRule(
            name="block_item",
            line_number=211,
            body=Alternation(choices=[RuleReference(name="at_rule", is_token=False), RuleReference(name="declaration_or_nested", is_token=False)]),
        ),
        GrammarRule(
            name="declaration_or_nested",
            line_number=217,
            body=Alternation(choices=[RuleReference(name="declaration", is_token=False), RuleReference(name="qualified_rule", is_token=False)]),
        ),
        GrammarRule(
            name="declaration",
            line_number=231,
            body=Sequence(elements=[RuleReference(name="property", is_token=False), RuleReference(name="COLON", is_token=True), RuleReference(name="value_list", is_token=False), OptGroup(element=RuleReference(name="priority", is_token=False)), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="property",
            line_number=233,
            body=Alternation(choices=[RuleReference(name="IDENT", is_token=True), RuleReference(name="CUSTOM_PROPERTY", is_token=True)]),
        ),
        GrammarRule(
            name="priority",
            line_number=238,
            body=Sequence(elements=[RuleReference(name="BANG", is_token=True), Literal(value="important")]),
        ),
        GrammarRule(
            name="value_list",
            line_number=251,
            body=Sequence(elements=[RuleReference(name="value", is_token=False), Repetition(element=RuleReference(name="value", is_token=False))]),
        ),
        GrammarRule(
            name="value",
            line_number=253,
            body=Alternation(choices=[RuleReference(name="DIMENSION", is_token=True), RuleReference(name="PERCENTAGE", is_token=True), RuleReference(name="NUMBER", is_token=True), RuleReference(name="STRING", is_token=True), RuleReference(name="IDENT", is_token=True), RuleReference(name="HASH", is_token=True), RuleReference(name="CUSTOM_PROPERTY", is_token=True), RuleReference(name="UNICODE_RANGE", is_token=True), RuleReference(name="function_call", is_token=False), RuleReference(name="SLASH", is_token=True), RuleReference(name="COMMA", is_token=True), RuleReference(name="PLUS", is_token=True), RuleReference(name="MINUS", is_token=True)]),
        ),
        GrammarRule(
            name="function_call",
            line_number=267,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="FUNCTION", is_token=True), RuleReference(name="function_args", is_token=False), RuleReference(name="RPAREN", is_token=True)]), RuleReference(name="URL_TOKEN", is_token=True)]),
        ),
        GrammarRule(
            name="function_args",
            line_number=272,
            body=Repetition(element=RuleReference(name="function_arg", is_token=False)),
        ),
        GrammarRule(
            name="function_arg",
            line_number=274,
            body=Alternation(choices=[RuleReference(name="DIMENSION", is_token=True), RuleReference(name="PERCENTAGE", is_token=True), RuleReference(name="NUMBER", is_token=True), RuleReference(name="STRING", is_token=True), RuleReference(name="IDENT", is_token=True), RuleReference(name="HASH", is_token=True), RuleReference(name="CUSTOM_PROPERTY", is_token=True), RuleReference(name="COMMA", is_token=True), RuleReference(name="SLASH", is_token=True), RuleReference(name="PLUS", is_token=True), RuleReference(name="MINUS", is_token=True), RuleReference(name="STAR", is_token=True), Sequence(elements=[RuleReference(name="FUNCTION", is_token=True), RuleReference(name="function_args", is_token=False), RuleReference(name="RPAREN", is_token=True)])]),
        ),
    ],
)
