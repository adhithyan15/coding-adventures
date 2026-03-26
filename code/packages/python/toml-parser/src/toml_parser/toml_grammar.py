# AUTO-GENERATED FILE - DO NOT EDIT
from grammar_tools.parser_grammar import (
    ParserGrammar, GrammarRule, GrammarElement,
    RuleReference, Literal, Sequence, Alternation,
    Repetition, Optional as OptGroup, Group
)

TomlGrammar = ParserGrammar(
    version=1,
    rules=[
        GrammarRule(
            name="document",
            line_number=38,
            body=Repetition(element=Alternation(choices=[RuleReference(name="NEWLINE", is_token=True), RuleReference(name="expression", is_token=False)])),
        ),
        GrammarRule(
            name="expression",
            line_number=49,
            body=Alternation(choices=[RuleReference(name="array_table_header", is_token=False), RuleReference(name="table_header", is_token=False), RuleReference(name="keyval", is_token=False)]),
        ),
        GrammarRule(
            name="keyval",
            line_number=57,
            body=Sequence(elements=[RuleReference(name="key", is_token=False), RuleReference(name="EQUALS", is_token=True), RuleReference(name="value", is_token=False)]),
        ),
        GrammarRule(
            name="key",
            line_number=65,
            body=Sequence(elements=[RuleReference(name="simple_key", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="DOT", is_token=True), RuleReference(name="simple_key", is_token=False)]))]),
        ),
        GrammarRule(
            name="simple_key",
            line_number=82,
            body=Alternation(choices=[RuleReference(name="BARE_KEY", is_token=True), RuleReference(name="BASIC_STRING", is_token=True), RuleReference(name="LITERAL_STRING", is_token=True), RuleReference(name="TRUE", is_token=True), RuleReference(name="FALSE", is_token=True), RuleReference(name="INTEGER", is_token=True), RuleReference(name="FLOAT", is_token=True), RuleReference(name="OFFSET_DATETIME", is_token=True), RuleReference(name="LOCAL_DATETIME", is_token=True), RuleReference(name="LOCAL_DATE", is_token=True), RuleReference(name="LOCAL_TIME", is_token=True)]),
        ),
        GrammarRule(
            name="table_header",
            line_number=92,
            body=Sequence(elements=[RuleReference(name="LBRACKET", is_token=True), RuleReference(name="key", is_token=False), RuleReference(name="RBRACKET", is_token=True)]),
        ),
        GrammarRule(
            name="array_table_header",
            line_number=104,
            body=Sequence(elements=[RuleReference(name="LBRACKET", is_token=True), RuleReference(name="LBRACKET", is_token=True), RuleReference(name="key", is_token=False), RuleReference(name="RBRACKET", is_token=True), RuleReference(name="RBRACKET", is_token=True)]),
        ),
        GrammarRule(
            name="value",
            line_number=121,
            body=Alternation(choices=[RuleReference(name="BASIC_STRING", is_token=True), RuleReference(name="ML_BASIC_STRING", is_token=True), RuleReference(name="LITERAL_STRING", is_token=True), RuleReference(name="ML_LITERAL_STRING", is_token=True), RuleReference(name="INTEGER", is_token=True), RuleReference(name="FLOAT", is_token=True), RuleReference(name="TRUE", is_token=True), RuleReference(name="FALSE", is_token=True), RuleReference(name="OFFSET_DATETIME", is_token=True), RuleReference(name="LOCAL_DATETIME", is_token=True), RuleReference(name="LOCAL_DATE", is_token=True), RuleReference(name="LOCAL_TIME", is_token=True), RuleReference(name="array", is_token=False), RuleReference(name="inline_table", is_token=False)]),
        ),
        GrammarRule(
            name="array",
            line_number=140,
            body=Sequence(elements=[RuleReference(name="LBRACKET", is_token=True), RuleReference(name="array_values", is_token=False), RuleReference(name="RBRACKET", is_token=True)]),
        ),
        GrammarRule(
            name="array_values",
            line_number=142,
            body=Sequence(elements=[Repetition(element=RuleReference(name="NEWLINE", is_token=True)), OptGroup(element=Sequence(elements=[RuleReference(name="value", is_token=False), Repetition(element=RuleReference(name="NEWLINE", is_token=True)), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), Repetition(element=RuleReference(name="NEWLINE", is_token=True)), RuleReference(name="value", is_token=False), Repetition(element=RuleReference(name="NEWLINE", is_token=True))])), OptGroup(element=RuleReference(name="COMMA", is_token=True)), Repetition(element=RuleReference(name="NEWLINE", is_token=True))]))]),
        ),
        GrammarRule(
            name="inline_table",
            line_number=162,
            body=Sequence(elements=[RuleReference(name="LBRACE", is_token=True), OptGroup(element=Sequence(elements=[RuleReference(name="keyval", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="keyval", is_token=False)]))])), RuleReference(name="RBRACE", is_token=True)]),
        ),
    ],
)
