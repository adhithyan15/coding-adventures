# VER04p — Warrant Floats (WX-FL)

**Status:** Specification only. Nothing here is implemented. It is the normative
text for backlog item WX-FL0. The implementation slices are FL1-S0 to FL1-S7
(section 17). Implementation is ordered last of the four frozen features (design
memo 2, section 5), after WX-G, WX-D2, WX-F, LP-012, the stateful slice S1 and
text T1, so that the softfloat evaluator, which touches all eighteen lanes, does
not hold the shared-crate queue.

**Parent:** [VER04 §2–§8](VER04-warrant-language.md) (§3: integer semantics),
[VER04d](VER04d-warrant-type-system.md),
[VER04h](VER04h-warrant-derived-evidence.md) (domains, budget, controls),
[VER04i](VER04i-polyglot-warrant-evidence.md) (corpus encoding, bindings, lane
runners, the polyglot rule), [VER04j](VER04j-warrant-v1-extensions.md) (header,
fixed-width types, WX-B bitwise, WX-E tables), [VER04k](VER04k-warrant-v1-wave4.md)
(WX-D folds, WX-F drivers), [VER04l](VER04l-warrant-text.md) (style, header `v2`,
stream tags `60`/`61`, LP-012 kinds), [VER04i](VER04i-polyglot-warrant-evidence.md)
section 2.6.1 (the stream-tag and diagnostic-code registry), [VER30](VER30-warrant-expansion-roadmap.md)
(the frozen features), [VER-BACKLOG](VER-BACKLOG.md) LP-012.

**Section references.** As in VER04l, where this document needs a corpus or
binding rule it states a requirement on VER04i; it does not edit VER04i, VER04j,
VER04k, VER04l or VER30. The backlog rows it needs are listed in section 18.

## 0. Why this exists

Warrant v0 to v1 have no floating point. VER30 froze it. The inventory counts
about 26 crates blocked on floats alone and several hundred whose signatures
mention `f32` or `f64` somewhere. Floats are the feature where "the same code in
18 languages" is least true: x87 extended precision, fused multiply-add,
flush-to-zero, engines that rewrite NaN payloads, a language (JavaScript) with
only a double. A contract that says "returns the right float" is meaningless
until it says which bits, and a check that compares a lane's float to the same
lane's float proves only that the lane agrees with itself.

This document therefore does three things and no more:

1. It gives floats a **bit-pattern value model**: a float is 32 or 64 bits, and
   every operation is a pure function on bit patterns defined by IEEE 754-2019.
2. It makes the **oracle a softfloat model written in integers**, evaluated in
   every lane; the host FPU and libm are the subject, never the reference.
3. It makes every float claim **say what it claims**: which equality relation,
   which tolerance (in ulps or relative error), which domain kind, and whether
   the lane's environment was fit to run it at all.

| Added | Where |
|---|---|
| types `F32`, `F64`; literals `f32(0x...)`, `f64(0x...)`; the header `v2` gate | 2, 4 |
| round-to-nearest-even only; canonical NaN; signed zero; subnormals | 3 |
| built-ins `fadd fsub fmul fdiv fsqrt fma fneg fabs copysign f32_of_f64 f64_of_f32 f_to_int f32_of_int f64_of_int bits f32_of_bits f64_of_bits fclass` | 5 |
| three relations `bits_eq`, `same`, `ieee_eq`; `total_le`, `ieee_lt`; `==` is a type error | 4.3 |
| tolerance claims `within_ulps`, `rel_err_le`, and why libm functions appear only through them | 5.4, 6 |
| the softfloat obligations and its own verification | 8 |
| the float environment probe and the `not-live` rule | 9 |
| corpus encoding (bits only; stream tags `62`, `63`) | 10 |
| domains `f32_edge`, `f64_edge`, `f32_all`; honest labels | 11 |
| lane binding contract per language family | 12 |
| faults (sabotage controls), security, the pilot `trig` | 13, 14, 15 |

## 1. Decisions of record (owner, 2026-11-16)

| # | Decision | Where applied |
|---|---|---|
| a | The VER30 freeze on floats is lifted: spec first, with adversarial review before FL1. | this document |
| b | IEEE 754 binary32 and binary64 only. | 2, 16 |
| c | Float values travel in the corpus and manifests as bit patterns only, never as decimal text. | 10 |
| d | Expected values are computed in every lane by an integer-only softfloat model, never by the host libm or host float operations. A lane whose environment fails the float probe reports the rung `not-live`. | 8, 9 |
| e | `==` on a float (or on any type containing one) is a type error. Three relations exist: `bits_eq`, `same` (the default) and `ieee_eq`. | 4.3 |
| f | Tolerance claims are stated in ulps or relative error with the explicit semantics of 5.4. | 5.4 |
| g | Honest labels: `binary64` is never `full`; `binary32` is `full` only on a declared 2^32 domain. Fail closed. | 11 |
| h | Pilot: `trig`. | 15 |

Section 19 lists what remains open. The rounding mode is decided in 3.2 (fixed;
no mode control), which the memo had left to default.

## 2. Versioning and compatibility

A document is a **v2 document** exactly when its header is `warrant name v2`
(VER04l 2.1). Floats join `v2`; this document defines only its own elements.
Floats add **no new token**: every float construct is a type name or a call. In a
v0 or v1 document those names are therefore ordinary identifiers, exactly as
before (an undeclared one is W0202, a declared one keeps its meaning), which is
what keeps the byte-identity gate true. A float construct in a v2 document that
does not use text is legal; text and floats are independent (section 17 FL1-S0 confirms the
header parser accepts a v2 document using either, both or neither).

| Element | Kind | In a v0/v1 document | In a v2 document |
|---|---|---|---|
| `F32`, `F64` | type names | ordinary identifiers | reserved (W0201/W0203 as a declaration or binder name) |
| every name of 4.2, 4.3 and 5: the 18 operations, the relations and orders `bits_eq same ieee_eq ieee_lt ieee_le total_le`, the predicates `within_ulps rel_err_le`, the literal heads `f32 f64` and the named constants | function names | ordinary identifiers; an undeclared call is W0202 | reserved (W0201/W0203); FL1-S0 checks the in-repo v1 documents for collisions, as VER04l 2.2 does |
| `f32(0x..)`, `f64(0x..)` | literals | **not** W0101: the text is an ordinary call of a function named `f32` (W0202 if undeclared; a v1 model named `f32` keeps working). Unlike VER04l's `t"..."` this is not a new token | literals (4.2) |
| decimal literals such as `1.5f64` | none | W0101 (as today) | **not accepted at all** in FL1 (open question FL-9); write the bits |
| binding keys `float_env=`, `nonfinite=`, `payloads=` | schema keys | unreachable | required or allowed as 9 and 12 say |

The byte-identity tests of VER04j 1.3 and VER04l 2.3 stay in force: prelude parts
for floats (the softfloat model, the probe, the float domains) are emitted only
for a document that uses a float type, exactly as the `Text` parts are.

## 3. The semantic model

### 3.1 Formats

A value of `F32` is a 32-bit pattern, of `F64` a 64-bit pattern, interpreted by
IEEE 754-2019 binary32 and binary64. **All 2^32 and 2^64 patterns are values**,
every NaN payload included. The model has no "invalid float".

```text
binary32:  1 sign | 8 exponent (bias 127)  | 23 fraction
binary64:  1 sign | 11 exponent (bias 1023) | 52 fraction

exponent all ones, fraction 0      -> infinity (signed)
exponent all ones, fraction != 0   -> NaN (fraction MSB 1 = quiet, 0 = signalling)
exponent 0,        fraction 0      -> zero (signed)
exponent 0,        fraction != 0   -> subnormal: (-1)^s * 0.fraction * 2^(1 - bias)
otherwise                          -> normal:    (-1)^s * 1.fraction * 2^(e - bias)
```

Worked example, binary64:

| Pattern | Meaning |
|---|---|
| `0x3ff0000000000000` | 1.0 |
| `0x0000000000000000` / `0x8000000000000000` | +0 / -0 |
| `0x0000000000000001` | 2^-1074, the smallest positive subnormal |
| `0x0010000000000000` | 2^-1022, the smallest positive normal |
| `0x7ff0000000000000` | +infinity |
| `0x7ff0000000000001` | a **signalling** NaN, payload 1 |
| `0x7ff8000000000001` | a **quiet** NaN, payload 1 |
| `0x7ff8000000000000` | the canonical NaN (3.3) |

### 3.2 Rounding and exceptions

Every arithmetic result is correctly rounded under **roundTiesToEven** (IEEE
"round to nearest, ties to even"). **There is no rounding-mode control** in the
language, the corpus or the bindings: the mode is part of the semantics, not of
the environment. A lane whose environment is in another mode fails the probe
(section 9). There are no exception flags and no traps: an invalid operation
yields the canonical NaN, division by zero yields a signed infinity, overflow
yields infinity, underflow yields a subnormal or zero, and none of these is
observable except through the result. **Subnormals are fully supported**:
results below 2^-1022 (2^-126) are gradual, never flushed.

Three points that are otherwise left to the reader:

* **Overflow threshold.** With precision `p` (24 or 53) and `emax` (127 or 1023),
  an exact result of magnitude `>= 2^emax * (2 - 2^-p)` rounds to infinity; below
  it, to the largest finite. For binary64 the threshold is `2^1024 - 2^970` (the
  midpoint between the largest finite and 2^1024, a tie that goes to the even
  side, which is infinity). The same rule serves `f32_of_f64` and `f*_of_int`.
* **Tininess is not observable.** IEEE lets an implementation detect underflow
  before or after rounding; the choice changes only the underflow flag, never the
  result. Flags are out of scope, so the model has nothing to choose.
* **Flags are out of scope for the subject too.** No claim can mention a flag, and
  a subject that raises a flag is not wrong. A subject that **traps** is a
  different matter: see P13 (9.1).

Worked rounding example (binary64). Doubles near 1.0 are spaced 2^-52 apart.

```text
a = 1.0                       = 0x3ff0000000000000
b = 1 + 2^-52  (next double)  = 0x3ff0000000000001

fadd(a, 2^-53):   exact sum 1 + 2^-53 is the midpoint of a and b.
                  Tie -> even fraction (last bit 0)   => 0x3ff0000000000000  (a)
fadd(b, 2^-53):   exact sum 1 + 3*2^-53 is the midpoint of b and 1 + 2^-51.
                  Tie -> even                         => 0x3ff0000000000002
```

