# TW03 Phase 3 — heap primitives across the native backends

## Why this spec exists

[TW03](TW03-lisp-primitives-and-gc.md) Phase 3 is "Lisp on every
backend": cons cells, `car`, `cdr`, `null?`, `pair?`, symbols,
`quote`, and `nil` running natively on JVM / CLR / BEAM (and
later WASM) without falling back to vm-core interpretation.

Phase 2 (closures) shipped the precedent for "object-typed
register slot" lowering in CLR Phase 2c.5 / JVM Phase 2c.5.  This
spec extends the same pattern from one boxed reference kind
(closures) to four (closures + cons + symbol + nil), so the
existing typed-pool plumbing in
[ir-to-jvm-class-file](../packages/python/ir-to-jvm-class-file/)
and [ir-to-cil-bytecode](../packages/python/ir-to-cil-bytecode/)
already does most of the work — we just teach the heap-primitive
opcodes how to allocate and inspect.

## Acceptance criterion

```python
>>> from twig_jvm_compiler import run_source
>>> run_source(
...   "(define (length xs)"
...   "  (if (null? xs) 0 (+ 1 (length (cdr xs)))))"
...   "(length (cons 1 (cons 2 (cons 3 nil))))",
...   class_name="Length",
... ).stdout.strip()
b'\\x03'   # = 3
```

The same source must produce exit-code 3 on real `dotnet` via
`twig_clr_compiler.run_source` and on real `erl` via
`twig_beam_compiler.run_source`.

## Cross-backend value model

The IR-level convention introduced for closures in Phase 2c.5 is
unchanged: every IR register has both an int slot and an
object-reference slot, and per-register typing decides which slot
each operation reads or writes.  Phase 3 adds three new
reference-kind tags inside the object slot:

| Tag    | Backend representation (JVM / CLR / BEAM) |
|--------|--------------------------------------------|
| `int`     | unboxed int (existing) |
| `closure` | `Closure_<name>` instance (Phase 2 — existing) |
| `cons`    | `coding_adventures.twig.runtime.Cons` instance |
| `symbol`  | interned `coding_adventures.twig.runtime.Symbol` |
| `nil`     | a single sentinel `Nil` instance (per assembly) |

On BEAM the mapping is even more direct: cons → `[H \| T]`,
symbol → atom, nil → `[]`.  No extra "runtime" classes needed.

## New IR ops (this spec — Phase 3a)

Added to `compiler-ir` v0.5.0 (opcodes 55–62, leaving the
existing 0–54 stable so older serialized IR text round-trips):

```
MAKE_CONS    dst, head_reg, tail_reg
CAR          dst, src
CDR          dst, src
IS_NULL      dst, src
IS_PAIR      dst, src
MAKE_SYMBOL  dst, name_label
IS_SYMBOL    dst, src
LOAD_NIL     dst
```

`name_label` reuses `IrLabel` — the existing IR text format
already round-trips identifier-shaped labels, so symbol names
travel through `print_ir` / `parse_ir` for free.

`IS_NULL`, `IS_PAIR`, `IS_SYMBOL` write into an int-typed
register so the result feeds straight into `BRANCH_Z` /
`BRANCH_NZ` without the boxing dance the closure ops needed.

## Implementation phases

This is a multi-PR change; staging mirrors what worked for
Phase 2:

### Phase 3a — IR ops + scaffolding (this PR)

- Add the eight opcodes to `compiler-ir`.
- Tests: opcode-numbering invariant + parse/print round-trip
  for every new op.
- No backend lowering yet — that lives in 3b/3c/3d.
- This unblocks parallel work on JVM03, CLR03, BEAM03.

### Phase 3b — JVM heap primitives (JVM03).  **Shipped.**

- `coding_adventures.twig.runtime.Cons` (final fields `int head`
  + `Object tail`), `Symbol` (`String name`, static
  HashMap-backed `intern(String) Symbol`), and a `Nil` singleton
  sentinel — all auto-included in the multi-class JAR by
  `lower_ir_to_jvm_classes` whenever a heap opcode appears.
- `MAKE_CONS` lowers to `new Cons; dup; iload head; aload tail;
  invokespecial Cons.<init>(I,LObject;)V; aastore
  __ca_objregs[dst]`.
- `CAR` lowers to `aaload __ca_objregs[src]; checkcast Cons;
  getfield head:I → __ca_regs[dst]`; `CDR` mirrors with
  `getfield tail:Object → __ca_objregs[dst]`.
