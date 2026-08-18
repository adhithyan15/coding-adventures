# AUTO-GENERATED FILE — DO NOT EDIT
# ruff: noqa: E501, F401
# Source: dartmouth_basic.grammar
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
                RuleReference(name='line', is_token=False),
            ),
            line_number=70,
        ),
        GrammarRule(
            name='line',
            body=
            Sequence(elements=[
                RuleReference(name='LINE_NUM', is_token=True),
                Optional(element=
                    RuleReference(name='statement', is_token=False),
                ),
                RuleReference(name='NEWLINE', is_token=True),
            ]),
            line_number=81,
        ),
        GrammarRule(
            name='statement',
            body=
            Alternation(choices=[
                RuleReference(name='let_stmt', is_token=False),
                RuleReference(name='print_stmt', is_token=False),
                RuleReference(name='input_stmt', is_token=False),
                RuleReference(name='if_stmt', is_token=False),
                RuleReference(name='goto_stmt', is_token=False),
                RuleReference(name='gosub_stmt', is_token=False),
                RuleReference(name='return_stmt', is_token=False),
                RuleReference(name='for_stmt', is_token=False),
                RuleReference(name='next_stmt', is_token=False),
                RuleReference(name='end_stmt', is_token=False),
                RuleReference(name='stop_stmt', is_token=False),
                RuleReference(name='rem_stmt', is_token=False),
                RuleReference(name='read_stmt', is_token=False),
                RuleReference(name='data_stmt', is_token=False),
                RuleReference(name='restore_stmt', is_token=False),
                RuleReference(name='dim_stmt', is_token=False),
                RuleReference(name='def_stmt', is_token=False),
            ]),
            line_number=91,
        ),
        GrammarRule(
            name='let_stmt',
            body=
            Sequence(elements=[
                Literal(value='LET'),
                RuleReference(name='variable', is_token=False),
                RuleReference(name='EQ', is_token=True),
                RuleReference(name='expr', is_token=False),
            ]),
            line_number=121,
        ),
        GrammarRule(
            name='print_stmt',
            body=
            Sequence(elements=[
                Literal(value='PRINT'),
                Optional(element=
                    RuleReference(name='print_list', is_token=False),
                ),
            ]),
            line_number=137,
        ),
        GrammarRule(
            name='print_list',
            body=
            Sequence(elements=[
                RuleReference(name='print_item', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='print_sep', is_token=False),
                        RuleReference(name='print_item', is_token=False),
                    ]),
                ),
                Optional(element=
                    RuleReference(name='print_sep', is_token=False),
                ),
            ]),
            line_number=139,
        ),
        GrammarRule(
            name='print_item',
            body=
            Alternation(choices=[
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='expr', is_token=False),
            ]),
            line_number=141,
        ),
        GrammarRule(
            name='print_sep',
            body=
            Alternation(choices=[
                RuleReference(name='COMMA', is_token=True),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=143,
        ),
        GrammarRule(
            name='input_stmt',
            body=
            Sequence(elements=[
                Literal(value='INPUT'),
                RuleReference(name='variable', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='variable', is_token=False),
                    ]),
                ),
            ]),
            line_number=155,
        ),
        GrammarRule(
            name='if_stmt',
            body=
            Sequence(elements=[
                Literal(value='IF'),
                RuleReference(name='expr', is_token=False),
                RuleReference(name='relop', is_token=False),
                RuleReference(name='expr', is_token=False),
                Literal(value='THEN'),
                RuleReference(name='NUMBER', is_token=True),
            ]),
            line_number=170,
        ),
        GrammarRule(
            name='relop',
            body=
            Alternation(choices=[
                RuleReference(name='EQ', is_token=True),
                RuleReference(name='LT', is_token=True),
                RuleReference(name='GT', is_token=True),
                RuleReference(name='LE', is_token=True),
                RuleReference(name='GE', is_token=True),
                RuleReference(name='NE', is_token=True),
            ]),
            line_number=172,
        ),
        GrammarRule(
            name='goto_stmt',
            body=
            Sequence(elements=[
                Literal(value='GOTO'),
                RuleReference(name='NUMBER', is_token=True),
            ]),
            line_number=183,
        ),
        GrammarRule(
            name='gosub_stmt',
            body=
            Sequence(elements=[
                Literal(value='GOSUB'),
                RuleReference(name='NUMBER', is_token=True),
            ]),
            line_number=198,
        ),
        GrammarRule(
            name='return_stmt',
            body=
            Literal(value='RETURN'),
            line_number=200,
        ),
        GrammarRule(
            name='for_stmt',
            body=
            Sequence(elements=[
                Literal(value='FOR'),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='EQ', is_token=True),
                RuleReference(name='expr', is_token=False),
                Literal(value='TO'),
                RuleReference(name='expr', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='STEP'),
                        RuleReference(name='expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=222,
        ),
        GrammarRule(
            name='next_stmt',
            body=
            Sequence(elements=[
                Literal(value='NEXT'),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=224,
        ),
        GrammarRule(
            name='end_stmt',
            body=
            Literal(value='END'),
            line_number=233,
        ),
        GrammarRule(
            name='stop_stmt',
            body=
            Literal(value='STOP'),
            line_number=234,
        ),
        GrammarRule(
            name='rem_stmt',
            body=
            Literal(value='REM'),
            line_number=247,
        ),
        GrammarRule(
            name='read_stmt',
            body=
            Sequence(elements=[
                Literal(value='READ'),
                RuleReference(name='variable', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='variable', is_token=False),
                    ]),
                ),
            ]),
            line_number=263,
        ),
        GrammarRule(
            name='data_stmt',
            body=
            Sequence(elements=[
                Literal(value='DATA'),
                RuleReference(name='NUMBER', is_token=True),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='NUMBER', is_token=True),
                    ]),
                ),
            ]),
            line_number=265,
        ),
        GrammarRule(
            name='restore_stmt',
            body=
            Literal(value='RESTORE'),
            line_number=267,
        ),
        GrammarRule(
            name='dim_stmt',
            body=
            Sequence(elements=[
                Literal(value='DIM'),
                RuleReference(name='dim_decl', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='dim_decl', is_token=False),
                    ]),
                ),
            ]),
            line_number=280,
        ),
        GrammarRule(
            name='dim_decl',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='NUMBER', is_token=True),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='NUMBER', is_token=True),
                    ]),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=282,
        ),
        GrammarRule(
            name='def_stmt',
            body=
            Sequence(elements=[
                Literal(value='DEF'),
                RuleReference(name='USER_FN', is_token=True),
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='RPAREN', is_token=True),
                RuleReference(name='EQ', is_token=True),
                RuleReference(name='expr', is_token=False),
            ]),
            line_number=295,
        ),
        GrammarRule(
            name='variable',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='NAME', is_token=True),
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expr', is_token=False),
                    Repetition(element=
                        Sequence(elements=[
                            RuleReference(name='COMMA', is_token=True),
                            RuleReference(name='expr', is_token=False),
                        ]),
                    ),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=312,
        ),
        GrammarRule(
            name='expr',
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
            line_number=335,
        ),
        GrammarRule(
            name='term',
            body=
            Sequence(elements=[
                RuleReference(name='power', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='STAR', is_token=True),
                                RuleReference(name='SLASH', is_token=True),
                            ]),
                        ),
                        RuleReference(name='power', is_token=False),
                    ]),
                ),
            ]),
            line_number=337,
        ),
        GrammarRule(
            name='power',
            body=
            Sequence(elements=[
                RuleReference(name='unary', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='CARET', is_token=True),
                        RuleReference(name='power', is_token=False),
                    ]),
                ),
            ]),
            line_number=343,
        ),
        GrammarRule(
            name='unary',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='MINUS', is_token=True),
                    RuleReference(name='primary', is_token=False),
                ]),
                RuleReference(name='primary', is_token=False),
            ]),
            line_number=348,
        ),
        GrammarRule(
            name='primary',
            body=
            Alternation(choices=[
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='STRING', is_token=True),
                Sequence(elements=[
                    RuleReference(name='BUILTIN_FN', is_token=True),
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expr', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='USER_FN', is_token=True),
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expr', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                RuleReference(name='variable', is_token=False),
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expr', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
            ]),
            line_number=366,
        ),
    ],
)
