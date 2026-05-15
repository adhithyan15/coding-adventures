# AUTO-GENERATED FILE — DO NOT EDIT
# ruff: noqa: E501, F401
# Source: /Users/adhithya/Documents/coding-adventures/.claude/worktrees/nice-wing-09855c/code/grammars/sql.grammar
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
                RuleReference(name='replace_stmt', is_token=False),
                RuleReference(name='update_stmt', is_token=False),
                RuleReference(name='delete_stmt', is_token=False),
                RuleReference(name='create_table_stmt', is_token=False),
                RuleReference(name='drop_table_stmt', is_token=False),
                RuleReference(name='alter_table_stmt', is_token=False),
                RuleReference(name='create_index_stmt', is_token=False),
                RuleReference(name='drop_index_stmt', is_token=False),
                RuleReference(name='create_view_stmt', is_token=False),
                RuleReference(name='drop_view_stmt', is_token=False),
                RuleReference(name='create_trigger_stmt', is_token=False),
                RuleReference(name='drop_trigger_stmt', is_token=False),
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
            line_number=31,
        ),
        GrammarRule(
            name='with_clause',
            body=
            Sequence(elements=[
                Literal(value='WITH'),
                Optional(element=
                    Literal(value='RECURSIVE'),
                ),
                RuleReference(name='cte_def', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value=','),
                        RuleReference(name='cte_def', is_token=False),
                    ]),
                ),
            ]),
            line_number=32,
        ),
        GrammarRule(
            name='cte_def',
            body=
            Sequence(elements=[
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
                Literal(value='AS'),
                Literal(value='('),
                RuleReference(name='query_stmt', is_token=False),
                Literal(value=')'),
            ]),
            line_number=33,
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
            line_number=34,
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
                Optional(element=
                    Sequence(elements=[
                        Literal(value='FROM'),
                        RuleReference(name='table_ref', is_token=False),
                        Repetition(element=
                            RuleReference(name='join_clause', is_token=False),
                        ),
                    ]),
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
            line_number=38,
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
            line_number=43,
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
            line_number=44,
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
                        Alternation(choices=[
                            Sequence(elements=[
                                Literal(value='AS'),
                                RuleReference(name='NAME', is_token=True),
                            ]),
                            RuleReference(name='NAME', is_token=True),
                        ]),
                    ),
                ]),
            ]),
            line_number=52,
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
            line_number=53,
        ),
        GrammarRule(
            name='join_clause',
            body=
            Alternation(choices=[
                Group(element=
                    Sequence(elements=[
                        Optional(element=
                            RuleReference(name='join_type', is_token=False),
                        ),
                        Literal(value='JOIN'),
                        RuleReference(name='table_ref', is_token=False),
                        Optional(element=
                            Alternation(choices=[
                                Sequence(elements=[
                                    Literal(value='ON'),
                                    RuleReference(name='expr', is_token=False),
                                ]),
                                Sequence(elements=[
                                    Literal(value='USING'),
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
                            ]),
                        ),
                    ]),
                ),
                Group(element=
                    Sequence(elements=[
                        Literal(value=','),
                        RuleReference(name='table_ref', is_token=False),
                    ]),
                ),
            ]),
            line_number=61,
        ),
        GrammarRule(
            name='join_type',
            body=
            Alternation(choices=[
                Literal(value='CROSS'),
                Literal(value='INNER'),
                Literal(value='NATURAL'),
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
            line_number=63,
        ),
        GrammarRule(
            name='where_clause',
            body=
            Sequence(elements=[
                Literal(value='WHERE'),
                RuleReference(name='expr', is_token=False),
            ]),
            line_number=67,
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
            line_number=68,
        ),
        GrammarRule(
            name='having_clause',
            body=
            Sequence(elements=[
                Literal(value='HAVING'),
                RuleReference(name='expr', is_token=False),
            ]),
            line_number=69,
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
            line_number=70,
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
            line_number=71,
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
            line_number=72,
        ),
        GrammarRule(
            name='conflict_clause',
            body=
            Sequence(elements=[
                Literal(value='OR'),
                Group(element=
                    Alternation(choices=[
                        Literal(value='REPLACE'),
                        Literal(value='IGNORE'),
                        Literal(value='ABORT'),
                        Literal(value='FAIL'),
                        Literal(value='ROLLBACK'),
                    ]),
                ),
            ]),
            line_number=94,
        ),
        GrammarRule(
            name='insert_stmt',
            body=
            Sequence(elements=[
                Literal(value='INSERT'),
                Optional(element=
                    RuleReference(name='conflict_clause', is_token=False),
                ),
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
                Optional(element=
                    RuleReference(name='upsert_clause', is_token=False),
                ),
                Optional(element=
                    RuleReference(name='returning_clause', is_token=False),
                ),
            ]),
            line_number=96,
        ),
        GrammarRule(
            name='upsert_clause',
            body=
            Sequence(elements=[
                Literal(value='ON'),
                Literal(value='CONFLICT'),
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
                Group(element=
                    Alternation(choices=[
                        Sequence(elements=[
                            Literal(value='DO'),
                            Literal(value='NOTHING'),
                        ]),
                        Sequence(elements=[
                            Literal(value='DO'),
                            Literal(value='UPDATE'),
                            Literal(value='SET'),
                            RuleReference(name='upsert_assignment', is_token=False),
                            Repetition(element=
                                Sequence(elements=[
                                    Literal(value=','),
                                    RuleReference(name='upsert_assignment', is_token=False),
                                ]),
                            ),
                        ]),
                    ]),
                ),
            ]),
            line_number=113,
        ),
        GrammarRule(
            name='upsert_assignment',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                Literal(value='='),
                RuleReference(name='expr', is_token=False),
            ]),
            line_number=118,
        ),
        GrammarRule(
            name='replace_stmt',
            body=
            Sequence(elements=[
                Literal(value='REPLACE'),
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
                Optional(element=
                    RuleReference(name='returning_clause', is_token=False),
                ),
            ]),
            line_number=119,
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
            line_number=123,
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
            line_number=124,
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
                Optional(element=
                    RuleReference(name='returning_clause', is_token=False),
                ),
            ]),
            line_number=126,
        ),
        GrammarRule(
            name='assignment',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                Literal(value='='),
                RuleReference(name='expr', is_token=False),
            ]),
            line_number=128,
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
                Optional(element=
                    RuleReference(name='returning_clause', is_token=False),
                ),
            ]),
            line_number=130,
        ),
        GrammarRule(
            name='returning_clause',
            body=
            Sequence(elements=[
                Literal(value='RETURNING'),
                RuleReference(name='expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value=','),
                        RuleReference(name='expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=132,
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
            line_number=136,
        ),
        GrammarRule(
            name='col_def',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='col_type', is_token=False),
                Repetition(element=
                    RuleReference(name='col_constraint', is_token=False),
                ),
            ]),
            line_number=138,
        ),
        GrammarRule(
            name='col_type',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='('),
                        RuleReference(name='NUMBER', is_token=True),
                        Repetition(element=
                            Sequence(elements=[
                                Literal(value=','),
                                RuleReference(name='NUMBER', is_token=True),
                            ]),
                        ),
                        Literal(value=')'),
                    ]),
                ),
            ]),
            line_number=142,
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
                            Sequence(elements=[
                                Literal(value='('),
                                RuleReference(name='NAME', is_token=True),
                                Literal(value=')'),
                            ]),
                        ),
                    ]),
                ),
            ]),
            line_number=143,
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
            line_number=148,
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
            line_number=152,
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
                RuleReference(name='index_col', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value=','),
                        RuleReference(name='index_col', is_token=False),
                    ]),
                ),
                Literal(value=')'),
            ]),
            line_number=161,
        ),
        GrammarRule(
            name='index_col',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    Alternation(choices=[
                        Literal(value='ASC'),
                        Literal(value='DESC'),
                    ]),
                ),
            ]),
            line_number=166,
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
            line_number=168,
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
            line_number=176,
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
            line_number=178,
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
            line_number=184,
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
            line_number=185,
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
            line_number=186,
        ),
        GrammarRule(
            name='savepoint_stmt',
            body=
            Sequence(elements=[
                Literal(value='SAVEPOINT'),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=202,
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
            line_number=203,
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
            line_number=204,
        ),
        GrammarRule(
            name='expr',
            body=
            RuleReference(name='or_expr', is_token=False),
            line_number=208,
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
            line_number=209,
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
            line_number=210,
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
            line_number=211,
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
                            Optional(element=
                                RuleReference(name='in_expr', is_token=False),
                            ),
                            Literal(value=')'),
                        ]),
                        Sequence(elements=[
                            Literal(value='NOT'),
                            Literal(value='IN'),
                            Literal(value='('),
                            Optional(element=
                                RuleReference(name='in_expr', is_token=False),
                            ),
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
                            Literal(value='GLOB'),
                            RuleReference(name='additive', is_token=False),
                        ]),
                        Sequence(elements=[
                            Literal(value='NOT'),
                            Literal(value='GLOB'),
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
                        Sequence(elements=[
                            Literal(value='IS'),
                            Literal(value='DISTINCT'),
                            Literal(value='FROM'),
                            RuleReference(name='additive', is_token=False),
                        ]),
                        Sequence(elements=[
                            Literal(value='IS'),
                            Literal(value='NOT'),
                            Literal(value='DISTINCT'),
                            Literal(value='FROM'),
                            RuleReference(name='additive', is_token=False),
                        ]),
                    ]),
                ),
            ]),
            line_number=212,
        ),
        GrammarRule(
            name='in_expr',
            body=
            Alternation(choices=[
                RuleReference(name='query_stmt', is_token=False),
                RuleReference(name='value_list', is_token=False),
            ]),
            line_number=228,
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
            line_number=230,
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
                                Literal(value='||'),
                            ]),
                        ),
                        RuleReference(name='multiplicative', is_token=False),
                    ]),
                ),
            ]),
            line_number=231,
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
            line_number=232,
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
            line_number=233,
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
                RuleReference(name='cast_expr', is_token=False),
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
            line_number=253,
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
            line_number=263,
        ),
        GrammarRule(
            name='function_call',
            body=
            Sequence(elements=[
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='NAME', is_token=True),
                        Literal(value='REPLACE'),
                    ]),
                ),
                Literal(value='('),
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='STAR', is_token=True),
                        Sequence(elements=[
                            Literal(value='DISTINCT'),
                            RuleReference(name='value_list', is_token=False),
                        ]),
                        Optional(element=
                            RuleReference(name='value_list', is_token=False),
                        ),
                    ]),
                ),
                Literal(value=')'),
                Optional(element=
                    RuleReference(name='filter_clause', is_token=False),
                ),
            ]),
            line_number=277,
        ),
        GrammarRule(
            name='filter_clause',
            body=
            Sequence(elements=[
                Literal(value='FILTER'),
                Literal(value='('),
                Literal(value='WHERE'),
                RuleReference(name='expr', is_token=False),
                Literal(value=')'),
            ]),
            line_number=281,
        ),
        GrammarRule(
            name='cast_expr',
            body=
            Sequence(elements=[
                Literal(value='CAST'),
                Literal(value='('),
                RuleReference(name='expr', is_token=False),
                Literal(value='AS'),
                RuleReference(name='NAME', is_token=True),
                Literal(value=')'),
            ]),
            line_number=287,
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
            line_number=316,
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
                Optional(element=
                    RuleReference(name='frame_clause', is_token=False),
                ),
            ]),
            line_number=317,
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
            line_number=318,
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
            line_number=319,
        ),
        GrammarRule(
            name='frame_clause',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='frame_unit', is_token=False),
                    Literal(value='BETWEEN'),
                    RuleReference(name='frame_bound', is_token=False),
                    Literal(value='AND'),
                    RuleReference(name='frame_bound', is_token=False),
                ]),
                Sequence(elements=[
                    RuleReference(name='frame_unit', is_token=False),
                    RuleReference(name='frame_bound', is_token=False),
                ]),
            ]),
            line_number=341,
        ),
        GrammarRule(
            name='frame_unit',
            body=
            Alternation(choices=[
                Literal(value='ROWS'),
                Literal(value='RANGE'),
                Literal(value='GROUPS'),
            ]),
            line_number=343,
        ),
        GrammarRule(
            name='frame_bound',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='UNBOUNDED'),
                    Literal(value='PRECEDING'),
                ]),
                Sequence(elements=[
                    Literal(value='UNBOUNDED'),
                    Literal(value='FOLLOWING'),
                ]),
                Sequence(elements=[
                    Literal(value='CURRENT'),
                    Literal(value='ROW'),
                ]),
                Sequence(elements=[
                    RuleReference(name='expr', is_token=False),
                    Literal(value='PRECEDING'),
                ]),
                Sequence(elements=[
                    RuleReference(name='expr', is_token=False),
                    Literal(value='FOLLOWING'),
                ]),
            ]),
            line_number=344,
        ),
        GrammarRule(
            name='create_trigger_stmt',
            body=
            Sequence(elements=[
                Literal(value='CREATE'),
                Literal(value='TRIGGER'),
                RuleReference(name='NAME', is_token=True),
                Group(element=
                    Alternation(choices=[
                        Literal(value='BEFORE'),
                        Literal(value='AFTER'),
                    ]),
                ),
                Group(element=
                    Alternation(choices=[
                        Literal(value='INSERT'),
                        Literal(value='UPDATE'),
                        Literal(value='DELETE'),
                    ]),
                ),
                Literal(value='ON'),
                RuleReference(name='NAME', is_token=True),
                Literal(value='FOR'),
                Literal(value='EACH'),
                Literal(value='ROW'),
                Literal(value='BEGIN'),
                RuleReference(name='trigger_body_stmt', is_token=False),
                Literal(value=';'),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='trigger_body_stmt', is_token=False),
                        Literal(value=';'),
                    ]),
                ),
                Literal(value='END'),
            ]),
            line_number=370,
        ),
        GrammarRule(
            name='trigger_body_stmt',
            body=
            Alternation(choices=[
                RuleReference(name='insert_stmt', is_token=False),
                RuleReference(name='replace_stmt', is_token=False),
                RuleReference(name='update_stmt', is_token=False),
                RuleReference(name='delete_stmt', is_token=False),
                RuleReference(name='query_stmt', is_token=False),
            ]),
            line_number=375,
        ),
        GrammarRule(
            name='drop_trigger_stmt',
            body=
            Sequence(elements=[
                Literal(value='DROP'),
                Literal(value='TRIGGER'),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='IF'),
                        Literal(value='EXISTS'),
                    ]),
                ),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=377,
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
            line_number=392,
        ),
        GrammarRule(
            name='case_operand',
            body=
            RuleReference(name='expr', is_token=False),
            line_number=393,
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
            line_number=394,
        ),
    ],
)
