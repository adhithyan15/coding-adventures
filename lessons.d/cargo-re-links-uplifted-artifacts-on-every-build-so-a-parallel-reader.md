---
category: Rust
---

# Cargo re-links uplifted artifacts on every build, so a parallel reader can see them missing

**What went wrong.** `lang-aot`'s test helper `gc_core_capi_archive` ran a
nested `cargo build --release -p gc-core-capi` on every call and then checked
that `target/release/libgc_core_capi.a` existed. Its comment said concurrent
calls were safe because they "serialise on cargo's own target-dir lock". They
were not. That lock only covers the *build*. Cargo re-creates the file it copies
up to `target/<profile>/` (the "uplifted" artifact) on every build, even a fully
cached one: it deletes the old file and hard-links a new one. On macOS CI, one
test thread's no-op build deleted the archive between another thread's build
returning and its `is_file()` check. That failed
`pr5a_string_failures_are_registered_traps_on_every_column` with
"Cargo-reported archive does not exist", on a PR that never touched
`lang-aot` (#19859).

**How it was confirmed.** Run `cargo build --release -p gc-core-capi` three
times: `stat` shows a new inode each time, with the same mtime. Then run 40
cached builds in a loop with a shell polling `[ -f archive ]`. It saw the file
missing 263 times in about 2.3 million checks.

**Fix.** Build once per test process with a `OnceLock<PathBuf>` and hand every
thread that path. No nested Cargo runs after the first build in that process,
so nothing can delete the file under a reader.

**What to do differently.** Treat anything under `target/<profile>/` as
unstable while another `cargo` command can run against the same target
directory, *even when nothing needs rebuilding*. A test that shells out to
Cargo for an artifact should build it once per process and reuse the path.
If other processes may build the same package at the same time, copy the
artifact somewhere private or build into a private `--target-dir` (as
`twig-aot/build.rs` does). A comment saying the target-dir lock makes a call
safe is only true for the build itself, never for what happens after it
returns.
