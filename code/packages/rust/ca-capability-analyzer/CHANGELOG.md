# Changelog

## 0.2.0 — 2026-11-08

- Generated capability module and canonical effect certificate support (VER23 / VER24 / VER-P027b):
  - Sealed capability module generator (`src/capabilities.rs`, VER23) enforcing "you cannot name what you did not declare":
    - Pure computation packages receive `PureCapabilities` zero-effect token with `#![deny(unsafe_code)]`;
    - Capability-bearing packages receive typed witness handles strictly limited to declared capabilities;
    - `--generate-module` and `--check-module` for pre-commit generation and CI regeneration verification;
  - Canonical effect proof certificates (`effect_certificate.json`, schema `ver24-effect-cert-v1`, VER24):
    - Independent small certificate checker reading package sources, manifests, and certificates directly;
    - Cryptographic binding via deterministic multi-file SHA-256 source and manifest digests;
    - Verification of public entry points, declared effect bounds, unconditional roots, and honest `"proved"` label;
    - Comprehensive sabotage tests detecting source/manifest tampering, uncertified entry points, undeclared effects, and dishonest labels;
    - Self-contained FIPS 180-4 SHA-256 cryptographic hasher with zero external dependencies.

## 0.1.0 — 2026-11-08

- Initial implementation of the Rust Static Capability Analyzer (`ca-capability-analyzer-rs`, VER08 / VER08a / VER-P027a):
  - Zero third-party dependencies: implemented purely with the Rust standard library;
  - Strict `required_capabilities.json` schema validation against Spec 13 closed category/action taxonomy;
  - Automatic fallback to pure computation (`capabilities = []`) on absent manifest;
  - Deny-by-default ambient authority detection for filesystem (`fs`), networking (`net`), process execution (`proc`), environment access (`env`), system timing (`time`), standard streams (`stdin`/`stdout`), threads (`thread`), and foreign function interface (`ffi`);
  - Pure computation core allowlist (`core::*`, `std::vec::*`, `std::cmp::*`, `std::mem::*`, `core::hint::black_box`);
  - Banned construct inspection (`unsafe_code`, `inline_assembly`, `foreign_linkage`) with fingerprint exception support;
  - Resource quotas: 10,000 file cap, 4 MiB per-file limit, 64 MiB total source limit, 1 MiB manifest limit, symlink refusal;
  - Diagnostic formatting with stable exit codes (0 for pass, 1 for violations, 2 for tool errors).
