---
category: Testing & coverage
---

# Coverage include filters must match workspace-relative source paths

Linux CI passed all native Rust numerical tests and the checkpoint/resume
example, then tarpaulin failed with "No coverage results collected". The
library-only filter `*/neural-framework/src/*` required a parent directory that
does not exist after tarpaulin strips Cargo's workspace root.

Tarpaulin 0.37.2 resolves include paths relative to `metadata.workspace_root`:
here `code/packages/rust`, so the source is `neural-framework/src/lib.rs`.
Use `neural-framework/src/*`, retain integration tests and the 95% threshold,
and regression-test that library sources match while dependencies/tests do not.
Check the tool's path normalization, not just whether a flag exists. Windows
LLVM coverage evidence does not replace the actual Linux tarpaulin CI run.
