# AUTO-GENERATED FILE — DO NOT EDIT
# Source: json.grammar
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
            name='value',
            body=
            Alternation(choices=[
                RuleReference(name='object', is_token=False),
                RuleReference(name='array', is_token=False),
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='TRUE', is_token=True),
                RuleReference(name='FALSE', is_token=True),
                RuleReference(name='NULL', is_token=True),
            ]),
            line_number=28,
        ),
        GrammarRule(
            name='object',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACE', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='pair', is_token=False),
                        Repetition(element=
                            Sequence(elements=[
                                RuleReference(name='COMMA', is_token=True),
                                RuleReference(name='pair', is_token=False),
                            ]),
                        ),
                    ]),
                ),
                RuleReference(name='RBRACE', is_token=True),
            ]),
            line_number=34,
        ),
        GrammarRule(
            name='pair',
            body=
            Sequence(elements=[
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='value', is_token=False),
            ]),
            line_number=38,
        ),
        GrammarRule(
            name='array',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACKET', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='value', is_token=False),
                        Repetition(element=
                            Sequence(elements=[
                                RuleReference(name='COMMA', is_token=True),
                                RuleReference(name='value', is_token=False),
                            ]),
                        ),
                    ]),
                ),
                RuleReference(name='RBRACKET', is_token=True),
            ]),
            line_number=42,
        ),
    ],
)
