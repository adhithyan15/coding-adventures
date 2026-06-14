# LANG16 — gc-core: Heap and Garbage Collection Integration

> **Forward reference:** LANG20 specifies how a per-language
> `LangBinding` registers its `ClassRef` namespace with `gc-core` and
> provides the `trace_object` callback the collector dispatches
> through during marking.  The "language frontend's root scanner"
> referenced throughout this spec is LANG20's `LangBinding::trace_value`.
> See [LANG20 §"Cross-language value representation"](LANG20-multilang-runtime.md)
> for the uniform 16-byte heap object header every binding must
> agree on.

## Overview

`gc-core` is the **language-agnostic heap and garbage collection layer** for
the LANG pipeline.  It plugs in beneath `vm-core` (LANG02), is observed by
`jit-core` (LANG03), and is supported in AOT binaries via `vm-runtime`
(LANG15).

A standalone [`garbage-collector`](../packages/python/garbage-collector)
package and matching spec [`garbage-collector.md`](garbage-collector.md)
already exist.  They define `GarbageCollector` (ABC), `MarkAndSweepGC`
(implementation), `HeapObject` types (`ConsCell`, `Symbol`, `LispClosure`),
and `SymbolTable`.  But that package is **orphaned** — nothing in LANG01–15
allocates, no opcodes box values onto a heap, no frame holds heap roots,
and no AOT binary scans them.  A Lisp frontend would have to bypass the
whole LANG pipeline to use it.

LANG16 wires the existing GC into the pipeline.  It introduces:

- New **allocation opcodes** in InterpreterIR (LANG01)
- A **`ref<T>` type** in the IIR type system
- A **`may_alloc` flag** on `IIRInstr` so the dispatcher and JIT know
  where safepoints are
- A **root-scanning interface** on `vm-core` that walks the register file
  and frame stack, yielding heap addresses to the collector
- **Stack map** sections for compiled frames so the JIT and AOT can
  participate in collection
- **Write-barrier trampolines** for collectors that need them (generational,
  incremental)
- A **`gc_runtime_<target>.a`** companion to `vm_runtime_<target>.a` for
  level-3 AOT binaries (LANG15)

The existing `garbage-collector` package is **not rewritten**.  LANG16 adds
a thin `gc-core` adapter that exposes `GarbageCollector` instances to
`vm-core` through the new interface and registers them with `vm-runtime`
through the LANG15 hook.

---

## Why this is its own spec

The LANG pipeline today has a clean primitives-only mental model: every
register slot holds a `u8`/`u16`/`u32`/`u64`/`bool`/`str`/`any` value; the
VM is a pure dataflow machine; AOT can compile a Tetrad program to bare 4004
ROM with no runtime.

Adding a heap touches **every layer**:

| Layer | Today | After LANG16 |
|-------|-------|--------------|
| LANG01 IIR | Primitive types only | Adds `ref<T>` and 6 alloc opcodes |
| LANG02 vm-core | Stack-only register file | Roots include heap pointers |
| LANG03 jit-core | No GC awareness | Emits stack maps + barriers |
| LANG04 aot-core | Pure native code | Emits stack-map and root-table sections |
| LANG15 vm-runtime | Level 3 has hooks (no contents) | Hooks defined here |

LANG16 must land as a single coherent spec because piecemeal additions
(opcodes without root scanning, root scanning without stack maps) leave the
system in a state where any non-trivial Lisp program crashes on its first
collection.

---

## IIR additions

### New type: `ref<T>`

The `type_hint` field on `IIRInstr` (LANG01) gains one new form:

```
ref<T>     where T is one of: u8, u16, u32, u64, bool, str, any, ref<...>
```

Examples:

| `type_hint` | Meaning |
|-------------|---------|
| `ref<u8>`       | Pointer to a heap-allocated u8 (a "boxed" byte) |
| `ref<any>`      | Pointer to a heap object of unknown shape (Lisp's default) |
| `ref<ref<any>>` | Pointer to a pointer (e.g., a cons cell's `cdr` slot) |

Concretely, `ref<T>` values are **opaque heap addresses**.  The runtime
representation is an unsigned integer of the target's pointer width
(`uintptr_t` in the `vm-runtime` C ABI; LANG15's `VM_RESULT_REF` variant).
Dereferencing happens through opcodes; arithmetic on refs is forbidden.

The IIR type-inference pass (used by aot-core and the type checker) gains
two rules:

```
alloc T         → ref<T>
field_load p f  → typeof(f-th field of T)   where p : ref<T>
```

### New opcodes

The seven opcodes below are added to LANG01.  They are the **complete**
heap surface — anything more exotic (weak references, finalizers,
guarded loads) is library code on top of these primitives.

| Mnemonic        | Operands              | Description                                         | `may_alloc` |
|-----------------|-----------------------|-----------------------------------------------------|:-----------:|
| `alloc`         | (size, kind)          | Allocate `size` bytes of heap memory tagged `kind`; `dest = ref<any>` | yes |
| `box`           | (value,)              | Allocate a single-slot heap cell holding `value`; `dest = ref<typeof(value)>` | yes |
| `unbox`         | (ref,)                | `dest = *ref`; trap if ref is null                  | no  |
| `field_load`    | (ref, offset)         | `dest = *(ref + offset)`                            | no  |
| `field_store`   | (ref, offset, value)  | `*(ref + offset) = value`; emits write barrier      | no* |
| `is_null`       | (ref,)                | `dest = (ref == NULL)`                              | no  |
| `safepoint`     | ()                    | Yield to GC if a collection is pending              | yes |

\* `field_store` does not allocate, but it **may** trigger a barrier action
that allocates (e.g., card-table marking on generational GC).  The JIT must
treat it as a safepoint if the active collector requires it; the active
collector advertises this via a `wants_store_barrier_safepoint()` query.

### `may_alloc` flag

`IIRInstr` (LANG01) gains one field:

```python
@dataclass
class IIRInstr:
    ...
    may_alloc: bool = field(default=False, repr=False)
```

The compiler frontend sets `may_alloc=True` for any instruction that can
reach a heap allocation — every opcode marked "yes" above, plus any `call`
to a function whose transitive callee set is not allocation-free.

The JIT uses `may_alloc` to determine **safepoint locations**: collections
can only happen at safepoints, and stack maps only need to describe the
register file at safepoints.  This dramatically reduces stack-map size
versus emitting one per instruction.

### `kind` tags

The `kind` operand on `alloc` is a 16-bit integer that identifies the
heap-object layout to the collector.  Kinds are language-defined; the
collector receives the registered layout descriptor and uses it for
trace and finalization.

```python
class HeapKind:
    """Registered with the GC at VM startup; vm-core forwards to gc-core."""
    kind_id: int
    size: int
    field_offsets: list[int]   # byte offsets of ref-typed fields, for tracing
    finalizer: Callable | None
```

The compiler frontend assigns kind ids from a per-module table; the linker
(LANG10) merges them into a process-wide table at link time.

---

## vm-core additions

### Root enumeration

The collector needs to know every live heap address held by the VM.
`vm-core` exposes:

```python
class VMCore:
    def enumerate_roots(self) -> Iterator[int]:
        """Yield every heap address held in the register file or any frame.

        Includes:
        - Every register in every active VMFrame whose declared or observed
          type matches ref<...>
        - Every value in the builtin call argument scratch space
        - Every shadow frame held alive for deopt
        """
```

`enumerate_roots` is implemented in `vm-core/helpers/gc_hooks.py` (LANG15
already reserves the file).  It does not run a collection itself — it
yields addresses so that whatever `GarbageCollector` instance is bound can
mark them.

### GC binding

`VMConfig` (LANG02) gains:

```python
@dataclass
class VMConfig:
    ...
    gc: GarbageCollector | None = None
    safepoint_interval: int = 4096   # max instructions between forced safepoints
```

Programs that don't use heap opcodes leave `gc=None`; their `vm-core` runs
unchanged, with zero overhead.  Programs that do allocate must set `gc` —
attempting to dispatch `alloc`/`box`/`safepoint` with `gc=None` raises a
`VMError("no garbage collector configured")`.

### Safepoint dispatch

The dispatch loop (LANG02) is augmented:

```python
def dispatch_loop(self) -> int | None:
    while self._frames:
        frame = self._frames[-1]
        instr = frame.fn.instructions[frame.ip]

        if instr.may_alloc and self._gc is not None:
            self._maybe_collect()           # consult gc.should_collect()

        result = self._dispatch(frame, instr)
        ...
```

`_maybe_collect` calls `self._gc.collect(self.enumerate_roots())` when the
collector signals it is time.  Forced safepoints (every
`safepoint_interval` instructions) handle pure compute loops with no
allocation.

### Write barriers

For collectors that require them (generational, incremental), the
dispatcher invokes `gc.write_barrier(parent_ref, child_ref)` from the
`field_store` handler.  Mark-and-sweep ignores it (the current
`MarkAndSweepGC` implementation already conforms; the method is a no-op
for it).

---

## jit-core additions

### Stack maps

When `jit-core` (LANG03) compiles a function, every safepoint in the
compiled code emits a **stack map entry** describing which native registers
and stack slots hold heap pointers at that point:

```
StackMapEntry:
  4 bytes  pc_offset      (offset in the compiled function)
  2 bytes  ref_count
  ref_count entries:
    1 byte   location_kind (0=register, 1=stack_slot, 2=spill)
    1 byte   index
```

Stack maps are emitted as a side table next to the compiled code (analogous
to debug-sidecar — LANG13).  At collection time, the runtime walks the
native frame stack, finds each compiled frame's stack map for the current
PC, and yields each ref location's value to the collector.

### Deopt as a GC concern

When a guard fails, the JIT-compiled frame is replaced by an interpreter
frame.  The deopt path was already specified in LANG03; LANG16 adds: deopt
is a safepoint.  After deopt, the new interpreter frame's register file is
authoritative for root scanning; the abandoned compiled frame contributes
no roots.

### Write-barrier trampolines

For collectors that need barriers, the JIT emits a call to
`vm_gc_write_barrier(parent, child)` after every store of a ref into a
heap object.  The trampoline is provided by `gc_runtime_<target>.a`
(LANG15 reloc kind `RT_ENTRY_PCREL`).

---

## aot-core additions

AOT inherits everything jit-core does, plus two binary sections in `.aot`
(LANG04 snapshot format):

```
.aot file (extended):
  ...
  Stack map section (optional):
    Same StackMapEntry layout as jit-core, but for AOT-compiled native code.
  GC root table section (optional):
    Static roots — global variables, intern-table entries — that are alive
    for the whole program lifetime.  vm_init reads this section and pins
    these refs at startup.
```

The flags byte in the LANG04 header gains:

```
bit 2:  stack map section present
bit 3:  gc root table section present
```

AOT linking pulls in `gc_runtime_<target>.a` whenever bit 2 or bit 3 is
set.  Tetrad-style fully-typed programs leave both bits clear and pay no
cost.

---

## vm-runtime additions (LANG15 level 3)

LANG15 reserved level 3 for "profiler + shadow frames + GC hooks" without
defining the hooks.  LANG16 defines them:

```c
/* Register the language frontend's root scanner.  Called once at vm_init.
 * The scanner is invoked by the collector with an emit callback; the
 * scanner calls emit() for every heap address it finds.  */
void vm_gc_register_root_scanner(vm_instance_t *vm,
                                 void (*scan)(void *ctx, void (*emit)(uintptr_t)),
                                 void *ctx);

/* The collector calls this from inside scan() to register a root.  */
typedef void (*vm_gc_emit_t)(uintptr_t addr);

/* Allocate `size` bytes of heap memory tagged `kind`.  Returns the heap
 * address.  May trigger a collection.  */
uintptr_t vm_gc_alloc(vm_instance_t *vm, uint32_t size, uint16_t kind);

/* Write barrier — must be called by JIT/AOT code after any field_store
 * of a ref into a heap object.  No-op for collectors that don't need it.  */
void vm_gc_write_barrier(vm_instance_t *vm, uintptr_t parent, uintptr_t child);

/* Force a collection.  Mainly for tests.  */
void vm_gc_collect(vm_instance_t *vm);
```

These entry points live in `gc_runtime_<target>.a`, not
`vm_runtime_<target>.a`.  An AOT binary that uses heap opcodes links both;
an AOT binary that doesn't links neither.

---

## Adapter to the existing garbage-collector package

The current `GarbageCollector` ABC (in
[`garbage-collector/src/garbage_collector/gc.py`](../packages/python/garbage-collector/src/garbage_collector/gc.py))
already has the right shape.  LANG16 adds a thin adapter:

```python
# vm_core/helpers/gc_hooks.py

from garbage_collector import GarbageCollector

class VMGCAdapter:
    """Bridge between vm-core and a GarbageCollector implementation."""

    def __init__(self, vm: VMCore, gc: GarbageCollector) -> None:
        self._vm = vm
        self._gc = gc

    def maybe_collect(self) -> None:
        """Consult the collector; run a collection if it asks."""
        if self._gc.should_collect():
            freed = self._gc.collect(roots=self._vm.enumerate_roots())
            self._vm._stats.gc_freed += freed
            self._vm._stats.gc_runs += 1

    def alloc(self, size: int, kind: int) -> int:
        return self._gc.allocate(_layout_for(kind), size)

    def write_barrier(self, parent: int, child: int) -> None:
        self._gc.write_barrier(parent, child)
```

The adapter calls `should_collect()` on the GC.  This requires adding
`should_collect()` to the `GarbageCollector` ABC — a one-method addition
that defaults to `True` (mark-and-sweep is happy to collect on every
safepoint for simplicity, and the dispatcher's safepoint interval keeps
overhead bounded).  Production collectors will implement it based on
heap occupancy.

---

## Migration impact

LANG16 is a meaningful but contained refactor:

1. **interpreter-ir** (LANG01) — add 7 opcodes, the `ref<T>` type form, and
   the `may_alloc` field.  Existing IIR programs are unaffected (default
   `may_alloc=False`, no opcodes use the new types).
2. **vm-core** (LANG02) — add `enumerate_roots`, `_maybe_collect`, the
   `gc` config field, and the dispatch-loop safepoint check.  Programs
   without a configured GC see no behaviour change.
3. **garbage-collector** package — add `should_collect()` and
   `write_barrier()` to the ABC; mark-and-sweep gets trivial implementations.
4. **jit-core** (LANG03) — add stack-map emission alongside generated
   code; add write-barrier trampoline calls.
5. **aot-core** (LANG04) — add stack-map and root-table sections to `.aot`;
   set the new header flags.
6. **vm-runtime / gc-runtime** (LANG15) — add the GC C ABI; build
   `gc_runtime_<target>.a` per target.

PR sequencing within LANG16 work:

1. IIR opcode + type + flag additions (with tests, no callers yet)
2. vm-core enumerate_roots + dispatch additions (with a Lisp-toy test)
3. garbage-collector ABC additions
4. jit-core stack maps
5. aot-core sections
6. vm-runtime / gc-runtime ABI

Steps 1–3 unblock a Lisp frontend that runs interpreted-only.  Steps 4–6
extend GC support to JIT and AOT respectively, and can land independently
in either order.

---

## Out of scope

- **Choosing a default collector** — LANG16 wires *whatever* `GarbageCollector`
  the language frontend chooses.  Mark-and-sweep is the only one that
  exists today; generational and concurrent collectors are future work.
- **Heap layout optimisation** — object layouts are kind-based and
  language-defined.  Type-feedback-driven layout (e.g., V8 hidden classes)
  is out of scope.
- **Finalizers with ordering guarantees** — finalizers run after the
  object is unreachable; ordering between finalizers is unspecified.
- **Weak references** — can be implemented as a library on top of
  `is_null` + an explicit ref table maintained by the language frontend;
  no IIR opcode needed.
- **Cross-VM heap sharing** — every `vm_instance_t` owns its own heap.

---

## Public API summary

```python
# Frontend uses
from interpreter_ir import IIRInstr  # gains ref<T>, may_alloc, 7 new opcodes
from vm_core import VMCore, VMConfig
from garbage_collector import MarkAndSweepGC

vm = VMCore(VMConfig(
    register_count=8,
    opcodes=LISP_OPCODES,
    gc=MarkAndSweepGC(),
))
vm.execute(module)
```

```c
/* AOT binary uses (level 3 + gc-core) */
vm_gc_register_root_scanner(vm, my_scan, my_ctx);
uintptr_t cell = vm_gc_alloc(vm, sizeof(ConsCell), KIND_CONS);
vm_gc_write_barrier(vm, parent_cell, cell);
```

---

## Relationship to other specs

| Spec | Relationship |
|------|--------------|
| LANG01 (interpreter-ir) | LANG16 adds opcodes, type, and flag fields |
| LANG02 (vm-core) | LANG16 adds root scanning + safepoint dispatch |
| LANG03 (jit-core) | LANG16 adds stack maps + barriers |
| LANG04 (aot-core) | LANG16 adds stack-map and root-table sections |
| LANG15 (vm-runtime) | LANG16 fills in level-3 GC hooks; adds gc_runtime_<target>.a |
| garbage-collector | LANG16 adopts existing ABC; adds two methods |

LANG16 is the largest of the LANG specs because it is the one that
**touches every layer of the stack**.  Once landed, the LANG pipeline can
host any garbage-collected language (Lisp, Scheme, ML, JavaScript) on the
same infrastructure that runs Tetrad.
