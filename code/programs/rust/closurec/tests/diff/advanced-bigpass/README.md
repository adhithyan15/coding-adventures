# `advanced-bigpass` — end-to-end ADVANCED proof (size + runtime equivalence)

Most diff fixtures isolate a single transformation. This one proves the **whole
ADVANCED pipeline** cooperates on a realistic little geometry module, and that
it shrinks the code **without changing what the program does**.

## The module

`input/a.js` defines four functions and reports three values:

```js
function unusedPerimeter(w, h) { return 2 * (w + h); }  // never used
function area(w, h)            { return w * h; }         // called once, literal args
function hypotSq(a, b)         { return a * a + b * b; } // called once, literal args
function scale(x)              { return x * 10; }        // called once + passed to sink
report(area(3, 4), hypotSq(3, 4), scale(7));
sink(scale);
```

## What ADVANCED produces

```text
function f(x){return x * 10};report(12,25,f(7));sink(f);
```

| pass                          | evidence                                       |
|-------------------------------|------------------------------------------------|
| dead-code elimination         | `unusedPerimeter` is gone                       |
| single-use inline + fold      | `area(3,4)` → `12`, `hypotSq(3,4)` → `25`       |
| global renaming (ADVANCED-only)| `scale` → `f` (SIMPLE keeps `scale`)           |
| live-reference retention      | `f(7)` and `sink(f)` survive                    |

## Runtime equivalence

The optimized program reports the **same observable values** as the original:

| reported value | original computes        | optimized output            |
|----------------|--------------------------|-----------------------------|
| 1st            | `area(3,4)` = `3*4` = 12  | literal `12`                |
| 2nd            | `hypotSq(3,4)` = `9+16` = 25 | literal `25`             |
| 3rd            | `scale(7)` = `7*10` = 70  | `f(7)`, `f` ≡ `x*10`, = 70  |

The two folded literals are asserted directly; the third is preserved
structurally (identical body, renamed). `report` / `sink` are undeclared externs
(sinks) and are left untouched.

## Honest size measurement

Comparing against the raw source would conflate optimization with comment +
whitespace stripping. The test baselines against **`WHITESPACE_ONLY`** (strips
comments + whitespace, performs no optimization), so the measured shrink is
attributable to optimization alone:

```text
WHITESPACE_ONLY: 195 bytes   (comments/whitespace gone, nothing optimized)
ADVANCED:         56 bytes   (DCE + inline + fold + rename)  → ~71% smaller
```

## Files

- `flags.txt` — `--compilation_level ADVANCED --js input/a.js`.
- `input/a.js` — the four-function module (heavily commented; comments are
  stripped by the compiler and do not affect the measured output).
- `expected.stdout` — the byte-exact ADVANCED output.

The integration test `tests/diff_advanced_bigpass.rs` asserts byte-exact stdout,
the four passes' evidence, the runtime-equivalence literals, the ADVANCED-vs-
SIMPLE rename differential, the >50%-vs-WHITESPACE_ONLY optimization savings, and
a guard that ADVANCED did not fall back to the whitespace re-stitcher.
