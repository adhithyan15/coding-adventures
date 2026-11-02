# VER00 — The Verified Shipping Program

**Status:** Draft umbrella spec. Nothing here is implemented. Child specs
(VER01–VER19, listed in §11) are written one at a time before their code.

**Layer:** Repository-wide policy and tooling.

**Builds on:** TF00 (portable test framework), B06 (zero-dependency
migration), D21 (capability cage), LANG23 (refinement types and solver
tier), LANG24 (constraint VM), UI47 (host capability effects).

**Revises:** TF07 (DOM testing) — see §6.5.

---

## 1. The policy in one paragraph

Nothing leaves this repository — not a library on npm, crates.io, PyPI,
RubyGems or Hex, not an app, not an editor extension — unless the code it
contains has been **unit tested, property tested, fuzzed and formally
verified**, and its **capability manifest has been audited**. "The code it
contains" means the package *and every in-repo package it depends on*,
because those ship inside it. The gate that enforces this is the only path
to a release, and every tool the gate relies on is built in this
repository, from scratch, in the same literate style as everything else
here.

As of this spec, almost nothing has been published. That is deliberate:
the reason most packages have not shipped is that there was no gate worth
trusting. This program builds the gate first.

## 2. Why one program and not five projects

The obvious way to read the policy is as a checklist of five separate tools:
a test runner, a property tester, a fuzzer, a prover, a capability auditor.
Built that way, each tool needs its own description of what the code is
supposed to do, and those five descriptions drift apart. The unit tests say
one thing, the properties say a slightly different thing, the fuzz harness
checks only "does not crash", and the proof — when one exists — proves a
fourth thing nobody else checks.

This spec takes the opposite approach. There is **one description of what a
package claims**, written once in a formal specification language (§4), and
every kind of evidence is *compiled* from it:

```
                        ┌──────────────────────────────┐
                        │  package.warrant              │
                        │  (one formal statement of     │
                        │   what the package claims)    │
                        └──────────────┬───────────────┘
                                       │  warrant compiler (per-language backends)
        ┌──────────────┬───────────────┼───────────────┬───────────────┬──────────────┐
        ▼              ▼               ▼               ▼               ▼              ▼
   examples →     properties →    input shapes →   scenarios →    budgets →      claims →
   UNIT tests     PROPERTY tests  FUZZ targets     INTEGRATION    MICRO-         PROOF
   (pure, fast)   (generate +     (coverage-       tests (real    BENCHMARKS     obligations
                   shrink)         guided)          capabilities)                 (solver +
                                                                                   kernel)
        │              │               │               │               │              │
        └──────────────┴───────────────┴───────┬───────┴───────────────┴──────────────┘
                                               ▼
                                  assurance report per package
                                               │
                     + capability audit (required_capabilities.json, §8)
                                               ▼
                                     SHIP GATE (§9)  ──►  publish / release
```

Three things follow from this shape, and they are the reason for it:

1. **Claims cannot drift.** The property test and the proof obligation for
   `insert_adds_element` come from the same formula. If one changes, both
   change.
2. **Each kind of evidence backs up the others.** A property that fails in
   sampling is never sent to the prover. A proof obligation the prover cannot
   discharge still has hundreds of thousands of sampled cases behind it. A
   fuzz crash shrinks to a minimal input that becomes a permanent unit
   example.
3. **The gate can ask a precise question.** Not "does this package have
   tests?" but "for every public symbol, is there a claim, and for every
   claim, what discharged it?"

### 2.1 Every tool, in every language

This repository is written in about 22 languages, and the program covers
all of them. Every tool it builds gets a **lane** in every language, the
way TF00 already plans for unit testing:

- the unit-test framework;
- the property engine;
- the fuzz engine;
- chaos injection;
- the microbenchmark harness;
- the simulated host;
- the Warrant compiler's backend;
- the capability analyzer.

Each tool is specified once, pinned by a shared conformance corpus that
includes negative cases, and implemented natively per language. The point
is the one that motivated TF00 and B06: the repository controls its own
destiny everywhere and depends on nobody's release schedule. Vitest and
Jest go first because they cause the most churn, but the end state has no
third-party testing dependency in any language.

**No lane, no shipping.** A rung is live *for a language* only once that
language's lane for the rung exists and passes its conformance corpus. A
package in a language without the lane cannot ship.

## 3. The evidence ladder

Each package climbs a fixed ladder. A rung counts only when it passes *in
the shipping configuration*, at the tagged commit.

| Rung | Evidence | Answers | Generated from |
|---|---|---|---|
| **E0** | Security audit | Does the package do only what it declares, and is every known weakness class impossible here or tested and proven absent? | `required_capabilities.json` + analyzer; weakness registry (§8.2) |
| **E1** | Unit tests | Do the stated examples hold? Fast, pure, deterministic. | `example` blocks |
| **E2** | Integration tests | Does it work against real capabilities? | `scenario` blocks |
| **E3** | Property tests | Do the claims hold over generated inputs? | `claim` blocks |
| **E4** | Fuzzing | Does it survive hostile input, with no crash, hang or invariant breach? | `input` shapes + claims |
| **E5** | Formal verification | Do the claims hold for *all* inputs? | `claim` blocks → proof obligations |
| **E6** | Native/UI tests | Does the shipped surface work on the real platform? | `scenario` blocks tagged `[ui]` |
| — | Microbenchmarks | Does it meet its stated cost? | `budget` blocks |

