# VER06 — Coverage-Guided Fuzz Engine and Conformance Protocol

**Parent:** [VER00 §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md), [VER04](VER04-warrant-language.md), [VER05](VER05-property-engine.md), [VER26 §3](VER26-ct-compare-pilot.md).

This specification defines the architecture, mutational algebra, corpus management, coverage feedback interface, crash deduplication, private failure disclosure policy, and the `fuzz-v1` conformance corpus for the verified coverage-guided fuzz engine (Level 2 in the VER testing hierarchy).

It delivers **VER-P013**: formal specification and portable conformance corpus (`fuzz-v1`) establishing uniform, reproducible, zero-third-party-dependency fuzz testing across all language implementations.

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

The fuzz engine sits at **Level 2** alongside the [VER05](VER05-property-engine.md) property engine. Where property-based testing explores the input space through structured generative schemas and algebraic shrinking, the fuzz engine drives byte-level and grammar-aware mutational exploration guided by program execution feedback:

```text
Seed Corpus ──► [Mutational Engine] ──► Target Execution (Simulated Host)
     ▲                  │                           │
     │                  ▼                           ▼
     │            Candidate Buffer          Execution Outcome:
     │                                      • Pass / No new coverage
     │                                      • New Coverage Edge
     │                                      • Crash / Panic / Timeout / OOM
     │                                              │
     ├────── [Coverage Admitted] ◄──────────────────┤
     │                                              ▼
     └────────────────────────────────── [Crash Deduplicated & Minimized]
                                                    │
                                                    ▼
                                     Private Store & Opaque TAP v14
```

---

## 2. Deterministic Seeding and PRNG Protocol

All mutation scheduling, byte selections, and splicing choices derive strictly from the SplitMix64 PRNG specified in **TF01 §5.1** and **VER05 §2.1**:

### 2.1 PRNG State Machine
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
To sample integers uniformly in $[0, n-1]$ for $1 \le n \le 2^{32}$, implementations use rejection sampling over the threshold $t = (2^{64} - n) \bmod n$ (TF01 §5.1, VER05 §2.2).

### 2.3 Sub-Seed Derivation for Fuzz Targets
Each fuzz target within a test suite runs with an isolated sub-seed derived deterministically from the run seed and target identifier:
$$\text{sub\_seed}(\text{run\_seed}, \text{target\_id}) = \text{SplitMix64}(\text{run\_seed} \oplus \text{fnv1a64}(\text{target\_id})).\text{next\_u64}()$$

This guarantees that:
1. Reordering or adding unrelated targets does not change the mutation sequence of an existing target when run under a fixed sub-seed.
2. Every discovered finding is 100% reproducible across runs and platforms using `--seed <sub_seed>`.

---

## 3. Mutational Operators Algebra

The fuzz engine defines a deterministic, closed algebra of primitive mutators operating on byte slices.

### 3.1 Single & Multi-Bit Flips
- `bit_flip_1(bytes, bit_index)`: Invert bit at index $\text{bit\_index} \in [0, 8 \cdot \text{len} - 1]$.
  $$\text{byte\_idx} = \lfloor\text{bit\_index} / 8\rfloor, \quad \text{bit\_pos} = \text{bit\_index} \bmod 8$$
  $$\text{bytes}[\text{byte\_idx}] \gets \text{bytes}[\text{byte\_idx}] \oplus (1 \ll \text{bit\_pos})$$
- `bit_flip_2(bytes, bit_index)`: Invert 2 consecutive bits modulo the total bit length.
- `bit_flip_4(bytes, bit_index)`: Invert 4 consecutive bits modulo the total bit length.

### 3.2 Byte Inversions
- `byte_flip_1(bytes, byte_index)`: Invert byte at $\text{byte\_index} \in [0, \text{len} - 1]$ via XOR `0xFF`.
- `byte_flip_2(bytes, byte_index)`: Invert 2 consecutive bytes (wrapping at buffer boundary).
- `byte_flip_4(bytes, byte_index)`: Invert 4 consecutive bytes (wrapping at buffer boundary).

### 3.3 Byte Arithmetic
- `byte_arithmetic(bytes, byte_index, delta, width, is_big_endian)`:
  - Width $W \in \{1, 2, 4, 8\}$ bytes.
  - Delta $D \in \{-35, -8, -1, 1, 8, 35\}$.
  - Reads integer of width $W$, adds $D$ modulo $2^{8W}$, and writes back in specified endianness.

