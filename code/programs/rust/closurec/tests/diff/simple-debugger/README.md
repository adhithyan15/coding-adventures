# Fixture: `simple-debugger`

End-to-end oracle for `--compilation_level SIMPLE` across a `debugger;`
statement: CLOC21 made it representable, **CLOC24 strips it**.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | A single-use function and foldable arithmetic surrounding a `debugger;` statement |
| `expected.stdout` | The optimized output (see below) |

```text
report(1);var x=3;use(x);
```

What this proves — none of which was reachable before CLOC21 (any `debugger`
forced a WHITESPACE_ONLY fallback):

* **Inlining** — `log` is single-use, so `log(1)` becomes `report(1)` and
  the declaration is deleted.
* **Constant folding** — `1 + 2` ⇒ `3`.
* **`debugger;` stripped** — CLOC24 removes the `debugger` statement at
  SIMPLE/ADVANCED, exactly as the upstream Closure Compiler does: a
  development-only breakpoint has no effect on a shipped program. (At
  WHITESPACE_ONLY, which runs no optimization passes, the `debugger` is kept.)

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-debugger/input/a.js \
    > tests/diff/simple-debugger/expected.stdout
```
