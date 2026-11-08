# Changelog

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
