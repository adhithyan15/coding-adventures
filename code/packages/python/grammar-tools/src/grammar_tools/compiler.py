"""
compiler.py — Compile TokenGrammar and ParserGrammar into Python source code.

The grammar-tools library is the reference implementation for .tokens and
.grammar file parsing. This module adds the *compile* step: given a parsed
grammar object, generate Python source code that embeds the grammar as native
Python data structures.

Why compile grammars?
---------------------

The default workflow reads .tokens and .grammar files at runtime:

    source = Path("json.tokens").read_text()
    grammar = parse_token_grammar(source)

This works fine during development, but it has a cost:

1. **File I/O at startup** — every process that needs the grammar must find
   and open the files. Packages walk up the directory tree to find
   ``code/grammars/``, which is a side-channel coupling to the repo layout.

2. **Parse overhead at startup** — the grammar must be re-parsed on every
   run, even though the grammar never changes between runs.

3. **Deployment coupling** — the .tokens and .grammar files must ship with
   the program, not just the compiled code.

Compiling a grammar to Python eliminates all three issues. The generated
file is a plain Python module that directly instantiates ``TokenGrammar`` or
``ParserGrammar`` with literal data — no I/O, no parsing, no file paths.

The generated code looks like this (json.tokens → json_tokens.py)::

    # AUTO-GENERATED FILE — DO NOT EDIT
    # Source: json.tokens
    from grammar_tools.token_grammar import TokenDefinition, PatternGroup, TokenGrammar

    TOKEN_GRAMMAR = TokenGrammar(
        version=1,
        case_sensitive=True,
        definitions=[
            TokenDefinition(name="STRING", pattern=..., is_regex=True, ...),
            ...
        ],
        ...
    )

The downstream package then does::

    from .generated.json_tokens import TOKEN_GRAMMAR

No file paths, no parsing, no directory walking.

Design principles
-----------------

- **Round-trip fidelity**: compiling then loading gives back an object
  equivalent to parsing the original file. All fields are preserved.
- **Human-readable output**: the generated code is formatted with
  indentation and meaningful names, not minified.
- **Literate**: the header comment in each generated file explains what it
  is and how to regenerate it.
"""

from __future__ import annotations

from grammar_tools.parser_grammar import (
    Alternation,
    GrammarElement,
    GrammarRule,
    Group,
    Literal,
    NegativeLookahead,
    OneOrMoreRepetition,
    Optional,
    ParserGrammar,
    PositiveLookahead,
    Repetition,
    RuleReference,
    SeparatedRepetition,
    Sequence,
)
from grammar_tools.token_grammar import PatternGroup, TokenDefinition, TokenGrammar

# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------

# Indentation constant — 4 spaces, following PEP 8.
_I = "    "


def _repr_str(value: str) -> str:
    """Return a safe repr for a string value.

    Uses Python's built-in repr() which handles all escaping correctly —
    backslashes, quotes, control characters. We strip the surrounding
    single quotes that repr() adds and re-wrap in double quotes for
    consistency with the rest of the codebase.

    Example::
        _repr_str('hello')   → '"hello"'
        _repr_str('a\\nb')   → '"a\\\\nb"'
        _repr_str('say "hi"') → '"say \\"hi\\""'
    """
    # repr("hello") → "'hello'" — we swap the outer delimiter to " for
    # consistency and because most grammar patterns don't contain " but
    # may contain '.
    r = repr(value)
    # repr() sometimes uses double quotes (if value contains '), fall through.
    return r


def _repr_opt_str(value: str | None) -> str:
    """Return repr for an optional string (None or a str)."""
    if value is None:
        return "None"
    return _repr_str(value)


# ---------------------------------------------------------------------------
# TokenGrammar compilation
# ---------------------------------------------------------------------------


def _compile_token_definition(defn: TokenDefinition, indent: str) -> str:
    """Render one TokenDefinition as a constructor call.

    Example output (inside a list)::

        TokenDefinition(
            name="STRING",
            pattern='"([^"\\\\]|\\\\[...]*"',
            is_regex=True,
            line_number=25,
            alias=None,
        ),
    """
    i1 = indent + _I
    lines = [
        f"{indent}TokenDefinition(",
        f"{i1}name={_repr_str(defn.name)},",
        f"{i1}pattern={_repr_str(defn.pattern)},",
        f"{i1}is_regex={defn.is_regex!r},",
        f"{i1}line_number={defn.line_number!r},",
        f"{i1}alias={_repr_opt_str(defn.alias)},",
        f"{indent}),",
    ]
    return "\n".join(lines)


def _compile_token_def_list(defs: list[TokenDefinition], indent: str) -> str:
    """Render a list of TokenDefinitions inside square brackets.

    If the list is empty, returns ``[]``.
    If non-empty, renders as a multi-line bracketed list.
    """
    if not defs:
        return "[]"
    inner = indent + _I
    items = "\n".join(_compile_token_definition(d, inner) for d in defs)
    return f"[\n{items}\n{indent}]"


