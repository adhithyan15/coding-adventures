# AUTO-GENERATED FILE — DO NOT EDIT
# Source: starlark.grammar
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
    Optional,
    ParserGrammar,
    Repetition,
    RuleReference,
    Sequence,
)

# fmt: off  # noqa: E501 — generated code may have long lines

PARSER_GRAMMAR = ParserGrammar(
    version=1,
    rules=[
        GrammarRule(
            name='file',
            body=
            Repetition(element=
                Alternation(choices=[
                    RuleReference(name='NEWLINE', is_token=True),
                    RuleReference(name='statement', is_token=False),
                ]),
            ),
            line_number=34,
        ),
        GrammarRule(
            name='statement',
            body=
            Alternation(choices=[
                RuleReference(name='compound_stmt', is_token=False),
                RuleReference(name='simple_stmt', is_token=False),
            ]),
            line_number=48,
        ),
        GrammarRule(
            name='simple_stmt',
            body=
            Sequence(elements=[
                RuleReference(name='small_stmt', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='SEMICOLON', is_token=True),
                        RuleReference(name='small_stmt', is_token=False),
                    ]),
                ),
                RuleReference(name='NEWLINE', is_token=True),
            ]),
            line_number=52,
        ),
        GrammarRule(
            name='small_stmt',
            body=
            Alternation(choices=[
                RuleReference(name='return_stmt', is_token=False),
                RuleReference(name='break_stmt', is_token=False),
                RuleReference(name='continue_stmt', is_token=False),
                RuleReference(name='pass_stmt', is_token=False),
                RuleReference(name='load_stmt', is_token=False),
                RuleReference(name='assign_stmt', is_token=False),
            ]),
            line_number=54,
        ),
        GrammarRule(
            name='return_stmt',
            body=
            Sequence(elements=[
                Literal(value='return'),
                Optional(element=
                    RuleReference(name='expression', is_token=False),
                ),
            ]),
            line_number=68,
        ),
        GrammarRule(
            name='break_stmt',
            body=
            Literal(value='break'),
            line_number=71,
        ),
        GrammarRule(
            name='continue_stmt',
            body=
            Literal(value='continue'),
            line_number=74,
        ),
        GrammarRule(
            name='pass_stmt',
            body=
            Literal(value='pass'),
            line_number=79,
        ),
        GrammarRule(
            name='load_stmt',
            body=
            Sequence(elements=[
                Literal(value='load'),
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='STRING', is_token=True),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='load_arg', is_token=False),
                    ]),
                ),
                Optional(element=
                    RuleReference(name='COMMA', is_token=True),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=88,
        ),
        GrammarRule(
            name='load_arg',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='NAME', is_token=True),
                    RuleReference(name='EQUALS', is_token=True),
                    RuleReference(name='STRING', is_token=True),
                ]),
                RuleReference(name='STRING', is_token=True),
            ]),
            line_number=89,
        ),
        GrammarRule(
            name='assign_stmt',
            body=
            Sequence(elements=[
                RuleReference(name='expression_list', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='assign_op', is_token=False),
                                RuleReference(name='augmented_assign_op', is_token=False),
                            ]),
                        ),
                        RuleReference(name='expression_list', is_token=False),
                    ]),
                ),
            ]),
            line_number=110,
        ),
        GrammarRule(
            name='assign_op',
            body=
            RuleReference(name='EQUALS', is_token=True),
            line_number=113,
        ),
        GrammarRule(
            name='augmented_assign_op',
            body=
            Alternation(choices=[
                RuleReference(name='PLUS_EQUALS', is_token=True),
                RuleReference(name='MINUS_EQUALS', is_token=True),
                RuleReference(name='STAR_EQUALS', is_token=True),
                RuleReference(name='SLASH_EQUALS', is_token=True),
                RuleReference(name='FLOOR_DIV_EQUALS', is_token=True),
                RuleReference(name='PERCENT_EQUALS', is_token=True),
                RuleReference(name='AMP_EQUALS', is_token=True),
                RuleReference(name='PIPE_EQUALS', is_token=True),
                RuleReference(name='CARET_EQUALS', is_token=True),
                RuleReference(name='LEFT_SHIFT_EQUALS', is_token=True),
                RuleReference(name='RIGHT_SHIFT_EQUALS', is_token=True),
                RuleReference(name='DOUBLE_STAR_EQUALS', is_token=True),
            ]),
            line_number=115,
        ),
        GrammarRule(
            name='compound_stmt',
            body=
            Alternation(choices=[
                RuleReference(name='if_stmt', is_token=False),
                RuleReference(name='for_stmt', is_token=False),
                RuleReference(name='def_stmt', is_token=False),
            ]),
            line_number=124,
        ),
        GrammarRule(
            name='if_stmt',
            body=
            Sequence(elements=[
                Literal(value='if'),
                RuleReference(name='expression', is_token=False),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='suite', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='elif'),
                        RuleReference(name='expression', is_token=False),
                        RuleReference(name='COLON', is_token=True),
                        RuleReference(name='suite', is_token=False),
                    ]),
                ),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='else'),
                        RuleReference(name='COLON', is_token=True),
                        RuleReference(name='suite', is_token=False),
                    ]),
                ),
            ]),
            line_number=136,
        ),
        GrammarRule(
            name='for_stmt',
            body=
            Sequence(elements=[
                Literal(value='for'),
                RuleReference(name='loop_vars', is_token=False),
                Literal(value='in'),
                RuleReference(name='expression', is_token=False),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='suite', is_token=False),
            ]),
            line_number=150,
        ),
        GrammarRule(
            name='loop_vars',
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
            line_number=156,
        ),
        GrammarRule(
            name='def_stmt',
            body=
            Sequence(elements=[
                Literal(value='def'),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='LPAREN', is_token=True),
                Optional(element=
                    RuleReference(name='parameters', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='suite', is_token=False),
            ]),
            line_number=166,
        ),
        GrammarRule(
            name='suite',
            body=
            Alternation(choices=[
                RuleReference(name='simple_stmt', is_token=False),
                Sequence(elements=[
                    RuleReference(name='NEWLINE', is_token=True),
                    RuleReference(name='INDENT', is_token=True),
                    Repetition(element=
                        RuleReference(name='statement', is_token=False),
                    ),
                    RuleReference(name='DEDENT', is_token=True),
                ]),
            ]),
            line_number=177,
        ),
        GrammarRule(
            name='parameters',
            body=
            Sequence(elements=[
                RuleReference(name='parameter', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='parameter', is_token=False),
                    ]),
                ),
                Optional(element=
                    RuleReference(name='COMMA', is_token=True),
                ),
            ]),
            line_number=198,
        ),
        GrammarRule(
            name='parameter',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='DOUBLE_STAR', is_token=True),
                    RuleReference(name='NAME', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='STAR', is_token=True),
                    RuleReference(name='NAME', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='NAME', is_token=True),
                    RuleReference(name='EQUALS', is_token=True),
                    RuleReference(name='expression', is_token=False),
                ]),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=200,
        ),
        GrammarRule(
            name='expression_list',
            body=
            Sequence(elements=[
                RuleReference(name='expression', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='expression', is_token=False),
                    ]),
                ),
                Optional(element=
                    RuleReference(name='COMMA', is_token=True),
                ),
            ]),
            line_number=234,
        ),
        GrammarRule(
            name='expression',
            body=
            Alternation(choices=[
                RuleReference(name='lambda_expr', is_token=False),
                Sequence(elements=[
                    RuleReference(name='or_expr', is_token=False),
                    Optional(element=
                        Sequence(elements=[
                            Literal(value='if'),
                            RuleReference(name='or_expr', is_token=False),
                            Literal(value='else'),
                            RuleReference(name='expression', is_token=False),
                        ]),
                    ),
                ]),
            ]),
            line_number=239,
        ),
        GrammarRule(
            name='lambda_expr',
            body=
            Sequence(elements=[
                Literal(value='lambda'),
                Optional(element=
                    RuleReference(name='lambda_params', is_token=False),
                ),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=244,
        ),
        GrammarRule(
            name='lambda_params',
            body=
            Sequence(elements=[
                RuleReference(name='lambda_param', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='lambda_param', is_token=False),
                    ]),
                ),
                Optional(element=
                    RuleReference(name='COMMA', is_token=True),
                ),
            ]),
            line_number=245,
        ),
        GrammarRule(
            name='lambda_param',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='NAME', is_token=True),
                    Optional(element=
                        Sequence(elements=[
                            RuleReference(name='EQUALS', is_token=True),
                            RuleReference(name='expression', is_token=False),
                        ]),
                    ),
                ]),
                Sequence(elements=[
                    RuleReference(name='STAR', is_token=True),
                    RuleReference(name='NAME', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='DOUBLE_STAR', is_token=True),
                    RuleReference(name='NAME', is_token=True),
                ]),
            ]),
            line_number=246,
        ),
        GrammarRule(
            name='or_expr',
            body=
            Sequence(elements=[
                RuleReference(name='and_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='or'),
                        RuleReference(name='and_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=250,
        ),
        GrammarRule(
            name='and_expr',
            body=
            Sequence(elements=[
                RuleReference(name='not_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='and'),
                        RuleReference(name='not_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=254,
        ),
        GrammarRule(
            name='not_expr',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='not'),
                    RuleReference(name='not_expr', is_token=False),
                ]),
                RuleReference(name='comparison', is_token=False),
            ]),
            line_number=258,
        ),
        GrammarRule(
            name='comparison',
            body=
            Sequence(elements=[
                RuleReference(name='bitwise_or', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='comp_op', is_token=False),
                        RuleReference(name='bitwise_or', is_token=False),
                    ]),
                ),
            ]),
            line_number=267,
        ),
        GrammarRule(
            name='comp_op',
            body=
            Alternation(choices=[
                RuleReference(name='EQUALS_EQUALS', is_token=True),
                RuleReference(name='NOT_EQUALS', is_token=True),
                RuleReference(name='LESS_THAN', is_token=True),
                RuleReference(name='GREATER_THAN', is_token=True),
                RuleReference(name='LESS_EQUALS', is_token=True),
                RuleReference(name='GREATER_EQUALS', is_token=True),
                Literal(value='in'),
                Sequence(elements=[
                    Literal(value='not'),
                    Literal(value='in'),
                ]),
            ]),
            line_number=269,
        ),
        GrammarRule(
            name='bitwise_or',
            body=
            Sequence(elements=[
                RuleReference(name='bitwise_xor', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='PIPE', is_token=True),
                        RuleReference(name='bitwise_xor', is_token=False),
                    ]),
                ),
            ]),
            line_number=275,
        ),
        GrammarRule(
            name='bitwise_xor',
            body=
            Sequence(elements=[
                RuleReference(name='bitwise_and', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='CARET', is_token=True),
                        RuleReference(name='bitwise_and', is_token=False),
                    ]),
                ),
            ]),
            line_number=276,
        ),
        GrammarRule(
            name='bitwise_and',
            body=
            Sequence(elements=[
                RuleReference(name='shift', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='AMP', is_token=True),
                        RuleReference(name='shift', is_token=False),
                    ]),
                ),
            ]),
            line_number=277,
        ),
        GrammarRule(
            name='shift',
            body=
            Sequence(elements=[
                RuleReference(name='arith', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='LEFT_SHIFT', is_token=True),
                                RuleReference(name='RIGHT_SHIFT', is_token=True),
                            ]),
                        ),
                        RuleReference(name='arith', is_token=False),
                    ]),
                ),
            ]),
            line_number=280,
        ),
        GrammarRule(
            name='arith',
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
            line_number=284,
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
                                RuleReference(name='FLOOR_DIV', is_token=True),
                                RuleReference(name='PERCENT', is_token=True),
                            ]),
                        ),
                        RuleReference(name='factor', is_token=False),
                    ]),
                ),
            ]),
            line_number=289,
        ),
        GrammarRule(
            name='factor',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Group(element=
                        Alternation(choices=[
                            RuleReference(name='PLUS', is_token=True),
                            RuleReference(name='MINUS', is_token=True),
                            RuleReference(name='TILDE', is_token=True),
                        ]),
                    ),
                    RuleReference(name='factor', is_token=False),
                ]),
                RuleReference(name='power', is_token=False),
            ]),
            line_number=295,
        ),
        GrammarRule(
            name='power',
            body=
            Sequence(elements=[
                RuleReference(name='primary', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='DOUBLE_STAR', is_token=True),
                        RuleReference(name='factor', is_token=False),
                    ]),
                ),
            ]),
            line_number=303,
        ),
        GrammarRule(
            name='primary',
            body=
            Sequence(elements=[
                RuleReference(name='atom', is_token=False),
                Repetition(element=
                    RuleReference(name='suffix', is_token=False),
                ),
            ]),
            line_number=320,
        ),
        GrammarRule(
            name='suffix',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='DOT', is_token=True),
                    RuleReference(name='NAME', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='LBRACKET', is_token=True),
                    RuleReference(name='subscript', is_token=False),
                    RuleReference(name='RBRACKET', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    Optional(element=
                        RuleReference(name='arguments', is_token=False),
                    ),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
            ]),
            line_number=322,
        ),
        GrammarRule(
            name='subscript',
            body=
            Alternation(choices=[
                RuleReference(name='expression', is_token=False),
                Sequence(elements=[
                    Optional(element=
                        RuleReference(name='expression', is_token=False),
                    ),
                    RuleReference(name='COLON', is_token=True),
                    Optional(element=
                        RuleReference(name='expression', is_token=False),
                    ),
                    Optional(element=
                        Sequence(elements=[
                            RuleReference(name='COLON', is_token=True),
                            Optional(element=
                                RuleReference(name='expression', is_token=False),
                            ),
                        ]),
                    ),
                ]),
            ]),
            line_number=334,
        ),
        GrammarRule(
            name='atom',
            body=
            Alternation(choices=[
                RuleReference(name='INT', is_token=True),
                RuleReference(name='FLOAT', is_token=True),
                Sequence(elements=[
                    RuleReference(name='STRING', is_token=True),
                    Repetition(element=
                        RuleReference(name='STRING', is_token=True),
                    ),
                ]),
                RuleReference(name='NAME', is_token=True),
                Literal(value='True'),
                Literal(value='False'),
                Literal(value='None'),
                RuleReference(name='list_expr', is_token=False),
                RuleReference(name='dict_expr', is_token=False),
                RuleReference(name='paren_expr', is_token=False),
            ]),
            line_number=343,
        ),
        GrammarRule(
            name='list_expr',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACKET', is_token=True),
                Optional(element=
                    RuleReference(name='list_body', is_token=False),
                ),
                RuleReference(name='RBRACKET', is_token=True),
            ]),
            line_number=359,
        ),
        GrammarRule(
            name='list_body',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='expression', is_token=False),
                    RuleReference(name='comp_clause', is_token=False),
                ]),
                Sequence(elements=[
                    RuleReference(name='expression', is_token=False),
                    Repetition(element=
                        Sequence(elements=[
                            RuleReference(name='COMMA', is_token=True),
                            RuleReference(name='expression', is_token=False),
                        ]),
                    ),
                    Optional(element=
                        RuleReference(name='COMMA', is_token=True),
                    ),
                ]),
            ]),
            line_number=361,
        ),
        GrammarRule(
            name='dict_expr',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACE', is_token=True),
                Optional(element=
                    RuleReference(name='dict_body', is_token=False),
                ),
                RuleReference(name='RBRACE', is_token=True),
            ]),
            line_number=367,
        ),
        GrammarRule(
            name='dict_body',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='dict_entry', is_token=False),
                    RuleReference(name='comp_clause', is_token=False),
                ]),
                Sequence(elements=[
                    RuleReference(name='dict_entry', is_token=False),
                    Repetition(element=
                        Sequence(elements=[
                            RuleReference(name='COMMA', is_token=True),
                            RuleReference(name='dict_entry', is_token=False),
                        ]),
                    ),
                    Optional(element=
                        RuleReference(name='COMMA', is_token=True),
                    ),
                ]),
            ]),
            line_number=369,
        ),
        GrammarRule(
            name='dict_entry',
            body=
            Sequence(elements=[
                RuleReference(name='expression', is_token=False),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=372,
        ),
        GrammarRule(
            name='paren_expr',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                Optional(element=
                    RuleReference(name='paren_body', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=379,
        ),
        GrammarRule(
            name='paren_body',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='expression', is_token=False),
                    RuleReference(name='comp_clause', is_token=False),
                ]),
                Sequence(elements=[
                    RuleReference(name='expression', is_token=False),
                    RuleReference(name='COMMA', is_token=True),
                    Optional(element=
                        Sequence(elements=[
                            RuleReference(name='expression', is_token=False),
                            Repetition(element=
                                Sequence(elements=[
                                    RuleReference(name='COMMA', is_token=True),
                                    RuleReference(name='expression', is_token=False),
                                ]),
                            ),
                            Optional(element=
                                RuleReference(name='COMMA', is_token=True),
                            ),
                        ]),
                    ),
                ]),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=381,
        ),
        GrammarRule(
            name='comp_clause',
            body=
            Sequence(elements=[
                RuleReference(name='comp_for', is_token=False),
                Repetition(element=
                    Alternation(choices=[
                        RuleReference(name='comp_for', is_token=False),
                        RuleReference(name='comp_if', is_token=False),
                    ]),
                ),
            ]),
            line_number=397,
        ),
        GrammarRule(
            name='comp_for',
            body=
            Sequence(elements=[
                Literal(value='for'),
                RuleReference(name='loop_vars', is_token=False),
                Literal(value='in'),
                RuleReference(name='or_expr', is_token=False),
            ]),
            line_number=399,
        ),
        GrammarRule(
            name='comp_if',
            body=
            Sequence(elements=[
                Literal(value='if'),
                RuleReference(name='or_expr', is_token=False),
            ]),
            line_number=401,
        ),
        GrammarRule(
            name='arguments',
            body=
            Sequence(elements=[
                RuleReference(name='argument', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='argument', is_token=False),
                    ]),
                ),
                Optional(element=
                    RuleReference(name='COMMA', is_token=True),
                ),
            ]),
            line_number=420,
        ),
        GrammarRule(
            name='argument',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='DOUBLE_STAR', is_token=True),
                    RuleReference(name='expression', is_token=False),
                ]),
                Sequence(elements=[
                    RuleReference(name='STAR', is_token=True),
                    RuleReference(name='expression', is_token=False),
                ]),
                Sequence(elements=[
                    RuleReference(name='NAME', is_token=True),
                    RuleReference(name='EQUALS', is_token=True),
                    RuleReference(name='expression', is_token=False),
                ]),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=422,
        ),
    ],
)
