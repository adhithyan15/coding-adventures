# AUTO-GENERATED FILE — DO NOT EDIT
# Source: sql.grammar
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
                RuleReference(name='statement', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value=';'),
                        RuleReference(name='statement', is_token=False),
                    ]),
                ),
                Optional(element=
                    Literal(value=';'),
                ),
            ]),
            line_number=10,
        ),
        GrammarRule(
            name='statement',
            body=
            Alternation(choices=[
                RuleReference(name='select_stmt', is_token=False),
                RuleReference(name='insert_stmt', is_token=False),
                RuleReference(name='update_stmt', is_token=False),
                RuleReference(name='delete_stmt', is_token=False),
                RuleReference(name='create_table_stmt', is_token=False),
                RuleReference(name='drop_table_stmt', is_token=False),
            ]),
            line_number=12,
        ),
        GrammarRule(
            name='select_stmt',
            body=
            Sequence(elements=[
                Literal(value='SELECT'),
                Optional(element=
                    Alternation(choices=[
                        Literal(value='DISTINCT'),
                        Literal(value='ALL'),
                    ]),
                ),
                RuleReference(name='select_list', is_token=False),
                Literal(value='FROM'),
                RuleReference(name='table_ref', is_token=False),
                Repetition(element=
                    RuleReference(name='join_clause', is_token=False),
                ),
                Optional(element=
                    RuleReference(name='where_clause', is_token=False),
                ),
                Optional(element=
                    RuleReference(name='group_clause', is_token=False),
                ),
                Optional(element=
                    RuleReference(name='having_clause', is_token=False),
                ),
                Optional(element=
                    RuleReference(name='order_clause', is_token=False),
                ),
                Optional(element=
                    RuleReference(name='limit_clause', is_token=False),
                ),
            ]),
            line_number=17,
        ),
        GrammarRule(
            name='select_list',
            body=
            Alternation(choices=[
                RuleReference(name='STAR', is_token=True),
                Sequence(elements=[
                    RuleReference(name='select_item', is_token=False),
                    Repetition(element=
                        Sequence(elements=[
                            Literal(value=','),
                            RuleReference(name='select_item', is_token=False),
                        ]),
                    ),
                ]),
            ]),
            line_number=22,
        ),
        GrammarRule(
            name='select_item',
            body=
            Sequence(elements=[
                RuleReference(name='expr', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='AS'),
                        RuleReference(name='NAME', is_token=True),
                    ]),
                ),
            ]),
            line_number=23,
        ),
        GrammarRule(
            name='table_ref',
            body=
            Sequence(elements=[
                RuleReference(name='table_name', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='AS'),
                        RuleReference(name='NAME', is_token=True),
                    ]),
                ),
            ]),
            line_number=25,
        ),
        GrammarRule(
            name='table_name',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='.'),
                        RuleReference(name='NAME', is_token=True),
                    ]),
                ),
            ]),
            line_number=26,
        ),
        GrammarRule(
            name='join_clause',
            body=
            Sequence(elements=[
                RuleReference(name='join_type', is_token=False),
                Literal(value='JOIN'),
                RuleReference(name='table_ref', is_token=False),
                Literal(value='ON'),
                RuleReference(name='expr', is_token=False),
            ]),
            line_number=28,
        ),
        GrammarRule(
            name='join_type',
            body=
            Alternation(choices=[
                Literal(value='CROSS'),
                Literal(value='INNER'),
                Group(element=
                    Sequence(elements=[
                        Literal(value='LEFT'),
                        Optional(element=
                            Literal(value='OUTER'),
                        ),
                    ]),
                ),
                Group(element=
                    Sequence(elements=[
                        Literal(value='RIGHT'),
                        Optional(element=
                            Literal(value='OUTER'),
                        ),
                    ]),
                ),
                Group(element=
                    Sequence(elements=[
                        Literal(value='FULL'),
                        Optional(element=
                            Literal(value='OUTER'),
                        ),
                    ]),
                ),
            ]),
            line_number=29,
        ),
        GrammarRule(
            name='where_clause',
            body=
            Sequence(elements=[
                Literal(value='WHERE'),
                RuleReference(name='expr', is_token=False),
            ]),
            line_number=32,
        ),
        GrammarRule(
            name='group_clause',
            body=
            Sequence(elements=[
                Literal(value='GROUP'),
                Literal(value='BY'),
                RuleReference(name='column_ref', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value=','),
                        RuleReference(name='column_ref', is_token=False),
                    ]),
                ),
            ]),
            line_number=33,
        ),
        GrammarRule(
            name='having_clause',
            body=
            Sequence(elements=[
                Literal(value='HAVING'),
                RuleReference(name='expr', is_token=False),
            ]),
            line_number=34,
        ),
        GrammarRule(
            name='order_clause',
            body=
            Sequence(elements=[
                Literal(value='ORDER'),
                Literal(value='BY'),
                RuleReference(name='order_item', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value=','),
                        RuleReference(name='order_item', is_token=False),
                    ]),
                ),
            ]),
            line_number=35,
        ),
        GrammarRule(
            name='order_item',
            body=
            Sequence(elements=[
                RuleReference(name='expr', is_token=False),
                Optional(element=
                    Alternation(choices=[
                        Literal(value='ASC'),
                        Literal(value='DESC'),
                    ]),
                ),
            ]),
            line_number=36,
        ),
        GrammarRule(
            name='limit_clause',
            body=
            Sequence(elements=[
                Literal(value='LIMIT'),
                RuleReference(name='NUMBER', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='OFFSET'),
                        RuleReference(name='NUMBER', is_token=True),
                    ]),
                ),
            ]),
            line_number=37,
        ),
        GrammarRule(
            name='insert_stmt',
            body=
            Sequence(elements=[
                Literal(value='INSERT'),
                Literal(value='INTO'),
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='('),
                        RuleReference(name='NAME', is_token=True),
                        Repetition(element=
                            Sequence(elements=[
                                Literal(value=','),
                                RuleReference(name='NAME', is_token=True),
                            ]),
                        ),
                        Literal(value=')'),
                    ]),
                ),
                Literal(value='VALUES'),
                RuleReference(name='row_value', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value=','),
                        RuleReference(name='row_value', is_token=False),
                    ]),
                ),
            ]),
            line_number=41,
        ),
        GrammarRule(
            name='row_value',
            body=
            Sequence(elements=[
                Literal(value='('),
                RuleReference(name='expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value=','),
                        RuleReference(name='expr', is_token=False),
                    ]),
                ),
                Literal(value=')'),
            ]),
            line_number=44,
        ),
        GrammarRule(
            name='update_stmt',
            body=
            Sequence(elements=[
                Literal(value='UPDATE'),
                RuleReference(name='NAME', is_token=True),
                Literal(value='SET'),
                RuleReference(name='assignment', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value=','),
                        RuleReference(name='assignment', is_token=False),
                    ]),
                ),
                Optional(element=
                    RuleReference(name='where_clause', is_token=False),
                ),
            ]),
            line_number=46,
        ),
        GrammarRule(
            name='assignment',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                Literal(value='='),
                RuleReference(name='expr', is_token=False),
            ]),
            line_number=48,
        ),
        GrammarRule(
            name='delete_stmt',
            body=
            Sequence(elements=[
                Literal(value='DELETE'),
                Literal(value='FROM'),
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    RuleReference(name='where_clause', is_token=False),
                ),
            ]),
            line_number=50,
        ),
        GrammarRule(
            name='create_table_stmt',
            body=
            Sequence(elements=[
                Literal(value='CREATE'),
                Literal(value='TABLE'),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='IF'),
                        Literal(value='NOT'),
                        Literal(value='EXISTS'),
                    ]),
                ),
                RuleReference(name='NAME', is_token=True),
                Literal(value='('),
                RuleReference(name='col_def', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value=','),
                        RuleReference(name='col_def', is_token=False),
                    ]),
                ),
                Literal(value=')'),
            ]),
            line_number=54,
        ),
        GrammarRule(
            name='col_def',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='NAME', is_token=True),
                Repetition(element=
                    RuleReference(name='col_constraint', is_token=False),
                ),
            ]),
            line_number=56,
        ),
        GrammarRule(
            name='col_constraint',
            body=
            Alternation(choices=[
                Group(element=
                    Sequence(elements=[
                        Literal(value='NOT'),
                        Literal(value='NULL'),
                    ]),
                ),
                Literal(value='NULL'),
                Group(element=
                    Sequence(elements=[
                        Literal(value='PRIMARY'),
                        Literal(value='KEY'),
                    ]),
                ),
                Literal(value='UNIQUE'),
                Group(element=
                    Sequence(elements=[
                        Literal(value='DEFAULT'),
                        RuleReference(name='primary', is_token=False),
                    ]),
                ),
            ]),
            line_number=57,
        ),
        GrammarRule(
            name='drop_table_stmt',
            body=
            Sequence(elements=[
                Literal(value='DROP'),
                Literal(value='TABLE'),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='IF'),
                        Literal(value='EXISTS'),
                    ]),
                ),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=60,
        ),
        GrammarRule(
            name='expr',
            body=
            RuleReference(name='or_expr', is_token=False),
            line_number=64,
        ),
        GrammarRule(
            name='or_expr',
            body=
            Sequence(elements=[
                RuleReference(name='and_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='OR'),
                        RuleReference(name='and_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=65,
        ),
        GrammarRule(
            name='and_expr',
            body=
            Sequence(elements=[
                RuleReference(name='not_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='AND'),
                        RuleReference(name='not_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=66,
        ),
        GrammarRule(
            name='not_expr',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='NOT'),
                    RuleReference(name='not_expr', is_token=False),
                ]),
                RuleReference(name='comparison', is_token=False),
            ]),
            line_number=67,
        ),
        GrammarRule(
            name='comparison',
            body=
            Sequence(elements=[
                RuleReference(name='additive', is_token=False),
                Optional(element=
                    Alternation(choices=[
                        Sequence(elements=[
                            RuleReference(name='cmp_op', is_token=False),
                            RuleReference(name='additive', is_token=False),
                        ]),
                        Sequence(elements=[
                            Literal(value='BETWEEN'),
                            RuleReference(name='additive', is_token=False),
                            Literal(value='AND'),
                            RuleReference(name='additive', is_token=False),
                        ]),
                        Sequence(elements=[
                            Literal(value='NOT'),
                            Literal(value='BETWEEN'),
                            RuleReference(name='additive', is_token=False),
                            Literal(value='AND'),
                            RuleReference(name='additive', is_token=False),
                        ]),
                        Sequence(elements=[
                            Literal(value='IN'),
                            Literal(value='('),
                            RuleReference(name='value_list', is_token=False),
                            Literal(value=')'),
                        ]),
                        Sequence(elements=[
                            Literal(value='NOT'),
                            Literal(value='IN'),
                            Literal(value='('),
                            RuleReference(name='value_list', is_token=False),
                            Literal(value=')'),
                        ]),
                        Sequence(elements=[
                            Literal(value='LIKE'),
                            RuleReference(name='additive', is_token=False),
                        ]),
                        Sequence(elements=[
                            Literal(value='NOT'),
                            Literal(value='LIKE'),
                            RuleReference(name='additive', is_token=False),
                        ]),
                        Sequence(elements=[
                            Literal(value='IS'),
                            Literal(value='NULL'),
                        ]),
                        Sequence(elements=[
                            Literal(value='IS'),
                            Literal(value='NOT'),
                            Literal(value='NULL'),
                        ]),
                    ]),
                ),
            ]),
            line_number=68,
        ),
        GrammarRule(
            name='cmp_op',
            body=
            Alternation(choices=[
                Literal(value='='),
                RuleReference(name='NOT_EQUALS', is_token=True),
                Literal(value='<'),
                Literal(value='>'),
                Literal(value='<='),
                Literal(value='>='),
            ]),
            line_number=78,
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
                                Literal(value='+'),
                                Literal(value='-'),
                            ]),
                        ),
                        RuleReference(name='multiplicative', is_token=False),
                    ]),
                ),
            ]),
            line_number=79,
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
                                Literal(value='/'),
                                Literal(value='%'),
                            ]),
                        ),
                        RuleReference(name='unary', is_token=False),
                    ]),
                ),
            ]),
            line_number=80,
        ),
        GrammarRule(
            name='unary',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='-'),
                    RuleReference(name='unary', is_token=False),
                ]),
                RuleReference(name='primary', is_token=False),
            ]),
            line_number=81,
        ),
        GrammarRule(
            name='primary',
            body=
            Alternation(choices=[
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='STRING', is_token=True),
                Literal(value='NULL'),
                Literal(value='TRUE'),
                Literal(value='FALSE'),
                RuleReference(name='function_call', is_token=False),
                RuleReference(name='column_ref', is_token=False),
                Sequence(elements=[
                    Literal(value='('),
                    RuleReference(name='expr', is_token=False),
                    Literal(value=')'),
                ]),
            ]),
            line_number=82,
        ),
        GrammarRule(
            name='column_ref',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='.'),
                        RuleReference(name='NAME', is_token=True),
                    ]),
                ),
            ]),
            line_number=85,
        ),
        GrammarRule(
            name='function_call',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                Literal(value='('),
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='STAR', is_token=True),
                        Optional(element=
                            RuleReference(name='value_list', is_token=False),
                        ),
                    ]),
                ),
                Literal(value=')'),
            ]),
            line_number=86,
        ),
        GrammarRule(
            name='value_list',
            body=
            Sequence(elements=[
                RuleReference(name='expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value=','),
                        RuleReference(name='expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=87,
        ),
    ],
)
