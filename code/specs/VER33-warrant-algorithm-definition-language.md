# VER33 — Warrant as the algorithm definition language

**Status:** Draft for owner approval, **revised after the first adversarial review**
(Fable, round 1: section 15). Spec only; no code exists and none is written with it.
It records owner decisions of 2026-11-24 (section 0) and the questions they leave open
(section 12). Designed against the **final** [VER32](VER32-proof-interchange-format.md)
(PR #20009, commit `d01ad9a9ab8`): separate signed model files and goals lists, the
base environment taken from the sign-off, the `Pif.*` namespace, `tcb.pif.printer`.
**Cross-spec follow-up applied 2026-11-24** (section 15, "Follow-up 2026-11-24"):
owner decisions in section 12.1; the SMT library moved to the proof side of the chain;
K1 split from the `Init` tier; the naming of ADL-002 and ADL-004a fixed from the
implementation work. Implementation is the `ADL-*` queue in
[VER-BACKLOG.d/warrant-adl.md](VER-BACKLOG.d/warrant-adl.md) (milestones in section 11),
which starts only after the owner approves this design and a second review round has
been answered.

**Layer:** Specification language, layer 4: the user-facing object above the proof
machinery. [VER04](VER04-warrant-language.md) is Warrant, the language people write.
[VER11a](VER11a-lean4-proof-kernel.md) is the kernel that decides what is a theorem;
VER32 is how a proof reaches it. This document decides how a *definition of an
algorithm* travels through all of that, to real code in every language lane and back,
**and how a team that wants only a slice of it can take only that slice**.

**Builds on:** VER04 and the family VER04a to VER04s; [VER05](VER05-property-engine.md)
and [VER06](VER06-coverage-guided-fuzz-engine.md); [VER25](VER25-wir-core-interpreter.md)
(WIR, section 8 normative semantics), VER25a, VER25b; VER11a sections 0, 1, 6, 7, 9 to
11, 13, 14; VER32; [VER31](VER31-tiered-release-standard.md) (claim rungs, buckets
`a|b|c`); [LEAN00](LEAN00-lean-on-the-generic-pipeline.md);
[LANG24](LANG24-constraint-vm.md) and VER34 (the solver, drafted in parallel, as read at
`a3fa78ef314`); [LG01](LG01-logic-gates-contract.md),
[LG02](LG02-basic-gates-cross-lane-contract.md) and
[VER-BACKLOG.d/logic-gates-everywhere.md](VER-BACKLOG.d/logic-gates-everywhere.md)
(the first adopter).

**Amends:** nothing by rewriting. Section 13 lists short dated notes the named
specifications should carry.

**Follow-up specifications, named here and not yet written** (backlog items, section 11):
**VER33a** (lifters: interface, subset registry, the kernel-side grammar interpreter,
export and whole-unit rules) and **VER33b** (the ADL-WIR value encoding, `Refines`, the
certificate format).

---

## 0. The design in one page

**The owner's decisions (2026-11-24).**

1. The repository wants an **algorithm definition language**, and it is **Warrant,
   extended**, not a new language. "We can compile, check that against the kernel, prove
   that it works and then compile the algorithm definition language into actual code. We
   will also support the other way where we can convert code into the algorithm
   definition language that our kernel understands."
2. **Partial adoption is first-class.** "People should also be able to use partial
   features from it. If someone already has code, they should be able to generate
   exhaustive fuzz tests, pbt tests and others from a Warrant file." Nobody is forced to
   adopt code generation or proofs; existing code never has to be regenerated. Section 1
   is the adoption ladder; every later section says which rung it belongs to.
3. Generated code must be **idiomatic** in each language (section 7.7).
4. The repository's **Constraint VM grows into Warrant's solver** (VER34 designs it;
   section 6.3 says how Warrant uses it, and the rule that it is untrusted).
5. Big problems decompose into **small, PR-sized units** (about 100 to 400 lines) that fit
   the existing architecture. Section 11 and the backlog shard are cut that way.

**The loop.**

1. One Warrant definition (`model` functions, structural or measured recursion, `claim`s)
   is written in an opt-in **algorithm profile** (section 4).
2. An untrusted **elaborator** turns each model function into a kernel definition in a
   **separate model PIF file** and each claim into a **goal** on a separate **goals
   list** (section 5).
3. **Proof producers** (case split, induction, SAT/SMT with certificates, Lean as the
   escape hatch) prove the goals. The kernel checks them (section 6).
4. In the reverse direction, untrusted **lifters** turn lane source (hand-written or
   generated) into WIR, and the kernel checks that the code **refines the model**
   (section 8). This is the only link between the model and **shipped text**.
5. Untrusted **lane backends** turn the definition into idiomatic source per language,
   and an untrusted **WIR compiler** produces WIR with a certificate (section 7). Their
   output has **no evidence of its own beyond tests** until the reverse check covers it.

| # | Decision | Section |
|---|---|---|
| D1 | The algorithm definition language **is Warrant**, with an opt-in **algorithm profile**. Existing documents keep their meaning. New syntax goes through `code/grammars/warrant/*.tokens` and `.grammar` and the shared grammar-driven parser, never a hand-written parser | 3, 4.9 |
| D2 | **Adoption is a ladder, A0 to A4**; each rung is useful alone; a higher rung never invalidates a lower one. **The gate computes the rung; it is never declared** | 1 |
| D3 | **A package claim rises above `tested` only through its own signed goal**, elaborated from the claim's own text. Model theorems and `Equiv` are *lemmas* in that goal's proof, never the claim itself | 1.4 |
| D4 | **Totality is required** in the profile: structural recursion, bounded loops as `fold` over `range`, and well-founded recursion by a measure, elaborated to a fuel function that returns `Option`, with a **gate-generated totality goal** | 4.1, 5.5 |
| D5 | **Errors and rejections are values.** Input contracts are part of the definition, with an exact-rejection goal | 4.4 |
| D6 | **Shallow embedding.** Each model function is one kernel definition in a separate, reviewed, signed model file | 5 |
| D7 | The elaborator is untrusted for **validity** and carries **meaning**. Two different propositions are kept apart: `<pkg>.model.faithful` (b), "this text is what we meant", closed by a signature; and `warrant.model.elaboration-faithful` (c), "the kernel definitions mean what the evaluator means", closed **only** by mechanical evidence | 5.8 |
| D8 | **The solver is untrusted.** `bounded(k)` needs a checked certificate; a bare UNSAT is `solver-attested`, capped at `tested`; a SAT answer is confirmed by evaluation | 6.3 |
| D9 | **Reverse is the only link to shipped text.** A lifter judges the **whole compilation unit**, the **export map** and the **build profiles**, over the lane function's **real parameter domain** | 8 |
| D10 | **A3 has two levels**: A3-WIR (the existing front end, `wir.lowering.faithful` open) and A3-source (the source text in the goal, which is K1). `proved` is blocked per lane on VER11a Q3 | 8.0, 9 |
| D11 | The forward WIR compiler is **optional (P2)** and contributes **no evidence about shipped code**. **A4 without a lifter is `tested`, full stop** | 7 |
| D12 | **Generated code is an oracle and differential, not a replacement**, by default (Q1) | 7.8 |
| D13 | Floats, effects and state are **not** in the first profile | 4.5 |
| D14 | The first adopter is the **eleven basic logic gates**, then adders, then ct-compare | 10 |

**What it costs.** A larger Warrant checker and elaborator; layered reviewed preludes; a
backend and (much more) a **reviewed kernel semantics of each lane's subset, with a
kernel-side parser** (section 8.4); a certificate-reconstructing solver. The ladder is the
answer to that cost: A0 and A1 need none of it.

**What it buys.** One text yields examples, property tests, fuzzing, exhaustive runs,
solver-found edge cases, a kernel proof for every input, generated code in 18 lanes, and a
check that real code is the definition. Nothing asks anyone to trust a new compiler.

---

## 1. The adoption ladder

The ladder is the central principle of this document. Each rung is **independently
useful**, built on the rungs below it only for *evidence*, never for *permission*.

| Rung | What you have | What you get | VER31 claim rung | Lane tooling it needs |
|---|---|---|---|---|
| **A0** | **Existing code + a Warrant file** | Bindings to the existing functions; generated unit tests and portable test vectors; property tests; coverage-guided fuzzing; exhaustive tests where the domain is small; hostile-input and rejection tests from input contracts | `tested` (rung 1); `exhaustive` where a domain is full (not yet emitted, LV-007d), and it then means **the Warrant type's domain**, not the lane function's (1.3) | A lane harness, PBT and fuzz bindings |
| **A1** | A0 + the **solver** | Inputs that satisfy, violate or sit on the boundary of preconditions and contracts; bounded model-versus-code checks; each counterexample becomes a regression test | **`tested`** for every lane (solver-chosen vectors are executed tests). **`bounded(k)` only for the one lane whose WIR was symbolically executed**, with a checked certificate (`drat:` or `pif:`), `k` inside the signed VC text; it opens `tcb.bitblast.encoding-correct` and `tcb.symexec.engine-correct`. An UNSAT without a checked certificate is **`solver-attested`**, capped at `tested` | None per lane for vectors; a front end to WIR for the bounded check (Rust today) |
| **A2** | A Warrant model in the algorithm profile | **Kernel proofs of the model's properties** for every input | The **model-theorem category** (VER32 Q8), *never a package rung* and never rendered as "proved" | None |
| **A3-WIR** | A2 + existing code, lifted by the lane's existing or planned **untrusted front end to WIR** | Code checked against the model at WIR level, per function | `checked` at most (VER31 rung 3): `wir.lowering.faithful` and the other (c) ids stay open | A front end to WIR, WIR semantics (VER25, PIF-019) |
| **A3-source** | A2 + existing code, with the **source text in the goal**, parsed by a kernel-side grammar interpreter | Code checked against the model with a reviewed lane semantics | `checked`; **`proved` only after** VER11a Q3 (bucket of `src.<lane>.lean-semantics.faithful`), KRN-019 and the VER31 amendments of 13 | A lane subset semantics, a grammar for the kernel interpreter, **K1** |
| **A4** | A2 + a backend | **Generated idiomatic code** in each lane | **`tested` and no more.** It rises only by the reverse check of 8.7 (A3-WIR or A3-source) | A backend per lane; a lifter to rise above `tested` |

### 1.1 What each rung takes and gives

- **A0 needs no change to the user's code and no model.** A Warrant file written today
  drives generated tests. The oracles are Warrant's own operators, never the code under
  test.
- **A1 changes how inputs are chosen, not what is claimed.** The solver's output is
  portable vectors (TFV, VER04e; corpus, VER04i) every lane already runs. Only a
  certificate-backed bounded check on a lane whose WIR was symbolically executed reaches
  `bounded(k)`.
- **A2 needs a model, not code.** The proof is about the specification.
- **A3 needs a model and existing code,** which is not regenerated, reformatted or moved.
- **A4 needs a model and gives code,** always optional, separate by default (7.8).

A higher rung never *invalidates* a lower one; a lifter refusal (8.5) leaves the package at
A0/A1/A2. **Fail closed means keep the lower rung, never block the release** (VER31
section 5).

### 1.2 What exists today, per rung

Read from the specifications on 2026-11-24; per-lane status is authoritative in the
[portable Warrant backlog shard](VER-BACKLOG.d/portable-warrant.md).

| Rung | Exists | Missing |
|---|---|---|
| A0 | Parser, checker, Rust binding schema (VER04c, VER04d); ground evaluator; L0 vectors and a Rust harness (VER04e); relation-biased property tests, bounded-exhaustive and fuzz suites (VER04f, VER04h); property engine and several lane bindings (VER05, VER05a to f); coverage-guided fuzzing and lane bindings (VER06, VER06b to g); polyglot corpus and manifests (VER04i, VER04r) | Lanes without a runner; `exhaustive` with `domain: full` (LV-007d) |
| A1 | `cdcl-sat` with DRAT, the Constraint VM stack, `smt-lib-format` (LANG24) | The solver as a Warrant service (VER34); boundary generation; WIR-to-SMT encoding; the regression loop |
| A2 | Kernel under construction (KRN-002); PIF specified (VER32) | Elaborator, ADL prelude layers, producers |
| A3 | A Rust-subset front end to WIR (`warrant-ir`, VER25a/b), **WIR-level only** | VER25c Rust semantics (KRN-015/016); the kernel-side grammar interpreter; every other lane |
| A4 | Nothing | Backends |

### 1.3 What a rung's words mean (finding 25)

`exhaustive` and the gate's "domain" statement refer to the **Warrant type**: `domain: full`
over `U8` means all 256 values, **not** the domain of a lane function whose parameter is
wider (`int`, `number`). The report prints the lane's parameter type beside the Warrant
domain, and the link status (1.5) states which values were quantified.

### 1.4 The package-claim binding rule (finding 1)

VER31 rungs are computed **per package claim**. The gates' package claims are the LG02
claims over `U8`. The model file's claims are restated over `Int`, by hand, by the
elaborator, and are about a different module. **Matching them by name is forbidden.**

**Rule.** A package claim rises above `tested` **only through its own signed goal**, which
the gate (through the elaborator, with the goals list signed under VER32 G6) elaborates
**from the claim's own text**, in one of two shapes:

| Shape | Statement | Needs |
|---|---|---|
| **Over the code** (VER11a 7.1, 7.3) | the claim with each `op` call replaced by the lifted program's run and its outcome (`outcome`, 8.2) | A3-WIR or A3-source |
| **Over the embedded domain** (gate-side *transport rule*) | the claim with each `op` call replaced by the realized model function at the canonical embedding, each `forall a: U8` replaced by `forall a: Int` with the hypothesis `0 <= a and a <= 255` (K0 admits literals up to 255), and each **`exists a: U8` replaced by `exists a: Int` with the range as a *conjunct*** (`0 <= a and a <= 255 and ...`, never an implication). The gate applies this rule itself; it is not a function of the model file's claims | A2, K0 for `U8` ranges up to 255; K1 for wider types |

**Scope of the transport rule (finding R1).** The rule replaces only `op` calls and `U8`
binders, so any other fixed-width subterm would be ill-typed at K0 or silently moved to `Int`
(`a + 1u8` wraps at 255 over `U8` but is exact over `Int`: `op(a + 1u8)` means `op(0)` in the
package claim and `model(256)` in the transported goal). **The rule therefore applies only
when every fixed-width subterm of the claim is a binder, an in-range literal, a comparison or
one of the listed canonical helpers (`b2u`-style embeddings); any fixed-width arithmetic,
`wrap<T>` or `try<T>` is refused (K1).** A refused claim stays at `tested`.

**What the transported goal is worth.** The embedded-domain goal **alone is a model theorem**:
it is about the realized model, not the shipped code. **The package claim rises above `tested`
only when `Equiv` (8.2) holds for every `op` the claim mentions**, with the transported goal
as the lemma that carries the claim's content to the code.

**Model theorems and `Equiv` are lemmas in that goal's proof**, never the claim. Restating
`and_gate_truth_table` in the model file with a narrower hypothesis or a smaller domain
produces a different goal; the package claim's own goal is unaffected, and the signer sees
the package claim's elaboration, not the model's. A claim with no signed goal stays at
`tested`. The ADL-002 model claims (10.2) are **model theorems**, never package claims.

### 1.5 The gate computes the rung

The release gate computes the A-rung of each claim and never takes it as declared. It is
printed **beside** the VER31 rung, and **A2 is never rendered as a package-level word
containing "proved"**. The **link status** states, per claim: model-only or code-linked;
hand-written or generated; **the domain actually quantified** (`Int`, `0..255`, "integer
inputs only"); and the build profiles covered (8.1). Q7.

### 1.6 Which rung each later section serves

| Section | Rung |
|---|---|
| 4 Profile, 5 Elaboration, 6.1 and 6.2 Proofs | **A2** |
| 6.3 The solver | **A1**; **A2/A3** for VC discharge |
| 7 Forward | **A4** (tests only), optional |
| 8 Reverse | **A3-WIR**, **A3-source**; closes **A4** |
| 9 Assumptions and rungs | all |
| 10 Logic gates | A0 and A1 today; A2 at G-M1; A3-WIR at G-M2; A4 at G-M4 |

---

## 2. Purpose, the loop and the trust

### 2.1 The picture

```
  PROOFS about the definition (A2)                      (all arrows are untrusted programs)

  Warrant source ─► parser ─► checked AST ─► ADL elaborator ─┬─► model PIF file   (definitions only; signed)
  (model, rec model,        (grammar-driven,   (section 5)   └─► goals list       (claim id + display CTE; signed)
   claim, realizes)          VER04c/d)                                │
        │                                                             ▼
        │                       proof producers (case split, induction, SAT/SMT + certificate, Lean) ─► claim file
        │                                                             ▼
        │                                                  kernel (TRUSTED): theorem = signed goal

  REVERSE: the only link to shipped text (A3)
  shipped lane unit ─► lane front end ─► lifter (untrusted) ─► WIR' + hints + certificate
   (whole file/package,   (grammar-driven)                              │
    export map, profiles)                                               ▼
                         kernel: lane-semantics(unit text, exports, profiles) ≈ WIR'  and  Refines(WIR', model f)

  FORWARD: generation (A4); no evidence about shipped code of its own
  checked AST ─┬─► lane backends ─► idiomatic lane source + tests ──► A0 evidence only (`tested`)
               └─► WIR compiler (optional, P2) ─► WIR + certificate ─► cross-check oracle

  SOLVER (untrusted): preconditions ─► boundary vectors (A1, `tested`);
                      WIR ─► SMT, bounded ─► UNSAT + certificate (`bounded(k)`, one lane) or counterexample;
                      VCs ─► UNSAT + certificate reconstructed into PIF (A2/A3)
```

The kernel appears in two roles: proofs about the model, and refinement of lifted WIR. The
reverse path is the **only** route from the model to shipped code. Generation produces text
that is `tested` until the reverse path covers it.

### 2.2 What is trusted and what is not

| Component | Trusted? | Where its trust goes |
|---|---|---|
| `lean-kernel` (VER11a section 4) | **Yes** | `kernel.checker.correct` (c); `kernel.logic.consistent` (open until decided) |
| PIF reader, SHA-256, import resolver, gate-binding code, **printer** (VER32 1.1) | **Yes** | `pif.reader.correct`, `tcb.pif.gate-binding`, **`tcb.pif.printer`** (all (c)); the printer covers goals **and definitions**, which the model-file signature relies on; the always-present `tcb.report.checker-correct` |
| **Reviewed semantics files** (layered): PIF prelude, **ADL prelude layers**, WIR semantics, the **ADL-WIR value encoding**, each lane's subset semantics and grammar | **Reviewed text signed by digest**: where *meaning* trust sits | `pif.prelude.faithful`, `adl.prelude.faithful` (b); `wir.semantics.lean-faithful` (c); `adl.wir-abi.faithful` (b); `src.<lane>.lean-semantics.faithful` (c) |
| **Platform compilers and runtimes** and the layers below | **Trusted, bucket (a)** ("We can definitely trust the compiler. Everything else has to be verified.") | `tcb.<lane>-toolchain.trusted` and the VER31 (a) ids |
| The **Warrant elaborator** | Untrusted for **validity**; carries **meaning** | signed model file (`<pkg>.model.faithful`, (b)) and `warrant.model.elaboration-faithful` (c) (5.8) |
| The **solver** (VER34) and the **WIR-to-SMT encoder** | **No**, but the encoder's meaning matters for `bounded` | UNSAT needs a certificate; `bounded(k)` also opens `tcb.bitblast.encoding-correct`, `tcb.symexec.engine-correct` and `tcb.vcgen.claim-faithful`, and `tcb.drat.checker-correct` while the DRAT route lasts (6.3) |
| The **SMT library** (`Pif.Lrat`, `Pif.Prop`, `Pif.BV`, their bridges; VER34 6.8) | **No, and it is not meaning.** It is **proof plumbing on the proof side** of the chain (VER32 E7): unsigned, not allow-listed, checked by the kernel like any other file | none: a wrong library definition can at worst make a proof fail, because no proof-side file may declare a constant occurring in a signed goal (VER34 CVM-025). Its faithfulness to the signed meaning (the WIR semantics and the ADL fixed-width definitions) is a **bridge theorem** (VER34 6.9), not an id |
| The **WIR compiler** and its certificate emitter | **No**; no evidence of its own about shipped code | none |
| **Lane backends** | **No** | `<lane>.backend.code-link` (c) until the reverse path covers the text (8.7) |
| **Lifters**, front ends, invariant inference | **No, for validity** | `src.<lane>.lean-semantics.faithful` (c) and, at A3-WIR, `wir.lowering.faithful` |
| **Proof producers** | **No** | none |
| Test, property, fuzz generators and harnesses | Not trusted for proof; rung A0 evidence | none |

### 2.3 Where meaning trust sits

Producers are untrusted for validity, and trust moves for *meaning* (VER32 1.3) into:

1. **Warrant text to kernel text**: the elaborator (5.8).
2. **Kernel text about code**: WIR's semantics, the value encoding, and each lane's subset
   semantics and grammar. A wrong one yields a true theorem about a different machine.
3. **What a goal says**: the signed goals list (VER32 3.6), signed with the **base
   environment** digest taken from the sign-off (VER32 E1, E6).

A wrong lifter or backend never yields a wrong *proof*; at worst a statement about
something other than the code given, which the signed goal and the source-text-in-the-goal
rule (VER11a 14.3, A3-source) catch.

### 2.4 Why the reverse direction is the point

Generation alone produces text nothing connects to the model. Lifting is the **only** link
between a model and shipped text, for hand-written and generated code alike. A generated
file is therefore no better evidenced than a hand-written one until it is lifted, and the
spec says so (D11). The forward WIR compiler is *not* that link: it checks a program
nobody ships (finding 11), so it is optional (7.2).

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

A kernel definition must terminate: the kernel reduces it (VER11a 6.2) and a function that
can diverge cannot be a theorem's subject. Four admitted shapes:

| Shape | Surface | Why total | Elaborates to (5.5) |
|---|---|---|---|
| **Non-recursive** | `model f(...) = e`, calling only earlier models | acyclic call graph | a plain definition |
| **Bounded loop** | `fold(range(n), init, step)` and the other WX-D forms (VER04k 2.7) | `range(n)` has `n` elements | `Nat.rec`/`List.rec` through a prelude helper |
| **Structural recursion** | `rec model f(...) -> T decreasing p = e`, every recursive call on a derived term of `p` (VER04s 4.2, RR1 to RR4) | the argument is a proper subterm | the kernel recursor of `p`'s type |
| **Well-founded recursion** | `rec model f(...) -> T measure m = e`, `m` a `Nat` expression over the parameters | the measure strictly decreases | a **fuel** function returning `Option`, plus a **gate-generated totality goal** (5.5) |

Further rules:

- **Structural recursion on `Nat`** is allowed, with arms `0` and `n + 1` (a new pattern
  form; VER04s does not peel a `Nat` today).
- **Mutual recursion** is a group (VER04s 4.1, at most 16 members). The first elaborator
  slice refuses groups (5.5).
- **A `measure` model is outside A0/A1 evidence** (finding 14). VER04s 1.2 rejects "general
  recursion with a fuel limit as the only guard" because an oracle cannot confirm
  termination and the labels would not be honest. A `measure` model therefore **does not
  enter generated tests, the corpus IR or the polyglot rule** until its totality theorem
  (5.5) is checked; the evaluator refuses it **for test generation** with a diagnostic, and runs it under its
  meter **only for the elaboration differential** (5.8), so `elaboration-faithful` has vectors
  to close on (backlog ADL-005e). It is an **A2-only** construct in the first profile. Structural recursion (`decreasing`) and `fold` over
  `range` are the A0/A1 paths and follow VER04s RR4's polynomial bound. No VER04s amendment
  is needed; the backlog has no 18-lane `measure` rows.
- **A definition whose totality is not discharged is not a model:** the gate refuses any
  goal whose footprint contains a measure-defined constant unless its totality theorem is
  in the environment (5.5); the gate knows which constants are measure-defined **from its own
  elaboration run**, not from a `_fuel` naming convention. It is not a producer policy; it is a gate rule.
- The ground evaluator keeps its work budget (VER04 section 2), 1,000,000 work units and a
  bounded `Int` (VER04j W0703). Budget exhaustion is a diagnostic, never a value.

### 4.2 Types

| Warrant type | Kernel image (5.3) | Tier |
|---|---|---|
| `Bool` | `Bool` | K0 |
| `Int`, `Nat` | `Int`, `Nat` (inductives) | K0 |
| records, tuples, variants, `error` enums, aliases | inductives (records through `T.rec`, no `proj` at K0); right-nested `Prod`; the aliased type | K0 |
| `Option<T>`, `List<T>`; `Result<T,E>` | `Option`, `List` (Init-identical); `Pif.Result` | K0 |
| `U8`..`U64`, `I8`..`I64` | a fixed-width word type from the ADL prelude (native `Adl`/`Pif` words, 6 below) | **K1** |
| `Bytes`, `Bytes<N>` | `List` of bytes; an indexed vector `Adl.Vec α n` | **K1** |

**K0** needs only the kernel features VER32 lists for G-M1: inductives, recursors,
iota, beta, delta, zeta, proof irrelevance, function eta; no literals, quotients or
projections. **K1** (split 2026-11-24, VER34 13.1 item 2) adds **literals and `Nat`
acceleration** (`lits`, VER32 3.5 `prim` roles, which enter the footprint, allowed by the
gate's named configuration, VER32 3.5) and **native fixed-width words in the `Adl` and
`Pif` namespaces, with no `Init`** (the same word representation VER34's `Pif.BV` bridges
use, VER32 PIF-019), because a 64-bit constant cannot be a unary `Nat`. **K1 imports
nothing from Lean.** Importing Lean's `Init` through the Lean converter (PIF-008/009) is a
**separate, later tier, the `Init` tier (named K2 here)**, needed **only for the Lean
escape hatch** (6.2, ADL-060b); there is **no third bridge** between a K1 word and an
`Init` `BitVec` on the main path. A definition using a K1 type is accepted by the checker
and by every test lane (A0/A1 unaffected); the **elaborator refuses it with a diagnostic
naming the tier**. K0 admits `Nat` and `Int`
literals up to 255; larger ones are a K1 diagnostic.

`Int` in a definition compiled to a fixed-width lane needs a range fact (the compiler and the transport rule, 1.4); the
compiler refuses an `Int` expression it cannot bound unless the lane has unbounded
integers (Python, Ruby, Elixir, Haskell `Integer`; **not** Perl or Haskell `Int`).

### 4.3 Overflow: three declared choices

Warrant already settles the default (VER04j 2.3, VER04r 3.2): `+ - *` on a fixed-width type
**wrap**, in every profile and lane; `Int` and `Nat` arithmetic is exact; no implicit
conversion. The profile keeps that and names the three choices so a definition says which
it means and each tool has one mapping:

| Choice | Written | WIR overflow mode (VER25 2.1) | Lane code |
|---|---|---|---|
| **wrap** | `a + b` on `Un`/`Sn`; `wrap<T>(i)` | `Wrap` | the lane's wrapping operation, **spelled explicitly** (for example `wrapping_add`), so the result does not depend on a build profile (8.1) |
| **checked** | compute in `Int`, then `try<T>(i)`: `Option<T>`; the error is a value | `Wrap` arithmetic plus an explicit range test | an explicit range test; no reliance on host overflow behaviour |
| **trap** | *checked*, plus a binding-level **error normalisation** (`panic_to_error`, VER04d 5) | `Trap` **only if the compiler states and the kernel proves the goal** `trap_eq_test` (the trapping operation equals the model's range test at every input); otherwise the compiler **refuses** to use `Trap` mode and emits the explicit test | the lane's panicking or throwing form |

**Trap is not a third arithmetic.** It is checked arithmetic whose error case the binding
declares to be a panic. The printed model names every wrapping operation explicitly
(`Adl.U8.wrapAdd`), so a signer sees them, and the **lifter subset refuses a bare `+` whose
meaning depends on the build profile** (8.1). Whether the profile should also *require* an
explicit spelling in Warrant source is Q2.

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

Today an `op` (a signature bound to a host export, VER04d 5) and a `model` are unrelated.
The profile adds one fact, as **data in the binding schema**:

```json
"nand_gate": { "export": "nand_gate", "fresh": false,
  "realizes": { "module": "logic_gates_model", "model": "nand_gate",
                "embedding": "int-by-value",
                "panic_messages": ["a must be 0 or 1", "b must be 0 or 1"] },
  "negative_controls": ["always_true"] }
```

`realizes` names a model function and **selects among canonical embeddings** (`int-by-value`
for integers, `bytes-identity`, componentwise `Option`/`Result`), nothing else: no
user-supplied conversion function. It does **not** state the domain. The domain of the
refinement is fixed by the source (finding 2):

- **The quantifier ranges over every value of the lane function's parameter types**, as the
  kernel-parsed source declares them, through a **total embedding from the lane type into the
  model type**. A Go gate taking `int` quantifies over all `int`s; one that begins with
  `a = int(uint8(a))` therefore **fails** `Equiv` at 256, where the model says `Err`. The
  binding's Warrant `U8` types play no part.
- Where the lane domain **does not embed** (dynamic lanes: `true`, `1.0`, `"1"`, `-0`), the
  link status says **"integer inputs only"** and the claim's rung is **capped** (A3 at most
  `checked`, never `proved`); type-confusion inputs stay covered only by the LG02 matrix.
- **`enc` of a result is partial**: a model result that does not fit the lane's result type
  is a **refinement failure**, never a truncation.
- `panic_messages` is the **finite, declared set** of the lane's panic or exception
  identities that count as the modelled rejection (the gates' two `validate_bit` messages
  both mean `INVALID_INPUT`, as LG01 says). Any other trap (an `unwrap`, an index, a debug
  overflow) is **not** the modelled rejection and fails `Refines` (8.2).

A realization yields a **link goal** (8.2). Realization is optional; without it the op has
A0/A1 evidence only.

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
refusal) are allocated by ADL-001 in the existing registry. **ADL-001a allocated** (VER04i
registry, `W0801` to `W0807`; implemented in `rust/warrant-grammar`):

| Code | Stage | Meaning |
|---|---|---|
| `W0801` | parsing | Bad `profile` clause: a name other than `algorithm`, a missing name, a second clause, or a profile on a v0 document (a profile **requires `v1`**; `pure` is the absence of a clause, never spelled) |
| `W0802` | parsing | `measure` in an algorithm-profile document (the `rec ... measure` form and the word anywhere). Admitted by ADL-001b |
| `W0803` | parsing | Unbounded loop or recursion: `rec`, `while`, `loop`, `for`, `until`, `repeat`, `goto`. `fold` over `range` is not refused |
| `W0804` | parsing, checking | `F32` or `F64` (4.5) |
| `W0805` | parsing, checking | Effects and state (4.5): `inout`, `fresh`, `vectors`, `ensures`, `driver`, `mut`, `mutable`, `state`, `effect`, `io` |
| `W0806` | binding | Invalid `realizes`: unknown embedding, missing model function or module, a model module not in `profile algorithm`, arity or type mismatch, `realizes` without `embedding` (or the reverse), `panic` without `realizes` |
| `W0807` | binding | Invalid `panic` template: empty, wildcard (`*`, `?`, or only format holes), duplicate, more than 16, or compiler-generated (`attempt to`, `unwrap()`, `index out of bounds`, ...) |

**Panic templates (2026-11-24 security review of ADL-001a).** A template must begin with a
literal **anchor of at least 8 ASCII alphanumeric characters** counted before its first `{}`
hole (spaces and punctuation may sit between them; no leading hole; holes after the anchor are
allowed), so `a must be 0 or 1, got {}` passes and `{}: {}`, `a{}`, `{} !` fail: a near-wildcard
would otherwise match `index out of bounds: the len is 3 but the index is 5` (R5). Only
printable ASCII (0x20 to 0x7E) is allowed, which refuses NBSP, zero-width, bidi and U+2028/2029
characters. The compiler-generated denylist is matched after collapsing whitespace, and again
with the holes removed, and covers the Rust, Go, Java, Python and JS/TS wordings. It is a
backstop: the long-term check is the lifter confirming each template against a literal panic
message in the lane source (ADL-032, ADL-033). `ModelModule` is built only from a parsed file
(its profile is read from the file) and duplicate model module names are `W0806`; the profile
words are also reserved in the module name.

Inside the profile the refused words are reserved wherever they appear (a model named
`state` is refused), the first one in source order wins, and a document without the
clause never sees them. **Binding syntax (ADL-001a):** in `evidence.bindings`, `op NAME ...
realizes=MODULE.MODEL embedding=int-by-value|bytes-identity` and one `panic OP TEMPLATE`
line per message (the template is the rest of the line, because it contains spaces);
`Option`, `Result`, `List` and tuples embed componentwise around the leaf embedding.
Nothing consumes a realization yet.

---

## 5. Elaboration to the kernel: meaning (rung A2)

### 5.1 Shape

The elaborator is a program from a *checked Warrant AST* to two artefacts:

| Artefact | Contents | Trust |
|---|---|---|
| **Model PIF file**, one per module | one kernel `def` per `model`/`rec model` and, per declared type, an `inductive` block, in dependency order; **definitions only, no theorems** | reviewed through the printer (VER32 3.6), signed by digest (`<pkg>.model.faithful`, (b)); the gate checks `parse(print(F)) = F` over it (`tcb.pif.printer`) |
| **Goals list** | a text file, **one line per goal**: the claim id, one space, the **display CTE** of the goal (VER32 9.2), and a line feed | each goal signed under VER32 G6 with the **base environment digest** taken from the sign-off |

The proof producer writes a *claim file* containing only theorems and goal bindings; it may
declare no constant that occurs in a goal and no axiom. **The signed base is fixed by the
sign-off** (VER32 E1), in order: `PIF prelude, ADL prelude layers, [WIR semantics at
A3-WIR], model file(s), [axioms file, classical claims only]` (VER32 E8). The **proof
side** follows and is **not signed and not allow-listed** (VER32 E4, E7): the SMT
libraries when bounded or SMT-derived facts are used (VER34 6.8: `Pif.Lrat`, `Pif.Prop`,
`Pif.BV`, bridges), then the claim file. (The first revision put the SMT library in the
signed base; that is superseded: libraries are proof plumbing, not meaning, and a change
to one must not re-sign a model.)

The model file is **shallow**: a Warrant `model f` is a kernel constant `f` of the translated
type, so `f 3` reduces in the kernel by beta, delta, iota and zeta.

**Names.** A kernel name is `<module>.<name>`; types `<module>.<Type>`, constructors
`<module>.<Type>.<Ctor>`; every component is ASCII `[A-Za-z_][A-Za-z0-9_']*` (VER32 3.8).
Natives live in `Pif.*` and helpers in `Adl.*`; **one rule**: a name that is not
declaration-identical to Lean's `Init` is `Pif.*`, `Adl.*` or the module's own, and a type
lives in **exactly one** place. A **claim id** matches VER32 G3's charset
`[A-Za-z0-9_.-]{1,128}`: `<module>.claim.<name>` (finding 18). **Generated goals live under a reserved segment**,
`<module>.gen.<name>.total`, `<module>.gen.<name>.witness.<n>`, `<module>.gen.<name>.beq_spec`
and so on, and **`claim`, `gen` and `contract` are reserved** (a module, type or claim named
any of them is a diagnostic), so a user name cannot collide with a generated id. `contract`
is the segment of the exact-rejection goals `<module>.contract.<f>.<n>` (5.10); it joins the
reserved set for the same reason `gen` is in it (the redesign recorded in 15). The totality rule compares the
**regenerated goal digest**, not the id.

### 5.2 The ADL prelude, as append-only layers

A single prelude digest would void every signature in the repository whenever a helper is
added (VER32 3.7: a file's bytes depend on everything before it). So the ADL prelude is
**append-only layers, each its own chain entry**, with a stated ordering rule:

| Layer | Contents | Needed by |
|---|---|---|
| L1 `adl.core` | `Adl.ite.{u}` (type argument explicit, 5.4), `Adl.Int.{beq, le, lt}` and `add`, ..., `Adl.Nat.{beq, ble, blt}`, `Adl.foldRange`, `Adl.Option.isSome`, `Option`/`Pif.Result` helpers, generic `beq` combinators | every model |
| L2 `adl.list` | `List` helpers (`get?`, `append`, `length`, folds) and their specification lemmas | models using lists |
| L3 `adl.fixed` (K1) | native fixed-width words (no `Init`), `Vec`, bytes | K1 models |

**Ordering rule (finding R2).** VER32 E2 makes a file's `import`s the digests of **everything
before it**, so a layer used under two different prefixes is **two files with two
signatures**. Therefore the layers form **one global append-only sequence L1..Ln**; **a model's
chain is always a *prefix* L1..Lk, never a subset**; and a new layer or helper goes **at the
end** (a new layer file, never an edit; an edit re-signs everything after it). A model that
needs only `adl.core` has the prefix `[L1]` and **does not change digest when later layers are
appended**. **Costs, stated plainly:** one chain entry and one allow-list entry per layer in
the prefix, at most the 65-file chain of VER32 3.7; and a **model file imported under a longer
prefix is a new file with a new digest and a new signature**. The gates' model under `[L1]` and
the same text under `[L1, L2]` (for the adders, which need lists) are **different files**.
Cross-module models (the adders calling gate models) place the called module after the shared
prefix, so editing the gates' model re-signs the adders' base chain; the sign-off records the
base chain per claim (VER32 E1), so the re-sign is per claim set, not repository-wide.

**Proof libraries are not layers.** The prefix rule governs only the signed base. A proof
library (VER34) is on the proof side and is added to, or changed, without touching any
prefix, signature or allow-list entry.

Each layer is allow-listed and signed (`adl.prelude.faithful`, (b)). **Helper specification
lemmas are mandatory** (5.10), so a helper that disagrees with its specification is found by
the kernel, not only by review.

### 5.3 Types

| Warrant | Kernel |
|---|---|
| `Bool`, `Nat`, `Int` | `Bool`, `Nat`, `Int` |
| `Option<T>`, `List<T>` | `Option ⟦T⟧`, `List ⟦T⟧` (declaration-identical to `Init`) |
| `Result<T,E>` | `Pif.Result ⟦T⟧ ⟦E⟧` |
| `(T1, ..., Tn)` | `Prod ⟦T1⟧ (Prod ⟦T2⟧ ... ⟦Tn⟧)`; the last component is not wrapped |
| `error E = A \| B`, enum, variant | `inductive <M>.E` in declaration order; the gates' `GateError` is **`Pif.GateError`, declared once in the gate model file** (ADL-002; **not in any prelude**: `Pif.Result` is in the PIF prelude, the error type is the gates' own), and never redeclared |
| record | one-constructor inductive; **field access is `T.rec` with a projection function the elaborator defines** (K0 has no `proj`: VER32 puts projections and structure eta in slice 3b). **Records stay through `T.rec` permanently**, not switching to `proj` at slice 3b, so no model file changes digest and nothing is re-signed |
| alias `model T = U` | `⟦U⟧` |
| `Nat <: Int` widening | `Int.ofNat` at the widening site |
| `Bytes<N> <: Bytes` widening (K1) | `Adl.Vec.toList` |

### 5.4 Expressions

| Warrant | Kernel (⟦·⟧) |
|---|---|
| variable, parameter | de Bruijn variable |
| `0`, `1`, small literals (K0) | `Int.ofNat`/`Nat.succ` chains over `Nat.zero` |
| `if c then a else b` | `Adl.ite.{u} ⟦T⟧ ⟦c⟧ ⟦a⟧ ⟦b⟧` with the **result type `⟦T⟧` as an explicit first argument** (`alpha`), and `Adl.ite.{u} (alpha : Sort u) (c : Bool) (t e : alpha) : alpha := Bool.rec (motive := fun _ => alpha) e t c` (ADL-004a) |
| `not a`, `a and b`, `a or b`, `a => b` | `Pif.not`, `Pif.and`, `Pif.or` (defined by `Bool.rec`, hence lazy); `a => b` is `Pif.or (Pif.not a) b` |
| `match e with ...` | the type's recursor with a non-dependent motive, **after pattern compilation** (below) |
| call `g(a, b)` | `g ⟦a⟧ ⟦b⟧` |
| `a == b` | `Adl.beq_T ⟦a⟧ ⟦b⟧` where a top-level claim `==` is stated as `Eq T` (5.10); the elaborator **derives** `beq_T` per declared type |
| `a < b`, `<=`, `>`, `>=`, `==` on `Int` | **`Adl.Int.lt`, `Adl.Int.le`, `Adl.Int.beq`** (there is no `Int.ble` or `Int.blt` in the prelude). `a > b` is `Adl.Int.lt ⟦b⟧ ⟦a⟧` and `a >= b` is `Adl.Int.le ⟦b⟧ ⟦a⟧` (the arguments are swapped, no extra constant) |
| `a < b`, `<=`, `==` on `Nat` | **`Adl.Nat.blt`, `Adl.Nat.ble`, `Adl.Nat.beq`**, with `>` and `>=` by swapping, as for `Int` |
| `len`, `at`, `concat`, `div`, `fixed<N>` | the prelude function of the same name |
| `fold(range(n), init, f)` | `Adl.foldRange ⟦n⟧ ⟦init⟧ ⟦f⟧`; `foldRange 0 a f = a`, `foldRange (k+1) a f = f (foldRange k a f) k` (the order of VER04k 2.7) |
| `forall x: T \| p. q` (claims) | `∀ x : ⟦T⟧, Eq Bool ⟦p⟧ true → Eq Bool ⟦q⟧ true` |
| `exists x: T \| p. q` (claims) | `Exists (fun x => And (Eq Bool ⟦p⟧ true) (Eq Bool ⟦q⟧ true))` |

**Pattern compilation (finding 21).** Warrant matching is **first-match** (VER04j 4.5), with
`_` wildcards in v1. The K0 elaborator accepts only **flat** matches: each arm a single
constructor pattern with variable or `_` fields, optionally **one final `_` arm**. A final
`_` is **expanded into one arm per constructor not yet listed**, each with the wildcard's
body (binders unused); with flat patterns, first-match and "the arm for this constructor"
coincide. Arms are then reordered into declaration order. **Nested patterns, tuple patterns
with wildcards, and literal patterns are refused** (the `Nat` patterns `0` and `n + 1` of 4.1 are **exempt**) with a diagnostic (the source is rewritten
as nested matches); a later slice may add a decision-tree compiler.

### 5.5 Definitions and recursion

- **Non-recursive**: `def <M>.f : Π (x : T) .., R := fun x .. => ⟦body⟧`.
- **Bounded loop**: through `Adl.foldRange` and the other prelude folds.
- **Structural recursion on `p : T`**, in the first slice **primitive recursion on direct
  constructor fields**: `f p args = T.rec (motive := fun _ => Π args', R) (case_C1 ...) ... p
  args`. The recursive hypothesis for a child field is a function of the other parameters, so
  carries and accumulators may change per call. A deeper derived term, a `fold` over children
  or a mutual group is refused.
- **Well-founded recursion by measure `m`** (A2-only, 4.1): no `WellFounded.fix`. Instead
  (finding 4):
  - `f_fuel : Nat → Π params, Option R` by `Nat.rec` on the fuel, returning **`none` at
    exhaustion**. There is **no default value**, so exhaustion can never coincide with a real
    result (an exact-rejection goal cannot be true by exhaustion).
  - **`f` itself is not a constant.** The only constants are `f_fuel` and the measure `m`;
    goals about `f` are stated over `f_fuel` at fuel `m x + 1`, as
    `Eq (Option R) (f_fuel (m x + 1) x) (some e)`, and are admissible only when the
    totality goal below is in the environment.
  - The **gate generates, from the name alone**, a fixed-shape goal
    `<module>.gen.<f>.total : ∀ x, Eq Bool (Adl.Option.isSome (f_fuel (m x + 1) x)) true` (**`Adl.Option.isSome`**, an `adl.core` helper, not an `Option.isSome` from `Init`; ADL-004a) and a
    **fuel-monotonicity** goal `<module>.gen.<f>.mono`. These lines appear on the goals list
    and are signed with it. The **per-call-site obligations** `m(args) < m(params)` are the
    **producer's private proof method**, not a signed object: a dropped call site makes the
    producer's proof fail, not the signed statement weaker.
  - **Gate rule:** any goal whose footprint contains a measure-defined constant is
    **refused** unless `<f>.total` is accepted in the same environment. The theorems live in
    the **claim file**; for reuse across claim files, in a **lemma file placed in the base
    chain after the model file** (its digest then enters the sign-off, so a re-proof
    re-signs; the claim file is the default).
  - The counterexample `rec model g(n: Nat) -> Bool measure n = g(n)` therefore elaborates to
    a `g_fuel` that is `none` everywhere, `g.total` is **unprovable**, and every goal
    mentioning `g` is refused. A constant `false` can never be proved about.

  - **`.mono` statement.** `<module>.gen.<f>.mono : ∀ n k x v, f_fuel n x = some v →
    f_fuel (n + k) x = some v`.
  - **Callers (finding R3).** Another model's call to `f` would receive `Option R`, and the
    model file has no theorems. **The first slice refuses calls to a measure model from other
    models** (a diagnostic naming the caller). A later slice may define `f` in a **second model
    file placed after the lemma file** that holds `.total`, which then re-signs per 5.2.

### 5.6 What the elaborator cannot do

It does not choose tactics, prove, or rewrite a model. Refusals are diagnostics naming the
construct and the tier (K1, mutual groups, deeper recursion, nested patterns, floats,
state). **No silent partial output**: an unelaborated function is absent and listed.

### 5.7 Determinism

The model file is deterministic: dependency order (ties by source order), names as 5.1, arms
in constructor order, helpers by name, no inlining. Two correct elaborators produce the same
bytes **given the same translation tables** (5.4). That is why agreement of two elaborators
catches implementation slips but **not table errors**; table errors are caught by 5.8's
differential and by the specification lemmas (5.10).

### 5.8 Trusting the elaborator for meaning

Two different propositions, kept apart (finding 6, Q6):

| Proposition | Id | What closes it |
|---|---|---|
| "This printed model and these goals are what we meant" | `<pkg>.model.faithful`, **(b)** | **A signature** on the model file digest and the goals list (VER32 G6) |
| "The kernel definitions mean what the Warrant evaluator means" | `warrant.model.elaboration-faithful`, **(c)** | **Mechanical evidence only**: all three of (1), (2), (3) below. A signature **never** closes it |

The signature is not a discharge of the second proposition (VER31 3.3: a (c) id closes only
by a discharge the checker accepts).

1. **A second, independent elaborator**, written without reading the first, with agreement on
   the model file and goals list bytes across the corpus. Catches implementation slips.
2. **The evaluator-versus-kernel differential, as kernel theorems** (finding 7). For every
   generated vector, example, bounded-exhaustive case and a seeded sample, the harness states
   the **kernel theorem** `Eq T (f args) expected`, with `expected` built by an **independent
   encoder** (a separate crate; it shares no code with the elaborator or with a decoder), and
   proved by `Eq.refl` through the ordinary PIF path. No new normaliser and no decoder are in
   the loop: the kernel's definitional equality is the test. Requirements:
   - **per-function coverage**: every arm and branch of every definition in the signed file is
     reached by a non-skipped vector (a three-constructor sum needs a three-constructor
     vector, so arm-reordering bugs cannot hide behind two-constructor corpora);
   - a **maximum skip ratio** for budget-exhausted or K0-unary-limited cases (default 10%, and
     no function with all vectors skipped);
   - **claims are covered**: every claim is instantiated at ground values and its hypothesis,
     `=>` and quantifier translation are checked by comparing the **hypothesis and the
     conclusion separately** with the evaluator's values, **false ones included** (not only
     `Eq Bool ⟦instance⟧ true`);
   - an `Eq.refl` that **exceeds kernel limits counts as a skip** under the 10% cap;
   - a **goal-level mutation corpus** (drop or add a hypothesis, swap sides, weaken a domain),
     each mutant detected, as VER11a 7.4 requires, and an **elaborator mutation corpus** (swap
     `ite` branches, drop an arm, reorder constructors, flip a comparison).
3. **Mandatory specification lemmas** (5.10): `beq_T a b = true ↔ a = b` (an `And` of both directions) for every derived
   `beq_T`, and each helper against its specification.

`proved` for any claim whose statement mentions an elaborated function requires **all three**
(Q6). **Honest limits.** The evaluator's meaning is **budget-bounded**: "faithful to the
evaluator" holds **wherever the evaluator returns a value** (finding 26); K0's unary numbers
restrict vectors to small values and the evaluator's `Int` is bounded (VER04j W0703), so large
values are covered only by the lemmas. None of this checks that the *evaluator* is what the
VER04 specification says; that is A0's own question.

### 5.9 Worked example: the logic gates

`logic-gates-basic.warrant` declares gates as `op`s and has no gate definition; the model
file, **new, in the algorithm profile**, defines them. The domain is `Int` (VER32 Q4). The
**rejection guard is `to_bit(a) = None`, not `a > 1u8`**, because over `Int` the latter does
not reject `-1` (VER32 9.2): that is a **meaning change the `U8` sign-off does not cover**,
reviewed under `gates.model.faithful`. Long `match` expressions continue inside parentheses.

```text
warrant logic_gates_model v1 profile algorithm
  error GateError = INVALID_INPUT          -- maps to Pif.GateError (5.3)

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

(The other nine gates follow; `nand_or` and `nand_xor` are the Rust source's constructions as
`model`s. The **oracles** stay Warrant's own operators in the LG02 files; the model is the
**definition**, the LG02 rows the independent check. The LG02 file has 26 claims: 11
`*_truth_table`, 11 `*_rejects_non_bit`, 4 `*_equals_*`; VER32 9.2 lists their treatment.)

**Elaborated model file**, printed form (the on-disk PIF is VER32 section 4; universe levels
omitted):

```text
def logic_gates_model.to_bit : Int → Option Bool :=
  fun (x : Int) =>
    Adl.ite (Option Bool) (Adl.Int.beq x (Int.ofNat Nat.zero))
      (Option.some Bool Bool.false)
      (Adl.ite (Option Bool) (Adl.Int.beq x (Int.ofNat (Nat.succ Nat.zero)))
         (Option.some Bool Bool.true) (Option.none Bool))

def logic_gates_model.from_bit : Bool → Int :=
  fun (b : Bool) => Adl.ite Int b (Int.ofNat (Nat.succ Nat.zero)) (Int.ofNat Nat.zero)

def logic_gates_model.nand_gate : Int → Int → Pif.Result Int Pif.GateError :=
  fun (a b : Int) =>
    Option.rec (motive := fun _ => Pif.Result Int Pif.GateError)
      (Pif.Result.err Pif.GateError.INVALID_INPUT)                  -- none first
      (fun (x : Bool) =>
         Option.rec (motive := fun _ => Pif.Result Int Pif.GateError)
           (Pif.Result.err Pif.GateError.INVALID_INPUT)
           (fun (y : Bool) => Pif.Result.ok (from_bit (Pif.not (Pif.and x y))))
           (to_bit b))
      (to_bit a)

def logic_gates_model.nand_not : Int → Pif.Result Int Pif.GateError :=
  fun (a : Int) => nand_gate a a
def logic_gates_model.nand_and : Int → Int → Pif.Result Int Pif.GateError :=
  fun (a b : Int) =>
    Pif.Result.rec (motive := fun _ => Pif.Result Int Pif.GateError)
      (fun (n : Int) => nand_not n)           -- ok, then err (declaration order)
      (fun (e : Pif.GateError) => Pif.Result.err e)
      (nand_gate a b)
```

(The recursor takes minor premises in constructor declaration order, so for `Option` and
`Pif.Result` the arms are reordered; `Bool.rec` takes `false` first; `Adl.ite α c t e := Bool.rec (motive := fun _ => α)
e t c`, the result type `α` being an explicit argument, 5.4.)

**One line of the goals list**, as VER32 9.2 fixes the format:

```text
logic_gates_model.claim.nand_and_equals_and ∀ (a : Int) (b : Int), Eq (Pif.Result Int Pif.GateError) (logic_gates_model.nand_and a b) (logic_gates_model.and_gate a b)
```

**The proof** is written by the case-split producer (VER32 PIF-006) in the claim file:
generalise `to_bit a` and `to_bit b`, then `Bool.rec` on the two bits, leaves by `Eq.refl`.
A producer cannot weaken the claim: the theorem's type must equal the signed goal.

**Not the package claim.** This goal is a **model theorem** (1.4). The package's own claim
`nand_and_equals_and` over `U8` gets its **own goal**, produced by the gate's transport rule
(`forall a: Int, b: Int | 0 <= a <= 255 ...`), whose proof uses this theorem and `Equiv` as
lemmas.

**What the signer sees**: the printed model file, the printed goals list and, per package
claim, its transport elaboration. Nothing was written by the proof producer.

### 5.10 Mandatory lemmas and witnesses (findings 8, 9; redesigned 2026-11-25)

Gate conditions, each a **generated line on the goals list** (claim ids as in 5.1), proved in
the claim file or the prelude layer. The first design of this section had the gate *detect*
Warrant constructs in kernel terms and relax an obligation whenever a detector said "no". Three
security-review rounds each found a new way for that to fail open, and Fable's design review
(15, 2026-11-25) traced all of them to that one habit. The design below is **derive, don't
detect**: the gate derives every obligation from the signed goals by a fixed syntactic
transformation, and nothing a detector decides can remove one.

**What an accepted chain guarantees.**

- **(G1)** For every `claim` goal `Π Γ. C` with a binder, the kernel has proved
  `(Π Γ. False) → False`, i.e. the telescope is inhabited.
- **(G2)** For every claim with a hypothesis, the kernel has proved the hypotheses and the
  conclusion at closed values the goal displays.
- **(G3)** For every constant outside the pinned prelude and ADL files whose result type reduces
  to `Pif.Result a b`, the kernel has proved one input returning `ok` and one returning `err`.
- **(G4)** Every constant named `…beq` has type `T → T → Bool`, and its `beq_spec` is proved.
- **(G5)** Every `contract` goal is a biconditional `P ↔ f xs = err E` over a single call of `f`.

The gate derives each generated statement from the signed claim and model by a fixed syntactic
transformation. **No rule exempts a goal because the gate failed to recognise something.**

**What the gate does not guarantee.**

- A guard written inside a Boolean conclusion or inside a model definition is **not a
  hypothesis**, and gets no witness.
- A `Result` nested inside another type gets **no outcome witnesses**.
- That every source `rejects`, `==` and `measure` produced its goal is covered by
  `warrant.model.elaboration-faithful` (5.8 item 1), **not by a gate condition**.

**The invariants** the design keeps, which a reviewer should test any later change against:

- **I1 Totality.** The obligations are a **total function** of the signed goals list plus the
  base. No "no" from a detector removes an obligation: where the gate cannot classify
  something, it keeps the obligation, or it refuses.
- **I2 The kernel decides semantics.** The gate may only classify a binder by its **kernel
  sort**. It never inspects a body to decide what a goal "means".
- **I3 Discovery is role-independent.** A constant is found by its name and type wherever it is
  declared; which file or role declares it changes nothing about whether it is checked.
- **I4 Applicability is an explicit input.** Whether the ADL rules apply (`--profile adl|plain`)
  is a value the sign-off binds and the signature covers; it is never inferred from the files.
- **I5 Generated goals are functions of claims and the model.** No generated goal depends on
  the proof side, so a proof producer cannot change what it is asked to prove.
- **I6 Completeness against source belongs to elaboration faithfulness.** "Every source
  construct produced its goal" is a statement about the elaborator, closed by 5.8's mechanical
  evidence, not by a gate condition that would have to re-derive source from kernel terms.


**Claim shape (R1; refusal only).** The gate splits each `claim` goal's statement into
`Π Γ. C`. This is the rule by which the gate classifies binders (I2) and it closes the
conclusion-type hiding of finding F6. It is enforced **by refusal only**: it never exempts
anything.

- `C` must be `Eq U l r`, where `U` is a closed type with a whitelisted data head: `Bool`,
  `Nat`, `Int`, `List`, `Option`, `Prod`, `Pif.Result`, or a model-declared inductive. This is
  the **only use of a data whitelist**, and it prevents a hypothesis hiding inside the
  conclusion's type (for example `Eq Prop (h → Q) True`).
- For each binder in `Γ`, the kernel infers the sort of its type:
  - `Sort 0`: a **hypothesis**. It must be a non-dependent `Eq Bool p true`, otherwise refuse.
  - `Sort 1`: a **variable** of any type. No data check is made, because inhabitation is
    covered by G1.
  - Any other sort, or an inference error: refuse.

| Condition | Statement |
|---|---|
| **`beq` specification** | for every constant **found by the reserved name `…beq` only** (its name ends in `beq`; discovery never looks at how a constant is defined), whose type must be `T → T → Bool`, and for every derived `beq_T`: `beq_T a b = true ↔ a = b` (`<module>.<T>.beq_spec`), **emitted as an `And` of the two implications** `And (beq_T a b = true → a = b) (a = b → beq_T a b = true)`: the PIF prelude has **no `Iff`** (`Iff` appears only through the `iff` role of an axioms file, VER32 3.5). The same holds for every `↔` in this section and in 4.4. **A goal uses `Eq T` wherever the claim is a top-level `==`**, and `beq_T` only nested inside a Bool expression |
| **Helper specifications** | `Adl.Int.{beq, le, lt}`, `Adl.Nat.{beq, ble, blt}`, `Adl.Option.isSome`, `fdiv`, `get?` and each other helper against its stated specification, in its prelude layer. **Deferred from `adl.core` to ADL-008a** (the first layer ships the definitions and the specifications it can prove at once): `Int` order against `add`, `add` commutative and associative, `sub a a = 0`, `Nat.sub` against `add`, and `beq_spec` for `Option`, `Result` and `Prod` |
| **Non-vacuity (i)** | for **every `claim` goal with a binder**, `(Π Γ. False) → False`: the telescope of the goal is inhabited. This replaces the earlier `Exists x, hypothesis x = true`, which needed the gate to find the hypothesis inside the goal (a detector; I1) |
| **Non-vacuity (ii)** | **outcome witnesses for every `Result` model in the module**: one input returning `ok` and one returning `err`, whatever goals mention the model. This replaces the per-goal wording, which let a goal with no recognisable `Result` skip the witness |
| **Run witness** | **unchanged**: for every claim with a hypothesis, one concrete run (`Eq.refl`) at closed values the goal displays, for the hypotheses and for the conclusion |
| **Exact rejection** | `rejects P with E` elaborates to the **single goal** `<module>.contract.<f>.<n>`: `P x ↔ f x = Err E` (4.4), as an `And` of both directions. A claim `forall x | P. f x == Err E` is **one-directional** and carries **no** exactness obligation. **Interim:** until `rejects` exists as a source construct (backlog ADL-008d), the gate's exact-rejection check is a **lint, not a guarantee**: it can report a missing converse, but its silence proves nothing (G5 holds only for `contract` goals that are present) |
| **Totality** `.total`, `.mono` | **Moved to "not yet a gate condition"** (backlog ADL-008e). 5.5 still says what the goals are; the gate does not yet require them. Until it does, a `measure` model is covered only by 5.5's refusal rule, not by this list |

**Configuration.**

- `--profile adl|plain` is **required whenever goals exist**, and is **recorded in the sign-off
  together with `rules sha256`** (VER32 3.9), so a signature covers the profile that was run.
- The rules file `pif-rules 2` **pins the prelude and ADL layer digests** (VER32 3.9).
- **The gate refuses to build** with any constant (`Eq`, `And`, `False`, `Exists`, `Bool`,
  `Pif.Result` and its constructors and recursor) that is **not declared in a pinned file**,
  so a base file cannot shadow a constant the obligations are written in terms of.

---

## 6. Proofs and the solver (rungs A1, A2)

### 6.1 Automation first (A2)

For each goal the producers are tried in order, all untrusted, all emitting PIF:

| Producer | Handles | Source |
|---|---|---|
| **Case split** | finite inductives, `Option`/`Result`/`Bool` trees, bounded-depth splits on `Nat`/`Int` with a symbolic remainder, `False.rec` on contradictory hypotheses, `Eq.refl` leaves | VER32 PIF-006 |
| **Induction and rewriting term builder** | `Nat.rec`/`List.rec` induction, `Eq.rec`/`congrArg`: "for every width" theorems, fuel lemmas | VER32 PIF-018 |
| **Linear arithmetic certificates** | bounds over `Nat`, per-call-site measure obligations | VER11a KRN-012a |
| **SAT/SMT with certificates** | finite bit-level facts, bounded arithmetic, through the solver (6.3) and proof reconstruction | KRN-009, VER34 |

### 6.2 Escape hatch, and failing closed

A goal no producer closes can be proved **in Lean**, but the hatch is **not free** (finding
17). A Lean proof about a model whose chain is `PIF prelude, ADL layers, model` must be
elaborated against **exactly those definitions**. Lean's automation emits terms over `Init`
lemmas that are absent, and VER32 6.4 says the native prelude and `Init` are not
interchangeable. So the hatch needs **one of**:

1. **`Init` in the base chain** (the **`Init` tier, K2**, 4.2: not K1; a different base digest,
   so the **model is re-elaborated and re-signed**); or
2. **PIF-010's PIF-to-Lean import as a hard dependency**, so Lean elaborates against the real
   chain, **plus a closure check on the resulting proof term** (every constant it mentions is
   in the chain or declaration-identical to a chain entry).

The hatch is the only user of the `Init` tier (ADL-060b); K1 needs none of it. Until one is
scheduled, the hatch is available only for goals whose proofs mention only chain
constants (checked by closure). **Fail closed**: a claim without a checked proof keeps its
lower rung. A refuted claim goes through VER27's finding protocol.

### 6.3 The solver: how Warrant uses it (A1, and VC discharge for A2/A3)

The repository's Constraint VM ([LANG24](LANG24-constraint-vm.md): `constraint-core`,
`constraint-instructions`, `constraint-engine`, `constraint-vm`, with `cdcl-sat` and
`smt-lib-format`) is being grown into an SMT solver for Warrant. **VER34 designs it**;
this document states what Warrant asks of it and the rule that bounds its authority.

**Evidence kinds.** The solver yields three kinds, mapped to the VER31 ladder as VER34 6.6 and
6.7 fix it. **The scheme names are `drat:sha256:` and `pif:sha256:`, and the evidence kind
`solver-attested` is capped at `tested`** (the amendment text is owned by VER34's CVM-170b,
which ADL-006b cites). **`k` is signed on `pif:` (it is part of the goal's statement) and
record-bound on `drat:`** (a certificate record binds `(VC digest, k, CNF digest, DRAT
digest)`, VER34 6.7):

| Evidence | Claim rung |
|---|---|
| Executed tests, **including solver-generated boundary vectors** | `tested` |
| UNSAT **plus a checked certificate** (scheme `drat:sha256:` checked by `drat-check`, or `pif:sha256:` checked by the kernel), `k` inside the signed VC text | `bounded(k)` |
| UNSAT, **no certificate** | **`solver-attested`**: printed in the report, **capped at `tested`**, never accepted as `proof.certificate` |
| SAT (a model) | confirmed by evaluation; **refutes** the claim |

(LRAT alone is not a VER31 scheme; LRAT is reconstructed into PIF, 6.1.)

**The DRAT route's rules (VER34 6.7), which this document cites rather than restates.**
Until the route is retired: (1) the evidence strategy is `blast, sat` only, **formula-level
passes off**; (2) a **certificate record** binds the VC, `k`, the CNF and the DRAT proof, and
the gate recomputes the CNF digest from the VC; (3) the **non-vacuity** check (a SAT witness
for the preconditions at size exactly `k`, and the UNSAT core must contain the negated
conclusion); (4) the ids below are listed with the claim; (5) **DRAT stops being rung
evidence for new claims once CVM-170a exists**; (6) the **sunset**: every existing DRAT-backed
`bounded(k)` claim is re-evidenced through PIF by the release after the CVM-174 report, or
drops to `tested` (CVM-159a).

**Three uses.**

| Use | Rung | What Warrant asks | What is done with the answer |
|---|---|---|---|
| **Input generation** | A1 | For each input contract and precondition: a model of `P(x)`, of `not P(x)`, and models **on the boundary** (`x` minimal or maximal in `P`, `x = c` and `c ± 1` per constant, lengths 0, 1, limit, limit+1); inputs reaching each branch of the model | Portable vectors for **every lane**. Rung **`tested`**. Labelled `boundary`, never `exhaustive` |
| **Bounded model-versus-code check** | A1 | "Find `x` of size at most `k` with `code(x) != model(x)`" for **the one lane whose WIR was symbolically executed** (Rust today; `warrant-ir` has `sym.rs`) | **`bounded(k)`** only with a checked certificate, only for that lane. **Other lanes get test vectors, which are `tested`** (VER31 6.2 caps lane packages at `tested`) |
| **VC discharge** | A2, A3 | Bit-level and arithmetic goals of sections 5 to 8 | UNSAT plus a certificate **reconstructed into a kernel proof term**; only the PIF counts |

**What `k` measures** (the VC text states it per type; finding 5): for `Int`/`Nat` and
fixed-width types, **bit length** (`|x| < 2^k`); for `Bytes` and `List`, **element count**;
for records and tuples, **componentwise**; for recursive types, **node count**. VER31 orders
`bounded(N)` by size, and these are sizes, not magnitudes.

**The rules (D8).**

- The solver is untrusted. A bare UNSAT **never** moves a rung above `tested`.
- A `bounded(k)` claim opens **`tcb.bitblast.encoding-correct`**, **`tcb.symexec.engine-correct`**
  and **`tcb.vcgen.claim-faithful`** (the claim-to-VC generator, owned by ADL-013), and, **while
  the DRAT route lasts, `tcb.drat.checker-correct`** ((c); VER31 4 force-adds the first two), because DRAT certifies that a
  CNF is unsatisfiable, **not that the CNF encodes the question**. The earlier "a wrong
  encoding can never give an accepted proof" holds only for **kernel-reconstructed** proofs;
  it is **false for `bounded`**, which is not kernel-reconstructed.
- **Non-vacuity check for every UNSAT used as evidence**: the constraints **without** the
  negated goal must be **SAT**, with the model **confirmed by evaluation**, so an
  over-constrained encoding cannot give a valid but vacuous UNSAT.
- A SAT answer is a *candidate* counterexample, **confirmed by evaluation** of the model and
  the code before anything is reported; a non-reproducing model is a solver or encoding
  defect, never a result.
- A confirmed counterexample becomes a **permanent regression example, stored in the corpus
  (the generated vector store), not in the `.warrant` file** (finding 22): editing the
  `.warrant` file changes the digest VER31's sign-off binds and breaks LG02's byte-identity
  guard. Provenance (seed, solver version) is recorded with it.

**Lists in the solver (VER34 request 2).** Warrant `List` and `Bytes` are modelled as
**arrays plus a length**, not as datatypes. Reason: arrays land in VER34's milestone M2 and
datatypes in M3 (CVM-M3); bounded checks fix a maximum length anyway; and WIR's memory is
array-shaped, so the symbolic-execution encoding is natural. **Datatype encodings** are
adopted later for ADT-typed models (trees, variants), at M3.

**Prerequisites from VER34** (cited in the backlog rows): minimisation and maximisation for
boundary points (CVM-020 to CVM-022), seeded determinism so vectors reproduce byte for byte
(CVM-023), proof-producing and certificate reconstruction into `pif:` (**CVM-170a**, with
**CVM-170b** owning the VER31 amendment text, and CVM-173), and the SMT library in `Pif.*`
bit-vectors (VER34 6.8), which sits on the **proof side** after the signed base, never in it (5.1).

**A WIR-to-SMT encoder and a claim-to-VC generator are Warrant's, not VER34's** (backlog
ADL-012a to ADL-014). Their meaning is what the two `tcb.*` ids name.

---

## 7. Forward: generating code (rung A4)

### 7.1 What goes in and what comes out

```
 checked Warrant AST ──► (a) lane backends ──► idiomatic lane source + generated tests     (A4)
          │
          └────────────► (b) WIR compiler (optional, P2) ──► WIR + certificate ──► cross-check oracle
```

Both read the **same checked AST** and the structured, typed model IR that VER04r already
gives every lane. The brief said lane backends read WIR; this spec decides they read the
**structured IR** (Q9, Fable agrees): WIR is a control-flow graph and relooping it makes
unidiomatic text. Consequence, stated plainly (finding 11): **nothing the forward path proves
is about shipped text.** The reverse check (8) is the only link, and it judges the final text
whatever made it.

### 7.2 The WIR compiler is optional and is not evidence about shipped code

An earlier draft made forward *per-function translation validation* the centre of A4 and put
`Refines(WIR, f)` on the gates' critical path. That checks a program **nobody ships and nothing
consumes**: the reverse path proves `Refines(WIR', f)` for the *lifted* program. So:

- The compiler is **P2**, off the gates' critical path.
- **"Validated" is dropped from A4.** A4 without a lifter is `tested`, full stop (D11).
- Its remaining uses: (i) an **oracle**: a WIR program compiled from the model, run on the WIR
  interpreter and on the kernel, is a three-way differential against the evaluator; (ii) a
  **reference** a lifter may reuse later, if a cheap simulation `WIR' ~ WIR` is specified
  (VER33b) to reuse the forward certificate; (iii) the symbolic-execution input for the
  bounded check on code that has no lane front end.

Its lowering table, certificate and loop templates (below) are kept, **marked optional**.

| Model construct | WIR |
|---|---|
| scalar parameters and results (`Bool`, `Un`, `Sn`) | WIR scalars; `Int` only with a range fact (4.2) |
| `if`, flat `match` on scalar-encoded sums | blocks and conditional branches |
| `Option`/`Result` of scalars, error enums | the **ADL-WIR encoding** (VER33b, `adl.wir-abi.faithful`); a panic-normalised error is a `Trap` with the reason tag of 8.2 |
| bounded loop, structural recursion on a list | a counted loop by the templates below |
| fixed-width `+ - *` | `Wrap`; checked and trap per 4.3 |
| anything else | **refused**, listed |

**Loop templates** (each a fixed pattern; unmatched shapes are refused). **T1 accumulate**
(`fold(range(n), init, step)`): after `i` iterations `acc = foldRange i init step`, step by
delta/iota since `foldRange` is `Nat.rec`. **T2 output-building zip/map** (structural
recursion over two lists): after `i` steps the output prefix and carry equal the model's.
**T3 search**: invariant "no match among the first `i`", variant `len - i`. Trees (T4) need an
explicit stack and are out of the first slice. The ripple-carry adder (10.6) is T2.

### 7.3 Backends

A backend (untrusted) reads the structured IR and the lane's **style profile** and writes
source text, a generated test file, packaging and a provenance header. The repository's
per-lane formatter, where one exists, is run last. Each backend's output is exercised by A0
immediately: the **same Warrant file's generated tests run against the generated code**.

### 7.4 Idiomatic output, per lane

The owner wants generated code that reads as if a person of that language wrote it. Each row
is a **style profile** consumed by the backend, reviewed with that lane's first backend.

| Concern | What the profile fixes |
|---|---|
| **Naming** | `snake_case` functions in Rust, Python, Ruby, Elixir, Lua, Perl, OCaml, C, C++; `camelCase` in Java, Kotlin, TypeScript, Dart, Swift, Haskell, F#; exported `PascalCase` in Go and C# public members. The Warrant names stay in the provenance comment |
| **Errors as values vs panics** | Decided by the binding's `error_normalization` (4.3, 4.4), not by taste. Value style: Rust `Result`, Go `(T, error)`, Haskell `Either`/`Maybe`, OCaml and F# `result`/`option`, Elixir `{:ok, v}`/`{:error, e}`, Swift `throws` or `Result`, Kotlin `Result` or a sealed class, Java a sealed result or an exception where the contract says panic, C an out-parameter with status, C++ `std::expected`-style or exception. Panic style (the gates' LG01 contract): the lane's native panic or exception with the declared message identity |
| **Optional values** | the lane's own `Option`, nullable with an explicit check, `Maybe`, `option`, `nil` plus ok-flag |
| **Control flow** | `match`/`switch`/pattern matching where the lane has them, else a conditional chain; `fold` over `range` becomes the lane's loop |
| **Arithmetic** | wrapping and range tests **spelled explicitly** so no result depends on a build profile (4.3, 8.1) |
| **Packaging** | the repository's per-lane layout: manifest, `BUILD`, `README.md`, `CHANGELOG.d/`, tests, coverage thresholds (CLAUDE.md) |
| **Literate comments** | the Warrant doc comments, the model text quoted, truth tables and worked examples (CLAUDE.md rule 14); a "generated, do not edit, regenerate with" header with the source and model digests |

**How idiomatic coexists with validation.** The two meet at the **lifter's subset** (8.3). A
backend may emit only constructs the lane's lifter accepts and the lane's subset semantics
defines. Idioms outside the subset (iterator chains, closures and higher-order helpers,
generics-heavy abstractions, macros, exceptions as control flow, reflection) are **not
emitted** until the subset grows. So: (1) the backend emits idiomatic text *within* the
subset; (2) a formatter may reformat it, and validation is on the **final text** of the
**whole unit** (8.1), so formatting is not trusted; (3) the lifter maps it back to WIR and the
kernel checks the refinement (8.7). A lane with no lifter gets the same text at `tested` with
`<lane>.backend.code-link` open. **Widening idioms widens a lane's reviewed semantics**; the
default starts with the plainest idiom a person would write.

**Staleness (finding 3).** A hand-edited file keeps its "generated, do not edit" header. So **CI
regenerates every generated package and requires a byte-identical diff** (as VER31 F10 does
for evidence); a generated file that differs from regeneration is rejected, and the provenance
digests are inputs to the check.

### 7.5 Packaging, independence, and replacement

Generated code is **additive** by default: a separate module or package (for example
`rust/logic-gates-adl`, never overwriting `rust/logic-gates`), with its own tests and the
differential against the hand-written lane **over the full input domain**. In order of use:

1. an **oracle**: a second implementation, **independent of the hand-written lane only**. The
   generated code and its tests derive from **one model**, and the gate models were written
   from the Rust source, so a shared misreading is shared (finding 23);
2. a **differential**: hand-written against generated, exhaustively where the domain is small;
3. **a lane that does not exist yet**: C and C++ have no gates package. There the generated
   code *is* the lane, and the report says **"generated" and `tested`**, not "independently
   verified";
4. a **replacement** of a hand-written lane, only by owner decision (Q1).

A hand-written package is never regenerated against its will; A3 (lifting) exists so that it
need not be.

---

## 8. Reverse: lifting code to the kernel (rung A3; also raises A4)

### 8.0 Two levels, named (finding 13)

| Level | What the goal contains | Open ids | Needs |
|---|---|---|---|
| **A3-WIR** | The WIR program produced by the **existing untrusted front end** (Rust today, VER25a/b), by value; the link from source to WIR is **not** checked | `wir.lowering.faithful` (c) and the other `wir.*` ids stay open; the front end replaces `gates.model.code-link` by them, it does not close them | PIF-019 (straight-line WIR semantics), no `lits` |
| **A3-source** | The **source text itself** as a string whose digest is the file's, parsed by a kernel-side grammar interpreter (VER11a 14.3), with the lane semantics | `src.<lane>.lean-semantics.faithful` (c), `src.<lane>.subset-conformance` (c) | **K1**: a string literal in a goal needs `lits` and the string roles (VER32 3.5 `LiteralBeforeRole`); the lane semantics; PIF-009 |

G-M2 is **A3-WIR**. Closing a backend id at A3-WIR *trades* `<lane>.backend.code-link` for
`wir.lowering.faithful`; it does not close it. **Every A3-source row depends on K1 and
PIF-009**, so the lifter rows in the backlog (ADL-031 and the per-lane rows) carry that
dependency. *(Follow-up 2026-11-24: K1 no longer imports `Init`, 4.2. The A3-source
rows keep PIF-009 conservatively until ADL-030 decides whether the lane semantics need
`Init`; if they do not, PIF-009 drops off those rows and stays only on ADL-060b.)*

### 8.1 The pipeline, and what the goal covers

```
shipped unit ──► lane front end (grammar-driven, code/grammars/<lang>) ──► typed lane AST
             ──► lifter (untrusted): lane AST → WIR + simulation hints + certificate
             ──► kernel checks (a) lane-semantics(unit) ≈ WIR (A3-source) and (b) Refines WIR f
```

**The goal covers the whole compilation unit, not a function's text** (finding 3):

1. **The whole file or package unit.** The kernel-side parser reads **the entire unit**, and
   **either rejects it or skips an item only through an explicit opaque-item form** that is
   admissible only where the semantics shows the item **cannot affect any lifted function**:
   **closed name resolution, no local macros, no `cfg`**. A skipped `macro_rules!`, `use`,
   `#[cfg]` or C `#define` changes meaning, so each is **refused, not skipped**. The real
   `gates.rs` has `and_n(&[u8])`, test modules, attributes and format-string panics; the
   Rust subset must **admit or explicitly wall off** each before A3 exists for it, and says so
   (finding 29: a `&str` parameter and format-string `assert!` are subset additions).
   **Opaque items are declarations only**: no **load-time effects** (Python, Ruby and
   JavaScript top-level statements, crate inner attributes, C `#pragma`). **Export resolution
   includes the package-level unit** (crate root, manifest, `package.json`). VER33a carries
   these rules, and **the whole-unit rule applies to A3-source only**; the link-status field
   says so (A3-WIR checks no source text).
2. **The export map is part of the kernel statement.** `"export": "nand_gate"` is resolved
   **by the kernel's semantics** (modules, `pub use`, `package.json` `exports`, a rebinding in
   `__init__.py`), never by the untrusted lifter. `pub use evil::nand_gate` therefore makes
   the *exported* function `evil`'s, and the goal fails.
