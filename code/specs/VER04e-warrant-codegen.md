# VER04e — Warrant Code Generation and L0 Test Vector Compiler (P008)

**Parent:** [VER04 §5–§6, §8](VER04-warrant-language.md), [VER04c](VER04c-guarded-warrant-parser.md), [VER04d](VER04d-warrant-type-system.md), [VER26 §3–§5](VER26-ct-compare-pilot.md), [TF01](TF01-bootstrap-harness.md).

This specification defines the deterministic code generation and test vector compilation pipeline for the pure profile of Warrant v0. It operates on the checked AST produced by `checker::check_warrant` and host bindings validated by `binding::validate_bindings`.

It delivers **VER-P008**: compiling Warrant examples, claims, bounds, inputs, and bindings into canonical portable test vectors (TFV) and deterministic native Rust L0 test harnesses running on `coding_adventures_test_bootstrap`.

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
                   [VER04e: codegen]
                         ├──► Canonical TFV Test Records (Portable Test Vectors)
                         └──► Rust L0 Test Harness (Native Suite with Zero External Dependencies)
```

The codegen pipeline performs three core functions:
1. **Pure Model Evaluation:** Evaluates pure model functions and expressions over concrete ground inputs under strict computational budgets (recursion depth $\le 64$, work quota $\le 1,000,000$ steps).
2. **Canonical TFV Vector Compilation:** Emits portable TFV test vectors for each `example` block and ground `claim` instance.
3. **Rust L0 Harness Generation:** Compiles a standalone, reproducible Rust test suite using `coding_adventures_test_bootstrap` that executes bound public operations against model expectations, validates fresh-output ownership, normalizes declared panic-based errors, and runs negative fault-injection controls.

---

## 2. Ground Model Evaluation

### 2.1 Evaluator Environment and Values
The pure evaluator computes values for expressions containing only ground literals, model function calls, and built-in operations:

```text
Value ::= Bool(bool)
        | Int(i128)
        | Nat(u64)
        | U8(u8)
        | U64(u64)
        | Bytes(Vec<u8>)
        | List(Vec<Value>)
        | Option(Option<Box<Value>>)
        | Result(Result<Box<Value>, NominalError>)
```

Where `NominalError` carries the error type name and constructor name (e.g. `SelectError.CT_SELECT_LENGTH_MISMATCH`).

### 2.2 Built-in Operational Semantics
- `len(seq)`: returns `Nat(seq.len())` for `Bytes` and `List`.
- `at(seq, idx)`: returns `Option(Some(item))` if $0 \le idx < len(seq)$, else `Option(None)`.
- `concat(a, b)`: concatenates two `Bytes` or two `List` values of compatible types.
- `fixed<N>(bytes)`: returns `Option(Some(bytes))` if $bytes.len() == N$, else `Option(None)`.
- `div(a, b)`: returns `Option(None)` if $b = 0$, else `Option(Some(floor(a / b)))`.
- `Some(v)`, `None`, `Ok(v)`, `Err(e)`: construct typed wrappers.

### 2.3 Evaluation Budgets and Guards
To guarantee termination and prevent resource exhaustion:
- **Call Depth:** Limited to 64 active stack frames. Exceeding depth aborts with diagnostic `W0703`.
- **Work Quota:** Every arithmetic operation, comparison, indexing, model call, and loop/element traversal charges 1 work unit. Exceeding 1,000,000 units aborts with diagnostic `W0703`.
- **Groundness:** Any non-ground term (e.g. free variables or uninstantiated static parameters) aborts evaluation with diagnostic `W0702`.

---

## 3. Canonical TFV Vector Generation

Each `example` block and concrete ground `claim` instance generates a portable test vector formatted as a TFV record (`Record`):

### 3.1 Record Schema
```text
record:
  id: <module_name>/<declaration_name>
  op: <bound_operation_name>
  <param_name>: <encoded_value>
  ...
  equal: <expected_bool_or_result>
  [output: <expected_output_bytes>]
  [fresh_output: <bool>]
  [error_id: <expected_error_name>]
  byte_positions: <u64>
  byte_reads: <u64>
  byte_writes: <u64>
  u64_bits: <u64>
