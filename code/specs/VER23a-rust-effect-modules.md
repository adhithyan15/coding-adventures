# VER23a — Rust Generated Capability Modules & Proof Certificates

**Parent:** [VER00 §8.6, §8.6.5](VER00-verified-shipping-program.md), [VER23](VER23-generated-capability-modules.md), [VER24](VER24-capability-proof-certificates.md), [VER26 §5 Step 27](VER26-ct-compare-pilot.md).

This specification formalizes **VER-P027b**: the generation of sealed Rust capability modules (`src/capabilities.rs`), canonical effect proof certificates (`effect_certificate.json`), and the standalone small certificate checker.

---

## 1. Objectives and Verification Ladder Placement

```text
  ┌─────────────────────────────────────────────────────────────┐
  │ L4: VER-P027b  Generated Effect Module & Proof Certificate  │ ◄── THIS SPEC
  ├─────────────────────────────────────────────────────────────┤
  │ L4: VER-P027a  Rust Capability Analyzer & Name Allowlist    │
  ├─────────────────────────────────────────────────────────────┤
  │ L3: VER20      Bounds & Constant-Time Trace Proofs (UNSAT)  │
  ├─────────────────────────────────────────────────────────────┤
  │ L2: VER06a     Coverage-Guided Fuzz Engine & Targets        │
  ├─────────────────────────────────────────────────────────────┤
  │ L5: VER07a     Equal-Instruction-Count Microbenchmarks       │
  ├─────────────────────────────────────────────────────────────┤
  │ L0: TF01       Conformance Corpus & Sabotage Runner         │
  └─────────────────────────────────────────────────────────────┘
```

The verified shipping program mandates two complementary guarantees for package capabilities:

1. **Constructive Confinement (VER23 — You cannot name what you did not declare):**
   A package cannot call an undeclared capability because no function or handle exists for it in scope. For every package, a generated capability module (`src/capabilities.rs`) exposes *only* the specific operations declared in `required_capabilities.json`. For pure computation packages (`effects = ∅`), the module exposes zero capability functions, providing compile-time assurance that no ambient side effects can be invoked.

2. **Proof-Carrying Code & Independent Verification (VER24 — Claims carry their own proof):**
   The large capability analyzer produces an effect certificate (`effect_certificate.json`). A small, independent checker verifies this certificate **directly against the package sources and manifest** rather than blindly trusting the certificate or the analyzer summary.

3. **Zero Third-Party Dependencies:**
   All cryptographic hashing (FIPS 180-4 SHA-256), AST analysis, module generation, and certificate verification are implemented using Rust's standard library alone (`#![deny(unsafe_code)]`).

---

## 2. Generated Capability Module Specification (VER23)

### 2.1 File Location and Regeneration Invariant
- Location: `<package_root>/src/capabilities.rs` (or module file designated by crate root).
- Generator header:
  ```rust
  // Generated from required_capabilities.json. Do not edit.
  ```
- Regeneration Check:
  `ca-capability-analyzer --dir <package_root> --check-module` compares the on-disk `src/capabilities.rs` with the deterministic output of the generator. Any drift fails CI with exit code `1`.

### 2.2 Content for Pure Computation Packages (`effects = ∅`)
When `capabilities` is empty (such as `rust/ct-compare`), the module defines:
```rust
// Generated from required_capabilities.json. Do not edit.
// Package: rust/ct-compare
// Declared capabilities: none (pure computation)
// Proof label: proved

#![deny(unsafe_code)]

/// Pure computation witness token.
///
/// This package declares zero capabilities in required_capabilities.json.
/// No I/O, network, filesystem, process, environment, or threading handles
/// are exposed by this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PureCapabilities;
```
Because no functions for filesystem, network, or process execution exist in `capabilities.rs`, any unauthorized call attempt fails at compilation time.

### 2.3 Content for Capability-Bearing Packages
When capabilities are declared (e.g. `stdout:write:*` or `fs:read:*`), the generator emits typed wrapper tokens and methods restricted strictly to the declared category, action, and target pattern.

---

## 3. Canonical Effect Certificate Schema (VER24)

The effect certificate is serialized to `<package_root>/effect_certificate.json` using the canonical schema `ver24-effect-cert-v1`:

