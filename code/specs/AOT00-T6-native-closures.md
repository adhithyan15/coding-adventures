# AOT00-T6 — native closures (codegen + GC) (design)

> Status: **design, pre-implementation.** Committed for sign-off before any code, exactly as the
> AOT00-T1/T3/T4/T5 specs were. Closes the last major Twig *language-feature* gap in the
> native-AOT path: closures currently do not compile to native code at all.

---

## 1. The problem — closures are interpreted, not compiled

The Twig native-AOT pipeline (`twig-ir-compiler` → IIR → `aot-core` → `aarch64-backend` /
`x86_64-backend` → native binary) compiles most functions to machine code. But **any function
that allocates or calls a closure falls back to the embedded VM interpreter.**

`aot-core::compile` (`core.rs:136-148`) tries each function through the backend; a function the
backend **cannot compile** returns `None` and is routed to `untyped_fns`, serialised into an IIR
table, and executed at run time by a `VmRuntime` (`twig-vm`) embedded in the image. Neither
native backend has an `alloc_closure` / `call_closure` handler (grep of both `lib.rs` finds these
op names only in comments/tests), so **every closure-bearing function is interpreted.** That is
correct but slow, and — the reason this matters for the GC arc — closure objects live on the
**VM's managed `MarkAndSweepGC` heap**, entirely separate from the native `FlatHeap` (gc-core).
So "every Twig heap object is under the native GC" cannot be true until closures compile natively.

This rung makes `alloc_closure` / `call_closure` / `make_builtin_closure` **compile to native
machine code**, allocate the closure object through **gc-core** (precise + movable), and thereby
(a) run closures at native speed and (b) bring the last Twig heap type under `FlatHeap`.

### 1.1 Current closure model (what we must preserve semantically)

- **`alloc_closure(Str(fn_name), cap0..capN) -> closure`** (`compiler.rs:1256-1263`): a lambda is
  lifted to a top-level IIR function `fn_name` whose parameter list is **`captures ++ formal_args`**
  (`compiler.rs:1204,1232`), i.e. the captured free variables are prepended to the lambda's
  declared parameters. Free-var order is stable (`free_vars.rs`), shared by the lambda's params and
  the `alloc_closure` capture operands, so the two line up.
- **`call_closure(handle, arg0..argM) -> any`**: extract `(fn_name, captures)` from the handle and
  invoke `fn_name` with **`captures ++ args`** (VM: `dispatch.rs` `exec_call_closure`).
- **`make_builtin_closure`**: a 0-capture closure wrapping a runtime builtin.
- Closures are distinguished from cons pairs by a class tag (`LispyClass::Closure`).

### 1.2 Why native codegen is not trivial — the marshaling problem

The blocker is **dynamic capture count vs. static register allocation.** `call_closure(handle,
args)` is dynamic — the handle can be any closure — so the number of captures to *prepend to the
argument registers* is not known at the call site. A register-based ABI cannot dynamically choose
"load capture[i] into argument register i" for a runtime-varying capture count. The current
"captures-as-leading-params" convention only works because the VM builds the argument vector at
run time.

The standard fix is an **environment-pointer calling convention** (§2): captures are **not**
passed as separate argument registers; instead the closure passes *itself* as a hidden first
argument and the callee loads its captures from that pointer. The capture count is then a
compile-time constant *inside each callee* (it loads exactly its own captures), and the call site
only marshals a fixed prefix (`env`) plus the statically-known `args`.

---

## 2. Design — the environment-pointer calling convention

### 2.1 Closure object layout (a gc-core kind)

A closure is a heap record:

```
offset 0 : code_ptr   — the native address of the lifted function `fn_name` (a CODE pointer,
                        NOT a heap reference; never traced or relocated as data)
offset 8 : cap0        ┐
offset 16: cap1        │ the captured free-variable values, in free_vars order — each a Twig
   …                   │ `any` value (tagged: immediate int/nil/bool, or a heap reference)
offset 8+8k: capK      ┘
```

Registered as a **gc-core kind via `register_ref_array_kind(fixed = [], tail_from = 8)`** (the T5
API, already in `gc-core-capi`): the word at offset 0 is *excluded* from tracing (it is a code
address, and a code address must never be treated as a heap reference — tracing it would be a bug,
relocating it a catastrophe), and every word from offset 8 onward is a **reference** (the
captures). This makes closures **precise** (a captured immediate that looks like a pointer never
pins) **and movable** (the compacting collector relocates a closure and fixes up its captured
references — the code_ptr is a non-ref word, left untouched). This is exactly the "header + ref
tail" object T5 was built for.

The handle is the payload address OR-tagged with the heap tag (`LISPY_TAG_HEAP = 0b111`), same as
cons cells; a class byte in the object (or a distinct kind id) distinguishes a closure from a pair
for `pair?`/`procedure?` predicates — **decision D1 below.**

### 2.2 The lifted function's native signature

Each lifted lambda `fn_name` compiles to native code with signature:

```
fn_name(env, arg0, arg1, …, argM) -> any
```

