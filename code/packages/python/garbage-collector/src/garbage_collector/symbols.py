"""Symbol Table — Interning symbols for identity-based equality.

==========================================================================
Chapter 1: Why Symbol Interning?
==========================================================================

In Lisp, symbols are names — ``foo``, ``bar``, ``factorial``. Two uses
of the same symbol name must be ``eq`` (identity-equal). This means they
must be the *same object* in memory, not just two objects with the same
name.

Without interning::

    addr1 = gc.allocate(Symbol("foo"))   # address 0
    addr2 = gc.allocate(Symbol("foo"))   # address 1
    addr1 == addr2  → False!  # Different addresses, eq fails!

With interning::

    table = SymbolTable(gc)
    addr1 = table.intern("foo")  # allocates Symbol("foo") at address 0
    addr2 = table.intern("foo")  # returns address 0 (same symbol!)
    addr1 == addr2  → True!   # Same address, eq works!

==========================================================================
Chapter 2: How It Works
==========================================================================

The ``SymbolTable`` maintains a dictionary mapping symbol names to their
heap addresses. When you intern a name:

1. If the name is already in the table AND the symbol is still alive on
   the heap, return the existing address.
2. Otherwise, allocate a new ``Symbol`` on the heap and record its address.

The symbols live on the GC heap, so they can be collected if no code
references them. After a GC cycle, the symbol table's entries might
point to freed addresses — ``intern()`` detects this and re-allocates.

==========================================================================
Chapter 3: Integration with GC
==========================================================================

The symbol table does NOT prevent garbage collection. If no roots
reference a symbol, it gets collected like any other heap object.
The next time someone interns that name, a fresh Symbol is allocated.

This is the correct behavior: symbols that nobody uses should be freed.
If you want a symbol to survive collection, keep a reference to it
(on the stack, in a variable, etc.).
"""

from __future__ import annotations

from garbage_collector.gc import GarbageCollector, Symbol


class SymbolTable:
    """Interns symbols so that equal names share the same heap address.

    A symbol table ensures identity-based equality for symbols: two
    references to ``'foo`` get the same heap address, making ``(eq 'foo 'foo)``
    true.

    Usage::

        gc = MarkAndSweepGC()
        table = SymbolTable(gc)

        a = table.intern("foo")   # Allocates Symbol("foo")
        b = table.intern("foo")   # Returns same address
        assert a == b             # Identity equality works!

        c = table.intern("bar")   # Different name, different address
        assert a != c

    The symbol table works with any GC algorithm — it only uses the
    ``allocate``, ``deref``, and ``is_valid_address`` methods from the
    ``GarbageCollector`` interface.
    """

    def __init__(self, gc: GarbageCollector) -> None:
        """Create a symbol table backed by the given garbage collector.

        Args:
            gc: The garbage collector to allocate symbols on. Symbols
                are heap objects managed by the GC like everything else.
        """
        self._gc = gc
        # Maps symbol names to their heap addresses.
        self._table: dict[str, int] = {}

    def intern(self, name: str) -> int:
        """Intern a symbol name, returning its heap address.

        If the symbol has been interned before and is still alive on
        the heap, returns the existing address. Otherwise, allocates
        a new ``Symbol`` object on the heap.

        Args:
            name: The symbol name to intern (e.g., ``"foo"``).

        Returns:
            The heap address of the interned symbol.
        """
        # -----------------------------------------------------------
        # Check if we already have this symbol AND it's still alive.
        # After a GC cycle, the address might point to a freed object.
        # -----------------------------------------------------------
        if name in self._table:
            address = self._table[name]
            if self._gc.is_valid_address(address):
                return address

        # -----------------------------------------------------------
        # Allocate a new Symbol on the heap and record its address.
        # -----------------------------------------------------------
        address = self._gc.allocate(Symbol(name=name))
        self._table[name] = address
        return address

    def lookup(self, name: str) -> int | None:
        """Look up a symbol without allocating.

        Returns the heap address if the symbol is interned and alive,
        or ``None`` if not.

        Args:
            name: The symbol name to look up.

        Returns:
            The heap address, or ``None`` if not found.
        """
        if name in self._table:
            address = self._table[name]
            if self._gc.is_valid_address(address):
                return address
        return None

    def all_symbols(self) -> dict[str, int]:
        """Return all currently interned (alive) symbols.

        Returns:
            A dictionary mapping symbol names to heap addresses,
            containing only symbols that are still alive on the heap.
        """
        return {
            name: addr
            for name, addr in self._table.items()
            if self._gc.is_valid_address(addr)
        }
