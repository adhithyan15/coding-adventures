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
   A package cannot call an undeclared capability because no function or handle exists for it in scope. For every package, a generated capability module (`src/capabilities.rs`) exposes *only* the specific operations declared in `required_capabilities.json`. For pure computation packages (`effects = ∅`), the module exposes zero capability functions, but see the caveat below: the module is a witness, not a compiler-enforced barrier.

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
**Caveat (divergence V1):** nothing in `capabilities.rs` prevents other code from naming `std::fs` directly; the Rust compiler does not enforce "you cannot name what you did not declare". The file only carries witness tokens and an up-to-date-with-manifest check (`--check-module`). The actual enforcement is the textual analysis of VER08a, which also scans `capabilities.rs` itself. `#![deny(unsafe_code)]` in the generated file is a lint on that module only and can be overridden by an `#[allow]` in the same file, which is why the file is scanned.

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
      "signature": "pub fn ct_eq(a: &[u8], b: &[u8]) -> bool",
      "effects": []
    },
    {
      "name": "ct_eq_fixed",
      "signature": "pub fn ct_eq_fixed<const N: usize>(a: &[u8; N], b: &[u8; N]) -> bool",
      "effects": []
    },
    {
      "name": "ct_select_bytes",
      "signature": "pub fn ct_select_bytes(choice: bool, a: &[u8], b: &[u8]) -> Result<Vec<u8>, CtSelectError>",
      "effects": []
    },
    {
      "name": "ct_eq_u64",
      "signature": "pub fn ct_eq_u64(a: u64, b: u64) -> bool",
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

### 3.1 Digest Definitions (exact; what the hash covers and does not cover)

- `source_digest` = SHA-256 over the following byte stream, files sorted by forward-slash path **relative to `src/`** (bytewise):

  ```text
  for each file:  u64_be(len(path)) || path_utf8 || u64_be(len(content)) || content
  ```

  `content` is the file's bytes with every `\r\n` replaced by `\n` (lone `\r` kept). Length framing (not the earlier `path || 0x00 || bytes` sketch) makes the stream injective: no file can masquerade as another's path or contents.
  - **Covered:** every `.rs` file under `src/`, except exactly `src/capabilities.rs`. Hidden directories are skipped. A symlink anywhere under `src/` is an error (never silently skipped).
  - **Not covered:** `Cargo.toml`, `build.rs`, any `.rs` outside `src/`, non-`.rs` files (`include_str!` data), `src/capabilities.rs` (bound instead by the `--check-module` equality with the generator, which `--verify-cert` runs first), and any file named `capabilities.rs` below `src/` (these *are* covered; only the top-level generated file is exempt).
  - A change to a file the digest does not cover cannot be detected by the digest. `--verify-cert` therefore *also* re-runs the full analysis (§4.1 item 9), which covers everything in VER08a §5, so a capability or banned-construct regression anywhere in the package still fails.
- `manifest_digest` = SHA-256 of the bytes of `required_capabilities.json` with `\r\n` normalised to `\n`; of the empty string when the manifest is absent.

### 3.2 Honest Verification Label

Per VER00 §7.6.5:
- `"proved"`: the checker verified, *directly against the files on disk*, that the whole package (not per entry point) uses no capability outside `declared_effects`, uses no unexcepted banned construct, contains no `CAP003` construct, and that every certificate field equals the value recomputed from disk.
- `generate_certificate` **refuses** (error, exit `2`, no file written) when the analysis is not clean. A certificate is never emitted for a package the analyzer rejects. (Previous revision: the label was hard-coded and no analysis ran at emit time.)
- Never round up to `"proved"` if any check is skipped, unverified or approximated. The label is a *package-level lexical* claim and inherits every limit in VER08a §8; in particular it says nothing about third-party dependencies, which are not analysed.

### 3.3 Entry points, effects, roots (precise meaning)

- `entry_points`: every `pub fn` (optionally `const`/`unsafe`/`async`/`extern "…"`) found *lexically* in `src/lib.rs`, in source order, **including** those inside `impl` blocks and inline `mod` blocks there (named by bare identifier; duplicates are allowed and compared positionally). `pub(crate)`/`pub(super)`/`pub(in …)` functions are not entry points. **Not enumerated:** functions in other files reached via `pub mod x;`, `pub use` re-exports, macro-generated functions. This is a lexical inventory, not the crate's real public API. The inventory (and `source_digest`, which covers `src/**/*.rs`) describes the library only if `src/lib.rs` **is** the library root, so both `--emit-cert` and `--verify-cert` refuse a package whose `Cargo.toml` sets `[lib] path` to anything other than `src/lib.rs` (`./src/lib.rs` is the same path); a target root that is not a `.rs` file is `CAP003` in the analysis itself (VER08a §3.5, D41).
- `signature`: the source text from `pub` up to (excluding) the first `{` or `;` at parenthesis/bracket depth 0, whitespace collapsed to single spaces. (Previous revision truncated at the first `;` anywhere, so `[u8; N]` parameters were cut short; the committed `ct_eq_fixed` signature was wrong.)
- `effects`: the analyzer does **not** attribute effects to individual entry points (no call graph). The sound over-approximation is recorded: every entry point's `effects` equals `declared_effects`. The checker requires exactly that; an entry claiming fewer effects than the package declares is rejected as an under-approximation. For pure packages this is `[]`.
- `unconditional_roots`: always `[]`. Static initialisers, `Drop` impls and module top-levels are covered because the analysis is whole-package; the checker rejects any non-empty value because nothing computes one.
- `banned_construct_exceptions`: must equal the manifest's exceptions *including justification text*, sorted by construct.
- **Excepted constructs are visible, not absent.** A certificate for a package with an `unsafe_code` exception records the exception, and `--verify-cert` prints it; the analyzer's report (VER08a §6.1) states how many `unsafe` occurrences exist. "proved" with an `unsafe_code` exception does **not** mean "no unsafe".

---

## 4. Small Standalone Certificate Checker (VER24)

The checker is fail-closed and works by *recomputation*: it rebuilds the expected certificate from the package on disk and requires the parsed file to equal it field for field.

```text
   certificate.json ──parse (strict: no dup keys, no unknown keys, exact types)──┐
                                                                                 ▼
   package on disk ──check-module, digests, manifest, entry points, analysis──► expected cert
                                                                                 │
                                                           equal? ── yes ──► PASS (0)
                                                                     no  ──► FAIL (1) / error (2)
```

### 4.1 Verification Invariants

1. **Strict parse:** valid JSON, no duplicate keys, only the nine schema fields, correct types, `schema_version == "ver24-effect-cert-v1"`. Whitespace and key order are semantically neutral (a re-indented or reordered but equal certificate verifies).
2. **Module drift:** `src/capabilities.rs` equals the generator output for the manifest.
3. **Manifest digest** equals the recomputed value.
4. **Source digest** equals the recomputed value (§3.1).
5. **Package** equals the manifest `package` (or `rust/<dirname>` when no manifest).
6. **Declared effects** equal the manifest's `category:action:target` strings, in manifest order.
7. **Entry points** equal the lexical inventory of `src/lib.rs` positionally (name and signature), each with `effects == declared_effects`.
8. **Roots** empty; **banned exceptions** equal manifest (§3.3).
9. **Whole-package analysis** is clean (no `CAP001`, unexcepted `CAP002`, or `CAP003`).
10. **Label** is `"proved"`.

Any single-field alteration of a valid certificate fails at least one of these (tamper-tested field by field).

---

## 5. CLI Interface

`ca-capability-analyzer-rs` extends its CLI with the following flags:

```text
  --generate-module        Emit generated src/capabilities.rs for the target package
  --check-module           Verify on-disk src/capabilities.rs matches generator output
  --emit-cert <FILE>       Generate canonical effect proof certificate to <FILE>
  --verify-cert <FILE>     Run small certificate checker against package sources and <FILE>
```

`--generate-module` refuses (exit `2`) when `src` or `src/capabilities.rs` is a symbolic link, checked with `symlink_metadata` before writing: following it would overwrite a file outside the package. The module is written to a new temporary file in `src/` (`.capabilities.rs.tmpN`, opened with `create_new`, so an existing name - a symlink included - is never followed) and `rename`d over `src/capabilities.rs`. A rename replaces the directory entry, so a `src/capabilities.rs` that is a *hard link* to a file outside the package leaves that file untouched (`fs::write` truncated it in place). On a write failure the temporary file is left behind and named in the error: the analyzer declares no `fs:delete`. The name is not `.rs`, so it is never scanned or digested.

Exit Codes:
- `0`: Success (checks passed, certificate verified, or modules match).
- `1`: Security violation, certificate mismatch, or uncommitted module drift.
- `2`: Resource quota exceeded, I/O error, or malformed JSON/syntax.

## 6. Divergence Log (previous revision -> this revision)

| # | Previous | Now | Why |
|---|---|---|---|
| V1 | spec claimed compile-time confinement | spec states it is a witness only | the claim was false |
| V2 | digest sketched as `path || 0x00 || bytes`; code used u64 length framing and CRLF normalisation | spec matches code (length framing) | spec was wrong; framing is the safer choice |
| V3 | manifest digest "raw bytes" | CRLF-normalised | code behaviour, cross-platform stability |
| V4 | `generate_certificate` hard-coded `proved`, never ran analysis | refuses unless analysis clean | a certificate for a violating package must not exist |
| V5 | signature truncated at first `;` | bracket-aware | cert contained `pub fn ct_eq_fixed<const N: usize>(a: &[u8` |
| V6 | entry `effects` always `[]` | `== declared_effects` | `[]` on a capability-bearing package was an unsupported purity claim |
| V7 | checker ignored `unconditional_roots`, exception justifications, unknown fields, duplicate JSON keys; matched entry points by name (first wins) | recomputation + strict parse + positional compare | tamper evidence for every field |
| V8 | any nested `capabilities.rs` excluded from digest | only `src/capabilities.rs` | nested file was an unbound hole |
| V9 | symlinks silently skipped by digest walker, refused by scanner | digest walker errors too | consistency |
| V10 | cert JSON writer escaped only `"` | full JSON escaping | backslash/newline in justification produced invalid JSON |
| V11 | `--generate-module` followed a symlinked `src` or `src/capabilities.rs` and wrote outside the package | refused before writing (§5) | round-3 security review (LOW) |
| V12 | entry points were always read from `src/lib.rs`: with `[lib] path = "src/lib.txt"` the certificate listed a decoy `src/lib.rs` and said `proved` while `lib.txt` was compiled (review `cert-nonrs.json`) | certificates refuse any `[lib] path` other than `src/lib.rs`; non-`.rs` target roots are `CAP003` in the analysis (§3.3) | second review of round 3 (H5) |
| V13 | `--generate-module` wrote with `fs::write`, which truncates in place: a `src/capabilities.rs` hard-linked to a file outside the package overwrote that file (case `generator-hardlink`) | write a temporary file in `src/`, then `rename` it into place (§5) | second review of round 3 (LOW) |
