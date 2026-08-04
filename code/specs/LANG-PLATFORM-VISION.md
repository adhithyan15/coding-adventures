# LANG-VM Platform — Vision, Architecture & Roadmap

**Status:** vision / north-star — for sign-off and iteration.
**Supersedes in scope:** none (this is the umbrella the `LANG*`/`LS*`/`SIR*` specs implement).
**Relates to:** `LANG-PLATFORM-MATRIX.md`, `LANG-FULL-IMPLEMENTATION.md`,
`lang-full-e6-dispatch.md`, `LANG00`–`LANG77`, `LS00`–`LS04`, the `SIR*` specs.

---

## 0. The promise

> **You write a `.tokens` file and a `.grammar` file, and one frontend that lowers
> your AST to a shared IR. In return you get, for free or near-free: a lexer, a
> parser, a syntax highlighter, a language server, a debugger, ahead-of-time
> compilation to *every* backend we support, a JIT compiler, and source-to-source
> transpilation to *every* other language on the platform. Your language runs
> debuggably on the VM at reasonable speed; paired with the JIT it flies; compiled
> AOT it produces a self-contained native executable in the class of GraalVM Native
> Image / .NET NativeAOT — for *both* static and dynamic languages. A BASIC program
> should be able to reach V8-class performance.**

Two standing constraints on how we get there:

- **No backend is privileged.** Optimization lives in *our own* backend-agnostic
  middle-end, not in any single backend. **LLVM is one target among peers** (WASM,
  JVM, CLR, native, BEAM) — used where it adds value, never a dependency, never a
  constraint on our semantics. "Being fast" must hold with LLVM absent.
- **One component per language.** A language author writes exactly one thing — a
  frontend that lowers their AST to a canonical IR. Everything before it (lexer,
  parser, highlighter, LSP) is generated from the grammar; everything after it (the
  IRs, optimizer, backends, bridges, tooling) is shared across all languages.

---

## 1. The model — composable stages, many pipelines

The platform is **not** one linear chain. It is a **set of typed stages** (the
packages), each a transform between a small number of **canonical representations**.
A **pipeline is a validated composition** — a path through the stage graph — chosen
by *(source language, desired output, execution mode)*.

### 1.1 The canonical representations (the "types" that flow between stages)

```
Source text ─►[lexer]─► Tokens ─►[parser]─► AST/CST ─►[frontend]─┐
                                                                  ├─► IIR  (LOW — typed, execution layer)
                                                     [bridges] ⇅   │
                                                                  └─► SIR  (HIGH — source-idiom / semantic layer)

  from IIR  ─► native · WASM · JVM · CLR · BEAM · VM · JIT · AOT        (execute / machine code)
  from SIR  ─► Ruby · Python · JavaScript · TypeScript · Go · Rust · C  (transpile / source)
```

**IIR and SIR are the two hubs.** Everything routes through one or both:

- **IIR** (`interpreter_ir`) — the low, execution IR: typed, lowered, close to the
  machine. Runs on the VM/JIT/AOT and all machine backends.
- **SIR** (`semantic_ir`) — the high, semantic IR: preserves source-level idiom
  (methods, blocks, exceptions, collection ops). Emits idiomatic source in any target
  language.

The two are **bridged both ways** (§1.4), so a language enters at whichever hub fits
and reaches *every* output.

### 1.2 The stages (each package declares: consumes → produces)

| Stage (package family) | Consumes | Produces |
|---|---|---|
| `grammar-tools` (GrammarLexer/Parser) | `.tokens`/`.grammar` + Source | Tokens, AST/CST |
| `<lang>-iir-compiler` | AST | **IIR** |
| `<lang>-to-semantic-ir` | AST | **SIR** |
| `iir-builtin-lowering` passes | IIR | IIR (lowered) |
| **optimizer** (middle-end) | IIR/CIR | IIR/CIR (optimized) |
| `vm-core` / `jit-core` | IIR | execution / native code |
| `iir-to-{llvm,wasm,jvm-class-file,cil-bytecode,beam}`, native backends | IIR | machine/bytecode |
| `iir-to-semantic-ir` *(new bridge — lift)* | IIR | SIR |
| `semantic-ir-to-iir` *(new bridge — lower)* | SIR | IIR |
| `semantic-ir-to-{ruby,python,javascript,typescript,go,rust,c}` | SIR | target source |
| tooling: LSP (`LS00`–`LS02`), DAP (`dap-adapter-core`, `debug-sidecar`), REPL, notebook, packager | grammar + IR | editor/debug/run surfaces |

