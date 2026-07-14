# E6D7 — closures on WASM (LANG-FULL E6d-7, TW5)

**Status:** Draft — 2026-07-12 (spec-first design note; sign-off = merge)
**Parent:** `lang-full-e6-dispatch.md` §3.4 + §4 item 7 (this note fulfils the
"⚠ design note first" gate that entry sets).
**Goal:** close the **last E6 backend gap** — a Twig/lisp closure (`lambda` /
`LABEL` capturing free variables) runs on **WASM**, so closures work on all five
code-gen backends `[NativeAot, Llvm, Wasm, Jvm, Clr]`, matching the interpreter.

## 0. One-paragraph summary

JVM and CLR already run closures; NativeAot and LLVM run them through the C
runtime; **WASM hard-rejects `alloc_closure`/`call_closure`** (`ClosureOpcode`
in `iir-to-wasm/src/validate.rs`). Nothing about a closure is WASM-hostile: a
closure is just a **heap object holding a dispatch index + captured values**, and
calling it is a **switch on that index to a direct call of the body**. WASM
already has both halves — the `alloc`/`field_load`/`field_store` heap path (the
same one `cons`/records ride) and `br_table`. This note commits to **option (a)
from §3.4**: lower a WASM closure to the **heap form** and synthesize a
**dispatcher function** — the exact WASM twin of the JVM/CLR `__callClosure`
switch — so **no new WasmGC `funcref`/`call_indirect`/table type is introduced**.

## 1. Decision (surfacing the §3.4 choice)

§3.4 offered two options:

- **(a)** heap-form lowering + a generated dispatcher (reuse the shared heap
  substrate; a closure is a heap object like a cons). *Preferred.*
- **(b)** a native WasmGC `$Closure` struct + `call_indirect` over a `funcref`
  table.

**We take (a).** Rationale:

1. **Reuses the proven substrate.** The closure's *data* rides the identical
   `alloc`/`field_*` heap path that `cons`, `list`, records, and unions already
   lower to on WASM (E6d-1/3/5/6, all shipped) — zero new value model, zero new
   WasmGC type, so it stays within the `anyref`/`i31ref` world the other dynamic
   ops already validate against.
2. **Mirrors JVM/CLR exactly.** Those backends do *not* use indirect function
   references either: they synthesize a `__callClosure(long[], long[])` /
   `object[]` dispatcher that reads the closure's dispatch index and runs an
   if-chain (`lload idx; ldc target; lcmp; ifeq case_N`) to a **direct**
   `invokestatic`/`call` of each closure body. Cross-backend *agreement* (the
   matrix invariant) is easiest when WASM uses the same shape.
3. **`call_indirect` + a `funcref` table (b)** would add a WasmGC type, a table
   section, and element-segment plumbing for no behavioural gain at E6's scale
   (a handful of closure bodies per module, all statically known — a closed
   world, so a switch is complete). (b) becomes attractive only at whole-program
   scale with thousands of indirect targets; that is a T4/T5 (whole-program /
   optimization) concern under the AOT00 roadmap, not E6.

## 2. The closure value on WASM

A closure is a heap object built by the **same `alloc` the cons path uses**, with
a fixed header + the captured `DynValue`s:

```text
  closure heap object  =  [ dispatch_index : i32 ][ n_captures : i32 ][ cap0 ][ cap1 ] …
                           └────────── header ──────────┘ └──── captured DynValues ────┘
```

