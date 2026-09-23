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
  avoid set).

**Only the second guard is a soundness requirement.** Without it, a
generated short name could alias the caught value and miscompile the
handler — that is the case
`fresh_name_avoids_colliding_with_catch_param` pins. The first guard is
a conservative choice. An earlier revision of this file claimed that
dropping *either* would miscompile; that is false for the first, and
upstream Closure is the counterexample: measured against the pinned
oracle it renames catch parameters at both SIMPLE and ADVANCED, and
satisfies the second guard by choosing a non-colliding fresh name
instead of reserving the original.

So this fixture is a regression test for a rule `closurec` chose, not an
oracle for upstream's behaviour. Upstream does not emit this output:
it inlines `process` away entirely, and the catch binding in what
survives is renamed. The parity cost is tracked as CCR-022
([#15856](https://github.com/adhithyan15/coding-adventures/issues/15856));
the probes are in `code/specs/CLOC19-try-catch.md`.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level ADVANCED \
    --js tests/diff/advanced-try-catch-rename/input/a.js \
    > tests/diff/advanced-try-catch-rename/expected.stdout
```
