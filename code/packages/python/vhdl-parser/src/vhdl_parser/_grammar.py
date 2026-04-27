# AUTO-GENERATED FILE — DO NOT EDIT
# Source: vhdl.grammar
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
    version=0,
    rules=[
        GrammarRule(
            name='design_file',
            body=
            Repetition(element=
                RuleReference(name='design_unit', is_token=False),
            ),
            line_number=67,
        ),
        GrammarRule(
            name='design_unit',
            body=
            Sequence(elements=[
                Repetition(element=
                    RuleReference(name='context_item', is_token=False),
                ),
                RuleReference(name='library_unit', is_token=False),
            ]),
            line_number=69,
        ),
        GrammarRule(
            name='context_item',
            body=
            Alternation(choices=[
                RuleReference(name='library_clause', is_token=False),
                RuleReference(name='use_clause', is_token=False),
            ]),
            line_number=71,
        ),
        GrammarRule(
            name='library_clause',
            body=
            Sequence(elements=[
                Literal(value='library'),
                RuleReference(name='name_list', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=74,
        ),
        GrammarRule(
            name='use_clause',
            body=
            Sequence(elements=[
                Literal(value='use'),
                RuleReference(name='selected_name', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=77,
        ),
        GrammarRule(
            name='selected_name',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='DOT', is_token=True),
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='NAME', is_token=True),
                                Literal(value='all'),
                            ]),
                        ),
                    ]),
                ),
            ]),
            line_number=80,
        ),
        GrammarRule(
            name='name_list',
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
            line_number=82,
        ),
        GrammarRule(
            name='library_unit',
            body=
            Alternation(choices=[
                RuleReference(name='entity_declaration', is_token=False),
                RuleReference(name='architecture_body', is_token=False),
                RuleReference(name='package_declaration', is_token=False),
                RuleReference(name='package_body', is_token=False),
            ]),
            line_number=84,
        ),
        GrammarRule(
            name='entity_declaration',
            body=
            Sequence(elements=[
                Literal(value='entity'),
                RuleReference(name='NAME', is_token=True),
                Literal(value='is'),
                Optional(element=
                    RuleReference(name='generic_clause', is_token=False),
                ),
                Optional(element=
                    RuleReference(name='port_clause', is_token=False),
                ),
                Literal(value='end'),
                Optional(element=
                    Literal(value='entity'),
                ),
                Optional(element=
                    RuleReference(name='NAME', is_token=True),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=115,
        ),
        GrammarRule(
            name='generic_clause',
            body=
            Sequence(elements=[
                Literal(value='generic'),
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='interface_list', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=120,
        ),
        GrammarRule(
            name='port_clause',
            body=
            Sequence(elements=[
                Literal(value='port'),
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='interface_list', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=121,
        ),
        GrammarRule(
            name='interface_list',
            body=
            Sequence(elements=[
                RuleReference(name='interface_element', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='SEMICOLON', is_token=True),
                        RuleReference(name='interface_element', is_token=False),
                    ]),
                ),
            ]),
            line_number=126,
        ),
        GrammarRule(
            name='interface_element',
            body=
            Sequence(elements=[
                RuleReference(name='name_list', is_token=False),
                RuleReference(name='COLON', is_token=True),
                Optional(element=
                    RuleReference(name='mode', is_token=False),
                ),
                RuleReference(name='subtype_indication', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='VAR_ASSIGN', is_token=True),
                        RuleReference(name='expression', is_token=False),
                    ]),
                ),
            ]),
            line_number=127,
        ),
        GrammarRule(
            name='mode',
            body=
            Alternation(choices=[
                Literal(value='in'),
                Literal(value='out'),
                Literal(value='inout'),
                Literal(value='buffer'),
            ]),
            line_number=135,
        ),
        GrammarRule(
            name='architecture_body',
            body=
            Sequence(elements=[
                Literal(value='architecture'),
                RuleReference(name='NAME', is_token=True),
                Literal(value='of'),
                RuleReference(name='NAME', is_token=True),
                Literal(value='is'),
                Repetition(element=
                    RuleReference(name='block_declarative_item', is_token=False),
                ),
                Literal(value='begin'),
                Repetition(element=
                    RuleReference(name='concurrent_statement', is_token=False),
                ),
                Literal(value='end'),
                Optional(element=
                    Literal(value='architecture'),
                ),
                Optional(element=
                    RuleReference(name='NAME', is_token=True),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=157,
        ),
        GrammarRule(
            name='block_declarative_item',
            body=
            Alternation(choices=[
                RuleReference(name='signal_declaration', is_token=False),
                RuleReference(name='constant_declaration', is_token=False),
                RuleReference(name='type_declaration', is_token=False),
                RuleReference(name='subtype_declaration', is_token=False),
                RuleReference(name='component_declaration', is_token=False),
                RuleReference(name='function_declaration', is_token=False),
                RuleReference(name='function_body', is_token=False),
                RuleReference(name='procedure_declaration', is_token=False),
                RuleReference(name='procedure_body', is_token=False),
            ]),
            line_number=163,
        ),
        GrammarRule(
            name='signal_declaration',
            body=
            Sequence(elements=[
                Literal(value='signal'),
                RuleReference(name='name_list', is_token=False),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='subtype_indication', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='VAR_ASSIGN', is_token=True),
                        RuleReference(name='expression', is_token=False),
                    ]),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=192,
        ),
        GrammarRule(
            name='constant_declaration',
            body=
            Sequence(elements=[
                Literal(value='constant'),
                RuleReference(name='name_list', is_token=False),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='subtype_indication', is_token=False),
                RuleReference(name='VAR_ASSIGN', is_token=True),
                RuleReference(name='expression', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=194,
        ),
        GrammarRule(
            name='variable_declaration',
            body=
            Sequence(elements=[
                Literal(value='variable'),
                RuleReference(name='name_list', is_token=False),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='subtype_indication', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='VAR_ASSIGN', is_token=True),
                        RuleReference(name='expression', is_token=False),
                    ]),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=196,
        ),
        GrammarRule(
            name='type_declaration',
            body=
            Sequence(elements=[
                Literal(value='type'),
                RuleReference(name='NAME', is_token=True),
                Literal(value='is'),
                RuleReference(name='type_definition', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=221,
        ),
        GrammarRule(
            name='subtype_declaration',
            body=
            Sequence(elements=[
                Literal(value='subtype'),
                RuleReference(name='NAME', is_token=True),
                Literal(value='is'),
                RuleReference(name='subtype_indication', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=222,
        ),
        GrammarRule(
            name='type_definition',
            body=
            Alternation(choices=[
                RuleReference(name='enumeration_type', is_token=False),
                RuleReference(name='array_type', is_token=False),
                RuleReference(name='record_type', is_token=False),
            ]),
            line_number=224,
        ),
        GrammarRule(
            name='enumeration_type',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='NAME', is_token=True),
                        RuleReference(name='CHAR_LITERAL', is_token=True),
                    ]),
                ),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='NAME', is_token=True),
                                RuleReference(name='CHAR_LITERAL', is_token=True),
                            ]),
                        ),
                    ]),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=230,
        ),
        GrammarRule(
            name='array_type',
            body=
            Sequence(elements=[
                Literal(value='array'),
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='index_constraint', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
                Literal(value='of'),
                RuleReference(name='subtype_indication', is_token=False),
            ]),
            line_number=235,
        ),
        GrammarRule(
            name='index_constraint',
            body=
            Sequence(elements=[
                RuleReference(name='discrete_range', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='discrete_range', is_token=False),
                    ]),
                ),
            ]),
            line_number=237,
        ),
        GrammarRule(
            name='discrete_range',
            body=
            Alternation(choices=[
                RuleReference(name='subtype_indication', is_token=False),
                Sequence(elements=[
                    RuleReference(name='expression', is_token=False),
                    Group(element=
                        Alternation(choices=[
                            Literal(value='to'),
                            Literal(value='downto'),
                        ]),
                    ),
                    RuleReference(name='expression', is_token=False),
                ]),
            ]),
            line_number=238,
        ),
        GrammarRule(
            name='record_type',
            body=
            Sequence(elements=[
                Literal(value='record'),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='NAME', is_token=True),
                        RuleReference(name='COLON', is_token=True),
                        RuleReference(name='subtype_indication', is_token=False),
                        RuleReference(name='SEMICOLON', is_token=True),
                    ]),
                ),
                Literal(value='end'),
                Literal(value='record'),
                Optional(element=
                    RuleReference(name='NAME', is_token=True),
                ),
            ]),
            line_number=242,
        ),
        GrammarRule(
            name='subtype_indication',
            body=
            Sequence(elements=[
                RuleReference(name='selected_name', is_token=False),
                Optional(element=
                    RuleReference(name='constraint', is_token=False),
                ),
            ]),
            line_number=250,
        ),
        GrammarRule(
            name='constraint',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expression', is_token=False),
                    Group(element=
                        Alternation(choices=[
                            Literal(value='to'),
                            Literal(value='downto'),
                        ]),
                    ),
                    RuleReference(name='expression', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                Sequence(elements=[
                    Literal(value='range'),
                    RuleReference(name='expression', is_token=False),
                    Group(element=
                        Alternation(choices=[
                            Literal(value='to'),
                            Literal(value='downto'),
                        ]),
                    ),
                    RuleReference(name='expression', is_token=False),
                ]),
            ]),
            line_number=252,
        ),
        GrammarRule(
            name='concurrent_statement',
            body=
            Alternation(choices=[
                RuleReference(name='process_statement', is_token=False),
                RuleReference(name='signal_assignment_concurrent', is_token=False),
                RuleReference(name='component_instantiation', is_token=False),
                RuleReference(name='generate_statement', is_token=False),
            ]),
            line_number=267,
        ),
        GrammarRule(
            name='signal_assignment_concurrent',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='LESS_EQUALS', is_token=True),
                RuleReference(name='waveform', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=275,
        ),
        GrammarRule(
            name='waveform',
            body=
            Sequence(elements=[
                RuleReference(name='waveform_element', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='waveform_element', is_token=False),
                    ]),
                ),
            ]),
            line_number=277,
        ),
        GrammarRule(
            name='waveform_element',
            body=
            RuleReference(name='expression', is_token=False),
            line_number=278,
        ),
        GrammarRule(
            name='process_statement',
            body=
            Sequence(elements=[
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='NAME', is_token=True),
                        RuleReference(name='COLON', is_token=True),
                    ]),
                ),
                Literal(value='process'),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='LPAREN', is_token=True),
                        RuleReference(name='sensitivity_list', is_token=False),
                        RuleReference(name='RPAREN', is_token=True),
                    ]),
                ),
                Optional(element=
                    Literal(value='is'),
                ),
                Repetition(element=
                    RuleReference(name='process_declarative_item', is_token=False),
                ),
                Literal(value='begin'),
                Repetition(element=
                    RuleReference(name='sequential_statement', is_token=False),
                ),
                Literal(value='end'),
                Literal(value='process'),
                Optional(element=
                    RuleReference(name='NAME', is_token=True),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=310,
        ),
        GrammarRule(
            name='sensitivity_list',
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
            line_number=318,
        ),
        GrammarRule(
            name='process_declarative_item',
            body=
            Alternation(choices=[
                RuleReference(name='variable_declaration', is_token=False),
                RuleReference(name='constant_declaration', is_token=False),
                RuleReference(name='type_declaration', is_token=False),
                RuleReference(name='subtype_declaration', is_token=False),
            ]),
            line_number=320,
        ),
        GrammarRule(
            name='sequential_statement',
            body=
            Alternation(choices=[
                RuleReference(name='signal_assignment_seq', is_token=False),
                RuleReference(name='variable_assignment', is_token=False),
                RuleReference(name='if_statement', is_token=False),
                RuleReference(name='case_statement', is_token=False),
                RuleReference(name='loop_statement', is_token=False),
                RuleReference(name='return_statement', is_token=False),
                RuleReference(name='null_statement', is_token=False),
            ]),
            line_number=332,
        ),
        GrammarRule(
            name='signal_assignment_seq',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='LESS_EQUALS', is_token=True),
                RuleReference(name='waveform', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=345,
        ),
        GrammarRule(
            name='variable_assignment',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='VAR_ASSIGN', is_token=True),
                RuleReference(name='expression', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=349,
        ),
        GrammarRule(
            name='if_statement',
            body=
            Sequence(elements=[
                Literal(value='if'),
                RuleReference(name='expression', is_token=False),
                Literal(value='then'),
                Repetition(element=
                    RuleReference(name='sequential_statement', is_token=False),
                ),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='elsif'),
                        RuleReference(name='expression', is_token=False),
                        Literal(value='then'),
                        Repetition(element=
                            RuleReference(name='sequential_statement', is_token=False),
                        ),
                    ]),
                ),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='else'),
                        Repetition(element=
                            RuleReference(name='sequential_statement', is_token=False),
                        ),
                    ]),
                ),
                Literal(value='end'),
                Literal(value='if'),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=359,
        ),
        GrammarRule(
            name='case_statement',
            body=
            Sequence(elements=[
                Literal(value='case'),
                RuleReference(name='expression', is_token=False),
                Literal(value='is'),
                Repetition(element=
                    Sequence(elements=[
                        Literal(value='when'),
                        RuleReference(name='choices', is_token=False),
                        RuleReference(name='ARROW', is_token=True),
                        Repetition(element=
                            RuleReference(name='sequential_statement', is_token=False),
                        ),
                    ]),
                ),
                Literal(value='end'),
                Literal(value='case'),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=375,
        ),
        GrammarRule(
            name='choices',
            body=
            Sequence(elements=[
                RuleReference(name='choice', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='PIPE', is_token=True),
                        RuleReference(name='choice', is_token=False),
                    ]),
                ),
            ]),
            line_number=379,
        ),
        GrammarRule(
            name='choice',
            body=
            Alternation(choices=[
                RuleReference(name='expression', is_token=False),
                RuleReference(name='discrete_range', is_token=False),
                Literal(value='others'),
            ]),
            line_number=380,
        ),
        GrammarRule(
            name='loop_statement',
            body=
            Sequence(elements=[
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='NAME', is_token=True),
                        RuleReference(name='COLON', is_token=True),
                    ]),
                ),
                Optional(element=
                    Alternation(choices=[
                        Sequence(elements=[
                            Literal(value='for'),
                            RuleReference(name='NAME', is_token=True),
                            Literal(value='in'),
                            RuleReference(name='discrete_range', is_token=False),
                        ]),
                        Sequence(elements=[
                            Literal(value='while'),
                            RuleReference(name='expression', is_token=False),
                        ]),
                    ]),
                ),
                Literal(value='loop'),
                Repetition(element=
                    RuleReference(name='sequential_statement', is_token=False),
                ),
                Literal(value='end'),
                Literal(value='loop'),
                Optional(element=
                    RuleReference(name='NAME', is_token=True),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=394,
        ),
        GrammarRule(
            name='return_statement',
            body=
            Sequence(elements=[
                Literal(value='return'),
                Optional(element=
                    RuleReference(name='expression', is_token=False),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=401,
        ),
        GrammarRule(
            name='null_statement',
            body=
            Sequence(elements=[
                Literal(value='null'),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=402,
        ),
        GrammarRule(
            name='component_declaration',
            body=
            Sequence(elements=[
                Literal(value='component'),
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    Literal(value='is'),
                ),
                Optional(element=
                    RuleReference(name='generic_clause', is_token=False),
                ),
                Optional(element=
                    RuleReference(name='port_clause', is_token=False),
                ),
                Literal(value='end'),
                Literal(value='component'),
                Optional(element=
                    RuleReference(name='NAME', is_token=True),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=428,
        ),
        GrammarRule(
            name='component_instantiation',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='COLON', is_token=True),
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='NAME', is_token=True),
                        Sequence(elements=[
                            Literal(value='entity'),
                            RuleReference(name='selected_name', is_token=False),
                            Optional(element=
                                Sequence(elements=[
                                    RuleReference(name='LPAREN', is_token=True),
                                    RuleReference(name='NAME', is_token=True),
                                    RuleReference(name='RPAREN', is_token=True),
                                ]),
                            ),
                        ]),
                    ]),
                ),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='generic'),
                        Literal(value='map'),
                        RuleReference(name='LPAREN', is_token=True),
                        RuleReference(name='association_list', is_token=False),
                        RuleReference(name='RPAREN', is_token=True),
                    ]),
                ),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='port'),
                        Literal(value='map'),
                        RuleReference(name='LPAREN', is_token=True),
                        RuleReference(name='association_list', is_token=False),
                        RuleReference(name='RPAREN', is_token=True),
                    ]),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=433,
        ),
        GrammarRule(
            name='association_list',
            body=
            Sequence(elements=[
                RuleReference(name='association_element', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='association_element', is_token=False),
                    ]),
                ),
            ]),
            line_number=440,
        ),
        GrammarRule(
            name='association_element',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Optional(element=
                        Sequence(elements=[
                            RuleReference(name='NAME', is_token=True),
                            RuleReference(name='ARROW', is_token=True),
                        ]),
                    ),
                    RuleReference(name='expression', is_token=False),
                ]),
                Sequence(elements=[
                    Optional(element=
                        Sequence(elements=[
                            RuleReference(name='NAME', is_token=True),
                            RuleReference(name='ARROW', is_token=True),
                        ]),
                    ),
                    Literal(value='open'),
                ]),
            ]),
            line_number=441,
        ),
        GrammarRule(
            name='generate_statement',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='COLON', is_token=True),
                Group(element=
                    Alternation(choices=[
                        RuleReference(name='for_generate', is_token=False),
                        RuleReference(name='if_generate', is_token=False),
                    ]),
                ),
            ]),
            line_number=464,
        ),
        GrammarRule(
            name='for_generate',
            body=
            Sequence(elements=[
                Literal(value='for'),
                RuleReference(name='NAME', is_token=True),
                Literal(value='in'),
                RuleReference(name='discrete_range', is_token=False),
                Literal(value='generate'),
                Repetition(element=
                    RuleReference(name='concurrent_statement', is_token=False),
                ),
                Literal(value='end'),
                Literal(value='generate'),
                Optional(element=
                    RuleReference(name='NAME', is_token=True),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=466,
        ),
        GrammarRule(
            name='if_generate',
            body=
            Sequence(elements=[
                Literal(value='if'),
                RuleReference(name='expression', is_token=False),
                Literal(value='generate'),
                Repetition(element=
                    RuleReference(name='concurrent_statement', is_token=False),
                ),
                Literal(value='end'),
                Literal(value='generate'),
                Optional(element=
                    RuleReference(name='NAME', is_token=True),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=470,
        ),
        GrammarRule(
            name='package_declaration',
            body=
            Sequence(elements=[
                Literal(value='package'),
                RuleReference(name='NAME', is_token=True),
                Literal(value='is'),
                Repetition(element=
                    RuleReference(name='package_declarative_item', is_token=False),
                ),
                Literal(value='end'),
                Optional(element=
                    Literal(value='package'),
                ),
                Optional(element=
                    RuleReference(name='NAME', is_token=True),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=491,
        ),
        GrammarRule(
            name='package_body',
            body=
            Sequence(elements=[
                Literal(value='package'),
                Literal(value='body'),
                RuleReference(name='NAME', is_token=True),
                Literal(value='is'),
                Repetition(element=
                    RuleReference(name='package_body_declarative_item', is_token=False),
                ),
                Literal(value='end'),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='package'),
                        Literal(value='body'),
                    ]),
                ),
                Optional(element=
                    RuleReference(name='NAME', is_token=True),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=495,
        ),
        GrammarRule(
            name='package_declarative_item',
            body=
            Alternation(choices=[
                RuleReference(name='type_declaration', is_token=False),
                RuleReference(name='subtype_declaration', is_token=False),
                RuleReference(name='constant_declaration', is_token=False),
                RuleReference(name='signal_declaration', is_token=False),
                RuleReference(name='component_declaration', is_token=False),
                RuleReference(name='function_declaration', is_token=False),
                RuleReference(name='procedure_declaration', is_token=False),
            ]),
            line_number=499,
        ),
        GrammarRule(
            name='package_body_declarative_item',
            body=
            Alternation(choices=[
                RuleReference(name='type_declaration', is_token=False),
                RuleReference(name='subtype_declaration', is_token=False),
                RuleReference(name='constant_declaration', is_token=False),
                RuleReference(name='function_body', is_token=False),
                RuleReference(name='procedure_body', is_token=False),
            ]),
            line_number=507,
        ),
        GrammarRule(
            name='function_declaration',
            body=
            Sequence(elements=[
                Optional(element=
                    Alternation(choices=[
                        Literal(value='pure'),
                        Literal(value='impure'),
                    ]),
                ),
                Literal(value='function'),
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='LPAREN', is_token=True),
                        RuleReference(name='interface_list', is_token=False),
                        RuleReference(name='RPAREN', is_token=True),
                    ]),
                ),
                Literal(value='return'),
                RuleReference(name='subtype_indication', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=523,
        ),
        GrammarRule(
            name='function_body',
            body=
            Sequence(elements=[
                Optional(element=
                    Alternation(choices=[
                        Literal(value='pure'),
                        Literal(value='impure'),
                    ]),
                ),
                Literal(value='function'),
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='LPAREN', is_token=True),
                        RuleReference(name='interface_list', is_token=False),
                        RuleReference(name='RPAREN', is_token=True),
                    ]),
                ),
                Literal(value='return'),
                RuleReference(name='subtype_indication', is_token=False),
                Literal(value='is'),
                Repetition(element=
                    RuleReference(name='process_declarative_item', is_token=False),
                ),
                Literal(value='begin'),
                Repetition(element=
                    RuleReference(name='sequential_statement', is_token=False),
                ),
                Literal(value='end'),
                Optional(element=
                    Literal(value='function'),
                ),
                Optional(element=
                    RuleReference(name='NAME', is_token=True),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=528,
        ),
        GrammarRule(
            name='procedure_declaration',
            body=
            Sequence(elements=[
                Literal(value='procedure'),
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='LPAREN', is_token=True),
                        RuleReference(name='interface_list', is_token=False),
                        RuleReference(name='RPAREN', is_token=True),
                    ]),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=537,
        ),
        GrammarRule(
            name='procedure_body',
            body=
            Sequence(elements=[
                Literal(value='procedure'),
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='LPAREN', is_token=True),
                        RuleReference(name='interface_list', is_token=False),
                        RuleReference(name='RPAREN', is_token=True),
                    ]),
                ),
                Literal(value='is'),
                Repetition(element=
                    RuleReference(name='process_declarative_item', is_token=False),
                ),
                Literal(value='begin'),
                Repetition(element=
                    RuleReference(name='sequential_statement', is_token=False),
                ),
                Literal(value='end'),
                Optional(element=
                    Literal(value='procedure'),
                ),
                Optional(element=
                    RuleReference(name='NAME', is_token=True),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=540,
        ),
        GrammarRule(
            name='expression',
            body=
            RuleReference(name='logical_expr', is_token=False),
            line_number=577,
        ),
        GrammarRule(
            name='logical_expr',
            body=
            Sequence(elements=[
                RuleReference(name='relation', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='logical_op', is_token=False),
                        RuleReference(name='relation', is_token=False),
                    ]),
                ),
            ]),
            line_number=584,
        ),
        GrammarRule(
            name='logical_op',
            body=
            Alternation(choices=[
                Literal(value='and'),
                Literal(value='or'),
                Literal(value='xor'),
                Literal(value='nand'),
                Literal(value='nor'),
                Literal(value='xnor'),
            ]),
            line_number=585,
        ),
        GrammarRule(
            name='relation',
            body=
            Sequence(elements=[
                RuleReference(name='shift_expr', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='relational_op', is_token=False),
                        RuleReference(name='shift_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=589,
        ),
        GrammarRule(
            name='relational_op',
            body=
            Alternation(choices=[
                RuleReference(name='EQUALS', is_token=True),
                RuleReference(name='NOT_EQUALS', is_token=True),
                RuleReference(name='LESS_THAN', is_token=True),
                RuleReference(name='LESS_EQUALS', is_token=True),
                RuleReference(name='GREATER_THAN', is_token=True),
                RuleReference(name='GREATER_EQUALS', is_token=True),
            ]),
            line_number=590,
        ),
        GrammarRule(
            name='shift_expr',
            body=
            Sequence(elements=[
                RuleReference(name='adding_expr', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='shift_op', is_token=False),
                        RuleReference(name='adding_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=595,
        ),
        GrammarRule(
            name='shift_op',
            body=
            Alternation(choices=[
                Literal(value='sll'),
                Literal(value='srl'),
                Literal(value='sla'),
                Literal(value='sra'),
                Literal(value='rol'),
                Literal(value='ror'),
            ]),
            line_number=596,
        ),
        GrammarRule(
            name='adding_expr',
            body=
            Sequence(elements=[
                RuleReference(name='multiplying_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='adding_op', is_token=False),
                        RuleReference(name='multiplying_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=600,
        ),
        GrammarRule(
            name='adding_op',
            body=
            Alternation(choices=[
                RuleReference(name='PLUS', is_token=True),
                RuleReference(name='MINUS', is_token=True),
                RuleReference(name='AMPERSAND', is_token=True),
            ]),
            line_number=601,
        ),
        GrammarRule(
            name='multiplying_expr',
            body=
            Sequence(elements=[
                RuleReference(name='unary_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='multiplying_op', is_token=False),
                        RuleReference(name='unary_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=604,
        ),
        GrammarRule(
            name='multiplying_op',
            body=
            Alternation(choices=[
                RuleReference(name='STAR', is_token=True),
                RuleReference(name='SLASH', is_token=True),
                Literal(value='mod'),
                Literal(value='rem'),
            ]),
            line_number=605,
        ),
        GrammarRule(
            name='unary_expr',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='abs'),
                    RuleReference(name='unary_expr', is_token=False),
                ]),
                Sequence(elements=[
                    Literal(value='not'),
                    RuleReference(name='unary_expr', is_token=False),
                ]),
                Sequence(elements=[
                    Group(element=
                        Alternation(choices=[
                            RuleReference(name='PLUS', is_token=True),
                            RuleReference(name='MINUS', is_token=True),
                        ]),
                    ),
                    RuleReference(name='unary_expr', is_token=False),
                ]),
                RuleReference(name='power_expr', is_token=False),
            ]),
            line_number=608,
        ),
        GrammarRule(
            name='power_expr',
            body=
            Sequence(elements=[
                RuleReference(name='primary', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='POWER', is_token=True),
                        RuleReference(name='primary', is_token=False),
                    ]),
                ),
            ]),
            line_number=614,
        ),
        GrammarRule(
            name='primary',
            body=
            Alternation(choices=[
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='REAL_NUMBER', is_token=True),
                RuleReference(name='BASED_LITERAL', is_token=True),
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='CHAR_LITERAL', is_token=True),
                RuleReference(name='BIT_STRING', is_token=True),
                Sequence(elements=[
                    RuleReference(name='NAME', is_token=True),
                    Optional(element=
                        Sequence(elements=[
                            RuleReference(name='TICK', is_token=True),
                            RuleReference(name='NAME', is_token=True),
                        ]),
                    ),
                ]),
                Sequence(elements=[
                    RuleReference(name='NAME', is_token=True),
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
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expression', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                RuleReference(name='aggregate', is_token=False),
                Literal(value='null'),
            ]),
            line_number=622,
        ),
        GrammarRule(
            name='aggregate',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='element_association', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='element_association', is_token=False),
                    ]),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=638,
        ),
        GrammarRule(
            name='element_association',
            body=
            Sequence(elements=[
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='choices', is_token=False),
                        RuleReference(name='ARROW', is_token=True),
                    ]),
                ),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=639,
        ),
    ],
)