### 1.3 A pipeline is a wiring of those stages

Same package set, many pipelines:

| Goal | Pipeline |
|---|---|
| BASIC → native exe | `basic → AST → IIR → optimizer → x86_64-backend` |
| BASIC → Ruby source | `basic → AST → IIR → iir→sir → SIR → sir→ruby` |
| **Ruby → native exe** | `ruby → SIR → sir→iir → IIR → optimizer → native` |
| Ruby → Python | `ruby → SIR → sir→python` |
| Twig → WASM | `twig → AST → IIR → iir→wasm` |
| BASIC → JIT run | `basic → AST → IIR → vm-core + jit-core` |
| Python → Go | `python → SIR → sir→go` |

Nothing is duplicated: the optimizer, the two IRs, every backend, and each bridge are
written **once** and reused by every pipeline.

### 1.4 The two bridges (the fold)

Folding SIR into the platform is exactly the pair of bridges between the hubs:

- **`iir-to-semantic-ir` (LIFT, IIR → SIR).** Gives every IIR-frontend language
  (BASIC, Nib, ALGOL, Oct, Brainfuck, Twig, McCarthy) transpilation to every SIR
  target. **Harder direction** (low → high): IIR has lowered some source structure,
  so output is correctness-first, and *idiomatic* output needs the frontend to
  annotate IIR with semantic provenance (this loop was a `for`; this was a method;
  this was a collection op).
- **`semantic-ir-to-iir` (LOWER, SIR → IIR).** The **natural** direction (high →
  low), and the **near-term jackpot**: Ruby / Python / JavaScript already have SIR
  frontends, so this bridge gives them **native compilation + JIT + AOT via the LANG
  VM for free — no new frontend to write.** Its lowering of OOP/exceptions/
  collections/dynamic-dispatch *is* the E6 + GC substrate (§3), so it is gated on
  Phase 0.

Result: **one platform, two layers, bridged both ways.** The "IIR-VM work" and the
"SIR work" are the same stage-set, not two projects.

### 1.5 The composition layer (the architectural principle)

Because pipelines are compositions, the platform wants a **thin composition/driver
layer**: every stage declares its **port types** (which representation in, which out);
a driver **assembles, type-checks, and routes** a pipeline for any *(input, output)*
request — rejecting compositions that don't type-check and choosing the shortest
valid path. `lang-aot` is the first instance of this driver (frontend → IIR passes →
backend); the generalization routes through *either or both* hubs, so `BASIC→Ruby`
and `Ruby→native` are just two entries in the same routing table.

---

## 2. The promises — bar, current state, gap

### Promise A — Front-end & tooling for free
**Bar:** grammar in → lexer, parser, highlighter, LSP, DAP debugger out.
**State: strong.** `grammar-tools` drives lexer/parser for dozens of languages; the
LSP framework (`LS00`–`LS02`, `grammar-lsp-bridge`) and DAP stack (`dap-adapter-core`,
`basic-dap`, `debug-sidecar`, `aot-debug`) exist, plus a VSCode-extension generator.
**Gap:** syntax-highlighter generation (TextMate/Tree-sitter) from `.tokens`; LSP
semantic-feature parity; a one-command "new language" scaffold.

### Promise B — Runs everywhere (all execution backends)
**Bar:** every construct of every language *executes* on every backend.
**State: real, execution-guarded** (`lang_matrix.rs` runs + output-checks each
feature per backend). Static/imperative languages largely complete.
**Gap:** dynamic languages need E6 (in progress); GC gates real workloads.

