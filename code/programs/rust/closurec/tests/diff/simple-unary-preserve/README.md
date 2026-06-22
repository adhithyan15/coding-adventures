# Fixture: `simple-unary-preserve`

End-to-end oracle for the **prefix-unary-operator-drop miscompile** fix
(bridge + emitter).

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | Side-effecting `var` inits + a `report(...)` call using `!`, `-`, `~`, and `!(a == b)` |
| `expected.stdout` | The optimized output (see below) |

```text
var a=first();var b=second();var c=third();report(!a,-b,~c,!(a == b));
```

## What this proves

* **Prefix operators survive the typed pipeline.** Before the fix, the bridge
  discriminated the two `unary_expression` grammar alternatives by counting AST
  *child nodes*; because the operator is a *token* (filtered out by
  `node_children`), every prefix-operator form looked like a pass-through and
  the operator was silently dropped — `!a` → `a`, `-b` → `b`, `~c` → `c`. That
  is a **miscompile** at SIMPLE/ADVANCED. The operators now round-trip.

* **Precedence is preserved.** `!(a == b)` keeps its parentheses. Emitting
  `!a == b` would reparse as `(!a) == b` — a different program. `emit_unary`
  now emits its argument at unary binding strength so any lower-precedence
  operand is parenthesised.

* **This is the SIMPLE optimizer, not the WHITESPACE_ONLY fallback.** The
  unused `var dead = 4 + 5;` is removed by the typed pipeline. Its absence in
  the output proves the program reached the SIMPLE passes (WHITESPACE_ONLY
  keeps every byte, including `dead`).

## Why side-effecting initializers

`a`, `b`, `c` are bound to call results (`first()` / `second()` / `third()`),
which the constant-folder cannot evaluate, so the operands stay as identifiers
and the unary operators remain real prefix operators rather than folding to
literals. This keeps the regression target — *the operator itself* — visible in
the output.
