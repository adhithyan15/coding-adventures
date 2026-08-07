# LANG-FULL E6 (layer 2) — general dynamic dispatch on the code-gen backends

**Status:** design / specs-first, for sign-off.
**Enabler:** E6. **Depends on:** E6 layer-1 (typed `i64` module globals,
[`lang-full-e6-globals.md`](lang-full-e6-globals.md), ✅ landed) · E5 (heap
aggregates) · the McCarthy Lisp L3b dynamic-value work (✅ landed — see §2).
**Unblocks:** Twig lists (**TW3**), records / match / unions (**TW6**),
lambda / closures (**TW5**), dynamic globals, and dynamically-typed arithmetic —
i.e. the Lisp core that today runs only on the tree-walking `twig-vm`.

---

## 0. One-paragraph summary

E6 is the roadmap's "dynamic dispatch" fork — the reason most of Twig (lists,
closures, records, symbols, dynamic `+`) runs only on the interpreter. The good
news, established by survey (§2): **the dynamic-value substrate already exists on
every code-gen backend.** McCarthy Lisp's full `cons`/`car`/`cdr`/symbol/`lambda`/
recursion suite runs end-to-end on WASM, JVM, CLR, LLVM (and BEAM) today, via a
**uniform boxed reference** (`ref<any>` = `anyref` / `java.lang.Object` /
`System.Object` / a tagged 64-bit word) and two **language-agnostic** shared
lowering passes in `iir-builtin-lowering`. Those passes run for Twig too — so
**Twig `(car (cons 42 0))` already lowers and runs on the code-gen backends**
(verified: exit 42 on WASM), it just has no matrix proof and the spec still marks
`TW3 ☐`. E6-layer-2 is therefore **not** a from-scratch build: it is (a) *proving
and locking in* what already works, and (b) *extending the shared catalog* — more
`call_builtin` primitives (dynamic arithmetic, list ops, `make_symbol`),
multi-field heap structs (records/unions), and closures on the one backend that
lacks them (WASM) — always through the shared, allowlist-gated machinery, never
per-backend hacks and never open-ended name dispatch.

## 1. Goal & non-goals

### 1.1 Goal

A dynamically-typed Lisp value — an `any` that at run time is an integer, `nil`,
`#t`/`#f`, an interned symbol, a cons pair, or a closure — is a first-class value
on **all code-gen backends** (native-AOT, LLVM, WASM, JVM, CLR), verified by
**running** a Twig program that exercises it (result observed as an exit code or
stdout). The reference semantics are `lispy-runtime`'s `LispyValue` and
`twig-vm`'s dispatch (§2.1).

### 1.2 Non-goals (explicit follow-ups)

- **Garbage collection** of dynamic values. The heap allocator is `Box::leak` /
  bump today (`lispy-runtime/src/heap.rs`); leaks are the existing E5 shape and a
  later GC spec (LANG16) owns them.
- **The full numeric tower.** Layer-2 boxes **integers** (and the singletons);
  `f64`/bignum/char inside an `any` are deferred (`LispyValue` itself has no float
  variant yet — `operand.rs:90`). Dynamic `+` over floats is a follow-up slice.
- **`eval` / `apply` over runtime-constructed code**, `define-macro`, first-class
  continuations. Out of scope.
- **Open-ended dynamic dispatch.** Dispatch is over a **fixed, compiled-in
  catalog** of primitive names, never a name taken from runtime data — see §4
  (the [[dynamic-dispatch-rce]] constraint).

## 2. Current state (surveyed)

### 2.1 The dynamic-value reference: `LispyValue` + `twig-vm`

The Twig interpreter's value is **`LispyValue`** — a `#[repr(transparent)]` tagged
`u64` (`lispy-runtime/src/value.rs:124`), **not** `vm-core::Value` (which is only
5 scalar variants and does **not** run dynamic Twig — this is why the matrix's
generic `Vm`/`Jit` columns cannot execute a Twig `cons` program; see §5). Six
live cases, 3-bit low tag:

| Kind | tag | payload |
|---|---|---|
| Int | `000` | high 61 bits, signed (±2⁶⁰) |
| Nil | `001` | singleton (distinct from `#f`) |
| Symbol | `010` | high 32 bits = interned `SymbolId` |
| False | `011` | singleton |
| True | `101` | singleton |
| Heap | `111` | word with low bits cleared = pointer to an `ObjectHeader`-prefixed object |