Formal verification is **required everywhere** (E5 is not optional). The
general question of what "verified" means for code written in 22 languages
is **deferred** (§7.5). In the meantime E5 is discharged by SAT-backed
proof wherever the code already has a precise formal meaning (§7.6), and
the assurance report labels exactly what each proof covers. Microbenchmarks are evidence, not a rung: they gate only the
packages that state a budget.

Rungs switch on **one at a time** as the tooling for each lands (§10). Once a
rung is live it is blocking for every shipped package. A package that
shipped before a rung went live must climb it before its next release.

## 4. Warrant: the specification language

*Warrant* is a working name: a warrant is what justifies a claim. The language
is the centre of the program, and its child spec (VER04) does the real design.
This section fixes the requirements and a sketch, so the other child specs
have something to build against.

### 4.1 What it is not

The Ruby world's Cucumber/Gherkin is the obvious comparison, and the place
to start is where it stops. A Gherkin feature file is structured *English*:

```gherkin
Given an empty bitset
When I insert 3
Then the bitset contains 3
```

Each line is matched by a regular expression to a "step definition" written
by hand in each language. The file has no semantics of its own. It cannot
be type-checked, it cannot be turned into a property or a proof, and the
step definitions — where all the meaning lives — are rewritten in every
language. Gherkin is a good way to make example tests readable. It is not
a specification language.

Warrant keeps Gherkin's best idea, `given / when / then` scenarios a newcomer
can read, as **one block kind** inside a language that also has a type
system and a logic.

### 4.2 Requirements

1. **Formal.** Every Warrant file has a precise meaning. Claims are formulas
   in a typed logic: first-order logic plus the theories the solver supports
   (§7.2), with quantifiers that are bounded where they must be.
2. **Language-neutral.** A Warrant file never mentions Rust, Python or
   TypeScript. A per-language **binding** (§4.4) maps its symbols onto code.
3. **One source, every rung.** Every block kind compiles to at least one
   rung of §3. The compiler is the only thing that writes generated tests.
4. **Readable by a newcomer.** The repo's literate-programming rule applies:
   prose, tables and examples sit beside the formulas.
5. **Checkable on its own.** The compiler type-checks a Warrant file and
   rejects claims that cannot be true, using the solver, before any code
   exists. Specs-first then has teeth: an inconsistent spec fails before it
   is implemented.

### 4.3 Sketch

```
warrant bitset

  -- The abstract model: what a BitSet *means*. Implementations in any
  -- language are checked against this, never against each other.
  model BitSet = Set<Nat>

  op empty  : BitSet
  op insert : (s: BitSet, i: Nat | i < 2^20) -> BitSet
  op member : (s: BitSet, i: Nat) -> Bool
  op encode : BitSet -> Bytes
  op decode : Bytes -> Result<BitSet, DecodeError>

  -- E1: a concrete example, compiled to a pure unit test.
  example insert_into_empty:
    member(insert(empty, 3), 3) == true

  -- E3 + E5: one formula, sampled by the property engine,
  -- then discharged by the prover.
  claim insert_adds:
    forall s: BitSet, i: Nat | i < 2^20.
      model(insert(s, i)) == model(s) ∪ {i}

  claim round_trip:
    forall s: BitSet. decode(encode(s)) == Ok(s)

  -- E4: decode accepts untrusted bytes, so it must survive anything.
  input decode.bytes : Bytes  hostile
    ensures terminates, no_panic, steps <= 64 * len(bytes)

  -- E2: an integration scenario that needs a real capability.
  scenario saves_and_reloads [integration] requires fs.write, fs.read:
    given a temp dir d and any s: BitSet
    when  save(s, d / "x.bits") and t = load(d / "x.bits")
    then  t == Ok(s)

  -- Microbenchmark budget, checked empirically across n.
  budget insert: amortised O(1), allocations <= 1 per call
```

### 4.4 Bindings

A binding maps `bitset.insert` to `BitSet::insert` in Rust,
`bitset.insert` in Python, `insert` in the TypeScript module, and so on.
It also says how to build a value of each model type (a generator) and how
to read a value back into the model (an *abstraction function*). The binding
is the one place hand-written glue remains, so it is kept small and is
itself checked: the compiler emits a round-trip test from model to value
and back for every bound type.

### 4.5 Generated tests are committed

The compiler's output is committed alongside the package and kept current by
a regeneration check, the same pattern as the existing
`*-grammar-regen-check` CI jobs. Generated tests are code people read, and
the literate rule applies to them too: the generator emits the explaining
prose with each test.

## 5. Test tiers and what each may touch

The user-facing rule: **unit tests are pure and very fast. Anything that
touches the outside world is an integration test, and it says so.**

