# LANG-VM Platform — Vision, Architecture & Roadmap

**Status:** vision / north-star — for sign-off and iteration.
**Supersedes in scope:** none (this is the umbrella the `LANG*` specs implement).
**Relates to:** `LANG-PLATFORM-MATRIX.md` (the plumbing proof), `LANG-FULL-IMPLEMENTATION.md`
(the per-language feature campaign), `lang-full-e6-dispatch.md` (dynamic dispatch),
`LANG00`–`LANG77`, `LS00`–`LS04`.

---

## 0. The promise

> **You write a `.tokens` file and a `.grammar` file. In return you get, for free
> or near-free: a lexer, a parser, a syntax highlighter, a language server, a
> debugger, ahead-of-time compilation to *every* backend we support, and a JIT
> compiler. Your language runs debuggably on the VM at reasonable speed; paired
> with the JIT it flies; compiled AOT it produces a self-contained native
> executable in the class of GraalVM Native Image / .NET NativeAOT — and this
> holds for *both* statically-typed and dynamically-typed languages. A BASIC
> program should be able to reach V8-class performance. And because the IIR
> bridges to our Semantic IR (SIR), any language that compiles to IIR can *also*
> be transpiled to any language with an SIR backend — write BASIC, get idiomatic
> Ruby, Python, JavaScript, TypeScript, Go, Rust, or C.**

**No backend is privileged.** Optimization lives in *our own* backend-agnostic
middle-end, not in any single backend. **LLVM is one target among peers** (WASM,
JVM, CLR, native, BEAM) — used where it adds value, never a dependency and never a
constraint on our semantics. "Being fast" must not require shipping LLVM.

That is the whole point of the LANG-VM. Everything below measures the distance
between that promise and today, and sequences the work to close it.

Design lineage: the VM + JIT were designed with **V8** in mind (a fast baseline
interpreter that gathers type feedback, feeding a speculative optimizing JIT with
deoptimization). The AOT target is **GraalVM Native Image / CLR NativeAOT** (a
closed-world, whole-program, profile-informed native compile that works even for
dynamic languages). The DX target is **"a language in an afternoon."**

---

## 1. The pipeline (one architecture, many outputs)

```text
   .tokens + .grammar
        │  grammar-tools  (GrammarLexer / GrammarParser)         ── FREE ──►  lexer, parser, CST/AST
        ▼
   <lang>-iir-compiler        (the ONLY per-language code you must write)
        │  lowers AST → interpreter_ir::IIRModule   ◄── the NARROW WAIST
        ▼
   iir-builtin-lowering + type/refinement layer                        (SHARED)
        ▼
   OPTIMIZING MIDDLE-END      backend-agnostic: SSA opts, inlining, const-prop,   (SHARED)
        │                     DCE, spec­ialization. The SAME optimizer feeds JIT & AOT.
        │                     NO backend is privileged; LLVM is not the optimizer.
        │
        ├─────────────► VM (vm-core)         baseline, debuggable, reasonable perf
        │                    │ type feedback (observed_type, deopt profiling)
        │                    ▼
        ├─────────────► JIT (jit-core)       hot functions → specialized native code
        │
        ├─────────────► AOT                  whole program → one native/managed artifact
        │                    ├ native  (x86_64-backend / aarch64-backend)   self-contained
        │                    ├ LLVM    (iir-to-llvm)      ── ONE peer backend, not the optimizer
        │                    ├ WASM    (iir-to-wasm)
        │                    ├ JVM     (iir-to-jvm-class-file)
        │                    ├ CLR     (iir-to-cil-bytecode)
        │                    └ BEAM    (iir-to-beam)
        │
        └─────────────► IIR → SIR bridge  (iir-to-semantic-ir, NEW)   ── TRANSPILATION ──►
                             │  lifts IIR → semantic_ir::SIR
                             ▼
                        SIR backends:  Ruby · Python · JavaScript · TypeScript · Go · Rust · C
                        (any IIR-frontend language → any SIR-target language, automatically)

   Tooling, all driven off the SAME grammar + IIR (mostly FREE):
        LSP (LS00–LS02) · DAP debugger (dap-adapter-core, debug-sidecar, LANG06/13/14/25)
        REPL (LANG08) · notebook kernel (LANG09) · syntax highlighter · packager (LANG10)
```