### 3.4 Boundary Integer Splicing
Injects critical edge-case values at `byte_index`:
- 8-bit interest values: `0`, `1`, `127` (`0x7F`), `128` (`0x80`), `255` (`0xFF`).
- 16-bit interest values: `0`, `1`, `255`, `256`, `32767` (`0x7FFF`), `32768` (`0x8000`), `65535` (`0xFFFF`).
- 32-bit interest values: `0`, `1`, `65535`, `65536`, `0x7FFFFFFF`, `0x80000000`, `0xFFFFFFFF`.
- 64-bit interest values: `0`, `1`, `0xFFFFFFFF`, `0x100000000`, `0x7FFFFFFFFFFFFFFF`, `0x8000000000000000`, `0xFFFFFFFFFFFFFFFF`.

### 3.5 Block Manipulations
- `block_delete(bytes, offset, length)`: Removes $\text{length}$ bytes starting at $\text{offset}$, shrinking buffer. Buffer is never shrunk below minimum length (default: 0).
- `block_insert(bytes, offset, insert_bytes)`: Inserts $\text{insert\_bytes}$ at $\text{offset}$, expanding buffer up to maximum length budget (default: 65,536 bytes).
- `block_duplicate(bytes, src_offset, length, dst_offset)`: Copies $\text{length}$ bytes from $\text{src\_offset}$ and inserts them at $\text{dst\_offset}$.

### 3.6 Splicing
- `splice(target, donor, target_split, donor_split)`: Constructs a composite buffer taking $\text{target}[0..\text{target\_split}]$ concatenated with $\text{donor}[\text{donor\_split}..]$.

---

## 4. Corpus Management and Coverage Feedback

### 4.1 Coverage Bitmap
Coverage feedback is tracked via an 8,192-byte (65,536-bit) shared coverage bitmap $B$.
Each bit represents an execution edge or branch point encountered during target execution:
$$B[i] = 1 \iff \text{edge } i \text{ executed}$$

### 4.2 Admission Rule
An execution input $I$ is admitted to the seed corpus $\mathcal{C}$ if and only if its execution trace $T(I)$ discovers at least one previously unseen edge:
$$\text{admit}(I) \iff \exists i \in [0, 65535] : (T(I)[i] = 1) \land (B_{\text{cumulative}}[i] = 0)$$

Upon admission:
1. $B_{\text{cumulative}} \gets B_{\text{cumulative}} \lor T(I)$.
2. $I$ is added to $\mathcal{C}$ tagged with its byte length, edge discovery count, and parent generation.

### 4.3 Power Schedule (Energy Allocation)
Seeds in the corpus receive mutation budgets (energy) proportional to:
$$\text{energy}(S) = \text{base\_energy} \times \left(1 + \frac{\text{new\_edges}(S)}{\text{total\_edges}}\right) \times \frac{1}{\sqrt{\text{len}(S)}}$$
Favors shorter inputs that discover high proportions of new coverage.

---

## 5. Crash Classification and Deduplication

### 5.1 Execution Outcomes
Every fuzz target invocation yields one of four outcomes:
- `Pass`: Execution completed normally within time and memory budgets.
- `Crash(signature)`: Unhandled panic, exception, signal (`SIGSEGV`, `SIGBUS`, `SIGFPE`), or assertion failure.
- `Timeout`: Execution exceeded wall-clock / instruction step budget.
- `OOM`: Execution exceeded memory allocation limit.

### 5.2 Crash Signature Deduplication
To prevent flooding reports with identical defects, crashes are deduplicated by their normalized crash signature:
$$\text{sig\_hash} = \text{fnv1a64}(\text{error\_kind} \mathbin{:} \text{frame}_1 \mathbin{:} \text{frame}_2 \mathbin{:} \dots) \bmod 2^{32}, \text{ printed as 8 lowercase hex digits}$$
(amended in the audit, §9.4 item 6, to the form the fuzz-v1 corpus pins; frames
are joined by `:` without escaping)
Crashes with matching `sig_hash` increment the hit counter for that signature but do not trigger duplicate failure reports.

### 5.3 Minimization (Shrinking Crashing Inputs)
When a novel crash is encountered, the fuzz engine invokes an automated minimization pass:
1. Try block deletion (halving chunk sizes) while verifying the minimized input still triggers the exact same `sig_hash`.
2. Try byte zeroing / normalization on non-essential payload regions.
3. Record the minimal crashing input in private storage.

