# Garbage Collector

Language-agnostic garbage collection framework with pluggable algorithms.

## What is this?

A garbage collector automatically reclaims memory that a program is no longer using. This package provides:

1. **An abstract interface** (`GarbageCollector`) — the contract that all GC algorithms implement
2. **Mark-and-sweep** (`MarkAndSweepGC`) — the classic algorithm from McCarthy's 1960 Lisp
3. **Heap object types** (`ConsCell`, `Symbol`, `LispClosure`) — things that live on the managed heap
4. **Symbol interning** (`SymbolTable`) — ensures identity-based equality for symbols

## How it fits in the stack

```
Logic Gates → Arithmetic → CPU → Assembler → Lexer → Parser → Compiler → [GC] → VM
```

The GC sits alongside the VM. Any language plugin (Lisp, Scheme, ML) can use it by passing a `GarbageCollector` instance to its VM factory.

## Usage

```python
from garbage_collector import MarkAndSweepGC, ConsCell, SymbolTable

# Create a GC
gc = MarkAndSweepGC()

# Allocate objects on the heap
addr = gc.allocate(ConsCell(car=42, cdr=99))
cell = gc.deref(addr)  # Get the object back

# Run garbage collection
freed = gc.collect(roots=[addr])  # addr is reachable, so 0 freed

# Symbol interning
table = SymbolTable(gc)
a = table.intern("foo")
b = table.intern("foo")
assert a == b  # Same name → same address
```

## Pluggable algorithms

The `GarbageCollector` ABC defines the contract. Swap algorithms in one line:

```python
vm = create_lisp_vm(gc=MarkAndSweepGC())  # default
vm = create_lisp_vm(gc=RefCountGC())       # swap in reference counting
```

Future algorithms (reference counting, generational, copying, tri-color) implement the same ABC.

## Mark-and-sweep algorithm

Two phases:

1. **Mark**: Starting from roots (stack, globals), recursively follow all references and mark each reachable object
2. **Sweep**: Walk the entire heap, delete any unmarked object, clear marks for next cycle

Handles reference cycles correctly (unlike reference counting).
