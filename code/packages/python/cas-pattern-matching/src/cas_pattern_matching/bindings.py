"""Immutable name-to-IR mapping produced by the matcher.

A successful :func:`match` returns a :class:`Bindings` containing every
named pattern's captured value. The matcher never mutates an existing
binding map; it returns a new one with each captured name added (or
returns the same instance when nothing changed). This makes the matcher
easy to reason about: each call's output depends only on its inputs.
"""

from __future__ import annotations

from collections.abc import Iterator, Mapping
from dataclasses import dataclass, field

from symbolic_ir import IRNode


@dataclass(frozen=True)
class Bindings(Mapping[str, IRNode]):
    """Immutable mapping from pattern name to captured IR.

    Use :meth:`bind` to extend; do not mutate ``_data`` directly.
    """

    _data: dict[str, IRNode] = field(default_factory=dict)

    def bind(self, name: str, value: IRNode) -> Bindings:
        """Return a new ``Bindings`` with ``name -> value`` added.

        If ``name`` is already bound to ``value``, returns ``self``
        unchanged. If ``name`` is bound to a different value the caller
        is expected to have checked already; this method does not
        validate.
        """
        if name in self._data and self._data[name] == value:
            return self
        new_data = dict(self._data)
        new_data[name] = value
        return Bindings(new_data)

    # ---- Mapping protocol ------------------------------------------------

    def __getitem__(self, name: str) -> IRNode:
        return self._data[name]

    def __iter__(self) -> Iterator[str]:
        return iter(self._data)

    def __len__(self) -> int:
        return len(self._data)

    def __contains__(self, key: object) -> bool:
        return key in self._data

    def __repr__(self) -> str:
        items = ", ".join(f"{k}={v!r}" for k, v in self._data.items())
        return f"Bindings({items})"