- `env` (arg register 0) = the closure object payload address.
- `arg0..argM` (arg registers 1..M+1) = the lambda's declared arguments (M static per function).
- The **function prologue loads its captures from `env`**: for each captured free variable *i*,
  `capture_i = load [env + 8 + 8*i]`. The body then references captures as ordinary locals.

This replaces the current "captures are leading params" with "captures are env loads." M (the
declared arg count) is static per function, so the register assignment is fully static.

### 2.3 The three ops' native lowering

- **`alloc_closure(Str(fn_name), cap0..capN)`**:
  1. `__gc_alloc_kind(8 + 8*N, closure_kind)` → payload `p` (closure_kind registered lazily, §2.1).
  2. Store `code_ptr` of `fn_name` at `[p+0]` — a **relocation to the local function symbol**
     `fn_name` (all lifted functions are linked into the one `__text` section, so this is a
     link-time-resolvable local address; on aarch64 an `ADRP`+`ADD` **is unsound for a non-page-
     aligned base** — use `ADR` for a ±1 MiB range, or a data relocation, per
     [[feedback_adrp_page_immediate_cannot_be_baked]]; on x86_64 a RIP-relative `lea`).
  3. Store each capture at `[p + 8 + 8*i]`.
  4. Return `p | LISPY_TAG_HEAP`.
- **`call_closure(handle, arg0..argM)`**:
  1. `env = handle & ~0b111` (strip tag → payload address).
  2. `code_ptr = load [env + 0]`.
  3. Marshal: arg-reg[0] = `env`; arg-reg[1..M+1] = `arg0..argM` (M static at this call site).
  4. Indirect call: `blr code_ptr` (aarch64) / `call code_ptr` (x86_64). Result in the return reg.
- **`make_builtin_closure`**: a 0-capture closure whose `code_ptr` is a small adapter that forwards
  to the named `__twig_*`/`__dyn_*` builtin (or a distinguished code_ptr the call path recognises).
  **Decision D2 below** — builtins may be simpler to keep on a dedicated path.

### 2.4 GC safety

- The `code_ptr` at offset 0 is never traced (excluded by `tail_from = 8`) and never relocated —
  correct, it is a `__text` address, not a heap datum.
- Captures (offset ≥ 8) are precise references: traced on mark, relocated + fixed up on compaction,
  recorded by the generational barrier when a closure is old and a capture is young.
- A live closure handle sits in a stack slot / register across any safepoint; it is discovered by
  the precise stack map (if the slot is a reference slot) or conservatively (pin-when-unsure)
  otherwise — the same contract cons cells already satisfy. An `env` pointer held in arg-reg 0
  during a call is live across any safepoint inside the callee and is discoverable the same way.
- **Indirect-call safepoints:** `call_closure`'s indirect `blr`/`call` is a call-return safepoint
  exactly like a direct call; the existing stack-map emission (which records every call-return
  offset) covers it with no special handling — verify in the emission rung.

---

## 3. Open decisions (resolved here so implementation is unambiguous)

- **D1 — closure vs. pair discrimination.** Use a **distinct gc-core kind id** for closures
  (registered separately from the cons kind) and, for `procedure?`/`pair?`, compare the object's
  kind — OR store a 1-byte class tag in a spare header byte. *Recommendation:* reuse the T5
  `register_ref_array_kind` closure kind and expose the object's kind via a cheap
  `__gc_kind_of(ptr)` capi accessor (new, tiny) for the predicates. Keeps the value tag uniform
  (`LISPY_TAG_HEAP`) and avoids stealing payload bytes.
- **D2 — builtins as closures.** `make_builtin_closure` wraps a runtime builtin with 0 captures.
  *Recommendation:* give each such closure a `code_ptr` to a generated per-builtin adapter
  `fn(env, args…) → __twig_<builtin>(args…)`, so `call_closure` is uniform (no builtin/non-builtin
  branch). Adapters are tiny and generated alongside `__gc_init_stackmaps`.
- **D3 — where the env-pointer transform lives → NATIVE-LOCAL (decided).** A **blast-radius audit**
  (2026-07-29) found the current captures-as-params closure IIR is consumed by **~48 crates and
  ~130 closure tests**, including seven other language backends (JVM, CIL, WASM, BEAM, and
  `semantic-ir-to-{c,go,javascript,python,rust,typescript}`) plus the Ruby/Python frontends.
  Changing the closure calling convention in `twig-ir-compiler` (a global shape change) would
  destabilise **all** of them. Therefore the env-pointer transform is **native-path-only**: an
  **`aot-core` IIR→IIR pass** rewrites *only* the functions that are closure targets, and *only*
  when compiling for a native backend — every other engine keeps consuming the unchanged
  captures-as-params IIR. Concretely:
  - Collect the set of closure-target function names = every `name` appearing in an
    `alloc_closure(Str(name), …)` across the module.
  - For each such function, rewrite its signature from `(cap0..capN, arg0..argM)` to
    `(env, arg0..argM)` and prepend `field_load env, (i+1) -> cap_i` for each capture (env[0] is the
    code_ptr, captures start at word 1), so the body — which already references captures by those
    names — is unchanged below the prologue.
  - Non-closure functions and every non-native engine are untouched. `twig-vm` keeps its current
    `exec_call_closure` (captures-as-params) for the interpreter fallback and other engines.
  This contains the change to `aot-core` + the two native backends + the twig-aot runtime, with
  **zero risk** to the other seven engines' closure support — the decisive reason to prefer it over
  a global shape change.

