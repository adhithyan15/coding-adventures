---
category: Compiler / VM / language pipeline
---

# A parser change that accepts more syntax must ship its early errors in the same PR, because the test262 gate counts every negative test it stops rejecting as a regression

The V8C test262 parse-level gate (`v8-test262.yml`) went red on main on
2026-11-09 and stayed red for 48 hours, failing about half of the PRs that touched
`javascript-parser`: 0 regressions at #19032, 16 after #19048, 19 after #19061,
264 after #19068. It is not a required check, so four PRs merged over it.

Cause: the grammar used to *refuse* object-method shorthand, `async function`
expressions and async methods outright. Every negative test about them (duplicate
parameters, `"use strict"` with a default parameter, `await` as an identifier,
`super.x` in a plain function, a rest parameter with a trailing comma, ...)
"passed" because the program failed to parse, not because any early error fired.
`javascript-parser` had no function-level early errors at all, so when #19068
taught the grammar to parse those constructs, 245 listed negatives became
accepted programs overnight. Eleven more regressions came from #19048: `if (x)
let⏎{}` had passed only because `let` was parsed as a declaration, and the
correct reading (an identifier) needed a token fix-up the grammar lacked.

The lesson is the one in "A test262 pass list can hold tests that pass only
because the parser misreads them", seen from the other side: widening what the
grammar accepts is itself a conformance change. Before merging a grammar or
parser change that accepts new syntax, run the runner over the directories that
syntax lives in (`v8-test262 --test262 DIR language/expressions/async-function
...` takes a minute; the full run takes about fifteen) and either land the early
errors in the same PR or keep the new syntax behind the failing check until they
exist. Do not merge over a red non-required gate: the "regressions" are real
programs the engine now accepts and would compile.

The fix is in `javascript-parser/src/function_errors.rs` (functions, strict mode,
template escapes), the assignment-target validator in `early_errors.rs`, and
`normalize_statement_body_let` in `asi.rs`. Group regressions by root cause
(here: five early-error families and one token fix-up) rather than fixing them
test by test; 264 failures were about a dozen rules.