def _compile_pattern_group(group: PatternGroup, indent: str) -> str:
    """Render one PatternGroup as a constructor call (value in the dict)."""
    i1 = indent + _I
    defs_src = _compile_token_def_list(group.definitions, i1)
    lines = [
        f"{indent}PatternGroup(",
        f"{i1}name={_repr_str(group.name)},",
        f"{i1}definitions={defs_src},",
        f"{indent}),",
    ]
    return "\n".join(lines)


def compile_token_grammar(grammar: TokenGrammar, source_file: str = "") -> str:
    """Generate Python source code that embeds *grammar* as a ``TokenGrammar``.

    Args:
        grammar: A parsed (and ideally validated) ``TokenGrammar`` to compile.
        source_file: The original .tokens filename, used only in the header
            comment. Pass an empty string to omit the source reference.

    Returns:
        A string of valid Python source code. Write it to a ``.py`` file.

    Example::

        grammar = parse_token_grammar(Path("json.tokens").read_text())
        code = compile_token_grammar(grammar, "json.tokens")
        Path("json_tokens.py").write_text(code)
    """
    # Strip newlines so a crafted filename cannot break out of the comment line
    # and inject arbitrary statements into the generated file.
    source_file = source_file.replace("\n", "_").replace("\r", "_")
    source_line = f"# Source: {source_file}\n" if source_file else ""

    # Build the groups dict source. The dict maps str → PatternGroup.
    if grammar.groups:
        i1 = _I + _I  # two levels for dict values
        group_entries = []
        for name, group in grammar.groups.items():
            group_src = _compile_pattern_group(group, i1)
            group_entries.append(f"{_I * 2}{_repr_str(name)}: {group_src.lstrip()}")
        groups_src = "{\n" + ",\n".join(group_entries) + f"\n{_I}}}"
    else:
        groups_src = "{}"

    # Build the full TokenGrammar constructor.
    i1 = _I
    defs_src = _compile_token_def_list(grammar.definitions, i1)
    skip_src = _compile_token_def_list(grammar.skip_definitions, i1)
    err_src = _compile_token_def_list(grammar.error_definitions, i1)

    body = "\n".join([
        "TOKEN_GRAMMAR = TokenGrammar(",
        f"{i1}version={grammar.version!r},",
        f"{i1}case_insensitive={grammar.case_insensitive!r},",
        f"{i1}case_sensitive={grammar.case_sensitive!r},",
        f"{i1}definitions={defs_src},",
        f"{i1}keywords={grammar.keywords!r},",
        f"{i1}mode={_repr_opt_str(grammar.mode)},",
        f"{i1}escape_mode={_repr_opt_str(grammar.escape_mode)},",
        f"{i1}skip_definitions={skip_src},",
        f"{i1}reserved_keywords={grammar.reserved_keywords!r},",
        f"{i1}error_definitions={err_src},",
        f"{i1}groups={groups_src},",
        f"{i1}layout_keywords={grammar.layout_keywords!r},",
        ")",
    ])

    return (
        "# AUTO-GENERATED FILE — DO NOT EDIT\n"
        "# ruff: noqa: E501, F401\n"
        f"{source_line}"
        "# Regenerate with: grammar-tools compile-tokens <source.tokens>\n"
        "#\n"
        "# This file embeds a TokenGrammar as native Python data structures.\n"
        "# Downstream packages import TOKEN_GRAMMAR directly instead of\n"
        "# reading and parsing the .tokens file at runtime.\n"
        "\n"
        "from grammar_tools.token_grammar import PatternGroup, TokenDefinition, TokenGrammar\n"  # noqa: E501
        "\n"
        "# fmt: off  # noqa: E501 — generated code may have long lines\n"
        "\n"
        f"{body}\n"
    )


# ---------------------------------------------------------------------------
# ParserGrammar compilation
# ---------------------------------------------------------------------------