### Promise C — Debuggable VM, reasonable perf
**Bar:** breakpoint/step/inspect out of the box; VM fast enough to be useful.
**State: partial-to-good** (VM + DAP + debug-sidecar; native-AOT debug info specced).
**Gap:** turnkey source-level debugging across languages/backends; native-AOT debug
info end-to-end; a faster VM baseline tuned for the JIT hand-off.

### Promise D — Fast (JIT = V8-class, AOT = GraalVM/NativeAOT-class), no LLVM lock-in
**Bar:** hot code → optimized **native** machine code with type-feedback + deopt;
whole-program AOT in the NativeAOT class — via *our* optimizer, LLVM optional.
**JIT state:** the *architecture* is real (tiered; VM gathers `observed_type`;
speculative `specialise()` → CIR; `CIROptimizer`; deopt via `deopt_rate`). **But** the
shipped backends are `Null`/`Echo` + a CIR executor — **host-native codegen is a
"future backend," so the compiled tier is fast specialized-CIR, not machine code;**
op coverage is incomplete (falls back to the interpreter); the optimizer is
const-fold + DCE.
**AOT state:** direct-native backends emit *correct but unoptimized* code. The only
optimized path today routes through LLVM — **exactly the dependency we reject.** The
real gap is a **shared, backend-agnostic optimizing middle-end** (SSA, inlining,
const/copy-prop, DCE, specialization) that *every* backend consumes, so native
binaries are fast because *our* optimizer made them fast. Dynamic AOT additionally
needs a whole-program closed-world specialization pass.

### Promise E — Universal transpilation (write any language, get any language)
**Bar:** any IIR-frontend language transpiles to any SIR target, and any SIR-frontend
language compiles to native — automatically, via the §1.4 bridges.
**State: new — the two bridges do not exist yet.** The hubs and their emitters are
built independently; Twig already has *both* an IIR and an SIR frontend, proving the
dual path but by hand-writing two frontends. The bridges make it free for everyone.
**Gap / hard part:** the LIFT (`iir→sir`) has an idiom ceiling set by IIR provenance;
the LOWER (`sir→iir`) is natural but gated on E6 + GC (its target substrate).

---

## 3. The hard problems (named honestly)

1. **A backend-agnostic optimizing middle-end (the LLVM-independence problem).** Our
   *own* optimizer over the shared IIR/CIR — SSA, inlining, const/copy-prop, DCE,
   loop/scalar opts, specialization — feeding *all* backends (LLVM a peer). The
   Cranelift/GraalVM philosophy: own your middle-end. The single largest build on
   the "fast" promise.
2. **A native JIT backend.** The direct-native emitters already produce machine code;
   they are just not wired into `jit-core::Backend`. Bridging them is the highest-
   leverage step toward "the JIT flies," and it shares the Problem-1 optimizer.
3. **Garbage collection.** The dynamic heap is `Box::leak`/bump — it *leaks*. The
   LANG20 `ObjectHeader` GC-ABI exists; the collector (`gc-core`) does not. A real
   precise/generational GC is a **prerequisite** for dynamic languages and for
   `sir→iir` execution.
4. **Dynamic-language *performance*.** Ruby/Python/Lua/BASIC-at-V8 can't be AOT-typed;
   it needs runtime type feedback → inline caches → speculative specialization →
   deopt (`LANG20`). The infrastructure exists (profiling + deopt); the specialization
   of dynamic dispatch does not. Deepest, longest-horizon work; shares E6's substrate.
5. **Dynamic AOT (the GraalVM case).** Closed-world whole-program points-to +
   profile-guided specialization + a packaged GC'd runtime. A major effort distinct
   from the JIT.
6. **The two IR bridges (§1.4).** `sir→iir` (lower) is natural but gated on E6 + GC.
   `iir→sir` (lift) is the harder direction; idiomatic output likely needs IIR to
   carry semantic provenance. Own spec: `iir-semantic-ir-bridge.md`.

---

## 4. Roadmap (phases + a parallel track; each independently valuable)

Ordering: **make dynamic languages *possible* before *fast*; make the JIT emit
*native code* before *optimal*; build the *optimizer* before chasing peak numbers.**