```

### 3.2 Logical Work Accounting
Logical work fields represent public input shapes (not measured hardware instructions):
- `byte_positions`: Length of equal byte inputs, or 0 on length mismatch / scalar operations.
- `byte_reads`: $2 \times \text{byte\_positions}$ for comparisons.
- `byte_writes`: $\text{byte\_positions}$ for mutating or selecting operations, 0 for pure comparisons.
- `u64_bits`: 64 for 64-bit integer comparisons, 0 otherwise.

---

## 4. Rust L0 Harness Generation

The Rust backend generates self-contained Rust source code adhering to:

### 4.1 Strict Dependency Boundaries
- **Zero third-party dependencies:** Uses only `std` and internal crates `coding_adventures_test_bootstrap` and the target package crate (e.g. `coding_adventures_ct_compare`).
- No external runners (no `vitest`, no external crates).

### 4.2 Real Export Invocation
- Calls the exact bound public symbols declared in `ValidatedBindings` (e.g. `ct_eq`, `ct_eq_fixed`, `ct_select_bytes`, `ct_eq_u64`).
- For const-generic operations (e.g. `fixed<N>`), generates a dedicated `fixed_dispatch` function containing arms for all instantiated widths.
- Never substitutes slice or runtime-width entry points for const generics.

### 4.3 Fresh Output and Non-Aliasing Checks
For operations declared `fresh`:
1. Call export with non-empty input buffers.
2. Record inputs before invocation.
3. Mutate the returned buffer (e.g. invert the first byte `output[0] ^= 0xff`).
4. Assert:
   - Inputs remain identical to pre-call snapshots.
   - Mutated buffer does not alias the memory of either input (`output.as_ptr() != left.as_ptr() && output.as_ptr() != right.as_ptr()`).
   - For empty results, verify non-aliasing only if capacity is non-zero (empty vectors may share dangling sentinels).

### 4.4 Error Normalization and Panic Containment
- Where the model specifies a semantic `Result` error but the native host API signals error via panic (e.g. length mismatch on slice selection), the harness executes the call inside `std::panic::catch_unwind`.
- Panic messages are suppressed to prevent secret leakage or uncontrolled diagnostic pollution.
- Expected panic is normalized to the declared error constructor (e.g. `CT_SELECT_LENGTH_MISMATCH`).
- An unexpected panic on an admitted input yields `Verdict::fail("panic", ...)`.

### 4.5 Mandatory Fault-Injection and Sabotage Controls
The generated harness must include negative control cases that inject faults and prove the test harness detects broken behavior:
- `wrong-equality`: Inverts comparison results; must fail baseline comparison.
- `wrong-selection`: Inverts selection boolean; must fail output byte comparison.
- `missing-error`: Replaces error path with empty slice; must fail missing-error check.
- `wrong-work`: Corrupts logical work fields; must fail work validation.

---

## 5. Determinism and Checked Regeneration

1. **Deterministic Byte Output:** Code generation is a pure function of `(CheckedWarrantModule, ValidatedBindings)`. Identical inputs produce byte-for-byte identical output files.
2. **Provenance Header:** Emitted files include a canonical header:
   ```rust
   // GENERATED BY warrant-grammar codegen (VER04e). DO NOT EDIT MANUALLY.
   // Source: <spec_path>
   // Target: Rust L0 test harness
   ```
3. **Drift Detection:** CLI or test asserts that committed generated test files match newly generated bytes exactly.

---

## 6. Diagnostic Error Codes

| ID | Phase / Description |
|---|---|
| `W0701` | Unsupported codegen target or uninstantiated static generic width |
| `W0702` | Non-ground term in compile-time model evaluation |
| `W0703` | Evaluator resource exhaustion (depth > 64 or work quota > 1,000,000) |
| `W0704` | Missing or incompatible host binding for operation |
| `W0705` | Duplicate or conflicting test case identifier |

---

## 7. Pilot Document: `ct-compare.warrant`

The pilot document `code/specs/ct-compare.warrant` provides the canonical specification for `ct-compare`:
- Operations: `eq`, `eq_fixed`, `select_bytes`, `eq_u64`.
- Pure model `same(a: Bytes, b: Bytes) -> Bool = a == b`.
- Explicit public/secret labels for all parameters.
- Platform and environment assumptions (`none`).
- Bounded input contracts with hostile admission and `SelectError.CT_SELECT_LENGTH_MISMATCH`.
- Example suites covering equal, unequal, length mismatch, fixed widths 0 and 16, and single-bit sweeps.