| Tier | May touch | Speed budget | Runs where | Enforced by |
|---|---|---|---|---|
| Unit (E1, E3) | Nothing real: only the simulated host | each test < 10 ms median; package suite < 5 s | every PR | zero-capability cage + simulated host |
| Integration (E2) | Capabilities in the package manifest, scoped to a temp sandbox | per-scenario timeout | every PR touching the package | cage limited to declared capabilities |
| Fuzz (E4) | Simulated host only | time budget per target | PR smoke run + nightly long run | cage + engine limits |
| Native / UI (E6) | The real platform | per-platform | platform runners | native frameworks (XCTest, Compose instrumentation, a real browser) |
| Proof (E5) | — (static) | solver budget per obligation | PR (cached) + full nightly | proof kernel |

Purity is **enforced, not requested**. The unit-tier runner starts with an
empty capability set. A unit test that reaches the real filesystem, network,
clock, randomness or a thread fails with a `CapabilityViolationError` naming
the call — the same error D21 already defines. The speed budget is enforced
the same way: a unit test that exceeds it fails the run, and the fix is to
move it to the integration tier, not to raise the budget.

Each tier also runs **in shuffled order with a recorded seed**, so hidden
dependencies between tests surface as failures instead of luck.

UI tests use each platform's own framework. That is the one place this
program does not build its own tooling: the platform frameworks are the only
things that drive the real platform.

## 6. The simulated host

### 6.1 The idea

D21 already puts every OS effect behind a secure wrapper: `SecureFile.read`,
`SecureNet.connect`, and so on. Today those wrappers check the manifest and
then call the real OS. This program adds a second backend: a **simulated
host** that answers the same calls deterministically, in memory, under the
test's control.

```
            package code
                 │  calls
                 ▼
     capability wrappers (D21)  ──── check manifest
                 │
       ┌─────────┴──────────┐
       ▼                    ▼
   real host            simulated host
   (integration,        (unit, property, fuzz)
    production)          ├─ virtual clock (time only moves when the test says)
                         ├─ in-memory filesystem
                         ├─ simulated network (loopback graph, latency, partitions)
                         ├─ seeded randomness
                         ├─ simulated workers/threads (deterministic scheduler)
                         ├─ simulated DOM (our own dom-core)
                         └─ fault plan (what goes wrong, and when)
```

This is *deterministic simulation testing*, the technique behind
FoundationDB and TigerBeetle. Its payoff is that a failure found once can be
replayed exactly forever: same seed, same schedule, same faults, same bug.

### 6.2 Fault plans

A fault plan is data, either written by hand in Warrant or generated by the
property engine:

```
fault plan worker_dies_mid_job:
  after worker.received(2) : worker.crash
  after 30 ms               : net.partition(app, api)
  on    fs.write("cache/*") : fs.error(ENOSPC)
```

The fake web worker is exactly this: the code under test posts messages to
what it thinks is a worker. The simulated host delivers them on a schedule
the test controls, and the fault plan makes the worker crash, stall, reply
out of order or drop a message. When the property engine *generates* fault
plans and shrinks failures, the result is the smallest sequence of faults
that breaks the code.

### 6.3 Taxonomy extension

D21's eight categories (`fs`, `net`, `proc`, `env`, `ffi`, `time`, `stdin`,
`stdout`) do not cover everything a test needs to control. VER03 proposes
adding:

| Category | Actions | Why |
|---|---|---|
| `rand` | `read` | Unseeded randomness makes a test nondeterministic. |
| `thread` | `spawn`, `message` | Workers, threads and actors must be schedulable. |
| `ui` | `read`, `write`, `event` | DOM and native views, so UI logic can be unit-tested. |

Adding these is a D21 change and goes through D21's own review.
Over-declaring is still impossible: there is no "all" value.

### 6.4 Chaos injection in the integration tier

Fault plans are not only for the simulated host. The **real** host's
wrappers accept a fault plan too, so an integration scenario can run against
the real filesystem, network or process table while the wrapper injects
failures at chosen points:

- a full disk;
- a slow or partial write;
- a connection reset in the middle of a response;
- a child process killed;
- the clock jumping backwards.

```
scenario survives_disk_full [integration, chaos] requires fs.write:
  given a temp dir d and any s: BitSet
  under fault plan disk_fills_after(4 KiB)
  when  r = save(s, d / "x.bits")
  then  r == Err(NoSpace) and not exists(d / "x.bits.partial")
```

Because injection happens in the capability wrapper, which every language
lane already has, chaos testing comes to every language at no extra cost per
language. Scenarios tagged `[chaos]` run on every PR that touches the
package. Long randomized chaos runs, with plans generated by the property
engine, run nightly.

### 6.5 DOM: two tiers, not one

TF07 currently plans DOM testing by driving a real browser. This program
splits that:

- **Unit tier:** UI logic talks to the DOM through the `ui` capability. The
  simulated host backs it with the repo's own `dom-core` (Rust, C and C++
  implementations already exist), exposed to TypeScript through WebAssembly.
  No browser and no jsdom: fast, deterministic, with fault plans.
