# AUTO-GENERATED FILE — DO NOT EDIT
# Source: verilog.grammar
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
    version=0,
    rules=[
        GrammarRule(
            name='source_text',
            body=
            Repetition(element=
                RuleReference(name='description', is_token=False),
            ),
            line_number=42,
        ),
        GrammarRule(
            name='description',
            body=
            RuleReference(name='module_declaration', is_token=False),
            line_number=44,
        ),
        GrammarRule(
            name='module_declaration',
            body=
            Sequence(elements=[
                Literal(value='module'),
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    RuleReference(name='parameter_port_list', is_token=False),
                ),
                Optional(element=
                    RuleReference(name='port_list', is_token=False),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
                Repetition(element=
                    RuleReference(name='module_item', is_token=False),
                ),
                Literal(value='endmodule'),
            ]),
            line_number=73,
        ),
        GrammarRule(
            name='parameter_port_list',
            body=
            Sequence(elements=[
                RuleReference(name='HASH', is_token=True),
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='parameter_declaration', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='parameter_declaration', is_token=False),
                    ]),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=91,
        ),
        GrammarRule(
            name='parameter_declaration',
            body=
            Sequence(elements=[
                Literal(value='parameter'),
                Optional(element=
                    RuleReference(name='range', is_token=False),
                ),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='EQUALS', is_token=True),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=94,
        ),
        GrammarRule(
            name='localparam_declaration',
            body=
            Sequence(elements=[
                Literal(value='localparam'),
                Optional(element=
                    RuleReference(name='range', is_token=False),
                ),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='EQUALS', is_token=True),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=95,
        ),
        GrammarRule(
            name='port_list',
            body=
            Sequence(elements=[
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='port', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='port', is_token=False),
                    ]),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=115,
        ),
        GrammarRule(
            name='port',
            body=
            Sequence(elements=[
                Optional(element=
                    RuleReference(name='port_direction', is_token=False),
                ),
                Optional(element=
                    RuleReference(name='net_type', is_token=False),
                ),
                Optional(element=
                    Literal(value='signed'),
                ),
                Optional(element=
                    RuleReference(name='range', is_token=False),
                ),
                RuleReference(name='NAME', is_token=True),
            ]),
            line_number=117,
        ),
        GrammarRule(
            name='port_direction',
            body=
            Alternation(choices=[
                Literal(value='input'),
                Literal(value='output'),
                Literal(value='inout'),
            ]),
            line_number=119,
        ),
        GrammarRule(
            name='net_type',
            body=
            Alternation(choices=[
                Literal(value='wire'),
                Literal(value='reg'),
                Literal(value='tri'),
                Literal(value='supply0'),
                Literal(value='supply1'),
            ]),
            line_number=120,
        ),
        GrammarRule(
            name='range',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACKET', is_token=True),
                RuleReference(name='expression', is_token=False),
                RuleReference(name='COLON', is_token=True),
                RuleReference(name='expression', is_token=False),
                RuleReference(name='RBRACKET', is_token=True),
            ]),
            line_number=122,
        ),
        GrammarRule(
            name='module_item',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='port_declaration', is_token=False),
                    RuleReference(name='SEMICOLON', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='net_declaration', is_token=False),
                    RuleReference(name='SEMICOLON', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='reg_declaration', is_token=False),
                    RuleReference(name='SEMICOLON', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='integer_declaration', is_token=False),
                    RuleReference(name='SEMICOLON', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='parameter_declaration', is_token=False),
                    RuleReference(name='SEMICOLON', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='localparam_declaration', is_token=False),
                    RuleReference(name='SEMICOLON', is_token=True),
                ]),
                RuleReference(name='continuous_assign', is_token=False),
                RuleReference(name='always_construct', is_token=False),
                RuleReference(name='initial_construct', is_token=False),
                RuleReference(name='module_instantiation', is_token=False),
                RuleReference(name='generate_region', is_token=False),
                RuleReference(name='function_declaration', is_token=False),
                RuleReference(name='task_declaration', is_token=False),
            ]),
            line_number=139,
        ),
        GrammarRule(
            name='port_declaration',
            body=
            Sequence(elements=[
                RuleReference(name='port_direction', is_token=False),
                Optional(element=
                    RuleReference(name='net_type', is_token=False),
                ),
                Optional(element=
                    Literal(value='signed'),
                ),
                Optional(element=
                    RuleReference(name='range', is_token=False),
                ),
                RuleReference(name='name_list', is_token=False),
            ]),
            line_number=174,
        ),
        GrammarRule(
            name='net_declaration',
            body=
            Sequence(elements=[
                RuleReference(name='net_type', is_token=False),
                Optional(element=
                    Literal(value='signed'),
                ),
                Optional(element=
                    RuleReference(name='range', is_token=False),
                ),
                RuleReference(name='name_list', is_token=False),
            ]),
            line_number=176,
        ),
        GrammarRule(
            name='reg_declaration',
            body=
            Sequence(elements=[
                Literal(value='reg'),
                Optional(element=
                    Literal(value='signed'),
                ),
                Optional(element=
                    RuleReference(name='range', is_token=False),
                ),
                RuleReference(name='name_list', is_token=False),
            ]),
            line_number=177,
        ),
        GrammarRule(
            name='integer_declaration',
            body=
            Sequence(elements=[
                Literal(value='integer'),
                RuleReference(name='name_list', is_token=False),
            ]),
            line_number=178,
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
            line_number=179,
        ),
        GrammarRule(
            name='continuous_assign',
            body=
            Sequence(elements=[
                Literal(value='assign'),
                RuleReference(name='assignment', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='assignment', is_token=False),
                    ]),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=198,
        ),
        GrammarRule(
            name='assignment',
            body=
            Sequence(elements=[
                RuleReference(name='lvalue', is_token=False),
                RuleReference(name='EQUALS', is_token=True),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=199,
        ),
        GrammarRule(
            name='lvalue',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='NAME', is_token=True),
                    Optional(element=
                        RuleReference(name='range_select', is_token=False),
                    ),
                ]),
                RuleReference(name='concatenation', is_token=False),
            ]),
            line_number=203,
        ),
        GrammarRule(
            name='range_select',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACKET', is_token=True),
                RuleReference(name='expression', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='COLON', is_token=True),
                        RuleReference(name='expression', is_token=False),
                    ]),
                ),
                RuleReference(name='RBRACKET', is_token=True),
            ]),
            line_number=206,
        ),
        GrammarRule(
            name='always_construct',
            body=
            Sequence(elements=[
                Literal(value='always'),
                RuleReference(name='AT', is_token=True),
                RuleReference(name='sensitivity_list', is_token=False),
                RuleReference(name='statement', is_token=False),
            ]),
            line_number=243,
        ),
        GrammarRule(
            name='initial_construct',
            body=
            Sequence(elements=[
                Literal(value='initial'),
                RuleReference(name='statement', is_token=False),
            ]),
            line_number=244,
        ),
        GrammarRule(
            name='sensitivity_list',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='sensitivity_item', is_token=False),
                    Repetition(element=
                        Sequence(elements=[
                            Group(element=
                                Alternation(choices=[
                                    Literal(value='or'),
                                    RuleReference(name='COMMA', is_token=True),
                                ]),
                            ),
                            RuleReference(name='sensitivity_item', is_token=False),
                        ]),
                    ),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='STAR', is_token=True),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
            ]),
            line_number=246,
        ),
        GrammarRule(
            name='sensitivity_item',
            body=
            Sequence(elements=[
                Optional(element=
                    Alternation(choices=[
                        Literal(value='posedge'),
                        Literal(value='negedge'),
                    ]),
                ),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=250,
        ),
        GrammarRule(
            name='statement',
            body=
            Alternation(choices=[
                RuleReference(name='block_statement', is_token=False),
                RuleReference(name='if_statement', is_token=False),
                RuleReference(name='case_statement', is_token=False),
                RuleReference(name='for_statement', is_token=False),
                Sequence(elements=[
                    RuleReference(name='blocking_assignment', is_token=False),
                    RuleReference(name='SEMICOLON', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='nonblocking_assignment', is_token=False),
                    RuleReference(name='SEMICOLON', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='task_call', is_token=False),
                    RuleReference(name='SEMICOLON', is_token=True),
                ]),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=259,
        ),
        GrammarRule(
            name='block_statement',
            body=
            Sequence(elements=[
                Literal(value='begin'),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='COLON', is_token=True),
                        RuleReference(name='NAME', is_token=True),
                    ]),
                ),
                Repetition(element=
                    RuleReference(name='statement', is_token=False),
                ),
                Literal(value='end'),
            ]),
            line_number=275,
        ),
        GrammarRule(
            name='if_statement',
            body=
            Sequence(elements=[
                Literal(value='if'),
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='expression', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
                RuleReference(name='statement', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='else'),
                        RuleReference(name='statement', is_token=False),
                    ]),
                ),
            ]),
            line_number=286,
        ),
        GrammarRule(
            name='case_statement',
            body=
            Sequence(elements=[
                Group(element=
                    Alternation(choices=[
                        Literal(value='case'),
                        Literal(value='casex'),
                        Literal(value='casez'),
                    ]),
                ),
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='expression', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
                Repetition(element=
                    RuleReference(name='case_item', is_token=False),
                ),
                Literal(value='endcase'),
            ]),
            line_number=301,
        ),
        GrammarRule(
            name='case_item',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='expression_list', is_token=False),
                    RuleReference(name='COLON', is_token=True),
                    RuleReference(name='statement', is_token=False),
                ]),
                Sequence(elements=[
                    Literal(value='default'),
                    Optional(element=
                        RuleReference(name='COLON', is_token=True),
                    ),
                    RuleReference(name='statement', is_token=False),
                ]),
            ]),
            line_number=306,
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
            ]),
            line_number=309,
        ),
        GrammarRule(
            name='for_statement',
            body=
            Sequence(elements=[
                Literal(value='for'),
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='blocking_assignment', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
                RuleReference(name='expression', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
                RuleReference(name='blocking_assignment', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
                RuleReference(name='statement', is_token=False),
            ]),
            line_number=313,
        ),
        GrammarRule(
            name='blocking_assignment',
            body=
            Sequence(elements=[
                RuleReference(name='lvalue', is_token=False),
                RuleReference(name='EQUALS', is_token=True),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=317,
        ),
        GrammarRule(
            name='nonblocking_assignment',
            body=
            Sequence(elements=[
                RuleReference(name='lvalue', is_token=False),
                RuleReference(name='LESS_EQUALS', is_token=True),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=318,
        ),
        GrammarRule(
            name='task_call',
            body=
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
            line_number=321,
        ),
        GrammarRule(
            name='module_instantiation',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                Optional(element=
                    RuleReference(name='parameter_value_assignment', is_token=False),
                ),
                RuleReference(name='instance', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='instance', is_token=False),
                    ]),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=340,
        ),
        GrammarRule(
            name='parameter_value_assignment',
            body=
            Sequence(elements=[
                RuleReference(name='HASH', is_token=True),
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='expression', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='expression', is_token=False),
                    ]),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=343,
        ),
        GrammarRule(
            name='instance',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='port_connections', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=345,
        ),
        GrammarRule(
            name='port_connections',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='named_port_connection', is_token=False),
                    Repetition(element=
                        Sequence(elements=[
                            RuleReference(name='COMMA', is_token=True),
                            RuleReference(name='named_port_connection', is_token=False),
                        ]),
                    ),
                ]),
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
            ]),
            line_number=347,
        ),
        GrammarRule(
            name='named_port_connection',
            body=
            Sequence(elements=[
                RuleReference(name='DOT', is_token=True),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='LPAREN', is_token=True),
                Optional(element=
                    RuleReference(name='expression', is_token=False),
                ),
                RuleReference(name='RPAREN', is_token=True),
            ]),
            line_number=350,
        ),
        GrammarRule(
            name='generate_region',
            body=
            Sequence(elements=[
                Literal(value='generate'),
                Repetition(element=
                    RuleReference(name='generate_item', is_token=False),
                ),
                Literal(value='endgenerate'),
            ]),
            line_number=377,
        ),
        GrammarRule(
            name='generate_item',
            body=
            Alternation(choices=[
                RuleReference(name='genvar_declaration', is_token=False),
                RuleReference(name='generate_for', is_token=False),
                RuleReference(name='generate_if', is_token=False),
                RuleReference(name='module_item', is_token=False),
            ]),
            line_number=379,
        ),
        GrammarRule(
            name='genvar_declaration',
            body=
            Sequence(elements=[
                Literal(value='genvar'),
                RuleReference(name='NAME', is_token=True),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='NAME', is_token=True),
                    ]),
                ),
                RuleReference(name='SEMICOLON', is_token=True),
            ]),
            line_number=384,
        ),
        GrammarRule(
            name='generate_for',
            body=
            Sequence(elements=[
                Literal(value='for'),
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='genvar_assignment', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
                RuleReference(name='expression', is_token=False),
                RuleReference(name='SEMICOLON', is_token=True),
                RuleReference(name='genvar_assignment', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
                RuleReference(name='generate_block', is_token=False),
            ]),
            line_number=386,
        ),
        GrammarRule(
            name='generate_if',
            body=
            Sequence(elements=[
                Literal(value='if'),
                RuleReference(name='LPAREN', is_token=True),
                RuleReference(name='expression', is_token=False),
                RuleReference(name='RPAREN', is_token=True),
                RuleReference(name='generate_block', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        Literal(value='else'),
                        RuleReference(name='generate_block', is_token=False),
                    ]),
                ),
            ]),
            line_number=390,
        ),
        GrammarRule(
            name='generate_block',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Literal(value='begin'),
                    Optional(element=
                        Sequence(elements=[
                            RuleReference(name='COLON', is_token=True),
                            RuleReference(name='NAME', is_token=True),
                        ]),
                    ),
                    Repetition(element=
                        RuleReference(name='generate_item', is_token=False),
                    ),
                    Literal(value='end'),
                ]),
                RuleReference(name='generate_item', is_token=False),
            ]),
            line_number=393,
        ),
        GrammarRule(
            name='genvar_assignment',
            body=
            Sequence(elements=[
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='EQUALS', is_token=True),
                RuleReference(name='expression', is_token=False),
            ]),
            line_number=396,
        ),
        GrammarRule(
            name='function_declaration',
            body=
            Sequence(elements=[
                Literal(value='function'),
                Optional(element=
                    RuleReference(name='range', is_token=False),
                ),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='SEMICOLON', is_token=True),
                Repetition(element=
                    RuleReference(name='function_item', is_token=False),
                ),
                RuleReference(name='statement', is_token=False),
                Literal(value='endfunction'),
            ]),
            line_number=415,
        ),
        GrammarRule(
            name='function_item',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='port_declaration', is_token=False),
                    RuleReference(name='SEMICOLON', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='reg_declaration', is_token=False),
                    RuleReference(name='SEMICOLON', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='integer_declaration', is_token=False),
                    RuleReference(name='SEMICOLON', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='parameter_declaration', is_token=False),
                    RuleReference(name='SEMICOLON', is_token=True),
                ]),
            ]),
            line_number=420,
        ),
        GrammarRule(
            name='task_declaration',
            body=
            Sequence(elements=[
                Literal(value='task'),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='SEMICOLON', is_token=True),
                Repetition(element=
                    RuleReference(name='task_item', is_token=False),
                ),
                RuleReference(name='statement', is_token=False),
                Literal(value='endtask'),
            ]),
            line_number=425,
        ),
        GrammarRule(
            name='task_item',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    RuleReference(name='port_declaration', is_token=False),
                    RuleReference(name='SEMICOLON', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='reg_declaration', is_token=False),
                    RuleReference(name='SEMICOLON', is_token=True),
                ]),
                Sequence(elements=[
                    RuleReference(name='integer_declaration', is_token=False),
                    RuleReference(name='SEMICOLON', is_token=True),
                ]),
            ]),
            line_number=430,
        ),
        GrammarRule(
            name='expression',
            body=
            RuleReference(name='ternary_expr', is_token=False),
            line_number=458,
        ),
        GrammarRule(
            name='ternary_expr',
            body=
            Sequence(elements=[
                RuleReference(name='or_expr', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='QUESTION', is_token=True),
                        RuleReference(name='expression', is_token=False),
                        RuleReference(name='COLON', is_token=True),
                        RuleReference(name='ternary_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=464,
        ),
        GrammarRule(
            name='or_expr',
            body=
            Sequence(elements=[
                RuleReference(name='and_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='LOGIC_OR', is_token=True),
                        RuleReference(name='and_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=467,
        ),
        GrammarRule(
            name='and_expr',
            body=
            Sequence(elements=[
                RuleReference(name='bit_or_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='LOGIC_AND', is_token=True),
                        RuleReference(name='bit_or_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=468,
        ),
        GrammarRule(
            name='bit_or_expr',
            body=
            Sequence(elements=[
                RuleReference(name='bit_xor_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='PIPE', is_token=True),
                        RuleReference(name='bit_xor_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=471,
        ),
        GrammarRule(
            name='bit_xor_expr',
            body=
            Sequence(elements=[
                RuleReference(name='bit_and_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='CARET', is_token=True),
                        RuleReference(name='bit_and_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=472,
        ),
        GrammarRule(
            name='bit_and_expr',
            body=
            Sequence(elements=[
                RuleReference(name='equality_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='AMP', is_token=True),
                        RuleReference(name='equality_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=473,
        ),
        GrammarRule(
            name='equality_expr',
            body=
            Sequence(elements=[
                RuleReference(name='relational_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='EQUALS_EQUALS', is_token=True),
                                RuleReference(name='NOT_EQUALS', is_token=True),
                                RuleReference(name='CASE_EQ', is_token=True),
                                RuleReference(name='CASE_NEQ', is_token=True),
                            ]),
                        ),
                        RuleReference(name='relational_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=477,
        ),
        GrammarRule(
            name='relational_expr',
            body=
            Sequence(elements=[
                RuleReference(name='shift_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='LESS_THAN', is_token=True),
                                RuleReference(name='LESS_EQUALS', is_token=True),
                                RuleReference(name='GREATER_THAN', is_token=True),
                                RuleReference(name='GREATER_EQUALS', is_token=True),
                            ]),
                        ),
                        RuleReference(name='shift_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=484,
        ),
        GrammarRule(
            name='shift_expr',
            body=
            Sequence(elements=[
                RuleReference(name='additive_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='LEFT_SHIFT', is_token=True),
                                RuleReference(name='RIGHT_SHIFT', is_token=True),
                                RuleReference(name='ARITH_LEFT_SHIFT', is_token=True),
                                RuleReference(name='ARITH_RIGHT_SHIFT', is_token=True),
                            ]),
                        ),
                        RuleReference(name='additive_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=489,
        ),
        GrammarRule(
            name='additive_expr',
            body=
            Sequence(elements=[
                RuleReference(name='multiplicative_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='PLUS', is_token=True),
                                RuleReference(name='MINUS', is_token=True),
                            ]),
                        ),
                        RuleReference(name='multiplicative_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=494,
        ),
        GrammarRule(
            name='multiplicative_expr',
            body=
            Sequence(elements=[
                RuleReference(name='power_expr', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        Group(element=
                            Alternation(choices=[
                                RuleReference(name='STAR', is_token=True),
                                RuleReference(name='SLASH', is_token=True),
                                RuleReference(name='PERCENT', is_token=True),
                            ]),
                        ),
                        RuleReference(name='power_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=495,
        ),
        GrammarRule(
            name='power_expr',
            body=
            Sequence(elements=[
                RuleReference(name='unary_expr', is_token=False),
                Optional(element=
                    Sequence(elements=[
                        RuleReference(name='POWER', is_token=True),
                        RuleReference(name='unary_expr', is_token=False),
                    ]),
                ),
            ]),
            line_number=496,
        ),
        GrammarRule(
            name='unary_expr',
            body=
            Alternation(choices=[
                Sequence(elements=[
                    Group(element=
                        Alternation(choices=[
                            RuleReference(name='PLUS', is_token=True),
                            RuleReference(name='MINUS', is_token=True),
                            RuleReference(name='BANG', is_token=True),
                            RuleReference(name='TILDE', is_token=True),
                            RuleReference(name='AMP', is_token=True),
                            RuleReference(name='PIPE', is_token=True),
                            RuleReference(name='CARET', is_token=True),
                            Sequence(elements=[
                                RuleReference(name='TILDE', is_token=True),
                                RuleReference(name='AMP', is_token=True),
                            ]),
                            Sequence(elements=[
                                RuleReference(name='TILDE', is_token=True),
                                RuleReference(name='PIPE', is_token=True),
                            ]),
                            Sequence(elements=[
                                RuleReference(name='TILDE', is_token=True),
                                RuleReference(name='CARET', is_token=True),
                            ]),
                        ]),
                    ),
                    RuleReference(name='unary_expr', is_token=False),
                ]),
                RuleReference(name='primary', is_token=False),
            ]),
            line_number=508,
        ),
        GrammarRule(
            name='primary',
            body=
            Alternation(choices=[
                RuleReference(name='NUMBER', is_token=True),
                RuleReference(name='SIZED_NUMBER', is_token=True),
                RuleReference(name='REAL_NUMBER', is_token=True),
                RuleReference(name='STRING', is_token=True),
                RuleReference(name='NAME', is_token=True),
                RuleReference(name='SYSTEM_ID', is_token=True),
                Sequence(elements=[
                    RuleReference(name='LPAREN', is_token=True),
                    RuleReference(name='expression', is_token=False),
                    RuleReference(name='RPAREN', is_token=True),
                ]),
                RuleReference(name='concatenation', is_token=False),
                RuleReference(name='replication', is_token=False),
                Sequence(elements=[
                    RuleReference(name='primary', is_token=False),
                    RuleReference(name='LBRACKET', is_token=True),
                    RuleReference(name='expression', is_token=False),
                    Optional(element=
                        Sequence(elements=[
                            RuleReference(name='COLON', is_token=True),
                            RuleReference(name='expression', is_token=False),
                        ]),
                    ),
                    RuleReference(name='RBRACKET', is_token=True),
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
            ]),
            line_number=518,
        ),
        GrammarRule(
            name='concatenation',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACE', is_token=True),
                RuleReference(name='expression', is_token=False),
                Repetition(element=
                    Sequence(elements=[
                        RuleReference(name='COMMA', is_token=True),
                        RuleReference(name='expression', is_token=False),
                    ]),
                ),
                RuleReference(name='RBRACE', is_token=True),
            ]),
            line_number=534,
        ),
        GrammarRule(
            name='replication',
            body=
            Sequence(elements=[
                RuleReference(name='LBRACE', is_token=True),
                RuleReference(name='expression', is_token=False),
                RuleReference(name='concatenation', is_token=False),
                RuleReference(name='RBRACE', is_token=True),
            ]),
            line_number=540,
        ),
    ],
)
