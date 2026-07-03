# CLOC12 — Upstream Closure Compiler test suite as the byte-identical contract

> Status: spec. Implementation lands across many follow-up PRs
> per the slicing in §7. CLOC12.01 = this spec only.

## 1. Purpose

CLOC11 makes `closurec` a drop-in replacement at the **CLI surface**
level. CLOC12 makes it a drop-in replacement at the **byte-output**
level — given the same inputs and the same flags, `closurec` should
produce the same bytes the upstream Java Closure Compiler would.

The only path to that claim that is *actually verifiable*, instead of
aspirational, is to have a comprehensive regression suite that fails
loudly the moment we drift. CLOC12 says:

> The upstream Closure Compiler's own unit-test suite IS that
> regression suite. We port it, verbatim where we can, into Rust
> tests against our pass crates. Passing a ported test means we agree
> with upstream on that exact input/output pair. Failing a ported
> test is a tracked gap.

That suite is already written, already maintained by upstream, and
already pinned to a stable behavior contract by being part of CI for
a decade-plus production compiler. We don't have to invent the
oracle. We just translate it.

## 2. Why this beats "run the JAR and diff"

The alternative — vendoring `closure-compiler.jar`, invoking both
binaries in CI, diffing stdout — was rejected at user direction
because:

1. **It pulls a JVM toolchain into our CI** for every PR, slow
   and fragile across OS versions.
2. **It diffs bytes without explaining intent.** A diff says
   "output line 42 column 8 differs"; an upstream test named
   `testFoldNotComparisonWithNaN` says *why* a behavior matters.
3. **The JAR is a moving target tied to Java version, classpath,
   and `-Xss` settings.** Pinning is hard. Reproducibility is
   hard.
4. **It doesn't survive offline development.** Tests should
   run on a laptop with no network and no Java install.

Porting tests gives us:

- **Self-contained Rust tests** — no JVM, no jar, no diff script.
- **Documented intent** — every test name is a sentence about
  what the compiler is supposed to do.
- **Modular work-list** — each upstream test file maps to one of
  our pass crates, so progress is measurable per crate.
- **Stable contract** — we pin to one upstream tag at a time;
  upgrading the pin becomes its own deliberate PR.

## 3. Upstream layout we mirror

Upstream Closure (`github.com/google/closure-compiler`) lays its
tests out by what they exercise:

```
src/test/java/com/google/javascript/jscomp/
  PeepholeFoldConstantsTest.java         — constant folding
  PeepholeRemoveDeadCodeTest.java        — dead-code elimination
  PeepholeMinimizeConditionsTest.java    — control-flow folding
  PeepholeReplaceKnownMethodsTest.java   — known-method folding
  RenameVariablesTest.java               — variable renaming
  RenamePropertiesTest.java              — property renaming
  InlineFunctionsTest.java               — function inlining
  RemoveUnusedVarsTest.java              — dead variable removal
  CollapseAnonymousFunctionsTest.java    — function collapsing
  CollapsePropertiesTest.java            — namespace flattening
  CodePrinterTest.java                   — emitter formatting
  SourceMapGeneratorV3Test.java          — source-map serialization
  CommandLineRunnerTest.java             — CLI-level integration
  ...
```

Each Java file maps to a Rust file in the corresponding pass crate's
`tests/upstream/` subdirectory:

```
code/packages/rust/closure-pass-constant-fold/
  tests/
    upstream/
      peephole_fold_constants_test.rs
      UPSTREAM_SHA
      ATTRIBUTION.md
```

The Rust file name is the Java class name `snake_cased` and stripped
of the `.java` extension. One Rust file per upstream Java file. No
mixing of upstream tests into our own `tests/*.rs` — they stay in
`tests/upstream/` so the boundary is obvious.

## 4. Pinning policy

The upstream commit our ports track is recorded **once per pass
crate**, in `tests/upstream/UPSTREAM_SHA`:

```
# UPSTREAM_SHA
# Tracks google/closure-compiler at this commit.
# All test files in this directory were ported from that snapshot.
commit:  <40-char SHA>
tag:     v20250402
date:    2025-04-02
url:     https://github.com/google/closure-compiler/tree/<sha>
```

The SHA is per-crate, not per-repo, so different passes can ride
different upstream pins. Upgrading the pin for a crate is a
deliberate PR that:

1. Bumps `UPSTREAM_SHA` to a new commit.
2. Re-ports every Java test file in scope, diff-checking against
   the previous port.
