# Fixture: `simple-asi-string-newline`

End-to-end oracle for ASI **Rule 1 across a string-ending statement**. This is
the case the Phase-2 line-terminator heuristic conservatively *declined* (a
string predecessor could span source lines without that showing in its cooked
value). The lexer now records `TOKEN_PRECEDED_BY_NEWLINE` directly on each
token, so ASI reads the flag and the limitation is gone.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | Semicolon-free statements; the first ends in a string literal `"total"` |
| `expected.stdout` | The optimized output (see below) |

```text
var label="total";var n=3;show(label,n);
```

What this proves:

* **The program parses** — a statement ending in a string before a newline now
  ASI-recovers instead of degrading to `WHITESPACE_ONLY`.
* **`1 + 2` folds to `3`** (`var n=3`) — an optimization only reachable from the
  SIMPLE pipeline; a whitespace-only fallback keeps `1+2`.

The `simple_asi_string_newline_did_not_fall_back_to_whitespace_only` guard
asserts `n=3` is present and `1+2` is absent.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-asi-string-newline/input/a.js \
    > tests/diff/simple-asi-string-newline/expected.stdout
```
