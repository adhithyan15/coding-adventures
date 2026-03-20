"""Mark-and-Sweep Garbage Collector — The classic algorithm.

==========================================================================
Chapter 1: History
==========================================================================

Mark-and-sweep was invented by John McCarthy in 1960 for the original
Lisp implementation. It's the simplest tracing garbage collector and
the foundation for understanding all other GC algorithms.

The idea is beautifully simple:

1. **Mark**: Starting from roots (stack, globals), follow all references
   and mark each reachable object.
2. **Sweep**: Walk the entire heap. Delete any object that wasn't marked.
3. **Reset**: Clear all marks for the next cycle.

That's it. Three steps. Everything reachable survives. Everything else
is garbage.

==========================================================================
Chapter 2: How the Mark Phase Works
==========================================================================

The mark phase is a graph traversal. Starting from root values, we
recursively follow references:

1. For each root value, check if it's a valid heap address.
2. If it is, look up the object and mark it.
3. Ask the object for its references (``obj.references()``).
4. Recursively mark each referenced object.

This is essentially a depth-first search through the object graph.

Example::

    Stack: [addr:0, addr:5]
    Heap:
        0: ConsCell(car=1, cdr=2)  ← reachable from stack
        1: Symbol("x")             ← reachable from 0.car
        2: ConsCell(car=3, cdr=4)  ← reachable from 0.cdr
        3: Symbol("y")             ← reachable from 2.car
        4: NIL (not a heap addr)
        5: Symbol("z")             ← reachable from stack

    After marking: {0, 1, 2, 3, 5} are marked.

==========================================================================
Chapter 3: How the Sweep Phase Works
==========================================================================

After marking, we walk every object in the heap:

- If it's marked → unmark it (for next cycle) and keep it.
- If it's NOT marked → it's unreachable. Delete it.

::

    Before sweep: heap = {0: ✓, 1: ✓, 2: ✓, 3: ✓, 5: ✓, 6: ✗, 7: ✗}
    After sweep:  heap = {0, 1, 2, 3, 5}   (6 and 7 freed)

==========================================================================
Chapter 4: Handling Cycles
==========================================================================

One of mark-and-sweep's great strengths: it handles reference cycles
correctly. If A references B and B references A, but neither is reachable
from a root, both are correctly identified as garbage.

Reference counting GCs cannot do this — they need a separate cycle
detector. Mark-and-sweep gets it for free because it starts from roots,
not from the objects themselves.

==========================================================================
Chapter 5: Trade-offs
==========================================================================

Pros:
- Simple to implement and understand
- Correctly handles reference cycles
- No overhead on normal operations (no ref count updates)

Cons:
- **Stop-the-world pause**: The program must pause during collection.
  The pause is proportional to heap size (must visit every object).
- **Heap fragmentation**: Freed objects leave holes. Objects aren't
  compacted. (A copying GC solves this.)
- **Not incremental**: Must complete the entire mark-sweep in one go.
  (Tri-color marking solves this.)
"""

from __future__ import annotations

from typing import Any

from garbage_collector.gc import GarbageCollector, HeapObject


