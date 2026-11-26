# VER05i — C Property-Based Testing Engine

**Parent:** [VER00 §7.2, §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md), [VER05](VER05-property-engine.md), [VER05d](VER05d-go-pbt-engine.md), [VER05e](VER05e-java-pbt-engine.md), [VER05g](VER05g-csharp-pbt-engine.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-C-PBT.

This specification defines the C implementation of the Level 2 Property-Based
Testing (PBT) engine. C had no test-bootstrap, no property engine and no fuzz
engine before this change. The engine conforms to the `pbt-v1` corpus
(`code/specs/fixtures/pbt-v1/cases.json`, 25 cases) with exact outputs, minimal
witnesses **and shrink step counts**, and implements the verdict-honesty rules of
VER05 §7.1 and the hierarchical shrink-budget tree of
[VER05e §4.1](VER05e-java-pbt-engine.md). It is a port of the structure of
[VER05d](VER05d-go-pbt-engine.md) (explicit big integers, no generics) with the
hardening of the Java and C# lanes carried over from the start; every place where
C forces a different shape is listed in §12.

The letter `i` follows VER05h (Ruby, PR #20017), the highest letter in use.

---

## 1. Architecture and Placement

The package is `code/packages/c/test-bootstrap`: pure ISO C (C17 as the shared
`iso-harness` compiles it, `-pedantic-errors -Wall -Wextra -Werror`, and
`/permissive- /W4 /WX` on MSVC), libc only, no extension, no third-party code.

```text
 tests / consumers                          conformance
 ───────────────────                        ───────────
 pbt_check(parent, seed, gen, prop, opts)   pbt_corpus_evaluate(json_cases)
        │                                          │ (strict JSON parse, then
        ▼                                          │  the SAME engine functions)
   pbt_gen (sample + shrink function pointers + void *self) ◄───────┘
     │ map / filter / flat_map / one_of / frequency / list / tuple
     ▼
   integrated shrinkers: int walk, sequence shrinker (bytes and lists)
     │
     ▼
   pbt_budget tree     pbt_rng (SplitMix64, FNV-1a)     pbt_big (bignum)
```

| Header | Role |
|---|---|
| `pbt_big.h` | sign-magnitude big integer in fixed `uint32_t` limbs (§3) |
| `pbt_json.h` | strict JSON reader with hard caps (§8) |
| `pbt_tfv.h` | strict validator for TFV v1 text, so `corpus_reject` is checked by a real parser (§8) |
| `pbt.h` | PRNG, tagged values, generators, shrinkers, budget, runner, canonical rendering, corpus evaluation |
| `pbt_hooks.h` | test seams: sabotage function pointers, allocation counter and failure injection (§9) |

The library performs no file, clock, network or environment access, so
`required_capabilities.json` is capability-free. Reading `cases.json` is the job
of the test, which walks up from its working directory to the repository root
and reads the committed fixture, never a copy. The CI flag is a parameter
(`pbt_ci_from_value(getenv("CI"))` is the caller's call), never read inside.

**Single-threaded.** Every type here is single-threaded by contract: no
engine call may be made concurrently on the same budget tree, and there is no
lock. The Java lane needs a lock and an inheritable thread-local because user
code can spawn threads that share the ambient scope; C has no ambient scope at
all (§6), so that class of bug cannot exist.

---

## 2. Deterministic PRNG (`pbt_rng`)

`pbt_rng` is the TF01 §5.1 / VER05 §2.1 state machine on a `uint64_t` (unsigned
arithmetic wraps, so no masking). Seed 0's first output is `0xe220a8397b1dcdaf`,
pinned by a test.

### 2.1 Bounded draws

`pbt_rng_bounded(r, n)` is uniform on `[0, n)` for `1 <= n <= 2^64-1`. The
rejection threshold is `t = (2^64 - n) mod n`, which in C is `(0 - n) % n` on a
`uint64_t`: unsigned negation wraps modulo 2^64, so the spelling is **correct
as written** (unlike Python, which cannot spell it, and Java, which needs
`Long.remainderUnsigned`). The same expression computed through a *signed*
type is wrong for `n > 2^63`; a pinned test at `n = 2^63 + 1` (threshold
`2^63 - 1`) with forced draws (§8) fails if the threshold is miscomputed.
`n == 0` is a caller bug: it returns 0 and consumes nothing.

**`pbt_rng_bounded(r, 1)` consumes one stream word** (the §2.2 loop run once),
as the Rust, Go and Java lanes do.

`pbt_rng_bounded_big(r, span, out)` is uniform on `[0, span)` for any
`span >= 1` of at most 1000 bits, with the same rules as VER05d §2.1:

```text
span <= 2^64-1        bounded(span)
span == 2^64          next()
span  > 2^64          k-word rejection, k = max(2, ceil(bitlen(span-1)/64)):
                        raw = w1 << 64(k-1) | ... | wk   (k draws, first most significant)
                        t   = (2^(64k) - span) mod span
                        redraw while raw < t;  offset = raw mod span
```

### 2.2 FNV-1a and sub-seeds

`pbt_fnv1a64` (offset `0xcbf29ce484222325`, prime `0x100000001b3`) and
`pbt_sub_seed(run_seed, id)`:

```text
sub_seed(run_seed, id) = SplitMix64( run_seed XOR fnv1a64(utf8(id)) ).next()
```

per VER05 §2.3, as Rust, Python, Go and Java do. Vectors computed with the
independent reference are pinned in the tests.

---

## 3. Integers wider than 64 bits (`pbt_big`)

`unsigned __int128` is not ISO C. The corpus contains `i128` and the rejection
sampler needs `2^(64k)`, so the lane carries a small big integer:

```c
typedef struct { int neg; uint32_t n; uint32_t d[PBT_BIG_LIMBS]; } pbt_big;
```

sign plus magnitude in little-endian `uint32_t` limbs, normalised (no leading
zero limb, zero is `n == 0` and not negative). `PBT_BIG_LIMBS` is 36 (1152
bits). The type is a plain value: copying it by assignment is a deep copy, so
there is no allocation, no ownership and no aliasing hazard.

**Caps.** A value that a caller or a fixture can create has at most
`PBT_BIG_MAX_BITS = 1000` bits of magnitude, and a decimal token at most
`PBT_BIG_MAX_DIGITS = 300` digits. Every operation reports overflow of the limb
array through its return value (nonzero) instead of wrapping; the engine
refuses a generator whose bounds exceed the cap (the constructor returns
`NULL`) and the corpus runner reports an over-wide integer as a problem. The
widest value the engine builds itself is `2^(64k)` with `k <= 16`
(1025 bits), which fits.

Operations: compare, add, subtract, negate, multiply and add a `uint32_t`,
shift left, floor-halve, bit length, full division with remainder (schoolbook
shift-subtract: it is O(bits^2 / 32) and only used for spans above 2^64),
decimal parse and print, and conversion to and from `uint64_t`/`int64_t`.
`pbt_big_floor_half` is `floor(x / 2)` with floor semantics for negative `x`
(`-3 -> -2`), matching Python's `//` and the reference's bisection step.

---

## 4. Values and the Generator Algebra

### 4.1 Tagged values

`pbt_val` is a tagged union: `NIL`, `BOOL`, `INT` (a `pbt_big` stored inline),
`BYTES`, `STR`, `LIST`, `TUPLE`, and `PTR` (an opaque pointer the engine never
follows). `BYTES`/`STR` own a `uint8_t` buffer, `LIST`/`TUPLE` own an array of
child values. `pbt_val_clone` is deep and bounded (`PBT_MAX_VAL_DEPTH = 64`),
`pbt_val_free` frees recursively with the same bound. Every constructor and the
clone check the allocation and the `count * width` product; a failure returns a
nonzero status and leaves the destination a valid, empty, freeable value.

**Property contract.** Properties and predicates receive `const pbt_val *`. They
must not mutate or free it. The shrinkers hand candidates that alias parts of
the working value (a prefix is a pointer and a length, not a copy); the `const`
qualifier is how the C lane replaces the "fresh copy per candidate" of the
garbage-collected lanes. The work budget (§6) still charges every candidate at
its logical size.

### 4.2 Generators

A generator is a heap object with a reference count, a sampler function
pointer, an optional shrinker function pointer, a `void *self` and an optional
destructor. Constructors **borrow** their children (they retain them) and
return `NULL` on invalid arguments or allocation failure; a `NULL` child makes
the combinator return `NULL`, so a chain can be built and checked once.
`pbt_gen_release` drops a reference; the last one runs the destructor.

Sampling returns `0` or a status (`PBT_E_EXHAUSTED`, `PBT_E_NOMEM`,
`PBT_E_GEN`, `PBT_E_LIMIT`): generation can fail on data (a filter that never
accepts), and every combinator propagates it, freeing what it had built.
Sampling depth is bounded (64) by a counter in the run, so a recursive
`flat_map` cannot overflow the C stack. Draw order is the compatibility
contract; `pbt-v1` pins it.

| C | VER05 | Sampling rule |
|---|---|---|
| `pbt_gen_bool()` | `gen_bool` | `bounded(2) == 1`; shrinks `true` to `false` |
| `pbt_gen_u8()` | `gen_u8` | `bounded(256)` |
| `pbt_gen_u64()` | `gen_u64` | `next()` |
| `pbt_gen_i128()` | `gen_i128` | words `w1`, `w2`; `(w1 << 64) \| w2`, signed past 2^127 |
| `pbt_gen_int(min, max)` | `gen_i128(min, max)` | `min + bounded_big(max-min+1)` |
| `pbt_gen_int_i64(min, max)` | — | same draws as `pbt_gen_int` |
| `pbt_gen_bytes(min, max)` | `gen_bytes` | `L = min + bounded(span)`, then `L` draws of `bounded(256)` |
| `pbt_gen_constant(v)` | — | draws nothing; clones `v` |
| `pbt_gen_map(g, f, self)` | `map` | no extra draws; **drops shrinking** (no inverse) |
| `pbt_gen_filter(g, pred, self, n)` | `filter` | up to `n` (default 100, at most 10,000) samples then `PBT_E_EXHAUSTED`; shrinking preserved, candidates failing `pred` rejected |
| `pbt_gen_flat_map(g, f, self)` | — | sample, then sample from the generator `f` returns; no shrinking |
| `pbt_gen_one_of(gens, n)` | `one_of` | `bounded(n)`, then that arm |
| `pbt_gen_frequency(arms, n)` | `frequency` | `bounded(total)`; arms in order; zero-weight arms never selected; total `1..2^32` |
| `pbt_gen_list(elem, min, max)` | `list` | length via `bounded(span)`, then elements in order |
| `pbt_gen_tuple(gens, n)` | — | `n <= 8` generators left to right; shrink coordinate-wise |
| `pbt_gen_custom(...)` | — | a user sampler/shrinker pair |

`pbt_gen_ascii` is not provided (open item 4 of VER05 §7.4, as in every lane).

---

## 5. Integrated Shrinking

All shrinkers take a *falsifies* predicate (true while the value **still
fails**) and return the number of accepted moves. `steps` counts accepted moves
only; the target probe is one move. The user-facing property convention is the
corpus one: `1` holds, `0` fails, a negative value is a property error
(`property-panic`, §7).

1. **Integer walk** (VER05d §4.1): target probe (one step), then the walk:
   bisection candidate `current - floor(diff/2)` when `|diff| > 1`, else (or if
   refused) the unit step `current - sign(diff)`; candidates stay in `[min, max]`.
2. **Sequence shrinking** (bytes and lists, VER05d §4.2): empty/prefix probe;
   bisection (left, then right half) while `len > max(1, min_len)`; single
   deletion right to left; element-wise walk left to right **without** the
   target probe. `steps < max_steps` guards every loop head.
3. **Generator-attached shrinking:** `int` toward the in-range point nearest 0;
   `bytes` and `list` respect `min_len`; `bool` toward `false`; tuples one
   coordinate at a time in repeated passes until a pass makes no progress, under
   one shared attempt counter of `16 * max_steps + 64`. `map`, `flat_map`,
   `one_of` and `frequency` have no shrinker.

Shrinker function pointers have the shape
`int shrink(void *self, pbt_budget *b, pbt_val *value, pbt_pred falsifies, int max_steps, int *steps)`:
`value` is updated **in place** after each accepted move and is valid and
freeable on every return, including an error; `*steps` is **set** (not added to)
to the accepted moves; `b` is the enclosing budget node (§6) that the shrinker
joins. `pbt_check` runs a shrinker on a deep copy of the candidate, so a
shrinker that fails or lies cannot damage the witness.

### 5.1 Memory behaviour specific to C

The sequence shrinker works on the element array **in place and allocates
nothing**:

- a prefix or half candidate is a *view* (a pointer and a length into the
  working array);
- a deletion candidate is made by shifting the tail left over the element,
  evaluating the shorter view, and then either keeping the shift (accepted:
  the removed element is freed) or shifting back (rejected);
- an element candidate temporarily overwrites the element and restores it.

So a property that obeys the `const` contract (§4.1) cannot tell the difference
from a fresh copy, a rejected candidate leaves the working value exactly as it
was (tested byte for byte), and the shrinker has no out-of-memory path of its
own. A 1 MiB `bytes` value is never widened to one big integer per byte, which
would cost 170 bytes per element. The budget (§6) is still charged **before**
every candidate at its logical size, exactly as in the garbage-collected lanes.
Bytes shrinking charges one candidate size per attempt (Java charges two
because it converts through a `List<Integer>`; C has no conversion to charge).

---

## 6. The Shrink Budget Tree

A direct port of VER05e §4.1, simplified by two facts of C: it is
single-threaded (no lock) and there is no ambient scope, so the "current budget"
is an **explicit context parameter passed down** (`pbt_budget *`), never global
state. Re-entrancy is therefore natural: a property that wants to run a nested
`pbt_check` or `pbt_gen_shrink` passes the scope it was handed.

A scope is a node on the caller's stack (`pbt_budget` storage) with a `parent`,
a `root`, two counters (**attempts**, one per property evaluation, cap
`16 * max_steps + 64`; **work**, elements copied, at least 1 per charge, cap
`PBT_MAX_SHRINK_WORK` = 64M) and its own caps.

- `pbt_budget_charge(b, elements)` (an attempt) and
  `pbt_budget_charge_copy(b, elements)` (work only, so attempt counts and the
  corpus are unchanged) walk from the node to the root and check room on
  **every** node, then increment **all**: all or nothing, so a refusal leaves
  every counter unchanged. The room test is `cost > max - used`, which cannot
  wrap (`UINT64_MAX` is refused). The check also refuses once
  `attempts >= max_attempts`, even for a work-only charge. Callers charge
  **before** building a candidate; once refused, every loop ends and a sequence
  shrinker returns its input unchanged.
- The public entry points that run user code (`pbt_check`'s shrink,
  `pbt_gen_shrink`, `pbt_shrink_int`, `pbt_shrink_list_ints`,
  `pbt_shrink_bytes`, `pbt_simulate_replay`) always open a **child** node of the
  scope they are given with a fresh local cap, and charge every property
  evaluation they make, the initial one included. Internal combinators
  (`tuple`, `list`, `filter`, `bytes`) **join** the node they are handed. There
  is no classification of a call as "inside a property" or "inside a shrinker":
  the chain always reaches the outermost root, so the total work of one
  top-level `pbt_check` is capped by that root whatever the nesting. Owning
  nesting is capped at `PBT_BUDGET_MAX_DEPTH = 64`; deeper scopes join.
- `pbt_check` runs its **whole body, case loop included,** in one scope: a root
  when it is given no parent, a child otherwise, with an attempt cap of
  `num_tests + (16 * max_steps + 64)`. A check **nested** in a property (a
  parent was passed) charges one attempt at entry (refused: it returns
  `property-exhausted` without evaluating anything), one per case after the
  first, and the size of every sampled value as work; so neither nesting depth,
  branching, `num_tests`, nor large generated values multiply the outer
  budget. A starved nested check is exhausted, never a pass; a **top-level**
  check has no parent and always evaluates its first case. The single
  re-verification charges one attempt to the check scope (not the possibly spent
  shrink scope); if refused the shrunk value is not adopted and the original
  witness is reported with 0 steps.
- **Filter sampling.** Every retry of a `filter` is charged whenever a budget
  exists: inside a nested check it is one **attempt** up the chain, at a
  top-level check root it is one element of **work** (`charge_copy(1)`), because
  the top-level attempt cap would wrongly exhaust a legitimate rare filter.
  Refused: the sample fails with `PBT_E_EXHAUSTED`, and the root's `refused`
  flag also turns a would-be pass into `property-exhausted`.
  `max_retries` is capped at 10,000; the corpus case (100) is unchanged.
- **Nesting is capped.** Each scope carries an explicit count of enclosing
  checks. A check whose parent already has 64 enclosing checks, or whose parent
  sits so deep that a new scope would merely join it, evaluates nothing, marks
  the root refused and returns `property-exhausted` (so the run can never pass).
- **Starvation is never a pass.** The root records that a charge was refused;
  a top-level check returns `property-exhausted` instead of passed if any charge
  in its tree was refused. A falsified verdict is never overridden.
- Generation work is approximate: a sampled value's work is its collection
  length plus, one level deeper, the length of each child collection or string,
  visiting at most 10,000 children.

`pbt_attempt_budget` clamps a negative step count to zero so it cannot wrap into a
huge cap. `pbt_corpus_evaluate` refuses a corpus of more than 10,000 cases (its
duplicate-id check is pairwise). TFV scratch strings follow one ownership rule:
`keep()` never frees on failure, the caller does.

A scope is opened with `pbt_budget_open_*(storage, parent, ...)` and closed with
`pbt_budget_close(storage)`; both are cheap and allocate nothing. `close` records
the last totals in the test hooks (§9), the analogue of Java's `lastWork`.

The pbt-v1 results are unchanged by the budget: 25/25 with exact minimals and
step counts.

---

## 7. Runner, Verdicts and Failure Vocabulary

`pbt_check(parent, seed, gen, prop, opts, shrink_override, &verdict)` runs
`opts->num_tests` (default 100) cases from `SplitMix64(seed)`. The return value is
`0` when a verdict was produced and `PBT_E_ARG`/`PBT_E_NOMEM` otherwise; in
**every** case the verdict is a failure (`property-exhausted`) unless every case
ran and held, so there is no way to mistake a refused call for a pass.

- `num_tests == 0` → **fail `property-exhausted`** (VER05 §7.1.1).
- generator error (any nonzero sample status, including `PBT_E_NOMEM`) →
  `property-exhausted`.
- property returns a negative value → `property-panic` (C has no exception to
  catch; this is the lane's spelling of "a property that panics").
- property returns `0` → shrink, then `property-counterexample`.
- every case held → pass.

The reported witness is re-verified (VER05 §7.1.2): if the shrinker fails,
returns a value that no longer falsifies, one on which the property reports an
error, or an out-of-range step count, the **original** candidate is reported
with 0 shrink steps. The shrinker runs on a deep copy of the candidate, so a
failed shrinker cannot damage the original. `shrink_override` replaces the
generator's shrinker so a defective shrinker can be tested. While shrinking, a
candidate on which the property errors counts as *not falsifying*.

Closed failure ids: `property-counterexample`, `property-exhausted`,
`property-timeout`, `property-panic`; `pbt_is_failure_id(s)` is true only for
those four strings. `property-timeout` is registered but never produced.

### 7.1 CI redaction

`opts->ci` is `PBT_CI_UNSET` (the zero value), `PBT_CI_ON` or `PBT_CI_OFF`.
**`UNSET` is treated as CI**: the engine fails closed. `pbt_ci_from_value(const
char *)` is a pure helper (the caller passes `getenv("CI")`): anything except
`NULL`, empty, `0` or `false` (trimmed, any case) means CI. When redacting, the
verdict carries **no seed, no counterexample and no minimal value** (`has_seed ==
0`, both values are `nil`) and the message carries no concrete value: only
iteration, shrink steps and `repro-fnv`, the FNV-1a of
`iter:<i>;steps:<s>;shrunk:<canonical value>`. Caveat (VER05 §7.4 #3):
**`repro-fnv` is a correlation id, not a secret**; it is brute-forceable over a
small witness space. A keyed digest is the only real fix and is not implemented.
There is no wall-clock timeout: a property that never returns hangs the run.

### 7.2 Canonical rendering

`pbt_canon(v, &str)` is total and bounded: nesting deeper than 16 renders as
`...`, more than 10,000 nodes truncates with `...`, the output is capped at
1 MiB, control characters, quotes and backslashes in strings are escaped
(`\xNN`, `\"`, `\\`, `\n`...), and a value of an unknown kind renders as
`<unrenderable>`. An out-of-memory in rendering degrades to the constant
`<unrenderable>`, so `pbt_check` always returns a verdict.

---

## 8. Conformance, Strict Parsing and Negative Controls

`pbt_json_parse(text, len, &doc)` is the strict reader: depth at most 64, size
at most 8 MiB, at most 200,000 nodes, duplicate object keys refused, invalid
UTF-8 refused (overlong forms, surrogates, code points above U+10FFFF), leading
zeros and every other malformed number refused, lone surrogate escapes refused,
unescaped control characters refused, trailing data refused. **Numbers are kept
as text** (the corpus carries its integers as decimal strings, so no number is
ever converted to a double). `pbt_tfv_validate` is VER05d's validator: header
`tfv 1`, one space between tokens, lowercase even hex, canonical integers (at
most 1000 digits), counts that agree, no duplicate field, record id or map key,
strict UTF-8 in `S`, depth 64, at most 1 MiB, at most 200,000 tokens.

`pbt_corpus_evaluate(document, &report)` takes the whole parsed document (so a
truncated or reshaped document goes through the same strict path) and evaluates every case with the engine's own
public functions and returns problem strings (none means conform) plus the
evaluated count.

- **Strict decoding:** exactly the fields of the op; ints are canonical decimal
  strings (at most 300 digits in this lane, §12); bytes even-length lowercase
  hex; an unknown op, property or malformed value is a *problem*, never a skip;
  a non-string id is a problem, not a crash. Every problem line starts with its
  case id.
- **CWE-400 caps** as VER05d §6: counts, retries, lengths and list sizes above
  10,000 are problems; `count x max_len <= 2^20` for `gen_bytes`,
  `count x ceil(bitlen/8) <= 2^20` for `gen_bounded`; a `gen_*` case whose
  `outputs` length is not `count` is refused before generating.
- **Bounded reports:** fixture-derived fragments are clipped (200 bytes per
  fragment, 400 per problem) and at most 1000 problems are stored.
- **Recursion depth:** decoding and comparison are capped at 64 levels.
- **Coverage gates:** every op and all four failure ids present, an unregistered
  id rejected, case count equal to `PBT_V1_CASE_COUNT` (25).
- `corpus_reject` texts must be rejected by `pbt_tfv_validate`;
  `filter_exhausted` runs end to end through `pbt_check` and counts predicate
  calls exactly (`max_retries`).

The corpus test loads the committed fixture, asserts zero problems and exactly
25 evaluated, and carries the negative controls. **Per-operation corruption, not
a single case:** for *every one* of the 25 real cases the test deep-copies the
parsed corpus, applies the corruption its op defines (flipped output, wrong
minimal, steps+1, wrong failure id, flipped validity, a valid TFV in place of a
reject text), and asserts that the run fails **and that a problem names that
case's id**, and no other case's id (the PT-F5 lesson: a single-case negative
control proves only one path). Truncated and dropped-case lists and an unknown
op are problems. Engine sabotage through the function-pointer seams of §9 (a
changed unit step, a changed bisection, a skipped deletion phase, a shifted RNG,
a wrong rejection threshold) must make the corpus run report problems.

Because the corpus draws spans small enough that a wrong rejection threshold
almost never fires, the PRNG tests force the rejection branch: the SplitMix64
finalizer is invertible, so a test computes a state whose next output is an
exact chosen value (0, `t - 1`, `t`) and checks that a correct
`bounded(3)` (threshold 1) rejects a draw of 0, that `bounded(2^63 + 1)`
(threshold `2^63 - 1`) rejects `2^63 - 2` and accepts `2^63 - 1`, and that both
bisection halves of the sequence shrinker are taken (a property that fails only
on the right half, and one that fails only on the left).

The independent reference `code/scripts/pbt_conformance.py` must still report 0
problems.

---

## 9. Test Seams, Allocation Discipline and Memory Safety

**Allocation.** Every allocation goes through `pbt_alloc`/`pbt_realloc`/
`pbt_free` (an internal header). Each checks for `NULL`, computes
`count * width` and every list growth with an overflow-checked helper, and
forwards to `malloc`. There are no variable-length arrays, no `alloca`, and no
size taken from untrusted data without a cap. A caller of an engine function
always gets a nonzero status on out-of-memory and a verdict that is not a pass.

**Test seams** (`pbt_hooks.h`): one global structure of function pointers and
counters, reset by `pbt_hooks_reset()`. It carries the sabotage seams of §8
(`bisect`, `unit`, `skip_deletion`, `threshold`, `new_rng`), `default_max_work`
(the analogue of Java's `ShrinkBudget.defaultMaxWork`), `last_work` and
`last_attempts`, and the allocator seams: `live_allocs`, `alloc_count`, `injected`
and `fail_after` (fail the N-th allocation from now on). `pbt_hooks_reset()` restores
every seam but keeps the allocation counters, which are accounting. The OOM test runs a
whole scenario failing allocation 1, 2, 3 ... until none fails, and asserts at
every point: no crash, no leak (`live_allocs == 0` after cleanup), no pass
verdict from a run that lost an allocation. The seams are the only mutable
global state in the package and no production path writes them. They are a plain
mutable global: test-only and **not thread-safe**.

**Sanitizers.** `tools/run.sh` runs the suite under every available compiler in
strict ISO mode, then, when the compiler can link it, once more with
`-fsanitize=address,undefined -fno-sanitize-recover=all`. The harness has no
sanitizer mode, so the extra pass lives in the package's `run.sh`.

---

## 10. Build and Inputs

`BUILD` is `# build-tool: deps=c/iso-harness` plus `sh tools/run.sh`;
`BUILD_windows` runs `tools\run.ps1`. The C lane has no coverage gate today
(no package under `code/packages/c` runs gcov or llvm-cov and CI has no C
coverage step), so none is added to `BUILD`; `tools/coverage.sh` is an
on-demand `llvm-cov` report that the changelog quotes. The package reads
`code/specs/fixtures/pbt-v1/` in its tests, so `c/test-bootstrap` is registered
as a repository-input consumer of that directory in the Go build tool
(`repository_inputs.go`), as the other lanes are. `required_capabilities.json`
is empty; `src` performs no I/O and the tests only read the committed fixture.

## 11. Reference

Pinned vectors and the corpus come from the independent reference
`code/scripts/pbt_conformance.py`; the C lane never trusts its own output to
define expected values.

## 12. Divergences from VER05 / other lanes

| # | Topic | C | Others | Reason |
|---|---|---|---|---|
| 1 | `sub_seed` | `SplitMix64(seed ^ fnv).next()` | TS/Dart omit `next()` | follow VER05 §2.3, Rust, Python, Go, Java |
| 2 | threshold | `(0 - n) % n` on `uint64_t` | Python `((1<<64)-n)%n`, Java `remainderUnsigned` | unsigned wraparound is exact in C; pinned at `2^63+1` |
| 3 | `bounded(1)` | consumes one word | Python raw does not | VER05 §2.2 loop |
| 4 | wide spans | k-word rejection | Dart `raw % span` | VER05 §7.2 |
| 5 | integers | `pbt_big`, 1000 bits, 300 digits | Go/Java: unbounded `BigInteger` | `__int128` is not ISO C; a cap is a CWE-400 control |
| 6 | invalid arguments | constructors return `NULL`; entry points return a status and a failing verdict | Go panics, Java throws | no exception in C; never a pass |
| 7 | property convention | `1` holds, `0` fails, negative is `property-panic` | Go/Java `true` holds, exception is panic | no exception in C |
| 8 | ambient scope | none: explicit `pbt_budget *` parameter | Java `InheritableThreadLocal` | re-entrancy without global state; no threads |
| 9 | locking | none, single-threaded by contract | Java: one lock per tree | §1 |
| 10 | candidate copies | `const` views; deletions shift the tail in place and restore | fresh copy per candidate | `const` replaces the defensive copy; work is still charged |
| 11 | bytes shrink charge | one candidate size per attempt | Java two (list conversion) | no conversion in C |
| 12 | tuples | `pbt_gen_tuple`, arity at most 8 | Go `Tuple2`/`Tuple3` | no generics |
| 13 | CI default | redact unless `PBT_CI_OFF`; `pbt_ci_from_value` pure helper | Rust reads `CI` | capability-free package, fail closed |
| 14 | `corpus_reject` | `pbt_tfv_validate`, a validator only | Python has the full parser | no C TF01 runner yet |
| 15 | recoverable failure | a property reports failure by return value; a crash, `abort`, `exit` or infinite loop in a property is not recoverable | Go/Java recover panics | C has no unwinding; `setjmp` across user frames is not safe |
| 16 | `property-timeout` | registered, never produced | same | VER05 §7.4 #5 |
| 17 | JSON | hand-written strict parser | Go `encoding/json` | zero-dependency rule |
| 18 | string rendering | `\xNN` escapes, `"` and `\` escaped | Go `strconv.Quote`, Java escapes | no cross-lane digest vector is pinned for strings |
| 19 | sabotage hooks | function pointers in `pbt_hooks_t` | Go unexported vars, Java package-private statics | same purpose; tests reset in a final step |
