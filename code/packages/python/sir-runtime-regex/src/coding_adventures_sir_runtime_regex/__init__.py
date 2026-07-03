"""coding-adventures-sir-runtime-regex — the SIR ``regex`` builtin for Python.

The Ruby→SIR frontend lowers a regex literal ``/pat/flags`` to
``BuiltinCall("regex", [StrLit pattern, StrLit flags])``.  This package is the
Python runtime for that builtin: it compiles the pattern with the standard
:mod:`re` engine while translating Ruby's flag spellings and line-anchor
conventions::

    from coding_adventures_sir_runtime_regex import compile, is_match, match_data
    pat = compile(r"\\d+", "i")     # Ruby /\\d+/i  → re.Pattern
    is_match(pat, "abc 42")          # True  (unanchored search, like Ruby =~)
    match_data(r"\\d+", "abc 42")    # "42"  (group 0, or None on no match)

Ruby's regex dialect differs from Python ``re`` chiefly in how inline flags are
spelled (Ruby ``/m`` is Python ``DOTALL``, not ``MULTILINE``) and in that
``^`` / ``$`` are *always* line anchors — see :mod:`coding_adventures_sir_runtime_regex.regex`
for the full mapping table and the ``\\A`` / ``\\z`` nuance.

Note :func:`compile` deliberately shadows :func:`builtins.compile` /
:func:`re.compile`: ``regex`` is the SIR builtin name and emitted code addresses
this package's ``compile`` by qualified name, so the shadow is intentional and
harmless.

See ``code/specs/sir-runtime.md``.
"""

from __future__ import annotations

from .regex import Val, compile, is_match, match_data

__all__ = [
    "Val",
    "compile",
    "is_match",
    "match_data",
]
