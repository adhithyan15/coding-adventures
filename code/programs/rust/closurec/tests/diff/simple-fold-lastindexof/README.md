# Fixture: `simple-fold-lastindexof`

End-to-end oracle for folding `String.prototype.lastIndexOf(needle)` on
string-literal operands at `--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | `"abcabc".lastIndexOf("bc")`, `"abcabc".lastIndexOf("z")`, `"abc".lastIndexOf("")`, `"ab".lastIndexOf("b")` |
| `expected.stdout` | The folded output: `var a=4,b=-1,c=3,d=1;report(a,b,c,d);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose `constant-fold`
pass folds a `"…".lastIndexOf("…")` call into the UTF-16 code-unit index of the
**last** occurrence, or `-1` when absent (ECMAScript §22.1.3.9). It is the mirror
of the already-folded `indexOf` (first occurrence), reusing the same machinery
with Rust's `str::rfind`:

- `"abcabc".lastIndexOf("bc")` → `4` — the *last* "bc" (indexOf would give 1);
- `"abcabc".lastIndexOf("z")` → `-1` — absent;
- `"abc".lastIndexOf("")` → `3` — an empty needle yields the string **length**
  (not 0), because it matches at every position and `lastIndexOf` takes the
  highest;
- `"ab".lastIndexOf("b")` → `1`.

Indices are UTF-16 code units, so `"💩x💩x".lastIndexOf("x")` → `5`. The fold is
left for the runtime when either operand is not a string literal or when the
two-argument `fromIndex` overload is used (`lastIndexOf(needle, fromIndex)`).
The same input under `WHITESPACE_ONLY` keeps the calls intact.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-lastindexof/input/a.js \
    > tests/diff/simple-fold-lastindexof/expected.stdout
```
