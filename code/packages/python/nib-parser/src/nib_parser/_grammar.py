# AUTO-GENERATED FILE — DO NOT EDIT
# ruff: noqa: E501, F401
# Source: nib.grammar
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
            line_number=42,
        ),
        GrammarRule(
            name='top_decl',
            body=
            Alternation(choices=[
                RuleReference(name='const_decl', is_token=False),
                RuleReference(name='static_decl', is_token=False),
                RuleReference(name='fn_decl', is_token=False),
            ]),
            line_number=47,
        ),
        GrammarRule(
            name='const_decl',
            body=
            Sequence(elements=[
                Literal(value='const'),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='type', is_token=False),
                RuleReference(name='EQ', is_token=True),
                RuleReference(name='expr', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=60,
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
            line_number=66,
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
            line_number=77,
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
            line_number=80,
        ),
        GrammarRule(
            name='param',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='type', is_token=False),
            ]),
            line_number=87,
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
            line_number=98,
        ),
        GrammarRule(
            name='stmt',
            body=
            Alternation(choices=[
                RuleReference(name='let_stmt', is_token=False),
                RuleReference(name='assign_stmt', is_token=False),
                RuleReference(name='return_stmt', is_token=False),
                RuleReference(name='for_stmt', is_token=False),
                RuleReference(name='while_stmt', is_token=False),
                RuleReference(name='if_stmt', is_token=False),
                RuleReference(name='expr_stmt', is_token=False),
            ]),
            line_number=113,
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
            line_number=126,
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
            line_number=131,
        ),
        GrammarRule(
            name='return_stmt',
            body=
            Sequence(elements=[
                Literal(value='return'),
                RuleReference(name='expr', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=136,
        ),
        GrammarRule(
            name='for_stmt',
            body=
            Sequence(elements=[
                Literal(value='for'),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='type', is_token=False),
                Literal(value='in'),
                RuleReference(name='expr', is_token=False),
                RuleReference(name='RANGE', is_token=True),
                RuleReference(name='expr', is_token=False),
                RuleReference(name='block', is_token=False),
            ]),
            line_number=159,
        ),
        GrammarRule(
            name='while_stmt',
            body=
            Sequence(elements=[
                Literal(value='while'),
                RuleReference(name='expr', is_token=False),
                RuleReference(name='block', is_token=False),
            ]),
            line_number=170,
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
            line_number=176,
        ),
        GrammarRule(
            name='expr_stmt',
            body=
            Sequence(elements=[
                RuleReference(name='expr', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=183,
        ),
        GrammarRule(
            name='type',
            body=
            Alternation(choices=[
                Literal(value='u4'),
                Literal(value='u8'),
                Literal(value='bcd'),
                Literal(value='bool'),
            ]),
            line_number=218,
        ),
        GrammarRule(
            name='expr',
            body=
            RuleReference(name='or_expr', is_token=False),
            line_number=259,
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
            line_number=265,
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
            line_number=269,
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
            line_number=274,
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
            line_number=280,
        ),
        GrammarRule(
            name='add_expr',
            body=
            Sequence(elements=[
                RuleReference(name='shift_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='PLUS', is_token=True),
                                RuleReference(name='MINUS', is_token=True),
                                RuleReference(name='WRAP_ADD', is_token=True),
                                RuleReference(name='SAT_ADD', is_token=True),
                            ]),
                        ),
                        RuleReference(name='shift_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=293,
        ),
        GrammarRule(
            name='shift_expr',
            body=
            Sequence(elements=[
                RuleReference(name='mul_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='SHL', is_token=True),
                                RuleReference(name='SHR', is_token=True),
                            ]),
                        ),
                        RuleReference(name='mul_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=298,
        ),
        GrammarRule(
            name='mul_expr',
            body=
            Sequence(elements=[
                RuleReference(name='bitwise_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='STAR', is_token=True),
                                RuleReference(name='SLASH', is_token=True),
                                RuleReference(name='PERCENT', is_token=True),
                            ]),
                        ),
                        RuleReference(name='bitwise_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=308,
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
            line_number=314,
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
            line_number=322,
        ),
        GrammarRule(
            name='primary',
            body=
            Alternation(choices=[
                RuleReference(name='INT_LIT', is_token=True),
                RuleReference(name='HEX_LIT', is_token=True),
                Literal(value='true'),
                Literal(value='false'),
                RuleReference(name='call_expr', is_token=False),
                RuleReference(name='NAME', is_token=True),
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expr', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
            ]),
            line_number=330,
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
            line_number=353,
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
            line_number=356,
        ),
    ],
)