def _compile_element(element: GrammarElement, indent: str) -> str:
    """Recursively render a grammar element as a constructor expression.

    The grammar element types form a tree — Sequence contains elements,
    Alternation contains choices, Repetition/Optional/Group wrap a child.
    We recurse depth-first to render the full tree.

    Indentation grows by one level (_I) for each level of nesting to keep
    the output readable.
    """
    i1 = indent + _I
    match element:
        case RuleReference(name=name, is_token=is_token):
            return (
                f"{indent}RuleReference(name={_repr_str(name)}, "
                f"is_token={is_token!r}),"
            )

        case Literal(value=value):
            return f"{indent}Literal(value={_repr_str(value)}),"

        case Sequence(elements=elements):
            elems_src = "\n".join(
                _compile_element(e, i1) for e in elements
            )
            return "\n".join([
                f"{indent}Sequence(elements=[",
                elems_src,
                f"{indent}]),",
            ])

        case Alternation(choices=choices):
            choices_src = "\n".join(
                _compile_element(c, i1) for c in choices
            )
            return "\n".join([
                f"{indent}Alternation(choices=[",
                choices_src,
                f"{indent}]),",
            ])

        case Repetition(element=child):
            child_src = _compile_element(child, i1)
            return "\n".join([
                f"{indent}Repetition(element=",
                child_src,
                f"{indent}),",
            ])

        case Optional(element=child):
            child_src = _compile_element(child, i1)
            return "\n".join([
                f"{indent}Optional(element=",
                child_src,
                f"{indent}),",
            ])

        case Group(element=child):
            child_src = _compile_element(child, i1)
            return "\n".join([
                f"{indent}Group(element=",
                child_src,
                f"{indent}),",
            ])

        case PositiveLookahead(element=child):
            child_src = _compile_element(child, i1)
            return "\n".join([
                f"{indent}PositiveLookahead(element=",
                child_src,
                f"{indent}),",
            ])

        case NegativeLookahead(element=child):
            child_src = _compile_element(child, i1)
            return "\n".join([
                f"{indent}NegativeLookahead(element=",
                child_src,
                f"{indent}),",
            ])

        case OneOrMoreRepetition(element=child):
            child_src = _compile_element(child, i1)
            return "\n".join([
                f"{indent}OneOrMoreRepetition(element=",
                child_src,
                f"{indent}),",
            ])

        case SeparatedRepetition(
            element=elem, separator=sep, at_least_one=at_least_one
        ):
            elem_src = _compile_element(elem, i1)
            sep_src = _compile_element(sep, i1)
            return "\n".join([
                f"{indent}SeparatedRepetition(",
                f"{i1}element=",
                elem_src,
                f"{i1}separator=",
                sep_src,
                f"{i1}at_least_one={at_least_one!r},",
                f"{indent}),",
            ])

        case _:
            raise TypeError(f"Unknown grammar element type: {type(element)}")


def _compile_grammar_rule(rule: GrammarRule, indent: str) -> str:
    """Render one GrammarRule as a constructor call."""
    i1 = indent + _I
    body_src = _compile_element(rule.body, i1)
    return "\n".join([
        f"{indent}GrammarRule(",
        f"{i1}name={_repr_str(rule.name)},",
        f"{i1}body=",
        body_src,
        f"{i1}line_number={rule.line_number!r},",
        f"{indent}),",
    ])


def compile_parser_grammar(grammar: ParserGrammar, source_file: str = "") -> str:
    """Generate Python source code that embeds *grammar* as a ``ParserGrammar``.

    Args:
        grammar: A parsed (and ideally validated) ``ParserGrammar`` to compile.
        source_file: The original .grammar filename, used only in the header
            comment. Pass an empty string to omit the source reference.

    Returns:
        A string of valid Python source code. Write it to a ``.py`` file.

    Example::

        grammar = parse_parser_grammar(Path("json.grammar").read_text())
        code = compile_parser_grammar(grammar, "json.grammar")
        Path("json_parser.py").write_text(code)
    """
    # Strip newlines so a crafted filename cannot break out of the comment line.
    source_file = source_file.replace("\n", "_").replace("\r", "_")
    source_line = f"# Source: {source_file}\n" if source_file else ""

    i1 = _I
    i2 = _I + _I
    if grammar.rules:
        rule_items = "\n".join(_compile_grammar_rule(r, i2) for r in grammar.rules)
        rules_src = f"[\n{rule_items}\n{i1}]"
    else:
        rules_src = "[]"

    body = "\n".join([
        "PARSER_GRAMMAR = ParserGrammar(",
        f"{i1}version={grammar.version!r},",
        f"{i1}rules={rules_src},",
        ")",
    ])

    return (
        "# AUTO-GENERATED FILE — DO NOT EDIT\n"
        "# ruff: noqa: E501, F401\n"
        f"{source_line}"
        "# Regenerate with: grammar-tools compile-grammar <source.grammar>\n"
        "#\n"
        "# This file embeds a ParserGrammar as native Python data structures.\n"
        "# Downstream packages import PARSER_GRAMMAR directly instead of\n"
        "# reading and parsing the .grammar file at runtime.\n"
        "\n"
        "from grammar_tools.parser_grammar import (\n"
        "    Alternation,\n"
        "    GrammarRule,\n"
        "    Group,\n"
        "    Literal,\n"
        "    NegativeLookahead,\n"
        "    OneOrMoreRepetition,\n"
        "    Optional,\n"
        "    ParserGrammar,\n"
        "    PositiveLookahead,\n"
        "    Repetition,\n"
        "    RuleReference,\n"
        "    SeparatedRepetition,\n"
        "    Sequence,\n"
        ")\n"
        "\n"
        "# fmt: off  # noqa: E501 — generated code may have long lines\n"
        "\n"
        f"{body}\n"
    )
