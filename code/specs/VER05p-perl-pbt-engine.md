# VER05p — Perl Property-Based Testing Engine

**Parent:** [VER00 §7.2, §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md), [VER05](VER05-property-engine.md), [VER05e](VER05e-java-pbt-engine.md), [VER05g](VER05g-csharp-pbt-engine.md), [VER05h](VER05h-ruby-pbt-engine.md), [VER05i](VER05i-c-pbt-engine.md). Plan: polyglot-testing-tooling, item PT-PERL-PBT.

This specification defines the Perl 5 implementation of the Level 2 Property-Based
Testing (PBT) engine. Perl had no test-bootstrap and no property engine before this
change. The engine conforms to the `pbt-v1` corpus
(`code/specs/fixtures/pbt-v1/cases.json`, 25 cases) with exact outputs, minimal
witnesses **and shrink step counts**, and implements the verdict-honesty rules of
VER05 §7.1 and the hierarchical shrink-budget tree of
[VER05e §4.1](VER05e-java-pbt-engine.md), which is part of the first version.

(Letters `j`, `k` and `l` belong to the C++, Swift and F# lanes, `m` to Elixir, `n` to
Haskell and `o` to Lua, all open on their own branches; `p` is the next free letter.)

The algorithms are those of the Ruby and C lanes and the hardening list is that of the
Lua, Elixir and Haskell lanes (VER05o, VER05m, VER05n), whose Canon escape set is
copied exactly. This document does not repeat the algorithms. It states what is the
same (§1), what Perl's number model, error model, overloading, ties and dynamic scoping
change (§3 to §5), and the tests that pin each hazard (§10).

---

## 1. Same as Java, C#, Ruby, C, Elixir, Haskell and Lua (normative by reference)

- VER05e §2: SplitMix64; `bounded(n)` with the rejection threshold
  `t = (2^64 - n) mod n`; `bounded_big`; FNV-1a 64; **`bounded(1)` consumes one
  word**; `sub_seed(run_seed, id) = SplitMix64(run_seed xor fnv1a64(id)).next_u64()`.
- VER05e §3: the generator algebra and its **draw order**, the cross-lane contract
  that `pbt-v1` pins.
- VER05e §4: integrated shrinking in the order target probe, integer walk (bisection
  `current - floor(diff/2)` when `|diff| > 1`, else the unit step `current - sign(diff)`),
  and the sequence shrinker (empty or prefix probe, bisection left then right, single
  deletion right to left, element-wise walk left to right without a target probe).
  `steps` counts accepted moves only; `steps < max_steps` guards every loop head.
- VER05e §4.1: the budget tree (restated for Perl in §5).
- VER05 §7.1 and VER05e §5: the runner, the closed failure vocabulary, verdict honesty,
  witness re-verification, CI redaction. An unset CI mode is CI.
- VER05e §6: conformance runner, strict decoding, CWE-400 caps, negative controls.

## 2. Package and modules

