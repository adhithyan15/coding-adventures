# sir-polymorphic-operators — Ruby `+` / `*` on strings and arrays

## Status

New. Design/spec PR (specs-first). Backlog gap discovered during the #59
class-method work: the `ruby-to-semantic-ir` CHANGELOG explicitly flags a
`_sir_plus` gap on `super + "…"`. Toward the north star (any Ruby → correct
same-result output): Ruby overloads `+` and `*` by receiver type, and today the
tagged-enum backends only implement the **numeric** case.

## Current state (2026-07-01 survey)

Ruby's arithmetic operators are polymorphic:

| Expr | Ruby result |
|---|---|
| `1 + 2` | `3` (numeric) |
| `"a" + "b"` | `"ab"` (String concat) |
| `[1] + [2]` | `[1, 2]` (Array concat) |
| `"ab" * 3` | `"ababab"` (String repeat) |
| `[0] * 3` | `[0, 0, 0]` (Array repeat) |
| `[1,2] * ", "` | `"1, 2"` (Array join — `*` with a String) |

All of these lower to the same SIR `+`/`*` builtins (`_sir_plus` / `_sir_times`),
so the **runtime helper** must dispatch on operand type. Survey of the five
backend runtimes:

| Backend | `+` on strings | `+` on arrays | `*` repeat/join | Mechanism |
|---|---|---|---|---|
| Python (`sir-runtime-core.add`) | ✅ works | ✅ works | ⚠️ `mul` is numeric-fold only — `str*int`/`arr*int` **broken** | native `+=` on `Any` (str/list concat for free); `*` uses numeric `*=` |
| JS (inline) | ✅ native `+` | ⚠️ JS `[]+[]` → `""` **wrong** | ⚠️ broken | native `+` (string ok, array wrong) |
| TS (`sir-runtime-core`, ts) | same as JS/py port | ⚠️ audit | ⚠️ audit | — |
| **Go** (inline) | ❌ `_sir_plus` numeric-only → `_sir_as_int("a")` garbage | ❌ | ❌ | `runtime.rs:212` int/float fold only |
| **Rust** (inline) | ❌ `plus` numeric-only → `as_i64` garbage | ❌ | ❌ | `runtime.rs:204` int/float fold only |

So `"Tom" + " with"` (a real OOP e2e we currently sidestep with numeric `+`) is
**wrong in Go/Rust** and array `+`/`*` is wrong or broken in most backends. This
is a correctness gap on the core translation path, not just a missing stdlib
method.

## Design — runtime-only, per backend (NO core-IR, NO frontend change)

Make `plus` and `times` **type-dispatched** in every backend runtime, matching
Ruby exactly:

- **`+`**: if the first operand is a **String** → require all operands be
  strings and concatenate (Ruby raises `TypeError` on `"a" + 1`; defer the raise
  to the [[sir-typed-runtime-errors]] cascade — for now concat via display is
  acceptable only if it matches the reference, else coerce-reject). If the first
  operand is a **Seq/array** → concatenate element lists (new array, no aliasing).
  Otherwise the existing numeric fold (int/float promotion) is unchanged.
- **`*`**: **String × Integer** → repeat (`"ab"*3`). **Seq × Integer** → repeat
  the element list. **Seq × String** → join with the separator. Otherwise numeric
  fold.
- Numeric `+`/`*` semantics (int/float promotion, variadic fold) are preserved
  exactly — this is purely *adding* the string/array arms ahead of the numeric
  path, dispatched on the **runtime tag** (Go type switch, Rust `match`, JS
  `typeof`/`Array.isArray`, Python `isinstance`), never reflection
  ([[dynamic-dispatch-rce]]).

Arity note: Ruby `+`/`*` are binary. The SIR builtins are variadic (numeric fold);
the string/array arms fold left-associatively over ≥2 operands, preserving the
existing variadic contract.

## Milestones (one PR per backend runtime — disjoint, parallelizable)

| # | Crate(s) | Content |
|---|---|---|
| PO0 | `code/specs/` | this spec |
| PO1 | `sir-runtime-core` (py) | `mul` string/array arms (`add` already concats via `+=`; add array-repeat + string-repeat) |
| PO2 | `sir-runtime-core` (ts) + `semantic-ir-to-typescript` | string/array `+`/`*`; fix `[]+[]` |
| PO3 | `semantic-ir-to-javascript` (inline runtime) | fix array `+` (native `[]+[]` wrong) + `*` |
| PO4 | `semantic-ir-to-go` (inline runtime) | `_sir_plus`/`_sir_times` string+Seq arms via type switch |
| PO5 | `semantic-ir-to-rust` (inline runtime) | `plus`/`times` `Value::Str`/`Value::Seq` arms |

Each: unit tests + **execution-proof** through the native toolchain
(`"a"+"b"→"ab"`, `"ab"*3→"ababab"`, `[1]+[2]→[1,2]`, `[0]*3→[0,0,0]`,
`[1,2]*", "→"1, 2"`), matching the reference backend, plus a regression that
numeric `+`/`*` is unchanged. Security-review gate (type-dispatch is explicit,
no reflection). Cross-backend parity: one golden operator suite through all 5.
Sequencing: these touch the same backend `runtime.rs` files as the
[[sir-typed-runtime-errors]] T1–T5 cascade and #60 puts — serialize per crate to
avoid rebase thrash.

## Out of scope

- `TypeError` on `"a" + 1` (belongs to [[sir-typed-runtime-errors]]).
- `-`/`/`/`%` overloads (Ruby has few; `%` string-format is a separate large
  feature — string interpolation/format cascade).
- Operator methods defined on user classes (`def +(other)`) — that's OOP operator
  dispatch, a later mixin/metaprogramming item.
