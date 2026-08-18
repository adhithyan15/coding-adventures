# AUTO-GENERATED FILE — DO NOT EDIT
# ruff: noqa: E501, F401
# Source: macsyma.grammar
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
    version=2,
    rules=[
        GrammarRule(
            name='program',
            body=
            Repetition(element=
                RuleReference(name='statement', is_token=False),
            ),
            line_number=31,
        ),
        GrammarRule(
            name='statement',
            body=
            Sequence(elements=[
                RuleReference(name='expression', is_token=False),
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='SEMI', is_token=True),
                        RuleReference(name='DOLLAR', is_token=True),
                    ]),
                ),
            ]),
            line_number=33,
        ),
        GrammarRule(
            name='expression',
            body=
            Alternation(choices=[
                RuleReference(name='if_expr', is_token=False),
                RuleReference(name='for_expr', is_token=False),
                RuleReference(name='while_expr', is_token=False),
                RuleReference(name='block_expr', is_token=False),
                RuleReference(name='return_expr', is_token=False),
                RuleReference(name='assign', is_token=False),
            ]),
            line_number=44,
        ),
        GrammarRule(
            name='if_expr',
            body=
            Sequence(elements=[
                Literal(value='if'),
                RuleReference(name='expression', is_token=False),
                Literal(value='then'),
                RuleReference(name='expression', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='elseif'),
                        RuleReference(name='expression', is_token=False),
                        Literal(value='then'),
                        RuleReference(name='expression', is_token=False),
                    ]),
                ),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='else'),
                        RuleReference(name='expression', is_token=False),
                    ]),
                ),
            ]),
            line_number=54,
        ),
        GrammarRule(
            name='for_expr',
            body=
            Alternation(choices=[
                RuleReference(name='for_each_expr', is_token=False),
                RuleReference(name='for_range_expr', is_token=False),
            ]),
            line_number=67,
        ),
        GrammarRule(
            name='for_each_expr',
            body=
            Sequence(elements=[
                Literal(value='for'),
                RuleReference(name='NAME', is_token=True),
                Literal(value='in'),
                RuleReference(name='expression', is_token=False),
                Literal(value='do'),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=69,
        ),
        GrammarRule(
            name='for_range_expr',
            body=
            Sequence(elements=[
                Literal(value='for'),
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        Literal(value=':'),
                        RuleReference(name='expression', is_token=False),
                    ]),
                ),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='step'),
                        RuleReference(name='expression', is_token=False),
                    ]),
                ),
                Group(element=
                    Alternation(choices=[
                        Literal(value='thru'),
                        Literal(value='while'),
                        Literal(value='unless'),
                    ]),
                ),
                RuleReference(name='expression', is_token=False),
                Literal(value='do'),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=71,
        ),
        GrammarRule(
            name='while_expr',
            body=
            Sequence(elements=[
                Literal(value='while'),
                RuleReference(name='expression', is_token=False),
                Literal(value='do'),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=76,
        ),
        GrammarRule(
            name='block_expr',
            body=
            Sequence(elements=[
                Literal(value='block'),
                Literal(value='('),
                Optional(element=
                    RuleReference(name='arglist', is_token=False),
                ),
                Literal(value=')'),
            ]),
            line_number=82,
        ),
        GrammarRule(
            name='return_expr',
            body=
            Sequence(elements=[
                Literal(value='return'),
                Literal(value='('),
                RuleReference(name='expression', is_token=False),
                Literal(value=')'),
            ]),
            line_number=87,
        ),
        GrammarRule(
            name='assign',
            body=
            Sequence(elements=[
                RuleReference(name='logical_or', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='COLON', is_token=True),
                                RuleReference(name='COLONEQ', is_token=True),
                            ]),
                        ),
                        RuleReference(name='assign', is_token=False),
                    ]),
                ),
            ]),
            line_number=92,
        ),
        GrammarRule(
            name='logical_or',
            body=
            Sequence(elements=[
                RuleReference(name='logical_and', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='or'),
                        RuleReference(name='logical_and', is_token=False),
                    ]),
                ),
            ]),
            line_number=97,
        ),
        GrammarRule(
            name='logical_and',
            body=
            Sequence(elements=[
                RuleReference(name='logical_not', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='and'),
                        RuleReference(name='logical_not', is_token=False),
                    ]),
                ),
            ]),
            line_number=98,
        ),
        GrammarRule(
            name='logical_not',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='not'),
                    RuleReference(name='logical_not', is_token=False),
                ]),
                RuleReference(name='comparison', is_token=False),
            ]),
            line_number=99,
        ),
        GrammarRule(
            name='comparison',
            body=
            Sequence(elements=[
                RuleReference(name='additive', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='EQ', is_token=True),
                                RuleReference(name='HASH', is_token=True),
                                RuleReference(name='LT', is_token=True),
                                RuleReference(name='GT', is_token=True),
                                RuleReference(name='LEQ', is_token=True),
                                RuleReference(name='GEQ', is_token=True),
                            ]),
                        ),
                        RuleReference(name='additive', is_token=False),
                    ]),
                ),
            ]),
            line_number=103,
        ),
        GrammarRule(
            name='additive',
            body=
            Sequence(elements=[
                RuleReference(name='multiplicative', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='PLUS', is_token=True),
                                RuleReference(name='MINUS', is_token=True),
                            ]),
                        ),
                        RuleReference(name='multiplicative', is_token=False),
                    ]),
                ),
            ]),
            line_number=105,
        ),
        GrammarRule(
            name='multiplicative',
            body=
            Sequence(elements=[
                RuleReference(name='unary', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='STAR', is_token=True),
                                RuleReference(name='SLASH', is_token=True),
                            ]),
                        ),
                        RuleReference(name='unary', is_token=False),
                    ]),
                ),
            ]),
            line_number=106,
        ),
        GrammarRule(
            name='unary',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Group(element=
                        Alternation(choices=[
                            RuleReference(name='MINUS', is_token=True),
                            RuleReference(name='PLUS', is_token=True),
                        ]),
                    ),
                    RuleReference(name='unary', is_token=False),
                ]),
                RuleReference(name='power', is_token=False),
            ]),
            line_number=110,
        ),
        GrammarRule(
            name='power',
            body=
            Sequence(elements=[
                RuleReference(name='postfix', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='CARET', is_token=True),
                                RuleReference(name='STAREQ', is_token=True),
                            ]),
                        ),
                        RuleReference(name='unary', is_token=False),
                    ]),
                ),
            ]),
            line_number=114,
        ),
        GrammarRule(
            name='postfix',
            body=
            Sequence(elements=[
                RuleReference(name='atom', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='LPAREN', is_token=True),
                        Optional(element=
                            RuleReference(name='arglist', is_token=False),
                        ),
                        RuleReference(name='RPAREN', is_token=True),
                    ]),
                ),
            ]),
            line_number=118,
        ),
        GrammarRule(
            name='arglist',
            body=
            Sequence(elements=[
                RuleReference(name='expression', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='expression', is_token=False),
                    ]),
                ),
            ]),
            line_number=119,
        ),
        GrammarRule(
            name='atom',
            body=
            Alternation(choices=[
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='NAME', is_token=True),
                Literal(value='true'),
                Literal(value='false'),
                RuleReference(name='group', is_token=False),
                RuleReference(name='list', is_token=False),
            ]),
            line_number=121,
        ),
        GrammarRule(
            name='group',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='expression', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=129,
        ),
        GrammarRule(
            name='list',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACKET', is_token=True),
                Optional(element=
                    RuleReference(name='arglist', is_token=False),
                ),
                RuleReference(name='RBRACKET', is_token=True),
            ]),
            line_number=130,
        ),
    ],
)