---

## 6. Security and Privacy Disclosure Policy (VER00 §7.3)

In accordance with repository policy, **a crash or counterexample is never published before its fix**:
1. **Public Logs (CI & TAP v14)**:
   - Must **NEVER** print raw payload bytes, hex dumps, ASCII snippets, or seeds.
   - Public TAP v14 output emits only an opaque, closed failure token:
     `not ok <N> - <target_id> [fuzz-crash:<hex8_sig_hash>]`
2. **Closed Failure Vocabulary**:
   Only five failure IDs are permitted:
   - `fuzz-crash` (unhandled panic, assertion, memory violation)
   - `fuzz-timeout` (instruction budget or timeout exceeded)
   - `fuzz-oom` (memory allocation ceiling exceeded)
   - `fuzz-leak` (resource / memory leak detected)
   - `fuzz-panic` (explicit language panic)
3. **Private Artifact Storage**:
   The full crashing input, reproduction command, and stack trace are written strictly to private local artifact storage (`/tmp` or out-of-repo path) and excluded from git commits and public CI artifacts.

---

## 7. Dependency-Free Coverage Source Inventory

Every language lane implements coverage collection without external packages or nightly toolchains:

| Language | Coverage Mechanism | Dependency Status |
|---|---|---|
| **Rust** | Stable `-C instrument-coverage` in-process read, or compiled AST branch counters | Zero crates |
| **Go** | Native `-cover` block counters | Go standard toolchain |
| **TypeScript / JS** | V8 precise coverage via `node:inspector` | Node.js built-in |
| **Python** | `sys.monitoring` (Python 3.12+) or `sys.settrace` branch recorder | Python standard library |
| **Ruby** | Built-in `Coverage` module (`Coverage.start(lines: true, branches: true)`) | Ruby standard library |
| **Elixir / Erlang** | BEAM `:cover` module | Erlang/OTP built-in |
| **Self-Compiled Languages** | In-repo compiler emits branch counter increments directly | In-repo compilers |

When running in an environment where no coverage instrumentation is accessible, the engine operates in **black-box generational mode**, and the assurance report explicitly marks the coverage tier as `black-box-uninstrumented`.

---

## 8. The `fuzz-v1` Conformance Corpus

The conformance corpus (`code/specs/fixtures/fuzz-v1/cases.json` and `cases.tfv`) establishes 25 canonical test vectors verifying:
1. `fuzz/mutate_bit_flip/*`: Bit flip operations produce exact bit-level inversions.
2. `fuzz/mutate_byte_flip/*`: Byte flip operations match byte XOR `0xFF`.
3. `fuzz/mutate_arithmetic/*`: Endian-aware integer additions wrap correctly.
4. `fuzz/mutate_boundary/*`: Boundary integers are correctly embedded.
5. `fuzz/mutate_block/*`: Block deletions, insertions, and duplicates maintain bounds.
6. `fuzz/mutate_splice/*`: Splicing preserves target prefix and donor suffix.
7. `fuzz/mutate_sequence/*`: Multi-step sequential mutation is deterministic for fixed seeds.
8. `fuzz/corpus_admit/*`: Novel coverage triggers admission; duplicate coverage is rejected.
9. `fuzz/crash_dedup/*`: Identical crash signatures deduplicate; distinct signatures diverge.
10. `fuzz/failure_id/*`: Failure vocabulary rejects unregistered IDs.
11. `fuzz/private_reporting/*`: Failure formatting produces opaque tokens without raw bytes.
12. `fuzz/corpus_reject/*`: Malformed corpus entries and corrupt syntax fail closed.

---

## 9. Audit of the Rust lane: testable contracts and divergences

Tests: `code/packages/rust/test-bootstrap/tests/fuzz_properties.rs` (31 tests)
and the fuzz-v1 corpus run (`fuzz_corpus.rs`, `fuzz_conformance.py`).

### 9.1 Campaign contracts

1. **Determinism.** `fuzz_target` is a pure function of (initial corpus,
   options, target): the same seed gives the same iterations, admitted seeds,
   edge count and crash records. Different seeds give different campaigns.
2. **Budget.** With a live corpus, `iterations == max_iterations` exactly;
   no mutated candidate exceeds `max_input_len` (initial seeds are run as
   given). An empty initial corpus is replaced by one `[0]` seed. Seeds that
   crash, time out or run out of memory are never mutated: a corpus of
   crashers fuzzes nothing (`iterations == 0`).
