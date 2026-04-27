# AUTO-GENERATED FILE — DO NOT EDIT
# Source: lisp.grammar
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
            name='program',
            body=
            Repetition(element=
                RuleReference(name='sexpr', is_token=False),
            ),
            line_number=2,
        ),
        GrammarRule(
            name='sexpr',
            body=
            Alternation(choices=[
                RuleReference(name='atom', is_token=False),
                RuleReference(name='list', is_token=False),
                RuleReference(name='quoted', is_token=False),
            ]),
            line_number=3,
        ),
        GrammarRule(
            name='atom',
            body=
            Alternation(choices=[
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='SYMBOL', is_token=True),
                RuleReference(name='STRING', is_token=True),
            ]),
            line_number=4,
        ),
        GrammarRule(
            name='list',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='list_body', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=5,
        ),
        GrammarRule(
            name='list_body',
            body=
            Optional(element=
                Sequence(elements=[
                    RuleReference(name='sexpr', is_token=False),
                    Repetition(element=
                        RuleReference(name='sexpr', is_token=False),
                    ),
                    Optional(element=
                        Sequence(elements=[
                            RuleReference(name='DOT', is_token=True),
                            RuleReference(name='sexpr', is_token=False),
                        ]),
                    ),
                ]),
            ),
            line_number=6,
        ),
        GrammarRule(
            name='quoted',
            body=
            Sequence(elements=[
                RuleReference(name='QUOTE', is_token=True),
                RuleReference(name='sexpr', is_token=False),
            ]),
            line_number=7,
        ),
    ],
)