3. **Build profiles.** C's `assert` vanishes under `-DNDEBUG`; a bare Rust `+` panics in debug
   and wraps in release. The lane semantics **quantifies over every supported build profile**,
   or the subset **refuses profile-dependent constructs**. The default is refusal: arithmetic
   is spelled explicitly (4.3) and validation is never a `debug_assert!` (`gates.rs` itself
   records that validation was once a `debug_assert!` that compiled away).
4. **Staleness**: generated packages are regenerated byte-identically in CI (7.4).

### 8.2 The equivalence statement

For a lane function `h` and the model `f` it realizes (4.8):

```
outcome : ⟦R⟧ → Status                         -- total, in the reviewed ABI file (VER33b)
outcome (ok v)  = ret (enc v)                  -- enc is PARTIAL: v unrepresentable = failure, never truncation
outcome (err e) = trap (tag e)                 -- tag e: the reason tag of a message in binding.panic_messages

Equiv h f := ∀ args ∈ Dom(h),                  -- Dom(h) = EVERY value of h's declared parameter types
               ∃ n, let s = run n unit (call h args) in
                    s.status = outcome (f (embed args))  ∧  s.status ≠ stuck
```

- **`Dom(h)`** is read from the source, not the binding (4.8).
- **`outcome` is one total function**, covering `Result`, `Option`, `Bool`→`Int` and nested
  results by the ABI file's clauses, replacing the earlier `Result`-only sketch.
