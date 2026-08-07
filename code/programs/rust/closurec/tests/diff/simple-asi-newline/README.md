# Fixture: `simple-asi-newline`

End-to-end oracle for **CLOC26 Phase 2** — Automatic Semicolon Insertion at a
**line terminator** (ECMAScript ASI Rule 1). Implemented in the
`javascript-parser` `asi` module (retry-on-parse-error).

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | Three statements on separate lines with **no** semicolons |
| `expected.stdout` | The optimized output (see below) |

```text
var w=4,s=3;report(w * s);
```

What this proves:

* **The program parses at all** — before Phase 2, a statement boundary marked
  only by a newline (no `;`) failed the grammar parse and closurec degraded the
  *whole program* to `WHITESPACE_ONLY`. ASI Rule 1 inserts a `;` before each
  token that is preceded by a line terminator, so the typed pipeline runs.
* **`1 + 2` folds to `3`** (`var s=3`) — an optimization only reachable from the
  SIMPLE pipeline. Its presence is the proof ASI worked: a WHITESPACE_ONLY
  fallback keeps `var s=1+2` verbatim.

The `simple_asi_newline_did_not_fall_back_to_whitespace_only` guard asserts the
output contains `s=3` and does **not** contain `1+2`.

Soundness note: ASI Rule 1 fires *only* when a token is preceded by a line
terminator — two statements on the **same** line (`a = 1 b = 2`) remain a
genuine syntax error and are not "recovered" (closurec degrades them, exactly as
before). And because ASI only acts on a real parse failure, any program that
already parses is byte-for-byte unchanged.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-asi-newline/input/a.js \
    > tests/diff/simple-asi-newline/expected.stdout
```
