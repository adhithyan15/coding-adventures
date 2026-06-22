# LANG-FULL E6 (layer 1) — typed module globals accessible from functions

**Status:** spec / specs-first, for sign-off.
**Enabler:** E6. **Scope of this doc:** the tractable, *run-verifiable* first layer
of E6 — a **typed (`i64`) module-level global** that a *function body* can read and
write, working by **running** on **all 7 backends**. The broader E6 (general
`call_builtin` / closures / dynamic dispatch on the code-gen backends — roadmap
line 217) layers on top of this and is **out of scope here**.

## Why this layer first

E6 in the roadmap is the "dynamic dispatch" fork. But the *concrete* thing every
blocked item actually needs first is humbler: **a function reading/writing a
variable that outlives its own frame.** That unblocks, with no closure machinery:

- **ALGOL `own` variables (AL6)** and procedures that read an enclosing-block
  variable — the canonical typed global.
- **Oct `static` globals (O3)** — `out`-verifiable.
- **BASIC `GOSUB`-adjacent** shared state.
- The storage substrate every later closure/upvalue design will reuse.

It is also *run-verifiable* end-to-end (a typed program that computes a number),
unlike the dynamically-typed Twig global path (see "The frontend problem" below).

## Current state (surveyed)

The IIR ops already exist (LANG32) — `global_load("name") -> %dest` and
`global_store("name", %v)`, with the name carried as an `Operand::Str` (never a
register). Per-backend status **today**:

| Backend | global_load / global_store | Storage model |
|---|---|---|
| **BEAM** (`iir-to-beam`) | ✅ works | `erlang:put/get` process dict |
| **WASM** (`iir-to-wasm`) | ✅ works | module mutable `global` (`global.get/set`) |
| **native x86_64** (`x86_64-backend`) | ✅ works | `_twig_globals` data section, PcRel32 + `slot*8` |
| **native aarch64** (`aarch64-backend`) | ✅ works | `_twig_globals` `__DATA`, ADRP/ADD + `slot*8` (≤4095 slots) |
| **VM** (`vm-core`) | ⚠ **verify** | (no dedicated impl found — confirm it executes the ops) |
| **JIT** (`jit-core`) | ⚠ **verify** | (same) |
| **LLVM** (`iir-to-llvm`) | ❌ rejected | not in `SUPPORTED_OPS` whitelist |
| **JVM** (`iir-to-jvm-class-file`) | ❌ rejected | `UnsupportedOp "… LANG32b"` |
| **CLR** (`iir-to-cil-bytecode`) | ❌ rejected | `UnsupportedOp "… LANG32b"` |

So the backend work is: **LLVM, JVM, CLR** (the three `LANG32b`-deferred
rejections), plus **confirming VM/JIT** actually run the ops.

### The frontend problem

The Twig frontend *does* emit globals — but **dynamically typed**. A function
that reads a top-level value captured by a lambda emits
`call_builtin "global_get"` (→ `global_load`, `type_hint = "any"`) and its
arithmetic stays `call_builtin "+"` (the `any` path). The code-gen backends
reject that `any` arithmetic **independently** of globals, so the dynamic Twig
path can't give a clean cross-backend *run* proof. We therefore drive the proof
from a **statically-typed** frontend (ALGOL), whose globals are `i64`.

## Design

### Storage model per backend (all 8-byte / `i64` slots)

- **LLVM**: one module global per name — `@__twig_global_<name> = internal global i64 0`.
  `global_load` → `load i64, ptr @…`; `global_store` → `store i64 %v, ptr @…`.
  Add `global_load`/`global_store` to `SUPPORTED_OPS`.
- **JVM**: add a `fields` table to `JvmClassFile`; one `static long G_<name>;`
  per global. `global_load` → `getstatic` (0xB2); `global_store` → `putstatic`
  (0xB3). Names assigned a slot/field lazily on first encounter (mirrors the
  native `global_slots` map).