3. New tests upstream added since the old pin → port them.
4. Tests upstream removed/renamed since the old pin → remove them
   from our side too (matching upstream's deprecation).
5. Tests whose expected output changed upstream → update our
   expected output, document the upstream change in the commit
   message.

This is the same playbook you'd use to vendor any third-party
test corpus. It keeps drift explicit.

## 5. Attribution

Upstream Closure Compiler is Apache-2.0. Porting tests is allowed.
Every `tests/upstream/` directory carries an `ATTRIBUTION.md`:

```
# Attribution

Tests in this directory are ported from the Google Closure
Compiler under the Apache License, Version 2.0:

    https://github.com/google/closure-compiler
    LICENSE: https://www.apache.org/licenses/LICENSE-2.0

Files ported:

  - peephole_fold_constants_test.rs
      from src/test/java/com/google/javascript/jscomp/
           PeepholeFoldConstantsTest.java
      tracked SHA: see UPSTREAM_SHA

Translation notes:

  - JUnit `@Test` becomes Rust `#[test]`.
  - `assertPrint(in, out)` becomes a call to a small helper that
    runs the pass and asserts byte-equal output.
  - `disable_*()` / `enable_*()` Java setup methods become
    explicit fields on a test-context struct.
  - Tests that exercise upstream-only features become
    `#[ignore = "..."]` with a gap reference.
```

Every ported Rust file carries a per-file header:

```rust
//! Ported from PeepholeFoldConstantsTest.java in
//! google/closure-compiler, Apache-2.0.
//!
//! Upstream SHA: see tests/upstream/UPSTREAM_SHA.
//!
//! Translation policy: see code/specs/CLOC12-upstream-test-suite-port.md.
```

## 6. Translation conventions

Upstream Java tests share patterns that translate predictably:

| Upstream Java                          | Rust port                                  |
|----------------------------------------|---------------------------------------------|
| `@Test public void testFoo()`          | `#[test] fn test_foo()`                     |
| `assertPrint("a", "b")`                | `assert_pass_eq("a", "b")` helper           |
| `assertSame("a")`                      | `assert_pass_eq("a", "a")` (identity)       |
| `testSame("a")`                        | `assert_pass_eq("a", "a")`                  |
| `test(srcs, expected)`                 | `assert_pass_eq(srcs, expected)`            |
| `@Before public void setUp()`          | a `TestCtx::new()` constructor              |
| `enable_normalize()`                   | `ctx.normalize = true`                      |
| `disable_normalize()`                  | `ctx.normalize = false`                     |
| `getFolder()` / pass-under-test choice | choose the right `closure-pass-*` crate     |

The `assert_pass_eq` helper lives in a tiny per-crate test-support
module. It runs the pass (and only that pass) over the input,
re-emits, and asserts byte-equal. Where upstream lexes/parses
through their `Compiler` harness, we go directly through
`javascript-lexer` + `javascript-ast` + the pass crate + `closure-emitter`,
which keeps each test focused on one transform.

### 6.1 Ignored tests

A ported test that fails today is **not** removed and is **not**
silently `unwrap()`-ed. It is marked `#[ignore = "..."]` with a
short reason and a tracking pointer:

```rust
#[test]
#[ignore = "blocked on closure-pass-constant-fold lacking template literal folding (CLOC12.gap-001)"]
fn test_fold_template_literal_concat() {
    assert_pass_eq("`a` + `b`", "`ab`");
}
```

Gaps go in `code/specs/CLOC12-gaps.md` with `gap-NNN` IDs so any
ported `#[ignore]` reason resolves to a real entry.

`cargo test` runs ignored tests under `--include-ignored`, so a CI
job (`closurec-parity-ignored`) measures the gap count over time
and treats *decreases* as progress and *increases* as regressions.

### 6.2 Tests we'll never port

Some upstream tests don't make sense for us — Java-VM-internal
checks, deprecated GWT scaffolding, tests of removed flags. Those
are listed in `tests/upstream/SKIPPED.md` per crate with one-line
reasons. We don't want to silently drop them; we want to record
*why* a port is intentionally skipped so a future port-bump PR
doesn't accidentally re-add them.

## 7. Slicing

Each slice is one upstream test file (or a coherent subset, capped
at ~25 ported tests per PR so review stays tractable).

| Slice     | Upstream file                              | Target crate                                   |
|-----------|--------------------------------------------|-------------------------------------------------|
| 12.01     | (this spec)                                | (none)                                          |
| 12.02     | PeepholeFoldConstantsTest (subset 1)       | closure-pass-constant-fold                      |
| 12.03     | PeepholeFoldConstantsTest (subset 2)       | closure-pass-constant-fold                      |
| 12.04     | PeepholeRemoveDeadCodeTest                 | closure-pass-dce                                |
| 12.05     | PeepholeMinimizeConditionsTest             | closure-pass-fold-control-flow                  |
| 12.06     | PeepholeReplaceKnownMethodsTest            | closure-pass-constant-fold                      |
| 12.07     | CodePrinterTest (subset 1)                 | closure-emitter                                 |
| 12.08     | CodePrinterTest (subset 2)                 | closure-emitter                                 |
| 12.09     | SourceMapGeneratorV3Test                   | closure-source-map                              |
| 12.10     | RenameVariablesTest                        | closure-pass-rename                             |
| 12.11     | RemoveUnusedVarsTest                       | closure-pass-remove-unused-vars                 |
| 12.12     | InlineFunctionsTest                        | closure-pass-inline                             |
| 12.13     | CollapsePropertiesTest                     | closure-pass-collapse-properties                |
| 12.14     | CommandLineRunnerTest (subset)             | closurec (binary integration)                   |
| 12.15+    | continue per crate                         | as needed                                       |

The first port (12.02) is deliberately against `closure-pass-constant-fold`
because that crate already has a real (non-identity) implementation, so
the ported tests will tell us something signal-rich on day one. Subsequent
slices fan out by pass.

## 8. Correlation-vector interaction

Ported tests are about byte-output equivalence, not CV trace shape.
They run with CV disabled — the default — and assert only on the
emitted JavaScript bytes.

A separate future slice (CLOC12.NN, out of scope here) may add a
companion port that re-runs each upstream test with `--correlation_vector`
on and asserts that *every input token shows up in the CV trace*
(coverage invariant), but that is not part of the byte-identical
contract and is not gated by CLOC12.01.

## 9. Acceptance for CLOC12.01

This PR ships **only this spec file**. Done means:

- `code/specs/CLOC12-upstream-test-suite-port.md` exists with §§1–8.
- `code/specs/CLOC12-gaps.md` exists as an empty seed file with the
  `gap-NNN` numbering convention documented but no entries yet.
- Spec is reviewable in isolation; no code change, no test change,
  no Cargo change.

The first actual port (12.02) lands as a follow-up PR.
