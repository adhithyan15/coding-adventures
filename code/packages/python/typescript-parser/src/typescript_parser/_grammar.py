# AUTO-GENERATED FILE — DO NOT EDIT
# Source: typescript.grammar
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
                RuleReference(name='statement', is_token=False),
            ),
            line_number=34,
        ),
        GrammarRule(
            name='statement',
            body=
            Alternation(choices=[
                RuleReference(name='var_declaration', is_token=False),
                RuleReference(name='assignment', is_token=False),
                RuleReference(name='expression_stmt', is_token=False),
            ]),
            line_number=35,
        ),
        GrammarRule(
            name='var_declaration',
            body=
            Sequence(elements=[
                RuleReference(name='KEYWORD', is_token=True),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='EQUALS', is_token=True),
                RuleReference(name='expression', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=36,
        ),
        GrammarRule(
            name='assignment',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='EQUALS', is_token=True),
                RuleReference(name='expression', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=37,
        ),
        GrammarRule(
            name='expression_stmt',
            body=
            Sequence(elements=[
                RuleReference(name='expression', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=38,
        ),
        GrammarRule(
            name='expression',
            body=
            Sequence(elements=[
                RuleReference(name='term', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='PLUS', is_token=True),
                                RuleReference(name='MINUS', is_token=True),
                            ]),
                        ),
                        RuleReference(name='term', is_token=False),
                    ]),
                ),
            ]),
            line_number=39,
        ),
        GrammarRule(
            name='term',
            body=
            Sequence(elements=[
                RuleReference(name='factor', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='STAR', is_token=True),
                                RuleReference(name='SLASH', is_token=True),
                            ]),
                        ),
                        RuleReference(name='factor', is_token=False),
                    ]),
                ),
            ]),
            line_number=40,
        ),
        GrammarRule(
            name='factor',
            body=
            Alternation(choices=[
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='KEYWORD', is_token=True),
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expression', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
            ]),
            line_number=41,
        ),
    ],
)
