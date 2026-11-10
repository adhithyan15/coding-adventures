# VER05 — Property-Based Testing Engine and Conformance Protocol

**Parent:** [VER00 §7.2, §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md), [VER04](VER04-warrant-language.md), [VER04e](VER04e-warrant-codegen.md), [VER26 §3](VER26-ct-compare-pilot.md).

This specification defines the architecture, generator algebra, integrated shrinking protocol, deterministic replay mechanics, privacy disclosure rules, and the `pbt-v1` conformance corpus for the verified property-based testing engine (Level 2 in the VER testing hierarchy).

It delivers **VER-P009**: formal specification and portable conformance corpus (`pbt-v1`) establishing uniform, reproducible property testing across all language implementations with zero third-party dependencies.

---

## 1. Architecture and Testing Pyramid Placement

```text
  ┌─────────────────────────────────────────────────────────────┐
  │ L5: VER01 / VER28  Assurance Report & Ship Gate             │
  ├─────────────────────────────────────────────────────────────┤
  │ L4: VER08 / VER22-24 Capability Analysis & Sandboxing       │
  ├─────────────────────────────────────────────────────────────┤
  │ L3: VER09-11 / 19-20 Prover Kernel, SMT, Timing Proofs     │
  ├─────────────────────────────────────────────────────────────┤
  │ L2: VER05-06 / P009-015 Property Engine & Fuzzing (L2)      │ ◄── THIS SPEC
  ├─────────────────────────────────────────────────────────────┤
  │ L1: VER04 / P005-008 Warrant Compiler -> Tests (L1)         │
  ├─────────────────────────────────────────────────────────────┤
  │ L0: TF01 / P003-004 test-bootstrap Harness & TFV v1 (L0)    │
  └─────────────────────────────────────────────────────────────┘
```

The property engine is positioned at **L2** (runtime generative testing). It bridges formal model claims ([VER04 §6](VER04-warrant-language.md)) and executable test harnesses ([TF01](TF01-bootstrap-harness.md)):

```text
Warrant Claims ──► [VER05 Generator Algebra] ──► Candidate Draw
                            │                          │
                            ▼                          ▼
                  Integrated Shrinker         Implementation Test
                            │                          │
                            ▼                          ▼
                  Minimal Counterexample     Pass / Falsification
                            │                          │
                            └──────────┬───────────────┘
                                       │
                                       ▼
                       Deterministic Replay & TAP v14
```

---

## 2. Deterministic Randomness & Seeding Protocol

All generation and shrinking decisions derive strictly from the PRNG specified in **TF01 §5.1**:

### 2.1 PRNG State Machine (SplitMix64)
- State: 64-bit unsigned integer $s \in [0, 2^{64}-1]$.
- Step:
  $$\begin{aligned}
  s' &= (s + \text{0x9e3779b97f4a7c15}) \bmod 2^{64} \\
  z_1 &= (s' \oplus (s' \gg 30)) \times \text{0xbf58476d1ce4e5b9} \bmod 2^{64} \\
  z_2 &= (z_1 \oplus (z_1 \gg 27)) \times \text{0x94d049bb133111eb} \bmod 2^{64} \\
  \text{output} &= z_2 \oplus (z_2 \gg 31)
  \end{aligned}$$
- Pinned reference vector: Seed `0` produces first output `0xe220a8397b1dcdaf`.

### 2.2 Unbiased Bounded Draws
To sample integers uniformly in $[0, n-1]$ for $1 \le n \le 2^{32}$, implementations must use rejection sampling over the threshold $t = (2^{64} - n) \bmod n$ (TF01 §5.1):
```text
threshold = (-n) % n
loop:
    x = next_u64()
    if x >= threshold:
        return x % n
```

### 2.3 Sub-Seed Derivation
Properties within a test suite run with isolated, deterministically derived sub-seeds:
$$\text{sub\_seed}(\text{run\_seed}, \text{property\_id}) = \text{SplitMix64}(\text{run\_seed} \oplus \text{fnv1a64}(\text{property\_id})).\text{next\_u64}()$$
This guarantees that:
1. Reordering or adding unrelated tests does not change the candidate sequence for an unchanged test ID when running under a fixed sub-seed.
2. Every failure can be re-run in isolation with `--seed <sub_seed>`.

---

## 3. Generator Algebra

Generators construct typed values from random draws while enforcing invariants and bounds.

### 3.1 Primitive Generators

