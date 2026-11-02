# VER00 — The Verified Shipping Program

**Status:** Draft umbrella spec. Nothing here is implemented. Child specs
(VER01–VER23, listed in §11) are written one at a time before their code.

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
| **E0** | Security audit | Does the package do only what it declares, and is every known weakness class impossible here or tested and proven absent? | `required_capabilities.json` + analyzer + locked-box observation (§8.5); weakness registry (§8.2) |
| **E1** | Unit tests | Do the stated examples hold? Fast, pure, deterministic. | `example` blocks |
| **E2** | Integration tests | Does it work against real capabilities? | `scenario` blocks |
| **E3** | Property tests | Do the claims hold over generated inputs? | `claim` blocks |
| **E4** | Fuzzing | Does it survive hostile input, with no crash, hang or invariant breach? | `input` shapes + claims |
| **E5** | Formal verification | Do the claims hold for *all* inputs? | `claim` blocks → proof obligations |
| **E6** | Native/UI tests | Does the shipped surface work on the real platform? | `scenario` blocks tagged `[ui]` |
| E5 (bounds) | Resource bounds | Is every call bounded in steps, memory, depth and output, and constant-time where it holds a secret? | `bound` blocks → proof obligations |
| — | Microbenchmarks | Does it meet its measured targets, and do observations stay within proved bounds? | `budget` blocks + `bound` blocks |

Formal verification is **required everywhere** (E5 is not optional), and
it means one precise thing: an **assume–guarantee** statement (§7.5). *If*
the package's stated assumptions about its language, runtime and inputs
hold, *then* its claims hold for every input. We do not try to control what
the language or platform does with our code. We control our own logic,
write down exactly what it relies on, and prove the logic under those
assumptions. The proofs are SAT-backed (§7.6), and the assurance report
labels exactly what each proof covers.

Resource bounds are part of E5: they are claims and are proved (§7.4).
Microbenchmarks are evidence, not a rung. They gate packages that state a
`budget`, and they check every package's proved bounds against observation.

Rungs switch on **one at a time** as the tooling for each lands (§10). Once a
rung is live it is blocking for every shipped package. A package that
shipped before a rung went live must climb it before its next release.

This is a deliberate, owner-approved relaxation of §1 during the build-out,
and it is never hidden. A rung that is not yet live is reported as
`not-live`, never `pass`, both in the assurance report and on the published
package. Which rungs are live, and which language lanes have passed
conformance, is part of the gate's policy data read from the protected ref
(§9.2.1). A release commit therefore cannot declare a rung live or dead.

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
6. **Explicit about what it relies on.** Every guarantee is conditional on
   named assumptions (§7.5.1). A Warrant file states them in an `assume`
   section; nothing the proof relies on may be left implicit.

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

  -- What every guarantee below is conditional on (§7.5.2).
  -- Platform assumptions are cited from the shared catalogue, never
  -- invented; each one has a conformance test in every language lane.
  assume platform:
    u64.wraps                 -- fixed-width unsigned arithmetic wraps mod 2^64
    array.index.bounds_checked
    bytes.immutable
  -- Environment assumptions are the caller's side of the contract.
  -- Never about hostile input: that is checked, not assumed (§7.5.2).
  -- Formal, so the solver can use it; prose assumptions prove nothing.
  assume environment:
    save : no_concurrent_writer(dir)

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
    rejects len(bytes) > 2^20 with DecodeError.TooLarge   -- enforced in code, and proved
    ensures terminates, no_panic, steps <= 64 * len(bytes)

  -- E2: an integration scenario that needs a real capability.
  scenario saves_and_reloads [integration] requires fs.write, fs.read:
    given a temp dir d and any s: BitSet
    when  save(s, d / "x.bits") and t = load(d / "x.bits")
    then  t == Ok(s)

  -- Resource bounds: claims, so they are proved (§7.4.1).
  -- Required for every op that takes a hostile input.
  bound decode:
    steps  <= 64 * len(bytes) + 512
    heap   <= 2 * len(bytes)
    depth  <= 32
  bound insert:
    steps  <= 12 * calls + 40          -- amortised O(1), constants explicit

  -- Microbenchmark budget: a measured target, regression-checked, never
  -- called a guarantee (§7.4.3).
  budget insert: median <= 15 ns on ci-linux-x64, allocations <= 1 per call
```

### 4.4 Bindings

A binding maps `bitset.insert` to `BitSet::insert` in Rust,
`bitset.insert` in Python, `insert` in the TypeScript module, and so on.
It also says how to build a value of each model type (a generator) and how
to read a value back into the model (an *abstraction function*). The binding
is the one place hand-written glue remains, so it is kept small and is
itself checked: the compiler emits a round-trip test from model to value
and back for every bound type.

The binding is also the place where a proof could be pointed at the wrong
code: a binding that maps a claim onto a helper or a stub instead of the
real entry point would let everything pass while the shipped API goes
unchecked. So the gate never trusts the binding's own idea of the API. The
language analyzer reads the **real exports** of the shipped artifact
independently. Every exported symbol must be bound, and every binding
target must be an exported symbol (§9.1).

The binding's hand-written generators and abstraction function could also
make claims hold trivially. An abstraction function that maps every value
to the same model value satisfies any equality, for example. So E1 and E3
include **negative checks on the binding itself**:

- the generators must produce values that map to distinct model values;
- the abstraction function must distinguish values that the model
  distinguishes;
- deliberately corrupted implementations, in a small built-in mutation set,
  must make at least one claim fail.

A binding that cannot fail these checks fails the gate.

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

**The simulated host never ships.** A shipped library that could be switched
onto seeded randomness or a frozen clock would be a gift to an attacker:
predictable keys and nonces, rewound expiry checks. So the simulated
backend lives in a test-only module that the shipping build configuration
excludes (a `cfg`/feature flag, a test-only package, or the language's
equivalent). There is no runtime switch, environment variable or config
file that selects it. The VER08 analyzer checks the shipped artifact and
fails the gate if any simulated-host code is reachable from it, and the
weakness registry carries this as its own class (§8.2).

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

As with the simulated host, the fault-plan hook is **compiled out of
shipped artifacts**. Shipped code cannot load a fault plan, and an
attacker who controls configuration cannot use one to inject disk-full or
connection-reset failures. The same VER08 reachability check enforces this.

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
- **Replay**: every failure records a seed that reproduces it exactly.
  Locally the seed is printed. In CI it is stored privately, never in public
  logs (§7.3).
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

**A crash is not published before its fix.** In a public repository, a
minimized crashing input is often a working exploit. So is a replay seed,
because deterministic replay makes a seed a complete reproducer. The class
of a crash is usually unknown when it is found, so the rule cannot wait for
classification:

- **Every** fuzz, chaos or property failure, **every SAT/SMT
  counterexample** and every failing proof obligation is private until
  triaged. A solver counterexample is a concrete exploit input, just as a
  fuzz crash is. Public
  CI logs never print failing seeds or inputs. They print an opaque failure
  ID, and the reproducer goes to a private store outside the repository.
  On a public repository, any signed-in user can download a GitHub Actions
  artifact, so that is not private.
- Long fuzz and chaos campaigns run in a private context, not as public
  nightly jobs.
- Triage defaults to the private security advisory path. A failure leaves
  it only once someone has classified it as not security-relevant.
- For a package that has never shipped, the regression example is
  committed with the fix. For a package that has already shipped, the fix
  is developed in a private advisory fork. The regression example reaches
  public `main` only after the fixed release has been published, so the
  example never goes public while the released version is still
  vulnerable.

### 7.4 Bounds, timing and microbenchmarks — VER07, VER20

Performance gets the same treatment as correctness: a stated claim, then
tests, then proof. Some performance claims can be **proved**, and some can
only be **measured**. The program is explicit about which is which.

#### 7.4.1 What can be proved: boundedness

A **bound** is a claim about how much of a resource an operation can use,
as a function of its input size. Bounds are claims like any other, and they
are proved under assumptions exactly as §7.5 describes:

| Bound | Means | Why it matters |
|---|---|---|
| `terminates` | Every call returns | A hang is a denial of service |
| `steps <= f(n)` | Abstract operations, counted in the SIR cost model | The basis of every timing guarantee below |
| `heap <= f(n)` | Bytes allocated | Memory exhaustion (CWE-400, CWE-770) |
| `depth <= k` | Recursion or stack depth | Stack overflow on hostile nesting |
| `output <= f(n)` | Size of what is produced | Decompression bombs (CWE-409) |
| `steps` independent of a `secret` | Constant time | Timing side channels in cryptographic code (secrets found by default, see below) |

How they are proved:

1. **A cost semantics for SIR.** Every SIR operation is assigned an abstract
   cost. Symbolic execution (§7.6.2) carries a cost counter alongside the
   values, so a bound becomes one more formula for the solver.
2. **Loops and recursion** need a *ranking function*: a quantity that
   strictly decreases on every iteration and cannot go below zero. Its
   initial value bounds the iteration count. The solver checks a ranking
   function it is given, and can often find one for simple loops by itself.
   Without one, the bound is proved only for inputs up to size `n`, labelled
   `bounded(n)`.
3. **Asymptotic claims** such as `amortised O(1)` are proved as concrete
   inequalities (`steps <= 12 * calls + 40`), never as big-O notation, which
   hides the constants and cannot be checked.
4. **Constant time** is proved by *self-composition*. Run the code twice, on
   two inputs that agree on everything public and differ only in the secret,
   and ask the solver for a pair whose step counts or memory addresses
   differ. UNSAT means no such pair exists: the code's timing reveals nothing
   about the secret, under the cost model.

   **Secrets are found, not just declared.** This is the same rule as
   hostile input (§7.5.2): if secrecy depended only on the author's
   annotation, leaving it off would make every constant-time proof pass
   trivially, because timing can reveal nothing about zero secrets. So
   taint analysis treats these as secret by default:
   - output of the `rand` capability;
   - values of key, nonce, MAC and password types listed in the registry;
   - inputs to and outputs of cryptographic primitives;
   - any value compared against one of those.

   Declassifying a value is an exemption needing approval (§9.1). In a
   crypto-relevant package, a constant-time proof with no secret inputs is
   vacuous and fails the gate (§7.6.3). Crypto relevance is **computed, not
   listed**, so a new in-house primitive is covered from its first commit.
   The analyzer marks a package crypto-relevant if it:
   - defines or uses key, nonce, MAC or password types;
   - calls a cryptographic primitive or the `rand` capability;
   - performs the bit-mixing and table-lookup patterns typical of cipher
     and hash implementations.

   Opting out is an approved exemption.

   **The cost model is not the machine.** On a third-party runtime, a
   SIR-level constant-time proof says nothing about real timing: JIT
   compilers, data-dependent bignum arithmetic, caches and branch
   prediction all sit between SIR steps and the clock. Using the proof
   there would require a catalogue assumption that "SIR step count
   determines timing", and **the catalogue holds no such entry** for JIT or
   managed runtimes (V8, the JVM, .NET, CPython, BEAM). On those runtimes
   the timing-side-channel weakness row is therefore `not-yet-provable` and
   needs an approved exemption. Only a `proved-to-<layer>` proof that
   reaches the gate-level CPU (§7.4.2) discharges it without conditions.

Bounds are **required**, not optional, for every operation that accepts a
`hostile` input. Proving them is how the denial-of-service classes in the
weakness registry (§8.2) are discharged: CWE-400 resource exhaustion,
CWE-409 decompression bombs, CWE-674 uncontrolled recursion and CWE-1333
regular-expression blow-up.

#### 7.4.2 What can be proved: timing, on our own stack

Wall-clock time on a third-party platform cannot be proved. It depends on
the CPU, caches, frequency scaling and everything else the program does not
control. So a timing guarantee takes the same assume–guarantee shape as
everything else:

```
   proved:     steps(decode, n)  <=  64n + 512
   assumed:    one SIR step      <=  t_step ns   on platform P  (measured, §7.4.3)
   guarantee:  time(decode, n)   <=  (64n + 512) * t_step ns    on P
