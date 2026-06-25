# Fixture: `simple-fold-substr`

End-to-end oracle for folding the legacy `String.prototype.substr(start[,
length])` on a string-literal receiver at `--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | `"abcde".substr(1, 2)`, `"abcde".substr(1)`, `"abcde".substr(-2)`, `"abcde".substr(10)` |
| `expected.stdout` | The folded output: `var a="bc";var b="bcde";var c="de";var d="";report(a,b,c,d);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose `constant-fold`
pass folds a `"…".substr(startLit[, lengthLit])` call into the substring string
literal (ECMAScript Annex B §B.2.3.1). `substr` completes the slice family
(`slice`, `substring`, `substr`); unlike the other two its **second argument is
a *length*, not an end index**, and this fixture is chosen to exercise that:

- `"abcde".substr(1, 2)` → `"bc"` — start at 1, take 2 code units;
- `"abcde".substr(1)` → `"bcde"` — the length defaults to "the rest";
- `"abcde".substr(-2)` → `"de"` — a negative start counts from the end (then
  clamps to 0);
- `"abcde".substr(10)` → `""` — a start past the end yields the empty string.

Indices are UTF-16 code units. The fold is left for the runtime (the call
survives) when an argument is not an integer literal (a fractional value would
need `ToInteger` coercion we don't model), when there are more than two
arguments, or when the cut would split a surrogate pair into a lone surrogate.
The same input under `WHITESPACE_ONLY` keeps the calls intact.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-substr/input/a.js \
    > tests/diff/simple-fold-substr/expected.stdout
```
