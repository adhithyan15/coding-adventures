# Fixture: `simple-asi-restricted`

End-to-end oracle for **ASI Phase 3 — restricted productions** (ECMAScript
§12.10.1) at `--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | `function f(){return` ⏎ `42}` ⏎ `report(f())` |
| `expected.stdout` | `function f(){return};report(f());` |

A line terminator is **not allowed** between `return` and its argument, so

```js
function f(){return
42}
```

is `function f(){ return; 42; }` — an empty return followed by the (now dead)
expression statement `42` — **not** `return 42`. closurec's grammar is
newline-blind, so without Phase 3 it would parse `return 42` and re-emit it: a
silent miscompile that changes what `f` returns. The Phase-3 pre-pass
(`force_restricted_semicolons` in `javascript-parser`) forces the semicolon, the
typed SIMPLE pipeline then drops the dead `42`, and the output is
`function f(){return};report(f());`.

The **absence of `42`** is the double proof: it is gone only because (a) the
restricted production was honored (`return` did not swallow it) and (b) the
SIMPLE typed pipeline ran (the `WHITESPACE_ONLY` re-stitcher cannot remove it —
it would emit `function f(){return 42}`, the very miscompile this guards
against).

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-asi-restricted/input/a.js \
    > tests/diff/simple-asi-restricted/expected.stdout
```
