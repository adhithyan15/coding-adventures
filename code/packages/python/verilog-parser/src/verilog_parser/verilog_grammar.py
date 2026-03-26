# AUTO-GENERATED FILE - DO NOT EDIT
from grammar_tools.parser_grammar import (
    ParserGrammar, GrammarRule, GrammarElement,
    RuleReference, Literal, Sequence, Alternation,
    Repetition, Optional as OptGroup, Group
)

VerilogGrammar = ParserGrammar(
    version=0,
    rules=[
        GrammarRule(
            name="source_text",
            line_number=42,
            body=Repetition(element=RuleReference(name="description", is_token=False)),
        ),
        GrammarRule(
            name="description",
            line_number=44,
            body=RuleReference(name="module_declaration", is_token=False),
        ),
        GrammarRule(
            name="module_declaration",
            line_number=73,
            body=Sequence(elements=[Literal(value="module"), RuleReference(name="NAME", is_token=True), OptGroup(element=RuleReference(name="parameter_port_list", is_token=False)), OptGroup(element=RuleReference(name="port_list", is_token=False)), RuleReference(name="SEMICOLON", is_token=True), Repetition(element=RuleReference(name="module_item", is_token=False)), Literal(value="endmodule")]),
        ),
        GrammarRule(
            name="parameter_port_list",
            line_number=91,
            body=Sequence(elements=[RuleReference(name="HASH", is_token=True), RuleReference(name="LPAREN", is_token=True), RuleReference(name="parameter_declaration", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="parameter_declaration", is_token=False)])), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="parameter_declaration",
            line_number=94,
            body=Sequence(elements=[Literal(value="parameter"), OptGroup(element=RuleReference(name="range", is_token=False)), RuleReference(name="NAME", is_token=True), RuleReference(name="EQUALS", is_token=True), RuleReference(name="expression", is_token=False)]),
        ),
        GrammarRule(
            name="localparam_declaration",
            line_number=95,
            body=Sequence(elements=[Literal(value="localparam"), OptGroup(element=RuleReference(name="range", is_token=False)), RuleReference(name="NAME", is_token=True), RuleReference(name="EQUALS", is_token=True), RuleReference(name="expression", is_token=False)]),
        ),
        GrammarRule(
            name="port_list",
            line_number=115,
            body=Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="port", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="port", is_token=False)])), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="port",
            line_number=117,
            body=Sequence(elements=[OptGroup(element=RuleReference(name="port_direction", is_token=False)), OptGroup(element=RuleReference(name="net_type", is_token=False)), OptGroup(element=Literal(value="signed")), OptGroup(element=RuleReference(name="range", is_token=False)), RuleReference(name="NAME", is_token=True)]),
        ),
        GrammarRule(
            name="port_direction",
            line_number=119,
            body=Alternation(choices=[Literal(value="input"), Literal(value="output"), Literal(value="inout")]),
        ),
        GrammarRule(
            name="net_type",
            line_number=120,
            body=Alternation(choices=[Literal(value="wire"), Literal(value="reg"), Literal(value="tri"), Literal(value="supply0"), Literal(value="supply1")]),
        ),
        GrammarRule(
            name="range",
            line_number=122,
            body=Sequence(elements=[RuleReference(name="LBRACKET", is_token=True), RuleReference(name="expression", is_token=False), RuleReference(name="COLON", is_token=True), RuleReference(name="expression", is_token=False), RuleReference(name="RBRACKET", is_token=True)]),
        ),
        GrammarRule(
            name="module_item",
            line_number=139,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="port_declaration", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]), Sequence(elements=[RuleReference(name="net_declaration", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]), Sequence(elements=[RuleReference(name="reg_declaration", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]), Sequence(elements=[RuleReference(name="integer_declaration", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]), Sequence(elements=[RuleReference(name="parameter_declaration", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]), Sequence(elements=[RuleReference(name="localparam_declaration", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]), RuleReference(name="continuous_assign", is_token=False), RuleReference(name="always_construct", is_token=False), RuleReference(name="initial_construct", is_token=False), RuleReference(name="module_instantiation", is_token=False), RuleReference(name="generate_region", is_token=False), RuleReference(name="function_declaration", is_token=False), RuleReference(name="task_declaration", is_token=False)]),
        ),
        GrammarRule(
            name="port_declaration",
            line_number=174,
            body=Sequence(elements=[RuleReference(name="port_direction", is_token=False), OptGroup(element=RuleReference(name="net_type", is_token=False)), OptGroup(element=Literal(value="signed")), OptGroup(element=RuleReference(name="range", is_token=False)), RuleReference(name="name_list", is_token=False)]),
        ),
        GrammarRule(
            name="net_declaration",
            line_number=176,
            body=Sequence(elements=[RuleReference(name="net_type", is_token=False), OptGroup(element=Literal(value="signed")), OptGroup(element=RuleReference(name="range", is_token=False)), RuleReference(name="name_list", is_token=False)]),
        ),
        GrammarRule(
            name="reg_declaration",
            line_number=177,
            body=Sequence(elements=[Literal(value="reg"), OptGroup(element=Literal(value="signed")), OptGroup(element=RuleReference(name="range", is_token=False)), RuleReference(name="name_list", is_token=False)]),
        ),
        GrammarRule(
            name="integer_declaration",
            line_number=178,
            body=Sequence(elements=[Literal(value="integer"), RuleReference(name="name_list", is_token=False)]),
        ),
        GrammarRule(
            name="name_list",
            line_number=179,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="NAME", is_token=True)]))]),
        ),
        GrammarRule(
            name="continuous_assign",
            line_number=198,
            body=Sequence(elements=[Literal(value="assign"), RuleReference(name="assignment", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="assignment", is_token=False)])), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="assignment",
            line_number=199,
            body=Sequence(elements=[RuleReference(name="lvalue", is_token=False), RuleReference(name="EQUALS", is_token=True), RuleReference(name="expression", is_token=False)]),
        ),
        GrammarRule(
            name="lvalue",
            line_number=203,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="NAME", is_token=True), OptGroup(element=RuleReference(name="range_select", is_token=False))]), RuleReference(name="concatenation", is_token=False)]),
        ),
        GrammarRule(
            name="range_select",
            line_number=206,
            body=Sequence(elements=[RuleReference(name="LBRACKET", is_token=True), RuleReference(name="expression", is_token=False), OptGroup(element=Sequence(elements=[RuleReference(name="COLON", is_token=True), RuleReference(name="expression", is_token=False)])), RuleReference(name="RBRACKET", is_token=True)]),
        ),
        GrammarRule(
            name="always_construct",
            line_number=243,
            body=Sequence(elements=[Literal(value="always"), RuleReference(name="AT", is_token=True), RuleReference(name="sensitivity_list", is_token=False), RuleReference(name="statement", is_token=False)]),
        ),
        GrammarRule(
            name="initial_construct",
            line_number=244,
            body=Sequence(elements=[Literal(value="initial"), RuleReference(name="statement", is_token=False)]),
        ),
        GrammarRule(
            name="sensitivity_list",
            line_number=246,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="sensitivity_item", is_token=False), Repetition(element=Sequence(elements=[Group(element=Alternation(choices=[Literal(value="or"), RuleReference(name="COMMA", is_token=True)])), RuleReference(name="sensitivity_item", is_token=False)])), RuleReference(name="RPAREN", is_token=True)]), Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="STAR", is_token=True), RuleReference(name="RPAREN", is_token=True)])]),
        ),
        GrammarRule(
            name="sensitivity_item",
            line_number=250,
            body=Sequence(elements=[OptGroup(element=Alternation(choices=[Literal(value="posedge"), Literal(value="negedge")])), RuleReference(name="expression", is_token=False)]),
        ),
        GrammarRule(
            name="statement",
            line_number=259,
            body=Alternation(choices=[RuleReference(name="block_statement", is_token=False), RuleReference(name="if_statement", is_token=False), RuleReference(name="case_statement", is_token=False), RuleReference(name="for_statement", is_token=False), Sequence(elements=[RuleReference(name="blocking_assignment", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]), Sequence(elements=[RuleReference(name="nonblocking_assignment", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]), Sequence(elements=[RuleReference(name="task_call", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="block_statement",
            line_number=275,
            body=Sequence(elements=[Literal(value="begin"), OptGroup(element=Sequence(elements=[RuleReference(name="COLON", is_token=True), RuleReference(name="NAME", is_token=True)])), Repetition(element=RuleReference(name="statement", is_token=False)), Literal(value="end")]),
        ),
        GrammarRule(
            name="if_statement",
            line_number=286,
            body=Sequence(elements=[Literal(value="if"), RuleReference(name="LPAREN", is_token=True), RuleReference(name="expression", is_token=False), RuleReference(name="RPAREN", is_token=True), RuleReference(name="statement", is_token=False), OptGroup(element=Sequence(elements=[Literal(value="else"), RuleReference(name="statement", is_token=False)]))]),
        ),
        GrammarRule(
            name="case_statement",
            line_number=301,
            body=Sequence(elements=[Group(element=Alternation(choices=[Literal(value="case"), Literal(value="casex"), Literal(value="casez")])), RuleReference(name="LPAREN", is_token=True), RuleReference(name="expression", is_token=False), RuleReference(name="RPAREN", is_token=True), Repetition(element=RuleReference(name="case_item", is_token=False)), Literal(value="endcase")]),
        ),
        GrammarRule(
            name="case_item",
            line_number=306,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="expression_list", is_token=False), RuleReference(name="COLON", is_token=True), RuleReference(name="statement", is_token=False)]), Sequence(elements=[Literal(value="default"), OptGroup(element=RuleReference(name="COLON", is_token=True)), RuleReference(name="statement", is_token=False)])]),
        ),
        GrammarRule(
            name="expression_list",
            line_number=309,
            body=Sequence(elements=[RuleReference(name="expression", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="expression", is_token=False)]))]),
        ),
        GrammarRule(
            name="for_statement",
            line_number=313,
            body=Sequence(elements=[Literal(value="for"), RuleReference(name="LPAREN", is_token=True), RuleReference(name="blocking_assignment", is_token=False), RuleReference(name="SEMICOLON", is_token=True), RuleReference(name="expression", is_token=False), RuleReference(name="SEMICOLON", is_token=True), RuleReference(name="blocking_assignment", is_token=False), RuleReference(name="RPAREN", is_token=True), RuleReference(name="statement", is_token=False)]),
        ),
        GrammarRule(
            name="blocking_assignment",
            line_number=317,
            body=Sequence(elements=[RuleReference(name="lvalue", is_token=False), RuleReference(name="EQUALS", is_token=True), RuleReference(name="expression", is_token=False)]),
        ),
        GrammarRule(
            name="nonblocking_assignment",
            line_number=318,
            body=Sequence(elements=[RuleReference(name="lvalue", is_token=False), RuleReference(name="LESS_EQUALS", is_token=True), RuleReference(name="expression", is_token=False)]),
        ),
        GrammarRule(
            name="task_call",
            line_number=321,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="LPAREN", is_token=True), OptGroup(element=Sequence(elements=[RuleReference(name="expression", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="expression", is_token=False)]))])), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="module_instantiation",
            line_number=340,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), OptGroup(element=RuleReference(name="parameter_value_assignment", is_token=False)), RuleReference(name="instance", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="instance", is_token=False)])), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="parameter_value_assignment",
            line_number=343,
            body=Sequence(elements=[RuleReference(name="HASH", is_token=True), RuleReference(name="LPAREN", is_token=True), RuleReference(name="expression", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="expression", is_token=False)])), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="instance",
            line_number=345,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="LPAREN", is_token=True), RuleReference(name="port_connections", is_token=False), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="port_connections",
            line_number=347,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="named_port_connection", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="named_port_connection", is_token=False)]))]), OptGroup(element=Sequence(elements=[RuleReference(name="expression", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="expression", is_token=False)]))]))]),
        ),
        GrammarRule(
            name="named_port_connection",
            line_number=350,
            body=Sequence(elements=[RuleReference(name="DOT", is_token=True), RuleReference(name="NAME", is_token=True), RuleReference(name="LPAREN", is_token=True), OptGroup(element=RuleReference(name="expression", is_token=False)), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="generate_region",
            line_number=377,
            body=Sequence(elements=[Literal(value="generate"), Repetition(element=RuleReference(name="generate_item", is_token=False)), Literal(value="endgenerate")]),
        ),
        GrammarRule(
            name="generate_item",
            line_number=379,
            body=Alternation(choices=[RuleReference(name="genvar_declaration", is_token=False), RuleReference(name="generate_for", is_token=False), RuleReference(name="generate_if", is_token=False), RuleReference(name="module_item", is_token=False)]),
        ),
        GrammarRule(
            name="genvar_declaration",
            line_number=384,
            body=Sequence(elements=[Literal(value="genvar"), RuleReference(name="NAME", is_token=True), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="NAME", is_token=True)])), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="generate_for",
            line_number=386,
            body=Sequence(elements=[Literal(value="for"), RuleReference(name="LPAREN", is_token=True), RuleReference(name="genvar_assignment", is_token=False), RuleReference(name="SEMICOLON", is_token=True), RuleReference(name="expression", is_token=False), RuleReference(name="SEMICOLON", is_token=True), RuleReference(name="genvar_assignment", is_token=False), RuleReference(name="RPAREN", is_token=True), RuleReference(name="generate_block", is_token=False)]),
        ),
        GrammarRule(
            name="generate_if",
            line_number=390,
            body=Sequence(elements=[Literal(value="if"), RuleReference(name="LPAREN", is_token=True), RuleReference(name="expression", is_token=False), RuleReference(name="RPAREN", is_token=True), RuleReference(name="generate_block", is_token=False), OptGroup(element=Sequence(elements=[Literal(value="else"), RuleReference(name="generate_block", is_token=False)]))]),
        ),
        GrammarRule(
            name="generate_block",
            line_number=393,
            body=Alternation(choices=[Sequence(elements=[Literal(value="begin"), OptGroup(element=Sequence(elements=[RuleReference(name="COLON", is_token=True), RuleReference(name="NAME", is_token=True)])), Repetition(element=RuleReference(name="generate_item", is_token=False)), Literal(value="end")]), RuleReference(name="generate_item", is_token=False)]),
        ),
        GrammarRule(
            name="genvar_assignment",
            line_number=396,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="EQUALS", is_token=True), RuleReference(name="expression", is_token=False)]),
        ),
        GrammarRule(
            name="function_declaration",
            line_number=415,
            body=Sequence(elements=[Literal(value="function"), OptGroup(element=RuleReference(name="range", is_token=False)), RuleReference(name="NAME", is_token=True), RuleReference(name="SEMICOLON", is_token=True), Repetition(element=RuleReference(name="function_item", is_token=False)), RuleReference(name="statement", is_token=False), Literal(value="endfunction")]),
        ),
        GrammarRule(
            name="function_item",
            line_number=420,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="port_declaration", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]), Sequence(elements=[RuleReference(name="reg_declaration", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]), Sequence(elements=[RuleReference(name="integer_declaration", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]), Sequence(elements=[RuleReference(name="parameter_declaration", is_token=False), RuleReference(name="SEMICOLON", is_token=True)])]),
        ),
        GrammarRule(
            name="task_declaration",
            line_number=425,
            body=Sequence(elements=[Literal(value="task"), RuleReference(name="NAME", is_token=True), RuleReference(name="SEMICOLON", is_token=True), Repetition(element=RuleReference(name="task_item", is_token=False)), RuleReference(name="statement", is_token=False), Literal(value="endtask")]),
        ),
        GrammarRule(
            name="task_item",
            line_number=430,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="port_declaration", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]), Sequence(elements=[RuleReference(name="reg_declaration", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]), Sequence(elements=[RuleReference(name="integer_declaration", is_token=False), RuleReference(name="SEMICOLON", is_token=True)])]),
        ),
        GrammarRule(
            name="expression",
            line_number=458,
            body=RuleReference(name="ternary_expr", is_token=False),
        ),
        GrammarRule(
            name="ternary_expr",
            line_number=464,
            body=Sequence(elements=[RuleReference(name="or_expr", is_token=False), OptGroup(element=Sequence(elements=[RuleReference(name="QUESTION", is_token=True), RuleReference(name="expression", is_token=False), RuleReference(name="COLON", is_token=True), RuleReference(name="ternary_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="or_expr",
            line_number=467,
            body=Sequence(elements=[RuleReference(name="and_expr", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="LOGIC_OR", is_token=True), RuleReference(name="and_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="and_expr",
            line_number=468,
            body=Sequence(elements=[RuleReference(name="bit_or_expr", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="LOGIC_AND", is_token=True), RuleReference(name="bit_or_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="bit_or_expr",
            line_number=471,
            body=Sequence(elements=[RuleReference(name="bit_xor_expr", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="PIPE", is_token=True), RuleReference(name="bit_xor_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="bit_xor_expr",
            line_number=472,
            body=Sequence(elements=[RuleReference(name="bit_and_expr", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="CARET", is_token=True), RuleReference(name="bit_and_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="bit_and_expr",
            line_number=473,
            body=Sequence(elements=[RuleReference(name="equality_expr", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="AMP", is_token=True), RuleReference(name="equality_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="equality_expr",
            line_number=477,
            body=Sequence(elements=[RuleReference(name="relational_expr", is_token=False), Repetition(element=Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="EQUALS_EQUALS", is_token=True), RuleReference(name="NOT_EQUALS", is_token=True), RuleReference(name="CASE_EQ", is_token=True), RuleReference(name="CASE_NEQ", is_token=True)])), RuleReference(name="relational_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="relational_expr",
            line_number=484,
            body=Sequence(elements=[RuleReference(name="shift_expr", is_token=False), Repetition(element=Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="LESS_THAN", is_token=True), RuleReference(name="LESS_EQUALS", is_token=True), RuleReference(name="GREATER_THAN", is_token=True), RuleReference(name="GREATER_EQUALS", is_token=True)])), RuleReference(name="shift_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="shift_expr",
            line_number=489,
            body=Sequence(elements=[RuleReference(name="additive_expr", is_token=False), Repetition(element=Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="LEFT_SHIFT", is_token=True), RuleReference(name="RIGHT_SHIFT", is_token=True), RuleReference(name="ARITH_LEFT_SHIFT", is_token=True), RuleReference(name="ARITH_RIGHT_SHIFT", is_token=True)])), RuleReference(name="additive_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="additive_expr",
            line_number=494,
            body=Sequence(elements=[RuleReference(name="multiplicative_expr", is_token=False), Repetition(element=Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="PLUS", is_token=True), RuleReference(name="MINUS", is_token=True)])), RuleReference(name="multiplicative_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="multiplicative_expr",
            line_number=495,
            body=Sequence(elements=[RuleReference(name="power_expr", is_token=False), Repetition(element=Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="STAR", is_token=True), RuleReference(name="SLASH", is_token=True), RuleReference(name="PERCENT", is_token=True)])), RuleReference(name="power_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="power_expr",
            line_number=496,
            body=Sequence(elements=[RuleReference(name="unary_expr", is_token=False), OptGroup(element=Sequence(elements=[RuleReference(name="POWER", is_token=True), RuleReference(name="unary_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="unary_expr",
            line_number=508,
            body=Alternation(choices=[Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="PLUS", is_token=True), RuleReference(name="MINUS", is_token=True), RuleReference(name="BANG", is_token=True), RuleReference(name="TILDE", is_token=True), RuleReference(name="AMP", is_token=True), RuleReference(name="PIPE", is_token=True), RuleReference(name="CARET", is_token=True), Sequence(elements=[RuleReference(name="TILDE", is_token=True), RuleReference(name="AMP", is_token=True)]), Sequence(elements=[RuleReference(name="TILDE", is_token=True), RuleReference(name="PIPE", is_token=True)]), Sequence(elements=[RuleReference(name="TILDE", is_token=True), RuleReference(name="CARET", is_token=True)])])), RuleReference(name="unary_expr", is_token=False)]), RuleReference(name="primary", is_token=False)]),
        ),
        GrammarRule(
            name="primary",
            line_number=518,
            body=Alternation(choices=[RuleReference(name="NUMBER", is_token=True), RuleReference(name="SIZED_NUMBER", is_token=True), RuleReference(name="REAL_NUMBER", is_token=True), RuleReference(name="STRING", is_token=True), RuleReference(name="NAME", is_token=True), RuleReference(name="SYSTEM_ID", is_token=True), Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="expression", is_token=False), RuleReference(name="RPAREN", is_token=True)]), RuleReference(name="concatenation", is_token=False), RuleReference(name="replication", is_token=False), Sequence(elements=[RuleReference(name="primary", is_token=False), RuleReference(name="LBRACKET", is_token=True), RuleReference(name="expression", is_token=False), OptGroup(element=Sequence(elements=[RuleReference(name="COLON", is_token=True), RuleReference(name="expression", is_token=False)])), RuleReference(name="RBRACKET", is_token=True)]), Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="LPAREN", is_token=True), OptGroup(element=Sequence(elements=[RuleReference(name="expression", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="expression", is_token=False)]))])), RuleReference(name="RPAREN", is_token=True)])]),
        ),
        GrammarRule(
            name="concatenation",
            line_number=534,
            body=Sequence(elements=[RuleReference(name="LBRACE", is_token=True), RuleReference(name="expression", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="expression", is_token=False)])), RuleReference(name="RBRACE", is_token=True)]),
        ),
        GrammarRule(
            name="replication",
            line_number=540,
            body=Sequence(elements=[RuleReference(name="LBRACE", is_token=True), RuleReference(name="expression", is_token=False), RuleReference(name="concatenation", is_token=False), RuleReference(name="RBRACE", is_token=True)]),
        ),
    ],
)
