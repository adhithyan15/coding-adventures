# LANG15 — vm-runtime: Linkable Runtime Library

## Overview

`vm-runtime` is the **linkable, embeddable form of `vm-core`** (LANG02).  It
exists so that AOT-compiled binaries (LANG04) can ship without depending on a
host Python interpreter, while still falling back to interpretation for the
parts of a program that could not be fully specialised.

LANG04 already names `vm-runtime` and shows it as a box in the AOT pipeline,
but never pinned down:

- Which `vm-core` symbols are public ABI vs. internal helpers
- How the IIR table is laid out inside an `.aot` file
- How a native-code function calls back into the interpreter
- How the runtime is built per target (Intel 4004 vs. Intel 8080 vs. RISC-V)
- How root scanning and GC hooks compose with the runtime (forward reference
  to LANG16 — gc-core)

LANG15 nails those down.  It is to LANG04 what LANG13 (debug-sidecar) is to
LANG06 (debug integration): the on-disk and ABI contract that everything else
hangs off.

This spec covers:

1. The split of `vm-core` into `core/` (dispatch) and `helpers/` (handlers,
   profiler, builtins) so that AOT can link a *subset*
2. The `vm-runtime` C ABI — entry points an AOT binary can call
3. The `vm_iir_table` binary format — how IIR for unspecialised functions is
   embedded in the `.aot` file
4. The relocation contract — how the linker resolves stub call sites to
   runtime entry points
5. The build matrix — pre-compiled `vm_runtime_<target>.a` artefacts, naming,
   and where they live
6. The Python in-process variant — a development convenience that satisfies
   the same ABI through Python callables

---

## Why this is a separate spec

`vm-core` (LANG02) is a Python-resident interpreter.  Its public API is
Python-level: `VMCore(...).execute(module)`.  An AOT binary running on an
Intel 4004 has no Python interpreter, no Python objects, and no Python heap.

The same dispatch loop and opcode handlers must therefore exist in **two
forms**:

| Form | Used by | Calling convention |
|------|---------|---------------------|
| `vm-core` (Python) | tests, REPL, JIT path, dev tooling | Python method calls |
| `vm-runtime` (native) | AOT binaries | Target-arch C ABI |

`vm-runtime` is the second form.  It is the same code, recompiled to native
machine code through the LANG05 backend protocol, packaged as a static library
that an AOT binary links against.

If `vm-core` is monolithic — handlers, profiler, builtins, and shadow-frame
support all entangled — then the AOT binary either pays for everything or
gets nothing.  LANG15 splits `vm-core` so that AOT binaries link only what they
need.

---

## Subset levels

Different AOT targets need different amounts of runtime.  LANG15 defines four
**runtime levels**:

| Level | Name        | Includes                                          | Typical use |
|-------|-------------|---------------------------------------------------|-------------|
| 0     | `none`      | No runtime; pure native code                      | Fully typed Tetrad on Intel 4004 |
| 1     | `minimal`   | Dispatch loop + arithmetic/control-flow handlers  | Mostly typed program with one polymorphic helper |
| 2     | `standard`  | Level 1 + builtins registry + I/O                 | Programs that call `print`, `input`, file I/O |
| 3     | `full`      | Level 2 + profiler + shadow frames + GC hooks     | Hybrid AOT/JIT (AOT cold path, JIT hot path) |

The AOT compiler picks the lowest level that satisfies the program.  Tetrad
programs end up at level 0 (zero runtime, zero overhead — the original Tetrad
ROM-only story).  Lisp programs with macros end up at level 3.

---

## vm-core refactor: `core/` vs. `helpers/`

The current [vm-core/src/vm_core/](../packages/python/vm-core/src/vm_core/)
package layout is monolithic.  LANG15 reorganises it as:

```
vm-core/
  src/vm_core/
    core/                      # Level 1 — always linked
      __init__.py
      dispatch.py              # dispatch_loop, _dispatch
      frame.py                 # VMFrame, RegisterFile
      errors.py                # VMError, FrameOverflowError
      handlers_arith.py        # add, sub, mul, div, mod, neg
      handlers_bitwise.py      # and, or, xor, shl, shr, not
      handlers_compare.py      # cmp_eq, cmp_lt, cmp_gt, cmp_le, cmp_ge
      handlers_control.py      # jmp, jmp_if, jmp_if_false, label, ret
      handlers_mem.py          # const, load_reg, store_reg, load_mem, store_mem
      handlers_call.py         # call, call_indirect (intra-module only)

    helpers/                   # Levels 2+ — linked on demand
      __init__.py
      builtins.py              # builtins registry + standard builtins
      io.py                    # io_in, io_out, print, input
      profiler.py              # VMProfiler — type observation
      shadow.py                # shadow frame suspend/resume for JIT deopt
      metrics.py               # VMMetrics dataclass
      gc_hooks.py              # forward declared by LANG16; stubs here

    abi/                       # The C-ABI surface (LANG15 §"vm-runtime C ABI")
      __init__.py
      entrypoints.py           # vm_execute, vm_call_builtin, vm_resolve_iir
      iir_table.py             # vm_iir_table reader/writer
      reloc.py                 # relocation entry types

    config.py                  # VMConfig
    core.py                    # VMCore — top-level Python facade
    __init__.py
```

**Backwards-compat:** `vm_core.VMCore`, `vm_core.VMConfig`, `vm_core.VMMetrics`
remain importable from `vm_core` as before — the refactor is internal.  The
existing test suite must pass unchanged after the move.

The split rule is mechanical: anything that an AOT level-1 program needs goes
in `core/`; anything else goes in `helpers/`.  This means a level-1 build of
`vm-runtime` links exactly the files in `core/` and the `abi/` shim — nothing
from `helpers/`.

---

## vm-runtime C ABI

Native AOT binaries call back into `vm-runtime` through a stable C ABI.  All
entry points use the C calling convention of the target architecture.

### Header (level 1 — minimal)

```c
/* vm_runtime.h — generated alongside each vm_runtime_<target>.a build */

/* Opaque handle to a VM instance (one per AOT binary, created at startup). */
typedef struct vm_instance vm_instance_t;

/* Result of a vm_execute call.  Tag discriminates the union. */
typedef enum {
    VM_RESULT_VOID = 0,
    VM_RESULT_U8   = 1,
    VM_RESULT_U16  = 2,
    VM_RESULT_U32  = 3,
    VM_RESULT_U64  = 4,
    VM_RESULT_BOOL = 5,
    VM_RESULT_STR  = 6,
    VM_RESULT_REF  = 7,    /* heap pointer, opaque to vm-runtime; LANG16 */
    VM_RESULT_TRAP = 0xFF, /* execution trapped; see trap_code */
} vm_result_tag_t;

typedef struct {
    vm_result_tag_t tag;
    union {
        uint64_t u;        /* fits all integer/bool/ref variants */
        const char *s;     /* str — null-terminated, owned by vm-runtime */
        uint16_t trap_code;
    } v;
} vm_result_t;

/* Initialise a VM instance.  Called once at AOT binary startup.
 * iir_table and table_size point at the embedded vm_iir_table section.  */
vm_instance_t *vm_init(const uint8_t *iir_table, uint32_t table_size);

/* Tear down (called at AOT binary shutdown; optional on hosts without atexit). */
void vm_shutdown(vm_instance_t *vm);

/* Execute the function at fn_index in the IIR table.  args is a packed array
 * of vm_result_t-sized slots; argc is the count.  */
vm_result_t vm_execute(vm_instance_t *vm,
                       uint32_t fn_index,
                       const vm_result_t *args,
                       uint32_t argc);
```

### Header additions for level 2 (standard)

```c
/* Call a registered builtin by index (resolved at link time from the
 * builtins registry — no string lookups at runtime).  */
vm_result_t vm_call_builtin(vm_instance_t *vm,
                            uint16_t builtin_index,
                            const vm_result_t *args,
                            uint32_t argc);
```

### Header additions for level 3 (full)

```c
/* JIT/deopt — only present when the AOT binary embeds a JIT compiler too,
 * which is rare; documented for completeness.  */
vm_result_t vm_resume_at(vm_instance_t *vm,
                         uint32_t fn_index,
                         uint32_t ip,
                         const vm_result_t *registers,
                         uint32_t register_count);

/* GC hooks — forward declared here, defined in LANG16.  */
void vm_gc_register_root_scanner(vm_instance_t *vm,
                                 void (*scan)(void *ctx, void (*emit)(uintptr_t)),
                                 void *ctx);
```