- **UI tier (E6):** a real browser, for what only a real browser can show:
  layout, real events, real timing.

The 33 packages that depend on jsdom, happy-dom or Testing Library move to
the unit tier where they can and to the UI tier where they must.

## 7. The rungs in more detail

### 7.1 Unit testing (E1) — TF00, extended

TF00 is adopted unchanged as the unit-test foundation: one portable
assertion vocabulary pinned by a conformance corpus, a bootstrap harness, and
per-language adapters over each language's built-in runner (`node:test`
replaces Vitest and Jest). TF00's sequencing stands, and its TF01 phase,
which removes Vitest and Jest from the ~450 TypeScript packages without
DOM, is the **first work item of this whole program** (§10). It is the
only item with an immediate security payoff: it removes the dependencies
behind most of the repository's Dependabot alerts and the churn of updates
they cause.

What this program adds to TF00: the tier and speed enforcement of §5, the
simulated host of §6, and Warrant as the source of unit examples.

### 7.2 Property testing (E3) — VER05

An in-house property engine, specified once with a conformance corpus and
ported like TF00:

- **Generators** derived from Warrant types and refinements (`i: Nat | i < 2^20`).
- **Integrated shrinking**: shrinking is built into the generator, as in
  Hypothesis, so shrunk values still satisfy their refinements.
- **Replay**: every failure prints a seed that reproduces it exactly.
- **Stateful (model-based) testing**: generate operation sequences, run them
  against the implementation and the Warrant `model`, and compare after each step.
- **Generated fault plans** for the simulated host (§6.2).

The existing hand-rolled sweeps move onto it first. Their properties are
already written, and they gain shrinking and replay. Those sweeps include
`regex-engine`'s cross-checks, the gate-level vs functional simulator
differentials, the compressor round-trips and the seeded data-structure tests.

### 7.3 Fuzzing (E4) — VER06

An in-house coverage-guided fuzzer with a lane in every language (§2.1).
The engine's core is specified once and implemented per language:

- corpus management;
- mutation;
- Warrant-aware structured generation;
- crash deduplication and minimisation.

The hard part is coverage feedback without depending on nightly Rust or
libFuzzer. VER06 inventories a dependency-free coverage source for every
language. Some examples:

| Language | Coverage source |
|---|---|
| TypeScript/JS | V8 precise coverage via the built-in `node:inspector` |
| Python | `sys.monitoring` (3.12+) |
| Go | the `-cover` counters, already used by native `go test -fuzz` |
| Rust | stable `-C instrument-coverage` counters, read in-process |
| Ruby | the built-in `Coverage` module |
| Elixir / Erlang | OTP's `:cover` |
| Languages we compile ourselves | instrumentation emitted by our own compilers |

A language with no usable coverage source still gets a lane: black-box,
generation-based fuzzing driven by Warrant types. That is weaker, and the
assurance report says so. Every crash becomes a committed regression
example in the package's Warrant file.

### 7.4 Microbenchmarks — VER07

An in-house harness:

- warm-up, then automatic calibration of the batch size;
- robust statistics: median, median absolute deviation and bootstrap
  confidence intervals;
- comparison against a stored baseline.

One design point matters more than the rest: **shared CI runners are too noisy
to gate on wall-clock time.** Gating uses deterministic counts:

- instructions retired, where the platform exposes them;
- VM instruction counts, for code running on the repo's own VMs;
- allocations, which every language can count.

Wall-clock numbers are reported but never block. Complexity budgets
(`amortised O(1)`) are checked by fitting counts across a range of sizes.

### 7.5 Formal verification (E5) — VER09 to VER13

Built in-house, in layers, each of which checks the one below:

1. **SAT** — grow `constraint-engine` from DPLL to CDCL (watched literals,
   clause learning, restarts). Its Cargo description already claims CDCL
   while the code is DPLL; this closes the gap.
