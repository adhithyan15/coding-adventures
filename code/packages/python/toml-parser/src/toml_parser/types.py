"""TOML Types — Python representations of TOML values.

TOML maps to Python types naturally. This module defines:

- ``TOMLDocument`` — an ordered dictionary representing a TOML document.
- ``TOMLValue`` — a type alias for all possible TOML value types.

TOML-to-Python Type Mapping
----------------------------

TOML is designed to map unambiguously to a hash table. Each TOML type has
a natural Python counterpart:

========================= ==========================
TOML Type                  Python Type
========================= ==========================
String                     ``str``
Integer                    ``int``
Float                      ``float``
Boolean                    ``bool``
Offset Date-Time           ``datetime.datetime`` (with ``tzinfo``)
Local Date-Time            ``datetime.datetime`` (without ``tzinfo``)
Local Date                 ``datetime.date``
Local Time                 ``datetime.time``
Array                      ``list``
Table / Inline Table       ``TOMLDocument`` (``dict``)
========================= ==========================

Why TOMLDocument Instead of Plain dict?
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

A ``TOMLDocument`` is just a ``dict`` subclass. We use a named type for two
reasons:

1. **Clarity** — functions that return ``TOMLDocument`` make their intent
   clear. A return type of ``dict[str, Any]`` could be anything.

2. **Extensibility** — if we later need to add metadata (source positions,
   original formatting for round-tripping), we have a place to put it without
   breaking the ``dict`` interface.

Using a ``dict`` subclass means all existing ``dict`` operations work:
``document["key"]``, ``document.get("key")``, ``"key" in document``, etc.
"""

from __future__ import annotations

import datetime
from typing import Union

# ---------------------------------------------------------------------------
# TOMLValue — The union of all types a TOML value can be
# ---------------------------------------------------------------------------
#
# This type alias describes the *recursive* structure of TOML values:
#
# - Scalars: str, int, float, bool, datetime, date, time
# - Containers: list[TOMLValue], TOMLDocument (dict mapping str → TOMLValue)
#
# Python's type system doesn't support recursive type aliases perfectly, so
# we use a forward reference for the recursive cases. The ``type`` statement
# (Python 3.12+) handles this cleanly, but we use Union for broader
# compatibility with type checkers.

TOMLValue = Union[
    str,
    int,
    float,
    bool,
    datetime.datetime,
    datetime.date,
    datetime.time,
    list,  # list[TOMLValue] — recursive
    "TOMLDocument",  # dict[str, TOMLValue] — recursive
]


# ---------------------------------------------------------------------------
# TOMLDocument — An ordered dictionary representing a TOML document
# ---------------------------------------------------------------------------


class TOMLDocument(dict):
    """An ordered dictionary representing a parsed TOML document.

    ``TOMLDocument`` is a plain ``dict`` subclass. Keys are always strings,
    and values are ``TOMLValue`` instances (strings, integers, floats,
    booleans, datetimes, lists, or nested ``TOMLDocument``s).

    Since Python 3.7, dictionaries preserve insertion order, so the order
    of key-value pairs in the TOML source is maintained.

    Usage::

        doc = TOMLDocument()
        doc["name"] = "TOML"
        doc["version"] = "1.0.0"
        doc["owner"] = TOMLDocument({"name": "Tom", "age": 42})

        # Standard dict operations all work:
        assert doc["name"] == "TOML"
        assert "owner" in doc
        assert list(doc.keys()) == ["name", "version", "owner"]

    Why not just use dict?
    ~~~~~~~~~~~~~~~~~~~~~~

    You could. ``TOMLDocument`` is fully compatible with ``dict``. The named
    type provides:

    1. **Type clarity** — ``parse_toml() -> TOMLDocument`` is more descriptive
       than ``parse_toml() -> dict[str, Any]``.
    2. **isinstance checks** — you can distinguish TOML tables from regular
       dicts: ``isinstance(value, TOMLDocument)``.
    3. **Extensibility** — a natural place to add metadata (source positions,
       comments) without breaking the dict interface.
    """

    def __repr__(self) -> str:
        """Show the document as ``TOMLDocument({...})`` for clarity.

        This makes it obvious in debug output that you're looking at a
        parsed TOML document, not just an anonymous dict.
        """
        return f"TOMLDocument({dict.__repr__(self)})"
