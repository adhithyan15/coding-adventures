# Fixture: `simple-treeshake`

End-to-end oracle for the `treeshake` pass in `--compilation_level SIMPLE`
(CLOC12.159, PR-C).

| File | Role |
|------|------|
| `flags.txt` | `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | One unused top-level function, one used one |
| `expected.stdout` | `function live(){return 2};log(live());sink(live);` |

The SIMPLE pipeline is now
`constant-fold → fold-control-flow → dce → inline → remove-unused-vars →
treeshake`. The final pass deletes top-level `function`/`class`
declarations that nothing references:

| Source | Fate |
|--------|------|
| `function dead() { return 1; }` | removed — never called |
| `function live() { return 2; }` | kept — referenced; the value use `sink(live)` makes the inliner decline it |

`treeshake` is the function-shaped complement to `remove-unused-vars`
(which deliberately skips functions). Removing an unused function
declaration is always safe — declaring a function has no side effect, so
no purity gate is needed. The same input under `WHITESPACE_ONLY` keeps
both functions.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-treeshake/input/a.js \
    > tests/diff/simple-treeshake/expected.stdout
```
