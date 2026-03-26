# AUTO-GENERATED FILE - DO NOT EDIT
from grammar_tools.parser_grammar import (
    ParserGrammar, GrammarRule, GrammarElement,
    RuleReference, Literal, Sequence, Alternation,
    Repetition, Optional as OptGroup, Group
)

ExcelGrammar = ParserGrammar(
    version=1,
    rules=[
        GrammarRule(
            name="formula",
            line_number=15,
            body=Sequence(elements=[RuleReference(name="ws", is_token=False), OptGroup(element=Sequence(elements=[RuleReference(name="EQUALS", is_token=True), RuleReference(name="ws", is_token=False)])), RuleReference(name="expression", is_token=False), RuleReference(name="ws", is_token=False)]),
        ),
        GrammarRule(
            name="ws",
            line_number=17,
            body=Repetition(element=RuleReference(name="SPACE", is_token=True)),
        ),
        GrammarRule(
            name="req_space",
            line_number=18,
            body=Sequence(elements=[RuleReference(name="SPACE", is_token=True), Repetition(element=RuleReference(name="SPACE", is_token=True))]),
        ),
        GrammarRule(
            name="expression",
            line_number=20,
            body=RuleReference(name="comparison_expr", is_token=False),
        ),
        GrammarRule(
            name="comparison_expr",
            line_number=22,
            body=Sequence(elements=[RuleReference(name="concat_expr", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="ws", is_token=False), RuleReference(name="comparison_op", is_token=False), RuleReference(name="ws", is_token=False), RuleReference(name="concat_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="comparison_op",
            line_number=23,
            body=Alternation(choices=[RuleReference(name="EQUALS", is_token=True), RuleReference(name="NOT_EQUALS", is_token=True), RuleReference(name="LESS_THAN", is_token=True), RuleReference(name="LESS_EQUALS", is_token=True), RuleReference(name="GREATER_THAN", is_token=True), RuleReference(name="GREATER_EQUALS", is_token=True)]),
        ),
        GrammarRule(
            name="concat_expr",
            line_number=26,
            body=Sequence(elements=[RuleReference(name="additive_expr", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="ws", is_token=False), RuleReference(name="AMP", is_token=True), RuleReference(name="ws", is_token=False), RuleReference(name="additive_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="additive_expr",
            line_number=27,
            body=Sequence(elements=[RuleReference(name="multiplicative_expr", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="ws", is_token=False), Group(element=Alternation(choices=[RuleReference(name="PLUS", is_token=True), RuleReference(name="MINUS", is_token=True)])), RuleReference(name="ws", is_token=False), RuleReference(name="multiplicative_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="multiplicative_expr",
            line_number=28,
            body=Sequence(elements=[RuleReference(name="power_expr", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="ws", is_token=False), Group(element=Alternation(choices=[RuleReference(name="STAR", is_token=True), RuleReference(name="SLASH", is_token=True)])), RuleReference(name="ws", is_token=False), RuleReference(name="power_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="power_expr",
            line_number=29,
            body=Sequence(elements=[RuleReference(name="unary_expr", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="ws", is_token=False), RuleReference(name="CARET", is_token=True), RuleReference(name="ws", is_token=False), RuleReference(name="unary_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="unary_expr",
            line_number=30,
            body=Sequence(elements=[Repetition(element=Sequence(elements=[RuleReference(name="prefix_op", is_token=False), RuleReference(name="ws", is_token=False)])), RuleReference(name="postfix_expr", is_token=False)]),
        ),
        GrammarRule(
            name="prefix_op",
            line_number=31,
            body=Alternation(choices=[RuleReference(name="PLUS", is_token=True), RuleReference(name="MINUS", is_token=True)]),
        ),
        GrammarRule(
            name="postfix_expr",
            line_number=32,
            body=Sequence(elements=[RuleReference(name="primary", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="ws", is_token=False), RuleReference(name="PERCENT", is_token=True)]))]),
        ),
        GrammarRule(
            name="primary",
            line_number=34,
            body=Alternation(choices=[RuleReference(name="parenthesized_expression", is_token=False), RuleReference(name="constant", is_token=False), RuleReference(name="function_call", is_token=False), RuleReference(name="structure_reference", is_token=False), RuleReference(name="reference_expression", is_token=False), RuleReference(name="bang_reference", is_token=False), RuleReference(name="bang_name", is_token=False), RuleReference(name="name_reference", is_token=False)]),
        ),
        GrammarRule(
            name="parenthesized_expression",
            line_number=43,
            body=Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="ws", is_token=False), RuleReference(name="expression", is_token=False), RuleReference(name="ws", is_token=False), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="constant",
            line_number=45,
            body=Alternation(choices=[RuleReference(name="NUMBER", is_token=True), RuleReference(name="STRING", is_token=True), RuleReference(name="KEYWORD", is_token=True), RuleReference(name="ERROR_CONSTANT", is_token=True), RuleReference(name="array_constant", is_token=False)]),
        ),
        GrammarRule(
            name="array_constant",
            line_number=47,
            body=Sequence(elements=[RuleReference(name="LBRACE", is_token=True), RuleReference(name="ws", is_token=False), RuleReference(name="array_row", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="ws", is_token=False), RuleReference(name="SEMICOLON", is_token=True), RuleReference(name="ws", is_token=False), RuleReference(name="array_row", is_token=False)])), OptGroup(element=Sequence(elements=[RuleReference(name="ws", is_token=False), RuleReference(name="SEMICOLON", is_token=True)])), RuleReference(name="ws", is_token=False), RuleReference(name="RBRACE", is_token=True)]),
        ),
        GrammarRule(
            name="array_row",
            line_number=48,
            body=Sequence(elements=[RuleReference(name="array_item", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="ws", is_token=False), RuleReference(name="COMMA", is_token=True), RuleReference(name="ws", is_token=False), RuleReference(name="array_item", is_token=False)])), OptGroup(element=Sequence(elements=[RuleReference(name="ws", is_token=False), RuleReference(name="COMMA", is_token=True)]))]),
        ),
        GrammarRule(
            name="array_item",
            line_number=49,
            body=Alternation(choices=[RuleReference(name="NUMBER", is_token=True), RuleReference(name="STRING", is_token=True), RuleReference(name="KEYWORD", is_token=True), RuleReference(name="ERROR_CONSTANT", is_token=True)]),
        ),
        GrammarRule(
            name="function_call",
            line_number=51,
            body=Sequence(elements=[RuleReference(name="function_name", is_token=False), RuleReference(name="LPAREN", is_token=True), RuleReference(name="ws", is_token=False), OptGroup(element=RuleReference(name="function_argument_list", is_token=False)), RuleReference(name="ws", is_token=False), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="function_name",
            line_number=52,
            body=Alternation(choices=[RuleReference(name="FUNCTION_NAME", is_token=True), RuleReference(name="NAME", is_token=True)]),
        ),
        GrammarRule(
            name="function_argument_list",
            line_number=53,
            body=Sequence(elements=[RuleReference(name="function_argument", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="ws", is_token=False), RuleReference(name="COMMA", is_token=True), RuleReference(name="ws", is_token=False), RuleReference(name="function_argument", is_token=False)])), OptGroup(element=Sequence(elements=[RuleReference(name="ws", is_token=False), RuleReference(name="COMMA", is_token=True)]))]),
        ),
        GrammarRule(
            name="function_argument",
            line_number=54,
            body=OptGroup(element=RuleReference(name="expression", is_token=False)),
        ),
        GrammarRule(
            name="reference_expression",
            line_number=56,
            body=RuleReference(name="union_reference", is_token=False),
        ),
        GrammarRule(
            name="union_reference",
            line_number=57,
            body=Sequence(elements=[RuleReference(name="intersection_reference", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="ws", is_token=False), RuleReference(name="COMMA", is_token=True), RuleReference(name="ws", is_token=False), RuleReference(name="intersection_reference", is_token=False)]))]),
        ),
        GrammarRule(
            name="intersection_reference",
            line_number=58,
            body=Sequence(elements=[RuleReference(name="range_reference", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="req_space", is_token=False), RuleReference(name="range_reference", is_token=False)]))]),
        ),
        GrammarRule(
            name="range_reference",
            line_number=59,
            body=Sequence(elements=[RuleReference(name="reference_primary", is_token=False), OptGroup(element=Sequence(elements=[RuleReference(name="ws", is_token=False), RuleReference(name="COLON", is_token=True), RuleReference(name="ws", is_token=False), RuleReference(name="reference_primary", is_token=False)]))]),
        ),
        GrammarRule(
            name="reference_primary",
            line_number=61,
            body=Alternation(choices=[RuleReference(name="parenthesized_reference", is_token=False), RuleReference(name="prefixed_reference", is_token=False), RuleReference(name="external_reference", is_token=False), RuleReference(name="structure_reference", is_token=False), RuleReference(name="a1_reference", is_token=False), RuleReference(name="bang_reference", is_token=False), RuleReference(name="bang_name", is_token=False), RuleReference(name="name_reference", is_token=False)]),
        ),
        GrammarRule(
            name="parenthesized_reference",
            line_number=70,
            body=Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="ws", is_token=False), RuleReference(name="reference_expression", is_token=False), RuleReference(name="ws", is_token=False), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="prefixed_reference",
            line_number=71,
            body=Sequence(elements=[RuleReference(name="REF_PREFIX", is_token=True), Group(element=Alternation(choices=[RuleReference(name="a1_reference", is_token=False), RuleReference(name="name_reference", is_token=False), RuleReference(name="structure_reference", is_token=False)]))]),
        ),
        GrammarRule(
            name="external_reference",
            line_number=72,
            body=RuleReference(name="REF_PREFIX", is_token=True),
        ),
        GrammarRule(
            name="bang_reference",
            line_number=73,
            body=Sequence(elements=[RuleReference(name="BANG", is_token=True), Group(element=Alternation(choices=[RuleReference(name="CELL", is_token=True), RuleReference(name="COLUMN_REF", is_token=True), RuleReference(name="ROW_REF", is_token=True), RuleReference(name="NUMBER", is_token=True)]))]),
        ),
        GrammarRule(
            name="bang_name",
            line_number=74,
            body=Sequence(elements=[RuleReference(name="BANG", is_token=True), RuleReference(name="name_reference", is_token=False)]),
        ),
        GrammarRule(
            name="name_reference",
            line_number=75,
            body=RuleReference(name="NAME", is_token=True),
        ),
        GrammarRule(
            name="column_reference",
            line_number=77,
            body=Sequence(elements=[OptGroup(element=RuleReference(name="DOLLAR", is_token=True)), Group(element=Alternation(choices=[RuleReference(name="COLUMN_REF", is_token=True), RuleReference(name="NAME", is_token=True)]))]),
        ),
        GrammarRule(
            name="row_reference",
            line_number=78,
            body=Sequence(elements=[OptGroup(element=RuleReference(name="DOLLAR", is_token=True)), Group(element=Alternation(choices=[RuleReference(name="ROW_REF", is_token=True), RuleReference(name="NUMBER", is_token=True)]))]),
        ),
        GrammarRule(
            name="a1_reference",
            line_number=80,
            body=Alternation(choices=[RuleReference(name="CELL", is_token=True), RuleReference(name="column_reference", is_token=False), RuleReference(name="row_reference", is_token=False), RuleReference(name="COLUMN_REF", is_token=True), RuleReference(name="ROW_REF", is_token=True), RuleReference(name="NAME", is_token=True), RuleReference(name="NUMBER", is_token=True)]),
        ),
        GrammarRule(
            name="structure_reference",
            line_number=82,
            body=Sequence(elements=[OptGroup(element=RuleReference(name="table_name", is_token=False)), RuleReference(name="intra_table_reference", is_token=False)]),
        ),
        GrammarRule(
            name="table_name",
            line_number=83,
            body=Alternation(choices=[RuleReference(name="TABLE_NAME", is_token=True), RuleReference(name="NAME", is_token=True)]),
        ),
        GrammarRule(
            name="intra_table_reference",
            line_number=84,
            body=Alternation(choices=[RuleReference(name="STRUCTURED_KEYWORD", is_token=True), RuleReference(name="structured_column_range", is_token=False), Sequence(elements=[RuleReference(name="LBRACKET", is_token=True), RuleReference(name="ws", is_token=False), OptGroup(element=RuleReference(name="inner_structure_reference", is_token=False)), RuleReference(name="ws", is_token=False), RuleReference(name="RBRACKET", is_token=True)])]),
        ),
        GrammarRule(
            name="inner_structure_reference",
            line_number=87,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="structured_keyword_list", is_token=False), OptGroup(element=Sequence(elements=[RuleReference(name="ws", is_token=False), RuleReference(name="COMMA", is_token=True), RuleReference(name="ws", is_token=False), RuleReference(name="structured_column_range", is_token=False)]))]), RuleReference(name="structured_column_range", is_token=False)]),
        ),
        GrammarRule(
            name="structured_keyword_list",
            line_number=89,
            body=Sequence(elements=[RuleReference(name="STRUCTURED_KEYWORD", is_token=True), Repetition(element=Sequence(elements=[RuleReference(name="ws", is_token=False), RuleReference(name="COMMA", is_token=True), RuleReference(name="ws", is_token=False), RuleReference(name="STRUCTURED_KEYWORD", is_token=True)]))]),
        ),
        GrammarRule(
            name="structured_column_range",
            line_number=90,
            body=Sequence(elements=[RuleReference(name="structured_column", is_token=False), OptGroup(element=Sequence(elements=[RuleReference(name="ws", is_token=False), RuleReference(name="COLON", is_token=True), RuleReference(name="ws", is_token=False), RuleReference(name="structured_column", is_token=False)]))]),
        ),
        GrammarRule(
            name="structured_column",
            line_number=91,
            body=Alternation(choices=[RuleReference(name="STRUCTURED_COLUMN", is_token=True), Sequence(elements=[RuleReference(name="AT", is_token=True), RuleReference(name="STRUCTURED_COLUMN", is_token=True)])]),
        ),
    ],
)
