"""Error types and the closed set of failure ids.

Two different things can go wrong in a lane run, and they must never be
confused:

* **The run cannot be trusted to start.**  The corpus is malformed, the
  binding names an export that does not exist, the command line is wrong.
  These raise :class:`LaneError` (a subclass per cause) and the command exits
  with status ``2``.  No verdict is produced, because there is nothing to
  give a verdict *about*.

* **The library under test is wrong.**  A case disagrees with the model.
  That is a *verdict* (``not ok <id> <failure-id>``), exit status ``1``.  It
  is data, not an exception, because a negative control *wants* it.

The failure ids are a closed set (VER04i section 6.2).  A manifest checker
can therefore reject an id it has never heard of instead of guessing.
"""

from __future__ import annotations

# ---------------------------------------------------------------------------
# Verdict failure ids (closed set; VER04i 6.1 and 6.2)
# ---------------------------------------------------------------------------
COUNT_MISMATCH = "count-mismatch"
STREAM_MISMATCH = "stream-mismatch"
CALL_COUNT_MISMATCH = "call-count-mismatch"
RESULT_MISMATCH = "result-mismatch"
TYPE_MISMATCH = "type-mismatch"
MISSING_ERROR = "missing-error"
UNEXPECTED_ERROR = "unexpected-error"
UNCAUGHT_FAULT = "uncaught-fault"

FAILURE_IDS = frozenset(
    {
        COUNT_MISMATCH,
        STREAM_MISMATCH,
        CALL_COUNT_MISMATCH,
        RESULT_MISMATCH,
        TYPE_MISMATCH,
        MISSING_ERROR,
        UNEXPECTED_ERROR,
        UNCAUGHT_FAULT,
    }
)


class LaneError(Exception):
    """The run cannot proceed; the command exits with status 2.

    ``code`` is a short stable slug (``"malformed-json"``, ``"digest"`` ...)
    so tests and callers can match on the cause without parsing prose.  The
    message never echoes untrusted input wholesale: it names the *kind* of
    problem and, where useful, an identifier from the corpus.
    """

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code
        self.message = message

    def __str__(self) -> str:
        return f"{self.code}: {self.message}"


class CorpusError(LaneError):
    """The corpus, its TFV file or its model IR is malformed or inconsistent."""


class BindingError(LaneError):
    """The binding file is malformed, incomplete or does not match the code."""


class UsageError(LaneError):
    """The command line or environment asks for something refused."""


def need(condition: bool, code: str, message: str) -> None:
    """Raise :class:`CorpusError` unless ``condition`` holds.

    Corpus validation is a long list of "this must be true"; one helper keeps
    each of them to a single readable line.
    """
    if not condition:
        raise CorpusError(code, message)
