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
