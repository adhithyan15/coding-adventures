# Attribution

Tests in this directory are ported from the Google Closure Compiler
under the Apache License, Version 2.0:

    https://github.com/google/closure-compiler
    LICENSE: https://www.apache.org/licenses/LICENSE-2.0

## Files ported

- `code_printer_test.rs`
    - upstream: `test/com/google/javascript/jscomp/CodePrinterTest.java`
    - blob SHA at port time: `64944e8e615c95b0cf845aab86f77d634776a5b1`
    - tracked commit: see `UPSTREAM_SHA`

- `code_printer_function_test.rs`
    - upstream: `test/com/google/javascript/jscomp/CodePrinterTest.java`
      (the `testFunctionExpression*` / IIFE / `function`-at-statement-start
      cases)
    - tracked commit: see `UPSTREAM_SHA`
    - Isolates `emit_function_expression` + the precedence wrap that landed
      with `Expression::FunctionExpression` (CLOC12.149) and became reachable
      once the bridge converted `function_expression` (gap-153). 12 active
      `#[test]`s, no `#[ignore]` — the emitter conforms to every covered
      shape (anonymous/named, params, body, IIFE, member-object,
      call-argument, generator/async prefixes).

## Translation notes

Fourth port under CLOC12 (after `closure-pass-constant-fold` in
CLOC12.02, `closure-pass-dce` in CLOC12.04, and
`closure-pass-fold-control-flow` in CLOC12.05). First port that
targets the *emitter* rather than a transform pass — the shape of
assertions is "given AST, emit string equal to X" instead of "given
AST, fold to AST'".

- Upstream tests use `assertPrint(input_js, expected_js)` and
  `assertPrintSame(js)` which both lex/parse the input through their
  own compiler harness and pretty-print the result. Our `emit()`
  takes a typed `Program` directly — there's no parser bridge yet.
  Ports here hand-construct typed-AST inputs.
- **Coverage scope is narrow today.** Upstream `CodePrinterTest` is
  ~263 `@Test` methods covering BigInt, optional chaining,
  trailing-comma policies, spread, async/await, classes, generators,
  template literals, regex, and every operator precedence corner.
  Most of these reference Phase 2+ AST node variants we don't have
  yet (BigInt, OptionalCallExpression, TemplateLiteral, etc.).
- **Our emitter unconditionally wraps every ExpressionStatement in
  parens** — `(2 + 3);` instead of upstream's `2+3;`. That's a
  deliberate Phase 1 simplification documented in the emitter's
  crate-level docs. It means most upstream `assertPrintSame` tests
  fail today — the input form they expect to be unchanged isn't what
  our emitter produces. Each such case becomes a `#[ignore]` with
  a gap describing the divergence rather than a behaviour bug.
- Each ported test docstring records the upstream `assertPrint*`
  line being modelled so a future re-port can diff cleanly.

## Ignored tests

See `code/specs/CLOC12-gaps.md` for `gap-NNN` entries that gate
ignored ports.

## Skipped (intentionally not ported)

None yet.