```

On the repository's own stack the assumption disappears, by §7.5.6. Our
gate-level CPUs are cycle-accurate: they model the pipeline, and caches
too where a model has them. So a step bound proved at the SIR level can be
carried down through our compiler to **an exact worst-case cycle count**,
proved against the netlist itself. That is worst-case execution time (WCET)
analysis, normally done with approximate hardware models; here the hardware
model *is* the hardware being shipped. Constant-time claims carry down the
same way: the self-composition miter is built over the gate-level CPU, and
UNSAT proves that cycle counts do not depend on the secret.
These are labelled `proved-to-<layer>` like any other chained guarantee.

#### 7.4.3 What can only be measured: the microbenchmark harness

An in-house harness, with a lane in every language (§2.1):

- warm-up, then automatic calibration of the batch size;
- robust statistics: median, median absolute deviation and bootstrap
  confidence intervals;
- comparison against a stored baseline;
- sweeps over input sizes, for complexity checks.

One design point matters more than the rest: **shared CI runners are too
noisy to gate on wall-clock time.** Gating uses deterministic counts:

- instructions retired, where the platform exposes them;
- VM instruction counts, for code running on the repo's own VMs;
- allocations, which every language can count.

Wall-clock numbers are reported but never block. Measurement has three jobs
that proofs cannot do:

1. **Calibrate timing assumptions.** The harness measures `t_step` per
   platform, which turns a proved step bound into a time estimate (§7.4.2).
   Calibration is labelled `measured`, never `proved`, and so is any time
   bound derived from it. A measured median is not a worst case, so nothing
   may size a denial-of-service timeout from it. On third-party platforms
   the only worst-case guarantee is the proved step bound itself.
2. **Check that proved bounds are honest.** If an observed count ever
   exceeds a proved bound, the proof, the cost model or the front end is
   wrong. The gate treats that as a failure of the proof, not of the
   benchmark. If observed counts sit far below the bound across the whole
   sweep, the report flags the bound as loose, because a bound 1000× too
   high guarantees almost nothing.
3. **Catch regressions** in packages that state a `budget`: a performance
   target that is a goal, not a guarantee.

```
   bound   (proved)     decode: steps <= 64n + 512        ← guarantee
   budget  (measured)   decode: median <= 2 µs/KiB on P   ← target, regression-checked
```

### 7.5 Formal verification (E5) — VER09 to VER14

The proof machinery is built in-house, in layers, each of which checks the
one below:

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

#### 7.5.1 What "verified" means: assume–guarantee

A proof is about a formal meaning. A proof about Python code would, taken
literally, need a formal semantics of all of Python, and of CPython, the OS
and the CPU beneath it. Twenty-two of those is a career, not a project, and
most of it describes things this repository does not control anyway. Once
our code is handed to a language runtime, what the runtime does with it is
not ours to guarantee.

So E5 proves a **conditional guarantee**, the standard framing in
verification work, known as assume–guarantee reasoning:

```
    assumptions (platform + environment)   ⇒   claims hold for every input
```

What we control, and therefore prove, is the **implementation logic** of
the package. What we do not control is written down as an **assumption**,
named and explicit:

```
                      ┌────────────────────────────────────┐
   PROVED             │  our implementation logic          │  ← the package
                      ├────────────────────────────────────┤
   ASSUMED, NAMED,    │  the language subset we use        │  e.g. "i64 wraps",
   AND TESTED         │  the standard library calls we use │  "sort is stable",
   (§7.5.3)           │  the inputs we accept              │  "len(input) ≤ 2^20"
                      ├────────────────────────────────────┤
   OUT OF SCOPE       │  compiler, runtime, OS, hardware   │  not ours to control
                      └────────────────────────────────────┘
```

The guarantee is only as strong as its assumptions are true, so assumptions
are never left implicit. The next three subsections keep them honest.

#### 7.5.2 Assumptions are declared

Every Warrant file has an `assume` section (§4.3), in two kinds:

- **Platform assumptions** describe how the language and standard library
  behave on the constructs the package uses. They come from a shared,
  per-language **assumption catalogue** (VER13), so `i64.wraps` means the
  same thing everywhere. A package may only cite catalogue entries. It may
  not invent private assumptions about the platform.
- **Environment assumptions** describe the inputs and the world the package
  is given, for example "the callback does not re-enter" or "no other
  process writes this directory". These are the package's preconditions.
  The release gate shows them to users, because they are the contract a
  caller must keep.

**Hostile input is never assumed about.** An attacker does not keep a
contract. So an environment assumption may not constrain a hostile input:
"the input is at most 2^20 bytes" cannot be assumed of bytes that come off
the network. Such a limit must instead be a `rejects` clause. The code
enforces it, and the proof shows the code rejects every violating input.

**Every input is hostile unless approved otherwise.** If hostility were an
annotation the author adds, leaving it off would quietly switch off every
protection that depends on it: the ban above, the mandatory bounds of
§7.4.1, and E4. So the default is reversed. Every parameter of every
exported symbol is `hostile`. The analyzer also marks as hostile any value
that data flow traces back to a `net`, `fs`, `stdin`, `env` or IPC
capability, wherever it ends up. Marking an input `trusted` is a
downgrade, and it is an exemption (§9.1) that a named approver must
approve with a physical-presence signature.

The gate enforces three rules:

- the Warrant compiler refuses an environment assumption on a hostile
  input;
- a weakness-matrix row (§8.2) is never discharged by a proof that
  relies on an environment assumption about attacker-controlled data;
- every environment assumption that a proof uses must be a **formal**
  predicate the solver can read, not prose. Prose assumptions may document
  a package, but no proof may rest on one.

The assurance report lists, for every proved claim, the exact assumptions
the proof used. An assumption no proof uses is flagged for removal.

#### 7.5.3 Assumptions are tested in every language

Each platform assumption in the catalogue has a **conformance test** in its
language's lane: does integer overflow wrap as assumed, is `sort` stable,
does a map return the last value stored. These run on every toolchain the
repository supports. When a new language version breaks one, the gate fails
and names the assumption, rather than the guarantee silently becoming
false.

#### 7.5.4 The code stays inside the assumed subset

Assumptions describe a **subset** of each language: the constructs whose
behaviour the catalogue pins down. The per-language analyzer (VER08, the
same one that audits capabilities) rejects code that leaves that subset:

- `eval` and dynamic code loading;
- monkey-patching, and redefining builtins;
- reflection used to reach private state;
- `unsafe`, FFI and `cgo` without a matching native proof (§8.3).

That restriction is on *our* code, so it is fully in scope. Code that
genuinely needs a construct outside the subset is not provable. It ships
with tests and fuzzing, labelled `not-yet-provable` (§7.6.5) until the
catalogue grows to cover it.

#### 7.5.5 The model of our logic

To prove anything, the code must be translated into a precise model. That
model is `semantic-ir` (§7.6.2). Front ends already lower Python,
JavaScript, Ruby, Java, C and Twig into it, and SIR21 pins its integer
semantics. A front end is our code too, so it is checked like our code:
the property engine continuously runs the SIR model and the real
implementation side by side on the same generated inputs, and any
divergence is a bug in the front end.

Languages without a SIR front end get one, in the order packages in that
language want to ship (VER14). Until a language has one, its packages are
labelled `not-yet-provable`.

#### 7.5.6 Owned layers: assumptions become guarantees

The assume–guarantee shape composes, and this repository is unusual in
owning the layers underneath its libraries. It has its own:

- Python lexer, parser and compiler (`python-parser`, `python-iir-compiler`);
- Python VM (PY02);
- register VM and IR stack;
- gate-level CPUs, down to `logic-gates` and transistors.

When the layer beneath a package is one of ours, a platform assumption the
package makes is **not an assumption at all. It is a claim of the layer
below, and that layer proves it.**

```
   library        claims C          assume i64.wraps, sort.stable
                     ▲                         │
                     │ discharged by           ▼
   our Python VM  claims: i64.wraps, sort.stable      assume: ISA semantics
                     ▲                                       │
                     │ discharged by                         ▼
   our CPU (gate-  claims: ISA semantics (proved by           assume: Boolean gates
   level netlist)  equivalence checking, §7.6.1)              behave as gates
