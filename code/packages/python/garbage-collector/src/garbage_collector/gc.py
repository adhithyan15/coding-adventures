"""Garbage Collector — Abstract interface and heap object types.

==========================================================================
Chapter 1: What Lives on the Heap?
==========================================================================

In a running program, some values are simple — integers, booleans. These
live directly on the stack or in variables. But structured values — cons
cells, closures, symbols — need to be *allocated* on a managed heap.

A ``HeapObject`` is anything that lives on the managed heap. Each heap
object has an address (an integer), and other values can reference it by
that address. The garbage collector's job is to figure out which heap
objects are still reachable and free the rest.

Think of the heap as a big dictionary::

    heap = {
        0: ConsCell(car=42, cdr=1),       # A cons cell at address 0
        1: ConsCell(car=99, cdr=NIL),     # Linked from address 0
        2: Symbol(name="factorial"),       # A symbol
        3: LispClosure(code=..., env={}), # A closure
    }

The stack and variables hold *addresses* (integers like 0, 1, 2, 3).
To get the actual object, you ``deref(address)`` through the GC.

==========================================================================
Chapter 2: The HeapObject Hierarchy
==========================================================================

All heap-allocated objects inherit from ``HeapObject``. Currently:

- ``ConsCell`` — a pair of values (the fundamental Lisp building block)
- ``Symbol`` — an interned name (like an atom in Lisp)
- ``LispClosure`` — a function + its captured environment

Future languages can add their own types (arrays, records, etc.) by
subclassing ``HeapObject``. The GC doesn't need to know what the objects
*are* — it only needs to know how to find references *within* them (for
the mark phase).

==========================================================================
Chapter 3: The GarbageCollector ABC
==========================================================================

The ``GarbageCollector`` abstract class defines the contract that all
GC algorithms must implement. This is the interface that VMs use —
they never depend on a specific algorithm.

Five methods:

1. ``allocate(obj)`` — Store an object on the heap, return its address.
2. ``deref(address)`` — Look up an object by address.
3. ``collect(roots)`` — Run a collection cycle. The ``roots`` parameter
   tells the GC where to start scanning (stack values, global variables).
   Returns the number of objects freed.
4. ``heap_size()`` — How many objects are currently on the heap.
5. ``stats()`` — Introspection counters for debugging and testing.

By programming against this ABC, VMs can swap GC algorithms without
changing any other code.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any

# =========================================================================
# Heap Object Types
# =========================================================================
#
# These are the things that live on the managed heap. Each type stores
# its data as plain Python values. References to other heap objects are
# stored as integer addresses.
#
# The ``marked`` field is used by tracing GCs (like mark-and-sweep) to
# track which objects have been visited during the mark phase. Non-tracing
# GCs (like reference counting) can ignore it.
# =========================================================================


@dataclass
class HeapObject(ABC):
    """Base class for anything that lives on the managed heap.

    Every heap object has a ``marked`` flag used by tracing garbage
    collectors. During the mark phase, reachable objects get marked.
    During the sweep phase, unmarked objects are freed.

    Subclasses should store references to other heap objects as integer
    addresses (not direct Python references). This lets the GC track
    reachability through the address space.
    """

    marked: bool = field(default=False, repr=False)

    def references(self) -> list[int]:
        """Return all heap addresses that this object references.

        The GC calls this during the mark phase to find transitive
        references. Override in subclasses that hold references to
        other heap objects.

        Returns:
            A list of integer heap addresses. Values that aren't valid
            heap addresses (strings, None, sentinel objects) are filtered
            out by the GC.
        """
        return []


@dataclass
class ConsCell(HeapObject):
    """A cons cell — the fundamental building block of Lisp lists.

    A cons cell is simply a pair: ``car`` (the first element) and ``cdr``
    (the rest). Lists are chains of cons cells::

        (1 2 3) = ConsCell(1, ConsCell(2, ConsCell(3, NIL)))

    The ``car`` and ``cdr`` can be:
    - An integer (a number value, or a heap address)
    - A string
    - The NIL sentinel
    - Any other value the language supports

    When ``car`` or ``cdr`` is an integer that happens to be a valid heap
    address, the GC treats it as a reference and follows it during marking.
    """

    car: Any = None
    cdr: Any = None

    def references(self) -> list[int]:
        """Return heap addresses referenced by this cons cell.

        Both ``car`` and ``cdr`` might be heap addresses (integers).
        The GC will check whether each address is actually valid before
        following it.
        """
        refs = []
        if isinstance(self.car, int):
            refs.append(self.car)
        if isinstance(self.cdr, int):
            refs.append(self.cdr)
        return refs


@dataclass
class Symbol(HeapObject):
    """An interned symbol — a named atom in Lisp.

    Symbols are interned: every occurrence of the same name maps to the
    same heap address. This makes identity-based equality work::

        (eq 'foo 'foo)  → t   (same address, so eq is true)

    Symbols don't reference other heap objects, so ``references()``
    returns an empty list.
    """

    name: str = ""


@dataclass
class LispClosure(HeapObject):
    """A function closure — compiled code + captured environment.

    When a ``lambda`` expression is evaluated, it captures the current
    environment (variable bindings visible at the point of definition).
    The result is a closure: the compiled function body (a CodeObject)
    plus the captured environment.

    The ``env`` dictionary maps variable names to their values. Values
    that are integers might be heap addresses (references to other
    heap objects). The ``params`` list names the parameters that the
    function expects.
    """

    code: Any = None
    env: dict[str, Any] = field(default_factory=dict)
    params: list[str] = field(default_factory=list)

    def references(self) -> list[int]:
        """Return heap addresses referenced by captured variables.

        Any integer value in the captured environment might be a heap
        address. The GC checks validity before following.
        """
        return [v for v in self.env.values() if isinstance(v, int)]


# =========================================================================
# Garbage Collector ABC
# =========================================================================
#
# This is the interface that all GC algorithms implement. VMs use this
# ABC — they never import a specific algorithm directly. This makes
# algorithms hot-swappable.
# =========================================================================


class GarbageCollector(ABC):
    """Abstract base class for garbage collection algorithms.

    This defines the contract that all GC algorithms must implement.
    VMs depend on this interface, never on a specific algorithm.

    Usage::

        gc = MarkAndSweepGC()              # or any other algorithm
        addr = gc.allocate(ConsCell(1, 2)) # allocate a cons cell
        cell = gc.deref(addr)              # get it back
        freed = gc.collect(roots=[addr])   # run a collection cycle

    The ``roots`` parameter to ``collect()`` tells the GC where to start
    scanning. Roots are typically:
    - Values on the VM stack
    - Values in global variables
    - Values in local variable slots of active call frames

    Everything reachable from roots survives. Everything else is freed.
    """

    @abstractmethod
    def allocate(self, obj: HeapObject) -> int:
        """Allocate an object on the heap and return its address.

        Args:
            obj: The heap object to store.

        Returns:
            An integer address that can be used to ``deref`` the object.
        """

    @abstractmethod
    def deref(self, address: int) -> HeapObject:
        """Look up a heap object by its address.

        Args:
            address: The integer address returned by ``allocate``.

        Returns:
            The heap object stored at that address.

        Raises:
            KeyError: If the address is not valid (object was freed or
                never allocated).
        """

    @abstractmethod
    def collect(self, roots: list[Any]) -> int:
        """Run a garbage collection cycle.

        Identifies all objects reachable from ``roots`` and frees
        everything else.

        Args:
            roots: A list of values to scan for heap references.
                Integers that are valid heap addresses are treated
                as references. Non-integer values are ignored.

        Returns:
            The number of objects freed during this cycle.
        """

    @abstractmethod
    def heap_size(self) -> int:
        """Return the number of objects currently on the heap."""

    @abstractmethod
    def stats(self) -> dict[str, int]:
        """Return introspection counters for debugging and testing.

        Returns:
            A dictionary with at least these keys:
            - ``total_allocations``: total objects ever allocated
            - ``total_collections``: total GC cycles run
            - ``total_freed``: total objects ever freed
        """

    def is_valid_address(self, address: int) -> bool:
        """Check whether an address points to a live heap object.

        This is used during root scanning to distinguish heap addresses
        from plain integer values (like the number 42).

        The default implementation tries ``deref`` and catches KeyError.
        Subclasses may override for efficiency.
        """
        try:
            self.deref(address)
            return True
        except KeyError:
            return False

    # ----------------------------------------------------------------
    # LANG16 dispatch hooks — consulted by vm-core's safepoint logic
    # and by the ``field_store`` opcode handler.
    #
    # Default implementations make the simplest possible policy:
    # collect at every safepoint, no write barrier needed.  That keeps
    # mark-and-sweep and any future trace-from-roots collector correct
    # without overrides.  Generational / incremental / concurrent /
    # reference-counting collectors override these to implement their
    # own policy.
    # ----------------------------------------------------------------

    def should_collect(self) -> bool:
        """Tell vm-core whether to run a collection at the next safepoint.

        vm-core consults this *after* dispatching an instruction whose
        ``IIRInstr.may_alloc`` flag is True (and at the periodic
        forced-safepoint interval).  Returning True triggers
        ``collect(roots)`` with the VM's current root set.

        The default returns True — i.e. "collect at every safepoint".
        That is the simplest correct policy: it never wastes memory by
        deferring a needed collection, and the dispatcher's safepoint
        interval bounds the overhead.  Mark-and-sweep is happy with
        this policy and ships unchanged.

        Generational, incremental, or concurrent collectors override
        to defer collections until a policy-specific trigger fires
        (young-gen full, allocation-byte threshold, periodic timer,
        etc.).
        """
        return True

    def write_barrier(self, parent_address: int, child_address: int) -> None:
        """Record a reference write into a heap object.

        vm-core invokes this from the ``field_store`` opcode handler
        whenever a heap reference is stored into another heap object.
        The arguments are::

            parent_address — heap address of the object being mutated
            child_address  — heap address being stored into one of
                             its fields

        The default is a no-op.  Mark-and-sweep does not need write
        barriers because it re-scans the entire heap from roots on
        every collection — every reachable child gets re-discovered.

        Generational collectors override to maintain a remembered set
        (old → young pointers).  Incremental / concurrent collectors
        override for tri-color marking invariants (re-mark grey on
        black → white writes).  Reference-counting collectors typically
        also need the *old* field value to decref it; they hook
        ``field_store`` directly and drive ``incref(child)`` /
        ``decref(old)`` from there, so this barrier is rarely the
        right place for them either.
        """
        # No-op default.  Arguments are explicitly dropped so any
        # "unused argument" linter that takes the docstring at its
        # word stays quiet.
        del parent_address, child_address
