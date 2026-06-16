# Fixture: `simple-dce`

End-to-end oracle for the `dce` (dead-code elimination) pass in
`--compilation_level SIMPLE` (CLOC12.157, PR-3).

| File | Role |
|------|------|
| `flags.txt` | `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | A function whose body has dead branches and post-`return` code |
| `expected.stdout` | `function f(){keep();return 1};` |

The SIMPLE pipeline is now `constant-fold → fold-control-flow → dce`. This
fixture shows all three composing inside one function body:

| Source line | Fate |
|-------------|------|
| `keep();` | live, before the `return` — retained |
| `if (4 > 5) { neverRuns(); }` | `4>5`⇒`false` ⇒ `if(false){…}`⇒`;` ⇒ dce sweeps the empty statement |
| `return 1;` | block terminator — retained |
| `alsoDead();` | after the `return` — dce drops it (dead-after-terminator) |

dce runs **last** so it cleans up the empty `;` debris the control-flow
folder leaves behind, and removes unreachable code after a `return`. The same
input under `WHITESPACE_ONLY` keeps every statement verbatim.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-dce/input/a.js \
    > tests/diff/simple-dce/expected.stdout
```
