# VER05o — Lua Property-Based Testing Engine

**Parent:** [VER00 §7.2, §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md), [VER05](VER05-property-engine.md), [VER05e](VER05e-java-pbt-engine.md), [VER05g](VER05g-csharp-pbt-engine.md), [VER05h](VER05h-ruby-pbt-engine.md), [VER05i](VER05i-c-pbt-engine.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-LUA-PBT.

This specification defines the Lua implementation of the Level 2 Property-Based
Testing (PBT) engine. Lua had no test-bootstrap and no property engine before this
change. The engine conforms to the `pbt-v1` corpus
(`code/specs/fixtures/pbt-v1/cases.json`, 25 cases) with exact outputs, minimal
witnesses **and shrink step counts**, and implements the verdict-honesty rules of
VER05 §7.1 and the hierarchical shrink-budget tree of
[VER05e §4.1](VER05e-java-pbt-engine.md), which is part of the first version.

(Letters `j`, `k` and `l` belong to the C++, Swift and F# lanes, `m` to Elixir and
`n` to Haskell, all open on their own branches; `o` is the next free letter.)

The algorithms are those of the Ruby and C lanes ([VER05h](VER05h-ruby-pbt-engine.md),
[VER05i](VER05i-c-pbt-engine.md)); the hardening list is that of the Elixir and Haskell
lanes (VER05m, VER05n), whose Canon escape set is copied exactly. This document does
not repeat the algorithms. It states what is the same (§1), what Lua's number model,
error model, metatables and coroutines change (§3 to §5), and the tests that pin each
hazard (§10).

---

## 1. Same as Java, C#, Ruby, C, Elixir and Haskell (normative by reference)

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
- VER05e §4.1: the budget tree (restated for Lua in §5).
- VER05 §7.1 and VER05e §5: the runner, the closed failure vocabulary, verdict honesty,
  witness re-verification, CI redaction. **A property holds only on exactly `true`**;
  an unset CI mode is CI.
- VER05e §6: conformance runner, strict decoding, CWE-400 caps, negative controls.

## 2. Package and modules

`code/packages/lua/test_bootstrap` (the build tool names it `lua/test_bootstrap`),
rockspec `coding-adventures-test-bootstrap`, root module
`coding_adventures.test_bootstrap`, Lua `>= 5.4` (CI pins 5.4.7), tests under
`busted`. **Zero third-party dependencies**: pure Lua 5.4 standard library.

| Module | Role |
|---|---|
| `init` | facade: `Int`, `Prng`, `Gens`, `Shrink`, `Runner`, `Budget`, `Corpus`, ... |
| `int` | arbitrary-size integers over Lua's 64-bit integers (§3) |
| `prng` | `Rng`, `next_word` (the raw word), `next_u64` (an `Int`), `bounded`, `bounded_big`, `udivmod`, `fnv1a64`, `sub_seed` |
| `limits`, `failure_ids`, `hooks` | caps, the four ids, sabotage seams |
| `budget` | the budget tree (§5) |
| `gen`, `gens` | `Gen` and the generator algebra, corpus entry points |
| `shrink` | integer walk, sequence shrinker, `shrink_int/list/bytes`, `simulate_replay` |
| `runner` | `Options`, `check`, verdicts |
| `safe` | error classification (§4) |
| `canon`, `strict_json`, `values`, `tfv` | rendering, strict decoding, TFV validator |
| `corpus` | the pbt-v1 conformance evaluator |

The library does no file, clock, network or environment access, so
`required_capabilities.json` is capability-free. The CI flag is a parameter
(`Options.ci_from_value(os.getenv("CI"))` is the caller's call). Tests read the
committed fixture by walking up to the repository root.

## 3. Lua's number model (hazard 1)

Lua 5.4 has 64-bit **signed** integers that wrap on overflow, 64-bit floats, and
comparison operators that are signed. `//` and `%` floor. `>>` is a *logical* shift.

- **Words.** The PRNG works on *words*: a Lua integer used as an unsigned 64-bit
  pattern. Multiplication and addition wrap exactly as SplitMix64 and FNV-1a
  require, so no masking is needed. A word is **never** compared with `<`, divided
  with `//` or printed with `%d`: the engine uses `math.ult`, an unsigned
  divide/modulo (`udivmod`, below) and `Int.from_u64` for every such operation.
- **`Int`: the mathematical integer.** Everything public (seeds, shrink targets,
  `u64` and `i128` values, corpus integers) is an `Int`: a Lua integer when the
  value fits in the signed 64-bit range, otherwise a **Big** (a private
  sign-magnitude record, base 2^24 limbs). The representation is **normalized**:
  a value in `[-2^63, 2^63-1]` is *always* a native Lua integer, so equality of two
  `Int`s is `Int.eq`, never `==` (Bigs are tables). Floats are not `Int`s:
  `Int.is(3.0)` is false, and a float given as a seed, bound, count, length or
  weight is a `TypeError` (checked with `math.type`), even when integral, so a
  float can never reach the PRNG path or become a table key by accident. Every
  arithmetic function has a native fast path guarded by an overflow test and falls
  through to limbs on overflow; `math.mininteger` has its own test (`-min` wraps).
  Fixture integers are capped at 1,000 digits, generator spans at 16,384 bits.
- **Unsigned divide.** `udivmod(a, b)` on words: if `b < 0` (bit 63 set) the
  quotient is 0 or 1; if `a >= 0` it is `a // b`, `a % b`; else
  `q = ((a >> 1) // b) << 1`, `r = a - q*b`, and one correction step. The rejection
  threshold is `t = (0 - n) umod n` (`0 - n` is `2^64 - n` as a word), computed
  unsigned, and a draw `x` is kept when `not math.ult(x, t)`. Spelling it
  `(-n) % n` on a signed integer gives a wrong value for every `n` (floored modulo of a
  negative number by its own magnitude is 0 when `n` is small, and is simply wrong
  above `2^63`): the pins of §10 fix `n = 1, 2, 3, 2^63, 2^63 + 1, 2^64 - 1` and `2^64`.
- **`bounded(n)`** takes an `Int` with `1 <= n <= 2^64` (`2^64` is one raw word);
  anything else, and any float, is an error. Results are `Int`s (a result above
  `2^63 - 1` is a Big).
- **Seeds** are `Int`s of any size, reduced modulo 2^64 (two's complement, so `-1` is
  `2^64 - 1`); booleans, floats and strings are a `TypeError`.
- **Floats in witnesses** render with `%.17g` and a `.0` suffix when integral
  (`3.0`, never `3`), so `Canon` distinguishes `3` from `3.0`; `nan`, `inf`, `-inf`
  are spelled out. `Values.same` treats an integer and a float as different kinds.

## 4. Errors, metatables and strings (hazards 2, 3, 4, 5)

- **Errors are any value.** The runner calls a property with `pcall`. The error may
  be a string, a table, `nil`, a function, a coroutine. Classification is **by the
  result of `pcall` and `type()` only**: the engine never calls `tostring`, never
  reads `.message` or any field of an error and never uses `==`/`#`/`pairs` on it, so
  a table with a throwing `__tostring`, a `__index` that errors, or `error(nil)` is a
  `property-panic`, never an engine crash. Redacted messages name only `type(err)`
  (`string`, `table`, `nil`, ...). Local messages render the error with `Canon`
  (structural, clipped to 200 bytes). Inside shrinking and re-verification an error
  means "this candidate does not falsify" and is never accepted as smaller.
  Sample-time errors of any kind are `property-exhausted`.
- **A hostile environment cannot make the engine raise.** A property can patch the string
  metatable's `__index`, `string.*`, `table.*`, `utf8.*` or `math.*`. Every module captures
  the standard functions it uses in locals at load time and calls them as plain functions
  (never `s:method()`), and `Runner.check` runs its body under `pcall`, so an engine failure
  is an honest `property-panic` naming only the error's type, never a thrown error. The
  engine restores nothing it does not own. Author-supplied tables (`Options.new`'s table,
  `one_of`/`frequency`/`tuple` arrays, `Hooks.with` overrides) are read with `next`,
  `rawget` and `rawlen` only. `Int.parse` caps digits itself (1000 by default) and
  `Int.shl`/`Int.pow2` refuse results wider than 2^20 bits; the runner copies a Big witness
  before handing it to a shrinker.
- **The budget is cooperative, not a security boundary.** In-process Lua code can write
  `scope.closed = false` or `scope.parent = nil`, replace a metatable, or open a fresh root
  by calling `Runner.check` without `parent`; no proxy is built, matching the other lanes'
  in-VM limits. `Budget.is_scope` and the `Options` metatable check are type checks that
  catch mistakes, not defences against a hostile holder. The budget bounds honest nesting.
- **No user metamethod ever runs.** `Canon`, `Values.same`, the work estimate
  (`size_of`) and the CI digest read user tables with `rawget`, `rawequal`, `rawlen`
  and `next` only; `__index`, `__newindex`, `__eq`, `__len`, `__pairs`, `__tostring`,
  `__lt`, `__concat`, `__call`, `__gc` and `__close` of a user value are never
  triggered by the engine (the tests install a metatable whose every metamethod
  records and raises, and assert the log is empty). Engine-owned records are plain
  tables or carry the engine's own metatable.
- **Cycles and depth.** `Canon` caps nesting at 16 (`...`), nodes at 10,000 and
  output at 1 MiB, **enforced while traversing**; a table that contains itself, a
  deep chain and a 1,000,000-entry table terminate with a truncated rendering.
  Tables are rendered as `[a,b]` when their keys are exactly `1..n`, else as
  `%{k=>v}`, with entries **sorted by the bytes of the rendered key** so the digest
  does not depend on `next` order; strings are compared bytewise by an explicit
  comparator (Lua's `<` uses `strcoll`, which depends on the host locale), so the package
  never queries or sets the locale and needs no environment access.
- **Strings are byte strings.** `utf8.len(s)` (strict: it refuses surrogates, overlong
  forms and code points above U+10FFFF) decides whether a string is text. Valid
  strings are quoted with the **Elixir Canon escape set exactly**: `\n`, `\r`, `\t`
  short forms; `\"` and `\\` inside quotes only; every C0 control, DEL (U+007F) and C1
  control (U+0080 to U+009F); U+2028, U+2029; the bidirectional controls U+061C,
  U+200E, U+200F, U+202A to U+202E, U+2066 to U+2069; the invisible formatting
  characters U+00AD, U+034F, U+180E, U+200B to U+200D, U+2060 to U+2064, U+FEFF,
  U+FFF9 to U+FFFB and the tag block U+E0000 to U+E007F, as `\uXXXX` (four lowercase
  hex digits) or `\u{X...}` above U+FFFF. Any other string is `0x<lowercase hex>`.
  Strings are clipped to the remaining output budget **before** escaping. Integers
  above 100,000 bits are `<bigint N bits>`. Functions, userdata and threads are the
  fixed placeholders `#Function`, `#Userdata`, `#Thread`. A failure while rendering
  turns the whole witness into `<unrenderable>`. The CI digest is FNV-1a of
  `iter:<i>;steps:<s>;shrunk:<canonical>`, 16 hex digits.
- **Corpus values.** Decoded corpus integers are `Int`s, bool are booleans, lists
  are arrays, bytes are Lua strings and **text is a wrapped record** (so equality
  tells a byte string from text, as the Ruby lane tells binary from UTF-8 strings).
- **JSON.** Hand-written strict reader (Lua has none): nesting deeper than 64 (the 65th
  refused before recursing), documents over 8 MiB, more than 200,000 values, duplicate
  keys (compared as decoded bytes), invalid UTF-8 and a BOM, raw control characters in
  strings, lone or reversed surrogate escapes, leading zeros, `+1`, `NaN`, comments,
  trailing commas and trailing data are refused. Numbers stay text; no JSON number is
  ever converted to a Lua number.

## 5. The shrink-budget tree and its carrier (hazards 6, 7)

A direct port of VER05e §4.1 in the form of VER05n §5: a scope (`Budget`) is a node
with a `parent`, a `root`, a `depth`, a count of enclosing checks, caps (`max_attempts`
and `max_work`) and counters **attempts** (one per property evaluation, cap
`16 * max_steps + 64`) and **work** (elements copied, at least 1 per charge, cap
64M), a `closed` flag and a `refused` flag shared by the whole tree.

- **All or nothing.** `charge(n)` (an attempt) and `charge_copy(n)` (work only, so the
  corpus is unchanged) walk from the node to the root and require on every node
  *not closed*, `attempts < max_attempts` and `cost <= max_work - work` (overflow-safe);
  only then are all counters incremented. A refusal sets the shared `refused` flag and
  leaves every counter unchanged. A cost that is not a non-negative integer (a float, NaN, a
  string, `nil`, a negative number) is refused (fail closed); zero counts as one. Callers charge **before** building a candidate.
- **Accounting.** One property evaluation is one attempt, charged by the `falsifies`
  function an entry point hands to a shrinker; a sequence shrinker additionally charges
  the **copy** before building each candidate (VER05n §5).
- **Public entries open a child, internal combinators join.** `Gen.shrink`, `shrink_int`,
  `shrink_list`, `shrink_bytes`, `simulate_replay` and the runner's shrink open a
  **child** of the scope they are given (a fresh root when none) with a fresh local
  cap; `tuple`, `list`, `filter`, `bytes` shrinkers **join** the scope they are handed.
  Owning nesting is capped at depth 64; deeper scopes join.
- **`check` wraps its whole body** in one scope with attempt cap
  `num_tests + 16 * max_steps + 64`. A **nested** check (given a `parent`) pays 1
  attempt at entry (refused: `property-exhausted` without evaluating anything), 1 per
  case after the first, the sampled value's size as work (a string its bytes; a table
  its entry count plus, one level deeper, the size of each string or table child,
  visiting at most 10,000 entries) and 1 attempt per `filter` retry. A **top-level**
  check always evaluates its first case and charges filter retries as **work**. A
  check whose parent already has 64 enclosing checks evaluates nothing and is
  `property-exhausted`. If any charge in the tree was refused, a top-level check
  that would pass is `property-exhausted`; a falsified verdict is never overridden.
  The single re-verification charges the check scope; if refused the original witness
  is reported with 0 steps.
- **A closed or stale handle is SPENT** (VER05n §5): a node whose own `closed` flag, or
  any ancestor's, is set refuses every charge. There is no resolution to a nearest open
  ancestor and a stale handle is never a way to obtain a fresh root; a check run under
  a spent parent is `property-exhausted`.

### 5.1 The carrier is an explicit parameter (decision)

Java and Ruby keep the current scope in an inheritable thread-local or fiber storage;
Elixir keeps it in the process dictionary. Lua has no inheritance of any of these:
a module-level variable is shared by **every coroutine in the state**, so a scope
opened in one coroutine and left suspended by a `coroutine.yield` would be the
"ambient" scope of whatever coroutine runs next, and a coroutine that is
garbage-collected or closed mid-scope would leave the variable stale. Two designs
were considered.

| Design | Result |
|---|---|
| Ambient state: a module variable, or a weak table keyed by `coroutine.running()` | Rejected. The module variable is wrong under yields; the per-coroutine table must be restored on every exit including a coroutine closed from outside, a new coroutine does not inherit it (silently a fresh root), and "which budget am I charging?" is invisible at the call site |
| **Explicit parameter** | **Chosen**, as in the C lane (VER05i §6) and Haskell (VER05n §5.1) |

Every function that can open a scope takes the parent explicitly: `Gen.shrink(gen, v,
max_steps, falsifies, parent)`, `Shrink.shrink_int(..., parent)`, `Options.new{parent =
scope}`. A property is called as `prop(value, scope)` where `scope` is the scope of the
running check, so `Runner.check(seed2, gen2, prop2, Options.new{parent = scope})` nested
inside a property charges the outer tree. A generator sampler is `sampler(rng, scope)`
and a custom shrinker `shrinker(value, falsifies, max_steps, scope)`. There is **no
module-level mutable state in production paths**: nothing to restore, so a property
that yields across the `pcall` boundary, a coroutine killed with `coroutine.close`, an
error object that is itself a coroutine, or a coroutine that captured a handle cannot
corrupt another check. A coroutine that holds a handle charges the same tree through the
handle (Lua is single-threaded: there is no concurrent charging, only interleaving at
yields, and every charge is atomic with respect to it); a coroutine that holds no handle
can only open a fresh root, and the bound is then per flow, documented and pinned. Scopes
are opened as `local scope <close> = Budget.open_*(...)` so a scope closes on every exit
including an error and `coroutine.close`; a scope abandoned inside a coroutine that is
never resumed and never closed is reclaimed by the garbage collector with the coroutine,
and its parent's closing already makes any retained handle spent.

## 6. Runner, verdicts and failure vocabulary

`Runner.check(seed, gen, prop, options)` runs `num_tests` (default 100) cases from
`SplitMix64(seed)` and always returns a verdict (invalid arguments raise instead).

- `num_tests == 0` → **fail `property-exhausted`**.
- a generator error of any kind (a thrown value, an exhausted filter, a refused charge)
  → `property-exhausted`.
- the property raises (any value, including `nil`) → `property-panic`.
- the property returns anything but `true` (`false`, `nil`, `1`, no value) → shrink,
  then `property-counterexample`: a function that forgets its return cannot pass.
- every case held → pass.

The reported witness is **re-verified**: if the shrinker raises, returns a value that no
longer falsifies, one on which the property raises, or an out-of-range step count, the
original candidate is reported with 0 steps. `shrink` (a function) overrides the
generator's shrinker so a defective shrinker can be tested. A redacted verdict (CI on or
unset) carries no seed, counterexample or minimal value, and its message no concrete
value. `repro-fnv` is a correlation id, not a secret (VER05 §7.4 #3). There is no
wall-clock timeout: `property-timeout` is registered and never produced; tests bound
runaway properties with `debug.sethook` (§10).

## 7. Conformance and negative controls

`Corpus.evaluate(cases)` decodes strictly (exactly the fields of the op; canonical decimal
ints of at most 1,000 digits; even lowercase hex bytes; unknown op, property or
malformed value is a problem; every problem starts with its case id), applies the
CWE-400 caps of VER05h, requires every op and all four failure ids, an unregistered id
rejected and exactly 25 cases, and checks `corpus_reject` texts with the real TFV validator.
The test replays the committed fixture (25 evaluated, zero problems) and, **for every
case and every corruption its op defines** (flipped output, wrong minimal, steps + 1,
wrong failure id, flipped validity, a valid TFV in place of a reject text), asserts the
run fails and every problem starts with that case's id and no other. Dropped and
truncated corpora and an unknown op fail; every sabotage hook (bisection, unit step,
skipped deletion, shifted RNG, wrong rejection threshold) makes the run fail.
`Hooks.with(overrides, fn)` swaps seams for one call and restores them on every exit; they
are the only module-level mutable state and no production path writes them.

## 8. Divergences from the other lanes

| # | Topic | Lua | Others | Reason |
|---|---|---|---|---|
| 1 | integers | native int64 or private Big, normalized | Ruby/Elixir bignum, C `pbt_big` | no bignum in Lua |
| 2 | property result | only `true` holds | Ruby truthy | fail closed |
| 3 | errors | any value; classified by type only | exception classes | `error` takes any value |
| 4 | ambient carrier | none: explicit parameter | fiber storage / process dictionary | §5.1 |
| 5 | locking | none (single-threaded) | Mutex / CAS | coroutines interleave only at yields |
| 6 | closed ancestor | spent, never resolved upward | Ruby/Elixir resolve upward | stale handles are never a fresh root |
| 7 | text vs bytes | text is a wrapped record | encodings / tags | Lua strings are bytes |
| 8 | tuples | arrays, arity at most 64 | native tuples | no tuple type |
| 9 | hooks | module table, `Hooks.with` | accessors | test seam only |
| 10 | `property-timeout` | registered, never produced | same | VER05 §7.4 #5 |

## 9. Build and inputs

`BUILD` and `BUILD_windows` run `cd tests && busted . --verbose --pattern=test_` like
the lane's other packages; the corpus conformance is one of the test files. The package
reads `code/specs/fixtures/pbt-v1/` in its tests, so `lua/test_bootstrap` is registered
as a repository-input consumer of that directory in the Go build tool
(`repository_inputs.go`) beside the other lanes; the selection tests that enumerate
consumers fail closed until Lua is added. Coverage is measured on demand with luacov
(`LUA_COVERAGE=1 busted . --pattern=test_ --coverage`; CI installs no luacov, so no gate
is added to `BUILD`) and the changelog quotes the measurement (97.2% of executable lines
at release). The tests' instruction-count bound is skipped under `LUA_COVERAGE`, because
luacov credits lines to the frame two levels above its hook and a chained hook would
misattribute them.

## 10. Tests

- Replay of the committed corpus: exactly 25 evaluated, zero problems; the negative
  controls and sabotage hooks of §7.
- **Pinned tests for what the corpus cannot catch** (values shared with the C, Ruby and
  Haskell lanes, re-derived by `code/scripts/pbt_conformance.py`):
  - the rejection draw: seed `7046029254386353131` starts a stream whose first word is
    `0` and second `16294208416658607535`; `bounded(3)` has threshold 1, must reject the
    `0` and answer `16294208416658607535 mod 3 = 1`; a threshold of 0 still passes the
    whole corpus and the test asserts both facts;
  - the `2^63 + 1` boundary: threshold `2^63 - 1`; seed 3's first word
    `2092789425003139053` is below it, so `bounded` answers from the second word
    `3694763184872335752`; seed 1's first word `1227844342346046656` is accepted;
    unsigned thresholds are also checked at `n = 1, 2, 2^63, 2^64 - 1`;
  - `bounded(1)` consumes a word (after `bounded(1)` from seed 7 the next word is
    `309689372594955804`);
  - floored negative bisection (`-81` gives candidate `-40`, not `-41`), the step
    counts of VER05n §8.1 (`(101, 15)`, `(0, 1)`, `(51, 49)`, `([5], 3)`, `(10, 1)`,
    `shrink_bytes` 16 steps, `simulate_replay 42` `(413, 101)`), and both bisection halves.
- **Hazard 1 (arithmetic):** word wrap at `maxinteger`/`mininteger`, `udivmod` against a
  Python-generated vector table, `Int` add/sub/mod/shift/decimal against the same table,
  float seeds/counts refused, `3.0` never equal to `3`, a 1,000-digit fixture integer
  accepted and a 1,001-digit one refused, a negative seed wrapping.
- **Hazard 2/3 (errors, metatables):** `error(nil)`, `error(table_with_throwing_tostring)`,
  `error(setmetatable({}, {__index = raise}))`, an error that is a coroutine; a metatable
  trap on every metamethod during `Canon`, `same`, `size_of` and the digest.
- **Hazard 4/5 (hostile values, strings):** a self-containing table, a depth-1000 chain, a
  million-entry table, a 10 MiB string, invalid UTF-8, every escape class.
- **Hazard 6 (coroutines):** a check inside a coroutine whose property yields and is
  resumed; a coroutine closed mid-check closes its scope; a handle captured by a
  coroutine charges the same tree, after close it is spent.
- **Hazard 7 (budget tree):** the required set of VER05e's `ShrinkBudgetTest` (nesting
  depth 1 to 3 and the branching case, a recursing custom shrinker, a filter predicate that
  shrinks, a filter sampler rejecting everything, a refused charge leaving every counter
  unchanged, a huge cost, scope release after an error, depth-cap joining, re-entrancy
  equality, starvation never passing, a refused re-verification, top-level filter retries
  as work, a closed handle being spent and never a fresh root).
- **Security-review fixes:** trapped option/array tables, a property that patches the
  string metatable and library functions (still a verdict), Int digit and bit caps, a Big
  witness copied for the shrinker, no locale query, 8 MiB of JSON escapes.
- **Hazard 8 (no hangs):** every test that runs user code that could loop installs an
  instruction-count hook (`debug.sethook`) that raises, so a regression fails instead of
  hanging.
- A regression test per hardening item of §3 to §7.