### Why integer indices, not strings

Builtin lookups, function lookups, and IIR table indexing all use 16-bit or
32-bit indices, never strings.  The string names are resolved **at link time**
by the LANG10 linker (see "Relocation contract" below).  Runtime overhead is a
single array access — comparable to a virtual method dispatch.

This is the same trick the JVM uses for `invokevirtual`: the verifier
resolves method names to vtable indices at class load time so the interpreter
never does a string lookup.

---

## vm_iir_table binary format

The `vm_iir_table` section of an `.aot` file is a flat, position-independent
encoding of every `IIRFunction` that the AOT compiler could not fully
specialise.  It uses the LANG01 wire format with a small index header.

```
vm_iir_table:
  Header (16 bytes):
    4 bytes  magic        0x49 0x49 0x52 0x54  ("IIRT")
    2 bytes  version      0x01 0x00
    2 bytes  flags        bit 0: little-endian; bit 1: index 64-bit (default 32)
    4 bytes  function_count
    4 bytes  index_offset  (byte offset of the function index table)

  Function index table (function_count * 8 bytes):
    For each function, two 32-bit fields:
      4 bytes  name_offset   (offset into the string pool below)
      4 bytes  body_offset   (offset into the IIR body section)

  IIR body section (variable):
    Concatenated LANG01-encoded IIRFunction bodies.

  String pool (variable):
    Null-terminated UTF-8 names (function names, register names, opcodes).
```

The index is sorted by `name_offset` so that name → index resolution at link
time is a binary search.  At runtime the AOT binary always passes the integer
index directly — the name is only used for debugging.

The format intentionally mirrors the [debug-sidecar](LANG13-debug-sidecar.md)
file format so that the same writer/reader scaffolding can be reused.

---

## Relocation contract

When the AOT compiler emits a call to an unspecialised function, it does not
know the function's index in the final IIR table — that is assigned by the
linker after all functions have been laid out.  The compiler emits a
**relocation entry** instead:

```
Relocation entry (16 bytes):
  4 bytes  site_offset     (byte offset in the native code section)
  4 bytes  symbol_offset   (offset into the relocation string pool)
  2 bytes  reloc_kind      (see table below)
  2 bytes  addend          (constant added to the resolved value)
  4 bytes  reserved        (zero for now; future: section index)
```

| reloc_kind | Name             | Resolves to                               |
|------------|------------------|-------------------------------------------|
| 0x0001     | `IIR_FN_INDEX`   | 32-bit index into vm_iir_table            |
| 0x0002     | `BUILTIN_INDEX`  | 16-bit index into builtins registry       |
| 0x0003     | `RT_ENTRY_ABS`   | Absolute address of a vm-runtime entry    |
| 0x0004     | `RT_ENTRY_PCREL` | PC-relative offset to a vm-runtime entry  |
| 0x0005     | `STRING_POOL`    | Offset of a string in the .aot string pool|
| 0x0006     | `GC_ROOT_TABLE`  | Offset into the GC root table (LANG16)    |

The linker walks the relocation list and patches each `site_offset` in the
native code section with the resolved value.  This is the same mechanism ELF
uses for `R_X86_64_PC32`, `R_X86_64_64`, etc. — we just keep our own kind
namespace because we are not constrained by Unix history.

---

## Build matrix

`vm-runtime` is pre-compiled per (target_arch, runtime_level) pair.  The
artefacts live in:

```
code/packages/python/vm-core/
  prebuilt/
    vm_runtime_intel4004_l0.a       /* never built — level 0 has no runtime */
    vm_runtime_intel8080_l1.a
    vm_runtime_intel8080_l2.a
    vm_runtime_intel8080_l3.a
    vm_runtime_riscv32_l1.a
    vm_runtime_riscv32_l2.a
    vm_runtime_riscv32_l3.a
    vm_runtime_x86_64_l1.a
    vm_runtime_x86_64_l2.a
    vm_runtime_x86_64_l3.a
    vm_runtime_wasm32_l1.a
    vm_runtime_wasm32_l2.a
    vm_runtime_wasm32_l3.a
```

Builds are produced by a `vm-runtime-builder` program (separate from
`aot-core`) that takes:

- A target triple (e.g., `riscv32-unknown-none`)
- A backend implementing LANG05 for that target
- A runtime level (1, 2, or 3)

…and produces a `.a` static library plus a `vm_runtime.h` header.  The builder
is itself just an `aot-core` invocation against the `vm-core/abi/` and
`vm-core/core/` (and optionally `vm-core/helpers/`) Python sources, lowered to
IIR, then to native via the backend.  In other words, **`vm-runtime` is built
by `aot-core` compiling `vm-core`** — the runtime bootstraps itself.

For the bring-up (PR3 in the migration plan), only the in-process Python
variant is required.  Real per-target builds land later, as backends mature.

---

## Python in-process variant

For development and tests, an `InProcessVMRuntime` provides the same ABI
surface backed by Python callables:

```python
from vm_core.abi import InProcessVMRuntime
from interpreter_ir import IIRModule

rt = InProcessVMRuntime.from_module(module)
result = rt.vm_execute(fn_index=0, args=[42, 7])
```

`InProcessVMRuntime` exists so that AOT path tests (LANG04) can run end-to-end
without a target backend — the "AOT binary" in tests is just a Python object
that satisfies the ABI.  Once a real backend lands (e.g., RISC-V), the same
test can be re-run against the real `.a` library through a tiny ctypes shim.

---

## Interaction with LANG16 (gc-core)

`vm-runtime` includes GC hooks at level 3.  LANG16 (gc-core) defines:

- The set of allocation opcodes (`alloc`, `box`, …) the dispatcher must handle
- The root-scanning callback signature
- The write-barrier trampoline ABI
- The GC support library (`gc_runtime_<target>.a`) shipped alongside vm-runtime

LANG15 deliberately stops at "there is a hook"; the contents of the hook are
LANG16's responsibility.  An AOT binary that uses `alloc` opcodes pulls in
both `vm_runtime_<target>_l3.a` and `gc_runtime_<target>.a`.

---

## Migration impact

This spec implies a **non-trivial refactor of vm-core** and clarifies (does
not break) LANG04.  The migration order is:

1. Add `vm-core/src/vm_core/{core,helpers,abi}/` directory layout, move files
   accordingly, keep public re-exports.  All existing tests pass unchanged.
2. Land the `InProcessVMRuntime` Python variant — covered by new tests in
   `vm-core/tests/test_abi.py`.
3. Update `aot-core/src/aot_core/vm_runtime.py` to delegate to
   `InProcessVMRuntime` for in-process AOT.
4. Define the `vm_iir_table` writer in `aot-core/src/aot_core/snapshot.py` and
   reader in `vm-core/src/vm_core/abi/iir_table.py`.
5. Backends/relocation/per-target prebuilts land later, as targets mature.

PRs land in that order.  Steps 1–4 are required before Tetrad can migrate
(PR4 in the master plan); steps after are independent.

---

## Out of scope

- **Dynamic loading of vm-runtime** — `vm-runtime` is always statically linked
  into the AOT binary.  No `dlopen`, no shared libraries.
- **ABI versioning across major releases** — header `version` field is reserved
  for forward compatibility but no migration story is defined yet.
- **Inlining vm-runtime entry points** — calls into vm-runtime are always real
  function calls; whole-program LTO across the boundary is out of scope.
- **Profile-guided runtime level selection** — picking level 1 vs. 2 is
  driven by a static scan of the IIR module, not profile data.

---

## Public API summary

```python
# Python-side (development/test)
from vm_core.abi import InProcessVMRuntime, IIRTableWriter, IIRTableReader

rt = InProcessVMRuntime.from_module(module)
rt.vm_execute(fn_index, args)        -> VMResult
rt.vm_call_builtin(builtin_index, args) -> VMResult

writer = IIRTableWriter()
writer.add_function(name, iir_function)
blob = writer.serialise()             -> bytes (the vm_iir_table section)

reader = IIRTableReader(blob)
fn_index = reader.lookup("main")
iir = reader.get(fn_index)
```

```c
/* Native-side (AOT binaries) — see "vm-runtime C ABI" above */
vm_instance_t *vm_init(...);
vm_result_t   vm_execute(...);
vm_result_t   vm_call_builtin(...);     /* level 2+ */
void          vm_gc_register_root_scanner(...); /* level 3, LANG16 */
```
