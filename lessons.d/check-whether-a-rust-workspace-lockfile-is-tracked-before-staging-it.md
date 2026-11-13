---
category: Repo policy / workflow reminders
---

# Check whether a Rust workspace lockfile is tracked before staging it

`Cargo.lock` files are tracked in this repository, one beside every Cargo root
that resolves a third-party crate (the main workspace at
`code/packages/rust/` plus each detached `*-native` crate and standalone
program). The old policy ignored them, so this lesson used to say "leave the
lock out"; that is no longer true. The rules now:

- A PR that adds or changes a crate must commit the updated lock. Generate it
  with `cargo update --workspace` in the root directory (or simply build);
  that refreshes our own path packages and never moves a pinned third-party
  version.
- To resolve a lock conflict, take main's `Cargo.lock` and rerun
  `cargo update --workspace`. Never hand-merge a lockfile.
- To bump a third-party crate, run `cargo update -p <crate> --precise <ver>`
  and commit the lock together with the reason for the bump.
- CI runs `python3 code/scripts/cargo_lockfiles.py check` and rejects a
  missing or stale lock, printing the exact command to run. Run it locally
  (optionally with `--root <dir>`) before pushing.
- Roots whose whole dependency closure is path-only have nothing to pin and
  have no lock; `cargo_lockfiles.py roots` marks the ones that need one.
- Stage lockfiles by explicit path; never `git add -A`.
