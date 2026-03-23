"""Verilog Preprocessor — expands macros and conditionals before tokenization.

The Verilog preprocessor is a simplified version of the C preprocessor.
It processes directives that start with a backtick (`) and operates on
raw source text before the lexer sees it.

Supported Directives
--------------------

`` `define NAME value ``
    Define a text macro. Every subsequent occurrence of `` `NAME `` in
    the source is replaced with ``value``.

`` `define NAME(a, b) expression ``
    Define a parameterized macro. Occurrences of `` `NAME(x, y) `` are
    replaced with ``expression`` where ``a`` and ``b`` are substituted
    with ``x`` and ``y``.

`` `undef NAME ``
    Remove a previously defined macro.

`` `ifdef NAME `` / `` `ifndef NAME ``
    Conditional compilation. If ``NAME`` is defined (or not defined),
    include the following lines. Otherwise, skip them.

`` `else ``
    Flip the current conditional — include lines that were being skipped,
    and vice versa.

`` `endif ``
    End a conditional block.

`` `include "filename" ``
    File inclusion. Currently stubbed — emits a warning comment and
    removes the directive. Full file inclusion requires a file resolver
    callback, which is deferred to a future spec.

`` `timescale unit/precision ``
    Time unit specification. Stripped from the source (no semantic meaning
    for synthesis or parsing).

Design as a Stepping Stone
--------------------------

This preprocessor is intentionally structured as a clean, extractable
module. When C preprocessor support is added later, the core logic
(macro table, condition stack, text substitution) can be extracted into
a shared ``preprocessor`` package. The Verilog preprocessor would then
become a thin configuration of that shared engine.

The key differences between Verilog and C preprocessors:

    +-----------------------+-------------------+-------------------+
    | Feature               | Verilog           | C                 |
    +-----------------------+-------------------+-------------------+
    | Directive prefix      | ` (backtick)      | # (hash)          |
    | Macro reference       | `NAME             | NAME              |
    | Token pasting (##)    | No                | Yes               |
    | Stringification (#)   | No                | Yes               |
    | Variadic macros       | No                | Yes (__VA_ARGS__) |
    | #pragma / #error      | No                | Yes               |
    +-----------------------+-------------------+-------------------+

Verilog is strictly a subset of C's preprocessor capabilities.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field


@dataclass
class MacroDef:
    """A macro definition — either simple or parameterized.

    Simple macro:
        `define WIDTH 8
        → MacroDef(name="WIDTH", body="8", params=None)

    Parameterized macro:
        `define MAX(a, b) ((a) > (b) ? (a) : (b))
        → MacroDef(name="MAX", body="((a) > (b) ? (a) : (b))",
                   params=["a", "b"])
    """

    name: str
    body: str
    params: list[str] | None = None


# ---------------------------------------------------------------------------
# Regex patterns for directive parsing
# ---------------------------------------------------------------------------

# Matches: `define NAME(params) body
# or:      `define NAME body
_DEFINE_WITH_PARAMS = re.compile(
    r"^\s*`define\s+([a-zA-Z_]\w*)\(([^)]*)\)\s*(.*)"
)
_DEFINE_SIMPLE = re.compile(
    r"^\s*`define\s+([a-zA-Z_]\w*)\s*(.*)"
)
_UNDEF = re.compile(r"^\s*`undef\s+([a-zA-Z_]\w*)")
_IFDEF = re.compile(r"^\s*`ifdef\s+([a-zA-Z_]\w*)")
_IFNDEF = re.compile(r"^\s*`ifndef\s+([a-zA-Z_]\w*)")
_ELSE = re.compile(r"^\s*`else\b")
_ENDIF = re.compile(r"^\s*`endif\b")
_INCLUDE = re.compile(r'^\s*`include\s+"([^"]*)"')
_TIMESCALE = re.compile(r"^\s*`timescale\b.*")

# Matches macro references: `NAME or `NAME(args)
_MACRO_REF = re.compile(r"`([a-zA-Z_]\w*)")
_MACRO_CALL = re.compile(r"`([a-zA-Z_]\w*)\(")


def verilog_preprocess(
    source: str,
    predefined: dict[str, str] | None = None,
) -> str:
    """Process Verilog preprocessor directives.

    This function is designed to be used as a ``pre_tokenize`` hook on the
    ``GrammarLexer``. It takes raw Verilog source text and returns
    preprocessed text with macros expanded and conditionals resolved.

    Args:
        source: Raw Verilog source code.
        predefined: Optional dictionary of predefined macros
            (name → value). Useful for passing in ``+define+`` flags.

    Returns:
        Preprocessed source code with directives resolved.

    Line Number Preservation
    ------------------------

    When lines are excluded by conditionals or stripped (``timescale``,
    ``include``), they are replaced with empty strings. This preserves
    line numbers so that error messages from the lexer/parser point to
    the correct location in the original source.

    Example::

        source = '''
        `define WIDTH 8
        wire [`WIDTH-1:0] data;
        '''
        result = verilog_preprocess(source)
        # result contains: wire [8-1:0] data;
    """
    macros: dict[str, MacroDef] = {}
    if predefined:
        for name, value in predefined.items():
            macros[name] = MacroDef(name=name, body=value)

    # Condition stack: True = include current section, False = skip.
    # Starts with True (unconditional inclusion).
    condition_stack: list[bool] = [True]

    lines = source.split("\n")
    result: list[str] = []

    for line in lines:
        # Check if we're in an active (included) section
        active = all(condition_stack)

        # --- Conditional directives (always processed, even when inactive) ---

        ifdef_match = _IFDEF.match(line)
        if ifdef_match:
            name = ifdef_match.group(1)
            if active:
                condition_stack.append(name in macros)
            else:
                # Nested conditional inside inactive section — push False
                condition_stack.append(False)
            result.append("")  # Preserve line number
            continue

        ifndef_match = _IFNDEF.match(line)
        if ifndef_match:
            name = ifndef_match.group(1)
            if active:
                condition_stack.append(name not in macros)
            else:
                condition_stack.append(False)
            result.append("")
            continue

        if _ELSE.match(line):
            if len(condition_stack) > 1:
                # Only flip if the parent section is active
                parent_active = all(condition_stack[:-1])
                if parent_active:
                    condition_stack[-1] = not condition_stack[-1]
            result.append("")
            continue

        if _ENDIF.match(line):
            if len(condition_stack) > 1:
                condition_stack.pop()
            result.append("")
            continue

        # --- Skip inactive lines ---

        if not active:
            result.append("")
            continue

        # --- `define ---

        define_params_match = _DEFINE_WITH_PARAMS.match(line)
        if define_params_match:
            name = define_params_match.group(1)
            params = [p.strip() for p in define_params_match.group(2).split(",")]
            body = define_params_match.group(3).strip()
            macros[name] = MacroDef(name=name, body=body, params=params)
            result.append("")
            continue

        define_match = _DEFINE_SIMPLE.match(line)
        if define_match:
            name = define_match.group(1)
            body = define_match.group(2).strip()
            macros[name] = MacroDef(name=name, body=body)
            result.append("")
            continue

        # --- `undef ---

        undef_match = _UNDEF.match(line)
        if undef_match:
            name = undef_match.group(1)
            macros.pop(name, None)
            result.append("")
            continue

        # --- `include (stubbed) ---

        include_match = _INCLUDE.match(line)
        if include_match:
            filename = include_match.group(1)
            result.append(f"/* `include \"{filename}\" — not resolved */")
            continue

        # --- `timescale (stripped) ---

        if _TIMESCALE.match(line):
            result.append("")
            continue

        # --- Macro expansion ---

        expanded = _expand_macros(line, macros)
        result.append(expanded)

    return "\n".join(result)


def _expand_macros(line: str, macros: dict[str, MacroDef]) -> str:
    """Expand macro references in a single line.

    Handles both simple macros (`` `WIDTH `` → ``8``) and parameterized
    macros (`` `MAX(a, b) `` → ``((a) > (b) ? (a) : (b))``).

    Expansion is single-pass to avoid infinite loops from recursive macros.
    """
    # Process parameterized macros first (they contain parentheses).
    # We search left-to-right, expanding each reference once.
    pos = 0
    result_parts: list[str] = []

    while pos < len(line):
        # Look for backtick-prefixed identifier
        match = _MACRO_REF.search(line, pos)
        if not match:
            result_parts.append(line[pos:])
            break

        # Add text before the macro reference
        result_parts.append(line[pos:match.start()])
        name = match.group(1)

        if name not in macros:
            # Not a defined macro — keep the reference as-is
            result_parts.append(match.group(0))
            pos = match.end()
            continue

        macro = macros[name]

        if macro.params is not None:
            # Parameterized macro — look for opening paren
            call_match = _MACRO_CALL.match(line, match.start())
            if call_match:
                # Find matching closing paren (handle nested parens)
                args_start = call_match.end()
                args_str, end_pos = _extract_macro_args(line, args_start)
                args = [a.strip() for a in args_str.split(",")]

                # Substitute parameters in the macro body
                body = macro.body
                for param, arg in zip(macro.params, args):
                    body = body.replace(param, arg)

                result_parts.append(body)
                pos = end_pos
            else:
                # Parameterized macro referenced without args — keep as-is
                result_parts.append(match.group(0))
                pos = match.end()
        else:
            # Simple macro — direct text substitution
            result_parts.append(macro.body)
            pos = match.end()

    return "".join(result_parts)


def _extract_macro_args(text: str, start: int) -> tuple[str, int]:
    """Extract macro arguments from ``start`` (after opening paren) to closing paren.

    Handles nested parentheses so that ``MAX((a+b), c)`` correctly
    extracts ``(a+b)`` and ``c`` as two arguments.

    Returns:
        A tuple of (argument_string, position_after_closing_paren).
    """
    depth = 1
    pos = start
    while pos < len(text) and depth > 0:
        ch = text[pos]
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        pos += 1
    # pos is now one past the closing paren
    return text[start:pos - 1], pos
