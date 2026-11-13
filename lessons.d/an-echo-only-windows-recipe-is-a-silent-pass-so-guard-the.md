---
category: CI & GitHub Actions
---

# An echo-only Windows recipe is a silent pass, so guard the shape and make the allowlist shrink-only

A `BUILD_windows` made only of `echo` lines reports `BUILT` in a fraction of a second and counts as green, so 336 packages (BUILD-002) had "Windows support" that ran no code. Each skip was reasonable when written and none had an owner, so nothing ever forced it to be revisited. Haskell's `if not exist cabal.exe (echo cabal not found -- skipping)` probe passed in under a second for the same reason: the check that decides to skip also decides to pass.

The fix is a guard on the shape, not a one-off sweep. `code/scripts/audit_build_windows.py --check` (the `contracts` job) now fails when the recipe Windows actually runs does nothing: all lines are `echo`/`exit 0`/`if ... echo`/empty, or the package has no Windows build file at all. Real exceptions live in `code/specs/data/windows-skip-allowlist.json` with a category, reason and evidence, and the list can only shrink: an entry whose package is converted is *stale* and fails the audit, and `ALLOWLIST_CAPS` fixes the entry count per category so a new entry shows up as a reviewed change to a number.

What to do differently:

- Never write `echo ... skipping` as a Windows recipe. Give the package a real recipe; if it truly needs an Apple framework or a platform API, add a reasoned allowlist entry and say why in the PR.
- When you convert a package, delete its allowlist entry and lower its cap in the same PR; the audit tells you which.
- A guardrail needs a shrink-only rule from day one. An allowlist that can grow is a TODO list that nobody reads; one that fails when an entry goes stale cannot rot.
- When you detect a shape, run the detector over the whole tree and read every hit before turning it blocking. Here that also surfaced bugs hiding as skips (nine Rust native-extension crates that echo "compile-only" on every OS), and scaffold generators that were still emitting the shape for new packages (Haskell, Perl, Lua).
