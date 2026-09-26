# V8C09 — `v8-test262`: the conformance gate

**Status:** proposed (2026-09-25). First spec of the revised V8C series
([V8C01, revision 2026-09-25](V8C01-overview.md)): test262 is the gate from
the first PR.

## 1. What this is

[test262](https://github.com/tc39/test262) is TC39's conformance suite for
ECMAScript: roughly 50,000 files, each a small program with a YAML header
saying how to run it and what should happen. Every engine that claims
conformance reports against it.

`v8-test262` is the runner. It reads the suite, decides how to run each test,
runs it against whatever part of our engine exists, and compares the result
with a checked-in list. It exists **before** the engine does, because its
first job needs only the parser: whether `javascript-parser` accepts the
programs it should and rejects the ones it must.

## 2. A test file

```js
// Copyright (C) 2015 the V8 project authors. All rights reserved.
/*---
esid: sec-let-and-const-declarations
description: let declarations are not hoisted out of blocks
negative:
  phase: parse
  type: SyntaxError
flags: [onlyStrict]
features: [let]
includes: [compareArray.js]
---*/
$DONOTEVALUATE();
let x; let x;
```

The runner reads the text between `/*---` and `---*/` as YAML. It uses:

| key | meaning for the runner |
|---|---|
| `negative.phase` | `parse` (rejected before running), `resolution` (module linking), or `runtime` |
| `negative.type` | the error constructor expected (`SyntaxError`, `ReferenceError`, …) |
| `flags` | `onlyStrict`, `noStrict`, `raw` (run as-is, no harness), `module`, `async`, `generated`, `CanBlockIsFalse` / `CanBlockIsTrue` |
| `features` | proposal and feature tags (`let`, `class`, `BigInt`, `Temporal`, …); used to group results and to skip features we have not started |
| `includes` | harness files from `harness/` prepended before the test (`assert.js` and `sta.js` always, unless `raw`) |

A test without `onlyStrict`, `noStrict`, `raw` or `module` runs **twice**: once
as sloppy script and once with `"use strict";` prepended. Each run is a
separate result.

## 3. How each phase is judged

The runner has three **levels**, added as the engine grows. A test's result is
`pass`, `fail`, or `skip` (with a reason: a feature we exclude, a flag we do
not support yet).

1. **Parse (P0, this spec's first gate).** No execution. A test whose
   `negative.phase` is `parse` passes when the parser **rejects** it. Every
   other test passes when the parser **accepts** it (including the harness
   includes). Nothing runs, so runtime-negative tests count as "must parse".
2. **Run (P1 on).** The program runs on the engine; a test passes when it
   completes without an uncaught exception, or, if negative at `runtime`,
   when it throws an error of `negative.type`. `async` tests pass when they
   print `Test262:AsyncTestComplete`.
3. **Module (later).** `module` tests and `resolution`-phase negatives, once
   modules exist.

A level never makes a result it cannot judge look like a pass: a test the
current level cannot judge is `skip`, with the reason, never `pass`.

A **parse budget** bounds the input handed to the parser (4 MiB). It was
1 MiB while `staging/sm/String/string-upper-lower-mapping.js` (3.2 MB) made
javascript-parser allocate about 35 GB and got the CI runner killed: the
shared packrat parser memoised a deep clone of each subtree at every (rule,
position), and the expression grammar has some twenty precedence levels. The
memo now shares subtrees (`parser`'s `BuiltNode`), that file parses in about
3 GB, and the budget was raised above it so it is judged again. An
over-budget source is a `skip` with that reason and stays on the pass list.

## 4. The expected-pass list

`code/packages/rust/v8-test262/expected/<level>.txt` lists every test id that
passes at that level, one per line: the path under `test/` plus `#strict` or
`#sloppy` when the test runs twice.

- **It may only grow.** The gate fails if a listed test no longer passes. A
  test that newly passes is reported, and `--update` adds it; review sees the
  growth in the diff.
- **A pass comes from the engine, never from recognising a test.** No code
  path may look at a test's path, description or source text to decide its
  outcome (V8C01 revision, point 4). A test we cannot pass stays off the list.
- The list is sorted and deduplicated so concurrent PRs merge cleanly.

## 5. Where the suite comes from

- **Pinned upstream.** `v8-test262/TEST262_REVISION` holds the test262 commit
  the lists were measured at. CI fetches exactly that commit (a shallow fetch
  of one SHA) into a cache. Moving the pin is its own PR, with the list
  changes it causes.
- **Local runs** point `TEST262_DIR` at a checkout. The runner refuses a
  checkout whose `HEAD` does not match the pin, unless told otherwise.
- **The crate's own unit tests** use small hand-written files in the same
  format (header parsing, flag expansion, include resolution, classification);
  no test262 file is copied into the repository.

## 6. Output

- A summary per top-level directory (`language/expressions`, `built-ins/Array`,
  …): pass, fail, skip, with skip reasons counted.
- `--json` for tooling, and the exact failing ids with the first line of the
  error, for triage.

## 7. What P0 will show, honestly

`javascript-parser` is a grammar-driven parser. It has no separate Script and
Module goals, and it does not implement the **early errors** of the
specification's static semantics (duplicate `let` bindings, `break` outside a
loop, strict-mode-only restrictions, …). Many `negative.phase: parse` tests are
early errors, so the first parse-level list will be far from complete, and the
gap is the next parser work: a static-semantics pass over the typed AST, then a
Module goal. The runner measures that gap; it does not hide it.

## 8. Crate

`code/packages/rust/v8-test262`: a library (header parser, classifier,
list comparison) and a binary (`v8-test262 --level parse [--update] [--json]
[filter…]`), no network access of its own. Dependencies: `javascript-parser`,
a YAML parser for the header subset test262 uses, `walkdir`-style traversal.
CI: a job that restores the pinned suite and runs the parse level, blocking
on a shrunk list.

## 9. Order

1. This spec.
2. The crate: header parser, classifier, strict/sloppy expansion, include
   resolution, list comparison; unit tests on hand-written files.
3. The parse level against the pinned suite; the first `expected/parse.txt`.
4. The CI job.
5. Static semantics (early errors) in `javascript-parser`, each slice growing
   the list.