```

Proofs chain upward. The library's claims hold if the VM's claims hold, and
those hold if the CPU's claims hold. The only assumptions left are at the
bottom of the stack this repository owns. The goal is the one pursued by
the verified-stack projects: CompCert (a verified C compiler), CakeML (a
verified ML compiler and runtime) and seL4 (a verified OS kernel).

Three rules make this work:

1. **Catalogue entries are claims somewhere.** Each entry in the
   per-language assumption catalogue (VER13) is, word for word, a claim in
   the Warrant file of the repository's own implementation of that
   language, where one exists. Assumption and claim are the same formula,
   so discharging one with the other is a check, not an argument.
2. **The report says which stack a guarantee is for.** One package, two
   guarantees:
   - On a third-party runtime (CPython, V8, the JVM), its platform
     assumptions stay assumptions, backed by the conformance tests of
     §7.5.3.
   - On the repository's own stack, those same assumptions are discharged
     by proofs, and the report labels the guarantee `proved-to-<layer>`,
     naming the lowest owned layer the proof chain reaches.
3. **A layer can only discharge what it has proved.** If our Python VM's
   `sort.stable` claim is only `bounded(k)`, a library relying on it
   inherits `bounded(k)`. The weakest label in a chain is the chain's label,
   the same never-round-up rule as §7.6.5.

This also says where proof effort pays off most. Proving a claim once in
a low layer, such as our VM's integer arithmetic, discharges that assumption
for every package above it. Lower-layer proofs are therefore the
highest-leverage work in the program.

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
hands back the `x`: a concrete counterexample. It goes through the same
private triage as a fuzz crash (§7.3), then becomes a permanent unit test.
If it answers **UNSAT**, no such `x` exists among *all* inputs,
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
meaning: the model of our logic, under the package's stated platform
assumptions (§7.5.5).

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
  that contradicts a `claim`, and either one **fails the gate**. The repository has already learned this lesson
  the hard way (`lessons.d/a-substring-check-is-vacuous-for-the-shortest-member-of-the-set.md`).
- **Consistency**: the claims of a package can all be true at once. A spec
  that contradicts itself is rejected before anyone implements it.
- **Model-level proofs**: claims about the Warrant `model` are proved
  outright, before any implementation exists.
- **Assumption hygiene**: the solver checks that the `assume` section is
  itself consistent, and finds any assumption strong enough to make a claim
  trivially true. An assumption like `len <= 0` proves everything and
  guarantees nothing. An inconsistent or trivializing assumption **fails
  the gate**. It is not merely flagged, because a proof that rests on one
  would otherwise ship labelled `proved`.
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

Every E5 result in the assurance report carries exactly one label, plus the
list of assumptions it used. The gate never rounds a weaker label up:

| Label | Meaning |
|---|---|
| `proved` | UNSAT with a checked certificate, over all inputs, under the listed assumptions |
| `proved-to-<layer>` | As `proved`, with platform assumptions discharged by our own layers down to `<layer>` (§7.5.6) |
| `proved-per-run` | Translation validation: this artifact, not the tool in general |
| `bounded(k)` | No counterexample within `k` cycles, unrollings or steps |
| `proved-over-model` | Proved about the Warrant model only; no SIR front end links the implementation yet |
| `not-yet-provable` | The code uses constructs outside the assumed subset, or its language has no front end yet. It ships on tests and fuzzing, and the report says so. |

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
3. **The whole shipped closure is audited.** A package's effective
   capabilities are the union of its own manifest and those of every
   package in its shipped closure, and that union is what users see. The
   closure is computed from the artifact itself (§9.2, step 1). A
   third-party dependency in it counts as holding unbounded capabilities.
4. **Capability changes need approval at release.** The gate compares the
   **effective** set from rule 3 at the release tag against a baseline. It
   never compares the package's own manifest alone, because a capability
   added to a dependency changes what ships just as much. Any new capability
   blocks the release until it is approved, the way Apple treats a change to
   an app's entitlements. Three rules keep this from being bypassed:
   - **An immutable baseline.** The baseline is the effective capability set
     recorded in the previous release's assurance report, which is stored
     where a releaser cannot rewrite it (§9.2). It is never taken from a
     mutable git tag or from the external registry.
   - **A first release has no baseline**, so every capability in it counts as
     new and needs approval.
   - **Approval is a physical-presence signature** by a named approver
     (§9.1, §9.5). It is not a CODEOWNERS click or an environment approval,
     because an agent holding an account's API token can perform either.
     It is recorded alongside the assurance report, not in release notes
     the releaser writes. §9.1 ties each approval to the exact change it
     approves.

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
| Applies to | C, C++, Rust `unsafe`, any FFI boundary. Not safe Rust, Python, Ruby, TS, JVM, .NET (without `unsafe`), BEAM. Go only conditionally: even without `unsafe`/cgo, a data race on a multi-word value (an interface, slice or string) can corrupt memory. So Go's exclusion holds only once race freedom is discharged. Goroutines are a language construct, not a D21 capability call, so the simulated host's scheduler cannot control them, and §8.4 alone does not apply. VER15 specifies a Go-specific discharge, such as a race-freedom analysis over SIR or a restricted concurrency subset, plus `-race` on all tests. |
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

### 8.5 Observed capabilities: running the package in a locked box — VER22

The capability analyzers of §8.1 read the code and infer what it *might*
do. That is necessary, and it is not enough: an analyzer has blind spots,
and a manifest can be wrong in ways nobody reads closely enough to notice.
This section adds the dynamic half. Run the package inside a locked box
and **watch what it actually does**.

#### 8.5.1 What runs in the box

A library does nothing on its own, so something must drive it. The driver
is the harness the Warrant compiler already generates (§4): the package's
examples, integration scenarios, property tests and fuzz targets, run as a
program. Programs and command-line tools also run directly, under their
`scenario` blocks. For gate runs, the box is a VM (§8.5.4). That is
stricter than the general "OS or VM boundary" §9.2.1 asks of dynamic
rungs, so a dynamic rung that also feeds this check must run in the VM.

#### 8.5.2 The box: built on `capability-os-sandbox`

The starting point is `capability-os-sandbox`. It lowers a
`required_capabilities.json` into a deny-by-default sandbox *plan* per OS:

- seccomp and Landlock on Linux;
- Seatbelt on macOS;
- AppContainer on Windows;
- equivalents on FreeBSD and OpenBSD.

Every rule is labelled with how well it is enforced, and an unsupported
capability is a launch failure, never a silent downgrade.

**What is missing is most of the enforcement.** Today the only code that
applies a plan is the macOS Seatbelt applier, and its profile still starts
from `(allow default)`. The base deny-all policy is modelled but not
installed. There is **no applier at all** for Linux, Windows or the BSDs,
and Linux is the platform the gate's VM uses. VER22 owns building them,
in coordination with D18S, and it settles which of the two specs owns which
platform. Two platform limits shape the design:

- Landlock restricts network use by port only, not address.
- Seatbelt filters network by port, not host.

So enforcing *which host* a package may reach needs the host broker or a
filter on the host side of the VM's network device. The kernel primitives
inside the box cannot do it.

#### 8.5.3 Enforcement and observation are separate jobs

Enforcement must be **race-free**. Observation must **miss nothing**. No
single mechanism does both, so the box uses each for what it does well.

**Enforcement rests only on race-free primitives:**

- Landlock for the filesystem;
- the host-side egress filter for the network;
- the **broker** (below), for the process spawns and wildcard filesystem
  grants that `capability-os-sandbox` routes through it;
- seccomp filters that return an error or kill the process, for whole
  classes of system call.

**seccomp user notification is used to observe, never to approve.** A
supervisor process that is told about each relevant system call is the
natural way to watch. But it reads pointer arguments (paths, socket
addresses) out of the process's memory. If it then tells the kernel to
continue, the kernel reads those arguments *again*, and another thread can
swap a declared path for an undeclared one in between. A `userfaultfd`
stall or a symlink swapped after the check works just as well. The
`seccomp_unotify(2)` manual itself warns against using "continue" for
security policy. So the supervisor **never lets a pointer-argument call
continue on the strength of its own check**. Either:

- it **only observes, and lets the race-free layer decide**. This is the
  default;
- or, where VER22 justifies it per system call, it *emulates* the call on
  its own copy of the arguments. Emulation must not become a way around
  enforcement. A supervisor opening a file with its own credentials would
  skip Landlock and the package's permissions entirely, because both are
  checked only in the process that opens. So the emulation runs in a
  helper with:
  - the package's user, group and supplementary groups, in the same user
    namespace;
  - an **empty set of Linux capabilities** (no `CAP_DAC_OVERRIDE` or
    similar) and `no_new_privs`;
  - **the same Landlock ruleset**;
  - the same mount namespace;
  - the root and working directory **of the calling thread at the time of
    the call**, read under a `NOTIF_ID_VALID` check. Threads can
    `chdir` or `unshare(CLONE_FS)`, so the values from launch time are not
    reliable;
  - directory descriptors for relative paths taken from the package via
    `pidfd_getfd`.

  The helper resolves paths with `openat2` and its `RESOLVE_*` flags,
  injects the resulting descriptor with `SECCOMP_IOCTL_NOTIF_ADDFD`, and
  confirms with `NOTIF_ID_VALID` that the call is still current after
  reading memory. Its denials are recorded like any other.

`userfaultfd` stays denied by the base policy in every mode.

**Every denial is evidence.** Landlock denials, seccomp errors and
Seatbelt denials are silent by default. A package that tries an undeclared
`connect`, catches the error and carries on would leave no trace and pass,
and in production, outside the box, that attempt succeeds. So every
denial, at every layer, is recorded as an observation:

- **seccomp.** Denials are preferably delivered as user notifications
  that the supervisor answers with an error. That is lossless, and it is
  race-free because it denies rather than continues. Filters that return
  an error or kill directly are installed with `SECCOMP_FILTER_FLAG_LOG`,
  with those actions enabled in `/proc/sys/kernel/seccomp/actions_logged`.
  (`SECCOMP_RET_LOG` is *not* a way to record denials: it means "log, then
  **allow**".)
- **Landlock** auditing needs Linux 6.15 or later. Because the launcher
  restricts itself and then starts the runtime with `execve`, the domain
  is created with `LANDLOCK_RESTRICT_SELF_LOG_NEW_EXEC_ON`. Otherwise
  denials after the `exec` are not logged.
- **The broker** logs every request it refuses.
- **The host-side egress filter and the resolver** record every packet
  they drop and every name they refuse. The supervisor's own record of
  `connect` and `send` calls cannot stand in for these logs. It only
  observes, so it reads pointer arguments racily: a second thread can
  show it a declared address while the kernel sends to an undeclared one.
  All of these channels must be lossless, and a drop that cannot be
  observed makes the run "not checked".
- **Seatbelt** (development only, §8.5.4) logs deny violations unless they
  are silenced with `(with no-report)`. That logging is rate-limited.

The audit path can lose records, through rate limits or a full backlog.
**Any loss of audit records during a run turns its result into "not
checked"**, never into an empty set of denials.

**Any denial fails the gate, whatever the process's exit status.** Two
kinds of error are tagged and not counted as denials:

- failures injected on purpose for chaos testing (below);
- **policy-shaping returns**, such as forcing `clone3` to fail with
  `ENOSYS` (§8.5.5). Without this tag, `SECCOMP_FILTER_FLAG_LOG` would log
  that return and fail every run that starts a thread.

Audit loss is detected by comparing the audit subsystem's `lost` counter
before and after the run. Gate runs disable audit rate limiting and set a
large backlog. **A gate run requires a guest in which
every enforcing layer's denials are observable**: a Linux 6.15+ guest
kernel, auditing enabled, and the logging flags above. If any enforcing
layer's denials cannot be observed, the result is "not checked", which
blocks the gate, never "pass, with a caveat".

**Two modes.**

| Mode | Behaviour | Used for |
|---|---|---|
| **Enforce** | Anything outside the manifest is denied; every denial is recorded and fails the run | **All** gate evidence |
| **Observe** | Observable operations are allowed and recorded. Two sets of denials stay in place in this mode, as in every mode. First, the base policy's denials of *unobservable* channels: the architecture check (so 32-bit and x32 system-call numbers cannot alias others), `io_uring`, `ptrace`, `process_vm_writev`, `userfaultfd`, `bpf` and `perf_event_open`. Second, the integrity denials of §8.5.4: `vsock`, and signals to the supervisor. | Proposing a manifest for a new package; finding over-declaration |

Observe mode never supplies gate evidence. A manifest drafted from it is a
**proposal for human review**, because it would otherwise launder whatever
the code happened to do straight into the declaration. And untrusted code,
such as code from a pull request, runs in observe mode **only in a VM with
no credentials and with network egress sinkholed**, never in a container
on a maintainer's machine (§9.4, rule 2).

**Chaos comes for free.** The same supervisor can make an observed call
fail on purpose, with `ENOSPC`, `ECONNRESET` or a short write. That brings
the fault plans of §6.4 to every language at the system-call level.

#### 8.5.4 The box's own integrity

The supervisor runs inside the same guest as the code it watches, so it
must be protected from that code:

- it runs as a **separate user**;
- the package is denied `ptrace`, signals to the supervisor, and `vsock`;
- the channel to the host accepts **one** connection, opened before the
  package starts, and its listening socket never leaks into the box;
- a supervisor crash or timeout fails the run, and a call the supervisor
  has not answered is never allowed to proceed;
- network traffic is observed and enforced from the **host side** of the
  VM's network device wherever possible, out of the guest's reach.

Even so, **a compromised guest kernel defeats observation**. Observations
leave the box only through the host-side signed channel of §9.2.1, but
that signature covers only what the supervisor reported. So the gate's
security never rests on observation alone. It rests on the static check
S ⊆ D (§8.5.7) together with race-free enforcement.

**No VM, no check.** If the VM boundary is unavailable, the gate fails. It
never falls back to a container or to Seatbelt on the bare host. A platform
that cannot observe yields the result **"not checked"**, never "pass":
otherwise an empty observation set would satisfy every check in §8.5.7
vacuously. macOS has no unprivileged per-system-call supervisor (Endpoint
Security needs an entitlement and root), so until VER22 specifies one,
**the gate's locked-box runs use Linux guests**. macOS boxes are for
development.

#### 8.5.5 From system calls to capabilities

The mapping is a **closed allowlist over system-call numbers, per CPU
architecture**. Any system call the mapping does not classify is a
violation in enforce mode and an "unclassified" failure in observe mode.
Targets are checked as well as categories: which path, which address,
which program. The table below is **illustrative**. VER22 holds the
complete mapping.

| Observed (examples; each family includes its aliases) | Capability |
|---|---|
| `open`, `openat`, `openat2`, `creat`, `open_by_handle_at` with read flags; `stat`, `statx`, `newfstatat`, `faccessat2`, `readlink`, `getdents64` | `fs.read`, `fs.list` on that path |
| The same opens with write or create flags; `rename`, `renameat2`, `unlink`, `unlinkat`, `rmdir`, `mkdir`, `link`, `symlink`, `truncate`, `chmod`, `chown`, `setxattr`, `mknod` | `fs.write`, `fs.create`, `fs.delete` on that path |
| `socket` of any family; `connect`; `sendto`, `sendmsg`, `sendmmsg` (whose address is a nested pointer) | `net.connect` to that **IP and port**. AF_UNIX, AF_NETLINK, AF_PACKET, AF_VSOCK and raw sockets are each mapped explicitly and denied unless declared |
| `bind`, `listen` | `net.listen` on that address |
| `execve`, `execveat`, `fexecve`; `clone` and `clone3` creating a process, `vfork`; `kill`, `tkill`, `tgkill`, `pidfd_send_signal`, `rt_sigqueueinfo` | `proc.exec` of that program, `proc.fork`, `proc.signal` |
| `clone` with `CLONE_THREAD` | `thread.spawn` (§6.3), not `proc.fork`. seccomp cannot read `clone3`'s flags, which sit behind a pointer, so the box makes `clone3` fail with `ENOSYS`. The C library then falls back to `clone`, whose flags are register arguments |
| Reads of `/dev/urandom` or `/dev/random`; `getrandom` when it is a real system call | `rand.read` |
| A read or write on the descriptor that *currently* refers to the process's stdin or stdout, after tracking `dup`, `dup2`, `fcntl` and descriptors passed with `SCM_RIGHTS` or `pidfd_getfd` | `stdin.read`, `stdout.write` |

Three rules make the network rows honest:

- **Targets are IP-level.** System calls see an IP address and port, not a
  hostname. Shared CDN addresses and domain fronting defeat any attempt
  to match addresses to declared hostnames. A hostname-level grant is
  enforced only when egress goes through a trusted host-side proxy or
  broker that resolves the name itself and sees the TLS SNI and HTTP Host.
- **`net.dns` is checked against the name queried.** D21 defines a DNS
  grant by the host*name* being resolved (`net:dns:{host}`). Restricting
  *which resolver* is used is not enough: a recursive resolver forwards any
  name, including one under an attacker's domain, straight to that
  attacker's server, which is the classic way to exfiltrate data through
  DNS. So:
  - DNS goes only to the configured resolver. A connection to any other
    host on port 53 is `net.connect` to that host;
  - the observed target is the **queried name**, taken from the broker log
    or a host-side parse of the DNS traffic, and checked exactly against
    the declared names. "Exactly" means after a defined canonical form:
    case folded (resolvers randomise letter case), trailing dot removed,
    and IDNA/punycode normalised;
  - the broker or resolver **refuses** undeclared names.

  Low-bandwidth channels remain even then, through the timing, type and
  number of queries for *declared* names. They are accepted, in writing,
  as residual risk. VER22 enumerates every resolver channel: classic DNS over UDP and TCP,
  DNS over HTTPS (443) and over TLS (853), mDNS (5353), and the local
  resolver daemons' sockets (nscd, systemd-resolved, macOS mDNSResponder).
- **Brokered operations are observed at the broker, and the broker stays
  inside the VM.** `capability-os-sandbox` lowers network grants, process
  grants and wildcard filesystem grants to a broker. The package then
  performs those operations by writing to a broker connection, not with
  `connect` or `execve`, so system-call observation alone misses them.
  - The observation set includes the **broker's own operation log**.
  - The broker is part of the gate's trust root (§9.4).
  - The broker does filesystem work on the package's behalf, so it must
    not become a way around enforcement, as emulation could (§8.5.3):
    - it resolves every path from a pinned descriptor for the grant's root,
      using `openat2` with `RESOLVE_BENEATH`, `RESOLVE_NO_MAGICLINKS` and
      `RESOLVE_NO_SYMLINKS` (or `RESOLVE_NO_XDEV` as required);
    - it matches the wildcard against the *resolved* path;
    - it runs as the package's user, with no Linux capabilities and a
      Landlock domain limited to the grant's base directory;
    - it passes descriptors back, not paths.
  - It also gets the supervisor's integrity protections (§8.5.4): a
    separate user, and no `ptrace` of it or signals to it from the
    package.
  - **Brokered process spawns and filesystem operations run inside the
    guest.** A broker on the VM's host would serve the *host's*
    filesystem and start programs *outside* the VM, escaping the boundary
    entirely. Children the broker starts get the restricted subset plan of
    D18S under the same supervisor, and their system calls and denials
    join O, charged to the package.
  - Only network egress mediation sits on the VM host.

**What the box cannot observe**, which the report lists as not observable
rather than implying it was checked:

- Environment variables: reading one is a memory read. The box instead
  starts the package with an explicitly constructed environment block, so
  an undeclared variable is simply absent.
- Most clock reads, which on Linux go through the vDSO without entering
  the kernel.
- Randomness from the vDSO (`vgetrandom`, Linux 6.11+ with glibc 2.41+)
  or from a CPU instruction.
- **`ffi` is only partly observable.** Mapping a *new* library is visible.
  Calling into a library that is already loaded (libc, through ctypes or
  `dlsym`) is not, and neither is making anonymous or `memfd` memory
  executable, which JIT runtimes do constantly. A package that declares
  `ffi` is boxed with the VM as the real boundary, every observation is
  charged to the package, and stack attribution is never used for it.
  (`capability-os-sandbox` currently refuses to launch any plan with an
  `ffi` grant. VER22 must settle that explicitly rather than special-case
  it.)

These remain the job of the language-level wrappers and the static
analyzer.

#### 8.5.6 Separating the runtime from the package

An interpreter does a great deal before the package runs a single line.
Python and Node read their standard libraries, locale data and random
devices just to start. Charging that to the package would make every
manifest wrong. Excusing it carelessly would hide real violations, and the
code under test gets a say in how carefully it is excused. Hence one hard
rule:

**Nothing may excuse an observation except an exact entry in a protected
baseline.** The baseline is a profile of the runtime alone, an empty
harness on the same toolchain. It is recorded and stored under the
protected ref (§9.4) as exact (operation, target) pairs, never categories.
An observation matching one of those pairs is charged to the runtime.
**Everything else is charged to the package.**

**Attribution only moves charges; it never removes one.** Reading the
call stack to see which module made a call can *move* a charge between
the package and an in-repo dependency. That charge is then checked
against the dependency's own manifest, which is exactly the closure union
of §8.1. Attribution can never move a charge to "the runtime", because
the code under test can forge stacks:

- callbacks that runtime frames run (finalizers, exit and signal handlers,
  thread pools, promise jobs, import hooks);
- monkey-patched standard-library functions;
- fake frame metadata (a forged `co_filename`, a `sourceURL` posing as
  `node:internal`, a custom `prepareStackTrace`);
- native code that forges its own unwind data.

Building reliable attribution is the research part of VER22. Until it
exists, every charge not excused by the baseline goes to the package,
including charges that really belong to its dependencies. That is why the
checks in §8.5.7 compare against the package's **effective** capability
set (§8.1, rule 3), the union over its whole closure. Attribution between
the package and its in-repo dependencies is as forgeable as attribution to
the runtime. But it can only move charges *within* that union, so it is
harmless while gate decisions use the effective set.

Two further rules keep the baseline workable:

- **The enforce-mode plan grants exactly the baseline pairs**, so the
  runtime's own start-up reads are not denied and do not fail every run.
- **Runtimes with unpredictable targets** need a rule of their own. A
  runtime that writes caches or temporary files under generated names
  cannot be described as exact pairs. VER22 must specify the rule, for
  example a dedicated per-run directory granted to the runtime alone,
  without falling back to whole categories.

#### 8.5.7 Three views, cross-checked

Each package has three capability sets: what its manifest **declares**
(D), what the static analyzer **infers** (S), and what the box
**observes** in enforce mode (O). D and S are both taken over the
package's whole closure, as its effective set (§8.1, rule 3). The gate
checks:

| Check | If it fails |
|---|---|
| O ⊆ D | A **violation**: the package did, or tried to do, something it never declared. This fails the gate. |
| S ⊆ D | Today's static check (§8.1): the code *could* do something undeclared. This fails the gate. |
| O ⊆ S | **The static analyzer has a blind spot**: the package did something the analyzer could not see. This fails the gate, and it is filed as a bug against the analyzer, which gets a regression test. |
| D ⊆ O ∪ S | Over-declaration: a declared capability that is neither observed nor inferred. This is a warning that proposes narrowing the manifest. |

The third row is the bonus. The box is a continuous, free test oracle for
the capability analyzers in all 22 languages, and the analyzers are part
of the gate's own trust root (§9.4).

#### 8.5.8 What it cannot do

- **It sees only the code paths that actually run.** A capability used
  only on an input the harness never generates goes unobserved. That is
  why O is checked against S as well as against D, and why the harness is
  driven by the property and fuzz engines rather than by fixed examples:
  their job here is to make O as large as possible.
- **It is no defence against a package that knows it is being watched.** A
  hostile package can detect the box, through the seccomp status in
  `/proc/self/status`, the hypervisor CPU flag or the VM's firmware
  strings, and behave until it is outside. A time bomb never fires during
  a test. Dynamic checking therefore gives assurance against **honest
  mistakes and analyzer blind spots**, not against an adversarial
  package. Against an adversary, the protection is the static check
  (S ⊆ D), the capability analyzers of §8.1, and review.
- **A passing run is evidence, not proof.** It shows nothing undeclared
  happened **on the paths exercised**. Proving that nothing undeclared
  *can* happen is the static analyzer's job, and the report never conflates
  the two.
- **The box is configured from the protected ref.** The sandbox plan is
  derived from the manifest by the gate's own code. The supervisor, the
  mapping, the broker and the baseline profiles are part of the gate's
  trust root (§9.4), never supplied by the commit being judged.

### 8.6 Generated capability modules: you cannot name what you did not declare — VER23

The capability analyzer of §8.1 has a hard job. It must recognise every
way a program in a given language can touch the outside world: every
standard-library function, every alias, every indirect route. Whatever it
fails to recognise is a blind spot. This section turns the problem around.
**Generate, from the manifest, the only module through which the package
can reach the world, and forbid everything else.** A package that declared
only `fs.write` on `cache/` then has no way to even *write the words* for
reading a file. There is no function for it to call.

This is D21's design taken to its strongest form. It is the same idea as
capability-safe language subsets such as Joe-E for Java and SES (Secure
ECMAScript) for JavaScript.

#### 8.6.1 What the generator emits

For each package, in each language, the generator turns
`required_capabilities.json` into one module. It contains **exactly one
function per declared capability, scoped to the declared target**, and
nothing else:

```
manifest:  { "category": "fs", "action": "write", "target": "cache/*" }

