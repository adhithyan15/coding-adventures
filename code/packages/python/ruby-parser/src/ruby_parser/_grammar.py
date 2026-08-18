# AUTO-GENERATED FILE — DO NOT EDIT
# ruff: noqa: E501, F401
# Source: ruby.grammar
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
            line_number=27,
        ),
        GrammarRule(
            name='statement',
            body=
            Alternation(choices=[
                RuleReference(name='endless_def_statement', is_token=False),
                RuleReference(name='def_statement', is_token=False),
                RuleReference(name='class_statement', is_token=False),
                RuleReference(name='module_statement', is_token=False),
                RuleReference(name='if_statement', is_token=False),
                RuleReference(name='unless_statement', is_token=False),
                RuleReference(name='while_statement', is_token=False),
                RuleReference(name='until_statement', is_token=False),
                RuleReference(name='case_statement', is_token=False),
                RuleReference(name='begin_statement', is_token=False),
                RuleReference(name='return_statement', is_token=False),
                RuleReference(name='break_statement', is_token=False),
                RuleReference(name='next_statement', is_token=False),
                RuleReference(name='redo_statement', is_token=False),
                RuleReference(name='retry_statement', is_token=False),
                RuleReference(name='yield_statement', is_token=False),
                RuleReference(name='alias_statement', is_token=False),
                RuleReference(name='undef_statement', is_token=False),
                RuleReference(name='multi_assignment', is_token=False),
                RuleReference(name='modifier_statement', is_token=False),
                RuleReference(name='rightward_assignment', is_token=False),
                RuleReference(name='index_assignment', is_token=False),
                RuleReference(name='assignment', is_token=False),
                RuleReference(name='defined_expression', is_token=False),
                RuleReference(name='method_with_block', is_token=False),
                RuleReference(name='method_call', is_token=False),
                RuleReference(name='method_call_no_paren', is_token=False),
                RuleReference(name='expression_stmt', is_token=False),
            ]),
            line_number=28,
        ),
        GrammarRule(
            name='multi_assignment',
            body=
            Sequence(elements=[
                RuleReference(name='mlhs_target', is_token=False),
                RuleReference(name='COMMA', is_token=True),
                RuleReference(name='mlhs_target', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='mlhs_target', is_token=False),
                    ]),
                ),
                RuleReference(name='EQUALS', is_token=True),
                RuleReference(name='expression', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='expression', is_token=False),
                    ]),
                ),
            ]),
            line_number=71,
        ),
        GrammarRule(
            name='mlhs_target',
            body=
            Sequence(elements=[
                Optional(element=
                    Literal(value='*'),
                ),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=72,
        ),
        GrammarRule(
            name='modifier_statement',
            body=
            Sequence(elements=[
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='assignment', is_token=False),
                        RuleReference(name='method_call_no_paren', is_token=False),
                        RuleReference(name='method_call', is_token=False),
                        RuleReference(name='expression_stmt', is_token=False),
                    ]),
                ),
                Group(element=
                    Alternation(choices=[
                        Literal(value='if_modifier'),
                        Literal(value='unless_modifier'),
                        Literal(value='while_modifier'),
                        Literal(value='until_modifier'),
                    ]),
                ),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=108,
        ),
        GrammarRule(
            name='def_statement',
            body=
            Sequence(elements=[
                Literal(value='def'),
                Optional(element=
                    RuleReference(name='def_receiver', is_token=False),
                ),
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='LPAREN', is_token=True),
                        Optional(element=
                            RuleReference(name='params', is_token=False),
                        ),
                        RuleReference(name='RPAREN', is_token=True),
                    ]),
                ),
                Repetition(element=
                    Sequence(elements=[
                        NegativeLookahead(element=
                            Literal(value='rescue'),
                        ),
                        NegativeLookahead(element=
                            Literal(value='ensure'),
                        ),
                        NegativeLookahead(element=
                            Literal(value='end'),
                        ),
                        RuleReference(name='statement', is_token=False),
                    ]),
                ),
                Repetition(element=
                    RuleReference(name='rescue_clause', is_token=False),
                ),
                Optional(element=
                    RuleReference(name='ensure_clause', is_token=False),
                ),
                Literal(value='end'),
            ]),
            line_number=132,
        ),
        GrammarRule(
            name='def_receiver',
            body=
            Sequence(elements=[
                RuleReference(name='singleton_receiver', is_token=False),
                Literal(value='.'),
            ]),
            line_number=138,
        ),
        GrammarRule(
            name='endless_def_statement',
            body=
            Sequence(elements=[
                Literal(value='def'),
                Optional(element=
                    RuleReference(name='def_receiver', is_token=False),
                ),
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='LPAREN', is_token=True),
                        Optional(element=
                            RuleReference(name='params', is_token=False),
                        ),
                        RuleReference(name='RPAREN', is_token=True),
                    ]),
                ),
                RuleReference(name='EQUALS', is_token=True),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=147,
        ),
        GrammarRule(
            name='class_statement',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='class'),
                    Literal(value='<<'),
                    RuleReference(name='singleton_receiver', is_token=False),
                    Repetition(element=
                        Sequence(elements=[
                            NegativeLookahead(element=
                                Literal(value='end'),
                            ),
                            RuleReference(name='statement', is_token=False),
                        ]),
                    ),
                    Literal(value='end'),
                ]),
                Sequence(elements=[
                    Literal(value='class'),
                    RuleReference(name='NAME', is_token=True),
                    Optional(element=
                        Sequence(elements=[
                            Literal(value='<'),
                            RuleReference(name='NAME', is_token=True),
                        ]),
                    ),
                    Repetition(element=
                        Sequence(elements=[
                            NegativeLookahead(element=
                                Literal(value='end'),
                            ),
                            RuleReference(name='statement', is_token=False),
                        ]),
                    ),
                    Literal(value='end'),
                ]),
            ]),
            line_number=168,
        ),
        GrammarRule(
            name='singleton_receiver',
            body=
            Alternation(choices=[
                Literal(value='self'),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=170,
        ),
        GrammarRule(
            name='module_statement',
            body=
            Sequence(elements=[
                Literal(value='module'),
                RuleReference(name='NAME', is_token=True),
                Repetition(element=
                    Sequence(elements=[
                        NegativeLookahead(element=
                            Literal(value='end'),
                        ),
                        RuleReference(name='statement', is_token=False),
                    ]),
                ),
                Literal(value='end'),
            ]),
            line_number=171,
        ),
        GrammarRule(
            name='method_with_block',
            body=
            Sequence(elements=[
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='NAME', is_token=True),
                        RuleReference(name='KEYWORD', is_token=True),
                    ]),
                ),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='LPAREN', is_token=True),
                        Optional(element=
                            Sequence(elements=[
                                RuleReference(name='expression', is_token=False),
                                Repetition(element=
                                    Sequence(elements=[
                                        RuleReference(name='COMMA', is_token=True),
                                        RuleReference(name='expression', is_token=False),
                                    ]),
                                ),
                            ]),
                        ),
                        RuleReference(name='RPAREN', is_token=True),
                    ]),
                ),
                RuleReference(name='block', is_token=False),
            ]),
            line_number=173,
        ),
        GrammarRule(
            name='block',
            body=
            Alternation(choices=[
                RuleReference(name='do_block', is_token=False),
                RuleReference(name='brace_block', is_token=False),
            ]),
            line_number=174,
        ),
        GrammarRule(
            name='do_block',
            body=
            Sequence(elements=[
                Literal(value='do'),
                Optional(element=
                    RuleReference(name='block_params', is_token=False),
                ),
                Repetition(element=
                    Sequence(elements=[
                        NegativeLookahead(element=
                            Literal(value='end'),
                        ),
                        RuleReference(name='statement', is_token=False),
                    ]),
                ),
                Literal(value='end'),
            ]),
            line_number=175,
        ),
        GrammarRule(
            name='brace_block',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACE', is_token=True),
                Optional(element=
                    RuleReference(name='block_params', is_token=False),
                ),
                Repetition(element=
                    RuleReference(name='statement', is_token=False),
                ),
                RuleReference(name='RBRACE', is_token=True),
            ]),
            line_number=176,
        ),
        GrammarRule(
            name='block_params',
            body=
            Sequence(elements=[
                Literal(value='|'),
                RuleReference(name='NAME', is_token=True),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='NAME', is_token=True),
                    ]),
                ),
                Optional(element=
                    Sequence(elements=[
                        Literal(value=';'),
                        RuleReference(name='NAME', is_token=True),
                        Repetition(element=
                            Sequence(elements=[
                                RuleReference(name='COMMA', is_token=True),
                                RuleReference(name='NAME', is_token=True),
                            ]),
                        ),
                    ]),
                ),
                Literal(value='|'),
            ]),
            line_number=186,
        ),
        GrammarRule(
            name='return_statement',
            body=
            Sequence(elements=[
                Literal(value='return'),
                Optional(element=
                    RuleReference(name='expression', is_token=False),
                ),
            ]),
            line_number=188,
        ),
        GrammarRule(
            name='break_statement',
            body=
            Sequence(elements=[
                Literal(value='break'),
                Optional(element=
                    RuleReference(name='expression', is_token=False),
                ),
            ]),
            line_number=189,
        ),
        GrammarRule(
            name='next_statement',
            body=
            Sequence(elements=[
                Literal(value='next'),
                Optional(element=
                    RuleReference(name='expression', is_token=False),
                ),
            ]),
            line_number=190,
        ),
        GrammarRule(
            name='redo_statement',
            body=
            Literal(value='redo'),
            line_number=194,
        ),
        GrammarRule(
            name='retry_statement',
            body=
            Literal(value='retry'),
            line_number=198,
        ),
        GrammarRule(
            name='alias_statement',
            body=
            Sequence(elements=[
                Literal(value='alias'),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=209,
        ),
        GrammarRule(
            name='undef_statement',
            body=
            Sequence(elements=[
                Literal(value='undef'),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=221,
        ),
        GrammarRule(
            name='yield_statement',
            body=
            Sequence(elements=[
                Literal(value='yield'),
                Optional(element=
                    RuleReference(name='yield_args', is_token=False),
                ),
            ]),
            line_number=243,
        ),
        GrammarRule(
            name='yield_args',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    Optional(element=
                        Sequence(elements=[
                            RuleReference(name='call_arg', is_token=False),
                            Repetition(element=
                                Sequence(elements=[
                                    RuleReference(name='COMMA', is_token=True),
                                    RuleReference(name='call_arg', is_token=False),
                                ]),
                            ),
                        ]),
                    ),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='call_arg', is_token=False),
                    Repetition(element=
                        Sequence(elements=[
                            RuleReference(name='COMMA', is_token=True),
                            RuleReference(name='call_arg', is_token=False),
                        ]),
                    ),
                ]),
            ]),
            line_number=244,
        ),
        GrammarRule(
            name='super_args',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    Optional(element=
                        Sequence(elements=[
                            RuleReference(name='call_arg', is_token=False),
                            Repetition(element=
                                Sequence(elements=[
                                    RuleReference(name='COMMA', is_token=True),
                                    RuleReference(name='call_arg', is_token=False),
                                ]),
                            ),
                        ]),
                    ),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='call_arg', is_token=False),
                    Repetition(element=
                        Sequence(elements=[
                            RuleReference(name='COMMA', is_token=True),
                            RuleReference(name='call_arg', is_token=False),
                        ]),
                    ),
                ]),
            ]),
            line_number=271,
        ),
        GrammarRule(
            name='params',
            body=
            Alternation(choices=[
                Literal(value='...'),
                Sequence(elements=[
                    RuleReference(name='param', is_token=False),
                    Repetition(element=
                        Sequence(elements=[
                            RuleReference(name='COMMA', is_token=True),
                            RuleReference(name='param', is_token=False),
                        ]),
                    ),
                ]),
            ]),
            line_number=300,
        ),
        GrammarRule(
            name='param',
            body=
            Sequence(elements=[
                Optional(element=
                    Alternation(choices=[
                        Literal(value='*'),
                        Literal(value='**'),
                    ]),
                ),
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    Alternation(choices=[
                        Sequence(elements=[
                            RuleReference(name='COLON', is_token=True),
                            Optional(element=
                                RuleReference(name='expression', is_token=False),
                            ),
                        ]),
                        Sequence(elements=[
                            RuleReference(name='EQUALS', is_token=True),
                            RuleReference(name='expression', is_token=False),
                        ]),
                    ]),
                ),
            ]),
            line_number=345,
        ),
        GrammarRule(
            name='if_statement',
            body=
            Sequence(elements=[
                Literal(value='if'),
                RuleReference(name='expression', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        NegativeLookahead(element=
                            Literal(value='else'),
                        ),
                        NegativeLookahead(element=
                            Literal(value='elsif'),
                        ),
                        NegativeLookahead(element=
                            Literal(value='end'),
                        ),
                        RuleReference(name='statement', is_token=False),
                    ]),
                ),
                Repetition(element=
                    RuleReference(name='elsif_clause', is_token=False),
                ),
                Optional(element=
                    RuleReference(name='else_clause', is_token=False),
                ),
                Literal(value='end'),
            ]),
            line_number=346,
        ),
        GrammarRule(
            name='elsif_clause',
            body=
            Sequence(elements=[
                Literal(value='elsif'),
                RuleReference(name='expression', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        NegativeLookahead(element=
                            Literal(value='else'),
                        ),
                        NegativeLookahead(element=
                            Literal(value='elsif'),
                        ),
                        NegativeLookahead(element=
                            Literal(value='end'),
                        ),
                        RuleReference(name='statement', is_token=False),
                    ]),
                ),
            ]),
            line_number=347,
        ),
        GrammarRule(
            name='else_clause',
            body=
            Sequence(elements=[
                Literal(value='else'),
                Repetition(element=
                    Sequence(elements=[
                        NegativeLookahead(element=
                            Literal(value='end'),
                        ),
                        RuleReference(name='statement', is_token=False),
                    ]),
                ),
            ]),
            line_number=348,
        ),
        GrammarRule(
            name='unless_statement',
            body=
            Sequence(elements=[
                Literal(value='unless'),
                RuleReference(name='expression', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        NegativeLookahead(element=
                            Literal(value='else'),
                        ),
                        NegativeLookahead(element=
                            Literal(value='end'),
                        ),
                        RuleReference(name='statement', is_token=False),
                    ]),
                ),
                Optional(element=
                    RuleReference(name='else_clause', is_token=False),
                ),
                Literal(value='end'),
            ]),
            line_number=349,
        ),
        GrammarRule(
            name='while_statement',
            body=
            Sequence(elements=[
                Literal(value='while'),
                RuleReference(name='expression', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        NegativeLookahead(element=
                            Literal(value='end'),
                        ),
                        RuleReference(name='statement', is_token=False),
                    ]),
                ),
                Literal(value='end'),
            ]),
            line_number=350,
        ),
        GrammarRule(
            name='until_statement',
            body=
            Sequence(elements=[
                Literal(value='until'),
                RuleReference(name='expression', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        NegativeLookahead(element=
                            Literal(value='end'),
                        ),
                        RuleReference(name='statement', is_token=False),
                    ]),
                ),
                Literal(value='end'),
            ]),
            line_number=351,
        ),
        GrammarRule(
            name='case_statement',
            body=
            Sequence(elements=[
                Literal(value='case'),
                RuleReference(name='expression', is_token=False),
                Repetition(element=
                    Alternation(choices=[
                        RuleReference(name='when_clause', is_token=False),
                        RuleReference(name='in_clause', is_token=False),
                    ]),
                ),
                Optional(element=
                    RuleReference(name='else_clause', is_token=False),
                ),
                Literal(value='end'),
            ]),
            line_number=374,
        ),
        GrammarRule(
            name='when_clause',
            body=
            Sequence(elements=[
                Literal(value='when'),
                RuleReference(name='expression', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='expression', is_token=False),
                    ]),
                ),
                Repetition(element=
                    Sequence(elements=[
                        NegativeLookahead(element=
                            Literal(value='when'),
                        ),
                        NegativeLookahead(element=
                            Literal(value='in'),
                        ),
                        NegativeLookahead(element=
                            Literal(value='else'),
                        ),
                        NegativeLookahead(element=
                            Literal(value='end'),
                        ),
                        RuleReference(name='statement', is_token=False),
                    ]),
                ),
            ]),
            line_number=375,
        ),
        GrammarRule(
            name='in_clause',
            body=
            Sequence(elements=[
                Literal(value='in'),
                RuleReference(name='pattern', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        NegativeLookahead(element=
                            Literal(value='when'),
                        ),
                        NegativeLookahead(element=
                            Literal(value='in'),
                        ),
                        NegativeLookahead(element=
                            Literal(value='else'),
                        ),
                        NegativeLookahead(element=
                            Literal(value='end'),
                        ),
                        RuleReference(name='statement', is_token=False),
                    ]),
                ),
            ]),
            line_number=397,
        ),
        GrammarRule(
            name='pattern',
            body=
            Alternation(choices=[
                RuleReference(name='array_pattern', is_token=False),
                RuleReference(name='hash_pattern', is_token=False),
                RuleReference(name='class_pattern', is_token=False),
                RuleReference(name='pin_pattern', is_token=False),
                RuleReference(name='literal_pattern', is_token=False),
                RuleReference(name='binding_pattern', is_token=False),
            ]),
            line_number=398,
        ),
        GrammarRule(
            name='literal_pattern',
            body=
            Alternation(choices=[
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='symbol_literal', is_token=False),
                RuleReference(name='KEYWORD', is_token=True),
            ]),
            line_number=399,
        ),
        GrammarRule(
            name='binding_pattern',
            body=
            RuleReference(name='NAME', is_token=True),
            line_number=400,
        ),
        GrammarRule(
            name='array_pattern',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACKET', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='splat_pattern', is_token=False),
                                RuleReference(name='pattern', is_token=False),
                            ]),
                        ),
                        Repetition(element=
                            Sequence(elements=[
                                RuleReference(name='COMMA', is_token=True),
                                Group(element=
                                    Alternation(choices=[
                                        RuleReference(name='splat_pattern', is_token=False),
                                        RuleReference(name='pattern', is_token=False),
                                    ]),
                                ),
                            ]),
                        ),
                    ]),
                ),
                RuleReference(name='RBRACKET', is_token=True),
            ]),
            line_number=401,
        ),
        GrammarRule(
            name='hash_pattern',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACE', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='hash_pattern_pair', is_token=False),
                        Repetition(element=
                            Sequence(elements=[
                                RuleReference(name='COMMA', is_token=True),
                                RuleReference(name='hash_pattern_pair', is_token=False),
                            ]),
                        ),
                    ]),
                ),
                RuleReference(name='RBRACE', is_token=True),
            ]),
            line_number=402,
        ),
        GrammarRule(
            name='hash_pattern_pair',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='COLON', is_token=True),
                Optional(element=
                    RuleReference(name='pattern', is_token=False),
                ),
            ]),
            line_number=403,
        ),
        GrammarRule(
            name='splat_pattern',
            body=
            Sequence(elements=[
                Literal(value='*'),
                Optional(element=
                    RuleReference(name='NAME', is_token=True),
                ),
            ]),
            line_number=410,
        ),
        GrammarRule(
            name='pin_pattern',
            body=
            Sequence(elements=[
                Literal(value='^'),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=415,
        ),
        GrammarRule(
            name='class_pattern',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='LPAREN', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='pattern', is_token=False),
                        Repetition(element=
                            Sequence(elements=[
                                RuleReference(name='COMMA', is_token=True),
                                RuleReference(name='pattern', is_token=False),
                            ]),
                        ),
                    ]),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=421,
        ),
        GrammarRule(
            name='begin_statement',
            body=
            Sequence(elements=[
                Literal(value='begin'),
                Repetition(element=
                    Sequence(elements=[
                        NegativeLookahead(element=
                            Literal(value='rescue'),
                        ),
                        NegativeLookahead(element=
                            Literal(value='ensure'),
                        ),
                        NegativeLookahead(element=
                            Literal(value='end'),
                        ),
                        RuleReference(name='statement', is_token=False),
                    ]),
                ),
                Repetition(element=
                    RuleReference(name='rescue_clause', is_token=False),
                ),
                Optional(element=
                    RuleReference(name='ensure_clause', is_token=False),
                ),
                Literal(value='end'),
            ]),
            line_number=442,
        ),
        GrammarRule(
            name='rescue_clause',
            body=
            Sequence(elements=[
                Literal(value='rescue'),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='exception_list', is_token=False),
                        Literal(value='=>'),
                        RuleReference(name='NAME', is_token=True),
                    ]),
                ),
                Repetition(element=
                    Sequence(elements=[
                        NegativeLookahead(element=
                            Literal(value='rescue'),
                        ),
                        NegativeLookahead(element=
                            Literal(value='ensure'),
                        ),
                        NegativeLookahead(element=
                            Literal(value='end'),
                        ),
                        RuleReference(name='statement', is_token=False),
                    ]),
                ),
            ]),
            line_number=451,
        ),
        GrammarRule(
            name='exception_list',
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
            line_number=452,
        ),
        GrammarRule(
            name='ensure_clause',
            body=
            Sequence(elements=[
                Literal(value='ensure'),
                Repetition(element=
                    Sequence(elements=[
                        NegativeLookahead(element=
                            Literal(value='end'),
                        ),
                        RuleReference(name='statement', is_token=False),
                    ]),
                ),
            ]),
            line_number=453,
        ),
        GrammarRule(
            name='index_write_receiver_postfix',
            body=
            Alternation(choices=[
                RuleReference(name='dot_call', is_token=False),
                RuleReference(name='scope_resolution', is_token=False),
                RuleReference(name='index_suffix', is_token=False),
            ]),
            line_number=506,
        ),
        GrammarRule(
            name='index_assignment',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='index_write_receiver_postfix', is_token=False),
                        PositiveLookahead(element=
                            RuleReference(name='index_write_receiver_postfix', is_token=False),
                        ),
                    ]),
                ),
                RuleReference(name='index_suffix', is_token=False),
                RuleReference(name='EQUALS', is_token=True),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=507,
        ),
        GrammarRule(
            name='assignment',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='EQUALS', is_token=True),
                        Literal(value='+='),
                        Literal(value='-='),
                        Literal(value='*='),
                        Literal(value='/='),
                        Literal(value='%='),
                        Literal(value='**='),
                        Literal(value='<<='),
                        Literal(value='>>='),
                        Literal(value='&='),
                        Literal(value='|='),
                        Literal(value='^='),
                        Literal(value='||='),
                        Literal(value='&&='),
                    ]),
                ),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=508,
        ),
        GrammarRule(
            name='rightward_assignment',
            body=
            Sequence(elements=[
                RuleReference(name='expression', is_token=False),
                Literal(value='=>'),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=527,
        ),
        GrammarRule(
            name='method_call',
            body=
            Sequence(elements=[
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='NAME', is_token=True),
                        Sequence(elements=[
                            NegativeLookahead(element=
                                Literal(value='super'),
                            ),
                            RuleReference(name='KEYWORD', is_token=True),
                        ]),
                    ]),
                ),
                RuleReference(name='LPAREN', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='call_arg', is_token=False),
                        Repetition(element=
                            Sequence(elements=[
                                RuleReference(name='COMMA', is_token=True),
                                RuleReference(name='call_arg', is_token=False),
                            ]),
                        ),
                    ]),
                ),
                RuleReference(name='RPAREN', is_token=True),
                Repetition(element=
                    RuleReference(name='dot_call', is_token=False),
                ),
            ]),
            line_number=544,
        ),
        GrammarRule(
            name='dot_call',
            body=
            Sequence(elements=[
                Literal(value='.'),
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='NAME', is_token=True),
                        RuleReference(name='KEYWORD', is_token=True),
                    ]),
                ),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='LPAREN', is_token=True),
                        Optional(element=
                            Sequence(elements=[
                                RuleReference(name='call_arg', is_token=False),
                                Repetition(element=
                                    Sequence(elements=[
                                        RuleReference(name='COMMA', is_token=True),
                                        RuleReference(name='call_arg', is_token=False),
                                    ]),
                                ),
                            ]),
                        ),
                        RuleReference(name='RPAREN', is_token=True),
                    ]),
                ),
                Optional(element=
                    RuleReference(name='block', is_token=False),
                ),
            ]),
            line_number=545,
        ),
        GrammarRule(
            name='scope_resolution',
            body=
            Sequence(elements=[
                Literal(value='::'),
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='NAME', is_token=True),
                        RuleReference(name='KEYWORD', is_token=True),
                    ]),
                ),
            ]),
            line_number=553,
        ),
        GrammarRule(
            name='call_arg',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='NAME', is_token=True),
                    RuleReference(name='COLON', is_token=True),
                    RuleReference(name='expression', is_token=False),
                ]),
                Sequence(elements=[
                    Optional(element=
                        Alternation(choices=[
                            Literal(value='*'),
                            Literal(value='**'),
                            Literal(value='&'),
                        ]),
                    ),
                    RuleReference(name='expression', is_token=False),
                ]),
            ]),
            line_number=608,
        ),
        GrammarRule(
            name='method_call_no_paren',
            body=
            Sequence(elements=[
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='NAME', is_token=True),
                        Sequence(elements=[
                            NegativeLookahead(element=
                                Literal(value='super'),
                            ),
                            RuleReference(name='KEYWORD', is_token=True),
                        ]),
                    ]),
                ),
                NegativeLookahead(element=
                    Literal(value='<'),
                ),
                NegativeLookahead(element=
                    Literal(value='>'),
                ),
                NegativeLookahead(element=
                    Literal(value='<='),
                ),
                NegativeLookahead(element=
                    Literal(value='>='),
                ),
                NegativeLookahead(element=
                    Literal(value='!='),
                ),
                NegativeLookahead(element=
                    Literal(value='&&'),
                ),
                NegativeLookahead(element=
                    Literal(value='||'),
                ),
                NegativeLookahead(element=
                    Literal(value='<<'),
                ),
                RuleReference(name='expression', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='expression', is_token=False),
                    ]),
                ),
            ]),
            line_number=656,
        ),
        GrammarRule(
            name='expression_stmt',
            body=
            RuleReference(name='expression', is_token=False),
            line_number=659,
        ),
        GrammarRule(
            name='expression',
            body=
            RuleReference(name='ternary', is_token=False),
            line_number=766,
        ),
        GrammarRule(
            name='ternary',
            body=
            Sequence(elements=[
                RuleReference(name='range', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='?'),
                        RuleReference(name='expression', is_token=False),
                        Literal(value=':'),
                        RuleReference(name='expression', is_token=False),
                    ]),
                ),
            ]),
            line_number=767,
        ),
        GrammarRule(
            name='range',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Group(element=
                        Alternation(choices=[
                            Literal(value='...'),
                            Literal(value='..'),
                        ]),
                    ),
                    RuleReference(name='logical_or', is_token=False),
                ]),
                Sequence(elements=[
                    RuleReference(name='logical_or', is_token=False),
                    Optional(element=
                        Sequence(elements=[
                            Group(element=
                                Alternation(choices=[
                                    Literal(value='...'),
                                    Literal(value='..'),
                                ]),
                            ),
                            Optional(element=
                                RuleReference(name='logical_or', is_token=False),
                            ),
                        ]),
                    ),
                ]),
            ]),
            line_number=768,
        ),
        GrammarRule(
            name='logical_or',
            body=
            Sequence(elements=[
                RuleReference(name='logical_and', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                Literal(value='||'),
                                Literal(value='or'),
                            ]),
                        ),
                        RuleReference(name='logical_and', is_token=False),
                    ]),
                ),
            ]),
            line_number=769,
        ),
        GrammarRule(
            name='logical_and',
            body=
            Sequence(elements=[
                RuleReference(name='logical_not', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                Literal(value='&&'),
                                Literal(value='and'),
                            ]),
                        ),
                        RuleReference(name='logical_not', is_token=False),
                    ]),
                ),
            ]),
            line_number=770,
        ),
        GrammarRule(
            name='logical_not',
            body=
            Sequence(elements=[
                Repetition(element=
                    Group(element=
                        Alternation(choices=[
                            Literal(value='!'),
                            Literal(value='not'),
                        ]),
                    ),
                ),
                RuleReference(name='comparison', is_token=False),
            ]),
            line_number=777,
        ),
        GrammarRule(
            name='comparison',
            body=
            Sequence(elements=[
                RuleReference(name='shift', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                Literal(value='=='),
                                Literal(value='!='),
                                Literal(value='<='),
                                Literal(value='>='),
                                Literal(value='<'),
                                Literal(value='>'),
                            ]),
                        ),
                        RuleReference(name='shift', is_token=False),
                    ]),
                ),
            ]),
            line_number=793,
        ),
        GrammarRule(
            name='shift',
            body=
            Sequence(elements=[
                RuleReference(name='sum', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='<<'),
                        RuleReference(name='sum', is_token=False),
                    ]),
                ),
            ]),
            line_number=794,
        ),
        GrammarRule(
            name='sum',
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
            line_number=795,
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
                            ]),
                        ),
                        RuleReference(name='factor', is_token=False),
                    ]),
                ),
            ]),
            line_number=796,
        ),
        GrammarRule(
            name='super_expr',
            body=
            Sequence(elements=[
                Literal(value='super'),
                Optional(element=
                    RuleReference(name='super_args', is_token=False),
                ),
            ]),
            line_number=865,
        ),
        GrammarRule(
            name='index_suffix',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACKET', is_token=True),
                RuleReference(name='expression', is_token=False),
                RuleReference(name='RBRACKET', is_token=True),
            ]),
            line_number=877,
        ),
        GrammarRule(
            name='factor',
            body=
            Sequence(elements=[
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='defined_expression', is_token=False),
                        RuleReference(name='lambda_literal', is_token=False),
                        RuleReference(name='super_expr', is_token=False),
                        RuleReference(name='method_call', is_token=False),
                        RuleReference(name='NUMBER', is_token=True),
                        RuleReference(name='STRING', is_token=True),
                        RuleReference(name='NAME', is_token=True),
                        Group(element=
                            Sequence(elements=[
                                NegativeLookahead(element=
                                    Literal(value='end'),
                                ),
                                NegativeLookahead(element=
                                    Literal(value='rescue'),
                                ),
                                NegativeLookahead(element=
                                    Literal(value='ensure'),
                                ),
                                NegativeLookahead(element=
                                    Literal(value='else'),
                                ),
                                NegativeLookahead(element=
                                    Literal(value='elsif'),
                                ),
                                NegativeLookahead(element=
                                    Literal(value='when'),
                                ),
                                NegativeLookahead(element=
                                    Literal(value='then'),
                                ),
                                NegativeLookahead(element=
                                    Literal(value='in'),
                                ),
                                NegativeLookahead(element=
                                    Literal(value='do'),
                                ),
                                RuleReference(name='KEYWORD', is_token=True),
                            ]),
                        ),
                        RuleReference(name='symbol_literal', is_token=False),
                        RuleReference(name='array_literal', is_token=False),
                        RuleReference(name='hash_literal', is_token=False),
                        Sequence(elements=[
                            RuleReference(name='LPAREN', is_token=True),
                            RuleReference(name='expression', is_token=False),
                            RuleReference(name='RPAREN', is_token=True),
                        ]),
                        RuleReference(name='unary_minus', is_token=False),
                    ]),
                ),
                Repetition(element=
                    Alternation(choices=[
                        RuleReference(name='dot_call', is_token=False),
                        RuleReference(name='scope_resolution', is_token=False),
                        RuleReference(name='index_suffix', is_token=False),
                    ]),
                ),
            ]),
            line_number=878,
        ),
        GrammarRule(
            name='lambda_literal',
            body=
            Sequence(elements=[
                Literal(value='->'),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='LPAREN', is_token=True),
                        Optional(element=
                            RuleReference(name='params', is_token=False),
                        ),
                        RuleReference(name='RPAREN', is_token=True),
                    ]),
                ),
                RuleReference(name='block', is_token=False),
            ]),
            line_number=897,
        ),
        GrammarRule(
            name='unary_minus',
            body=
            Sequence(elements=[
                RuleReference(name='MINUS', is_token=True),
                RuleReference(name='factor', is_token=False),
            ]),
            line_number=898,
        ),
        GrammarRule(
            name='defined_expression',
            body=
            Sequence(elements=[
                Literal(value='defined?'),
                RuleReference(name='factor', is_token=False),
            ]),
            line_number=909,
        ),
        GrammarRule(
            name='symbol_literal',
            body=
            Sequence(elements=[
                Literal(value=':'),
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='NAME', is_token=True),
                        RuleReference(name='KEYWORD', is_token=True),
                        RuleReference(name='STRING', is_token=True),
                    ]),
                ),
            ]),
            line_number=916,
        ),
        GrammarRule(
            name='array_literal',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACKET', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='expression', is_token=False),
                        Repetition(element=
                            Sequence(elements=[
                                RuleReference(name='COMMA', is_token=True),
                                RuleReference(name='expression', is_token=False),
                            ]),
                        ),
                    ]),
                ),
                RuleReference(name='RBRACKET', is_token=True),
            ]),
            line_number=917,
        ),
        GrammarRule(
            name='hash_literal',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACE', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='hash_entry', is_token=False),
                        Repetition(element=
                            Sequence(elements=[
                                RuleReference(name='COMMA', is_token=True),
                                RuleReference(name='hash_entry', is_token=False),
                            ]),
                        ),
                    ]),
                ),
                RuleReference(name='RBRACE', is_token=True),
            ]),
            line_number=918,
        ),
        GrammarRule(
            name='hash_entry',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='NAME', is_token=True),
                    RuleReference(name='COLON', is_token=True),
                    RuleReference(name='expression', is_token=False),
                ]),
                Sequence(elements=[
                    RuleReference(name='NAME', is_token=True),
                    RuleReference(name='COLON', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='expression', is_token=False),
                    Literal(value='=>'),
                    RuleReference(name='expression', is_token=False),
                ]),
            ]),
            line_number=919,
        ),
    ],
)
