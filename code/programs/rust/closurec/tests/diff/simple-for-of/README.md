# Fixture: `simple-for-of`

End-to-end oracle for `--compilation_level SIMPLE` optimization through a
`for`-`of` loop (CLOC23).

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | A single-use function, plus a `for (const item of items) { … }` whose body holds foldable arithmetic, followed by a statement after the loop |
| `expected.stdout` | The optimized output (see below) |

```text
report(1);for(const item of items){var x=3;use(item,x)}after();
```

What this proves now runs **through** a for-of loop — none of which was
reachable before CLOC23 (any `for`-`of` forced a WHITESPACE_ONLY fallback):

* **Inlining** — `log` is single-use, so `log(1)` becomes `report(1)` and the
  declaration is deleted.
* **Constant folding** — `1 + 2` ⇒ `3` even though it sits inside the loop body.
* **Control-flow preservation** — `for (const item of items) { … }` survives
  verbatim, including the loop variable.
* **Not a terminator** — `after()` after the loop stays reachable, because a
  for-of body may run zero times (an empty iterable).

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-for-of/input/a.js \
    > tests/diff/simple-for-of/expected.stdout
```
