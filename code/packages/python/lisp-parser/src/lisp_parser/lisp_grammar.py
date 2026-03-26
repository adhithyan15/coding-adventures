# AUTO-GENERATED FILE - DO NOT EDIT
from grammar_tools.parser_grammar import (
    ParserGrammar, GrammarRule, GrammarElement,
    RuleReference, Literal, Sequence, Alternation,
    Repetition, Optional as OptGroup, Group
)

LispGrammar = ParserGrammar(
    version=1,
    rules=[
        GrammarRule(
            name="program",
            line_number=2,
            body=Repetition(element=RuleReference(name="sexpr", is_token=False)),
        ),
        GrammarRule(
            name="sexpr",
            line_number=3,
            body=Alternation(choices=[RuleReference(name="atom", is_token=False), RuleReference(name="list", is_token=False), RuleReference(name="quoted", is_token=False)]),
        ),
        GrammarRule(
            name="atom",
            line_number=4,
            body=Alternation(choices=[RuleReference(name="NUMBER", is_token=True), RuleReference(name="SYMBOL", is_token=True), RuleReference(name="STRING", is_token=True)]),
        ),
        GrammarRule(
            name="list",
            line_number=5,
            body=Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="list_body", is_token=False), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="list_body",
            line_number=6,
            body=OptGroup(element=Sequence(elements=[RuleReference(name="sexpr", is_token=False), Repetition(element=RuleReference(name="sexpr", is_token=False)), OptGroup(element=Sequence(elements=[RuleReference(name="DOT", is_token=True), RuleReference(name="sexpr", is_token=False)]))])),
        ),
        GrammarRule(
            name="quoted",
            line_number=7,
            body=Sequence(elements=[RuleReference(name="QUOTE", is_token=True), RuleReference(name="sexpr", is_token=False)]),
        ),
    ],
)
