# Fixture: `advanced-try-catch-rename`

End-to-end oracle for `--compilation_level ADVANCED` renaming soundness
across a `catch` binding (CLOC19).

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level ADVANCED --js input/a.js` |
| `input/a.js` | A function whose body references its param inside a `try` block and references a local inside the `catch` body, with a catch param `err` |
| `expected.stdout` | The renamed output (see below) |

```text
function c(a){var b;b=a + 1;try{compute(b)}catch(err){report(err,b)}return b};c(7);
```

This fixture pins the **catch-param-soundness** guarantee that makes
ADVANCED renaming safe in the presence of `try`/`catch`:

* `process` ⇒ `c`, `value` ⇒ `a`, `temp` ⇒ `b` — ordinary local/global
  renaming.
* The param use *inside the try block* is rewritten consistently
  (`value + 1` ⇒ `a + 1`) and the local use *inside the catch body*
  becomes `report(err, b)` — the rewrite reaches into both nested
  blocks.
* The catch binding **`err` is preserved verbatim**: it is never
  renamed (catch params are not in the local-rename set) and no other
  local is ever aliased onto it (the catch param joins the fresh-name
  avoid set). If either guard were missing, `err` would collide with a
  generated short name and miscompile the handler.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level ADVANCED \
    --js tests/diff/advanced-try-catch-rename/input/a.js \
    > tests/diff/advanced-try-catch-rename/expected.stdout
```
