# AUTO-GENERATED FILE - DO NOT EDIT
from grammar_tools.parser_grammar import (
    ParserGrammar, GrammarRule, GrammarElement,
    RuleReference, Literal, Sequence, Alternation,
    Repetition, Optional as OptGroup, Group
)

TypescriptGrammar = ParserGrammar(
    version=1,
    rules=[
        GrammarRule(
            name="program",
            line_number=29,
            body=Repetition(element=RuleReference(name="statement", is_token=False)),
        ),
        GrammarRule(
            name="statement",
            line_number=30,
            body=Alternation(choices=[RuleReference(name="var_declaration", is_token=False), RuleReference(name="assignment", is_token=False), RuleReference(name="expression_stmt", is_token=False)]),
        ),
        GrammarRule(
            name="var_declaration",
            line_number=31,
            body=Sequence(elements=[RuleReference(name="KEYWORD", is_token=True), RuleReference(name="NAME", is_token=True), RuleReference(name="EQUALS", is_token=True), RuleReference(name="expression", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="assignment",
            line_number=32,
            body=Sequence(elements=[RuleReference(name="NAME", is_token=True), RuleReference(name="EQUALS", is_token=True), RuleReference(name="expression", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="expression_stmt",
            line_number=33,
            body=Sequence(elements=[RuleReference(name="expression", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="expression",
            line_number=34,
            body=Sequence(elements=[RuleReference(name="term", is_token=False), Repetition(element=Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="PLUS", is_token=True), RuleReference(name="MINUS", is_token=True)])), RuleReference(name="term", is_token=False)]))]),
        ),
        GrammarRule(
            name="term",
            line_number=35,
            body=Sequence(elements=[RuleReference(name="factor", is_token=False), Repetition(element=Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="STAR", is_token=True), RuleReference(name="SLASH", is_token=True)])), RuleReference(name="factor", is_token=False)]))]),
        ),
        GrammarRule(
            name="factor",
            line_number=36,
            body=Alternation(choices=[RuleReference(name="NUMBER", is_token=True), RuleReference(name="STRING", is_token=True), RuleReference(name="NAME", is_token=True), RuleReference(name="KEYWORD", is_token=True), Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="expression", is_token=False), RuleReference(name="RPAREN", is_token=True)])]),
        ),
    ],
)
