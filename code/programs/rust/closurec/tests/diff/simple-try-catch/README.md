# Fixture: `simple-try-catch`

End-to-end oracle for `--compilation_level SIMPLE` optimization through a
`try`/`catch`/`finally` statement (CLOC19).

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | A single-use function, plus a `try`/`catch (e)`/`finally` whose blocks hold foldable arithmetic and dead-code-after-`return` |
| `expected.stdout` | The optimized output (see below) |

```text
report(1);try{var x=3;risky(x)}catch(e){var y=12;handle(e,y);return}finally{cleanup()}
```

What this proves now runs **inside** try/catch/finally — none of which
was reachable before CLOC19 (any `try` forced a WHITESPACE_ONLY
fallback):

* **Inlining** — `log` is single-use, so `log(1)` becomes `report(1)`
  and the declaration is deleted.
* **Constant folding** — `1 + 2` ⇒ `3` and `3 * 4` ⇒ `12` even though
  they sit inside the `try` and `catch` blocks.
* **Dead-code elimination** — `dead(99)` after the `return` in the catch
  handler is unreachable and gets truncated.
* **Control-flow preservation** — `try`, `catch (e)`, and `finally`
  survive verbatim; the catch binding `e` is untouched.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-try-catch/input/a.js \
    > tests/diff/simple-try-catch/expected.stdout
```
