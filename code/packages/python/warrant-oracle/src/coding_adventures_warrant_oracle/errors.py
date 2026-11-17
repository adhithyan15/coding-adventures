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
