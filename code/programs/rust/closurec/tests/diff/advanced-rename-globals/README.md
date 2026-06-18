# Fixture: `advanced-rename-globals`

End-to-end oracle for the `rename-globals` pass — the first place
`--compilation_level ADVANCED` produces **smaller** output than SIMPLE
(CLOC13.I).

| File | Role |
|------|------|
| `flags.txt` | `--compilation_level ADVANCED --js input/a.js` |
| `input/a.js` | A top-level helper that survives SIMPLE (multi-statement body, called) |
| `expected.stdout` | `function a(){sideEffect();return value};a();` |

`helper` is a top-level `function` with a multi-statement body, so `inline`
leaves it, and it is called, so `treeshake` keeps it. The divergence:

| Level | Output |
|-------|--------|
| SIMPLE | `function helper(){sideEffect();return value};helper();` (top-level name kept — it may be externally visible) |
| ADVANCED | `function a(){sideEffect();return value};a();` (`rename-globals` shortens the private `helper` → `a`) |

`sideEffect` / `value` are free globals (not declared here) → never renamed.
A `--externs` file declaring `helper` would keep its name under ADVANCED too,
since the externs set is the do-not-rename boundary that makes the rename
sound.

The companion harness `tests/diff_advanced_rename_globals.rs` runs BOTH levels
on this input and asserts ADVANCED renamed `helper` while SIMPLE kept it.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level ADVANCED \
    --js tests/diff/advanced-rename-globals/input/a.js \
    > tests/diff/advanced-rename-globals/expected.stdout
```