The first line shows why "round half up" is a fault; the second why "always
round down" is. Both are controls (13). A second, everyday example:
`fadd(0.1, 0.2)` with `0.1 = 0x3fb999999999999a` and `0.2 = 0x3fc999999999999a`
returns `0x3fd3333333333334`, one ulp above `0.3 = 0x3fd3333333333333`. That
is correct IEEE behaviour; it is exactly why `==` is not offered (4.3).

Binary32 has the same shape: `fadd(f32(0x4b800000), f32(0x3f800000))` is
16777216 + 1, a tie, and returns `0x4b800000` (even), while
`16777216 + 2` returns `0x4b800001`.

### 3.3 NaN canonicalisation

Hardware disagrees on NaN results: x86 SSE produces the "default NaN" with the
**sign bit set** (`0xfff8000000000000`), ARM and RISC-V produce the positive
canonical NaN, and some engines rewrite payloads on a move. The model removes
the disagreement by definition:

* **Every arithmetic operation whose result is a NaN returns the canonical NaN**:
  `0x7ff8000000000000` (F64) or `0x7fc00000` (F32). Operand payloads and signs
  are **not** propagated by arithmetic. A **signalling** NaN operand is a NaN
  like any other: no trap, no flag, canonical result. This covers `fma(0, inf, c)`
  and `fma(inf, 0, c)` for every `c`, a NaN `c` included (IEEE leaves the flag
  optional there; the result is a NaN either way).
* **Payload-carrying operations are the bit operations only**: `fneg`, `fabs`,
  `copysign`, `bits`, `f64_of_bits`/`f32_of_bits`, and transport (the corpus).
  `fneg(0x7ff8000000000001)` is `0xfff8000000000001`: payload kept, sign flipped.
* `f32_of_f64` of a NaN returns the canonical F32 NaN (the payload is not
  truncated into the narrower format, because truncation can produce an infinity);
  `f64_of_f32` of a NaN returns the canonical F64 NaN.

Because lanes cannot be trusted to reproduce a payload, a claim about the bits of
a **computed** NaN is made under `same` (4.3), which identifies all NaNs. A claim
about a payload is allowed only for the payload-carrying operations, and only in
lanes that declare `payloads=preserved` (12); in others it is `not-live` (JS
engines are the standing example). A host may also **quiet** a signalling NaN
merely by moving it (an x87 load, a C `float` promoted to `double` in a variadic
call, a float32 held in a double in the f64-only lanes of 12.2). A quieted NaN is
still a NaN, so `same` is unaffected; only payload claims are, and P8 decides.

### 3.4 Signed zero

+0 and -0 are distinct values with distinct bits and distinct behaviour:

```text
fdiv(1, +0) = +infinity          fdiv(1, -0) = -infinity
fadd(+0, -0) = +0 (RNE)          fadd(-0, -0) = -0
fmul(-1, +0) = -0                fsqrt(-0)    = -0
copysign(3, -0) = -3             ieee_eq(+0, -0) = true     same(+0, -0) = false
```

The complete zero rules, so no operation is left to inference:

| Operation | Sign of a zero result |
|---|---|
| `fadd`, `fsub` | an exact zero sum of operands with **opposite** effective signs is `+0` (so `fsub(x, x) = +0`, `fadd(-0, +0) = +0`); with **equal** effective signs it keeps that sign (`fadd(-0, -0) = -0`, `fsub(-0, +0) = -0`). A zero produced by **rounding** cannot occur in add or sub (a tiny sum is exact) |
| `fmul`, `fdiv` | always the XOR of the operand signs, for zeros, infinities and underflowed results alike (`fmul(-tiny, tiny)` underflows to `-0`) |
| `fma(a, b, c)` | if `a*b + c` is **exactly** zero: the product sign `sign(a) XOR sign(b)` when it equals `sign(c)`, else `+0`. If the exact value is non-zero and rounds to zero: the sign of the exact value |
| `fsqrt` | `fsqrt(+0) = +0`, `fsqrt(-0) = -0` |
| `f32_of_f64`, `f64_of_f32` | the sign is kept, including when `f32_of_f64` underflows to zero |
| `f32_of_int`, `f64_of_int` | `0` gives `+0`. No `Int` gives `-0` |
| `f_to_int` | `-0` and every value in `(-1, 0)` give `Some(0)` |

`fadd(x, x_negated)` for any finite x is **+0** under roundTiesToEven. A
"sin(-0) must be -0" example is therefore a real contract and a classic bug
(`flip-zero-sign`, 13).

### 3.5 Relationship to integers (VER04 section 3)

`Int` is mathematical; `Nat` is Int refined; `U8..U64` and `I8..I64` wrap. None
of them converts to a float implicitly (4.4). `bits(F64) -> U64` and
`bits(F32) -> U32` are exact and total, as are `f64_of_bits(U64)` and
`f32_of_bits(U32)`; they are the only bridge that is a bijection and they are how
every float enters and leaves the corpus.

## 4. Surface syntax

### 4.1 Types

`F32` and `F64` are primitive types. They are **not** orderable by `<`, not
hashable, not valid map keys or set elements (collections are a separate
specification), and not admitted as `Nat`/`Int` anywhere.

### 4.2 Literals

```text
F32_LITERAL = "f32" "(" HEX_LITERAL_u32 ")"        -- f32(0x3f800000)
F64_LITERAL = "f64" "(" HEX_LITERAL_u64 ")"        -- f64(0x3ff0000000000000)
```

The operand is exactly a lowercase hex literal of exactly 8 (resp. 16) digits.
`f64(0x3ff)` is W0101 (the width is visible in the digits, as for VER04j hex).
Any of the 2^32/2^64 patterns, NaN payloads and infinities included, is a literal.
Named conveniences (`f64_zero`, `f64_one`, `f64_inf`, `f64_nan` = canonical) are
nullary built-ins with fixed bits and are spelled out in the grammar artifact.

### 4.3 Equality, order and the three relations

**`==` and `!=` on `F32`, `F64` or any type containing one (a list of floats, a
record with a float field, `Option<F64>`) are type errors (W0301)**, with the
message "float equality needs a relation: bits_eq, same or ieee_eq". This follows
the `len(Text)` precedent (VER04l decision b): the unit of comparison is always
written. The relations:

| Relation | Definition | NaN | Zeros | Lifts through composites |
|---|---|---|---|---|
| `bits_eq(a, b)` | the bit patterns are equal | `NaN(p)` equals `NaN(p)` only | `+0` differs from `-0` | yes (component-wise) |
| `same(a, b)` | bits equal after mapping **every** NaN to the canonical NaN of its width | all NaNs equal each other | `+0` differs from `-0` | yes |
| `ieee_eq(a, b)` | IEEE 754 `compareQuietEqual` | a NaN is unequal to everything, itself included | `+0` equals `-0` | scalars only (a composite needs a fold) |

`same` is the **default** comparison: the one the generated claims, the
`expect` of an op binding and the docs use unless the author writes another. Why:
it is reflexive, symmetric and transitive (so a result can be stored, hashed and
diffed), it separates the zeros (the thing authors forget), and it is blind to
the NaN payload that hardware does not agree on. `bits_eq` on a value that the
model computed through arithmetic and that may be a NaN is an **advisory** (no new
diagnostic code is allocated, following VER04l 2.4): "bit-compare of a possible
NaN; arithmetic NaNs are canonical, the lane's may not be", printed in the
assurance report as an open exemption (FL-1). It is noisy, not unsound: the claim
fails closed when the lane's NaN bits differ. `ieee_eq` is reflexive
only on non-NaN; it is offered because specifications often say "equal" to mean
this, and the author must then say so.

Order predicates (total functions to `Bool`, never operators in FL1):
`ieee_lt(a, b)` and `ieee_le(a, b)` are the IEEE partial order (false if either is
NaN; `-0` and `+0` are neither less nor greater, so `ieee_le(-0, +0)` and
`ieee_le(+0, -0)` are both true); `total_le(a, b)` is IEEE `totalOrder`:
`-NaN < -inf < ... < -0 < +0 < ... < +inf < +NaN`. Its definition is one line of
integer arithmetic, so it cannot be implemented two ways: with `mag(u)` the bits
without the sign bit, let `key(u) = mag(u)` if the sign bit is 0 and
`key(u) = -mag(u) - 1` if it is 1; then `total_le(a, b)` iff `key(a) <= key(b)`
over `Int`. Consequences, all stated: `-0` (key -1) is below `+0` (key 0);
a positive signalling NaN is below a positive quiet NaN, and the reverse for
negative ones; NaNs **are** distinguished by payload, so `total_le` is a total
order with respect to `bits_eq`, **not** `same`, and a claim that sorts computed
NaNs by `total_le` has the payload caveat of 3.3. `<` and `<=` on floats are
W0301, so no author can silently write a partial order where they meant a total
one.

Worked example.

```text
a = f64(0x7ff8000000000000)   -- canonical quiet NaN
b = f64(0xfff8000000000001)   -- a negative NaN with payload 1
bits_eq(a, b)  = false        same(a, b)  = true       ieee_eq(a, b) = false
z = f64(0x0000000000000000)   w = f64(0x8000000000000000)
bits_eq(z, w)  = false        same(z, w)  = false      ieee_eq(z, w) = true
```

### 4.3.1 Float binders and results in claims