class MarkAndSweepGC(GarbageCollector):
    """Mark-and-sweep garbage collector implementation.

    The heap is a dictionary mapping integer addresses to ``HeapObject``
    instances. Addresses are assigned sequentially starting from 0.

    Usage::

        gc = MarkAndSweepGC()
        addr = gc.allocate(ConsCell(car=42, cdr=NIL))
        cell = gc.deref(addr)
        freed = gc.collect(roots=[addr])  # cell is reachable, so 0 freed

    Thread Safety
    -------------
    This implementation is NOT thread-safe. It's designed for educational
    use in a single-threaded VM.
    """

    def __init__(self) -> None:
        """Initialize an empty heap with no allocations."""
        # ---------------------------------------------------------------
        # The heap: maps integer addresses to HeapObject instances.
        # This is the core data structure. Everything else is bookkeeping.
        # ---------------------------------------------------------------
        self._heap: dict[int, HeapObject] = {}

        # ---------------------------------------------------------------
        # Next address to assign. Monotonically increasing — we never
        # reuse addresses. This simplifies debugging and avoids dangling
        # pointer issues.
        #
        # Addresses start at 0x10000 (65536) rather than 0 to avoid
        # ambiguity between heap addresses and small integers. Without
        # this offset, a cons cell (car=1, cdr=2) would have car values
        # indistinguishable from heap addresses 1 and 2, making it
        # impossible for the formatter to know whether "1" is the number
        # or a pointer to heap object #1. Starting at 65536 puts heap
        # addresses far above typical integer values used in programs.
        # ---------------------------------------------------------------
        self._next_address: int = 0x10000

        # ---------------------------------------------------------------
        # Counters for introspection and testing.
        # ---------------------------------------------------------------
        self._total_allocations: int = 0
        self._total_collections: int = 0
        self._total_freed: int = 0

    def allocate(self, obj: HeapObject) -> int:
        """Store an object on the heap and return its address.

        Each call assigns a new, never-before-used address. Addresses
        are monotonically increasing integers starting from 0x10000 (65536)
        to avoid ambiguity with small integer values in programs.

        Args:
            obj: The heap object to store.

        Returns:
            The integer address of the newly allocated object.

        Example::

            gc = MarkAndSweepGC()
            a = gc.allocate(ConsCell(car=1, cdr=2))  # returns 0x10000
            b = gc.allocate(Symbol(name="x"))          # returns 0x10001
        """
        address = self._next_address
        self._next_address += 1
        self._heap[address] = obj
        self._total_allocations += 1
        return address

    def deref(self, address: int) -> HeapObject:
        """Look up a heap object by its address.

        Args:
            address: The integer address returned by ``allocate``.

        Returns:
            The heap object at that address.

        Raises:
            KeyError: If the address is not valid (freed or never allocated).
        """
        return self._heap[address]

    def collect(self, roots: list[Any]) -> int:
        """Run a mark-and-sweep collection cycle.

        Phase 1 — Mark: Starting from roots, recursively mark all
        reachable objects.

        Phase 2 — Sweep: Walk the heap, delete unmarked objects,
        clear marks on surviving objects.

        Args:
            roots: Values to scan for heap references. Integers that
                are valid heap addresses are followed. Other values
                (strings, None, sentinel objects) are ignored.

        Returns:
            The number of objects freed.
        """
        self._total_collections += 1

        # Phase 1: Mark
        # Start from roots and recursively mark all reachable objects.
        for root in roots:
            self._mark_value(root)

        # Phase 2: Sweep
        # Walk the heap. Delete unmarked objects, clear marks on the rest.
        to_delete = []
        for address, obj in self._heap.items():
            if obj.marked:
                obj.marked = False  # Reset for next cycle
            else:
                to_delete.append(address)

        for address in to_delete:
            del self._heap[address]

        freed = len(to_delete)
        self._total_freed += freed
        return freed

    def _mark_value(self, value: Any) -> None:
        """Recursively mark a value and everything it references.

        If ``value`` is an integer that is a valid heap address, mark
        the object at that address and recursively mark its references.

        If ``value`` is a list or dict, scan its contents for heap
        addresses. This handles root scanning of VM stacks and
        variable tables.

        Args:
            value: A value to scan. Could be an int (possibly a heap
                address), a list, a dict, or anything else.
        """
        if isinstance(value, int):
            # -------------------------------------------------------
            # This integer might be a heap address. Check if it is.
            # If so, mark the object and follow its references.
            # -------------------------------------------------------
            if value in self._heap:
                obj = self._heap[value]
                if not obj.marked:
                    obj.marked = True
                    # Follow references from this object
                    for ref in obj.references():
                        self._mark_value(ref)

        elif isinstance(value, list):
            # -------------------------------------------------------
            # Scan list contents (e.g., a VM stack is a list of values)
            # -------------------------------------------------------
            for item in value:
                self._mark_value(item)

        elif isinstance(value, dict):
            # -------------------------------------------------------
            # Scan dict values (e.g., global variables, local slots)
            # -------------------------------------------------------
            for v in value.values():
                self._mark_value(v)

    def heap_size(self) -> int:
        """Return the number of objects currently on the heap."""
        return len(self._heap)

    def stats(self) -> dict[str, int]:
        """Return introspection counters.

        Returns:
            A dictionary with:
            - ``total_allocations``: total objects ever allocated
            - ``total_collections``: total GC cycles run
            - ``total_freed``: total objects ever freed
            - ``heap_size``: current number of live objects
        """
        return {
            "total_allocations": self._total_allocations,
            "total_collections": self._total_collections,
            "total_freed": self._total_freed,
            "heap_size": self.heap_size(),
        }

    def is_valid_address(self, address: int) -> bool:
        """Check whether an address points to a live heap object.

        More efficient than the default implementation — directly
        checks the heap dictionary instead of try/except.
        """
        return address in self._heap
