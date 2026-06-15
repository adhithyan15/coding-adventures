"""Regular expressions — the SIR ``regex`` builtin (Ruby dialect on Python ``re``).

The Ruby→SIR frontend lowers a regex literal ``/pat/flags`` to
``BuiltinCall("regex", [StrLit pattern, StrLit flags])``.  This module is the
*Python* landing point for that builtin: it compiles the pattern with Python's
standard :mod:`re` engine, translating Ruby's flag and anchoring conventions so
the compiled object behaves the way the Ruby source intended.

**Why a translation layer at all — Ruby's dialect differs from Python's.**
Both engines descend from Perl-compatible regular expressions, so the *syntax*
(``\\d``, ``[a-z]``, ``(?:...)``, ``a|b``, ``+ * ?``) is almost entirely
shared.  Where they diverge is in **how the inline flags are spelled** and **what
the line anchors ``^`` / ``$`` mean by default**:

================  =====================  ==========================================
Ruby flag char    Python ``re`` flag     Meaning
================  =====================  ==========================================
``i``             :data:`re.IGNORECASE`  Case-insensitive matching.
``m``             :data:`re.DOTALL`      Ruby ``/m`` ("multiline") makes ``.`` match
                                         a newline.  This is Python's *DOTALL*, **not**
                                         Python's :data:`re.MULTILINE`.  This is the
                                         classic foot-gun: the same letter means
                                         different things in the two engines.
``x``             :data:`re.VERBOSE`     "Extended" mode — unescaped whitespace and
                                         ``#`` comments in the pattern are ignored.
(any other)       — (ignored)            Unknown flag characters are silently dropped.
================  =====================  ==========================================

**The ``^`` / ``$`` nuance.**  In Ruby, ``^`` and ``$`` *always* anchor to the
start/end of a **line**, not the whole string — there is no flag to turn that
off; the whole-string anchors are the separate escapes ``\\A`` (start of string)
and ``\\z`` / ``\\Z`` (end of string).  In Python the default is the opposite:
``^`` / ``$`` anchor to the *whole string* unless :data:`re.MULTILINE` is set.
To make a Ruby pattern behave faithfully we therefore **always OR in**
:data:`re.MULTILINE`, regardless of the flag string.  (``\\A`` / ``\\z`` carry
over unchanged, so whole-string anchoring is still available to the pattern
author.)

**Match semantics.**  Ruby's ``=~`` and ``String#match?`` perform an *unanchored
search* — a hit anywhere in the string counts — not a full-string match.  The
:func:`is_match` and :func:`match_data` helpers below therefore use
:func:`re.Pattern.search`, never ``fullmatch``.
"""

from __future__ import annotations

import re
from typing import Any

# The SIR universal value type at this package's boundary.  A "pattern" handed
# to the match helpers may be either a precompiled :class:`re.Pattern` or a raw
# pattern string, so we accept the widest type and narrow internally.
Val = Any


def compile(pattern: str, flags: str = "") -> re.Pattern[str]:
    """Compile a Ruby-dialect regex ``pattern`` under Ruby-dialect ``flags``.

    ``flags`` is the inline-flag *string* exactly as it trails a Ruby literal —
    e.g. the ``"imx"`` of ``/.../imx``.  Each recognised character contributes a
    Python :mod:`re` flag per the truth table in the module docstring; unknown
    characters are ignored.  :data:`re.MULTILINE` is *always* included so that
    Ruby's line-anchored ``^`` / ``$`` behave correctly (see the module
    docstring's ``\\A`` / ``\\z`` note).

    Note this function intentionally shadows the builtin :func:`compile` and
    :func:`re.compile`: ``regex`` is the SIR builtin's name, and this module is
    addressed as ``coding_adventures_sir_runtime_regex.compile`` by emitted
    code, so the shadow never bites a caller.
    """
    # Map of Ruby inline-flag characters to their Python re equivalents.  Built
    # as a dict (not a chain of ifs) so the table reads like the docstring's.
    mapping = {
        "i": re.IGNORECASE,
        "m": re.DOTALL,  # Ruby /m == dot-matches-newline == Python DOTALL.
        "x": re.VERBOSE,
    }
    # Ruby's ^/$ are always line anchors, so MULTILINE is unconditional.
    py_flags = re.MULTILINE
    for ch in flags:
        # Unknown characters contribute nothing (mapping.get default 0).
        py_flags |= mapping.get(ch, 0)
    return re.compile(pattern, py_flags)


def _compiled(pattern: Any) -> re.Pattern[str]:
    """Coerce ``pattern`` to a compiled :class:`re.Pattern`.

    Accepts either an already-compiled pattern (returned untouched, so a
    caller's flags are preserved) or a raw value compiled on the spot with the
    default Ruby flag set (empty string ⇒ just the unconditional
    :data:`re.MULTILINE`).
    """
    if isinstance(pattern, re.Pattern):
        return pattern
    return compile(str(pattern))


def is_match(pattern: Any, string: str) -> bool:
    """True iff ``pattern`` matches anywhere in ``string`` (Ruby ``=~`` / ``match?``).

    This is an *unanchored search*, mirroring Ruby semantics — a hit anywhere in
    the string counts, not a full-string match.  ``pattern`` may be a compiled
    :class:`re.Pattern` or a raw pattern string.
    """
    return _compiled(pattern).search(string) is not None


def match_data(pattern: Any, string: str) -> str | None:
    """Return the matched substring (group 0), or ``None`` if there is no match.

    A minimal model of Ruby's ``String#match``: Ruby returns a truthy
    ``MatchData`` (whose ``[0]`` is the matched text) or ``nil``.  Here we expose
    just the matched substring, which is the most common thing emitted code
    needs, and ``None`` for the no-match (``nil``) case.
    """
    m = _compiled(pattern).search(string)
    return m.group(0) if m is not None else None
