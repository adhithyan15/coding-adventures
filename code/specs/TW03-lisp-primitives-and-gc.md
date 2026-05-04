# TW03 — Full Lisp surface + GC across every Twig backend

## Why this spec exists

The Twig real-runtime trilogy is complete: source code runs on
JVM (`twig-jvm-compiler`), CLR (`twig-clr-compiler`), BEAM
(`twig-beam-compiler`), and WebAssembly (`twig-jit-wasm`).
What's still missing is the **language surface itself**: most of
those backends only handle integer arithmetic + `let`.

Per the user's vision: "We should support all the primitives a
functional programming language like Lisp supports with the only
exception being macros."  And separately: "We also need to
incorporate garbage collection as well."

This spec scopes the remaining work into a coordinated cross-
backend roadmap.  Every primitive needs a *consistent* lowering
across all five backends — or an explicit deviation noted in the
backend-specific spec.

## What "the full Lisp surface" means here

Strict subset of R5RS Scheme **minus macros** (the whole reason
for the macro carve-out is that `let-syntax` / `define-syntax`
turn the parser into a Turing machine and we want to keep the
parser tiny).  Concretely:

| Primitive | TW00 (vm-core) | JVM | CLR | BEAM | JIT/WASM |
|-----------|:--------------:|:---:|:---:|:----:|:--------:|
| Integer literals + arithmetic | ✅ | ✅ | ✅ | ✅ | ✅ |
| `let` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `begin` | ✅ | ✅ | ✅ | ✅ | ✅ |
| **`if`** | ✅ | ✅ | ❌ | ❌ | via fallback |
| **Top-level `define`** | ✅ | ✅ | ❌ | ❌ | via fallback |
| **Function calls + recursion** | ✅ | ✅ | ❌ | ❌ | via fallback |
| **Comparison** (`=`, `<`, `>`) | ✅ | ✅ | ❌ | ❌ | via fallback |
| **`lambda` + closures** | ✅ | ❌ | ❌ | ❌ | via fallback |
| **`cons` / `car` / `cdr`** | ✅ | ❌ | ❌ | ❌ | ❌ |
| **`null?` / `pair?`** | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Symbols + `quote`** | ✅ | ❌ | ❌ | ❌ | ❌ |
| **`print` / I/O** | ✅ | ❌ | ❌ | ❌ | ❌ |
| **`letrec` / mutual recursion** | partial | ✅ | ❌ | ❌ | via fallback |

Cells marked "via fallback" mean the JIT/WASM path falls back
to interpretation transparently — the *behaviour* is correct,
but the function isn't actually JIT-compiled.

## Phase plan

### Phase 1 — Branching + functions on every native backend

Goal: every primitive marked with ✅ on the JVM column gets a ✅
on CLR and BEAM.  Today JVM is the most-covered backend; CLR
and BEAM are missing `if`, `define`, function calls, comparison.

**Scope:**
- Extend `ir-to-cil-bytecode` to emit `BRANCH_Z` / `BRANCH_NZ` /
  `JUMP` / `CMP_*` / `CALL` (some already partial).
- Extend `ir-to-beam` similarly — branch lowering needs live-
  register tracking because the BEAM verifier checks operand
  liveness across labels.
- Extend `twig-clr-compiler` and `twig-beam-compiler` frontends
  to accept `if`, `define`, comparison, function calls, and
  recursion now that the lowering supports them.

Acceptance: `(define (fact n) (if (= n 0) 1 (* n (fact (- n 1))))) (fact 5)`
returns 120 from real `java`, real `dotnet`, AND real `erl`.

### Phase 2 — Closures and the JAR / multi-class story

This is JVM02 Phases 2 and 3 from the existing JVM02 spec, plus
parallel CLR and BEAM work:

- **JVM**: `jvm-jar-writer` ships (✅), then multi-class
  lowering in `ir-to-jvm-class-file` (closures → captured-env
  class with `apply` method), then `twig-jvm-compiler` accepts
  `Lambda` and emits a JAR.
- **CLR**: same pattern.  Multi-class CLR is more natural than
  on JVM because PE/CLI already supports multiple types per
  assembly — no "JAR equivalent" needed; just emit more
  `TypeDef` rows.
- **BEAM**: closures are **fun objects** with a `FunT` chunk
  describing each `lambda` and a `MakeFun2` opcode at
  construction sites.  `beam-bytecode-encoder` needs to grow
  `FunT` support.
- **JIT/WASM**: closures already fall back to interpretation;
  Phase 4 (below) is about making them WASM-compileable too.

Acceptance: `((make-adder 7) 35) → 42` on every backend.

### Phase 3 — Heap primitives: cons, car, cdr, lists, symbols

This is the part that unlocks "Lisp on every backend".  Today
the heap lives Python-side in `twig.heap.Heap`, exposed to
`vm-core` via `call_builtin`.  The native backends have no
equivalent.

**Cross-backend convention:**
- A "heap handle" is a 64-bit tagged pointer:
  - bits 63-3: object identity
  - bits 2-0: type tag (0=int, 1=cons, 2=symbol, 3=closure,
    4=nil, 5..7=reserved)
- Tagged ints stay unboxed across all backends — no allocation
  for arithmetic.
