# `simple-fold-array-of` — static `Array.of(...)` → array literal `[...]`

End-to-end fixture proving that, at `--compilation_level SIMPLE`, the typed
constant-fold pass collapses a static `Array.of(v0, v1, …)` call (ECMAScript
§23.1.2.3) to the array literal `[v0, v1, …]`.

| call                | result      | note                                        |
|---------------------|-------------|---------------------------------------------|
| `Array.of()`        | `[]`        | empty argument list                          |
| `Array.of(7)`       | `[7]`       | one element — NOT `Array(7)`'s length-7 array|
| `Array.of(1, 2, 3)` | `[1, 2, 3]` | elements preserved in order                  |
| `Array.of(x, y)`    | `[x, y]`    | identifier args preserved                    |
| `q.of(1)`           | *unfolded*  | only the bare global `Array.of` folds        |

## Soundness

`Array.of` always builds a fresh array whose elements are exactly its arguments,
in order — it is **not** the `Array(n)` constructor, where a single numeric
argument sets the *length* (`Array(7)` is a length-7 hole array, `Array.of(7)`
is `[7]`). Folding `Array.of(a, b, c)` to the array literal `[a, b, c]`
preserves every element expression in evaluation order, so no argument is
dropped, duplicated, or reordered and all side effects are retained — the fold
is exact for **any** argument list. We would decline a spread argument
(`Array.of(...xs)`), whose element count is unknown at compile time, but the AST
has no call-argument spread variant yet, so every argument is a plain expression
and the fold always applies. `Array.of` is a STATIC METHOD, so it dispatches
through the `MemberExpression` callee arm — only the bare global `Array.of(...)`
folds, never a shadowed receiver (`q.of(...)` is left alone).

## Files

- `flags.txt` — CLI flags (`--compilation_level SIMPLE --js input/a.js`).
- `input/a.js` — five `var` bindings flowing into `report(...)` so each stays
  referenced past remove-unused-vars and the fold is observable.
- `expected.stdout` — the byte-exact SIMPLE output:

  ```text
  var a=[];var b=[7];var c=[1,2,3];var d=[x,y];var e=q.of(1);report(a,b,c,d,e);
  ```

The integration test `tests/diff_simple_fold_array_of.rs` runs the binary
against these flags and asserts byte-exact stdout, the four folds (incl. the
critical `Array.of(7)` → `[7]` distinction from `Array(7)`), the declined
non-global receiver, and a regression guard that the typed SIMPLE pipeline ran
(not the WHITESPACE_ONLY fallback): exactly one `Array.of`/`.of(` call remains.
