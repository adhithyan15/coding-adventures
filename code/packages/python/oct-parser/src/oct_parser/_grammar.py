# AUTO-GENERATED FILE — DO NOT EDIT
# ruff: noqa: E501, F401
# Source: oct.grammar
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
                RuleReference(name='top_decl', is_token=False),
            ),
            line_number=48,
        ),
        GrammarRule(
            name='top_decl',
            body=
            Alternation(choices=[
                RuleReference(name='static_decl', is_token=False),
                RuleReference(name='fn_decl', is_token=False),
            ]),
            line_number=52,
        ),
        GrammarRule(
            name='static_decl',
            body=
            Sequence(elements=[
                Literal(value='static'),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='type', is_token=False),
                RuleReference(name='EQ', is_token=True),
                RuleReference(name='expr', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=69,
        ),
        GrammarRule(
            name='fn_decl',
            body=
            Sequence(elements=[
                Literal(value='fn'),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='LPAREN', is_token=True),
                Optional(element=
                    RuleReference(name='param_list', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='ARROW', is_token=True),
                        RuleReference(name='type', is_token=False),
                    ]),
                ),
                RuleReference(name='block', is_token=False),
            ]),
            line_number=88,
        ),
        GrammarRule(
            name='param_list',
            body=
            Sequence(elements=[
                RuleReference(name='param', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='param', is_token=False),
                    ]),
                ),
            ]),
            line_number=92,
        ),
        GrammarRule(
            name='param',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='type', is_token=False),
            ]),
            line_number=96,
        ),
        GrammarRule(
            name='block',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACE', is_token=True),
                Repetition(element=
                    RuleReference(name='stmt', is_token=False),
                ),
                RuleReference(name='RBRACE', is_token=True),
            ]),
            line_number=106,
        ),
        GrammarRule(
            name='stmt',
            body=
            Alternation(choices=[
                RuleReference(name='let_stmt', is_token=False),
                RuleReference(name='static_decl', is_token=False),
                RuleReference(name='assign_stmt', is_token=False),
                RuleReference(name='return_stmt', is_token=False),
                RuleReference(name='if_stmt', is_token=False),
                RuleReference(name='while_stmt', is_token=False),
                RuleReference(name='loop_stmt', is_token=False),
                RuleReference(name='break_stmt', is_token=False),
                RuleReference(name='expr_stmt', is_token=False),
            ]),
            line_number=117,
        ),
        GrammarRule(
            name='let_stmt',
            body=
            Sequence(elements=[
                Literal(value='let'),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='type', is_token=False),
                RuleReference(name='EQ', is_token=True),
                RuleReference(name='expr', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=139,
        ),
        GrammarRule(
            name='assign_stmt',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='EQ', is_token=True),
                RuleReference(name='expr', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=148,
        ),
        GrammarRule(
            name='return_stmt',
            body=
            Sequence(elements=[
                Literal(value='return'),
                Optional(element=
                    RuleReference(name='expr', is_token=False),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=157,
        ),
        GrammarRule(
            name='if_stmt',
            body=
            Sequence(elements=[
                Literal(value='if'),
                RuleReference(name='expr', is_token=False),
                RuleReference(name='block', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='else'),
                        RuleReference(name='block', is_token=False),
                    ]),
                ),
            ]),
            line_number=166,
        ),
        GrammarRule(
            name='while_stmt',
            body=
            Sequence(elements=[
                Literal(value='while'),
                RuleReference(name='expr', is_token=False),
                RuleReference(name='block', is_token=False),
            ]),
            line_number=174,
        ),
        GrammarRule(
            name='loop_stmt',
            body=
            Sequence(elements=[
                Literal(value='loop'),
                RuleReference(name='block', is_token=False),
            ]),
            line_number=181,
        ),
        GrammarRule(
            name='break_stmt',
            body=
            Sequence(elements=[
                Literal(value='break'),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=185,
        ),
        GrammarRule(
            name='expr_stmt',
            body=
            Sequence(elements=[
                RuleReference(name='expr', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=196,
        ),
        GrammarRule(
            name='type',
            body=
            RuleReference(name='NAME', is_token=True),
            line_number=230,
        ),
        GrammarRule(
            name='expr',
            body=
            RuleReference(name='or_expr', is_token=False),
            line_number=269,
        ),
        GrammarRule(
            name='or_expr',
            body=
            Sequence(elements=[
                RuleReference(name='and_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='LOR', is_token=True),
                        RuleReference(name='and_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=274,
        ),
        GrammarRule(
            name='and_expr',
            body=
            Sequence(elements=[
                RuleReference(name='eq_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='LAND', is_token=True),
                        RuleReference(name='eq_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=278,
        ),
        GrammarRule(
            name='eq_expr',
            body=
            Sequence(elements=[
                RuleReference(name='cmp_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='EQ_EQ', is_token=True),
                                RuleReference(name='NEQ', is_token=True),
                            ]),
                        ),
                        RuleReference(name='cmp_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=285,
        ),
        GrammarRule(
            name='cmp_expr',
            body=
            Sequence(elements=[
                RuleReference(name='add_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='LT', is_token=True),
                                RuleReference(name='GT', is_token=True),
                                RuleReference(name='LEQ', is_token=True),
                                RuleReference(name='GEQ', is_token=True),
                            ]),
                        ),
                        RuleReference(name='add_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=291,
        ),
        GrammarRule(
            name='add_expr',
            body=
            Sequence(elements=[
                RuleReference(name='bitwise_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='PLUS', is_token=True),
                                RuleReference(name='MINUS', is_token=True),
                            ]),
                        ),
                        RuleReference(name='bitwise_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=297,
        ),
        GrammarRule(
            name='bitwise_expr',
            body=
            Sequence(elements=[
                RuleReference(name='unary_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='AMP', is_token=True),
                                RuleReference(name='PIPE', is_token=True),
                                RuleReference(name='CARET', is_token=True),
                            ]),
                        ),
                        RuleReference(name='unary_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=302,
        ),
        GrammarRule(
            name='unary_expr',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Group(element=
                        Alternation(choices=[
                            RuleReference(name='BANG', is_token=True),
                            RuleReference(name='TILDE', is_token=True),
                        ]),
                    ),
                    RuleReference(name='unary_expr', is_token=False),
                ]),
                RuleReference(name='primary', is_token=False),
            ]),
            line_number=310,
        ),
        GrammarRule(
            name='primary',
            body=
            Alternation(choices=[
                RuleReference(name='intrinsic_call', is_token=False),
                RuleReference(name='call_expr', is_token=False),
                RuleReference(name='INT_LIT', is_token=True),
                RuleReference(name='HEX_LIT', is_token=True),
                RuleReference(name='BIN_LIT', is_token=True),
                Literal(value='true'),
                Literal(value='false'),
                RuleReference(name='NAME', is_token=True),
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expr', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
            ]),
            line_number=323,
        ),
        GrammarRule(
            name='call_expr',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='LPAREN', is_token=True),
                Optional(element=
                    RuleReference(name='arg_list', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=341,
        ),
        GrammarRule(
            name='arg_list',
            body=
            Sequence(elements=[
                RuleReference(name='expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=344,
        ),
        GrammarRule(
            name='intrinsic_call',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='in'),
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expr', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                Sequence(elements=[
                    Literal(value='out'),
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expr', is_token=False),
                    RuleReference(name='COMMA', is_token=True),
                    RuleReference(name='expr', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                Sequence(elements=[
                    Literal(value='adc'),
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expr', is_token=False),
                    RuleReference(name='COMMA', is_token=True),
                    RuleReference(name='expr', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                Sequence(elements=[
                    Literal(value='sbb'),
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expr', is_token=False),
                    RuleReference(name='COMMA', is_token=True),
                    RuleReference(name='expr', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                Sequence(elements=[
                    Literal(value='rlc'),
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expr', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                Sequence(elements=[
                    Literal(value='rrc'),
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expr', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                Sequence(elements=[
                    Literal(value='ral'),
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expr', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                Sequence(elements=[
                    Literal(value='rar'),
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expr', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                Sequence(elements=[
                    Literal(value='carry'),
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                Sequence(elements=[
                    Literal(value='parity'),
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expr', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
            ]),
            line_number=370,
        ),
    ],
)