3. **Honest report.** Every execution, triage included, lands somewhere:
   `crashes` (deduplicated), `timeouts` (count), `ooms` (count), or the
   corpus. `FuzzReport::is_clean()` is true only if at least one mutation
   iteration ran **and** there was no crash, timeout or OOM. A campaign with
   `max_iterations == 0`, or one that never got past triage, is never clean.
   (The first Rust implementation dropped timeouts and OOMs silently, so a
   target that timed out on every input produced the same report as a clean
   one.)
4. **Dedup and minimization.** One `CrashRecord` per signature, for initial
   seeds as well as mutants; `hits` counts every campaign execution with that
   signature; a *novel* crash is minimized (§5.3) before it is stored. The
   minimizer keeps the exact signature, can reach the empty input, and never
   trades one crash for another.
5. **Feedback.** Coverage guidance is observable: on a three-stage staircase
   target the engine cracks the crash for 5 of 6 seeds in 6000 iterations with
   per-stage coverage and 1 of 6 with none (blind mutation).

### 9.2 Mutator contracts

For every byte string of length 0..=2 (65,793 inputs): `bit_flip` and
`byte_flip` preserve length and are involutions for offsets
`{0, 1, 9, usize::MAX}` and counts `{0, 1, 4, 17, usize::MAX}`. Their windows
wrap around the buffer and apply by **parity**, so the cost is O(len) for any
`count` (`bit_flip(&[0,0], 0, usize::MAX) == [0x00, 0x80]`; the previous
`count`-iteration loop never terminated). `byte_arithmetic` is undone by the
negated delta for widths 1 and 2 in both endiannesses, wraps modulo 2^(8w),
and returns its input unchanged for unsupported widths or a too-short buffer.
Block operations obey exact length arithmetic and `splice` is
`target[..ts] ++ donor[ds..]` with both splits clamped.

### 9.3 Private reporting

`format_tap_failure` reduces every argument to its grammar (ids
`[A-Za-z0-9._/-]`, hash lowercase hex; anything else becomes `_`), so no
argument can add a TAP line. Well-formed arguments are unchanged:
`not ok <N> - <target> [<failure-id>:<hex8>]`.

### 9.4 Divergences found in the audit

| # | Spec | Rust lane | Resolution |
|---|---|---|---|
| 1 | §5.3 a novel crash is minimized | `minimize_crashing_input` existed but was never called; one-byte inputs were never tried for deletion | Fixed |
| 2 | §5.1 four outcomes reported | Timeout and OOM silently dropped | Fixed (counts + `is_clean`) |
| 3 | §5.2 report each signature once | Crashing initial seeds produced duplicate records | Fixed |
| 4 | §3.1-3.2 flips take any count | `count = usize::MAX` hung | Fixed (parity) |
| 5 | §6.1 opaque token | newline in an id forged a TAP line | Fixed (sanitised) |
| 6 | §5.2 `sig_hash = fnv1a64(normalize(frame) ... error_kind)` | Low 32 bits of `fnv1a64(kind ":" frames joined by ":")`. Corpus-pinned, so the **spec is amended to the implementation**. Known limit: frames are not escaped, so one frame `x:y` and two frames `x`, `y` deduplicate as one crash; 32 bits collide near 65k distinct signatures | Spec amended; limit documented |
| 7 | §3.4 boundary values of 16/32/64 bits; §3.1-3.3 widths 2/4/8, flips of 2 and 4 | The mutators exist and are corpus-pinned, but `mutate_sequence` (and therefore `fuzz_target`) only ever uses 1-bit/1-byte flips, width-1 arithmetic and the 8-bit interest list | Open: the engine is weaker than the algebra it specifies |
| 8 | §6.3 crashing input goes to private storage | `CrashRecord.minimal_input` holds raw bytes in memory; nothing is written anywhere. Callers must not print it; TAP goes only through `format_tap_failure` | Open: no artifact store exists |
| 9 | §6.2 five failure ids | `fuzz-leak` and `fuzz-panic` are registered but the engine cannot produce them (`FuzzOutcome` has four variants) | Open: recorded |
| 10 | §7 coverage source | The engine takes a caller-supplied bitmap; there is no instrumentation, so every "coverage-guided" claim is only as good as the target's own edge reporting (`black-box-uninstrumented` when it records none) | Recorded |
