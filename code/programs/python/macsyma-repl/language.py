"""``MacsymaLanguage`` — the language plugin for ``coding_adventures_repl``.

Wires together::

    user input  →  macsyma_lexer.tokenize
                →  macsyma_parser.parse
                →  macsyma_compiler.compile_macsyma(wrap_terminators=True)
                →  symbolic_vm.VM(macsyma_runtime.MacsymaBackend).eval
                →  cas_pretty_printer.pretty(.., MacsymaDialect())

For each top-level statement in the input, the language records the
input IR in :class:`History`, evaluates it, records the output, and
returns one combined string (one line per displayed result, or
``None`` if every statement was suppressed).
"""

from __future__ import annotations

import re

from cas_pretty_printer import MacsymaDialect, pretty
from coding_adventures_repl import Language
from macsyma_compiler import compile_macsyma
from macsyma_compiler.compiler import _STANDARD_FUNCTIONS
from macsyma_parser import parse_macsyma
from macsyma_runtime import History, MacsymaBackend, extend_compiler_name_table
from symbolic_ir import IRApply, IRNode, IRSymbol
from symbolic_vm import VM

# Extend the compiler's name table with the full MACSYMA vocabulary:
# factor, expand, simplify, solve, subst, limit, taylor, length, first, …
# This is idempotent (dict.update), so creating multiple MacsymaLanguage
# instances is safe. It must happen at module load time so the first
# call to compile_macsyma() already knows all MACSYMA function names.
extend_compiler_name_table(_STANDARD_FUNCTIONS)

_DIALECT = MacsymaDialect()
_QUIT_COMMANDS = frozenset({":quit", ":q", "quit", "quit()", "quit();", "quit;"})

# The macsyma lexer's NAME regex requires `%` to be followed by an
# identifier character (so `%pi`, `%e`, `%o3` lex as one NAME). Bare
# `%` — the user-facing shorthand for "the most recent output" — is
# not a valid NAME on its own. We rewrite it to ``%oN`` (where N is
# the current last-output index) before the lexer sees it.
_BARE_PERCENT_RE = re.compile(r"%(?![a-zA-Z0-9_])")


class MacsymaLanguage(Language):
    """REPL language plugin for the MACSYMA pipeline.

    Holds the persistent state that survives across evaluation turns:
    the :class:`History` table and the :class:`MacsymaBackend` (whose
    environment carries variable bindings and function definitions).

    Each call to :meth:`eval` may evaluate one *or several* top-level
    statements (a single user line can contain ``a:1$ a + 1;``). The
    return value is a single string suitable for display, or ``None``
    when every statement was suppressed with ``$``.
    """

    history: History
    backend: MacsymaBackend
    vm: VM

    def __init__(self) -> None:
        self.history = History()
        self.backend = MacsymaBackend(history=self.history)
        self.vm = VM(self.backend)

    # ------------------------------------------------------------------
    # Language protocol
    # ------------------------------------------------------------------

    def eval(self, input: str) -> tuple[str, str | None] | str:
        stripped = input.strip()
        if not stripped:
            return ("ok", None)
        if stripped.lower() in _QUIT_COMMANDS:
            return "quit"

        # Auto-append ``;`` so a line typed without a terminator still
        # parses (better UX than rejecting valid expressions).
        source = stripped if stripped.endswith((";", "$")) else stripped + ";"

        # Rewrite bare ``%`` (Maxima's "most recent output" shorthand)
        # to ``%oN`` where N is the index of the most recent output.
        # If there is no recent output yet, leave it alone — the lexer
        # will raise an "unexpected character" error which we surface.
        last_n = len(self.history.outputs)
        if last_n > 0:
            source = _BARE_PERCENT_RE.sub(f"%o{last_n}", source)

        try:
            ast = parse_macsyma(source)
            statements = compile_macsyma(ast, wrap_terminators=True)
        except Exception as exc:
            return ("error", f"parse error: {exc}")

        outputs: list[str] = []
        for stmt in statements:
            displayed, inner = _split_wrapper(stmt)
            try:
                result = self.vm.eval(inner)
            except Exception as exc:
                return ("error", f"runtime error: {exc}")
            self.history.record_input(inner)
            self.history.record_output(result)
            if displayed:
                idx = len(self.history.outputs)
                outputs.append(f"(%o{idx}) {pretty(result, _DIALECT)}")

        if not outputs:
            return ("ok", None)
        return ("ok", "\n".join(outputs))


def _split_wrapper(stmt: IRNode) -> tuple[bool, IRNode]:
    """Return ``(should_display, inner)`` from a Display/Suppress wrapper.

    Statements that aren't wrapped are treated as Display (best-effort —
    the user typed something the compiler returned without a wrapper,
    which shouldn't normally happen when ``wrap_terminators=True``).
    """
    if (
        isinstance(stmt, IRApply)
        and isinstance(stmt.head, IRSymbol)
        and len(stmt.args) == 1
    ):
        if stmt.head.name == "Display":
            return True, stmt.args[0]
        if stmt.head.name == "Suppress":
            return False, stmt.args[0]
    return True, stmt
