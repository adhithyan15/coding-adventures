# VER15 — Weakness Registry and Per-Package Weakness Matrix

**Parent:** [VER00 §8.2](VER00-verified-shipping-program.md), [VER26 §5 Step 28](VER26-ct-compare-pilot.md).

This specification formalizes **VER15**: the repository-wide weakness registry (`code/specs/data/weakness-registry-v1.json`) and the rules governing per-package weakness matrix construction and closure.

---

## 1. Objectives and Principles

Vulnerabilities in shipped software are not random. The CVEs published daily cluster into recurring structural weaknesses catalogued by MITRE as the Common Weakness Enumeration (CWE). The verified shipping program replaces ad-hoc security scanning with an exhaustive in-repo **weakness registry**.

The registry establishes three non-negotiable invariants:

1. **Exhaustive Matrix Accounting:**
   Every shipped package's assurance report carries a **weakness matrix** containing exactly one row per registry class. No registry row may be omitted or left unaddressed.
2. **Deterministic Row Closure:**
   Each row must be closed in exactly one of three ways:
   - `excluded`: The class cannot occur in the language or package configuration, and the capability analyzer proves that the code remains strictly inside the safe subset (e.g. safe Rust with zero `unsafe`, zero FFI, zero ambient authority).
   - `discharged`: The class is applicable, and the package provides an attack corpus, targeted fuzzing, and formal proof meeting the minimum proof label floor.
   - `exemption`: A computed exemption when evidence is below the required label floor or depends on unverified hardware layers. Exemptions cannot be declared by package authors; they are derived mechanically.
3. **Fail-Closed Gate:**
   Any row left open blocks release authorization. An unapproved exemption forces the assurance verdict to `report-only`.

---

## 2. Weakness Registry Catalog (v1)

The canonical weakness registry data file is located at `code/specs/data/weakness-registry-v1.json`. It defines 26 canonical classes across 6 primary security families:

### 2.1 Family 1: Memory Safety
Applies to C, C++, Rust `unsafe`, and FFI boundaries. Excluded by construction in safe Rust, Go (with race-freedom), and garbage-collected runtimes.

| CWE | Name | Description | Floor | Exclusion Rationale |
|---|---|---|---|---|
| **CWE-416** | Use After Free | Pointer dereferenced after memory release | `proved` | Safe Rust ownership/borrow model prevents manual deallocation; verified by analyzer (`no unsafe/ffi`). |
| **CWE-787** | Out-of-bounds Write | Write operation past buffer bounds | `proved` | Safe Rust bounds checks on indexing and slice operations; verified by analyzer. |
| **CWE-125** | Out-of-bounds Read | Read operation past buffer bounds | `proved` | Safe Rust bounds checks on indexing and slice operations; verified by analyzer. |
| **CWE-415** | Double Free | Free called twice on same allocation | `proved` | Automatic single-ownership drop semantics; verified by analyzer. |
| **CWE-476** | NULL Pointer Dereference | Dereferencing null pointer | `proved` | Non-nullable references (`&T`, `&mut T`) and checked `Option<T>`; verified by analyzer. |
| **CWE-190** | Integer Overflow to Buffer Bounds | Arithmetic wraparound causing buffer miscalculation | `proved` | Overflow checks in debug/release or explicitly bounded math with checked indexing. |

### 2.2 Family 2: Injection
Applies to packages with ambient process, shell, filesystem, database, or network capabilities.

| CWE | Name | Description | Floor | Exclusion Rationale |
|---|---|---|---|---|
| **CWE-78** | OS Command Injection | Untrusted input in shell/process execution | `proved` | Package lacks `proc` capability; verified by analyzer. |
| **CWE-89** | SQL Injection | Untrusted input concatenated into SQL queries | `proved` | Package lacks database or SQL engine capabilities; verified by analyzer. |
| **CWE-22** | Path Traversal | Unauthorized directory climbing (`../`) | `proved` | Package lacks `fs` capability; verified by analyzer. |
| **CWE-79** | Cross-Site Scripting (XSS) | Unsanitized untrusted input in HTML/DOM context | `proved` | Package lacks `ui` capability and DOM output sinks; verified by analyzer. |
| **CWE-94** | Code Injection / Eval | Dynamic code evaluation (`eval`, dynamic compilation) | `proved` | Language lacks runtime eval; verified by analyzer banned-construct check. |
| **CWE-116** | Improper Output Encoding | Unescaped control characters in delimited protocols | `proved` | Package produces no serialized output protocols or enforces type-safe serializers. |
| **CWE-117** | Log Injection | Newline/delimiter injection into log streams | `proved` | Package lacks `stdout` or log sink capabilities; verified by analyzer. |

### 2.3 Family 3: Parsing, Algorithmic Complexity, and Resource Exhaustion (DoS)
Applies to input parsers, decoders, and data processing loops.

