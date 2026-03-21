"""Starlark Built-in Functions — The standard library of Starlark.

==========================================================================
Chapter 1: What Are Built-in Functions?
==========================================================================

Built-in functions are functions that are always available in Starlark without
importing them. They're implemented in the host language (Python) rather than
in Starlark bytecode. When the VM encounters a call to ``len(x)`` or
``range(10)``, it dispatches to the Python function registered here.

The Starlark specification defines approximately 30 built-in functions. This
module implements the most commonly used ones. Each function takes a list of
arguments and returns a value, following the protocol defined by
``GenericVM.register_builtin()``.

==========================================================================
Chapter 2: Starlark vs Python Built-ins
==========================================================================

Starlark's built-ins are a strict subset of Python's, with some restrictions:

- ``sorted()`` always returns a new list (no in-place sort)
- ``range()`` returns a list, not a lazy range object
- ``type()`` returns a string, not a type object
- ``print()`` returns None (output is captured by the VM)
- No ``eval()``, ``exec()``, ``globals()``, ``locals()`` (security)
"""

from __future__ import annotations

from typing import Any

from virtual_machine import VMTypeError


# =========================================================================
# Type functions
# =========================================================================


def builtin_type(args: list[Any]) -> str:
    """type(x) — Return the type name as a string.

    Unlike Python's ``type()`` which returns a type object, Starlark's
    ``type()`` returns a plain string. This is simpler and avoids
    metaprogramming complexity.

    >>> type(42) → "int"
    >>> type("hello") → "string"
    >>> type([1, 2]) → "list"
    """
    if len(args) != 1:
        raise VMTypeError(f"type() takes exactly 1 argument ({len(args)} given)")
    value = args[0]
    if value is None:
        return "NoneType"
    if isinstance(value, bool):
        return "bool"
    if isinstance(value, int):
        return "int"
    if isinstance(value, float):
        return "float"
    if isinstance(value, str):
        return "string"
    if isinstance(value, list):
        return "list"
    if isinstance(value, dict):
        return "dict"
    if isinstance(value, tuple):
        return "tuple"
    return type(value).__name__


def builtin_bool(args: list[Any]) -> bool:
    """bool(x) — Convert to boolean.

    Follows Starlark truthiness rules.
    """
    if len(args) != 1:
        raise VMTypeError(f"bool() takes exactly 1 argument ({len(args)} given)")
    value = args[0]
    if value is None:
        return False
    if isinstance(value, bool):
        return value
    if isinstance(value, (int, float)):
        return value != 0
    if isinstance(value, (str, list, dict, tuple)):
        return len(value) > 0
    return True


def builtin_int(args: list[Any]) -> int:
    """int(x) — Convert to integer.

    Supports: int, float (truncates), string (parses), bool.
    Optional second arg specifies base for string conversion.
    """
    if len(args) < 1 or len(args) > 2:
        raise VMTypeError(
            f"int() takes 1 or 2 arguments ({len(args)} given)"
        )
    value = args[0]
    if len(args) == 2:
        base = args[1]
        if not isinstance(value, str):
            raise VMTypeError("int() can't convert non-string with explicit base")
        return int(value, base)
    if isinstance(value, bool):
        return 1 if value else 0
    if isinstance(value, int):
        return value
    if isinstance(value, float):
        return int(value)
    if isinstance(value, str):
        return int(value)
    raise VMTypeError(f"int() argument must be a string or a number, not '{type(value).__name__}'")


def builtin_float(args: list[Any]) -> float:
    """float(x) — Convert to float."""
    if len(args) != 1:
        raise VMTypeError(f"float() takes exactly 1 argument ({len(args)} given)")
    value = args[0]
    if isinstance(value, (int, float)):
        return float(value)
    if isinstance(value, str):
        return float(value)
    raise VMTypeError(f"float() argument must be a string or a number, not '{type(value).__name__}'")


def builtin_str(args: list[Any]) -> str:
    """str(x) — Convert to string representation."""
    if len(args) != 1:
        raise VMTypeError(f"str() takes exactly 1 argument ({len(args)} given)")
    value = args[0]
    if value is None:
        return "None"
    if isinstance(value, bool):
        return "True" if value else "False"
    if isinstance(value, str):
        return value
    return repr(value)


# =========================================================================
# Collection functions
# =========================================================================


def builtin_len(args: list[Any]) -> int:
    """len(x) — Return the length of a collection or string."""
    if len(args) != 1:
        raise VMTypeError(f"len() takes exactly 1 argument ({len(args)} given)")
    value = args[0]
    if isinstance(value, (str, list, dict, tuple)):
        return len(value)
    raise VMTypeError(f"object of type '{type(value).__name__}' has no len()")


def builtin_list(args: list[Any]) -> list:
    """list(x) — Convert an iterable to a list."""
    if len(args) == 0:
        return []
    if len(args) != 1:
        raise VMTypeError(f"list() takes at most 1 argument ({len(args)} given)")
    return list(args[0])


def builtin_dict(args: list[Any]) -> dict:
    """dict() — Create a new dictionary.

    Called with no args: returns empty dict.
    Called with an iterable of (key, value) pairs: creates dict from pairs.
    """
    if len(args) == 0:
        return {}
    if len(args) == 1:
        return dict(args[0])
    raise VMTypeError(f"dict() takes at most 1 argument ({len(args)} given)")


def builtin_tuple(args: list[Any]) -> tuple:
    """tuple(x) — Convert an iterable to a tuple."""
    if len(args) == 0:
        return ()
    if len(args) != 1:
        raise VMTypeError(f"tuple() takes at most 1 argument ({len(args)} given)")
    return tuple(args[0])


