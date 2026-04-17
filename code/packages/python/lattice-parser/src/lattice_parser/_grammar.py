# AUTO-GENERATED FILE — DO NOT EDIT
# Source: lattice.grammar
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
            name='stylesheet',
            body=
            Repetition(element=
                RuleReference(name='rule', is_token=False),
            ),
            line_number=37,
        ),
        GrammarRule(
            name='rule',
            body=
            Alternation(choices=[
                RuleReference(name='lattice_rule', is_token=False),
                RuleReference(name='at_rule', is_token=False),
                RuleReference(name='qualified_rule', is_token=False),
            ]),
            line_number=39,
        ),
        GrammarRule(
            name='lattice_rule',
            body=
            Alternation(choices=[
                RuleReference(name='variable_declaration', is_token=False),
                RuleReference(name='mixin_definition', is_token=False),
                RuleReference(name='function_definition', is_token=False),
                RuleReference(name='use_directive', is_token=False),
                RuleReference(name='lattice_control', is_token=False),
            ]),
            line_number=51,
        ),
        GrammarRule(
            name='variable_declaration',
            body=
            Sequence(elements=[
                RuleReference(name='VARIABLE', is_token=True),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='value_list', is_token=False),
                Optional(element=
                    Alternation(choices=[
                        RuleReference(name='BANG_DEFAULT', is_token=True),
                        RuleReference(name='BANG_GLOBAL', is_token=True),
                    ]),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=69,
        ),
        GrammarRule(
            name='mixin_definition',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='@mixin'),
                    RuleReference(name='FUNCTION', is_token=True),
                    Optional(element=
                        RuleReference(name='mixin_params', is_token=False),
                    ),
                    RuleReference(name='RPAREN', is_token=True),
                    RuleReference(name='block', is_token=False),
                ]),
                Sequence(elements=[
                    Literal(value='@mixin'),
                    RuleReference(name='IDENT', is_token=True),
                    RuleReference(name='block', is_token=False),
                ]),
            ]),
            line_number=102,
        ),
        GrammarRule(
            name='mixin_params',
            body=
            Sequence(elements=[
                RuleReference(name='mixin_param', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='mixin_param', is_token=False),
                    ]),
                ),
            ]),
            line_number=105,
        ),
        GrammarRule(
            name='mixin_param',
            body=
            Sequence(elements=[
                RuleReference(name='VARIABLE', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='COLON', is_token=True),
                        RuleReference(name='mixin_value_list', is_token=False),
                    ]),
                ),
            ]),
            line_number=112,
        ),
        GrammarRule(
            name='mixin_value_list',
            body=
            Sequence(elements=[
                RuleReference(name='mixin_value', is_token=False),
                Repetition(element=
                    RuleReference(name='mixin_value', is_token=False),
                ),
            ]),
            line_number=117,
        ),
        GrammarRule(
            name='mixin_value',
            body=
            Alternation(choices=[
                RuleReference(name='DIMENSION', is_token=True),
                RuleReference(name='PERCENTAGE', is_token=True),
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='IDENT', is_token=True),
                RuleReference(name='HASH', is_token=True),
                RuleReference(name='CUSTOM_PROPERTY', is_token=True),
                RuleReference(name='UNICODE_RANGE', is_token=True),
                RuleReference(name='function_call', is_token=False),
                RuleReference(name='VARIABLE', is_token=True),
                RuleReference(name='SLASH', is_token=True),
                RuleReference(name='PLUS', is_token=True),
                RuleReference(name='MINUS', is_token=True),
            ]),
            line_number=119,
        ),
        GrammarRule(
            name='include_directive',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='@include'),
                    RuleReference(name='FUNCTION', is_token=True),
                    Optional(element=
                        RuleReference(name='include_args', is_token=False),
                    ),
                    RuleReference(name='RPAREN', is_token=True),
                    Group(element=
                        Alternation(choices=[
                            RuleReference(name='SEMICOLON', is_token=True),
                            RuleReference(name='block', is_token=False),
                        ]),
                    ),
                ]),
                Sequence(elements=[
                    Literal(value='@include'),
                    RuleReference(name='IDENT', is_token=True),
                    Group(element=
                        Alternation(choices=[
                            RuleReference(name='SEMICOLON', is_token=True),
                            RuleReference(name='block', is_token=False),
                        ]),
                    ),
                ]),
            ]),
            line_number=130,
        ),
        GrammarRule(
            name='include_args',
            body=
            Sequence(elements=[
                RuleReference(name='include_arg', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='include_arg', is_token=False),
                    ]),
                ),
            ]),
            line_number=133,
        ),
        GrammarRule(
            name='include_arg',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='VARIABLE', is_token=True),
                    RuleReference(name='COLON', is_token=True),
                    RuleReference(name='value_list', is_token=False),
                ]),
                RuleReference(name='value_list', is_token=False),
            ]),
            line_number=137,
        ),
        GrammarRule(
            name='lattice_control',
            body=
            Alternation(choices=[
                RuleReference(name='if_directive', is_token=False),
                RuleReference(name='for_directive', is_token=False),
                RuleReference(name='each_directive', is_token=False),
                RuleReference(name='while_directive', is_token=False),
            ]),
            line_number=160,
        ),
        GrammarRule(
            name='if_directive',
            body=
            Sequence(elements=[
                Literal(value='@if'),
                RuleReference(name='lattice_expression', is_token=False),
                RuleReference(name='block', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='@else'),
                        Literal(value='if'),
                        RuleReference(name='lattice_expression', is_token=False),
                        RuleReference(name='block', is_token=False),
                    ]),
                ),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='@else'),
                        RuleReference(name='block', is_token=False),
                    ]),
                ),
            ]),
            line_number=164,
        ),
        GrammarRule(
            name='for_directive',
            body=
            Sequence(elements=[
                Literal(value='@for'),
                RuleReference(name='VARIABLE', is_token=True),
                Literal(value='from'),
                RuleReference(name='lattice_expression', is_token=False),
                Group(element=
                    Alternation(choices=[
                        Literal(value='through'),
                        Literal(value='to'),
                    ]),
                ),
                RuleReference(name='lattice_expression', is_token=False),
                RuleReference(name='block', is_token=False),
            ]),
            line_number=171,
        ),
        GrammarRule(
            name='each_directive',
            body=
            Sequence(elements=[
                Literal(value='@each'),
                RuleReference(name='VARIABLE', is_token=True),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='VARIABLE', is_token=True),
                    ]),
                ),
                Literal(value='in'),
                RuleReference(name='each_list', is_token=False),
                RuleReference(name='block', is_token=False),
            ]),
            line_number=176,
        ),
        GrammarRule(
            name='each_list',
            body=
            Sequence(elements=[
                RuleReference(name='value', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='value', is_token=False),
                    ]),
                ),
            ]),
            line_number=179,
        ),
        GrammarRule(
            name='while_directive',
            body=
            Sequence(elements=[
                Literal(value='@while'),
                RuleReference(name='lattice_expression', is_token=False),
                RuleReference(name='block', is_token=False),
            ]),
            line_number=184,
        ),
        GrammarRule(
            name='lattice_expression',
            body=
            RuleReference(name='lattice_or_expr', is_token=False),
            line_number=203,
        ),
        GrammarRule(
            name='lattice_or_expr',
            body=
            Sequence(elements=[
                RuleReference(name='lattice_and_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='or'),
                        RuleReference(name='lattice_and_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=205,
        ),
        GrammarRule(
            name='lattice_and_expr',
            body=
            Sequence(elements=[
                RuleReference(name='lattice_comparison', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='and'),
                        RuleReference(name='lattice_comparison', is_token=False),
                    ]),
                ),
            ]),
            line_number=207,
        ),
        GrammarRule(
            name='lattice_comparison',
            body=
            Sequence(elements=[
                RuleReference(name='lattice_additive', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='comparison_op', is_token=False),
                        RuleReference(name='lattice_additive', is_token=False),
                    ]),
                ),
            ]),
            line_number=209,
        ),
        GrammarRule(
            name='comparison_op',
            body=
            Alternation(choices=[
                RuleReference(name='EQUALS_EQUALS', is_token=True),
                RuleReference(name='NOT_EQUALS', is_token=True),
                RuleReference(name='GREATER', is_token=True),
                RuleReference(name='GREATER_EQUALS', is_token=True),
                RuleReference(name='LESS', is_token=True),
                RuleReference(name='LESS_EQUALS', is_token=True),
            ]),
            line_number=211,
        ),
        GrammarRule(
            name='lattice_additive',
            body=
            Sequence(elements=[
                RuleReference(name='lattice_multiplicative', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='PLUS', is_token=True),
                                RuleReference(name='MINUS', is_token=True),
                            ]),
                        ),
                        RuleReference(name='lattice_multiplicative', is_token=False),
                    ]),
                ),
            ]),
            line_number=214,
        ),
        GrammarRule(
            name='lattice_multiplicative',
            body=
            Sequence(elements=[
                RuleReference(name='lattice_unary', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='STAR', is_token=True),
                                RuleReference(name='SLASH', is_token=True),
                            ]),
                        ),
                        RuleReference(name='lattice_unary', is_token=False),
                    ]),
                ),
            ]),
            line_number=219,
        ),
        GrammarRule(
            name='lattice_unary',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='MINUS', is_token=True),
                    RuleReference(name='lattice_unary', is_token=False),
                ]),
                RuleReference(name='lattice_primary', is_token=False),
            ]),
            line_number=221,
        ),
        GrammarRule(
            name='lattice_primary',
            body=
            Alternation(choices=[
                RuleReference(name='VARIABLE', is_token=True),
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='DIMENSION', is_token=True),
                RuleReference(name='PERCENTAGE', is_token=True),
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='IDENT', is_token=True),
                RuleReference(name='HASH', is_token=True),
                Literal(value='true'),
                Literal(value='false'),
                Literal(value='null'),
                RuleReference(name='function_call', is_token=False),
                RuleReference(name='map_literal', is_token=False),
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='lattice_expression', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
            ]),
            line_number=224,
        ),
        GrammarRule(
            name='map_literal',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='map_entry', is_token=False),
                RuleReference(name='COMMA', is_token=True),
                RuleReference(name='map_entry', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='map_entry', is_token=False),
                    ]),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=235,
        ),
        GrammarRule(
            name='map_entry',
            body=
            Sequence(elements=[
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='IDENT', is_token=True),
                        RuleReference(name='STRING', is_token=True),
                    ]),
                ),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='lattice_expression', is_token=False),
            ]),
            line_number=237,
        ),
        GrammarRule(
            name='function_definition',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='@function'),
                    RuleReference(name='FUNCTION', is_token=True),
                    Optional(element=
                        RuleReference(name='mixin_params', is_token=False),
                    ),
                    RuleReference(name='RPAREN', is_token=True),
                    RuleReference(name='function_body', is_token=False),
                ]),
                Sequence(elements=[
                    Literal(value='@function'),
                    RuleReference(name='IDENT', is_token=True),
                    RuleReference(name='function_body', is_token=False),
                ]),
            ]),
            line_number=261,
        ),
        GrammarRule(
            name='function_body',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACE', is_token=True),
                Repetition(element=
                    RuleReference(name='function_body_item', is_token=False),
                ),
                RuleReference(name='RBRACE', is_token=True),
            ]),
            line_number=264,
        ),
        GrammarRule(
            name='function_body_item',
            body=
            Alternation(choices=[
                RuleReference(name='variable_declaration', is_token=False),
                RuleReference(name='return_directive', is_token=False),
                RuleReference(name='lattice_control', is_token=False),
            ]),
            line_number=266,
        ),
        GrammarRule(
            name='return_directive',
            body=
            Sequence(elements=[
                Literal(value='@return'),
                RuleReference(name='lattice_expression', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=268,
        ),
        GrammarRule(
            name='use_directive',
            body=
            Sequence(elements=[
                Literal(value='@use'),
                RuleReference(name='STRING', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='as'),
                        RuleReference(name='IDENT', is_token=True),
                    ]),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=281,
        ),
        GrammarRule(
            name='at_rule',
            body=
            Sequence(elements=[
                RuleReference(name='AT_KEYWORD', is_token=True),
                RuleReference(name='at_prelude', is_token=False),
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='SEMICOLON', is_token=True),
                        RuleReference(name='block', is_token=False),
                    ]),
                ),
            ]),
            line_number=294,
        ),
        GrammarRule(
            name='at_prelude',
            body=
            Repetition(element=
                RuleReference(name='at_prelude_token', is_token=False),
            ),
            line_number=296,
        ),
        GrammarRule(
            name='at_prelude_token',
            body=
            Alternation(choices=[
                RuleReference(name='IDENT', is_token=True),
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='DIMENSION', is_token=True),
                RuleReference(name='PERCENTAGE', is_token=True),
                RuleReference(name='HASH', is_token=True),
                RuleReference(name='CUSTOM_PROPERTY', is_token=True),
                RuleReference(name='UNICODE_RANGE', is_token=True),
                RuleReference(name='VARIABLE', is_token=True),
                RuleReference(name='function_in_prelude', is_token=False),
                RuleReference(name='paren_block', is_token=False),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='COMMA', is_token=True),
                RuleReference(name='SLASH', is_token=True),
                RuleReference(name='DOT', is_token=True),
                RuleReference(name='STAR', is_token=True),
                RuleReference(name='PLUS', is_token=True),
                RuleReference(name='MINUS', is_token=True),
                RuleReference(name='GREATER', is_token=True),
                RuleReference(name='TILDE', is_token=True),
                RuleReference(name='PIPE', is_token=True),
                RuleReference(name='EQUALS', is_token=True),
                RuleReference(name='AMPERSAND', is_token=True),
                RuleReference(name='CDO', is_token=True),
                RuleReference(name='CDC', is_token=True),
            ]),
            line_number=298,
        ),
        GrammarRule(
            name='function_in_prelude',
            body=
            Sequence(elements=[
                RuleReference(name='FUNCTION', is_token=True),
                RuleReference(name='at_prelude_tokens', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=306,
        ),
        GrammarRule(
            name='paren_block',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='at_prelude_tokens', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=307,
        ),
        GrammarRule(
            name='at_prelude_tokens',
            body=
            Repetition(element=
                RuleReference(name='at_prelude_token', is_token=False),
            ),
            line_number=308,
        ),
        GrammarRule(
            name='qualified_rule',
            body=
            Sequence(elements=[
                RuleReference(name='selector_list', is_token=False),
                RuleReference(name='block', is_token=False),
            ]),
            line_number=314,
        ),
        GrammarRule(
            name='selector_list',
            body=
            Sequence(elements=[
                RuleReference(name='complex_selector', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='complex_selector', is_token=False),
                    ]),
                ),
            ]),
            line_number=320,
        ),
        GrammarRule(
            name='complex_selector',
            body=
            Sequence(elements=[
                RuleReference(name='compound_selector', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Optional(element=
                            RuleReference(name='combinator', is_token=False),
                        ),
                        RuleReference(name='compound_selector', is_token=False),
                    ]),
                ),
            ]),
            line_number=322,
        ),
        GrammarRule(
            name='combinator',
            body=
            Alternation(choices=[
                RuleReference(name='GREATER', is_token=True),
                RuleReference(name='PLUS', is_token=True),
                RuleReference(name='TILDE', is_token=True),
            ]),
            line_number=324,
        ),
        GrammarRule(
            name='compound_selector',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='simple_selector', is_token=False),
                    Repetition(element=
                        RuleReference(name='subclass_selector', is_token=False),
                    ),
                ]),
                Sequence(elements=[
                    RuleReference(name='subclass_selector', is_token=False),
                    Repetition(element=
                        RuleReference(name='subclass_selector', is_token=False),
                    ),
                ]),
            ]),
            line_number=326,
        ),
        GrammarRule(
            name='simple_selector',
            body=
            Alternation(choices=[
                RuleReference(name='IDENT', is_token=True),
                RuleReference(name='STAR', is_token=True),
                RuleReference(name='AMPERSAND', is_token=True),
                RuleReference(name='VARIABLE', is_token=True),
                RuleReference(name='PERCENTAGE', is_token=True),
            ]),
            line_number=331,
        ),
        GrammarRule(
            name='subclass_selector',
            body=
            Alternation(choices=[
                RuleReference(name='class_selector', is_token=False),
                RuleReference(name='id_selector', is_token=False),
                RuleReference(name='placeholder_selector', is_token=False),
                RuleReference(name='attribute_selector', is_token=False),
                RuleReference(name='pseudo_class', is_token=False),
                RuleReference(name='pseudo_element', is_token=False),
            ]),
            line_number=334,
        ),
        GrammarRule(
            name='placeholder_selector',
            body=
            RuleReference(name='PLACEHOLDER', is_token=True),
            line_number=338,
        ),
        GrammarRule(
            name='class_selector',
            body=
            Sequence(elements=[
                RuleReference(name='DOT', is_token=True),
                RuleReference(name='IDENT', is_token=True),
            ]),
            line_number=340,
        ),
        GrammarRule(
            name='id_selector',
            body=
            RuleReference(name='HASH', is_token=True),
            line_number=342,
        ),
        GrammarRule(
            name='attribute_selector',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACKET', is_token=True),
                RuleReference(name='IDENT', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='attr_matcher', is_token=False),
                        RuleReference(name='attr_value', is_token=False),
                        Optional(element=
                            RuleReference(name='IDENT', is_token=True),
                        ),
                    ]),
                ),
                RuleReference(name='RBRACKET', is_token=True),
            ]),
            line_number=344,
        ),
        GrammarRule(
            name='attr_matcher',
            body=
            Alternation(choices=[
                RuleReference(name='EQUALS', is_token=True),
                RuleReference(name='TILDE_EQUALS', is_token=True),
                RuleReference(name='PIPE_EQUALS', is_token=True),
                RuleReference(name='CARET_EQUALS', is_token=True),
                RuleReference(name='DOLLAR_EQUALS', is_token=True),
                RuleReference(name='STAR_EQUALS', is_token=True),
            ]),
            line_number=346,
        ),
        GrammarRule(
            name='attr_value',
            body=
            Alternation(choices=[
                RuleReference(name='IDENT', is_token=True),
                RuleReference(name='STRING', is_token=True),
            ]),
            line_number=349,
        ),
        GrammarRule(
            name='pseudo_class',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='COLON', is_token=True),
                    RuleReference(name='FUNCTION', is_token=True),
                    RuleReference(name='pseudo_class_args', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='COLON', is_token=True),
                    RuleReference(name='IDENT', is_token=True),
                ]),
            ]),
            line_number=351,
        ),
        GrammarRule(
            name='pseudo_class_args',
            body=
            Repetition(element=
                RuleReference(name='pseudo_class_arg', is_token=False),
            ),
            line_number=354,
        ),
        GrammarRule(
            name='pseudo_class_arg',
            body=
            Alternation(choices=[
                RuleReference(name='IDENT', is_token=True),
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='DIMENSION', is_token=True),
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='HASH', is_token=True),
                RuleReference(name='PLUS', is_token=True),
                RuleReference(name='COMMA', is_token=True),
                RuleReference(name='DOT', is_token=True),
                RuleReference(name='STAR', is_token=True),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='AMPERSAND', is_token=True),
                Sequence(elements=[
                    RuleReference(name='FUNCTION', is_token=True),
                    RuleReference(name='pseudo_class_args', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='LBRACKET', is_token=True),
                    RuleReference(name='pseudo_class_args', is_token=False),
                    RuleReference(name='RBRACKET', is_token=True),
                ]),
            ]),
            line_number=356,
        ),
        GrammarRule(
            name='pseudo_element',
            body=
            Sequence(elements=[
                RuleReference(name='COLON_COLON', is_token=True),
                RuleReference(name='IDENT', is_token=True),
            ]),
            line_number=361,
        ),
        GrammarRule(
            name='block',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACE', is_token=True),
                RuleReference(name='block_contents', is_token=False),
                RuleReference(name='RBRACE', is_token=True),
            ]),
            line_number=371,
        ),
        GrammarRule(
            name='block_contents',
            body=
            Repetition(element=
                RuleReference(name='block_item', is_token=False),
            ),
            line_number=373,
        ),
        GrammarRule(
            name='block_item',
            body=
            Alternation(choices=[
                RuleReference(name='lattice_block_item', is_token=False),
                RuleReference(name='at_rule', is_token=False),
                RuleReference(name='declaration_or_nested', is_token=False),
            ]),
            line_number=375,
        ),
        GrammarRule(
            name='lattice_block_item',
            body=
            Alternation(choices=[
                RuleReference(name='variable_declaration', is_token=False),
                RuleReference(name='include_directive', is_token=False),
                RuleReference(name='lattice_control', is_token=False),
                RuleReference(name='content_directive', is_token=False),
                RuleReference(name='extend_directive', is_token=False),
                RuleReference(name='at_root_directive', is_token=False),
            ]),
            line_number=381,
        ),
        GrammarRule(
            name='content_directive',
            body=
            Sequence(elements=[
                Literal(value='@content'),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=391,
        ),
        GrammarRule(
            name='extend_directive',
            body=
            Sequence(elements=[
                Literal(value='@extend'),
                RuleReference(name='selector_list', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=399,
        ),
        GrammarRule(
            name='at_root_directive',
            body=
            Sequence(elements=[
                Literal(value='@at-root'),
                Group(element=
                    Alternation(choices=[
                        Sequence(elements=[
                            RuleReference(name='selector_list', is_token=False),
                            RuleReference(name='block', is_token=False),
                        ]),
                        RuleReference(name='block', is_token=False),
                    ]),
                ),
            ]),
            line_number=404,
        ),
        GrammarRule(
            name='declaration_or_nested',
            body=
            Alternation(choices=[
                RuleReference(name='declaration', is_token=False),
                RuleReference(name='qualified_rule', is_token=False),
            ]),
            line_number=406,
        ),
        GrammarRule(
            name='declaration',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='property', is_token=False),
                    RuleReference(name='COLON', is_token=True),
                    RuleReference(name='value_list', is_token=False),
                    Optional(element=
                        RuleReference(name='priority', is_token=False),
                    ),
                    RuleReference(name='SEMICOLON', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='property', is_token=False),
                    RuleReference(name='COLON', is_token=True),
                    RuleReference(name='block', is_token=False),
                ]),
            ]),
            line_number=415,
        ),
        GrammarRule(
            name='property',
            body=
            Alternation(choices=[
                RuleReference(name='IDENT', is_token=True),
                RuleReference(name='CUSTOM_PROPERTY', is_token=True),
            ]),
            line_number=418,
        ),
        GrammarRule(
            name='priority',
            body=
            Sequence(elements=[
                RuleReference(name='BANG', is_token=True),
                Literal(value='important'),
            ]),
            line_number=420,
        ),
        GrammarRule(
            name='value_list',
            body=
            Sequence(elements=[
                RuleReference(name='value', is_token=False),
                Repetition(element=
                    RuleReference(name='value', is_token=False),
                ),
            ]),
            line_number=431,
        ),
        GrammarRule(
            name='value',
            body=
            Alternation(choices=[
                RuleReference(name='DIMENSION', is_token=True),
                RuleReference(name='PERCENTAGE', is_token=True),
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='IDENT', is_token=True),
                RuleReference(name='HASH', is_token=True),
                RuleReference(name='CUSTOM_PROPERTY', is_token=True),
                RuleReference(name='UNICODE_RANGE', is_token=True),
                RuleReference(name='function_call', is_token=False),
                RuleReference(name='VARIABLE', is_token=True),
                RuleReference(name='SLASH', is_token=True),
                RuleReference(name='COMMA', is_token=True),
                RuleReference(name='PLUS', is_token=True),
                RuleReference(name='MINUS', is_token=True),
                RuleReference(name='map_literal', is_token=False),
            ]),
            line_number=433,
        ),
        GrammarRule(
            name='function_call',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='FUNCTION', is_token=True),
                    RuleReference(name='function_args', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                RuleReference(name='URL_TOKEN', is_token=True),
            ]),
            line_number=439,
        ),
        GrammarRule(
            name='function_args',
            body=
            Repetition(element=
                RuleReference(name='function_arg', is_token=False),
            ),
            line_number=442,
        ),
        GrammarRule(
            name='function_arg',
            body=
            Alternation(choices=[
                RuleReference(name='DIMENSION', is_token=True),
                RuleReference(name='PERCENTAGE', is_token=True),
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='IDENT', is_token=True),
                RuleReference(name='HASH', is_token=True),
                RuleReference(name='CUSTOM_PROPERTY', is_token=True),
                RuleReference(name='COMMA', is_token=True),
                RuleReference(name='SLASH', is_token=True),
                RuleReference(name='PLUS', is_token=True),
                RuleReference(name='MINUS', is_token=True),
                RuleReference(name='STAR', is_token=True),
                RuleReference(name='VARIABLE', is_token=True),
                Sequence(elements=[
                    RuleReference(name='FUNCTION', is_token=True),
                    RuleReference(name='function_args', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
            ]),
            line_number=444,
        ),
    ],
)
