# Fixture: `simple-negation-fold`

End-to-end oracle for the **negation-push** optimization (upstream Closure's
`PeepholeMinimizeConditions`): `!(a == b)` → `a != b`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | `report(!(a == b), !(a === b), !(a < b))` over non-foldable operands |
| `expected.stdout` | The optimized output (see below) |

```text
var a=first();var b=second();report(a != b,a !== b,!(a < b));
```

## What this proves

* **Negation pushes through equality.** `!(a == b)` → `a != b` and
  `!(a === b)` → `a !== b`. Sound because `!=`/`!==` are *defined* as the
  boolean negation of `==`/`===` (ECMAScript §13.10).

* **Relational operators are NOT inverted.** `!(a < b)` stays `!(a < b)` — it
  is **not** `a >= b`, which would be wrong when an operand is `NaN`
  (`!(NaN < 1)` is `true`, `NaN >= 1` is `false`). This is the NaN-safety
  guard. (It also exercises the unary-precedence fix: the `!` keeps its
  parentheses around `a < b`.)

* **This is the SIMPLE optimizer, not the WHITESPACE_ONLY fallback.** The
  unused `var dead = 8 + 9;` is removed by the typed pipeline; its absence
  proves the program reached the SIMPLE passes.

## Why side-effecting initializers

`a`, `b` are bound to call results (`first()` / `second()`), which the
constant-folder cannot evaluate, so the equality comparison survives as a real
binary expression rather than folding to a boolean literal — keeping the
negation-push rewrite visible in the output.