- `dispatch_index` is a **module-local, compile-time integer** assigned to each
  distinct closure body (the same index JVM's dispatcher switches on). It is
  *not* an address — closures are a closed set per module, so a dense `0..N`
  index is enough and keeps the dispatcher a `br_table`.
- `cap_i` are the free variables captured at `alloc_closure` time, each a tagged
  `DynValue` (`anyref`/`i31ref`), stored with `field_store` exactly like a cons
  car/cdr.
- The whole object is an `anyref` — indistinguishable at the type level from a
  cons or a record, so it flows through `any`/`ref<any>` positions unchanged
  (the E6d-2b `ref<any>` typing already covers it).

`alloc_closure(fn_name, cap0…)` → `alloc` a `(2 + n)`-field object,
`field_store` the header + captures. `field_load`/`field_store` already validated
on WASM.

## 3. Calling a closure: the synthesized dispatcher

`call_closure(handle, arg0…)` lowers to a **direct `call` of a synthesized WASM
function** `$__dyn_call_closure` — the WASM twin of `__callClosure`:

```wat
;; conceptual; generated once per module that uses closures
(func $__dyn_call_closure (param $clo anyref) (param $args anyref) (result anyref)
  (local $idx i32)
  (local.set $idx (call $__field_load_i32 (local.get $clo) (i32.const 0)))  ;; dispatch_index
  (br_table $case0 $case1 … $default (local.get $idx))
  ;; $caseK: call the K-th closure body directly, threading its captures + args
  (block $caseK
     (return (call $closure_body_K
                   (call $__field_load (local.get $clo) (i32.const 2))   ;; cap0
                   … (call $arg_load (local.get $args) (i32.const 0)) …))) ;; args
  …
  ;; $default: unreachable — closed-world index is always in range
)
```

- **Args marshalling.** WASM functions are fixed-arity, but closures are applied
  at various arities. Mirror the JVM `long[]`: the caller boxes the actual args
  into a small heap **argument vector** (an `alloc`+`field_store` array, the same
  primitive lists use) and passes its handle; each `caseK` `field_load`s the
  fixed number of args its body expects. (A later optimization may specialize
  fixed-arity dispatchers; E6d-7 ships the uniform boxed-args path for agreement
  with the other backends.)
- **Direct calls only.** Every `$closure_body_K` is a statically-known function
  in the module — the dispatcher `call`s it directly. No `call_indirect`, no
  table, no `funcref`.
- **Recursion / `LABEL`.** A recursive closure captures itself; the self-reference
  is just another captured `DynValue` in the object, resolved by the same
  `field_load` — no special case (this is how JVM/CLR already handle `LABEL`).

The dispatcher is emitted only when the module contains ≥1 `call_closure`; a
module with no closures (every Twig/Brainfuck program today) is byte-for-byte
unchanged.

## 4. Where the lowering lives

- **Frontend / shared passes — unchanged.** The Twig→IIR path already emits
  `alloc_closure`/`call_closure` (LANG34), and `iir-builtin-lowering::closure`
  (`lower_closure_builtins`) already upgrades any legacy `make_closure`/
  `apply_closure` `call_builtin`s to those ops. WASM consumes the same IIR JVM/CLR
  do — no frontend or shared-pass change.
- **`iir-to-wasm` (the whole change):**
  1. `validate.rs` — remove `alloc_closure`/`call_closure` from the rejected set
     (the `ClosureOpcode` refusal); accept them like the other heap ops.
  2. `lower.rs` — lower `alloc_closure` to the header+captures `alloc`/`field_store`
     sequence (§2); collect each distinct closure body → assign a dispatch index.
  3. `lower.rs` — synthesize `$__dyn_call_closure` (§3) and lower `call_closure` to
     a boxed-args `alloc` + a `call $__dyn_call_closure`.
- **No runtime-C / native / LLVM / JVM / CLR change** — those columns already run
  closures.

## 5. PR breakdown (dependency-ordered; each small, run-verified)

1. **E6d-7a — WASM closure lowering.** validate.rs accept + lower.rs
   alloc/call_closure → heap object + synthesized dispatcher. Unit tests on the
   emitted WASM (structure) + a `wasm-execution` run of a single-capture closure.
2. **E6d-7b — matrix cell, all 5 code-gen backends.** Add
   `((lambda (x) (+ x 1)) 41)` → **42** (a capture-free apply) and a
   capturing cell `(((lambda (x) (lambda (y) (+ x y))) 40) 2)` → **42** to
   `lang_matrix.rs` on `[NativeAot, Llvm, Wasm, Jvm, Clr]`; run-verify WASM agrees
   with the other four (the E6d-2b integration-test pattern for native/LLVM; the
   matrix guard's pre-existing local Jvm-Twig harness break means CI is the
   authoritative matrix gate).

## 6. Verification & robustness gate (AOT00)

Proven by the E6d-7b matrix cell **running** on all five code-gen backends and
agreeing on the known result, plus the WASM structural unit tests. Robustness
axis raised: **L2 language-completeness** — closures, the last dynamic feature
missing from a code-gen backend, now run everywhere; **cross-backend agreement**
(the matrix invariant) is mandatory and is the acceptance test. This closes E6
layer-2 dispatch on the code-gen backends.

## 7. Non-goals / follow-ups

- **`call_indirect`/`funcref` (option b)** — deferred to a whole-program /
  optimization pass (AOT00 T4/T5), not needed at E6 scale.
- **Fixed-arity dispatcher specialization** — the uniform boxed-args vector ships
  first for cross-backend agreement; per-arity specialization is a later
  optimization.
- **Escape analysis / stack-allocated closures** — an AOT00 T5 optimization, out
  of scope here.

## 8. Implementation status & divergence from the plan (E6d-7a)

**Approach — IIR pass, not in-backend lowering.** E6d-7a implements the closure
lowering as a shared IIR pass (`iir-builtin-lowering::lower_closures_to_heap`),
not as new `iir-to-wasm` codegen (§4's plan). It rewrites `alloc_closure`/
`call_closure` to the cons-heap form + a synthesized `__dyn_call_closure`
dispatcher using only ops every heap backend already lowers (`cons`/`car`/`cdr`,
a dynamic `=`, `call`, `jmp_if_false`). This is strictly better: **zero** new
backend codegen, and it works for *any* backend lacking a native closure model.
`iir-to-wasm`'s `alloc_closure`/`call_closure` rejection stays as a defensive
guard (the pass eliminates the ops upstream).

**The design note was wrong about native/LLVM.** §0/§1 assumed "NativeAot and
LLVM run [closures] through the C runtime". They do **not** — both `BackendRefused`
`alloc_closure`/`call_closure` (only JVM/CLR run closures natively, via their
`long[]`/`object[]` dispatch). So the same pass lights up **NativeAot + WASM**
together (run-verified, exit 42), a bigger win than WASM-alone.

**The index test is a dynamic `=`, not `unbox`+`cmp_eq`.** The unboxed integer
width differs by backend (WASM i31/i32 vs native i64), so a hand-rolled
`unbox`+`cmp_eq` cannot be typed uniformly. Using the dynamic `=` (the proven
E6d-6 match/union tag test) delegates the width + boxed-bool `jmp_if_false` to
already-uniform machinery.

**LLVM is a follow-up.** The dynamic `=` on the LLVM column hits a *pre-existing*
`lower_dynamic_arith` comparison-width bug (`cmp_eq` typed `bool` → `icmp i1` on
an i64), which also affects E6d-6 match/union on LLVM (latent — the LLVM column
was never run locally). Filed separately; once fixed, wire the pass into the LLVM
pipeline and add `Llvm` to the two closure matrix cells. **E6d-7a ships on
[NativeAot, Wasm, Jvm, Clr].**
