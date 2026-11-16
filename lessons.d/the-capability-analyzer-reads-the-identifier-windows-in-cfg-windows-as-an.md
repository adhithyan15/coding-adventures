---
category: Rust
---

# The capability analyzer reads the identifier windows in cfg(windows) as an undeclared ffi call, so gate Windows code with target_os

After WD-004b1's `lane_loader.rs` changed `ca-assurance-report`'s sources, CI failed
`--verify-cert` on a stale `source_digest` (see the existing lesson on changing a
crate's source). Regenerating with `--emit-cert` then refused to emit: the analyzer
reports `CAP001 ffi:call:*` for the bare identifier `windows` in `cfg!(windows)` and
`#[cfg(windows)]`, because `windows` is on its list of FFI crate names. The code only
reads files, so the right response is not to widen `required_capabilities.json` but
to spell the gate `target_os = "windows"`, which the analyzer ignores and which means
the same thing. Order of work: inspect what the new code does against the declared
capabilities, change the code (never the manifest) if it trips a false positive, then
`--emit-cert` and confirm only `source_digest` moved in the diff.
