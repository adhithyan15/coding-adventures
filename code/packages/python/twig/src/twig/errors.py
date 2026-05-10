"""Errors raised by the Twig front-end and runtime.

Why a layered hierarchy
-----------------------
Twig has three distinct failure surfaces:

* **Parse-time**  — malformed source: bad tokens, unmatched parens,
  unsupported constructs.  These never reach the compiler.
* **Compile-time** — well-formed source but unsupported semantics:
  free-variable references that can't be resolved, malformed
  ``define`` / ``lambda`` headers, etc.
* **Runtime**     — execution-time problems: type errors,
  out-of-bounds heap handles, division by zero.

Tools (an LSP, a notebook kernel, a REPL) want to handle these
classes differently — for example, a parser error has a source
location while a runtime error has a value and a frame.  Giving each
its own exception class with a stable identity keeps that easy.

All three inherit from :class:`TwigError` so callers that just want
"something went wrong with the user's program" can catch one.
"""

from __future__ import annotations


class TwigError(Exception):
    """Base class for every Twig-level failure."""


class TwigParseError(TwigError):
    """Source text could not be parsed.

    Carries an optional ``line`` / ``column`` for editor integrations.
    """

    def __init__(
        self,
        message: str,
        *,
        line: int | None = None,
        column: int | None = None,
    ) -> None:
        if line is not None and column is not None:
            super().__init__(f"{message} (line {line}, col {column})")
        else:
            super().__init__(message)
        self.line = line
        self.column = column


class TwigCompileError(TwigError):
    """Twig source is well-formed but cannot be compiled to IIR."""


class TwigRuntimeError(TwigError):
    """A Twig program raised an error during execution."""


class TwigExitRequest(TwigError):
    """Raised by ``host/exit`` instead of calling ``sys.exit`` directly.

    Using a domain-specific exception instead of ``sys.exit`` ensures
    that an embedded ``TwigVM`` (inside a REPL, a language server, a
    test harness, or a web service) does not kill the host Python
    process unconditionally.  CLI entry points that want the standard
    "exit the process" behaviour should catch this and forward:

    .. code-block:: python

        try:
            vm.run(source)
        except TwigExitRequest as e:
            sys.exit(e.code)

    Attributes
    ----------
    code:
        The integer exit code the Twig program requested.
    """

    def __init__(self, code: int) -> None:
        super().__init__(f"exit({code})")
        self.code = code
