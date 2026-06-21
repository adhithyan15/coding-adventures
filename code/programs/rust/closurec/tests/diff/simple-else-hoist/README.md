# Fixture: `simple-else-hoist`

End-to-end oracle for **CLOC25** — dropping a redundant `else` after an `if`
consequent that unconditionally terminates (upstream Closure's
`MinimizeExitPoints`). The transform lives in the `fold-control-flow` pass.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | A function whose `if` consequent ends in `return`, with a multi-statement `else` |
| `expected.stdout` | The optimized output (see below) |

```text
function classify(n){if(n < 0){return negative(n)}record(n);return positive(n)};report(classify(5));
```

What this proves:

* **`else` hoisted** — the consequent `{ return negative(n); }` always exits, so
  the `else` body `{ record(n); return positive(n); }` only runs when the test
  was false. `fold-control-flow` lifts it out to right after the `if`, deleting
  the `else` keyword and its braces.
* **Reachable code preserved** — the hoisted statements (`record(n)` and the
  `return positive(n)`) and the trailing `report(classify(5))` all survive; the
  function is not tree-shaken because it is called.

Contrast with `--compilation_level WHITESPACE_ONLY`, which runs no optimization
passes and keeps the `else` verbatim
(`if(n<0)return negative(n);else{record(n);return positive(n)}`). The
`simple_else_hoist_did_not_fall_back_to_whitespace_only` guard asserts the
optimized output contains **no** `else`, so a silent degrade to the whitespace
fallback would fail the test.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-else-hoist/input/a.js \
    > tests/diff/simple-else-hoist/expected.stdout
```
