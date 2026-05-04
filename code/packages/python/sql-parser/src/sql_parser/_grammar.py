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
    version=2,
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
                RuleReference(name='query_stmt', is_token=False),
                RuleReference(name='insert_stmt', is_token=False),
                RuleReference(name='update_stmt', is_token=False),
                RuleReference(name='delete_stmt', is_token=False),
                RuleReference(name='alter_table_stmt', is_token=False),
                RuleReference(name='create_table_stmt', is_token=False),
                RuleReference(name='drop_table_stmt', is_token=False),
                RuleReference(name='create_index_stmt', is_token=False),
                RuleReference(name='drop_index_stmt', is_token=False),
                RuleReference(name='create_view_stmt', is_token=False),
                RuleReference(name='drop_view_stmt', is_token=False),
                RuleReference(name='begin_stmt', is_token=False),
                RuleReference(name='commit_stmt', is_token=False),
                RuleReference(name='rollback_to_stmt', is_token=False),
                RuleReference(name='rollback_stmt', is_token=False),
                RuleReference(name='savepoint_stmt', is_token=False),
                RuleReference(name='release_stmt', is_token=False),
            ]),
            line_number=12,
        ),
        GrammarRule(
            name='query_stmt',
            body=
            Sequence(elements=[
                Optional(element=
                    RuleReference(name='with_clause', is_token=False),
                ),
                RuleReference(name='select_stmt', is_token=False),
                Repetition(element=
                    RuleReference(name='set_op_clause', is_token=False),
                ),
            ]),
            line_number=27,
        ),
        GrammarRule(
            name='with_clause',
            body=
            Sequence(elements=[
                Literal(value='WITH'),
                Optional(element=Literal(value='RECURSIVE')),
                RuleReference(name='cte_def', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value=','),
                        RuleReference(name='cte_def', is_token=False),
                    ]),
                ),
            ]),
            line_number=28,
        ),
        GrammarRule(
            name='cte_def',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                Literal(value='AS'),
                Literal(value='('),
                RuleReference(name='query_stmt', is_token=False),
                Literal(value=')'),
            ]),
            line_number=29,
        ),
        GrammarRule(
            name='set_op_clause',
            body=
            Sequence(elements=[
                Group(element=
                    Alternation(choices=[
                        Literal(value='UNION'),
                        Literal(value='INTERSECT'),
                        Literal(value='EXCEPT'),
                    ]),
                ),
                Optional(element=
                    Literal(value='ALL'),
                ),
                RuleReference(name='select_stmt', is_token=False),
            ]),
            line_number=28,
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
            line_number=32,
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
            line_number=37,
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
            line_number=38,
        ),
        GrammarRule(
            name='table_ref',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='('),
                    RuleReference(name='query_stmt', is_token=False),
                    Literal(value=')'),
                    Literal(value='AS'),
                    RuleReference(name='NAME', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='table_name', is_token=False),
                    Optional(element=
                        Sequence(elements=[
                            Literal(value='AS'),
                            RuleReference(name='NAME', is_token=True),
                        ]),
                    ),
                ]),
            ]),
            line_number=43,
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
            line_number=44,
        ),
        GrammarRule(
            name='join_clause',
            body=
            Sequence(elements=[
                RuleReference(name='join_type', is_token=False),
                Literal(value='JOIN'),
                RuleReference(name='table_ref', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='ON'),
                        RuleReference(name='expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=46,
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
            line_number=47,
        ),
        GrammarRule(
            name='where_clause',
            body=
            Sequence(elements=[
                Literal(value='WHERE'),
                RuleReference(name='expr', is_token=False),
            ]),
            line_number=50,
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
            line_number=51,
        ),
        GrammarRule(
            name='having_clause',
            body=
            Sequence(elements=[
                Literal(value='HAVING'),
                RuleReference(name='expr', is_token=False),
            ]),
            line_number=52,
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
            line_number=53,
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
            line_number=54,
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
            line_number=55,
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
                RuleReference(name='insert_body', is_token=False),
            ]),
            line_number=63,
        ),
        GrammarRule(
            name='insert_body',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='VALUES'),
                    RuleReference(name='row_value', is_token=False),
                    Repetition(element=
                        Sequence(elements=[
                            Literal(value=','),
                            RuleReference(name='row_value', is_token=False),
                        ]),
                    ),
                ]),
                RuleReference(name='query_stmt', is_token=False),
            ]),
            line_number=66,
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
            line_number=67,
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
            line_number=69,
        ),
        GrammarRule(
            name='assignment',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                Literal(value='='),
                RuleReference(name='expr', is_token=False),
            ]),
            line_number=71,
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
            line_number=73,
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
            line_number=77,
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
            line_number=79,
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
                Group(element=
                    Sequence(elements=[
                        Literal(value='CHECK'),
                        Literal(value='('),
                        RuleReference(name='expr', is_token=False),
                        Literal(value=')'),
                    ]),
                ),
                Group(element=
                    Sequence(elements=[
                        Literal(value='REFERENCES'),
                        RuleReference(name='NAME', is_token=True),
                        Optional(element=
                            Group(element=
                                Sequence(elements=[
                                    Literal(value='('),
                                    RuleReference(name='NAME', is_token=True),
                                    Literal(value=')'),
                                ]),
                            ),
                        ),
                    ]),
                ),
            ]),
            line_number=80,
        ),
        GrammarRule(
            name='alter_table_stmt',
            body=
            Sequence(elements=[
                Literal(value='ALTER'),
                Literal(value='TABLE'),
                RuleReference(name='NAME', is_token=True),
                Literal(value='ADD'),
                Optional(element=
                    Literal(value='COLUMN'),
                ),
                RuleReference(name='col_def', is_token=False),
            ]),
            line_number=88,
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
            line_number=83,
        ),
        GrammarRule(
            name='create_index_stmt',
            body=
            Sequence(elements=[
                Literal(value='CREATE'),
                Optional(element=
                    Literal(value='UNIQUE'),
                ),
                Literal(value='INDEX'),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='IF'),
                        Literal(value='NOT'),
                        Literal(value='EXISTS'),
                    ]),
                ),
                RuleReference(name='NAME', is_token=True),
                Literal(value='ON'),
                RuleReference(name='NAME', is_token=True),
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
            line_number=92,
        ),
        GrammarRule(
            name='drop_index_stmt',
            body=
            Sequence(elements=[
                Literal(value='DROP'),
                Literal(value='INDEX'),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='IF'),
                        Literal(value='EXISTS'),
                    ]),
                ),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=95,
        ),
        GrammarRule(
            name='create_view_stmt',
            body=
            Sequence(elements=[
                Literal(value='CREATE'),
                Literal(value='VIEW'),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='IF'),
                        Literal(value='NOT'),
                        Literal(value='EXISTS'),
                    ]),
                ),
                RuleReference(name='NAME', is_token=True),
                Literal(value='AS'),
                RuleReference(name='query_stmt', is_token=False),
            ]),
            line_number=108,
        ),
        GrammarRule(
            name='drop_view_stmt',
            body=
            Sequence(elements=[
                Literal(value='DROP'),
                Literal(value='VIEW'),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='IF'),
                        Literal(value='EXISTS'),
                    ]),
                ),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=110,
        ),
        GrammarRule(
            name='begin_stmt',
            body=
            Sequence(elements=[
                Literal(value='BEGIN'),
                Optional(element=
                    Literal(value='TRANSACTION'),
                ),
            ]),
            line_number=101,
        ),
        GrammarRule(
            name='commit_stmt',
            body=
            Sequence(elements=[
                Literal(value='COMMIT'),
                Optional(element=
                    Literal(value='TRANSACTION'),
                ),
            ]),
            line_number=102,
        ),
        GrammarRule(
            name='rollback_stmt',
            body=
            Sequence(elements=[
                Literal(value='ROLLBACK'),
                Optional(element=
                    Literal(value='TRANSACTION'),
                ),
            ]),
            line_number=103,
        ),
        GrammarRule(
            name='savepoint_stmt',
            body=
            Sequence(elements=[
                Literal(value='SAVEPOINT'),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=126,
        ),
        GrammarRule(
            name='release_stmt',
            body=
            Sequence(elements=[
                Literal(value='RELEASE'),
                Optional(element=
                    Literal(value='SAVEPOINT'),
                ),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=127,
        ),
        GrammarRule(
            name='rollback_to_stmt',
            body=
            Sequence(elements=[
                Literal(value='ROLLBACK'),
                Literal(value='TO'),
                Optional(element=
                    Literal(value='SAVEPOINT'),
                ),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=128,
        ),
        GrammarRule(
            name='expr',
            body=
            RuleReference(name='or_expr', is_token=False),
            line_number=107,
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
            line_number=108,
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
            line_number=109,
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
            line_number=110,
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
                            RuleReference(name='in_expr', is_token=False),
                            Literal(value=')'),
                        ]),
                        Sequence(elements=[
                            Literal(value='NOT'),
                            Literal(value='IN'),
                            Literal(value='('),
                            RuleReference(name='in_expr', is_token=False),
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
            line_number=111,
        ),
        GrammarRule(
            name='in_expr',
            body=
            Alternation(choices=[
                RuleReference(name='query_stmt', is_token=False),
                RuleReference(name='value_list', is_token=False),
            ]),
            line_number=123,
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
            line_number=125,
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
            line_number=126,
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
            line_number=127,
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
            line_number=128,
        ),
        GrammarRule(
            name='primary',
            body=
            Alternation(choices=[
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='BLOB', is_token=True),
                Literal(value='NULL'),
                Literal(value='TRUE'),
                Literal(value='FALSE'),
                RuleReference(name='case_expr', is_token=False),
                RuleReference(name='window_func_call', is_token=False),
                RuleReference(name='function_call', is_token=False),
                Sequence(elements=[
                    Literal(value='EXISTS'),
                    Literal(value='('),
                    RuleReference(name='query_stmt', is_token=False),
                    Literal(value=')'),
                ]),
                Sequence(elements=[
                    Literal(value='('),
                    RuleReference(name='query_stmt', is_token=False),
                    Literal(value=')'),
                ]),
                RuleReference(name='column_ref', is_token=False),
                Sequence(elements=[
                    Literal(value='('),
                    RuleReference(name='expr', is_token=False),
                    Literal(value=')'),
                ]),
            ]),
            line_number=140,
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
            line_number=147,
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
            line_number=148,
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
            line_number=149,
        ),
        GrammarRule(
            name='case_expr',
            body=
            Sequence(elements=[
                Literal(value='CASE'),
                Optional(element=
                    RuleReference(name='case_operand', is_token=False),
                ),
                RuleReference(name='case_when', is_token=False),
                Repetition(element=
                    RuleReference(name='case_when', is_token=False),
                ),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='ELSE'),
                        RuleReference(name='expr', is_token=False),
                    ]),
                ),
                Literal(value='END'),
            ]),
            line_number=164,
        ),
        GrammarRule(
            name='case_operand',
            body=
            RuleReference(name='expr', is_token=False),
            line_number=165,
        ),
        GrammarRule(
            name='case_when',
            body=
            Sequence(elements=[
                Literal(value='WHEN'),
                RuleReference(name='expr', is_token=False),
                Literal(value='THEN'),
                RuleReference(name='expr', is_token=False),
            ]),
            line_number=166,
        ),
        GrammarRule(
            name='window_func_call',
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
                Literal(value='OVER'),
                Literal(value='('),
                RuleReference(name='window_spec', is_token=False),
                Literal(value=')'),
            ]),
            line_number=192,
        ),
        GrammarRule(
            name='window_spec',
            body=
            Sequence(elements=[
                Optional(element=
                    RuleReference(name='partition_clause', is_token=False),
                ),
                Optional(element=
                    RuleReference(name='order_clause', is_token=False),
                ),
            ]),
            line_number=193,
        ),
        GrammarRule(
            name='partition_clause',
            body=
            Sequence(elements=[
                Literal(value='PARTITION'),
                Literal(value='BY'),
                RuleReference(name='expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value=','),
                        RuleReference(name='expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=194,
        ),
    ],
)
