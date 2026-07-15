# `simple-fold-number-parse` — static `Number.parseInt/parseFloat` → numeric

End-to-end fixture proving that, at `--compilation_level SIMPLE`, the typed
constant-fold pass collapses the ES2015 static methods `Number.parseInt(string[,
radix])` and `Number.parseFloat(string)` (ECMAScript §21.1.2.12/.13) to a numeric
literal when the single argument is a string literal.

These are the *same function objects* as the global `parseInt`/`parseFloat`
(`Number.parseInt === parseInt`), so they run the identical leading-prefix scan —
the fold reuses the existing `fold_parse_int` / `fold_parse_float` helpers.

| call                              | result      | note                          |
|-----------------------------------|-------------|-------------------------------|
| `Number.parseInt("12px")`         | `12`        | trailing garbage ignored      |
| `Number.parseInt("FF", 16)`       | `255`       | explicit radix                |
| `Number.parseInt("0x1F")`         | `31`        | `0x` prefix → hex             |
| `Number.parseFloat("3.14e2abc")`  | `314`       | leading float prefix          |
| `Number.parseInt("")`             | *unfolded*  | `NaN` has no literal → decline|

## Soundness

These are STATIC METHOD calls, so they dispatch through the `MemberExpression`
callee arm (alongside `String.fromCharCode`/`fromCodePoint` and the
`Number.isInteger` statics) — only the bare global `Number.parseX(...)` folds,
never a shadowed receiver (`n.parseInt(...)` is left alone). As with the global
forms, a `NaN` / `±Infinity` result is DECLINED (JavaScript has no literal token
for either), and `parseInt` only folds with a missing or integer-literal radix.

## Files

- `flags.txt` — CLI flags (`--compilation_level SIMPLE --js input/a.js`).
- `input/a.js` — five `var` bindings flowing into `report(...)` so each stays
  referenced past remove-unused-vars and the fold is observable.
- `expected.stdout` — the byte-exact SIMPLE output:

  ```text
  var a=12,b=255,c=31,d=314,e=Number.parseInt("");report(a,b,c,d,e);
  ```

The integration test `tests/diff_simple_fold_number_parse.rs` runs the binary
against these flags and asserts byte-exact stdout, the per-binding numeric folds
(including the explicit-radix and `0x` cases), the declined `NaN` call, and a
regression guard that the typed SIMPLE pipeline ran (not the WHITESPACE_ONLY
fallback): exactly one `Number.parse…(` call (the declined `NaN`) remains.