- **The tag set is in the signed `Equiv` goal.** Its entries are **exact format templates**,
  resolved by the kernel semantics to **panic sites in the unit**; **empty or wildcard entries
  are refused**, and **compiler-generated panics (overflow, index, `unwrap`) can never be
  listed**. The outcome is stated as **membership**: the status is a trap **whose tag is in the
  set** (`outcome (err e)` is the set of admissible traps for `e`, and `Equiv` says the run's
  status lies in it).
- **Trap identity.** VER11a's `obs` forgets trap *reason text* for constant-time observation.
  `Equiv` does **not**: it keeps the **tag**, drawn from the binding's finite
  `panic_messages` set. An unrelated panic (`unwrap`, index, debug overflow) has a **different
  tag**, so it fails `Equiv`; the two `validate_bit` messages are both in the declared set, so
  the hand-written gates pass. A lane whose rejection has several distinguishable identities
  is only as precise as the model's error type.
- **What it does not say.** `Equiv` says nothing about steps, heap or the trace. **Bound and
  constant-time claims need their own goals over the lifted program** (VER11a 8, 9.3).
- **Witnesses are mandatory** (5.10): a domain witness, one witness per outcome, one concrete
  run, so an empty `Dom(h)` or an unsatisfiable guard cannot produce a contentless `Equiv`.