- `IS_NULL` lowers to an `if_acmpne` identity test against
  `Nil.INSTANCE`; `IS_PAIR` / `IS_SYMBOL` lower to
  `instanceof Cons` / `instanceof Symbol`.  All three write
  their 0/1 result into `__ca_regs[dst]` so it feeds straight
  into `BRANCH_Z`.
- `MAKE_SYMBOL` lowers to `ldc "name"; invokestatic
  Symbol.intern(String) Symbol; aastore __ca_objregs[dst]`.
- `LOAD_NIL` lowers to `getstatic Nil.INSTANCE; aastore
  __ca_objregs[dst]`.

End-to-end proof on real `java`:
`(length (cons 1 (cons 2 (cons 3 nil)))) → 3` runs and exits
with stdout = `\x03`.

Limitation (intentional, scoped to a follow-up):
`Cons.head` is typed `int` so the spec acceptance criterion
(list-of-ints) works without typed-register inference for cons
head slots.  Heterogeneous cells (`(cons 'foo nil)`) need a
follow-up that widens `head` to `Object` with autoboxing and
threads typing for the head-read site.

### Phase 3c — CLR heap primitives (CLR03)

- Same shape as JVM but as additional `TypeDef` rows in the
  single PE/CLI assembly (the multi-TypeDef plumbing from
  CLR02 Phase 2b carries over verbatim).
- `Cons` / `Symbol` / `Nil` types under the assembly's
  namespace; `Symbol.Intern(string)` static method.
- `IS_PAIR` / `IS_SYMBOL` lower to `isinst` + `ldnull; cgt.un`.

### Phase 3d — BEAM heap primitives (BEAM03)

- `MAKE_CONS` lowers to `put_list head, tail, dst`.
- `CAR` / `CDR` lower to `get_hd` / `get_tl`.
- `IS_NULL` lowers to `is_nil` test op (matches `[]`).
- `IS_PAIR` lowers to `is_nonempty_list`.
- `MAKE_SYMBOL` lowers to a literal atom in the atom table
  (BEAM atoms are global per VM, so "interning" is free).
- `IS_SYMBOL` lowers to `is_atom`.
- `LOAD_NIL` lowers to `move nil dst` (the `[]` literal).

### Phase 3e — Twig frontend acceptance

- `twig.parser` already accepts `cons`, `car`, `cdr`, `null?`,
  `pair?`, `quote` / `'foo`, `nil` (the AST nodes exist for
  the vm-core path).  Currently `twig_jvm_compiler` /
  `twig_clr_compiler` / `twig_beam_compiler` raise
  `TwigCompileError` for these; the per-backend Phase 3e
  removes the rejection and emits the new IR ops.
- New end-to-end tests: `length`, `reverse`, `member?`,
  symbol-based dispatch — all running on real
  `java` / `dotnet` / `erl`.

## Risk register

- **Per-program nil sentinel.**  JVM and CLR each need a single
  `Nil` instance shared across all closure / heap operations
  in the same assembly.  Mitigation: emit a static `NIL` field
  on the program's main class, initialised in `<clinit>`.
- **Symbol intern table thread safety.**  HashMap-based interns
  are not thread-safe.  Twig today is single-threaded; document
  the limit and revisit if/when we add a thread spec.
- **Tagged-pointer migration.**  TW03's headline design quotes a
  64-bit tagged pointer; the closure work shipped a parallel
  object-slot pool instead.  Phase 3 stays consistent with the
  shipped closure design — no migration in this phase.  If we
  later add an FFI / WASM backend that genuinely needs tagged
  pointers, that's a separate spec.
- **`IS_PAIR` on closure / symbol values.**  Returns 0 (good).
  The Cons/Symbol/Closure classes are unrelated, so
  `instanceof Cons` correctly says "no" on a Symbol or Closure
  reference.  Tests will lock this down.

## Out of scope

- **Mutation** (`set-car!`, `set-cdr!`).  Cons cells are
  immutable in TW03 v1.  R5RS allows mutation but our cons
  fields are `final` for simplicity.  Add a separate spec if a
  use case appears.
- **Vector / hashmap / record types.**  Pure-Lisp v1 is just
  cons + atom + nil.
- **Equal-symbols-by-name across assemblies.**  Each compiled
  assembly has its own intern table.  Two Twig programs running
  in the same JVM will have separate `'foo` symbols.  Document
  it; revisit if cross-assembly symbol equality matters.
- **GC** — TW03 Phase 4 territory.  On JVM/CLR/BEAM the host GC
  reclaims our objects automatically once Phase 3 lands; no
  extra compiler work needed.  On WASM, see TW04.
