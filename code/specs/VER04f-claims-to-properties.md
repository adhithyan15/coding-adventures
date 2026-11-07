# VER04f — Warrant Claims to Property-Based Tests (VER-P011)

**Parent:** [VER04 §5–§6](VER04-warrant-language.md), [VER04c](VER04c-guarded-warrant-parser.md), [VER04d](VER04d-warrant-type-system.md), [VER04e](VER04e-warrant-codegen.md), [VER05](VER05-property-engine.md), [TF01](TF01-bootstrap-harness.md).

This specification defines the compilation of Warrant first-order `claim` declarations into generative property-based tests (Level 2) powered by the native `coding_adventures_test_bootstrap::pbt` engine.

It delivers **VER-P011**: compiling Warrant universal claims, parameter refinements, and public host API bindings into executable property tests that automatically detect broken implementations, extract minimal counterexamples via integrated shrinking, and preserve privacy in public CI output.

---

## 1. Architecture and Pipeline Placement

```text
Warrant Source ──► [VER04c: parser] ──► WarrantFile (AST)
                         │
                         ▼
                   [VER04d: checker] ──► CheckedWarrantModule
                         │
                         ▼
                   [VER04d: binding] ──► ValidatedBindings
                         │
                         ▼
                   [VER04f: claims-to-properties]
                         ├──► Generator Derivation from Quantified Types & Refinements
                         ├──► Property Evaluation over Pure Models & Bound Public APIs
                         └──► Registration into test-bootstrap Suite with Integrated Shrinking
```

### 1.1 Separation of Concerns
1. **Ground Examples (VER04e / P008):** `example` declarations are strictly ground (no free variables) and compile to deterministic L0 test vectors in canonical TFV format.
2. **Universal Claims (VER04f / P011):** `claim` declarations contain universally quantified variables (`forall v: T [| P(v)]. body`) and compile to Level 2 property-based tests.
3. **Execution Engine (VER05 / P010):** All property tests execute via `coding_adventures_test_bootstrap::pbt`, using `SplitMix64` PRNG replay, sub-seed isolation, and integrated shrinking.

---

## 2. Claim Syntax and Semantic Structure

### 2.1 Warrant Claim AST
A claim declaration in Warrant syntax:
```warrant
claim equal_reflexive:
  forall a: Bytes. eq(a, a) == true

claim equal_agrees:
  forall a: Bytes, b: Bytes. eq(a, b) == same(a, b)

claim select_length_invariant:
  forall a: Bytes, b: Bytes | len(a) == len(b). select(a, b, true) == a
```

In the checked AST:
- `ClaimDecl.name`: Unique test identifier within the test namespace (`W0201`).
- `ClaimDecl.body`: Must be `Expression::Quantified` with `Quantifier::Forall`.
- `Quantified.binders`: List of typed binders `(name, type, optional refinement)`.
- `Quantified.condition`: Boolean expression asserting invariants, relations, or equivalence between pure models and host operations.

### 2.2 Syntactic & Type Constraints
- **Universal Only:** Top-level claims for generative property testing must be universally quantified (`forall`). Existential quantifiers (`exists`) are verified via SAT/SMT provers (VER09/VER10) or witness synthesis, not PBT.
- **Closed Formula:** The condition expression must contain no free variables other than the quantified binder variables.
- **Type Compatibility:** All binder types must possess corresponding generator mappings defined in §3.

---

## 3. Type-to-Generator Mapping

Each quantified binder `v: T [| P(v)]` is compiled to a `Gen<T>` combinator pipeline:

| Warrant Type $T$ | Native Rust Type | Generator Constructor |
|---|---|---|
| `Bool` | `bool` | `Gen::bool_gen()` |
| `U8` | `u8` | `Gen::u8_gen()` |
| `U64` | `u64` | `Gen::u64_gen()` |
| `Nat` | `u64` | `Gen::bounded(0, default_nat_bound)` (default $0 \le n \le 65536$) |
| `Bytes` | `Vec<u8>` | `Gen::bytes_gen(0, default_bytes_max_len)` (default $0 \le \text{len} \le 64$) |
| `[T; N]` | `Vec<T>` | `list(T_gen, N, N)` |
| `T \| P` (Refinement) | $T$ | `T_gen.filter(\|v\| eval_refinement(P, v), max_retries)` |