The statement is the **same for hand-written and generated** code.

### 8.3 Two lift targets

| Target | For | Idea | Needs |
|---|---|---|---|
| **To WIR (default)** | imperative lanes: Rust, C, C++, Go, Java, Kotlin, C#, Swift, Dart; dynamic lanes later | Lower to WIR, reuse the lockstep and obligation machinery (VER11a 8, 9), validate the lowering per function | the lane subset semantics, a hint generator |
| **Direct, to a kernel definition** | pure functional lanes: Haskell, OCaml, F#, Elixir (function subset) | The lifter produces a kernel definition from the lane text and a producer proves it equal to the model | a pure-subset semantics, **no heap, loops or WIR** |

WIR stays the default because it keeps one machine under 18 lanes (VER11a 14.4). The direct
target is a per-lane decision (Q3).

**One kernel-side grammar interpreter (finding 31).** Eighteen "parsers defined in the kernel"
would be eighteen hand-written parsers in another language, against the owner's rule that
parsers come from `code/grammars/<lang>`. The decision recorded here for VER33a (ADL-030) to
confirm: **one grammar interpreter, written once as kernel definitions and reviewed once**, with
**each lane's grammar passed as data** (its digest in the goal, generated from the lane's
`.tokens`/`.grammar` files). The per-lane meaning surface shrinks to the lane's *semantics*;
the parser is shared. Its cost and the KRN-016 performance risk (reading whole literate files by
kernel reduction) are measured on Rust first.

