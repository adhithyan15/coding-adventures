# capability-cage

Rust port of `code/packages/go/capability-cage/`. Implements the
runtime ring of the capability cage: manifest loading, secure
wrappers, and the swappable `Backend` trait.

See `code/specs/capability-cage-rust.md` for the full design and
`code/specs/13-capability-security.md` for the cross-language
taxonomy.

## What this crate is

The cage's job is to ensure that no Rust package in this repo
performs an OS operation that its `required_capabilities.json`
hasn't declared. Three rings of enforcement:

1. **Lint-time** — a separate `cargo` lint (future package) flags
   raw stdlib usage outside of the secure wrappers.
2. **Runtime** — the secure-wrapper functions in this crate (e.g.
   `secure_file::read_file`) check the manifest before every OS
   call.
3. **Hard-cage** — Tier 2 / 3 agents inject a `HostRpcBackend`
   (provided by `host-runtime-rust`, future) that routes calls
   over `secure-host-channel` instead of hitting stdlib at all.

This crate ships rings 2 and 3's seams. The wrappers and the
`Backend` trait are stable contracts; anything below them is a
swappable detail.

Manifest loading also runs the Chief-of-Staff read/write separation
validator. Optional `flavor` (`ingestion`, `actuation`, `internal`)
and `trust` (`trusted`, `untrusted`) fields let a manifest override
the default classifier when a capability crosses an agent boundary.
`CapabilitySurfaceSummary` gives host and catalog code a payload-free
read-side view of category, action, target-count, and boundary annotation
coverage without exposing raw target strings.

## Quick example

```rust
use capability_cage::{Capability, Category, Action, Manifest, secure_file};
use std::path::Path;

let m = Manifest::try_new(vec![
    Capability::new(
        Category::Fs,
        Action::Read,
        "./grammars/*.tokens",
        "load lexer DFA",
    ).unwrap(),
])?;

// Reading a file the manifest covers — succeeds (calls OpenBackend).
let bytes = secure_file::read_file(&m, Path::new("./grammars/json.tokens"))?;

// Reading a path the manifest does NOT cover — fails before any
// stdlib call. The error wraps a CapabilityViolationError.
let err = secure_file::read_file(&m, Path::new("/etc/passwd")).unwrap_err();
assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
```

## Module surface

| Module          | What it does                                           |
|-----------------|--------------------------------------------------------|
| (root)          | re-exports of the public API                            |
| `category`      | `Category`, `Action` enums + valid-pair table           |
| `capability`    | `Capability` struct (immutable, validated)              |
| `errors`        | `CapabilityViolationError`, `ManifestError`, `InvalidCombination` |
| `glob`          | `match_target(pattern, candidate)` for fs / net targets |
| `manifest`      | `Manifest` with `has`/`check`/`try_new`/`load_from_str`/`load_from_file` and payload-free surface summaries |
| `audit`         | `Operation<T>`, `OperationRecord`, and `AuditSink` envelope types |
| `backend`       | `Backend` trait + `OpenBackend` / `TestBackend` / `DenyAllBackend`, `with_backend(...)` guard |
| `secure_file`   | `read_file` / `write_file` / `create_file` / `delete_file` / `list_dir` |
| `secure_env`    | `read_var` / `write_var` for manifest-checked environment access |

## Backend swap

Tests inject a `TestBackend` to assert on the call sequence:

```rust
use capability_cage::{TestBackend, with_backend, secure_file, ...};
use std::sync::Arc;

let backend = Arc::new(TestBackend::new().with_response("./out", b"hi"));
let _guard = with_backend(backend.clone());
// ... secure-wrapper calls now route through TestBackend ...
let calls = backend.calls();
```

The backend slot is process-wide, but `with_backend` serializes
overlapping overrides so the crate's own tests can run with Rust's
default parallel test scheduler.

## Out of scope (V1)

Spec-defined but landing in subsequent PRs:

- `secure_net` / `secure_proc` / `secure_time` / `secure_stdio` modules
- `build.rs` codegen for `package_manifest()`
- Cross-language conformance suite shared with the Go cage
- The CI lint that rejects raw stdlib usage

## Dependencies

- `coding-adventures-json-parser` — manifest JSON parsing
- `coding-adventures-json-value`  — typed JSON values
- `read-write-separation` — manifest-level RWS classification and validation

No std-OS access from this crate's own code: everything happens
inside `Backend` implementations (which the consumer chooses).

## Development

```bash
bash BUILD          # cargo test -p capability-cage
```
