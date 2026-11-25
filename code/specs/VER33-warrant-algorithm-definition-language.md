# VER33 — Warrant as the algorithm definition language

**Status:** Draft for owner approval and adversarial review. Spec only; no code
exists and none is written with it. It records owner decisions of 2026-11-24
(section 0) and the questions they leave open (section 12). Designed against the
**revised** [VER32](VER32-proof-interchange-format.md) (separate signed model files,
environment digests, no producer-written model definitions) as read on 2026-11-24.
Implementation is the `ADL-*` queue of section 11, which starts only after the owner
approves this design and an adversarial review has been answered.

**Layer:** Specification language, layer 4: the user-facing object above the proof
machinery. [VER04](VER04-warrant-language.md) is Warrant, the language people write.
[VER11a](VER11a-lean4-proof-kernel.md) is the kernel that decides what is a theorem;
VER32 is how a proof reaches it. This document decides how a *definition of an
algorithm* travels through all of that, to real code in every language lane and
back, **and how a team that wants only a slice of it can take only that slice**.

**Builds on:** VER04 and the family VER04a to VER04s (pure profile; type system
VER04d; code generation VER04e; claims to properties VER04f, VER04h; polyglot lanes
VER04i; fixed-width types VER04j; sequence built-ins VER04k; stateful objects
VER04m; collections VER04o; floats VER04p; corpus v2 VER04r; recursive types
VER04s); [VER05](VER05-property-engine.md) and [VER06](VER06-coverage-guided-fuzz-engine.md)
(property and fuzz engines); [VER25](VER25-wir-core-interpreter.md) (WIR, section 8
normative semantics), VER25a, VER25b; VER11a sections 0, 1, 6, 7, 9 to 11, 13, 14;
VER32; [VER31](VER31-tiered-release-standard.md) (claim rungs, buckets `a|b|c`);
[LEAN00](LEAN00-lean-on-the-generic-pipeline.md);
[LANG24](LANG24-constraint-vm.md) and VER34 (the solver, being drafted in
parallel); [LG01](LG01-logic-gates-contract.md), [LG02](LG02-basic-gates-cross-lane-contract.md)
and [VER-BACKLOG.d/logic-gates-everywhere.md](VER-BACKLOG.d/logic-gates-everywhere.md)
(the first adopter).

**Amends:** nothing by rewriting. Section 13 lists short dated notes that the named
specifications should carry.

---

## 0. The design in one page

**The owner's decisions (2026-11-24).**

1. The repository wants an **algorithm definition language**, and it is **Warrant,
   extended**, not a new language. "We can compile, check that against the kernel,
   prove that it works and then compile the algorithm definition language into actual
   code. We will also support the other way where we can convert code into the
   algorithm definition language that our kernel understands."
2. **Partial adoption is first-class.** "People should also be able to use partial
   features from it. If someone already has code, they should be able to generate
   exhaustive fuzz tests, pbt tests and others from a Warrant file." Nobody is forced
   to adopt code generation or proofs; existing code never has to be regenerated.
   Section 1 is the adoption ladder, and every later section says which rung it
   belongs to.
3. Generated code must be **idiomatic** in each language (section 7.7).
4. The repository's **Constraint VM grows into Warrant's solver** (VER34 designs it;
   section 6.3 here says how Warrant uses it and the rule that it is untrusted).

**The loop.**

1. One Warrant definition (`model` functions, structural or measured recursion,
   `claim`s) is written in an opt-in **algorithm profile** (section 4).
2. An untrusted **elaborator** turns each model function into a kernel definition in a
   **separate model PIF file**, and each claim into a **goal** (section 5).
3. **Proof producers** (case split, induction, SAT/SMT with certificates, Lean as the
   escape hatch) prove the goals. The kernel checks them (section 6).
4. An untrusted **compiler** turns the definition into **WIR** with a certificate; the
   kernel checks that the WIR program *refines* the model (section 7). Untrusted
   **lane backends** turn the definition into idiomatic source per language.
5. In reverse, untrusted **lifters** turn lane source (hand-written or generated)
   into WIR, and the kernel checks the same refinement against the same model
   (section 8).

| # | Decision | Section |
|---|---|---|
| D1 | The algorithm definition language **is Warrant**, with an opt-in **algorithm profile**. Existing documents keep their meaning. New syntax goes through `code/grammars/warrant/*.tokens` and `.grammar` and the shared grammar-driven parser, never a hand-written parser | 3, 4.9 |
| D2 | **Adoption is a ladder, A0 to A4**; each rung is useful alone, and a higher rung never invalidates a lower one | 1 |
| D3 | **Totality is required** in the profile: structural recursion, bounded loops as `fold` over `range`, and well-founded recursion by a declared measure with a proof obligation | 4.1, 5.5 |
| D4 | **Errors and rejections are values.** Input contracts are part of the definition | 4.4 |
| D5 | **Shallow embedding.** Each model function is one kernel definition of the same name in a separate, reviewed, signed model file | 5 |
| D6 | The elaborator is untrusted for **validity** and carries **meaning**; three mitigations: signed model file, evaluator-versus-kernel differential, catalogue id `warrant.model.elaboration-faithful` (c) | 5.8 |
| D7 | **The solver is untrusted.** An UNSAT answer reaches `proved` only through a certificate reconstructed into a kernel proof; a counterexample is confirmed by evaluation | 6.3 |
| D8 | **Forward = per-function translation validation**; backends are untrusted and closed by the reverse path | 7 |
| D9 | **Reverse = lifting** with a reviewed subset semantics per lane; fail closed outside it | 8 |
| D10 | **Generated code is first an oracle and differential, not a replacement** (owner question Q1) | 7.8 |
| D11 | Floats, effects and state are **not** in the first profile | 4.5, 4.6 |
| D12 | The first adopter is the **eleven basic logic gates**, then adders, then ct-compare | 10 |

**What it costs.** A larger Warrant checker and elaborator; a reviewed prelude; a WIR
compiler with a certificate emitter; one backend per lane; and above all **a reviewed
kernel semantics of each lane's subset** (section 8.4). The adoption ladder is the
answer to that cost: A0 and A1 need none of it.

**What it buys.** One text yields examples, property tests, fuzzing, exhaustive runs,
solver-found edge cases, a kernel proof for every input, generated code in 18 lanes,
and a check that real code is the definition. A disagreement anywhere is a finding
about a named definition. Nothing asks anyone to trust a new compiler.

---

## 1. The adoption ladder

The ladder is the central principle of this document. Each rung is **independently
useful**, built on the ones below it only for *evidence*, never for *permission*.

| Rung | What you have | What you get | VER31 claim rung | Lane tooling it needs |
|---|---|---|---|---|
| **A0** | **Existing code + a Warrant file** | Bindings to the existing functions; generated unit tests and portable test vectors; property tests; coverage-guided fuzzing; exhaustive tests where the domain is small; hostile-input and rejection tests from input contracts | `tested` (rung 1); `exhaustive` where a domain is full (not yet emitted, LV-007d) | A lane harness (VER04i manifest and runner), PBT and fuzz bindings for that lane |
| **A1** | A0 + the **solver** | Inputs that satisfy, violate or sit exactly on the boundary of preconditions and input contracts; bounded model-versus-code checks; every counterexample becomes a permanent regression example | `tested`; `bounded(N)` when a checked certificate exists (VER31 rung 2) | **None per lane**: the solver runs where vectors are generated, and its output is portable test vectors |
| **A2** | A Warrant model in the algorithm profile | **Kernel proofs of the model's properties** for every input: truth tables, equivalences, rejection, "for every width" | The **model-theorem category** (VER32 Q8), *not* a package rung: nothing links the model to code yet | None: the proof is about the model |
| **A3** | A2 + **existing code** | Lift the code to WIR and check it against the model (per function) | `checked`, then `proved` once no (c) id is open (VER31 rungs 3 and 4) | A lifter and a reviewed subset semantics for the lane |
| **A4** | A2 + a backend | **Generated idiomatic code** in each lane, validated per function; the generated text can be lifted (A3) to close the backend | A0 evidence immediately; `checked`/`proved` through the WIR link (7) and the reverse check (8.7) | A backend per lane (and a lifter, to close it) |

### 1.1 What each rung takes and gives

- **A0 needs no change to the user's code and no model.** A Warrant file written
  today, with `op`s, `example`s, `claim`s, input contracts and bindings, drives
  generated tests. The oracles are Warrant's own operators, never the code under
  test. A team with 10,000 lines of existing code adopts A0 and stops there if it
  wants.
- **A1 changes how inputs are chosen, not what is claimed.** It is lane-independent:
  the solver produces portable vectors (TFV, VER04e; corpus, VER04i) that every lane
  already knows how to run.
- **A2 needs a model, not code.** A team can specify an algorithm and prove properties
  of the specification with no implementation at all.
- **A3 needs a model and existing code.** The code is not regenerated, reformatted or
  moved.
- **A4 needs a model and gives code.** It is the only rung that writes code, and it
  is always optional: generated code lands as a separate package or module by default
  (7.8).

A higher rung never *invalidates* a lower one: a package at A3 still runs its A0 tests
in CI; a lifter refusal (8.5) leaves the package at A0/A1/A2 and says so in the
report. **Fail closed** means "keep the lower rung", never "block the release"
(VER31 section 5: `Tested` is the minimum publishable tier).

### 1.2 What exists today, per rung

Read from the specifications on 2026-11-24; per-lane status is authoritative in the
[portable Warrant backlog shard](VER-BACKLOG.d/portable-warrant.md), not here.

| Rung | Exists | Missing |
|---|---|---|
| A0 | Parser, checker, bindings schema for Rust (VER04c, VER04d); ground evaluator; L0 test vectors and a Rust harness (VER04e); claims to property tests with relation-biased generators, bounded-exhaustive and fuzz suites (VER04f, VER04h); a property engine with replay and shrinking and PBT bindings for several lanes (VER05, VER05a to VER05f); a coverage-guided fuzz engine with lane bindings (VER06, VER06b to VER06g); the polyglot corpus and lane manifests (VER04i, VER04r) | Lanes without a runner (the 18-lane rollout is in progress); `exhaustive` with a stated `domain: full` (LV-007d); bindings beyond Rust for non-Rust ops are lane work (VER04i) |
| A1 | A SAT solver emitting DRAT (`cdcl-sat`), the Constraint VM stack (`constraint-core`, `-instructions`, `-engine`, `-vm`) and an SMT-LIB reader (`smt-lib-format`), per LANG24 | The solver as a Warrant service: boundary-value generation from contracts, model-versus-code bounded checks, the regression-test loop, a fuller theory set (VER34) |
| A2 | The kernel is in construction (KRN-002); the PIF spec is drafted (VER32) | The elaborator, the ADL prelude, producers (VER32 PIF-006, PIF-018) |
| A3 | A Rust-subset front end to WIR (`warrant-ir`, VER25a/b) | Reviewed Lean semantics of the Rust subset (VER25c, KRN-015/016); every other lane |
| A4 | Nothing | Compiler, backends, lifters (sections 7 and 8) |

