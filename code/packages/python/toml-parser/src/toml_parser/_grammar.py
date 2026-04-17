# AUTO-GENERATED FILE — DO NOT EDIT
# Source: toml.grammar
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
            name='document',
            body=
            Repetition(element=
                Alternation(choices=[
                    RuleReference(name='NEWLINE', is_token=True),
                    RuleReference(name='expression', is_token=False),
                ]),
            ),
            line_number=38,
        ),
        GrammarRule(
            name='expression',
            body=
            Alternation(choices=[
                RuleReference(name='array_table_header', is_token=False),
                RuleReference(name='table_header', is_token=False),
                RuleReference(name='keyval', is_token=False),
            ]),
            line_number=49,
        ),
        GrammarRule(
            name='keyval',
            body=
            Sequence(elements=[
                RuleReference(name='key', is_token=False),
                RuleReference(name='EQUALS', is_token=True),
                RuleReference(name='value', is_token=False),
            ]),
            line_number=57,
        ),
        GrammarRule(
            name='key',
            body=
            Sequence(elements=[
                RuleReference(name='simple_key', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='DOT', is_token=True),
                        RuleReference(name='simple_key', is_token=False),
                    ]),
                ),
            ]),
            line_number=65,
        ),
        GrammarRule(
            name='simple_key',
            body=
            Alternation(choices=[
                RuleReference(name='BARE_KEY', is_token=True),
                RuleReference(name='BASIC_STRING', is_token=True),
                RuleReference(name='LITERAL_STRING', is_token=True),
                RuleReference(name='TRUE', is_token=True),
                RuleReference(name='FALSE', is_token=True),
                RuleReference(name='INTEGER', is_token=True),
                RuleReference(name='FLOAT', is_token=True),
                RuleReference(name='OFFSET_DATETIME', is_token=True),
                RuleReference(name='LOCAL_DATETIME', is_token=True),
                RuleReference(name='LOCAL_DATE', is_token=True),
                RuleReference(name='LOCAL_TIME', is_token=True),
            ]),
            line_number=82,
        ),
        GrammarRule(
            name='table_header',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACKET', is_token=True),
                RuleReference(name='key', is_token=False),
                RuleReference(name='RBRACKET', is_token=True),
            ]),
            line_number=92,
        ),
        GrammarRule(
            name='array_table_header',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACKET', is_token=True),
                RuleReference(name='LBRACKET', is_token=True),
                RuleReference(name='key', is_token=False),
                RuleReference(name='RBRACKET', is_token=True),
                RuleReference(name='RBRACKET', is_token=True),
            ]),
            line_number=104,
        ),
        GrammarRule(
            name='value',
            body=
            Alternation(choices=[
                RuleReference(name='BASIC_STRING', is_token=True),
                RuleReference(name='ML_BASIC_STRING', is_token=True),
                RuleReference(name='LITERAL_STRING', is_token=True),
                RuleReference(name='ML_LITERAL_STRING', is_token=True),
                RuleReference(name='INTEGER', is_token=True),
                RuleReference(name='FLOAT', is_token=True),
                RuleReference(name='TRUE', is_token=True),
                RuleReference(name='FALSE', is_token=True),
                RuleReference(name='OFFSET_DATETIME', is_token=True),
                RuleReference(name='LOCAL_DATETIME', is_token=True),
                RuleReference(name='LOCAL_DATE', is_token=True),
                RuleReference(name='LOCAL_TIME', is_token=True),
                RuleReference(name='array', is_token=False),
                RuleReference(name='inline_table', is_token=False),
            ]),
            line_number=121,
        ),
        GrammarRule(
            name='array',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACKET', is_token=True),
                RuleReference(name='array_values', is_token=False),
                RuleReference(name='RBRACKET', is_token=True),
            ]),
            line_number=140,
        ),
        GrammarRule(
            name='array_values',
            body=
            Sequence(elements=[
                Repetition(element=
                    RuleReference(name='NEWLINE', is_token=True),
                ),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='value', is_token=False),
                        Repetition(element=
                            RuleReference(name='NEWLINE', is_token=True),
                        ),
                        Repetition(element=
                            Sequence(elements=[
                                RuleReference(name='COMMA', is_token=True),
                                Repetition(element=
                                    RuleReference(name='NEWLINE', is_token=True),
                                ),
                                RuleReference(name='value', is_token=False),
                                Repetition(element=
                                    RuleReference(name='NEWLINE', is_token=True),
                                ),
                            ]),
                        ),
                        Optional(element=
                            RuleReference(name='COMMA', is_token=True),
                        ),
                        Repetition(element=
                            RuleReference(name='NEWLINE', is_token=True),
                        ),
                    ]),
                ),
            ]),
            line_number=142,
        ),
        GrammarRule(
            name='inline_table',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACE', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='keyval', is_token=False),
                        Repetition(element=
                            Sequence(elements=[
                                RuleReference(name='COMMA', is_token=True),
                                RuleReference(name='keyval', is_token=False),
                            ]),
                        ),
                    ]),
                ),
                RuleReference(name='RBRACE', is_token=True),
            ]),
            line_number=162,
        ),
    ],
)
