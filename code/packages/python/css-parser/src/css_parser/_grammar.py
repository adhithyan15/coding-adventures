# AUTO-GENERATED FILE — DO NOT EDIT
# Source: css.grammar
# Regenerate with: grammar-tools compile-grammar <source.grammar>
#
# This file embeds a ParserGrammar as native Python data structures.
# Downstream packages import PARSER_GRAMMAR directly instead of
# reading and parsing the .grammar file at runtime.

from grammar_tools.parser_grammar import (
    Alternation,
    GrammarRule,
    Group,
    Literal,
    NegativeLookahead,
    OneOrMoreRepetition,
    Optional,
    ParserGrammar,
    PositiveLookahead,
    Repetition,
    RuleReference,
    SeparatedRepetition,
    Sequence,
)

# fmt: off  # noqa: E501 — generated code may have long lines

PARSER_GRAMMAR = ParserGrammar(
    version=1,
    rules=[
        GrammarRule(
            name='stylesheet',
            body=
            Repetition(element=
                RuleReference(name='rule', is_token=False),
            ),
            line_number=33,
        ),
        GrammarRule(
            name='rule',
            body=
            Alternation(choices=[
                RuleReference(name='at_rule', is_token=False),
                RuleReference(name='qualified_rule', is_token=False),
            ]),
            line_number=35,
        ),
        GrammarRule(
            name='at_rule',
            body=
            Sequence(elements=[
                RuleReference(name='AT_KEYWORD', is_token=True),
                RuleReference(name='at_prelude', is_token=False),
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='SEMICOLON', is_token=True),
                        RuleReference(name='block', is_token=False),
                    ]),
                ),
            ]),
            line_number=55,
        ),
        GrammarRule(
            name='at_prelude',
            body=
            Repetition(element=
                RuleReference(name='at_prelude_token', is_token=False),
            ),
            line_number=61,
        ),
        GrammarRule(
            name='at_prelude_token',
            body=
            Alternation(choices=[
                RuleReference(name='IDENT', is_token=True),
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='DIMENSION', is_token=True),
                RuleReference(name='PERCENTAGE', is_token=True),
                RuleReference(name='HASH', is_token=True),
                RuleReference(name='CUSTOM_PROPERTY', is_token=True),
                RuleReference(name='UNICODE_RANGE', is_token=True),
                RuleReference(name='function_in_prelude', is_token=False),
                RuleReference(name='paren_block', is_token=False),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='COMMA', is_token=True),
                RuleReference(name='SLASH', is_token=True),
                RuleReference(name='DOT', is_token=True),
                RuleReference(name='STAR', is_token=True),
                RuleReference(name='PLUS', is_token=True),
                RuleReference(name='MINUS', is_token=True),
                RuleReference(name='GREATER', is_token=True),
                RuleReference(name='TILDE', is_token=True),
                RuleReference(name='PIPE', is_token=True),
                RuleReference(name='EQUALS', is_token=True),
                RuleReference(name='AMPERSAND', is_token=True),
                RuleReference(name='CDO', is_token=True),
                RuleReference(name='CDC', is_token=True),
            ]),
            line_number=63,
        ),
        GrammarRule(
            name='function_in_prelude',
            body=
            Sequence(elements=[
                RuleReference(name='FUNCTION', is_token=True),
                RuleReference(name='at_prelude_tokens', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=71,
        ),
        GrammarRule(
            name='paren_block',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='at_prelude_tokens', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=72,
        ),
        GrammarRule(
            name='at_prelude_tokens',
            body=
            Repetition(element=
                RuleReference(name='at_prelude_token', is_token=False),
            ),
            line_number=73,
        ),
        GrammarRule(
            name='qualified_rule',
            body=
            Sequence(elements=[
                RuleReference(name='selector_list', is_token=False),
                RuleReference(name='block', is_token=False),
            ]),
            line_number=85,
        ),
        GrammarRule(
            name='selector_list',
            body=
            Sequence(elements=[
                RuleReference(name='complex_selector', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='complex_selector', is_token=False),
                    ]),
                ),
            ]),
            line_number=96,
        ),
        GrammarRule(
            name='complex_selector',
            body=
            Sequence(elements=[
                RuleReference(name='compound_selector', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Optional(element=
                            RuleReference(name='combinator', is_token=False),
                        ),
                        RuleReference(name='compound_selector', is_token=False),
                    ]),
                ),
            ]),
            line_number=105,
        ),
        GrammarRule(
            name='combinator',
            body=
            Alternation(choices=[
                RuleReference(name='GREATER', is_token=True),
                RuleReference(name='PLUS', is_token=True),
                RuleReference(name='TILDE', is_token=True),
            ]),
            line_number=112,
        ),
        GrammarRule(
            name='compound_selector',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='simple_selector', is_token=False),
                    Repetition(element=
                        RuleReference(name='subclass_selector', is_token=False),
                    ),
                ]),
                Sequence(elements=[
                    RuleReference(name='subclass_selector', is_token=False),
                    Repetition(element=
                        RuleReference(name='subclass_selector', is_token=False),
                    ),
                ]),
            ]),
            line_number=124,
        ),
        GrammarRule(
            name='simple_selector',
            body=
            Alternation(choices=[
                RuleReference(name='IDENT', is_token=True),
                RuleReference(name='STAR', is_token=True),
                RuleReference(name='AMPERSAND', is_token=True),
            ]),
            line_number=131,
        ),
        GrammarRule(
            name='subclass_selector',
            body=
            Alternation(choices=[
                RuleReference(name='class_selector', is_token=False),
                RuleReference(name='id_selector', is_token=False),
                RuleReference(name='attribute_selector', is_token=False),
                RuleReference(name='pseudo_class', is_token=False),
                RuleReference(name='pseudo_element', is_token=False),
            ]),
            line_number=139,
        ),
        GrammarRule(
            name='class_selector',
            body=
            Sequence(elements=[
                RuleReference(name='DOT', is_token=True),
                RuleReference(name='IDENT', is_token=True),
            ]),
            line_number=145,
        ),
        GrammarRule(
            name='id_selector',
            body=
            RuleReference(name='HASH', is_token=True),
            line_number=150,
        ),
        GrammarRule(
            name='attribute_selector',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACKET', is_token=True),
                RuleReference(name='IDENT', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='attr_matcher', is_token=False),
                        RuleReference(name='attr_value', is_token=False),
                        Optional(element=
                            RuleReference(name='IDENT', is_token=True),
                        ),
                    ]),
                ),
                RuleReference(name='RBRACKET', is_token=True),
            ]),
            line_number=161,
        ),
        GrammarRule(
            name='attr_matcher',
            body=
            Alternation(choices=[
                RuleReference(name='EQUALS', is_token=True),
                RuleReference(name='TILDE_EQUALS', is_token=True),
                RuleReference(name='PIPE_EQUALS', is_token=True),
                RuleReference(name='CARET_EQUALS', is_token=True),
                RuleReference(name='DOLLAR_EQUALS', is_token=True),
                RuleReference(name='STAR_EQUALS', is_token=True),
            ]),
            line_number=163,
        ),
        GrammarRule(
            name='attr_value',
            body=
            Alternation(choices=[
                RuleReference(name='IDENT', is_token=True),
                RuleReference(name='STRING', is_token=True),
            ]),
            line_number=166,
        ),
        GrammarRule(
            name='pseudo_class',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='COLON', is_token=True),
                    RuleReference(name='FUNCTION', is_token=True),
                    RuleReference(name='pseudo_class_args', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='COLON', is_token=True),
                    RuleReference(name='IDENT', is_token=True),
                ]),
            ]),
            line_number=173,
        ),
        GrammarRule(
            name='pseudo_class_args',
            body=
            Repetition(element=
                RuleReference(name='pseudo_class_arg', is_token=False),
            ),
            line_number=181,
        ),
        GrammarRule(
            name='pseudo_class_arg',
            body=
            Alternation(choices=[
                RuleReference(name='IDENT', is_token=True),
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='DIMENSION', is_token=True),
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='HASH', is_token=True),
                RuleReference(name='PLUS', is_token=True),
                RuleReference(name='COMMA', is_token=True),
                RuleReference(name='DOT', is_token=True),
                RuleReference(name='STAR', is_token=True),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='AMPERSAND', is_token=True),
                Sequence(elements=[
                    RuleReference(name='FUNCTION', is_token=True),
                    RuleReference(name='pseudo_class_args', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='LBRACKET', is_token=True),
                    RuleReference(name='pseudo_class_args', is_token=False),
                    RuleReference(name='RBRACKET', is_token=True),
                ]),
            ]),
            line_number=183,
        ),
        GrammarRule(
            name='pseudo_element',
            body=
            Sequence(elements=[
                RuleReference(name='COLON_COLON', is_token=True),
                RuleReference(name='IDENT', is_token=True),
            ]),
            line_number=190,
        ),
        GrammarRule(
            name='block',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACE', is_token=True),
                RuleReference(name='block_contents', is_token=False),
                RuleReference(name='RBRACE', is_token=True),
            ]),
            line_number=200,
        ),
        GrammarRule(
            name='block_contents',
            body=
            Repetition(element=
                RuleReference(name='block_item', is_token=False),
            ),
            line_number=202,
        ),
        GrammarRule(
            name='block_item',
            body=
            Alternation(choices=[
                RuleReference(name='at_rule', is_token=False),
                RuleReference(name='declaration_or_nested', is_token=False),
            ]),
            line_number=211,
        ),
        GrammarRule(
            name='declaration_or_nested',
            body=
            Alternation(choices=[
                RuleReference(name='declaration', is_token=False),
                RuleReference(name='qualified_rule', is_token=False),
            ]),
            line_number=217,
        ),
        GrammarRule(
            name='declaration',
            body=
            Sequence(elements=[
                RuleReference(name='property', is_token=False),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='value_list', is_token=False),
                Optional(element=
                    RuleReference(name='priority', is_token=False),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=231,
        ),
        GrammarRule(
            name='property',
            body=
            Alternation(choices=[
                RuleReference(name='IDENT', is_token=True),
                RuleReference(name='CUSTOM_PROPERTY', is_token=True),
            ]),
            line_number=233,
        ),
        GrammarRule(
            name='priority',
            body=
            Sequence(elements=[
                RuleReference(name='BANG', is_token=True),
                Literal(value='important'),
            ]),
            line_number=238,
        ),
        GrammarRule(
            name='value_list',
            body=
            Sequence(elements=[
                RuleReference(name='value', is_token=False),
                Repetition(element=
                    RuleReference(name='value', is_token=False),
                ),
            ]),
            line_number=251,
        ),
        GrammarRule(
            name='value',
            body=
            Alternation(choices=[
                RuleReference(name='DIMENSION', is_token=True),
                RuleReference(name='PERCENTAGE', is_token=True),
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='IDENT', is_token=True),
                RuleReference(name='HASH', is_token=True),
                RuleReference(name='CUSTOM_PROPERTY', is_token=True),
                RuleReference(name='UNICODE_RANGE', is_token=True),
                RuleReference(name='function_call', is_token=False),
                RuleReference(name='SLASH', is_token=True),
                RuleReference(name='COMMA', is_token=True),
                RuleReference(name='PLUS', is_token=True),
                RuleReference(name='MINUS', is_token=True),
            ]),
            line_number=253,
        ),
        GrammarRule(
            name='function_call',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='FUNCTION', is_token=True),
                    RuleReference(name='function_args', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                RuleReference(name='URL_TOKEN', is_token=True),
            ]),
            line_number=267,
        ),
        GrammarRule(
            name='function_args',
            body=
            Repetition(element=
                RuleReference(name='function_arg', is_token=False),
            ),
            line_number=272,
        ),
        GrammarRule(
            name='function_arg',
            body=
            Alternation(choices=[
                RuleReference(name='DIMENSION', is_token=True),
                RuleReference(name='PERCENTAGE', is_token=True),
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='IDENT', is_token=True),
                RuleReference(name='HASH', is_token=True),
                RuleReference(name='CUSTOM_PROPERTY', is_token=True),
                RuleReference(name='COMMA', is_token=True),
                RuleReference(name='SLASH', is_token=True),
                RuleReference(name='PLUS', is_token=True),
                RuleReference(name='MINUS', is_token=True),
                RuleReference(name='STAR', is_token=True),
                Sequence(elements=[
                    RuleReference(name='FUNCTION', is_token=True),
                    RuleReference(name='function_args', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
            ]),
            line_number=274,
        ),
    ],
)