def builtin_range(args: list[Any]) -> list:
    """range(stop) or range(start, stop[, step]) — Return a list of integers.

    Unlike Python's lazy range(), Starlark's range() returns a concrete list.
    This is because Starlark forbids lazy evaluation for determinism.
    """
    if len(args) == 1:
        return list(range(args[0]))
    elif len(args) == 2:
        return list(range(args[0], args[1]))
    elif len(args) == 3:
        return list(range(args[0], args[1], args[2]))
    raise VMTypeError(
        f"range() takes 1 to 3 arguments ({len(args)} given)"
    )


def builtin_sorted(args: list[Any]) -> list:
    """sorted(x[, key][, reverse]) — Return a new sorted list.

    For simplicity, only supports positional args:
    sorted(iterable) or sorted(iterable, reverse).
    """
    if len(args) < 1 or len(args) > 2:
        raise VMTypeError(
            f"sorted() takes 1 or 2 arguments ({len(args)} given)"
        )
    iterable = args[0]
    reverse = bool(args[1]) if len(args) > 1 else False
    return sorted(iterable, reverse=reverse)


def builtin_reversed(args: list[Any]) -> list:
    """reversed(x) — Return a reversed list."""
    if len(args) != 1:
        raise VMTypeError(f"reversed() takes exactly 1 argument ({len(args)} given)")
    return list(reversed(args[0]))


def builtin_enumerate(args: list[Any]) -> list:
    """enumerate(x[, start]) — Return list of (index, value) pairs."""
    if len(args) < 1 or len(args) > 2:
        raise VMTypeError(
            f"enumerate() takes 1 or 2 arguments ({len(args)} given)"
        )
    start = args[1] if len(args) > 1 else 0
    return list(enumerate(args[0], start))


def builtin_zip(args: list[Any]) -> list:
    """zip(*iterables) — Return list of tuples."""
    return list(zip(*args))


# =========================================================================
# Logic and math functions
# =========================================================================


def builtin_min(args: list[Any]) -> Any:
    """min(x, y, ...) or min(iterable) — Return the smallest element."""
    if len(args) == 1 and hasattr(args[0], '__iter__'):
        return min(args[0])
    return min(args)


def builtin_max(args: list[Any]) -> Any:
    """max(x, y, ...) or max(iterable) — Return the largest element."""
    if len(args) == 1 and hasattr(args[0], '__iter__'):
        return max(args[0])
    return max(args)


def builtin_abs(args: list[Any]) -> Any:
    """abs(x) — Return the absolute value."""
    if len(args) != 1:
        raise VMTypeError(f"abs() takes exactly 1 argument ({len(args)} given)")
    return abs(args[0])


def builtin_all(args: list[Any]) -> bool:
    """all(iterable) — Return True if all elements are truthy."""
    if len(args) != 1:
        raise VMTypeError(f"all() takes exactly 1 argument ({len(args)} given)")
    return all(args[0])


def builtin_any(args: list[Any]) -> bool:
    """any(iterable) — Return True if any element is truthy."""
    if len(args) != 1:
        raise VMTypeError(f"any() takes exactly 1 argument ({len(args)} given)")
    return any(args[0])


# =========================================================================
# String functions
# =========================================================================


def builtin_repr(args: list[Any]) -> str:
    """repr(x) — Return a string representation."""
    if len(args) != 1:
        raise VMTypeError(f"repr() takes exactly 1 argument ({len(args)} given)")
    return repr(args[0])


def builtin_hasattr(args: list[Any]) -> bool:
    """hasattr(x, name) — Return True if x has the named attribute."""
    if len(args) != 2:
        raise VMTypeError(f"hasattr() takes exactly 2 arguments ({len(args)} given)")
    return hasattr(args[0], args[1])


def builtin_getattr(args: list[Any]) -> Any:
    """getattr(x, name[, default]) — Get a named attribute."""
    if len(args) < 2 or len(args) > 3:
        raise VMTypeError(
            f"getattr() takes 2 or 3 arguments ({len(args)} given)"
        )
    if len(args) == 3:
        return getattr(args[0], args[1], args[2])
    return getattr(args[0], args[1])


# =========================================================================
# I/O functions
# =========================================================================


def builtin_print(args: list[Any]) -> None:
    """print(*args) — Print arguments.

    In Starlark, print() always returns None. The output is captured
    by the VM's output list rather than going to stdout.
    """
    # The actual printing is handled by the PRINT opcode handler
    # This function returns None because print() returns None
    return None


# =========================================================================
# Registration helper
# =========================================================================


def get_all_builtins() -> dict[str, Any]:
    """Return a dict mapping built-in function names to their implementations.

    This is used by ``create_starlark_vm()`` to register all built-ins
    with the GenericVM.
    """
    return {
        # Type functions
        "type": builtin_type,
        "bool": builtin_bool,
        "int": builtin_int,
        "float": builtin_float,
        "str": builtin_str,
        # Collection functions
        "len": builtin_len,
        "list": builtin_list,
        "dict": builtin_dict,
        "tuple": builtin_tuple,
        "range": builtin_range,
        "sorted": builtin_sorted,
        "reversed": builtin_reversed,
        "enumerate": builtin_enumerate,
        "zip": builtin_zip,
        # Logic and math
        "min": builtin_min,
        "max": builtin_max,
        "abs": builtin_abs,
        "all": builtin_all,
        "any": builtin_any,
        # String/utility
        "repr": builtin_repr,
        "hasattr": builtin_hasattr,
        "getattr": builtin_getattr,
        # I/O
        "print": builtin_print,
    }
