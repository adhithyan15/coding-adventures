# Fixture: `simple-fold-split`

End-to-end oracle for folding `String.prototype.split(separator[, limit])` on a
string-literal receiver at `--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | `"a,b,c".split(",")`, `"abc".split("")`, `"a,b,c".split(",", 2)`, `"abc".split()` |
| `expected.stdout` | The folded output: `var a=["a","b","c"];var b=["a","b","c"];var c=["a","b"];var d=["abc"];report(a,b,c,d);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose `constant-fold`
pass folds a `"…".split(sepLiteral[, limit])` call into an **array literal** of
the piece strings (ECMAScript §22.1.3.23) — the first fold that produces an
`ArrayExpression` rather than a scalar:

- `"a,b,c".split(",")` → `["a","b","c"]` — split at each separator occurrence;
- `"abc".split("")` → `["a","b","c"]` — empty separator splits into single
  UTF-16 code units;
- `"a,b,c".split(",", 2)` → `["a","b"]` — the optional non-negative integer
  `limit` caps the piece count;
- `"abc".split()` → `["abc"]` — no separator yields the whole string.

The fold is left for the runtime (the call survives) when the separator is not a
string literal (a regex separator needs a regex engine), when the limit is not a
non-negative integer literal, or — for the empty-separator per-code-unit split —
when the receiver holds an astral character whose surrogate pair would produce a
lone surrogate. The same input under `WHITESPACE_ONLY` keeps the calls intact.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-split/input/a.js \
    > tests/diff/simple-fold-split/expected.stdout
```
