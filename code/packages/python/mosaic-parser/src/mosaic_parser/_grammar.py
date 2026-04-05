# AUTO-GENERATED FILE — DO NOT EDIT
# Source: mosaic.grammar
# Regenerate with: grammar-tools compile-grammar <source.grammar>
#
# This file embeds a ParserGrammar as native Python data structures.
# Downstream packages import PARSER_GRAMMAR directly instead of
# reading and parsing the .grammar file at runtime.

from grammar_tools.parser_grammar import (
    Alternation,
    GrammarRule,
    Optional,
    ParserGrammar,
    Repetition,
    RuleReference,
    Sequence,
)

# fmt: off  # noqa: E501 — generated code may have long lines

def _ref(name: str) -> RuleReference:
    """Shorthand: create a token or rule reference by name."""
    return RuleReference(name=name, is_token=name.isupper() or name[0].isupper() and "_" not in name and name.isupper())


def _tok(name: str) -> RuleReference:
    """Create a token reference (UPPERCASE)."""
    return RuleReference(name=name, is_token=True)


def _rule(name: str) -> RuleReference:
    """Create a rule reference (lowercase)."""
    return RuleReference(name=name, is_token=False)


def _gr(name: str, body: object, line: int) -> GrammarRule:  # type: ignore[return]
    """Shorthand to create a GrammarRule with a line number."""
    return GrammarRule(name=name, body=body, line_number=line)  # type: ignore[call-arg]


PARSER_GRAMMAR = ParserGrammar(
    version=1,
    rules=[

        # file = { import_decl } component_decl ;
        _gr("file", Sequence(elements=[
            Repetition(element=_rule("import_decl")),
            _rule("component_decl"),
        ]), 9),

        # import_decl = KEYWORD NAME [ KEYWORD NAME ] KEYWORD STRING SEMICOLON ;
        _gr("import_decl", Sequence(elements=[
            _tok("KEYWORD"),   # "import"
            _tok("NAME"),      # component name
            Optional(element=Sequence(elements=[
                _tok("KEYWORD"),   # "as"
                _tok("NAME"),      # alias
            ])),
            _tok("KEYWORD"),   # "from"
            _tok("STRING"),    # path
            _tok("SEMICOLON"),
        ]), 12),

        # component_decl = KEYWORD NAME LBRACE { slot_decl } node_tree RBRACE ;
        _gr("component_decl", Sequence(elements=[
            _tok("KEYWORD"),   # "component"
            _tok("NAME"),      # component name
            _tok("LBRACE"),
            Repetition(element=_rule("slot_decl")),
            _rule("node_tree"),
            _tok("RBRACE"),
        ]), 30),

        # slot_decl = KEYWORD NAME COLON slot_type [ EQUALS default_value ] SEMICOLON ;
        _gr("slot_decl", Sequence(elements=[
            _tok("KEYWORD"),   # "slot"
            _tok("NAME"),      # slot name
            _tok("COLON"),
            _rule("slot_type"),
            Optional(element=Sequence(elements=[
                _tok("EQUALS"),
                _rule("default_value"),
            ])),
            _tok("SEMICOLON"),
        ]), 50),

        # slot_type = list_type | KEYWORD | NAME ;
        # NOTE: list_type must be tried BEFORE KEYWORD, because both begin
        # with the KEYWORD token "list". If KEYWORD is tried first, the parser
        # consumes "list" and the list_type rule never matches.
        _gr("slot_type", Alternation(choices=[
            _rule("list_type"),
            _tok("KEYWORD"),
            _tok("NAME"),
        ]), 58),

        # list_type = KEYWORD LANGLE slot_type RANGLE ;
        _gr("list_type", Sequence(elements=[
            _tok("KEYWORD"),   # "list"
            _tok("LANGLE"),    # "<"
            _rule("slot_type"),
            _tok("RANGLE"),    # ">"
        ]), 63),

        # default_value = STRING | NUMBER | DIMENSION | COLOR_HEX | KEYWORD ;
        _gr("default_value", Alternation(choices=[
            _tok("STRING"),
            _tok("DIMENSION"),
            _tok("NUMBER"),
            _tok("COLOR_HEX"),
            _tok("KEYWORD"),
        ]), 70),

        # node_tree = node_element ;
        _gr("node_tree", _rule("node_element"), 78),

        # node_element = NAME LBRACE { node_content } RBRACE ;
        _gr("node_element", Sequence(elements=[
            _tok("NAME"),
            _tok("LBRACE"),
            Repetition(element=_rule("node_content")),
            _tok("RBRACE"),
        ]), 82),

        # node_content = when_block | each_block | slot_reference | child_node
        #              | property_assignment
        _gr("node_content", Alternation(choices=[
            _rule("when_block"),
            _rule("each_block"),
            _rule("slot_reference"),
            _rule("child_node"),
            _rule("property_assignment"),
        ]), 90),

        # property_assignment = ( NAME | KEYWORD ) COLON property_value SEMICOLON ;
        _gr("property_assignment", Sequence(elements=[
            Alternation(choices=[
                _tok("NAME"),
                _tok("KEYWORD"),
            ]),
            _tok("COLON"),
            _rule("property_value"),
            _tok("SEMICOLON"),
        ]), 100),

        # property_value = slot_ref | enum_value | STRING | DIMENSION | NUMBER
        #                | COLOR_HEX | KEYWORD | NAME ;
        _gr("property_value", Alternation(choices=[
            _rule("slot_ref"),
            _rule("enum_value"),
            _tok("STRING"),
            _tok("DIMENSION"),
            _tok("NUMBER"),
            _tok("COLOR_HEX"),
            _tok("KEYWORD"),
            _tok("NAME"),
        ]), 108),

        # slot_ref = AT NAME ;
        _gr("slot_ref", Sequence(elements=[
            _tok("AT"),
            _tok("NAME"),
        ]), 120),

        # enum_value = NAME DOT NAME ;
        _gr("enum_value", Sequence(elements=[
            _tok("NAME"),
            _tok("DOT"),
            _tok("NAME"),
        ]), 125),

        # child_node = node_element ;
        _gr("child_node", _rule("node_element"), 130),

        # slot_reference = AT NAME SEMICOLON ;
        _gr("slot_reference", Sequence(elements=[
            _tok("AT"),
            _tok("NAME"),
            _tok("SEMICOLON"),
        ]), 135),

        # when_block = KEYWORD slot_ref LBRACE { node_content } RBRACE ;
        _gr("when_block", Sequence(elements=[
            _tok("KEYWORD"),   # "when"
            _rule("slot_ref"),
            _tok("LBRACE"),
            Repetition(element=_rule("node_content")),
            _tok("RBRACE"),
        ]), 145),

        # each_block = KEYWORD slot_ref KEYWORD NAME LBRACE { node_content } RBRACE ;
        _gr("each_block", Sequence(elements=[
            _tok("KEYWORD"),   # "each"
            _rule("slot_ref"),
            _tok("KEYWORD"),   # "as"
            _tok("NAME"),      # loop variable
            _tok("LBRACE"),
            Repetition(element=_rule("node_content")),
            _tok("RBRACE"),
        ]), 155),
    ],
)
