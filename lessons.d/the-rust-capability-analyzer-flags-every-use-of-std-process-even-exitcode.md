---
category: Security boundaries
---

# The Rust capability analyzer flags every use of std::process (even ExitCode) as proc:exec and a closure parameter named fs as file access

Adding the first CLI to `rust/warrant-corpus-gen` (CV2-S3c) failed `ca_capability_analyzer` three ways that the code did not deserve: `std::process::exit` and `std::process::ExitCode` both read as `proc` capabilities (`signal` / `exec`) although nothing is spawned; `std::process::id()` read as `proc:exec`; and an unrelated closure parameter named `fs` (`|(_, fs)| fs.iter()`) in already-merged code read as `fs:read`. The analyzer is name-based, not type-based.

Fix: `main` returns `Result<(), Failed>` (exit status 0/1 with no `std::process`), the temporary file name is fixed (`.<name>.tmp`) instead of pid-derived, and the closure parameter was renamed. Run `cargo run -q -p coding_adventures_ca_capability_analyzer -- --dir . --verbose` as soon as a crate gains any I/O, and prefer renaming an identifier or restructuring over declaring a capability the code does not use. Declare only what the verbose output lists, with a justification that names the guard (path guard, symlink refusal, temp-then-rename).