---

## 4. PR breakdown (each builds + tests + `/security-review`; native rungs get an execute
differential on aarch64)

1. **This spec.**
2. **aot-core native-only closure-env transform (D3).** A new `aot-core` IIR→IIR pass, run *only*
   before native codegen, that rewrites each closure-target function `(cap0..capN, arg0..argM)` →
   `(env, arg0..argM)` + a `field_load env,(i+1)` capture prologue. Closure targets = the `name`s in
   `alloc_closure(Str(name),…)`. Non-closure functions and all non-native engines untouched;
   `twig-vm` and the other six backends keep the captures-as-params model. Unit tests: a closure
   function is transformed (params → env-loads), a non-closure function is not, capture order is
   preserved. No backend lowering yet — the transformed functions still won't compile until PR-4/5,
   so they continue to route to the VM (behaviour unchanged) until then.
3. **gc-core-capi: `closure_kind` + `__gc_kind_of`.** A thin capi helper the runtime uses to
   register the closure kind (`register_ref_array_kind([], 8)`) and read an object's kind (for
   `procedure?`). Unit-tested.
4. **Backend `alloc_closure` lowering** (aarch64 + x86_64): allocate via `__gc_alloc_kind`, store
   the `code_ptr` (ADR / RIP-lea reloc to the local function symbol) + captures, tag the handle.
   Emission unit tests (the op lowers; the function-symbol relocation is recorded).
5. **Backend `call_closure` lowering** (aarch64 + x86_64): strip tag, load code_ptr, marshal
   `env + args`, indirect call. Emission unit tests + the indirect-call safepoint check.
6. **aot-core + end-to-end.** With 4 + 5, closure-bearing functions now compile (no longer routed
   to the VM) — assert `functions_untyped` drops for a closure program. **Native execute
   differentials (aarch64):** (a) a closure captures a value and is called → returns the correct
   result (native, not interpreted); (b) GC: the closure and its captured heap value relocate under
   `gc_collect_compacting` and the capture is fixed up (mirrors the T5 stress differential through
   the live Twig closure path); (c) `functions_untyped == 0` for the program.
7. **Docs + memory.** Update the convergence memory + this spec's status; note the Twig heap
   surface is now fully native-GC-managed (cons ✓, strings ✓, records via `alloc`, closures ✓).

### 4.1 Sequencing constraint — the transform is COUPLED to native compilation (do not land PR-2 alone)

The naive "land the env transform first, lowering later" plan is **unsound**: a closure-target
function transformed to the env shape but then *not* compiled natively falls back to the embedded
`twig-vm`, which expects the **original captures-as-params** shape → the interpreter would run a
function whose parameter list it no longer understands, breaking closures across the ~130 tests.

Therefore:
- **The env transform must be applied to a closure-target function ONLY when that function — and
  the caller sites' `alloc_closure`/`call_closure` — will actually compile natively.** A function
  (or its closure target) that falls back to the VM must be left in the **original** shape.
- Practically this means PR-2's transform lands **together with** PR-4/5's `alloc_closure` /
  `call_closure` lowering, guarded so the transform is only committed for functions that pass
  native compilation; anything that would fall back is emitted unchanged. Implement it as: try to
  compile the whole closure cluster natively with the transform applied to a *copy*; on success use
  the transformed native code, on any failure discard the transform and route the original IIR to
  the VM. (Equivalently: gate the transform on "all ops in the cluster are backend-supported.")
- The safest first *mergeable* unit is therefore **PR-3 (the gc-core-capi `closure_kind` +
  `__gc_kind_of`)**, which is independent and testable; the transform + both lowerings + the
  partition/fallback guard land as one coherent native-closures PR (or a tightly-reviewed pair),
  not as isolated slices.

## 5. Verification (end-to-end)
- Per-PR: `cargo test` (twig-ir-compiler, twig-vm, backends, gc-core-capi, twig-aot), clippy, Miri
  on any new unsafe, `/security-review`.
- Headline: a compiled **native aarch64** program `((lambda (x) (+ x c)) …)` capturing `c` returns
  the right value **and** its closure relocates under compaction with the capture fixed up — closures
  run natively and live under the native GC.
- Regression guard: the full existing closure suite (VM + the other seven engines) stays green —
  the env transform is native-path-only and applied only to functions that compile natively, so any
  closure that falls back to the interpreter keeps its original captures-as-params shape (§4.1).

## 6. Non-goals
- Escape analysis / stack-allocating non-escaping closures (an optimisation; heap-allocate all
  closures first).
- Inlining direct calls of known closures (a later optimisation).
- Tail-call optimisation of `call_closure` (separate concern).