Heap objects (`heap.rs`): **ConsCell** (`car`,`cdr`), **Closure** (`fn_name`,
`flags`, `captures`), **LangString**. Truthiness is Scheme (only `#f`/`nil`
false). Builtin dispatch is a static `resolve_builtin(name) -> fn(&[LispyValue])`
match (`binding.rs:314`); closure application (`twig-vm/src/dispatch.rs:1930`,
`~2607`) resolves a heap closure, prepends captures, and recurses into the callee.

### 2.2 The substrate already on the code-gen backends (from McCarthy L3b)

`ref<any>` — "a boxed value that is an atom OR a heap object" — is exactly a
general dynamic value, and it is already mapped on every code-gen backend:

| Backend | `ref<any>` | cons `ref<LispyPair>` | atom box / unbox | pair? test |
|---|---|---|---|---|
| WASM (WasmGC) | `anyref` | `(ref $LispyPair)` 2-field struct | `ref.i31` / `i31.get_s` | `ref.test $LispyPair` |
| JVM | `java.lang.Object` | `Object[2]` | `Integer.valueOf` / `checkcast;intValue` | `instanceof Object[]` |
| CLR | `System.Object` | `object[2]` | `box int32` / `unbox.any` | `isinst object[]` |
| LLVM | tagged `i64` | tagged `i64` (runtime-owned) | `__twig_lispy_box_int` / `_unbox_int` | `__twig_lispy_pair` |

The heap-object IIR family — `alloc` / `field_load` / `field_store` / `is_null`,
plus `box` / `unbox` — is lowered by **two shared, language-agnostic passes** in
`iir-builtin-lowering`, applied by `lang-aot` for *every* source language
(`lib.rs:528/709/794` managed, `442` LLVM):

1. **`lower_heap_builtins`** — `cons`→`alloc`+2×`field_store`; `car`/`cdr`→
   `field_load[0/1]`; `null?`→`is_null`.
2. **`lower_lisp_repr_structural`** (managed) / **`lower_lisp_repr`** (LLVM
   tagged-word) — the use-site-directed, **gate-free** boxing pass: it inserts
   `box` at every site where an atom flows into a dynamic slot, retypes lisp
   params/returns to `ref<any>`, and unboxes the entry result to a machine int.

Because these are gate-free and language-agnostic, **Twig already rides them.**

### 2.3 What already works vs what's rejected (empirical)

Compiling Twig through the real pipeline (lang-aot), observed directly:

| Twig program | code-gen result |
|---|---|
| `(car (cons 42 0))` | ✅ **lowers + runs, exit 42** (WASM) |
| `(car (cdr (cons 1 (cons 42 0))))` | ✅ **runs, exit 42** (WASM) |
| `(length (list 1 2 3))` | ❌ `call_builtin "list"` / `"length"` not in allowlist |
| `((lambda (x) (+ x 1)) 41)` | ❌ `call_builtin "+"` not in allowlist **and** `alloc_closure`/`call_closure` rejected by WASM |

So the gaps are specific and enumerable, not architectural.

### 2.4 `call_builtin` allowlists today (the seam E6 widens)

The catalog is small and identical-ish across backends
(`CALL_BUILTIN_SUPPORTED_NAMES` in each `validate.rs`;
`SUPPORTED_BUILTINS`+`LISPY_BUILTINS` for LLVM):

- WASM / JVM: `putchar getchar print_i64 input_i64 input_str pair? not equal?`
- CLR: same minus `input_i64`/`input_str`
- LLVM: I/O + `LISPY_BUILTINS` (`lispy_cons/car/cdr/pair_p/equal/not/box_int/
  unbox_int/nil/…` → `__twig_lispy_*` C-runtime calls)

**Dynamic arithmetic (`+ - * /`), comparisons, and list ops are absent from all of
them** — the single biggest gap. Closures: `call_closure`/`alloc_closure` are
supported on JVM/CLR (`long[]` / `object[]` dispatch arrays, `call_closure`'s
`"any"` hint whitelisted) but **hard-rejected on WASM** (`ClosureOpcode`); a
`iir-builtin-lowering` "Phase 4" downgrade pass exists but is not wired into the
Twig→WASM path.

## 3. Design

### 3.1 Principle — extend the *shared catalog*, structurally, once