### 8.4 The per-lane cost, stated honestly

Each lane needs **a reviewed kernel semantics of its subset** *and the machinery around it*.
The count that earlier drafts gave (function, integers, comparison, `if`, call, `return`, panic)
omits:

- the **grammar data** and the parse of **whole literate files** by kernel reduction;
- **export and module semantics** (8.1);
- the **build-profile matrix** (8.1) and the **toolchain version pin**;
- a **differential harness against the real toolchain** (the oracle for the semantics);
- the integer and overflow semantics, **which differ between Java, Kotlin and C#** even where
  their object models look alike, so they do **not** share one semantics;
- for dynamic lanes, the type-confusion behaviour (TypeScript `number` doubles reject `-0`
  and non-integers; Python rejects `bool`, `int` subclasses and floats).

KRN-S0 and VER11a 14.3 count "about fifteen" constructs for ct-compare's Rust subset;
KRN-015/016 are the calibration. **Every estimate here is an estimate**: a gates-only subset per
static lane is perhaps a third of the Rust construct count *for the semantics file alone*, and
the surrounding machinery is a comparable amount again; a general algorithm subset is several
times more; a dynamic lane adds a type system's worth. It is the dominant long-term cost of the
programme and the reason A0 to A2 are first-class. **The second lane is chosen after KRN-015/016
report measured cost and K1 is scheduled** (Q3).

