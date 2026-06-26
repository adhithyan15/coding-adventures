# Fixture: `simple-fold-substring`

End-to-end oracle for folding `String.prototype.substring(start[, end])` on a
string-literal receiver at `--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | `"abcd".substring(1, 3)`, `"abcd".substring(3, 1)`, `"abcd".substring(-2)`, `"abcd".substring(10)` |
| `expected.stdout` | The folded output: `var a="bc";var b="bc";var c="abcd";var d="";report(a,b,c,d);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose `constant-fold`
pass folds a `"…".substring(startLit[, endLit])` call into the substring string
literal (ECMAScript §22.1.3.24). `substring` is the sibling of the already-folded
`slice`, but with two distinct semantics that this fixture is chosen to exercise:

- `"abcd".substring(1, 3)` → `"bc"` — the plain half-open range `[1, 3)`;
- `"abcd".substring(3, 1)` → `"bc"` — `start > end`, so the endpoints **swap**
  (this is where `substring` and `slice` differ on argument *order*);
- `"abcd".substring(-2)` → `"abcd"` — a negative argument **clamps to 0**; it
  never counts from the end the way `slice` does (`"abcd".slice(-2)` is `"cd"`);
- `"abcd".substring(10)` → `""` — a start past the end clamps to `len`.

Indices are UTF-16 code units. The fold is left for the runtime (the call
survives) when an argument is not an integer literal (a fractional value would
need `ToInteger` coercion we don't model), when there are more than two
arguments, or when the cut would split a surrogate pair into a lone surrogate.
The same input under `WHITESPACE_ONLY` keeps the calls intact.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-substring/input/a.js \
    > tests/diff/simple-fold-substring/expected.stdout
```
