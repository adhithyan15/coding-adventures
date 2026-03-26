# AUTO-GENERATED FILE - DO NOT EDIT
from grammar_tools.parser_grammar import (
    ParserGrammar, GrammarRule, GrammarElement,
    RuleReference, Literal, Sequence, Alternation,
    Repetition, Optional as OptGroup, Group
)

SqlGrammar = ParserGrammar(
    version=1,
    rules=[
        GrammarRule(
            name="program",
            line_number=10,
            body=Sequence(elements=[RuleReference(name="statement", is_token=False), Repetition(element=Sequence(elements=[Literal(value=";"), RuleReference(name="statement", is_token=False)])), OptGroup(element=Literal(value=";"))]),
        ),
        GrammarRule(
            name="statement",
            line_number=12,
            body=Alternation(choices=[RuleReference(name="select_stmt", is_token=False), RuleReference(name="insert_stmt", is_token=False), RuleReference(name="update_stmt", is_token=False), RuleReference(name="delete_stmt", is_token=False), RuleReference(name="create_table_stmt", is_token=False), RuleReference(name="drop_table_stmt", is_token=False)]),
        ),
        GrammarRule(
            name="select_stmt",
            line_number=17,
            body=Sequence(elements=[Literal(value="SELECT"), OptGroup(element=Alternation(choices=[Literal(value="DISTINCT"), Literal(value="ALL")])), RuleReference(name="select_list", is_token=False), Literal(value="FROM"), RuleReference(name="table_ref", is_token=False), Repetition(element=RuleReference(name="join_clause", is_token=False)), OptGroup(element=RuleReference(name="where_clause", is_token=False)), OptGroup(element=RuleReference(name="group_clause", is_token=False)), OptGroup(element=RuleReference(name="having_clause", is_token=False)), OptGroup(element=RuleReference(name="order_clause", is_token=False)), OptGroup(element=RuleReference(name="limit_clause", is_token=False))]),
        ),
        GrammarRule(
            name="select_list",
            line_number=22,
            body=Alternation(choices=[RuleReference(name="STAR", is_token=True), Sequence(elements=[RuleReference(name="select_item", is_token=False), Repetition(element=Sequence(elements=[Literal(value=","), RuleReference(name="select_item", is_token=False)]))])]),
        ),
        GrammarRule(
            name="select_item",
            line_number=23,
            body=Sequence(elements=[RuleReference(name="expr", is_token=False), OptGroup(element=Sequence(elements=[Literal(value="AS"), RuleReference(name="NAME", is_token=True)]))]),
        ),
        GrammarRule(
            name="table_ref",
            line_number=25,
            body=Sequence(elements=[RuleReference(name="table_name", is_token=False), OptGroup(element=Sequence(elements=[Literal(value="AS"), RuleReference(name="NAME", is_token=True)]))]),
        ),
        GrammarRule(
            name="table_name",
            line_number=26,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), OptGroup(element=Sequence(elements=[Literal(value="."), RuleReference(name="NAME", is_token=True)]))]),
        ),
        GrammarRule(
            name="join_clause",
            line_number=28,
            body=Sequence(elements=[RuleReference(name="join_type", is_token=False), Literal(value="JOIN"), RuleReference(name="table_ref", is_token=False), Literal(value="ON"), RuleReference(name="expr", is_token=False)]),
        ),
        GrammarRule(
            name="join_type",
            line_number=29,
            body=Alternation(choices=[Literal(value="CROSS"), Literal(value="INNER"), Group(element=Sequence(elements=[Literal(value="LEFT"), OptGroup(element=Literal(value="OUTER"))])), Group(element=Sequence(elements=[Literal(value="RIGHT"), OptGroup(element=Literal(value="OUTER"))])), Group(element=Sequence(elements=[Literal(value="FULL"), OptGroup(element=Literal(value="OUTER"))]))]),
        ),
        GrammarRule(
            name="where_clause",
            line_number=32,
            body=Sequence(elements=[Literal(value="WHERE"), RuleReference(name="expr", is_token=False)]),
        ),
        GrammarRule(
            name="group_clause",
            line_number=33,
            body=Sequence(elements=[Literal(value="GROUP"), Literal(value="BY"), RuleReference(name="column_ref", is_token=False), Repetition(element=Sequence(elements=[Literal(value=","), RuleReference(name="column_ref", is_token=False)]))]),
        ),
        GrammarRule(
            name="having_clause",
            line_number=34,
            body=Sequence(elements=[Literal(value="HAVING"), RuleReference(name="expr", is_token=False)]),
        ),
        GrammarRule(
            name="order_clause",
            line_number=35,
            body=Sequence(elements=[Literal(value="ORDER"), Literal(value="BY"), RuleReference(name="order_item", is_token=False), Repetition(element=Sequence(elements=[Literal(value=","), RuleReference(name="order_item", is_token=False)]))]),
        ),
        GrammarRule(
            name="order_item",
            line_number=36,
            body=Sequence(elements=[RuleReference(name="expr", is_token=False), OptGroup(element=Alternation(choices=[Literal(value="ASC"), Literal(value="DESC")]))]),
        ),
        GrammarRule(
            name="limit_clause",
            line_number=37,
            body=Sequence(elements=[Literal(value="LIMIT"), RuleReference(name="NUMBER", is_token=True), OptGroup(element=Sequence(elements=[Literal(value="OFFSET"), RuleReference(name="NUMBER", is_token=True)]))]),
        ),
        GrammarRule(
            name="insert_stmt",
            line_number=41,
            body=Sequence(elements=[Literal(value="INSERT"), Literal(value="INTO"), RuleReference(name="NAME", is_token=True), OptGroup(element=Sequence(elements=[Literal(value="("), RuleReference(name="NAME", is_token=True), Repetition(element=Sequence(elements=[Literal(value=","), RuleReference(name="NAME", is_token=True)])), Literal(value=")")])), Literal(value="VALUES"), RuleReference(name="row_value", is_token=False), Repetition(element=Sequence(elements=[Literal(value=","), RuleReference(name="row_value", is_token=False)]))]),
        ),
        GrammarRule(
            name="row_value",
            line_number=44,
            body=Sequence(elements=[Literal(value="("), RuleReference(name="expr", is_token=False), Repetition(element=Sequence(elements=[Literal(value=","), RuleReference(name="expr", is_token=False)])), Literal(value=")")]),
        ),
        GrammarRule(
            name="update_stmt",
            line_number=46,
            body=Sequence(elements=[Literal(value="UPDATE"), RuleReference(name="NAME", is_token=True), Literal(value="SET"), RuleReference(name="assignment", is_token=False), Repetition(element=Sequence(elements=[Literal(value=","), RuleReference(name="assignment", is_token=False)])), OptGroup(element=RuleReference(name="where_clause", is_token=False))]),
        ),
        GrammarRule(
            name="assignment",
            line_number=48,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), Literal(value="="), RuleReference(name="expr", is_token=False)]),
        ),
        GrammarRule(
            name="delete_stmt",
            line_number=50,
            body=Sequence(elements=[Literal(value="DELETE"), Literal(value="FROM"), RuleReference(name="NAME", is_token=True), OptGroup(element=RuleReference(name="where_clause", is_token=False))]),
        ),
        GrammarRule(
            name="create_table_stmt",
            line_number=54,
            body=Sequence(elements=[Literal(value="CREATE"), Literal(value="TABLE"), OptGroup(element=Sequence(elements=[Literal(value="IF"), Literal(value="NOT"), Literal(value="EXISTS")])), RuleReference(name="NAME", is_token=True), Literal(value="("), RuleReference(name="col_def", is_token=False), Repetition(element=Sequence(elements=[Literal(value=","), RuleReference(name="col_def", is_token=False)])), Literal(value=")")]),
        ),
        GrammarRule(
            name="col_def",
            line_number=56,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="NAME", is_token=True), Repetition(element=RuleReference(name="col_constraint", is_token=False))]),
        ),
        GrammarRule(
            name="col_constraint",
            line_number=57,
            body=Alternation(choices=[Group(element=Sequence(elements=[Literal(value="NOT"), Literal(value="NULL")])), Literal(value="NULL"), Group(element=Sequence(elements=[Literal(value="PRIMARY"), Literal(value="KEY")])), Literal(value="UNIQUE"), Group(element=Sequence(elements=[Literal(value="DEFAULT"), RuleReference(name="primary", is_token=False)]))]),
        ),
        GrammarRule(
            name="drop_table_stmt",
            line_number=60,
            body=Sequence(elements=[Literal(value="DROP"), Literal(value="TABLE"), OptGroup(element=Sequence(elements=[Literal(value="IF"), Literal(value="EXISTS")])), RuleReference(name="NAME", is_token=True)]),
        ),
        GrammarRule(
            name="expr",
            line_number=64,
            body=RuleReference(name="or_expr", is_token=False),
        ),
        GrammarRule(
            name="or_expr",
            line_number=65,
            body=Sequence(elements=[RuleReference(name="and_expr", is_token=False), Repetition(element=Sequence(elements=[Literal(value="OR"), RuleReference(name="and_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="and_expr",
            line_number=66,
            body=Sequence(elements=[RuleReference(name="not_expr", is_token=False), Repetition(element=Sequence(elements=[Literal(value="AND"), RuleReference(name="not_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="not_expr",
            line_number=67,
            body=Alternation(choices=[Sequence(elements=[Literal(value="NOT"), RuleReference(name="not_expr", is_token=False)]), RuleReference(name="comparison", is_token=False)]),
        ),
        GrammarRule(
            name="comparison",
            line_number=68,
            body=Sequence(elements=[RuleReference(name="additive", is_token=False), OptGroup(element=Alternation(choices=[Sequence(elements=[RuleReference(name="cmp_op", is_token=False), RuleReference(name="additive", is_token=False)]), Sequence(elements=[Literal(value="BETWEEN"), RuleReference(name="additive", is_token=False), Literal(value="AND"), RuleReference(name="additive", is_token=False)]), Sequence(elements=[Literal(value="NOT"), Literal(value="BETWEEN"), RuleReference(name="additive", is_token=False), Literal(value="AND"), RuleReference(name="additive", is_token=False)]), Sequence(elements=[Literal(value="IN"), Literal(value="("), RuleReference(name="value_list", is_token=False), Literal(value=")")]), Sequence(elements=[Literal(value="NOT"), Literal(value="IN"), Literal(value="("), RuleReference(name="value_list", is_token=False), Literal(value=")")]), Sequence(elements=[Literal(value="LIKE"), RuleReference(name="additive", is_token=False)]), Sequence(elements=[Literal(value="NOT"), Literal(value="LIKE"), RuleReference(name="additive", is_token=False)]), Sequence(elements=[Literal(value="IS"), Literal(value="NULL")]), Sequence(elements=[Literal(value="IS"), Literal(value="NOT"), Literal(value="NULL")])]))]),
        ),
        GrammarRule(
            name="cmp_op",
            line_number=78,
            body=Alternation(choices=[Literal(value="="), RuleReference(name="NOT_EQUALS", is_token=True), Literal(value="<"), Literal(value=">"), Literal(value="<="), Literal(value=">=")]),
        ),
        GrammarRule(
            name="additive",
            line_number=79,
            body=Sequence(elements=[RuleReference(name="multiplicative", is_token=False), Repetition(element=Sequence(elements=[Group(element=Alternation(choices=[Literal(value="+"), Literal(value="-")])), RuleReference(name="multiplicative", is_token=False)]))]),
        ),
        GrammarRule(
            name="multiplicative",
            line_number=80,
            body=Sequence(elements=[RuleReference(name="unary", is_token=False), Repetition(element=Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="STAR", is_token=True), Literal(value="/"), Literal(value="%")])), RuleReference(name="unary", is_token=False)]))]),
        ),
        GrammarRule(
            name="unary",
            line_number=81,
            body=Alternation(choices=[Sequence(elements=[Literal(value="-"), RuleReference(name="unary", is_token=False)]), RuleReference(name="primary", is_token=False)]),
        ),
        GrammarRule(
            name="primary",
            line_number=82,
            body=Alternation(choices=[RuleReference(name="NUMBER", is_token=True), RuleReference(name="STRING", is_token=True), Literal(value="NULL"), Literal(value="TRUE"), Literal(value="FALSE"), RuleReference(name="function_call", is_token=False), RuleReference(name="column_ref", is_token=False), Sequence(elements=[Literal(value="("), RuleReference(name="expr", is_token=False), Literal(value=")")])]),
        ),
        GrammarRule(
            name="column_ref",
            line_number=85,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), OptGroup(element=Sequence(elements=[Literal(value="."), RuleReference(name="NAME", is_token=True)]))]),
        ),
        GrammarRule(
            name="function_call",
            line_number=86,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), Literal(value="("), Group(element=Alternation(choices=[RuleReference(name="STAR", is_token=True), OptGroup(element=RuleReference(name="value_list", is_token=False))])), Literal(value=")")]),
        ),
        GrammarRule(
            name="value_list",
            line_number=87,
            body=Sequence(elements=[RuleReference(name="expr", is_token=False), Repetition(element=Sequence(elements=[Literal(value=","), RuleReference(name="expr", is_token=False)]))]),
        ),
    ],
)