| Generator | Signature | Sampling Rule |
|---|---|---|
| `gen_bool` | `() -> bool` | `bounded(2) == 1` |
| `gen_u8` | `() -> u8` | `bounded(256) as u8` |
| `gen_u64` | `() -> u64` | `next_u64()` |
| `gen_i128` | `(min: i128, max: i128) -> i128` | Uniform draw over $[min, max]$ using rejection sampling |
| `gen_nat` | `(max: u64) -> u64` | `bounded(max + 1)` |
| `gen_bytes` | `(min_len: usize, max_len: usize) -> Vec<u8>` | Draw length $L \in [min\_len, max\_len]$; draw $L$ bytes |
| `gen_ascii` | `(min_len: usize, max_len: usize) -> String` | Draw length $L$; draw printable ASCII chars ($32 \le c \le 126$) |

### 3.2 Combinators

1. **`map(gen, f)`**: Maps generated value $v$ to $f(v)$.
2. **`filter(gen, pred, max_retries)`**: Re-samples candidate up to `max_retries` (default: 100). If no candidate satisfies `pred`, generation aborts with `property-exhausted`.
3. **`one_of([gen_0, gen_1, ...])`**: Uniform selection among a list of generators.
4. **`frequency([(w_0, gen_0), (w_1, gen_1), ...])`**: Weighted random selection proportional to positive integer weights $w_i$.
5. **`list(min_len, max_len, elem_gen)`**: Generates a vector of items produced by `elem_gen`.

### 3.3 Warrant Refinement Derivation
Warrant types and refinements compile directly into bounded generators:
- `x: Nat | x < 1024` $\implies$ `gen_nat(1023)`
- `b: Bytes | len(b) == 32` $\implies$ `gen_bytes(32, 32)`
- `x: Int | x >= -10 and x <= 50` $\implies$ `gen_i128(-10, 50)`
- `p: T | predicate(p)` $\implies$ `filter(gen_T, predicate, 100)`

---

## 4. Integrated Shrinking Protocol

Unlike traditional external shrinkers where shrunk values may violate type invariants or preconditions, **VER05 mandates integrated shrinking** (Hypothesis-style). Shrinking operations are bounded within the generator's definition, guaranteeing all refinements hold during every shrink step.

