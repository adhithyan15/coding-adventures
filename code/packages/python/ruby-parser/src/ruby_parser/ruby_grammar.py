# AUTO-GENERATED FILE - DO NOT EDIT
from grammar_tools.parser_grammar import (
    ParserGrammar, GrammarRule, GrammarElement,
    RuleReference, Literal, Sequence, Alternation,
    Repetition, Optional as OptGroup, Group
)

RubyGrammar = ParserGrammar(
    version=1,
    rules=[
        GrammarRule(
            name="program",
            line_number=22,
            body=Repetition(element=RuleReference(name="statement", is_token=False)),
        ),
        GrammarRule(
            name="statement",
            line_number=23,
            body=Alternation(choices=[RuleReference(name="assignment", is_token=False), RuleReference(name="method_call", is_token=False), RuleReference(name="expression_stmt", is_token=False)]),
        ),
        GrammarRule(
            name="assignment",
            line_number=24,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="EQUALS", is_token=True), RuleReference(name="expression", is_token=False)]),
        ),
        GrammarRule(
            name="method_call",
            line_number=25,
            body=Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="NAME", is_token=True), RuleReference(name="KEYWORD", is_token=True)])), RuleReference(name="LPAREN", is_token=True), OptGroup(element=Sequence(elements=[RuleReference(name="expression", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="expression", is_token=False)]))])), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="expression_stmt",
            line_number=26,
            body=RuleReference(name="expression", is_token=False),
        ),
        GrammarRule(
            name="expression",
            line_number=27,
            body=Sequence(elements=[RuleReference(name="term", is_token=False), Repetition(element=Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="PLUS", is_token=True), RuleReference(name="MINUS", is_token=True)])), RuleReference(name="term", is_token=False)]))]),
        ),
        GrammarRule(
            name="term",
            line_number=28,
            body=Sequence(elements=[RuleReference(name="factor", is_token=False), Repetition(element=Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="STAR", is_token=True), RuleReference(name="SLASH", is_token=True)])), RuleReference(name="factor", is_token=False)]))]),
        ),
        GrammarRule(
            name="factor",
            line_number=29,
            body=Alternation(choices=[RuleReference(name="NUMBER", is_token=True), RuleReference(name="STRING", is_token=True), RuleReference(name="NAME", is_token=True), RuleReference(name="KEYWORD", is_token=True), Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="expression", is_token=False), RuleReference(name="RPAREN", is_token=True)])]),
        ),
    ],
)
