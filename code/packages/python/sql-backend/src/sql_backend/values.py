"""
SQL value model
===============

The SQL pipeline is organized around a small, closed set of value types — the
things a SQL expression can evaluate to at runtime. Every row the backend
produces is a mapping from column name to one of these values, and every
literal or intermediate result inside the VM is also one of these values.

We deliberately keep this set small (five variants) for two reasons:

1. **Correctness.** Arithmetic, comparison, and NULL propagation rules are
   easier to get right when there are five cases to cover than when we have
   to think about every SQLite storage class. The VM implements three-valued
   logic (TRUE / FALSE / NULL); that only works if NULL is a first-class value
   and not, say, a ``None`` smuggled through a generic ``Any`` type.

2. **Portability.** This same value set has to map cleanly onto 17 target
   languages. Rust gets an enum, TypeScript a tagged union, Go an interface
   with variants — but they all have exactly the same five cases. Anything
   more exotic would not port.

Everywhere else in this package we alias the Python runtime types:

    SqlValue = None | int | float | str | bool

We rely on Python's dynamic typing here instead of defining a wrapper class.
A wrapper would cost us nothing but boxing overhead on every comparison,
and Python's ``isinstance`` already discriminates between the five variants.

One sharp edge: in Python, ``bool`` is a subclass of ``int``. That means
``isinstance(True, int)`` returns True. Code that needs to distinguish
booleans from integers must check ``bool`` *first*. The helper
:func:`sql_type_name` below does exactly that.
"""

from __future__ import annotations

# Five-variant SQL value. We use a type alias rather than a wrapper class —
# see module docstring for the reasoning.
SqlValue = None | int | float | str | bool


def sql_type_name(value: SqlValue) -> str:
    """Return the SQL type name of ``value``.

    Useful for error messages and conformance tests. The check order matters:
    ``bool`` must be tested before ``int`` because Python's ``bool`` is a
    subclass of ``int`` (``True is 1``, ``False is 0`` — historical oddity).
    """
    if value is None:
        return "NULL"
    if isinstance(value, bool):  # Must come before int — see docstring.
        return "BOOLEAN"
    if isinstance(value, int):
        return "INTEGER"
    if isinstance(value, float):
        return "REAL"
    if isinstance(value, str):
        return "TEXT"
    raise TypeError(f"not a SqlValue: {value!r} ({type(value).__name__})")


def is_sql_value(value: object) -> bool:
    """Return True if ``value`` is one of the five SqlValue variants.

    Use this at trust boundaries — e.g. when accepting values from
    user-supplied dictionaries — to reject anything we can't handle before it
    reaches the VM.
    """
    return value is None or isinstance(value, bool | int | float | str)
