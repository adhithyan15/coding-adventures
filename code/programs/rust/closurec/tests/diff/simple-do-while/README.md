# Fixture: `simple-do-while`

End-to-end oracle for `--compilation_level SIMPLE` optimization through a
`do`/`while` loop (CLOC20).

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | A single-use function, plus a `do { … } while (cond)` whose body holds foldable arithmetic, followed by a statement after the loop |
| `expected.stdout` | The optimized output (see below) |

```text
report(1);do{var x=3;step(x)}while(cond);foo();
```

What this proves now runs **through** a do-while — none of which was
reachable before CLOC20 (any `do`-`while` forced a WHITESPACE_ONLY
fallback):

* **Inlining** — `log` is single-use, so `log(1)` becomes `report(1)` and
  the declaration is deleted.
* **Constant folding** — `1 + 2` ⇒ `3` even though it sits inside the
  do-while body.
* **Control-flow preservation** — `do { … } while (cond)` survives
  verbatim.
* **Not a terminator** — `foo()` after the loop stays reachable, because a
  do-while can fall through (the body runs, the test fails, control
  continues).

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-do-while/input/a.js \
    > tests/diff/simple-do-while/expected.stdout
```
