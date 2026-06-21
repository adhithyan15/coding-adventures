# Fixture: `simple-asi-block`

End-to-end oracle for **CLOC26 Phase 1** — Automatic Semicolon Insertion before
a `}` / end-of-input. Implemented in the `javascript-parser` crate's `asi`
module (retry-on-parse-error).

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | A function whose body omits the `;` after `return w * s` (no `;` before the `}`) |
| `expected.stdout` | The optimized output (see below) |

```text
function area(w){var s;s=3;return w * s};report(area(10));
```

What this proves:

* **The program parses at all** — before ASI, a missing `;` before the `}`
  failed the grammar parse and closurec degraded the *whole program* to
  `WHITESPACE_ONLY`. ASI supplies the `;`, so the typed pipeline runs.
* **`1 + 2` folds to `3`** (`var s; s = 3;`) — an optimization that can *only*
  come from the SIMPLE pipeline. Its presence is the proof ASI worked: a
  WHITESPACE_ONLY fallback would keep `var s=1+2` verbatim.

The `simple_asi_block_did_not_fall_back_to_whitespace_only` guard asserts the
output contains `s=3` and does **not** contain `1+2`.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-asi-block/input/a.js \
    > tests/diff/simple-asi-block/expected.stdout
```