### 1.3 Which rung each later section serves

| Section | Rung |
|---|---|
| 4 The algorithm profile, 5 Elaboration, 6.1 and 6.2 Proofs | **A2** |
| 6.3 The solver | **A1** (input generation, bounded checks); **A2/A3** (VC discharge) |
| 7 Forward | **A4** (the WIR compiler also serves A3's equivalence checking) |
| 8 Reverse | **A3**; **A4** (closing backends) |
| 9 Assumptions and rungs | all |
| 10 Logic gates | A0 and A1 on all 11 gates today; A2 at G-M1; A3 at G-M2; A4 at G-M4 |

---

## 2. Purpose, the loop and the trust

### 2.1 The picture

```
  FORWARD: definition to code                          (all arrows are untrusted programs)

  Warrant source ──► parser ──► checked AST ──┬──► ADL elaborator ───► model PIF file  (one per module, signed)
  (model, rec model,        (grammar-driven,  │      (section 5)       goals list      (claim id, goal term, digest)
   claim, realizes)          VER04c/d)        │
        │                                     ├──► WIR compiler ─────► WIR program + refinement certificate
        │ A0/A1: tests, PBT, fuzz,            │      (section 7.2)             │
        │ solver vectors for EXISTING code    │                                ▼
        ▼                                     │                      kernel: Refines(WIR, model f)   ◄── TRUSTED
   existing lane code                         │
                                              └──► lane backends ────► idiomatic lane source, 18 lanes
                                                     (section 7.6)           │
  REVERSE: code to definition                                                │
  lane source ──► lane lifter ──► WIR' + hints ──► kernel: Refines(WIR', model f) and lane-semantics(src) ≈ WIR'
  (hand-written     (grammar-driven,                       ▲
   or generated)     refuses outside the subset)           └── the SAME model file, the SAME goal shape as forward
                      (section 8)

  SOLVER (untrusted): preconditions ─► boundary inputs (A1); model vs code, bounded (A1);
                      verification conditions ─► UNSAT + certificate ─► kernel proof term (A2/A3)

  PROOFS about the definition, independent of code (A2):
  goals list + model PIF file ──► proof producers ──► PIF ──► kernel ◄── TRUSTED
```

The kernel appears three times and is the same kernel: proofs about the model,
refinement of compiled WIR, refinement of lifted WIR. Everything else is search.

### 2.2 What is trusted and what is not

| Component | Trusted? | Where its trust goes |
|---|---|---|
| `lean-kernel` (VER11a section 4) | **Yes** | `kernel.checker.correct` (c); `kernel.logic.consistent` (VER32 9.2: open until the owner decides) |
| PIF reader, SHA-256, import resolver, gate-binding code, goal printer (VER32 1.1) | **Yes** | `pif.reader.correct`, `tcb.pif.gate-binding`, `tcb.goal.printer` (all (c)); the always-present `tcb.report.checker-correct` |
| **Reviewed semantics files**: PIF prelude, **ADL prelude**, WIR semantics, the **ADL-to-WIR value encoding**, each lane's subset semantics | **Reviewed text signed by digest**: this is where *meaning* trust sits | `pif.prelude.faithful` (b), `adl.prelude.faithful` (b), `wir.semantics.lean-faithful` (c), `adl.wir-abi.faithful` (b), `src.<lane>.lean-semantics.faithful` (c) |
| **Platform compilers and runtimes** (`rustc`, `go`, `javac`, ...) and the layers below | **Trusted, bucket (a)**: "We can definitely trust the compiler. Everything else has to be verified." | `tcb.<lane>-toolchain.trusted` and the VER31 (a) ids |
| The **Warrant elaborator** | Untrusted for **validity**; carries **meaning** | signed model file (`<pkg>.model.faithful`, (b)) and `warrant.model.elaboration-faithful` (c) (5.8) |
| The **solver** (VER34) | **No** | none: UNSAT needs a certificate the kernel checks; SAT needs evaluation (6.3) |
| The **WIR compiler** and certificate emitter | **No** | none beyond the semantics files |
| **Lane backends** | **No** | `<lane>.backend.code-link` (c) until the reverse path covers the generated text (8.7) |
| **Lifters** and invariant inference | **No, for validity** (the goal names the source text, VER11a 14.3) | `src.<lane>.lean-semantics.faithful` (c) |
| **Proof producers** | **No** | none: an ill-typed term is rejected |
| Test, property, fuzz generators and harnesses | Not trusted for proof; they are rung A0 evidence | none |

### 2.3 Where meaning trust sits

Producers are untrusted for validity, and trust moves for *meaning* (VER32 1.3) into
three places:

1. **Warrant text to kernel text.** The elaborator: covered by the signed model file
   and by `warrant.model.elaboration-faithful` (5.8). It decides whether a proof is
   about the algorithm a person wrote.
2. **Kernel text about code.** WIR's semantics, the model-to-WIR value encoding and
   each lane's subset semantics: reviewed files with their own ids, and differential
   tests against real toolchains. A wrong one yields a true theorem about a different
   machine.
3. **What a goal says.** The signed goals list (VER32 3.6): a canonical term whose
   digest, with the environment digest, a person signs.

A wrong lifter or backend never yields a wrong *proof*; at worst a statement about
something other than the code given, which the signed goal and the
source-text-in-the-goal rule (VER11a 14.3) catch.

### 2.4 Why two directions

Forward alone leaves backends unverified: nothing says generated Rust is the WIR
program the kernel checked. Reverse alone leaves the model hand-written or absent.
Together **reverse closes forward**: generated code is lifted back and checked
against the same model, so a backend needs no proof; it needs to produce text the
lifter accepts and the kernel agrees with. It is also the only honest cover for code
that was never generated, such as the hand-written published gates.

---

## 3. Why extend Warrant

### 3.1 What exists

| Piece | Status | Source |
|---|---|---|
| `model f(x: T) -> R = expr`: pure, total, first-order, acyclic | checker and ground evaluator | VER04 section 4, VER04d |
| Types: Bool, Int, Nat, U8..U64, I8..I64, Bytes, `Bytes<N>`, `List`, `Option`, `Result`, tuples, records, variants, aliases, `error` types | v0 plus VER04j | VER04 3, VER04j 2 |
| Wrapping fixed-width arithmetic as the language semantics, `wrap<T>`, `try<T>` | specified and implemented | VER04j 2.3, VER04r 3.2 |
| `match` on Option/Result, `if`, first-order quantifiers in claims | checker | VER04 4 |
| `claim`, `example`, input contracts (`rejects ... with`), `ensures`, `bound`, `labels`, bindings to host `op`s | checker | VER04 5 to 7, VER04d |
| Ground evaluator; generated tests, properties, bounded-exhaustive and fuzz; portable model IR | implemented for the pure profile | VER04e, VER04f, VER04h, VER04i, VER04r |
| `fold`, `map`, `all`, `any`, `range` with a meter | specified (WX-D) | VER04k 2.7 |
| `rec model ... decreasing p`, structural recursion check, recursive types | **specified**, not implemented (WX-R1) | VER04s 4 |
| Collections and generics; stateful objects; floats | specified | VER04o, VER04m, VER04p |

Warrant today produces **tests, never implementations** (VER04e). Its `model`
functions are the algorithm in everything but name but are used only as oracles. The
gates' Warrant file shows it: `logic-gates-basic.warrant` declares the eleven gates
as `op`s (signatures bound to code) and writes only a helper `b2u` as a `model`; the
oracle is Warrant's own `and`, `or`, `not`. The *definition* of `nand_and` exists
only as Rust and 16 other texts.

### 3.2 What must be added

| Addition | Why | Rung | Section |
|---|---|---|---|
| Opt-in **algorithm profile** with the totality rules | A kernel definition must terminate | A2 | 4.1, 4.9 |
| `rec model ... measure`; `Nat` as a decreasing parameter | Loops over counts and numerically decreasing algorithms | A2 | 4.1, 5.5 |
| **Realization** (data in the binding schema: this `op` implements that model) | Today nothing says which definition an `op` implements | A3, A4 | 4.8 |
| **Elaborator** and reviewed **ADL prelude** | The kernel needs terms | A2 | 5 |
| **Solver service** for Warrant | Boundary inputs, bounded checks, VC discharge | A1 | 6.3 |
| **WIR compiler**, **lane backends** | Forward direction | A4 | 7 |
| **Lifters** and per-lane subset semantics | Reverse direction | A3 | 8 |
| Differential check: evaluator against kernel reduction | Ties the tests to the proof | A2 | 5.8 |

### 3.3 Alternatives considered

| Alternative | Why not |
|---|---|
| **A new algorithm definition language** | Two languages with overlapping types is two things to review and keep in sync. Warrant already has the grammar, checker, evaluator, type system, claim classes and eighteen lanes of generated evidence. A new language would re-earn all of that, and would force a *migration* before anyone got A0 |
| **A Lean subset as the definition language** | Makes Lean source the user-facing object, against the owner's principle that nobody writing a `.warrant` file looks at proof machinery (VER11a 0.1); puts a Lean elaborator (LEAN00 L3) on every definition's critical path; and the 17 non-Lean lanes need the Warrant evidence ladder anyway |
| **Dafny or F\* style: verified imperative programs** | Their definitions are programs; ours are pure functions, so one text is an executable oracle for every lane's tests *and* a kernel term with no heap. Their VC generators and SMT solvers are in the trust base; ours are search outside a small kernel. And partial adoption is awkward: you adopt their language to get anything |
| **Keep Warrant test-only** (today) | Leaves "proved for every input" and generated code unreachable |

### 3.4 Prior art: what we borrow and what differs

| System | What it does | We borrow | Differs here |
|---|---|---|---|
| **Fiat Cryptography** (Coq) | One specification derived by proof-producing rewriting into verified code, printed as C and, through further printers, Rust, Go, Java and others | One definition, many targets; target-independent form checked once; the last step is a printer | Fiat's final printers sit outside its proof. We cover that step by lifting the printed text back (8.7) |
| **Dafny** | Verified programs compiled by unverified backends to C#, Java, JavaScript, Go, Python and others; verification via Boogie and Z3 | Multi-target compilation from one verified source | Backends are unverified and the solver is trusted. Ours: untrusted solver (certificates), backends closed by the reverse direction |
| **F\* / Low\*** (to C via KaRaMeL) | A low-level subset extracted to C, correctness argued once | A small, low-level subset with a defined mapping | Once-for-all extractor correctness, 18 times over, is too much; we validate per function |
| **Coq and Lean extraction** | Erase proofs, print definitions as OCaml, Haskell, Scheme (Coq); C via an IR (Lean) | Executable definitions | Extraction is unverified; no reverse direction |
| **CakeML** | Verified compiler; a proof-producing translator from HOL functions to programs with a theorem; Myreen's decompilation into logic lifts machine code to HOL functions | **Proof-producing translation** and **decompilation into logic**: exactly our two arrows | One verified compiler in HOL4; we validate per function against a semantics in a different logic |
| **seL4 AutoCorres** | Lifts C to a monadic Isabelle form per function, each with a refinement theorem | The closest cousin of the lifter: per-function lifting with a theorem, not a trusted tool | One language; ours is one kernel and N lane semantics |
| **Aeneas, hax, Creusot** | Translate a Rust subset to a pure form in Lean, Rocq, F\* | Pure or borrow-checked subsets lift cheaply to pure functions | They trust the translator; we check its output against a model |
| **Why3** | WhyML programs, VC generation, many provers, extraction to OCaml and C | VC generator separate from solvers | Our final judge is one small kernel |
| **Lean code generation** | Definitions compiled to C via an IR; `@[implemented_by]` swaps in native code | Definitions are executable; a fast path may replace a reference | The replacement is unverified there; here a replacement is exactly what the lifter checks |

Nobody both generates and lifts with a small kernel as the only judge, in many
languages, **with every rung usable on its own**. The cost of that is section 8.4.

---

## 4. The algorithm profile of Warrant (rung A2 and above)

A document opts in with a header word (4.9). Inside the profile a **definition** is a
`model` or `rec model` function. Anything outside the profile in a profile document
is **rejected, not ignored** (VER04 section 1). A document that does not opt in is
untouched, and still gets A0 and A1.

### 4.1 Totality is required

A kernel definition must terminate: the kernel reduces it (VER11a 6.2) and a function
that can diverge cannot be a theorem's subject. Four admitted shapes:

| Shape | Surface | Why total | Elaborates to (5.5) |
|---|---|---|---|
| **Non-recursive** | `model f(...) = e`, calling only earlier models | acyclic call graph | a plain definition |
| **Bounded loop** | `fold(range(n), init, step)` and the other WX-D forms over finite sequences (VER04k 2.7) | `range(n)` has `n` elements | `Nat.rec` or `List.rec` through a prelude helper |
| **Structural recursion** | `rec model f(...) -> T decreasing p = e`, every recursive call on a derived term of `p` (VER04s 4.2, RR1 to RR4) | the argument is a proper subterm | the kernel recursor of `p`'s type |
| **Well-founded recursion** | `rec model f(...) -> T measure m = e`, `m` a `Nat` expression over the parameters; each recursive call carries the **obligation** `m(args) < m(params)` under its path condition | the measure strictly decreases in `Nat` | an explicit **fuel** definition; the obligation is a kernel goal |

Further rules:

- **Structural recursion on `Nat`** is allowed, with arms `0` and `n + 1` (new pattern
  form; VER04s does not peel a `Nat` today).
- **Mutual recursion** is a group (VER04s 4.1, at most 16 members). The first
  elaborator slice refuses groups (5.5); the checker admits them so that evidence
  generation is not narrowed.
- **A definition whose totality is not discharged is not a model.** Warrant-side the
  checker's rules (RR1 to RR4, a `measure` present) are the admission test;
  kernel-side a `measure` function's obligations must be proved before its equation
  lemma exists and before the compiler may emit code that relies on termination
  (7.3). Fail closed: it stays at A0/A1.
- The ground evaluator keeps its work budget (VER04 section 2). Budget exhaustion is a
  diagnostic, never a value; the differential check (5.8) skips such cases and counts
  them.

### 4.2 Types

| Warrant type | Kernel image (5.3) | Tier |
|---|---|---|
| `Bool` | `Bool` | K0 |
| `Int`, `Nat` | `Int`, `Nat` (inductives) | K0 |
| records, tuples, variants, `error` enums, aliases | inductives; right-nested `Prod`; the aliased type | K0 |
| `Option<T>`, `Result<T,E>`, `List<T>` | ADL prelude `Option`, `Result`; `List` | K0 |
| `U8`..`U64`, `I8`..`I64` | a fixed-width word type from the ADL prelude | **K1** |
| `Bytes`, `Bytes<N>` | `List` of bytes; an indexed vector `Adl.Vec α n` | **K1** |

**K0** needs only the kernel features VER32 lists for G-M1: inductives, recursors,
iota, beta, delta, zeta, proof irrelevance, function eta; no literals, quotients or
projections. **K1** adds literals and `Nat` acceleration (VER32 3.5 `prim` roles,
which enter the footprint) and imports Lean's `Init` through the Lean converter
(PIF-008/009), because a 64-bit constant cannot be a unary `Nat`. A definition using a
K1 type is accepted by the checker and by every test lane (A0/A1 unaffected); the
**elaborator refuses it with a diagnostic naming the tier**. K0 admits `Nat` and `Int`
literals up to 255; larger ones are a K1 diagnostic.

`Int` in a definition compiled to a fixed-width lane needs a range fact (7.3); the
compiler refuses an `Int` expression it cannot bound unless the lane has unbounded
integers (Python, Ruby, Elixir, Haskell `Integer`; **not** Perl or Haskell `Int`).

### 4.3 Overflow: three declared choices

Warrant already settles the default (VER04j 2.3, VER04r 3.2): `+ - *` on a fixed-width
type **wrap**, in every profile and lane; `Int` and `Nat` arithmetic is exact; no
implicit conversion. The profile keeps that and names the three choices so a
definition says which it means and each tool has one mapping:

| Choice | Written | WIR overflow mode (VER25 2.1) | Lane code |
|---|---|---|---|
| **wrap** | `a + b` on `Un`/`Sn`; `wrap<T>(i)` | `Wrap` | the lane's wrapping operation |
| **checked** | compute in `Int`, then `try<T>(i)`: `Option<T>`; the error is a value | `Wrap` arithmetic plus an explicit range test | an explicit range test; no reliance on host overflow behaviour |
| **trap** | *checked*, plus a binding-level **error normalisation** (`panic_to_error`, VER04d 5): the op panics exactly when the model returns the error | `Trap` when the compiler shows it equals the test | the lane's panicking or throwing form |

**Trap is not a third arithmetic.** It is checked arithmetic whose error case the
binding declares to be a panic, as LG01 already treats rejection. The default is the
existing wrap; whether the profile should *require* explicit spelling is Q2.

### 4.4 Errors and rejections are values

A model never throws. Where code rejects an input, overflows or fails, the model
returns `Err`/`None` **as a value**.

- `error GateError = INVALID_INPUT` and `Result<Int, GateError>` are existing forms.
- An **input contract** (`input f.x: rejects P with E`) is part of the definition. In
  the profile `P` must be exactly the condition under which the model returns `Err(E)`;
  the elaborator generates the goal "`P(x)` iff `f(x) == Err(E)`" (exact rejection,
  VER11a 0.1) rather than taking the contract on trust. At A0/A1 the same contract
  drives hostile-input tests and solver-chosen boundary inputs.
- The model **does not distinguish error identities the code does not**. LG01's single
  case means "this call panicked", so `GateError` has one constructor. A panic-only
  binding maps to `Err` only for one-constructor error types; otherwise the compiler
  refuses.

### 4.5 No floats at first; effects and state out

The kernel has no float model and WIR has no floating point (VER32 9.3 option C). The
profile refuses `F32`/`F64` (VER04p) in an elaborated definition. A later float
profile needs at minimum: a reviewed kernel IEEE 754 model (binary32/64, rounding,
NaN and signed zero; VER04p's `same` is the equality it must support); WIR float
operations with the same semantics and a per-lane account of which platform
operations are correctly rounded (bucket (a)); proofs about rounding (interval or
bit-level lemmas, or a verified-library import); and libm-style functions modelled
as relations, not functions. A0/A1 for float code already follow VER04p.

The profile is **pure**: no I/O, mutation, reflection or host calls (VER04 section
4). Stateful objects (VER04m) will elaborate to state-passing functions over an
explicit state record, and objects with identity to the WIR memory model (VER11a
6.3); opaque types (VER04n) to abstract types with interface axioms discharged by a
witness model. That later profile is not specified here (ADL-020).

### 4.6 What a definition is not

Not an implementation: it says nothing about loops versus recursion in a lane, layout,
allocation, in-place update or constant-time behaviour. Constant time and resource
bounds stay claims about **code** (VER11a section 8, `bound`, `labels`), checked at
WIR level on whatever program the compiler or a lifter produces.

### 4.7 Partial profiles

A document may use the profile for some functions only. A function outside the
profile (for example one using `F64`, or a K1 type before K1) keeps its A0/A1
evidence; the report states per function which rung it reaches and why.

### 4.8 Realization: how an `op` says it implements a model function

Today an `op` (a signature bound to a host export, VER04d 5) and a `model` are
unrelated. The profile adds one fact, as **data in the binding schema**, where a
Warrant document already meets code:

```json
"nand_gate": { "export": "nand_gate", "fresh": false,
  "realizes": { "module": "logic_gates_model", "model": "nand_gate",
                "embed": ["U8->Int", "U8->Int"],
                "result": "Result<U8,GateError>->Result<Int,GateError>" },
  "negative_controls": ["always_true"] }
```

`realizes` names a model function and the **canonical embeddings** of the op's
parameter and result types into the model's (`Un` to `Int` by value, `Nat` to `Int`,
`Bytes<N>` to `Bytes`, `Result`/`Option` componentwise), nothing else: no
user-supplied conversion function, which would be unreviewed code in the statement.
It yields a **link goal** (7.3, 8.2). Non-canonical embeddings are a checker error.
Realization is optional; without it the op has A0/A1 evidence only.

### 4.9 How the profile enters the language

New syntax and the header word go through `code/grammars/warrant/warrant.tokens` and
`warrant.grammar`, the shared grammar-driven parser, parse-tree pinning and drift
tests (VER04 section 11, VER04c). **No hand-written parser anywhere**, including the
elaborator, compiler and every lifter.

| Addition | Form |
|---|---|
| Profile header | `warrant NAME v1 profile algorithm` (a profile word after the version; existing documents do not change meaning; Q8) |
| Measured recursion | `rec model f(params) -> T measure expr = body` (`measure` reserved inside the profile) |
| `Nat` peeling | patterns `0` and `n + 1` in `match` on a `Nat` decreasing parameter |
| Realization | data only, in the binding schema (4.8) |

Diagnostic codes for the new checks (totality, measure type, realization shape, tier
refusal) are allocated by ADL-001 in the existing registry.

---

## 5. Elaboration to the kernel: meaning (rung A2)

### 5.1 Shape

The elaborator is a program from a *checked Warrant AST* to two artefacts:

| Artefact | Contents | Trust |
|---|---|---|
| **Model PIF file**, one per module | one kernel `def` per `model`/`rec model` (and per type declaration an `inductive` block), in dependency order; **definitions only, no theorems** | reviewed by a person, signed by digest (`<pkg>.model.faithful`, (b)) |
| **Goals list** | per `claim` (and per generated goal such as exact rejection): claim id, the goal term (CTE, VER32 3.6) and its readable print | each line signed with the environment digest (VER32 G6) |

The proof producer writes a *separate claim file* containing only theorems and goal
bindings; it may declare no constant that occurs in a goal and no axiom (VER32 3.8,
A2). The environment chain is `PIF prelude, ADL prelude, model file(s), claim file`
(VER32 E1/E2: linear, fixed by the gate, not by the claim file).

The model file is **shallow**: a Warrant `model f` is a kernel constant `f` of the
translated type, not data describing `f`. So `f 3` reduces in the kernel by
beta/delta/iota/zeta and the kernel is also the model evaluator.

**Names.** A kernel name is `<module>.<name>` (type declarations: `<module>.<Type>`,
constructors `<module>.<Type>.<Ctor>`); every component is ASCII
`[A-Za-z_][A-Za-z0-9_']*` (VER32 3.8). A claim theorem is `<module>.claim.<name>`; its
goal id is the string `<module>/<claim name>`. Warrant already forbids shadowing
(W0203), so de Bruijn conversion is deterministic.

### 5.2 The ADL prelude

A second reviewed file, after the PIF prelude, with helper definitions the elaborator
refers to by name instead of inlining: `Adl.ite`, `Result`, `Option` helpers,
`Adl.beq` for each base type and generic `beq` combinators, `Int` operations defined
structurally, `Adl.foldRange`, `Adl.List` helpers (`get?`, `append`, `length`), `fdiv`,
and at K1 the fixed-width and `Vec` types. It is **digest-allow-listed and signed**
(`adl.prelude.faithful`, (b)). Each helper is covered by the differential check
(5.8) against the evaluator's built-in of the same name, so a prelude definition that
disagrees with Warrant's meaning is found by test, not only by review.

### 5.3 Types

| Warrant | Kernel |
|---|---|
| `Bool`, `Nat`, `Int` | `Bool`, `Nat`, `Int` |
| `Option<T>`, `Result<T,E>`, `List<T>` | `Option ⟦T⟧`, `Result ⟦T⟧ ⟦E⟧`, `List ⟦T⟧` |
| `(T1, ..., Tn)` | `Prod ⟦T1⟧ (Prod ⟦T2⟧ ... ⟦Tn⟧)`; the last component is not wrapped |
| `error E = A \| B`, enum, variant | `inductive <M>.E` with constructors in declaration order |
| record | one-constructor inductive; field access is `proj` |
| alias `model T = U` | `⟦U⟧` (transparent) |
| `Nat <: Int` widening | `Int.ofNat` inserted at the widening site |
| `Bytes<N> <: Bytes` widening (K1) | `Adl.Vec.toList` |

### 5.4 Expressions

| Warrant | Kernel (⟦·⟧) |
|---|---|
| variable, parameter | de Bruijn variable |
| `0`, `1`, small literals (K0) | `Int.ofNat`/`Nat.succ` chains over `Nat.zero` |
| `if c then a else b` | `Adl.ite ⟦c⟧ ⟦a⟧ ⟦b⟧`, with `Adl.ite c t e := Bool.rec e t c` (`Bool.rec` takes the `false` case first) |
| `not a`, `a and b`, `a or b`, `a => b` | `Bool` functions defined by `Bool.rec`, hence lazy; `a => b` is `not a or b` |
| `match e with ...` | the type's recursor with a non-dependent motive; **arms are reordered into constructor declaration order**; arm binders become lambda binders |
| call `g(a, b)` | `g ⟦a⟧ ⟦b⟧` (arguments are evaluated by reduction, so order is immaterial in a pure total language) |
| `a == b` on a base type, ADT, tuple | `Adl.beq_T ⟦a⟧ ⟦b⟧`; the elaborator **derives** `beq_T` for every declared type in the model file, from the constructors |
| `a < b`, `<=` on `Int`/`Nat` | `Adl.Int.lt`, `Adl.Nat.ble` (prelude, `Bool`-valued) |
| `len`, `at`, `concat`, `div`, `fixed<N>` | the prelude function of the same name |
| `fold(range(n), init, f)` | `Adl.foldRange ⟦n⟧ ⟦init⟧ ⟦f⟧` where `foldRange 0 a f = a` and `foldRange (k+1) a f = f (foldRange k a f) k` (the order of VER04k 2.7), defined with `Nat.rec` |
| `forall x: T \| p. q` (claims only) | `∀ x : ⟦T⟧, Eq Bool ⟦p⟧ true → Eq Bool ⟦q⟧ true` |
| `exists x: T \| p. q` (claims only) | `Exists (fun x : ⟦T⟧ => And (Eq Bool ⟦p⟧ true) (Eq Bool ⟦q⟧ true))` |

**Claims.** A claim is a prefix of quantifiers over a `Bool` body. Its goal is
`∀ binders, hyps → Eq Bool ⟦body⟧ true`; when the body is a top-level `==`
(the usual claim shape), it is stated as `Eq T ⟦a⟧ ⟦b⟧`, which is the same fact by
the generated `beq_spec` lemma. The two forms are the *same statement* up to a lemma
the elaborator also emits and the producer proves; the goal list prints which form
was used. Quantifiers nested inside a Bool expression are refused (the evaluator
cannot interpret them either).

### 5.5 Definitions and recursion

- **Non-recursive**: `def <M>.f : Π (x : T) .., R := fun x .. => ⟦body⟧`, regular
  delta hint.
- **Bounded loop**: through `Adl.foldRange` and the other prelude folds; no new
  kernel feature.
- **Structural recursion on `p : T`**, restricted in the first slice to **primitive
  recursion on direct constructor fields**: `f p args = T.rec (motive := fun _ => Π
  args', R) (case_C1 ...) ... p args`. The recursive hypothesis for a child field of
  type `T` is a function from the other parameters to `R`, so other parameters may
  change in a call (carries, accumulators). A call on a deeper derived term, through
  a `fold` over children, or in a mutual group is refused with a diagnostic (the
  fuel route handles it, or a later `below`-style construction).
- **Well-founded recursion by measure `m`**: *no `WellFounded.fix`* (it does not
  reduce, and VER11a 6.2 deliberately avoids well-founded recursion in the
  semantics). Instead:
  `f_fuel : Nat → Π params, R` by `Nat.rec` on the fuel, with base case a **canonical
  default** of `R` (Bool: false; Nat/Int: 0; Option: none; List: nil; Result: the first
  error constructor with defaults; records componentwise; otherwise refuse), and
  `f x := f_fuel (m x + 1) x`. The **obligations** are (i) per call site,
  `path condition → m(args) < m(params)` and (ii) the equation lemma
  `f x = body[f]`, proved from a generic lemma "fuel above the measure changes
  nothing" (an induction, PIF-018). Until (i) is proved the exhaustion branch could
  be reached, so `f` is not admitted for compilation and its claims stay at A0/A1.
  Concrete evaluation still reduces (`m x` is computed first), so the differential
  check works.

### 5.6 What the elaborator cannot do

It does not choose tactics, does not prove anything, and does not rewrite a model for
convenience. Refusals are diagnostics naming the construct and the tier (K1, mutual
groups, deeper recursion, floats, state). **No silent partial output**: a function that
cannot be elaborated is absent from the model file and listed.

### 5.7 Two implementers must agree

The model file is deterministic: definitions in the order of Warrant dependency
(topological, ties by source order), names as 5.1, arms in constructor order,
prelude helpers by name, no inlining, no optimisation. Two correct elaborators
produce the **same canonical bytes** (VER32 4.5 caveat: canonical text, not an
interning-order claim; the digest is over the file's text). That is what makes the
second-elaborator route (5.8) meaningful.

### 5.8 Trusting the elaborator for meaning

The elaborator is untrusted for validity (a wrong term is rejected or simply is the
wrong statement) but *decides what is proved*. Three mitigations, each partial:

1. **Signature.** A person reads the printed model file and goals list and signs the
   file digest and the goals aggregate (VER32 G6, PIF-020). This converts the question
   into a (b) statement, `<pkg>.model.faithful`: "this text is the algorithm we meant".
2. **Differential check (evaluator versus kernel).** For every generated test vector,
   example, bounded-exhaustive case and a seeded random sample: evaluate the ground
   call with Warrant's evaluator; elaborate the same call to a kernel term (the
   elaborator's own term builder, applied to value-encoded arguments); reduce it with
   the kernel to constructor normal form; decode; the two values must be equal. A
   **construct coverage gate** requires every construct of 5.4 and every prelude helper
   to be hit; an **elaborator mutation corpus** (swap `ite` branches, drop a match arm,
   reorder constructors, flip a comparison) must be caught. This is evidence, not a
   proof: finite vectors cannot show faithfulness for all inputs.
   It also compares the *evaluator and the kernel*, so a bug in either is found.
3. **Assumption id `warrant.model.elaboration-faithful`**, bucket **(c)**: "the
   kernel definitions mean what the Warrant evaluator means". It records the one place
   where the tests of A0 and the proofs of A2 are tied to the same definition. Two routes
   close it: **(a)** a **second, independent elaborator** written without reading the first
   (as KRN-019 for the kernel); agreement means equal model-file digests on the whole
   corpus; **(b)** the signature of (1), which converts it to the (b) id above for a
   package that accepts a signed model. Q6 asks which suffices for `proved`.

Honest limit: with only (1) and (2), a subtly wrong elaborator that is consistent with
the signed text but not with the evaluator would be caught only if a vector reaches
it. That is why (3) stays open until route (a).

### 5.9 Worked example: the logic gates

The existing `logic-gates-basic.warrant` declares gates as `op`s and has no gate
definition. The model file, **new, in the algorithm profile**, defines them (the
domain is `Int`, per VER32 Q4; `U8` is a corollary by restriction). Layout is
simplified: long `match` expressions continue inside parentheses (VER04 section 5).

```text
warrant logic_gates_model v1 profile algorithm
  error GateError = INVALID_INPUT

  model to_bit(x: Int) -> Option<Bool> =
    if x == 0 then Some(false) else (if x == 1 then Some(true) else None)
  model from_bit(b: Bool) -> Int = if b then 1 else 0

  model nand_gate(a: Int, b: Int) -> Result<Int, GateError> =
    match to_bit(a) with
      None -> Err(GateError.INVALID_INPUT)
    | Some(x) -> (match to_bit(b) with
                    None -> Err(GateError.INVALID_INPUT)
                  | Some(y) -> Ok(from_bit(not (x and y))))
  model and_gate(a: Int, b: Int) -> Result<Int, GateError> =
    match to_bit(a) with
      None -> Err(GateError.INVALID_INPUT)
    | Some(x) -> (match to_bit(b) with
                    None -> Err(GateError.INVALID_INPUT)
                  | Some(y) -> Ok(from_bit(x and y)))
  model nand_not(a: Int) -> Result<Int, GateError> = nand_gate(a, a)
  model nand_and(a: Int, b: Int) -> Result<Int, GateError> =
    match nand_gate(a, b) with Err(e) -> Err(e) | Ok(n) -> nand_not(n)

  claim nand_and_equals_and:
    forall a: Int, b: Int. nand_and(a, b) == and_gate(a, b)
```

(The other nine gates follow the same shape; `nand_or` and `nand_xor` are the Rust
source's four-NAND constructions as `model`s. Their *oracles* remain Warrant's own
operators in the LG02 files; the model above is the **definition**, the LG02 truth
table rows are the independent check, as VER32 T2 requires.)

**Elaborated model file**, printed form (the on-disk PIF is VER32 section 4; names
shortened, universe levels omitted):

```text
inductive logic_gates_model.GateError : Type
  | INVALID_INPUT : logic_gates_model.GateError

def logic_gates_model.to_bit : Int → Option Bool :=
  fun (x : Int) =>
    Adl.ite (Adl.Int.beq x (Int.ofNat Nat.zero))
      (Option.some Bool Bool.false)
      (Adl.ite (Adl.Int.beq x (Int.ofNat (Nat.succ Nat.zero)))
         (Option.some Bool Bool.true) (Option.none Bool))

def logic_gates_model.from_bit : Bool → Int :=
  fun (b : Bool) => Adl.ite b (Int.ofNat (Nat.succ Nat.zero)) (Int.ofNat Nat.zero)

def logic_gates_model.nand_gate : Int → Int → Result Int GateError :=
  fun (a b : Int) =>
    Option.rec (motive := fun _ => Result Int GateError)
      (Result.err INVALID_INPUT)                                   -- none first
      (fun (x : Bool) =>
         Option.rec (motive := fun _ => Result Int GateError)
           (Result.err INVALID_INPUT)
           (fun (y : Bool) => Result.ok (from_bit (Bool.not (Bool.and x y))))
           (to_bit b))
      (to_bit a)

def logic_gates_model.nand_not : Int → Result Int GateError :=
  fun (a : Int) => nand_gate a a
def logic_gates_model.nand_and : Int → Int → Result Int GateError :=
  fun (a b : Int) =>
    Result.rec (motive := fun _ => Result Int GateError)
      (fun (n : Int) => nand_not n)            -- ok case (declaration order: ok, err)
      (fun (e : GateError) => Result.err e)
      (nand_gate a b)
```

(The recursor takes minor premises in constructor declaration order, so for
`Option` (`none`, `some`) and `Result` (`ok`, `err`) the arms are reordered from the
source's. `Bool.rec` takes `false` first.) The ADL prelude's `Adl.ite` is
`fun c t e => Bool.rec e t c`.

**The goal** (one line of the goals list):

```text
goal "logic_gates_model/nand_and_equals_and"
  ∀ (a : Int) (b : Int),
    Eq (Result Int GateError) (logic_gates_model.nand_and a b) (logic_gates_model.and_gate a b)
  sha256:<CTE digest>       -- recomputed by the gate; the printed text is the readable form
```

**The proof** is written by the case-split producer (VER32 PIF-006) in the claim file:
generalise `to_bit a` and `to_bit b` (`Option.rec`, four cases), then `Bool.rec` on the
two bits, each leaf closed by `Eq.refl` after beta/iota/delta. The `none` cases reduce
both sides to `Result.err INVALID_INPUT`; the four `some` cases are the NAND
identity at that bit pair. A producer cannot weaken the claim: the type of the
theorem must equal the signed goal.

**The differential check** on this file: the evaluator computes `nand_and(1, 1) ==
Ok(1)`; the kernel reduces `nand_and (Int.ofNat 1) (Int.ofNat 1)` to
`Result.ok (Int.ofNat 1)`; decode; equal. The seeded sample includes `-1`, `2`, `0`
and `1` in each argument position, hitting both `none` branches.

**What the signer sees**: the printed model file above (about 11 definitions' worth)
and one goals list. Nothing in either was written by the proof producer.

---

## 6. Proofs and the solver (rungs A1, A2)

### 6.1 Automation first (A2)

For each goal the producers are tried in order, all untrusted, all emitting PIF:

| Producer | Handles | Source |
|---|---|---|
| **Case split** | finite inductives, `Option`/`Result`/`Bool` trees, bounded-depth splits on `Nat`/`Int` with a symbolic remainder, `False.rec` on contradictory hypotheses, `Eq.refl` leaves | VER32 PIF-006 |
| **Induction and rewriting term builder** | `Nat.rec`/`List.rec` induction with a motive, `Eq.rec`/`congrArg` rewriting: "for every width" theorems and the equation lemmas of 5.5 | VER32 PIF-018 |
| **Linear arithmetic certificates** | bounds such as `m(args) < m(params)` over `Nat` | VER11a KRN-012a |
| **SAT/SMT with certificates** | finite bit-level facts, bounded arithmetic, via the solver (6.3) and LRAT/proof reconstruction | KRN-009, VER34 |

### 6.2 Escape hatch, and failing closed

A goal no producer closes can be proved **in Lean** through VER32's Lean profile
(Lean source elaborated by the pinned toolchain, converted to PIF), or later by
native proof scripts. The kernel treats it identically. **Fail closed**: a claim
without a checked proof keeps its lower rung, `tested`, `bounded(N)` or
`exhaustive`. A refuted claim (a counterexample) goes through VER27's finding
protocol, and the counterexample becomes a regression example (6.3).

### 6.3 The solver: how Warrant uses it (A1, and VC discharge for A2/A3)

The repository's Constraint VM ([LANG24](LANG24-constraint-vm.md): `constraint-core`,
`constraint-instructions`, `constraint-engine`, `constraint-vm`, with `cdcl-sat` and
`smt-lib-format`) is being grown into a full SMT solver for Warrant to use. **VER34
designs the solver**: theories (bit-vectors, linear integer arithmetic, arrays,
uninterpreted functions, datatypes), the front end, performance, and the certificate
formats. This document does not design it. It states what Warrant asks of it and
the rule that bounds its authority.

**Three uses.**

| Use | Rung | What Warrant asks | What comes back, and what is done with it |
|---|---|---|---|
| **Input generation** | A1 | For each input contract and precondition: a model of `P(x)`, a model of `not P(x)`, and models **on the boundary** (`x` minimal or maximal in `P`, `x = c` and `c ± 1` for each constant `c`, lengths at 0, 1, the limit, limit+1). For claim bodies: inputs reaching each branch of the model | Portable test vectors (TFV, corpus) added to the generated suites of **every lane**. Boundary cases beat random generation for `rejects x > 10` style contracts: the interesting input is exactly 10 and 11, and a solver finds it by construction. Sampling stays labelled `sample`, solver-boundary cases `boundary`; neither is called exhaustive |
| **Bounded model-versus-code check** | A1 | "Find `x` with `|x| <= k` such that `code(x) != model(x)`", with the code given as its WIR program (symbolic execution: `warrant-ir` has `sym.rs` and `ct_proof.rs`) or, for lanes without WIR, as the oracle of the corpus | UNSAT up to `k` with a checked certificate (DRAT/LRAT) is a **`bounded(k)`** claim (VER31 rung 2). A **counterexample** is **confirmed by evaluation**: run the model and the code on `x`; if they disagree it is real, and it is committed as a **permanent regression example** (a generated `example` with the seed and solver version as provenance) so the bug stays fixed. If they agree, the solver or the encoding is wrong: a finding about the solver, never a result |
| **Verification-condition discharge** | A2, A3 | The goals of sections 5 to 8 that are bit-level or arithmetic: obligation lemmas for a lifted program, fuel obligations (5.5), range facts (7.3) | UNSAT **plus a certificate reconstructed into a kernel proof term** (LRAT to proof terms, KRN-009; theory lemmas per VER34). Only the reconstructed PIF counts. SAT is a counterexample, confirmed by evaluation. `unknown` and timeouts say nothing |

**The rule (D7): the solver is untrusted.**

- An UNSAT answer by itself **never** moves a claim's rung. It counts towards
  `bounded` or `proved` **only** with a certificate the kernel (or, for DRAT at
  `bounded`, the existing checked-certificate path) validates (VER11a 9.5, VER32).
- A SAT answer is a *candidate* counterexample, **confirmed by evaluation** with
  Warrant's evaluator and the code under test before anything is reported. A model that
  does not reproduce is discarded and counted as a solver or encoding defect.
- The solver's *encoding* of a Warrant expression (how `U8 + U8` or `at(xs, i)` becomes
  an SMT term) is itself checked: the differential harness of 5.8 replays encoded
  terms against the evaluator on the generated vectors. A wrong encoding then shows as
  a spurious counterexample or a mismatch, never as an accepted proof, because a
  proof needs the kernel's checking of the reconstructed term.
- Solver output does not change what is claimed. It changes which inputs are tried and
  how a proof is found.

**What is needed from VER34**, stated as requirements, not design: models with
minimisation and maximisation (boundary points); incremental `push`/`pop`; a bounded
quantifier-free fragment covering Warrant's Bool, Int, fixed-width, list and ADT
operations; proof-producing mode (LRAT for the SAT core, proof terms or checkable
lemmas per theory); deterministic output for a fixed seed so generated vectors
reproduce byte for byte (VER04e).

---

## 7. Forward: compiling definitions to code (rung A4)

### 7.1 What goes in and what comes out

```
 checked Warrant AST ──► (a) WIR compiler ──► WIR program + certificate ─► kernel: Refines (7.3)
          │
          └────────────► (b) lane backends ──► idiomatic lane source + tests
```

Both consumers read the **same checked AST** (and the same structured, typed model IR
that VER04r already gives every lane). The decision here that the brief did not settle:
the lane backends read that **structured IR**, not the WIR. WIR is a control-flow graph
of basic blocks; recovering readable loops and conditionals from it ("relooping") would
make backends hard to write and their output unidiomatic. The WIR program is the **proof
pivot**, not the source of generated text. The link between a backend's text and the
model is the *reverse path* (8.7), which does not care how the text was made (Q9).

### 7.2 Model to WIR: an untrusted compiler

The compiler lowers each admitted definition to a WIR function:

| Model construct | WIR |
|---|---|
| scalar parameters and results (`Bool`, `Un`, `Sn`) | WIR scalars; `Int` only with a range fact (4.2) |
| `if`, `match` on scalar-encoded sums | blocks and conditional branches |
| `Option`/`Result` of scalars, error enums | the **ADL-WIR value encoding** (a reviewed file, `adl.wir-abi.faithful`): tag and payload packed or returned as defined there; **a panic-normalised error is a `Trap` with a reason identity** |
| bounded loop (`fold` over `range`), structural recursion on a list | a counted loop with explicit state, by the templates of 7.5 |
| fixed-width `+ - *` | `Wrap` mode; checked and trap per 4.3 |
| anything else (ADTs with heap, trees, mutual groups, deeper recursion) | **refused**: the function stays at A0/A1/A2 and is listed |

### 7.3 Per-function translation validation

For each model function `f` with realization (4.8) the compiler emits a WIR function
`F` and a **certificate**. The kernel checks one theorem, the **refinement**:

```
Refines P F f :=
  ∀ args ∈ dom(f_embedded),
    ∃ n, let s = run n P (call F (enc args)) in
         (f args = ok v  → s.status = ret (enc v))
       ∧ (f args = err e → s.status = trap (reason e))
       ∧ s.status ≠ stuck ∧ no other trap                  -- no undefined behaviour
```

`run`, `status`, `trap` are VER11a section 6 (VER25 section 8 is normative; for the
loop-free gates, VER32's native straight-line semantics PIF-019). `enc` and `reason`
are the reviewed ADL-WIR encoding. The theorem *names the model constant `f`* from the
model file and the WIR program *by value* (VER11a 7.3), so it cannot be about another
function. It is a signed goal like any other.

**The certificate** is what makes checking mechanical rather than a proof search:

- the **simulation relation** (which WIR variables hold which model quantities at each
  program point);
- the **loop invariants** at loop headers and the **variant** (fuel);
- per-block **obligation terms**, mostly `Eq.refl` after symbolic execution by the
  kernel's own reduction (VER11a section 0: the kernel does the symbolic execution itself).

The compiler derives all three from the *shape* of the definition (7.5), so none is
guessed. If a certificate fails to check, the program is **not linked**; the function
keeps A0/A1/A2.

### 7.4 Why validation per function and not a compiler proof

Proving the compiler correct once is a project of its own, and a second one per lane
backend. A refinement theorem per function costs a kernel run per function and needs
no trust in the compiler (VER11a 14.3 makes the same choice for the Rust front end). A
compiler bug costs a failed validation, a message and a fix; it cannot cost a wrong
result.

### 7.5 Loop templates: deriving the invariant from the definition

First slice templates, each a fixed pattern; anything unmatched is refused:

| Template | Model shape | Loop | Invariant |
|---|---|---|---|
| **T1 accumulate** | `fold(range(n), init, step)` | `for i in 0..n { acc = step(acc, i) }` | after `i` iterations `acc = foldRange i init step`; the step is `foldRange (i+1) = step (foldRange i) i`, which holds by **delta/iota** since `foldRange` is `Nat.rec` |
| **T2 map and zip building an output** | structural recursion `f(x::xs, y::ys, c) = let (o, c') = g(x, y, c) in o :: f(xs, ys, c')` | one loop over the index, output buffer grows | after `i` steps `out[0..i)` equals the first `i` elements of `f xs ys c` and the carry equals the model's carry after `i` steps |
| **T3 search** | a recursion with an early result | loop with exit | invariant "no match among the first `i`", variant `len - i` |

A recursion that is not tail-shaped (**T4, trees**) needs an explicit stack and is out
of the first slice. The ripple-carry adder (10.5) is T2.

### 7.6 Lane backends

A backend (untrusted) reads the structured IR and the lane's **style profile** and
writes source text, a generated test file, packaging, and a provenance header. It is
written in the repo's usual way: one backend per lane; the repo's per-lane formatter, where one exists,
is run last (printing is not parsing, so no grammar-driven parser is needed here), and each backend's output is exercised by A0 immediately: the **same
Warrant file's generated tests run against the generated code** (examples, properties,
fuzz). A backend that produces code failing its own definition's tests is caught on
the first run.

### 7.7 Idiomatic output, per lane

The owner wants generated code that reads as if a person of that language wrote it.
Idiomatic means the following, per lane. Each row is a **style profile** consumed by
the backend, reviewed with the first backend of that lane.

| Concern | What the profile fixes |
|---|---|
| **Naming** | `snake_case` functions in Rust, Python, Ruby, Elixir, Lua, Perl, OCaml, C, C++; `camelCase` in Java, Kotlin, TypeScript, Dart, Swift, Haskell, F#; exported `PascalCase` in Go and C# public members. The Warrant names stay in the provenance comment so the mapping is auditable |
| **Errors as values vs panics** | Decided by the binding's `error_normalization` (4.3, 4.4), not by taste. Value style: Rust `Result<T,E>`, Go `(T, error)`, Haskell `Either`/`Maybe`, OCaml and F# `result`/`option`, Elixir `{:ok, v}`/`{:error, e}`, Swift `throws` or `Result`, Kotlin `Result` or a sealed class, Java a sealed `Result` or an exception where the contract says panic, C an out-parameter with a status code, C++ `std::expected`-style or exception. Panic style (the gates' LG01 contract): the lane's native panic or exception, with the contract's message identity |
| **Optional values** | `Option`, nullable with an explicit check, `Maybe`, `option`, `nil` plus ok-flag, `None`: whichever the lane's own standard library uses |
| **Control flow** | a model `match` becomes `match`, `switch` or pattern matching where the lane has them and a conditional chain where it does not; a `fold` over `range` becomes the lane's loop, not a generic helper |
| **Packaging** | the repository's per-lane layout: package manifest, `BUILD`, `README.md`, `CHANGELOG.d/`, tests beside the source, coverage thresholds (CLAUDE.md) |
| **Literate comments** | the Warrant doc comments, the model text quoted, truth tables and worked examples, in the repository's literate style (CLAUDE.md rule 14); a "generated, do not edit, regenerate with" header with the source and model digests |

**How idiomatic coexists with per-function validation.** The two requirements meet at
one place: the **lifter's subset** (8.3). A backend may emit only constructs the lane's
lifter accepts and the lane's subset semantics defines. Idioms outside the subset
(iterator chains, closures and higher-order helpers, generics-heavy abstractions, macros,
exceptions used for control flow, reflection) are **not emitted** until the subset
grows to cover them. So:

1. the backend emits idiomatic text *within* the subset;
2. a formatter may reformat it; validation is on the **final text**, so formatting is
   not trusted;
3. the lifter maps the final text back to WIR and the kernel checks the refinement
   (8.7). A lane with no lifter yet gets the same generated text with A0/A1 evidence
   and the open id `<lane>.backend.code-link`.

The tension is real and stated: **widening idioms means widening a lane's reviewed
semantics**. The default is to start with the plainest idiom that is still what a
person would write (a loop and an `if`), and add idioms when an owner decision
buys the semantics (8.4).

### 7.8 Packaging, and whether generated code replaces hand-written code

Generated code is **additive** by default: a separate module or package
(for example `rust/logic-gates-adl`, never overwriting `rust/logic-gates`), carrying
its own tests and the differential against the hand-written lane **over the full input
domain**. It is, in order of use:

1. an **oracle**: a second independent implementation in each lane;
2. a **differential**: hand-written against generated, exhaustively where the domain is
   small;
3. a **lane that does not exist yet**: C and C++ have no gates package (the audit in
   the logic-gates backlog), so generated code can simply be that lane;
4. a **replacement** of a hand-written lane, only by owner decision (Q1).

A hand-written package is never regenerated against its will; A3 (lifting) exists so
that it need not be.

---

## 8. Reverse: lifting code to the kernel (rung A3; also closes A4)

### 8.1 The pipeline

```
lane source ──► lane front end (grammar-driven, code/grammars/<lang>) ──► typed lane AST
            ──► lifter (untrusted): lane AST → WIR + simulation hints + certificate
            ──► kernel checks (a) lane-semantics(source text) ≈ WIR and (b) Refines WIR f
```

The goal **names the source text itself** as a string whose digest is the file's, parsed
by a parser *defined in the kernel* for the subset (VER11a 14.3), so no lane parser or
lifter is in the trust base. Source outside the subset does not parse in the kernel's
definition, so no goal exists for it.

### 8.2 The equivalence statement

For a function `h` in the lane and a model `f` it realizes (4.8):

```
Equiv h f := ∀ args ∈ dom(f_embedded), lane_run(h, args) = outcome(f args)
```

where `outcome (ok v)` is the lane's normal return of the embedded value and
`outcome (err e)` is the lane's panic or error return of the binding. It is proved as
`lane_run ≈ WIR run` (translation validation, 8.3) and `Refines` (the same theorem as
7.3). The statement is the same for **hand-written and generated** code: it never asks
how the code was made.

### 8.3 Two lift targets

| Target | For | Idea | Needs |
|---|---|---|---|
| **To WIR (default)** | imperative lanes: Rust, C, C++, Go, Java, Kotlin, C#, Swift, Dart, and the dynamic lanes later | Lower to WIR (as the Rust front end already does), reuse the lockstep and obligation machinery of VER11a sections 8 and 9, validate the lowering per function | the lane's subset semantics (8.4), a simulation hint generator |
| **Direct, to a kernel definition** | pure functional lanes: Haskell, OCaml, F#, Elixir (function subset) | The lifter produces a kernel definition from the lane text, and a producer proves it equal to the model (rewriting, induction), as Aeneas and hax do for Rust | a subset semantics for the pure subset, but **no heap, loops or WIR** |

WIR stays the default because it keeps one machine under 18 lanes (VER11a 14.4). The
direct target is cheaper for functional lanes; it is a per-lane decision (Q3).

### 8.4 The per-lane cost, stated honestly

Each lane needs **a reviewed kernel semantics of its subset**: the integer types and
their overflow, the control constructs, calls, the panic or exception mechanism, the
standard-library functions the subset admits, and (for dynamic lanes) the type-confusion
behaviour (VER32 9.2: TypeScript `number` doubles reject `-0` and non-integers; Python
rejects `bool`, `int` subclasses and floats). That is VER25c's job for Rust: the
KRN-S0 spike and VER11a 14.3 count "about fifteen" constructs for ct-compare's subset,
and KRN-015/016 are the calibration. Everything below is an **estimate, not a
measurement**: a *gates-only* subset per static lane is perhaps a third of that (function,
integer parameters, comparison, `if`, call, `return`, panic), a *general algorithm*
subset (loops, arrays or lists, ADTs) several times more, and a dynamic lane adds a type
system's worth of "what does this operator do to a string". It is the dominant long-term
cost of the whole programme, and the reason A0 to A2 are first-class.

**Per-lane order** (recommendation; Q3):

| Order | Lane(s) | Reason |
|---|---|---|
| 1 | **Rust** | Front end exists (VER25a/b); VER25c and KRN-015/016 are already queued; gives the calibration |
| 2 | **Go** | Static, simple, no exceptions (panics only), small grammar; the gates are plain functions |
| 3 | **OCaml** (then F#, Haskell) | Pure functional, **direct** lift target, smallest semantics, tests the second target |
| 4 | Java, Kotlin, C# | Static OO with exceptions; one subset semantics shared in structure |
| 5 | Swift, Dart, C, C++ | C/C++ add undefined behaviour and memory (a real semantics project) |
| 6 | TypeScript, Python, Ruby, Lua, Perl, Elixir | Dynamic: the type-confusion matrix must be inside the semantics; the largest cost; **may stay at A0/A1 indefinitely** |

### 8.5 Fail closed

A lifter refuses anything outside the subset, and **refusal is not an error in the
release**: the package keeps A0/A1 and A2 (model theorems), and the report says
"not lifted: construct X outside subset Y of lane Z". A lifter never guesses, widens,
rewrites or "approximates" source. A function that lifts but whose certificate fails is
reported as unlinked, with the failing obligation.

### 8.6 Lanes without a lifter

They keep the whole ladder up to what they can have: A0 (generated tests, PBT, fuzz,
exhaustive), A1 (solver vectors, which need no lane tooling), and A2 (the model is
proved regardless of lane). The report states per lane which rung is reached. A generated
backend (A4) without a lifter ships with `<lane>.backend.code-link` open, honestly, at
`tested`.

### 8.7 Lifting generated code closes the backend

For a lane with both a backend and a lifter, the build runs
`lift(generate(model))` and checks `Equiv`. A pass **closes `<lane>.backend.code-link`**:
the backend is now irrelevant to the claim because the *text* is the object checked.
A failure is a backend bug (the text is not the definition) or a lifter limit (the text is
outside the subset); both are visible, and both fail closed.

---

## 9. Assumptions and VER31 rungs

### 9.1 Ids the pipeline opens

Buckets are VER31's. Ids marked *new* need registration (VER13, signed bucket file) by
ADL-006; the rest come from VER11a, VER32.

| Id | Bucket | Opened by | Closes by |
|---|---|---|---|
| `kernel.checker.correct` | (c) | any kernel-checked claim | independent agreement (VER11a Q6, KRN-019) |
| `kernel.logic.consistent` | (c) open until decided (proposed (a)) | any kernel-checked claim | owner decision (VER11a Q5) |
| `pif.reader.correct`, `tcb.pif.gate-binding`, `tcb.goal.printer`, `tcb.report.checker-correct` | (c) | A2 | VER32 routes |
| `pif.prelude.faithful` | (b) | A2 | signature on the prelude digest |
| **`adl.prelude.faithful`** *(new)* | (b) | A2 | signature on the ADL prelude digest; each helper checked against the evaluator (5.8) |
| **`warrant.model.elaboration-faithful`** *(new)* | **(c)** | A2 | second independent elaborator, or the signed model (5.8) |
| **`<pkg>.model.faithful`** *(new per package; `gates.model.faithful` in VER32)* | (b) | A2 | signature on the model file digest |
| `spec.goal.faithful` | (b) | A2 | signed goals list |
| **`adl.wir-abi.faithful`** *(new)* | (b) | A3, A4 (forward) | signature on the encoding file digest |
| `wir.semantics.lean-faithful`, `wir.lowering.faithful`, `wir.typing.checker-guarantees`, `wir.interp.differential-only` | (c) | WIR link | VER11a K-M2 and KRN-006/007 |
| **`src.<lane>.lean-semantics.faithful`** *(new per lane; `src.rust.lean-semantics.faithful` in VER11a 14.3)* | (c) | A3 for that lane | review plus differential tests against the real toolchain; bucket per VER11a Q3 |
| **`src.<lane>.subset-conformance`** | (c) | A3 | closed by the kernel-defined parser refusing out-of-subset source |
| **`<lane>.backend.code-link`** *(new)* | (c) | A4 for that lane | the reverse check of 8.7 on the generated text |
| `tcb.<lane>-toolchain.trusted` and the platform ids | **(a)** | A3, A4 | accepted baseline |
| `<pkg>.model.code-link` (VER32: `gates.model.code-link`) | (c) | A2 only | closed by A3, replaced by the `wir.*` and `src.*` ids |
| solver | **none** | A1, VC discharge | not an assumption: certificates or evaluation (6.3) |

The compiler, backends, lifters, producers and solver have **no ids of their own**: the
design makes them untrusted, and their failures surface as rejected terms, failed
validations or unreproduced counterexamples.

### 9.2 What rung a package claim reaches at each stage

| Stage | Package claim rung (VER31 section 4) | Model theorem |
|---|---|---|
| Today (A0) | `tested`; `exhaustive` when `domain: full` is emitted | none |
| A1 | `tested`; `bounded(k)` with a checked certificate | none |
| A2 | unchanged | **model-checked** (the VER32 Q8 category), relative to the model-to-code ids |
| A3 (some lanes) | `checked`: a certificate for every input but an open (c) or (b) id remains | model-checked, now with the link |
| A3 complete, signed | `proved`: no open (c); every (b) covered by a valid sign-off | as above |
| A4 generated lane | `tested` at once; `checked`/`proved` only through the reverse check | as above |

**VER31 amendments needed**, as short notes (section 13): a **model-theorem category**
(VER32 Q8, already proposed), and a **link-status field** so the report distinguishes a
model-only theorem from a code-linked one and a hand-written lane from a generated one.
VER32 finding 14 is why a model theorem cannot reuse `checked`: `checked` needs rung 1
(`tested`) as a floor, and a model theorem has no tests to be at rung 1 of.

---

## 10. The first adopter: logic gates, end to end

### 10.1 The rungs for the gates, in order

| Step | Rung | What |
|---|---|---|
| (0) | A0, today | The LG02 corpus drives exhaustive truth-table and rejection tests in every lane; the Rust lane has VER04 examples and claims; per-lane audit gaps remain (logic-gates backlog) |
| (a) | A2 prep | Gate models in the algorithm profile (10.2) |
| (b) | **A2, G-M1** | VER32's model-level theorems T1 to T5 over the models |
| (c) | **A4, G-M4** | Generate the 11 gates for every lane from the model |
| (d) | **A3, G-M2** | Lift the hand-written Rust gates and check against the model |
| (e) | A2/A3 | Adders (G-M3) as the first recursive algorithm, then ct-compare |

### 10.2 (a) The models

The gates exist as `op`s; their definition does not. **ADL-002** adds
`code/specs/logic-gates-model.warrant` (algorithm profile; 5.9), a separate file so
the LG02 corpus files and their byte-identity drift guard stay untouched. It defines
`to_bit`, `from_bit`, the seven primitive gates over `Int`, and the four NAND
constructions as `model`s (retiring VER32 9.2's hand-written structural section), plus
`ref_g` specification functions, and restates the 22 LG02 claims over `Int` as goals.
A guard compares the oracle rows with the LG02 corpus. The existing `op`s get
`realizes` data in their binding file (4.8), one entry per gate.

### 10.3 (b) G-M1 on the models

VER32 9.2 and 9.5 apply verbatim, with the model file now produced by the ADL elaborator
(PIF-017 = ADL-003) from the file of 10.2. T1 (domain), T2 (truth tables against the
LG02 rows), T3 (every NAND-built gate equals its primitive over **every** integer), T4
(rejection) and T5 (non-vacuity). The honest note of VER32 stands: T1 and T4 restate
`to_bit`; only T3, and T2 against LG02, carry content.

### 10.4 (c) Generate all eleven gates (G-M4)

Per lane, in the order of 8.4 (backends are cheaper than lifters, so the order of
backends is independent): the backend writes the 11 gates in the lane's idiom (7.7),
with LG01's panic-or-exception contract from the binding, the LG02 corpus tests, and
the package files. A0 runs at once. Where a lifter exists the reverse check closes
the backend id. The generated Rust is a pure bit-level function: it does not call the
CMOS model.

### 10.5 (d) Lift Rust back (G-M2), and the transistor path

The hand-written Rust primitive gates route through the floating-point CMOS model
(`CMOSNand::evaluate_digital(a, b).unwrap()`), a path the Rust subset does not cover
(VER32 9.3). So the model is **bit-level** and the CMOS path is a separate matter:

- **Model versus hand-written Rust** is G-M2, under VER32's Q2 options. Option A: the
  four NAND-built gates fully, with `nand_gate` a stated (c) call contract
  (`gates.nand.primitive-contract`); **7 primitives not covered**. Option D: an integer
  switch-level `evaluate_digital` path lowerable to WIR, covering all 11, with the
  voltage model kept as a differential. Neither changes this spec: both are `Equiv` goals
  against the same model.
- **Generated Rust versus hand-written Rust** is a differential, over all 65,536 input
  pairs and the rejection cases, run at A0 as soon as G-M4 exists.
- The generated gates being bit-level is **why they are not a replacement** by default
  (Q1): the library's identity includes its transistor routing (VER32 Q2 option B
  rejected for the same reason).

### 10.6 (e) Adders, then ct-compare

The **ripple-carry adder** is the first recursive algorithm. A sketch (not run through
the checker; names illustrative until ADL-001's errata pass):

```text
  rec model add_bits(xs: List<Bool>, ys: List<Bool>, c: Bool)
      -> Option<(List<Bool>, Bool)> decreasing xs =
    match (xs, ys) with
      (Nil, Nil)         -> Some((Nil, c))
    | (Cons(x, xr), Cons(y, yr)) ->
        (match add_bits(xr, yr, carry(x, y, c)) with
           None -> None
         | Some((rest, c2)) -> Some((Cons(sum(x, y, c), rest), c2)))
    | _                  -> None                      -- unequal lengths rejected
```

It elaborates by **primitive recursion on `xs`** (5.5): the recursive hypothesis is a
function of `ys` and `c`, which is exactly why the carry may change per call. G-M3a
proves, for **every width**, `value(sum) + 2^n * carry = value xs + value ys + c` by
induction (PIF-018). It compiles by template T2 (7.5) with the invariant "after `i`
steps the output prefix and the carry equal the model's". G-M3b (lift the hand-written
Rust loop) uses the VER11a block-invariant rule and needs allocation and memory
semantics. **ct-compare** follows (VER32 PIF-015): its Warrant file gets a model for
`eq`/`select`, and its existing constant-time claims remain code-level (VER11a section
8); the model adds functional claims (K-M1c) and a clean realization for lifting.

---

## 11. Backlog

The same rows are added to [VER-BACKLOG.md](VER-BACKLOG.md) under "Warrant algorithm
definition language (VER33)", after the VER32 section if it exists on the base, else
after the VER11a/LEAN00 section; the VER32 section lands in a parallel PR. Order is
dependency order; one bounded change per PR; spec before code. `PIF-*`, `KRN-*` and
`G-M*` are VER32 and VER11a ids.

| Priority / ID | Work (rung) | Prerequisite and acceptance |
|---|---|---|
| P1 / ADL-000 | This spec; adversarial review; owner answers to section 12 | review answered; Q1 to Q9 decided |
| P1 / ADL-001 | **Profile syntax and checks** (A2): `profile algorithm` header, `measure` form, `Nat` peeling, totality diagnostics, realization data in the binding schema; tokens and grammar changes with drift tests; positive and negative corpus | after ADL-000; recursion parts after WX-R1 (VER04s), loop parts after WX-D2 (VER04k) |
| P1 / ADL-002 | **Gate models in the profile** (A2): `code/specs/logic-gates-model.warrant`, the 11 gates over `Int`, `ref_g`, 22 claims over `Int`, oracle-versus-LG02 guard, `realizes` entries; retires VER32 9.2's hand-written NAND section | after ADL-000; uses only existing syntax (K0) |
| P1 / ADL-003 | **Elaborator, K0** (A2): model file, goals list, deterministic names, derived `beq`, per-construct unit tests; the general form of VER32 PIF-017; the printed model file for the signature | after PIF-004, ADL-002 |
| P1 / ADL-004 | **ADL prelude** (A2): helper definitions of 5.2 as a reviewed, allow-listed file; extends PIF-004 | after PIF-004, KRN-002 slice 3a |
| P1 / ADL-005 | **Evaluator-versus-kernel differential** (A2): kernel normaliser entry point, vector replay, construct coverage gate, elaborator mutation corpus | after ADL-003, KRN-002 slice 3a (full reduction) |
| P1 / ADL-006 | **VER13/VER31 registration** (all): ids of 9.1, link-status field, model-theorem category with PIF-020 | after ADL-000 |
| P1 / ADL-007 | **A1 vector generation** (A1): solver-driven boundary and branch inputs from contracts and models into TFV/corpus; regression-example loop | after VER34's front-end slice; usable before A2 |
| P1 / ADL-008 | **A1 bounded model-versus-code** (A1): WIR symbolic execution with the solver, `bounded(k)` with a checked certificate, counterexample confirmed by evaluation | after ADL-007, VER34 certificates |
| P2 / ADL-009 | **Second independent elaborator** (A2): written without reading the first; agreement on model-file digests | after ADL-003 |
| P1 / ADL-010 | **ADL-WIR encoding and `Refines`** (A4): the reviewed `adl.wir-abi` file, the refinement goal template, the certificate format | after PIF-019, KRN-006a/b/f |
| P1 / ADL-011 | **WIR compiler, K0 straight-line** (A4): scalar and `Option`/`Result` models; certificate emitter; gates first; every refusal listed | after ADL-003, ADL-010 |
| P1 / G-M1 | VER32's model-level gate proofs, with the ADL model file | after ADL-003, ADL-004, VER32 PIF-006/-020 |
| P1 / ADL-012 | **Backend framework and Rust backend** (A4): style profile format, provenance header, packaging, the Rust style profile, `rust/logic-gates-adl` as an oracle package with the full-domain differential | after ADL-011 |
| P1 / G-M4 | **Generate the 11 gates** in the order of 8.4, one lane per PR, each with its generated tests | after ADL-012; per lane |
| P1 / ADL-013 | **Lifter framework spec** (A3), written as VER33a: lifter interface, subset registry, refusal protocol, kernel-defined parser per subset, hint format | after ADL-000 |
| P1 / G-M2 | VER32's Rust gates lifted and checked (scope per VER32 Q2) | after G-M1, VER32 PIF-019, ADL-010 |
| P1 / ADL-014 | **Round trip, Rust** (A3, A4): lift the generated Rust back; `rust.backend.code-link` closes | after G-M2, ADL-012 |
| P2 / ADL-015 | **Second and third lifters** (A3): Go, then OCaml (direct target), each with its VER25c-style semantics, calibrated by KRN-015/016's measured cost | after ADL-013, KRN-016 |
| P2 / ADL-016 | **Recursive elaboration** (A2): primitive recursion on direct fields; fuel for `measure`; obligation goals | after ADL-003, PIF-018, ADL-001 |
| P2 / G-M3a | Adders at the model level in the profile, for every width | after ADL-016 |
| P2 / ADL-017 | **Loop-template compiler** (A4): T1 and T2 with derived invariants | after ADL-011, KRN-012 |
| P2 / G-M3b | Adder code level (generated, then lifted) | after ADL-017, G-M2, KRN-012 |
| P2 / ADL-018 | **VC discharge with the solver** (A2, A3): bit-level obligations as certificates reconstructed into PIF | after VER34 proof mode, KRN-009 |
| P3 / ADL-019 | **K1**: `lits`, Lean `Init` `BitVec` fixed-width, `Vec`, bytes | after PIF-009 |
| P3 / ADL-020 | Later profiles: floats (needs a kernel IEEE model), stateful objects (VER04m), opaque types (VER04n) | after VER04p FL1, VER04m S1 |
| P3 / ADL-021 | Further backends and lifters, one lane per PR, in the order of 8.4, dynamic lanes last | after ADL-015 |

---

## 12. Owner questions

| # | Question | Recommendation | Alternatives |
|---|---|---|---|
| Q1 | Do generated lanes **replace** hand-written ones? | **No.** Generated code is an oracle and a differential everywhere hand-written code exists; it **is** the lane where none does (C, C++ gates). Revisit per package after a full-domain differential and a lift of both. The hand-written gates also carry the transistor routing | Replace per lane when the generated text lifts and passes the same corpus; or replace everywhere |
| Q2 | **Overflow defaults**: keep `+ - *` wrapping and spell checked or trap through `try<T>` plus the binding normalisation, or require an explicit spelling inside the profile? | **Keep wrap as the language semantics (no change to meaning).** Inside the profile, report every wrapping operation per function and **warn** on a bare fixed-width `+ - *` that no claim covers; do not forbid it | Require `wrapping_add`/`checked_add` spellings in the profile (stricter, breaks A2 copy-paste from existing models) |
| Q3 | **Which lanes get lifters first**? | Rust, then **Go**, then **OCaml** (the direct target), then Java/Kotlin/C#; dynamic lanes last and possibly never (8.4) | Prioritise by user demand, or take TypeScript early because the gates' dynamic-lane gaps are the largest |
| Q4 | Is the model-file **signature per function or per module**? | **Per module.** VER32's environment digest folds the whole file, so a per-function signature cannot stand alone; the report also prints **per-function CTE digests** so a reviewer sees which function changed | Per function (finer review, but each needs a stable sub-environment) |
| Q5 | Where do the elaborator and compiler live? | A new crate over the checked AST of `rust/warrant-grammar` (which stays an evidence crate), so the checker's size and the compiler's can be budgeted apart | Inside `warrant-grammar` |
| Q6 | Does the **signature** close `warrant.model.elaboration-faithful` for `proved`, or must a second elaborator agree? | **Signature as the interim route (converts it to (b)), second elaborator as the closing route**; the report says which was used | Second elaborator mandatory for `proved` (stronger, delays the first `proved`) |
| Q7 | Should the **adoption ladder** be a first-class report field (rung per function, per lane)? | **Yes**: A0 to A4 printed per function and per lane, so partial adoption is visible and a lifter refusal reads as "A2, not A3", not as a failure | Only the VER31 rungs |
| Q8 | Profile word in the header, or a new Warrant version number? | **Profile word** (`profile algorithm`): the version space is already crowded (v0, v1, v2 in VER04l) and a profile is a restriction plus additions, not a successor | `v3` |
| Q9 | Do backends read the **structured model IR** or **WIR**? (The brief says WIR to backends.) | **Structured IR** (7.1): WIR is a CFG and relooping makes unidiomatic text; correctness is unaffected because the reverse check judges the final text | WIR to backends: one fewer input format, harder generation |

---

## 13. Relationship to other specifications

Short dated notes the named specifications should carry (2026-11-24); **no other spec is
rewritten here.**

| Spec | Note |
|---|---|
| VER04 | Section 4: the "no recursion" rule is relaxed *only* in the algorithm profile (via VER04s `rec model` and the `measure` form). Section 1: the pure profile gains a pointer to VER33 for models that become kernel definitions |
| VER04d | Section 5: the binding schema gains the optional `realizes` field (4.8) |
| VER04e, VER04f, VER04h | "Warrant generates tests, never implementations" is no longer the whole story: VER33 generates code (A4). Test generation is unchanged and is rung A0 |
| VER04i | The lane rollout is also the A0 rung; the polyglot rule (every feature in every lane) applies to A0/A1 and to backends, but **not** to lifters, which are per-lane by cost (8.4) |
| VER05, VER06 | Solver-generated vectors (A1) are added to the corpora they run; no engine change |
| VER11a | Section 7.1: the claim elaborator also elaborates models; section 14: a K-M1/K-M2 adopter is a lane-lifted ADL function; section 9.5: the solver's certificates are the producer named here |
| VER25 | WIR is also the lifters' target (8.3); the overflow-mode mapping of 4.3 is the contract with VER25 section 2.1 |
| VER32 | See below |
| VER31 | Model-theorem category and link-status field (9.2); the new ids of 9.1 |
| LEAN00 | The direct lift target (8.3) is another producer of kernel definitions, and the same PIF path |
| LANG24, VER34 | VER34 designs the solver; VER33 states only the uses and the untrusted rule (6.3) |

### 13.1 What VER32 should change for VER33 to work

1. **Section 9.2** says the NAND constructions are "not expressible in Warrant today". A
   `model` can define them; they are not expressible as `op`s. Replace the hand-written
   structural section by the ADL-002 file; PIF-017 and ADL-003 are the same item.
2. **Goals are not in the model file.** VER32 9.2 says the model file contains "the 22
   claims as the goals"; a goal needs a theorem, and a `theorem` needs a proof. State that
   the elaborator emits a **goals list** beside the model file (definitions only), signed
   by G6.
3. **Prelude layering.** `Option`, `Result` and the helpers are in an **ADL prelude** after
   the PIF prelude; `Result` and `GateError` must not be redeclared per model file.
   Allow the environment chain `PIF prelude, ADL prelude, model, claim`.
4. **A normalisation entry point.** The differential check needs the kernel's reduction
   exposed as an untrusted subcommand (`pif-eval`: reduce a closed term to normal form
   and print it). Add it to PIF-003.
5. **Link status.** VER32 Q8's model-theorem category must also name the **link status**
   (model-only, code-linked, hand-written or generated lane), and PIF-016 must register
   VER33's ids with the gates' ones.
6. **Section 9.6** should link to this file and say PIF-017 = ADL-003.

---

## 14. What this spec does not promise

- That any rung above A0 exists for any package. A0 largely exists today; A1 to A4 are a
  queue.
- That a lane without a lifter ever gets one. The dynamic lanes may stay at A0/A1.
- That generated code replaces anything, or that it is the fastest or smallest.
- That the model is *right*. The signature says it is what a person meant; the proof says
  the properties hold of it; neither says the algorithm is the one the world needs.
- That floats, state, heap structures or concurrency are covered. Sections 4.5 and 7.2 list
  what is refused.
- A design for the solver (VER34) or for the kernel (VER11a) or for PIF (VER32).
- That the cost estimates of 8.4 are measurements. They are estimates until KRN-015/016
  report actual sizes.