- **CLR**: add a static-fields table to the program artifact; one
  `.field static int64 G_<name>` per global. `global_load` → `ldsfld`;
  `global_store` → `stsfld`.
- **native / wasm / beam**: unchanged (already correct).

Slots/fields are allocated lazily, consistently with the existing native
`collect_global_slots` ordering, so a program's globals get stable identities
across backends.

### Frontend (ALGOL enclosing-scope variable)

A procedure that reads/writes a variable declared in an **enclosing block**
lowers that access to `global_load`/`global_store` (typed `i64`) instead of the
current "only your own parameter is in scope" rejection. Concretely:

```algol
begin integer counter;
  integer procedure bump; bump := counter := counter + 1;
  counter := 40;
  result := bump + 1     ⇒ 42
end
```

`bump`'s `counter` references lower to `global_load "counter"` /
`global_store "counter"`. The enclosing-block scalar becomes a module global
(slot in `_twig_globals` / a static field / an LLVM global). This is the typed,
run-verifiable proof program.

(BASIC and Oct globals can follow the same shape in later slices; ALGOL is first
because its block scoping makes "enclosing variable = global" the natural model.)

## PR breakdown (each its own small PR, run-verified) — **LAYER 1 COMPLETE ✅**

0. ✅ **E6-spec** — this doc (#6490).
1. ✅ **E6-verify-vm-jit** (#6495) — survey showed VM/JIT had *no* handling of the
   lowered ops (only the dynamic `call_builtin` table); added a name-keyed
   `globals` map + the two handlers; JIT cold-interprets on it. RUN ⇒ 42.
2. ✅ **E6-llvm** (#6499) — index-based `@__twig_global_N = internal global i64`
   + load/store. RUN on real `clang` ⇒ 42.
3. ✅ **E6-jvm** (#6503) — `JvmFieldInfo`/`fields` on `JvmClassFile` + `static
   long G_N` + `getstatic`/`putstatic`. RUN on real `java` ⇒ 42. (0.17.1 fixed a
   `global_load`→i32-dest `l2i` narrowing the matrix proof caught.)
4. ✅ **E6-clr** (#6510) — `.field public static int64 G_N` + `ldsfld`/`stsfld`
   (+`conv` at the i32 boundary). RUN on real `ilasm`+`dotnet` ⇒ 42.
5. ✅ **E6-algol-frontend** (#6514) — capture analysis: a procedure that reads/
   writes an enclosing-block scalar materialises it as a typed global. RUN on VM
   ⇒ 42.
6. ✅ **E6-matrix** — the ALGOL global program (`incr(2)` over a shared `counter`
   ⇒ 42) RUNS on **all 7 backends** in `lang_matrix.rs`. The E6-layer-1
   completion proof. (Procedure named `incr`, not `add`: `add` is a CIL opcode.)

**E6 layer 1 (typed module globals accessible from functions) is DONE — every
backend runs the same typed ALGOL global program.** Follow-ups (separate, out of
this layer): general `any`-dispatch / closures (the broader E6); CLR reserved-
word identifier quoting; nested-procedure capture; `array`/`f64` globals.

## Out of scope (later E6 layers)

- General `call_builtin` / dynamic `+`/`-`/… on the code-gen backends (the `any`
  arithmetic path) — needed for the *Twig* dynamic globals and for TW5 closures.
- Closures / captured upvalues (heap cells, not module globals).
- `f64` / non-`i64` globals (the slot is 8 bytes; `f64` reinterpret is a small
  follow-up once typed-`i64` lands).

## Verification standard

Every implementation PR is verified by **running** the backend's output (the
LANG-FULL standard), not byte-comparison alone — `clang` for LLVM, `java` for
JVM, `dotnet` for CLR, the in-repo runtimes for WASM/VM/JIT, a real executable
(or the `x86-simulator`) for native. The E6-matrix PR runs the same typed ALGOL
global program on all 7 and asserts exit code 42.
