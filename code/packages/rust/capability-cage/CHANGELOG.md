# Changelog

## 0.1.0 — 2026-05-08

Initial release. V1 scope per `code/specs/capability-cage-rust.md`:

- `Category` and `Action` enums (8 categories, 14 actions) with the
  19 valid pairings from the spec.
- Immutable `Capability` and `Manifest` types.
- `Manifest::load_from_str` / `Manifest::load_from_file` that parse
  the `required_capabilities.json` schema and reject invalid
  combinations (unknown category, unknown action, unsupported pair,
  empty target).
- `Manifest::has` / `Manifest::check` with glob matching against
  manifest targets.
- Glob matcher (`match_target`) supporting:
  - exact literal match
  - `*` (one path component / one wildcard within a segment)
  - `**` (any number of components)
  - net targets `host:port` with `*` wildcards on either side
  - Windows backslashes normalized to `/`
- `Backend` trait with three implementations:
  - `OpenBackend` — calls stdlib `std::fs`
  - `DenyAllBackend` — refuses every call (good test default)
  - `TestBackend` — records every call, returns scripted responses
- `with_backend(...)` for scoped process-wide backend swap, including
  serialized overrides for parallel test safety.
- `secure_file` module with `read_file`, `write_file`, `create_file`,
  `delete_file`, `list_dir` — all manifest-checked and delegating to
  the current backend.

Out of scope for v1 (will land in subsequent PRs):

- Operation/audit envelope (`Operation<T>`, `AuditSink`).
- Other secure-wrapper categories (`secure_net`, `secure_proc`,
  `secure_env`, `secure_time`, `secure_stdio`).
- `build.rs`-driven package_manifest() codegen.
- Cross-language conformance suite shared with the Go cage.
- The lint that rejects raw stdlib usage outside of the backend.
- RWS Phase 1 enforcement (per `read-write-separation.md`).