- Cons cells / symbols / closures allocate via the **backend's
  native GC**:
  - JVM: emit a `Cons` / `Symbol` / `Closure` class; allocations
    use `new`; JVM GC reclaims.
  - CLR: same pattern; .NET GC reclaims.
  - BEAM: cons cells and atoms are first-class BEAM terms; use
    `put_list` / atom-table.  BEAM's GC reclaims.
  - WASM: this is where it gets interesting — see Phase 4.

**New IR ops** (additions to `compiler-ir`):
- `MAKE_CONS dst, head, tail` — allocate a cons cell.
- `CAR dst, src`
- `CDR dst, src`
- `IS_NULL dst, src` — sets dst=1 if src is nil.
- `IS_PAIR dst, src`
- `MAKE_SYMBOL dst, name_idx` — intern a symbol.
- `IS_SYMBOL dst, src`

Acceptance: `(define (length xs) (if (null? xs) 0 (+ 1 (length (cdr xs))))) (length (cons 1 (cons 2 (cons 3 nil))))`
returns 3 from every backend.

### Phase 4 — GC

This is the user-named requirement: "We also need to
incorporate garbage collection as well."

**Per-backend approach:**

- **vm-core (TW01 work, already specced in TW00)**: replace the
  refcounted `Heap` with a mark-sweep collector.  Same Python
  data structures, just walk them on a trigger and reclaim
  unreachable objects.
- **JVM / CLR / BEAM**: piggyback on the host GC.  Cons cells,
  closures, and symbols are real JVM/CLR/BEAM objects, so the
  host runtime collects them with no extra work from us.  This
  is the **simplest** GC story and lands automatically once the
  Phase 3 heap-handle convention is in place.
- **WASM**: WASM has no built-in GC.  Two paths:
  1. **WASM GC proposal** (post-MVP, supported in modern V8 / 
     SpiderMonkey).  Adds `struct.new`, `array.new` opcodes,
     reference types.  Cleaner but pins us to a specific WASM
     dialect.
  2. **Custom GC inside linear memory**.  Implement Cheney's
     two-space copying collector or a simple mark-sweep
     directly in the emitted WASM.  More portable but ~600 LoC
     of additional WASM emission.

  Recommendation: **path 2** for now — keeps `wasm-backend`
  self-contained.  Revisit when the WASM GC proposal stabilises
  in our `wasm-runtime`.

**New work item**: TW01 (vm-core mark-sweep) is already on the
roadmap.  TW04 will cover the WASM custom-GC.

### Phase 5 — Macros (explicitly out of scope)

User vision: "the only exception being macros."  Documented
non-goal.  If macros are ever needed, that's a separate spec
that adds `syntax-rules` to the grammar and a macro-expansion
pass in the compiler.

## Sister specs

| Spec | Scope |
|------|-------|
| TW00 | base Twig language + vm-core + refcounted heap |
| TW01 | mark-sweep GC for vm-core (already specced) |
| TW02 | twig-jvm-compiler (shipped) |
| TW02.5 | (becomes redundant — folded into Phase 2 above) |
| **TW03 (this)** | full Lisp surface + GC across every backend |
| TW04 | WASM custom-GC (Phase 4 sub-track) |
| JVM01 | recursion fix in ir-to-jvm-class-file (shipped) |
| JVM02 | JAR for closures (Phase 1 jar-writer shipped) |
| BEAM01 | Twig on real erl (Phases 2-4 shipped) |
| CLR01 | real-dotnet conformance for cli-assembly-writer (shipped) |

## Risk register

- **IR-op churn**.  Adding 7+ heap ops to `compiler-ir` requires
  every backend's lowering to opt in or explicitly reject.
  Mitigation: add ops one at a time; backends that don't support
  a new op raise a clear error rather than silently miscompile.
- **Cross-backend value type drift**.  The 64-bit tagged-pointer
  convention only works if every backend agrees on the tags.
  Mitigation: a single `compiler-ir.value_tags` module owns the
  enum; every backend imports it.
- **WASM GC complexity**.  Implementing Cheney's collector
  inside WASM linear memory is ~600 LoC of careful work.
  Mitigation: TW04 is its own focused sub-spec; not blocking
  Phases 1-3.
- **Symbol interning across backends**.  Symbols need a canonical
  identity so `(eq? 'foo 'foo)` is true.  JVM/CLR/BEAM each
  have native atom/symbol-like primitives we can lean on; WASM
  needs explicit string-table interning.

## Acceptance criteria for this spec

This spec is "done" when:

1. `(define (fact n) (if (= n 0) 1 (* n (fact (- n 1))))) (fact 5)`
   returns 120 from real `java`, real `dotnet`, and real `erl`
   (not just JVM, where it works today).
2. `((make-adder 7) 35)` returns 42 from real `java` and real
   `dotnet`.
3. `(length (cons 1 (cons 2 (cons 3 nil))))` returns 3 from real
   `java`, real `dotnet`, and real `erl`.
4. `(eq? 'foo 'foo)` returns #t from every backend.
5. A pathological program that allocates 100 000 cons cells
   without holding references to them runs to completion on
   every backend (i.e. the GC works).
