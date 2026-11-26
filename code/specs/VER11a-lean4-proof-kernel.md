# VER11a — Proof certificates and the trusted kernel, part 2: a Lean 4 kernel

**Status:** Draft for owner approval. Spec only; no code exists. The kernel
logic is an owner decision (section 3); the remaining owner questions and the
recorded answers are in section 16. Implementation is the `KRN-*` queue in
[VER-BACKLOG.md](VER-BACKLOG.md), which starts only after the owner approves
this design and an adversarial review (Fable) has been answered.

**Layer:** Proof machinery, layer 3 (VER00 §7.5 step 3). VER11 part 1 is the
propositional kernel (`drat-check`). This part is the kernel that checks
everything else. It is also layer L1 of the Lean programme
([LEAN00](LEAN00-lean-on-the-generic-pipeline.md)): **one kernel serves
both Lean proofs and the repository's own `proved` labels.**

**Builds on:** [VER00](VER00-verified-shipping-program.md) §7.5 and §7.6,
[VER09](VER09-cdcl-sat.md) (CDCL), [VER10](VER10-bit-blasting.md)
(AIG, Tseitin, bit-vectors), [VER11](VER11-drat-proofs.md) (DRAT),
[VER13](VER13-assumption-catalogue.md) (assumption catalogue),
[VER19](VER19-symbolic-execution.md) (symbolic execution),
[VER20](VER20-resource-timing-proofs.md) (self-composition and bounds),
[VER25](VER25-wir-core-interpreter.md), [VER25a](VER25a-rust-subset-grammar.md)
and [VER25b](VER25b-rust-to-wir-frontend.md) (WIR and the Rust front end),
[VER26](VER26-ct-compare-pilot.md) (the ct-compare pilot),
[VER27](VER27-layered-validation-roadmap.md) §4.7 and §9 (honest state),
[VER29](VER29-layer-contract-linking.md) (layer contracts),
[VOS30](VOS30-proof-route-and-capacity.md) §3.6, §4.2, §4.5, §4.6 (cuts,
collapse, vacuity) and [LANG24](LANG24-constraint-vm.md) (Constraint VM).

**Consumed by:** the assurance report's `proved` label for every claim whose
proof goes through WIR; the tiered-release standard (another session's spec,
being written in parallel; section 12.4 lists what the two must agree on);
LEAN00 layers L2 to L5; VER12 (verification-condition generation), which is
carved out of sections 7 to 9 before its code is written.

---

## 0. The design in one page

**The problem.** `rust/ct-compare` has four claims. Today one is `proved` and
three are `bounded(128)`, and each of the four rests on 53 assumptions. About
half of those are gaps in our own tools: the symbolic executor might be wrong,
the bit-blaster might be wrong, loops are unrolled only 128 times, and every
DRAT certificate is the same two-clause refutation of `x ∧ ¬x` (section 2).
None of that is about the compiler or the CPU. It is about us.

**The fix.** Move the trust out of the large tools and into one program, the
**kernel**, as Lean, Coq, HOL and Isabelle do. The symbolic executor, the
bit-blaster, the SAT solver and the invariant finder all stay, but they become
*proof search*: whatever they find is turned into a proof term that the
kernel type-checks. A wrong tool produces an ill-typed term and no proof. It
never produces a wrong proof.

**The owner's decision (2026-11-23).** The kernel implements **Lean 4's type
theory** and checks Lean 4's own export format. Then the repository can check
any Lean proof, reuse Lean's core library, and grow into a general Lean-like
machine (LEAN00), while the same kernel checks the repository's own claims.

**The decisions, each argued below.**

| # | Question | Decision or recommendation | Section |
|---|---|---|---|
| D1 | Kernel logic | **Lean 4's type theory** (owner decision): dependent types, universe polymorphism, inductive types with recursors, quotients, definitional eta, proof irrelevance, `Nat` and `String` literals | 3 |
| D2 | Kernel interface | **Lean 4's export format** (lean4export, pinned version) is the only input. Lean's own toolchain, our Rust tools and any program in any language can write it; the kernel checks it | 4 |
| D3 | What a WIR program means | A **clock-indexed functional semantics of WIR, written in Lean**, becomes the normative meaning of WIR | 6 |
| D4 | Constant-time proofs | **Lockstep self-composition.** One generic theorem, proved once: if a relation on pairs of states holds at every program point and each step preserves it while emitting equal events, then the two runs produce the same observation trace after any number of steps. The relation is "public variables equal and traces equal so far". Loops of every length are covered | 8 |
| D5 | Where obligations come from | Proof-producing VC generation: the untrusted label inference guesses the invariant; each program point's obligation is proved with a **step lemma** proved once in `warrant-lib`, by a generated Lean proof. A one-week spike in official Lean gates the queue | 9 |
| D6 | How obligations are discharged | Step lemmas and invariant facts, `rfl` for what reduces, a linear-arithmetic certificate procedure, core `BitVec` lemmas, then the Constraint VM and SAT with **LRAT turned into proof terms**. Escape hatch: Lean proofs, written by hand or by an agent | 9, 10 |
| D7 | How assumptions show | Lean's own `#print axioms`, extended: the standard three Lean axioms, any oracle axiom, formal hypotheses by VER13 id, and the assumptions behind every trusted definition the statement mentions, grouped by VER31's buckets `a|b|c` | 11 |
| D8 | Trust in the kernel | Under the owner's rule only the compiler is trusted, so the kernel's own correctness is a (c) id that must be closed: literate source within a budget (about 8,500 lines), adversarial corpora, mutation testing, and **independent agreement** with the official Lean kernel and our own second checker (route R1), later a verified kernel (R2). The logic's consistency cannot be proved at all and is put to the owner as a floor. The claim elaborator leaves the trust base: the printed goal is signed | 11.2, 12 |
| D9 | Milestones | **K-M0**: our kernel checks Lean's core library export (LEAN00 L1). **K-M1**: all four ct-compare claims checked for *all lengths* at WIR level, moving from `bounded(128)` to VER31's `checked`. **K-M2**: translation validation of the Rust-to-WIR step closes its six (c) ids. `proved` then needs the owner's decisions on the kernel's own ids and two model ids (Q5–Q7) | 13, 14 |

**What the decision costs.** VER00 §7.5 promised a kernel of "a few hundred
lines". A Lean 4 kernel is an order of magnitude larger, because dependent
types, inductive types, universes and definitional unfolding must all be
checked. This spec replaces that promise with a budget and an independent
check (section 12). VER00 is amended to match (backlog KRN-001).

**What the decision buys.** Lean's core library (`Init`, and the parts of
`Std` we need) already defines `Nat`, `List`, `BitVec`, `UInt8`, `Option` and
thousands of lemmas about them. If our kernel checks that library, we can
use it, and most of the 15,000 to 25,000 lines of library the HOL design
needed are not written at all. And because the kernel can *compute*
(definitional reduction), a block of a concrete program run on a symbolic
state is often proved by `rfl`: the kernel does the symbolic execution
itself.

### 0.1 What this is for: one Warrant definition, every rung (owner, 2026-11-23)

The owner's framing, which this spec serves: "if someone specifies that this
function should take in between 1-10, I want to make sure that anything above
10 or any other malformed input… not even gets in… This has to expand out to
any class of CVEs… Automatically generate unit test, fuzz test, property
test, etc from a single well scoped definition of things."

So the user-facing object is the **Warrant claim**, never WIR. WIR (and the
Lean semantics of it) is internal plumbing: a proof needs a precise meaning
of the code, and WIR is where the repository keeps one. Nobody writing a
`.warrant` file should have to look at it.

Each Warrant claim class already generates tests (VER04h, VER05, VER06). The
kernel adds the top rung for the same clause: a proof for every input. The
claim classes, the weakness classes they close, and the kernel goal each one
becomes:

| Warrant clause | Weakness classes (CWE) | Generated today | Kernel goal template (section 7) | First adopter |
|---|---|---|---|---|
| `input f.x: rejects P with E` (input contract) | CWE-20 improper input validation, CWE-1284 unchecked quantity, CWE-129 unchecked index, CWE-1287 wrong type | Boundary examples on both sides of `P`, properties over generated valid and invalid inputs, fuzzing with malformed inputs | **Exact rejection at the boundary**: every input with `P` returns `E` and causes no effect (no capability call, no write to caller-visible memory, and, where the clause says so, no read of the rejected data); every input without `P` never returns `E` | `wasm-leb128`: `rejects max_bits > 64 with MALFORMED` (the SEC-006 bug, found by Warrant, was exactly "a value above the range got in"); ct-compare `select_bytes`: `rejects len(a) != len(b)` |
| `ensures no_panic, terminates` | CWE-248 uncaught exception, CWE-617 reachable assertion, CWE-190 overflow trap, CWE-835 infinite loop | Fuzzing | Every input in the contract ends in `ret`, never `trap`, `stuck` or non-termination | Every ct-compare operation; the `wasm-leb128` decoders |
| Memory safety (implicit in every proof) | CWE-125, CWE-787 out of bounds, CWE-416, CWE-415, CWE-476 | Fuzzing | No reachable `stuck` (6.2) | ct-compare |
| `bound steps / heap / depth` | CWE-400, CWE-770 resource exhaustion, CWE-674 recursion | Microbenchmarks, fuzzing with budgets | Terminates with cost `≤ B(n)`, heap `≤ H(n)`, depth `≤ D` | ct-compare |
| `labels … secret` (constant time) | CWE-208, CWE-385 timing | Equal-instruction-count benchmark | Two-run lockstep: equal observations (section 8) | ct-compare |
| `claim ∀ x. f(x) == model(x)` | CWE-682 incorrect calculation | Examples, properties, exhaustive runs | `∀ x, result = model x` | ct-compare (K-M1c), gf256 |
| Information flow (VER24, later) | CWE-78, CWE-89, CWE-79 injection | Taint tests (later) | Noninterference from hostile sources to sinks | none yet |

