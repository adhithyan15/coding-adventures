---
category: TypeScript / JavaScript
---

# A JavaScript regression test built on + can pass by accident, because + concatenates strings and arrays

Issue #18466 reported that the JavaScript backend's closure wrapper,
`(..._a) => __lambda_0(_a, ..._a)`, shadows a captured user variable named
`_a`.  The first regression row was the issue's own repro,
`(define (f _a) (lambda (x) (+ x _a))) (print ((f 10) 1))`, expecting `11`.
It **passed** against the unfixed backend.  The capture read the wrapper's
argument array `[1]` instead of `10`, and `__Sir.plus(1, [1])` fell back to
JavaScript's `+`, which turned the array into the string `"1"` and printed
`"11"`, the expected answer reached by the wrong route.

Fix: the row now multiplies, `(* x _a)` with `x = 2`, expecting `20`.  The
buggy program prints `4`, because `2 * [2]` coerces the array to the number 2.

**When a JavaScript-backed test checks a value computed with `+`, pick
operands whose concatenation cannot spell the expected sum, or use `*` or
`-`.**  Run every new regression test against the unfixed code and look at
the value it actually printed, not only at whether it failed.