The invariant that makes the promise possible: **a language author writes exactly
one component — the frontend that lowers their AST to the shared IIR.** Everything
to the left of the waist is generated from the grammar; everything to the right is
shared across all languages.

---

## 2. The four promises — bar, current state, gap

### Promise A — "Front-end and tooling for free"

**Bar:** grammar in → lexer, parser, syntax highlighter, LSP, and DAP debugger out,
with no hand-written language-specific tooling.

**State: strong.** `grammar-tools` drives lexer/parser generation for dozens of
languages (ALGOL, BASIC, CSS, Java, JavaScript, Haskell, C#, F#, …). The
LSP framework (`LS00`–`LS02`, `grammar-lsp-bridge`, per-language bridges) and DAP
stack (`dap-adapter-core`, `basic-dap`, `debug-sidecar`, `aot-debug`) exist, plus a
VSCode-extension generator (`LS04`).

**Gap:**
- **Syntax highlighter** generation from `.tokens` (e.g. emit a TextMate / Tree-sitter
  grammar) is not yet a first-class one-command output.
- **LSP feature parity** — hover/completion/rename/find-refs are uneven across
  languages; the grammar-driven server needs the type/refinement layer wired for
  semantic features.
- **One-command scaffold** — "new-language" should generate the frontend skeleton +
  wire all tooling; today it's several manual steps.

### Promise B — "Runs everywhere (AOT to all backends by default)"

**Bar:** every construct of every language compiles to and *runs on* all backends,
verified by execution.

**State: real and rigorously guarded.** 8+ languages share the pipeline; the
`lang_matrix.rs` guardrail *executes* each feature on every backend whose toolchain
is present and checks output — not "validated," executed. Static/imperative
languages (Nib, Brainfuck, BASIC, ALGOL, Oct) are largely complete.

**Gap:**
- **Dynamic languages** need the general dynamic-dispatch substrate — **E6** (in
  progress: cons + dynamic arithmetic land; lists/closures/records/symbols remain).
- **GC** (below) gates real dynamic programs.

### Promise C — "Debuggable VM with reasonable performance"

**Bar:** set a breakpoint, step, inspect locals — out of the box — while the VM
alone is fast enough for real use.

**State: partial-to-good.** The VM executes the full typed IIR; a debug-sidecar
format and DAP adapter exist (`LANG06/13`, `05d/05e`), and native-AOT debug info is
specced (`LANG14/25`).

**Gap:**
- Source-level debugging is not yet turnkey across all languages/backends.
- **Native-AOT debugging** (DWARF/PDB line tables for the direct-native path) is
  specced but not end-to-end.
- VM baseline speed is "reasonable interpreter," not yet a fast threaded/register
  baseline tuned for the JIT hand-off.

### Promise D — "Fast: JIT = V8-class, AOT = GraalVM/NativeAOT-class"

**This is the biggest gap and the heart of the vision.**

**JIT — bar:** hot code compiles to optimized **native machine code**, with
type-feedback specialization + deopt, so a hot BASIC loop approaches V8.

**JIT state:** the *architecture* is real — tiered (VM gathers `observed_type`),
speculative `specialise()` → CIR, a `CIROptimizer` (const-fold + DCE), a pluggable
`Backend`, and **deoptimization** (`deopt_rate` → invalidate/`UNSPECIALIZABLE`).
**But:** the shipped backends are `NullBackend`/`EchoBackend` + a generic CIR
executor — **host-native x86-64/aarch64 codegen is a "future backend," so the
compiled tier is fast specialized-CIR, not native machine code**; op coverage is
incomplete (falls back to the interpreter for arrays / the dynamic path); the
optimizer is const-fold + DCE only.

**AOT — bar:** a whole-program native executable in the class of GraalVM Native
Image / CLR NativeAOT — self-contained, fast-starting, small — for static *and*
dynamic languages.

**AOT state:** the direct-native backends (`x86_64-backend`/`aarch64-backend`) emit
*correct but unoptimized* machine code (no SSA/regalloc/inlining/scheduling). Today
the only way to get optimized native output is to route through `iir-to-llvm` and
lean on LLVM's `-O` passes — **but that is exactly the dependency we are rejecting**
(§0: LLVM is one peer backend, not our optimizer). The real gap is therefore a
**shared, backend-agnostic optimizing middle-end** (SSA-form IIR/CIR, inlining,
const-prop, DCE, register-allocation-agnostic scheduling, specialization) that
*every* backend consumes — so a native binary is fast because *our* optimizer made
it fast, whether or not LLVM is on the machine. Dynamic-language AOT (the hard
GraalVM-style case) additionally needs a whole-program closed-world specialization
pass, which does not exist yet.

### Promise E — "Universal transpilation (write any language, get any language)"

**Bar:** any language with an **IIR frontend** (BASIC, Nib, ALGOL, Oct, Brainfuck,
Twig, McCarthy, …) can be **transpiled to source** in any language with an **SIR
backend** (Ruby, Python, JavaScript, TypeScript, Go, Rust, C) — automatically,
without writing a per-language transpiler.

**State: new — the bridge does not exist yet.** The two IRs and their emitters are
built independently today: the **IIR** side (interpreter IR → machine code, this
platform) and the **SIR** side (semantic IR → source; frontends
`{javascript,python,ruby,twig}-to-semantic-ir`, backends
`semantic-ir-to-{c,go,javascript,python,rust,typescript}`). **Twig already has both
a `twig-ir-compiler` (→ IIR) and a `twig-to-semantic-ir` (→ SIR)** — proving a
language *can* be dual-path, but requiring a hand-written second frontend.

**The unlock is one bridge: `iir-to-semantic-ir`.** Lift an `IIRModule` to a
`semantic_ir::SIR` module once, and *every* IIR-frontend language inherits *every*
SIR target — write BASIC, get Ruby/Python/Go/Rust/C source, for free. This unifies
the two narrow waists into one platform: a single frontend (grammar + IIR lowering)
yields **both** native execution (VM/JIT/AOT) **and** source-to-source
transpilation.

**Gap / hard part:** IIR is *lower* than SIR (it has lowered some source structure —
a `for` may be conditional jumps, a high-level call may be a `call_builtin`). Lifting
IIR → SIR is therefore correctness-first (reconstruct structured control flow, map
ops to SIR nodes) with idiom-fidelity as a follow-on — and may motivate IIR carrying
a little more semantic metadata (loop/collection provenance) so the lift emits
*idiomatic* target code, not just correct code. See §3.6.

---

## 3. The hard problems (named honestly)

1. **A backend-agnostic optimizing middle-end (the LLVM-independence problem).** We
   deliberately do *not* want optimization quality to depend on LLVM. So the platform
   needs its *own* optimizer over the shared IIR/CIR — SSA form, inlining,
   const/copy-propagation, DCE, loop and scalar opts, and specialization — feeding
   *all* backends (native, WASM, JVM, CLR, and yes LLVM as a peer). The direct-native
   backends then emit already-optimized code; LLVM, when present, is a peer target
   that may add its own final lowering, never the source of our speed. This is the
   Cranelift/GraalVM philosophy — own your middle-end — and it is the single largest
   build on the "fast" promise.
2. **A native JIT backend.** The direct-native emitters already produce machine
   code — they are not yet wired into the JIT's `Backend` trait. Bridging them is
   the single highest-leverage step toward "the JIT flies."
3. **Garbage collection.** Today the dynamic heap is `Box::leak`/bump — it *leaks*.
   The LANG20 GC-ABI `ObjectHeader` substrate exists, but the collector does not
   (`gc-core` returns `false`). A real (generational, precise) GC is a **prerequisite**
   for dynamic languages, not a nicety.
4. **Dynamic-language *performance* (the V8 promise).** Ruby/Python/Lua/BASIC-at-V8
   speed cannot be AOT-typed; it needs **runtime type feedback → inline caches →
   speculative specialization → deopt**. The *infrastructure* exists (VM profiling +
   deopt tracking); the *specialization of dynamic dispatch* does not. This is the
   deepest, longest-horizon work (V8/PyPy territory), and it is the same substrate
   E6 lays down.
5. **Dynamic AOT (the GraalVM promise).** GraalVM Native Image / NativeAOT make a
   **closed-world** assumption and do whole-program points-to + specialization +
   a minimal runtime. Reaching that for our dynamic languages means a
   whole-program analysis pass + profile-guided specialization at AOT time + a
   packaged GC'd runtime — a major, distinct effort from the JIT.
6. **The IIR → SIR lift (§2 Promise E).** Bridging the two IRs is a *lift* (lower →
   higher), the harder direction: IIR has already lowered some source structure, so
   reconstructing SIR-level semantics is correctness-first (rebuild structured
   control flow, map ops), with *idiomatic* output as a second stage. The cleanest
   design likely has the **frontend annotate IIR with semantic provenance** (this
   loop was a `for`; this call was a method; this was a collection op) so the lift
   emits idiomatic target code, not decompiler-grade output. A design spec
   (`iir-to-semantic-ir.md`) owns this.

---

## 4. Roadmap (phased; each phase is independently valuable)

Ordering principle: **make dynamic languages *possible* before making them *fast*;
make the JIT emit *native code* before making it *optimal*.**

### Phase 0 — Foundation (in progress)
- **E6 dynamic dispatch** — the shared substrate every dynamic language rides
  (`lang-full-e6-dispatch.md`). Cons ✅, dynamic arithmetic ✅; lists, symbols,
  records/unions, closures-on-WASM, dynamic globals remain.
- **Real GC** (`LANG16`) — promote `gc-core` from scaffolding to a working precise/
  generational collector over the existing `ObjectHeader` ABI. *Gates all dynamic
  languages for real workloads.*

### Phase 1 — The JIT emits native code ("VM + JIT flies")
- **Bridge the direct-native backends into `jit-core::Backend`** → a genuine
  machine-code JIT (the emitters already exist).
- **Complete JIT op coverage** — no silent fallback to the tree-walker (arrays, the
  dynamic path).
- **A fast VM baseline** tuned for the JIT hand-off (threaded/register dispatch).
- *Proof gate:* a hot numeric loop (e.g. BASIC `FOR`) within a small constant factor
  of an equivalent V8 run.

### Phase 2 — The backend-agnostic optimizer ("GraalVM/NativeAOT-class, no LLVM lock-in")
- **Build our own optimizing middle-end** over the shared IIR/CIR — SSA form,
  inlining, const/copy-prop, DCE, loop/scalar opts, specialization — that **every**
  backend consumes. This is the deliberate LLVM-independence: our native binaries are
  fast because *our* optimizer made them fast. LLVM stays a peer backend that may add
  final lowering, never a required dependency.
- **Shared JIT/AOT optimizer** — the same middle-end serves the JIT's hot-path
  compile and the AOT whole-program compile (the JIT's existing `CIROptimizer` is the
  seed).
- **End-to-end native-AOT debug info** (`LANG14/25`) so optimized native binaries are
  still debuggable.
- *Proof gate:* a static-language program AOT'd to a self-contained native binary
  (via the direct-native backend, **LLVM absent**) with start-up + throughput in the
  NativeAOT class.

### Phase T — IIR↔SIR bridge (universal transpilation) — *parallel track*
Independent of the performance phases; can proceed alongside Phase 0–2.
- **`iir-to-semantic-ir` bridge** — lift an `IIRModule` → `semantic_ir::SIR`,
  correctness-first, so every IIR-frontend language gains every SIR target
  (Ruby/Python/JS/TS/Go/Rust/C).
- **Semantic-provenance annotations on IIR** (loop/method/collection origin) so the
  lift emits *idiomatic* target source, not decompiler-grade code.
- *Proof gate:* a BASIC (or Nib/ALGOL) program transpiles to runnable Ruby **and**
  Go **and** Rust that produces the same result — verified by running the emitted
  source. (`iir-to-semantic-ir.md` design spec first.)

### Phase 3 — Dynamic speed ("V8-class dynamic & BASIC")
- **Type-feedback inline caches** on the dynamic (`ref<any>`) dispatch path, driven
  by the VM's existing profiling.
- **Speculative specialization + deopt** of dynamic `call_builtin`/method dispatch in
  the JIT (`LANG20` multilang-runtime: ICs, deopt).
- **Profile-guided dynamic AOT** — a closed-world specialization pass so a dynamic
  program can AOT to a fast native binary (the GraalVM-style case).
- *Proof gate:* a dynamically-typed hot loop within a small constant factor of V8.

### Phase 4 — DX completeness ("a language in an afternoon")
- **One-command scaffold:** `.tokens` + `.grammar` → frontend skeleton + all tooling
  wired.
- **Syntax-highlighter generation** (TextMate / Tree-sitter) from `.tokens`.
- **LSP feature parity** (semantic hover/complete/rename/refs via the type layer).
- **Turnkey debugger UX** across languages/backends.

### Phase 5 — New frontends (the payoff)
- **Lua** (small, dynamic — the natural first new dynamic frontend on the Phase-0/3
  substrate), then **Python**, **Ruby** (large dynamic semantics + stdlib, built
  incrementally per the north-star), and **C/C++** (static, low-level — pointers/
  structs/unions/UB; leans on E5 + byte-memory, and is the *easiest* to optimize).
- Repo-specific DSLs ride the same rails for free.

---

## 5. Success criteria (measurable)

- **DX:** a new toy language (tokens+grammar+frontend) runs on all backends *and*
  has working LSP + debugger in < 1 day.
- **Coverage:** every language's grammar constructs execute on every backend
  (the matrix stays green as languages are added).
- **JIT:** a hot numeric loop within ~2–3× of V8; **AOT (static):** within the
  NativeAOT class on start-up + throughput; **AOT/JIT (dynamic):** within a small
  constant factor of V8 on a specialized hot path.
- **Transpilation:** an IIR-frontend program (e.g. BASIC) emits runnable source in
  every SIR target (Ruby/Python/JS/TS/Go/Rust/C) that produces the same result —
  verified by running the emitted source.
- **LLVM-independence:** the optimized-AOT proof gate passes with LLVM *not
  installed* — our middle-end, not LLVM, is the source of native speed.
- **Rock-solid:** GC'd (no leaks), and the shared IIR + passes carry property-test /
  fuzz coverage (the narrow waist's blast radius demands it).

---

## 6. Non-goals & sequencing discipline

- **No LLVM lock-in.** Optimization lives in *our* backend-agnostic middle-end;
  LLVM is one peer backend, never a dependency and never a constraint on our IR
  semantics. "Fast" must hold with LLVM absent.
- Do not chase dynamic *speed* (Phase 3) before dynamic *correctness* (Phase 0) and a
  native JIT (Phase 1) — a fast wrong answer is worthless, and ICs need a native
  tier to specialize into.
- Keep the "one component per language" invariant sacred: any feature that would
  force per-language backend code is a design smell — push it into the shared IIR /
  lowering passes (as E6 and the string/array work did).
