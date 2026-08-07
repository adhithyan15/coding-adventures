# Fixture: `simple-fold-bitnot`

End-to-end oracle for unary bitwise-NOT folding at
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | Four `var` declarations whose initializers are `~<numeric literal>` expressions |
| `expected.stdout` | The folded output: `var a=-6,b=0,c=-6,d=9;report(a,b,c,d);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose
`constant-fold` pass now folds the unary `~` operator on a numeric
literal under ES `ToInt32` semantics — reusing the very same `to_int32`
coercion that the binary `&`/`|`/`^` operators already use, so the two
stay bit-for-bit consistent:

```
~5    →  -6   (~ToInt32(5)  = ~5  = -6)
~-1   →   0   (~ToInt32(-1) = ~-1 =  0)
~5.9  →  -6   (ToInt32 truncates toward zero first → ~5)
~~9   →   9   (double complement is the ToInt32 identity; folds
               bottom-up in one walk: ~9 → -10, then ~-10 → 9)
```

The same input under `WHITESPACE_ONLY` keeps `~5` etc. unfolded (that
level never runs the typed pipeline).

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-bitnot/input/a.js \
    > tests/diff/simple-fold-bitnot/expected.stdout
```