### Phase 0 — Foundation *(in progress)*
- **E6 dynamic dispatch** (`lang-full-e6-dispatch.md`) — the substrate every dynamic
  language and the `sir→iir` bridge ride. Cons ✅, dynamic arithmetic ✅; lists,
  symbols, records/unions, closures-on-WASM, dynamic globals remain.
- **Real GC** (`LANG16`) — promote `gc-core` from scaffolding to a working collector.
  *Gates all dynamic languages and native execution of the SIR half.*

### Phase 1 — The JIT emits native code ("VM + JIT flies")
- Bridge the direct-native backends into `jit-core::Backend`; complete op coverage;
  a fast VM baseline. *Proof:* a hot BASIC `FOR` within a small factor of V8.

### Phase 2 — The backend-agnostic optimizer ("fast, LLVM-optional")
- Build our own optimizing middle-end (SSA/inlining/DCE/specialization) that every
  backend consumes and that serves both JIT and AOT (the `CIROptimizer` is the seed).
- End-to-end native-AOT debug info (`LANG14/25`). *Proof:* a static program AOT'd to a
  self-contained native binary **with LLVM absent**, start-up + throughput in the
  NativeAOT class.

### Phase 3 — Dynamic speed ("V8-class dynamic & BASIC")
- Type-feedback inline caches + speculative specialization + deopt on the dynamic
  path (JIT), and profile-guided dynamic AOT. *Proof:* a dynamic hot loop within a
  small factor of V8.

### Phase 4 — DX completeness ("a language in an afternoon")
- One-command scaffold; TextMate/Tree-sitter highlighter gen; LSP feature parity;
  turnkey debugger UX.

### Phase 5 — New frontends (the payoff)
- **Reuse first:** Ruby/Python/JS get native compilation via `sir→iir` (Phase 0 +
  Track T) — no new frontend. Then **Lua** (grammar → IIR), then **C/C++** (static,
  low-level; leans on E5 + byte-memory; easiest to optimize). Repo DSLs ride free.

### Track T — The IR bridges (universal transpilation) *(parallel to Phase 0–2)*
- **`sir→iir` (lower)** — first, once Phase 0 lands: gives the SIR languages native
  execution. High value, natural direction.
- **`iir→sir` (lift)** + **IIR semantic-provenance annotations** — gives IIR languages
  idiomatic transpilation. *Proof:* a BASIC program transpiles to runnable Ruby **and**
  Go **and** Rust producing the same result (verified by running the emitted source).
- Design spec `iir-semantic-ir-bridge.md` first.

---

## 5. Success criteria (measurable)

- **DX:** a new toy language runs on all backends *and* has working LSP + debugger in
  < 1 day.
- **Coverage:** every grammar construct executes on every backend (matrix stays green
  as languages are added).
- **JIT:** hot numeric loop within ~2–3× of V8. **AOT (static):** NativeAOT-class
  start-up + throughput. **Dynamic:** within a small factor of V8 on a specialized
  hot path.
- **LLVM-independence:** the optimized-AOT gate passes with **LLVM not installed**.
- **Transpilation:** an IIR-frontend program emits runnable source in every SIR target
  producing the same result; a SIR-frontend language compiles to a native binary —
  both verified by running.
- **Rock-solid:** GC'd (no leaks); the shared IRs + passes carry property-test / fuzz
  coverage (the hubs' blast radius demands it).

---

## 6. Non-goals & sequencing discipline

- **No LLVM lock-in.** Optimization is *ours*; LLVM is one peer backend, never a
  dependency or a constraint on our IR semantics.
- Do not chase dynamic *speed* (Phase 3) before dynamic *correctness* (Phase 0) and a
  native JIT (Phase 1); ICs need a native tier to specialize into.
- Keep the **one-component-per-language** and **stage-port-typing** invariants sacred:
  any feature that would force per-language backend code, or a stage that only works
  in one pipeline, is a design smell — push it into a shared stage or an IR hub (as
  the E6 / string / array work did).
