# AUTO-GENERATED FILE — DO NOT EDIT
# Source: excel.grammar
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
            name='formula',
            body=
            Sequence(elements=[
                RuleReference(name='ws', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='EQUALS', is_token=True),
                        RuleReference(name='ws', is_token=False),
                    ]),
                ),
                RuleReference(name='expression', is_token=False),
                RuleReference(name='ws', is_token=False),
            ]),
            line_number=15,
        ),
        GrammarRule(
            name='ws',
            body=
            Repetition(element=
                RuleReference(name='SPACE', is_token=True),
            ),
            line_number=17,
        ),
        GrammarRule(
            name='req_space',
            body=
            Sequence(elements=[
                RuleReference(name='SPACE', is_token=True),
                Repetition(element=
                    RuleReference(name='SPACE', is_token=True),
                ),
            ]),
            line_number=18,
        ),
        GrammarRule(
            name='expression',
            body=
            RuleReference(name='comparison_expr', is_token=False),
            line_number=20,
        ),
        GrammarRule(
            name='comparison_expr',
            body=
            Sequence(elements=[
                RuleReference(name='concat_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='comparison_op', is_token=False),
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='concat_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=22,
        ),
        GrammarRule(
            name='comparison_op',
            body=
            Alternation(choices=[
                RuleReference(name='EQUALS', is_token=True),
                RuleReference(name='NOT_EQUALS', is_token=True),
                RuleReference(name='LESS_THAN', is_token=True),
                RuleReference(name='LESS_EQUALS', is_token=True),
                RuleReference(name='GREATER_THAN', is_token=True),
                RuleReference(name='GREATER_EQUALS', is_token=True),
            ]),
            line_number=23,
        ),
        GrammarRule(
            name='concat_expr',
            body=
            Sequence(elements=[
                RuleReference(name='additive_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='AMP', is_token=True),
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='additive_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=26,
        ),
        GrammarRule(
            name='additive_expr',
            body=
            Sequence(elements=[
                RuleReference(name='multiplicative_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='ws', is_token=False),
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='PLUS', is_token=True),
                                RuleReference(name='MINUS', is_token=True),
                            ]),
                        ),
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='multiplicative_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=27,
        ),
        GrammarRule(
            name='multiplicative_expr',
            body=
            Sequence(elements=[
                RuleReference(name='power_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='ws', is_token=False),
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='STAR', is_token=True),
                                RuleReference(name='SLASH', is_token=True),
                            ]),
                        ),
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='power_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=28,
        ),
        GrammarRule(
            name='power_expr',
            body=
            Sequence(elements=[
                RuleReference(name='unary_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='CARET', is_token=True),
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='unary_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=29,
        ),
        GrammarRule(
            name='unary_expr',
            body=
            Sequence(elements=[
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='prefix_op', is_token=False),
                        RuleReference(name='ws', is_token=False),
                    ]),
                ),
                RuleReference(name='postfix_expr', is_token=False),
            ]),
            line_number=30,
        ),
        GrammarRule(
            name='prefix_op',
            body=
            Alternation(choices=[
                RuleReference(name='PLUS', is_token=True),
                RuleReference(name='MINUS', is_token=True),
            ]),
            line_number=31,
        ),
        GrammarRule(
            name='postfix_expr',
            body=
            Sequence(elements=[
                RuleReference(name='primary', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='PERCENT', is_token=True),
                    ]),
                ),
            ]),
            line_number=32,
        ),
        GrammarRule(
            name='primary',
            body=
            Alternation(choices=[
                RuleReference(name='parenthesized_expression', is_token=False),
                RuleReference(name='constant', is_token=False),
                RuleReference(name='function_call', is_token=False),
                RuleReference(name='structure_reference', is_token=False),
                RuleReference(name='reference_expression', is_token=False),
                RuleReference(name='bang_reference', is_token=False),
                RuleReference(name='bang_name', is_token=False),
                RuleReference(name='name_reference', is_token=False),
            ]),
            line_number=34,
        ),
        GrammarRule(
            name='parenthesized_expression',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='ws', is_token=False),
                RuleReference(name='expression', is_token=False),
                RuleReference(name='ws', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=43,
        ),
        GrammarRule(
            name='constant',
            body=
            Alternation(choices=[
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='KEYWORD', is_token=True),
                RuleReference(name='ERROR_CONSTANT', is_token=True),
                RuleReference(name='array_constant', is_token=False),
            ]),
            line_number=45,
        ),
        GrammarRule(
            name='array_constant',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACE', is_token=True),
                RuleReference(name='ws', is_token=False),
                RuleReference(name='array_row', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='SEMICOLON', is_token=True),
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='array_row', is_token=False),
                    ]),
                ),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='SEMICOLON', is_token=True),
                    ]),
                ),
                RuleReference(name='ws', is_token=False),
                RuleReference(name='RBRACE', is_token=True),
            ]),
            line_number=47,
        ),
        GrammarRule(
            name='array_row',
            body=
            Sequence(elements=[
                RuleReference(name='array_item', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='array_item', is_token=False),
                    ]),
                ),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='COMMA', is_token=True),
                    ]),
                ),
            ]),
            line_number=48,
        ),
        GrammarRule(
            name='array_item',
            body=
            Alternation(choices=[
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='KEYWORD', is_token=True),
                RuleReference(name='ERROR_CONSTANT', is_token=True),
            ]),
            line_number=49,
        ),
        GrammarRule(
            name='function_call',
            body=
            Sequence(elements=[
                RuleReference(name='function_name', is_token=False),
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='ws', is_token=False),
                Optional(element=
                    RuleReference(name='function_argument_list', is_token=False),
                ),
                RuleReference(name='ws', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=51,
        ),
        GrammarRule(
            name='function_name',
            body=
            Alternation(choices=[
                RuleReference(name='FUNCTION_NAME', is_token=True),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=52,
        ),
        GrammarRule(
            name='function_argument_list',
            body=
            Sequence(elements=[
                RuleReference(name='function_argument', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='function_argument', is_token=False),
                    ]),
                ),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='COMMA', is_token=True),
                    ]),
                ),
            ]),
            line_number=53,
        ),
        GrammarRule(
            name='function_argument',
            body=
            Optional(element=
                RuleReference(name='expression', is_token=False),
            ),
            line_number=54,
        ),
        GrammarRule(
            name='reference_expression',
            body=
            RuleReference(name='union_reference', is_token=False),
            line_number=56,
        ),
        GrammarRule(
            name='union_reference',
            body=
            Sequence(elements=[
                RuleReference(name='intersection_reference', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='intersection_reference', is_token=False),
                    ]),
                ),
            ]),
            line_number=57,
        ),
        GrammarRule(
            name='intersection_reference',
            body=
            Sequence(elements=[
                RuleReference(name='range_reference', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='req_space', is_token=False),
                        RuleReference(name='range_reference', is_token=False),
                    ]),
                ),
            ]),
            line_number=58,
        ),
        GrammarRule(
            name='range_reference',
            body=
            Sequence(elements=[
                RuleReference(name='reference_primary', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='COLON', is_token=True),
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='reference_primary', is_token=False),
                    ]),
                ),
            ]),
            line_number=59,
        ),
        GrammarRule(
            name='reference_primary',
            body=
            Alternation(choices=[
                RuleReference(name='parenthesized_reference', is_token=False),
                RuleReference(name='prefixed_reference', is_token=False),
                RuleReference(name='external_reference', is_token=False),
                RuleReference(name='structure_reference', is_token=False),
                RuleReference(name='a1_reference', is_token=False),
                RuleReference(name='bang_reference', is_token=False),
                RuleReference(name='bang_name', is_token=False),
                RuleReference(name='name_reference', is_token=False),
            ]),
            line_number=61,
        ),
        GrammarRule(
            name='parenthesized_reference',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='ws', is_token=False),
                RuleReference(name='reference_expression', is_token=False),
                RuleReference(name='ws', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=70,
        ),
        GrammarRule(
            name='prefixed_reference',
            body=
            Sequence(elements=[
                RuleReference(name='REF_PREFIX', is_token=True),
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='a1_reference', is_token=False),
                        RuleReference(name='name_reference', is_token=False),
                        RuleReference(name='structure_reference', is_token=False),
                    ]),
                ),
            ]),
            line_number=71,
        ),
        GrammarRule(
            name='external_reference',
            body=
            RuleReference(name='REF_PREFIX', is_token=True),
            line_number=72,
        ),
        GrammarRule(
            name='bang_reference',
            body=
            Sequence(elements=[
                RuleReference(name='BANG', is_token=True),
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='CELL', is_token=True),
                        RuleReference(name='COLUMN_REF', is_token=True),
                        RuleReference(name='ROW_REF', is_token=True),
                        RuleReference(name='NUMBER', is_token=True),
                    ]),
                ),
            ]),
            line_number=73,
        ),
        GrammarRule(
            name='bang_name',
            body=
            Sequence(elements=[
                RuleReference(name='BANG', is_token=True),
                RuleReference(name='name_reference', is_token=False),
            ]),
            line_number=74,
        ),
        GrammarRule(
            name='name_reference',
            body=
            RuleReference(name='NAME', is_token=True),
            line_number=75,
        ),
        GrammarRule(
            name='column_reference',
            body=
            Sequence(elements=[
                Optional(element=
                    RuleReference(name='DOLLAR', is_token=True),
                ),
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='COLUMN_REF', is_token=True),
                        RuleReference(name='NAME', is_token=True),
                    ]),
                ),
            ]),
            line_number=77,
        ),
        GrammarRule(
            name='row_reference',
            body=
            Sequence(elements=[
                Optional(element=
                    RuleReference(name='DOLLAR', is_token=True),
                ),
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='ROW_REF', is_token=True),
                        RuleReference(name='NUMBER', is_token=True),
                    ]),
                ),
            ]),
            line_number=78,
        ),
        GrammarRule(
            name='a1_reference',
            body=
            Alternation(choices=[
                RuleReference(name='CELL', is_token=True),
                RuleReference(name='column_reference', is_token=False),
                RuleReference(name='row_reference', is_token=False),
                RuleReference(name='COLUMN_REF', is_token=True),
                RuleReference(name='ROW_REF', is_token=True),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='NUMBER', is_token=True),
            ]),
            line_number=80,
        ),
        GrammarRule(
            name='structure_reference',
            body=
            Sequence(elements=[
                Optional(element=
                    RuleReference(name='table_name', is_token=False),
                ),
                RuleReference(name='intra_table_reference', is_token=False),
            ]),
            line_number=82,
        ),
        GrammarRule(
            name='table_name',
            body=
            Alternation(choices=[
                RuleReference(name='TABLE_NAME', is_token=True),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=83,
        ),
        GrammarRule(
            name='intra_table_reference',
            body=
            Alternation(choices=[
                RuleReference(name='STRUCTURED_KEYWORD', is_token=True),
                RuleReference(name='structured_column_range', is_token=False),
                Sequence(elements=[
                    RuleReference(name='LBRACKET', is_token=True),
                    RuleReference(name='ws', is_token=False),
                    Optional(element=
                        RuleReference(name='inner_structure_reference', is_token=False),
                    ),
                    RuleReference(name='ws', is_token=False),
                    RuleReference(name='RBRACKET', is_token=True),
                ]),
            ]),
            line_number=84,
        ),
        GrammarRule(
            name='inner_structure_reference',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='structured_keyword_list', is_token=False),
                    Optional(element=
                        Sequence(elements=[
                            RuleReference(name='ws', is_token=False),
                            RuleReference(name='COMMA', is_token=True),
                            RuleReference(name='ws', is_token=False),
                            RuleReference(name='structured_column_range', is_token=False),
                        ]),
                    ),
                ]),
                RuleReference(name='structured_column_range', is_token=False),
            ]),
            line_number=87,
        ),
        GrammarRule(
            name='structured_keyword_list',
            body=
            Sequence(elements=[
                RuleReference(name='STRUCTURED_KEYWORD', is_token=True),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='STRUCTURED_KEYWORD', is_token=True),
                    ]),
                ),
            ]),
            line_number=89,
        ),
        GrammarRule(
            name='structured_column_range',
            body=
            Sequence(elements=[
                RuleReference(name='structured_column', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='COLON', is_token=True),
                        RuleReference(name='ws', is_token=False),
                        RuleReference(name='structured_column', is_token=False),
                    ]),
                ),
            ]),
            line_number=90,
        ),
        GrammarRule(
            name='structured_column',
            body=
            Alternation(choices=[
                RuleReference(name='STRUCTURED_COLUMN', is_token=True),
                Sequence(elements=[
                    RuleReference(name='AT', is_token=True),
                    RuleReference(name='STRUCTURED_COLUMN', is_token=True),
                ]),
            ]),
            line_number=91,
        ),
    ],
)
