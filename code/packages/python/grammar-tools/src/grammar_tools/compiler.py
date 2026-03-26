# AUTO-GENERATED FILE - DO NOT EDIT
from __future__ import annotations

from typing import TYPE_CHECKING
if TYPE_CHECKING:
    from grammar_tools.token_grammar import TokenGrammar
    from grammar_tools.parser_grammar import ParserGrammar, GrammarElement

import json

def compile_tokens_to_python(grammar: "TokenGrammar", export_name: str) -> str:
    lines = []
    lines.append("# AUTO-GENERATED FILE - DO NOT EDIT")
    lines.append("from grammar_tools.token_grammar import TokenGrammar, TokenDefinition, PatternGroup")
    lines.append("")
    lines.append(f"{export_name} = TokenGrammar(")
    
    lines.append(f"    version={grammar.version},")
    lines.append(f"    case_insensitive={grammar.case_insensitive},")
    lines.append(f"    case_sensitive={grammar.case_sensitive},")
    
    if grammar.mode is not None:
        lines.append(f"    mode={json.dumps(grammar.mode)},")
    
    if grammar.escape_mode is not None:
        lines.append(f"    escape_mode={json.dumps(grammar.escape_mode)},")
        
    if grammar.keywords:
        lines.append(f"    keywords={json.dumps(grammar.keywords)},")
        
    if grammar.reserved_keywords:
        lines.append(f"    reserved_keywords={json.dumps(grammar.reserved_keywords)},")

    lines.append("    definitions=[")
    for defn in grammar.definitions:
        lines.append(f"        {_compile_token_def(defn)},")
    lines.append("    ],")

    if grammar.skip_definitions:
        lines.append("    skip_definitions=[")
        for defn in grammar.skip_definitions:
            lines.append(f"        {_compile_token_def(defn)},")
        lines.append("    ],")

    if grammar.error_definitions:
        lines.append("    error_definitions=[")
        for defn in grammar.error_definitions:
            lines.append(f"        {_compile_token_def(defn)},")
        lines.append("    ],")

    if grammar.groups:
        lines.append("    groups={")
        for gname, group in grammar.groups.items():
            lines.append(f"        {json.dumps(gname)}: PatternGroup(")
            lines.append(f"            name={json.dumps(group.name)},")
            lines.append("            definitions=[")
            for defn in group.definitions:
                lines.append(f"                {_compile_token_def(defn)},")
            lines.append("            ],")
            lines.append("        ),")
        lines.append("    },")

    lines.append(")")
    return "\n".join(lines) + "\n"

def _compile_token_def(defn) -> str:
    alias_str = f", alias={json.dumps(defn.alias)}" if defn.alias else ""
    return (
        f"TokenDefinition(name={json.dumps(defn.name)}, "
        f"pattern={json.dumps(defn.pattern)}, "
        f"is_regex={defn.is_regex}, "
        f"line_number={defn.line_number}{alias_str})"
    )

def compile_parser_to_python(grammar: "ParserGrammar", export_name: str) -> str:
    lines = []
    lines.append("# AUTO-GENERATED FILE - DO NOT EDIT")
    lines.append("from grammar_tools.parser_grammar import (")
    lines.append("    ParserGrammar, GrammarRule, GrammarElement,")
    lines.append("    RuleReference, Literal, Sequence, Alternation,")
    lines.append("    Repetition, Optional as OptGroup, Group")
    lines.append(")")
    lines.append("")
    lines.append(f"{export_name} = ParserGrammar(")
    lines.append(f"    version={grammar.version},")
    lines.append("    rules=[")
    for rule in grammar.rules:
        lines.append("        GrammarRule(")
        lines.append(f"            name={json.dumps(rule.name)},")
        lines.append(f"            line_number={rule.line_number},")
        lines.append(f"            body={_compile_grammar_element(rule.body)},")
        lines.append("        ),")
    lines.append("    ],")
    lines.append(")")
    return "\n".join(lines) + "\n"

def _compile_grammar_element(el: "GrammarElement") -> str:
    from grammar_tools.parser_grammar import (
        RuleReference, Literal, Sequence, Alternation,
        Repetition, Optional, Group
    )
    if isinstance(el, RuleReference):
        return f"RuleReference(name={json.dumps(el.name)}, is_token={el.is_token})"
    elif isinstance(el, Literal):
        return f"Literal(value={json.dumps(el.value)})"
    elif isinstance(el, Sequence):
        elems = ", ".join(_compile_grammar_element(e) for e in el.elements)
        return f"Sequence(elements=[{elems}])"
    elif isinstance(el, Alternation):
        choices = ", ".join(_compile_grammar_element(c) for c in el.choices)
        return f"Alternation(choices=[{choices}])"
    elif isinstance(el, Repetition):
        return f"Repetition(element={_compile_grammar_element(el.element)})"
    elif isinstance(el, Optional):
        return f"OptGroup(element={_compile_grammar_element(el.element)})"
    elif isinstance(el, Group):
        return f"Group(element={_compile_grammar_element(el.element)})"
    else:
        raise ValueError(f"Unknown element type: {type(el)}")
