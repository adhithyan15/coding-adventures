# AUTO-GENERATED FILE — DO NOT EDIT
# ruff: noqa: E501, F401
# Source: brainfuck.grammar
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
                RuleReference(name='instruction', is_token=False),
            ),
            line_number=15,
        ),
        GrammarRule(
            name='instruction',
            body=
            Alternation(choices=[
                RuleReference(name='loop', is_token=False),
                RuleReference(name='command', is_token=False),
            ]),
            line_number=21,
        ),
        GrammarRule(
            name='loop',
            body=
            Sequence(elements=[
                RuleReference(name='LOOP_START', is_token=True),
                Repetition(element=
                    RuleReference(name='instruction', is_token=False),
                ),
                RuleReference(name='LOOP_END', is_token=True),
            ]),
            line_number=27,
        ),
        GrammarRule(
            name='command',
            body=
            Alternation(choices=[
                RuleReference(name='RIGHT', is_token=True),
                RuleReference(name='LEFT', is_token=True),
                RuleReference(name='INC', is_token=True),
                RuleReference(name='DEC', is_token=True),
                RuleReference(name='OUTPUT', is_token=True),
                RuleReference(name='INPUT', is_token=True),
            ]),
            line_number=32,
        ),
    ],
)
