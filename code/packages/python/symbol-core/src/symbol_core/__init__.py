"""symbol-core — interned symbolic names for logic and algebra.

The first goal of this package is small but important: give the rest of the
repo a shared notion of a symbolic name that is more precise than a raw
string.

Why not use strings directly?

- A raw string might be prose (`"hello world"`), source text, or a symbolic
  identifier. The type does not tell us which one it is.
- Logic programming and symbolic math repeatedly compare names. Interning lets
  equal names share a canonical object.
- Future layers like Prolog, expression trees, and theorem provers can all
  build on the same primitive without smuggling semantics through arbitrary
  strings.

The package therefore defines:

- ``Symbol``: an immutable symbolic name with an optional namespace
- ``SymbolTable``: an interning table that guarantees canonical identity per
  ``(namespace, name)`` pair
- ``sym()``: a convenience helper that interns through a module-global table
"""

from __future__ import annotations

from dataclasses import dataclass
from threading import RLock

__all__ = [
    "DEFAULT_SYMBOL_TABLE",
    "Symbol",
    "SymbolError",
    "SymbolTable",
    "__version__",
    "is_symbol",
    "sym",
]

__version__ = "0.1.0"


class SymbolError(ValueError):
    """Raised when a caller attempts to create an invalid symbol."""


def _validate_symbol_part(value: object, *, field_name: str) -> str:
    """Validate one component of a symbol key.

    Symbols are intentionally conservative in the prototype:

    - the component must be a string
    - it cannot be empty
    - it cannot have leading or trailing whitespace

    We reject suspicious values instead of normalizing them silently. That
    keeps symbol identity explicit and unsurprising.
    """

    if not isinstance(value, str):
        msg = f"{field_name} must be a string"
        raise SymbolError(msg)

    if value == "":
        msg = f"{field_name} must not be empty"
        raise SymbolError(msg)

    if value != value.strip():
        msg = f"{field_name} must not contain leading or trailing whitespace"
        raise SymbolError(msg)

    return value


@dataclass(frozen=True, slots=True)
class Symbol:
    """A canonical symbolic name.

    ``namespace`` is optional so the first version can support both:

    - ``Symbol(name="parent", namespace=None)``
    - ``Symbol(name="sin", namespace="math")``
    """

    namespace: str | None
    name: str

    def __str__(self) -> str:
        """Render the most readable external spelling of this symbol."""

        if self.namespace is None:
            return self.name
        return f"{self.namespace}:{self.name}"

    def __repr__(self) -> str:
        """Show a precise constructor-style representation for debugging."""

        if self.namespace is None:
            return f"Symbol(name={self.name!r})"
        return f"Symbol(namespace={self.namespace!r}, name={self.name!r})"


class SymbolTable:
    """Intern symbols so equal names share one canonical object.

    Interning is one of those implementation details that has semantic value.
    If the table interns correctly, then:

    - equal symbolic names compare equal
    - repeated requests for the same name return the same object
    - downstream packages can cheaply use symbols as dictionary keys
    """

    def __init__(self) -> None:
        self._symbols: dict[tuple[str | None, str], Symbol] = {}
        self._lock = RLock()

    def intern(self, name: object, namespace: object = None) -> Symbol:
        """Return the canonical symbol for ``(namespace, name)``.

        If a symbol already exists, we return the existing instance.
        Otherwise we create it, store it, and return that newly interned
        object.
        """

        validated_name = _validate_symbol_part(name, field_name="name")
        validated_namespace: str | None
        if namespace is None:
            validated_namespace = None
        else:
            validated_namespace = _validate_symbol_part(
                namespace,
                field_name="namespace",
            )

        key = (validated_namespace, validated_name)
        with self._lock:
            cached = self._symbols.get(key)
            if cached is not None:
                return cached

            created = Symbol(namespace=validated_namespace, name=validated_name)
            self._symbols[key] = created
            return created

    def contains(self, name: object, namespace: object = None) -> bool:
        """Return whether the table has already interned this symbol key."""

        validated_name = _validate_symbol_part(name, field_name="name")
        validated_namespace: str | None
        if namespace is None:
            validated_namespace = None
        else:
            validated_namespace = _validate_symbol_part(
                namespace,
                field_name="namespace",
            )

        return (validated_namespace, validated_name) in self._symbols

    def size(self) -> int:
        """Return the number of canonical symbols held by this table."""

        return len(self._symbols)

    def __len__(self) -> int:
        """Make the table easy to inspect in tests and REPL sessions."""

        return self.size()


DEFAULT_SYMBOL_TABLE = SymbolTable()


def sym(
    name: object,
    namespace: object = None,
    *,
    table: SymbolTable | None = None,
) -> Symbol:
    """Intern a symbol through either the supplied or default symbol table."""

    active_table = DEFAULT_SYMBOL_TABLE if table is None else table
    return active_table.intern(name, namespace=namespace)


def is_symbol(value: object) -> bool:
    """Return ``True`` when ``value`` is a :class:`Symbol`."""

    return isinstance(value, Symbol)
