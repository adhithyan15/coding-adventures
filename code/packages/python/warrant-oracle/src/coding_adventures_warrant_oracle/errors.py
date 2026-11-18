"""Stable, safe failure type for the Warrant oracle.

Every consumer of hostile input in this package (the source reader, the IR
evaluator, the corpus reader) raises :class:`OracleError` and nothing else for
*expected* failures.  An error carries a short machine-readable ``code`` and a
message that never echoes input bytes (VER04r section 14: "no reproducing
material in public output").

Codes
-----
``source``      the file is not admissible source (size, encoding, NUL, BOM)
``lex``         a token could not be read
``parse``       the token stream is not a Warrant document
``type``        a well-formed document does not type check
``level``       a construct belongs to an IR feature level this oracle does not
                implement (VER04r section 10.4: never skipped, always named)
``limit``       a resource bound was exceeded (VER04r section 14)
``ir``          a malformed IR tree
``ir-overflow`` an exact ``Nat``/``Int`` left the 128-bit window (section 3.2)
``generation``  a generation-time refusal (the Rust generator's ``W0701`` family)
``corpus``      a corpus document disagrees with the oracle (a gate failure)
"""

from __future__ import annotations


class OracleError(Exception):
    """A named oracle failure; ``code`` is stable, ``message`` is safe to print."""

    def __init__(self, code: str, message: str, level: str | None = None) -> None:
        super().__init__(message)
        self.code = code
        self.message = message
        #: For ``code == "level"``: the IR level (``ir.fixed``, ``ir.data`` ...)
        #: that the unimplemented construct belongs to.
        self.level = level

    def __str__(self) -> str:
        return f"{self.code}: {self.message}"


class SizeError(OracleError):
    """A ``Bytes`` value past ``MAX_IR_VALUE`` (``ir-overflow``, checked before the allocation).

    A subclass, so every existing ``except OracleError`` and every ``code == "ir-overflow"``
    test behaves as before.  It exists so that ``omit_example`` can tell the one error that may
    qualify an example as ``value-over-limit`` (VER04r 8.4, [D34]) from the exact-integer window
    overflow, which shares the code but is an ordinary evaluation error that refuses.
    """

    def __init__(self, message: str) -> None:
        super().__init__("ir-overflow", message)
