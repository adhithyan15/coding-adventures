# Fixture: `simple-for-in`

End-to-end oracle for `--compilation_level SIMPLE` optimization through a
`for`-`in` loop (CLOC22).

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | A single-use function, plus a `for (var key in obj) { … }` whose body holds foldable arithmetic, followed by a statement after the loop |
| `expected.stdout` | The optimized output (see below) |

```text
report(1);for(var key in obj){var x=3;use(key,x)}after();
```

What this proves now runs **through** a for-in loop — none of which was
reachable before CLOC22 (any `for`-`in` forced a WHITESPACE_ONLY fallback):

* **Inlining** — `log` is single-use, so `log(1)` becomes `report(1)` and the
  declaration is deleted.
* **Constant folding** — `1 + 2` ⇒ `3` even though it sits inside the loop body.
* **Control-flow preservation** — `for (var key in obj) { … }` survives
  verbatim, including the loop variable.
* **Not a terminator** — `after()` after the loop stays reachable, because a
  for-in body may run zero times.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-for-in/input/a.js \
    > tests/diff/simple-for-in/expected.stdout
```
