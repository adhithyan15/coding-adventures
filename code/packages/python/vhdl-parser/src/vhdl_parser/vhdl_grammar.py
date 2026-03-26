# AUTO-GENERATED FILE - DO NOT EDIT
from grammar_tools.parser_grammar import (
    ParserGrammar, GrammarRule, GrammarElement,
    RuleReference, Literal, Sequence, Alternation,
    Repetition, Optional as OptGroup, Group
)

VhdlGrammar = ParserGrammar(
    version=0,
    rules=[
        GrammarRule(
            name="design_file",
            line_number=64,
            body=Repetition(element=RuleReference(name="design_unit", is_token=False)),
        ),
        GrammarRule(
            name="design_unit",
            line_number=66,
            body=Sequence(elements=[Repetition(element=RuleReference(name="context_item", is_token=False)), RuleReference(name="library_unit", is_token=False)]),
        ),
        GrammarRule(
            name="context_item",
            line_number=68,
            body=Alternation(choices=[RuleReference(name="library_clause", is_token=False), RuleReference(name="use_clause", is_token=False)]),
        ),
        GrammarRule(
            name="library_clause",
            line_number=71,
            body=Sequence(elements=[Literal(value="library"), RuleReference(name="name_list", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="use_clause",
            line_number=74,
            body=Sequence(elements=[Literal(value="use"), RuleReference(name="selected_name", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="selected_name",
            line_number=77,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), Repetition(element=Sequence(elements=[RuleReference(name="DOT", is_token=True), Group(element=Alternation(choices=[RuleReference(name="NAME", is_token=True), Literal(value="all")]))]))]),
        ),
        GrammarRule(
            name="name_list",
            line_number=79,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="NAME", is_token=True)]))]),
        ),
        GrammarRule(
            name="library_unit",
            line_number=81,
            body=Alternation(choices=[RuleReference(name="entity_declaration", is_token=False), RuleReference(name="architecture_body", is_token=False), RuleReference(name="package_declaration", is_token=False), RuleReference(name="package_body", is_token=False)]),
        ),
        GrammarRule(
            name="entity_declaration",
            line_number=112,
            body=Sequence(elements=[Literal(value="entity"), RuleReference(name="NAME", is_token=True), Literal(value="is"), OptGroup(element=RuleReference(name="generic_clause", is_token=False)), OptGroup(element=RuleReference(name="port_clause", is_token=False)), Literal(value="end"), OptGroup(element=Literal(value="entity")), OptGroup(element=RuleReference(name="NAME", is_token=True)), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="generic_clause",
            line_number=117,
            body=Sequence(elements=[Literal(value="generic"), RuleReference(name="LPAREN", is_token=True), RuleReference(name="interface_list", is_token=False), RuleReference(name="RPAREN", is_token=True), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="port_clause",
            line_number=118,
            body=Sequence(elements=[Literal(value="port"), RuleReference(name="LPAREN", is_token=True), RuleReference(name="interface_list", is_token=False), RuleReference(name="RPAREN", is_token=True), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="interface_list",
            line_number=123,
            body=Sequence(elements=[RuleReference(name="interface_element", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="SEMICOLON", is_token=True), RuleReference(name="interface_element", is_token=False)]))]),
        ),
        GrammarRule(
            name="interface_element",
            line_number=124,
            body=Sequence(elements=[RuleReference(name="name_list", is_token=False), RuleReference(name="COLON", is_token=True), OptGroup(element=RuleReference(name="mode", is_token=False)), RuleReference(name="subtype_indication", is_token=False), OptGroup(element=Sequence(elements=[RuleReference(name="VAR_ASSIGN", is_token=True), RuleReference(name="expression", is_token=False)]))]),
        ),
        GrammarRule(
            name="mode",
            line_number=132,
            body=Alternation(choices=[Literal(value="in"), Literal(value="out"), Literal(value="inout"), Literal(value="buffer")]),
        ),
        GrammarRule(
            name="architecture_body",
            line_number=154,
            body=Sequence(elements=[Literal(value="architecture"), RuleReference(name="NAME", is_token=True), Literal(value="of"), RuleReference(name="NAME", is_token=True), Literal(value="is"), Repetition(element=RuleReference(name="block_declarative_item", is_token=False)), Literal(value="begin"), Repetition(element=RuleReference(name="concurrent_statement", is_token=False)), Literal(value="end"), OptGroup(element=Literal(value="architecture")), OptGroup(element=RuleReference(name="NAME", is_token=True)), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="block_declarative_item",
            line_number=160,
            body=Alternation(choices=[RuleReference(name="signal_declaration", is_token=False), RuleReference(name="constant_declaration", is_token=False), RuleReference(name="type_declaration", is_token=False), RuleReference(name="subtype_declaration", is_token=False), RuleReference(name="component_declaration", is_token=False), RuleReference(name="function_declaration", is_token=False), RuleReference(name="function_body", is_token=False), RuleReference(name="procedure_declaration", is_token=False), RuleReference(name="procedure_body", is_token=False)]),
        ),
        GrammarRule(
            name="signal_declaration",
            line_number=189,
            body=Sequence(elements=[Literal(value="signal"), RuleReference(name="name_list", is_token=False), RuleReference(name="COLON", is_token=True), RuleReference(name="subtype_indication", is_token=False), OptGroup(element=Sequence(elements=[RuleReference(name="VAR_ASSIGN", is_token=True), RuleReference(name="expression", is_token=False)])), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="constant_declaration",
            line_number=191,
            body=Sequence(elements=[Literal(value="constant"), RuleReference(name="name_list", is_token=False), RuleReference(name="COLON", is_token=True), RuleReference(name="subtype_indication", is_token=False), RuleReference(name="VAR_ASSIGN", is_token=True), RuleReference(name="expression", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="variable_declaration",
            line_number=193,
            body=Sequence(elements=[Literal(value="variable"), RuleReference(name="name_list", is_token=False), RuleReference(name="COLON", is_token=True), RuleReference(name="subtype_indication", is_token=False), OptGroup(element=Sequence(elements=[RuleReference(name="VAR_ASSIGN", is_token=True), RuleReference(name="expression", is_token=False)])), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="type_declaration",
            line_number=218,
            body=Sequence(elements=[Literal(value="type"), RuleReference(name="NAME", is_token=True), Literal(value="is"), RuleReference(name="type_definition", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="subtype_declaration",
            line_number=219,
            body=Sequence(elements=[Literal(value="subtype"), RuleReference(name="NAME", is_token=True), Literal(value="is"), RuleReference(name="subtype_indication", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="type_definition",
            line_number=221,
            body=Alternation(choices=[RuleReference(name="enumeration_type", is_token=False), RuleReference(name="array_type", is_token=False), RuleReference(name="record_type", is_token=False)]),
        ),
        GrammarRule(
            name="enumeration_type",
            line_number=227,
            body=Sequence(elements=[RuleReference(name="LPAREN", is_token=True), Group(element=Alternation(choices=[RuleReference(name="NAME", is_token=True), RuleReference(name="CHAR_LITERAL", is_token=True)])), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), Group(element=Alternation(choices=[RuleReference(name="NAME", is_token=True), RuleReference(name="CHAR_LITERAL", is_token=True)]))])), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="array_type",
            line_number=232,
            body=Sequence(elements=[Literal(value="array"), RuleReference(name="LPAREN", is_token=True), RuleReference(name="index_constraint", is_token=False), RuleReference(name="RPAREN", is_token=True), Literal(value="of"), RuleReference(name="subtype_indication", is_token=False)]),
        ),
        GrammarRule(
            name="index_constraint",
            line_number=234,
            body=Sequence(elements=[RuleReference(name="discrete_range", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="discrete_range", is_token=False)]))]),
        ),
        GrammarRule(
            name="discrete_range",
            line_number=235,
            body=Alternation(choices=[RuleReference(name="subtype_indication", is_token=False), Sequence(elements=[RuleReference(name="expression", is_token=False), Group(element=Alternation(choices=[Literal(value="to"), Literal(value="downto")])), RuleReference(name="expression", is_token=False)])]),
        ),
        GrammarRule(
            name="record_type",
            line_number=239,
            body=Sequence(elements=[Literal(value="record"), Repetition(element=Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="COLON", is_token=True), RuleReference(name="subtype_indication", is_token=False), RuleReference(name="SEMICOLON", is_token=True)])), Literal(value="end"), Literal(value="record"), OptGroup(element=RuleReference(name="NAME", is_token=True))]),
        ),
        GrammarRule(
            name="subtype_indication",
            line_number=247,
            body=Sequence(elements=[RuleReference(name="selected_name", is_token=False), OptGroup(element=RuleReference(name="constraint", is_token=False))]),
        ),
        GrammarRule(
            name="constraint",
            line_number=249,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="expression", is_token=False), Group(element=Alternation(choices=[Literal(value="to"), Literal(value="downto")])), RuleReference(name="expression", is_token=False), RuleReference(name="RPAREN", is_token=True)]), Sequence(elements=[Literal(value="range"), RuleReference(name="expression", is_token=False), Group(element=Alternation(choices=[Literal(value="to"), Literal(value="downto")])), RuleReference(name="expression", is_token=False)])]),
        ),
        GrammarRule(
            name="concurrent_statement",
            line_number=264,
            body=Alternation(choices=[RuleReference(name="process_statement", is_token=False), RuleReference(name="signal_assignment_concurrent", is_token=False), RuleReference(name="component_instantiation", is_token=False), RuleReference(name="generate_statement", is_token=False)]),
        ),
        GrammarRule(
            name="signal_assignment_concurrent",
            line_number=272,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="LESS_EQUALS", is_token=True), RuleReference(name="waveform", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="waveform",
            line_number=274,
            body=Sequence(elements=[RuleReference(name="waveform_element", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="waveform_element", is_token=False)]))]),
        ),
        GrammarRule(
            name="waveform_element",
            line_number=275,
            body=RuleReference(name="expression", is_token=False),
        ),
        GrammarRule(
            name="process_statement",
            line_number=307,
            body=Sequence(elements=[OptGroup(element=Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="COLON", is_token=True)])), Literal(value="process"), OptGroup(element=Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="sensitivity_list", is_token=False), RuleReference(name="RPAREN", is_token=True)])), OptGroup(element=Literal(value="is")), Repetition(element=RuleReference(name="process_declarative_item", is_token=False)), Literal(value="begin"), Repetition(element=RuleReference(name="sequential_statement", is_token=False)), Literal(value="end"), Literal(value="process"), OptGroup(element=RuleReference(name="NAME", is_token=True)), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="sensitivity_list",
            line_number=315,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="NAME", is_token=True)]))]),
        ),
        GrammarRule(
            name="process_declarative_item",
            line_number=317,
            body=Alternation(choices=[RuleReference(name="variable_declaration", is_token=False), RuleReference(name="constant_declaration", is_token=False), RuleReference(name="type_declaration", is_token=False), RuleReference(name="subtype_declaration", is_token=False)]),
        ),
        GrammarRule(
            name="sequential_statement",
            line_number=329,
            body=Alternation(choices=[RuleReference(name="signal_assignment_seq", is_token=False), RuleReference(name="variable_assignment", is_token=False), RuleReference(name="if_statement", is_token=False), RuleReference(name="case_statement", is_token=False), RuleReference(name="loop_statement", is_token=False), RuleReference(name="return_statement", is_token=False), RuleReference(name="null_statement", is_token=False)]),
        ),
        GrammarRule(
            name="signal_assignment_seq",
            line_number=342,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="LESS_EQUALS", is_token=True), RuleReference(name="waveform", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="variable_assignment",
            line_number=346,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="VAR_ASSIGN", is_token=True), RuleReference(name="expression", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="if_statement",
            line_number=356,
            body=Sequence(elements=[Literal(value="if"), RuleReference(name="expression", is_token=False), Literal(value="then"), Repetition(element=RuleReference(name="sequential_statement", is_token=False)), Repetition(element=Sequence(elements=[Literal(value="elsif"), RuleReference(name="expression", is_token=False), Literal(value="then"), Repetition(element=RuleReference(name="sequential_statement", is_token=False))])), OptGroup(element=Sequence(elements=[Literal(value="else"), Repetition(element=RuleReference(name="sequential_statement", is_token=False))])), Literal(value="end"), Literal(value="if"), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="case_statement",
            line_number=372,
            body=Sequence(elements=[Literal(value="case"), RuleReference(name="expression", is_token=False), Literal(value="is"), Repetition(element=Sequence(elements=[Literal(value="when"), RuleReference(name="choices", is_token=False), RuleReference(name="ARROW", is_token=True), Repetition(element=RuleReference(name="sequential_statement", is_token=False))])), Literal(value="end"), Literal(value="case"), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="choices",
            line_number=376,
            body=Sequence(elements=[RuleReference(name="choice", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="PIPE", is_token=True), RuleReference(name="choice", is_token=False)]))]),
        ),
        GrammarRule(
            name="choice",
            line_number=377,
            body=Alternation(choices=[RuleReference(name="expression", is_token=False), RuleReference(name="discrete_range", is_token=False), Literal(value="others")]),
        ),
        GrammarRule(
            name="loop_statement",
            line_number=391,
            body=Sequence(elements=[OptGroup(element=Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="COLON", is_token=True)])), OptGroup(element=Alternation(choices=[Sequence(elements=[Literal(value="for"), RuleReference(name="NAME", is_token=True), Literal(value="in"), RuleReference(name="discrete_range", is_token=False)]), Sequence(elements=[Literal(value="while"), RuleReference(name="expression", is_token=False)])])), Literal(value="loop"), Repetition(element=RuleReference(name="sequential_statement", is_token=False)), Literal(value="end"), Literal(value="loop"), OptGroup(element=RuleReference(name="NAME", is_token=True)), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="return_statement",
            line_number=398,
            body=Sequence(elements=[Literal(value="return"), OptGroup(element=RuleReference(name="expression", is_token=False)), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="null_statement",
            line_number=399,
            body=Sequence(elements=[Literal(value="null"), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="component_declaration",
            line_number=425,
            body=Sequence(elements=[Literal(value="component"), RuleReference(name="NAME", is_token=True), OptGroup(element=Literal(value="is")), OptGroup(element=RuleReference(name="generic_clause", is_token=False)), OptGroup(element=RuleReference(name="port_clause", is_token=False)), Literal(value="end"), Literal(value="component"), OptGroup(element=RuleReference(name="NAME", is_token=True)), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="component_instantiation",
            line_number=430,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="COLON", is_token=True), Group(element=Alternation(choices=[RuleReference(name="NAME", is_token=True), Sequence(elements=[Literal(value="entity"), RuleReference(name="selected_name", is_token=False), OptGroup(element=Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="NAME", is_token=True), RuleReference(name="RPAREN", is_token=True)]))])])), OptGroup(element=Sequence(elements=[Literal(value="generic"), Literal(value="map"), RuleReference(name="LPAREN", is_token=True), RuleReference(name="association_list", is_token=False), RuleReference(name="RPAREN", is_token=True)])), OptGroup(element=Sequence(elements=[Literal(value="port"), Literal(value="map"), RuleReference(name="LPAREN", is_token=True), RuleReference(name="association_list", is_token=False), RuleReference(name="RPAREN", is_token=True)])), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="association_list",
            line_number=437,
            body=Sequence(elements=[RuleReference(name="association_element", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="association_element", is_token=False)]))]),
        ),
        GrammarRule(
            name="association_element",
            line_number=438,
            body=Alternation(choices=[Sequence(elements=[OptGroup(element=Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="ARROW", is_token=True)])), RuleReference(name="expression", is_token=False)]), Sequence(elements=[OptGroup(element=Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="ARROW", is_token=True)])), Literal(value="open")])]),
        ),
        GrammarRule(
            name="generate_statement",
            line_number=461,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="COLON", is_token=True), Group(element=Alternation(choices=[RuleReference(name="for_generate", is_token=False), RuleReference(name="if_generate", is_token=False)]))]),
        ),
        GrammarRule(
            name="for_generate",
            line_number=463,
            body=Sequence(elements=[Literal(value="for"), RuleReference(name="NAME", is_token=True), Literal(value="in"), RuleReference(name="discrete_range", is_token=False), Literal(value="generate"), Repetition(element=RuleReference(name="concurrent_statement", is_token=False)), Literal(value="end"), Literal(value="generate"), OptGroup(element=RuleReference(name="NAME", is_token=True)), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="if_generate",
            line_number=467,
            body=Sequence(elements=[Literal(value="if"), RuleReference(name="expression", is_token=False), Literal(value="generate"), Repetition(element=RuleReference(name="concurrent_statement", is_token=False)), Literal(value="end"), Literal(value="generate"), OptGroup(element=RuleReference(name="NAME", is_token=True)), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="package_declaration",
            line_number=488,
            body=Sequence(elements=[Literal(value="package"), RuleReference(name="NAME", is_token=True), Literal(value="is"), Repetition(element=RuleReference(name="package_declarative_item", is_token=False)), Literal(value="end"), OptGroup(element=Literal(value="package")), OptGroup(element=RuleReference(name="NAME", is_token=True)), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="package_body",
            line_number=492,
            body=Sequence(elements=[Literal(value="package"), Literal(value="body"), RuleReference(name="NAME", is_token=True), Literal(value="is"), Repetition(element=RuleReference(name="package_body_declarative_item", is_token=False)), Literal(value="end"), OptGroup(element=Sequence(elements=[Literal(value="package"), Literal(value="body")])), OptGroup(element=RuleReference(name="NAME", is_token=True)), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="package_declarative_item",
            line_number=496,
            body=Alternation(choices=[RuleReference(name="type_declaration", is_token=False), RuleReference(name="subtype_declaration", is_token=False), RuleReference(name="constant_declaration", is_token=False), RuleReference(name="signal_declaration", is_token=False), RuleReference(name="component_declaration", is_token=False), RuleReference(name="function_declaration", is_token=False), RuleReference(name="procedure_declaration", is_token=False)]),
        ),
        GrammarRule(
            name="package_body_declarative_item",
            line_number=504,
            body=Alternation(choices=[RuleReference(name="type_declaration", is_token=False), RuleReference(name="subtype_declaration", is_token=False), RuleReference(name="constant_declaration", is_token=False), RuleReference(name="function_body", is_token=False), RuleReference(name="procedure_body", is_token=False)]),
        ),
        GrammarRule(
            name="function_declaration",
            line_number=520,
            body=Sequence(elements=[OptGroup(element=Alternation(choices=[Literal(value="pure"), Literal(value="impure")])), Literal(value="function"), RuleReference(name="NAME", is_token=True), OptGroup(element=Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="interface_list", is_token=False), RuleReference(name="RPAREN", is_token=True)])), Literal(value="return"), RuleReference(name="subtype_indication", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="function_body",
            line_number=525,
            body=Sequence(elements=[OptGroup(element=Alternation(choices=[Literal(value="pure"), Literal(value="impure")])), Literal(value="function"), RuleReference(name="NAME", is_token=True), OptGroup(element=Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="interface_list", is_token=False), RuleReference(name="RPAREN", is_token=True)])), Literal(value="return"), RuleReference(name="subtype_indication", is_token=False), Literal(value="is"), Repetition(element=RuleReference(name="process_declarative_item", is_token=False)), Literal(value="begin"), Repetition(element=RuleReference(name="sequential_statement", is_token=False)), Literal(value="end"), OptGroup(element=Literal(value="function")), OptGroup(element=RuleReference(name="NAME", is_token=True)), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="procedure_declaration",
            line_number=534,
            body=Sequence(elements=[Literal(value="procedure"), RuleReference(name="NAME", is_token=True), OptGroup(element=Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="interface_list", is_token=False), RuleReference(name="RPAREN", is_token=True)])), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="procedure_body",
            line_number=537,
            body=Sequence(elements=[Literal(value="procedure"), RuleReference(name="NAME", is_token=True), OptGroup(element=Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="interface_list", is_token=False), RuleReference(name="RPAREN", is_token=True)])), Literal(value="is"), Repetition(element=RuleReference(name="process_declarative_item", is_token=False)), Literal(value="begin"), Repetition(element=RuleReference(name="sequential_statement", is_token=False)), Literal(value="end"), OptGroup(element=Literal(value="procedure")), OptGroup(element=RuleReference(name="NAME", is_token=True)), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="expression",
            line_number=574,
            body=RuleReference(name="logical_expr", is_token=False),
        ),
        GrammarRule(
            name="logical_expr",
            line_number=581,
            body=Sequence(elements=[RuleReference(name="relation", is_token=False), OptGroup(element=Sequence(elements=[RuleReference(name="logical_op", is_token=False), RuleReference(name="relation", is_token=False)]))]),
        ),
        GrammarRule(
            name="logical_op",
            line_number=582,
            body=Alternation(choices=[Literal(value="and"), Literal(value="or"), Literal(value="xor"), Literal(value="nand"), Literal(value="nor"), Literal(value="xnor")]),
        ),
        GrammarRule(
            name="relation",
            line_number=586,
            body=Sequence(elements=[RuleReference(name="shift_expr", is_token=False), OptGroup(element=Sequence(elements=[RuleReference(name="relational_op", is_token=False), RuleReference(name="shift_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="relational_op",
            line_number=587,
            body=Alternation(choices=[RuleReference(name="EQUALS", is_token=True), RuleReference(name="NOT_EQUALS", is_token=True), RuleReference(name="LESS_THAN", is_token=True), RuleReference(name="LESS_EQUALS", is_token=True), RuleReference(name="GREATER_THAN", is_token=True), RuleReference(name="GREATER_EQUALS", is_token=True)]),
        ),
        GrammarRule(
            name="shift_expr",
            line_number=592,
            body=Sequence(elements=[RuleReference(name="adding_expr", is_token=False), OptGroup(element=Sequence(elements=[RuleReference(name="shift_op", is_token=False), RuleReference(name="adding_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="shift_op",
            line_number=593,
            body=Alternation(choices=[Literal(value="sll"), Literal(value="srl"), Literal(value="sla"), Literal(value="sra"), Literal(value="rol"), Literal(value="ror")]),
        ),
        GrammarRule(
            name="adding_expr",
            line_number=597,
            body=Sequence(elements=[RuleReference(name="multiplying_expr", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="adding_op", is_token=False), RuleReference(name="multiplying_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="adding_op",
            line_number=598,
            body=Alternation(choices=[RuleReference(name="PLUS", is_token=True), RuleReference(name="MINUS", is_token=True), RuleReference(name="AMPERSAND", is_token=True)]),
        ),
        GrammarRule(
            name="multiplying_expr",
            line_number=601,
            body=Sequence(elements=[RuleReference(name="unary_expr", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="multiplying_op", is_token=False), RuleReference(name="unary_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="multiplying_op",
            line_number=602,
            body=Alternation(choices=[RuleReference(name="STAR", is_token=True), RuleReference(name="SLASH", is_token=True), Literal(value="mod"), Literal(value="rem")]),
        ),
        GrammarRule(
            name="unary_expr",
            line_number=605,
            body=Alternation(choices=[Sequence(elements=[Literal(value="abs"), RuleReference(name="unary_expr", is_token=False)]), Sequence(elements=[Literal(value="not"), RuleReference(name="unary_expr", is_token=False)]), Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="PLUS", is_token=True), RuleReference(name="MINUS", is_token=True)])), RuleReference(name="unary_expr", is_token=False)]), RuleReference(name="power_expr", is_token=False)]),
        ),
        GrammarRule(
            name="power_expr",
            line_number=611,
            body=Sequence(elements=[RuleReference(name="primary", is_token=False), OptGroup(element=Sequence(elements=[RuleReference(name="POWER", is_token=True), RuleReference(name="primary", is_token=False)]))]),
        ),
        GrammarRule(
            name="primary",
            line_number=619,
            body=Alternation(choices=[RuleReference(name="NUMBER", is_token=True), RuleReference(name="REAL_NUMBER", is_token=True), RuleReference(name="BASED_LITERAL", is_token=True), RuleReference(name="STRING", is_token=True), RuleReference(name="CHAR_LITERAL", is_token=True), RuleReference(name="BIT_STRING", is_token=True), Sequence(elements=[RuleReference(name="NAME", is_token=True), OptGroup(element=Sequence(elements=[RuleReference(name="TICK", is_token=True), RuleReference(name="NAME", is_token=True)]))]), Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="LPAREN", is_token=True), OptGroup(element=Sequence(elements=[RuleReference(name="expression", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="expression", is_token=False)]))])), RuleReference(name="RPAREN", is_token=True)]), Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="expression", is_token=False), RuleReference(name="RPAREN", is_token=True)]), RuleReference(name="aggregate", is_token=False), Literal(value="null")]),
        ),
        GrammarRule(
            name="aggregate",
            line_number=635,
            body=Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="element_association", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="element_association", is_token=False)])), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="element_association",
            line_number=636,
            body=Sequence(elements=[OptGroup(element=Sequence(elements=[RuleReference(name="choices", is_token=False), RuleReference(name="ARROW", is_token=True)])), RuleReference(name="expression", is_token=False)]),
        ),
    ],
)