generated (Python lane, abridged):
    # Generated from required_capabilities.json. Do not edit.
    def write_cache_file(path: CachePath, data: bytes) -> None: ...
```

- **There is no `read`, no `delete`, and no general `open`.** An undeclared
  operation is not refused at runtime; it cannot be expressed.
- **Targets are enforced where they are used, not just typed.** A
  `CachePath` type documents intent, but in Python, JavaScript,
  TypeScript and Ruby a type can be forged (`object.__new__`, a
  structural cast), and a check made when a path is constructed is out of
  date by the time it is used. So the **production backend** enforces the
  target at the moment of use:
  - it resolves every path against a directory descriptor pinned at
    start-up from a trusted root, never the working directory;
  - on Linux it uses `openat2` with `RESOLVE_BENEATH` and
    `RESOLVE_NO_MAGICLINKS`, plus `RESOLVE_NO_SYMLINKS` or
    `RESOLVE_NO_XDEV` where the grant requires;
  - on macOS, which has no `openat2`, it walks the path one component at a
    time with `openat` and `O_NOFOLLOW` (or `O_NOFOLLOW_ANY`);
  - on Windows it uses handle-relative opens and checks reparse points,
    alternate data streams, device names, 8.3 short names and case;
  - it rejects NUL bytes and defines exactly what a wildcard matches (for
    example, whether `*` crosses `/`).

  Hard links that already exist in the directory defeat `RESOLVE_BENEATH`,
  so VER23 says how granted directories are created and kept clean of
  them.

  A declared network target produces a function that connects to that
  address and port (§8.5.5), not a general `connect(host)`.

  A declared `proc.exec` produces a function that runs **one program**,
  pinned by absolute path and content hash. Each argument is checked
  against a schema, and options are separated from arguments with `--`. A
  free-form string argument is close to arbitrary code for many programs
  (`git -c core.sshCommand`, `tar --to-command`, `find -exec`). The
  program's own authority counts toward the package's effective set:
  unbounded, unless it is an in-repo package with its own manifest.
- **The manifest is compiled in.** As `capability-cage-generator` already
  does for Go, the grants become constants in the build, so editing a
  JSON file at runtime grants nothing.
- **Two backends, one interface.** The production backend performs the
  operation through the D21 runtime. The simulated-host backend (§6) sits
  behind the same functions for unit tests, and is compiled out of
  shipped artifacts (§6.1).
- **Auditing a package's effects means reading the short generated files
  of its closure**: its own module and those of its in-repo dependencies.
  Reviewers, the gate and the weakness registry all start there. A
  generated module may be imported **only by its own package**, so one
  package cannot borrow another's grants by importing them directly.

The repository already has two early generators. `capability-cage-generator`
(Go) bakes the manifest in, but still exposes every operation and checks
it at runtime. `required-capabilities-compiler` (Rust) generates an HTTP
client limited to the declared domains. VER23 moves Go to "only what was
declared", generalises the Rust compiler beyond HTTP, and adds the rest of
the language lanes (§2.1).

The generator is part of the gate's trust root (§9.4). The gate
**regenerates** every package's module and compares it with the committed
one, never trusting the committed copy (§9.2.1).

#### 8.6.2 Everything else is forbidden: a closed allowlist of names, per member

A generated module confines nothing if the package can reach the world
some other way. Imports are only one of those ways. Many languages hand
every program effects **without any import**:

- Python's builtins: `open`, `exec`, `eval`, `compile`, `__import__`,
  `print`, `input`;
- JavaScript's globals: `globalThis.process` (with `.binding`, `.dlopen`
  and `.env`), `fetch` and `WebSocket` in Node 18+, `WebAssembly`;
- Ruby's `Kernel` methods on every object (`system`, backticks, `spawn`,
  `exec`, `fork`, `open("|cmd")`) and core constants such as `File`, `IO`
  and `ObjectSpace`;
- Java's implicitly imported `java.lang`: `System.getenv`, `System.load`,
  `Runtime`, `ProcessBuilder`, `Class.forName`;
- Erlang and Elixir modules called by name, including `apply` with a
  computed module;
- Rust's fully qualified paths, such as `::std::fs::read`, with no `use`;
- Go's `print` and `println`.

So the rule is not an import list. **Every name the package uses that
resolves outside the package must be on a closed allowlist.** That covers
imports, builtins, globals, implicitly available namespaces and fully
qualified paths alike. What may be on the list:

- the package's own generated module;
- in-repo dependencies, each confined in the same way;
- standard-library members on the language's reviewed
  **pure-computation list**.

**The list is per member, not per module.** Allowing a whole module is
too coarse, because an allowed module is a doorway to everything reachable
from it. Python's `typing.sys`, `collections._sys` and `dataclasses.sys`
all lead to `sys.modules['os']`. So the list is kept at (module, member)
granularity, the way Joe-E's "taming database" works. A member is
reviewed against **everything reachable from it**, not just its documented
behaviour. Some members that look pure have effects in D21's categories,
and those are capabilities like any other, usable only through a
declaration:

| Looks pure | Is actually |
|---|---|
| `print`, `console.log`, `fmt.Println`, `println!` | `stdout.write` |
| `datetime.now`, `Date.now`, `time.Now` | `time.read` |
| `random`, `Math.random`, auto-seeded `math/rand` | `rand.read` |
| `codecs.open`, `io.open` | `fs.*` |
| Go `plugin`, `runtime/debug.WriteHeapDump` | `ffi.load`, `fs.write` |

The pure lists are **trust-root policy data** (§9.4), because a member
wrongly listed as pure is a hole. A member goes on a list only after that
review, and when in doubt it stays off.

**Reflection is rejected outright**, because it turns any allowed object
into a path to everything else. That means:

- double-underscore and private-attribute access
  (`().__class__.__mro__`, a function's `__globals__`);
- computed attribute and method access (`getattr` with a computed name,
  `send`, `__send__`, `public_send`, `method`, `const_get`,
  `instance_variable_get`, `operator.attrgetter`, `operator.methodcaller`);
- `constructor` access in JavaScript (`({}).constructor.constructor`
  reaches `Function` without writing either word);
- `gc`, `sys`, `inspect`, `ObjectSpace`;
- `eval`, `exec`, `compile`, `Function`, and dynamic import of a
  non-literal name;
- **deserialisers that look up names**: `pickle` and `marshal`, Ruby's
  `Marshal.load` and `YAML.unsafe_load`, and `JSON.load` with
  `create_additions`. These can call `os.system` from a byte string.

**Lock the globals as well, where the runtime allows it.** JavaScript runs
under SES-style `lockdown`: frozen built-in objects, with tamed `Function`
and `eval`. Python's builtins can be replaced at start-up. This is defence
in depth, not the primary control.

**The check runs on code as it is compiled, not as it is written.**
Several things never appear in source text:

- the output of macros: Rust proc-macros and `macro_rules!` from
  dependencies, which can emit a `$crate`-style path to `std::fs`;
- code that runs during compilation with full authority: Elixir macros
  and module attributes, Java annotation processors;
- embedded files (`include_bytes!`, `//go:embed`), Rust `global_asm!`,
  `#[no_mangle]` symbol interposition, and Go assembly (`.s`) files, which
  are native code (§8.6.3).