```json
{
  "schema_version": "ver24-effect-cert-v1",
  "package": "rust/ct-compare",
  "source_digest": "SHA256_HEX_DIGEST_OF_SORTED_SOURCES",
  "manifest_digest": "SHA256_HEX_DIGEST_OF_REQUIRED_CAPABILITIES_JSON",
  "declared_effects": [],
  "entry_points": [
    {
      "name": "ct_eq",
      "signature": "fn ct_eq(a: &[u8], b: &[u8]) -> bool",
      "effects": []
    },
    {
      "name": "ct_eq_fixed",
      "signature": "fn ct_eq_fixed<const N: usize>(a: &[u8; N], b: &[u8; N]) -> bool",
      "effects": []
    },
    {
      "name": "ct_select_bytes",
      "signature": "fn ct_select_bytes(choice: bool, a: &[u8], b: &[u8]) -> Result<Vec<u8>, CtSelectError>",
      "effects": []
    },
    {
      "name": "ct_eq_u64",
      "signature": "fn ct_eq_u64(a: u64, b: u64) -> bool",
      "effects": []
    }
  ],
  "unconditional_roots": [],
  "banned_construct_exceptions": [
    {
      "construct": "unsafe_code",
      "language": "rust",
      "justification": "Volatile memory read in ct_eq_u64 barrier prevents compiler optimization"
    }
  ],
  "label": "proved"
}
```

### 3.1 Digest Definitions
- `source_digest`: Canonical SHA-256 digest computed across all `.rs` files under `src/` (excluding `capabilities.rs`), sorted lexicographically by forward-slash relative path:
  $$\text{source\_digest} = \text{SHA-256}\left(\bigoplus_{i=1}^n \left(\text{rel\_path}_i \parallel \text{0x00} \parallel \text{bytes}_i\right)\right)$$
- `manifest_digest`: SHA-256 of the raw bytes of `required_capabilities.json`.

### 3.2 Honest Verification Label
Per VER00 §7.6.5:
- `"proved"`: The small checker verified that all public entry points and unconditional roots have been statically proven to invoke zero undeclared capabilities and that the source digests match.
- Never round up to `"proved"` if any check is skipped, unverified, or approximated.

---

## 4. Small Standalone Certificate Checker (VER24)

The small checker is designed to be concise, fail-closed, and readable in its entirety. It enforces the following verification pipeline:

```text
                  ┌───────────────────────────────┐
                  │ Target Package On Disk        │
                  │ (src/*.rs, capabilities.json) │
                  └──────────────┬────────────────┘
                                 │
           ┌─────────────────────┴─────────────────────┐
           ▼                                           ▼
┌───────────────────────┐                   ┌───────────────────────┐
│ Compute Digests       │                   │ Parse AST & Tokens    │
│ (source + manifest)   │                   │ (entry points, roots) │
└──────────┬────────────┘                   └───────────┬───────────┘
           │                                            │
           │           ┌─────────────────────┐          │
           └──────────►│ Small Cert Checker  │◄─────────┘
                       │ (verifies equality, │
                       │  effects = ∅,       │
                       │  closed vocabulary) │
                       └──────────┬──────────┘
                                  │
                                  ▼
                       Pass (0) / Fail (1 or 2)
```

### 4.1 Verification Invariants
1. **Source Digest Match:** The computed `source_digest` must match the certificate's `source_digest` byte-for-byte. Any modification to source files immediately invalidates the certificate.
2. **Manifest Digest Match:** The computed `manifest_digest` must match the certificate's `manifest_digest`.
3. **Declared Effects Agreement:** The `declared_effects` in the certificate must match the capabilities declared in `required_capabilities.json`.
4. **Public Entry Point Exhaustiveness:** Every `pub fn` discovered in the package's public interface (`src/lib.rs`) must be present in `entry_points`.
5. **No Undeclared Effects:** For each entry point, the verified effect set must be a subset of `declared_effects`. For pure packages, the effect set must be strictly empty ($E = \emptyset$).
6. **Unconditional Root Confinement:** All module top-levels, static initializers, and `Drop` implementations must have empty effect sets.
7. **Banned Construct Verification:** Any banned construct present in the source must be explicitly covered by an authorized exception in both `required_capabilities.json` and the certificate.
8. **Label Verification:** If effects are empty and no unexempt constructs exist, the honest label must be `"proved"`.

---

## 5. CLI Interface

`ca-capability-analyzer-rs` extends its CLI with the following flags:

```text
  --generate-module        Emit generated src/capabilities.rs for the target package
  --check-module           Verify on-disk src/capabilities.rs matches generator output
  --emit-cert <FILE>       Generate canonical effect proof certificate to <FILE>
  --verify-cert <FILE>     Run small certificate checker against package sources and <FILE>
```

Exit Codes:
- `0`: Success (checks passed, certificate verified, or modules match).
- `1`: Security violation, certificate mismatch, or uncommitted module drift.
- `2`: Resource quota exceeded, I/O error, or malformed JSON/syntax.