**Malformed structured input.** For a parser or decoder, "malformed" is
defined by the Warrant model of well-formed input, and the contract is a pair:
`decode(x) = ok v` exactly when `x` is well formed (with `v` the model's value),
and no byte string at all reaches `trap` or `stuck`. The second half is what
fuzzing approximates and the kernel proves.

The milestones (section 14) therefore lead with input contracts, then
constant time.

---

## 1. Scope

### 1.1 The scope rule (owner, 2026-11-23; VER31 §3)

This spec uses the buckets of [VER31](VER31-tiered-release-standard.md) §3
exactly as fixed there (`code/assumptions/buckets-v1.json`, enum `a|b|c`):

| Bucket | Contents | Blocks `proved`? | Closed by |
|---|---|---|---|
| (a) Accepted platform baseline | Layers `cc`, `mc`, `uarch`, `gate`, `phys`, `os`; the source language's own semantics; and the compiler that builds the verification tools (`tcb.rust-toolchain.trusted`) | No. Always printed | Nothing; assumed |
| (b) Statement | `spec.*`: the claim says what we mean | Yes, until signed | A signed statement sign-off (VER31 §3.2) |
| (c) The repository's own gaps | `wir.*`, every `tcb.*` except the compiler, our own source properties, our own process | **Yes** | Only by checked evidence the checker accepts, never by argument |

**The owner's trust rule (2026-11-23, VER31 §3.4):** "We can definitely
trust the compiler. Everything else has to be verified." In particular our
own checkers (`drat-check`, the report checker) are **not** a trusted kernel:
they are (c), to be verified, and this kernel is the named route for doing so.
That rule applies to this kernel too (section 11.2).

**This spec targets bucket (c) only.** It does not try to discharge bucket
(a). Where a bucket-(c) id cannot be closed by the kernel, the spec says so
and says what would close it.

### 1.2 What this spec covers, and what LEAN00 covers

This spec: the kernel (sections 3, 4), trust in it (section 12), and the whole
verification route from a Warrant claim and WIR code to a checked theorem
(sections 5 to 11, 13, 14).

[LEAN00](LEAN00-lean-on-the-generic-pipeline.md): Lean as a language on the
repository's generic pipeline: grammar, elaborator, command execution on the
LANG VM, decision procedures on the Constraint VM, and full source
compatibility. The kernel of this spec is its layer L1.

### 1.3 What neither covers

- Discharging bucket (a): no compiler, ISA or gate proofs (VER13 §4 and VER27
  cover those routes; they are unaffected).
- Proving the kernel correct in itself (the lean4lean direction; section
  12.6). A far-future item, not a milestone.
- Languages other than Rust, and packages other than ct-compare, for the
  verification route. The design is general; the milestones are not.

---

## 2. Where we are, and why the certificate is trivial

### 2.1 The current pipeline

```
   ct-compare.warrant ──► claim ids, labels, bounds
   src/lib.rs ──► VER25a parser ──► VER25b lowering ──► WIR ──► inliner
                                                                 │
          for every pair of concrete lengths (la, lb) ≤ 128:     ▼
              symbolic executor (VER19), contents symbolic, two runs
              ──► per-path trace comparison (VER20, ct_proof.rs)
              ──► AIG miter ──► Tseitin ──► cdcl-sat ──► drat-check
```

`evidence/proof-manifest.json` records 16,641 runs for `eq` (129 × 129
length pairs) and one certificate digest for all of them.

### 2.2 Why the miter collapses

Read the driver `ct_once` in `warrant-ir/tests/warrant_proofs.rs` and
`prove_constant_time_internal` in `warrant-ir/src/ct_proof.rs`. The driver
allocates each buffer with a **concrete size** and passes no public-equality
constraints. So:

1. **There is one path per run.** Every branch condition that depends on a
   length is a constant edge, and the executor never forks on it
   (`sym.rs`, the branch handling). The two runs are two straight-line
   traces.
2. **Step counts are compared in Rust**, as integers (`p1.steps != p2.steps`,
   `ct_proof.rs:350`). Equal counts add nothing to the miter.
3. **Memory offsets are concrete.** With concrete sizes the offset of the
   `i`-th load is the literal `i` in both runs, so `bv::eq(off1, off2)` is
   structurally `TRUE` and the difference edge is `FALSE`
   (`ct_proof.rs:440-443`).

Every contribution is `FALSE`, so `overall_miter == Edge::FALSE`, and the code
builds the fixed formula `x ∧ ¬x`, solves it and DRAT-checks it. The
certificate is valid. It says nothing about `ct_eq`.

The real reasoning was done by the executor's bookkeeping in Rust and by AIG
structural hashing. That is exactly what `tcb.symexec.engine-correct` and
`wir.proof.trivial-miter` admit. VOS30 §4.5 calls it a *legitimate collapse*:
correct, but not evidence. Two consequences:

- **Bounded by construction.** The loop count is the length, and lengths are
  enumerated up to 128. More SAT power does not change that; a different
  proof shape does (`wir.loop.bounded-k`).
- **A harder CNF is not the fix.** A non-collapsed miter would only move the
  trust from the engine's bookkeeping to the engine's encoding. The fix is to
  stop trusting the engine: facts like "both offsets are `i`" must be
  *derived in the kernel from the meaning of the program*. Once they are, it
  does not matter that some of them are easy.
- **Today's step evidence uses a different cost model.** The symbolic
  executor charges one step per instruction and nothing per terminator
  (`sym.rs`, the instruction loop), not the VER25 §5.1 schedule. The step
  bounds proved today are therefore bounds under that count. The kernel
  route uses the VER25 schedule (section 6.2), so its first run is also the
  first real test of `steps ≤ 64 * len + 512`.

### 2.3 What we keep

| Piece | Role after this spec |
|---|---|
| `cdcl-sat` (VER09) | Untrusted proof search. Gains LRAT output (section 9.5) |
| `drat-check` (VER11 part 1) | A fast pre-filter, and the checker of `bounded(k)` results. No longer the root of trust for kernel proofs |
| `bit-blaster` (VER10), Constraint VM (LANG24) | Untrusted search; their answers become proof terms (section 9.5) |
| `warrant-ir` symbolic executor (VER19, VER20) | Untrusted guide and counterexample finder. Still produces `bounded(k)` where the kernel route fails |
| Reference interpreter (VER25) | Counterexample replay; tested against the Lean semantics (section 6.5) |
| Negative controls (`proofneg/*`) | Must still fail under the kernel route (section 12.5) |

---

## 3. Decision D1: the kernel's logic

### 3.1 The decision

**Lean 4's type theory**, by owner decision on 2026-11-23, replacing this
spec's earlier recommendation of HOL. The owner's reason: "We should be able
to run any Lean proof through a lexer, parser and other packages… I would
love to build this as a general purpose Lean like machine that can execute
commands."

**The deciding argument: Lean is an oracle at every layer.** The owner's
second reason is that anchoring on Lean gives us something independent to
check against. A home-grown logic has no outside reference; Lean has one at
every layer this programme builds:

| Our layer | Oracle | Corpus |
|---|---|---|
| Kernel (this spec; LEAN00 L1) | Lean's own C++ kernel; the independent checkers nanoda_lib (Rust) and lean4lean (Lean) | Lean's core library and Mathlib: hundreds of thousands of declarations known to be valid. Lean's kernel tests: declarations known to be invalid |
| Parser (LEAN00 L2) | Lean's parser, comparing syntax trees | Lean core and Mathlib source |
| Elaborator (LEAN00 L3) | Lean's elaborator: the same source must produce declarations our kernel accepts with the same types | Same |
| Execution (LEAN00 L4) | `#eval` in Lean | Lean's test suite |
| Our own proofs | Lean's kernel re-checks every export we produce | Every ct-compare export |

Every oracle is used **for differential testing only, never as a
dependency of a check** (the same terms as Z3 in VER00 §7.5): our kernel's
verdict is the gate.

### 3.2 The alternatives, for the record

The comparison that led to the earlier HOL recommendation is kept because it
says what the decision costs and buys.

| | **Lean 4 (chosen)** | **HOL (HOL Light style)** | **First-order logic with induction (ACL2 / Milawa)** |
|---|---|---|---|
| Trusted base | Lean's own C++ kernel and the independent checkers nanoda_lib (Rust) and lean4lean (Lean) each run to thousands of lines: inductive types, universe levels, definitional unfolding, quotients, literals | A few hundred lines: ten rules, two definition principles, three axioms | Milawa's core checker is small; ACL2's trusted prover is very large |
| Expressiveness | Everything HOL has, plus types that depend on values (`BitVec n`, `Fin n`, `Bytes<N>` as a type). Proofs are terms | Functions, predicates, relations as terms; no dependent types | First-order only: no quantifying over predicates |
| Computation in the kernel | **Yes**: definitional reduction, with `Nat` arithmetic on literals done natively. A concrete program on *concrete* inputs reduces (witnesses, `decide` on small closed facts); on symbolic state reduction stops at the first branch on a free variable, so symbolic steps need lemmas (9.2) | No: computing is rewriting, outside the kernel, step by step | Yes, for ground terms |
| SAT certificates | Reflection (a verified LRAT checker run by kernel reduction), or LRAT steps as explicit proof terms (section 9.5) | Replay LRAT as resolution inferences | Replay as resolution |
| Library | **Lean's core library is reusable once checked**, and Mathlib beyond it | Must be written from scratch (estimated 15,000–25,000 lines) | Must be written |
| Ecosystem | Large: proofs, tactics, people | Mature but small in the agentic-AI world | Small |

**What the choice costs, stated plainly:** a trusted kernel roughly ten times
the size, and a checker whose correctness is harder to establish by reading
alone. Section 12 is the answer to that.

### 3.3 The logic, precisely

Lean 4's kernel type theory, as checked by Lean's kernel at the pinned Lean
version (section 4.4):

- **Terms**: bound variables (de Bruijn indices), free variables (local
  constants), sorts `Sort u`, constants with universe arguments, application,
  `fun`, dependent function types (`∀`/Π), `let`, literals (`Nat`, `String`),
  structure projections. Metadata is ignored. Metavariables never reach the
  kernel.
- **Universes**: `0`, `succ`, `max`, `imax`, parameters; universe-polymorphic
  declarations; level equality by normalisation.
- **Declarations**: axioms, definitions (with reducibility hints), theorems,
  opaque constants, the quotient package, and (mutual, nested) inductive
  types, from which the kernel itself derives constructors' types and
  recursors after checking positivity and universe constraints.
- **Definitional equality**: β, δ (unfolding, guided by hints), ζ (`let`), ι
  (recursor on constructor), projection reduction, quotient reduction, η for
  functions, η for structures, proof irrelevance, unit-like types, `Nat`
  literal operations (`Nat.add`, `sub`, `mul`, `div`, `mod`, `gcd`, `beq`,
  `ble`, `land`, `lor`, `xor`, `shiftLeft`, `shiftRight`, `pow`, `log2`)
  computed natively, and `String` literals expanding to `String.mk` of a
  character list.
- **Further kernel rules the implementation must match exactly** (each is a
  place independent checkers have disagreed): K-like reduction for
  recursors of `Prop` inductives with one constructor and no fields (`Eq.rec`
  reduces without its major premise being a constructor); structure η applied
  to a recursor's major premise; conversion between `Nat` literals and
  `Nat.zero`/`Nat.succ` at recursor majors and in definitional equality; the
  rule that a projection out of a `Prop` structure may not produce data;
  large-elimination (subsingleton) restrictions on recursors into `Sort u`;
  a theorem's type must be a `Prop`; the quotient package is accepted only
  after `Eq` has the expected shape; a cap on `Nat.pow` exponents for literal
  acceleration.
- **Primitive definitions are validated.** Literal acceleration is sound only
  if `Nat`, `Nat.add`, …, `String`, `Eq` and `Quot` in the environment *are* the
  expected definitions. Before enabling acceleration for a constant, the kernel
  checks that its declaration has exactly the expected form, as lean4lean
  does; otherwise an export that defines `Nat.add` as multiplication would
  prove `False`. `String` literal expansion follows the representation at the
  pin (it has changed across Lean releases).
- **Standard axioms**: `propext`, `Classical.choice`, `Quot.sound`.

**Refused**: `unsafe` and `partial` declarations are rejected (`partial`
definitions reach the kernel only as opaque constants, which are accepted;
their compiled bodies are not the kernel's business). `Lean.ofReduceBool` and
`Lean.trustCompiler` are accepted only as **oracle axioms** (section 4.5).
`sorryAx` is accepted by the checker but forces the claim's label to fail.

---

## 4. The kernel

### 4.1 Shape

A Rust crate, `lean-kernel`, with no third-party dependencies. It is a type
checker, not an LCF kernel: where HOL keeps an abstract `Thm` type, Lean
checks each declaration's value against its type and then adds it to an
**environment**. A theorem is a declaration whose type is a proposition and
whose value type-checks against it.

```
   export file ──► reader ──► declaration stream ──► checker ──► environment
                                                       │
                                     whnf, infer, is_def_eq, inductive
                                     checks, level normalisation
```

The checker is single-pass and append-only: each declaration may mention
only declarations already accepted. Names may not be redeclared.

### 4.2 Parts and budget

| Part | Budget (non-blank, non-comment lines of Rust) | Contents |
|---|---|---|
| Expressions and levels | ≤ 1,200 | Hash-consed terms, de Bruijn instantiate/abstract, universe levels and their normalisation |
| Type inference and checking | ≤ 1,500 | `infer`, `check`, `ensure_sort`, `ensure_pi`, local contexts |
| Reduction and definitional equality | ≤ 2,200 | `whnf` (β, δ, ζ, ι, projections, quotients, literals), lazy delta with hints, η, structure η, proof irrelevance, unit-like, caches |
| Inductive types | ≤ 1,800 | Mutual and nested inductives, positivity, universe checks, recursor generation, structure detection |
| Literals and primitive validation | ≤ 600 | `Nat` operations on literals, `String` expansion, the primitive-definition checks of 3.3 |
| Export reader | ≤ 600 | The pinned lean4export format (4.3) |
| **Total** | **≤ 8,500** | |

Outside the crate but in the trusted base, counted separately and reviewed
with the same care:

- **`bignum-core`, the surface used**: addition, subtraction, multiplication,
  division with remainder, gcd, power, comparison, and the bitwise operations,
  shifts and `log2` that `Nat` acceleration needs. The last group does not
  exist in `bignum-core` today and is added there (KRN-002a), tested against a
  naive reference.
- **`lean-kernel-ffi`**, the C-ABI and WASM wrapper (≤ 200 lines). It needs
  `unsafe` for raw pointers, so it is a separate crate; the kernel crate keeps
  `#![forbid(unsafe_code)]`.

The 8,500-line figure is an estimate made before reading the precedents in
detail; nanoda_lib and Lean's C++ kernel are of the same order and lean on
existing term and number libraries. The budget is a CI gate, revisited **once**
after KRN-002's feasibility reading, and then only by owner decision.

`#![forbid(unsafe_code)]`, no I/O in the checker, no feature flags, literate
source (repository rule 14): every reduction rule and every inductive-type
check is explained beside its code, with what goes wrong if it is missing.

### 4.3 The interface: Lean's export format

The kernel's only input is a **declaration export** in the format written by
lean4export, at a pinned version:

- Lean's toolchain writes it, so any Lean library (`Init`, `Std`, Mathlib) can
  be checked (LEAN00 L1);
- our Rust tools write it too, through a small untrusted writer
  (`lean-export-writer`), so proofs for Warrant claims need no Lean toolchain
  at all;
- any program in any of the repository's languages can write it, and the
  kernel is also built as a C-ABI library and as WASM (`lean-kernel-ffi`) with
  one entry point,
  `check(export bytes, trusted-base exports) → accepted declarations or error`.
  The wrapper decides what is reported as accepted, so it **is trusted** (4.2),
  and it may not filter, rename or summarise declarations.

The reader refuses anything it does not understand. A format version it does
not know is a hard error.

**The format, and an owner rule it collides with.** Recent lean4export
versions write newline-delimited JSON; older ones wrote a compact line format.
Which one the pin uses is fixed in KRN-003. A JSON reader in the trusted base
meets the owner's rule that lexers and parsers are grammar-driven, never
hand-written. Two ways out, owner question Q9: the reader is generated from a
grammar by `grammar-tools` (which then joins the trusted base, with its
generated code counted), or the kernel reads a minimal line format of our own,
produced from lean4export's output by an **untrusted** grammar-driven
converter whose mistakes the kernel would reject (a converter that drops or
alters a declaration yields a different environment, which the oracle
comparison and the goal check catch).

**Unsafe declarations.** lean4export may include `unsafe` declarations. The
reader skips and lists them; no safe declaration may refer to one, and the
kernel rejects one that does. "Every declaration accepted" in K-M0 means every
safe declaration.

**The oracle's harness.** lean4export only writes. Having the official kernel
check *our* exports needs a small replay program in Lean that adds each
declaration with `Lean.Environment.addDecl`. It is oracle tooling, run at the
pin, never part of a gate (KRN-004).

> **Amendment, 2026-11-24 (owner decision; [VER32](VER32-proof-interchange-format.md)).**
> The kernel's input is **not** lean4export's format. It is the repository's own
> Proof Interchange Format (PIF), a neutral, versioned, line-oriented format;
> the kernel logic of §3 is unchanged. The lean4export reading described above
> moves into an *untrusted* Lean→PIF converter (VER32 PIF-008), and the reader in
> the §4.2 row "Export reader" becomes the PIF reader (VER32 section 5). The
> "interface" paragraphs above are kept as the record of the earlier design; where
> they conflict with VER32, VER32 governs. The oracle harness of this section now
> replays PIF (VER32 PIF-010). VER32 also tightens §4.5's axiom classification: a
> logic axiom counts as logic base only if the kernel validated its type, and a
> claim file may declare no axiom of its own (VER32 1.4, rules A1 to A3).

### 4.4 Pinning

The Lean version (toolchain tag), the lean4export version and the export
format version are pinned together in one file. Lean's kernel changes rarely,
but it does change (new literal accelerations, for example). Moving the pin is
its own PR, with the corpora of section 12.5 rerun.

### 4.5 Axioms and oracles

The kernel accepts any `axiom` declaration, as Lean does, and the checker
classifies every axiom a theorem depends on (section 11):

| Axiom | Classification |
|---|---|
| `propext`, `Classical.choice`, `Quot.sound` | Logic base, listed under `kernel.logic.consistent` |
| `Lean.ofReduceBool`, `Lean.trustCompiler` | **Oracle**: trusts compiled code to evaluate a boolean. Listed as `tcb.lean.reduce-bool` (bucket (c)); our kernel implements `Lean.reduceBool` only if LEAN00 L4 supplies an evaluator, and then the evaluator is the gap |
| `sorryAx` | Forbidden in any claim's footprint |
| `Warrant.Assume.<id>` | A formal VER13 assumption stated as an axiom; must match the registered statement of `<id>` exactly (section 11.1) |
| Anything else | Rejected for claims |

**K-M1 and K-M2 use no oracle.**

### 4.6 Precedents, studied, not copied

Owner direction: study nanoda_lib (an independent Lean 4 type checker in
Rust) and lean4lean (Lean's kernel reimplemented in Lean, with partial proofs
of its soundness). The kernel is written fresh, from Lean's documented type
theory and these designs, for two reasons: the repository has no third-party
code, and independence is the point of a second implementation. Their test
corpora of tricky declarations, and the official kernel, are used as
differential oracles (section 12.3).

---

## 5. The library: reuse, then extend

### 5.1 What we reuse

Once K-M0 checks Lean's core library export, its declarations are in the
environment and may be used. For the ct-compare milestones that means:
`Nat`, `List`, `Array`, `Option`, `Prod`, `Fin`, `BitVec`, `UInt8`, `UInt64`
and their lemmas, and `decide`. Every reused declaration was checked by our
kernel; none is trusted because Lean's toolchain produced it.

### 5.2 What we write

A small Lean library, `Warrant` (source in `code/packages/lean/warrant-lib`),
holding:

| Module | Content |
|---|---|
| `Warrant.WIR.Syntax` | WIR as a Lean inductive type (VER25 §4) |
| `Warrant.WIR.Semantics` | The clock-indexed semantics (section 6) |
| `Warrant.WIR.Lockstep` | The generic relational theorem (section 8.2) |
| `Warrant.WIR.Invariant` | The generic unary invariant theorem (section 9.3) |
| `Warrant.WIR.Contract` | Modular call contracts, later (section 8.5) |
| `Warrant.SAT` | LRAT proof terms or the reflective LRAT checker (section 9.5) |

Estimated 5,000 to 10,000 lines of Lean. The spike's relational core alone
(semantics, step lemmas, relation, obligation lemmas, `lockstep_obs`) is about
1,200 lines for a simplified WIR; full WIR, calls, the unary side and the
Rust semantics of K-M2 come on top. Still a fraction of the HOL estimate,
because the core library is reused.

### 5.3 Who elaborates our Lean source

Lean source needs an elaborator before the kernel sees it. Until LEAN00 L3
exists, the **official Lean toolchain at the pin** elaborates `warrant-lib`
and writes its export. This is safe for trust (the export is re-checked by our
kernel, so the toolchain is an untrusted producer) but it is a third-party
tool in a build path, which the repository otherwise refuses. Section 16 Q2
asks the owner to accept it on the same terms as Z3 in VER00 §7.5 (a
development-time tool, never a dependency of a check), with the plan that
LEAN00 L3 replaces it. The checked exports' digests are committed; anyone can
regenerate and compare.

**Per-claim proofs** (section 9) are generated as **Lean source**: the Rust
VC builder writes a `.lean` file per claim (statements plus tactic scripts
from a small fixed vocabulary), and the pinned toolchain elaborates it. The
alternative, writing raw kernel terms from Rust, means building implicit
arguments, universe levels and instance terms by hand: a small elaborator in
its own right. It is kept as the fallback if Q2 is answered "no", with its
size budgeted then.

**Environment extensions.** An export carries kernel declarations only.
Instances, `@[simp]` sets, reducibility, structure and projection information,
coercions and notation live in Lean's environment extensions (in `.olean`
files). So even an in-house elaborator (LEAN00 L3) cannot elaborate against
`Init` from an export alone: it needs either a snapshot of those extensions,
dumped by the toolchain at the pin as data, or to elaborate `Init` from source
(which uses features far outside any core elaborator). LEAN00 §5 records this;
it means the official toolchain stays in the build path, as a producer of data,
until LEAN00 L5 (owner question Q2).

---

## 6. Decision D3: the meaning of WIR, written in Lean

### 6.1 Why the semantics must live in the kernel

VER00 §7.5.5 already says WIR "has a formal semantics stated in the proof
kernel's logic". It does not yet exist. This spec makes it exist, and makes it
**normative**: where the Lean definition and the Rust reference interpreter
disagree, the Lean definition is what WIR means, and the interpreter has a
bug.

### 6.2 A small-step semantics with structural fuel

The semantics is an **instruction-level small-step machine with an explicit
frame stack**, driven by a function defined by **structural** recursion on a
step counter (in the style of CakeML's clocked semantics):

```lean
structure Frame where
  fn : FnId; block : BlockId; ip : Nat     -- program point: instruction index
  vars : VarId → Option Value               -- VarId, FnId: numbers, not strings

structure State where
  stack  : List Frame      -- top of stack first
  status : Status          -- running | ret v | trap r | stuck ub
  mem    : Mem
  trace  : List Event
  steps  : Nat             -- accumulated cost, VER25 §5.1 schedule
  allocs : Nat             -- allocation counter = next provenance id
  heap   : Nat             -- bytes allocated with kind `heap`

def step : Program → State → State          -- one instruction or terminator
def run  : (n : Nat) → Program → State → State
  | 0,     _, s => s
  | n + 1, p, s => if s.status = .running then run n p (step p s) else s
```

- **Fuel counts steps, cost is accumulated.** `run` recurses on `n + 1 → n`,
  so it is structural and reduces in the kernel on concrete inputs (needed for
  the witnesses of 7.5). The cost of each step is *added* to `steps`, not
  subtracted from the fuel; an earlier draft subtracted cost from fuel, which
  is well-founded recursion and does not reduce. `warrant-lib` uses no
  well-founded recursion anywhere in the semantics.
- **One instruction per step.** `CallFunction` is an instruction in WIR
  (`ast.rs`), not a terminator, so a call pushes a frame from the middle of a
  block and the caller resumes at `ip + 1` when the callee returns. Traps and
  undefined behaviour mid-block need no special case: they end the step that
  caused them. This also matches the reference interpreter, which charges per
  instruction.
- **Termination and bounds** become one existential statement: there is an
  `n` with `(run n p s₀).status = .ret _` and `steps ≤ B`. Once a run has
  finished, more fuel changes nothing (a library lemma), so "some `n`" is the
  same as "every large enough `n`".
- **Program points, not names.** Functions and variables are numbered by the
  WIR-to-Lean embedding, so the kernel never compares strings while reducing.
  The numbering is printed with the goal (7.4).
- **Stack slots are not heap.** The lowering allocates small slots for locals
  (for example `bool_slot` for `choice as u8`, `lower.rs`). WIR's `Alloc`
  gains a kind, `stack` or `heap`, and only `heap` counts toward the `heap`
  bound, in bytes. This is a VER25 change plus a `warrant-ir` code change
  (KRN-006, KRN-006a). Without it the Warrant claim `heap <= 2 * len(a)` for
  `select_bytes` is **false at `len = 0`** as lowered today, and the first real
  check would refute it (finding protocol P12 of VER27 §7.4).
- **Events** are VER25 §5.2's `branch`, `memoryAccess` and
  `capabilityInvocation`. The observation of a run is

  ```lean
  def obs (s : State) := (s.status.kind, s.trace, s.steps)
  ```

  where `kind` forgets the returned value and the trap reason's text but keeps
  which outcome it was. The returned value is not observed: a constant-time
  comparison returns a secret-dependent boolean by design.
- **Allocation is deterministic**: a new allocation's provenance id is the
  allocation counter, so two runs that allocate in the same order get the same
  ids (needed for `select_bytes`'s fresh buffer, section 8).
- **Undefined behaviour is explicit** (`stuck`), as VER25 §3.2 requires.

### 6.3 Memory

```lean
structure Alloc where
  size  : Nat
  alive : Bool
  kind  : AllocKind        -- stack | heap
  tag   : Tag              -- the allocation site, and whether it holds a secret
  bytes : Nat → UInt8
def Mem := Nat → Option Alloc
```

- A slice argument is `(provenance, offset, length)`. Whether two slice
  arguments alias is decided by their provenances, which are inputs to the
  claim: the proof quantifies over them, so it covers aliased and disjoint
  arguments alike.
- The **tag** records where an allocation came from: an argument labelled
  `secret` or public, or an allocation site in the program, marked secret or
  public by the label inference. It is what the secret set of 8.3 is computed
  from, and `SameShape` requires equal tags in both runs.
- The **entry state** takes the allocation counter as an argument, and the
  premise of every goal includes `∀ p ≥ allocs, m p = none`: no fresh id can
  collide with an argument's provenance.

### 6.4 What the semantics leaves out

Concurrency, capability effects beyond recording the invocation, and dynamic
values. Functions that use them get no kernel proof yet and keep today's
labels.

### 6.5 Keeping the interpreter honest

The Lean semantics *computes*: on concrete inputs, kernel reduction (or LEAN00
L4's evaluator) produces the final `State`. A differential test runs the
reference interpreter and the Lean semantics on generated WIR programs and
inputs and compares results, including trace and step count. Two interpreter
changes come first (KRN-010), or the comparison fails by design: the
interpreter must report the trace and steps on every outcome (today `Trap`,
`Stuck` and `BudgetExceeded` carry none), and its step costs must follow the
same VER25 §5.1 schedule (the interpreter charges per instruction, as the
semantics now does). This test is
evidence about the interpreter. It is not part of any proof's trust.

### 6.6 An open model assumption until K-M2

Making the Lean semantics normative does not make it *right*: it is a
model, about 1,000 to 2,000 lines of Lean, that we wrote. Until K-M2 relates
it to Rust, "the Lean `run` is the WIR we intend, and WIR is what the code
does" is a gap in our own tools. It gets an id, `wir.semantics.lean-faithful`,
in **bucket (c)**, and it is one of the ids K-M1 leaves open (section 13). At
K-M2 the goal is restated over the Rust semantics (section 14.3), `run`
leaves the goal's footprint, and the id closes.

---

## 7. From Warrant claims to kernel goals

### 7.1 The elaborator

A small, separate Rust program, the **claim elaborator**, turns one Warrant
claim plus the WIR of the function it is bound to into one Lean proposition,
the **goal**. (It is unrelated to Lean's term elaborator; it translates a
fixed, small language with no unification.) The function's WIR becomes a
Lean constant whose value is the program's syntax.

Below, `entry p f args m a` is the initial state of function `f` with
allocation counter `a`, and `Pre` collects the well-formedness premises (each
slice inside a live allocation; `∀ q ≥ a, m q = none`; tags matching the
labels).

**An input contract** (`rejects`), for `decode_unsigned_bounded` in
`wasm-leb128.warrant` (`rejects max_bits > 64u32 with MALFORMED`):

```lean
-- every rejected input is refused, with no effect
∀ d o mb m a, Pre m a → mb > 64 →
  ∃ n, let s := run n leb (entry leb decodeUB [d, o, mb] m a)
       s.status = .ret (.err .MALFORMED) ∧ NoEffect m s ∧ NoRead d s
-- and the rejection is exact: no accepted input gets that answer for that reason
∀ d o mb m a n, Pre m a → mb ≤ 64 → o < len d →
  (run n leb (entry leb decodeUB [d, o, mb] m a)).status ≠ .ret (.err .MALFORMED)
    ∨ Malformed (bytes d o)
```

`NoEffect` says no capability was invoked and no caller-visible byte changed;
`NoRead d` says the trace contains no access to `d`'s allocation (the
`wasm-leb128` comment promises rejection "before it reads any byte"). Which of
these a clause demands is fixed per clause kind in the claim translator and
printed in the goal.

**Constant time**, for `ct_eq_soundness`, in words:

> For every number of steps, every pair of slice arguments, and every two
> memories that have the same shape and differ only inside allocations tagged
> secret, the observations of the two runs are equal.

```lean
∀ (n pa oa la pb ob lb a : Nat) (m₁ m₂ : Mem),
  Pre m₁ a → SameShape m₁ m₂ → AgreeOutside (secretTags m₁) m₁ m₂ →
  obs (run n ctc (entry ctc ctEq [.slice pa oa la, .slice pb ob lb] m₁ a))
    = obs (run n ctc (entry ctc ctEq [.slice pa oa la, .slice pb ob lb] m₂ a))
```

**Public inputs are shared variables; secret inputs are not.** The lengths and
provenances are the *same variables* in both runs: that is how "lengths are
public" is stated. Every **secret-labelled scalar** (`choice` of
`select_bytes`, both arguments of `eq_u64`) gets **two variables, one per run**.
If a translator shared one variable for a secret, the goal would become
`obs x = obs x`, true by reflexivity and worthless; the witness rule of 7.5
makes that impossible to prove, because it demands a witness pair that differs
in every secret input.

**Equal observations alone are not enough.** Two runs that both reach
undefined behaviour at the first step have equal observations, so a
constant-time goal is exported **together with a no-`stuck` goal under the same
premise**, and the checker requires both:

```lean
∀ n … m₁ a, Pre m₁ a → ¬ (run n ctc (entry ctc ctEq … m₁ a)).status.isStuck
```

**Termination, `no_panic` and the step bound** together:

```lean
∀ … m a, Pre m a → ∃ n, let s := run n ctc (entry ctc ctEq … m a)
  (∃ v, s.status = .ret v) ∧ s.steps ≤ 64 * la + 512
```

Heap and depth bounds are similar statements about `heap` and the maximum
stack length reached.

### 7.2 Why the constant-time goal is quantified over every step count

Quantifying the constant-time goal over every `n` makes it a statement about
every prefix of both runs. Runs that diverge and then stop early, or never
stop, are covered. It is the exact form of "traces equal so far" (section
8.3).

### 7.3 Binding the proof to the claim

This replaces the shape-checked `obligation_sha256` (`wir.proof.obligation-binding`).

1. `ca-assurance-report --check` **runs the claim elaborator itself**, from
   the `.warrant` file and the source at the pinned revision, and gets the
   goal `G`. It also lowers the source to WIR itself and builds the expected
   value of the program constant (`ctCompareWir`).
2. It builds the **environment** from a reviewed allow-list of export
   digests: Lean's core library and `warrant-lib` at the pin. Nothing else is
   loaded before the claim's export.
3. It checks the claim's export with the kernel on top of that environment.
   The claim's export **may not declare any constant that occurs in `G`**,
   except the program constant, whose value must be identical to the one
   built in step 1. A claim export that defined its own `obs` or `run` would
   therefore be rejected (redeclaring a library name is already refused by
   the kernel; declaring a fresh one that `G` uses is refused here).
4. It reads the theorem's type `T` and accepts only if `T` is identical to `G`
   (de Bruijn indices make alpha-equivalence syntactic), the no-`stuck`
   companion and the witnesses (7.5) are present, and every axiom in the
   footprint is allowed by section 4.5.

A valid proof of a different statement, or of the right statement about
different definitions, is rejected, however plausible its digest.

### 7.4 The goal is signed, so the claim elaborator need not be trusted

The claim elaborator translates meaning rather than deriving it: if it turned
`secret` into `public`, the kernel would prove the wrong thing. Under the
owner's rule (only the compiler is trusted) it cannot simply be trusted, and
proving a translator faithful to an *intention* is impossible in principle.
So the design takes it out of the trust path:

1. The elaborator prints each claim's goal as Lean source, and the assurance
   report carries the printed goals and their SHA-256.
2. The VER31 statement sign-off (§3.2) gains a field `goals_sha256`: the
   digests of the printed goals the signer read and accepts, alongside the
   `.warrant` digest it already binds. This is a proposed VER31 amendment.
3. The gate accepts a kernel proof only if its theorem's type is the goal
   whose digest was signed.

Now a wrong elaborator produces a goal that either a human rejects on reading
or whose digest does not match the signature. The `.warrant` reader and the
claim translator leave the trust base; their correctness becomes the (b) id
`spec.goal.faithful`, closed by the signature like every `spec.*` id. They
still get a budget (the translator ≤ 400 lines), literate source and a
mutation corpus (each mutation of a claim or a label must change the goal),
so that a reader can actually check what they sign.

**What a human can and cannot read.** A goal mentions the program, and the
program is a large term. At K-M1 that term is produced by the Rust lowering
and the WIR-to-Lean embedding (≤ 300 lines); no human reads it, and its
faithfulness to the source is exactly what `wir.lowering.faithful` and
`wir.semantics.lean-faithful` leave open. At K-M2 the goal instead mentions
the **source text itself** (a string literal whose digest is the source file's)
and a parser and semantics defined in Lean, so the embedding leaves the trust
base too (14.3).

### 7.5 Non-vacuity

A theorem with an unsatisfiable premise proves nothing (VOS30 §4.6). For every
goal `∀ x, P x → Q x` the proof must also export **witness theorems**:

- `∃ x, P x` with every registered hypothesis included (so jointly
  inconsistent hypotheses are caught);
- for two-run goals, a witness pair that satisfies the premise, has a
  **non-zero length** where lengths exist, and **differs in every secret
  input**: every secret byte region and every secret scalar (so an over-strong
  `SameShape` or `Pre` that forces empty or identical inputs is caught, and so
  is a secret wrongly shared between the runs);
- for input contracts, one witness input on each side of the predicate;
- one concrete witness run whose status is `ret _`, proved by kernel reduction
  of `run` on concrete inputs (so a premise that only admits failing runs is
  caught).

The checker requires all of them. This is the kernel form of VER00 §7.6.3's
vacuity rule and VER20 §3.

---

## 8. Decision D4: relational verification for two-run claims

### 8.1 The shape of the problem

Constant time is a 2-safety hyperproperty (VER20 §1): it relates two runs. Two
standard ways to prove it:

- **Self-composition** (Barthe, D'Argenio and Rezk): run the program twice in
  sequence and prove a one-run property of the composite. Simple, but the
  second loop's invariant must describe the whole first run.
- **Product programs** (Barthe, Crespo and Kunz): run the two copies in
  lockstep, so one invariant relates the two states at matching points.

For constant-time code the product is natural: the property is precisely that
the two runs take the same control path, so they *can* run in lockstep.

### 8.2 Lockstep, proved once

For a relation `R` on pairs of states:

```lean
def Lockstep (p : Program) (R : State → State → Prop) : Prop :=
  ∀ s₁ s₂, R s₁ s₂ →
    s₁.status.kind = s₂.status.kind ∧            -- R carries status, both sides
    (s₁.status = .running →
      let s₁' := step p s₁; let s₂' := step p s₂
      newEvents s₁ s₁' = newEvents s₂ s₂' ∧ s₁'.steps - s₁.steps = s₂'.steps - s₂.steps
        ∧ R s₁' s₂')
```

`step` runs one instruction or terminator of the top frame (a call pushes a
frame, a return pops one, section 6.2); `newEvents` is the list of events it
appended, including, for a terminator, which successor it chose.
`warrant-lib` proves, **once**, by induction on `n`:

```lean
theorem lockstep_obs (R) (h : Lockstep p R) :
  ∀ n s₁ s₂, R s₁ s₂ → obs (run n p s₁) = obs (run n p s₂)
```

plus the entry rule (`Pre → R` at the entry state). Calls need no extra rule,
because a call is a step on the frame stack (8.5). Because `R` holds *at every
program point*, loops need no separate treatment: a loop is a cycle in the
control-flow graph, and induction on `n` goes round it as many times as the
input requires. **This is the step that turns `bounded(128)` into all
lengths.** No unrolling depth exists. The per-step obligations are generated
and proved one program point at a time, and grouped by basic block for
reading (8.4).

`R` implies the two states are at the same program point, so "both branch the
same way" is part of the obligation that a terminator's events are equal. That obligation is the
guard-agreement condition of lockstep product programs, and it is exactly
CT01's promise that control flow does not depend on secrets.

### 8.3 The invariant: "public things equal, traces equal so far"

The relation is generated, not written:

```lean
R s₁ s₂ :=  s₁.status.kind = s₂.status.kind
          ∧ List.Forall₂ (fun f₁ f₂ => f₁.fn = f₂.fn ∧ f₁.block = f₂.block ∧ f₁.ip = f₂.ip
                ∧ ∀ v ∈ Public f₁.fn f₁.block f₁.ip, f₁.vars v = f₂.vars v)
              s₁.stack s₂.stack
          ∧ SameShape s₁.mem s₂.mem
          ∧ AgreeOutside (secretTags s₁.mem) s₁.mem s₂.mem
          ∧ s₁.trace = s₂.trace ∧ s₁.steps = s₂.steps
          ∧ s₁.allocs = s₂.allocs ∧ s₁.heap = s₂.heap
          ∧ WfState s₁ ∧ WfState s₂
```

- `Public f b i` is the set of variables live at program point `(b, i)` of
  function `f` that the **label inference** marks public: the existing taint propagation (VER25
  §2.2), untrusted. If it calls a secret-dependent variable public, the
  obligation "public variables equal afterwards" fails and there is no proof.
  It can make the proof fail; it cannot make it lie.
- `secretTags m` is the **set of allocations tagged secret, which grows during
  the run**: the argument allocations labelled secret at entry, plus every
  allocation made at a site the label inference marks secret (`acc`,
  `bool_slot`, the output `vec_buf` of `select_bytes`). The tag is stored in
  each allocation (6.3), allocation ids come from a counter that is equal in
  both runs, and `SameShape` requires equal tags, so the set is the same on
  both sides and the kernel checks it like everything else. A site wrongly
  marked public makes a later obligation fail; it cannot make one pass.
- `WfState` carries the framing facts that `no_panic` and memory safety need:
  every slice argument lies inside a live allocation, the allocation counter
  is above every live provenance, and live allocations keep their sizes.
- `trace` and `steps` equal is **"traces equal so far"**. It is preserved
  because each step's events and cost are equal.
- Secret values (`acc`, the loaded bytes, `mask`) appear nowhere in `R`.

This is the idea behind constant-time type systems, used as an invariant the
kernel checks, so no type system has to be trusted or proved sound as a whole.

### 8.4 Worked example: `ct_eq_equal_length`

After lowering, the loop has a header `H` (test `i < n`), a body `B` (bounds
checks, three volatile loads, one store, `i := i + 1`) and an exit `X` (load
`acc`, compare with zero, return).

| Block | Public variables | Obligations of its program points |
|---|---|---|
| `H` | `a`, `b` (provenance, offset, length), `n`, `i`, `acc_ptr` | `i₁ = i₂ ∧ n₁ = n₂ → (i₁ < n₁) = (i₂ < n₂)`: same successor, same `branch` event, same cost |
| `B` | same | Load addresses `(prov_a, off_a + i)`, `(prov_b, off_b + i)`, `(prov_acc, 0)` and the store address are equal because their parts are; `i + 1` is public; `acc`'s value changes but is not in `R`; the store goes into `acc`'s allocation, which is in `Secret s`. There are **no bounds-check branches** in WIR: `lower.rs` emits a pointer offset and a load, and an out-of-bounds access is `stuck`, on both sides alike because address, size and liveness agree |
| `X` | same | The load of `acc` has equal address; `acc == 0` is secret but is only returned, and the returned value is not observed |

Each obligation is about one block, over symbolic states constrained only by
`R`. None mentions a concrete `i`.

**What the spike added to `R`** (KRN-S0, REPORT.md F2):

- **Freshness** (`∀ q ≥ allocs, mem q = none`) is a field of `R`, not only an
  entry premise, because every `alloc` must show its new id was unused.
- **Pointer tag facts.** The lowering keeps the loop counter `i` in a stack
  slot and reloads it on every trip, so "`i` is public" holds only because
  that slot is public-tagged. The label table lists, per program point, which
  variables point into public-tagged and which into secret-tagged
  allocations, and `R` carries those facts.
- **Call results are secret in the caller** unless the callee's table says
  otherwise.
- **No `WfState` in the relational `R`**; it is for the unary claims.
- **`Lockstep` simplifies**: with trace and step equality inside `R`, the
  separate event and cost clauses of 8.2 are redundant, and `step` is total.
- **Stuckness must not depend on secrets.** The semantics makes variables
  total and ill-typed operations return a default, and the WIR type checker
  rules both out for well-typed programs (KRN-006 states it); otherwise `R`
  would need "same value shape" for secret values.
- **Not yet proved:** a `call` as a lockstep step (it needs the callee's label
  table). KRN-007 does it.

### 8.5 Calls: frames, not inlining

Today the inliner exposes `ct_eq_equal_length` to `ct_eq`'s proof
(`wir.inline.sound`). The kernel route does not inline. Because the semantics
has an explicit frame stack (6.2), a call is one step that pushes a frame
(from the middle of a block, since `CallFunction` is an instruction), and `R`
relates the two stacks frame by frame, including each frame's resume point
(8.3). So the callee's program points are simply more program points with
their own obligations, proved once per function and reused at every call
site. No contract theorem is needed for soundness.

**Later, for scale:** modular relational contracts, so a callee's proof does
not have to be re-checked per caller. A contract must then return the whole
post-state relation the caller needs (`SameShape`, agreement outside the
secret set, equal `allocs` and `heap`, and which new allocations are secret),
not just "equal observations". Not needed for ct-compare.

The **depth bound** needs more than an acyclic call graph: it needs the
invariant `stack.length ≤ D` at every reachable state. For an acyclic call
graph that is the longest call chain, computed from the program's syntax and
proved by `decide` on the concrete call graph, then carried as a conjunct of
the unary invariant (9.3). Recursion is future work.

### 8.6 What the lockstep rule cannot do

Runs that legitimately take different paths and re-join with equal
observations (a secret branch with padded arms) fail lockstep. CT01 forbids
such code, so for constant time this is the right refusal. Other two-run
claims that need asynchronous products are out of scope and fail closed.

### 8.7 Relation to VOS30

VOS30 §3.6 defers "relational cut points" because they need "a composition
rule for relations". `lockstep_obs` is that rule for cuts at block
boundaries. Adopting it is VOS30's choice; this spec does not change VOS30.

---

## 9. Verification conditions and how they are discharged

### 9.1 The pipeline

```
   .warrant + source ──► claim elaborator ──► goal G               (trusted, small)
   WIR ──► label inference ──► R (untrusted guess)
       ──► VC builder: one obligation per block (untrusted)
       ──► discharge: kernel reduction (rfl) | decide | Constraint VM + LRAT terms
       ──► assemble with lockstep_obs / invariant theorems + contracts
       ──► export (lean-export-writer) ──► kernel ──► ⊢ G
```

### 9.2 Per-step obligations: step lemmas, generated Lean proofs, and a spike first

An earlier draft said the kernel's own reduction would close each block's
obligation by `rfl`, running the concrete program on a symbolic state. That
does not work, and the reason matters for the design. Lean's kernel reduces
until it meets something it cannot decide: at a loop header the step branches
on `decide (i < n)`, and with `i` and `n` free variables `Nat.ble i n` does not
reduce (literal acceleration applies only to literals). The result is a stuck
`Decidable.rec`, not an explicit successor state. The same holds for every
bounds check whose truth comes from a hypothesis: a hypothesis never makes an
`if` reduce.

So the design is:

1. **Step lemmas, proved once in `warrant-lib`.** For each instruction and
   terminator kind, a lemma states the successor state under a side
   condition: `step_branch_true (h : eval c s = true) : step p s = …`,
   `step_load (h : InBounds s ptr w) : step p s = …`, `step_call`, `step_ret`,
   and so on. Each is proved by unfolding `step`, once.
2. **One obligation lemma per instruction kind, proved once.** Each says:
   if a decidable side condition on the label table holds at this program
   point, one step preserves `R`. The side condition is a few list
   inclusions (which variables are public, which point into public-tagged and
   which into secret-tagged allocations).
3. **The per-program part is data plus one `decide`.** The VC builder emits
   only the program constant and a **label table** (both untrusted). A single
   bounded `decide` checks the side condition at every program point, and the
   generic `lockstep` theorem does the rest. No proof text is generated for
   the relational case, so the implicit-argument and instance problems of
   generating Lean do not arise. (Generated Lean proofs, elaborated by the
   pinned toolchain under Q2, remain the plan only where a claim does not fit
   this shape; whether the unary claims of 9.3 do is tested next, KRN-012.)

**Outcome of the feasibility spike (KRN-S0, 2026-11-23): feasible, with
changes.** In official Lean 4.35.0, core only, the spike proved constant time
for `ct_eq_equal_length` for every length and every step count, with axioms
`[propext, Quot.sound]` and no `sorry`; a clean build takes about 4 seconds.
It found the data-plus-`decide` shape above, which replaces the earlier plan
of generated per-step proofs. Report:
[`code/learning/lean/krn-s0-spike/REPORT.md`](../learning/lean/krn-s0-spike/REPORT.md);
its findings are folded into 6.2, 8.2 to 8.4, 10.2 and the backlog.

### 9.3 Unary claims: input contracts, termination, bounds

The same machinery with a one-state invariant `I`:

```lean
theorem invariant_run (I) (h : InvStep p I) (hpost : ∀ s, I s → s.finished → Post s) :
  ∀ n s, I s → (run n p s).finished → Post (run n p s)
theorem terminates_by (I) (μ : State → Nat) (h : InvStep p I)
    (hdec : ∀ s, I s → s.running → μ (step p s) < μ s) :
  ∀ s, I s → ∃ n, (run n p s).finished
```

**Input contracts.** For a `rejects P with E` clause the invariant is short:
on the path where `P` holds, the function reaches its rejecting return before
any effect, and `NoEffect`/`NoRead` are conjuncts of the invariant along that
path. The exactness half (an accepted input never gets `E` for that reason)
is an invariant on the other path: once the check has passed, no later step
returns `E` unless the input is malformed in the model's sense. For
`wasm-leb128`'s `max_bits` check this is a handful of program points before the
first read.

**Counting loops** (ct-compare) use a template:
`WfState s ∧ i ≤ n ∧ n = la ∧ la = lb ∧ steps = c₀ + c₁ * i ∧ heap = h₀ ∧
stack.length ≤ D`, where `c₁` is the cost of one trip round the loop, computed
from the step costs. `WfState` and `n = la = lb` (the caller's length check)
make every access provably in bounds, so no step is `stuck` and `no_panic`
follows. The step bound
`64 * len + 512` follows from `c₀ ≤ 512` and `c₁ ≤ 64` by a small
**linear-arithmetic certificate procedure** over `Nat` (KRN-012a; Lean's own
`omega` needs LEAN00 L4). *(KRN-012a is a `Nat` procedure for the step and heap bounds
here. It is **not** the `Int`/`Pif.Rat` Farkas engine of VER34 (CVM-217), which is that
spec's own; neither reuses the other. VER34 13.1.)* Termination is the ranking function `n - i` with
`terminates_by`. The heap bound comes from the allocation sites, the depth
bound from the call graph (8.5).

Two front-end facts the template relies on, checked in KRN-013:

- **`ct_eq_fixed<N>` is monomorphised.** The lowering produces one WIR
  function per width, and the Warrant file's claims are per width (0, 1, 2, 4,
  16), so each width gets its own proof; all of them reuse the obligations of
  `ct_eq_equal_length`, whose length is symbolic. A proof "for every `N`"
  would need a WIR function parametric in `N`; not needed now.
- **Symbolic-size allocation.** `select_bytes` allocates `len` bytes. The
  VER19 executor refuses symbolic allocation sizes today, so as a predictor it
  needs that case added; the kernel side has no such limit.

### 9.4 Bit-level facts

Bit-level facts (`x ^^^ y = 0 ↔ x = y` on `UInt8`) are proved from Lean core's
`BitVec` lemmas, not by `decide`: enumerating 65,536 pairs through `Fin` and
`BitVec` instances in the kernel is impractical, and the needed `Decidable`
instance may not exist. **ct-compare's constant-time, bound, `no_panic` and
input-contract obligations need no bit-level facts at all** (they are about
addresses, branches and counts), so SAT and LRAT (9.5) are not on K-M1's
critical path. Bit-level facts first matter for the functional claims
(K-M1c) and for decoders such as `wasm-leb128`.

### 9.5 SAT and Constraint VM answers as proof terms

For propositional or bit-vector goals too large for `decide`, the Constraint
VM (LANG24) and `cdcl-sat` search, and their answer becomes a proof term in
one of two ways:

1. **LRAT as terms.** `cdcl-sat` emits LRAT (it knows each learnt clause's
   antecedents), or an untrusted DRAT-to-LRAT elaborator adds the hints. The
   VC builder turns each RUP step into a short term built from a few
   resolution lemmas in `Warrant.SAT`. Linear in the certificate; no
   reflection.
2. **Reflection.** A verified LRAT checker written in Lean (in `Warrant.SAT`,
   proved sound once) is run *by kernel reduction* on the certificate:
   `checkLRAT cnf cert = true` by `decide`. Shorter terms, more kernel time.

Both avoid `Lean.ofReduceBool`. Which is faster in our kernel is measured
(KRN-009). Either way `tcb.drat.checker-correct` and
`tcb.bitblast.encoding-correct` are closed for kernel proofs.

**Cross-spec follow-up, 2026-11-24 (VER34 section 13.1).** (1) **Reflection is a
hypothesis, not a result.** Whether the kernel's limits (call depth 200, term depth
1,000, 2,000,000 steps) and the chunked data encodings fit a verified LRAT checker run
by reduction is **measured by VER34's spike CVM-S1**, before any library work; the term
route (option 1) is a P1 fallback (CVM-153). (2) The verified LRAT checker is **not** a
`warrant-lib` Lean file: it is `Pif.Lrat` in VER34's proof-side library (CVM-150, CVM-151,
`checkLRAT_sound`), so KRN-009 now means **LRAT emission by `cdcl-sat` only**. (3) **KRN-019
(the independent second checker) and the PIF-010 replay harness must run the reflective
proofs under the named configuration `cvm-smt-v1`, with `lits`** (VER32 3.5; VER34
CVM-177), or `kernel.checker.correct` cannot close for those claims.

### 9.6 Why the proof is no longer trivial, stated precisely

"Non-trivial" was never the goal; *derived from the program's meaning* is.
After this spec every fact the old engine computed in Rust (equal step counts,
equal branch directions, equal offsets) is a kernel-checked consequence of the
definition of `run` and the program's WIR, and every per-step obligation is
over symbolic state, so it holds for every iteration of every loop. Some will
be easy. That is fine, because the easy part happens inside the kernel, not in
an engine we trust. `certificate_kind: "trivial-miter"` is retired for kernel
proofs; the new kind is `"lean-export"`.

---

## 10. How proofs are produced

### 10.1 Automation first

For a claim, the driver tries, in order:

1. the generated relational invariant (8.3) for two-run claims;
2. the template invariants of 9.3 for bounds, termination and input contracts;
3. for each program point: the matching step lemma, its side condition from
   the invariant, then `rfl`/`congrArg`; for `Nat` goals, the linear-arithmetic
   certificate procedure (KRN-012a); for bit-level goals, core `BitVec`
   lemmas, then the Constraint VM and SAT with LRAT terms.

K-M1 commits to testing the hypothesis that this succeeds for every one of
ct-compare's constant-time, step, heap, depth, termination and `no_panic`
claims.

### 10.2 The escape hatch: Lean proofs

When automation fails, a human or an agent writes the proof in **Lean**: a
file `proofs/<claim-id>.lean` next to the package, stating the invariant and
proving the obligations with ordinary Lean tactics. Example, the functional
claim `eq_agrees`, whose invariant automation cannot guess:

```lean
theorem eq_agrees_inv : LoopInv ctEqEqualLength hdr fun s =>
    (accOf s = 0) ↔ ∀ j, j < iOf s → byteAt s pa (oa + j) = byteAt s pb (ob + j) := by
  apply LoopInv.intro
  · intro s hs; exact entry_acc_zero hs          -- before the loop: both sides true
  · intro s hs hlt
    rw [loop_body_effect hs hlt, UInt8.or_eq_zero_iff, UInt8.xor_eq_zero_iff]
    exact forall_lt_succ_iff (hs.2) hlt
```

The example sticks to the tactics LEAN00's core elaborator will support
(`intro`, `exact`, `apply`, `rw`, `constructor`, `cases`, `induction`, `rfl`,
`decide` and `simp only` with named lemmas), so that hand-written proofs do not
have to be rewritten when the official toolchain leaves the path. `omega`,
full `simp` and `a[j]!`-style notation with automatic side goals are avoided
in `warrant-lib` and in escape-hatch proofs for the same reason.
The spike (KRN-S0) showed that real proofs also lean on bookkeeping tactics
(`have`, `obtain`, `by_cases`, `show`, `subst`, `unfold`, `<;>`) and on one
way to reduce an `if` once a hypothesis decides its test; LEAN00 §5.1's subset
includes them.

- Proofs are **untrusted**: whatever produced them, the kernel re-checks the
  export.
- Agents get Lean's error messages, which name the failing goal.
- Proofs are versioned with the package; CI regenerates the export; a proof
  that stops working fails the build.
- Until LEAN00 L3 exists, the pinned official toolchain elaborates these
  files (section 5.3, owner question Q2).

### 10.3 Fail closed

If neither automation nor a Lean proof works, the existing VER19/VER20
pipeline still runs, and the claim keeps the label it would have had
(`bounded(k)` or lower). The kernel route only ever raises a label when it
succeeds, and never rounds up (VER00 §7.6.5).

---

## 11. How assumptions appear

### 11.1 Four sources, all computed

The axiom list of a claim's theorem is computed by the checker, never declared
in a manifest. It extends Lean's `#print axioms`.

| Source | How it is found | Example |
|---|---|---|
| Logic axioms | Axioms the theorem transitively depends on (Lean's `#print axioms`) | `propext`, `Classical.choice`, `Quot.sound` |
| Oracle axioms | The same walk, classified by section 4.5 | none in K-M1 and K-M2 |
| Formal assumptions | Each hypothesis of the goal, and each `Warrant.Assume.<id>` axiom, must match the registered Lean statement of a VER13 entry; the id is listed | an environment precondition, if a claim has one |
| Definition footprint | Every constant the *goal* mentions is traced through the environment to its definitions. Each **trusted definition** (one whose meaning a human must accept: `run`, `obs`, `SameShape`, the WIR constant) is registered with the VER13 ids that say why it means what we think | `obs` ↦ `spec.obs-model.complete`; `run`'s cost ↦ `wir.cost.step-model` |

The footprint is the important part: most catalogue assumptions are not
formulas a proof uses but statements that a *definition* matches reality.
The footprint shows them automatically. A fixed per-stack baseline (for
example `cc.version-pinned`) is added, as today.

### 11.2 The kernel's own ids, under "everything else has to be verified"

A kernel moves trust; it cannot remove it. Every proof assistant ends in
something unproved: Lean's users trust Lean's kernel and the consistency of
its logic. Under the owner's rule only the compiler is trusted, so this spec
must say, for each thing the kernel route itself relies on, how it is
closed. Each new id needs a bucket in `buckets-v1.json` (an unbucketed id fails
VER31's check, and defaults to (c)). Each bucket move is an **amendment to
VER31 §3.4 with the owner's sign-off**; the column "Proposed" is a proposal,
not a decision.

| New id | Statement | Default | Proposed | How it would be closed |
|---|---|---|---|---|
| `kernel.checker.correct` | `lean-kernel` accepts only declarations Lean's type theory accepts | (c) | (c) | Two routes, owner question Q6: **R1 independent agreement**, evidence kind `independent-kernel-agreement`: every export the gate relies on is also accepted by the official Lean kernel at the pin *and* by our second checker (12.3), and the agreement records are checked evidence; **R2 a verified kernel**, the lean4lean direction (12.6): `lean-kernel`'s algorithm proved sound against a formal model of Lean's typing rules, the proof itself checked by both kernels. R1 is achievable soon; R2 is years. Until the owner accepts one, **no claim can be `proved`** |
| `kernel.logic.consistent` | Lean 4's type theory with `propext`, `Classical.choice` and `Quot.sound` proves no falsehood | (c) | **(a)**, as a mathematical floor | It cannot be closed: by Gödel's second incompleteness theorem no consistent system this strong proves its own consistency. Lean's consistency is known relative to ZFC with inaccessible cardinals (a paper proof). In (c) it blocks `proved` forever, for every claim. The proposal is (a), alongside `phys.*`, as the one mathematical assumption the programme rests on |
| `spec.goal.faithful` | The printed Lean goal says what the Warrant claim means | (b) | (b) | **Replaces the earlier idea of trusting the claim elaborator.** The VER31 statement sign-off gains a field `goals_sha256` binding the printed goal of every claim (7.4). A human signs the goal itself, so the `.warrant` reader, the claim translator and the WIR-to-Lean embedding no longer need to be trusted or verified: if they produce the wrong goal, the signed digest does not match |
| `wir.semantics.lean-faithful` | The Lean `run` is the WIR we intend, and the lowered WIR is what the code does | (c) | (c) | Closed at K-M2, when the goal is restated over the Rust semantics and `run` leaves the footprint (6.6, 14.3) |
| `src.rust.lean-semantics.faithful` | Our Lean semantics of the Rust subset, *including the events and costs it assigns*, says what Rust does | (c) | **(b)**, owner question Q3 | Like `spec.*`, it is a statement a person must read and accept: signed with the goal. It inherits what `wir.lowering.faithful` and `wir.cost.step-model` carried (14.3). Never (a): it is our model of Rust, not Rust |
| `tcb.lean.reduce-bool` | The evaluator deciding `Lean.reduceBool` is correct | (c) | (c) | Only if `Lean.ofReduceBool` is ever used; not in K-M1 or K-M2 |

What closes in the gate itself: the comparison "the theorem's type is the
goal" (7.3) is a kernel operation (checking `thm : G` is type checking), and
the axiom footprint is a kernel query, so the proof-relevant part of the
report checker moves into the kernel and is covered by
`kernel.checker.correct`. The rest of `tcb.report.checker-correct` (tier
arithmetic, JSON, digests) stays (c) by VER31 §3.4 and is VER31's to close.

### 11.3 Report fields

```json
"certificate_kind": "lean-export",
"export": "sha256:…",
"environment": {"lean_core": "sha256:…", "warrant_lib": "sha256:…"},
"goal": "<the elaborated goal, printed as Lean>",
"witness": "sha256:…",
"checkers": ["lean-kernel 1 (rust)", "lean 4.x.y kernel (differential, pinned)"],
"axioms": {
  "logic": ["propext", "Classical.choice", "Quot.sound"],
  "oracles": [],
  "formal": [],
  "definitions": {"Warrant.WIR.obs": ["spec.obs-model.complete"]},
  "by_bucket": {"a": [], "b": [], "c": []}
}
```

Buckets are joined by the VER31 gate from the signed bucket file, not
written by the report; the field above is what the gate prints. The gate
refuses `proved` if any (b) id is unsigned or any (c) id is open (VER31).

---

## 12. How the kernel itself is trusted

A Lean 4 kernel cannot be "read in full in one sitting" the way a HOL kernel
can. Trust comes from four things together.

### 12.1 Literate source within a budget

Section 4.2's budget, enforced in CI, and literate source: each reduction and
each inductive-type check explained beside its code with the failure it
prevents. A reader should be able to read it in full over a few days.

### 12.2 Adversarial corpora

- **Lean's own tests** of the kernel (declarations that must be rejected) and
  the adversarial export corpora that independent checkers are tested against,
  imported as data at the pin.
- **Our own negative corpus**, one or more cases per check in sections 3.3 and
  4.2: a non-positive inductive, a universe-inconsistent declaration, a
  recursor rule with a wrong argument, a definition whose value has a
  different type, a literal operation with a wrong result, a proof-irrelevance
  case that confuses `Prop` with `Type`, a redeclaration.
- **Mutation testing**: deleting or weakening any check in the source must
  make some corpus case pass that should fail.

### 12.3 Independent checking

- **Differential oracle (from K-M0)**: the official Lean kernel at the pin
  checks the same exports through a small replay harness (4.3); any
  disagreement fails CI and is a bug in one of them until shown otherwise.
  Our kernel's verdict is the gate. For a claim to reach `proved` under route
  R1 (11.2, Q6), the oracle's acceptance of **that claim's export** must be
  recorded as evidence; its absence is never silent, it leaves
  `kernel.checker.correct` open for that claim.
- **Our own second checker (later)**: an independent implementation, written
  from Lean's type theory and the export format by a different agent without
  reading `lean-kernel`, in a different language (TypeScript recommended, for
  the same reasons as VER09a's independent SAT lane). It is large (thousands of
  lines), so it is scheduled after K-M1 (KRN-019), not before.

### 12.4 What the tiered-release spec must agree on

Coordinated with the session that wrote VER31. Agreed: VER31's buckets
`a|b|c` and its assignment are used unchanged; K-M1 lands on VER31's rung 3,
`checked`. Proposed amendments, each needing the owner's sign-off and a
signed `buckets` revision:

- bucket entries for the new ids of 11.2 (they default to (c) until signed);
- the moves of Q5 and Q7 (11.2, 13);
- a statement sign-off field `goals_sha256` (7.4, Q8);
- a certificate form `lean-export:sha256:<hex>` beside `drat:sha256:<hex>` in
  VER31 §4, verified by `--check` through the kernel;
- an evidence kind `independent-kernel-agreement` if Q6 chooses route R1.

### 12.5 Sabotage controls for proofs

The existing `proofneg/secret-branch` and `proofneg/bound-too-small`, plus a
secret-indexed load and a balanced secret branch (8.6), must all fail to
produce a kernel proof, and the failure must name the block. A corpus of false
goals must defeat the automation.

### 12.6 Not now: verifying the kernel in itself

lean4lean shows the direction: the kernel written in Lean and proved sound
against a model of Lean's type theory. That is the far end of the "Lean-like"
arc and a possible P3 item after K-M2.

### 12.7 Diverse toolchains

`lean-kernel` is built with the pinned rustc; the differential oracle is
Lean's C++ kernel built by its own toolchain, so a rustc miscompilation of
the *kernel* would show up as a disagreement. The Rust-only components (the
`.warrant` reader, the claim translator, the embedding) are covered instead by
the signed goal (7.4): a miscompiled translator yields a goal whose digest does
not match. `tcb.rust-toolchain.trusted` is in bucket (a) by the owner's
decision (VER31 §3.4).

---

## 13. Disposition of every own-tool id in the ct-compare report

The ct-compare report lists 26 ids in the families `wir`, `tcb`, `spec` and
`src`, plus the legacy `memory.unaliased`.

**Key.** *Closed*: the kernel route makes it unnecessary for kernel-proved
claims (it may remain for `bounded(k)` results). *Open*: still a bucket-(c)
gap after the stated milestone. *Reclassified*: not a tool gap; moves to
bucket (a) or (b), pending the tiered-release spec.

| Id | After K-M1 | After K-M2 | Why |
|---|---|---|---|
| `wir.loop.bounded-k` | **Closed** | Closed | Block lockstep and block invariants hold for every trip count (8.2, 9.3) |
| `wir.proof.trivial-miter` | **Closed** (retired) | Closed | The certificate is a kernel-checked export derived from the semantics (9.6) |
| `wir.proof.obligation-binding` | **Closed** | Closed | The checker re-elaborates the goal and compares it with the theorem's type (7.3) |
| `wir.inline.sound` | **Closed** | Closed | No inlining; calls are steps on the frame stack (8.5) |
| `wir.bound.heap-unchecked` | **Closed** | Closed | Heap invariant from allocation sites (9.3) |
| `wir.bound.depth-unchecked` | **Closed** | Closed | Stack-length invariant from the call graph (8.5) |
| `tcb.symexec.engine-correct` | **Closed** | Closed | The executor only predicts; step lemmas and the kernel decide (9.2) |
| `tcb.bitblast.encoding-correct` | **Closed** | Closed | Core-library `BitVec` lemmas, LRAT terms (9.4, 9.5) |
| `tcb.drat.checker-correct` | **Closed** | Closed | LRAT turned into kernel-checked terms (9.5) |
| `tcb.evidence.manifest-honesty` | **Closed for proofs** | Closed for proofs | The gate re-checks the export itself; the id remains for test and benchmark evidence, which `proved` does not rest on |
| `tcb.report.checker-correct` | (c), reduced | (c), reduced | Stays (c) by VER31 §3.4. Its proof-relevant part (theorem type equals the signed goal; the axiom footprint) becomes kernel work, covered by `kernel.checker.correct` (11.2). The rest (tier arithmetic, JSON, digests) is VER31's to close |
| `tcb.rust-toolchain.trusted` | Reclassified (a) | (a) | A compiler; mitigated by the differential oracle (12.7) |
| `memory.unaliased` (legacy) | **Closed** | Closed | The goal quantifies over provenances, covering aliased and disjoint slices (6.3), given the allocation-counter well-formedness of 6.2 |
| `src.panic.paths-public` | **Closed at WIR** | Closed | The outcome kind is observed, so a trap in one run but not the other breaks equality; at K-M2 about Rust's panics |
| `wir.lowering.faithful` | **Open** | **Closed** | Needs a meaning for Rust; K-M2's translation validation (14.3) |
| `wir.interp.differential-only` | **Open** | **Closed** | Front-end trust moves from differential tests to per-function proofs |
| `wir.lowering.volatile-by-name` | **Open** | **Closed** | K-M2 lowers the volatile bodies for real and validates them |
| `src.rust.unsafe-region.sound` | **Open** | **Closed, relative to our Rust semantics** | K-M2 proves the volatile region never reaches `stuck` *under the aliasing and validity rules our Lean Rust semantics states*; whether those rules match Rust's is `src.rust.lean-semantics.faithful` |
| *new* `wir.semantics.lean-faithful` | **Open** | **Closed** | The Lean `run` is a model we wrote (6.6); K-M2 restates the goal over the Rust semantics, so `run` leaves the footprint |
| `wir.cost.step-model` | (c); **proposed (b)** | (c), or closed into `src.rust.lean-semantics.faithful` if Q7 moves it | It defines what "steps" means in the claim: a model we wrote, which a person can read and accept, not a tool that can be checked. Today (c) (VER31); moving it needs owner sign-off (Q7) |
| `wir.memory.provenance-model` | (c); **proposed (b)** | (c) unless Q7 moves it | WIR's provenance model is our artefact; it says what addresses mean, which only a person can accept. Today (c) (VER31); Q7 |
| `spec.obs-model.complete` | (b) | (b) | Now attached to `obs` by the footprint |
| `spec.obs-model.public-length` | (b) | (b) | Visible in the goal: lengths are shared variables |
| `spec.secret-labels.complete` | (b) | (b) | Visible in the goal as `secretProvs` |
| `spec.corpus.ct01-faithful` | (b) | (b) | |
| `spec.platform.declared-none` | (b) | (b) | |
| `src.rust.int-semantics` | (a) | (a) | Language semantics |
| `src.rust.subset-conformance` | **Open** (c) | **Closed** | At K-M2 the goal is stated over the source text parsed by a Lean-defined parser for the subset; code outside the subset does not parse, so no goal exists for it |
| *new* `kernel.checker.correct` | **Open** (c) | Open until Q6's route is accepted and its evidence exists | 11.2 |
| *new* `kernel.logic.consistent` | (c); **proposed (a)** | Same | Cannot be closed (Gödel); 11.2, Q5 |
| *new* `spec.goal.faithful` | (b), signed with the goal | (b) | 7.4, 11.2 |
| *new* `src.rust.lean-semantics.faithful` | n/a | owner (Q3) | 14.3 |

Net effect on bucket (c) for ct-compare, using VER31's assignment (20 open
(c) ids today: eleven `wir`, `memory.unaliased`, five `tcb` and three `src`):

| Stage | Open (c) ids | Which |
|---|---|---|
| Today | 20 | |
| After K-M1 | 11 | Six about the Rust-to-WIR step (`wir.lowering.faithful`, `wir.interp.differential-only`, `wir.lowering.volatile-by-name`, `wir.semantics.lean-faithful`, `src.rust.subset-conformance`, `src.rust.unsafe-region.sound`); two models (`wir.cost.step-model`, `wir.memory.provenance-model`); `tcb.report.checker-correct`; and the kernel's own two (`kernel.checker.correct`, `kernel.logic.consistent`) |
| After K-M2 | 5 | The two models, the report checker, and the kernel's own two |
| After K-M2, with the owner's decisions of 2026-11-23 in the signed bucket file | 2 | `kernel.logic.consistent` is (a); the two models are (b), closed by the signed sign-off; open remain `kernel.checker.correct` (closed per claim once the second checker, KRN-019, and the recorded oracle agreement exist) and `tcb.report.checker-correct` (VER31's to close) |

**Said plainly, twice.**

- K-M2 does not make the Rust-to-WIR question disappear. It trades six (c)
  ids for one statement, `src.rust.lean-semantics.faithful`, a model we wrote
  that a human signs (proposed (b), Q3). That id carries everything the six
  carried, including the event and cost model. It must never be (a): it is our
  model of Rust, not Rust.
- Under "everything except the compiler has to be verified", the kernel route
  cannot by itself make any claim `proved`. Something must end the regress: the
  consistency of the logic cannot be proved at all, and the kernel's
  correctness can be established only by agreement with independent kernels or
  by a long verification effort. Q5 and Q6 put that choice to the owner
  explicitly instead of hiding it in a bucket.

---

## 14. Milestones

### 14.0 K-S0: the feasibility spike

**Done, 2026-11-23: feasible, with changes** (9.2, and
`code/learning/lean/krn-s0-spike/REPORT.md`). Constant time for
`ct_eq_equal_length` proved for every length in official Lean 4.35.0, axioms
`[propext, Quot.sound]`. The per-program part is a label table checked by
`decide`, not generated proof text. Open from the spike: calls as lockstep
steps, the unary claims, and a faithful (not hand-transcribed, width-exact)
WIR, all carried by KRN-006, KRN-007 and KRN-012.

### 14.1 K-M0: the kernel checks Lean's core library

`lean-kernel` checks the export of Lean's core library (`Init`, and the `Std`
modules `warrant-lib` needs) at the pin, agrees with the official kernel on
every safe declaration, and rejects the whole negative corpus. This is also
LEAN00 L1's first milestone; Mathlib is LEAN00's next (it is not needed here).

### 14.2 K-M1: Warrant claims proved for every input, at WIR level

In the owner's order: input contracts first, then constant time.

**K-M1a, input contracts and `no_panic`.**

- `wasm-leb128`: `rejects max_bits > 64 with MALFORMED` and
  `rejects offset >= len(data) with MALFORMED`, each as exact rejection with no
  effect and (for `max_bits`) no read of the data; `no_panic` and termination
  for every byte string. This is the class of the SEC-006 bug. It depends on
  the Rust front end lowering `wasm-leb128` (VER25a/b, RS01 RS-B4); if it does
  not yet, `select_bytes` below goes first and `wasm-leb128` follows.
- ct-compare: `select_bytes`'s `rejects len(a) != len(b)` (a proved early,
  effect-free rejection on a public condition), and `no_panic` and termination
  for all four operations, for every length.

**K-M1b, constant time and resource bounds** for all four ct-compare
operations: constant time over the full observation (branches, addresses,
steps, outcome kind) with the no-`stuck` companion and witnesses; step, heap
and depth bounds.

Still open in bucket (c) after K-M1, and why: the six ids about the step from
Rust to WIR (`wir.lowering.faithful`, `wir.interp.differential-only`,
`wir.lowering.volatile-by-name`, `wir.semantics.lean-faithful`,
`src.rust.subset-conformance`, `src.rust.unsafe-region.sound`), because a
theorem about WIR says nothing about Rust until something relates the two;
plus the model and kernel ids of section 13.

**K-M1 may refute claims, and that is a result.** Two things are checked for
the first time: the step bound under the VER25 cost schedule (today's
evidence used a different count, 2.2), and the heap bound once stack slots and
heap are told apart (6.2). A refuted bound goes through VER27's finding
protocol (backlog row, failing test, fix, pinning claim, lesson); it is not
weakened to make the proof pass. Under the owner's rule the claims are therefore
**not yet `proved`**. VER31's claim ladder already has the right rung: K-M1
moves each claim to **`checked`** (rung 3: a certificate covers every input,
but own gaps remain).

**Stretch, K-M1c**: the functional claims (`eq_agrees`, `eq_fixed_agrees_*`,
`select_picks`, `select_rejects_mismatch`, `eq_u64_agrees`) as Lean proofs
through the escape hatch (10.2): the first real use of it.

### 14.3 K-M2: the step from source to WIR validated

1. **A Lean semantics of the Rust subset** ct-compare uses (VER25a):
   functions, `&[u8]`, `&[u8; N]`, `&mut u8`, `Vec<u8>` from `vec![0; n]`,
   `for i in 0..n`, indexing with its bounds-check panic, `if`, `return`,
   `assert_eq!`, `wrapping_sub`, `wrapping_neg`, `as`, the bitwise and
   comparison operators, `black_box`, and `read_volatile`/`write_volatile` on
   pointers derived from references, with Rust's undefined behaviour as
   `stuck`. It names the build profile it describes (`debug_assert_eq!`
   differs between profiles). Its own spec comes first (proposed VER25c).
2. **Per-function translation validation**: the front end emits, beside the
   WIR, a simulation hint (which Rust locals map to which WIR variables, which
   statements map to which blocks). A generic simulation theorem in
   `warrant-lib`, proved once, says a checked simulation preserves outcome,
   trace and steps. A wrong lowering fails to validate, so the lowering never
   has to be proved correct in general.
3. **The volatile region** is lowered from its body, and a `no stuck` theorem
   for it closes `src.rust.unsafe-region.sound`.

**The K-M2 export** is the K-M1 goal restated over the Rust semantics: the
same premises and conclusions, with `run` on WIR replaced by the Rust
semantics' `run` on the source. The goal mentions the **source text itself**,
as a string literal whose digest is the source file's, parsed by a parser for
the subset defined in Lean; `parse src = some ast` is proved by kernel
reduction (a performance risk for long files, measured in KRN-016). So no Rust
parser or embedding is in the trust base at K-M2. The export is proved from
the K-M1 theorems and the simulation theorem. Then `run` on WIR and the WIR
constant leave the goal's footprint, and the six Rust-to-WIR ids close.
Source outside the subset does not parse, so no goal exists for it; that is
what closes `src.rust.subset-conformance`.

**A cheaper route to the same guarantee** (decided when VER25c is written):
instead of per-function hints, make the lowering syntax-directed for the VER25a
subset, write it as a Lean function, and prove simulation **once per
construct** (about fifteen). The Rust lowering's output is then checked equal
to the Lean function's output by kernel reduction. Per-function validation
needs no proof about the lowering but a hint generator; per-construct proof
needs a Lean copy of the lowering but no hints. Either way the trust ends in
`src.rust.lean-semantics.faithful`.

With the owner's bucket decisions of 2026-11-23 (16.2) in a signed bucket
revision, the claims are then **`proved`** once two more things exist: the
recorded agreement of the official Lean kernel and our second checker on each
claim's export (closing `kernel.checker.correct`, KRN-019), and VER31's closure
of `tcb.report.checker-correct`. The (b) ids, including the printed goals, need
the owner's signed sign-off. The new assumption
`src.rust.lean-semantics.faithful` is backed by review and by differential
tests against `rustc`-built code; its bucket is owner question Q3.

RS01 moves the lowering onto the grammar-driven parser (RS-B4). Translation
validation is indifferent to how the lowering is written, one more reason to
validate per function.

### 14.4 Rejected alternative: prove the source directly

Skipping WIR and generating conditions from the Rust semantics would avoid
translation validation for Rust, but would need a relational program logic per
source language, eighteen times over. Keeping WIR as the one language with the
lockstep theorem and validating each front end against its source semantics
scales across lanes; the per-language cost is a source semantics and a
simulation checker, which K-M2 needs either way.

---

## 15. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| Kernel size and subtlety | A soundness bug in a large kernel | Budget, literate source, corpora, mutation testing, the differential oracle, a later second checker (12) |
| Kernel performance | Checking Lean core (and later Mathlib) is slow | Hash-consing, caches, lazy delta with hints; K-M0 measures; Mathlib is LEAN00's milestone, not this one |
| K-M0 does not converge | The kernel keeps disagreeing with the oracle on corner cases of Lean's type theory | The oracle and Lean's corpora localise every disagreement to one declaration; the budget is revisited once, by owner decision (4.2), rather than silently |
| K-M1 refutes a Warrant bound | A claim is false as stated | A finding, handled by VER27 §7.4's protocol (14.2) |
| Lean moves | Export format or kernel changes | One pin for toolchain, exporter and format; moving it is its own PR (4.4) |
| Official toolchain in the build path | Conflicts with the no-third-party rule | Untrusted producer only, digests committed, replaced by LEAN00 L3 (5.3, Q2) |
| Kernel reduction too slow for some blocks | Obligations time out | LRAT terms instead of `decide`; smaller blocks |
| The claim elaborator is wrong | Wrong theorem proved | Small, literate, mutation corpus, printed goal, human sign-off (7.4) |
| The Lean semantics of WIR disagrees with intent | Proofs about the wrong program | Differential evaluation against the interpreter (6.5); K-M2 ties it to Rust |
| Lockstep fails on legitimate code | No proof | Correct refusal for constant time (8.6); existing labels remain |

---

## 16. Owner decisions

### 16.1 Questions

| # | Question | Recommendation | Alternatives |
|---|---|---|---|
| Q1 | Kernel logic | *Decided*: Lean 4's type theory (section 3) | HOL; dependent type theory other than Lean's; FOL with induction |
| Q2 | May the pinned official Lean toolchain act as an **untrusted producer** whose output our kernel re-checks: elaborating `warrant-lib`, the generated per-claim proofs and hand-written Lean proofs, and dumping the environment-extension snapshot an in-house elaborator needs? | Yes, on Z3's terms (5.3): digests committed, never a gate dependency, replaced step by step by LEAN00 L3 and L5 | No: per-claim proofs are written as raw kernel terms by a Rust term builder (a small elaborator, budgeted then), and `warrant-lib` waits for LEAN00 L3 |
| Q3 | Bucket of `src.rust.lean-semantics.faithful` (our Lean meaning of the Rust subset, with its events and costs) | (b): a statement a person reads and signs | (c): it then blocks `proved` until something we cannot build (a verified Rust semantics) exists |
| Q5 | Bucket of `kernel.logic.consistent` | (a), as the programme's one mathematical floor: Gödel's theorem makes it impossible to close | (c): then no claim can ever be `proved` |
| Q6 | How `kernel.checker.correct` is closed | **R1 now**: independent agreement (official Lean kernel at the pin, plus our own second checker, both accepting every export the gate relies on), recorded as checked evidence; **R2 later**: a verified kernel (lean4lean direction) | R2 only (no `proved` for years); a different R1 quorum (for example adding nanoda_lib as a third oracle) |
| Q7 | Buckets of `wir.cost.step-model` and `wir.memory.provenance-model` | (b): they say what "steps" and "addresses" mean; only a person can accept that | (c), as today: closed only when lower layers replace them (VER13 R2/R3), so `proved` waits for owned hardware |
| Q8 | Statement sign-off binds the printed goals (`goals_sha256`, 7.4) | Yes: it takes the claim elaborator out of the trust base | No: the elaborator stays a (c) tool gap with no route to close it |
| Q9 | **Resolved 2026-11-24** (owner decision: our own Proof Interchange Format; see [VER32](VER32-proof-interchange-format.md)). The kernel reads PIF; lean4export is read by an untrusted converter. The narrower question of how the PIF reader is built is VER32 Q1. Original question: How the kernel reads lean4export's JSON without a hand-written parser | Our own minimal line format inside the kernel, produced by an untrusted grammar-driven converter (4.3) | A `grammar-tools`-generated JSON reader inside the trusted base; or an exemption for a tiny hand-written reader |
| Q10 | First adopter for input contracts | `wasm-leb128` (`rejects max_bits > 64`, the SEC-006 class), with ct-compare `select_bytes` as fallback (14.2) | ct-compare only |

Q4 of the earlier draft (when the second checker is built) is folded into Q6:
under R1 it is needed before any `proved`, so KRN-019 moves up to P1.

### 16.2 Recorded decisions

- **2026-11-24, Q9**: resolved in favour of "our own format". The kernel keeps Lean 4's type theory (§3.3) and reads a prover-agnostic Proof Interchange Format; every producer, Lean included, is an untrusted converter ([VER32](VER32-proof-interchange-format.md)). Logic gates are the first proof target, ahead of ct-compare (VER32 section 9). Q10's first adopter for input contracts is unchanged; the gates come before it in the queue.

- **2026-11-23, Q1**: the kernel implements Lean 4's type theory and checks
  Lean's export format; the first milestone is Lean's core library, then
  Mathlib (owner, relayed by the coordinating session).

- **2026-11-23, Q2 and LEAN00 QA**: yes. The pinned official Lean toolchain
  may act as an untrusted producer of data (elaborating `warrant-lib`, the
  generated per-claim proofs and hand-written proofs; dumping the extension
  snapshot; supplying compiled IR for LEAN00 L5), on Z3's terms: digests
  committed, our kernel is the gate, replaced step by step by our own layers
  (owner, in this session).
- **2026-11-23, Q5 and Q6**: floor plus agreement. `kernel.logic.consistent`
  goes to bucket (a) as the programme's one mathematical floor;
  `kernel.checker.correct` stays (c) and is closed per claim by recorded
  agreement of the official Lean kernel and our own second checker (route R1),
  with a verified kernel (R2) as the long-term route.
- **2026-11-23, Q3, Q7 and Q8**: `wir.cost.step-model`,
  `wir.memory.provenance-model` and (at K-M2) `src.rust.lean-semantics.faithful`
  move to bucket (b), closed by the signed statement sign-off, and the sign-off
  gains `goals_sha256` binding each claim's printed Lean goal (new (b) id
  `spec.goal.faithful`).