A `forall x: F64` claim states its relation in its body. A binding's `expect`
defaults to `same`; an `expect` under `ieee_eq` or `bits_eq` is written on the
binding (`compare same|bits_eq|ieee_eq`, FL1-S4). Comparison of a model result to
the subject's result never uses a host `==`: the runner compares the two bit
patterns (after `same`'s NaN mapping where selected) as integers.

### 4.4 Typing rules

```text
G |- a, b : F64            G |- a, b : F32
------------------------   ------------------------   (no mixing; each op is
G |- fadd(a, b) : F64      G |- fadd(a, b) : F32       monomorphic in its width)
```

* **No implicit conversion** between `F32`, `F64`, `Int`, `Nat` and the fixed-width
  integers. Mixing is W0301. Conversions are the explicit built-ins of 5.3.
* Arithmetic is by **named function**, no new operator tokens (as VER04j WX-C),
  so `a + b` on floats is W0301; this also keeps the old token set intact.
* `fma(a, b, c)` is its own function. `fadd(fmul(a, b), c)` **never** means a
  fused operation (two roundings), and a lane may not contract it (9, 12).
* `if`/`match` branches, `let`, models and folds treat floats as ordinary values;
  they have no special typing.
* A float inside `Option`, `Result`, `List`, tuples and records is allowed; the
  relations of 4.3 lift (`same` through each component). A list of floats compared
  with `==` is W0301.

## 5. Operations in scope

### 5.1 Basic arithmetic (correctly rounded, RNE)

| Function | Type | Notes |
|---|---|---|
| `fadd`, `fsub`, `fmul`, `fdiv` | `(Fw, Fw) -> Fw` | IEEE 5.4.1; `fdiv(x, 0)` is a signed infinity or NaN as IEEE says |
| `fsqrt` | `Fw -> Fw` | correctly rounded; `fsqrt(-x)` is the canonical NaN for `x > 0`; `fsqrt(-0) = -0` |
| `fma(a, b, c)` | `(Fw, Fw, Fw) -> Fw` | a*b+c with a **single** rounding |
| `frem` | not in scope | the IEEE remainder is an open question (FL-6) |

Why `fsqrt` and `fma` are in and `fmod`/`frem`/`floor`/`round` are not (yet): sqrt
and fma are the two operations beyond the basic four that IEEE requires to be
correctly rounded; they are the building blocks of every algorithm worth
verifying. `fsqrt` is native everywhere. **`fma` is not**: JavaScript, Lua, Ruby,
Perl and Python before 3.13 have no host `fma`, and some C libraries' `fma` on
targets without the instruction have been wrong. The **model** `fma` exists in
every lane regardless (it is integers). A **subject** `fma` binding exists only
where the host has one (12.2), and is checked like any other subject. The rounding functions are bit manipulations that an author can state
over `bits`; they are a convenience slice (FL1-S7).

### 5.2 Bit-level and sign operations (exact, payload-preserving)

`fneg`, `fabs`, `copysign(x, y)` (magnitude of x, sign of y): single-bit edits,
NaN payload preserved. `bits(x)`, `f32_of_bits(u)`, `f64_of_bits(u)`: bijections.
`fclass(x) -> Nat`: 0 NaN-signalling, 1 NaN-quiet, 2 -inf, 3 negative normal,
4 negative subnormal, 5 -0, 6 +0, 7 positive subnormal, 8 positive normal, 9 +inf
(a fixed table in the grammar artifact; the classification is data, not a host
`isnan`).

### 5.3 Conversions

| Function | Type | Semantics |
|---|---|---|
| `f32_of_f64` | `F64 -> F32` | rounds (RNE) with the overflow threshold of 3.2; a result below 2^-126 is **rounded** to the binary32 subnormal grid (gradual underflow, possibly to a signed zero), never flushed and not in general exact; NaN to canonical F32 NaN |
| `f64_of_f32` | `F32 -> F64` | exact; NaN to canonical F64 NaN |
| `f32_of_int`, `f64_of_int` | `Int -> F32`, `Int -> F64` | the integer, rounded RNE; magnitude at or above the overflow threshold of 3.2 gives the signed infinity; `0` gives `+0`. Two names, not one resolved "by context": every other operation takes its width from an argument, and `bits(f_of_int(3))` would have had none |
| `f_to_int` | `Fw -> Option<Int>` | **truncates toward zero** (a documented choice, FL-7); `None` for NaN and the infinities; any finite value gives `Some` of the exact mathematical integer, so no saturation and no host-defined overflow |

Conversion from the fixed-width integers goes through `Int` (they widen to `Int`),
so `f32_of_int`/`f64_of_int` have one rule each, not eight. The exact `Int` result is what makes
`f_to_int` portable: C's `(int)f` is undefined behaviour out of range, Java
saturates, JavaScript wraps modulo 2^32 in `|0`; none of them is ever the model.

### 5.4 Tolerance claims

`within_ulps(x, y, k)` and `rel_err_le(x, y, num, den)` are the only two
approximation predicates. Their **semantics are fixed, written out and printed**:

* **Widths.** Both float arguments have the **same** width (W0301 otherwise);
  the distance is counted in that width.
* **ulp distance.** Map each non-NaN float to a monotone integer: for bits `u`,
  `m(u) = u` if the sign bit is 0 else `-(u & ~sign)`, so `m(-0) = m(+0) = 0` and
  adjacent representable values differ by exactly 1 (the zeros coincide).
  `ulp_dist(x, y) = |m(x) - m(y)|` over `Int` (exact; no overflow, because `Int`
  is mathematical). **`ulp_dist` is infinite, and `within_ulps` false, if either
  argument is NaN**, and also if exactly one argument is an infinity (a finite
  value against an infinity, or `+inf` against `-inf`). `within_ulps` holds at an
  infinity only when both arguments are the **same** infinity, whatever `k`
  (decision FL-14, 2026-11-16: the strict rule, the same as `rel_err_le`; a
  subject that overflows where the reference is finite, or the reverse, is a
  defect and no tolerance hides it). `m` is therefore defined on the finite
  values only, and `max_finite` and `+inf` are **not** one step apart.
  `within_ulps(x, y, k)` holds iff `ulp_dist(x, y) <= k`,
  `k: Nat`, `k < 2^64`, `k` a literal or a bound parameter (never computed from the
  subject). Three consequences an author must know, printed with the claim:
  the unit is **representable steps**, not multiples of `ulp(y)` (the two differ
  across a power of two, where the spacing changes); the predicate is **blind to
  the sign of zero** (`within_ulps(-0, +0, 0)` holds), so a sign-of-zero contract
  needs its own `same` or `bits_eq` claim; and near zero it is an **absolute**
  bound (`k` steps of 2^-1074), so a wrong sign on a tiny result passes.
* **Relative error.** `rel_err_le(x, y, num, den)`, in this order of cases: false
  if either is NaN; if either is infinite, true iff both are the **same**
  infinity; if `y` is a zero, true iff `x` is a zero (of either sign); otherwise
  true iff `|x - y| * den <= |y| * num`, evaluated in **exact rational arithmetic**
  (`Int` products of the significands and exponents; no float arithmetic is used
  to check a float claim). `num`, `den: Nat`, `0 < den < 2^64`, `num < 2^64`, so
  the products are bounded (under 2,300 bits for binary64). Subnormal `y` is
  allowed and the formula is the same: the bound is relative, so on subnormals it
  is far stricter than an ulp bound near zero, and the author must pick the one
  they mean.
* **A NaN reference fails.** Both predicates are false when the reference is a
  NaN. A tolerance claim over a domain where the reference can be a NaN (`sin` of
  an infinity) therefore fails closed until the author splits it: `same` on the
  NaN part, the tolerance on the rest.
* **The reference `y`** is never the host libm. It is one of (FL-3):
  (i) a **bit-exact algorithmic model**: a WX-D2 fold that replays the documented
  operation order with the model's own `fadd`/`fmul`; (ii) a **WX-E table** of
  correctly rounded values with its generator pinned (the table is data; its
  hash is in the manifest); (iii) another subject operation of the same lane
  (differential claims only; labelled `differential`, never `reference`).
* **The label carries the tolerance**: `sampled  f64_edge(214)  approx(<=4 ulp)`
  or `... approx(rel<=1/2^50)`. A tolerance claim **never publishes as a
  bit-exact contract** under VER29 and never lifts a `sampled` label.
* **The vacuity guard.** Every claim `within_ulps(.., k)` must fail the fault
  `drift(k+1)` (13), which replaces the subject's result by the value `k+1` steps
  from the **reference** (not from the subject's own result: a subject already
  off by 2 in one direction, drifted 5 in the other, would land inside a 4-ulp
  bound and the guard would prove nothing). A tolerance claim that cannot detect a
  deliberate error one step over its own bound is vacuous: **W0601**, and
  `drift`/`rel-drift` may never be listed in `accepted_blind_faults` (VER04i 4).

## 6. Why there are no `sin`, `exp`, `pow`, `log` built-ins

libm-style functions are **not** built-ins and have **no model**. They enter a
specification only as **subject operations** (`op sin(x: F64) -> F64`) whose
contract is a tolerance claim against a reference of kind (i) or (ii) of 5.4,
for four reasons:

1. **IEEE 754 does not require them correctly rounded** (only `sqrt`, the five
   arithmetic operations, `fma` and conversions). Two conforming libms return
   different bits for the same input; glibc, musl, the BSD libm, macOS, MSVC, the
   JVM's `StrictMath` (fdlibm) and Go's pure-Go math all differ in the last place on
   some inputs.
2. **No correctly rounded model is cheap and universally agreed.** Writing one
   means a CRlibm-style table-driven implementation with proofs; it would be a
   project, not a built-in, and its hard cases (the table-maker's dilemma) cannot
   be enumerated for binary64.
3. **A built-in would silently choose a winner.** Whichever library the model used
   would become "the right answer" for every lane, and the host libm would become
   the oracle through the back door, the exact thing decision (d) forbids.
4. **The honest, checkable statement is a bound.** "Within 4 ulp of the correctly
   rounded value on this domain" is a fact a softfloat-evaluated reference can
   decide; "equals libm" is a fact about an installation.

Consequently a subject function's float result under a bit-exact claim is allowed
only against an algorithmic model that is part of the specification (the `trig`
pilot's 20-term series, section 15), and otherwise the claim must be a tolerance
claim. Constants such as pi are written as bits.

## 7. Evaluation semantics

The ground evaluator (the Rust checker, VER04d) evaluates every float expression
by the integer softfloat of section 8, so the model result is the same on every
build of the checker and on every host. A float literal, a fold over floats, a
`match` on `fclass`, an `if` on `ieee_lt` all reduce to operations on `U32`/`U64`
bit patterns and `Int` arithmetic. Budgets: `fmul`, `fdiv`, `fsqrt` and above all
`fma` (whose exact binary64 intermediate needs about 160 bits) need multi-word
intermediates; these are implemented with `U64` limbs and **do not wait for any
128-bit integer type** (WX-I128 is not a dependency). Float
expression size is charged to the existing evaluator budget (VER04 section 8); a
fold of 2^20 float operations costs what 2^20 integer folds cost.

## 8. The softfloat model: obligations and its own verification

### 8.1 Obligations on every lane's model

The `warrant-softfloat` model (one per lane, generated from one language-neutral
description like the SHA-256 of VER04i 2.6) MUST:

* use **only integer operations** on exact-width unsigned/signed integers with
  documented wrap/mask rules (VER04i 2.7's table of lane integer representations
  applies; Lua `math.ult`; OCaml `Int64`; JS `BigInt`; Perl `Math::BigInt` or a
  declared 64-bit build; Python/Ruby masks). It MUST NOT call any host float
  operation, float formatting or parsing, `Math.fround`, `float()`, `struct` with
  an `f`/`d` code, or libm, **except** inside the environment probe (9), which is
  allowed to, being the subject;
* implement `fadd fsub fmul fdiv fsqrt fma f32_of_f64 f64_of_f32 f32_of_int f64_of_int f_to_int`
  and the sign/class/relation operations exactly as sections 3 and 5 say, for both
  widths;
* be **branch-complete on the special cases** in a documented order (NaN, infinity,
  zero, subnormal) so a reader can check it against the standard;
* be written in the literate style of the repository: the case analysis, the guard
  / round / sticky bits, and a worked rounding example inline.

### 8.2 How the model is verified (it is itself a subject)

The model is the oracle, so it is verified harder than anything it checks.

1. **Berkeley TestFloat-style vectors.** The repository vendors, under
   `code/specs/data/softfloat/`, vector files for the operations above in a
   plain-text format (operation, rounding mode `near_even` only, operand bits,
   expected result bits, flags ignored) with a recorded provenance (generator
   program and version, seed, SHA-256). Every lane's model is run on the **whole
   vector set**, which exercises the TestFloat level-2 coverage (a fixed set of
   hard rounding, overflow, underflow, subnormal and tie cases per operation).
   The vector file is data; checking that a lane passes it is an L0 suite of kind
   `bounded-set` (the vector set is a named finite set), never printed as
   "exhaustive". The vectors must come from a generator that shares **no code**
   with the model description (Berkeley TestFloat does not).
2. **Exhaustive binary32 check of the reference model, nightly.** Because `F32`
   is only 2^32 patterns, the unary operations of the **Rust checker's model** are
   checked **over every input** (`fsqrt`, `f64_of_f32`, `f_to_int`, `fneg`, `fabs`,
   `fclass`). `f32_of_f64` cannot be exhausted over its 2^64 inputs and checking
   it only on f32-exact values would test nothing (they never round), so it is
   checked on the **rounding boundaries**: for every pair of adjacent binary32
   values, the binary64 midpoint and its two binary64 neighbours, plus the
   overflow threshold and its neighbours (about 3 x 2^32 cases, enumerable). The
   binary operations and `fma` are checked over a structured, enumerable
   cross-product of a stratified set (every exponent pair x a mantissa class,
   about 2^24 pairs). The comparison is against a **second, independent
   implementation**: the native C compiler's `float` in a **probe-passing,
   reference** environment (x86-64 SSE2, `-ffp-contract=off`), used here **only to
   cross-check the softfloat, never as the oracle of a Warrant claim**. Three
   rules keep the cross-check honest: results are compared under `same` (the
   hardware's default NaN has the sign bit set, 3.3); `f_to_int` is compared
   through `truncf` and an exact wide-integer conversion, never a C cast (which is
   undefined out of range); and `fma` uses the hardware instruction, not the
   host libm's `fmaf`. A disagreement is a defect of one of the two and blocks.
   This is a nightly job, not per-PR, and it is the evidence behind the label
   `bounded-set` (not `full`) for the binary operations. It runs on **one** model
   (minutes per operation in compiled code); the other lanes inherit it through
   item 3, which is why item 3 must hold exactly. Cost: FL-10. The model's
   exhaustive self-check runs on the **Rust** model only (decision FL-10,
   2026-11-16); no other lane runs it.
2a. **Binary64 has no exhaustive check, and that is printed.** The binary64
   operations get the vectors (item 1), the rational reference (item 2b) and a
   nightly hardware cross-check on a seeded pseudo-random and stratified set
   (seed and count in the manifest, kind `sampled`). The report line for the
   model says so: `softfloat: f32 unary full, f32 binary bounded-set, f64 sampled`.
2b. **An exact rational reference.** The corpus generator carries a second
   implementation of every operation that is structurally unlike the softfloat:
   decode the operands to exact rationals with unbounded integers, compute the
   exact result (for `fsqrt`, an integer square root with a remainder test),
   and round **once** by the definition of 3.2. It has no guard, round or sticky
   bits and so cannot share the model's rounding bugs. It is slow, and is run on
   the vectors, the edge lists and the stratified sets of both widths. This is
   the only check of `fma` and of binary64 that does not depend on a host FPU.
3. **Cross-lane agreement (a transcription check, not an independent one).** Each
   lane's softfloat is also run against the Rust checker's softfloat on the
   **same** cases generated from the corpus; the manifests carry
   `softfloat_corpus_sha256`, which every lane must reproduce (a lane cannot
   choose its own test points). All lane models descend from one description, so
   agreement between them catches a wrong transcription and **cannot** catch a
   wrong description: that is what items 1, 2, 2a and 2b are for, and the report
   must not present eighteen agreeing lanes as eighteen witnesses.
4. **Sabotage of the model.** The model has its own controls (13): `round-half-up`
   in the model, `ftz-in-model`, `payload-propagating-nan`, `sticky-bit-dropped`,
   `fma-as-mul-then-add`, `wrong-zero-sign-on-cancel`. Each must be caught by
   the TestFloat vectors, or the vector set is declared too weak.
5. **Proofs.** None. (A formal model such as a Flocq or Sail extract is out of
   scope, section 16; status `not-live`, as for every proof in the first slice.)

### 8.3 Divergence rule

Any disagreement between a lane's model, the checker's model, the vectors or the
reference cross-check is a **model defect** and blocks every float claim until
resolved. There is no "known divergence" list: a divergence means the evidence is
not trustworthy, and the lane's float rungs are `not-live` for that build.

## 9. The environment probe and the `not-live` rule

### 9.1 What is probed

Before any float case runs, each lane runs the **probe**: a fixed set of
computations in the **host** float arithmetic, whose correct results (as bits) are
in the corpus and are verified against the softfloat model at corpus-generation
time. The probe values are part of the corpus (`probe.json`, TFV rows). Each
probe isolates one hazard so the failure names its cause:

| Probe id | Computation (host) | A failure reveals |
|---|---|---|
| `P1-double-rounding` | `2^53 + (1 + 2^-52)`, bits `0x4340000000000000 + 0x3ff0000000000001`. The exact sum is just above a binary64 midpoint, so the correct result is `0x4340000000000001` (2^53 + 2). A 64-bit significand first rounds it **onto** the midpoint, and the second rounding then ties to even: `0x4340000000000000`. A binary32 analogue is included | x87 extended precision (`FLT_EVAL_METHOD != 0`), double rounding |
| `P2-fma-contract` | `a*b + c` chosen so a fused result differs from separately rounded in the last bit | FMA contraction (`-ffp-contract=fast`, Go on arm64/ppc64/s390x, Julia-style auto-fusion) |
| `P3-flush-subnormal-out` | an **exact** tiny result, `2^-1022 * 0.5`, and an **inexact** one, `0x0010000000000001 * 0.5` (a tie, correct result `0x0008000000000000`); both again in binary32. Both are needed: some flush modes act only on inexact tiny results | flush-to-zero (FTZ, ARM `FPCR.FZ`), `-ffast-math`, GPU/NEON defaults |
| `P4-flush-subnormal-in` | a subnormal **operand** with a **normal** result, `2^-1074 * 2^100` (correct: 2^-974; with DAZ: 0), and `2^-1074 + 2^-1074`; both again in binary32. The first isolates the input side, which a subnormal result would confuse with P3 | denormals-are-zero (DAZ) |
| `P5-rounding-mode` | a tie result for add, mul, div and `f32_of_f64` | a non-default rounding mode left by a library |
| `P6-reassociation` | `(a + b) + c` against `a + (b + c)` on values where they differ | reassociation (`-ffast-math`, `-fassociative-math`, LLVM `fast`) |
| `P7-sqrt-div-exact` | `sqrt` of a perfect square and of a hard-rounding value; a quotient `a / b` for which `a * (1/b)` differs in the last bit | an approximate `sqrt` (reciprocal-sqrt estimate with refinement); division by reciprocal (`-freciprocal-math`) |
| `P8-nan-bits` | the bits of `inf - inf` and `0 * inf` (and `0.0/0.0` where the host does not raise on it: Python, Perl and BEAM do); a quiet and a signalling payload NaN through a bit-cast round-trip, through a call and return, and through `fneg`/`fabs`, in **each** width | engines that rewrite payloads, quieting on moves, ARM default-NaN mode; selects the `payloads=` declaration (12) **per width** |
| `P9-narrow` | `f32_of_f64` of tie/overflow/subnormal values; a binary32 chain | `F32` computed in a wider type (`skip-fround`); JS without `Math.fround` |
| `P10-convert-int` | the host's integer-to-float conversion at 2^24 + 1, 2^53 + 1, 2^63 - 1, 2^64 - 1 (ties and round-up cases) and float-to-integer truncation **inside** the host integer's range. Out-of-range float-to-integer is **not** probed: it is undefined or host-defined almost everywhere, and a subject that relies on it is wrong by claim, not by environment | a lane that converts a 64-bit integer through a 53-bit double, or rounds an integer conversion in the wrong mode |
| `P11-compile-time` | the same arithmetic on **runtime-opaque** inputs and on **constant-folded** inputs; both must match | a compiler or JIT that folds with a different precision than the runtime (Go evaluates constant expressions exactly) |
| `P12-extended-exponent` | `(max_finite * 2) * 0.5` kept in one expression: correct `+inf`, a wider exponent range gives `max_finite`; and a product whose exact value is just above a **subnormal** midpoint (double rounding that P1 cannot see) | x87 with the precision control set to 53 bits (the 32-bit Windows and FreeBSD default): P1 passes there, yet overflow and subnormal results are still wrong |
| `P13-no-trap` | `inf - inf`, `0 * inf`, an overflow, an underflow and an addition with a **signalling** NaN operand must each **return**. Run in a child process; death by signal or an FP exception is the failure | unmasked floating-point exceptions (`feenableexcept`, MXCSR mask bits cleared by a library, a runtime that turns invalid into a fault): a hostile corpus value could otherwise kill the runner |

The probe operands are loaded from the corpus at runtime and passed through an
opaque barrier (a volatile read, an FFI call, a `black_box`) so that the compiler
cannot fold them: P11 is the control for that.

Four rules about **where and when** the probe runs. Without them a probe can pass
in an environment the subject does not run in:

1. **Same build.** The probe is compiled with the same compiler, flags, target and
   optimisation level as the subject, and linked into the same test executable
   (or loaded into the same interpreter process). `float_env_sha256` covers all
   of these.
2. **Same process and thread, before and after.** FTZ, DAZ, the rounding mode
   and the exception masks are per-thread state that a library can change at load
   time or mid-run. The probe runs on the thread that calls the subject,
   **before the first float case and again after the last**; either failing makes
   the whole float run `not-live`. A lane with worker threads probes each one.
3. **Natural form.** P2 and P6 write the expression the way a subject would
   (`a*b + c`, no defeating cast or temporary). Contraction and reassociation are
   decided per expression, so these two probes show what the toolchain **may** do
   to subject code; they cannot show what it did to a given subject.
4. **Necessary, not sufficient.** A passing probe authorises the lane to produce
   float evidence; it is never evidence about the subject. The subject is judged
   only by its claims against the model. A subject that fuses where the probe did
   not is caught there, as `fail`.

### 9.2 The `not-live` rule

**If any probe in P1 to P7 or P9 to P13 fails, or the lane cannot run the probe
at all, the lane reports every float-using suite's rung as `not-live`**, with the
reason printed (`not-live: float-env P2-fma-contract`), never as `fail` and never
omitted. P8 is different: it sets the lane's `payloads` declaration to `lost` for
the affected width, so only payload-preservation claims in that lane and width
become `not-live` and everything else proceeds. A lane that **declares**
`payloads=preserved` and fails P8 is `not-live` for floats like any other probe
failure: a declaration the probe contradicts is not quietly downgraded. The rules:

* The lane's non-float suites are **unaffected**.
* The report **prints** the float-environment status per lane in its own row,
  with the probe results (the probe ids and pass/fail).
* **No widening.** A failed probe never relaxes a tolerance, never switches to a
  host oracle, never excuses a claim. There is no per-lane tolerance override.
* **Fail closed.** A lane that has no probe implementation, or whose probe result
  is absent or stale (hash of the lane's compiler/flags/runtime does not match the
  manifest's `float_env_sha256`) is `not-live` for floats.
* Recovery is to fix the environment (flags, target, runtime version) and rerun.
  The rung is **never** `fail` merely because the probe failed: `fail` means the
  subject was wrong, `not-live` means the evidence could not be produced.

A lane's overall coverage claim for a float-bearing package is therefore honest by
construction: a lane that cannot prove its environment cannot claim anything about
its floats, and the shipping gate (VER00) treats `not-live` as non-authorising,
exactly as it does for proofs.

## 10. Corpus and manifest encoding: bit patterns only

**A float is never a decimal string, a JSON number or a host float anywhere in a
corpus or manifest.** This is decision (c) and a mechanical check.

| Place | Encoding |
|---|---|
| canonical stream (VER04i 2.6) | `62` then 4 bytes little-endian for `F32`; `63` then 8 bytes little-endian for `F64`. (`60`/`61` are VER04l's. `62`/`63` are this document's, as recorded in the registry of VER04i section 2.6.1; VER04r, which first claimed `62` for variants, moved to `64`/`65` on 2026-11-16. FL1-S0 reconfirms against the registry.) |
| `cases.tfv` (TF01 TFV) | a TFV string value `S` holding the lowercase hex bit pattern, 8 digits for `F32`, 16 for `F64`, no `0x` prefix, under a key that carries the width. **Not** a TFV integer, because a U64 parsed as a signed 64-bit integer by a lane would be a defect for patterns above 2^63 |
| `corpus.json` | a JSON **string** of the same hex; a JSON number, or a string that is not exactly 8/16 **lowercase** hex digits (uppercase, a `0x` prefix, a sign, whitespace), is a **corpus error**, so one value has one spelling and hashes are stable |
| manifest result fields | same hex strings; tolerances `k`, `num`, `den` are integers |
| `Option<F64>` etc. | the existing composite rules of VER04i 2.7 apply; the element is the hex string |

Enforcement:

* The **generator** (Python `warrant-corpus`, the cross-lane oracle of VER04i 2.3)
  **refuses to call `float()` on any corpus value**, forbids `json.dumps` of a
  float into a corpus (a lint in the generator and a unit test), and computes
  expected values only with its own port of the model (8.1), not with `struct` or
  Python floats. A corpus tree is checked by a validator that rejects any decimal
  point, exponent letter or JSON number in a float position.
* Each **lane runner** parses the hex into integers, builds the float with a
  bit-cast (`f64_of_bits`, `Float.intBitsToFloat`, `math.Float64frombits`,
  `DataView.setBigUint64`), calls the subject, and reads the result back **as
  bits** before comparing; the printed form of a result in a failure message is
  the hex, with at most a decimal rendering as an informal aid that is never
  parsed.
* **A decimal literal** (`0.1`) is not in the surface language in FL1 (2). If FL2
  adds float-to-text, its meaning is "the unique double nearest, ties even" and it
  is a distinct, version-pinned slice that needs text T1 (16).

Worked example of a TFV case row (illustrative):

```text
c12 R3 Kop Ssin Kx S3ff921fb54442d18 Kexpect S3ff0000000000000 Kcompare Ssame
```

(the bits of pi/2 in, the model's bits of the reference out; the row shows the
shape only: its `expect` bits were not computed and the key layout is whatever
TF01 and VER04i fix at FL1-S2).

## 11. Domains, edge sets and honest labels

### 11.1 The domains

| Descriptor | Order and content | Kind |
|---|---|---|
| `f32_edge` | the edge list below, in order | `sampled` |
| `f64_edge` | the same shapes in binary64 | `sampled` |
| `f32_all` | all 2^32 patterns ascending as unsigned integers | `full`, only under 11.3 |
| `f32_set(name)` | a named, enumerable, **finite** set, such as the TestFloat level-1 set or the stratified binary32 set of 8.2 | `bounded-set` |
| `f64_set(name)` | a named finite set of the same kind | `bounded-set` |
| WX-F `lengths`/`hints` on floats | not applicable | n/a |

The **edge list** for width `w` (fixed order; each entry is a bit pattern;
duplicates removed keeping the first):

1. `+0`, `-0`;
2. the smallest subnormal and its negation, the largest subnormal, the smallest
   normal (+ and -), the neighbours either side of each (next up and next down);
3. `1.0` and its two neighbours; `-1.0`; `2.0`, `0.5`; `1.5`;
4. the largest finite and its negation, and its predecessor;
5. `+inf`, `-inf`;
6. NaNs: the canonical, a **negative** NaN, a **signalling** NaN, a payload-1 quiet
   NaN, the maximum payload, a NaN with a mid-bit set;
7. the integer boundaries: 2^24 - 1, 2^24, 2^24 + 1 (binary32); 2^53 - 1, 2^53,
   2^53 + 1 (binary64); 2^31, 2^63, 2^64 and their neighbours (both widths, where
   representable);
8. halfway cases: for each of 1 and 2^k (for several k), the exact midpoint
   between adjacent values and the values one ulp either side;
9. the **sign of every one of the above** where not already listed;
10. the values `pi/2` and `pi` rounded to the width, and their neighbours (trig
    needs them; they are ordinary edge points).

The list is generated by the corpus generator from an exact table, hashed, and
recorded as `domain_sha256` in the manifest. `f64_edge` is about 214 entries,
`f32_edge` about 190 (the memo's count is a guide, the generator's is the truth).

### 11.2 The label rules (fail closed)

| Claim shape | Kind | Printed as |
|---|---|---|
| any binder `F64`, any domain | never `full` | `sampled` (edge) or `bounded-set` (named set) |
| `F64` binder, `f64_all` | **W0706**: not constructible | refused: 2^64 exceeds every budget |
| one `F32` binder, `f32_edge` | `sampled` | `x: F32 edge(190)` |
| one `F32` binder, a named `f32_set` | `bounded-set` | `x: F32 set(<name>, n)` |
| one `F32` binder, `f32_all`, declared 2^32 (11.3) | `full` | `x: F32 full(4294967296)` |
| two or more float binders | never `full` | the weaker of the binders' kinds, and for `f32_all` x anything W0706 (2^64) |
| a tolerance claim of any kind | its domain's kind | the label carries `approx(...)` |
| a claim over a float result only (no float binder) | by the binders present | a float result does not raise the kind |

A claim's kind is the **weakest binder kind** (VER04l 6.1); a refinement or
demotion never improves it. **Floats add no new kind**; they use LP-012's
`full`/`bounded-set`/`sampled` and never print an unqualified "exhaustive".
(The owner's shorthand "sample" is the existing `sampled`.) The honest sentence an
`F64` claim may print is "N edge points, M sets", never "all doubles".

### 11.3 The declared 2^32 domain for `F32`

`f32_all` exceeds the budget `B` (VER04h 3: 400,000) by four orders of magnitude,
and the 64 MiB domain-bytes budget. It is therefore admitted **only** when the
claim carries an explicit declaration

```text
claim ... : forall x: F32 in f32_all. ...        -- unary, F32 only
  budget nightly(4294967296)                     -- declared case count
```

and **only** under these conditions, all mechanical:

1. exactly **one** float binder and no other binder; any second binder is W0706;
2. the generator **streams** the domain (no materialised list; the 2^32 bound is on
   count, not on memory) and the manifest records
   `{kind: full, count: 4294967296, tier: nightly}`;
3. the suite belongs to the **nightly** tier. A per-PR run reports it `not-run`,
   never `pass`; the report row for `full` is **only** green for a tier-matching
   nightly manifest whose source hash equals the head;
4. the run counts **every** case and verifies the count equals 2^32 (a lane that
   skips patterns is a failure, not a smaller `full`). If the run is **sharded**,
   each shard is a contiguous range `[lo, hi)` of the unsigned patterns recorded
   in its manifest, and `full` is printed only when the shards of **one** source
   hash and **one** `float_env_sha256` tile `[0, 2^32)` exactly, with no gap and
   no overlap; shards from different nights or builds never combine;
5. the lane's float probe passed in that same run (9);
6. the lane is a **compiled lane** (decision FL-10, 2026-11-16; the compiled set
   of VER04r section 5: rust, go, c, cpp, java, kotlin, csharp, fsharp, swift,
   haskell, ocaml, dart-native). An interpreted lane (Python, Ruby,
   Perl, Lua, Elixir) prints `not-run` for an `f32_all` claim; it is never queued
   and never demoted silently. The model's own exhaustive self-check (8.2 item 2)
   runs on the Rust model only.

Anything not meeting all six is demoted to `sampled` or refused with W0706; there
is no path by which an unmet condition still prints `full`. `F64` has no such
path at all.

## 12. Lane binding contract per language family

### 12.1 The rule that applies to every lane

The binding file (VER04i 5) gains three optional data keys on any float-using
op: `float_env=` (the lane's probe-result reference), `payloads=preserved|lost`
(default `lost`), `nonfinite=ok|rejects:<CASE>|unreachable` (12.2, the BEAM row
and every other host that raises instead of returning a non-finite value). **The
binding is data only**; it names the subject callable and the width, never an
inline float expression. Every lane implements the **same thirteen probes** and the
**same softfloat model**, so equal rigour is a property of the corpus, not of the
lane's goodwill. A lane that does not (yet) implement them has no float claims:
`not-live`.

### 12.2 The table

| Lane family | Hazard and binding rule |
|---|---|
| **C, C++** | `-ffp-contract=off`, no `-ffast-math`/`-Ofast`/`-funsafe-math-optimizations`/`-fassociative-math`/`-ffinite-math-only`; `-fno-fast-math` explicit; `FLT_EVAL_METHOD == 0` asserted at compile time and probed at run time; `-msse2 -mfpmath=sse` on x86 (x87 targets, 32-bit x86 without SSE2, are **refused**); `-frounding-math` NOT used because the default is RNE; `-fno-builtin` is not required but the probe's P11 detects constant folding. Compilers pinned (`gcc -std=c11`, `clang`); MSVC `/fp:precise` or `/fp:strict`, never `/fp:fast`; Intel `icx`/`icc` default to a fast model and need `-fp-model=precise`. FTZ/DAZ bits in MXCSR are probed (P3/P4) because `-ffast-math` and some libraries set them at load time. C's out-of-range float-to-integer cast is undefined: a subject that uses one has no portable contract (15) |
| **Rust** | no fusion by default (Rust never contracts); i586/x87 targets refused (probed); `f32`/`f64` bit-casts via `to_bits`/`from_bits`; NaN bits are not guaranteed (hence `same`); `payloads` declared per target; `std` libm calls are subjects only |
| **Go** | the spec **permits** fusing `x*y + z`, for `float32` and `float64`, and the compiler does on arm64, ppc64, s390x, riscv64 and others; there is **no flag** to turn it off, so on those targets P2 fails and Go floats are `not-live` (FL-16). **Go float evidence is amd64-only** (decision FL-16, 2026-11-16): the Go lane's manifest records `GOARCH`, evidence from any other architecture is not float evidence, and the report says so; a subject written with explicit `float64(x*y)` conversions does not opt in. Constant expressions are evaluated exactly at compile time, unlike the same expression at run time (P11). An out-of-range `int(x)` is implementation-defined and differs between amd64 and arm64; the explicit `float64(x*y) + z` conversion prevents fusion in a hand-written model but the probe P2 is the arbiter, and a fused build **fails honestly** (`not-live`, not a workaround). `math.Float64bits`/`Float64frombits` for I/O. `math.FMA` is the subject for `fma` |
| **Java, Kotlin (JVM)** | strict IEEE semantics since Java 17 (JEP 306); **Java 16 and older and Kotlin targeting them are refused** unless the class is `strictfp` and the JVM option set is probed; `Float.floatToRawIntBits` (never `floatToIntBits`, which canonicalises NaN) and `Double.doubleToRawLongBits`; `Math.fma` for `fma`; `StrictMath` vs `Math` is a *subject* choice, never an oracle; Kotlin's `Float`/`Double` use the same raw-bits functions |
| **C#, F#** | .NET Core 3.0+ and .NET 5+ only (RyuJIT, SSE2, IEEE-conforming); the legacy x87 JIT (.NET Framework 32-bit) refused; `BitConverter.DoubleToInt64Bits`/`SingleToInt32Bits`; `Math.FusedMultiplyAdd`; JIT tiering is probed by P11 (opaque inputs) |
| **JS/TS (Node, Bun, Deno), Dart web** | **only binary64 exists**; `F64` is `number`; `F32` is computed as `Math.fround(op(...))` after **each** `+ - * / sqrt` of `Math.fround`ed operands, which is *exact* by the double-rounding-innocuity theorem for these operations (53 >= 2*24 + 2), and is **not** valid for `fma` or for any chain narrowed only at its end. **JavaScript has no `fma` at all**, in either width: a subject `fma` binding does not exist in these lanes (a documented `not-live` row), and a subject that emulates one is an ordinary subject checked against the model. An `F32` lives in a `number`, so signalling binary32 NaNs are quieted in transit: `payloads=lost` for `F32`. Floats cross as `BigInt` bits through `DataView` (`setBigUint64`/`getFloat64`), never as `number` parsed from text; engines may rewrite NaN payloads, so `payloads=lost` unless P8 proves otherwise; Dart web is the same; Dart VM is native. `Number` is never used as a corpus value type |
| **Python** | f64 only (CPython `float`); `F32` through `struct.pack('<f', x)`/`unpack` round-trip (which rounds RNE), and arithmetic on `F32` as f64 followed by that round-trip (exact for + - * / sqrt, the same double-rounding argument). Two traps: `struct.pack('<f', x)` **raises `OverflowError`** when a finite double is too large for binary32 where IEEE says infinity, so the adapter must handle overflow itself; and Python **raises** on `x / 0.0`, `math.sqrt(-1.0)` and `int()` of a NaN or infinity instead of returning a value, so such ops are bound `nonfinite=rejects:<CASE>`. `math.fma` exists only from 3.13; before that there is no subject `fma`. NaN payloads survive `struct` for `F64` on CPython but are not guaranteed, and binary32 signalling NaNs are quieted (`payloads=lost` for `F32`); the oracle in the generator never uses `float()` on corpus text (10). `numpy` float32, if used in a lane, is a subject library, never a model |
| **Ruby, Lua, Perl** | f64 only (Lua 5.3+ `string.pack('<d')`, `<f`; Ruby `[x].pack('E')`/`'e'`; Perl `pack 'd<'`/`'f<'`). **Perl refuses a build whose `NV` is a long double** (`$Config{nvsize} == 8` asserted) and dies on division by zero (`nonfinite=rejects`); Ruby's `Math.sqrt` raises on a negative argument; none of the three has a host `fma`; Lua is built with `LUA_FLOAT_TYPE = double`; Ruby `Float` is double. The lane's integer model uses the width rules of VER04i 2.7 |
| **Elixir/Erlang (BEAM)** | BEAM floats **cannot be NaN or infinite**: an operation that would produce one raises `badarith`. Non-finite values cross as **bits** (integers, never a BEAM float); an op that would return a non-finite value is bound `nonfinite: rejects:<CASE>` (the subject raises) or the case is `not-live`; the model itself runs on integer bits, unaffected. There is no binary32 type: `F32` is a double narrowed through `<<x::float-32>>`, with the same overflow caveat as Python. Whether `0.0` and `-0.0` match each other changed in OTP 27, so adapters compare bits, never terms |
| **Haskell, OCaml, Swift** | native IEEE; adapters compare **bits** (`castDoubleToWord64`, `Int64.bits_of_float`, `Double.bitPattern`), never the language's `Eq`; GHC without `-fexcess-precision` and, on the LLVM backend, without fast-math options passed through `-optlo`; 32-bit x86 OCaml and GHC use x87 and are refused by P1/P12; OCaml float arrays are unboxed doubles, fine; Swift `Float`/`Double` with `-Ounchecked` banned |
| **Other lanes** | any other lane (a future addition under VER04i 13.2) must add its own row **before** it may claim a float rung; the default for an absent row is `not-live` |

### 12.3 Reconciliation with the lane integer rules

A lane that holds floats in its own `float` type is the **subject**; a lane that
holds the model's integers is the **oracle**; the two meet only at the
`bits`/`of_bits` boundary. No lane may hold a `U64` model value in a float, and no
lane may hold a float result in an integer except through the exact bit-cast.

## 13. Sabotage controls (faults)

Per VER04h 6 and VER04i 4, a claim passes only if its control fails. Faults are
wrappers around the binding callable or around the model; floats add:

| Fault | Meaning | Must be caught by |
|---|---|---|
| `next-ulp` | a non-NaN result moves one step up in the integer order `m` of 5.4 (`+inf` moves one step **down**, never into the NaN range; `-0` and `+0` both move to the smallest positive subnormal); a NaN result becomes `+0` | every bit-exact claim. Defined on the order, not as "bits + 1", because bits + 1 on a negative value moves the other way, on `+inf` yields a NaN, and on a NaN is invisible to `same` |
| `drift(k+1)` | replace the result by the value `k+1` steps from the **reference** (5.4), once upward and once downward as two controls; a direction that would leave the infinities is skipped, and if both are skipped on every case the claim is vacuous (W0601) | every `within_ulps(.., k)` claim (the **vacuity guard**) |
| `rel-drift` | replace the result by the nearest float to the **reference** that violates the bound, found by exact rational search, in each direction | every `rel_err_le` claim (the same guard; W0601 if it cannot be built) |
| `flip-zero-sign` | `-0 <-> +0` on zero results | claims over zero-bearing edges |
| `nan-to-zero` | NaN result becomes `+0` | claims that include NaN edges |
| `nan-to-inf` | NaN result becomes `+inf` | likewise |
| `drop-payload` | a payload NaN becomes the canonical NaN | payload-preserving ops only (`fneg fabs copysign bits`); a no-op on lanes that declare `payloads=lost` |
| `inf-to-max` | infinity result clamped to the largest finite | overflow claims |
| `ftz` | subnormal results flush to zero | claims over subnormal edges **and** probe P3/P4 |
| `fuse-fma` | `fadd(fmul(a,b),c)` computed fused | the probe P2 **and** any claim whose model is a multiply-add chain |
| `round-toward-zero` | truncate instead of RNE | the probe P5 **and** claims over tie edges |
| `round-half-up` | ties away from even | claims over tie edges (the 3.2 example) |
| `skip-fround` | `F32` computed in f64 with no narrowing | the probe P9 and F32 claims |
| `decimal-round-trip` | the corpus value passed through a decimal text rendering | a claim whose edge set includes a value that does not survive a short decimal |
| `reassociate` | a fold evaluated in a different order | the probe P6 and fold claims |
| `canonicalise-bits` | `bits_eq` demoted to `same` | claims that distinguish NaN payloads |
| `integer-saturate` | `f_to_int` saturates | the conversion claims at 2^31, 2^63 |

Controls on the *probe* itself: a "good-environment simulator" that returns the
right bits, and a "bad-environment simulator" per probe; a probe is accepted only
if its bad simulator makes it fail. A lane that has **no** way to simulate a
hazard (for instance it cannot enable FTZ) records the probe as `unmutated`
(open exemption printed), not as passed.

Every claim's evidence row records which controls it was run against and whether
each was caught. An uncaught control is handled by the existing rule of VER04i 4
(`uncaught_faults`, `UncaughtFault` unless the specification lists it in
`accepted_blind_faults`); the two tolerance guards can never be listed there.

## 14. Security considerations

1. **The model is the oracle.** An error in the softfloat is a silent error in
   every lane's evidence. Hence 8.2 (vectors, nightly exhaustive check, the
   shared `softfloat_corpus_sha256`) and 8.3 (a divergence blocks).
2. **Untrusted bit patterns reach the model.** Every one of the 2^32/2^64 patterns
   is a valid input, including NaN payloads. The model MUST NOT branch on a host
   `isnan`; it classifies by bit fields. Operations MUST be total; no input may
   raise, loop for unbounded time or allocate in proportion to its value
   (`f64_of_int` of a huge `Int` is decided from its bit length and top bits, so
   it is `+inf` without walking the number; `fdiv` and `fsqrt` are
   fixed-iteration; `f_to_int` yields at most 1,024 bits; the tolerance
   predicates are bounded by the caps of 5.4).
3. **Denial of service by size.** A fold of floats is charged like any evaluator
   work (7); the nightly `f32_all` suite is bounded by its declared count and runs
   in its own tier and jail (VER08b).
4. **Float-to-decimal is a side channel and a parser hazard.** There is no
   float-to-text in FL1; corpora refuse decimal; the Python oracle does not
   `float()` (10). Both are mechanical so a "reasonable" shortcut cannot
   reintroduce a locale-dependent or library-dependent conversion.
5. **Secrecy.** **Floats are not constant-time.** Many CPUs execute `fdiv`,
   `fsqrt` and operations on subnormals at data-dependent speeds (the subnormal
   slowdown is a published timing channel). A float-valued `secret` parameter is
   **refused** (W0401, an invalid label) in FL1; the VER20 timing obligations have no float model.
6. **Environment probing runs foreign flags.** In FL1 the probe only **reads** FPU
   state (MXCSR, FPCR) and never writes it; it never enables a hazard in the
   shipping process (the bad-environment simulators run in the jail, in a
   throwaway child process).
7. **Corpus integrity.** The hex-only rule closes the injection that a decimal
   string (`"1e999999999"`, `"nan(0x...)"`, a locale comma) opens in a lane's
   parser; the validator rejects such a field before a lane sees it.
8. **A tolerance claim is not a proof.** It is printed as `approx(...)` and never
   upgraded; it cannot authorise a security-relevant property that needs exactness.
9. **Hostile values reach the subject, not only the model.** A corpus may hold any
   pattern, signalling NaNs included. On a host with unmasked exceptions that is
   a crash on demand, so P13 must pass before any case runs, and a subject that
   dies on a case is a `fail` for that case inside the existing jail and time
   limit, never a hung or dead runner. The model never sees a host float, so no
   pattern can trap it.
10. **The probe file is corpus.** `probe.json` is hashed with the corpus and its
   expected bits are recomputed by the lane's model before use; a probe file
   that says a flushed result is correct fails that recomputation instead of
   blessing a bad environment.

## 15. The pilot: `trig`

The pilot package is `trig` (16 ports, 5 dependents; the memo's inventory). Its
documented algorithm is a 20-term series (but see 15.1), so both the bit-exact model and the FMA
probe **bite**: a port that contracts or reorders the series produces different
bits and fails the probe or the claim.

```text
op sin(x: F64) -> F64
model sin_ref(x: F64) -> F64 = fold(range(20), ...)   -- the documented series,
                                                      -- fadd/fmul in its order
claim sin_bit_exact: forall x: F64 in f64_edge. same(sin(x), sin_ref(x))
claim sin_accurate:  forall r: SinRow in sin_table.
                       within_ulps(sin(r.x), r.y, 4)  -- r.y from a pinned WX-E table
example neg_zero: bits_eq(sin(f64(0x8000000000000000)), f64(0x8000000000000000))
```

### 15.1 What the ports actually do (review finding, 2026-11-16)

The sketch above assumes one documented algorithm. Reading five of the sixteen
ports at this commit (Go, C, Python, Elixir, Lua; read, not run) shows that
assumption is **false today**, and it changes what the pilot must do first:

| Point | Observed | Consequence |
|---|---|---|
| number of terms | Go and Lua loop `n = 1..20` (21 terms); Python loops `range(1, 20)` (20 terms) | no single `sin_ref` is bit-exact for both |
| range reduction | Go `float64(int(x / TwoPI))`, C `(double)(long long)q`, Lua `math.modf` (all truncate, then multiply and subtract, with rounding at each step); Elixir `Float.round(x / two_pi)` (rounds to nearest); Python `x % TWO_PI` (an exact floored remainder, no intermediate rounding) | three different reductions, different bits; Elixir also multiplies in the other operand order; a model of the Python port needs an exact remainder, which FL1 puts in S7 |
| `radians`, `degrees` | Go, Python, Elixir, Lua `deg * PI / 180` (multiply, then divide); C `deg * (PI / 180.0)` (one multiply by a pre-divided constant) | two roundings against one: the C port differs in the last bit on some inputs |
| out-of-range reduction | the truncating cast is undefined in C, implementation-defined in Go (amd64 and arm64 differ), and raises in Python for a NaN or infinity | on `f64_edge` (which holds 2^63, 2^64, the largest finite, the infinities and NaNs) the ports **disagree by construction**; the op needs a stated domain or a stated result there |
| fusion | `x - TwoPI * k` in the reduction is a multiply-subtract | this is where P2 and `fuse-fma` bite; the series itself (`term * (-x*x) / denom`, `sum += term`) has nothing to fuse |
| `sin(-0)` | in the Go port the first term is `-0` and the next is `+0`, so the sum is `+0` | the `neg_zero` example above is probably **false** for the package as written. It is kept as an illustration of the rule of 3.4, not as a fact about `trig` |

None of this is a reason to drop `trig`: a pilot that exposes five real cross-port
divergences before any tool runs is doing its job. It is a reason to **order the
work** (FL-15, decided 2026-11-16: the ports converge on one reference
algorithm, as separate work with its own specification first, every port changed
in its own PR, and **the `trig` pilot waits for it**; backlog row TRIG-CONV):

Under that decision all three stages below run **after** convergence; the text
that follows keeps the stage structure but no stage starts before TRIG-CONV is
done.

* **Stage A, bit-exact on the smallest op.** `radians` and `degrees`
  (`deg * PI / 180`, one multiply and one divide, no reduction) are claimed
  under `same` over `f64_edge` in every lane. A reading pass over all sixteen
  ports comes first, and the ports that group the constant differently (C, at
  least) get a one-line change. This exercises the whole float pipeline (model,
  corpus, probe, faults, labels) on an op whose only dispute is one
  parenthesis, and it is the first real catch the feature makes.
* **Stage B, tolerance on `sin` and `cos` as they are.** `sin_accurate` against
  the pinned table, on a domain where every port's reduction is defined (the
  table's rows; no value whose quotient leaves the 64-bit integer range). This
  needs no port change and is honest about being `approx`.
* **Stage C, bit-exact `sin` and `cos`, only after convergence.** A `trig`
  specification fixes the term count, the reduction (truncation through
  `f_to_int`, with the result for a NaN, an infinity and an out-of-range quotient
  written down), the operand order and the sign of `sin(-0)`; the ports are
  changed to match; then `sin_bit_exact` is claimed. Until then it is not
  claimed in any lane: a bit-exact claim that some lanes cannot meet by design
  would fail the polyglot rule (VER04i), not the ports.

### 15.2 Pilot obligations

Pilot obligations (the acceptance for FL1-S6):

1. Every bit-exact claim (Stage A, and `sin_bit_exact` once Stage C lands) is kind
   `sampled  f64_edge(214)`; **never** `full`.
2. `sin_accurate` carries `approx(<=4 ulp)` and passes the `drift(5)` vacuity guard.
3. The probe is required in every lane that claims `trig`; a Go arm64 build with
   fusion on is expected to be `not-live` (a result, not a failure of the spec).
4. The pilot is run in **at least** one lane of each hazard class: C (flags), Go
   (fusion), Java (strict), JS (f64-only, no F32 fma), Python (f64-only),
   Elixir (non-finite) before the feature is declared adopted.
5. `transistors` (the 4004 chain floor, the inventory's second candidate) is the
   second adopter, after `trig` passes in all lanes that exist.

Why not a security package: the float-heavy security crates (`json-value`,
`json-serializer`, `bounded-json`) need text, float-to-text (FL2) and recursive
types, none of which this slice supplies (memo section 6). `trig` has no security
weight and that is stated, not hidden.

## 16. Out of scope

* **Decimal floats** (IEEE 754 decimal32/64/128) and **binary16, bfloat16, binary128.**
* **`long double`**, x87 80-bit, and any lane whose `float` is wider than the
  declared width. They are refused by the probe, not modelled.
* **Interval arithmetic**, directed rounding modes, rounding-mode control, and
  exception flags. (The memo does not provide interval arithmetic; if it is wanted
  it is a separate document.)
* **Float to text and text to float** (a later slice FL2, after text T1,
  version-pinned, with its own correctness model: shortest round-trip and
  correctly rounded parse).
* **Float map keys / set elements** (collections), **constant-time floats**,
  **proofs** about floats (status `not-live`), **complex numbers**, **SIMD**.
* **A correctly rounded libm** as a model (6).
* **Host float formatting** in any report or error beyond an informal aid.

## 17. Backlog-ready slice list

| Slice | Content | Acceptance |
|---|---|---|
| FL1-S0 | errata pass on this spec against the then-current VER04i/j/k/l/m; confirm stream tags `62`/`63`; grammar artifact: tokens and reserved names; the header parser accepts `v2` with floats | spec change record updated; v0/v1 byte-identity tests pass |
| FL1-S1 | `warrant-softfloat` language-neutral description plus the **Rust checker** model (integer-only, literate); the TestFloat-style vector set vendored with provenance; the exact rational reference of 8.2 item 2b in the generator; the checker evaluates float expressions; the relations; the type errors (W0301) | vectors pass; the rational reference agrees on the vectors and both edge lists; `==` on a float is W0301 with the message of 4.3; negative tests per diagnostic |
| FL1-S2 | corpus: generator (no `float()`), validator, `f32_edge`/`f64_edge`, stream tags, TFV `S` encoding, the probe set `probe.json`; `softfloat_corpus_sha256` | a decimal in a float position is a corpus error; edge hash pinned |
| FL1-S3 | the exhaustive binary32 nightly cross-check job (C reference build, SSE2) and the `f32_all` streaming domain (11.3), `W0706` for the rest | a deliberately wrong model is caught; per-PR runs print `not-run` for `full` |
| FL1-S4 | tolerance predicates (`within_ulps`, `rel_err_le`), the label printing, the `compare` key on bindings | `drift(k+1)` guard fires; exact-rational check has no float arithmetic |
| FL1-S5 | first two lanes (Python and Rust) end to end: model, probe, runner, faults; the manifest fields (`float_env_sha256`, `payloads`) and the report rows | both lanes pass the pilot; the bad-environment simulator fails each probe |
| FL1-S6 | pilot `trig` in the hazard-class lanes of 15, in the stages of 15.1: A (`radians`/`degrees` bit-exact, after the one-line port alignments), B (`sin`/`cos` tolerance); then the remaining lanes one at a time. the whole pilot, Stage C (bit-exact `sin`/`cos`) included, waits for the `trig` convergence specification and the per-port changes (FL-15, TRIG-CONV, decided 2026-11-16) | pilot acceptance 1 to 5 for the stages claimed |
| FL1-S7 | convenience slice: `frem`, `ffloor`/`fceil`/`ftrunc`/`fround` over bits, `fmin`/`fmax` with the IEEE 2019 definitions (decision FL-6) | each defined as exact bit-level models with vectors |
| FL2 | float to text and back (needs T1; version-pinned) | separate specification |

## 18. Backlog and roadmap rows needed (for the other branch)

This document does not edit `VER-BACKLOG.md` or VER30. The rows it needs:

| Row (suggested id) | Content |
|---|---|
| WX-FL0 | this specification (VER04p); status: written, **adversarial review required before FL1-S0** |
| WX-FL1 (S0 to S7) | the implementation slices of section 17; depends on WX-G, WX-D2, WX-F, LP-012, S1, T1 (memo section 5); **not** on WX-I128 |
| WX-FL-REVIEW | the adversarial review pass of the owner's decision (a): three reviewers (IEEE-754 numerics, polyglot lane, security) |
| WX-FL2 | float to text and back (blocked on T1) |
| LP-012 amendment | the case-count kind table gains the `f32_edge`/`f64_edge`/`f32_set`/`f64_set`/`f32_all` rows of 11.1 |
| a VER30 change record | floats are no longer frozen; the order sentence of memo section 5 |
| a VER13 assumption | A-FLOAT-ENV: "the lane's float probe is complete for the hazards the lane can have" (discharged per lane by the probe set of 9.1) |
| a VER04i amendment request | the stream tags `62`/`63`; TFV `S`-hex encoding of floats; per-lane `float_env=`, `payloads=`, `nonfinite=` keys; the probe files |

## 19. Open questions and decisions

FL-10, FL-14, FL-15 and FL-16 were resolved by the owner on 2026-11-16 and are decisions; the other rows keep their defaults.

| # | Question | Default |
|---|---|---|
| FL-1 | Is the `bits_eq` advisory on a possibly-NaN computed value an error? | advisory, printed as an exemption (no new code). Review: keep; it fails closed, so it is noise, not unsoundness |
| FL-2 | Compare canonical NaN or bitwise by default? | `same` (decided in 4.3; recorded here so a reviewer can contest it) |
| FL-3 | Where may a tolerance reference come from? | a bit-exact model or a pinned WX-E table; a differential claim is labelled `differential` |
| FL-4 | What happens to a lane whose probe fails? | `not-live`, no widening (9.2); decided |
| FL-5 | Should `ieee_eq` lift through composites? | no, scalars only, until a customer needs it |
| FL-6 | `frem`, `fmin`/`fmax`, `ffloor`/`fround`: which definitions? | FL1-S7, bit-level models, IEEE 754-2019 minimum/maximum semantics, with NaN to canonical |
| FL-7 | `f_to_int` truncates toward zero; should rounding variants exist? | truncate only, others written over `fclass` and `f64_of_int` |
| FL-8 | The vector source: Berkeley TestFloat (BSD-3 licence) vendored, or a generator we own? | vendor generated vector files with provenance; the generator is not vendored; the licence is checked at FL1-S1 |
| FL-9 | Should an exactly-representable decimal literal (`1.5f64`) be accepted for readability? | no in FL1; bits only; reconsider with FL2 |
| FL-10 | A nightly `f32_all` cost: acceptable at about 2^32 cases per lane per op? | **Resolved 2026-11-16 (owner).** Accepted for compiled lanes only. `f32_all` claims run on compiled lanes only (11.3 item 6); interpreted lanes print `not-run`. The model's exhaustive self-check runs on the Rust model only (8.2 item 2). One op per lane per night, sharded across the nightly matrix (11.3 item 4); if a lane exceeds the budget, binary32 `full` is never claimed in it (fail closed). |
| FL-11 | Is the NaN rule "arithmetic returns the canonical NaN" too strong for lanes that propagate payloads (`payloads=preserved` subject)? | the claim is under `same`; payload claims only on bit-ops; revisit if a customer needs `fadd` payloads |
| FL-12 | Should the shipping gate (VER00) treat a `not-live` float rung as blocking per lane or per package? | per lane, reported; the package-level policy is VER00's, not this document's |
| FL-13 | Are the probes sufficient? | the first review added P12 (x87 with 53-bit precision control), P13 (unmasked exceptions), the inexact case of P3, the isolated-input case of P4, division in P7, per-width payload checks in P8, and the where-and-when rules of 9.1. Still open: a probe cannot show what the compiler did to a **given** subject expression (rule 3 of 9.1); the remaining reviewers are asked to attack that |
| FL-14 | Should `within_ulps` treat an infinity as one step past the largest finite (5.4), or hold only when both are the same infinity? | **Resolved 2026-11-16 (owner).** The strict rule. `within_ulps` at an infinity holds only when both arguments are the same infinity; a finite value against an infinity is never within any `k` (5.4). It matches `rel_err_le`. |
| FL-15 | The `trig` ports do not share one algorithm (15.1). Converge them, or weaken the pilot? | **Resolved 2026-11-16 (owner).** Converge. The `trig` ports are converged on one reference algorithm as separate work (backlog row TRIG-CONV): a `trig` specification first, then every port updated in its own PR. The floats pilot waits for it (15.1, FL1-S6). It changes result bits in some ports; the owner has accepted that. |
| FL-16 | Go fuses on arm64, ppc64, s390x and riscv64 with no flag to stop it, so P2 makes every Go float claim `not-live` there. Accept, or allow a subject written with explicit `float64(x*y)` conversions to opt in? | **Resolved 2026-11-16 (owner).** Accept. Go float evidence is amd64-only, and the report says so (12.2). No opt-in for explicit `float64(x*y)` conversions: it would make the probe depend on trusting the subject's source. |

## 20. Specification change record

| Date | Change |
|---|---|
| 2026-11-16 | First draft: lifts the freeze (owner decision a); fixes RNE (3.2); stream tags `62`/`63`; floats join header `v2` |
| 2026-11-16 | Adversarial review 1 (numerics, lanes, security). **Semantics:** zero-sign table for every operation, `fma` included (3.4); overflow threshold and tininess (3.2); signalling NaNs and `fma(0, inf, c)` (3.3); `total_le` defined by an integer key (4.3); `f32_of_f64` subnormal results are rounded, not exact (5.3); `f_of_int` "width by context" replaced by `f32_of_int` and `f64_of_int`. **Tolerance:** same-width rule, the unit is representable steps, blindness to the sign of zero, infinities and NaN references in `rel_err_le`, caps on `k`, `num`, `den`; `drift` and `rel-drift` are measured from the reference, not from the subject's result; vacuity is W0601. **Model verification:** an exact rational reference; `f32_of_f64` checked on rounding boundaries instead of exact values; binary64 printed as `sampled`; cross-lane agreement named a transcription check. **Probes:** P1 made concrete, P3, P4, P7, P8, P10 corrected, P12 and P13 added, the probe bracketed around the run in the subject's build, process and thread. **Lanes:** JavaScript has no `fma`; Python `struct` raises on binary32 overflow and Python, Perl and Ruby raise instead of returning non-finite values; Go targets and constants; a non-existent GHC flag removed. **Compatibility:** `f32(0x..)` in a v1 document is an ordinary call, not W0101 (byte identity); all new names reserved in v2; no new diagnostic code (`W0301w` removed, the `secret` refusal is W0401, uncaught controls follow VER04i 4). **Pilot:** the `trig` ports differ in term count, range reduction and out-of-range behaviour, and `sin(-0)` is probably `+0`; the pilot is staged (15.1). New open questions FL-14 to FL-16 |
| 2026-11-16 | Owner decisions (VER30 decisions 16, 18 and 19): FL-14 `within_ulps` is strict at infinity (5.4); FL-15 the `trig` ports converge first, as separate work, and the pilot waits (15.1, FL1-S6, backlog TRIG-CONV); FL-16 Go float evidence is amd64-only (12.2); FL-10 `f32_all` runs on compiled lanes only and the model self-check on Rust only (8.2, 11.3 item 6). Stream tags `62`/`63` kept; VER04r moved to `64`/`65` (VER04i 2.6.1 registry) |
