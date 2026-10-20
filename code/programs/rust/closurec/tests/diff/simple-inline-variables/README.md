# Fixture: `simple-inline-variables`

At `--compilation_level SIMPLE`, a top-level `const` is **not** propagated to
its ordinary uses (CCR-078, #15863).

| File | Role |
|------|------|
| `flags.txt` | `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | A top-level `const` bound to a literal, used in two expressions |
| `expected.stdout` | `const RATE=2;total(base*RATE);margin(RATE+1);` |

SIMPLE is open-world. A top-level binding is a property of the global object
that another script may read, so the reference compiler keeps it. It uses a
literal-initialized binding's value only to decide a statement condition
(`if (RATE)`, `RATE && f()`, `RATE ? a() : b()`), which then folds away.
`closure-pass-inline-variables` mirrors that in its open-world mode.

`expected.stdout` was captured from the pinned `closure-compiler-v20260915` jar
(sha256 verified) under Java 21.0.10:

```sh
java -Duser.language=en -Duser.country=US -Duser.timezone=UTC -Dfile.encoding=UTF-8 \
  -jar closure-compiler-v20260915.jar \
  --compilation_level SIMPLE_OPTIMIZATIONS \
  --js tests/diff/simple-inline-variables/input/a.js
```

Before CCR-078 this fixture pinned closurec's own output,
`const RATE=2;total(base*2);margin(3);`, which upstream never produces.
