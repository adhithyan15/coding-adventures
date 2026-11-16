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
stream tags `60`/`61`, LP-012 kinds), [VER30](VER30-warrant-expansion-roadmap.md)
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
| built-ins `fadd fsub fmul fdiv fsqrt fma fneg fabs copysign f32_of_f64 f64_of_f32 f_to_int f_of_int bits f32_of_bits f64_of_bits fclass` | 5 |
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
(VER04l 2.1). Floats join `v2`; this document defines only its own elements. A
float construct in a v0 or v1 document is **W0101** ("float construct in a
document older than v2"). A float construct in a v2 document that does not use
text is legal; text and floats are independent (section 17 FL1-S0 confirms the
header parser accepts a v2 document using either, both or neither).

| Element | Kind | In a v0/v1 document | In a v2 document |
|---|---|---|---|
| `F32`, `F64` | type names | ordinary identifiers | reserved (W0201/W0203 as a declaration or binder name) |
| the 17 built-in names of 5 | function names | ordinary identifiers; an undeclared call is W0202 | reserved |
| `f32(0x..)`, `f64(0x..)` | literals | W0101 | literals (4.2) |
| decimal literals such as `1.5f64` | none | W0101 | **not accepted at all** in FL1 (open question FL-9); write the bits |
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
  are **not** propagated by arithmetic.
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
engines are the standing example).

### 3.4 Signed zero

+0 and -0 are distinct values with distinct bits and distinct behaviour:

```text
fdiv(1, +0) = +infinity          fdiv(1, -0) = -infinity
fadd(+0, -0) = +0 (RNE)          fadd(-0, -0) = -0
fmul(-1, +0) = -0                fsqrt(-0)    = -0
copysign(3, -0) = -3             ieee_eq(+0, -0) = true     same(+0, -0) = false
```

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
model computed through arithmetic and that may be a NaN is a **warning W0301w**
("bit-compare of a possible NaN; arithmetic NaNs are canonical, the lane's may not
be") reported in the assurance report as an open exemption. `ieee_eq` is reflexive
only on non-NaN; it is offered because specifications often say "equal" to mean
this, and the author must then say so.

Order predicates (total functions to `Bool`, never operators in FL1):
`ieee_lt(a, b)` and `ieee_le(a, b)` are the IEEE partial order (false if either is
NaN; `-0` and `+0` are neither less nor greater); `total_le(a, b)` is IEEE
`totalOrder`: `-NaN < -inf < ... < -0 < +0 < ... < +inf < +NaN`, NaNs ordered by
payload. `<` and `<=` on floats are W0301, so no author can silently write a
partial order where they meant a total one.

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
and fma are the two operations IEEE requires to be correctly rounded **and** that
every target has natively; they are the building blocks of every algorithm worth
verifying. The rounding functions are bit manipulations that an author can state
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
| `f32_of_f64` | `F64 -> F32` | rounds (RNE); overflow to infinity; subnormal results exact; NaN to canonical F32 NaN |
| `f64_of_f32` | `F32 -> F64` | exact; NaN to canonical F64 NaN |
| `f_of_int` | `(Int) -> Fw` (width by context type) | the integer, rounded RNE; an `Int` above the format's range is infinity |
| `f_to_int` | `Fw -> Option<Int>` | **truncates toward zero** (a documented choice, FL-7); `None` for NaN and the infinities; any finite value gives `Some` of the exact mathematical integer, so no saturation and no host-defined overflow |

Conversion from the fixed-width integers goes through `Int` (they widen to `Int`),
so `f_of_int` has one rule, not eight. The exact `Int` result is what makes
`f_to_int` portable: C's `(int)f` is undefined behaviour out of range, Java
saturates, JavaScript wraps modulo 2^32 in `|0`; none of them is ever the model.

### 5.4 Tolerance claims

`within_ulps(x, y, k)` and `rel_err_le(x, y, num, den)` are the only two
approximation predicates. Their **semantics are fixed, written out and printed**:

* **ulp distance.** Map each non-NaN float to a monotone integer: for bits `u`,
  `m(u) = u` if the sign bit is 0 else `-(u & ~sign)`, so `m(-0) = m(+0) = 0` and
  adjacent representable values differ by exactly 1 (the zeros coincide).
  `ulp_dist(x, y) = |m(x) - m(y)|` over `Int` (exact; no overflow, because `Int`
  is mathematical). **`ulp_dist` is infinite, and `within_ulps` false, if either
  argument is NaN**, and between a finite value and an infinity it is the
  difference of the mapped integers (so `max_finite` and `+inf` are 1 apart: that
  is stated, not hidden). `within_ulps(x, y, k)` holds iff `ulp_dist(x, y) <= k`,
  `k: Nat`, `k` a literal or a bound parameter (never computed from the subject).
* **Relative error.** `rel_err_le(x, y, num, den)` holds iff both are finite and
  `|x - y| * den <= |y| * num`, evaluated in **exact rational arithmetic** (`Int`
  products of the significands and exponents; no float arithmetic is used to check
  a float claim). If `y = 0` it holds iff `x = y` bitwise-zero-or-negative-zero
  (that is, `x` is a zero). `num`, `den: Nat`, `den > 0`. Subnormal `y` is
  allowed and the formula is the same: the bound is relative, so on subnormals it
  is far stricter than an ulp bound near zero, and the author must pick the one
  they mean.
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
  `drift(k+1)` (13): a tolerance claim that cannot detect a deliberate error one
  ulp over its own bound is vacuous and is W0602 ("incomplete evidence").

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
bit patterns and `Int` arithmetic. Budgets: `fdiv` and `fsqrt` need a 64x64 to
128-bit multiply internally; this is implemented with `U64` limbs and **does not
wait for any 128-bit integer type** (WX-I128 is not a dependency). Float
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
* implement `fadd fsub fmul fdiv fsqrt fma f32_of_f64 f64_of_f32 f_of_int f_to_int`
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
   The vector file is data; checking that a lane passes it is an L0 suite of
   `exhaustive`-labelled kind `bounded-set` (the vector set is a named finite set).
2. **Exhaustive binary32 check, nightly.** Because `F32` is only 2^32 patterns,
   the unary operations are checked **over every input** (`fsqrt`, `f64_of_f32`,
   `f_to_int`, `fneg`, `fabs`, `fclass`, `f32_of_f64` of every f32-exact value) and
   the binary operations over a structured, enumerable complete cross-product of
   a stratified set (every exponent pair x a mantissa class, about 2^24 pairs),
   against a **second, independent implementation**: the native C compiler's
   `float` in a **probe-passing, reference** environment (SSE2, `-ffp-contract=off`),
   used here **only to cross-check the softfloat, never as the oracle of a
   Warrant claim**. A disagreement is a defect of one of the two and blocks. This
   is a nightly job, not per-PR, and it is the evidence behind the label
   `bounded-set` (not `full`) for the binary operations.
3. **An independent algorithmic cross-check.** Each lane's softfloat is also run
   against the Rust checker's softfloat on the **same** cases generated from the
   corpus; the manifests carry `softfloat_corpus_sha256`, which every lane must
   reproduce (a lane cannot choose its own test points).
4. **Sabotage of the model.** The model has its own controls (13): `round-half-up`
   in the model, `ftz-in-model`, `payload-propagating-nan`. Each must be caught by
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
| `P1-double-rounding` | the exact tie/near-tie sum of 2^53 + 1 + 2^-1 in binary64 sequence | x87 extended precision (`FLT_EVAL_METHOD != 0`), double rounding |
| `P2-fma-contract` | `a*b + c` chosen so a fused result differs from separately rounded in the last bit | FMA contraction (`-ffp-contract=fast`, Go on arm64/ppc64/s390x, Julia-style auto-fusion) |
| `P3-flush-subnormal-out` | `2^-1022 * 0.5` (binary64), `2^-126 * 0.5` (binary32) | flush-to-zero (FTZ), `-ffast-math`, GPU/NEON defaults |
| `P4-flush-subnormal-in` | `2^-1074 + 2^-1074`; `2^-149 + 2^-149` | denormals-are-zero (DAZ) |
| `P5-rounding-mode` | a tie result for add, mul, div and `f32_of_f64` | a non-default rounding mode left by a library |
| `P6-reassociation` | `(a + b) + c` against `a + (b + c)` on values where they differ | reassociation (`-ffast-math`, `-fassociative-math`, LLVM `fast`) |
| `P7-sqrt-exact` | `sqrt` of a perfect square and of a hard-rounding value | an approximate `sqrt` (reciprocal-sqrt estimate with refinement) |
| `P8-nan-bits` | the bits of `0.0/0.0`, `inf-inf`, a NaN round-trip | engines that rewrite payloads; selects the `payloads=` declaration (12) |
| `P9-narrow` | `f32_of_f64` of tie/overflow/subnormal values; a binary32 chain | `F32` computed in a wider type (`skip-fround`); JS without `Math.fround` |
| `P10-convert-int` | `f_of_int`/`f_to_int` at 2^24, 2^53, 2^63, 2^64 | a lane that casts through a 53-bit double or saturates |
| `P11-compile-time` | the same arithmetic on **runtime-opaque** inputs and on **constant-folded** inputs; both must match | a compiler or JIT that folds with a different precision than the runtime |

The probe operands are loaded from the corpus at runtime and passed through an
opaque barrier (a volatile read, an FFI call, a `black_box`) so that the compiler
cannot fold them: P11 is the control for that.

### 9.2 The `not-live` rule

**If any probe in P1 to P7, P9, P10, P11 fails, or the lane cannot run the probe
at all, the lane reports every float-using suite's rung as `not-live`**, with the
reason printed (`not-live: float-env P2-fma-contract`), never as `fail` and never
omitted. P8 is different: it sets the lane's `payloads` declaration to `lost`, so
only payload-preservation claims in that lane become `not-live` and everything
else proceeds. The rules:

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
| canonical stream (VER04i 2.6) | `62` then 4 bytes little-endian for `F32`; `63` then 8 bytes little-endian for `F64`. (`60`/`61` are VER04l's; `62`/`63` are the next free tags; FL1-S0 reconfirms against the then-current VER04i/l/m.) |
| `cases.tfv` (TF01 TFV) | a TFV string value `S` holding the lowercase hex bit pattern, 8 digits for `F32`, 16 for `F64`, no `0x` prefix, under a key that carries the width. **Not** a TFV integer, because a U64 parsed as a signed 64-bit integer by a lane would be a defect for patterns above 2^63 |
| `corpus.json` | a JSON **string** of the same hex; a JSON number, or a string that is not exactly 8/16 hex digits, is a **corpus error** |
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

(the bits of pi/2 in, the model's bits of the reference out).

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
   skips patterns is a failure, not a smaller `full`);
5. the lane's float probe passed in that same run (9).

Anything not meeting all five is demoted to `sampled` or refused with W0706; there
is no path by which an unmet condition still prints `full`. `F64` has no such
path at all.

## 12. Lane binding contract per language family

### 12.1 The rule that applies to every lane

The binding file (VER04i 5) gains three optional data keys on any float-using
op: `float_env=` (the lane's probe-result reference), `payloads=preserved|lost`
(default `lost`), `nonfinite=ok|rejects:<CASE>|unreachable` (12.3 Elixir). **The
binding is data only**; it names the subject callable and the width, never an
inline float expression. Every lane implements the **same eleven probes** and the
**same softfloat model**, so equal rigour is a property of the corpus, not of the
lane's goodwill. A lane that does not (yet) implement them has no float claims:
`not-live`.

### 12.2 The table

| Lane family | Hazard and binding rule |
|---|---|
| **C, C++** | `-ffp-contract=off`, no `-ffast-math`/`-Ofast`/`-funsafe-math-optimizations`/`-fassociative-math`/`-ffinite-math-only`; `-fno-fast-math` explicit; `FLT_EVAL_METHOD == 0` asserted at compile time and probed at run time; `-msse2 -mfpmath=sse` on x86 (x87 targets, 32-bit x86 without SSE2, are **refused**); `-frounding-math` NOT used because the default is RNE; `-fno-builtin` is not required but the probe's P11 detects constant folding. Compilers pinned (`gcc -std=c11`, `clang`); MSVC `/fp:precise` or `/fp:strict`, never `/fp:fast`. FTZ/DAZ bits in MXCSR are probed (P3/P4) because `-ffast-math` and some libraries set them at load time |
| **Rust** | no fusion by default (Rust never contracts); i586/x87 targets refused (probed); `f32`/`f64` bit-casts via `to_bits`/`from_bits`; NaN bits are not guaranteed (hence `same`); `payloads` declared per target; `std` libm calls are subjects only |
| **Go** | the spec **permits** fusing `x*y + z` on arm64, ppc64, s390x (and others); the explicit `float64(x*y) + z` conversion prevents fusion in a hand-written model but the probe P2 is the arbiter, and a fused build **fails honestly** (`not-live`, not a workaround). `math.Float64bits`/`Float64frombits` for I/O. `math.FMA` is the subject for `fma` |
| **Java, Kotlin (JVM)** | strict IEEE semantics since Java 17 (JEP 306); **Java 16 and older and Kotlin targeting them are refused** unless the class is `strictfp` and the JVM option set is probed; `Float.floatToRawIntBits` (never `floatToIntBits`, which canonicalises NaN) and `Double.doubleToRawLongBits`; `Math.fma` for `fma`; `StrictMath` vs `Math` is a *subject* choice, never an oracle; Kotlin's `Float`/`Double` use the same raw-bits functions |
| **C#, F#** | .NET Core 3.0+ and .NET 5+ only (RyuJIT, SSE2, IEEE-conforming); the legacy x87 JIT (.NET Framework 32-bit) refused; `BitConverter.DoubleToInt64Bits`/`SingleToInt32Bits`; `Math.FusedMultiplyAdd`; JIT tiering is probed by P11 (opaque inputs) |
| **JS/TS (Node, Bun, Deno), Dart web** | **only binary64 exists**; `F64` is `number`; `F32` is computed as `Math.fround(op(...))` after **each** `+ - * / sqrt` of `Math.fround`ed operands, which is *exact* by the double-rounding-innocuity theorem for these operations (53 >= 2*24 + 2), and is **not** valid for `fma`, so `fma` on `F32` in JS is computed by the softfloat only (the subject cannot be called, a documented `not-live` row). Floats cross as `BigInt` bits through `DataView` (`setBigUint64`/`getFloat64`), never as `number` parsed from text; engines may rewrite NaN payloads, so `payloads=lost` unless P8 proves otherwise; Dart web is the same; Dart VM is native. `Number` is never used as a corpus value type |
| **Python** | f64 only (CPython `float`); `F32` through `struct.pack('<f', x)`/`unpack` round-trip (which rounds RNE), and arithmetic on `F32` as f64 followed by that round-trip (exact for + - * / sqrt, the same double-rounding argument); NaN payloads survive `struct` on CPython but are not guaranteed; the oracle in the generator never uses `float()` on corpus text (10). `numpy` float32, if used in a lane, is a subject library, never a model |
| **Ruby, Lua, Perl** | f64 only (Lua 5.3+ `string.pack('<d')`, `<f`; Ruby `[x].pack('E')`/`'e'`; Perl `pack 'd<'`/`'f<'`). **Perl refuses a build whose `NV` is a long double** (`$Config{nvsize} == 8` asserted); Lua is built with `LUA_FLOAT_TYPE = double`; Ruby `Float` is double. The lane's integer model uses the width rules of VER04i 2.7 |
| **Elixir/Erlang (BEAM)** | BEAM floats **cannot be NaN or infinite**: an operation that would produce one raises `badarith`. Non-finite values cross as **bits** (integers, never a BEAM float); an op that would return a non-finite value is bound `nonfinite: rejects:<CASE>` (the subject raises) or the case is `not-live`; the model itself runs on integer bits, unaffected |
| **Haskell, OCaml, Swift** | native IEEE; adapters compare **bits** (`castDoubleToWord64`, `Int64.bits_of_float`, `Double.bitPattern`), never the language's `Eq`; GHC `-fno-fast-math` and no `-ffast-math`-like pragmas; OCaml float arrays are unboxed doubles, fine; Swift `Float`/`Double` with `-Ounchecked` banned |
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
| `next-ulp` | result bits + 1 (as an unsigned, wrapping within the class) | every bit-exact claim |
| `drift(k+1)` | move the result `k+1` ulps | every `within_ulps(.., k)` claim (the **vacuity guard**) |
| `rel-drift` | scale the error just past `num/den` | every `rel_err_le` claim |
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
each was caught; an uncaught control makes the claim W0602.

## 14. Security considerations

1. **The model is the oracle.** An error in the softfloat is a silent error in
   every lane's evidence. Hence 8.2 (vectors, nightly exhaustive check, the
   shared `softfloat_corpus_sha256`) and 8.3 (a divergence blocks).
2. **Untrusted bit patterns reach the model.** Every one of the 2^32/2^64 patterns
   is a valid input, including NaN payloads. The model MUST NOT branch on a host
   `isnan`; it classifies by bit fields. Operations MUST be total; no input may
   raise, loop for unbounded time or allocate in proportion to its value
   (`f_of_int` of a huge `Int` is `+inf` after a bounded loop, `fdiv` and
   `fsqrt` are fixed-iteration).
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
   **refused** (W0602) in FL1; the VER20 timing obligations have no float model.
6. **Environment probing runs foreign flags.** The probe sets or reads FPU state
   (MXCSR, FPCR) only through reads in FL1; it never enables a hazard in the
   shipping process (the bad-environment simulators run in the jail, in a
   throwaway child process).
7. **Corpus integrity.** The hex-only rule closes the injection that a decimal
   string (`"1e999999999"`, `"nan(0x...)"`, a locale comma) opens in a lane's
   parser; the validator rejects such a field before a lane sees it.
8. **A tolerance claim is not a proof.** It is printed as `approx(...)` and never
   upgraded; it cannot authorise a security-relevant property that needs exactness.

## 15. The pilot: `trig`

The pilot package is `trig` (16 ports, 5 dependents; the memo's inventory). Its
documented algorithm is a 20-term series, so both the bit-exact model and the FMA
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

Pilot obligations (the acceptance for FL1-S6):

1. `sin_bit_exact` kind `sampled  f64_edge(214)`; **never** `full`.
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
| FL1-S1 | `warrant-softfloat` language-neutral description plus the **Rust checker** model (integer-only, literate); the TestFloat-style vector set vendored with provenance; the checker evaluates float expressions; the relations; the type errors (W0301) | vectors pass; `==` on a float is W0301 with the message of 4.3; negative tests per diagnostic |
| FL1-S2 | corpus: generator (no `float()`), validator, `f32_edge`/`f64_edge`, stream tags, TFV `S` encoding, the probe set `probe.json`; `softfloat_corpus_sha256` | a decimal in a float position is a corpus error; edge hash pinned |
| FL1-S3 | the exhaustive binary32 nightly cross-check job (C reference build, SSE2) and the `f32_all` streaming domain (11.3), `W0706` for the rest | a deliberately wrong model is caught; per-PR runs print `not-run` for `full` |
| FL1-S4 | tolerance predicates (`within_ulps`, `rel_err_le`), the label printing, the `compare` key on bindings | `drift(k+1)` guard fires; exact-rational check has no float arithmetic |
| FL1-S5 | first two lanes (Python and Rust) end to end: model, probe, runner, faults; the manifest fields (`float_env_sha256`, `payloads`) and the report rows | both lanes pass the pilot; the bad-environment simulator fails each probe |
| FL1-S6 | pilot `trig` in the hazard-class lanes of 15; then the remaining lanes one at a time | pilot acceptance 1 to 5 |
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

## 19. Open questions

| # | Question | Default |
|---|---|---|
| FL-1 | Is the `bits_eq` warning on a possibly-NaN computed value an error? | warning W0301w, printed as an exemption |
| FL-2 | Compare canonical NaN or bitwise by default? | `same` (decided in 4.3; recorded here so a reviewer can contest it) |
| FL-3 | Where may a tolerance reference come from? | a bit-exact model or a pinned WX-E table; a differential claim is labelled `differential` |
| FL-4 | What happens to a lane whose probe fails? | `not-live`, no widening (9.2); decided |
| FL-5 | Should `ieee_eq` lift through composites? | no, scalars only, until a customer needs it |
| FL-6 | `frem`, `fmin`/`fmax`, `ffloor`/`fround`: which definitions? | FL1-S7, bit-level models, IEEE 754-2019 minimum/maximum semantics, with NaN to canonical |
| FL-7 | `f_to_int` truncates toward zero; should rounding variants exist? | truncate only, others written over `fclass` and `f_of_int` |
| FL-8 | The vector source: Berkeley TestFloat (BSD-3 licence) vendored, or a generator we own? | vendor generated vector files with provenance; the generator is not vendored; the licence is checked at FL1-S1 |
| FL-9 | Should an exactly-representable decimal literal (`1.5f64`) be accepted for readability? | no in FL1; bits only; reconsider with FL2 |
| FL-10 | A nightly `f32_all` cost: acceptable at about 2^32 cases per lane per op? | one op per lane per night, sharded across the nightly matrix; if it exceeds the budget, binary32 `full` is simply never claimed in that lane (fail closed) |
| FL-11 | Is the NaN rule "arithmetic returns the canonical NaN" too strong for lanes that propagate payloads (`payloads=preserved` subject)? | the claim is under `same`; payload claims only on bit-ops; revisit if a customer needs `fadd` payloads |
| FL-12 | Should the shipping gate (VER00) treat a `not-live` float rung as blocking per lane or per package? | per lane, reported; the package-level policy is VER00's, not this document's |
| FL-13 | Are the 11 probes sufficient (e.g. MXCSR exception masks, signalling-NaN quieting on loads, x87 control word on 64-bit Windows)? | adversarial review (WX-FL-REVIEW) is asked to extend them |

## 20. Specification change record

| Date | Change |
|---|---|
| 2026-11-16 | First draft: lifts the freeze (owner decision a); fixes RNE (3.2); stream tags `62`/`63`; floats join header `v2` |