**Per-lane order** (recommendation; Fable agrees on Rust first and declines to commit further):

| Order | Lane(s) | Reason |
|---|---|---|
| 1 | **Rust** | Front end exists; VER25c and KRN-015/016 are queued; gives the calibration |
| 2 | decided after the measurement; candidates **Go** (static, panics only) and **OCaml** (the direct target) | needs measured cost and the K1 dependency |
| later | Java, Kotlin, C# (separate semantics each), Swift, Dart, C, C++ (undefined behaviour and memory) | as above |
| last | TypeScript, Python, Ruby, Lua, Perl, Elixir (dynamic) | type-confusion inside the semantics; **may stay at A0/A1 indefinitely**; **not early** |

### 8.5 Fail closed

A lifter refuses anything outside the subset, and **a refusal is not a release error**: the
package keeps A0, A1 and A2, and the report says "not lifted: construct X outside subset Y of
lane Z". A lifter never guesses, widens, rewrites or approximates source. A function that lifts
but whose certificate fails is reported as unlinked, with the failing obligation.

### 8.6 Lanes without a lifter

They keep A0, A1 (vectors need no lane tooling) and A2 (the model is proved regardless of lane).
The report states per lane which rung is reached. A backend without a lifter ships at `tested`
with `<lane>.backend.code-link` open.

### 8.7 Lifting generated code raises the backend, and sabotage controls

For a lane with both a backend and a lifter, the build runs `lift(generate(model))` and checks
`Equiv`. **At A3-source** a pass closes `<lane>.backend.code-link`; **at A3-WIR** it trades it for
`wir.lowering.faithful` (8.0). A failure is a backend bug (the text is not the definition) or a
lifter limit (the text is outside the subset), and both fail closed.

**Sabotage controls** (VER11a 12.5), mandatory for the new producers: **a deliberately wrong
backend output** (a swapped branch, a dropped rejection, a widened domain, an exported
different function) **must fail the reverse check**, and a deliberately wrong certificate must
fail the kernel. A producer without its sabotage corpus is not accepted.

---

## 9. Assumptions and VER31 rungs

### 9.1 Ids the pipeline opens

Buckets are VER31's. Ids marked *new* are registered (VER13, owner-signed bucket file) by the
ADL-006 rows.

