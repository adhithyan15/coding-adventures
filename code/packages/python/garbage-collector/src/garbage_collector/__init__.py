"""Garbage Collector — Language-agnostic memory management framework.

This package provides an abstract garbage collection interface that any
virtual machine can use, plus concrete algorithm implementations.

The Big Picture
===============

When a program runs, it creates objects — cons cells, closures, strings,
arrays. Some of these objects become unreachable: no variable points to
them, no other object references them. Without cleanup, memory grows
without bound.

A **garbage collector** automatically finds and reclaims unreachable
objects. This package provides:

1. **An abstract interface** (``GarbageCollector``) — the contract that
   all GC algorithms implement. VMs depend on this interface, never on
   a specific algorithm.

2. **Mark-and-sweep** (``MarkAndSweepGC``) — the classic algorithm
   invented by John McCarthy for the original 1960 Lisp. Two phases:
   mark all reachable objects from roots, then sweep (delete) everything
   unmarked.

3. **Heap object types** (``HeapObject``, ``ConsCell``, ``LispClosure``,
   ``Symbol``) — the things that live on the managed heap.

4. **Symbol interning** (``SymbolTable``) — ensures that every reference
   to the symbol ``foo`` gets the same heap address, making identity-based
   equality (``eq``) work correctly.

Pluggable Algorithms
====================

The ``GarbageCollector`` ABC defines five methods::

    allocate(obj) -> int       # Store an object, get its address
    deref(address) -> obj      # Look up an object by address
    collect(roots) -> int      # Run a GC cycle, return freed count
    heap_size() -> int         # How many objects are on the heap
    stats() -> dict            # Introspection (allocations, collections, freed)

Any algorithm that implements these five methods can be plugged into any
VM. Swapping algorithms is one line::

    vm = create_lisp_vm(gc=MarkAndSweepGC())     # default
    vm = create_lisp_vm(gc=RefCountGC())          # swap in reference counting

Future algorithms (reference counting, generational, copying, tri-color)
implement the same ABC.
"""

from garbage_collector.gc import (
    ConsCell,
    GarbageCollector,
    HeapObject,
    LispClosure,
    Symbol,
)
from garbage_collector.mark_sweep import MarkAndSweepGC
from garbage_collector.symbols import SymbolTable

__all__ = [
    "ConsCell",
    "GarbageCollector",
    "HeapObject",
    "LispClosure",
    "MarkAndSweepGC",
    "Symbol",
    "SymbolTable",
]
