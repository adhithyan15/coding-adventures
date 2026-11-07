# sir-floored-modulo — Floored binary `%` semantics across SIR backends

## Status

Active design & implementation specification. Addresses item 2 of #18496
(SIR open issues triage, follow-up to #18482 and #18490).

## Motivation

In Python and Ruby, the binary `%` operator denotes **floored modulo**, mathematically
coupled to floored division (`div_floor` / `//`):

$$r = a - b \cdot \lfloor a / b \rfloor$$

Under floored modulo:
- The remainder $r$ has the same sign as the divisor $b$ (or is zero).
- The identity $(a // b) \cdot b + (a \% b) == a$ holds universally.

Concrete test vectors:
| Expression | Truncated Remainder (C/JS/Go/Rust `%`) | Floored Modulo (Python/Ruby `%`) |
|---|---|---|
| `7 % 3` | `1` | `1` |
| `-7 % 3` | `-1` | `2` |
| `7 % -3` | `1` | `-2` |
| `-7 % -3` | `-1` | `-1` |

When `python-to-semantic-ir` lowers `a % b`, it emits `BuiltinCall("%", [lhs, rhs])`.
However, across the seven SIR backends, handling of `"%"` is inconsistent or missing:

1. **Ruby (`semantic-ir-to-ruby`)**: Emits native `(a % b)`, which in Ruby is already
   floored modulo. Correct.
2. **JavaScript (`semantic-ir-to-javascript`)**: `runtime.rs` provides `mod(a, b)` via
   JS native `numOf(a) % numOf(b)`, which computes *truncated* remainder. Furthermore,
   division by zero produces `NaN` instead of raising `ZeroDivisionError`.
3. **TypeScript (`semantic-ir-to-typescript` / `sir-runtime-core`)**: `builtins` dispatch
   table has no `"%"` entry at all; callers fail at runtime.
4. **C (`semantic-ir-to-c`)**: Has `_sir_itmod` for `tmod` (truncating remainder), but
   does not recognize `"%"` in `emit.rs`. Emitting `BuiltinCall("%")` triggers
   `unsupported builtin '%'`.
5. **Go (`semantic-ir-to-go`)**: Does not recognize `"%"` in `emit.rs`; falls through
   to `_sir_call_builtin_by_name`, which panics with `unknown builtin: %`.
6. **Rust (`semantic-ir-to-rust`)**: Does not recognize `"%"` in `emit.rs`.
7. **Python (`semantic-ir-to-python`)**: Emits native `%` or runtime dispatch; Python
   native `%` is already floored modulo.

## Contract Specification

### Operation Signature

```text
BuiltinCall("%", [lhs, rhs])
```

- **Arity**: Exactly 2 arguments.
- **Divisor zero**: If `rhs == 0` (or `0.0`), raises a typed `ZeroDivisionError`
  with message `"integer division or modulo by zero"` or `"float modulo"`.
- **Integer semantics**: For 64-bit signed integers:
  $$q = \lfloor a / b \rfloor$$
  $$r = a - b \cdot q$$
  Equivalently, using native truncated remainder $rem = a \pmod b$:
  If $rem \ne 0$ and $\operatorname{sgn}(rem) \ne \operatorname{sgn}(b)$, then $r = rem + b$, else $r = rem$.
  Care must be taken to prevent integer overflow on `INT64_MIN % -1` (which is mathematically `0`).
- **Float semantics**:
  $$r = a - b \cdot \lfloor a / b \rfloor$$
  If either operand is a float, the result is a float (or boxed float where applicable).
- **Type Coercion / Error**: If operands are neither numeric nor convertible according to
  the runtime's type system, raises `TypeError`.

## Implementation Strategy

### 1. JavaScript (`semantic-ir-to-javascript`)
In `runtime.rs`, update `function mod(a, b)`:
- Check for zero divisor: throw `ZeroDivisionError`.
- Compute `let r = n % d;`
- If `r !== 0 && ((r < 0) !== (d < 0))`, `r += d;`
- Handle `-0` sign fixup (`if (r === 0) r = 0;`).
- Return boxed float if either operand is float, else integer.

### 2. TypeScript (`sir-runtime-core` / `semantic-ir-to-typescript`)
- In `code/packages/typescript/sir-runtime-core/src/arithmetic.ts`, add floored `mod(a: Val, b: Val): Val`.
- In `code/packages/typescript/sir-runtime-core/src/runtime.ts`, add `"%" : mod` to `builtins`.
- In `semantic-ir-to-typescript/src/emit.rs`, route `"%" => "__Sir.mod"`.

### 3. C (`semantic-ir-to-c`)
- In `runtime.rs`, add `SirValue _sir_mod(SirValue a, SirValue b)` implementing floored modulo
  for both `SIR_INT` and `SIR_FLOAT`.
- Route `"%" => ("_sir_mod", 2)` in `fixed_helper` in `emit.rs`.

### 4. Go (`semantic-ir-to-go`)
- In `runtime.rs`, add `_sir_mod(args []Value) Value` (or `_sir_mod(a, b Value) Value`)
  implementing floored modulo and zero-division check.
- Route `"%" => "_sir_mod"` in `emit.rs` and register in `_sir_call_builtin_by_name`.

### 5. Rust (`semantic-ir-to-rust`)
- In `runtime.rs`, implement `pub fn modulo(a: &Value, b: &Value) -> Value` with floored
  modulo arithmetic.
- Route `"%" => ("__sir::modulo", false)` in `emit_builtin_call`.

### 6. Conformance Testing (`sir-conformance`)
- In `sir-conformance/tests/python_statement_forms.rs`, add cross-backend test cases
  for `%` exercising all four sign combinations `(+, +)`, `(-, +)`, `(+, -)`, `(-, -)`,
  as well as zero divisor error handling.
