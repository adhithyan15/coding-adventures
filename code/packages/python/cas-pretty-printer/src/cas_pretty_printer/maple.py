"""Maple dialect.

Surface syntax differences from MACSYMA:

- Power can be ``^`` or ``**`` — output uses ``^``.
- Function names use Maple conventions (``diff`` not ``D``).

Otherwise mostly identical to MACSYMA: parens for calls, square
brackets for lists, lowercase function names.
"""

from __future__ import annotations

from cas_pretty_printer.macsyma import MacsymaDialect


class MapleDialect(MacsymaDialect):
    """Maple flavor — same spellings as MACSYMA for everything we cover so far."""

    name = "maple"
    # Maple uses `<>` for not-equal but accepts `#` like MACSYMA in some
    # contexts; use `<>` for output.
    binary_ops = {**MacsymaDialect.binary_ops, "NotEqual": " <> "}
