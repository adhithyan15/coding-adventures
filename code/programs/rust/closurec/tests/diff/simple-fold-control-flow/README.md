# Fixture: `simple-fold-control-flow`

End-to-end oracle for the `fold-control-flow` pass in `--compilation_level
SIMPLE` (CLOC12.156, PR-2).

| File | Role |
|------|------|
| `flags.txt` | `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | Three `if` statements with statically-decidable conditions |
| `expected.stdout` | `{takeThis()}{alsoKept()};` |

The SIMPLE pipeline is now `constant-fold → fold-control-flow`. Each `if`'s
condition is decided at compile time and the dead branch is pruned:

| Source | Result |
|--------|--------|
| `if (2 > 3) { keepElse(); } else { takeThis(); }` | `{takeThis()}` |
| `if (true) { alsoKept(); } else { dropped(); }` | `{alsoKept()}` |
| `if (4 > 5) { vanishes(); }` | `;` (empty statement) |

The `if (2 > 3)` row is the key one: `constant-fold` turns `2 > 3` into
`false`, and only *then* can `fold-control-flow` keep the `else` branch —
proving the two passes compose. The same input under `WHITESPACE_ONLY` keeps
every `if` verbatim.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-control-flow/input/a.js \
    > tests/diff/simple-fold-control-flow/expected.stdout
```
