"""
errors.py — Exception types for the CSV parser.

We define a narrow, specific error type rather than raising a generic ValueError or
RuntimeError. This allows callers to distinguish "the CSV was malformed" from any other
exception that might bubble up during processing.

Why only one error type?
  CSV parsing has very few conditions that are truly unrecoverable. By design we
  handle most anomalies gracefully (ragged rows, empty files, missing newlines at EOF).
  The only condition we cannot silently recover from is an *unclosed* quoted field —
  because we have no way of knowing where the field ends, so we cannot produce a
  meaningful result.
"""


class UnclosedQuoteError(ValueError):
    """Raised when a quoted field is opened with '"' but never closed before EOF.

    Example input that triggers this error:
        id,value
        1,"this is never closed

    The parser reaches the end of the input while still inside a quoted field.
    There is no safe way to guess where the field was meant to end, so we raise
    rather than silently producing corrupt data.

    Inherits from ValueError because the input string is invalid/malformed — it
    violates the CSV grammar. This mirrors the convention used by Python's standard
    library json.JSONDecodeError (which also inherits ValueError).
    """