| Id | Bucket | Opened by | Closes by |
|---|---|---|---|
| `kernel.checker.correct` | (c) | any kernel-checked claim | independent agreement (VER11a Q6, KRN-019) |
| `kernel.logic.consistent` | (c) open until decided (proposed (a)) | any kernel-checked claim | owner decision (VER11a Q5) |
| `pif.reader.correct`, `tcb.pif.gate-binding`, **`tcb.pif.printer`**, `tcb.report.checker-correct` | (c) | A2 | VER32 routes; the printer id covers goals **and** model files (`parse(print(F)) = F`) |
| `pif.prelude.faithful` | (b) | A2 | signature on the PIF prelude digest |
| **`adl.prelude.faithful`** *(new, per layer)* | (b) | A2 | signature on each layer digest; helpers also covered by the specification lemmas (5.10) |
| **`warrant.model.elaboration-faithful`** *(new)* | **(c)** | A2 | **mechanical evidence only**: the second elaborator, the `Eq.refl` differential with per-function coverage, and the mandatory specification lemmas (5.8). **Never by signature** |
| **`<pkg>.model.faithful`** *(new per package; `gates.model.faithful` in VER32)* | (b) | A2 | signature on the model file digest and goals list |
| `spec.goal.faithful` | (b) | A2 | signed goals list |
| **`adl.wir-abi.faithful`** *(new)* | (b) | A3 | signature on the encoding file digest |
| **`tcb.bitblast.encoding-correct`**, **`tcb.symexec.engine-correct`**, **`tcb.vcgen.claim-faithful`** *(new; the claim-to-VC generator, ADL-013)* | (c) | **A1 `bounded(k)`** | VER34 / VER31 routes (kernel-reconstructed proofs close the first two; the generator's id is registered by ADL-006a and owned by ADL-013, with VER32 PIF-016d) |
| **`tcb.drat.checker-correct`** | (c) | **A1 `bounded(k)` on the `drat:` route only, while it lasts** (VER34 6.7) | retiring the route: re-evidence through `pif:` (CVM-159a) |
| `wir.semantics.lean-faithful`, `wir.lowering.faithful`, `wir.typing.checker-guarantees`, `wir.interp.differential-only` | (c) | A3-WIR | VER11a K-M2, KRN-006/007 |
| **`src.<lane>.lean-semantics.faithful`** *(new per lane)* | (c) | A3-source | **VER11a Q3 decides its bucket once per lane**; review and differential tests are evidence, not a discharge (VER31 3.3) |
| **`src.<lane>.subset-conformance`** | (c) | A3-source | closed by the kernel-defined parser refusing out-of-subset source, **true only with the whole-unit and opaque-item rules of 8.1** |
| **`<lane>.backend.code-link`** *(new)* | (c) | A4 | the reverse check of 8.7 (A3-source closes it; A3-WIR trades it for `wir.lowering.faithful`) |
| `tcb.<lane>-toolchain.trusted` and the platform ids | **(a)** | A3, A4 | accepted baseline |
| `<pkg>.model.code-link` (VER32: `gates.model.code-link`) | (c) | A2 only | closed by A3, replaced by the `wir.*` and `src.*` ids |
| solver | none | A1, VC discharge | not an assumption **for kernel-reconstructed proofs**; for `bounded(k)` see the two `tcb.*` ids above |

**The SMT library has no id**: it is proof plumbing on the proof side (2.2, VER34 6.8), and its
faithfulness to the signed meaning is a bridge theorem. The WIR-to-SMT **encoder** is registered
under **ADL-012a/b** (the scalar and the array encoder), so the two `tcb.*` ids it relies on have
bucket entries and an owner before any `bounded(k)` claim can cite them.

The compiler, backends, lifters and producers have **no ids of their own**; their failures
surface as rejected terms, failed validations or unreproduced counterexamples.

### 9.2 What rung a package claim reaches at each stage

| Stage | Package claim rung (VER31 section 4) | Model theorem |
|---|---|---|
| Today (A0) | `tested`; `exhaustive` once `domain: full` is emitted (Warrant domain only, 1.3) | none |
| A1 | `tested`; `bounded(k)` **only** for the symbolically executed lane, with a checked certificate | none |
| A2 | **unchanged**: the transported goal alone is a **model theorem**; a package claim rises only through its own signed goal **and** `Equiv` for every `op` it mentions (1.4) | **model-checked** (VER32 Q8), relative to the model-to-code ids |
| A3-WIR (some lanes) | `checked`, with `wir.lowering.faithful` open | model-checked, now code-linked at WIR level |
| A3-source | `checked`; **`proved` is not reachable** until VER11a Q3 is decided for the lane, KRN-019 closes `kernel.checker.correct`, and the VER31 §6.2 amendment (13) lifts the `tested` cap on lane packages | as above |
| A4 generated lane | **`tested` and no more**, until the reverse check covers the text | as above |

**State plainly: `proved` for any lane is blocked on VER11a Q3, once per lane.** The row
"A3 complete, signed → `proved`" of the earlier draft was not reachable and the spec did not
say so.

**VER31 amendments needed** (section 13): the **model-theorem category** and **link status**
(VER32 Q8, PIF-020); the **§6.2 amendment** (lane packages are capped at `tested` today);
`solver-attested` as an evidence kind (VER34 6.6, **text owned by CVM-170b**); and the gate's computation of
the A-rung (1.5). VER32 finding 14 is why a model theorem cannot reuse `checked`: `checked`
needs rung 1 as a floor and a model theorem has no tests.

---

## 10. The first adopter: logic gates, end to end

### 10.1 The rungs for the gates, in order

| Step | Rung | What |
|---|---|---|
| (0) | A0, today | The LG02 corpus drives exhaustive truth-table and rejection tests in every lane (the audit gaps remain) |
| (a) | A2 prep | Gate models in the algorithm profile (10.2) |
| (b) | **A2, G-M1** | VER32's model-level theorems over the models. **G-M1 waits on ADL-002** (VER32 9.2) |
| (c) | A4, G-M4 | Generate the 11 gates for every lane from the model (`tested`) |
| (d) | **A3-WIR, G-M2** | Lift the hand-written Rust gates at WIR level and check against the model |
| (e) | A2/A3 | Adders (G-M3), then ct-compare |

### 10.2 (a) The models

The gates exist as `op`s; their definition does not. **ADL-002** adds
`code/specs/logic-gates-model.warrant` (algorithm profile, 5.9), a separate file so the LG02
corpus files and their byte-identity drift guard stay untouched. **It needs the `profile
algorithm` header and the `realizes` schema from ADL-001a** (earlier draft: "existing syntax
only", which was wrong). It defines `to_bit`, `from_bit`, the seven primitive gates over `Int`,
the four NAND constructions as `model`s, `ref_g` specification functions, and states the
**26** LG02 claims as **model theorems**. **Naming (decided with ADL-002, 2026-11-24):** the 26 keep
**LG02's names, stated over `Bool`** as the Warrant file states them; the **`Int` restatements are
separate claims** `<name>_over_int` and the checks against the specification function are
`<name>_matches_ref` (so an `Int` statement is never mistaken for the `Bool` one the Warrant file
signs, and no existing id is renamed). A guard compares the oracle rows with the
LG02 corpus. The existing `op`s get `realizes` data in their binding file, one per gate.

**These model claims are not the package claims** (1.4). Each package claim over `U8` gets its
own transport-rule goal; the model theorems and `Equiv` are lemmas in its proof.

### 10.3 (b) G-M1 on the models

VER32 9.2 and 9.5 apply verbatim: the model file and goals list from the ADL elaborator
(PIF-017 = ADL-003), T1 to T5. Mitigation (2) of 5.8 (the `Eq.refl` differential) and the
specification lemmas (5.10) are **gate conditions for G-M1's signature** (ADL-005 rows,
ADL-008a/b, are G-M1's prerequisites; finding 27). The honest note of VER32 stands: T1 and T4
restate `to_bit`; only T3 and T2 against LG02 carry content.

### 10.4 (c) Generate all eleven gates (G-M4)

Per lane, in the order of 8.4 (backends are independent of lifters): the backend writes the 11
gates in the lane's idiom (7.4), with LG01's panic-or-exception contract, the LG02 corpus tests
and the package files. A0 runs at once. The generated Rust is a pure bit-level function and does
not call the CMOS model. **Rung: `tested`** (D11).

### 10.5 (d) Lift Rust back (G-M2), and the transistor path

The hand-written Rust primitive gates route through the floating-point CMOS model
(`CMOSNand::evaluate_digital(a, b).unwrap()`), outside the Rust subset (VER32 9.3). So the model is
**bit-level** and the CMOS path is separate:

- **Model versus hand-written Rust** is G-M2, an **A3-WIR** goal, under VER32's Q2 options. Option
  A: the four NAND-built gates, with `nand_gate` a stated (c) call contract
  (`gates.nand.primitive-contract`); **7 primitives not covered**. Option D: an integer switch-level
  `evaluate_digital` path lowerable to WIR, covering all 11. Both are `Equiv` goals against the same
  model, with `Dom(h)` read from the Rust source's `u8` parameters.
- **Generated Rust versus hand-written Rust** is a differential over all 65,536 input pairs, at A0.
- The generated gates being bit-level is why they are **not** a replacement by default (Q1): the
  library's identity includes its transistor routing.

### 10.6 (e) Adders, then ct-compare

The **ripple-carry adder** is the first recursive algorithm. A sketch (not run through the checker;
names illustrative; flat matches only, 5.4):

```text
  rec model add_bits(xs: List<Bool>, ys: List<Bool>, c: Bool)
      -> Option<(List<Bool>, Bool)> decreasing xs =
    match xs with
      Nil         -> (match ys with Nil -> Some((Nil, c)) | Cons(y, yr) -> None)
    | Cons(x, xr) -> (match ys with
                        Nil -> None
                      | Cons(y, yr) ->
                          (match add_bits(xr, yr, carry(x, y, c)) with
                             None -> None
                           | Some(r) -> Some((Cons(sum(x, y, c), r.0), r.1))))
```

It elaborates by **primitive recursion on `xs`** (5.5): the recursive hypothesis is a function of
`ys` and `c`, which is why the carry may change per call. G-M3a proves, for **every width**,
`value(sum) + 2^n * carry = value xs + value ys + c` by induction (PIF-018). G-M3b lifts the
hand-written Rust loop with the VER11a block-invariant rule and needs allocation and memory
semantics. **ct-compare** follows (PIF-015): its Warrant file gets a model for `eq`/`select`; its
constant-time claims remain code-level (VER11a 8).

---

## 11. Backlog

The full queue is the shard [VER-BACKLOG.d/warrant-adl.md](VER-BACKLOG.d/warrant-adl.md); the
main [VER-BACKLOG.md](VER-BACKLOG.md) carries a pointer and these milestone headings. **Every
row is one PR of about 100 to 400 lines** (the owner's decomposition rule); per-lane lifter
rows (five PRs per lane) and per-lane backend rows (three PRs per lane) are in the shard.
VER33a and VER33b are named follow-up specification items (ADL-030, ADL-020), not files yet.

| Milestone | Rows | What it delivers |
|---|---|---|
| **M0** Spec and decisions | ADL-000, ADL-020, ADL-030 | round 2 review, VER33b, VER33a |
| **M1** Profile and the gate models | ADL-001a to c, ADL-002, ADL-002b, ADL-004a/b | header and realization, the gate model file, ADL prelude layers; unblocks G-M1 |
| **M2** Elaborator and evidence for its meaning | ADL-003a to c, ADL-005a to e, ADL-008a/b, G-M1 | elaborator, independent encoder, coverage, mutation, second elaborator, mandatory lemmas |
| **M3** Registration, gate, package-claim rule | ADL-006a, ADL-006b, ADL-006c1 to c3, ADL-007, ADL-009 | ids, VER31 amendments, report rung, transport rule, totality gate |
| **M4** Solver-backed testing (A1) | ADL-010, ADL-011, ADL-012a/b, ADL-013 to ADL-015 | boundary vectors, regression store, WIR-to-SMT, claim-to-VC, bounded driver, VC discharge |
| **M5** ABI, refinement, A3-WIR for Rust | ADL-021 to ADL-023, ADL-032, ADL-033, G-M2 | the Rust gates lifted at WIR level |
| **M6** Generated code (A4) | ADL-040 to ADL-047, G-M4 | backends per lane, packages, round trip, sabotage corpus, optional WIR compiler |
| **M7** Recursive algorithms | ADL-050, ADL-051, G-M3a, G-M3b | adders for every width |
| **M8** A3-source and lifters | ADL-031, ADL-035, ADL-L01 to L09, ADL-060a, ADL-060b, ADL-061 | kernel-side grammar interpreter, per-lane lifters, K1 (060a), the `Init` tier (060b) |

The dependency edges Fable's round 1 found are all explicit in the shard: ADL-002 needs ADL-001a;
G-M1 needs the differential and the lemma gate (ADL-005c, ADL-008a/b); ADL-003a needs ADL-004a's
names; ADL-009 and ADL-050/051 list WX-R1 and WX-D2; every A3-source row lists K1 and PIF-009;
ADL-010, ADL-012 to ADL-015 cite VER34's CVM-020 to CVM-023, **CVM-170a/170b** (not "CVM-170") and CVM-173; the
WIR-to-SMT encoder and the claim-to-VC generator are their own rows (ADL-012a/b, ADL-013), split
from VER34.

---

## 12. Owner questions

Each row gives the recommendation and **Fable's view** (round 1) beside it.

| # | Question | Recommendation (this revision) | Fable's view and alternatives |
|---|---|---|---|
| Q1 | Do generated lanes **replace** hand-written ones? | **No.** An oracle and a differential everywhere hand-written code exists; **the lane itself, reported "generated" and `tested`,** where none does (C, C++ gates) | **Agree.** Revisit only once the generated lane has a source-level lift and VER32 Q2 option D is settled. Alternatives: replace per lane once it lifts and passes the corpus; replace everywhere |
| Q2 | **Overflow defaults** | **Keep wrap** as the language semantics; checked is `try<T>`; trap is checked plus the binding normalisation, and `Trap` mode only with the `trap_eq_test` goal. **The signed printout names every wrapping operation explicitly** (`Adl.U8.wrapAdd`), and **the lifter subset refuses a profile-dependent bare `+`** (4.3, 8.1) | **Agree, and better than "warn where no claim covers it"**: the explicit printout and the refusal. Alternative: require `wrapping_*`/`checked_*` spellings in Warrant source |
| Q3 | **Which lanes get lifters first** | **Rust first. Decide the second lane after KRN-015/016 report measured cost and K1 is scheduled.** Candidates Go and OCaml; dynamic lanes last | **Agree on Rust first; do not commit to Go and OCaml yet;** disagrees with putting TypeScript early. Alternative: prioritise by demand |
| Q4 | Is the model-file signature **per function or per module**? | **Per module**, **conditional on** append-only prelude layers (5.2) and a **printed per-function digest diff** so a re-sign reviews only what changed | **Agree, conditional on prelude layering and the per-function diff.** Alternative: per function |
| Q5 | Where do the elaborator and compiler live? | A **new crate** over the checked AST of `rust/warrant-grammar`; **the independent value encoder for the differential is a separate crate, sharing no code with the elaborator** | **Agree**, with that condition |
| Q6 | What closes `warrant.model.elaboration-faithful`? | **Fable's view adopted (it supersedes the first draft).** The signature closes `<pkg>.model.faithful` (b), a **different proposition**. `proved` requires **all three**: the **second elaborator**, the **`Eq.refl` differential with per-function coverage**, and the **mandatory specification lemmas** (5.8) | **Disagreed with the first draft's "signature as the interim route".** This delays nothing, because `proved` is already blocked by KRN-019 and the lane-semantics ids |
| Q7 | Is the adoption rung a first-class report field? | **Yes**, computed by the gate, **never taken as declared**, always printed beside the VER31 rung; **A2 is never a package-level word containing "proved"**; link status states the **domain actually quantified** | **Agree**, with those additions |
| Q8 | Profile word or a new Warrant version number? | **Profile word** (`profile algorithm`); a profile is a restriction plus additions, and it also **tightens meaning** (exact rejection) | **Agree**, noting the tightening |
| Q9 | Backends read **structured IR** or **WIR**? | **Structured IR** (7.1); **this makes the reverse check the only link to shipped text, and the spec says so (D11)** | **Agree**, and asks that finding 11 be resolved, which 7.2 does |

### 12.1 Owner decisions, 2026-11-24

Recorded as **decided**. The recommendation text above is kept for the record.

| # | Decision | Condition or note |
|---|---|---|
| All | **Every recommendation that the drafter and Fable agreed on is accepted as a working decision**: Q1 to Q9 as recommended (Q6 as Fable's view, which superseded the first draft) | "Working decision": revisited only on new evidence from an implementation |
| Q3 | Rust first; the second lane is still decided after KRN-015/016 report and K1 is scheduled | **K1 now means native words and `lits`, with no `Init`** (4.2); the `Init` tier (K2) is only the escape hatch |
| (VER32 Q1) | VER32's **hand-written trusted reader exemption** is approved | Not a VER33 decision; recorded because 8.3's kernel-side grammar interpreter (ADL-030/031) argues from the same conflict and must still be decided on its own |
| (VER32 Q2) | VER32 **option D** (integer switch-level `evaluate_digital`) is decided | 10.5's two options collapse to option D as the route; option A survives as a relabelled first slice. G-M2 covers all 11 gates once G-M2-D lands |
| (VER32 E3/E4) | **The unsigned proof side is approved** | 5.1: the SMT library is on the proof side; the prefix rule governs only the signed base |

No new owner question arises from this follow-up in VER33.

---

## 13. Relationship to other specifications

Short dated notes the named specifications should carry (2026-11-24); **no other spec is
rewritten here.**

| Spec | Note |
|---|---|
| VER04 | Section 4: the "no recursion" rule is relaxed *only* in the algorithm profile (via VER04s `rec model`). **`measure` models are outside A0/A1 and the polyglot rule** until their totality theorem is checked; no VER04s amendment is needed (4.1) |
| VER04d | Section 5: the binding schema gains the optional `realizes` field (4.8) |
| VER04e, VER04f, VER04h | "Warrant generates tests, never implementations" is no longer the whole story; test generation is unchanged and is rung A0 |
| VER04i | The lane rollout is the A0 rung; the polyglot rule applies to A0/A1 and to backends, **not** to lifters, which are per-lane by cost (8.4) |
| VER05, VER06 | Solver-generated vectors (A1) join the corpora they run; no engine change |
| VER11a | 7.1: the claim elaborator also elaborates models; 14: an A3-source adopter is a lane-lifted ADL function; 9.5: the solver's certificates are the producer named here |
| VER25 | WIR is the lifters' target; the overflow-mode mapping of 4.3 is the contract with VER25 section 2.1 |
| VER31 | **Model-theorem category and link status** (VER32 Q8); **the §6.2 amendment: lane packages are capped at `tested` today, which blocks `proved` for every lane until VER11a Q3 is decided per lane**; `solver-attested` as an evidence kind (VER34 6.6); the gate-computed A-rung; the new ids of 9.1 |
| VER34 | The solver spec. VER33 states only the uses and the untrusted rule (6.3) |
| LEAN00 | The direct lift target (8.3) is another producer of kernel definitions, through PIF |

### 13.1 VER32 alignment (final, PR #20009, `d01ad9a9ab8`)

Aligned in this revision: the `Pif.*` namespace and `Pif.Result`/`Pif.GateError` (5.1, 5.3); the
base environment taken from the **sign-off** and the signature over the **base** digest (5.1); the
**separate goals list**, one line per claim (claim id, a space, the display CTE) (5.1); the
**`tcb.pif.printer`** id and the **file round trip** `parse(print(F)) = F` over model files (2.2,
5.1); **G-M1 waits on ADL-002**; **PIF-017 is ADL-003**; the `iff`/`nonempty` roles (VER32 3.5)
are relied on only through the kernel, with no VER33 text affected; the normative recursor layout
is the reason model-file arms are reordered into constructor order (5.4, 5.9); the 26 claims.

**Environment chain (corrected 2026-11-24).** The **signed base** is, in order: **PIF prelude, ADL
prelude layers (`adl.core`, `adl.list`, `adl.fixed`, in that fixed order), [WIR semantics],
model file(s), [axioms file]**. The **proof side** follows: **the SMT library** (`Pif.*`
bit-vectors, VER34 6.8), then the claim file. *The first revision placed the SMT library in the
signed base; VER34's confirmation pass found that this re-signs a model whenever a library
changes, and the owner approved the proof side (VER32 E7).* VER32 9.2 now reads this way.

**Status of the four requests (2026-11-24): all applied in VER32 and recorded in its "Follow-up
2026-11-24".** Their text is kept for the record.

Requests to VER32:

1. **`pif-eval` is no longer required.** The differential states vector theorems and proves them
   by `Eq.refl` (5.8), so no normaliser or decoder is needed. PIF-003 may drop it.
2. Make **link status** part of VER32 Q8's category, including **the domain quantified** (1.5),
   and let PIF-016 register VER33's ids.
3. Note in 9.6 that package claims never ride on model theorems (1.4).
4. Note that ADL prelude layers form one global append-only sequence and a model's chain is a prefix (5.2), which is VER32 E2's consequence.

---

## 14. What this spec does not promise

- That any rung above A0 exists for any package. A0 largely exists; A1 to A4 are a queue.
- That `proved` is reachable for any lane: it is **blocked** on VER11a Q3 per lane, KRN-019 and the
  VER31 §6.2 amendment (9.2).
- That a lane without a lifter ever gets one. Dynamic lanes may stay at A0/A1.
- That generated code replaces anything, that it is independently verified, or that it is the
  fastest or smallest.
- That the model is *right*: the signature says it is what a person meant, the proof says
  properties hold of it, and neither says the algorithm is the one the world needs.
- That floats, state, heap structures or concurrency are covered (4.5, 7.2).
- A design for the solver (VER34), the kernel (VER11a) or PIF (VER32).
- That the cost estimates of 8.4 are measurements.

---

## 15. Review record

**Fable, round 1** (adversarial review of the first draft, commit `acbbfe9b4fd`). Findings 1 to 5
critical, 6 to 17 major, 18 to 26 minor, 27 to 31 feasibility and backlog.

| # | Finding | Disposition |
|---|---|---|
| 1 | No rule binds a package claim to a theorem | **Fixed (1.4, 5.9, 10.2):** a package claim rises only through its own signed goal (code shape or the gate's transport rule); model theorems and `Equiv` are lemmas; K0 covers `U8` ranges to 255 |
| 2 | `dom` comes from binding data, not the lane parameter type | **Fixed (4.8, 8.2):** the quantifier ranges over the source's declared parameter types via a total embedding; the binding only selects an embedding; non-embeddable lanes are capped ("integer inputs only"); `enc` is partial |
| 3 | The reverse check judges a function's text, not the shipped unit | **Fixed (8.1, 7.4):** whole unit, explicit opaque-item rule, export map in the statement, build-profile quantification or refusal, CI regenerate-and-diff |
| 4 | The fuel elaboration is unsound as a gate | **Fixed (5.5):** `f_fuel` returns `Option`, no default; the gate generates the totality and monotonicity goals from the name; any goal whose footprint has a measure-defined constant is refused without `.total`; lemma location stated |
| 5 | A1 `bounded(k)` reachable with no evidence about the lane's code | **Fixed (1, 6.3, 9.1):** `bounded(k)` only for the symbolically executed lane; the two `tcb.*` ids; non-vacuity check; `k` defined per type; the false sentence removed |
| 6 | `elaboration-faithful`'s two routes close nothing | **Fixed (5.8, Q6):** two propositions split; the (c) id closes only by mechanical evidence (second elaborator, `Eq.refl` differential, lemmas); a signature closes only `<pkg>.model.faithful`. Fable's view adopted over the first draft |
| 7 | The differential is too weak | **Fixed (5.8):** vectors as `Eq.refl` kernel theorems, independent encoder, per-function coverage, skip ratio, claim instances, goal-level mutation corpus; `pif-eval` dropped |
| 8 | `beq_T` and helpers carry meaning on review alone | **Fixed (5.10):** `beq_spec` and helper specification lemmas are gate conditions; goals use `Eq T` at a top-level `==` |
| 9 | Non-vacuity not required of ADL goals | **Fixed (5.10, 8.2):** witness theorems mandatory for every hypothesis goal, `Refines` and `Equiv` |
| 10 | `Refines` is a sketch | **Fixed (8.2):** one total `outcome`, `enc` partial, trap identity from `panic_messages`, bounds and constant time as separate goals; the full definition is VER33b (ADL-020) |
| 11 | Forward `Refines(WIR, f)` is about a program nobody ships | **Fixed (7.1, 7.2, D11):** compiler demoted to optional P2, "validated" dropped from A4, A4 without a lifter is `tested`; the reverse check is the only link |
| 12 | `proved` at A3 has no honest route | **Fixed (1, 9.1, 9.2, 13):** `proved` stated as blocked per lane on VER11a Q3; the VER31 §6.2 amendment added; the subset-conformance caveat stated |
| 13 | A3 conflates two levels, and source-level is K1 | **Fixed (8.0):** A3-WIR and A3-source named with their open ids; K1 and PIF-009 on every source-level row |
| 14 | A `measure` model at A0/A1 is the alternative VER04s rejected | **Fixed (4.1, 13):** `measure` models excluded from A0/A1 evidence until the totality theorem is checked; no VER04s amendment, no 18-lane rows |
| 15 | Prelude churn invalidates signatures | **Fixed (5.2):** append-only layers, an ordering rule, cost per import, re-sign per claim set |
| 16 | Records elaborate to `proj`, K0 has none | **Fixed (5.3):** records through `T.rec` at K0; `proj` after slice 3b |
| 17 | The Lean escape hatch does not fit the chain | **Fixed (6.2):** needs `Init` in the base chain (K1, re-sign) or PIF-010's import plus a closure check |
| 18 | Goal ids contain `/` | **Fixed (5.1):** ids are `<module>.claim.<name>` in VER32 G3's charset |
| 19 | Namespace drift | **Fixed (5.1, 5.3, 5.9):** `Pif.*`/`Adl.*`, one rule, `GateError` in one place; `Pif.Result` |
| 20 | Stale VER32 references | **Fixed (2.2, 5.1, 13.1):** base chain from the sign-off, `tcb.pif.printer`, the file round trip |
| 21 | Match elaboration underspecified | **Fixed (5.4):** flat matches only, final `_` expanded, other patterns refused; the adder sketch rewritten |
| 22 | Regression examples in the `.warrant` file | **Fixed (6.3):** stored in the corpus |
| 23 | "Independent" generated lanes | **Fixed (7.5):** independent of the hand-written lane only; C/C++ reported "generated", `tested` |
| 24 | Trap spelling has no named goal | **Fixed (4.3):** `trap_eq_test` goal or refusal |
| 25 | `exhaustive` in wide lanes | **Fixed (1.3):** the rung means the Warrant domain; the lane parameter type is printed beside it |
| 26 | Budget caveat | **Fixed (5.8):** "faithful to the evaluator wherever it returns a value" |
| 27 | Wrong or missing dependencies | **Fixed (shard, 11):** ADL-002 needs ADL-001a; G-M1 needs ADL-005c, ADL-008a and ADL-008b; ADL-003a needs ADL-004a; ADL-009/050/051 list WX-R1 and WX-D2; ADL-003 = PIF-017 reconciled with VER32 |
| 28 | Rows that are not PR-sized | **Fixed (shard):** every row cut to one PR; per-lane lifter (5 PRs) and backend (3 PRs) rows; VER33a/VER33b named, not written |
| 29 | Missing items | **Fixed (shard, 8.1, 8.7):** `ca-assurance-report` changes (ADL-006c1 to c3), the VER31 amendment (ADL-006b), CI regeneration (ADL-040), witnesses and lemmas (ADL-008), the printer round trip (ADL-003a), sabotage corpus (ADL-044), export-map and whole-unit rules (ADL-032), Rust subset additions (ADL-033) |
| 30 | Per-lane estimates are low | **Fixed (8.4):** parser, export, profiles, pins, harness added; Java, Kotlin, C# not shared; stated as estimates |
| 31 | Kernel parsers vs the no-hand-written-parser rule | **Fixed (8.3, ADL-030/031):** one reviewed kernel-side grammar interpreter, lane grammars as data; ADL-030 to confirm |

**Fable, confirmation pass** (on `08bc4b916f5`): ready to merge as a draft for owner approval
once R1 to R4 are fixed; no further review needed.

| # | Finding | Disposition |
|---|---|---|
| R1 | Transport rule moves other `U8` subterms (`a + 1u8`) | **Fixed (1.4, 9.2):** applies only to binders, in-range literals, comparisons and canonical helpers; fixed-width arithmetic, `wrap`, `try` refused (K1); `exists` takes the range as a conjunct; the transported goal alone is a model theorem, the claim rises only with `Equiv` for every `op` it mentions |
| R2 | Prelude ordering contradicts VER32 E2 | **Fixed (5.2):** one global append-only sequence L1..Ln; a model's chain is a prefix; the cross-module cost (new digest and signature under a longer prefix) stated |
| R3 | Measure models: callers, differential, `.mono` | **Fixed (4.1, 5.5):** the first slice refuses calls from other models; the evaluator runs measure models under its meter for the differential only (backlog ADL-005e); `.mono` written out |
| R4 | Goal-id collisions | **Fixed (5.1, 5.5):** reserved `gen` and `claim` segments; the totality rule checks the regenerated goal digest; measure-defined constants known from the gate's own run |
| R5 | Trap tags | **Fixed (8.2):** the tag set is in the signed `Equiv` goal, exact templates resolved to panic sites, no wildcards or compiler-generated panics, outcome as membership |
| R6 | Rules VER33a must carry | **Fixed (8.1):** opaque items are declarations only, package-level export resolution, whole-unit rule is A3-source only |
| R7 | Differential details | **Fixed (5.4, 5.8):** hypothesis and conclusion compared separately, kernel-limit `Eq.refl` counts as a skip, records through `T.rec` permanently, `Nat` patterns exempt |
| B | Backlog | **Fixed (shard):** ADL-006c, ADL-002, ADL-012, ADL-008 split; ADL-009's corpus stated as hand-written PIF; KRN-002 slice 4 cited precisely; ADL-061 and the grouped lane rows marked P3 placeholders |

**VER34 requests** (five, from its draft at `a3fa78ef314`): certificate schemes and
`solver-attested` in 6.3 and the A1 row (done); lists as arrays plus a length (done, 6.3);
CVM prerequisites on ADL-010, ADL-014, ADL-015 (done); a WIR-to-SMT encoder and claim-to-VC
generator as their own rows (ADL-012a/b, ADL-013; done); the SMT library in the environment chain
(13.1; done).

### Follow-up 2026-11-24 (cross-spec follow-up; owner decisions and implementation findings)

Applied after VER32 and VER34 merged and after PIF-001 to PIF-004 and the first
ADL-004a/ADL-002 work. "VER34 13.1 n" is item n under VER33 in
[VER34](VER34-constraint-vm-smt-solver.md) section 13.1; "VER33 13.1 n" is one of this
document's own four requests to VER32.

| # | Change | Source | Where |
|---|---|---|---|
| G1 | **The SMT library is not meaning.** Removed from the "reviewed semantics files" row; a new row says it is proof plumbing on the proof side, with no id, faithful by bridge theorem | VER34 13.1 1 | 2.2, 9.1 |
| G2 | **K1 split.** K1 is `lits` plus native `Adl`/`Pif` fixed-width words, **no `Init`**; an `Init` import is a separate later tier (named K2 here) for the Lean escape hatch only (ADL-060b); no third bridge. Layer `adl.fixed` reworded; 6.2 now names the K2 tier | VER34 13.1 2, VER34 6.9 | 4.2, 5.2, 6.2, 8.0 note |
| G3 | **`tcb.vcgen.claim-faithful`** registered under ADL-013 and ADL-006a (with VER32 PIF-016d); the WIR-to-SMT encoder's two ids registered under ADL-012a/b | VER34 13.1 3 | 6.3, 9.1, backlog |
| G4 | **`tcb.drat.checker-correct`** added to rung A1's id list (with `tcb.bitblast.encoding-correct`, `tcb.symexec.engine-correct`) **while the DRAT route lasts**; its row names the sunset | VER34 13.1 4 | 2.2, 6.3, 9.1 |
| G5 | 6.3 **cites VER34 6.7's DRAT rules** (formula-level passes off, certificate record, non-vacuity, retirement, sunset) and the wording "`k` is signed on `pif:`, record-bound on `drat:`" | VER34 13.1 5 | 6.3 |
| G6 | The certificate schemes (`drat:`, `pif:`) and the `solver-attested` kind (capped at `tested`) are named; **ADL-006b cites CVM-170b**, which owns the VER31 amendment text | VER34 13.1 6 | 6.3, 9.2, backlog |
| G7 | **Already stated**: Warrant `List`/`Bytes` are **arrays plus a length** in the solver, datatypes at M3 (so M2, not M3). No edit beyond this record | VER34 13.1 7 | 6.3 (existing) |
| G8 | ADL-010 and ADL-012 to ADL-015 cite CVM-020 to CVM-023, **CVM-170a/170b** (not "CVM-170") and CVM-173 as prerequisites | VER34 13.1 8 | 6.3, 9.2, 11, backlog |
| G9 | 13.1's own environment-chain item corrected: **signed base, then proof side** | VER34 13.1 9, VER32 E7 | 13.1, 5.1 |
| G10 | **Reconciliation of VER33 13.1 with VER34 13.1.** (a) `pif-eval` dropped (VER33 request 1; VER34 needs none): same outcome. (b) VER33's chain "prelude, ADL layers, SMT library, model" **superseded by VER34's proof side**: where they conflict VER34 wins. (c) Link status in VER32 Q8 including the domain quantified, PIF-016 registering VER33's ids (requests 2): both applied in VER32. (d) Package claims never ride on model theorems; ADL layers' prefix rule is E2's consequence (requests 3, 4): applied in VER32 9.6. All four marked applied in 13.1 | VER33 13.1, VER34 13.1 | 13.1 |
| G11 | **ADL-004a naming**: `Adl.ite.{u} alpha c t e` (the result type is an explicit first argument); Int comparisons are `Adl.Int.{beq, le, lt}` (no `Int.ble` or `Int.blt`), Nat has `Adl.Nat.{beq, ble, blt}`; `>` and `>=` are `lt` and `le` with swapped arguments; `Option.isSome` in `.total` goals is `Adl.Option.isSome`; **`↔` is emitted as an `And` of both directions** (the PIF prelude has no `Iff`); the printed example is updated. **Deferred to ADL-008a**: Int order against add, add commutative and associative, `sub a a = 0`, `Nat.sub` against add, `beq_spec` for `Option`, `Result`, `Prod` | ADL-004a implementation finding | 5.2, 5.4, 5.5, 5.9, 5.10 |
| G12 | **`Pif.GateError` lives in the gate model file** (ADL-002), not in a prelude; `Pif.Result` is in the PIF prelude. The 5.3 rule "an error type already in the PIF prelude is `Pif.GateError`" is replaced | PIF-004 decision | 5.3 |
| G13 | **ADL-002 claim naming**: LG02's names over `Bool`, plus separate `<name>_over_int` and `<name>_matches_ref` claims | ADL-002 decision | 10.2, backlog |
| G14 | **Owner decisions recorded** (12.1): all agreed recommendations accepted as working decisions; VER32's Q1 exemption, Q2 option D and the proof side noted as they touch VER33 | owner, 2026-11-24 | 12.1 |
| G15 | **Judgment call, flagged**: after the K1 split the A3-source rows keep PIF-009 conservatively; whether the lane semantics need `Init` is left to ADL-030 | resolution of VER34 13.1 2 against the existing "K1 and PIF-009" rows | 8.0, backlog M8 |

**Not applied, and why.** Nothing in VER34 13.1 addressed to VER33 was left out. G7 needed no
edit because the text already says it. G15 is a recorded judgment, not a decision.

### Follow-up / G-M1 registration 2026-11-25

**Status: ADL-006a and ADL-006b are drafted, proposed, pending owner signature.** Nothing in
sections 1 to 14 above is edited; this is the review-record note.

| # | Change | Source | Where |
|---|---|---|---|
| J1 | **ADL-006b drafted** as VER31 section 13.4 to 13.6: the section 6.2 cap on lane packages lifts per claim, by computation through the claim's own signed goal and the ordinary ladder (finding 12; `proved` stays unreachable for any lane until VER11a Q3 is decided for it); the A-rung is computed by the gate, never declared, printed beside the VER31 rung, and **A2 is never rendered as a package-level word containing "proved"** (1.5, Q7); the model-theorem category and link status come from VER32 Q8 and PIF-020 (13.3 there). Marked "proposed, pending owner signature" | 1.5, 9.2, 13; backlog ADL-006b | VER31 13.4, 13.5 |
| J2 | **Resolved: VER31 cites CVM-170b for `solver-attested`, it does not copy it**, as the ADL-006b row says. VER31 13.6 carries a non-normative summary, and asks CVM-170b to read its `pif:sha256:` clause "as defined in VER31 13.2" so the scheme has one owner | VER34 6.6, 6.7; CVM-170b | VER31 13.6 |
| J3 | **ADL-006a drafted** as data (VER13 section 10; `catalogue-v2.proposed.json`, `buckets-v2.proposed.json`): the ids 9.1 marks *new* that G-M1 opens or relies on are registered with a proposed bucket and closing route. The scheme `<pkg>.model.faithful` is instantiated as `gates.model.faithful` (VER32 9.2 pins the spelling), so **`logic_gates.model.faithful` is not registered**. `tcb.vcgen.claim-faithful` and `adl.wir-abi.faithful` are registered now although A1 and A3 open them, because ADL-006a and PIF-016a name them. Not registered here (G-M2 and later): `wir.semantics.lean-faithful`, `wir.typing.checker-guarantees`, `src.<lane>.lean-semantics.faithful`, `src.<lane>.subset-conformance`, `<lane>.backend.code-link`, `tcb.<lane>-toolchain.trusted`, `<pkg>.model.code-link` beyond the gates'; `tcb.bitblast.encoding-correct`, `tcb.symexec.engine-correct` and `tcb.drat.checker-correct` are already in the catalogue | 9.1; backlog ADL-006a | VER13 section 10 |
| J4 | **G-M1 status (implemented, PRs pending).** Model file, 89-line goals list and claim file produced; 89 goals kernel-checked with empty footprints through `pif-check` with a **test-fixture** sign-off; vectors 1021/1021. G-M1's signature conditions are not yet met: ADL-005b/c and ADL-008a/b still gate it. Also remaining: the owner's signatures and the push queue | backlog G-M1 | warrant-adl shard |

**Not applied, and why.** No bucket moved and no VER31 rule is in force before the owner signs
VER31 section 13. ADL-006c1 to c3 (the gate code) wait for that signature.

### Fable design review of the mandatory gate, 2026-11-25

**Context.** The gate in `pif-check` enforces 5.10 before a Warrant-derived proof chain can be
signed. Three security-review rounds (below) each found a new fail-open hole, so the
coordinator asked Fable for an adversarial *design* review rather than a fourth patch.

**Root cause** (Fable, three parts):

- **RC1 The gate inverts the elaborator.** 5.10 defined obligations over Warrant *source*
  constructs (a `|` guard, `rejects`, a derived `==`, a `measure` model), but the gate never
  sees source. It re-derived "which construct was this?" from kernel terms, and the term
  language is full CIC, so every recogniser had an escape (a wrapper, an alias, a redex, a
  different role, a different name shape). `rejects P with E` was not even a source construct,
  and the elaborator's stub generator pattern-matched the claim's shape too, so the gate copied
  a heuristic instead of checking a fact.
- **RC2 A negative detection means "no obligation".** Every hole had this shape:
  `requires_exact_rejection` false skipped exactness; no hypotheses skipped all witnesses;
  `NotResult` skipped outcome witnesses; `is_data` true skipped inhabitation; a role other than
  model hid a declaration; a name shape other than `[m, s]` was skipped; and
  `is_algorithm_chain` false turned the whole gate off.
- **RC3 The gate reasons semantically itself, and the property has no definition.**
  Inhabitation and "returns `Err`" are undecidable and the kernel is the only sound judge; the
  gate's `is_data`/`data_types` was a private, weaker inhabitation checker inside the trusted
  base. 5.10 never stated what an accepted chain guarantees, so reviewers measured against
  intuition and every gap became a "hole". What held up was the part where the gate builds a
  statement and the kernel proves it (`Checker::verify` with exact comparison; the run
  witness's match, rebuild and compare).

**Findings F1 to F12** (at `a2d9029d167`):

| # | Severity | Finding (the attack in one line) | Closed by |
|---|---|---|---|
| F1 | HIGH | Discovery keyed on the chain role: real functions in a `library` file (`lib.f : Int → Pif.Result A E`) with a dummy model constant get no exact-rejection, no outcome witnesses and no `beq_spec` | I3 |
| F2 | HIGH | Empty instances of indexed families pass even after the N1 fix: `inductive m.T : Type → Type \| mk : m.T Bool` makes `(x : m.T Nat)` pass `is_data` though `m.T Nat` is empty; only a kernel-proved witness sees it | I2, G1 |
| F3 | MEDIUM | Outcome witnesses skipped by name shape or declaration kind (`m.sub.f`, `opaque`, `axiom`) | I1, I3, G3 |
| F4 | MEDIUM | Wrappers evade outcome witnesses (`m.R \| mk : Pif.Result A E → m.R`; `m.g x := isOk (m.f x)`; a `Result` model called only transitively) | I1, G3 |
| F5 | MEDIUM | Spec-level: a guard moved out of the telescope (`∀ x, imp (p x) (q x) = true`, or folded into a model) removes every obligation; detection cannot close it | "does not guarantee" list |
| F6 | MEDIUM | Latent: the conclusion's `Eq` type is unchecked, so `Eq Prop (h → Q) True` would hide a hypothesis; unprovable today only while the prelude lacks `propext`, `Quot` and `funext` | claim-shape rule (`Eq` at a data head) |
| F7 | MEDIUM | The gate's vocabulary (`Exists`, `And`, `Eq`, `Bool.true`, `Pif.Result.rec`) is looked up by name and the rules lemmas are bound by type digest only; the sign-off has no `rules` line, so a one-lemma rules file passes unless a consumer pins `rules_sha256` | `pif-rules 2`, `rules sha256` |
| F8 | MEDIUM | The gate and 5.10 disagree: `.total`/`.mono` are listed but not enforced, and such goals are refused as unrequired | totality row moved out |
| F9 | LOW (soundness), HIGH (usability) | False refusals: any `E → E → Bool` model not named `beq`; `List`/`Option`/`Prod` models; unused binders breaking the run witness; every one-directional `P → f x = Err E` forced to be exact; a `Result` model that never errs cannot be mentioned | G4, G5, exact-rejection row |
| F10 | LOW | `beq` discovery used two heuristics; an alias head, an uncurried test or a test outside the model role evades both | `beq_spec` by reserved name |
| F11 | LOW | Applicability detected from content: odd ids, a `library` role or `(h : G x)` hypotheses read as "not an algorithm chain", making `--accept-not-required` a label dodge | I4, `--profile` |
| F12 | informational | Distinctness (`Checker::used`) is not a soundness property once statements are exact; it can only cause refusals | none (dropped) |

**The redesign** is **"derive, don't detect"**, adopted by the coordinator; implementation is
on local branches (backlog ADL-008a/b, redesigned, implementation pending). It is stated in
5.10: guarantees G1 to G5, a list of what the gate does not guarantee, invariants I1 to I6, the
changed obligation rows (non-vacuity (i) and (ii), exact rejection as a `contract` goal,
`beq_spec` by reserved name, totality moved out of the gate), the explicit `--profile` and
`rules sha256` configuration, and `contract` as a reserved id segment (5.1). The `pif-rules 1`
format is refused rather than upgraded (VER32 3.9).

**Prior rounds 1 to 3** (security reviews of the ADL-008a/b implementation, before this
review). Their patches were to the detectors, the cause Fable named in RC1 and RC2; each fix
is superseded in mechanism by the redesign and each finding stays a test case for it.

| Round | Findings | Fixed then by |
|---|---|---|
| 1 | 4 HIGH, 4 MEDIUM, 1 LOW, 1 INFO. Obligations were discovered from producer-chosen labels and checked by loose "mentions X" tests. HIGH: a real claim disguised as `m.gen.c1` escaped every witness check; a foreign id prefix made `module_call` miss the model; a hypothesis not spelled `Eq Bool p true` was not counted; degenerate witnesses passed (`Exists Nat (fun x => x = x)`, identical `.ok`/`.err` witnesses, one theorem backing several obligations). MEDIUM: weak `beq_spec` and exact-rejection; `beq` found by name only; a role-label bypass; top-level-only called-model discovery | exact expected statements compared by hash-consed term equality; discovery from model files and claim types; a strict claim grammar |
| 2 | 1 HIGH, 4 MEDIUM. HIGH: hidden hypotheses as binder types (`h : m.G x`) and empty-type binders. MEDIUM: exponential `match_pattern`; `Result` detection failing open; rejection discovery evaded by redexes, wrappers or `isErr`; content enforcement dodged with library-filed models | greatest-fixpoint data-type check; memoisation; fail-closed `Result` classification; broader rejection discovery; goal-based enforcement with exit 5 |
| 3 | N1 HIGH: empty recursive types (`m.Void2 \| mk : m.Void2 → m.Void2`) and recursive `Prop` inductives pass the greatest-fixpoint check. N2 MEDIUM: a wrapper definition (`m.fail e := err e`) evades rejection discovery. N3 MEDIUM: exponential `is_data`. N4 LOW: quadratic `data_types` | superseded: Fable showed the planned N1 fix (least-fixpoint inhabitation) is itself bypassable (F2) |

**New requests recorded.** VER32 3.9 gains `rules sha256` and `profile` sign-off lines and the
`pif-rules 2` format. Backlog ADL-008d (`rejects P with E` as a source construct) and ADL-008e
(totality as a gate condition) are added.