- **2026-11-23, Q10**: `wasm-leb128` is the first adopter for proved input
  contracts, with ct-compare `select_bytes` as the fallback.

These bucket decisions take effect through the VER31 mechanism, not this
document: an amendment to VER31 §3.4 and a new owner-signed revision of
`code/assumptions/buckets-v1.json` (backlog KRN-011). Until that revision is
signed, the gate computes with the current file.

**Still open, with the recommendation as the working default:** Q9 (how the
kernel reads lean4export's JSON), settled when KRN-003 is specified; LEAN00
QB (pin policy), QC (implementation language) and QD (L3's ambition).

---

## 17. Relationship to other specifications

| Spec | Relationship |
|---|---|
| VER00 §7.5 | This is step 3's kernel for proof terms. Its "a few hundred lines" is replaced by section 4.2's budget and section 12's independent checking; VER00 is amended (KRN-001) |
| VER11 part 1 | `drat-check` stays as a pre-filter and for `bounded(k)`; kernel proofs use LRAT terms instead (9.5) |
| VER12 | Not yet written. Sections 7 to 9 are its design; VER12 is written from them before the VC builder's code (KRN-008) |
| VER13 | Gains the ids of 11.2; the reclassifications of section 13 are proposals for the tiered-release spec and the catalogue |
| VER19, VER20 | Their engines become untrusted predictors and keep producing `bounded(k)` where the kernel route fails. VER20 §5 gains a second certification route: a kernel-checked export |
| VER25 | Its semantics becomes a Lean definition and normative (6.1); VER25 is amended (KRN-006) |
| VER25a/b, RS01 | K-M2 validates whatever lowering exists, per function |
| VER27 §4.7 | Its "missing: non-trivial 2-safety miters, obligation binding, heap and depth proofs, VER12" are delivered by K-M1 |
| VER29 | A kernel theorem can be the claim a `contract.*` id names; linking is unchanged |
| VOS30 | `lockstep_obs` is the relational composition rule VOS30 §3.6 defers |
| LANG24 | The Constraint VM is an untrusted search back end whose answers become proof terms (9.5) |
| [VER32](VER32-proof-interchange-format.md) | The kernel's input is PIF (4.3 amended by VER32). The proof side (E7) lets proof libraries sit after the signed base; the reader's file classes are VER32 5.3 |
| [VER34](VER34-constraint-vm-smt-solver.md) | **VER34 is what 9.4 and 9.5 deliver.** Its reconstructors are untrusted producers of claim files on the proof side; KRN-009 (LRAT emission) is **P1** for it; its `Pif.Lrat` supersedes the verified-checker half of KRN-009; KRN-012a is not its `Int` Farkas engine; Q13 (Lean as an untrusted producer for library proofs) is decided, conditional on CVM-S2 |
| LEAN00 | This kernel is its layer L1; LEAN00 L3 replaces the official elaborator of 5.3; LEAN00 L4 may supply a `reduceBool` evaluator (4.5) |
| Tiered-release spec (in progress) | Shares bucket names, ids and the K-M1 tier name (12.4) |

## 18. What this spec does not promise

- That the kernel is small. It is budgeted, literate and independently
  checked; it is not a few hundred lines.
- Any discharge of bucket (a). A `proved` claim still assumes the compiler,
  the CPU and the rest, and says so.
- That every future claim yields to lockstep or templates. ct-compare is
  chosen because it does; harder code needs Lean proofs.
- A verified kernel. That is the lean4lean direction, after K-M2.

## 19. Review record

**Fable adversarial review, round 1 (2026-11-23)**, of the earlier HOL draft.
Most findings were about the verification route and carry over to this
version. Disposition:

| # | Finding | Severity | Disposition |
|---|---|---|---|
| 1 | Goal comparison did not pin trusted definitions; a claim export could define its own `obs` | blocker | Fixed: allow-listed environment digests, claim exports may not declare constants in `G`, program constant compared by value (7.3) |
| 2 | A program that is always `stuck` satisfies the constant-time goal | blocker | Fixed: mandatory no-`stuck` companion goal and a `ret` witness (7.1, 7.5) |
| 3 | `stepBlock` undefined for calls; contracts missing post-state | major | Fixed (and refined in round 2): frame-stack semantics, calls are steps; contracts deferred and specified (6.2, 8.5) |
| 4 | Secret set must grow during the run; fresh ids could collide with arguments | major | Fixed: `Secret s` from allocation sites; counter well-formedness (6.2, 8.3) |
| 5 | Heap claim probably false as lowered (`bool_slot`); count vs bytes | major | Fixed in design: stack vs heap `Alloc` kind, heap in bytes; flagged as a likely finding (6.2, 14.2) |
| 6 | Collapse diagnosis mechanism wrong; step model differs from VER25 | major | Fixed (2.2) |
| 7 | HOL definition side conditions incomplete | major | Moot for Lean; the Lean corpora and oracle play this role (12.2, 12.3) |
| 8 | 600-line HOL core unrealistic | major | Moot; Lean budget set at 8,500 lines, revisited once (4.2) |
| 9 | Bucket (k) hides tool gaps (`.warrant` reader, embedding, wrapper, Rust-only parts) | major | Fixed, and superseded by VER31: no (k) bucket; the kernel's ids are (c) with closing routes or owner proposals (11.2); the elaborator leaves the trust base through the signed goal (7.4); wrapper trusted and budgeted (4.3) |
| 10 | Reclassifications too generous; no id for the semantics model | major | Fixed: VER31's assignment kept; the model moves are proposals (Q7); new `wir.semantics.lean-faithful` in (c); unsafe-region closure stated relative to our semantics; report checker stays (c) (13) |
| 11 | K-M2 renames the gap; export unstated; per-construct route cheaper | major | Fixed: export stated, trade stated plainly, per-construct route recorded (13, 14.3) |
| 12 | K-M1 automation under-specified (`no_panic` framing, `eq_fixed<N>`, symbolic alloc) | major | Fixed (9.3) |
| 13 | HOL library estimates low by 2–3× | major | Largely moot: Lean's core library is reused (5) |
| 14 | OpenTheory omitted from the logic comparison | minor | Moot after the owner's decision |
| 15 | Fuel granularity differs from the interpreter; fuel recursion needs cost ≥ 1 | minor | Fixed (6.2, 6.5) |
| 16 | One-sided `running`; witness strength; jointly inconsistent hypotheses | minor | Fixed (8.2, 7.5) |

**Fable adversarial review, round 2 (2026-11-23)**, of this Lean version and
LEAN00, before the VER31 alignment. Disposition:

| # | Finding | Severity | Disposition |
|---|---|---|---|
| 1 | `rfl` on symbolic state does not reduce (stuck `Decidable.rec`); raw terms from Rust need a mini-elaborator | blocker | Fixed: step lemmas proved once, per-step obligations as generated Lean proofs, term writer only as fallback; feasibility spike K-S0 gates the queue (9.2, 14.0) |
| 2 | Fuel-as-cost is well-founded recursion and does not reduce | blocker | Fixed: structural step-count fuel, cost accumulated, bounds as `∃ n, ret ∧ steps ≤ B` (6.2, 7.1) |
| 3 | Calls are WIR instructions, not terminators | blocker | Fixed: instruction-level steps with a per-frame `ip` (6.2, 8.2, 8.5) |
| 4 | Secret scalars and `eq_u64` uncovered; shared variables make the goal `rfl` | blocker | Fixed: secret scalars get one variable per run; witnesses differ in every secret (7.1, 7.5) |
| 5 | An export lacks environment extensions; L3 cannot elaborate against `Init` from it; example used tactics outside L3 | blocker | Fixed: stated (5.3), LEAN00 L3-a exit claim corrected, extension snapshot is owner question Q2/LEAN00 QA; escape-hatch example and `warrant-lib` limited to L3's tactic subset (10.2) |
| 6 | `Secret s` had no state to be computed from; entry counter condition missing | major | Fixed: allocation tags; counter as an entry argument with `∀ q ≥ a, m q = none` in `Pre` (6.3, 8.3) |
| 7 | Primitive definitions not validated; missing kernel rules; `String` representation varies | major | Fixed (3.3) |
| 8 | Export format (NDJSON) vs the grammar-driven rule; oracle replay harness; unsafe declarations | major | Stated; owner question Q9; harness named (4.3) |
| 9 | Budget, `bignum-core` surface, `forbid(unsafe_code)` vs C-ABI | major | Fixed: budget 8,500 with the trusted `bignum-core` surface and a separate FFI crate counted (4.2) |
| 10 | Trust accounting overstated closure; missing questions | major | Fixed with the VER31 alignment: closed, open and proposed moves listed separately, Q3 offers (c), Q7 asks about the model ids, K-M2's goal names the source text so no Rust parser is trusted (13, 14.3) |
| 11 | `decide` on `UInt8` pairs impractical | major | Fixed: core `BitVec` lemmas; SAT/LRAT off K-M1's critical path (9.4) |
| 12 | LEAN00 L2 design gaps | major | Fixed in LEAN00 §4 |
| 13 | LEAN00 L5 option C omissions | major | Fixed in LEAN00 §7 |
| 14 | Backlog gaps and wrong dependencies | major | Fixed in VER-BACKLOG |
| 15–21 | Totals, id counts, string ids, mid-block cost, depth bound, `#reduce`, recorded oracle agreement | minor | Fixed (LEAN00 §0, §6.1; VER11a 6.2, 8.5, 12.3, 13) |

### Follow-up 2026-11-24 (cross-spec follow-up from VER34 section 13.1)

Applied from [VER34](VER34-constraint-vm-smt-solver.md) section 13.1, items 1 to 4 under VER11a, and
the backlog rows in `VER-BACKLOG.md`.

| # | Change | Source | Where |
|---|---|---|---|
| K1 | **KRN-009 (LRAT emission) is P1** (was P2 and "not on K-M1's critical path"): VER34's CVM-152 and CVM-246d need it. The row now says it is on the critical path of VER34 M1, not of K-M1 | VER34 13.1 (VER11a 1) | backlog |
| K2 | KRN-009's "or a verified LRAT checker in `warrant-lib` run by kernel reduction; measure both" is **superseded by CVM-150/151** (`Pif.Lrat`): KRN-009 is LRAT emission only | VER34 13.1 (VER11a 2) | 9.5, backlog |
| K3 | **KRN-012a is a `Nat` procedure**; it is not the `Int`/`Pif.Rat` Farkas engine (CVM-217) | VER34 13.1 (VER11a 3) | 9.3, backlog |
| K4 | 9.5 records that **reflection is a hypothesis measured by CVM-S1**, and that **KRN-019 and the replay harness run `cvm-smt-v1` proofs** (CVM-177) | VER34 13.1 (VER11a 4) | 9.5 |
| K5 | Relationship rows for VER32 and VER34 added to section 17 | this follow-up | 17 |
| K6 | **Owner decisions touching VER11a** (all recorded in VER32 12.1 and VER34 12.1): the hand-written trusted PIF reader is approved (VER11a Q9 stays resolved by VER32); the Lean toolchain is an untrusted producer for the kernel library proofs, conditional on CVM-S2. **VER11a Q5 (`kernel.logic.consistent`) is not decided by any of this** | owner, 2026-11-24 | none |

**Not applied, and why.** Nothing was left out.
