---
category: Testing & coverage
---

# Changing what a shared runtime value means needs every dependent crate's tests run, not only the crates the PR edits

LANG83 PR5b (#18838) made vm-core's `Value::Str` a string of bytes, and
`str_slice` stopped refusing to cut a UTF-8 character. Before pushing, the
merged tree was tested crate by crate, but only for the crates the PR edited.
`build (macos-latest)` then failed in `cobol-iir-compiler`, which the PR never
touched. Eight `jit_e2e` characterization tests pinned the *old* behaviour:
the compiled INSPECT reconstruction trapped on a non-ASCII source because
`str_slice` refused the split. With bytes it now runs, so `is_err()` failed.

The same review turned up a quieter break. The COBOL and Dartmouth BASIC test
harnesses printed with `args.first().and_then(Value::as_str).unwrap_or("")`.
`as_str` now answers only for valid UTF-8, so a program that printed a split
character would print *nothing*, and a test would compare against an empty
string without failing loudly. One COBOL case was doing exactly that.

**Fix.** Pin the new byte-based outputs (keeping the char-based oracle
asserts), read sinks with `Value::as_bytes`, and add a bytes runner for
output that is not UTF-8.

**Do differently.**

- When a change alters what a shared runtime type *means* (not only its API),
  list its reverse dependencies and run their tests too:
  `cargo tree -i <crate> -e normal --workspace --prefix none | sort -u`, or
  the build tool's `--list-affected`. The compiler catches API breaks; only the
  dependents' tests catch meaning changes.
- Grep the dependents' tests for characterization asserts that pin the old
  behaviour (`is_err()` beside a comment about the thing you changed).
- When a lossy accessor appears (`as_str()` returning `None` for some values),
  grep for `.unwrap_or("")` / `unwrap_or_default()` on it. Those calls turn a
  new `None` into silently empty output.
