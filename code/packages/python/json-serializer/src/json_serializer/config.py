"""Serializer configuration — controls how JSON text is formatted.

This module defines ``SerializerConfig``, a simple dataclass that lets callers
control indentation style, key sorting, and trailing newlines when producing
pretty-printed JSON.

The design follows the "configuration object" pattern: instead of passing many
keyword arguments to ``serialize_pretty()``, we bundle all formatting options
into a single, reusable object. This makes it easy to define a project-wide
style once and use it everywhere:

    >>> style = SerializerConfig(indent_size=4, sort_keys=True)
    >>> serialize_pretty(my_data, config=style)

Why a dataclass?
    Dataclasses give us ``__init__``, ``__repr__``, ``__eq__``, and keyword
    defaults for free. No boilerplate constructors needed.
"""

from __future__ import annotations

from dataclasses import dataclass


# ---------------------------------------------------------------------------
# SerializerConfig — the "knobs" for JSON formatting
# ---------------------------------------------------------------------------
#
# JSON has no opinion on whitespace (RFC 8259 says whitespace is insignificant
# between tokens). That gives us full control over formatting when producing
# output for human consumption.
#
# There are exactly four things a human might want to configure:
#
#   1. How wide is each indentation level?  (indent_size)
#   2. What character is used for indentation?  (indent_char: space or tab)
#   3. Should object keys be sorted?  (sort_keys)
#   4. Should the output end with a newline?  (trailing_newline)
#
# Compact mode (``serialize()``) ignores all of these — it always produces
# the smallest possible output with no unnecessary whitespace.
# ---------------------------------------------------------------------------


@dataclass
class SerializerConfig:
    """Configuration for JSON pretty-printing.

    Attributes
    ----------
    indent_size : int
        Number of indent characters per indentation level. Default: 2.
        Common alternatives: 4 (Python standard), 1 (when using tabs).

    indent_char : str
        Character used for indentation. Must be ``' '`` (space) or
        ``'\\t'`` (tab). Default: ``' '`` (space).

    sort_keys : bool
        Whether to sort object keys alphabetically. Default: False
        (preserve insertion order). Sorted keys produce deterministic
        output — useful for diffing, hashing, or canonical forms.

    trailing_newline : bool
        Whether to append ``'\\n'`` at the end of the top-level output.
        Default: False. Many text editors and POSIX tools expect files
        to end with a newline — set this to True when writing JSON files
        to disk.
    """

    indent_size: int = 2
    indent_char: str = " "
    sort_keys: bool = False
    trailing_newline: bool = False
