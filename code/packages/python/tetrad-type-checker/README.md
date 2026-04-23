# tetrad-type-checker

Stage 3 of the [Tetrad](../../specs/TET00-tetrad-language.md) pipeline. Walks the AST from `tetrad-parser` bottom-up, infers types for every expression, classifies functions into three tiers, and produces a `TypeCheckResult` that the compiler uses to decide whether to emit feedback slots.

## Pipeline position

```
source → [lexer] → [parser] → [type-checker] → [compiler] → [vm] → [jit]
                                     ↑ you are here
```

## Why types matter

```
Untyped:  fn add(a, b) { return a + b; }
  → compiler emits ADD r0, slot=N  (3 bytes per op, feedback slot)
  → JIT compiles after 100 calls

Typed:    fn add(a: u8, b: u8) -> u8 { return a + b; }
  → compiler emits ADD r0  (2 bytes per op, NO slot)
  → JIT compiles BEFORE the first call
```

## Three-tier system

| Status | Condition | JIT trigger |
|---|---|---|
| `FULLY_TYPED` | All params + return annotated; all body ops infer to u8 | Before call 1 |
| `PARTIALLY_TYPED` | Some annotations, or ops yield Unknown | After 10 calls |
| `UNTYPED` | No annotations | After 100 calls |

## Public API

```python
from tetrad_type_checker import check, check_source, TypeCheckResult

result = check_source("fn add(a: u8, b: u8) -> u8 { return a + b; }")
print(result.env.function_status["add"])   # FunctionTypeStatus.FULLY_TYPED
print(result.errors)                        # []
```

## Spec

See [`code/specs/TET02b-tetrad-type-checker.md`](../../specs/TET02b-tetrad-type-checker.md).
