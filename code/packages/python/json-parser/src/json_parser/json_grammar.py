# AUTO-GENERATED FILE - DO NOT EDIT
from grammar_tools.parser_grammar import (
    ParserGrammar, GrammarRule, GrammarElement,
    RuleReference, Literal, Sequence, Alternation,
    Repetition, Optional as OptGroup, Group
)

JsonGrammar = ParserGrammar(
    version=1,
    rules=[
        GrammarRule(
            name="value",
            line_number=28,
            body=Alternation(choices=[RuleReference(name="object", is_token=False), RuleReference(name="array", is_token=False), RuleReference(name="STRING", is_token=True), RuleReference(name="NUMBER", is_token=True), RuleReference(name="TRUE", is_token=True), RuleReference(name="FALSE", is_token=True), RuleReference(name="NULL", is_token=True)]),
        ),
        GrammarRule(
            name="object",
            line_number=34,
            body=Sequence(elements=[RuleReference(name="LBRACE", is_token=True), OptGroup(element=Sequence(elements=[RuleReference(name="pair", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="pair", is_token=False)]))])), RuleReference(name="RBRACE", is_token=True)]),
        ),
        GrammarRule(
            name="pair",
            line_number=38,
            body=Sequence(elements=[RuleReference(name="STRING", is_token=True), RuleReference(name="COLON", is_token=True), RuleReference(name="value", is_token=False)]),
        ),
        GrammarRule(
            name="array",
            line_number=42,
            body=Sequence(elements=[RuleReference(name="LBRACKET", is_token=True), OptGroup(element=Sequence(elements=[RuleReference(name="value", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="value", is_token=False)]))])), RuleReference(name="RBRACKET", is_token=True)]),
        ),
    ],
)
