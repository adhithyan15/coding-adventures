# Fixture: `simple-fold-strpred`

End-to-end oracle for string-literal substring-**predicate** folding at
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | three calls: `"hello".startsWith("he")`, `.endsWith("xo")`, `.includes("ell")` |
| `expected.stdout` | The folded output: `var a=!0;var b=!1;var c=!0;report(a,b,c);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose `constant-fold`
pass folds the single-argument `String#startsWith` / `endsWith` / `includes`
(ECMAScript §22.1.3.{23,7,9}) on two string literals to a **boolean literal**:
`"hello".startsWith("he")` → `true`, `"hello".endsWith("xo")` → `false`,
`"hello".includes("ell")` → `true`. The whole method call collapses to
`true`/`false`, so no call survives in the output.

These folds are sound for any pair of literals: JS matches by UTF-16 code unit
and Rust by UTF-8 byte, but both operands are valid strings (whole Unicode
scalars, no lone surrogates), so a prefix / suffix / substring relation holds
identically in either encoding. Only the one-argument form folds — the position
overloads (`startsWith(needle, pos)`, etc.) and a non-literal receiver pass
through. The same input under `WHITESPACE_ONLY` keeps the calls unfolded.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-strpred/input/a.js \
    > tests/diff/simple-fold-strpred/expected.stdout
```
