# simple-fold-comma-split

End-to-end oracle for **comma-sequence statement splitting** in
`closure-pass-fold-control-flow` (fcf 0.35.0).

A comma sequence used as an *expression statement* at a statement-list position
(a function body or the program body) splits into one statement per operand,
because the comma operator discards every value but the last and an expression
statement already discards its value:

```text
a(), b();        ->  a(); b();
x(), y(), z();   ->  x(); y(); z();
```

A comma sequence in a **single-statement body** (an `if`/`for` with no braces)
has no statement list to splice into, so it stays fused. In this fixture the
`if (cond) p(), q();` first collapses (fcf if -> `&&`) to `cond&&(p(),q())` with
the comma preserved, and the `for` body keeps `step(),tick()` fused.

## Files

- `input/a.js` — the source, exercising both the split and the kept cases.
- `flags.txt` — `--compilation_level SIMPLE --js input/a.js`.
- `expected.stdout` — byte-identical to the reference Closure Compiler
  (`v20260712`, `SIMPLE_OPTIMIZATIONS`, `ECMASCRIPT_2020`, `NO_TRANSPILE`).

## Expected output

```text
function f(){a();b()}x();y();z();cond&&(p(),q());for(;run();)step(),tick();
```

Verified against the oracle jar with `xxd` (exact bytes).