### 3.1 Composite Tuple Generators
For multi-variable claims `forall v1: T1, v2: T2, ... vn: Tn`:
1. The compiler synthesizes a composite generator drawing independent sub-samples for each variable from the isolated test `SplitMix64` PRNG.
2. If joint refinements exist (e.g. `| len(a) == len(b)`), the composite generator applies the joint predicate filter:
   $$\text{CompositeGen}.\text{filter}(|(v_1, \dots, v_n)| P(v_1, \dots, v_n), \text{max\_retries})$$
3. If the filter fails to find an admissible tuple within `max_retries` (default 100), the test reports `property-exhausted`.

---

## 4. Property Evaluation and Public API Integration

### 4.1 Evaluation Semantics
For each sampled candidate tuple $(c_1, \dots, c_n)$:
1. **Binding Dispatch:**
   - Pure model function calls (e.g. `same(a, b)`) evaluate directly via `ModelEvaluator`.
   - Bound public host operations (e.g. `eq(a, b)`) invoke the host function via the validated Rust binding (e.g. `ct_compare::eq(a, b)`).
2. **Panic Isolation:**
   - Host invocations are executed inside `std::panic::catch_unwind(AssertUnwindSafe(...))`.
   - Any unhandled panic is caught and formatted as `Verdict::fail("property-panic", ...)`.
3. **Verdict Evaluation:**
   - If the condition evaluates to `true`, the iteration passes.
   - If the condition evaluates to `false`, the property is **falsified**, and integrated shrinking begins.

### 4.2 Integrated Shrinking
When a property is falsified by candidate $(c_1, \dots, c_n)$:
1. The engine minimizes each component preserving joint refinements and type bounds:
   - `Bytes` components shrink via length bisection, chunk removal, and byte-wise reduction (`shrink_bytes`).
   - Integer/Nat components shrink towards 0 or lower bounds via bisection and linear step (`shrink_int`).
   - Element lists shrink via bisection and element minimization (`shrink_list`).
2. Every intermediate shrink candidate is re-tested against:
   - Joint refinements: must remain satisfied.
   - Property condition: must continue to falsify the claim.
3. The engine terminates when no further strictly smaller counterexample can be found or `max_shrink_steps` is exhausted.

---

## 5. Replay, Failure Vocabulary, and Privacy Protocol

### 5.1 Deterministic Sub-Seed Replay
Each claim receives an isolated sub-seed derived from the run seed:
$$\text{sub\_seed}(\text{run\_seed}, \text{"claim/"} \mathbin{\Vert} \text{claim\_name}) = \text{SplitMix64}(\text{run\_seed} \oplus \text{fnv1a64}(\text{"claim/"} \mathbin{\Vert} \text{claim\_name})).\text{next\_u64}()$$
Re-running the suite with `--seed <sub_seed>` executes the identical candidate sequence.

### 5.2 CI Mode Privacy Redaction (VER00 §7.3, VER05 §5.2)
- In CI mode (`CI=true` or `--ci`), public TAP output **redacts raw inputs and seeds**:
  ```yaml
  not ok 1 - claim/ct_compare_eq_agrees
    ---
    failure_id: "property-counterexample"
    message: "property falsified after 14 iterations (shrunk in 6 steps, repro-fnv: 7a9b0c1d2e3f48a1)"
    ...
  ```
- In local development mode (`CI` unset), the minimal witness values are emitted to aid developer debugging.

---

## 6. Conformance and Falsification Requirements

1. **All Valid Claims Pass:** When evaluated against a conforming host implementation, all claims must pass `num_tests` evaluations (default 100).
2. **Fault Injection Detection:** For every claim, deliberate mutation or sabotage of the host implementation must cause the property test to fail with `property-counterexample`.
3. **Strict Coverage:** The claim compilation pipeline and generated harnesses must achieve **$\ge 90\%$ Line Coverage AND $\ge 90\%$ Branch Coverage**.
4. **Zero Dependencies:** Pure Rust standard library implementation without third-party crates.
