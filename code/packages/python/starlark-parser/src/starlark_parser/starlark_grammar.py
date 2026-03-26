# AUTO-GENERATED FILE - DO NOT EDIT
from grammar_tools.parser_grammar import (
    ParserGrammar, GrammarRule, GrammarElement,
    RuleReference, Literal, Sequence, Alternation,
    Repetition, Optional as OptGroup, Group
)

StarlarkGrammar = ParserGrammar(
    version=1,
    rules=[
        GrammarRule(
            name="file",
            line_number=34,
            body=Repetition(element=Alternation(choices=[RuleReference(name="NEWLINE", is_token=True), RuleReference(name="statement", is_token=False)])),
        ),
        GrammarRule(
            name="statement",
            line_number=48,
            body=Alternation(choices=[RuleReference(name="compound_stmt", is_token=False), RuleReference(name="simple_stmt", is_token=False)]),
        ),
        GrammarRule(
            name="simple_stmt",
            line_number=52,
            body=Sequence(elements=[RuleReference(name="small_stmt", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="SEMICOLON", is_token=True), RuleReference(name="small_stmt", is_token=False)])), RuleReference(name="NEWLINE", is_token=True)]),
        ),
        GrammarRule(
            name="small_stmt",
            line_number=54,
            body=Alternation(choices=[RuleReference(name="return_stmt", is_token=False), RuleReference(name="break_stmt", is_token=False), RuleReference(name="continue_stmt", is_token=False), RuleReference(name="pass_stmt", is_token=False), RuleReference(name="load_stmt", is_token=False), RuleReference(name="assign_stmt", is_token=False)]),
        ),
        GrammarRule(
            name="return_stmt",
            line_number=68,
            body=Sequence(elements=[Literal(value="return"), OptGroup(element=RuleReference(name="expression", is_token=False))]),
        ),
        GrammarRule(
            name="break_stmt",
            line_number=71,
            body=Literal(value="break"),
        ),
        GrammarRule(
            name="continue_stmt",
            line_number=74,
            body=Literal(value="continue"),
        ),
        GrammarRule(
            name="pass_stmt",
            line_number=79,
            body=Literal(value="pass"),
        ),
        GrammarRule(
            name="load_stmt",
            line_number=88,
            body=Sequence(elements=[Literal(value="load"), RuleReference(name="LPAREN", is_token=True), RuleReference(name="STRING", is_token=True), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="load_arg", is_token=False)])), OptGroup(element=RuleReference(name="COMMA", is_token=True)), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="load_arg",
            line_number=89,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="EQUALS", is_token=True), RuleReference(name="STRING", is_token=True)]), RuleReference(name="STRING", is_token=True)]),
        ),
        GrammarRule(
            name="assign_stmt",
            line_number=110,
            body=Sequence(elements=[RuleReference(name="expression_list", is_token=False), OptGroup(element=Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="assign_op", is_token=False), RuleReference(name="augmented_assign_op", is_token=False)])), RuleReference(name="expression_list", is_token=False)]))]),
        ),
        GrammarRule(
            name="assign_op",
            line_number=113,
            body=RuleReference(name="EQUALS", is_token=True),
        ),
        GrammarRule(
            name="augmented_assign_op",
            line_number=115,
            body=Alternation(choices=[RuleReference(name="PLUS_EQUALS", is_token=True), RuleReference(name="MINUS_EQUALS", is_token=True), RuleReference(name="STAR_EQUALS", is_token=True), RuleReference(name="SLASH_EQUALS", is_token=True), RuleReference(name="FLOOR_DIV_EQUALS", is_token=True), RuleReference(name="PERCENT_EQUALS", is_token=True), RuleReference(name="AMP_EQUALS", is_token=True), RuleReference(name="PIPE_EQUALS", is_token=True), RuleReference(name="CARET_EQUALS", is_token=True), RuleReference(name="LEFT_SHIFT_EQUALS", is_token=True), RuleReference(name="RIGHT_SHIFT_EQUALS", is_token=True), RuleReference(name="DOUBLE_STAR_EQUALS", is_token=True)]),
        ),
        GrammarRule(
            name="compound_stmt",
            line_number=124,
            body=Alternation(choices=[RuleReference(name="if_stmt", is_token=False), RuleReference(name="for_stmt", is_token=False), RuleReference(name="def_stmt", is_token=False)]),
        ),
        GrammarRule(
            name="if_stmt",
            line_number=136,
            body=Sequence(elements=[Literal(value="if"), RuleReference(name="expression", is_token=False), RuleReference(name="COLON", is_token=True), RuleReference(name="suite", is_token=False), Repetition(element=Sequence(elements=[Literal(value="elif"), RuleReference(name="expression", is_token=False), RuleReference(name="COLON", is_token=True), RuleReference(name="suite", is_token=False)])), OptGroup(element=Sequence(elements=[Literal(value="else"), RuleReference(name="COLON", is_token=True), RuleReference(name="suite", is_token=False)]))]),
        ),
        GrammarRule(
            name="for_stmt",
            line_number=150,
            body=Sequence(elements=[Literal(value="for"), RuleReference(name="loop_vars", is_token=False), Literal(value="in"), RuleReference(name="expression", is_token=False), RuleReference(name="COLON", is_token=True), RuleReference(name="suite", is_token=False)]),
        ),
        GrammarRule(
            name="loop_vars",
            line_number=156,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="NAME", is_token=True)]))]),
        ),
        GrammarRule(
            name="def_stmt",
            line_number=166,
            body=Sequence(elements=[Literal(value="def"), RuleReference(name="NAME", is_token=True), RuleReference(name="LPAREN", is_token=True), OptGroup(element=RuleReference(name="parameters", is_token=False)), RuleReference(name="RPAREN", is_token=True), RuleReference(name="COLON", is_token=True), RuleReference(name="suite", is_token=False)]),
        ),
        GrammarRule(
            name="suite",
            line_number=177,
            body=Alternation(choices=[RuleReference(name="simple_stmt", is_token=False), Sequence(elements=[RuleReference(name="NEWLINE", is_token=True), RuleReference(name="INDENT", is_token=True), Repetition(element=RuleReference(name="statement", is_token=False)), RuleReference(name="DEDENT", is_token=True)])]),
        ),
        GrammarRule(
            name="parameters",
            line_number=198,
            body=Sequence(elements=[RuleReference(name="parameter", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="parameter", is_token=False)])), OptGroup(element=RuleReference(name="COMMA", is_token=True))]),
        ),
        GrammarRule(
            name="parameter",
            line_number=200,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="DOUBLE_STAR", is_token=True), RuleReference(name="NAME", is_token=True)]), Sequence(elements=[RuleReference(name="STAR", is_token=True), RuleReference(name="NAME", is_token=True)]), Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="EQUALS", is_token=True), RuleReference(name="expression", is_token=False)]), RuleReference(name="NAME", is_token=True)]),
        ),
        GrammarRule(
            name="expression_list",
            line_number=234,
            body=Sequence(elements=[RuleReference(name="expression", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="expression", is_token=False)])), OptGroup(element=RuleReference(name="COMMA", is_token=True))]),
        ),
        GrammarRule(
            name="expression",
            line_number=239,
            body=Alternation(choices=[RuleReference(name="lambda_expr", is_token=False), Sequence(elements=[RuleReference(name="or_expr", is_token=False), OptGroup(element=Sequence(elements=[Literal(value="if"), RuleReference(name="or_expr", is_token=False), Literal(value="else"), RuleReference(name="expression", is_token=False)]))])]),
        ),
        GrammarRule(
            name="lambda_expr",
            line_number=244,
            body=Sequence(elements=[Literal(value="lambda"), OptGroup(element=RuleReference(name="lambda_params", is_token=False)), RuleReference(name="COLON", is_token=True), RuleReference(name="expression", is_token=False)]),
        ),
        GrammarRule(
            name="lambda_params",
            line_number=245,
            body=Sequence(elements=[RuleReference(name="lambda_param", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="lambda_param", is_token=False)])), OptGroup(element=RuleReference(name="COMMA", is_token=True))]),
        ),
        GrammarRule(
            name="lambda_param",
            line_number=246,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="NAME", is_token=True), OptGroup(element=Sequence(elements=[RuleReference(name="EQUALS", is_token=True), RuleReference(name="expression", is_token=False)]))]), Sequence(elements=[RuleReference(name="STAR", is_token=True), RuleReference(name="NAME", is_token=True)]), Sequence(elements=[RuleReference(name="DOUBLE_STAR", is_token=True), RuleReference(name="NAME", is_token=True)])]),
        ),
        GrammarRule(
            name="or_expr",
            line_number=250,
            body=Sequence(elements=[RuleReference(name="and_expr", is_token=False), Repetition(element=Sequence(elements=[Literal(value="or"), RuleReference(name="and_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="and_expr",
            line_number=254,
            body=Sequence(elements=[RuleReference(name="not_expr", is_token=False), Repetition(element=Sequence(elements=[Literal(value="and"), RuleReference(name="not_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="not_expr",
            line_number=258,
            body=Alternation(choices=[Sequence(elements=[Literal(value="not"), RuleReference(name="not_expr", is_token=False)]), RuleReference(name="comparison", is_token=False)]),
        ),
        GrammarRule(
            name="comparison",
            line_number=267,
            body=Sequence(elements=[RuleReference(name="bitwise_or", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="comp_op", is_token=False), RuleReference(name="bitwise_or", is_token=False)]))]),
        ),
        GrammarRule(
            name="comp_op",
            line_number=269,
            body=Alternation(choices=[RuleReference(name="EQUALS_EQUALS", is_token=True), RuleReference(name="NOT_EQUALS", is_token=True), RuleReference(name="LESS_THAN", is_token=True), RuleReference(name="GREATER_THAN", is_token=True), RuleReference(name="LESS_EQUALS", is_token=True), RuleReference(name="GREATER_EQUALS", is_token=True), Literal(value="in"), Sequence(elements=[Literal(value="not"), Literal(value="in")])]),
        ),
        GrammarRule(
            name="bitwise_or",
            line_number=275,
            body=Sequence(elements=[RuleReference(name="bitwise_xor", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="PIPE", is_token=True), RuleReference(name="bitwise_xor", is_token=False)]))]),
        ),
        GrammarRule(
            name="bitwise_xor",
            line_number=276,
            body=Sequence(elements=[RuleReference(name="bitwise_and", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="CARET", is_token=True), RuleReference(name="bitwise_and", is_token=False)]))]),
        ),
        GrammarRule(
            name="bitwise_and",
            line_number=277,
            body=Sequence(elements=[RuleReference(name="shift", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="AMP", is_token=True), RuleReference(name="shift", is_token=False)]))]),
        ),
        GrammarRule(
            name="shift",
            line_number=280,
            body=Sequence(elements=[RuleReference(name="arith", is_token=False), Repetition(element=Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="LEFT_SHIFT", is_token=True), RuleReference(name="RIGHT_SHIFT", is_token=True)])), RuleReference(name="arith", is_token=False)]))]),
        ),
        GrammarRule(
            name="arith",
            line_number=284,
            body=Sequence(elements=[RuleReference(name="term", is_token=False), Repetition(element=Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="PLUS", is_token=True), RuleReference(name="MINUS", is_token=True)])), RuleReference(name="term", is_token=False)]))]),
        ),
        GrammarRule(
            name="term",
            line_number=289,
            body=Sequence(elements=[RuleReference(name="factor", is_token=False), Repetition(element=Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="STAR", is_token=True), RuleReference(name="SLASH", is_token=True), RuleReference(name="FLOOR_DIV", is_token=True), RuleReference(name="PERCENT", is_token=True)])), RuleReference(name="factor", is_token=False)]))]),
        ),
        GrammarRule(
            name="factor",
            line_number=295,
            body=Alternation(choices=[Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="PLUS", is_token=True), RuleReference(name="MINUS", is_token=True), RuleReference(name="TILDE", is_token=True)])), RuleReference(name="factor", is_token=False)]), RuleReference(name="power", is_token=False)]),
        ),
        GrammarRule(
            name="power",
            line_number=303,
            body=Sequence(elements=[RuleReference(name="primary", is_token=False), OptGroup(element=Sequence(elements=[RuleReference(name="DOUBLE_STAR", is_token=True), RuleReference(name="factor", is_token=False)]))]),
        ),
        GrammarRule(
            name="primary",
            line_number=320,
            body=Sequence(elements=[RuleReference(name="atom", is_token=False), Repetition(element=RuleReference(name="suffix", is_token=False))]),
        ),
        GrammarRule(
            name="suffix",
            line_number=322,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="DOT", is_token=True), RuleReference(name="NAME", is_token=True)]), Sequence(elements=[RuleReference(name="LBRACKET", is_token=True), RuleReference(name="subscript", is_token=False), RuleReference(name="RBRACKET", is_token=True)]), Sequence(elements=[RuleReference(name="LPAREN", is_token=True), OptGroup(element=RuleReference(name="arguments", is_token=False)), RuleReference(name="RPAREN", is_token=True)])]),
        ),
        GrammarRule(
            name="subscript",
            line_number=334,
            body=Alternation(choices=[RuleReference(name="expression", is_token=False), Sequence(elements=[OptGroup(element=RuleReference(name="expression", is_token=False)), RuleReference(name="COLON", is_token=True), OptGroup(element=RuleReference(name="expression", is_token=False)), OptGroup(element=Sequence(elements=[RuleReference(name="COLON", is_token=True), OptGroup(element=RuleReference(name="expression", is_token=False))]))])]),
        ),
        GrammarRule(
            name="atom",
            line_number=343,
            body=Alternation(choices=[RuleReference(name="INT", is_token=True), RuleReference(name="FLOAT", is_token=True), Sequence(elements=[RuleReference(name="STRING", is_token=True), Repetition(element=RuleReference(name="STRING", is_token=True))]), RuleReference(name="NAME", is_token=True), Literal(value="True"), Literal(value="False"), Literal(value="None"), RuleReference(name="list_expr", is_token=False), RuleReference(name="dict_expr", is_token=False), RuleReference(name="paren_expr", is_token=False)]),
        ),
        GrammarRule(
            name="list_expr",
            line_number=359,
            body=Sequence(elements=[RuleReference(name="LBRACKET", is_token=True), OptGroup(element=RuleReference(name="list_body", is_token=False)), RuleReference(name="RBRACKET", is_token=True)]),
        ),
        GrammarRule(
            name="list_body",
            line_number=361,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="expression", is_token=False), RuleReference(name="comp_clause", is_token=False)]), Sequence(elements=[RuleReference(name="expression", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="expression", is_token=False)])), OptGroup(element=RuleReference(name="COMMA", is_token=True))])]),
        ),
        GrammarRule(
            name="dict_expr",
            line_number=367,
            body=Sequence(elements=[RuleReference(name="LBRACE", is_token=True), OptGroup(element=RuleReference(name="dict_body", is_token=False)), RuleReference(name="RBRACE", is_token=True)]),
        ),
        GrammarRule(
            name="dict_body",
            line_number=369,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="dict_entry", is_token=False), RuleReference(name="comp_clause", is_token=False)]), Sequence(elements=[RuleReference(name="dict_entry", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="dict_entry", is_token=False)])), OptGroup(element=RuleReference(name="COMMA", is_token=True))])]),
        ),
        GrammarRule(
            name="dict_entry",
            line_number=372,
            body=Sequence(elements=[RuleReference(name="expression", is_token=False), RuleReference(name="COLON", is_token=True), RuleReference(name="expression", is_token=False)]),
        ),
        GrammarRule(
            name="paren_expr",
            line_number=379,
            body=Sequence(elements=[RuleReference(name="LPAREN", is_token=True), OptGroup(element=RuleReference(name="paren_body", is_token=False)), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="paren_body",
            line_number=381,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="expression", is_token=False), RuleReference(name="comp_clause", is_token=False)]), Sequence(elements=[RuleReference(name="expression", is_token=False), RuleReference(name="COMMA", is_token=True), OptGroup(element=Sequence(elements=[RuleReference(name="expression", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="expression", is_token=False)])), OptGroup(element=RuleReference(name="COMMA", is_token=True))]))]), RuleReference(name="expression", is_token=False)]),
        ),
        GrammarRule(
            name="comp_clause",
            line_number=397,
            body=Sequence(elements=[RuleReference(name="comp_for", is_token=False), Repetition(element=Alternation(choices=[RuleReference(name="comp_for", is_token=False), RuleReference(name="comp_if", is_token=False)]))]),
        ),
        GrammarRule(
            name="comp_for",
            line_number=399,
            body=Sequence(elements=[Literal(value="for"), RuleReference(name="loop_vars", is_token=False), Literal(value="in"), RuleReference(name="or_expr", is_token=False)]),
        ),
        GrammarRule(
            name="comp_if",
            line_number=401,
            body=Sequence(elements=[Literal(value="if"), RuleReference(name="or_expr", is_token=False)]),
        ),
        GrammarRule(
            name="arguments",
            line_number=420,
            body=Sequence(elements=[RuleReference(name="argument", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="argument", is_token=False)])), OptGroup(element=RuleReference(name="COMMA", is_token=True))]),
        ),
        GrammarRule(
            name="argument",
            line_number=422,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="DOUBLE_STAR", is_token=True), RuleReference(name="expression", is_token=False)]), Sequence(elements=[RuleReference(name="STAR", is_token=True), RuleReference(name="expression", is_token=False)]), Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="EQUALS", is_token=True), RuleReference(name="expression", is_token=False)]), RuleReference(name="expression", is_token=False)]),
        ),
    ],
)