So the allowlist is checked after expansion: Rust's HIR or MIR, or the
compiled form, depending on the language. Code that runs at compile time
comes only from trust-root crates and macros, or is itself subject to the
allowlist.

**The check covers every way the package can be built.** That means all
code reachable in the shipped artifact under every feature, extra or
`cfg` a consumer can enable (Cargo features are additive), including
`TYPE_CHECKING` blocks, `try`/`except ImportError` fallbacks and
`__main__` blocks. Test code is excluded only through the mechanism of
§6.1.

**An escape fails the gate, or loses the label.** If code steps outside
these rules, through reflection, `eval`, a dynamic import or a native
escape, the package is not quietly accepted and labelled
`not-yet-provable`, as §7.5.4 would do for proofs. One of two things
happens, and the report says which:

- the gate fails; or
- the package is labelled **"not language-confined"**. Its S then comes
  from the full effect-inference analyzer of §8.1, not from the
  allowlist, and that downgrade is an exemption needing approval (§9.1).

**S is computed from use, not from imports.** S is the set of generated
functions the package actually references, or can reach, not the whole
generated module. Otherwise S would equal D by construction, S ⊆ D would
say nothing, and over-declaration would never be found.

#### 8.6.3 Where the language cannot confine: native code

C and C++ can reach the kernel through any pointer, inline assembly or a
raw system-call instruction. So can Rust `unsafe`, Go cgo and assembly,
and anything behind an FFI boundary. This is how the `unsafe`-with-proof
rule of §7.5.4 and the allowlist above fit together: such code is
confined only by the means below. **No list of names or symbols can
confine native code in production.** Native code running inside an
interpreter that already has a full C library can find it anyway:

- by walking the dynamic linker's state or the auxiliary vector;
- through the vDSO or return addresses;
- by calling the host runtime's own API (`PyRun_SimpleString`, JNI) or
  `dlsym`.

A statically linked binary has no imported symbols to inspect. On x86,
instructions overlap, so a system-call instruction can hide inside other
instructions. Native code can also jump straight to a shim's own
system-call instruction, skipping its checks. The Go runtime and Rust's
standard library contain raw system calls of their own.

So, for native code:

- **The report says so plainly.** It labels the package **"not confined
  in production; observed on exercised paths only"**.
- **A restricted shim and binary checks still run, but as a lint against
  honest mistakes.** The shim exports only the declared operations, and
  never `syscall()`, `ioctl`, `prctl`, `dlopen` or `dlsym`. The binary is
  checked for undeclared imported symbols and memory that is both
  writable and executable. Neither is a security boundary, and the report
  does not present them as one.
- **The locked box (§8.5) observes the package only during gate runs.**
  It is not present in production, and a package that detects it can
  behave until released (§8.5.8).
- **Real confinement comes from one of three things:**
  - native-code proofs (§8.3);
  - a sandbox applied in production (D18S);
  - compiling the native code to WebAssembly and running it in the
    repository's own WebAssembly runtime, which confines it by
    construction (software fault isolation).

#### 8.6.4 Libraries take capabilities as arguments

A module-level function is still *ambient*: any code path in the package
can call it at any time. The stronger form, **object capabilities** in the
strict sense, is for a library to receive its capabilities as arguments.
The embedding program builds capability objects from its own generated
module in `main` and passes them in. A library's manifest then declares
what it **accepts**, and it can do nothing unless the program hands it the
capability. Capabilities can also be **attenuated** before being passed:
a program holding write access to `cache/` can hand a library a handle that
writes only to `cache/thumbnails/`. VER23 decides when this becomes the
default, starting at the latest with new library packages.

Attenuation is sound only when all of these hold, and VER23 makes them
requirements:

- **Handles are descriptors, not path prefixes.** As a string prefix,
  `cache/thumbnails` would also match `cache/thumbnails_x/` and
  `cache/thumbnails/../`.
- **A handle cannot be used to recover its parent's authority**, whether
  through `__dict__`, closure cells, private fields or reflection. Python
  and Ruby have no enforced privacy, so this depends on the reflection
  ban of §8.6.2.
- **Handles cannot be serialised, recreated or forged.**
- **Libraries hold no ambient grants** and cannot import any generated
  module other than their own.
- **There is no shared mutable global state** in which a library could
  hide or intercept capabilities. That needs frozen built-in objects (SES)
  and no mutable statics (Joe-E); otherwise patching a prototype would
  intercept handles passed in later.

#### 8.6.5 How the layers fit together

| Layer | Catches | Cannot catch |
|---|---|---|
| Generated module + per-member allowlist (§8.6) | Any undeclared effect in code that stays inside the confined subset. It makes S nearly exact | Native code and FFI; memory-safety bugs in the C code behind allowed standard-library modules (CPython is not a security boundary); Go data races that corrupt memory and so forge interfaces (§8.2) |
| Capability analyzer (§8.1) | Enforces the allowlist and the subset; computes S | Whatever the subset rules miss |
| Locked box (§8.5) | What the program actually did, including native code | Paths never exercised; a package that detects the box |
| Weakness registry and proofs (§8.2, E5) | Whole classes of misuse, *within* the declared capabilities | Anything outside their stated assumptions |

**A package is only as confined as its closure.** Its confinement label is
the **weakest** label anywhere in its closure. An importer of a dependency
labelled "not language-confined" or "not confined in production" carries
that label too, whatever its own code does.

