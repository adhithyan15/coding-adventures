# AUTO-GENERATED FILE — DO NOT EDIT
# ruff: noqa: E501, F401
# Source: iso.grammar
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
            line_number=16,
        ),
        GrammarRule(
            name='statement',
            body=
            Alternation(choices=[
                RuleReference(name='query_statement', is_token=False),
                RuleReference(name='dcg_statement', is_token=False),
                RuleReference(name='probabilistic_rule_statement', is_token=False),
                RuleReference(name='probabilistic_fact_statement', is_token=False),
                RuleReference(name='rule_statement', is_token=False),
                RuleReference(name='fact_statement', is_token=False),
            ]),
            line_number=18,
        ),
        GrammarRule(
            name='query_statement',
            body=
            Sequence(elements=[
                RuleReference(name='QUERY', is_token=True),
                RuleReference(name='goal', is_token=False),
                RuleReference(name='DOT', is_token=True),
            ]),
            line_number=21,
        ),
        GrammarRule(
            name='dcg_statement',
            body=
            Sequence(elements=[
                RuleReference(name='callable_term', is_token=False),
                RuleReference(name='DCG', is_token=True),
                RuleReference(name='dcg_body', is_token=False),
                RuleReference(name='DOT', is_token=True),
            ]),
            line_number=22,
        ),
        GrammarRule(
            name='rule_statement',
            body=
            Sequence(elements=[
                RuleReference(name='callable_term', is_token=False),
                RuleReference(name='RULE', is_token=True),
                RuleReference(name='goal', is_token=False),
                RuleReference(name='DOT', is_token=True),
            ]),
            line_number=23,
        ),
        GrammarRule(
            name='fact_statement',
            body=
            Sequence(elements=[
                RuleReference(name='callable_term', is_token=False),
                RuleReference(name='DOT', is_token=True),
            ]),
            line_number=24,
        ),
        GrammarRule(
            name='probabilistic_fact_statement',
            body=
            Sequence(elements=[
                RuleReference(name='probability', is_token=False),
                RuleReference(name='PROB_SEP', is_token=True),
                RuleReference(name='callable_term', is_token=False),
                RuleReference(name='DOT', is_token=True),
            ]),
            line_number=31,
        ),
        GrammarRule(
            name='probabilistic_rule_statement',
            body=
            Sequence(elements=[
                RuleReference(name='probability', is_token=False),
                RuleReference(name='PROB_SEP', is_token=True),
                RuleReference(name='callable_term', is_token=False),
                RuleReference(name='RULE', is_token=True),
                RuleReference(name='goal', is_token=False),
                RuleReference(name='DOT', is_token=True),
            ]),
            line_number=32,
        ),
        GrammarRule(
            name='probability',
            body=
            Alternation(choices=[
                RuleReference(name='FLOAT', is_token=True),
                RuleReference(name='INTEGER', is_token=True),
            ]),
            line_number=33,
        ),
        GrammarRule(
            name='goal',
            body=
            RuleReference(name='disjunction', is_token=False),
            line_number=35,
        ),
        GrammarRule(
            name='disjunction',
            body=
            Sequence(elements=[
                RuleReference(name='conjunction', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='SEMICOLON', is_token=True),
                        RuleReference(name='conjunction', is_token=False),
                    ]),
                ),
            ]),
            line_number=36,
        ),
        GrammarRule(
            name='conjunction',
            body=
            Sequence(elements=[
                RuleReference(name='goal_primary', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='goal_primary', is_token=False),
                    ]),
                ),
            ]),
            line_number=37,
        ),
        GrammarRule(
            name='goal_primary',
            body=
            Alternation(choices=[
                RuleReference(name='CUT', is_token=True),
                RuleReference(name='grouped_goal', is_token=False),
                RuleReference(name='naf_goal', is_token=False),
                RuleReference(name='equality_goal', is_token=False),
                RuleReference(name='callable_goal', is_token=False),
            ]),
            line_number=38,
        ),
        GrammarRule(
            name='grouped_goal',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='goal', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=39,
        ),
        GrammarRule(
            name='naf_goal',
            body=
            Sequence(elements=[
                RuleReference(name='NAF', is_token=True),
                RuleReference(name='goal_primary', is_token=False),
            ]),
            line_number=40,
        ),
        GrammarRule(
            name='equality_goal',
            body=
            Sequence(elements=[
                RuleReference(name='term', is_token=False),
                RuleReference(name='equality_operator', is_token=False),
                RuleReference(name='term', is_token=False),
            ]),
            line_number=41,
        ),
        GrammarRule(
            name='callable_goal',
            body=
            RuleReference(name='callable_term', is_token=False),
            line_number=42,
        ),
        GrammarRule(
            name='dcg_body',
            body=
            RuleReference(name='dcg_disjunction', is_token=False),
            line_number=44,
        ),
        GrammarRule(
            name='dcg_disjunction',
            body=
            Sequence(elements=[
                RuleReference(name='dcg_conjunction', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='SEMICOLON', is_token=True),
                        RuleReference(name='dcg_conjunction', is_token=False),
                    ]),
                ),
            ]),
            line_number=45,
        ),
        GrammarRule(
            name='dcg_conjunction',
            body=
            Sequence(elements=[
                RuleReference(name='dcg_primary', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='dcg_primary', is_token=False),
                    ]),
                ),
            ]),
            line_number=46,
        ),
        GrammarRule(
            name='dcg_primary',
            body=
            Alternation(choices=[
                RuleReference(name='CUT', is_token=True),
                RuleReference(name='grouped_dcg_body', is_token=False),
                RuleReference(name='braced_goal', is_token=False),
                RuleReference(name='equality_goal', is_token=False),
                RuleReference(name='list_term', is_token=False),
                RuleReference(name='callable_goal', is_token=False),
            ]),
            line_number=47,
        ),
        GrammarRule(
            name='grouped_dcg_body',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='dcg_body', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=49,
        ),
        GrammarRule(
            name='braced_goal',
            body=
            Sequence(elements=[
                RuleReference(name='LCURLY', is_token=True),
                RuleReference(name='goal', is_token=False),
                RuleReference(name='RCURLY', is_token=True),
            ]),
            line_number=50,
        ),
        GrammarRule(
            name='equality_operator',
            body=
            Alternation(choices=[
                Literal(value='='),
                Literal(value='\\='),
            ]),
            line_number=52,
        ),
        GrammarRule(
            name='callable_term',
            body=
            Alternation(choices=[
                RuleReference(name='compound_term', is_token=False),
                RuleReference(name='atom_term', is_token=False),
            ]),
            line_number=54,
        ),
        GrammarRule(
            name='term',
            body=
            Alternation(choices=[
                RuleReference(name='list_term', is_token=False),
                RuleReference(name='compound_term', is_token=False),
                RuleReference(name='atom_term', is_token=False),
                RuleReference(name='variable_term', is_token=False),
                RuleReference(name='anonymous_term', is_token=False),
                RuleReference(name='number_term', is_token=False),
                RuleReference(name='string_term', is_token=False),
            ]),
            line_number=55,
        ),
        GrammarRule(
            name='compound_term',
            body=
            Sequence(elements=[
                RuleReference(name='atom_token', is_token=False),
                RuleReference(name='LPAREN', is_token=True),
                Optional(element=
                    RuleReference(name='term_arguments', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=58,
        ),
        GrammarRule(
            name='term_arguments',
            body=
            Sequence(elements=[
                RuleReference(name='term', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='term', is_token=False),
                    ]),
                ),
            ]),
            line_number=59,
        ),
        GrammarRule(
            name='list_term',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACKET', is_token=True),
                Optional(element=
                    RuleReference(name='list_body', is_token=False),
                ),
                RuleReference(name='RBRACKET', is_token=True),
            ]),
            line_number=61,
        ),
        GrammarRule(
            name='list_body',
            body=
            Sequence(elements=[
                RuleReference(name='term', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='term', is_token=False),
                    ]),
                ),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='BAR', is_token=True),
                        RuleReference(name='term', is_token=False),
                    ]),
                ),
            ]),
            line_number=62,
        ),
        GrammarRule(
            name='atom_term',
            body=
            RuleReference(name='atom_token', is_token=False),
            line_number=64,
        ),
        GrammarRule(
            name='atom_token',
            body=
            RuleReference(name='ATOM', is_token=True),
            line_number=65,
        ),
        GrammarRule(
            name='variable_term',
            body=
            RuleReference(name='VARIABLE', is_token=True),
            line_number=66,
        ),
        GrammarRule(
            name='anonymous_term',
            body=
            RuleReference(name='ANON_VAR', is_token=True),
            line_number=67,
        ),
        GrammarRule(
            name='number_term',
            body=
            Alternation(choices=[
                RuleReference(name='FLOAT', is_token=True),
                RuleReference(name='INTEGER', is_token=True),
            ]),
            line_number=68,
        ),
        GrammarRule(
            name='string_term',
            body=
            RuleReference(name='STRING', is_token=True),
            line_number=69,
        ),
    ],
)
