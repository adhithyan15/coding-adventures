# Garbage Collector

## Overview

A garbage collector (GC) automatically reclaims memory that a program is no longer using. Without a GC, the programmer must manually allocate and free memory — a source of bugs like use-after-free, double-free, and memory leaks.

This package provides a **language-agnostic garbage collection framework**. It defines an abstract interface (`GarbageCollector`) that any virtual machine can use, with pluggable algorithm implementations. The first algorithm is **mark-and-sweep**, the simplest and most historically important GC algorithm (invented by John McCarthy for the original 1960 Lisp).

## Layer Position

```
Logic Gates → Arithmetic → CPU → Assembler → Lexer → Parser → Compiler → [YOU ARE HERE] → VM
```

The GC sits alongside the VM — it manages the heap that the VM allocates objects on. Any language plugin (Lisp, Scheme, ML, etc.) can use it by passing a `GarbageCollector` instance to its VM factory.

## Concepts

### Why do we need garbage collection?

Programs create objects at runtime — cons cells, closures, strings, arrays. Some of these objects become unreachable: no variable points to them, no other object references them. Without cleanup, memory grows without bound.

There are three approaches to memory management:

1. **Manual** (C, C++) — programmer calls `malloc`/`free`. Fast but error-prone.
2. **Reference counting** (Swift, CPython) — each object tracks how many references point to it. When the count hits zero, the object is freed. Simple but can't handle cycles (A → B → A).
3. **Tracing GC** (Java, Go, Lisp) — periodically scan all reachable objects from "roots" (stack, globals), free everything else. Handles cycles. Our mark-and-sweep is this type.

### The heap

The heap is where dynamically allocated objects live. In our implementation, the heap is a dictionary mapping integer addresses to `HeapObject` instances:

```python
heap = {
    0: ConsCell(car=42, cdr=1),    # A cons cell at address 0
    1: ConsCell(car=99, cdr=NIL),  # Another cons cell at address 1
}
```

The stack and global variables hold integer addresses (pointers) into the heap. To access an object, you dereference its address.

### Mark-and-sweep algorithm

Mark-and-sweep is a two-phase tracing algorithm:

**Phase 1 — Mark**: Starting from roots (values on the stack, global variables), recursively follow all references and mark each reachable object.

**Phase 2 — Sweep**: Walk the entire heap. Any object NOT marked is unreachable — delete it. Clear all marks for the next cycle.

```
Before collection:
  Stack: [addr:0, addr:2]
  Heap:  {0: ConsCell(1,2), 1: ConsCell(3,NIL), 2: Symbol("x"), 3: ConsCell(4,NIL)}
                                                                   ↑ unreachable!
  (addr:3 is only reachable through addr:1, which is reachable through addr:0)

  Mark phase: mark 0 → follow to 1 → follow to 3 → mark 2. Marked: {0, 1, 2, 3}
  Sweep phase: everything marked, nothing freed.

  Now suppose we pop addr:0 from the stack:
  Stack: [addr:2]
  Mark phase: mark 2. Marked: {2}
  Sweep phase: delete 0, 1, 3. Freed: 3 objects.
```

### Pluggable algorithm design

The `GarbageCollector` ABC defines the contract:

```python
class GarbageCollector(ABC):
    def allocate(self, obj: HeapObject) -> int     # Store object, return address
    def deref(self, address: int) -> HeapObject     # Look up object by address
    def collect(self, roots: Iterable[Any]) -> int  # Run GC, return freed count
    def heap_size(self) -> int                       # Current heap occupancy
    def stats(self) -> dict[str, int]                # Introspection counters
```

Any GC algorithm implements this ABC. VMs depend on the interface, never the implementation. Swapping algorithms is one line:

```python
vm = create_lisp_vm(gc=MarkAndSweepGC())     # default
vm = create_lisp_vm(gc=RefCountGC())          # swap in reference counting
vm = create_lisp_vm(gc=GenerationalGC())      # swap in generational
```

### Future algorithms

The ABC is designed to support these algorithms (not implemented yet):

| Algorithm | Key Idea | Pros | Cons |
|-----------|----------|------|------|
| Mark-and-sweep | Trace from roots, delete unreachable | Simple, handles cycles | Stop-the-world pause |
| Reference counting | Track ref count per object | Immediate cleanup, no pause | Can't handle cycles |
| Generational | Young/old partitions, collect young often | Short pauses (most objects die young) | More complex |
| Copying/semi-space | Copy live objects, swap halves | Compacts memory | Wastes half the heap |
| Tri-color marking | White/gray/black sets, incremental | Concurrent, short pauses | Most complex |

### HeapObject types

Objects on the heap extend `HeapObject`:

```python
class ConsCell(HeapObject):
    """A Lisp cons cell — the fundamental building block of lists."""
    car: Any   # The first element (int, str, heap address, NIL)
    cdr: Any   # The rest (typically another cons cell address, or NIL)

class LispClosure(HeapObject):
    """A function closure — code + captured environment."""
    code: Any           # The compiled function body
    env: dict[str, Any] # Captured variable bindings
    params: list[str]   # Parameter names
```

### Symbol interning

Lisp symbols need identity-based equality: `(eq 'foo 'foo)` must be true. This requires **interning** — ensuring that every occurrence of the symbol `foo` maps to the same heap address.

The `SymbolTable` provides this:

```python
table = SymbolTable(gc)
addr1 = table.intern("foo")  # Allocates a Symbol on the heap
addr2 = table.intern("foo")  # Returns the SAME address
assert addr1 == addr2         # Identity equality works!
```

## Public API

```python
# gc.py
class HeapObject(ABC): ...
class ConsCell(HeapObject): ...
class LispClosure(HeapObject): ...
class GarbageCollector(ABC): ...

# mark_sweep.py
class MarkAndSweepGC(GarbageCollector): ...

# symbols.py
class SymbolTable: ...
```

## Test Strategy

- **Allocate and dereference**: allocate objects, verify they can be dereferenced
- **Collection frees unreachable**: allocate objects, remove roots, collect, verify freed
- **Collection preserves reachable**: allocate, keep roots, collect, verify still there
- **Transitive reachability**: cons cell pointing to cons cell — both survive if root points to outer
- **Cycle handling**: create a reference cycle, verify mark-and-sweep handles it
- **Stats tracking**: verify allocation/collection/freed counters
- **Symbol interning**: same name → same address, different names → different addresses
- **Symbol GC**: interned symbols are freed when no references remain

## Package Structure

```
code/packages/python/garbage-collector/
├── BUILD
├── CHANGELOG.md
├── README.md
├── pyproject.toml
├── src/
│   └── garbage_collector/
│       ├── __init__.py
│       ├── gc.py           # ABC + HeapObject types
│       ├── mark_sweep.py   # MarkAndSweepGC implementation
│       └── symbols.py      # SymbolTable (symbol interning)
└── tests/
    ├── test_mark_sweep.py
    └── test_symbols.py
```
