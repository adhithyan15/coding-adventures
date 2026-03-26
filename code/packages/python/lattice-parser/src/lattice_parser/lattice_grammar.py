# AUTO-GENERATED FILE - DO NOT EDIT
from grammar_tools.parser_grammar import (
    ParserGrammar, GrammarRule, GrammarElement,
    RuleReference, Literal, Sequence, Alternation,
    Repetition, Optional as OptGroup, Group
)

LatticeGrammar = ParserGrammar(
    version=1,
    rules=[
        GrammarRule(
            name="stylesheet",
            line_number=37,
            body=Repetition(element=RuleReference(name="rule", is_token=False)),
        ),
        GrammarRule(
            name="rule",
            line_number=39,
            body=Alternation(choices=[RuleReference(name="lattice_rule", is_token=False), RuleReference(name="at_rule", is_token=False), RuleReference(name="qualified_rule", is_token=False)]),
        ),
        GrammarRule(
            name="lattice_rule",
            line_number=51,
            body=Alternation(choices=[RuleReference(name="variable_declaration", is_token=False), RuleReference(name="mixin_definition", is_token=False), RuleReference(name="function_definition", is_token=False), RuleReference(name="use_directive", is_token=False), RuleReference(name="lattice_control", is_token=False)]),
        ),
        GrammarRule(
            name="variable_declaration",
            line_number=69,
            body=Sequence(elements=[RuleReference(name="VARIABLE", is_token=True), RuleReference(name="COLON", is_token=True), RuleReference(name="value_list", is_token=False), OptGroup(element=Alternation(choices=[RuleReference(name="BANG_DEFAULT", is_token=True), RuleReference(name="BANG_GLOBAL", is_token=True)])), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="mixin_definition",
            line_number=102,
            body=Alternation(choices=[Sequence(elements=[Literal(value="@mixin"), RuleReference(name="FUNCTION", is_token=True), OptGroup(element=RuleReference(name="mixin_params", is_token=False)), RuleReference(name="RPAREN", is_token=True), RuleReference(name="block", is_token=False)]), Sequence(elements=[Literal(value="@mixin"), RuleReference(name="IDENT", is_token=True), RuleReference(name="block", is_token=False)])]),
        ),
        GrammarRule(
            name="mixin_params",
            line_number=105,
            body=Sequence(elements=[RuleReference(name="mixin_param", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="mixin_param", is_token=False)]))]),
        ),
        GrammarRule(
            name="mixin_param",
            line_number=112,
            body=Sequence(elements=[RuleReference(name="VARIABLE", is_token=True), OptGroup(element=Sequence(elements=[RuleReference(name="COLON", is_token=True), RuleReference(name="mixin_value_list", is_token=False)]))]),
        ),
        GrammarRule(
            name="mixin_value_list",
            line_number=117,
            body=Sequence(elements=[RuleReference(name="mixin_value", is_token=False), Repetition(element=RuleReference(name="mixin_value", is_token=False))]),
        ),
        GrammarRule(
            name="mixin_value",
            line_number=119,
            body=Alternation(choices=[RuleReference(name="DIMENSION", is_token=True), RuleReference(name="PERCENTAGE", is_token=True), RuleReference(name="NUMBER", is_token=True), RuleReference(name="STRING", is_token=True), RuleReference(name="IDENT", is_token=True), RuleReference(name="HASH", is_token=True), RuleReference(name="CUSTOM_PROPERTY", is_token=True), RuleReference(name="UNICODE_RANGE", is_token=True), RuleReference(name="function_call", is_token=False), RuleReference(name="VARIABLE", is_token=True), RuleReference(name="SLASH", is_token=True), RuleReference(name="PLUS", is_token=True), RuleReference(name="MINUS", is_token=True)]),
        ),
        GrammarRule(
            name="include_directive",
            line_number=130,
            body=Alternation(choices=[Sequence(elements=[Literal(value="@include"), RuleReference(name="FUNCTION", is_token=True), OptGroup(element=RuleReference(name="include_args", is_token=False)), RuleReference(name="RPAREN", is_token=True), Group(element=Alternation(choices=[RuleReference(name="SEMICOLON", is_token=True), RuleReference(name="block", is_token=False)]))]), Sequence(elements=[Literal(value="@include"), RuleReference(name="IDENT", is_token=True), Group(element=Alternation(choices=[RuleReference(name="SEMICOLON", is_token=True), RuleReference(name="block", is_token=False)]))])]),
        ),
        GrammarRule(
            name="include_args",
            line_number=133,
            body=Sequence(elements=[RuleReference(name="include_arg", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="include_arg", is_token=False)]))]),
        ),
        GrammarRule(
            name="include_arg",
            line_number=137,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="VARIABLE", is_token=True), RuleReference(name="COLON", is_token=True), RuleReference(name="value_list", is_token=False)]), RuleReference(name="value_list", is_token=False)]),
        ),
        GrammarRule(
            name="lattice_control",
            line_number=160,
            body=Alternation(choices=[RuleReference(name="if_directive", is_token=False), RuleReference(name="for_directive", is_token=False), RuleReference(name="each_directive", is_token=False), RuleReference(name="while_directive", is_token=False)]),
        ),
        GrammarRule(
            name="if_directive",
            line_number=164,
            body=Sequence(elements=[Literal(value="@if"), RuleReference(name="lattice_expression", is_token=False), RuleReference(name="block", is_token=False), Repetition(element=Sequence(elements=[Literal(value="@else"), Literal(value="if"), RuleReference(name="lattice_expression", is_token=False), RuleReference(name="block", is_token=False)])), OptGroup(element=Sequence(elements=[Literal(value="@else"), RuleReference(name="block", is_token=False)]))]),
        ),
        GrammarRule(
            name="for_directive",
            line_number=171,
            body=Sequence(elements=[Literal(value="@for"), RuleReference(name="VARIABLE", is_token=True), Literal(value="from"), RuleReference(name="lattice_expression", is_token=False), Group(element=Alternation(choices=[Literal(value="through"), Literal(value="to")])), RuleReference(name="lattice_expression", is_token=False), RuleReference(name="block", is_token=False)]),
        ),
        GrammarRule(
            name="each_directive",
            line_number=176,
            body=Sequence(elements=[Literal(value="@each"), RuleReference(name="VARIABLE", is_token=True), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="VARIABLE", is_token=True)])), Literal(value="in"), RuleReference(name="each_list", is_token=False), RuleReference(name="block", is_token=False)]),
        ),
        GrammarRule(
            name="each_list",
            line_number=179,
            body=Sequence(elements=[RuleReference(name="value", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="value", is_token=False)]))]),
        ),
        GrammarRule(
            name="while_directive",
            line_number=184,
            body=Sequence(elements=[Literal(value="@while"), RuleReference(name="lattice_expression", is_token=False), RuleReference(name="block", is_token=False)]),
        ),
        GrammarRule(
            name="lattice_expression",
            line_number=203,
            body=RuleReference(name="lattice_or_expr", is_token=False),
        ),
        GrammarRule(
            name="lattice_or_expr",
            line_number=205,
            body=Sequence(elements=[RuleReference(name="lattice_and_expr", is_token=False), Repetition(element=Sequence(elements=[Literal(value="or"), RuleReference(name="lattice_and_expr", is_token=False)]))]),
        ),
        GrammarRule(
            name="lattice_and_expr",
            line_number=207,
            body=Sequence(elements=[RuleReference(name="lattice_comparison", is_token=False), Repetition(element=Sequence(elements=[Literal(value="and"), RuleReference(name="lattice_comparison", is_token=False)]))]),
        ),
        GrammarRule(
            name="lattice_comparison",
            line_number=209,
            body=Sequence(elements=[RuleReference(name="lattice_additive", is_token=False), OptGroup(element=Sequence(elements=[RuleReference(name="comparison_op", is_token=False), RuleReference(name="lattice_additive", is_token=False)]))]),
        ),
        GrammarRule(
            name="comparison_op",
            line_number=211,
            body=Alternation(choices=[RuleReference(name="EQUALS_EQUALS", is_token=True), RuleReference(name="NOT_EQUALS", is_token=True), RuleReference(name="GREATER", is_token=True), RuleReference(name="GREATER_EQUALS", is_token=True), RuleReference(name="LESS", is_token=True), RuleReference(name="LESS_EQUALS", is_token=True)]),
        ),
        GrammarRule(
            name="lattice_additive",
            line_number=214,
            body=Sequence(elements=[RuleReference(name="lattice_multiplicative", is_token=False), Repetition(element=Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="PLUS", is_token=True), RuleReference(name="MINUS", is_token=True)])), RuleReference(name="lattice_multiplicative", is_token=False)]))]),
        ),
        GrammarRule(
            name="lattice_multiplicative",
            line_number=219,
            body=Sequence(elements=[RuleReference(name="lattice_unary", is_token=False), Repetition(element=Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="STAR", is_token=True), RuleReference(name="SLASH", is_token=True)])), RuleReference(name="lattice_unary", is_token=False)]))]),
        ),
        GrammarRule(
            name="lattice_unary",
            line_number=221,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="MINUS", is_token=True), RuleReference(name="lattice_unary", is_token=False)]), RuleReference(name="lattice_primary", is_token=False)]),
        ),
        GrammarRule(
            name="lattice_primary",
            line_number=224,
            body=Alternation(choices=[RuleReference(name="VARIABLE", is_token=True), RuleReference(name="NUMBER", is_token=True), RuleReference(name="DIMENSION", is_token=True), RuleReference(name="PERCENTAGE", is_token=True), RuleReference(name="STRING", is_token=True), RuleReference(name="IDENT", is_token=True), RuleReference(name="HASH", is_token=True), Literal(value="true"), Literal(value="false"), Literal(value="null"), RuleReference(name="function_call", is_token=False), RuleReference(name="map_literal", is_token=False), Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="lattice_expression", is_token=False), RuleReference(name="RPAREN", is_token=True)])]),
        ),
        GrammarRule(
            name="map_literal",
            line_number=235,
            body=Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="map_entry", is_token=False), RuleReference(name="COMMA", is_token=True), RuleReference(name="map_entry", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="map_entry", is_token=False)])), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="map_entry",
            line_number=237,
            body=Sequence(elements=[Group(element=Alternation(choices=[RuleReference(name="IDENT", is_token=True), RuleReference(name="STRING", is_token=True)])), RuleReference(name="COLON", is_token=True), RuleReference(name="lattice_expression", is_token=False)]),
        ),
        GrammarRule(
            name="function_definition",
            line_number=261,
            body=Alternation(choices=[Sequence(elements=[Literal(value="@function"), RuleReference(name="FUNCTION", is_token=True), OptGroup(element=RuleReference(name="mixin_params", is_token=False)), RuleReference(name="RPAREN", is_token=True), RuleReference(name="function_body", is_token=False)]), Sequence(elements=[Literal(value="@function"), RuleReference(name="IDENT", is_token=True), RuleReference(name="function_body", is_token=False)])]),
        ),
        GrammarRule(
            name="function_body",
            line_number=264,
            body=Sequence(elements=[RuleReference(name="LBRACE", is_token=True), Repetition(element=RuleReference(name="function_body_item", is_token=False)), RuleReference(name="RBRACE", is_token=True)]),
        ),
        GrammarRule(
            name="function_body_item",
            line_number=266,
            body=Alternation(choices=[RuleReference(name="variable_declaration", is_token=False), RuleReference(name="return_directive", is_token=False), RuleReference(name="lattice_control", is_token=False)]),
        ),
        GrammarRule(
            name="return_directive",
            line_number=268,
            body=Sequence(elements=[Literal(value="@return"), RuleReference(name="lattice_expression", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="use_directive",
            line_number=281,
            body=Sequence(elements=[Literal(value="@use"), RuleReference(name="STRING", is_token=True), OptGroup(element=Sequence(elements=[Literal(value="as"), RuleReference(name="IDENT", is_token=True)])), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="at_rule",
            line_number=294,
            body=Sequence(elements=[RuleReference(name="AT_KEYWORD", is_token=True), RuleReference(name="at_prelude", is_token=False), Group(element=Alternation(choices=[RuleReference(name="SEMICOLON", is_token=True), RuleReference(name="block", is_token=False)]))]),
        ),
        GrammarRule(
            name="at_prelude",
            line_number=296,
            body=Repetition(element=RuleReference(name="at_prelude_token", is_token=False)),
        ),
        GrammarRule(
            name="at_prelude_token",
            line_number=298,
            body=Alternation(choices=[RuleReference(name="IDENT", is_token=True), RuleReference(name="STRING", is_token=True), RuleReference(name="NUMBER", is_token=True), RuleReference(name="DIMENSION", is_token=True), RuleReference(name="PERCENTAGE", is_token=True), RuleReference(name="HASH", is_token=True), RuleReference(name="CUSTOM_PROPERTY", is_token=True), RuleReference(name="UNICODE_RANGE", is_token=True), RuleReference(name="VARIABLE", is_token=True), RuleReference(name="function_in_prelude", is_token=False), RuleReference(name="paren_block", is_token=False), RuleReference(name="COLON", is_token=True), RuleReference(name="COMMA", is_token=True), RuleReference(name="SLASH", is_token=True), RuleReference(name="DOT", is_token=True), RuleReference(name="STAR", is_token=True), RuleReference(name="PLUS", is_token=True), RuleReference(name="MINUS", is_token=True), RuleReference(name="GREATER", is_token=True), RuleReference(name="TILDE", is_token=True), RuleReference(name="PIPE", is_token=True), RuleReference(name="EQUALS", is_token=True), RuleReference(name="AMPERSAND", is_token=True), RuleReference(name="CDO", is_token=True), RuleReference(name="CDC", is_token=True)]),
        ),
        GrammarRule(
            name="function_in_prelude",
            line_number=306,
            body=Sequence(elements=[RuleReference(name="FUNCTION", is_token=True), RuleReference(name="at_prelude_tokens", is_token=False), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="paren_block",
            line_number=307,
            body=Sequence(elements=[RuleReference(name="LPAREN", is_token=True), RuleReference(name="at_prelude_tokens", is_token=False), RuleReference(name="RPAREN", is_token=True)]),
        ),
        GrammarRule(
            name="at_prelude_tokens",
            line_number=308,
            body=Repetition(element=RuleReference(name="at_prelude_token", is_token=False)),
        ),
        GrammarRule(
            name="qualified_rule",
            line_number=314,
            body=Sequence(elements=[RuleReference(name="selector_list", is_token=False), RuleReference(name="block", is_token=False)]),
        ),
        GrammarRule(
            name="selector_list",
            line_number=320,
            body=Sequence(elements=[RuleReference(name="complex_selector", is_token=False), Repetition(element=Sequence(elements=[RuleReference(name="COMMA", is_token=True), RuleReference(name="complex_selector", is_token=False)]))]),
        ),
        GrammarRule(
            name="complex_selector",
            line_number=322,
            body=Sequence(elements=[RuleReference(name="compound_selector", is_token=False), Repetition(element=Sequence(elements=[OptGroup(element=RuleReference(name="combinator", is_token=False)), RuleReference(name="compound_selector", is_token=False)]))]),
        ),
        GrammarRule(
            name="combinator",
            line_number=324,
            body=Alternation(choices=[RuleReference(name="GREATER", is_token=True), RuleReference(name="PLUS", is_token=True), RuleReference(name="TILDE", is_token=True)]),
        ),
        GrammarRule(
            name="compound_selector",
            line_number=326,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="simple_selector", is_token=False), Repetition(element=RuleReference(name="subclass_selector", is_token=False))]), Sequence(elements=[RuleReference(name="subclass_selector", is_token=False), Repetition(element=RuleReference(name="subclass_selector", is_token=False))])]),
        ),
        GrammarRule(
            name="simple_selector",
            line_number=330,
            body=Alternation(choices=[RuleReference(name="IDENT", is_token=True), RuleReference(name="STAR", is_token=True), RuleReference(name="AMPERSAND", is_token=True), RuleReference(name="VARIABLE", is_token=True)]),
        ),
        GrammarRule(
            name="subclass_selector",
            line_number=333,
            body=Alternation(choices=[RuleReference(name="class_selector", is_token=False), RuleReference(name="id_selector", is_token=False), RuleReference(name="placeholder_selector", is_token=False), RuleReference(name="attribute_selector", is_token=False), RuleReference(name="pseudo_class", is_token=False), RuleReference(name="pseudo_element", is_token=False)]),
        ),
        GrammarRule(
            name="placeholder_selector",
            line_number=337,
            body=RuleReference(name="PLACEHOLDER", is_token=True),
        ),
        GrammarRule(
            name="class_selector",
            line_number=339,
            body=Sequence(elements=[RuleReference(name="DOT", is_token=True), RuleReference(name="IDENT", is_token=True)]),
        ),
        GrammarRule(
            name="id_selector",
            line_number=341,
            body=RuleReference(name="HASH", is_token=True),
        ),
        GrammarRule(
            name="attribute_selector",
            line_number=343,
            body=Sequence(elements=[RuleReference(name="LBRACKET", is_token=True), RuleReference(name="IDENT", is_token=True), OptGroup(element=Sequence(elements=[RuleReference(name="attr_matcher", is_token=False), RuleReference(name="attr_value", is_token=False), OptGroup(element=RuleReference(name="IDENT", is_token=True))])), RuleReference(name="RBRACKET", is_token=True)]),
        ),
        GrammarRule(
            name="attr_matcher",
            line_number=345,
            body=Alternation(choices=[RuleReference(name="EQUALS", is_token=True), RuleReference(name="TILDE_EQUALS", is_token=True), RuleReference(name="PIPE_EQUALS", is_token=True), RuleReference(name="CARET_EQUALS", is_token=True), RuleReference(name="DOLLAR_EQUALS", is_token=True), RuleReference(name="STAR_EQUALS", is_token=True)]),
        ),
        GrammarRule(
            name="attr_value",
            line_number=348,
            body=Alternation(choices=[RuleReference(name="IDENT", is_token=True), RuleReference(name="STRING", is_token=True)]),
        ),
        GrammarRule(
            name="pseudo_class",
            line_number=350,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="COLON", is_token=True), RuleReference(name="FUNCTION", is_token=True), RuleReference(name="pseudo_class_args", is_token=False), RuleReference(name="RPAREN", is_token=True)]), Sequence(elements=[RuleReference(name="COLON", is_token=True), RuleReference(name="IDENT", is_token=True)])]),
        ),
        GrammarRule(
            name="pseudo_class_args",
            line_number=353,
            body=Repetition(element=RuleReference(name="pseudo_class_arg", is_token=False)),
        ),
        GrammarRule(
            name="pseudo_class_arg",
            line_number=355,
            body=Alternation(choices=[RuleReference(name="IDENT", is_token=True), RuleReference(name="NUMBER", is_token=True), RuleReference(name="DIMENSION", is_token=True), RuleReference(name="STRING", is_token=True), RuleReference(name="HASH", is_token=True), RuleReference(name="PLUS", is_token=True), RuleReference(name="COMMA", is_token=True), RuleReference(name="DOT", is_token=True), RuleReference(name="STAR", is_token=True), RuleReference(name="COLON", is_token=True), RuleReference(name="AMPERSAND", is_token=True), Sequence(elements=[RuleReference(name="FUNCTION", is_token=True), RuleReference(name="pseudo_class_args", is_token=False), RuleReference(name="RPAREN", is_token=True)]), Sequence(elements=[RuleReference(name="LBRACKET", is_token=True), RuleReference(name="pseudo_class_args", is_token=False), RuleReference(name="RBRACKET", is_token=True)])]),
        ),
        GrammarRule(
            name="pseudo_element",
            line_number=360,
            body=Sequence(elements=[RuleReference(name="COLON_COLON", is_token=True), RuleReference(name="IDENT", is_token=True)]),
        ),
        GrammarRule(
            name="block",
            line_number=370,
            body=Sequence(elements=[RuleReference(name="LBRACE", is_token=True), RuleReference(name="block_contents", is_token=False), RuleReference(name="RBRACE", is_token=True)]),
        ),
        GrammarRule(
            name="block_contents",
            line_number=372,
            body=Repetition(element=RuleReference(name="block_item", is_token=False)),
        ),
        GrammarRule(
            name="block_item",
            line_number=374,
            body=Alternation(choices=[RuleReference(name="lattice_block_item", is_token=False), RuleReference(name="at_rule", is_token=False), RuleReference(name="declaration_or_nested", is_token=False)]),
        ),
        GrammarRule(
            name="lattice_block_item",
            line_number=380,
            body=Alternation(choices=[RuleReference(name="variable_declaration", is_token=False), RuleReference(name="include_directive", is_token=False), RuleReference(name="lattice_control", is_token=False), RuleReference(name="content_directive", is_token=False), RuleReference(name="extend_directive", is_token=False), RuleReference(name="at_root_directive", is_token=False)]),
        ),
        GrammarRule(
            name="content_directive",
            line_number=390,
            body=Sequence(elements=[Literal(value="@content"), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="extend_directive",
            line_number=398,
            body=Sequence(elements=[Literal(value="@extend"), RuleReference(name="selector_list", is_token=False), RuleReference(name="SEMICOLON", is_token=True)]),
        ),
        GrammarRule(
            name="at_root_directive",
            line_number=403,
            body=Sequence(elements=[Literal(value="@at-root"), Group(element=Alternation(choices=[Sequence(elements=[RuleReference(name="selector_list", is_token=False), RuleReference(name="block", is_token=False)]), RuleReference(name="block", is_token=False)]))]),
        ),
        GrammarRule(
            name="declaration_or_nested",
            line_number=405,
            body=Alternation(choices=[RuleReference(name="declaration", is_token=False), RuleReference(name="qualified_rule", is_token=False)]),
        ),
        GrammarRule(
            name="declaration",
            line_number=414,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="property", is_token=False), RuleReference(name="COLON", is_token=True), RuleReference(name="value_list", is_token=False), OptGroup(element=RuleReference(name="priority", is_token=False)), RuleReference(name="SEMICOLON", is_token=True)]), Sequence(elements=[RuleReference(name="property", is_token=False), RuleReference(name="COLON", is_token=True), RuleReference(name="block", is_token=False)])]),
        ),
        GrammarRule(
            name="property",
            line_number=417,
            body=Alternation(choices=[RuleReference(name="IDENT", is_token=True), RuleReference(name="CUSTOM_PROPERTY", is_token=True)]),
        ),
        GrammarRule(
            name="priority",
            line_number=419,
            body=Sequence(elements=[RuleReference(name="BANG", is_token=True), Literal(value="important")]),
        ),
        GrammarRule(
            name="value_list",
            line_number=430,
            body=Sequence(elements=[RuleReference(name="value", is_token=False), Repetition(element=RuleReference(name="value", is_token=False))]),
        ),
        GrammarRule(
            name="value",
            line_number=432,
            body=Alternation(choices=[RuleReference(name="DIMENSION", is_token=True), RuleReference(name="PERCENTAGE", is_token=True), RuleReference(name="NUMBER", is_token=True), RuleReference(name="STRING", is_token=True), RuleReference(name="IDENT", is_token=True), RuleReference(name="HASH", is_token=True), RuleReference(name="CUSTOM_PROPERTY", is_token=True), RuleReference(name="UNICODE_RANGE", is_token=True), RuleReference(name="function_call", is_token=False), RuleReference(name="VARIABLE", is_token=True), RuleReference(name="SLASH", is_token=True), RuleReference(name="COMMA", is_token=True), RuleReference(name="PLUS", is_token=True), RuleReference(name="MINUS", is_token=True), RuleReference(name="map_literal", is_token=False)]),
        ),
        GrammarRule(
            name="function_call",
            line_number=438,
            body=Alternation(choices=[Sequence(elements=[RuleReference(name="FUNCTION", is_token=True), RuleReference(name="function_args", is_token=False), RuleReference(name="RPAREN", is_token=True)]), RuleReference(name="URL_TOKEN", is_token=True)]),
        ),
        GrammarRule(
            name="function_args",
            line_number=441,
            body=Repetition(element=RuleReference(name="function_arg", is_token=False)),
        ),
        GrammarRule(
            name="function_arg",
            line_number=443,
            body=Alternation(choices=[RuleReference(name="DIMENSION", is_token=True), RuleReference(name="PERCENTAGE", is_token=True), RuleReference(name="NUMBER", is_token=True), RuleReference(name="STRING", is_token=True), RuleReference(name="IDENT", is_token=True), RuleReference(name="HASH", is_token=True), RuleReference(name="CUSTOM_PROPERTY", is_token=True), RuleReference(name="COMMA", is_token=True), RuleReference(name="SLASH", is_token=True), RuleReference(name="PLUS", is_token=True), RuleReference(name="MINUS", is_token=True), RuleReference(name="STAR", is_token=True), RuleReference(name="VARIABLE", is_token=True), Sequence(elements=[RuleReference(name="FUNCTION", is_token=True), RuleReference(name="function_args", is_token=False), RuleReference(name="RPAREN", is_token=True)])]),
        ),
    ],
)
