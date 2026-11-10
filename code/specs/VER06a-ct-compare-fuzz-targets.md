# VER06a — Constant-Time Comparison Fuzz Targets

**Parent:** [VER00 §7.3](VER00-verified-shipping-program.md), [VER06](VER06-coverage-guided-fuzz-engine.md), [VER26 §3](VER26-ct-compare-pilot.md) (E4 Fuzz).

This specification defines the coverage-guided fuzz targets for `code/packages/rust/ct-compare`, delivering **VER-P015**. It integrates the Level 2 Rust Fuzz Engine (`coding_adventures_test_bootstrap::fuzz`) with the real `ct-compare` public API, verifying no-panic guarantees, functional agreement against reference models, length-mismatch contracts, input minimization, and privacy-preserving failure reporting.

---

## 1. Objectives and Invariants

The fuzz targets establish empirical robustness of `ct-compare` under hostile, pseudo-randomly mutated byte streams.

### 1.1 Invariants Checked

1. **Hostile Safety (No Uncontrolled Panics)**:
   For all arbitrary byte streams $(a, b) \in \Sigma^* \times \Sigma^*$, `ct_eq(a, b)` and `ct_eq_u64(u_a, u_b)` must never panic, crash, or exceed memory/step limits.
2. **Functional Agreement (Equivalence Oracle)**:
   $$\forall a, b \in \Sigma^* : \text{ct\_eq}(a, b) \iff (a == b)$$
   $$\forall a, b \in \Sigma^N : \text{ct\_eq\_fixed}(a, b) \iff (a == b)$$
   $$\forall a, b \in \mathbb{U}_{64} : \text{ct\_eq\_u64}(a, b) \iff (a == b)$$
3. **Symmetry & Reflexivity**:
   $$\text{ct\_eq}(a, b) = \text{ct\_eq}(b, a), \quad \text{ct\_eq}(a, a) = \text{true}$$
4. **Length-Mismatch Contract**:
   $$\text{len}(a) \neq \text{len}(b) \implies \text{ct\_eq}(a, b) = \text{false}$$
5. **Branchless Selection Contract**:
   For $\text{len}(a) = \text{len}(b)$:
   $$\text{ct\_select\_bytes}(a, b, \text{true}) = a, \quad \text{ct\_select\_bytes}(a, b, \text{false}) = b$$
   The returned buffer must be an independent fresh allocation.
   For $\text{len}(a) \neq \text{len}(b)$:
   Calling `ct_select_bytes` must catch the precondition and report the expected error (`CT_SELECT_LENGTH_MISMATCH` or panic with exact message `"ct_select_bytes requires equal-length slices"`).
6. **Privacy-Preserving Crash Reporting (VER00 §7.3)**:
   Any failure must be deduplicated via normalized FNV-1a stack hashing and emitted as an opaque TAP line containing zero raw payload bytes.

---

## 2. Input Decomposition & Packing

Because fuzz engines mutate a single continuous byte stream $I \in [0, 255]^*$, multi-argument targets require deterministic input splitting.

### 2.1 Two-Slice Packing
For target functions accepting two byte slices $(a, b)$:
1. If $\text{len}(I) = 0$: $a = \emptyset, b = \emptyset$.
2. If $\text{len}(I) = 1$: $a = I[0..0], b = I[0..1]$ (exercises length mismatch $0 \neq 1$).
3. If $\text{len}(I) \ge 2$:
   - Let $\text{split} = I[0] \bmod (\text{len}(I) + 1)$.
   - Slice payload $P = I[1..]$.
   - Let $s = \text{split} \bmod (\text{len}(P) + 1)$.
   - $a = P[..s], \quad b = P[s..]$.

This decomposition naturally generates:
- Equal-length identical slices ($s = \text{len}(P)/2$ and payload duplicated)
- Equal-length differing slices
- Divergent-length slices with varying prefixes and suffixes
- Empty vs non-empty slices

### 2.2 Slices With Boolean Choice Packing
For `ct_select_bytes(a, b, choice)`:
1. If $\text{len}(I) < 2$: use defaults $(a=\emptyset, b=\emptyset, \text{choice}=\text{false})$.
2. Choice bit: $\text{choice} = (I[0] \land 1) \neq 0$.
3. Split index: $\text{split} = I[1]$ mapped across remaining payload $I[2..]$.
4. Slices $a$ and $b$ derived from $I[2..]$.
5. To specifically target the equal-length requirement:
   - When $I[1] \land 1 = 0$, construct equal-length slices by splitting $I[2..]$ into two halves of length $L = \text{len}(I[2..])/2$.

### 2.3 64-Bit Integer Packing
For `ct_eq_u64`:
- Read two 8-byte integers in little-endian format from $I$.
- If $\text{len}(I) < 16$, pad with zeros.

---

## 3. Coverage Bitmap Instrumentation

The harness instruments logical and branch edges into the 65,536-bit `CoverageBitmap`:

1. **Length match vs mismatch edge**:
   $$\text{edge}_0 = 0 \quad (\text{equal lengths}), \quad \text{edge}_1 = 1 + (\text{len}(a) \oplus \text{len}(b)) \bmod 256$$
2. **Length buckets**:
   $$\text{edge}_{\text{len}} = 300 + \min(64, \text{len}(a))$$
3. **Prefix match depth**:
   For equal-length slices, count matching prefix bytes $k$:
   $$\text{edge}_{\text{prefix}} = 1000 + \min(256, k)$$
4. **Outcome edge**:
   $$\text{edge}_{\text{result}} = 2000 + (\text{result as usize})$$
5. **Selection choice edge**:
   $$\text{edge}_{\text{choice}} = 3000 + (\text{choice as usize})$$

This coverage structure guides the mutator to explore equal lengths, off-by-one mismatches, varying prefix alignments, and differing byte values.

---

## 4. Negative Falsification & Sabotage Testing

To verify that the fuzz harness is live and capable of detecting defects:
1. **Sabotaged Equality (Early Exit)**:
   An injected faulty equality implementation returns early on the first differing byte:
   $$\text{faulty\_eq}(a, b) = \text{zip}(a, b).\text{all}(|(x, y)| x == y)$$
   The fuzz engine must detect disagreement with standard model `==`, record a crash/failure outcome, minimize the counterexample, and emit an opaque signature.
2. **Sabotaged Selection (Inverted Choice)**:
   An injected faulty selection swaps `choice` outcomes. The harness must catch the defect and deduplicate the failure.

---

## 5. Harness Execution & Conformance

The fuzz targets run within `coding_adventures_ct_compare` tests as a dedicated non-libtest binary:
```rust
cargo test --package coding_adventures_ct_compare --test fuzz_targets
```
Validates:
- 100% agreement across all campaigns
- Minimum iteration count budget met
- Coverage bitmap expansion verified
- Negative sabotages caught and minimized
- Zero third-party dependencies (pure standard library)