The layers also simplify the rest of the program. With effects confined to
generated functions, S ⊆ D becomes close to automatic. An O ⊆ S failure in
the locked box then has two likely causes. Often it is a gap in the
runtime baseline, because allowed "pure" calls trigger lazy runtime reads:
time-zone and locale data, codec modules, the C library's name-service
files, `.pyc` writes. Otherwise it is a subset escape. The box reports
which. For proofs (§7.5), every effect is a call into the generated module
with a stated contract. The prover treats effects as functions it does
not need to look inside, and those contracts join the assumption
catalogue (§7.5.2) and its per-language tests (§7.5.3).

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
      "fuzz": null, "proof": { "label": "proved", "assumptions": ["u64.wraps"], "certificate": "drat:<hash>" } }
  ],
  "rungs": { "E0": "pass", "E1": "pass", "E2": "pass", "E3": "pass", "E4": "pass", "E5": "pass" },
  "public_symbols_without_claims": [],
  "exemptions": []
}
```

`public_symbols_without_claims` must be empty to ship. Every exported API is
covered by at least one claim, which is what turns "has tests" into "states
and backs up what it does". The set of exported symbols comes from the
analyzer reading the shipped artifact's real exports (§4.4), never from the
Warrant file or the binding.

**Exemptions are computed, never declared.** A package cannot mark a rung
as `n/a` itself. The gate derives it from the Warrant file and the code.
For example, E4 cannot be `n/a` if the package has any `hostile` input,
and E2 cannot be `n/a` if the package declares any capability. Any rung that
falls short of its full requirement is an **exemption**, and so is a claim
labelled `not-yet-provable`. Without this rule, adding exactly the riskiest
constructs (`eval`, `unsafe`, FFI) would be a way to skip the proof rung.
So every exemption is treated like a new capability (§8.1, rule 4): it
blocks the release until a named approver approves it with a
physical-presence signature (§9.1, §9.5), and the approval is recorded with
the report.

**Label floors.** "Pass" is not left to interpretation. Each rung and
each weakness class has a **minimum proof label** (§7.6.5), held in the
gate's policy data (§9.2.1). Anything below the floor is a computed
exemption. For example:

- **E5** needs `proved` or `proved-to-<layer>` for every claim.
  `proved-over-model` means the implementation was never linked to the
  proof, so it is below the floor.
- **Denial-of-service classes** (CWE-400, 409, 674, 1333) need `proved`, or
  `bounded(n)` with `n` at least the input limit the code enforces with
  `rejects`. A bound checked only up to a size smaller than what the code
  accepts proves nothing about the inputs that matter.
- **Timing side channels** need `proved-to-<layer>` reaching the gate-level
  CPU (§7.4.1).

**Approvals are tied to what they approve.** Each approval of a capability
change or an exemption is bound to a hash of the package, the version, the
**content hash of the shipped closure**, and the exact capability delta or
exemption set. Re-tagging the same version on different code therefore
invalidates the approval. It is invalid for any other
release, and a merge-time CODEOWNERS approval of unrelated code does not
count.

**Approvers are named, and an approval is a human act.** The eligible
approvers are listed in the gate's protected policy data, and admin bypass
is disabled on every protected environment and branch the gate relies on.
What separates approver from author is a **signature made with a key that
requires physical presence**, not a GitHub account. Precisely:

- The key is non-exportable and lives in dedicated hardware. That is
  either the approval app on the maintainer's phone, in the Secure Enclave
  or StrongBox (§9.5.1, preferred), or a FIDO2 security key such as
  OpenSSH's `ed25519-sk` (§9.5). Software prompts like `ssh-add -c` do not
  count, because local automation can click them.
- Every approval signature proves **both a physical act and the owner's
  verification**. For a security key, that means the signed flags byte
  has user presence (UP, a touch) and user verification (UV, the PIN)
  both set, and a signature with either clear is rejected. For the phone
  app, it means a biometric or passcode check the key itself enforces.
  The gate checks what the signature proves, never how a key claims to
  have been created.
- What is signed is the canonical approval message carrying the approval
  hash defined above (§9.5).
- The root-signed policy data names the approved public keys (§9.4,
  rule 5).

An agent holding an account's API token cannot produce such a signature.

**Who may approve what.** While this repository has one human maintainer,
that human is the approver for every change an agent authored, which, in
practice, is most of them. "Someone other than the author" means *not an
agent*: the agent identities of §9.4 can never approve. A change the human
authored personally also needs that human's physical-presence signature at
release. Requiring a *different* person would block every release, so that
requirement applies only once the project has a second maintainer, who is
then listed as an eligible approver.

### 9.2 One gate, every release path

Every release path goes through one gate, and only the gate's trusted
publishing workflow can publish (§9.2.1). Today the release paths are
`publish.yml` plus about 30 `release-*` and `deploy-*` workflows, and each
of them becomes a request to the gate rather than a publisher in its own
right. The gate:

1. resolves the package's shipped closure from the **shipped artifact's
   real contents and imports**. The build tool's declared dependency graph
   is a cross-check, and any mismatch between the two fails the gate.
   Third-party runtime dependencies are part of the closure too. Under B06,
   shipped closures should have none. Any that remain are treated as
   holding **unbounded** capabilities, must be pinned by hash in a lockfile,
   and are exemptions needing approval;
2. re-runs, or fetches trusted cached results for, every live rung on that
   closure at the tagged commit (§9.2.1);
3. runs the capability audit and the capability diff;
4. fails closed, so a missing or unparseable report is a failure, never a
   pass. This follows `ci.yml`'s existing "verify gate verdicts were
   emitted" step.

#### 9.2.1 The gate does not trust the commit it judges

Failing closed catches a *missing* report, not a *forged* one. A gate whose
logic comes from the commit being released can be rewritten by that commit
to say "pass". The commit could edit the gate workflow, the build tool, an
analyzer or a BUILD file. So:

- **The judge comes from a protected ref.** The gate workflow, the build
  tool, the analyzers, the Warrant compiler, the solver and the proof kernel
  all run from the default branch at a pinned SHA, never from the release
  commit. The release commit supplies only the code being judged.
- **The release commit cannot choose its judge.** The pin is not written
  in the release commit: if it were, an old commit could name an old,
  weaker gate. Both the pin and a **minimum gate version** live in a
  protected configuration that only maintainers can change, outside any
  workflow file a release commit carries. The gate rejects any pin not
  reachable from current `main`, and any pin older than the minimum.
- **The judge's rules come from the protected ref too.** The gate reads
  all of its policy data from the protected ref, never from the release
  commit: the weakness registry, the assumption catalogue, coverage floors,
  label floors, rung liveness and lane-conformance status, and CODEOWNERS. A release of an old commit is judged
  against **today's** registry. Policy data is part of every cache key.
- **Only reviewed code can be released.** The tagged commit must be reachable
  from protected `main`, and release tags are protected, so only maintainers
  can create them.
- **Generated artifacts are regenerated, not trusted.** The gate regenerates
  tests from the Warrant files (§4.5) and recomputes the assurance report
  itself. Committed copies are compared against the regenerated ones and
  never used in their place.
- **Caches are written only by trusted CI.** A cached rung result is usable
  only if trusted CI on a protected ref wrote it; fork and pull-request runs
  can read the cache but never write to it. GitHub Actions' built-in cache
  cannot enforce this, because every job in a run, including untrusted
  ones, holds the token that writes it. So the rung-result cache is a
  **separate store**: the trusted job signs every entry, and the signature
  is verified on every read against the trusted job's full signer identity
  (the publisher rule below). The cache key is a content hash
  of everything the result depends on: the whole shipped closure, every
  tool and solver binary, the Warrant files, the bindings, the assumption
  catalogue and the gate's policy data. Change any one, and the result is
  recomputed.
- **Certificates are re-checked, not looked up.** The report records a
  certificate's hash for identification, but at release the trusted kernel
  re-checks every certificate itself. A matching hash proves nothing about
  whether the proof is valid.
- **The judged bytes are the published bytes.** The gate itself builds the
  artifact in the shipping configuration and records its digest in the
  report. The publish step publishes **only that digest** and never
  rebuilds, because a rebuild could differ: other features or `cfg` flags
  (bringing the simulated host back), or different dependency resolution.
- **The gate signs a verdict, and only on a pass.** Standard build
  provenance records *how* an artifact was built, not *that it passed*.
  So the trusted job signs a dedicated **verdict attestation**, and only
  when every live rung passes. Its signed predicate binds:
  - the passing verdict and the full assurance report;
  - the artifact digest;
  - the repository and the protected release-tag ref, taken from the OIDC
    `repository` and `ref` claims;
  - the commit, which must be reachable from `main`;
  - the package name, version and registry target;
  - a hash of the gate's policy data at judging time, and an expiry.

  A failing or unreviewed build never yields a signature. The publisher
  rejects a verdict that has expired, or whose policy-data hash is no
  longer current. A passing verdict for a release that was cancelled
  therefore cannot be replayed weeks later, after the weakness registry has
  grown.
- **Only the current gate can publish.** A workflow that calls the gate is
  read from the release commit, so an old commit could call an old gate,
  and a workflow *name* says nothing about *which version* ran. So
  publishing is not done by anything the release commit triggers directly.
  It is a **separate top-level workflow that runs on `main`**, started by
  the gate's completion (for example `workflow_run` or `repository_dispatch`).
  The trigger's payload can be influenced by an attacker, so the publisher
  trusts nothing in it. It rejects triggers from any other repository or
  from forks. Before doing anything else, it verifies that:
  - the verdict attestation was signed from **exactly the gate's
    trusted-job workflow file**. GitHub's OIDC identity names a workflow
    file, not a job, which is why that file contains only the trusted job
    (§9.4, rule 6). A commit SHA alone identifies nothing: every workflow
    file at a commit shares the commit's SHA, and so does a branch anyone
    with write access pushes at that commit. So the publisher checks the
    full signer identity from the signing certificate, never from the
    predicate:
    - the OIDC issuer;
    - `repository_id` and `repository_owner_id`;
    - `job_workflow_ref`, which must be the exact path of the trusted-job
      workflow file;
    - `job_workflow_sha`, which must equal the pin in the root-signed
      policy data (§9.4, rule 5) and be at or above the minimum gate
      version;
    - `environment`, which must be the trusted job's dedicated environment;
    - the source ref, which must be a protected release tag;
    - the runner, which must be GitHub-hosted.

    Cache-entry signatures get the same check, with one difference: the
    source ref may be a protected release tag **or** `main`, because
    trusted runs on `main`, such as nightly runs, also write the cache.
    The harness's identity (§9.4, rule 6) is rejected in both places;
  - the signed predicate is a **passing** verdict, for this repository, a
    protected release-tag ref, and a commit reachable from `main`;
  - the package name, version, registry target and artifact digest in the
    predicate are exactly what is about to be published.

  **Registry credentials are confined to a protected `publish`
  environment.** Registry trusted publishing (npm, PyPI, crates.io,
  RubyGems) binds a repository and a workflow *filename*, not a branch. So
  without more, anyone with write access could push a branch carrying a
  modified workflow of the same name and get a registry token for arbitrary
  bytes. The `publish` environment closes that:
  - its deployment-branch policy allows only `main`;
  - admin bypass is disabled;
  - only the publisher workflow names it;
  - every registry's trusted-publisher configuration requires that
    environment name, or pins the ref where the registry supports that.

  With OIDC trusted publishing, **no long-lived publish secret exists**.
  Some targets have no OIDC trusted publishing (Hex, some app stores and
  extension marketplaces). For those, the secret lives only in the
  `publish` environment, behind required reviewers, and each such target is
  a named, approved exemption in the policy data.
- **The publisher handles the artifact as opaque bytes.** It holds a
  registry token, so it must never execute anything from the artifact:
  - no package-manager lifecycle scripts;
  - no commands that repackage or rebuild, such as `cargo publish`, which
    runs `build.rs`, or `mix hex.publish`. It uploads the verified bytes
    through the registry's API;
  - artifacts are extracted safely, outside the workspace, and the digest
    is verified before they are used;
  - every field of the trigger payload is treated as untrusted input.
- **The judged code never touches the judge's credentials.** §9.2.1 treats
  the release commit as hostile, and running it next to signing rights
  would undo that. The release commit's code includes build scripts
  (`build.rs`, `setup.py`, npm lifecycle hooks), tests and third-party
  install hooks. Following the SLSA Build Level 3 model:
  - **Building** the artifact happens in an isolated, unprivileged job. It
    has no OIDC token, no signing key, no publish credential and no
    cache-write right. Its only output is the artifact.
  - **Static judgments never run where judged code has run.** A build hook
    runs as the same OS user as anything else in its job. It could replace
    the analyzer binary or rewrite a results file, and hiding an undeclared
    `net` capability is exactly what a malicious package would want. So
    every static judgment runs **inside the trusted job itself**. It does
    not run in a separate job of the same run, because results handed
    between jobs of one run can be swapped (see the trusted-job rule
    below). And it judges **the artifact's own contents**, the bytes that
    ship, in the artifact's own languages, not the repository checkout.
    Most shipped bytes are build output: compiled `dist/` files, bundled
    extensions, wheels with native code. A build hook could inject a call
    into those that no source analysis would ever see. Any file in the
    artifact that no analyzer covers fails the gate. For compiled or
    bundled output, one of these must also hold:
    - a second, isolated build reproduces the artifact, and the two digests
      are identical; or
    - the build runs only build logic from the protected ref, and judged
      build hooks (`build.rs`, npm scripts, `setup.py`) are banned, or are
      themselves caged and judged.

    The static judgments are:
    - closure and export resolution;
    - capability and subset analysis;
    - weakness exclusions;
    - regenerating tests and proof obligations from Warrant;
    - certificate checking.
  - **Proof obligations come from the trusted side.** The trusted job
    generates every proof obligation itself and checks each certificate
    against *its own* obligation. A valid certificate for a different,
    easier formula would otherwise pass the kernel's check.
  - **Dynamic rungs run the judged code behind an OS or VM boundary** from
    the harness, so the code under test can neither patch the reporter nor
    forge its output. The D21 cage is a language-level wrapper, not an
    operating-system sandbox, so it cannot provide this boundary. The test
    job receives only the built artifact, and no build hooks run alongside
    the harness.
  - A separate trusted job, running only protected-ref code, re-derives the
    verdict, signs it only on a pass, writes the cache and hands off to
    publishing. Cache writes and signatures happen only there.
  - **Nothing passed between jobs is trusted on arrival.** Every job in a
    run holds a token that can replace that run's artifacts. A malicious
    build hook can steal it, send it off the runner, and swap results or
    bytes after the fact. So:
    - The trusted job **computes the artifact digest itself**, from the
      bytes it judges, and never takes a digest from the build job.
    - Every other job reports the digest of the artifact it consumed, and
      the trusted job rejects any mismatch. Static analysis and tests
      therefore cannot have seen different bytes.
    - Dynamic results are accepted only if the harness signed them with its
      own workflow-file identity (§9.4, rule 6), from the host side outside
      the VM boundary, where the code under test cannot reach the key.
    - Building runs in a **separate workflow run** from judging, so the
      build never holds the judging run's token at all. The build job has
      no network egress beyond fetching what the lockfile pins.
  - **Runners are ephemeral.** Trusted jobs never share a self-hosted or
    persistent runner with untrusted jobs, or with jobs defined by the
    caller in the same run.
- **The release's own report is stored immutably and signed**, because it
  becomes the next release's capability baseline (§8.1). Plain release
  assets are not enough, since anyone with write access can replace them.
  The store must be append-only (immutable releases or a transparency log),
  and the report is attested with the protected workflow's identity, which
  is verified whenever the report is used as a baseline. A package that
  has shipped before but whose baseline is missing or fails verification
  **fails the gate**. It is never treated as a first release.

### 9.3 Coverage, centrally

The per-package coverage thresholds now scattered across `pyproject.toml`,
`vitest.config.ts`, `test_helper.rb`, `mix.exs` and BUILD flags are replaced
by one floor the gate enforces: 95% for libraries and 80% for programs,
per CLAUDE.md. Languages that report no coverage today (Go, Rust, Swift,
Haskell, Lua, Perl, C/C++) get a reporter as part of TF00's lanes. BUILD
files that skip their tests when a tool is missing, as the Haskell ones do
today, fail instead.

Coverage counters are collected inside the process under test, so code
under test could inflate them. Coverage is therefore a **quality floor,
not a security property**, and the report says so. Where the platform
allows, the harness collects coverage from outside the process, through
the instrumentation hooks of §7.3, rather than trusting in-process
counters. No security conclusion rests on a coverage number.

### 9.4 The trust root, and the agents that work on it

The gate is only as strong as whoever can change it. Most changes to this
repository are made by coding agents, and an agent can be misled by
prompt injection from a web page, an issue, a dependency's README or a
test fixture. A misled agent does not follow policy. It uses whatever it
can reach. If it can reach the maintainer's GitHub token, it holds every
power the maintainer holds, and it could:

- add its own software key to the approver list;
- lower a label floor or a coverage floor;
- edit the gate, the publisher workflow or one of their in-repo
  dependencies, keeping the publisher's filename so registry trusted
  publishing still accepts it;
- turn admin bypass back on, or relax a ruleset.

It could then release, and the physical-presence requirement would be
defeated without ever being met. The rules below exist so that **one thing
stays outside every agent's reach: the ability to change the gate.** None of
them needs a paid or enterprise plan.

1. **Agents have their own GitHub identity.** Agents act through a
   separate identity, never the maintainer's token. Both options are free
   for a personal account:
   - a **GitHub App** that the maintainer creates and installs on their own
     repositories (recommended);
   - a **machine account**, added as a collaborator.

   The identity has *Contents* and *Pull requests* write, and nothing
   else: no Administration permission, no release-tag creation, and no
   ability to change rulesets, environments or settings.
2. **Agents cannot reach the maintainer's credentials.** A separate
   identity means nothing if the agent can read the maintainer's token
   anyway. Agents run where they have **no access** to:
   - the maintainer's credential stores: `gh` auth, the git credential
     helper and the keychain;
   - the `ssh-agent` socket;
   - plugged-in USB security keys.

   That means a separate OS user, a VM or container, or remote execution.
   An agent working in the maintainer's own login session is an
   **unconfined** agent, and the guarantees of this section do not hold
   while one runs.
3. **The maintainer's everyday token cannot administer.** The
   maintainer's own day-to-day token has no Administration scope.
   Repository settings, rulesets and environments are changed only in the
   web UI, after GitHub re-authenticates the maintainer with a passkey or
   security key ("sudo mode").
4. **The trust root needs a human, and it is computed.** The trust root is
   the gate workflow, the publisher workflow, the policy data and
   CODEOWNERS, plus **the entire in-repo transitive closure** of every tool
   they run. Listing the tools by name is not enough: the analyzers'
   parsers, the Ed25519 and SHA-2 packages under the signature verifier,
   `constraint-engine` and `semantic-ir` are all part of it, because
   weakening any of them weakens the gate. That closure is computed the
   same way as a shipped closure (§9.2, step 1), so:
   - CODEOWNERS and ruleset coverage for the trust root are **generated
     from the computed closure**, and the gate checks the coverage is
     complete;
   - every change inside the closure needs a review the agent identity
     cannot give: a ruleset requiring CODEOWNERS review by the human
     account, with no bypass. Auto-merge continues for everything outside
     the closure and stops at this boundary;
   - a gate-pin bump shows the human the diff of the **whole closure**
     since the previous pin, not a one-line change. A pin whose closure
     contains a commit that no human reviewed is refused.
5. **Policy is signed, and the publisher anchors the signature.** The
   policy data (approver keys, gate pin, minimum gate version, floors,
   liveness, registry) is signed with physical-presence **root keys**, held
   by the maintainer, that are separate from the daily approval key.
   - **The publisher checks it first.** The publisher, not only the gate,
     verifies the root signature over the policy data, including the pin,
     *before* using the pin. A gate chosen by a forged pin is never
     consulted.
   - **The genesis anchor is outside the policy.** The fingerprint of the
     first root is embedded in the publisher. Later roots form a chain in
     the style of TUF (The Update Framework). Each root is signed by the
     previous one and carries a strictly increasing version number, and
     rollback to an older root or policy is rejected.
   - **A threshold, with an offline backup.** Root actions need 2 of n
     keys: the maintainer's root key plus an offline hardware backup kept
     separately. Losing one key does not lock out every future policy
     change, so no ad hoc recovery path quietly becomes the real trust
     root.
   - **A genesis ceremony.** VER21 specifies how the first root is created:
     keys generated on the devices, fingerprints recorded on paper and in
     the first signed policy, done once by the maintainer by hand.
   - **What this protects.** A change that touches only the policy files
     cannot swap the approver list or the pin, because it lacks the root
     signature. A change to the publisher itself is caught by rule 4, not
     rule 5.
6. **`id-token: write` is tightly scoped, and the identity check carries
   it.** GitHub's OIDC identity names a *workflow file*, not a job, so
   every job in a file that can request a token gets that file's identity.
   - The gate's trusted job is the **only** job with `id-token: write` in
     its own dedicated workflow file. It is bound to a dedicated
     environment, and the signer-identity check (§9.2.1) also requires that
     environment's OIDC `environment` claim.
   - The harness that signs dynamic results has a **different** workflow
     file, and therefore a different identity. Verdict and cache verifiers
     reject that identity, so a harness compromise cannot forge a verdict.
   - The publisher has its own file and environment.

   The gate's static checks enforce these placements over
   `.github/workflows/`. A workflow on an agent-pushed branch can still
   *request* a token. What makes that harmless is the signer-identity
   check (exact file, pinned SHA, protected ref, environment), which no
   such token can satisfy.

### 9.5 Release approval with a hardware key — VER21

GitHub has no "approve this release with your security key" button, and
none is needed, because git can sign tags with a hardware-backed key.
**The release tag is the approval.**

1. **The gate does a dry run first.** The gate's trusted job judges the
   release candidate and signs a **release summary**:
   - the package, version and closure content hash;
   - new capabilities, with their justifications;
   - exemptions, with their reasons;
   - the weakness matrix and the proof labels;
   - the approval hash of §9.1.

   This summary is produced in CI under the gate's identity, not on the
   maintainer's machine.
2. **`release-approve` shows the signed summary.** The repository's
   `release-approve` tool fetches the summary and verifies the gate's
   signature on it. It then displays the summary along with a short
   fingerprint of the approval hash. The maintainer checks that fingerprint
   on a **second device**, for example the pull request or release page on
   a phone, before signing. A security key has no display. Without this
   step, a compromised tool on the signing machine could show one summary
   and have a different hash signed.
3. **Signing.** The tool creates a tag whose message is in a strict
   canonical form: exactly one approval-hash line, then the
   human-readable summary. The tag is signed through git's SSH signing
   with a **dedicated approval key**, a FIDO2 `ed25519-sk` key used for
   nothing else: not SSH login, and not commit signing. A key used all day
   for other things trains its owner to touch on reflex, which is what
   makes hijacking a touch easy. The key asks for a touch and its PIN.
   **The touch is the consent**: touch only for a signing you started, at
   the moment you started it.
4. **The gate verifies the tag itself.** It does not rely on GitHub's
   "Verified" badge. Its checks:
   - the tag is an annotated, signed tag;
   - the tag object's `object` and `tag` fields match the release commit
     and the release tag name;
   - the message is in the canonical form;
   - the SSHSIG magic, version and namespace (`git`) are correct, and the
     hash algorithm is on an allowlist (`sha512` or `sha256`);
   - the signature type matches the key type, `sk-ssh-ed25519@openssh.com`.
     `webauthn-sk-*` types are rejected. Key types the gate cannot yet
     verify, such as `sk-ecdsa-sha2-nistp256@openssh.com` before P-256
     lands, **fail closed**;
   - the key is a named approver key in the root-signed policy data;
   - the signed flags byte has **user presence (UP, 0x01) and user
     verification (UV, 0x04) both set**. Key-creation options such as
     `verify-required` or `no-touch-required` are not recorded in the key
     or the signature, so the gate checks the flags, not how the key was
     made. A signature with either flag clear is rejected;
   - the approval hash in the message equals the one the gate computes
     itself, from its own inputs, for the bytes about to ship.
5. **GitHub's own controls are a backstop, not the authority.** Tag rulesets
   restrict who may create release tags. The agent identity cannot create
   them, and unsigned or lightweight tags are rejected anyway. Uploading
   the key as a signing key gives the "Verified" badge.

One touch per release covers every capability change and exemption in
it. Nothing extra is needed per pull request.

#### 9.5.1 Approving from a phone (preferred)

A USB security key has one weakness for this job: **it has no screen**. It
signs whatever the computer it is plugged into sends it. So if that
computer is compromised, the key can be made to sign something other
than what the maintainer believes. A phone has a screen, and it is a
separate device that agents never run on. So the preferred approver is a
small **approval app** on the maintainer's phone, built in this repository
with its native UI stack.

1. **The key lives in the phone's secure hardware.** That is the Secure
   Enclave on iOS or StrongBox on Android. It is created on the device and
   **non-exportable**, and every use requires Face ID, a fingerprint or the
   device passcode, enforced by the hardware itself. It is device-bound and
   **never a synced passkey**: a passkey synced through iCloud Keychain or
   Google Password Manager is copied into a cloud account and onto other
   devices, possibly including the very computer this design distrusts.
   Where the platform offers **key attestation** (Android key attestation
   for StrongBox), the policy records it at enrollment. That proves the key
   is hardware-bound and gated on the user.
2. **The phone fetches the gate-signed summary itself.** It does not get
   the summary from the maintainer's computer. The phone holds the gate's
   public identity, verifies the signature on the summary (§9.5, step 1),
   recomputes the approval hash, and **displays the full summary on its own
   screen**. What the maintainer reads is what gets signed, with no
   computer in between.
3. **Approve with a biometric.** The phone signs the canonical approval
   message with its hardware key. These keys are P-256, so this needs the
   P-256 verifier from VER21.
4. **The signature travels back over an untrusted channel.** The phone
   posts it somewhere the gate reads, such as the release's tag message or
   an approval file. The channel needs no protection, because the gate
   verifies the signature against the named approver key and recomputes
   the approval hash. A forged or altered approval simply fails.

**Why this is stronger.** With the phone as the approver, compromising the
maintainer's computer gains an attacker no signing power and no way to
misrepresent what is being signed. To subvert a release, the attacker would
have to compromise **the phone** as well. Hardware-bound keys also cannot be
copied off either device. That is the property this design is after: one
compromised device does not compromise the key.

**Roles of the two kinds of key.**

| Key | Role |
|---|---|
| Phone approval app (Secure Enclave or StrongBox) | Day-to-day release approvals |
| USB FIDO2 security key | A second root key, held offline, for the 2-of-n root threshold (§9.4, rule 5), and a fallback approver if the phone is lost |

Losing the phone does not lock the project out: the offline USB root key,
together with any other remaining root key, enrolls a replacement
phone key through a root-signed policy update.

**What remains.** With the phone approver, the phone is part of the trust
base. With a USB key, the signing computer is. In that case, the dry-run
summary, the second-device fingerprint check, the dedicated key and rule 2
of §9.4 make subverting a release signature require *both* a compromised
computer *and* a maintainer who skips the fingerprint check. Neither setup
makes subversion impossible, and this spec does not claim either does.

**Building the hardware-key stack in-house.** In keeping with B06, VER21
builds the stack this needs in the repository, sharing it with the Vault
work in VLT-PM51:

| Piece | Status |
|---|---|
| Ed25519, SHA-256/512, HMAC, HKDF, canonical CBOR, base64, DER/ASN.1 | Exist, across many languages (and become trust root under §9.4 rule 4) |
| SSHSIG verification for `-sk` keys. The key signs `SHA256(application) ‖ flags ‖ counter (big-endian u32) ‖ SHA256(SSHSIG signed-data blob)`; the outer hash is always SHA-256. Inside the blob, the message hash is the SSHSIG `hash_algorithm`, `sha512` (git's default) or `sha256`. | New. Pure code on top of Ed25519, SHA-256 and SHA-512, and the first thing the gate needs |
| P-256 ECDSA | New. Needed for `ecdsa-sk` keys (older security keys support only those) and most WebAuthn and passkey credentials. It also unblocks VLT-PM51's `WebAuthnPrfAuthenticator`, which currently always refuses |
| USB HID transport: IOKit on macOS, hidraw on Linux, HID APIs on Windows | New. Replaces the `hidapi` dependency of `vault-webauthn-ctap2-hid` |
| CTAPHID framing and CTAP2 commands (`MakeCredential`, `GetAssertion`) | New. Replaces `ctap-hid-fido2` |
| `release-approve` CLI | New. Verifies and shows the gate-signed summary, then requests the signature |
| Phone approval app (iOS Secure Enclave, Android StrongBox), built on the repo's native UI stack | New, and the preferred approver (§9.5.1). Needs P-256 |
| PIV and OpenPGP smart cards (CCID/APDU); Secure Enclave; TPM | Later |

Until VER21 lands, the maintainer can sign with OpenSSH's own FIDO
support. The stock OpenSSH that ships with macOS may lack it, in which case
a separately installed OpenSSH is needed. The gate's verifier is the
first in-house piece, because it is the one the gate needs.

## 10. Phasing

Phases follow the "switch rungs on one at a time" decision. Each phase
ends with its rung **live and blocking**.

| Phase | Delivers | Rung live | Child specs |
|---|---|---|---|
| 0 | Gate skeleton, assurance report, central coverage, fail-loud BUILDs, TF01 (TypeScript off Vitest and Jest) | E1 + coverage | VER01, TF01–TF02 |
| 0b | Trust-root rules (agent identity, human review of trust-root paths, signed policy data); SSHSIG `-sk` verifier; signed-tag release approval | Approvals | VER01, VER21 |
| — | P-256, in-house USB HID and CTAP2, `release-approve` CLI | — | VER21 |
| 1 | Capability analyzers for all languages, capability diff, weakness registry v1 (exclusions) | E0 | VER08, VER15 |
| 1b | Generated capability modules and closed import allowlists: Go and Rust first, extending the existing generators, then every lane; native shim and binary checks | E0 language-level | VER23 |
| 2 | Test tiers, purity and speed enforcement, simulated host, taxonomy extension, chaos injection | E1 purity, E2 | VER02, VER03, VER17 |
| 2b | Checked allocator, checked C interpreter, native attack corpora | E0 for native code | VER16 |
| 2c | Locked-box capability check: Linux appliers and the base deny, race-free enforcement, recorded denials, closed system-call mapping, broker log, exact-target baseline, D/S/O cross-check. Live with baseline charging only; attribution is research. Gets most of its reach once phases 3–4 drive the harness. | E0 dynamic (Linux guests) | VER22 |
| 3 | Warrant v0 (examples, claims, models) with TypeScript and Rust backends; property engine | E3 | VER04, VER05 |
| 4 | Fuzz engine; `input` blocks | E4 | VER06 |
| 5 | Microbench harness in every lane; `budget` blocks; timing calibration | (budgets only) | VER07 |
| S1 | CDCL, DRAT checker, Tseitin/AIG encodings, bit-blaster, incremental solving | — | VER09–VER11 |
| S2 | Hardware equivalence checking, synthesis translation validation, CPU k-induction | E5 for hardware | VER18 |
| S3 | Vacuity, consistency and model proofs; solver-generated inputs (needs Warrant v0) | `proved-over-model` (below the E5 floor; spec-level only) | VER04, VER05 |
| S4 | SIR symbolic execution, bounded and inductive proofs, back-end translation validation | E5 `proved` / `bounded(k)` under assumptions | VER19 |
| S5 | SIR cost semantics, ranking functions, `bound` proofs, constant-time self-composition; gate-level WCET | E5 bounds | VER20 |
| 6 | SMT theories, VC generation | E5 widening | VER10, VER12 |
| 7 | Assumption catalogue and conformance tests per lane; subset enforcement; SIR front ends for remaining languages | E5 everywhere, under assumptions | VER13, VER14 |
| 8 | Discharging catalogue entries with proofs of our own VMs, compilers and CPUs | `proved-to-<layer>` | per layer |
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
| VER07 | Microbenchmark harness, deterministic cost counters, timing calibration |
| VER08 | Capability analyzers and release-time capability diff |
| VER09 | CDCL SAT (constraint-engine upgrade) |
| VER10 | SMT theories: EUF, LIA, bit-vectors, arrays |
| VER11 | Proof certificates and the trusted kernel |
| VER12 | Verification-condition generation from Warrant |
| VER13 | Platform assumption catalogue, per-language conformance tests, subset enforcement |
| VER14 | SIR front ends for the remaining languages, with differential front-end checking |
| VER15 | Weakness registry (CWE inventory) and per-package weakness matrix |
| VER16 | Native attack harness: checked allocator, checked C interpreter, attack corpora |
| VER17 | Chaos injection on the real host |
| VER18 | Hardware equivalence checking, synthesis translation validation, k-induction |
| VER19 | SIR symbolic execution and bit-precise proofs |
| VER20 | Resource-bound and timing proofs: cost semantics, ranking functions, constant time, gate-level WCET |
| VER21 | Hardware-key release approval: phone approval app, SSHSIG `-sk` verification, P-256, in-house USB HID and CTAP2, `release-approve`, root-key ceremony (shared with VLT-PM51) |
| VER22 | Locked-box capability check: `capability-os-sandbox` enforce and observe modes, seccomp user-notification supervisor, VM isolation, attribution, D/S/O cross-check |
| VER23 | Generated capability modules: exact per-package effect modules, target types, closed import allowlists, native shim and binary checks, object-capability passing |

## 12. Open decisions

1. **Resolved — the meaning of E5** (§7.5): assume–guarantee. We prove our
   implementation logic under named, tested platform and environment
   assumptions. We do not attempt to control the language or platform, and
   on our own stack, lower layers' proofs discharge those assumptions.
2. **Warrant syntax family**: an ML-style syntax like the §4.3 sketch, or one
   closer to an existing in-repo language (Twig, Nib) so the tooling can be
   reused.
3. **Speed budget numbers** (§5): 10 ms per unit test and 5 s per package
   suite are placeholders until they are measured against today's suites.
4. **Which packages ship first.** The first package through the full gate
   proves the gate. A small, pure, already well-tested package (`bitset`,
   `logic-gates`) is the candidate.

### 12.1 Open security findings: publishing and approval flow (deferred)

The publishing and approval flow (§9.2.1, §9.4, §9.5) went through seven
rounds of security review while this spec was drafted. The owner then
deferred further work on it. The seventh round's findings are recorded
here so that the child specs (VER01, VER21) address them. **None of §9.4
or §9.5 may be implemented until they are resolved.**

**MEDIUM**

1. **The approval clients are outside the trust root.** The phone app,
   `release-approve`, the HID, CTAP2 and P-256 stack, and their in-repo
   closure all belong in the human-reviewed trust root, along with the
   phone app's build and install path. The phone must run only an app
   built from a human-reviewed commit and installed by the maintainer,
   with updates approved like a pin bump. Otherwise an agent-modified app
   can show an honest-looking summary while signing the real approval
   hash.
2. **The maintainer's computer is in the trust base.** Its GitHub session
   can review and merge a weakened publisher, which then publishes from
   `main` without any physical presence. Two possible fixes:
   - trust-root review comes from a **separate GitHub account** whose
     device-bound passkey never exists on the daily computer;
   - the publisher, policy and genesis anchor move to a separate
     repository only that account can write.

   Until one of these is in place, §9.5.1's claim that a compromised
   computer gains no signing power is **not true**, and the computer's
   GitHub session is part of the trust base.
3. **A reproducible rebuild does not stop a malicious build hook.** A
   deterministic hook injects the same bytes into both builds, so the
   digests still match. When the judged commit carries build logic, the
   protected-ref build option, with judged hooks banned or caged, must be
   mandatory. Alternatively, proof obligations must come from the shipped
   artifact, or each build must be translation-validated against the
   proved source.

**LOW**

- **iOS signatures carry no flags.** Phone (Secure Enclave or StrongBox)
  signatures prove nothing about biometrics, so user verification must be
  established at enrollment:
  - Android: the key-attestation chain, with `userAuthenticationRequired`
    and per-operation authentication;
  - iOS: the recorded access-control flags.

  §9.1 must be reworded to match.
- **Summary verifiers need the full identity check.** The phone and
  `release-approve` must apply the full §9.2.1 signer-identity check, with
  the pin taken from the root-signed chain. The dry run needs a defined
  allowed ref. The phone must sign the hash it recomputes itself. The
  summary, verdict and cache entry need distinct predicate types.
- **The harness signature check needs specifying.** It must name the
  exact file, the pinned SHA, the allowed refs, the environment and the
  hosted runner.
- **The root threshold is wrong.** It must be 2-of-3, not 2-of-n with
  n=2. The offline root credential must never double as an approver
  plugged into the daily computer.
- **Phone mode needs its own tag rules.** It must define who creates the
  tag, which tag checks apply, and the canonical, domain-separated message
  the phone signs.
- **An unconfined agent run has lasting effects.** Once one has run, the
  guarantees do not hold until credentials are rotated and the machine is
  re-verified.
- **The USB path's second-device check must use the phone app.** A GitHub
  page that the computer's session can edit is not a second device.

**INFO**

- The trusted job parses hostile artifacts while able to mint OIDC
  tokens. So the analyzers must themselves meet E4 and bounds, and must
  run without access to the token-request variables.
- Rollback protection needs a mechanism: stored state, TUF-style
  expiry, or reliance on protected-`main` history.

## 13. What this does not buy

A proof is only as good as the claim it proves. A package can be fully
verified against a Warrant file that says the wrong thing. The defences
are:

- humans review Warrant files the way they review specs today, because
  they *are* the spec;
- every public symbol must be covered by a claim;
- mutation testing, a later child spec, checks that the claims are strong
  enough to notice when the code is broken.

A guarantee is also only as true as its assumptions. On a third-party
runtime, a platform bug that the conformance tests do not exercise can
falsify a proved claim. That is the accepted boundary: we guarantee our
logic, not the platform's. The assumption list printed with every
guarantee is what makes that boundary visible to users instead of hidden.

Moving all of this in-house also moves the bugs in-house. A bug in a test
framework, property engine or prover is worse than most, because it fails
by passing. Every tool in this program therefore ships with a conformance
corpus that includes **negative cases**: inputs that must fail. A trusted
kernel small enough to read in full is the last line of defence.