`code/packages/perl/test-bootstrap` (the build tool names it `perl/test-bootstrap`),
distribution `coding-adventures-test-bootstrap`, root namespace
`CodingAdventures::TestBootstrap`, Perl `>= 5.26` (the lane's floor), tests under
`prove` with `Test::More`. **Zero third-party dependencies**: the cpanfile requires
nothing outside the core (`Test::More` for the tests); the runtime uses only
`Scalar::Util`, `B`, `Math::BigInt`, `Encode` and `Config`.

| Module | Role |
|---|---|
| `TestBootstrap` | facade: loads every module, `$VERSION` |
| `Int` | arbitrary-size integers over Perl's 64-bit IVs and `Math::BigInt` (§3) |
| `Prng` | `Rng`, `next_word`, `next_u64`, `bounded`, `bounded_big`, `fnv1a64`, `sub_seed`, unsigned `threshold` |
| `Limits`, `FailureIds`, `Hooks` | caps, the four ids, sabotage seams |
| `Safe` | the only place user code runs and user values are inspected (§4) |
| `Budget` | the budget tree and its ambient carrier (§5) |
| `Gen`, `Gens`, `Shrink` | the generator algebra and the shrinkers |
| `Runner` | `Options`, `check`, verdicts |
| `Canon`, `StrictJson`, `Values`, `Tfv`, `Corpus` | rendering, strict decoding, TFV validator, the pbt-v1 evaluator |

The library does no file, clock, network or environment access (the CI flag is a
parameter: `Options->ci_from_value($ENV{CI})` is the caller's call). Tests read the
committed fixture by walking up to the repository root. `required_capabilities.json`
is capability-free.

## 3. Perl's number model (hazards 1 and 6)

Perl scalars are dual: an IV, a UV, an NV (double) or a string, converted silently.
`+`, `-`, `*` and `<<` on integers **fall over to NV** when the result leaves 64 bits,
which silently destroys SplitMix64 and FNV-1a; `/` is a float division; `use integer`
wraps with C semantics that the language does not define. The engine therefore never
relies on any of those for words.

- **Startup check.** `Config{ivsize}` and `Config{uvsize}` must be 8; the load fails
  with a clear message otherwise.
- **Words.** The PRNG works on *words*: a non-negative UV used as an unsigned 64-bit
  value. Bitwise `& | ^ >> <<` on non-negative integers are defined by the language and
  exact on UVs; addition and multiplication are built from **32-bit halves** (`add64`,
  `mul64`: every intermediate is below 2^53, so no scalar becomes an NV) and masked back
  to 64 bits; FNV-1a uses a specialised multiply (the prime is `2^40 + 435`) that a test
  checks against the general `mul64`. A test cross-checks both against `use integer` arithmetic and
  `Math::BigInt` on edge and random operands. Unsigned comparison is Perl's own `<` on
  UVs (exact), and `%` on UVs is exact; the rejection threshold is
  `t = ((~n % n) + 1) % n`, which is `(2^64 - n) mod n` without forming `2^64`. A signed
  `(-n) % n` is wrong for every `n` and never used. A word is never printed with `%d`,
  divided with `/` or compared as a float.
- **`Int`: the mathematical integer.** Everything public (seeds, shrink targets, `u64`
  and `i128` values, corpus integers) is an `Int`: a native integer scalar when the
  value fits in `[-2^63, 2^63-1]`, otherwise a `Math::BigInt`. The representation is
  **normalized**, so a `Math::BigInt` is always outside the native range. `Int::check`
  accepts only a scalar Perl holds as an integer (`B` flags: `IOK`, and if `NOK` is also
  set the value must be integral): a float literal (`3.0`, `1e3`), a string (`"5"`),
  `undef` and a reference are a `TypeError`, so a float cannot reach the PRNG path. A UV
  above `2^63-1` is converted to a big. Fixture integers are capped at **1,000 digits
  before parsing**, generator spans at 16,384 bits, shifts at 2^20 bits. `bit_length`
  of a big is exact up to 5,000 decimal digits and within one bit above that
  (`Math::BigInt`'s binary conversion is quadratic). Arithmetic has a
  native fast path (both operands below 2^62) and falls through to `Math::BigInt`;
  results are normalized back, so a native add can never overflow into an NV.
- **`bounded(n)`** takes an `Int` with `1 <= n <= 2^64` (`2^64` is one raw word).
- **Seeds** are `Int`s of any size reduced modulo 2^64 (two's complement: `-1` is `2^64-1`).
- **JSON numbers stay text** until `Int::parse`. Floats in witnesses render with `%.17g`
  (shortest round trip) and a `.0` suffix when integral, so Canon tells `3.0` from `3`;
  `nan`, `inf`, `-inf` are spelled out. `Values::same` treats an integer and a float as
  different kinds.

## 4. User values run code: the Safe boundary (hazards 2, 3, 4, 5, 8)

Everything in Perl can run code: stringification, numification, `eq`, `==`, `bool`,
dereference (`@{}`, `%{}`), `tie` (FETCH, FETCHSIZE, FIRSTKEY), DESTROY,
`$SIG{__DIE__}`. The rule is that **no user code runs except the property and the
generator and shrinker callbacks the caller handed in**, and those run only through
`Safe::call`.

- **Inspection is by `B`, never by dereference.** `Canon`, `Values::same`, the work
  estimate (`size_of`), the CI digest and the witness clone read a user value through
  `B::svref_2object` (`FLAGS`, `IVX`/`UVX`/`NV`/`PV`, `RV`, `FILL`, `ARRAYelt`, `KEYS`,
  `ARRAY`, `SvSTASH->NAME`). These read the internal slots and call no method, no
  overload and no tie. Any scalar, array or hash with get, set or tie magic
  (`SVs_GMG | SVs_SMG | SVs_RMG`) is **opaque** (`#Magic`) and never read through. The
  top-level value is inspected through `\$_[0]`, so not even a copy (which would FETCH)
  is made. The engine never stringifies, numifies or compares a user value;
  `Scalar::Util::blessed`, `reftype` and `refaddr` are the only reflective calls.
- **Exceptions are any value.** `Safe::call($code, @args)` runs `$code` in scalar
  context under `local $@; local $SIG{__DIE__} = 'DEFAULT'; local $SIG{__WARN__} =
  'DEFAULT'`, as `eval { $r = $code->(@a); 1 }`, and returns `(status, value)`. **The
  status is the result of `eval`, never `$@`**; `$@` is read once, only to hand the value
  on. An error is classified by `blessed`/`reftype`/`ref` only, never stringified. Local
  messages render it with Canon; redacted messages name only the kind. The caller's `$@`,
  `$_` and handlers are restored on every exit; a DESTROY that evals cannot clobber the
  status.
- **Non-local exits.** A property can `last`, `next` or `redo` out of the engine's loop
  (they cross sub and eval frames). `Safe::call` runs the user code inside a one-shot
  loop, so such an exit stops there and is reported as **escaped**: a `property-panic`
  in the runner and "does not falsify" while shrinking.
- **A property holds only on a non-reference defined scalar that is exactly one**, decided
  by the scalar's flags and slots: an IV equal to 1, an NV that is exactly 1.0 (`2/2`,
  `sqrt(1)`; `1 + 2**-52` and `1.0000000000000002` falsify), or a string whose bytes are
  exactly `"1"` (`!!1` is all three). A dualvar whose string is not `"1"` (`dualvar(2,
  "1")`, `dualvar(0, "1")`), `"1.0"`, a reference (even an object with a true `bool`),
  `undef`, `""`, `0`, `2`, a tied scalar, an empty return and any other value falsify.
  Taint magic does not matter.
- **Cycles and depth.** Canon caps nesting at 16 (`...`), nodes at 10,000 and output at
  1 MiB, **enforced while traversing**. Arrays render `[a,b]`, hashes `%{k=>v}` sorted by
  the bytes of the rendered entry, a blessed referent `Class=<structure>`, scalar
  references `\x`, code `#Code`, globs `#Glob`, regexes `#Regexp`. Engine recursion is
  bounded by these caps.
- **Strings are bytes (hazard 5).** Canon and JSON are defined on bytes. The utf8 flag is
  never consulted for meaning: a string is reduced to bytes (a flagged string that can be
  downgraded is the string of those characters; otherwise its UTF-8 octets), so the same
  characters in a flagged and an unflagged scalar render alike. The engine validates
  UTF-8 itself (`Encode`'s lax `utf8` decoder, which refuses overlong forms and
  truncation, plus an explicit refusal of surrogates and code points above U+10FFFF;
  noncharacters are valid text, as in the Lua lane). Valid strings are quoted with the **Elixir Canon escape set exactly**: `\n`,
  `\r`, `\t` short forms; `\"` and `\\` inside quotes only; every C0 control, DEL and C1
  control (U+0080 to U+009F); U+2028, U+2029; the bidirectional controls U+061C, U+200E,
  U+200F, U+202A to U+202E, U+2066 to U+2069; the invisible formatting characters U+00AD,
  U+034F, U+180E, U+200B to U+200D, U+2060 to U+2064, U+FEFF, U+FFF9 to U+FFFB and the
  tag block U+E0000 to U+E007F, as `\uXXXX` (four lowercase hex digits) or `\u{X...}`
  above U+FFFF. Any other string is `0x<lowercase hex>`. Strings are clipped to the
  remaining output budget **before** escaping. Integers above 100,000 bits are
  `<bigint N bits>`. A failure while rendering turns the whole witness into
  `<unrenderable>`. The CI digest is FNV-1a of `iter:<i>;steps:<s>;shrunk:<canonical>`,
  16 hex digits.
- **Corpus values.** Decoded corpus integers are `Int`s, booleans are two engine
  singletons (so equality tells `true` from `1`), lists are array references, bytes are
  plain strings and **text is a wrapped record**.
- **JSON.** Hand-written strict reader over bytes: nesting deeper than 64 (the 65th
  refused before recursing), documents over 8 MiB, more than 200,000 values, duplicate
  keys, invalid UTF-8 and a BOM, raw control characters, lone or reversed surrogate
  escapes, leading zeros, `+1`, `NaN`, comments, trailing commas and trailing data are
  refused. Numbers stay text.
- **Library hygiene.** The engine never uses `eval STRING`, `do FILE` or `require` with a
  data-derived path; the only `require`s are literal module names at load. Builtins
  (`die`, `sprintf`, ...) are bound at compile time, so a `*CORE::GLOBAL::` override
  installed by a property after load cannot reach the engine. After load every engine sub
  is made a **read-only glob** (`Safe::seal`), so `*Pkg::sub = sub {...}` and
  `local *Pkg::sub` die inside a property. A source-level redefinition by `eval STRING`
  is not blocked: the engine is cooperative, not sandboxed (documented below).
- **Not contained.** `exit`, `exec`, `fork` and `kill` inside a property are not
  contained, and a labelled `last OUTER` / `next OUTER` aimed at the *caller's* loop cannot
  be intercepted (an unlabelled one, and `redo`, are caught: `redo` is an escape, and the code
  is never run twice). `Safe::call` restores `%Hooks::H` and `$Budget::CURRENT` after every
  user call, so a property cannot leave them changed. The redacted `repro-fnv` is a
  non-secret fingerprint (a 64-bit FNV of a low-entropy string, brute-forceable for small
  domains), a correlation id and not a commitment.
- **Math::BigInt configuration.** The global `upgrade`, `downgrade`, `accuracy`,
  `precision` and `round_mode` of `Math::BigInt` must be at their defaults; the engine does
  not reset them.
- **Big integers.** A caller's `Math::BigInt` is refused when NaN or infinite, capped at
  5,000 digits before text conversion and copied and normalized on entry (`Int::check`);
  `Int::cmp` is correct even for an unnormalized small big and refuses NaN and infinity (`Values::same` classifies them as never equal to anything). Canon renders `NaN`, `inf`,
  `-inf` without measuring them; every `bit_length` correction loop is bounded.
- **Kind of a Perl scalar** (`Values::same`): a scalar Perl holds as an integer (`IOK`)
  whose string form, if it has one, is the canonical decimal is an int; otherwise a
  string is bytes and an NV is a float. A byte string that looks like a number and has been
  used numerically can therefore read as an int: corpus values are decoded by the
  engine and never pass through numeric use.
- **The budget is cooperative, not a security boundary.** In-process Perl code can modify
  a node hash, `local`-ise the ambient variable or open a fresh root; no proxy is built,
  matching the other lanes' in-VM limits. The budget bounds honest nesting.

## 5. The shrink-budget tree and its carrier (hazard 7)

A direct port of VER05e §4.1 in the form of VER05n §5 and VER05o §5: a scope is a node
with a `parent`, a `root`, a `depth`, a count of enclosing checks, caps (`max_attempts`
and `max_work`) and counters **attempts** (one per property evaluation, cap
`16 * max_steps + 64`) and **work** (elements copied, at least 1 per charge, cap 64M), a
`closed` flag and a `refused` flag shared by the whole tree.

- **All or nothing.** `charge(n)` (an attempt) and `charge_copy(n)` (work only, so the
  corpus is unchanged) walk from the node to the root and require on every node *not
  closed*, `attempts < max_attempts` and `cost <= max_work - work`; only then are all
  counters incremented. A refusal sets the shared `refused` flag and leaves every counter
  unchanged. A cost that is not a non-negative integer is refused; zero counts as one.
  Callers charge **before** building a candidate.
- **Public entries open a child, internal combinators join.** `Gen::shrink`,
  `shrink_int`, `shrink_list`, `shrink_bytes`, `simulate_replay` and the runner's shrink
  open a **child** of the scope they are given or of the ambient scope (a fresh root when
  there is none); `tuple`, `list`, `filter`, `bytes` shrinkers **join** the scope they
  are handed. Owning nesting is capped at depth 64; deeper scopes join.
- **`check` wraps its whole body** in one scope with attempt cap
  `num_tests + 16 * max_steps + 64`. A **nested** check pays 1 attempt at entry (refused:
  `property-exhausted` without evaluating anything), 1 per case after the first, the
  sampled value's size as work (a string its bytes; an array or hash its entry count plus,
  one level deeper, the size of each string or container child, visiting at most 10,000
  entries) and 1 attempt per `filter` retry. A **top-level** check always evaluates its
  first case and charges filter retries as **work**. A check whose parent already has 64
  enclosing checks evaluates nothing and is `property-exhausted`. If any charge in the
  tree was refused, a top-level check that would pass is `property-exhausted`; a
  falsified verdict is never overridden. The single re-verification charges the check
  scope; if refused the original witness is reported with 0 steps.
- **A closed or stale handle is SPENT**: a node whose own `closed` flag, or any
  ancestor's, is set refuses every charge, is never resolved to a nearest open ancestor
  and is never a fresh root; a check run under a spent parent is `property-exhausted`.

### 5.1 The carrier: dynamic scoping with `local` (decision)

Java and Ruby keep the current scope in an inheritable thread-local or fiber storage;
Elixir in the process dictionary; Lua and Haskell pass it as an explicit parameter
because they have no restorable ambient state. **Perl has exactly the restorable ambient
state the Java design wants: `local`.** `local $Budget::CURRENT = $scope` is undone on
every exit from the enclosing block, including `die`, `last` and unwinding past a user's
`eval`, with no `finally` to forget.

| Design | Result |
|---|---|
| Explicit parameter only (Lua, Haskell) | A custom shrinker, a `filter` predicate or a property that forgets to pass the parent silently opens a **fresh root**, re-opening the nested amplification VER05e §4.1 exists to close |
| **`local` ambient scope plus an optional explicit `parent`** | **Chosen.** Every public entry that opens a scope does `local $CURRENT = $node`; every public entry that needs a parent defaults to `$CURRENT`, so a property that calls `Runner::check`, a custom shrinker calling `Gen::shrink`, or `shrink_int` inside a property all charge the outer tree without cooperation. An explicit `parent` overrides the ambient one and is how a stale handle is passed and refused |

Internal combinators still receive their scope explicitly, so accounting inside one
shrink never depends on the ambient variable. Scopes are owned by a guard object whose
`DESTROY` closes the node (with `local ($@, $!, $?)`, so a close during unwinding cannot
clobber an exception in flight); the guard is a lexical so closing is deterministic, and a
handle a property stashes never keeps a scope open (node and guard are different
objects). Perl's core has no fibers; **ithreads** clone the variable and tree at thread
creation, so a thread charges its own copy, which is outside the supported surface.

## 6. Runner, verdicts and failure vocabulary

`Runner::check($seed, $gen, $prop, $options [, $shrink])` runs `num_tests` (default 100)
cases from `SplitMix64(seed)` and always returns a verdict (invalid arguments `die`).

- `num_tests == 0` → **fail `property-exhausted`**.
- a generator error of any kind (a thrown value, an exhausted filter, a refused charge)
  → `property-exhausted`.
- the property dies (any value, including `undef` and an object) or escapes with
  `last`/`next` → `property-panic`.
- the property returns anything but a plain `1` → shrink, then `property-counterexample`.
- every case held → pass.

The reported witness is **re-verified**: if the shrinker dies, returns a value that no
longer falsifies, one on which the property dies, or an out-of-range step count, the
original candidate is reported with 0 steps. `shrink` (a code reference) overrides the
generator's shrinker. A redacted verdict (CI on or unset) carries no seed, counterexample
or minimal value, and its message no concrete value. There is no wall-clock timeout:
`property-timeout` is registered and never produced; tests bound runaway properties with
`alarm`.

## 7. Conformance and negative controls

`Corpus::evaluate(\@cases)` decodes strictly (exactly the fields of the op; canonical
decimal ints of at most 1,000 digits; even lowercase hex bytes; unknown op, property or
malformed value is a problem; every problem starts with its case id), applies the CWE-400
caps, requires every op and all four failure ids, an unregistered id rejected and exactly
25 cases, and checks `corpus_reject` texts with the real TFV validator. The test replays
the committed fixture (25 evaluated, zero problems) and, **for every case and every
corruption its op defines**, asserts the run fails and every problem starts with that
case's id. Dropped and truncated corpora and an unknown op fail; every sabotage hook
(bisection, unit step, skipped deletion, shifted RNG, wrong rejection threshold) makes
the run fail. `Hooks::with(\%overrides, $code)` swaps the entries of `%Hooks::H` with `local`, so they
are restored on every exit; they are the only mutable package state (besides the ambient
budget variable). The rejection-threshold sabotage must be a *large* wrong value: a
threshold of 0 passes the whole corpus.

## 8. Divergences from the other lanes

| # | Topic | Perl | Others | Reason |
|---|---|---|---|---|
| 1 | integers | native IV or `Math::BigInt`, normalized | Ruby/Elixir bignum, C `pbt_big`, Lua private Big | core `Math::BigInt` suffices |
| 2 | words | UV with 32-bit-half `add64`/`mul64` | Java `long`, Lua wrapping int | no defined signed wrap, silent NV overflow |
| 3 | property result | only plain `1` holds | Lua `true`, Ruby truthy | no boolean type before 5.36; fail closed |
| 4 | errors | any value; `blessed`/`reftype`; non-local exits are a panic | exception classes | `die` takes any value, `last` crosses frames |
| 5 | ambient carrier | `local $CURRENT` plus optional explicit `parent` | thread-local / process dictionary / explicit | §5.1 |
| 6 | locking | none | Mutex / CAS | ithreads clone |
| 7 | closed ancestor | spent, never resolved upward | Ruby/Elixir resolve upward | stale handles are never a fresh root |
| 8 | inspection | `B` slots; magic is opaque | native reflection | overloads and ties run code |
| 9 | text vs bytes | text is a wrapped record; bool two singletons | encodings / tags | Perl strings have no kind |
| 10 | `property-timeout` | registered, never produced | same | VER05 §7.4 #5 |

## 9. Build and inputs

`BUILD` and `BUILD_windows` mirror the lane's leaf packages
(`cpanm --installdeps --quiet .` then `prove -l -v t/`; the shared Windows recipe
`run-perl-tests-windows.ps1`); the corpus conformance is one of the test files. The
package reads `code/specs/fixtures/pbt-v1/` in its tests, so `perl/test-bootstrap` is
registered as a repository-input consumer of that directory in the Go build tool
(`repository_inputs.go`) beside the other lanes. Coverage is measured on demand with
`Devel::Cover` (not core, never a dependency).

## 10. Tests

- Replay of the committed corpus: exactly 25 evaluated, zero problems; the negative
  controls and sabotage hooks of §7.
- **Pinned tests for what the corpus cannot catch** (shared with the C, Ruby, Lua and
  Haskell lanes):
  - the rejection draw: seed `7046029254386353131` starts a stream whose first word is
    `0` and second `16294208416658607535`; `bounded(3)` has threshold 1, must reject the
    `0` and answer `16294208416658607535 mod 3 = 1`; **a threshold of 0 still passes the
    whole corpus and the test asserts both facts**;
  - the `2^63 + 1` boundary: threshold `2^63 - 1`; seed 3's first word
    `2092789425003139053` is below it, so `bounded` answers from the second word
    `3694763184872335752`; seed 1's first word `1227844342346046656` is accepted;
    thresholds also at `n = 1, 2, 3, 2^63, 2^64 - 1`;
  - `bounded(1)` consumes a word (after `bounded(1)` from seed 7 the next word is
    `309689372594955804`);
  - floored negative bisection (`-81` gives candidate `-40`), the step counts of VER05n
    §8.1 (`(101, 15)`, `(0, 1)`, `(51, 49)`, `([5], 3)`, `(10, 1)`, `shrink_bytes` 16
    steps, `simulate_replay 42` `(413, 101)`), and both bisection halves.
- **Hazard 1:** SplitMix64 and FNV-1a vectors, `add64`/`mul64` against `use integer` and
  `Math::BigInt`, `Int` arithmetic across the 2^62 and 2^63 boundaries, floats, strings
  and `undef` refused as seeds, a 1,000-digit fixture integer accepted and a 1,001-digit
  one refused before parsing, a negative seed wrapping.
- **Hazard 2:** trap objects whose `""`, `0+`, `bool`, `eq`, `==`, `<=>`, `cmp`, `@{}`,
  `%{}` and DESTROY record and die, and tied arrays, hashes and scalars whose every
  method records and dies: Canon, `same`, `size_of` and the clone leave the log empty.
- **Hazard 3:** `die` with a string, `undef`, a plain reference and an object with a
  dying `""`; a dying `$SIG{__DIE__}` and `$SIG{__WARN__}`; a DESTROY that evals;
  `last`/`next` out of a property; the caller's `$@` untouched.
- **Hazards 4 and 5:** self-containing array and hash, a depth-1000 chain, a
  million-entry array, a 10 MiB string, invalid UTF-8, every escape class, flagged and
  unflagged strings with the same characters.
- **Hazard 7:** the required set of VER05e's `ShrinkBudgetTest` (nesting depth 1 to 3 and
  the branching case, a recursing custom shrinker, a filter predicate that shrinks, a
  filter sampler rejecting everything, a refused charge leaving every counter unchanged,
  a huge cost, scope release after an error, depth-cap joining, starvation never passing,
  a refused re-verification, top-level filter retries as work, a closed handle being
  spent and never a fresh root, the ambient scope restored after `die`).
- **Hazard 8:** a property that redefines engine subs and installs `*CORE::GLOBAL::`
  overrides still gets a verdict.
- **Hazard 9:** every test that runs possibly-looping user code sets an `alarm` with
  `local $SIG{ALRM}` that dies, so a regression fails instead of hanging.
