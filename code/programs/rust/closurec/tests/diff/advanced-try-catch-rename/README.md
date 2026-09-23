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
`fresh_name_avoids_catch_param_unused_in_its_own_body` pins. (Its
older sibling `fresh_name_avoids_colliding_with_catch_param` reads as
though it pins the same thing and does not: its handler mentions its
own binding, so the name reaches the avoid set regardless.) The first
guard is
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

That refusal is not unique to this fixture. Running all 126 fixtures of
`non-minify-unverified-stdout` under their own `flags.txt` against the
pinned oracle, **five** satisfy the predicate of the `upstream_refuses`
disposition added in CCR-081
([#15868](https://github.com/adhithyan15/coding-adventures/issues/15868))
while still being dispositioned `upstream_golden` — this one and the other
four `advanced-*` fixtures (`advanced-bigpass`,
`advanced-class-constructor`, `advanced-optimizes`,
`advanced-rename-globals`), all `JSC_UNDEFINED_VARIABLE`, all zero bytes of
stdout.

They are not in that cohort because the class was never scanned for it: the
twelve were taken from an earlier partition rather than by applying the
predicate to all 138. Their refusals *are* fixable by adding externs, which
is a real distinction from a fixture upstream will not compile however it is
invoked — but that distinction appears nowhere in the recorded predicate,
and `simple-importmeta`, which is in the cohort, has the same kind of
flag-fixable refusal. See #15868 for the follow-up.

The parity cost of reserving the catch binding is tracked as CCR-022
([#15856](https://github.com/adhithyan15/coding-adventures/issues/15856)).
The ladder fixtures had already recorded the same divergence —
`ladder_t4_try_catch_simple`'s README says "upstream renames the catch
parameter `e` to `a`, which closurec skips (CCR-022)", and the `_advanced`
one says the same in different words — so this is a correction catching up
with evidence the repo already had, not a new discovery. Full probes are in
`code/specs/CLOC19-try-catch.md`.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level ADVANCED \
    --js tests/diff/advanced-try-catch-rename/input/a.js \
    > tests/diff/advanced-try-catch-rename/expected.stdout
```