### 4.1 Canonical Ordering and Well-Founded Metric
Shrinking searches for a **canonical minimal counterexample** that strictly minimizes the complexity metric $M(c)$:
$$M(c) = (\text{structural\_length}(c), \sum_{i} \text{magnitude}(c_i))$$
A shrink candidate $c'$ is strictly smaller than $c$ ($c' \prec c$) if:
1. $M(c') < M(c)$ in lexicographical ordering, and
2. $c'$ satisfies all type refinements and preconditions, and
3. $c'$ still falsifies the target property.

### 4.2 Integer Shrinking Rules
For an integer $x \in [min, max]$ with shrink target $T \in [min, max]$ (default $T = 0$, or $T = min$ if $0 < min$, or $T = max$ if $0 > max$):
1. **Target direct test**: Test $T$. If it falsifies, immediate minimal shrink.
2. **Binary bisection**: Test midpoint $x' = x - \lfloor(x - T) / 2\rfloor$. If it falsifies, replace $x \leftarrow x'$ and repeat.
3. **Linear step**: If bisection cannot progress, test $x' = x - \text{sign}(x - T)$.

### 4.3 Sequence Shrinking Rules (Bytes and Lists)
For a sequence $S$ of length $L \in [min\_len, max\_len]$:
1. **Phase 1 — Length Minimization**:
   - Try minimal allowed length $min\_len$ by slicing $S[0..min\_len]$.
   - Try bisection: drop left half, drop right half.
   - Chunk removal: for chunk sizes $k \in \{2^p \mid 2^p \le L\}$ descending to 1, try removing chunk $S[i..i+k]$.
2. **Phase 2 — Element-wise Minimization**:
   - Iterate through remaining elements $e_i$ from left to right.
   - Shrink $e_i$ using its element type's shrinking rules while holding all other elements constant.
   - Accept the shrunk element if the full sequence continues to satisfy refinements and falsify the property.

### 4.4 Shrinking Budgets and Termination
- **Step Limit:** Maximum 1,000 shrink iterations per failure.
- **Cycle Prevention:** Shrinking candidates must strictly decrease $M(c)$. Non-decreasing candidates are discarded.
- If the step limit is reached, the engine terminates and returns the best counterexample found so far.

---

## 5. Replay, Failure Vocabulary, and Privacy Protocol

### 5.1 Closed Failure Vocabulary
Extending the TF01 failure schema (§6), property testing registers four closed failure IDs:

| Failure ID | Meaning |
|---|---|
| `property-counterexample` | Property was falsified by a candidate input; minimal witness identified. |
| `property-exhausted` | Filter precondition rejected too many candidates (`max_retries` exceeded). |
| `property-timeout` | Evaluation exceeded step limit or timeout budget. |
| `property-panic` | An unexpected runtime panic/exception escaped the property evaluation closure. |

### 5.2 Disclosure and Privacy Policy (VER00 §7.2, §7.3)
> [!CAUTION]
> **Exploit Prevention Rule (VER00 §7.3):**
> A deterministic PRNG seed combined with a public property specification produces an exact, working reproducer. In security-sensitive code (cryptography, parsers, allocators), counterexamples often represent exploitable vulnerabilities.

- **Local Development Mode (`--verbose` / default local):**
  The harness emits:
  - The exact reproducing `seed`.
  - The number of `shrink_steps`.
  - The minimal failing input `counterexample`.
- **CI / Hosted Mode (`CI=true` / public TAP output):**
  - Raw seeds and concrete counterexample values are **redacted** from public logs.
  - Public TAP v14 output emits an opaque `failure_id` and a SHA-256 content hash of the reproducer bundle:
    ```yaml
    not ok 1 - prop/ct_compare_eq_reflexive
      ---
      failure_id: property-counterexample
      message: "property falsified after 38 iterations"
      property:
        shrink_steps: 12
        reproducer_digest: "sha256-e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
      ...
    ```
  - The full reproducer bundle (`seed`, inputs, stack trace) is written to a secured, non-public artifact store outside public repository logs.

---

## 6. The `pbt-v1` Conformance Corpus Specification

The `pbt-v1` corpus is the standard cross-language validation suite for property engines. Stored in `code/specs/fixtures/pbt-v1/cases.json` and compiled to `cases.tfv`, it pins the behavior of every lane.

### 6.1 Schema Version 1
```json
{
  "schema_version": 1,
  "cases": [
    {
      "id": "case-identifier",
      "op": "operation_name",
      ...
    }
  ]
}
```

### 6.2 Operations Covered

| Operation | Required Fields | Verification Target |
|---|---|---|
| `gen_primitive` | `seed`, `type`, `count`, `outputs` | Deterministic outputs of primitive generators (`bool`, `u8`, `u64`, `i128`). |
| `gen_bounded` | `seed`, `min`, `max`, `count`, `outputs` | Unbiased bounded draws over range $[min, max]$. |
| `gen_bytes` | `seed`, `min_len`, `max_len`, `count`, `outputs` | Bounded byte sequence generation. |
| `shrink_int` | `initial`, `target`, `min`, `max`, `predicate`, `expected_minimal`, `expected_steps` | Integer shrinking to exact minimal witness. |
| `shrink_list` | `initial`, `predicate`, `expected_minimal`, `expected_steps` | Sequence length and element-wise reduction. |
| `shrink_refinement` | `initial`, `min_bound`, `predicate`, `expected_minimal` | Integrated shrinking ensuring values never cross refinement bounds. |
| `replay_consistency` | `seed`, `property_id`, `initial_fail`, `shrunk_minimal` | End-to-end replay reproducing identical initial and shrunk witnesses. |
| `filter_exhausted` | `seed`, `predicate`, `max_retries`, `verdict`, `failure_id` | Detection of unsatisfiable preconditions emitting `property-exhausted`. |

### 6.3 Conformance Requirements
1. Every language property engine must pass all cases in `cases.tfv`.
2. An independent Python validator (`code/scripts/pbt_conformance.py`) re-derives every expected value from first principles.
3. Falsification tests (`code/scripts/tests/test_pbt_conformance.py`) verify that deliberate corruptions of every case category are rejected.
4. Line and branch coverage for the conformance reference and native test suites must be **≥ 90%**.

---

## 7. Audit of the Rust lane: testable contracts and divergences

The tests are `code/packages/rust/test-bootstrap/tests/pbt_properties.rs`
(37 tests) and the pbt-v1 corpus run (`pbt_corpus.rs`, `pbt_conformance.py`).

### 7.1 Verdict honesty (must hold for every lane)

1. A property that evaluated **zero** cases (`num_tests == 0`) is a failure
   (`property-exhausted`), never a pass: a property that never ran proves
   nothing.
2. The reported minimal witness is **re-verified**: the engine calls the
   property on the shrinker's result and reports it only if the property still
   fails. If the shrinker returns a passing value, panics, or hits a property
   that panics on smaller input, the engine reports the *original* falsifying
   candidate with zero shrink steps. The verdict is `property-counterexample`
   in every case; the witness is always a real counterexample.
3. A generator error is `property-exhausted`; a panic in the property is
   `property-panic`; any falsification is `property-counterexample`. No path
   returns `Pass` other than "every case evaluated and held".
4. In CI mode the message carries no concrete value (`initial=`, `minimal=`).

### 7.2 Generators

- `gen_bounded_int(min, max)` is uniform over the **inclusive** range for every
  `min <= max` including the full `i128` range. The span is computed modulo
  2^128 (a computed span of 0 means the full range). Spans up to 2^64 use the
  conformance-pinned draws of §2.2; wider spans use 128-bit rejection sampling
  with `t = (2^128 - span) mod span`, as §3.1 requires ("rejection
  sampling"). (The first Rust implementation used `raw % span` for spans above
  2^64, which drew the low third of a `3 * 2^126` span with probability 1/2
  instead of 1/3; fixed.)
- Independent draws of two `Bytes` values **almost never collide**: for fixed
  length 16 the probability is 2^-128 (0 hits in 200,000 pairs measured); for
  lengths `0..=8` it is about 1.2%, nearly all of it the pair
  (empty, empty). Therefore a property whose interesting branch is "the two
  inputs are equal, or differ only in the last byte" is **not exercised** by
  independent generators. Measured consequence: an equality property
  `buggy_eq(a, b) == (a == b)` over independent 16-byte pairs passes for 1000
  cases against a comparison that ignores the last byte; a correlated
  generator (half copies, half copies with the last byte flipped, built from
  `one_of` and `map`) finds the bug. A conforming engine SHOULD provide a
  paired generator `gen_bytes_pair(min_len, max_len, p_equal, p_near)`; until
  it does, equality claims must be generated with an explicit correlated
  generator. The harness cannot detect a vacuous property: a property
  `a == b` compared with itself passes by construction.
- `frequency` never selects a zero-weight arm and follows its weights;
  `one_of` is uniform; `filter` samples exactly `max_retries` times before
  `property-exhausted`; `list` lengths are inclusive. `frequency` requires a
  total weight of at most 2^32 (it uses the bounded draw).

### 7.3 Shrinking: what is actually guaranteed

§4.1 speaks of a "canonical minimal counterexample". The implementation
guarantees a **local minimum**, which is the contract:

- Integers: the result falsifies the property, is inside `[min, max]`, and is
  never farther from the target than the start. For a **monotone** property
  (`x < t` holds) the result is exactly the threshold `t` from every failing
  start (checked for every `t` in 1..=120 and every start, both signs). For an
  arbitrary failing set the result is a local minimum (neither the bisection
  step nor the unit step falsifies); it is not claimed global.
- Lists and bytes: the result falsifies, is no longer than the start, keeps
  every element inside its original value range, and is **1-minimal by
  deletion**: removing any single element makes the property hold. Checked over
  every list of length 0..=4 over `{0,1,2,3}` (341 lists) for six property
  shapes. Element values are shrunk one at a time, so a property such as "no
  two adjacent equal elements" shrinks `[1,2,3,3,4,5]` to `[3,3]`, not the
  global minimum `[0,0]`.
- Budgets: `max_steps == 0` returns the start unchanged; any budget returns a
  falsifying value. `shrink_int`/`shrink_list` panic on a start that does not
  falsify (a caller bug, loud by design); `check_property` guards that case.
- Extremes: `i128::MIN`/`MAX` starts and opposite-extreme targets no longer
  overflow (saturating difference, unsigned absolute value).

### 7.4 Divergences found in the audit

| # | Spec | Rust lane | Resolution |
|---|---|---|---|
| 1 | §3.1 `gen_i128(min, max)` rejection sampling | `raw % span` above 2^64; overflow above 2^127 | Fixed (7.2) |
| 2 | §5.1 / §4.4 witness is a counterexample | shrinker result was trusted; `num_tests == 0` passed | Fixed (7.1) |
| 3 | §5.2 SHA-256 digest of the reproducer bundle | CI message carries `repro-fnv`, a 64-bit FNV-1a of `iter;steps;shrunk` and also prints `iteration` and `steps`; the digest preimage space is tiny, so it is **brute-forceable**, which a SHA-256 of the same low-entropy string would also be | Open caveat: a keyed digest (HMAC with a private key) is the only real fix; not implemented. CI output must still be treated as potentially revealing for small input domains |
| 4 | §3.1 lists `gen_nat`, `gen_ascii` | Not implemented (`Gen::bounded` covers `gen_nat`) | Open: spec lists them, nothing consumes them |
| 5 | §5.1 `property-timeout` | Registered but never produced; `PbtOptions.max_retries` and `max_shrink_steps` are not read by `check_property` (budgets live in the caller's shrinker and filters) | Open: documented, harmless |
| 6 | §4.3 chunk removal by powers of two | Bisection then single-element deletion | Equivalent guarantee (1-minimal), recorded |
