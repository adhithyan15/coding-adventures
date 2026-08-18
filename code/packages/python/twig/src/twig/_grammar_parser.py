# AUTO-GENERATED FILE — DO NOT EDIT
# ruff: noqa: E501, F401
# Source: twig.grammar
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
            Sequence(elements=[
                Optional(element=
                    RuleReference(name='module_form', is_token=False),
                ),
                Repetition(element=
                    RuleReference(name='form', is_token=False),
                ),
            ]),
            line_number=35,
        ),
        GrammarRule(
            name='module_form',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                Literal(value='module'),
                RuleReference(name='NAME', is_token=True),
                Repetition(element=
                    RuleReference(name='module_clause', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=49,
        ),
        GrammarRule(
            name='module_clause',
            body=
            Alternation(choices=[
                RuleReference(name='export_clause', is_token=False),
                RuleReference(name='import_clause', is_token=False),
                RuleReference(name='typed_clause', is_token=False),
            ]),
            line_number=50,
        ),
        GrammarRule(
            name='export_clause',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                Literal(value='export'),
                Repetition(element=
                    RuleReference(name='NAME', is_token=True),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=51,
        ),
        GrammarRule(
            name='import_clause',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                Literal(value='import'),
                Repetition(element=
                    RuleReference(name='NAME', is_token=True),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=52,
        ),
        GrammarRule(
            name='typed_clause',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                Literal(value='typed'),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=57,
        ),
        GrammarRule(
            name='form',
            body=
            Alternation(choices=[
                RuleReference(name='define', is_token=False),
                RuleReference(name='type_alias', is_token=False),
                RuleReference(name='record_def', is_token=False),
                RuleReference(name='union_def', is_token=False),
                RuleReference(name='expr', is_token=False),
            ]),
            line_number=65,
        ),
        GrammarRule(
            name='type_alias',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                Literal(value='type'),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='type_annotation', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=69,
        ),
        GrammarRule(
            name='record_def',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                Literal(value='record'),
                RuleReference(name='NAME', is_token=True),
                Repetition(element=
                    RuleReference(name='record_field', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=73,
        ),
        GrammarRule(
            name='record_field',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='type_annotation', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=74,
        ),
        GrammarRule(
            name='union_def',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                Literal(value='union'),
                RuleReference(name='NAME', is_token=True),
                Repetition(element=
                    RuleReference(name='union_variant', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=78,
        ),
        GrammarRule(
            name='union_variant',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='NAME', is_token=True),
                Repetition(element=
                    RuleReference(name='record_field', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=79,
        ),
        GrammarRule(
            name='define',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                Literal(value='define'),
                RuleReference(name='name_or_signature', is_token=False),
                RuleReference(name='expr', is_token=False),
                Repetition(element=
                    RuleReference(name='expr', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=103,
        ),
        GrammarRule(
            name='name_or_signature',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='NAME', is_token=True),
                    Optional(element=
                        Sequence(elements=[
                            RuleReference(name='COLON', is_token=True),
                            RuleReference(name='type_annotation', is_token=False),
                        ]),
                    ),
                ]),
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='NAME', is_token=True),
                    Repetition(element=
                        RuleReference(name='typed_param', is_token=False),
                    ),
                    Optional(element=
                        Sequence(elements=[
                            RuleReference(name='ARROW', is_token=True),
                            RuleReference(name='type_annotation', is_token=False),
                        ]),
                    ),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
            ]),
            line_number=105,
        ),
        GrammarRule(
            name='typed_param',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='NAME', is_token=True),
                    RuleReference(name='COLON', is_token=True),
                    RuleReference(name='type_annotation', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=110,
        ),
        GrammarRule(
            name='type_annotation',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    Repetition(element=
                        RuleReference(name='type_annotation', is_token=False),
                    ),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='INTEGER', is_token=True),
            ]),
            line_number=134,
        ),
        GrammarRule(
            name='expr',
            body=
            Alternation(choices=[
                RuleReference(name='atom', is_token=False),
                RuleReference(name='quoted', is_token=False),
                RuleReference(name='compound', is_token=False),
            ]),
            line_number=140,
        ),
        GrammarRule(
            name='atom',
            body=
            Alternation(choices=[
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='INTEGER', is_token=True),
                RuleReference(name='BOOL_TRUE', is_token=True),
                RuleReference(name='BOOL_FALSE', is_token=True),
                Literal(value='nil'),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=142,
        ),
        GrammarRule(
            name='quoted',
            body=
            Sequence(elements=[
                RuleReference(name='QUOTE', is_token=True),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=144,
        ),
        GrammarRule(
            name='compound',
            body=
            Alternation(choices=[
                RuleReference(name='if_form', is_token=False),
                RuleReference(name='let_form', is_token=False),
                RuleReference(name='let_star_form', is_token=False),
                RuleReference(name='begin_form', is_token=False),
                RuleReference(name='lambda_form', is_token=False),
                RuleReference(name='quote_form', is_token=False),
                RuleReference(name='match_form', is_token=False),
                RuleReference(name='apply', is_token=False),
            ]),
            line_number=149,
        ),
        GrammarRule(
            name='if_form',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                Literal(value='if'),
                RuleReference(name='expr', is_token=False),
                RuleReference(name='expr', is_token=False),
                RuleReference(name='expr', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=158,
        ),
        GrammarRule(
            name='let_form',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                Literal(value='let'),
                RuleReference(name='LPAREN', is_token=True),
                Repetition(element=
                    RuleReference(name='binding', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
                RuleReference(name='expr', is_token=False),
                Repetition(element=
                    RuleReference(name='expr', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=162,
        ),
        GrammarRule(
            name='binding',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='expr', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=163,
        ),
        GrammarRule(
            name='let_star_form',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                Literal(value='let*'),
                RuleReference(name='LPAREN', is_token=True),
                Repetition(element=
                    RuleReference(name='binding', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
                RuleReference(name='expr', is_token=False),
                Repetition(element=
                    RuleReference(name='expr', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=172,
        ),
        GrammarRule(
            name='begin_form',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                Literal(value='begin'),
                RuleReference(name='expr', is_token=False),
                Repetition(element=
                    RuleReference(name='expr', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=175,
        ),
        GrammarRule(
            name='lambda_form',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                Literal(value='lambda'),
                RuleReference(name='LPAREN', is_token=True),
                Repetition(element=
                    RuleReference(name='NAME', is_token=True),
                ),
                RuleReference(name='RPAREN', is_token=True),
                RuleReference(name='expr', is_token=False),
                Repetition(element=
                    RuleReference(name='expr', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=179,
        ),
        GrammarRule(
            name='quote_form',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                Literal(value='quote'),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=182,
        ),
        GrammarRule(
            name='match_form',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                Literal(value='match'),
                RuleReference(name='expr', is_token=False),
                Repetition(element=
                    RuleReference(name='match_arm', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=196,
        ),
        GrammarRule(
            name='match_arm',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='match_pat', is_token=False),
                RuleReference(name='expr', is_token=False),
                Repetition(element=
                    RuleReference(name='expr', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=197,
        ),
        GrammarRule(
            name='match_pat',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='NAME', is_token=True),
                    Repetition(element=
                        RuleReference(name='NAME', is_token=True),
                    ),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=198,
        ),
        GrammarRule(
            name='apply',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='expr', is_token=False),
                Repetition(element=
                    RuleReference(name='expr', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=204,
        ),
    ],
)
