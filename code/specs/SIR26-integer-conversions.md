# SIR26 — Integer conversions: the `Convert` node

## Status

Core-IR extension.  Adds one expression node — `Expr::Convert` — and one
feature — `Feature::Conversions` — to the narrow-waist Semantic IR
([SIR10](SIR10-narrow-waist-semantic-ir.md)), building on the integer type
system of [SIR21](SIR21-type-system-and-integer-semantics.md).

This is the mechanism that lets SIR **carry a source language's integer
width / wrapping / truncating behaviour** across the waist.  It is the enabling
node for the **C → SIR → Ruby** path: a C frontend (a later spec) inserts
`Convert` nodes per C's conversion rules, and a Ruby backend renders them as
masking so the loose target reproduces the strict source's results exactly.

This spec adds the node and its validation/traversal/text-format support in the
`semantic-ir` crate only.  Backend *rendering* (Ruby, C) and frontend *emission*
(C) land in follow-up PRs; until a backend accepts `Conversions`, a module
containing a `Convert` node is cleanly rejected by the capability check — the
node is inert and additive.

## The node

```rust
// in semantic_ir::nodes::Expr
Convert {
    /// The integer value being converted.
    value: Box<Expr>,
    /// The target integer type.  Its width + signedness define the result;
    /// the overflow mode is carried for the backend's record (see below).
    to: IntSpec,
    span: Span,
}
```

## Semantics — two's-complement reinterpretation

`Convert { value, to }` evaluates `value` (an integer) and reduces it to the
target integer type `to` by **two's-complement reinterpretation**:

1. Let `n = to.width` in bits.  Reduce the value modulo `2ⁿ` (mask to the low
   `n` bits).
2. If `to.signed` and the resulting bit `n−1` is set, subtract `2ⁿ` (sign
   extension), giving a value in `[−2ⁿ⁻¹, 2ⁿ⁻¹−1]`; otherwise the value is in
   `[0, 2ⁿ−1]`.

When `to.width == Arbitrary`, the conversion is the **identity** (a widen into
the unbounded integer — no bits are lost).  This is the one case that never
narrows.

This is exactly what a C cast / implicit integer conversion does under
two's-complement (`-fwrapv` for the signed case):

| example | `to` | reduces `value` … | e.g. |
|---|---|---|---|
| `(uint8_t)x` | `{W8, unsigned}` | mod 2⁸ | `300 → 44` |
| `(int32_t)x` | `{W32, signed}` | mod 2³², sign-fold | `4_000_000_000 → −294_967_296` |
| `(uint32_t)x` | `{W32, unsigned}` | mod 2³² | `−1 → 4_294_967_295` |
| `(int64_t)small` | `{W64, signed}` | value-preserving | `200 → 200` |

**Why the node instead of typed-arithmetic opcodes.** C's integer overflow is
always modular (unsigned = defined wraparound; signed = UB, rendered as
two's-complement wrap).  So arithmetic can stay **exact** (the existing dynamic
`+`/`-`/`*`, which on a bignum target is precise), and a `Convert` inserted
**after each width-bounded operation and at each cast/assignment** enforces the
width.  Narrowing an exact intermediate to width `n` equals C's wrapped result
at width `n` *at every operation*, so a C program and its translation agree
bit-for-bit.  This keeps the core addition to a single node rather than a family
of typed arithmetic ops (`add_i32_wrap`, …).

**Overflow mode.** `to.overflow` is carried but, for the conversions this spec
targets (C: `Wrap` for unsigned, `Undefined`→wrap for signed), the reduction is
always modular — so v0 rendering ignores it.  `Trap` / `Saturate` *conversions*
(a checked narrowing that raises, or a clamping one) are a later generalisation;
a backend that does not implement them may reject a `Convert` whose
`to.overflow` it cannot honour.

## Feature gate

A `Convert` node makes the validator observe **`Feature::Conversions`**
(kebab: `conversions`).  Because `to` is a concrete integer type, the validator
also observes the type-implied features already defined in
[SIR21](SIR21-type-system-and-integer-semantics.md): `SizedIntegers` (target
width ≠ `Arbitrary`), `Unsigned` (target `signed == false`), and
`WrappingArithmetic` (target overflow ≠ `Arbitrary`).  A module using `Convert`
must therefore declare all of these in its manifest, and a backend must accept
them to compile it — so no backend silently mishandles a typed conversion.

## Text format

`Convert` prints as `(convert <to> <value>)`, where `<to>` is the `IntSpec`
text form (SIR21): `(int u8 wrap)`, `(int i32 ub)`, … (and bare `int` for the
arbitrary spec).  Example:

```text
(convert (int u8 wrap) (var-ref t3 local))
```

The printer is inline (no source positions), matching the other expression
nodes.

## How backends will render it (follow-up PRs)

- **Ruby** — inlined mask helpers: `Convert{W8,unsigned}` → `(v) & 0xFF`;
  `Convert{W32,signed}` → `sir_i32(v)` where `sir_i32(v) = ((v & 0xFFFFFFFF) ^
  0x80000000) - 0x80000000`.  Arithmetic stays native bignum.
- **C** — a native cast to the target type (`(uint8_t)(v)`), which already wraps
  on the target hardware; this makes **C → SIR → C** round-trip.
- The existing dynamic backends (Python/JS/Go/Rust) will either render the mask
  or, until they do, simply not accept `Conversions` (clean rejection).

## Out of scope (this spec)

- Backend rendering and C-frontend emission (separate PRs).
- Float ↔ integer conversions (a later `Convert`-family extension).
- `Trap` / `Saturate` narrowing behaviour (a later generalisation).
