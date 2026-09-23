# Fixture: `advanced-try-catch-rename`

Regression test for `closurec`'s `--compilation_level ADVANCED` renaming
across a `catch` binding (CLOC19). **Not an oracle** — see the note below on
what upstream actually does with this input.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level ADVANCED --js input/a.js` |
| `input/a.js` | A function whose body references its param inside a `try` block and references a local inside the `catch` body, with a catch param `err` |
| `expected.stdout` | The renamed output (see below) |

```text
function c(a){var b=a+1;try{compute(b)}catch(err){report(err,b)}return b}c(7);
```

This fixture pins how `closurec` renames in the presence of `try`/`catch`:

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
oracle for upstream's behaviour.

**Under this fixture's own `flags.txt`, upstream produces nothing at all.**
`--compilation_level ADVANCED --js input/a.js` passes no externs, and
`compute`/`report` are free globals, so the pinned oracle exits 2 with two
`JSC_UNDEFINED_VARIABLE` errors and zero bytes of stdout. *With externs
added* it compiles and inlines `process` away entirely — ADVANCED gives
`try{compute(8)}catch(a){report(a,8)};`, folding `7 + 1`, dropping the
function, and renaming the catch binding. Either way, the bytes in
`expected.stdout` are ours.

That refusal also means this fixture satisfies the predicate of the
`upstream_refuses` disposition added in CCR-081
([#15868](https://github.com/adhithyan15/coding-adventures/issues/15868)),
while still sitting in `non-minify-unverified-stdout` as `upstream_golden`.
It was left out of that cohort because its refusal is harness
incompleteness — adding externs fixes it — but the recorded predicate does
not draw that distinction. See #15868 for the follow-up.

The parity cost of reserving the catch binding is tracked as CCR-022
([#15856](https://github.com/adhithyan15/coding-adventures/issues/15856)).
The `ladder_t4_try_catch_{simple,advanced}` fixtures had already recorded
the same divergence ("upstream renames the catch parameter `e` to `a`, which
closurec skips"), so this is a correction catching up with evidence the repo
already had, not a new discovery. Full probes are in
`code/specs/CLOC19-try-catch.md`.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level ADVANCED \
    --js tests/diff/advanced-try-catch-rename/input/a.js \
    > tests/diff/advanced-try-catch-rename/expected.stdout
```
