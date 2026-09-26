# v8-test262

The conformance gate for our JavaScript engine. It runs
[test262](https://github.com/tc39/test262), TC39's ECMAScript conformance
suite, and compares the results with a checked-in list that may only grow.

Spec: [`code/specs/V8C09-test262-runner.md`](../../../specs/V8C09-test262-runner.md).
Part of the V8C series ([V8C01](../../../specs/V8C01-overview.md)): a JavaScript
engine built from scratch on the LANG VM, for the Venture browser
([BR02](../../../specs/BR02-venture-completion-roadmap.md) P9).

## Where it fits

```text
javascript-lexer → javascript-parser → (V8C engine, not built yet) → LANG VM
                          ▲
                  v8-test262 measures this now (the parse level),
                  and the engine as it arrives (the run level)
```

The runner comes first on purpose: the engine is built against a gate, and the
gate's first level needs only the parser that already exists.

## Levels

| level | judges | status |
|---|---|---|
| `parse` | parse-negative tests must be rejected; everything else (and its harness includes) must be accepted | implemented |
| `run` | the program runs on the engine | with the engine (V8C phase P1) |
| `module` | module tests and resolution-phase negatives | later |

A result the current level cannot judge is a **skip** with a reason, never a
pass. Nothing in the runner looks at a test's path or text to decide its
outcome.

## Usage

```sh
# test262 at the pinned revision (see TEST262_REVISION)
git clone https://github.com/tc39/test262 /tmp/test262
git -C /tmp/test262 checkout "$(cat TEST262_REVISION)"

cargo run -p coding-adventures-v8-test262 -- --test262 /tmp/test262
cargo run -p coding-adventures-v8-test262 -- --test262 /tmp/test262 language/expressions
cargo run -p coding-adventures-v8-test262 -- --test262 /tmp/test262 --json
cargo run -p coding-adventures-v8-test262 -- --test262 /tmp/test262 --update   # only adds
```

Exit status is 0 with no regressions, 1 when a listed test stops passing, and
2 for a usage or input problem (including a checkout at another revision;
`--allow-revision-mismatch` overrides that for exploration).

## Files

- `TEST262_REVISION`: the test262 commit the lists are measured at. Moving it
  is its own change, together with the list updates it causes.
- `expected/parse.txt`: result ids that pass at the parse level, sorted, one
  per line (`language/expressions/…/x.js#strict`).
- `tests/fixtures/`: small hand-written files in test262's format for the
  runner's own tests. No test262 file is copied here.

## The first parse run (2026-09-25)

At test262 `7ab7faf`, **69,017 of 102,955** results pass at the parse level
(843 skipped: module goal). The biggest gaps are the grammar parser's missing
early errors and Script/Module goals, in `language/expressions` and
`language/statements`, then newer syntax such as `Temporal` and `RegExp`
features. CI (`v8-test262.yml`) fails if any listed result stops passing.

## Why the first parse run showed that

`javascript-parser` is grammar-driven: it has no separate Script and Module
goals, and it does not implement the specification's early errors (duplicate
`let` bindings, `break` outside a loop, strict-mode restrictions). Many
parse-negative tests are early errors, so the first list will be far from
complete. That gap is the next parser work, measured here.
