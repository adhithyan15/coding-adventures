# AUTO-GENERATED FILE — DO NOT EDIT
# ruff: noqa: E501, F401
# Source: algol/algol60.grammar
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
            RuleReference(name='block', is_token=False),
            line_number=47,
        ),
        GrammarRule(
            name='block',
            body=
            Sequence(elements=[
                Literal(value='begin'),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='declaration', is_token=False),
                        RuleReference(name='SEMICOLON', is_token=True),
                    ]),
                ),
                Repetition(element=
                    RuleReference(name='SEMICOLON', is_token=True),
                ),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='statement', is_token=False),
                        Repetition(element=
                            Sequence(elements=[
                                RuleReference(name='SEMICOLON', is_token=True),
                                Optional(element=
                                    RuleReference(name='statement', is_token=False),
                                ),
                            ]),
                        ),
                    ]),
                ),
                Literal(value='end'),
            ]),
            line_number=53,
        ),
        GrammarRule(
            name='declaration',
            body=
            Alternation(choices=[
                RuleReference(name='type_decl', is_token=False),
                RuleReference(name='own_decl', is_token=False),
                RuleReference(name='own_array_decl', is_token=False),
                RuleReference(name='array_decl', is_token=False),
                RuleReference(name='switch_decl', is_token=False),
                RuleReference(name='procedure_decl', is_token=False),
            ]),
            line_number=60,
        ),
        GrammarRule(
            name='type_decl',
            body=
            Sequence(elements=[
                RuleReference(name='type', is_token=False),
                RuleReference(name='ident_list', is_token=False),
            ]),
            line_number=71,
        ),
        GrammarRule(
            name='own_decl',
            body=
            Sequence(elements=[
                Literal(value='own'),
                RuleReference(name='type', is_token=False),
                RuleReference(name='ident_list', is_token=False),
            ]),
            line_number=76,
        ),
        GrammarRule(
            name='own_array_decl',
            body=
            Sequence(elements=[
                Literal(value='own'),
                Optional(element=
                    RuleReference(name='type', is_token=False),
                ),
                Literal(value='array'),
                RuleReference(name='array_segment', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='array_segment', is_token=False),
                    ]),
                ),
            ]),
            line_number=81,
        ),
        GrammarRule(
            name='type',
            body=
            Alternation(choices=[
                Literal(value='integer'),
                Literal(value='real'),
                Literal(value='boolean'),
                Literal(value='string'),
            ]),
            line_number=83,
        ),
        GrammarRule(
            name='ident_list',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='NAME', is_token=True),
                    ]),
                ),
            ]),
            line_number=85,
        ),
        GrammarRule(
            name='array_decl',
            body=
            Sequence(elements=[
                Optional(element=
                    RuleReference(name='type', is_token=False),
                ),
                Literal(value='array'),
                RuleReference(name='array_segment', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='array_segment', is_token=False),
                    ]),
                ),
            ]),
            line_number=93,
        ),
        GrammarRule(
            name='array_segment',
            body=
            Sequence(elements=[
                RuleReference(name='ident_list', is_token=False),
                RuleReference(name='LBRACKET', is_token=True),
                RuleReference(name='bound_pair', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='bound_pair', is_token=False),
                    ]),
                ),
                RuleReference(name='RBRACKET', is_token=True),
            ]),
            line_number=95,
        ),
        GrammarRule(
            name='bound_pair',
            body=
            Sequence(elements=[
                RuleReference(name='arith_expr', is_token=False),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='arith_expr', is_token=False),
            ]),
            line_number=99,
        ),
        GrammarRule(
            name='switch_decl',
            body=
            Sequence(elements=[
                Literal(value='switch'),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='ASSIGN', is_token=True),
                RuleReference(name='switch_list', is_token=False),
            ]),
            line_number=104,
        ),
        GrammarRule(
            name='switch_list',
            body=
            Sequence(elements=[
                RuleReference(name='desig_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='desig_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=106,
        ),
        GrammarRule(
            name='procedure_decl',
            body=
            Sequence(elements=[
                Optional(element=
                    RuleReference(name='type', is_token=False),
                ),
                Literal(value='procedure'),
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    RuleReference(name='formal_params', is_token=False),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
                Optional(element=
                    RuleReference(name='value_part', is_token=False),
                ),
                Repetition(element=
                    RuleReference(name='spec_part', is_token=False),
                ),
                RuleReference(name='proc_body', is_token=False),
            ]),
            line_number=113,
        ),
        GrammarRule(
            name='formal_params',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                Optional(element=
                    RuleReference(name='ident_list', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=118,
        ),
        GrammarRule(
            name='value_part',
            body=
            Sequence(elements=[
                Literal(value='value'),
                RuleReference(name='ident_list', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=123,
        ),
        GrammarRule(
            name='spec_part',
            body=
            Sequence(elements=[
                RuleReference(name='specifier', is_token=False),
                RuleReference(name='ident_list', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=130,
        ),
        GrammarRule(
            name='specifier',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='type', is_token=False),
                    Literal(value='array'),
                ]),
                Sequence(elements=[
                    RuleReference(name='type', is_token=False),
                    Literal(value='procedure'),
                ]),
                Literal(value='array'),
                Literal(value='label'),
                Literal(value='switch'),
                Literal(value='procedure'),
                RuleReference(name='type', is_token=False),
            ]),
            line_number=132,
        ),
        GrammarRule(
            name='proc_body',
            body=
            Alternation(choices=[
                RuleReference(name='block', is_token=False),
                RuleReference(name='statement', is_token=False),
            ]),
            line_number=140,
        ),
        GrammarRule(
            name='statement',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Repetition(element=
                        Sequence(elements=[
                            RuleReference(name='label', is_token=False),
                            RuleReference(name='COLON', is_token=True),
                        ]),
                    ),
                    RuleReference(name='unlabeled_stmt', is_token=False),
                ]),
                Sequence(elements=[
                    Repetition(element=
                        Sequence(elements=[
                            RuleReference(name='label', is_token=False),
                            RuleReference(name='COLON', is_token=True),
                        ]),
                    ),
                    RuleReference(name='cond_stmt', is_token=False),
                ]),
            ]),
            line_number=152,
        ),
        GrammarRule(
            name='label',
            body=
            Alternation(choices=[
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='INTEGER_LIT', is_token=True),
            ]),
            line_number=155,
        ),
        GrammarRule(
            name='unlabeled_stmt',
            body=
            Alternation(choices=[
                RuleReference(name='assign_stmt', is_token=False),
                RuleReference(name='dummy_stmt', is_token=False),
                RuleReference(name='goto_stmt', is_token=False),
                RuleReference(name='proc_stmt', is_token=False),
                RuleReference(name='compound_stmt', is_token=False),
                RuleReference(name='block', is_token=False),
                RuleReference(name='for_stmt', is_token=False),
            ]),
            line_number=165,
        ),
        GrammarRule(
            name='dummy_stmt',
            body=
            Alternation(choices=[
                PositiveLookahead(element=
                    RuleReference(name='SEMICOLON', is_token=True),
                ),
                PositiveLookahead(element=
                    Literal(value='end'),
                ),
                PositiveLookahead(element=
                    Literal(value='else'),
                ),
            ]),
            line_number=175,
        ),
        GrammarRule(
            name='cond_stmt',
            body=
            Sequence(elements=[
                Literal(value='if'),
                RuleReference(name='bool_expr', is_token=False),
                Literal(value='then'),
                RuleReference(name='unlabeled_stmt', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='else'),
                        RuleReference(name='statement', is_token=False),
                    ]),
                ),
            ]),
            line_number=181,
        ),
        GrammarRule(
            name='compound_stmt',
            body=
            Sequence(elements=[
                Literal(value='begin'),
                Repetition(element=
                    RuleReference(name='SEMICOLON', is_token=True),
                ),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='statement', is_token=False),
                        Repetition(element=
                            Sequence(elements=[
                                RuleReference(name='SEMICOLON', is_token=True),
                                Optional(element=
                                    RuleReference(name='statement', is_token=False),
                                ),
                            ]),
                        ),
                    ]),
                ),
                Literal(value='end'),
            ]),
            line_number=185,
        ),
        GrammarRule(
            name='assign_stmt',
            body=
            Sequence(elements=[
                RuleReference(name='left_part', is_token=False),
                Repetition(element=
                    RuleReference(name='left_part', is_token=False),
                ),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=191,
        ),
        GrammarRule(
            name='left_part',
            body=
            Sequence(elements=[
                RuleReference(name='variable', is_token=False),
                RuleReference(name='ASSIGN', is_token=True),
            ]),
            line_number=193,
        ),
        GrammarRule(
            name='goto_stmt',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='goto'),
                    RuleReference(name='desig_expr', is_token=False),
                ]),
                Sequence(elements=[
                    Literal(value='go'),
                    Literal(value='to'),
                    RuleReference(name='desig_expr', is_token=False),
                ]),
            ]),
            line_number=197,
        ),
        GrammarRule(
            name='proc_stmt',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='LPAREN', is_token=True),
                        Optional(element=
                            RuleReference(name='actual_params', is_token=False),
                        ),
                        RuleReference(name='RPAREN', is_token=True),
                    ]),
                ),
            ]),
            line_number=202,
        ),
        GrammarRule(
            name='actual_params',
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
            line_number=204,
        ),
        GrammarRule(
            name='for_stmt',
            body=
            Sequence(elements=[
                Literal(value='for'),
                RuleReference(name='variable', is_token=False),
                RuleReference(name='ASSIGN', is_token=True),
                RuleReference(name='for_list', is_token=False),
                Literal(value='do'),
                RuleReference(name='statement', is_token=False),
            ]),
            line_number=212,
        ),
        GrammarRule(
            name='for_list',
            body=
            Sequence(elements=[
                RuleReference(name='for_elem', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='for_elem', is_token=False),
                    ]),
                ),
            ]),
            line_number=214,
        ),
        GrammarRule(
            name='for_elem',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='arith_expr', is_token=False),
                    Literal(value='step'),
                    RuleReference(name='arith_expr', is_token=False),
                    Literal(value='until'),
                    RuleReference(name='arith_expr', is_token=False),
                ]),
                Sequence(elements=[
                    RuleReference(name='arith_expr', is_token=False),
                    Literal(value='while'),
                    RuleReference(name='bool_expr', is_token=False),
                ]),
                RuleReference(name='arith_expr', is_token=False),
            ]),
            line_number=218,
        ),
        GrammarRule(
            name='expression',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='if'),
                    RuleReference(name='bool_expr', is_token=False),
                    Literal(value='then'),
                    RuleReference(name='expression', is_token=False),
                    Literal(value='else'),
                    RuleReference(name='expression', is_token=False),
                ]),
                RuleReference(name='expr_eqv', is_token=False),
            ]),
            line_number=250,
        ),
        GrammarRule(
            name='expr_eqv',
            body=
            Sequence(elements=[
                RuleReference(name='expr_impl', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='eqv'),
                        RuleReference(name='expr_impl', is_token=False),
                    ]),
                ),
            ]),
            line_number=253,
        ),
        GrammarRule(
            name='expr_impl',
            body=
            Sequence(elements=[
                RuleReference(name='expr_or', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='impl'),
                        RuleReference(name='expr_or', is_token=False),
                    ]),
                ),
            ]),
            line_number=254,
        ),
        GrammarRule(
            name='expr_or',
            body=
            Sequence(elements=[
                RuleReference(name='expr_and', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='or'),
                        RuleReference(name='expr_and', is_token=False),
                    ]),
                ),
            ]),
            line_number=255,
        ),
        GrammarRule(
            name='expr_and',
            body=
            Sequence(elements=[
                RuleReference(name='expr_not', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='and'),
                        RuleReference(name='expr_not', is_token=False),
                    ]),
                ),
            ]),
            line_number=256,
        ),
        GrammarRule(
            name='expr_not',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='not'),
                    RuleReference(name='expr_not', is_token=False),
                ]),
                RuleReference(name='expr_cmp', is_token=False),
            ]),
            line_number=257,
        ),
        GrammarRule(
            name='expr_cmp',
            body=
            Sequence(elements=[
                RuleReference(name='expr_add', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='EQ', is_token=True),
                                RuleReference(name='NEQ', is_token=True),
                                RuleReference(name='LT', is_token=True),
                                RuleReference(name='LEQ', is_token=True),
                                RuleReference(name='GT', is_token=True),
                                RuleReference(name='GEQ', is_token=True),
                            ]),
                        ),
                        RuleReference(name='expr_add', is_token=False),
                    ]),
                ),
            ]),
            line_number=258,
        ),
        GrammarRule(
            name='expr_add',
            body=
            Sequence(elements=[
                Optional(element=
                    Alternation(choices=[
                        RuleReference(name='PLUS', is_token=True),
                        RuleReference(name='MINUS', is_token=True),
                    ]),
                ),
                RuleReference(name='expr_mul', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='PLUS', is_token=True),
                                RuleReference(name='MINUS', is_token=True),
                            ]),
                        ),
                        RuleReference(name='expr_mul', is_token=False),
                    ]),
                ),
            ]),
            line_number=259,
        ),
        GrammarRule(
            name='expr_mul',
            body=
            Sequence(elements=[
                RuleReference(name='expr_pow', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='STAR', is_token=True),
                                RuleReference(name='SLASH', is_token=True),
                                Literal(value='div'),
                                Literal(value='mod'),
                            ]),
                        ),
                        RuleReference(name='expr_pow', is_token=False),
                    ]),
                ),
            ]),
            line_number=260,
        ),
        GrammarRule(
            name='expr_pow',
            body=
            Sequence(elements=[
                RuleReference(name='expr_atom', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='CARET', is_token=True),
                                RuleReference(name='POWER', is_token=True),
                            ]),
                        ),
                        RuleReference(name='expr_atom', is_token=False),
                    ]),
                ),
            ]),
            line_number=261,
        ),
        GrammarRule(
            name='expr_atom',
            body=
            Alternation(choices=[
                RuleReference(name='INTEGER_LIT', is_token=True),
                RuleReference(name='REAL_LIT', is_token=True),
                RuleReference(name='STRING_LIT', is_token=True),
                Literal(value='true'),
                Literal(value='false'),
                RuleReference(name='proc_call', is_token=False),
                RuleReference(name='variable', is_token=False),
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expression', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
            ]),
            line_number=262,
        ),
        GrammarRule(
            name='arith_expr',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='if'),
                    RuleReference(name='bool_expr', is_token=False),
                    Literal(value='then'),
                    RuleReference(name='arith_expr', is_token=False),
                    Literal(value='else'),
                    RuleReference(name='arith_expr', is_token=False),
                ]),
                RuleReference(name='simple_arith', is_token=False),
            ]),
            line_number=274,
        ),
        GrammarRule(
            name='simple_arith',
            body=
            Sequence(elements=[
                Optional(element=
                    Alternation(choices=[
                        RuleReference(name='PLUS', is_token=True),
                        RuleReference(name='MINUS', is_token=True),
                    ]),
                ),
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
            line_number=278,
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
                                Literal(value='div'),
                                Literal(value='mod'),
                            ]),
                        ),
                        RuleReference(name='factor', is_token=False),
                    ]),
                ),
            ]),
            line_number=283,
        ),
        GrammarRule(
            name='factor',
            body=
            Sequence(elements=[
                RuleReference(name='primary', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='CARET', is_token=True),
                                RuleReference(name='POWER', is_token=True),
                            ]),
                        ),
                        RuleReference(name='primary', is_token=False),
                    ]),
                ),
            ]),
            line_number=289,
        ),
        GrammarRule(
            name='primary',
            body=
            Alternation(choices=[
                RuleReference(name='INTEGER_LIT', is_token=True),
                RuleReference(name='REAL_LIT', is_token=True),
                RuleReference(name='STRING_LIT', is_token=True),
                Literal(value='true'),
                Literal(value='false'),
                RuleReference(name='proc_call', is_token=False),
                RuleReference(name='variable', is_token=False),
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='arith_expr', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
            ]),
            line_number=291,
        ),
        GrammarRule(
            name='bool_expr',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='if'),
                    RuleReference(name='bool_expr', is_token=False),
                    Literal(value='then'),
                    RuleReference(name='bool_expr', is_token=False),
                    Literal(value='else'),
                    RuleReference(name='bool_expr', is_token=False),
                ]),
                RuleReference(name='simple_bool', is_token=False),
            ]),
            line_number=309,
        ),
        GrammarRule(
            name='simple_bool',
            body=
            Sequence(elements=[
                RuleReference(name='implication', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='eqv'),
                        RuleReference(name='implication', is_token=False),
                    ]),
                ),
            ]),
            line_number=312,
        ),
        GrammarRule(
            name='implication',
            body=
            Sequence(elements=[
                RuleReference(name='bool_term', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='impl'),
                        RuleReference(name='bool_term', is_token=False),
                    ]),
                ),
            ]),
            line_number=314,
        ),
        GrammarRule(
            name='bool_term',
            body=
            Sequence(elements=[
                RuleReference(name='bool_factor', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='or'),
                        RuleReference(name='bool_factor', is_token=False),
                    ]),
                ),
            ]),
            line_number=316,
        ),
        GrammarRule(
            name='bool_factor',
            body=
            Sequence(elements=[
                RuleReference(name='bool_secondary', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='and'),
                        RuleReference(name='bool_secondary', is_token=False),
                    ]),
                ),
            ]),
            line_number=318,
        ),
        GrammarRule(
            name='bool_secondary',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='not'),
                    RuleReference(name='bool_secondary', is_token=False),
                ]),
                RuleReference(name='bool_primary', is_token=False),
            ]),
            line_number=320,
        ),
        GrammarRule(
            name='bool_primary',
            body=
            Alternation(choices=[
                RuleReference(name='relation', is_token=False),
                Literal(value='true'),
                Literal(value='false'),
                RuleReference(name='proc_call', is_token=False),
                RuleReference(name='variable', is_token=False),
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='bool_expr', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
            ]),
            line_number=322,
        ),
        GrammarRule(
            name='relation',
            body=
            Sequence(elements=[
                RuleReference(name='simple_arith', is_token=False),
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='EQ', is_token=True),
                        RuleReference(name='NEQ', is_token=True),
                        RuleReference(name='LT', is_token=True),
                        RuleReference(name='LEQ', is_token=True),
                        RuleReference(name='GT', is_token=True),
                        RuleReference(name='GEQ', is_token=True),
                    ]),
                ),
                RuleReference(name='simple_arith', is_token=False),
            ]),
            line_number=332,
        ),
        GrammarRule(
            name='desig_expr',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='if'),
                    RuleReference(name='bool_expr', is_token=False),
                    Literal(value='then'),
                    RuleReference(name='desig_expr', is_token=False),
                    Literal(value='else'),
                    RuleReference(name='desig_expr', is_token=False),
                ]),
                RuleReference(name='simple_desig', is_token=False),
            ]),
            line_number=337,
        ),
        GrammarRule(
            name='simple_desig',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='NAME', is_token=True),
                    RuleReference(name='LBRACKET', is_token=True),
                    RuleReference(name='arith_expr', is_token=False),
                    RuleReference(name='RBRACKET', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='desig_expr', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                RuleReference(name='label', is_token=False),
            ]),
            line_number=340,
        ),
        GrammarRule(
            name='variable',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='LBRACKET', is_token=True),
                        RuleReference(name='subscripts', is_token=False),
                        RuleReference(name='RBRACKET', is_token=True),
                    ]),
                ),
            ]),
            line_number=352,
        ),
        GrammarRule(
            name='subscripts',
            body=
            Sequence(elements=[
                RuleReference(name='arith_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='arith_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=354,
        ),
        GrammarRule(
            name='proc_call',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='LPAREN', is_token=True),
                Optional(element=
                    RuleReference(name='actual_params', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=359,
        ),
    ],
)
