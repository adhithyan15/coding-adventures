---
category: Rust
---

# A Lean oracle script that calls findSysroot needs lean on PATH, not just invoked by path

`lean --run dump.lean` with an absolute path to the elan toolchain's `lean` runs the script,
but `initSearchPath (← findSysroot)` inside it shells out to a bare `lean` to ask for the
sysroot and fails with "could not execute external process 'lean'" when the toolchain `bin`
directory is not on `PATH` (elan installed the toolchain but no shim is on `PATH` in this
environment). The PIF prelude's `oracle/run.sh` therefore prepends the toolchain's `bin` to
`PATH` before running. Rule: when a script runs a pinned Lean by absolute path, also export its
`bin` directory on `PATH`; and keep the oracle's output committed so that no build or test ever
depends on Lean being installed.
