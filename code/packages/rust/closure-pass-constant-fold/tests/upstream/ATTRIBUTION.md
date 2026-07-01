# Attribution

Tests in this directory are ported from the Google Closure Compiler
under the Apache License, Version 2.0:

    https://github.com/google/closure-compiler
    LICENSE: https://www.apache.org/licenses/LICENSE-2.0

## Files ported

- `peephole_fold_constants_test.rs`
    - upstream: `test/com/google/javascript/jscomp/PeepholeFoldConstantsTest.java`
    - blob SHA at port time: `c67ab886ec14fe2ce9d70b5336ba38108c7152c2`
    - tracked commit: see `UPSTREAM_SHA`
- `peephole_replace_known_methods_test.rs`
    - upstream: `test/com/google/javascript/jscomp/PeepholeReplaceKnownMethodsTest.java`
    - tracked commit: see `UPSTREAM_SHA`
    - Covers the String-method folds our `ConstantFoldPass` performs today
      (indexOf, lastIndexOf, case conversion, slice, substring, substr, charAt,
      charCodeAt, repeat, trim, includes/startsWith/endsWith). Upstream folds we
      do not perform yet — `Math.abs`/`floor`/`ceil`/`round`, `Array#join`, and
      `String#concat` with coerced non-string args — are `#[ignore]` placeholders
      pinned to gap-141 … gap-143.

## Translation notes

This is the **first** port under CLOC12, deliberately scoped narrow.
It establishes the file-layout and gap-tracking pattern; later slices
expand coverage.

- Upstream tests are written against a JS source-string surface
  (`test("1 + 2", "3")`). closurec doesn't yet expose a public
  `parse_javascript_to_typed_program` entry point that produces the
  `javascript-ast::Program` directly — `javascript-parser` returns a
  `GrammarASTNode`. Until that bridge lands (a future CLOC11.* slice),
  ports here build typed `Program` values by hand using the same
  literal/expression constructors as `closure-pass-constant-fold`'s
  inline unit tests.
- The hand-construction means we port the *behavior* upstream is
  asserting, not its literal source-string surface. Each ported test
  documents both the upstream method name and the upstream
  `test(...)` line being modelled.
- Tests that need features the typed AST does not yet model in our
  literal builders (`typeof`, `void 0`, BigInt literals, `NaN` /
  `Infinity` as identifiers, regex literals, etc.) become
  `#[ignore = "blocked on gap-NNN"]`. Each gap is tracked in
  `code/specs/CLOC12-gaps.md`.
- JUnit `@Before setUp()` and the `late` / `useTypes` / `numRepetitions`
  / `assumeGettersPure` knobs are not modelled — they don't change
  the byte output of the folds we cover here.

## Ignored tests

See `code/specs/CLOC12-gaps.md` for the current set of `gap-NNN`
entries that gate ignored ports.

## Skipped (intentionally not ported)

None yet.
