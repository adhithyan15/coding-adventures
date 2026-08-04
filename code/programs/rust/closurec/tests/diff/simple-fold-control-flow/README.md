# Fixture: `simple-fold-control-flow`

End-to-end oracle for the `fold-control-flow` pass in `--compilation_level
SIMPLE` (CLOC12.156, PR-2).

| File | Role |
|------|------|
| `flags.txt` | `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | Three `if` statements with statically-decidable conditions |
| `expected.stdout` | `takeThis();alsoKept();` |

The SIMPLE pipeline is now `constant-fold → fold-control-flow`. Each `if`'s
condition is decided at compile time and the dead branch is pruned; the
surviving branch's block is then **flattened** (CLOC12.194 — a bare `{ … }`
with no block-scoped binding is redundant, so its braces are removed and the
statement runs directly in the enclosing list):

| Source | Result |
|--------|--------|
| `if (2 > 3) { keepElse(); } else { takeThis(); }` | `takeThis();` |
| `if (true) { alsoKept(); } else { dropped(); }` | `alsoKept();` |
| `if (4 > 5) { vanishes(); }` | *(removed — empty statement)* |

The `if (2 > 3)` row is the key one: `constant-fold` turns `2 > 3` into
`false`, and only *then* can `fold-control-flow` keep the `else` branch —
proving the two passes compose. Before CLOC12.194 the kept branches emitted as
bare blocks (`{takeThis()}{alsoKept()}`); now they flatten to `takeThis();` and
`alsoKept();`, matching the reference Closure Compiler. The same input under
`WHITESPACE_ONLY` keeps every `if` verbatim.

The third `if (4 > 5)` folds to a bare `EmptyStatement` (`;`); as of CLOC12.195
DCE sweeps that stray top-level `;` out, so the output is exactly
`takeThis();alsoKept();` — byte-identical to the reference Closure Compiler
(before CLOC12.195 a residual `;;` remained). `fold-control-flow` still
deliberately produces the `EmptyStatement`; DCE removes it afterward.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-control-flow/input/a.js \
    > tests/diff/simple-fold-control-flow/expected.stdout
```