Every new dynamic primitive is added in **`iir-builtin-lowering`** as a structural
lowering to ops the backends already run, so all five code-gen backends light up
from one change (the way `cons` does). Two lowering shapes:

- **Arithmetic / predicates over `any`** → *unbox → typed op → box*. E.g.
  `call_builtin "+" [any] a b` lowers to
  `unbox a→ia:i64 ; unbox b→ib:i64 ; add ia ib→s:i64 ; box s→r:any`.
  All backends already have `unbox`/`add`/`box`; **no backend change** is needed
  for the integer case. (Runtime type-checking / mixed int-float dispatch is a
  later slice — layer-2 lowers the **integer** contract and documents it; a
  non-integer operand is a clean trap, mirroring E4/E5 bounds traps.)
- **Constructors / heap shapes** (list, record, union) → `alloc` + `field_store`
  chains, reusing the cons machinery (§2.2).

Where a primitive genuinely needs runtime behaviour the structural form can't
express (e.g. `length` walking a cons chain), it lowers to a **synthesized IIR
helper function** built from ops all backends run (`field_load`, `cmp`, `add`,
`jmp` — the same technique BASIC's `__basic_print_int` and `GOSUB` stack use), not
a per-backend intrinsic.

### 3.2 Allowlist discipline (RCE-safe)

Per [[dynamic-dispatch-rce]]: dispatch is over a **compiled-in catalog** matching
`resolve_builtin`'s names (§2.1). E6 grows that catalog **explicitly** — each new
primitive is added to `CALL_BUILTIN_SUPPORTED_NAMES` (or given a structural
lowering that removes the `call_builtin` entirely). A `call_builtin` whose name
came from runtime data never occurs — names are always compile-time
`Operand::Str` chosen by the frontend from a fixed set. The validator keeps
rejecting any unknown name (fail-closed).

### 3.3 The `any` value model on each backend (unchanged from L3b)

E6 introduces **no new value representation** — it reuses §2.2. The one
generalization needed for records/unions (§ PR breakdown E6d-5/6) is a
**type-parameterized `alloc`**: today the managed backends hardcode the single
2-field `ref<LispyPair>` (`SUPPORTED_REF_TYPES = ["ref<LispyPair>","ref<any>"]`,
`alloc` rejects other hints). E6d-5 adds a small **struct-type registry** keyed by
name (`ref<Rec:field-count>`), so `alloc` can build an N-field object and
`field_load`/`field_store` index into it — the same opcodes, a wider type set.

### 3.4 Closures (WASM) — the one real backend gap

JVM/CLR already run closures. WASM rejects `alloc_closure`/`call_closure`. Two
options, decided in E6d-7 (its own design note): **(a)** wire the existing
`iir-builtin-lowering` "Phase-4" pass into the Twig→WASM path to downgrade
closures to the `call_builtin "make_closure"`/`"apply_closure"` heap form (reusing
the cons/heap substrate — a closure becomes a heap object like a cons), or **(b)**
a native WasmGC `$Closure` struct + `call_indirect`. **(a) is preferred** — it
reuses the shared substrate and needs no new WasmGC type. ⚠ Surface the choice at
E6d-7.

## 4. PR breakdown (dependency-ordered; each its own small, run-verified PR)

> Convention: each ☐ is one `feat(lang-full): E6d-… ` PR, security-reviewed,
> matrix-proven, babysat. "All code-gen backends" = NativeAot + LLVM + WASM + JVM +
> CLR (the generic `Vm`/`Jit` run typed IIR only; dynamic Twig cells are proven on
> the code-gen columns, which must agree with each other and the known result —
> and, where wired, the `twig-vm` reference; see §5).

1. **E6d-1 — TW3-core: prove `cons`/`car`/`cdr` cross-backend.** ✅ **LANDED**
   (`lang-aot` matrix). Two cells — `(car (cons 42 0))` → 42 and nested
   `(car (cdr (cons 1 (cons 42 0))))` → 42 — on `[NativeAot, Llvm, Wasm, Jvm, Clr]`.
   **Zero production code**: the surveyed fact held — the shared heap passes already
   lower Twig's `call_builtin "cons"/"car"/"cdr"` to the `ref<any>` substrate.
   Run-verified locally on **WASM** (in-process) and **real dotnet CLR**;
   native/LLVM/JVM via CI. Resolved §5's open question with option **(a)**: the
   cells list the 5 code-gen columns only (the generic `Vm`/`Jit` run `vm-core`
   typed IIR and can't execute `ref<any>`/`alloc`; `twig-vm` is the off-matrix
   reference). Turns the stale `TW3 ☐` into a guardrailed proof (the E4d-BA-input
   pattern).
2. **E6d-2 — dynamic integer arithmetic over `any`.** The generic dispatch seam.
   Structural lowering of `call_builtin "+"/"-"/"*"` (and the extended-division
   `quotient`/`remainder`/`modulo`) + comparisons `= < > <= >=` to *unbox → typed
   op → box*. Proof: `(+ (car (cons 41 0)) 1)` → 42 (forces `+` over an `any`).
   No per-backend change expected (all have unbox/op/box).
3. **E6d-3 — list builtins.**
   - **E6d-3a — `list` constructor.** ✅ **LANDED.** A shared
     `desugar_list_in_function` pass (head of both `lower_heap_builtins` and
     `lower_heap_builtins_runtime`) expands `call_builtin "list" a b c` → a nil
     `const` + right-to-left `cons` chain, so `list` rides the E6d-1 cons path on
     all 5 code-gen backends with no new backend op and no allowlist entry (the
     `list` builtin is gone before the backend sees it). Matrix:
     `(car (list 42 1 2))` → 42 and `(car (cdr (list 1 42 3)))` → 42; WASM + real
     dotnet CLR verified, native/LLVM/JVM via CI. `(list)` → nil.
   - **E6d-3b — list *operations* (✅ COMPLETE: `length` ✅, `list-ref` ✅,
     `append` ✅, `reverse` ✅, `assoc` ✅).** These walk/rebuild the cons chain,
     so they need a synthesized cons-walk helper, not a pure desugar. `null?`/
     `pair?` already lower. `length` (shipped): `iir-builtin-lowering`'s `lower_list_ops` rewrites
     `call_builtin "length" lst` → `call __dyn_list_length, lst` and injects (once)
     a recursive helper that is itself a **proper lisp function** returning a boxed
     `ref<any>` — base case `null? lst` → `box 0`, recurse `+ 1 (length (cdr lst))`
     via the dynamic `+` (E6d-2). Because it returns a genuine boxed value it
     composes: `(+ (length (list 1 2 3)) 39)` → 42. Shipping `length` also forced
     the **WASM nil-const fix** (iir-to-wasm 0.38.0): a `ref<…>` `const 0` now emits
     `ref.null`, so `null?`/`is_null` detects the terminator — previously the walk
     overran nil into `struct.get` on an i32; this also fixes `(null? (list))` → 1
     (CLR already lowered nil to `ldnull`). `list-ref` (shipped): same
     `lower_list_ops` rewrites `call_builtin "list-ref" lst n` → `call
     __dyn_list_ref, lst, n`, whose helper is `if ni==0 then car(lst) else
     list-ref(cdr(lst), ni-1)`. **The index is a boxed lisp value** — the
     uniform-anyref boundary (`dyn_repr_structural`/`lower_dyn_repr`) boxes every
     lisp-call arg, so a raw-`i64` index param faults (`expected i64, got I32(2)`);
     the helper takes `n : ref<any>`, unboxes it once, and the index test/decrement
     are typed `cmp_eq`/`sub` (raw bool feeds `jmp_if_false` directly). `append`
     (shipped): same `lower_list_ops` rewrites `call_builtin "append" a b` → `call
     __dyn_list_append, a, b`, whose helper *rebuilds* the first list —
     `if null?(a) then b else cons(car(a), append(cdr(a), b))`. No index, so no
     unbox/box (every value it touches is a reference); its one new op is the
     `cons` in the recursive arm (the E6d-1 heap builtin, lowered for the injected
     helper too). `reverse` (shipped): same `lower_list_ops` rewrites
     `call_builtin "reverse" a` → a **nil-seeded** call to a tail-recursive
     accumulator helper — `reverse_acc(a, acc) = if null?(a) then acc else
     reverse_acc(cdr(a), cons(car(a), acc))` — consing each element onto the
     accumulator's front; the call site seeds `acc` with a `const 0 :
     ref<LispyPair>` nil (the `list`-desugar sentinel). `assoc` (shipped): same
     `lower_list_ops` rewrites `call_builtin "assoc" key alist` → `call
     __dyn_list_assoc, key, alist`, whose helper searches an association list —
     `if null?(alist) then nil else if key==car(car(alist)) then car(alist) else
     assoc(key, cdr(alist))`. The key equality must be a raw bool for
     `jmp_if_false`, and `equal?` lowers unevenly across the managed/runtime paths,
     so V1 `assoc` **unboxes both keys to `i64` + typed `cmp_eq`** (the `list-ref`
     technique) — V1 keys are integers; symbol keys come with E6d-4. Proof
     (shipped): `(+ (length (list 1 2 3)) 39)` → 42, `(null? (list))` → 1,
     `(list-ref (list 10 20 42) 2)` → 42,
     `(car (cdr (append (list 1 42) (list 3))))` → 42,
     `(car (reverse (list 1 2 42)))` → 42,
     `(cdr (assoc 2 (list (cons 1 10) (cons 2 42) (cons 3 30))))` → 42,
     `(null? (assoc 9 …))` → 1; WASM + real dotnet CLR verified, native/LLVM/JVM
     via CI.
4. **E6d-4 — symbols / quote (✅ quote-literal identity; runtime create/name-recovery
   deferred).** A Twig quote literal (`'a` / `(quote a)`) now lowers to `const
   Var(name) : symbol` — the interned-const form McCarthy emits (twig-ir-compiler
   0.43.0) — rather than the runtime `make_symbol` string path (which needs
   data-section string emission the code-gen backends lack). This rides the
   already-wired `intern_symbols` (native) / `intern_symbols_structural` (managed)
   passes (§2.1): each distinct name → one module-wide id in a reserved high range,
   so a symbol never collides with an integer atom and `equal?` on symbols is
   bit-equality — no new value type. Twig has `equal?` (not `eq?`); `equal?` on
   two symbols is identity. Proof (shipped): `(equal? 'a 'a)` → #t (exit 1),
   `(equal? 'a 'b)` → #f (exit 0) on [NativeAot, Llvm, Wasm, Jvm, Clr]; WASM + real
   dotnet CLR verified, native/LLVM/JVM via CI. **Deferred:** runtime symbol
   *creation* (`string->symbol` over a runtime string) and `symbol->string` name
   recovery on the code-gen backends still need the `make_symbol` data-section path.
5. **E6d-5 — records (TW6, part 1) (✅ shipped).** A Twig `(record Name (f : T) …)`
   already erases (in the frontend) to a constructor that builds a cons chain via
   typed `alloc [ref<LispyPair>]` + `field_store`, and accessors `name-f(r)` =
   `car(cdr^i(r))` via `field_load` — so records reuse the E6d-1 heap substrate
   with **no new value type, no struct-type registry, and no frontend change**;
   they are a catalog-extension over cons. Shipping the proof fixed a latent
   **wasm-runtime** bug (0.4.0): struct field counts were indexed by per-function
   count, over-counting when functions share a signature (a record's constructor +
   N same-shape accessors + predicate), so every record `struct.set` trapped
   `field 0 out of range` on WASM; now indexed by the deduplicated function-type
   count. Proof (shipped): `(record Point (x : int) (y : int)) (point-x (Point 42
   7))` → 42 and `(point-y (Point 7 42))` → 42 on [NativeAot, Llvm, Wasm, Jvm,
   Clr]; WASM + real dotnet CLR verified, native/LLVM/JVM via CI.
6. **E6d-6 — unions / match (TW6, part 2) (✅ shipped).** A Twig `(union Name
   (Variant …) …)` erases (frontend, unchanged) to integer-tagged constructors —
   a cons `(tag . fields…)` — and `match` compares the scrutinee's tag (`car`) to
   each variant's integer tag, binding fields via `car(cdr^i)`, over the E6d-1
   heap substrate. Two fixes made it run on the code-gen backends: (1) the variant
   tag const is typed `i64` not `"any"` (twig-ir-compiler 0.44.0 — an `"any"`
   const was wrongly `unbox`ed → WASM `expected i32, got I64` trap); (2) a
   boxed-bool `jmp_if_false` (the boxed `=` tag-compare result) now branches on its
   RAW bool in **both** dyn_repr passes (iir-builtin-lowering 0.29.0 — structural
   for WASM/JVM/CLR/BEAM and the native `dyn_truthy` path for NativeAot/LLVM),
   because a boxed `#f` is a non-nil i31 / tagged-0 that the nil-/McCarthy-
   truthiness wrap mis-read as true, mis-dispatching every arm. This is a general
   fix (any `(if (= a b) …)` on the dynamic path). Proof (shipped):
   `(union Opt (Some (v : int)) (None)) (match (Some 42) ((Some v) v) ((None) 0))`
   → 42 and matching the 2nd variant → 42 on [NativeAot, Llvm, Wasm, Jvm, Clr];
   WASM + real dotnet CLR verified (full matrix re-run confirms no regression),
   native/LLVM/JVM via CI.
7. **E6d-7 — closures on WASM (TW5).** ✅ **design note written** —
   [`E6D7-wasm-closures.md`](E6D7-wasm-closures.md) commits to **option (a)**:
   heap-form closure (`[dispatch_index, captures…]`, reusing the `alloc`/`field_*`
   substrate) + a synthesized `$__dyn_call_closure` dispatcher (the WASM twin of
   JVM/CLR `__callClosure` — a `br_table` over statically-known bodies, **no
   `call_indirect`/`funcref`**). JVM/CLR/LLVM/native already run closures.
   Impl PRs: E6d-7a (iir-to-wasm lowering) → E6d-7b (matrix cell). Proof:
   `((lambda (x) (+ x 1)) 41)` → 42 on all 5 code-gen backends.
8. **E6d-8 — dynamic globals (✅ read/write roundtrip shipped; arith follow-up).**
   A forward-referenced Twig value global (read before its `define`) is emitted as
   `call_builtin "global_get"/"global_set"` over `any`. The shared `lower_global_io`
   pass rewrites those to typed `global_load`/`global_store` (which every backend
   already supports) — but **only the native `twig-aot` pipeline ran it**; the
   managed `lang-aot` pipelines (WASM/JVM/CLR/BEAM) + LLVM never did, so a dynamic
   global hit an unsupported `call_builtin`. Fix: add `lower_global_io` (step 0) to
   all those pipelines. Proof (shipped): `(define (f) g) (define g 42) (f)` → 42 on
   [NativeAot, Llvm, Wasm, Jvm, Clr] (`main` `global_store`s g=42, `f` `global_load`s
   it); WASM + real dotnet CLR verified, native/LLVM/JVM via CI. **Follow-up:** a
   dynamic global feeding dynamic *arithmetic* (`(+ g 2)`) traps — the slot stores a
   raw `i64` but `lower_dynamic_arith` treats the `any`-typed `global_load` result as
   boxed and inserts an `unbox`. Needs the slot widened to a boxed `any`.

Ordering rationale: E6d-2 (arithmetic) is the widest single unlock and blocks any
"compute a number dynamically" proof; lists/records/unions all reduce to cons
(E6d-1's substrate); closures (E6d-7) are the only irreducible new heap kind and
the only real backend gap (WASM). E6d-1 is first because it is nearly free and
converts the survey into a regression-guarded baseline everything else builds on.

## 5. Verification

Each PR adds/extends a `lang_matrix.rs` cell exercising a **dynamic** Twig program
(the value is genuinely `any`, not a folded constant), asserted on every code-gen
backend whose toolchain is present, guarded by the existing
`proven_columns_do_not_silently_skip` + `matrix_every_proven_cell_agrees`.
**Open question for E6d-1:** the generic `Vm`/`Jit` runners execute `vm-core`
typed IIR and cannot run the lowered lisp repr (`ref<any>`/`alloc`). Either (a)
Twig-dynamic cells list only the 5 code-gen backends (they cross-check each other
+ the known exit code — legitimate, mirrors how McCarthy's per-backend suites
prove cons today), or (b) a `twig-vm`/`jit_lisp` reference runner is wired into the
matrix as the `Vm`/`Jit` column for Twig. E6d-1 picks one; (a) is the lower-risk
default.

## 6. Out of scope (later E6 layers / other specs)

- GC of dynamic values (LANG16). `f64`/bignum/char inside `any` (numeric tower).
- `eval`/macros/continuations. Method dispatch (`send`) — Lispy has none.
- The exotic historical-arch encoders (GE225, Intel-4004/8008, …) — E6 targets the
  5 general code-gen backends, as the LANG-FULL campaign does.