| CWE | Name | Description | Floor | Evidence Required |
|---|---|---|---|---|
| **CWE-400** | Uncontrolled Resource Consumption | Memory, file descriptor, or CPU exhaustion | `bounded(k)` | Step bounds (`steps ≤ c·n + d`), zero-allocation proofs, or bounded input limits (`rejects`). |
| **CWE-409** | Decompression Bomb | Exponential output expansion in compressed inputs | `bounded(k)` | Maximum expansion ratios enforced before decoding. |
| **CWE-674** | Uncontrolled Recursion | Stack exhaustion via nested input recursion | `proved` | Iterative algorithms with bounded work stacks or recursion depth limits. |
| **CWE-1333** | Inefficient Regular Expression (ReDoS) | Exponential backtracking on adversarial strings | `proved` | Linear-time automata engines (DFA/Thompson NFA) or zero regex dependencies. |
| **CWE-407** | Inefficient Algorithmic Complexity | Hash flooding or quadratic lookup degradation | `bounded(k)` | Universal hashing with secret seeds or verified balanced search structures. |

### 2.4 Family 4: Cryptography and Information Leakage
Applies to security-sensitive data manipulation, comparison, and cryptographic primitives.

| CWE | Name | Description | Floor | Evidence Required |
|---|---|---|---|---|
| **CWE-208** | Observable Timing Discrepancy | Execution time or cache pattern reveals secret | `proved-to-gate-level` | Constant-time trace proof (self-composition), equal instruction counts, microarchitectural bounds. |
| **CWE-330** | Insufficiently Random Values | Weak PRNG seeds in cryptographic operations | `proved` | Package uses verified CSPRNG with `rand` capability; excluded if pure computation. |
| **CWE-327** | Broken or Risky Cryptography | Obsolete cipher or digest algorithms | `proved` | Verified modern primitives (ChaCha20, SHA-256) per specification. |
| **CWE-326** | Inadequate Encryption Strength | Insufficient key or block lengths | `proved` | Minimum parameter sizes enforced by compile-time type refinements. |

### 2.5 Family 5: Concurrency
Applies to multithreaded packages and shared mutable state.

| CWE | Name | Description | Floor | Exclusion Rationale |
|---|---|---|---|---|
| **CWE-362** | Concurrent Execution Race Condition | Unsynchronized access to shared state | `proved` | Package lacks `thread` capability; verified by analyzer. |
| **CWE-367** | Time-of-check Time-of-use (TOCTOU) | State mutation between validation and action | `proved` | Package lacks ambient `fs`/`proc` capabilities; verified by analyzer. |

### 2.6 Family 6: Deserialization and Object Mutation
Applies to object deserializers, dynamic typing, and reflection.

| CWE | Name | Description | Floor | Exclusion Rationale |
|---|---|---|---|---|
| **CWE-502** | Deserialization of Untrusted Data | Unsafe object instantiation during deserialization | `proved` | Strictly bounded, typed deserialization without class loader/reflection dispatch. |
| **CWE-1321** | Prototype Pollution | Mutation of object prototypes in dynamic runtimes | `proved` | Strongly typed language with no prototype chain; verified by analyzer. |

---

## 3. Schema of `weakness-registry-v1.json`

```json
{
  "version": 1,
  "last_updated": "2026-11-08",
  "classes": [
    {
      "cwe": 208,
      "name": "Observable Timing Discrepancy",
      "family": "cryptography",
      "description": "Differences in execution time, memory access patterns, or microarchitectural state vary with secret data, allowing secret extraction via timing side channels.",
      "applies_to": ["cryptography", "constant_time_comparison", "secret_handling"],
      "safe_subsets": [],
      "exclusion_rationale": null,
      "attack_patterns": [
        "Early return on byte mismatch in secret comparison",
        "Secret-dependent memory indexing causing cache line hits/misses",
        "Secret-dependent branching with observable pipeline stalls"
      ],
      "required_evidence": ["attack-corpus", "microbenchmark-instructions", "trace-self-composition-proof"],
      "minimum_proof_label": "proved-to-gate-level"
    }
  ]
}
```

---

## 4. Evaluation Rules for Per-Package Weakness Matrix

For any package $P$:

1. **Completeness:**
   $|\text{Matrix}(P)| = |\text{Registry}|$. Exactly 26 entries must be present.
2. **Exclusion Validation:**
   A row may be marked `excluded` only if:
   - The registry class defines `safe_subsets` including $P$'s language and configuration; and
   - The capability analyzer verifies that $P$ stays strictly inside the safe subset (e.g. no `unsafe`, no `ffi`, no undeclared capabilities).
3. **Discharge Validation:**
   A row may be marked `discharged` only if:
   - Concrete evidence artifacts exist for each item in `required_evidence`; and
   - The associated formal proof label equals or exceeds `minimum_proof_label`.
4. **Exemption Computation:**
   If a row is neither legitimately `excluded` nor fully `discharged`:
   - It is recorded as an `exemption`.
   - The exemption records the exact deficit (e.g. WIR-level proof exists, but hardware gate-level proof floor is not reached).
   - An exemption is `approved: false` until a cryptographic, hardware-token-backed signature is recorded per VER00 §9.1.
5. **No Open Rows:**
   Any missing or unclassified row triggers immediate validation abort with exit code `2`.
