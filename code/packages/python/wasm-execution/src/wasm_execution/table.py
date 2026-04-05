"""table.py --- WASM table implementation for indirect function calls.

A WASM table is an array of opaque function references. Tables enable
indirect function calls (``call_indirect``): instead of calling a function
by its index directly, code looks up a function reference in a table at
runtime. This is how WASM implements C-style function pointers.

Table elements are either a valid function index or None (uninitialized).
Accessing a None element via ``call_indirect`` causes a trap.
"""

from __future__ import annotations

from wasm_execution.host_interface import TrapError


class Table:
    """A resizable array of nullable function indices.

    In WASM 1.0, table elements are always funcref --- either a valid
    function index or None (uninitialized).
    """

    def __init__(self, initial_size: int, max_size: int | None = None) -> None:
        self._elements: list[int | None] = [None] * initial_size
        self._max_size = max_size

    def get(self, index: int) -> int | None:
        """Get the function index at the given table index. Traps on OOB."""
        if index < 0 or index >= len(self._elements):
            msg = f"Out of bounds table access: index={index}, table size={len(self._elements)}"
            raise TrapError(msg)
        return self._elements[index]

    def set(self, index: int, func_index: int | None) -> None:
        """Set the function index at the given table index. Traps on OOB."""
        if index < 0 or index >= len(self._elements):
            msg = f"Out of bounds table access: index={index}, table size={len(self._elements)}"
            raise TrapError(msg)
        self._elements[index] = func_index

    def size(self) -> int:
        """Return the current table size."""
        return len(self._elements)

    def grow(self, delta: int) -> int:
        """Grow the table by ``delta`` entries (initialized to None).

        Returns the old size on success, or -1 if growth would exceed max.
        """
        old_size = len(self._elements)
        new_size = old_size + delta
        if self._max_size is not None and new_size > self._max_size:
            return -1
        self._elements.extend([None] * delta)
        return old_size
