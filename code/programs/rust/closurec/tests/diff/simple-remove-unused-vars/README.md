# Fixture: `simple-remove-unused-vars`

End-to-end oracle for the `remove-unused-vars` pass in
`--compilation_level SIMPLE` (CLOC12.158, PR-B).

| File | Role |
|------|------|
| `flags.txt` | `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | Three top-level `var`s — one dead, one live, one impure |
| `expected.stdout` | `var live=10,impure=run();log(live);` |

The SIMPLE pipeline is now
`constant-fold → fold-control-flow → dce → inline → remove-unused-vars`.
The final pass deletes top-level bindings nothing references, when their
initializer is side-effect-free:

| Source | Fate |
|--------|------|
| `var dead = 1 + 2;` | removed — `constant-fold` folds it to the literal `3`, then `remove-unused-vars` drops the dead, pure-init declaration |
| `var live = 10;` | kept — referenced by `log(live)` |
| `var impure = run();` | kept — unreferenced, but the call initializer may have a side effect (purity gate) |

The `var dead` row proves `constant-fold` and `remove-unused-vars`
compose: the comparison must be folded to a literal before the binding
reads as a pure, removable declaration. `inline` (an identity pass today)
is in the pipeline only because `remove-unused-vars` declares
`depends_on = ["dce", "inline"]`. The same input under `WHITESPACE_ONLY`
keeps every declaration verbatim.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-remove-unused-vars/input/a.js \
    > tests/diff/simple-remove-unused-vars/expected.stdout
```