2. **SMT** — theories on top of SAT: equality with uninterpreted functions,
   linear integer arithmetic (Cooper's method exists), bit-vectors and arrays.
   `smt-lib-format` lets us run the standard benchmark suites, and Z3
   serves as a **differential oracle during development only**, never as a
   dependency.
3. **Proof certificates and a trusted kernel** — the solver does not get
   trusted. Every UNSAT answer comes with a certificate (DRAT for SAT, proof
   terms for theories) checked by a small LCF-style kernel. The kernel is the
   only thing that has to be right, and it is kept to a few hundred lines
   so it can be read in full.
4. **Verification-condition generation** — turns Warrant claims, plus the
   code they are bound to, into proof obligations.
5. **Refinement types** — LANG23 and the existing `lang-refinement-checker`
   become one consumer of the same stack.

**What does "verified" mean for code in 22 languages?** This is the open
question the program cannot hide. A proof is about a formal meaning, and a
proof about Python code needs a formal semantics of Python. There are three
honest options:

- **(a) Prove the model, test the code.** Proofs cover the Warrant model.
  Each implementation is tied to the model by stateful property tests and
  fuzzing. Cheap, but the shipped code itself is not proved.
- **(b) Semantics per language.** Write formal semantics for a verifiable
  subset of each language, and prove the code directly. True to the policy,
  but 22 semantics is a career, not a project.
- **(c) One verified core, compiled out.** Write the logic that needs
  proving once, in a core language whose semantics we own. Prove it there,
  then generate each language's implementation with the repo's existing code
  generators, checking each translation with translation validation.
  Hand-written code in each language shrinks to thin bindings. This plays
  to the repo's strength, which is compilers.

**This decision is deferred** (§12). The current leaning is (c) as the
destination, with (a) as the interim. Until it is made, §7.6 governs: prove
with SAT wherever a formal meaning already exists, and label everything
else honestly.

### 7.6 SAT first: prove what already has a formal meaning

A SAT solver answers one question: can this Boolean formula be made true?
That one question becomes a proof technique through a single trick.

**Proof by unsatisfiability.** To prove that an implementation `impl` always
agrees with its specification `spec`, build a formula that says "there is
an input where they disagree":

```
    ∃ x.  impl(x) ≠ spec(x)
```

Encode it as Boolean clauses and ask the solver. If it answers **SAT**, it
hands back the `x`: a concrete counterexample, which becomes a permanent
unit test. If it answers **UNSAT**, no such `x` exists among *all* inputs,
and the answer comes with a certificate the trusted kernel checks (§7.5,
step 3). For a 32-bit adder that is 2^64 input pairs proved at once,
something no amount of testing can match.

The one requirement is that `impl` has a precise formal meaning that can be
turned into clauses. In this repository that is already true in more places
than it first appears, and §7.6.1–§7.6.4 take them in order of how complete
the resulting proof is.

#### 7.6.1 Hardware: complete proofs — VER18

Gate-level netlists *are* Boolean formulas, so there is no semantics to
build. The repository has a full hardware stack to apply this to:

- `logic-gates` and `arithmetic` (adders, ALU);
- about 45 gate-level CPU models (`*-gatelevel`);
- `gate-netlist-format` (HNL), `hdl-ir`, `synthesis`, and Verilog and VHDL
  parsers.

| Target | Proof | Completeness |
|---|---|---|
| Combinational blocks (adders, ALUs, multiplexers, decoders) | **Equivalence checking**: build a "miter" circuit that is true exactly when the netlist and the Warrant spec (`sum == (a + b) mod 2^n`) disagree, and show it is UNSAT | Complete: every input |
| Synthesis | **Translation validation per run**: the HIR going in and the HNL coming out are proved equivalent every time `synthesis` runs, so a synthesis bug cannot reach a netlist silently | Complete, per run |
| Gate-level CPUs, one cycle | The next-state function of the gate-level CPU equals the Warrant ISA model, for every state and every instruction | Complete for one step |
| Gate-level CPUs, over time | **k-induction**: an invariant that holds at reset and is preserved by every step holds forever. Where induction fails, bounded model checking proves it up to `k` cycles. | Complete when induction succeeds; otherwise bounded, and labelled |

The 31 existing gate-level vs functional differential tests become the
fallback and the cross-check. This is the most complete proof available
anywhere in the program, which makes it the first target.

#### 7.6.2 Code that lowers to semantic-ir: bounded and complete proofs — VER19

`semantic-ir` (SIR10) is the repository's narrow-waist IR. Front ends
already lower Python, JavaScript, Ruby, Java, C and Twig into it, and SIR21
pins its integer semantics. That gives code in those languages a formal
meaning. It is the meaning of the SIR lowering, which the assurance report
names as such.

The pipeline:

1. **Symbolic execution of SIR.** Run the function on symbolic inputs instead
   of concrete ones, collecting every path as a formula.
2. **Bit-blasting.** Fixed-width integer operations become circuits, so
   arithmetic, comparisons and overflow are exact rather than approximated.
3. **Loops.** A loop with a Warrant invariant is proved by induction
   (complete). A loop without one is unrolled `k` times (bounded, and
   labelled `bounded(k)`).
4. **Claims.** Warrant claims about the function become the formula's goal.

Two further uses come cheaply:

- **Translation validation of the SIR back ends.** Lower SIR to C, read the C
  back through `c-to-semantic-ir`, and prove the two SIR modules equivalent.
  Every generated C file is then checked, not just the generator in general.
- **Trust in the front ends** comes from differential testing: SIR
  interpretation versus the real language runtime on the same inputs, run
  continuously by the property engine.

#### 7.6.3 Warrant specs themselves: in every language, immediately

These checks need no code semantics at all, so they apply to every package
in every language from the day Warrant exists (VER04, VER05):

- **Vacuity**: a claim whose precondition can never be true passes every
  test and proves nothing. The solver finds it, as it does an `example`
  that contradicts a `claim`. The repository has already learned this lesson
  the hard way (`lessons.d/a-substring-check-is-vacuous-for-the-shortest-member-of-the-set.md`).
- **Consistency**: the claims of a package can all be true at once. A spec
  that contradicts itself is rejected before anyone implements it.
- **Model-level proofs**: claims about the Warrant `model` are proved
  outright. That covers rung (a) of §7.5 and needs nothing else.
- **Solver-generated test inputs**: the solver finds inputs that satisfy
  awkward refinements (`i < 2^20 ∧ popcount(s) == 7`). It also finds
  inputs that take every branch of the model and that sit exactly on every
  boundary. These become unit and property cases in every language lane, so
  SAT strengthens testing even where it cannot prove.

#### 7.6.4 Repository configuration

The repository's own control data has a precise meaning:

- `ci-gates.json`;
- capability manifests and their closure;
- the build tool's shard plans;
- dependency graphs.

The solver can prove the properties that today are checked by example or
not at all. For instance: every path a gate claims maps to a gate that
runs; every shard contains all its prerequisites; and the capability
closure the release gate computes really is the union over the dependency
graph.

#### 7.6.5 Labels

Every E5 result in the assurance report carries exactly one label, and the
gate never rounds a weaker one up:

| Label | Meaning |
|---|---|
| `proved` | UNSAT with a checked certificate, over all inputs |
| `proved-per-run` | Translation validation: this artifact, not the tool in general |
| `bounded(k)` | No counterexample within `k` cycles, unrollings or steps |
| `proved-over-sir` | Proved about the SIR lowering of the code |
| `proved-over-model` | Proved about the Warrant model; the implementation is tied by tests |
| `not-yet-provable` | No formal meaning available yet; allowed only until the §7.5 decision |

#### 7.6.6 The solver work this needs

All of it is in-house and builds on `constraint-engine`:

1. **CDCL**: watched literals, clause learning, non-chronological
   backtracking and restarts (VER09).
2. **DRAT certificate checker**: tens of lines, and the first piece of the
   trusted kernel (VER11).
3. **Encodings**: Tseitin transformation, and-inverter graphs for hardware,
   and a bit-blaster for fixed-width arithmetic (VER10).
4. **Incremental solving**, so bounded model checking and k-induction reuse
   learned clauses between steps.
5. **Differential testing of the solver itself**: brute force on small random
   formulas, and the standard SAT competition benchmarks, with known answers
   from established solvers, as a development-time oracle only.

## 8. Security audit (E0)

E0 asks two questions. Does the package do only what it declares (§8.1)?
And is every known class of weakness either impossible here or tested and
proven absent (§8.2–§8.4)?

### 8.1 Capability audit — VER08

The manifest format exists (D21), and 3,341 of ~5,548 packages and programs
have one. What is missing is the audit:

1. **Every shipped package has a manifest.** No manifest means zero
   capabilities, which is a valid declaration, but it must be written down.
2. **An analyzer per language checks code against manifest.** Go and OCaml
   have analyzers today (`ca-capability-analyzer`). Every other language
   needs one, built on the repo's own parsers wherever they exist. The
   analyzer fails on **under-declaration**: a capability used but not
   declared. It warns on **over-declaration**: a capability declared but
   never used.
3. **Capability changes need approval at release.** The gate compares the
   manifest at the release tag against the last published version. Any new
   capability blocks the release until it is approved, the way Apple treats a
   change to an app's entitlements. The approval and its justification are
   recorded in the release notes.
4. **The whole shipped closure is audited.** A package's effective
   capabilities are the union of its own manifest and those of every in-repo
   package it depends on, and that union is what users see.

### 8.2 The weakness registry — VER15

Vulnerabilities are not random. The CVEs published every day fall into a few
hundred recurring classes, catalogued by MITRE as CWE (Common Weakness
Enumeration). Use-after-free is CWE-416, out-of-bounds write CWE-787, SQL
injection CWE-89. The program keeps an in-repo **weakness registry**, data
under `code/specs/data/`, that inventories every class relevant to code this
repository writes. Each entry records:

| Field | Example (CWE-416, use after free) |
|---|---|
| Class and description | Memory is used after it was freed. |
| Applies to | C, C++, Rust `unsafe`, any FFI boundary. Not safe Rust, Python, Ruby, TS, Go (without `unsafe`/cgo), JVM, .NET (without `unsafe`), BEAM. |
| Why it cannot occur elsewhere | The language has no manual deallocation. The analyzer must prove no `unsafe`, `ffi` or `cgo` is reachable. |
| Attack patterns | Free then read, free then write, double free, dangling pointer escaping a scope, use after `realloc`. |
| Required evidence | Attack corpus in the native harness (§8.3), fuzzing with the checked allocator, and a memory-safety proof obligation (E5). |

Every shipped package's assurance report then carries a **weakness matrix**:
one row per registry class. Each row is closed in exactly one of two ways:

- **Excluded by construction**: the class cannot occur in this language, and
  the analyzer confirms the package stays inside the safe subset. For
  example, it uses no `unsafe`, no `ffi` capability and no `eval`.
- **Applicable and discharged**: the class could occur, and the package has
  attack-pattern tests, fuzzing that targets it, and a proof that it cannot
  happen.

Any row left open blocks the release. When a CVE lands in a class the
registry lacks, the registry grows and every shipped package is re-checked
against the new class.

The registry covers the language-independent classes as well as memory
safety. VER15 enumerates them in full; the main families are:

- **Injection**: command, SQL, path traversal, template, log, header.
- **Parsing**: decompression bombs, unbounded recursion, algorithmic
  complexity (ReDoS, hash flooding), integer overflow in length fields,
  canonicalisation mismatches.
- **Cryptography**: non-constant-time comparison, weak randomness, nonce
  reuse.
- **Concurrency**: data races, time-of-check-to-time-of-use.
- **Resource exhaustion**: memory, file descriptors, unbounded queues.
- **Deserialization**: prototype pollution, unsafe object construction.

Each class gets an attack corpus written once as Warrant `input` and `claim`
blocks, and is compiled into every language lane like everything else.

### 8.3 Simulated attacks on native code — VER16

For C, C++, Rust `unsafe` and every FFI boundary, "we tested it" has to
mean the code was **attacked** with each memory-safety class and caught it.
Built in-house:

1. **A checked allocator.** A test-only `malloc`/`free` replacement with:
   - guard zones around every allocation;
   - freed memory poisoned and held in a quarantine instead of being
     reused, so a use-after-free reads poison and is detected;
   - a shadow map of which bytes are allocated, initialized or freed;
   - double-free and mismatched-free detection.

   This is the technique of AddressSanitizer and electric-fence, written
   here and linked into every native test and fuzz build.
2. **A checked interpreter for C.** The repo already has `c-lexer`,
   `c-parser` and `c-to-semantic-ir`. An interpreter over that IR that
   tracks the provenance and bounds of every pointer can catch any
   undefined behaviour at the moment it happens: out-of-bounds access,
   use-after-free, uninitialized reads, signed overflow, misaligned access.
   This is Miri's approach applied to C, and it is the strongest dynamic
   check available.
3. **Attack-pattern corpora**, one per registry class (§8.2). These are
   inputs and call sequences built to trigger each class, such as a length
   field one past the buffer or a callback that frees its own owner. They
   run under both of the above. A test passes when the attack is *refused*
   by the code, not merely survived.
4. **A memory-safety proof obligation** for every applicable package (E5):
   no reachable out-of-bounds access, use-after-free or double free, stated
   over the same pointer model the checked interpreter uses.

Miri continues to cover Rust `unsafe`, as it does today, until the in-house
tools match it.

### 8.4 Concurrency and time

Data races and time-of-check-to-time-of-use bugs need control over
scheduling, and the simulated host already provides it (§6.1). Its
deterministic scheduler explores interleavings systematically: every
ordering for small cases, randomized orderings with replayable seeds for
large ones. A race found once can be replayed exactly.

## 9. The ship gate — VER01

### 9.1 The assurance report

The build tool gains an `-assurance` mode. For each package it emits a
machine-readable report:

```json
{
  "package": "rust/bitset",
  "commit": "<sha>",
  "closure": ["rust/bitset", "rust/bit-ops"],
  "capabilities": { "declared": [], "analyzer": "pass", "new_since_last_release": [] },
  "weaknesses": [
    { "cwe": 416, "status": "excluded", "reason": "safe Rust; analyzer: no unsafe/ffi reachable" },
    { "cwe": 400, "status": "discharged", "evidence": ["attack-corpus", "fuzz", "proof"] }
  ],
  "claims": [
    { "id": "insert_adds", "example": "pass", "property": { "cases": 100000, "status": "pass" },
      "fuzz": null, "proof": { "label": "proved-over-model", "certificate": "drat:<hash>" } }
  ],
  "rungs": { "E0": "pass", "E1": "pass", "E2": "pass", "E3": "pass", "E4": "n/a", "E5": "pass" },
  "public_symbols_without_claims": []
}
```

`public_symbols_without_claims` must be empty to ship. Every exported API is
covered by at least one claim, which is what turns "has tests" into "states
and backs up what it does".

### 9.2 One gate, every release path

Every publish, release and deploy workflow calls a single reusable
`ship-gate.yml` and cannot publish without it passing. Today that means
`publish.yml` plus about 30 `release-*` and `deploy-*` workflows. The gate:

1. resolves the package's shipped closure from the build tool's dependency
   graph;
2. re-runs, or fetches cached results for, every live rung on that closure
   at the tagged commit;
3. runs the capability audit and the capability diff;
4. fails closed, so a missing or unparseable report is a failure, never a
   pass. This follows `ci.yml`'s existing "verify gate verdicts were
   emitted" step.

### 9.3 Coverage, centrally

The per-package coverage thresholds now scattered across `pyproject.toml`,
`vitest.config.ts`, `test_helper.rb`, `mix.exs` and BUILD flags are replaced
by one floor the gate enforces: 95% for libraries and 80% for programs,
per CLAUDE.md. Languages that report no coverage today (Go, Rust, Swift,
Haskell, Lua, Perl, C/C++) get a reporter as part of TF00's lanes. BUILD
files that skip their tests when a tool is missing, as the Haskell ones do
today, fail instead.

## 10. Phasing

Phases follow the "switch rungs on one at a time" decision. Each phase
ends with its rung **live and blocking**.

| Phase | Delivers | Rung live | Child specs |
|---|---|---|---|
| 0 | Gate skeleton, assurance report, central coverage, fail-loud BUILDs, TF01 (TypeScript off Vitest and Jest) | E1 + coverage | VER01, TF01–TF02 |
| 1 | Capability analyzers for all languages, capability diff, weakness registry v1 (exclusions) | E0 | VER08, VER15 |
| 2 | Test tiers, purity and speed enforcement, simulated host, taxonomy extension, chaos injection | E1 purity, E2 | VER02, VER03, VER17 |
| 2b | Checked allocator, checked C interpreter, native attack corpora | E0 for native code | VER16 |
| 3 | Warrant v0 (examples, claims, models) with TypeScript and Rust backends; property engine | E3 | VER04, VER05 |
| 4 | Fuzz engine; `input` blocks | E4 | VER06 |
| 5 | Microbench harness; `budget` blocks | (budgets only) | VER07 |
| S1 | CDCL, DRAT checker, Tseitin/AIG encodings, bit-blaster, incremental solving | — | VER09–VER11 |
| S2 | Hardware equivalence checking, synthesis translation validation, CPU k-induction | E5 for hardware | VER18 |
| S3 | Vacuity, consistency and model proofs; solver-generated inputs (needs Warrant v0) | E5 `proved-over-model` | VER04, VER05 |
| S4 | SIR symbolic execution, bounded and inductive proofs, back-end translation validation | E5 `proved-over-sir` | VER19 |
| 6 | SMT theories, VC generation | E5 widening | VER10, VER12 |
| 7 | Deferred: the general meaning of E5 (§7.5) | E5 everywhere | VER13–VER14 |
| — | Native/UI lanes per platform, when each app ships | E6 | per platform |

Each phase builds its tool's **reference lanes** first, TypeScript and
Rust, to settle the design. The remaining lanes follow against the same
conformance corpus (§2.1). A rung goes live per language as each lane
passes. Phases 1, 2 and 2b are independent of each other, and so are
phases 4 and 5.

The **S track** (SAT first, §7.6) depends on nothing in the testing phases
and starts immediately, in parallel with phase 0. S1 and S2 need only
`constraint-engine` and the hardware crates. S3 waits for Warrant v0, and
S4 for S1.

## 11. Child specs

| Spec | Title |
|---|---|
| VER01 | Ship gate, assurance report and central coverage |
| VER02 | Test tiers: purity, speed budgets, shuffled order |
| VER03 | Simulated host and fault plans (+ D21 taxonomy extension) |
| VER04 | Warrant: syntax, type system, logic, bindings |
| VER05 | Property engine: generators, integrated shrinking, stateful testing |
| VER06 | Coverage-guided fuzz engine |
| VER07 | Microbenchmark harness and deterministic cost counters |
| VER08 | Capability analyzers and release-time capability diff |
| VER09 | CDCL SAT (constraint-engine upgrade) |
| VER10 | SMT theories: EUF, LIA, bit-vectors, arrays |
| VER11 | Proof certificates and the trusted kernel |
| VER12 | Verification-condition generation from Warrant |
| VER13 | The verified core language |
| VER14 | Translation validation for generated implementations |
| VER15 | Weakness registry (CWE inventory) and per-package weakness matrix |
| VER16 | Native attack harness: checked allocator, checked C interpreter, attack corpora |
| VER17 | Chaos injection on the real host |
| VER18 | Hardware equivalence checking, synthesis translation validation, k-induction |
| VER19 | SIR symbolic execution and bit-precise proofs |

## 12. Open decisions

1. **The meaning of E5** (§7.5): **deferred** by the project owner. SAT-backed
   proof (§7.6) proceeds meanwhile, wherever a formal meaning already exists.
2. **Warrant syntax family**: an ML-style syntax like the §4.3 sketch, or one
   closer to an existing in-repo language (Twig, Nib) so the tooling can be
   reused.
3. **Speed budget numbers** (§5): 10 ms per unit test and 5 s per package
   suite are placeholders until they are measured against today's suites.
4. **Which packages ship first.** The first package through the full gate
   proves the gate. A small, pure, already well-tested package (`bitset`,
   `logic-gates`) is the candidate.

## 13. What this does not buy

A proof is only as good as the claim it proves. A package can be fully
verified against a Warrant file that says the wrong thing. The defences
are:

- humans review Warrant files the way they review specs today, because
  they *are* the spec;
- every public symbol must be covered by a claim;
- mutation testing, a later child spec, checks that the claims are strong
  enough to notice when the code is broken.

Moving all of this in-house also moves the bugs in-house. A bug in a test
framework, property engine or prover is worse than most, because it fails
by passing. Every tool in this program therefore ships with a conformance
corpus that includes **negative cases**: inputs that must fail. A trusted
kernel small enough to read in full is the last line of defence.
